use std::env;
use std::fs::File;
use std::io::copy;
use std::path::PathBuf;
use tempfile::tempdir;

#[cfg(target_arch = "x86_64")]
const ARCH: &str = "x86_64";
#[cfg(target_arch = "aarch64")]
const ARCH: &str = "aarch64";

fn get_architecture() -> Option<String> {
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

fn get_output_dir() -> PathBuf {
    let exe_path = env::current_exe().expect("Failed to get current exe path");
    exe_path.parent().unwrap().to_path_buf()
}

pub fn download_binary_and_unpack() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let artifact_name = get_architecture().ok_or("Unsupported platform")?;
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

    // Split extraction logic by OS
    #[cfg(target_os = "windows")]
    {
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
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
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
    }

    #[cfg(target_os = "windows")]
    Ok(output_dir.join("uv.exe"));
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    Ok(output_dir.join("uv"))
}


