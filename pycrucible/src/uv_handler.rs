#![cfg_attr(test, allow(dead_code, unused_variables, unused_imports))]

use std::env;
use std::fs::File;
use std::io::copy;
use std::path::PathBuf;
use std::str::FromStr;
use tempfile::tempdir;
use shared::debug_println;
use shared::spinner::{create_spinner_with_message, stop_and_persist_spinner_with_message};

use std::fs;
use std::io::{self, Cursor};
use zip::{write::FileOptions, ZipWriter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossTarget {
    LinuxX86_64,
    WindowsX86_64,
}

impl FromStr for CrossTarget {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "x86_64-unknown-linux-gnu" => Ok(CrossTarget::LinuxX86_64),
            "x86_64-pc-windows-gnu" => Ok(CrossTarget::WindowsX86_64),
            _ => Err(format!("Unsupported target: {}", s)),
        }
    }
}

impl CrossTarget {
    fn to_uv_artifact_name(&self) -> &'static str {
        match self {
            CrossTarget::LinuxX86_64 => "uv-x86_64-unknown-linux-gnu.tar.gz",
            CrossTarget::WindowsX86_64 => "uv-x86_64-pc-windows-msvc.zip",
        }
    }
}

fn get_architecture(target: Option<CrossTarget>) -> Option<String> {
    match target {
        Some(target) => Some(target.to_uv_artifact_name().to_string()),
        None => {
            let triple = match () {
                #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
                () => "uv-x86_64-unknown-linux-gnu.tar.gz",

                #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
                () => "uv-x86_64-apple-darwin.tar.gz",

                #[cfg(all(target_arch = "x86_64", target_os = "windows"))]
                () => "uv-x86_64-pc-windows-msvc.zip",

                #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
                () => "uv-aarch64-unknown-linux-gnu.tar.gz",

                #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
                () => "uv-aarch64-apple-darwin.tar.gz",

                #[cfg(all(target_arch = "aarch64", target_os = "windows"))]
                () => "uv-aarch64-pc-windows-msvc.zip",

                #[cfg(not(any(
                    all(target_arch = "x86_64", target_os = "linux"),
                    all(target_arch = "x86_64", target_os = "macos"),
                    all(target_arch = "x86_64", target_os = "windows"),
                    all(target_arch = "aarch64", target_os = "linux"),
                    all(target_arch = "aarch64", target_os = "macos"),
                    all(target_arch = "aarch64", target_os = "windows")
                )))]
                () => return None,
            };

            Some(triple.to_string())
        }
    }
}

fn get_output_dir() -> PathBuf {
    let exe_path = env::current_exe().expect("Failed to get current exe path");
    exe_path.parent().unwrap().to_path_buf()
}

pub fn download_binary_and_unpack(target: Option<CrossTarget>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let sp = create_spinner_with_message("Downloading `uv`");

    let artifact_name = get_architecture(target).ok_or("Unsupported platform")?;
    let base_url = "https://github.com/astral-sh/uv/releases/download/0.7.4";
    let url = format!("{}/{}", base_url, artifact_name);

    let dir = tempdir()?;
    let file_path = dir.path().join(&artifact_name);

    let response = reqwest::blocking::get(&url)?;
    if !response.status().is_success() {
        return Err(format!("Failed to download UV: {}", response.status()).into());
    }

    let mut dest = File::create(&file_path)?;
    let bytes = response.bytes()?;
    let mut content = bytes.as_ref();
    copy(&mut content, &mut dest)?;

    let output_dir = get_output_dir();
    std::fs::create_dir_all(&output_dir)?;

    let uv_binary_path = if artifact_name.ends_with(".zip") {
        output_dir.join("uv.exe")
    } else {
        output_dir.join("uv")
    };

    // Remove existing binary if it exists
    if uv_binary_path.exists() {
        std::fs::remove_file(&uv_binary_path)?;
    }

    // Extract the binary
    if artifact_name.ends_with(".zip") {
        let file = File::open(&file_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if file.name().contains("uv.exe") {
                let mut outfile = File::create(&uv_binary_path)?;
                std::io::copy(&mut file, &mut outfile)?;
                break;
            }
        }
    } else if artifact_name.ends_with(".tar.gz") {
        let file = File::open(&file_path)?;
        let decompressor = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decompressor);

        let mut found = false;
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            let path_str = path.to_string_lossy();
            
            // Look for the actual binary, usually named just 'uv'
            if path_str.ends_with("/uv") || path_str == "uv" {
                let mut outfile = File::create(&uv_binary_path)?;
                std::io::copy(&mut entry, &mut outfile)?;
                found = true;
                break;
            }
        }

        if !found {
            return Err("Could not find UV binary in archive".into());
        }
        
    } else {
        return Err("Unsupported archive format".into());
    }

    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&uv_binary_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&uv_binary_path, perms)?;
    }

    // Verify the binary works
    let output = std::process::Command::new(&uv_binary_path)
        .arg("--version")
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "UV binary verification failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ).into());
    }
    stop_and_persist_spinner_with_message(sp, "Downloaded `uv` successfully");
    Ok(uv_binary_path)
}


pub fn find_or_download_uv(uv_path: PathBuf, zip: &mut ZipWriter<&mut Cursor<Vec<u8>>>, options: FileOptions<'_, ()>) -> Result<(), io::Error> {
    debug_println!("[payload.embed_payload] - Looking for uv");
    let exe_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let local_uv = if uv_path.exists() {
        debug_println!("[payload.embed_payload] - uv found at specified path, using it");
        uv_path
    } else {
        // Try to find uv in system PATH
        if let Some(path) = which::which("uv").ok() {
            debug_println!("[payload.embed_payload] - uv found in system PATH at {:?}", path);
            path
        } else {
            debug_println!("[payload.embed_payload] - uv not found in system PATH, looking for local uv");
            exe_dir.join("uv")
        }
    };
    let uv_path = if local_uv.exists() {
        debug_println!("[payload.embed_payload] - uv found locally, using it");
        local_uv
    } else {
        // Download `uv` and copy it to zip
        debug_println!("[payload.embed_payload] - uv not found locally, downloading ...");
        let target: Option<CrossTarget> = None; // We're running locally
        download_binary_and_unpack(target)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&uv_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&uv_path, perms)?;
        debug_println!("[payload.embed_payload] - Set permissions for uv on linux");
    }
    zip.start_file("uv", options)?;
    let mut uv_file = fs::File::open(&uv_path)?;
    io::copy(&mut uv_file, zip)?;
    debug_println!("[payload.embed_payload] - Added uv to zip");
    Ok(())
}
