use std::{path::PathBuf, process::Command, process::Stdio};

#[cfg(unix)]
pub fn install_uv_unix(install_path: &PathBuf) -> Result<(), String> {
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