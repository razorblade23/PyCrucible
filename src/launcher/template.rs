pub const LAUNCHER_TEMPLATE: &str = r#"
use std::fs::{self, File};
use std::io::{self, Write, Cursor};
use std::env;
use std::path::PathBuf;
use std::process::Command;
use zip::ZipArchive;

static UV_BINARY: &[u8] = &[{uv_binary_array}];
static PAYLOAD_ZIP: &[u8] = &[{zip_binary_array}];

fn extract_files(base_dir: &PathBuf) -> std::io::Result<()> {
    let reader = Cursor::new(PAYLOAD_ZIP);
    let mut archive = ZipArchive::new(reader)?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = base_dir.join(file.name());
        
        if let Some(parent) = outpath.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let mut outfile = File::create(&outpath)?;
        io::copy(&mut file, &mut outfile)?;
        println!("[-]: Extracted {}", file.name());
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
        .arg(&[{entrypoint}]) // Run entrypoint
        .current_dir(&tmp_dir)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Python application failed"));
    }

    Ok(())
}"#;
