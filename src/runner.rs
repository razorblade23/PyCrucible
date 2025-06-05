use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use crate::project::load_project_config;
use crate::uv_handler::download_binary_and_unpack;
use crate::uv_handler::CrossTarget;


pub fn run_extracted_project(temp_dir: &Path) -> io::Result<()> {
    // Download UV if needed
    let target: Option<CrossTarget> = None; // We're running locally
    println!("Downloading uv");
    let uv_path = download_binary_and_unpack(target)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // Ensure UV binary has execute permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&uv_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&uv_path, perms)?;
    }

    // Verify Python files exist
    let config = load_project_config(temp_dir);
    let entrypoint = config.package.entrypoint;
    let entry_point_path = temp_dir.join(&entrypoint);
    
    if !entry_point_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Entry point {} not found", entry_point_path.display())
        ));
    }

    println!("Running uv pip sync");
    // Run uv pip sync with proper environment
    let status = Command::new(&uv_path)
        .arg("sync")
        .current_dir(&temp_dir)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "uv sync failed"));
    }

    println!("Running python app");
    let status = Command::new(&uv_path)
        .arg("run")
        .arg(entrypoint)
        .current_dir(&temp_dir)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Main application failed"));
    }

    Ok(())
}
