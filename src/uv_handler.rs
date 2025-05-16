use std::env;
use std::fs::File;
use std::io::copy;
use std::path::PathBuf;
use std::str::FromStr;
use tempfile::tempdir;


#[derive(Debug, Clone)]
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

// Modify your get_architecture function to accept an optional target
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

// Modify your download function to accept target
pub fn download_binary_and_unpack(target: Option<CrossTarget>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let artifact_name = get_architecture(target).ok_or("Unsupported platform")?;
    let base_url = "https://github.com/astral-sh/uv/releases/download/0.7.4";
    let url = format!("{}/{}", base_url, artifact_name);

    let dir = tempdir()?;
    let file_path = dir.path().join(&artifact_name);

    // Download the file
    let response = reqwest::blocking::get(&url)?;
    let mut dest = File::create(&file_path)?;
    let bytes = response.bytes()?;
    let mut content = bytes.as_ref();
    copy(&mut content, &mut dest)?;

    let output_dir = get_output_dir();

    // Check file extension and use appropriate extraction method
    if artifact_name.ends_with(".zip") {
        let file = File::open(&file_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if file.name().contains("uv.exe") {
                let outpath = output_dir.join("uv.exe");
                let mut outfile = File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
                break;
            }
        }
    } else if artifact_name.ends_with(".tar.gz") {
        let file = File::open(&file_path)?;
        let decompressor = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decompressor);
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            if path.to_string_lossy().contains("uv") && !path.to_string_lossy().ends_with("/") {
                let mut outfile = File::create(output_dir.join("uv"))?;
                std::io::copy(&mut entry, &mut outfile)?;
                break;
            }
        }
    } else {
        return Err("Unsupported archive format".into());
    }

    if artifact_name.ends_with(".zip") {
        Ok(output_dir.join("uv.exe"))
    } else {
        Ok(output_dir.join("uv"))
    }
}


