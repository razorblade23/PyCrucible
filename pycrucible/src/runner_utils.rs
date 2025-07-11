use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::os::unix::fs::PermissionsExt;


const RUNNER_GITHUB_OWNER: &str = "razorblade23";
const RUNNER_GITHUB_REPO: &str = "PyCrucible";
const RUNNER_VERSION: &str = "v0.3.0"; // Change as needed



#[cfg(target_os = "windows")]
const RUNNER_NAME: &str = "pycr_runner.exe";
#[cfg(not(target_os = "windows"))]
const RUNNER_NAME: &str = "pycr_runner";

fn get_executable_dir() -> io::Result<PathBuf> {
    let exe_path = env::current_exe()?;
    Ok(exe_path.parent().unwrap().to_path_buf())
}

fn runner_path() -> io::Result<PathBuf> {
    Ok(get_executable_dir()?.join(RUNNER_NAME))
}

pub fn is_runner_present() -> bool {
    runner_path().map(|p| p.exists()).unwrap_or(false)
}

fn github_runner_url() -> String {
    let asset_name = if cfg!(target_os = "windows") {
        "pycr_runner.exe"
    } else if cfg!(target_os = "macos") {
        "pycr_runner_macos"
    } else {
        "pycr_runner"
    };
    format!(
        "https://github.com/{}/{}/releases/download/{}/{}",
        GITHUB_OWNER, GITHUB_REPO, RUNNER_VERSION, asset_name
    )
}

pub fn download_runner() -> Result<(), Box<dyn std::error::Error>> {
    let url = github_runner_url();
    let dest = runner_path()?;

    let mut resp = reqwest::blocking::get(&url)?;
    if !resp.status().is_success() {
        return Err(format!("Failed to download runner: {}", resp.status()).into());
    }

    let mut out = fs::File::create(&dest)?;
    io::copy(&mut resp, &mut out)?;

    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&dest)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest, perms)?;
    }

    Ok(())
}

pub fn ensure_runner_present() -> Result<(), Box<dyn std::error::Error>> {
    if !is_runner_present() {
        println!("pycr_runner not found, downloading...");
        download_runner()?;
        println!("pycr_runner downloaded.");
    }
    Ok(())
}