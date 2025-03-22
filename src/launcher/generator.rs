use std::fs;
use std::io;
use std::process::Command;
use spinners::{Spinner, Spinners};

use super::template::LAUNCHER_TEMPLATE;

pub struct LauncherGenerator {
    config: crate::BuilderConfig,
}

impl LauncherGenerator {
    pub fn new(config: crate::BuilderConfig) -> Self {
        Self { config }
    }

    pub fn generate_and_compile(&self) -> io::Result<()> {
        let mut sp = Spinner::new(Spinners::Dots9, "Generating launcher source code ...".into());
        let source = self.generate_source();
        sp.stop_and_persist("✔", "Launcher source code generated".into());

        self.write_and_compile_source(&source)
        
    }

    fn generate_source_files_map(&self) -> String {
        let mut entries = Vec::new();
        
        // Convert byte arrays to Vec<u8>
        for file in &self.config.source_files {
            entries.push(format!(
                r#""{}" => vec![{}]"#,
                file.relative_path.to_str().unwrap(),
                byte_array_literal(&file.content)
            ));
        }
    
        // Add manifest file
        entries.push(format!(
            r#""pyproject.toml" => vec![{}]"#,
            byte_array_literal(&self.config.manifest)
        ));
    
        format!("hashmap! {{\n    {} \n}}", entries.join(",\n    "))
    }

    fn generate_source(&self) -> String {
        let source_files_map = self.generate_source_files_map();
        let uv_binary_array = byte_array_literal(&self.config.uv_binary);

        LAUNCHER_TEMPLATE
            .replace("{source_files_map}", &source_files_map)
            .replace("{uv_binary_array}", &uv_binary_array)
    }

    fn write_and_compile_source(&self, source: &str) -> io::Result<()> {
        let generated_source_path = "launcher_generated.rs";
        fs::write(generated_source_path, source)?;

        let mut sp = Spinner::new(Spinners::Dots9, "Compiling launcher binary ...".into());
        // Add required crates for compilation
        let status = Command::new("rustc")
            .arg(generated_source_path)
            .arg("--edition=2021")
            .arg("--extern")
            .arg(format!("once_cell={}", find_rlib("once_cell")?))
            .arg("--extern")
            .arg(format!("maplit={}", find_rlib("maplit")?))
            .arg("-o")
            .arg(&self.config.output_path)
            .status()?;

        if !status.success() {
            eprintln!("Failed to compile the launcher binary.");
            std::process::exit(1);
        } else {
            sp.stop_and_persist("✔", format!("Launcher binary created at: {}", self.config.output_path).into());
        }
        Ok(())
    }
}

fn byte_array_literal(data: &[u8]) -> String {
    data.iter()
        .map(|b| b.to_string())
        .collect::<Vec<String>>()
        .join(", ")
}

fn find_rlib(crate_name: &str) -> io::Result<String> {
    let entries = fs::read_dir("./target/debug/deps")?;
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with(&format!("lib{}", crate_name)) && name_str.ends_with(".rlib") {
            return Ok(entry.path().to_string_lossy().into_owned());
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Could not find .rlib for {}", crate_name),
    ))
}