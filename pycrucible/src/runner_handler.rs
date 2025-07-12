use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::os::unix::fs::PermissionsExt;
use shared::PYCRUCIBLE_RUNNER_NAME;


const RUNNER_GITHUB_OWNER: &str = "razorblade23";
const RUNNER_GITHUB_REPO: &str = "PyCrucible";
const RUNNER_VERSION: &str = "v0.3.0";


fn get_executable_dir() -> io::Result<PathBuf> {
    let exe_path = env::current_exe()?;
    Ok(exe_path.parent().unwrap().to_path_buf())
}

fn runner_path() -> io::Result<PathBuf> {
    Ok(get_executable_dir()?.join(PYCRUCIBLE_RUNNER_NAME))
}

pub fn is_runner_present() -> bool {
    runner_path().map(|p| p.exists()).unwrap_or(false)
}

fn github_runner_url() -> String {
    format!(
        "https://github.com/{}/{}/releases/download/{}/{}",
        RUNNER_GITHUB_OWNER, RUNNER_GITHUB_REPO, RUNNER_VERSION, PYCRUCIBLE_RUNNER_NAME
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