pub const LAUNCHER_TEMPLATE: &str = r#"
use std::fs::{self, File};
use std::io::{self, Write, Cursor};
use std::env;
use std::path::PathBuf;
use std::process::Command;
use zip::ZipArchive;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "embedded"]
struct AppFiles;

lazy_static::lazy_static! {
    static ref UV_BINARY: Vec<u8> = AppFiles::get("uv").expect("UV binary not found").data.into_owned();
    static ref PAYLOAD_ZIP: Vec<u8> = AppFiles::get("payload.zip").expect("Payload zip not found").data.into_owned();
}

fn extract_files(base_dir: &PathBuf) -> std::io::Result<()> {
    let reader = Cursor::new(PAYLOAD_ZIP.as_slice());
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
    let payload_dir = if {extract_to_temp} {
        let dir = env::temp_dir().join("python_app_payload");
        fs::create_dir_all(&dir)?;
        println!("[-]: Created temporary directory");
        dir
    } else {
        let exe_path = env::current_exe()?;
        let dir = exe_path.parent().unwrap().join("payload");
        fs::create_dir_all(&dir)?;
        println!("[-]: Created payload directory next to executable");
        dir
    };



    // Extract all files maintaining directory structure
    extract_files(&payload_dir)?;
    println!("[-]: Extracted all source files");

    // Setup UV binary
    let uv_path = payload_dir.join("uv");
    File::create(&uv_path)?.write_all(UV_BINARY.as_slice())?;

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
        .current_dir(&payload_dir)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "uv sync failed"));
    }
    println!("[-]: Synced virtual environment");

    // Run the main Python file
    let status = Command::new(&uv_path)
        .arg("run")
        .arg("{entrypoint}")
        .current_dir(&payload_dir)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Python application failed"));
    }

    Ok(())
}"#;
