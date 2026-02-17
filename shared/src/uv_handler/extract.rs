use crate::uv_handler::download::Archive;
use flate2::read::GzDecoder;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use zip::ZipArchive;

pub fn extract_uv<'a>(
    archive: &mut Archive<'a>,
    install_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(install_dir)?;

    match archive {
        Archive::Zip(reader) => extract_zip(reader, install_dir),
        Archive::TarGz(response) => extract_targz(*response, install_dir),
    }
}

fn extract_zip(
    reader: &std::io::Cursor<Vec<u8>>,
    install_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // NOTE: The zip crate requires the full archive in memory for random access.
    // True streaming extraction is not possible with this crate.
    // This function extracts only "uv.exe" and ignores the rest.
    let mut zip = ZipArchive::new(reader.clone())?;
    if let Ok(mut file) = zip.by_name("uv.exe") {
        let out_path = install_dir.join("uv.exe");
        let mut out = std::fs::File::create(out_path)?;
        std::io::copy(&mut file, &mut out)?;
    } else {
        return Err(format!(
            "uv.exe not found in zip archive for install dir {}",
            install_dir.display()
        )
        .into());
    }
    Ok(())
}

fn extract_targz(
    response: &mut reqwest::blocking::Response,
    install_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Streaming extraction: only extract the "uv" binary, skip all other entries.
    let decoder = GzDecoder::new(response);
    let mut archive = tar::Archive::new(decoder);
    let mut found = false;
    for entry in archive.entries()? {
        let mut entry = entry?;
        if let Ok(path) = entry.path() {
            if let Some(file_name) = path.file_name() {
                if file_name == "uv" {
                    let out_path = install_dir.join("uv");
                    let mut out = std::fs::File::create(&out_path)?;
                    std::io::copy(&mut entry, &mut out)?;
                    #[cfg(unix)]
                    std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(0o755))?;
                    found = true;
                    break;
                }
            }
        }
    }
    if !found {
        return Err(format!(
            "uv not found in tar.gz archive for install dir {}",
            install_dir.display()
        )
        .into());
    }
    Ok(())
}
