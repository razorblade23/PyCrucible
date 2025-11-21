use crate::debug_println;
use crate::uv_handler::uv_exists;
use std::{fs::File, io, path::Path, path::PathBuf, process::Command};
use {tempfile::tempdir, zip::ZipArchive};

fn is_ci() -> bool {
    std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok()
}

pub fn install_uv_windows(install_path: &PathBuf) -> Result<(), String> {
    if is_ci() {
        debug_println!("CI detected — using fallback UV binary download");
        download_uv_binary_for_windows(install_path);
        return Ok(());
    }

    debug_println!("Attempting UV install using PowerShell script...");

    let script_result = install_uv_via_powershell_script(install_path);

    match script_result {
        Ok(_) => {
            debug_println!("UV installer script completed, checking if binary was created...");
            if uv_exists(install_path).is_some() {
                debug_println!("UV installed successfully via script.");
                return Ok(());
            } else {
                debug_println!(
                    "UV script exited OK but no binary found — falling back to direct download.",
                );
            }
        }
        Err(err) => {
            debug_println!("UV installer script failed: {err}");
            debug_println!("Falling back to direct binary download...");
        }
    }

    download_uv_binary_for_windows(install_path);

    if !uv_exists(install_path).is_some() {
        eprintln!("Failed both installer script AND fallback binary download. Cannot continue.");
        return Err("Failed to install uv via both script and direct download.".to_string());
    }
    Ok(())
}

fn install_uv_via_powershell_script(install_path: &PathBuf) -> Result<(), String> {
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "irm https://astral.sh/uv/install.ps1 | iex",
        ])
        .env("UV_UNMANAGED_INSTALL", install_path)
        .status()
        .map_err(|e| format!("Failed to spawn PowerShell: {e}"))?;

    if !status.success() {
        return Err(format!(
            "PowerShell installer returned exit code {}",
            status
        ));
    }

    Ok(())
}

fn download_uv_binary_for_windows(install_path: &Path) {
    let dir = tempdir();
    match dir {
        Err(e) => panic!(
            "Failed to create temporary directory for uv download: {}",
            e
        ),
        Ok(d) => {
            let uv_temp = d.path().join("uv-windows.zip");
            let url = "https://github.com/astral-sh/uv/releases/download/0.8.5/uv-x86_64-pc-windows-msvc.zip";

            let status = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    &format!(
                        "Invoke-WebRequest '{}' -OutFile '{}'",
                        url,
                        uv_temp.display()
                    ),
                ])
                .status()
                .expect("Failed to run PowerShell for binary download");

            if !status.success() {
                panic!("Direct download of uv.exe failed");
            }
            debug_println!("Downloaded uv archive to {:?}", uv_temp);

            extract_uv_from_zip_archive(&uv_temp, install_path)
                .expect("Failed to extract uv from zip archive");
        }
    };
}

fn extract_uv_from_zip_archive(
    archive_path: &Path,
    install_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    debug_println!("Extracting uv from archive {:?}", archive_path);
    debug_println!("Extracting uv to {:?}", install_path);
    if !install_path.exists() {
        debug_println!("Filepath to cache do not exists, creating ...");
        std::fs::create_dir_all(install_path)?;
        debug_println!("Created install directory {:?}", install_path);
    }

    // Open the archive file
    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    debug_println!("Opened zip archive, contains {} files", archive.len());

    let uv_binary_path = install_path.join("uv.exe");

    // Iterate through files inside the ZIP
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        if file.name().contains("uv.exe") {
            debug_println!("Found uv.exe in archive at {}, extracting...", file.name());
            let mut outfile = File::create(&uv_binary_path)?;
            debug_println!("Created output file at {:?}", uv_binary_path);
            io::copy(&mut file, &mut outfile)?;
            debug_println!("Extracted uv.exe to {:?}", uv_binary_path);
            return Ok(());
        }
    }

    Err("uv.exe not found in archive".into())
}
