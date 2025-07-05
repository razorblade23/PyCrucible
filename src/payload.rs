use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write, Seek, SeekFrom, Cursor};
use std::path::{Path, PathBuf};
use zip::{write::FileOptions, ZipWriter};
use crate::config;
use crate::uv_handler::download_binary_and_unpack;
use crate::uv_handler::CrossTarget;
use crate::debug_println;

pub const FOOTER_SIZE: usize = 16;
pub const MAGIC_BYTES: &[u8] = b"PYCR";

#[derive(Debug)]
pub struct PayloadInfo {
    pub offset: u64,
    pub size: u32,
}

pub fn read_footer() -> io::Result<PayloadInfo> {
    let exe_path = std::env::current_exe()?;
    let mut file = fs::File::open(exe_path)?;
    let file_size = file.metadata()?.len();
    
    if file_size < FOOTER_SIZE as u64 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "No footer found"));
    }
    
    // Read footer
    let mut footer = [0u8; FOOTER_SIZE];
    file.seek(SeekFrom::End(-(FOOTER_SIZE as i64)))?;
    file.read_exact(&mut footer)?;

    // Validate magic bytes
    if &footer[0..4] != MAGIC_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid magic bytes"));
    }

    // Extract offset (8 bytes) and size (4 bytes)
    let offset = u64::from_le_bytes(footer[4..12].try_into().unwrap());
    let size = u32::from_le_bytes(footer[12..16].try_into().unwrap());

    Ok(PayloadInfo { offset, size })
}

pub fn embed_payload(source_files: &[PathBuf], manifest_path: &Path, project_config: config::ProjectConfig, uv_path: PathBuf, output_path: &Path) -> io::Result<()> {
    let current_exe = std::env::current_exe()?;
    fs::copy(&current_exe, output_path)?;
    debug_println!("[payload.embed_payload] - Copied itself to output path");

    // Create a memory buffer for the ZIP
    let mut cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut cursor);
    let options = FileOptions::<()>::default();

    // Copy source files and manifest file to .zip
    debug_println!("[payload.embed_payload] - Starting copy of source files to .zip");
    let source_dir = manifest_path.parent().unwrap().canonicalize()?;
    for source_file in source_files {
        let relative_path = source_file.strip_prefix(&source_dir)
            .unwrap_or(source_file.as_path());
        let relative_path = relative_path.to_string_lossy().replace("\\", "/");
        debug_println!("[payload.embed_payload] - Copied {:?} with relative path {:?} to zip", source_file, relative_path);
        let mut file = fs::File::open(source_file)?;
        zip.start_file(relative_path, options)?;
        io::copy(&mut file, &mut zip)?;
    }
    let mut manifest_file = fs::File::open(manifest_path)?;
    let manifest_file_name = manifest_path.file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid manifest file name"))?;
    zip.start_file(manifest_file_name, options)?;
    io::copy(&mut manifest_file, &mut zip)?;
    debug_println!("[payload.embed_payload] - Copied manifest file");

    // Serialize project config to TOML format
    let project_config_toml = toml::to_string(&project_config)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    let mut pycrucible_file = Cursor::new(project_config_toml);
    zip.start_file("pycrucible.toml", options)?;
    io::copy(&mut pycrucible_file, &mut zip)?;
    debug_println!("[payload.embed_payload] - pycrucible.toml copied");
    

    // Look for already downloaded uv to embed next to binary, if not, download it
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

    // Ensure UV binary has execute permissions
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
    io::copy(&mut uv_file, &mut zip)?;
    debug_println!("[payload.embed_payload] - Added uv to zip");

    // Finalize ZIP
    zip.finish()?;
    let payload = cursor.into_inner();
    debug_println!("[payload.embed_payload] - Zip finalized");

    // Open output file in append mode (the copied executable)
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(output_path)?;

    // Get offset where payload will start
    let offset = file.seek(SeekFrom::End(0))?;

    // Write payload
    file.write_all(&payload)?;

    // Create and write footer
    let mut footer = Vec::with_capacity(FOOTER_SIZE);
    footer.extend_from_slice(MAGIC_BYTES);
    footer.extend_from_slice(&offset.to_le_bytes());
    footer.extend_from_slice(&(payload.len() as u32).to_le_bytes());

    file.write_all(&footer)?;

    Ok(())
}

pub fn extract_payload(info: &PayloadInfo, target_dir: &Path) -> io::Result<()> {
    let exe_path = std::env::current_exe()?;
    let mut file = fs::File::open(exe_path)?;
    
    // Read payload
    file.seek(SeekFrom::Start(info.offset))?;
    let mut payload_data = vec![0u8; info.size as usize];
    file.read_exact(&mut payload_data)?;

    // Extract payload
    let reader = std::io::Cursor::new(payload_data);
    let mut archive = zip::ZipArchive::new(reader)?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = target_dir.join(file.name());
        
        if let Some(parent) = outpath.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let mut outfile = fs::File::create(&outpath)?;
        std::io::copy(&mut file, &mut outfile)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&outpath)?.permissions();
            // Set execute permission for UV binary
            if file.name().contains("uv") && !file.name().ends_with("/") {
                perms.set_mode(0o755);
            } else {
                // Set read/write permissions for Python files
                perms.set_mode(0o644);
            }
            fs::set_permissions(&outpath, perms)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io::{Seek, SeekFrom};
    use tempfile::tempdir;
    use super::*;

    // Helper for tests: extract from a specific file, not current_exe
    fn extract_payload_from_file(info: &PayloadInfo, target_dir: &std::path::Path, exe_path: &std::path::Path) -> std::io::Result<()> {
        let mut file = File::open(exe_path)?;

        file.seek(SeekFrom::Start(info.offset))?;
        let mut payload_data = vec![0u8; info.size as usize];
        file.read_exact(&mut payload_data)?;

        let reader = std::io::Cursor::new(payload_data);
        let mut archive = zip::ZipArchive::new(reader)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = target_dir.join(file.name());

            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&outpath)?.permissions();
                if file.name().contains("uv") && !file.name().ends_with("/") {
                    perms.set_mode(0o755);
                } else {
                    perms.set_mode(0o644);
                }
                fs::set_permissions(&outpath, perms)?;
            }
        }

        Ok(())
    }

    #[test]
    fn test_embed_and_extract_payload() {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();
        let file1 = src_dir.join("main.py");
        let file2 = src_dir.join("utils.py");
        fs::write(&file1, b"print('hello')").unwrap();
        fs::write(&file2, b"def foo(): pass").unwrap();

        let manifest = dir.path().join("manifest.toml");
        fs::write(&manifest, b"[project]\nname = 'test'").unwrap();

        let project_config = config::ProjectConfig {
            package: config::PackageConfig {
                entrypoint: "src/main.py".to_string(), 
                ..Default::default()
            },
            ..Default::default()
        };

        let uv_path = dir.path().join("uv");
        fs::write(&uv_path, b"uv-binary").unwrap();

        let output_path = dir.path().join("output_exe");
        let source_files = vec![file1.clone(), file2.clone()];

        let result = embed_payload(
            &source_files,
            &manifest,
            project_config,
            uv_path.clone(),
            &output_path,
        );
        assert!(result.is_ok());
        assert!(output_path.exists());

        let mut file = File::open(&output_path).unwrap();
        file.seek(SeekFrom::End(-(FOOTER_SIZE as i64))).unwrap();
        let mut footer = [0u8; FOOTER_SIZE];
        file.read_exact(&mut footer).unwrap();
        assert_eq!(&footer[0..4], MAGIC_BYTES);

        let offset = u64::from_le_bytes(footer[4..12].try_into().unwrap());
        let size = u32::from_le_bytes(footer[12..16].try_into().unwrap());
        let info = PayloadInfo { offset, size };

        let extract_dir = dir.path().join("extract");
        fs::create_dir(&extract_dir).unwrap();
        let result = extract_payload_from_file(&info, &extract_dir, &output_path);
        assert!(result.is_ok());

        assert!(extract_dir.join("src/main.py").exists());
        assert!(extract_dir.join("src/utils.py").exists());
        assert!(extract_dir.join("manifest.toml").exists());
        assert!(extract_dir.join("pycrucible.toml").exists());
        assert!(extract_dir.join("uv").exists());
    }
}
