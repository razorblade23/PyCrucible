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
            .spawn()
            .expect("Failed to start shell");

        let wget_status = wget.wait().expect("Failed to wait for wget");
        let sh_status = sh.wait().expect("Failed to wait for sh");

        if !wget_status.success() || !sh_status.success() {
            eprintln!("Installation failed.");
        }
    } else if cfg!(target_os = "windows") {
        // Download and run the install script via powershell if windows

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

        let mut ps_execute = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy", "Bypass",
                "-Command", "-", // Read script from stdin
            ])
            .env("UV_UNMANAGED_INSTALL", install_path)
            .stdin(Stdio::from(ps_download.stdout.take().unwrap()))
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
        debug_println!("[find_or_download_uv.embed_payload] - uv found at specified path, using it");
        Some(cli_uv_path.unwrap())
    } else {
        // Try to find uv in system PATH
        if let Some(path) = which::which("uv").ok() {
            debug_println!("[uv_handler.find_or_download_uv] - uv found in system PATH at {:?}", path);
            Some(path)
        } else {
            debug_println!("[uv_handler.find_or_download_uv] - uv not found in system PATH, looking for local uv");
            Some(exe_dir.join("uv"))
        }
    };
    let uv_path = if local_uv.is_some() {
        debug_println!("[uv_handler.find_or_download_uv] - uv found locally, using it");
        local_uv
    } else {
        debug_println!("[uv_handler.find_or_download_uv] - uv not found locally, downloading ...");
        let home = dirs::home_dir().unwrap();
        let uv_cache = home.join(".pycrucible").join("cache").join("uv");
        let sp = create_spinner_with_message("Downloading `uv` ...");
        download_and_install_uv(&uv_cache);
        stop_and_persist_spinner_with_message(sp, "Downloaded `uv` successfully");
        Some(uv_cache)
    };
    uv_path
}
    