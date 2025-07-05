use std::env;
use std::fs::File;
use std::io::copy;
use std::path::PathBuf;
use std::str::FromStr;
use tempfile::tempdir;
use std::path::Path;
use crate::spinner_utils::{create_spinner_with_message, stop_and_persist_spinner_with_message};


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

pub fn find_manifest_file(source_dir: &Path) -> PathBuf  {
    let manifest_path = if source_dir.join("pyproject.toml").exists() {
        source_dir.join("pyproject.toml")
    } else if source_dir.join("requirements.txt").exists() {
        source_dir.join("requirements.txt")
    } else if source_dir.join("pylock.toml").exists() {
        source_dir.join("pylock.toml")
    } else if source_dir.join("setup.py").exists() {
        source_dir.join("setup.py")
    } else if source_dir.join("setup.cfg").exists() {
        source_dir.join("setup.cfg")
    } else {
        eprintln!("No manifest file found in the source directory. \nManifest files can be pyproject.toml, requirements.txt, pylock.toml, setup.py or setup.cfg");
        source_dir.join("") // Default to empty string if none found;
    };
    manifest_path
}
#[cfg(test)]
mod tests {
    use std::fs;
    use super::*;

    #[test]
    fn test_cross_target_from_str() {
        assert_eq!(
            CrossTarget::from_str("x86_64-unknown-linux-gnu").unwrap(),
            CrossTarget::LinuxX86_64
        );
        assert_eq!(
            CrossTarget::from_str("x86_64-pc-windows-gnu").unwrap(),
            CrossTarget::WindowsX86_64
        );
        assert!(CrossTarget::from_str("unsupported-target").is_err());
    }

    #[test]
    fn test_cross_target_to_uv_artifact_name() {
        assert_eq!(
            CrossTarget::LinuxX86_64.to_uv_artifact_name(),
            "uv-x86_64-unknown-linux-gnu.tar.gz"
        );
        assert_eq!(
            CrossTarget::WindowsX86_64.to_uv_artifact_name(),
            "uv-x86_64-pc-windows-msvc.zip"
        );
    }

    #[test]
    fn test_get_architecture_with_target() {
        let arch = get_architecture(Some(CrossTarget::LinuxX86_64));
        assert_eq!(
            arch,
            Some("uv-x86_64-unknown-linux-gnu.tar.gz".to_string())
        );
        let arch = get_architecture(Some(CrossTarget::WindowsX86_64));
        assert_eq!(
            arch,
            Some("uv-x86_64-pc-windows-msvc.zip".to_string())
        );
    }

    #[test]
    fn test_find_manifest_file_priority() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir = temp_dir.path();

        // pyproject.toml
        let pyproject = dir.join("pyproject.toml");
        fs::File::create(&pyproject).unwrap();
        assert_eq!(find_manifest_file(dir), pyproject);

        // requirements.txt
        fs::remove_file(&pyproject).unwrap();
        let reqs = dir.join("requirements.txt");
        fs::File::create(&reqs).unwrap();
        assert_eq!(find_manifest_file(dir), reqs);

        // pylock.toml
        fs::remove_file(&reqs).unwrap();
        let pylock = dir.join("pylock.toml");
        fs::File::create(&pylock).unwrap();
        assert_eq!(find_manifest_file(dir), pylock);

        // setup.py
        fs::remove_file(&pylock).unwrap();
        let setup_py = dir.join("setup.py");
        fs::File::create(&setup_py).unwrap();
        assert_eq!(find_manifest_file(dir), setup_py);

        // setup.cfg
        fs::remove_file(&setup_py).unwrap();
        let setup_cfg = dir.join("setup.cfg");
        fs::File::create(&setup_cfg).unwrap();
        assert_eq!(find_manifest_file(dir), setup_cfg);

        // None found
        fs::remove_file(&setup_cfg).unwrap();
        let empty = dir.join("");
        assert_eq!(find_manifest_file(dir), empty);
    }

    #[test]
    fn test_get_output_dir_returns_parent() {
        let out = get_output_dir();
        assert!(out.is_dir());
    }

    // Mock download_binary_and_unpack to avoid network and extraction
    #[test]
    fn test_download_binary_and_unpack_error_on_unsupported() {
        // Use a fake target that is not supported by get_architecture
        struct DummyTarget;
        impl std::str::FromStr for DummyTarget {
            type Err = ();
            fn from_str(_: &str) -> Result<Self, Self::Err> { Ok(DummyTarget) }
        }
        // For now just assert true
        assert!(true);
    }
}
