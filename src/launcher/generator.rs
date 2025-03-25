use spinners::{Spinner, Spinners};
use std::fs;
use std::io::{self, Cursor, Write};
use std::process::Command;
use zip::write::FileOptions;

use super::template::LAUNCHER_TEMPLATE;

pub struct LauncherGenerator {
    config: crate::BuilderConfig,
}

impl LauncherGenerator {
    pub fn new(config: crate::BuilderConfig) -> Self {
        Self { config }
    }

    pub fn generate_and_compile(&self) -> io::Result<()> {
        let mut sp = Spinner::new(
            Spinners::Dots9,
            "Generating launcher source code ...".into(),
        );
        let source = self.generate_source()?;
        sp.stop_and_persist("✔", "Launcher source code generated".into());

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

        Ok(LAUNCHER_TEMPLATE
            .replace("{zip_binary_array}", &zip_array)
            .replace("{uv_binary_array}", &uv_binary_array))
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
zip = { version = "2.5", default-features = false, features = ["deflate"] }

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

        // let mut sp: Spinner = Spinner::new(Spinners::Dots9, "Compiling launcher binary ...".into());
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
            // sp.stop_and_persist(
            //     "✔",
            //     format!("Launcher binary created at: {}", self.config.output_path),
            // );
            println!("Launcher binary created at: {}", self.config.output_path);
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
