use std::{path::PathBuf, process::Command, process::Stdio};
use crate::debug_println;
use crate::spinner::{create_spinner_with_message, stop_and_persist_spinner_with_message};

pub fn download_and_install_uv(install_path: &PathBuf) {
    let _status = if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        // Download and run the install script via sh if unix-based OS
        let mut wget = Command::new("wget")
            .args(["-qO-", "https://astral.sh/uv/install.sh"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to start wget");

        let mut sh = Command::new("sh")
            .env("UV_UNMANAGED_INSTALL", install_path)
            .stdin(Stdio::from(wget.stdout.take().unwrap()))
            .stdout(Stdio::null())
            .spawn()
            .expect("Failed to start shell");

        let wget_status = wget.wait().expect("Failed to wait for wget");
        let sh_status = sh.wait().expect("Failed to wait for sh");

        if !wget_status.success() || !sh_status.success() {
            eprintln!("Installation failed.");
        }
    } else if cfg!(target_os = "windows") {
        // Download and run the install script via powershell if windows
        println!("Downloading and installing uv via PowerShell...");
        let mut ps_download = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy", "Bypass",
                "-Command", "Invoke-RestMethod https://astral.sh/uv/install.ps1",
            ])
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to start PowerShell script download");
        println!("Download complete, executing installation...");
        let mut ps_execute = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy", "Bypass",
                "-Command", "-", // Read script from stdin
            ])
            .env("UV_UNMANAGED_INSTALL", install_path)
            .stdin(Stdio::from(ps_download.stdout.take().unwrap()))
            .stdout(Stdio::null())
            .spawn()
            .expect("Failed to start PowerShell script execution");

        let download_status = ps_download.wait().expect("Failed to wait for PowerShell download");
        let execute_status = ps_execute.wait().expect("Failed to wait for PowerShell execution");

        if !download_status.success() || !execute_status.success() {
            eprintln!("UV installation failed.");
        }
    } else {
        eprintln!("Unsupported OS for uv installation.");
    };
}

pub fn find_or_download_uv(cli_uv_path: Option<PathBuf>) -> Option<PathBuf> {
    debug_println!("[uv_handler.find_or_download_uv] - Looking for uv");

    let exe_dir = std::env::current_exe().expect("Could not find current working directory. Exiting ....").parent().unwrap().to_path_buf();
    let local_uv = if cli_uv_path.is_some() {
        Some(cli_uv_path.unwrap())
    } else if let Ok(path) = which::which("uv") {
        Some(path)
    } else {
        let local_uv_path = exe_dir.join("uv");
        if local_uv_path.exists() {
            Some(local_uv_path)
        } else {
            None
        }
    };
    let uv_path = if local_uv.is_some() {
        debug_println!("[uv_handler.find_or_download_uv] - uv found locally, using it");
        local_uv
    } else {
        debug_println!("[uv_handler.find_or_download_uv] - uv not found locally, lets see if we have it cached ...");
        let home = dirs::home_dir().unwrap();

        let uv_install_root = home.join(".pycrucible").join("cache").join("uv");
        let candidates = vec![
            uv_install_root.join("uv"),
            uv_install_root.join("uv.exe"),
            uv_install_root.join("bin").join("uv"),
            uv_install_root.join("bin").join("uv.exe"),
        ];

        // Try to find an existing candidate without consuming the vector so we can reuse it after download.
        if let Some(uv_bin) = candidates.iter().find(|p| p.exists()).cloned() {
            debug_println!("[uv_handler.find_or_download_uv] - uv binary found cached at {:?}, no need to download.", uv_bin);
            return Some(uv_bin);
        }

        debug_println!("[uv_handler.find_or_download_uv] - uv binary not found locally, proceeding to download.");
        let sp = create_spinner_with_message("Downloading `uv` ...");
        download_and_install_uv(&uv_install_root);
        stop_and_persist_spinner_with_message(sp, "Downloaded `uv` successfully");

        // Search again after installation
        let uv_bin = match candidates.iter().find(|p| p.exists()).cloned() {
            Some(p) => p,
            None => {
                eprintln!("uv binary not found after installation.");
                return None;
            }
        };

        return Some(uv_bin);
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
    