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

pub fn embed_payload(source_files: &[PathBuf], manifest_path: &Path, project_config: config::ProjectConfig, output_path: &Path) -> io::Result<()> {
    // Copy the current executable to the output path
    debug_println!("Source files: {:?}", source_files);
    debug_println!("Manifest path: {:?}", manifest_path);
    debug_println!("Project config: {:?}", project_config);
    debug_println!("Output path: {:?}", output_path);

    let current_exe = std::env::current_exe()?;
    fs::copy(&current_exe, output_path)?;
    debug_println!("Copied itself to output path");

    // Create a memory buffer for the ZIP
    let mut cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut cursor);
    let options = FileOptions::<()>::default();

    // Copy source files and manifest file to .zip
    debug_println!("Starting copy of source files to .zip");
    let source_dir = manifest_path.parent().unwrap();
    for source_file in source_files {
        let relative_path = source_file.strip_prefix(source_dir)
            .unwrap_or(source_file.as_path());
        let mut file = fs::File::open(source_file)?;
        zip.start_file(relative_path.to_string_lossy(), options)?;
        io::copy(&mut file, &mut zip)?;
        debug_println!("Copied {:?} with relative path {:?} to zip", source_file, relative_path);
    }
    let mut manifest_file = fs::File::open(manifest_path)?;
    zip.start_file("pyproject.toml", options)?;
    io::copy(&mut manifest_file, &mut zip)?;
    debug_println!("Copied manifest file");

    // Serialize project config to TOML format
    let project_config_toml = toml::to_string(&project_config)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    let mut pycrucible_file = Cursor::new(project_config_toml);
    zip.start_file("pycrucible.toml", options)?;
    io::copy(&mut pycrucible_file, &mut zip)?;
    debug_println!("pycrucible.toml copied");
    

    // Look for already downloaded uv to embed next to binary, if not, download it
    debug_println!("Looking for uv");
    let exe_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let local_uv = exe_dir.join("uv");
    
    let uv_path = if local_uv.exists() {
        debug_println!("uv found locally, using it");
        local_uv
    } else {
        // Download `uv` and copy it to zip
        debug_println!("uv not found locally, downloading ...");
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
        debug_println!("Set permissions for uv on linux");
    }
    zip.start_file("uv", options)?;
    let mut uv_file = fs::File::open(&uv_path)?;
    io::copy(&mut uv_file, &mut zip)?;
    debug_println!("Added uv to zip");

    // Finalize ZIP
    zip.finish()?;
    let payload = cursor.into_inner();
    debug_println!("Zip finalized");

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
