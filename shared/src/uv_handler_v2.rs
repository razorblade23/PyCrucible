use std::{path::PathBuf, process::Command, process::Stdio};
use crate::debug_println;
use crate::spinner::{create_spinner_with_message, stop_and_persist_spinner_with_message};

#[cfg(target_os = "windows")]
use {
    tempfile::tempdir,
    zip::ZipArchive,
    std::fs::File,
    std::io::{self, Write},
    std::path::Path,
};

fn uv_exists(path: &PathBuf) -> Option<PathBuf> {
    let candidates = vec![
            path.join("uv"),
            path.join("uv.exe"),
            path.join("bin").join("uv"),
            path.join("bin").join("uv.exe"),
        ];

    let uv_bin = match candidates.iter().find(|p| p.exists()).cloned() {
        Some(p) => p,
        None => {
            eprintln!("uv binary not found.");
            return None;
        }
    };
    Some(uv_bin)
}

#[cfg(target_os = "windows")]
fn is_ci() -> bool {
    std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok()
}

#[cfg(target_os = "windows")]
fn install_uv_windows(install_path: &PathBuf) -> Result<(), String> {
    if is_ci() {
        println!("CI detected — using fallback UV binary download");
        download_uv_binary_for_windows(install_path);
        return Ok(());
    }

    println!("Attempting UV install using PowerShell script...");

    let script_result = install_uv_via_powershell_script(install_path);

    match script_result {
        Ok(_) => {
            println!("UV installer script completed, checking if binary was created...");
            if uv_exists(install_path).is_some() {
                println!("UV installed successfully via script.");
                return Ok(());
            } else {
                println!("UV script exited OK but no binary found — falling back to direct download.");
            }
        }
        Err(err) => {
            println!("UV installer script failed: {err}");
            println!("Falling back to direct binary download...");
        }
    }

    download_uv_binary_for_windows(install_path);

    if !uv_exists(install_path).is_some() {
        eprintln!("Failed both installer script AND fallback binary download. Cannot continue.");
        return Err("Failed to install uv via both script and direct download.".to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn install_uv_via_powershell_script(install_path: &PathBuf) -> Result<(), String> {
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy", "Bypass",
            "-Command",
            "irm https://astral.sh/uv/install.ps1 | iex"
        ])
        .env("UV_UNMANAGED_INSTALL", install_path)
        .status()
        .map_err(|e| format!("Failed to spawn PowerShell: {e}"))?;

    if !status.success() {
        return Err(format!("PowerShell installer returned exit code {}", status));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn download_uv_binary_for_windows(install_path: &Path) {
    let dir = tempdir();
    match dir {
        Err(e) => panic!("Failed to create temporary directory for uv download: {}", e),
        Ok(d) => {
            let uv_temp = d.path().join("uv-windows.zip");
            let url = "https://github.com/astral-sh/uv/releases/download/0.9.11/uv-aarch64-pc-windows-msvc.zip";
        
            let status = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    &format!("Invoke-WebRequest '{}' -OutFile '{}'", url, uv_temp.display()),
                ])
                .status()
                .expect("Failed to run PowerShell for binary download");
        
            if !status.success() {
                panic!("Direct download of uv.exe failed");
            }
            println!("Downloaded uv archive to {:?}", uv_temp);

            extract_uv_from_zip_archive(&uv_temp, install_path).expect("Failed to extract uv from zip archive");

        },
    };
}

#[cfg(target_os = "windows")]
fn extract_uv_from_zip_archive(
    archive_path: &Path,
    install_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Extracting uv from archive {:?}", archive_path);
    println!("Extracting uv to {:?}", install_path);
    
    // Open the archive file
    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    println!("Opened zip archive, contains {} files", archive.len());

    let uv_binary_path = install_path.join("uv.exe");

    // Iterate through files inside the ZIP
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        if file.name().contains("uv.exe") {
            println!("Found uv.exe in archive at {}, extracting...", file.name());
            let mut outfile = File::create(&uv_binary_path)?;
            println!("Created output file at {:?}", uv_binary_path);
            io::copy(&mut file, &mut outfile)?;
            println!("Extracted uv.exe to {:?}", uv_binary_path);
            return Ok(());
        }
    }

    Err("uv.exe not found in archive".into())
}


#[cfg(unix)]
fn install_uv_unix(install_path: &PathBuf) -> Result<(), String> {
    let mut curl = Command::new("curl")
        .args(["-sL", "https://astral.sh/uv/install.sh"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start curl");

    let mut sh = Command::new("sh")
        .env("UV_UNMANAGED_INSTALL", install_path)
        .stdin(Stdio::from(curl.stdout.take().unwrap()))
        .stdout(Stdio::null())
        .spawn()
        .expect("Failed to start shell");

    let curl_status = curl.wait().expect("Failed to wait for curl");
    let sh_status = sh.wait().expect("Failed to wait for sh");

    if !curl_status.success() || !sh_status.success() {
        eprintln!("Installation failed.");
        return Err("Installation of uv failed.".to_string());
    }
    Ok(())
}

pub fn download_and_install_uv_v2(install_path: &PathBuf) {
    #[cfg(unix)]
    {
        let installation_status = install_uv_unix(install_path);
        if installation_status.is_err() {
            eprintln!("uv installation via script failed: {}", installation_status.err().unwrap());
        }
    };
    #[cfg(target_os = "windows")]
    {
        let installation_status = install_uv_windows(install_path);
        if installation_status.is_err() {
            eprintln!("uv installation via script or direct download failed: {}", installation_status.err().unwrap());
        }
    };
}

// pub fn download_and_install_uv(install_path: &PathBuf) {
//     let _status = if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
//         // Download and run the install script via sh if unix-based OS
//         let mut wget = Command::new("wget")
//             .args(["-qO-", "https://astral.sh/uv/install.sh"])
//             .stdout(Stdio::piped())
//             .spawn()
//             .expect("Failed to start wget");

//         let mut sh = Command::new("sh")
//             .env("UV_UNMANAGED_INSTALL", install_path)
//             .stdin(Stdio::from(wget.stdout.take().unwrap()))
//             .stdout(Stdio::null())
//             .spawn()
//             .expect("Failed to start shell");

//         let wget_status = wget.wait().expect("Failed to wait for wget");
//         let sh_status = sh.wait().expect("Failed to wait for sh");

//         if !wget_status.success() || !sh_status.success() {
//             eprintln!("Installation failed.");
//         }
//     } else if cfg!(target_os = "windows") {
//         // Download and run the install script via powershell if windows
//         println!("Downloading and installing uv via PowerShell...");
//         Command::new("powershell")
//             .args([
//                 "-NoProfile",
//                 "-NonInteractive",
//                 "-ExecutionPolicy", "Bypass",
//                 "-Command",
//                 "irm https://astral.sh/uv/install.ps1 | iex"
//             ])
//             .env("UV_UNMANAGED_INSTALL", install_path)
//             .status()
//             .expect("Failed to install uv");

//         // let execute_status = ps_execute.wait().expect("Failed to wait for PowerShell execution");

//         // if !download_status.success() || !execute_status.success() {
//         //     eprintln!("UV installation failed.");
//         // }
//     } else {
//         eprintln!("Unsupported OS for uv installation.");
//     };
// }

pub fn find_or_download_uv(cli_uv_path: Option<PathBuf>) -> Option<PathBuf> {
    debug_println!("[uv_handler.find_or_download_uv] - Looking for uv");

    let exe_dir = std::env::current_exe().expect("Could not find current working directory. Exiting ....").parent().unwrap().to_path_buf();
    debug_println!("[uv_handler.find_or_download_uv] - Current working directory: {:?}", exe_dir);
    let local_uv = if cli_uv_path.is_some() {
        debug_println!("CLI supplied uv path detected, using it");
        let lc_uv = Some(cli_uv_path.unwrap());
        if lc_uv.as_ref().unwrap().exists() {
            debug_println!("CLI supplied uv path exists");
            lc_uv
        } else {
            debug_println!("CLI supplied uv path does not exist");
            None
        }
    } else if let Ok(path) = which::which("uv") {
        debug_println!("`which` returned uv path, using it");
        Some(path)
    } else {
        let local_uv_path = exe_dir.join("uv");
        if local_uv_path.exists() {
            debug_println!("Found uv next to binary, using it");
            Some(local_uv_path)
        } else {
            None
        }
    };
    let uv_path = if local_uv.is_some() {
        debug_println!("[uv_handler.find_or_download_uv] - uv found locally [{:?}], using it", local_uv.as_ref().unwrap().canonicalize());
        local_uv
    } else {
        debug_println!("[uv_handler.find_or_download_uv] - uv not found locally, lets see if we have it cached ...");
        let home = dirs::home_dir().unwrap();

        let uv_install_root = home.join(".pycrucible").join("cache").join("uv");
        
        let uv_bin = uv_exists(&uv_install_root);
        if uv_bin.is_some() {
            debug_println!("[uv_handler.find_or_download_uv] - uv found cached at {:?}, using it", uv_bin.as_ref().unwrap());
            return uv_bin;
        }

        debug_println!("[uv_handler.find_or_download_uv] - uv binary not found locally, proceeding to download.");
        let sp = create_spinner_with_message("Downloading `uv` ...");
        download_and_install_uv_v2(&uv_install_root);
        stop_and_persist_spinner_with_message(sp, "Downloaded `uv` successfully");

        let uv_bin = uv_exists(&uv_install_root);        if uv_bin.is_some() {
            debug_println!("[uv_handler.find_or_download_uv] - uv downloaded and found at {:?}, using it", uv_bin.as_ref().unwrap());
            return uv_bin;
        }

        return Some(uv_bin.expect("uv binary should exist after download"));
    };

    #[cfg(unix)]
    {
        use std::{fs, os::unix::fs::PermissionsExt};

        if let Some(ref path) = uv_path {
            if path.exists() {
                let mut perms = fs::metadata(path)
                    .expect("Could not stat uv binary")
                    .permissions();
                let current_mode = perms.mode() & 0o777;
                if current_mode == 0o755 {
                    println!("[uv_handler.find_or_download_uv] - uv permissions already 0o755, skipping chmod for {:?}", path);
                    return uv_path.clone();
                }

                perms.set_mode(0o755);
                fs::set_permissions(path, perms)
                    .expect("Could not chmod uv binary");
                println!("[uv_handler.find_or_download_uv] - Set executable permissions for uv at {:?}", path);
            } else {
                eprintln!("uv binary not found at {:?}", path);
            }
        }
    }
    uv_path
}
    