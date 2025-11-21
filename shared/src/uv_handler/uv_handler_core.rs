use crate::debug_println;
use crate::spinner::{create_spinner_with_message, stop_and_persist_spinner_with_message};
use std::path::PathBuf;

pub fn uv_exists(path: &PathBuf) -> Option<PathBuf> {
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

pub fn download_and_install_uv(install_path: &PathBuf) {
    #[cfg(unix)]
    {
        use crate::uv_handler::uv_handler_unix::install_uv_unix;
        let installation_status = install_uv_unix(install_path);
        if installation_status.is_err() {
            eprintln!(
                "uv installation via script failed: {}",
                installation_status.err().unwrap()
            );
        }
    };
    #[cfg(target_os = "windows")]
    {
        use crate::uv_handler::uv_handler_windows::install_uv_windows;
        let installation_status = install_uv_windows(install_path);
        if installation_status.is_err() {
            eprintln!(
                "uv installation via script or direct download failed: {}",
                installation_status.err().unwrap()
            );
        }
    };
}

pub fn find_or_download_uv(cli_uv_path: Option<PathBuf>) -> Option<PathBuf> {
    debug_println!("[uv_handler.find_or_download_uv] - Looking for uv");

    let exe_dir = std::env::current_exe()
        .expect("Could not find current working directory. Exiting ....")
        .parent()
        .unwrap()
        .to_path_buf();
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
        debug_println!(
            "[uv_handler.find_or_download_uv] - uv not found locally, lets see if we have it cached ..."
        );
        let home = dirs::home_dir().unwrap();

        let uv_install_root = home.join(".pycrucible").join("cache").join("uv");

        let uv_bin = uv_exists(&uv_install_root);
        if uv_bin.is_some() {
            debug_println!(
                "[uv_handler.find_or_download_uv] - uv found cached at {:?}, using it",
                uv_bin.as_ref().unwrap()
            );
            return uv_bin;
        }

        debug_println!(
            "[uv_handler.find_or_download_uv] - uv binary not found locally, proceeding to download."
        );
        let sp = create_spinner_with_message("Downloading `uv` ...");
        download_and_install_uv(&uv_install_root);
        stop_and_persist_spinner_with_message(sp, "Downloaded `uv` successfully");

        let uv_bin = uv_exists(&uv_install_root);
        if uv_bin.is_some() {
            debug_println!(
                "[uv_handler.find_or_download_uv] - uv downloaded and found at {:?}, using it",
                uv_bin.as_ref().unwrap()
            );
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
                // Skip chmoding if permission is already set
                if current_mode == 0o755 {
                    return uv_path.clone();
                }

                // Otherwise set permissions to execute the file
                perms.set_mode(0o755);
                fs::set_permissions(path, perms).expect("Could not chmod uv binary");
            } else {
                eprintln!("uv binary not found at {:?}", path);
            }
        }
    }
    uv_path
}
