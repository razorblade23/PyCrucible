use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write, Seek, SeekFrom, Cursor};
use std::path::{Path, PathBuf};
use zip::{write::FileOptions, ZipWriter};

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

pub fn embed_payload(source_files: &[PathBuf], manifest_path: &Path, output_path: &Path) -> io::Result<()> {
    // First, copy the current executable to the output path
    let current_exe = std::env::current_exe()?;
    fs::copy(&current_exe, output_path)?;

    // Create a memory buffer for the ZIP
    let mut cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut cursor);
    let options = FileOptions::<()>::default();

    let source_dir = manifest_path.parent().unwrap();
    // ...existing code for adding files to ZIP...

    // Finalize ZIP
    zip.finish()?;
    let payload = cursor.into_inner();
    println!("Finalized zip payload");

    println!("Opening in append mode");
    // Open output file in append mode (the copied executable)
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(output_path)?;

    println!("Getting the offset");
    // Get offset where payload will start
    let offset = file.seek(SeekFrom::End(0))?;

    println!("Writing payload");
    // Write payload
    file.write_all(&payload)?;

    println!("Writing footer");
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
