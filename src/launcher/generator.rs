use crate::launcher::config::load_project_config;
use crate::spinner_utils::{create_spinner_with_message, stop_and_persist_spinner_with_message};
use std::fs;
use std::io::{self, Cursor, Write};
use std::process::Command;
use zip::write::FileOptions;

use super::template::{LAUNCHER_TEMPLATE, CARGO_TOML};

pub struct LauncherGenerator<'a> {
    config: crate::BuilderConfig<'a>,
}

impl<'a> LauncherGenerator<'a> {
    pub fn new(config: crate::BuilderConfig<'a>) -> Self {
        Self { config }
    }

    pub fn generate_and_compile(&self) -> io::Result<()> {
        let sp: spinners::Spinner = create_spinner_with_message("Generating launcher template ...");
        let source = self.generate_source()?;
        stop_and_persist_spinner_with_message(sp, "Launcher source code generated");

        self.write_and_compile_source(&source)
    }

    fn generate_zip_payload(&self) -> io::Result<Vec<u8>> {
        let mut cursor = Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, ()>::default();

        // Add source files
        for file in &self.config.source_files {
            zip.start_file(file.relative_path.to_str().unwrap(), options)?;
            zip.write_all(&file.content)?;
        }

        // Add manifest
        zip.start_file("pyproject.toml", options)?;
        zip.write_all(&self.config.manifest)?;

        zip.finish()?;
        Ok(cursor.into_inner())
    }

    fn generate_source(&self) -> io::Result<String> {
        // Generate the zip payload
        let zip_data = self.generate_zip_payload()?;
        
        // Create embedded directory
        fs::create_dir_all("payload/embedded")?;
        
        // Write payload.zip and uv binary to embedded directory
        fs::write("payload/embedded/payload.zip", &zip_data)?;
        fs::write("payload/embedded/uv", &self.config.uv_binary)?;
        
        let launcher_config = load_project_config(&self.config.source_dir.to_path_buf());

        // Check for pre and post run hooks
        let hooks = launcher_config.hooks.unwrap();
        let prerun = &hooks.pre_run.unwrap();
        let postrun = &hooks.post_run.unwrap();

        // Create and modify launcher template
        let mut template = LAUNCHER_TEMPLATE.replace("{entrypoint}", &launcher_config.package.entrypoint);

        // hooks
        if prerun != "" {
            template = template.replace("{prerun}", prerun)
        }
        if postrun != "" {
            template = template.replace("{postrun}", postrun)
        }

        // extract to temporary dir
        template = template.replace("{extract_to_temp}", &self.config.extract_to_temp.to_string());
        // delete extracted directory
        template = template.replace("{delete_after_run}", &self.config.delete_after_run.to_string());
        
        Ok(template)
    }

    fn cross_compile(&self) -> io::Result<()> {
        let mut child = Command::new("cross")
            .arg("build")
            .arg("--release")
            .arg("--target")
            .arg(self.config.cross.as_ref().unwrap())
            .current_dir("payload")
            .env(
            "RUSTFLAGS",
            "-C opt-level=z -C target-cpu=native -C link-arg=-s -C embed-bitcode=yes -C codegen-units=1",
            )
            .spawn()?;

        if !child.wait()?.success() {
            eprintln!("Failed to compile the launcher binary.");
            std::process::exit(1);
        }

        fs::copy(
            "payload/target/release/pycrucible-launcher",
            &self.config.output_path,
        )?;
        println!("Launcher binary created at: {}", self.config.output_path);
        Ok(())
        }

    fn native_compile(&self) -> io::Result<()> {
        let mut child = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .current_dir("payload")
            .env(
            "RUSTFLAGS",
            "-C opt-level=z -C target-cpu=native -C link-arg=-s -C embed-bitcode=yes -C codegen-units=1",
            )
            .spawn()?;

        if !child.wait()?.success() {
            eprintln!("Failed to compile the launcher binary.");
            std::process::exit(1);
        }

        fs::copy(
            "payload/target/release/pycrucible-launcher",
            &self.config.output_path,
        )?;
        println!("Launcher binary created at: {}", self.config.output_path);
        Ok(())
        }

    fn write_and_compile_source(&self, source: &str) -> io::Result<()> {
        let generated_source_path = "payload/src/main.rs";

        fs::create_dir_all("payload/src")?;
        fs::write(generated_source_path, source)?;

        fs::write("payload/Cargo.toml", CARGO_TOML)?;

        if self.config.cross.is_some() {
            self.cross_compile()?;
        } else {
            self.native_compile()?;
        }

        // Clean up temporary files
        fs::remove_dir_all("payload")?;

        Ok(())
        }
    }

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;
    #[test]
    fn test_launcher_generator_new() {
        let temp_path = PathBuf::new();
        let config = crate::BuilderConfig {
            source_files: vec![],
            manifest: vec![],
            uv_binary: vec![],
            source_dir: temp_path.as_path(),
            output_path: String::new(),
            cross: None,
            extract_to_temp: true,
            delete_after_run: false
        };
        let generator = LauncherGenerator::new(config);
        assert!(generator.config.source_files.is_empty());
    }

    #[test]
    fn test_generate_zip_payload() -> io::Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, b"test content")?;

        let config = crate::BuilderConfig {
            source_files: vec![crate::SourceFile {
                relative_path: PathBuf::from("test.txt"),
                content: b"test content".to_vec(),
            }],
            manifest: b"[project]".to_vec(),
            uv_binary: vec![],
            source_dir: temp_dir.path(),
            output_path: String::new(),
            cross: None,
            extract_to_temp: true,
            delete_after_run: false
        };

        let generator = LauncherGenerator::new(config);
        let payload = generator.generate_zip_payload()?;
        assert!(!payload.is_empty());

        Ok(())
    }

    #[test]
    fn test_generate_source() -> io::Result<()> {
        let temp_dir = tempdir()?;
        fs::create_dir_all(temp_dir.path().join("src"))?;
        fs::write(temp_dir.path().join("src/main.py"), b"print('test')")?;
        fs::write(temp_dir.path().join("pyproject.toml"), b"[project]\nname='test'")?;

        let config = crate::BuilderConfig {
            source_files: vec![crate::SourceFile {
                relative_path: PathBuf::from("src/main.py"),
                content: b"print('test')".to_vec(),
            }],
            manifest: b"[project]\nname='test'".to_vec(),
            uv_binary: b"test binary".to_vec(),
            source_dir: temp_dir.path(),
            output_path: String::new(),
            cross: None,
            extract_to_temp: true,
            delete_after_run: false
        };

        let generator = LauncherGenerator::new(config);
        let source = generator.generate_source()?;
        assert!(source.contains("use rust_embed::RustEmbed"));
        assert!(source.contains("#[derive(RustEmbed)]"));
        assert!(source.contains("#[folder = \"embedded\"]"));

        // Check that the embedded files were created
        assert!(fs::metadata("payload/embedded/uv").is_ok());
        assert!(fs::metadata("payload/embedded/payload.zip").is_ok());

        Ok(())
    }
}
