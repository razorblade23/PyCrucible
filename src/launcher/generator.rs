use crate::launcher::config::load_project_config;
use crate::spinner_utils::{create_spinner_with_message, stop_and_persist_spinner_with_message};
use std::fs;
use std::io::{self, Cursor, Write};
use std::process::Command;
use zip::write::FileOptions;

use super::template::LAUNCHER_TEMPLATE;
use std::path::PathBuf;
use tempfile::tempdir;

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
        let zip_data = self.generate_zip_payload()?;
        let zip_array = byte_array_literal(&zip_data);
        let uv_binary_array = byte_array_literal(&self.config.uv_binary);
        let launcher_config = load_project_config(&self.config.source_dir.to_path_buf());

        Ok(LAUNCHER_TEMPLATE
            .replace("{zip_binary_array}", &zip_array)
            .replace("{uv_binary_array}", &uv_binary_array)
            .replace("{entrypoint}", &launcher_config.package.entrypoint))
    }

    fn write_and_compile_source(&self, source: &str) -> io::Result<()> {
        let generated_source_path = "payload/src/main.rs";

        fs::create_dir_all("payload/src")?;
        fs::write(generated_source_path, source)?;

        // Creating Cargo.toml for the launcher
        let cargo_toml = r#"[package]
name = "pycrucible-launcher"
version = "0.1.0"
edition = "2024"

[dependencies]
zip = { version = "3", default-features = false, features = ["deflate"] }

[profile.release]
opt-level = "z"     # Optimize for size
lto = "fat"         # More aggressive LTO
codegen-units = 1   # Optimize for size
panic = "abort"     # Remove panic unwinding
strip = "symbols"   # More aggressive stripping
debug = false       # No debug symbols
debug-assertions = false
incremental = false
overflow-checks = false
"#;

        fs::write("payload/Cargo.toml", cargo_toml)?;

        // Check for cross-compilation flag
        if self.config.cross.is_some() {
            let mut child = Command::new("cross")
                .arg("build")
                .arg("--release")
                .arg("--target")
                .arg(self.config.cross.as_ref().unwrap())
                .current_dir("payload")
                .env(
                    "RUSTFLAGS",
                    "-C opt-level=z -C target-cpu=native -C link-arg=-s -C embed-bitcode=yes -C lto=fat -C codegen-units=1",
                )
                .spawn()?;
            let status = child.wait()?;
                if !status.success() {
                    eprintln!("Failed to compile the launcher binary.");
                    std::process::exit(1);
                } else {
                    // Copy the binary to the desired output location
                    fs::copy(
                        "payload/target/release/pycrucible-launcher",
                        &self.config.output_path,
                    )?;
                    println!("Launcher binary created at: {}", self.config.output_path);
                }
        } else {
            let mut child = Command::new("cargo")
                .arg("build")
                .arg("--release")
                .current_dir("payload")
                .env(
                    "RUSTFLAGS",
                    "-C opt-level=z -C target-cpu=native -C link-arg=-s -C embed-bitcode=yes -C lto=fat -C codegen-units=1",
                )
                .spawn()?;

            let status = child.wait()?;
            if !status.success() {
                eprintln!("Failed to compile the launcher binary.");
                std::process::exit(1);
            } else {
                // Copy the binary to the desired output location
                fs::copy(
                    "payload/target/release/pycrucible-launcher",
                    &self.config.output_path,
                )?;
                println!("Launcher binary created at: {}", self.config.output_path);
            }
        }
        // Clean up temporary files
        fs::remove_dir_all("payload")?;

        Ok(())
    }
}

fn byte_array_literal(data: &[u8]) -> String {
    data.iter()
        .map(|b| b.to_string())
        .collect::<Vec<String>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_array_literal() {
        let data = vec![1, 2, 3, 255];
        assert_eq!(byte_array_literal(&data), "1, 2, 3, 255");
    }

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
        };

        let generator = LauncherGenerator::new(config);
        let payload = generator.generate_zip_payload()?;
        assert!(!payload.is_empty());

        Ok(())
    }
}
