pub const LAUNCHER_TEMPLATE: &str = r#"
#[macro_use]
extern crate maplit;

use std::fs::{self, File};
use std::io::Write;
use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::collections::HashMap;
use once_cell::sync::Lazy;

static SOURCE_FILES: Lazy<HashMap<&'static str, Vec<u8>>> = 
    Lazy::new(|| {source_files_map});

static UV_BINARY: &[u8] = &[{uv_binary_array}];

fn extract_files(base_dir: &PathBuf) -> std::io::Result<()> {
    for (path, content) in SOURCE_FILES.iter() {
        let full_path = base_dir.join(path);
        
        // Ensure parent directories exist
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let mut file = File::create(&full_path)?;
        file.write_all(content)?;
        println!("[-]: Extracted {}", path);
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    let tmp_dir = env::temp_dir().join("python_app_payload");
    fs::create_dir_all(&tmp_dir)?;
    println!("[-]: Created temporary directory");

    // Extract all files maintaining directory structure
    extract_files(&tmp_dir)?;
    println!("[-]: Extracted all source files");

    // Setup UV binary
    let uv_path = tmp_dir.join("uv");
    File::create(&uv_path)?.write_all(UV_BINARY)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&uv_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&uv_path, perms)?;
    }

    // Run UV sync
    let status = Command::new(&uv_path)
        .arg("sync")
        .current_dir(&tmp_dir)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "uv sync failed"));
    }
    println!("[-]: Synced virtual environment");

    // Run the main Python file
    let status = Command::new(&uv_path)
        .arg("run")
        .arg("main.py")
        .current_dir(&tmp_dir)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Python application failed"));
    }

    Ok(())
}"#;