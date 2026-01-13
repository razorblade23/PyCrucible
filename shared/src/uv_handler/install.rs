use crate::debug_println;
use crate::uv_handler::download;
use crate::uv_handler::extract;
use crate::uv_handler::platform;
use crate::{create_spinner_with_message, stop_and_persist_spinner_with_message};
use std::path::Path;
use std::path::PathBuf;

pub fn install_uv(version: &str, install_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let target = platform::target_triple();
    let url = download::build_release_url(version, &target);

    let mut download_result = download::download(&url)?;
    match download_result {
        download::DownloadResult::Zip(ref reader) => {
            let mut archive = download::Archive::Zip(reader.clone());
            extract::extract_uv(&mut archive, install_dir)?;
        }
        download::DownloadResult::TarGz(ref mut response) => {
            let mut archive = download::Archive::TarGz(response);
            extract::extract_uv(&mut archive, install_dir)?;
        }
    }
    Ok(())
}

pub fn uv_exists(path: &PathBuf) -> Option<PathBuf> {
    let candidates = [
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

pub fn find_or_download_uv(cli_uv_path: Option<PathBuf>, uv_version: &str) -> Option<PathBuf> {
    debug_println!("[uv_handler.find_or_download_uv] - Looking for uv");

    let exe_dir = std::env::current_exe()
        .expect("Could not find current working directory. Exiting ....")
        .parent()
        .unwrap()
        .to_path_buf();
    debug_println!(
        "[uv_handler.find_or_download_uv] - Current working directory: {:?}",
        exe_dir
    );
    // Check CLI supplied path first
    let local_uv = if cli_uv_path.is_some() {
        debug_println!("CLI supplied uv path detected, using it");
        let lc_uv = Some(cli_uv_path.as_ref().unwrap().clone());
        if lc_uv.as_ref().unwrap().exists() {
            debug_println!("CLI supplied uv path exists");
            lc_uv
        } else {
            debug_println!("CLI supplied uv path does not exist");
            None
        }
    // Check system path next
    } else if let Ok(path) = which::which("uv") {
        debug_println!("`which` returned uv path, using it");
        Some(path)
    // Check local uv next to binary
    } else {
        let local_uv_path = exe_dir.join("uv");
        if local_uv_path.exists() {
            debug_println!("Found uv next to binary, using it");
            Some(local_uv_path)
        } else {
            None
        }
    };
    // If not found locally, check cache or download
    let uv_path = if local_uv.is_some() {
        debug_println!(
            "[uv_handler.find_or_download_uv] - uv found locally [{:?}], using it",
            local_uv.as_ref().unwrap().canonicalize()
        );
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
        install_uv(uv_version, &uv_install_root).expect("uv installation failed");
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
                if current_mode == 0o755 {
                    debug_println!(
                        "[uv_handler.find_or_download_uv] - uv permissions already 0o755, skipping chmod for {:?}",
                        path
                    );
                    return uv_path.clone();
                }

                perms.set_mode(0o755);
                fs::set_permissions(path, perms).expect("Could not chmod uv binary");
                debug_println!(
                    "[uv_handler.find_or_download_uv] - Set executable permissions for uv at {:?}",
                    path
                );
            } else {
                eprintln!("uv binary not found at {:?}", path);
            }
        }
    }
    uv_path
}
