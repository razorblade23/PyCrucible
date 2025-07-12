use std::io::{self, Read, Seek, SeekFrom};
use std::fs;

pub const FOOTER_SIZE: usize = 16;
pub const MAGIC_BYTES: &[u8] = b"PYCRUCIB";

#[derive(Debug)]
pub struct PayloadInfo {
    pub offset: u64
}

pub fn read_footer() -> io::Result<PayloadInfo> {
    let exe_path = {
        let path = std::env::current_exe()?;
        path
    };

    let mut file = fs::File::open(exe_path)?;
    let file_size = file.metadata()?.len();
    
    // Ensure the file is large enough to contain the expected footer
    if file_size < FOOTER_SIZE as u64 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "File size is smaller than the expected footer size, indicating no footer is present"));
    }
    
    // Read footer
    let seek_position = file_size.checked_sub(FOOTER_SIZE as u64).ok_or(io::Error::new(io::ErrorKind::InvalidData, "Invalid seek position: file size is smaller than footer size"))?;
    file.seek(SeekFrom::Start(seek_position))?;
    let mut footer = [0u8; FOOTER_SIZE];
    file.read_exact(&mut footer)?;

    // Validate magic bytes
    const MAGIC_BYTES_LEN: usize = MAGIC_BYTES.len();
    if &footer[0..MAGIC_BYTES_LEN] != MAGIC_BYTES {
        println!("[read_footer] - footer reads: {:?}", &footer);
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid magic bytes"));
    }

    // Extract offset (8 bytes after magic)
    let offset = footer[MAGIC_BYTES_LEN..MAGIC_BYTES_LEN+8]
        .try_into()
        .map(u64::from_le_bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid footer format: unable to extract offset"))?;
    Ok(PayloadInfo { offset })
    
}