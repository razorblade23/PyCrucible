use std::io::{self, Read, Seek, SeekFrom};
use std::fs;

pub const FOOTER_SIZE: usize = 16; // 8 offset + 1 flag + 7 magic
pub const MAGIC_BYTES: &[u8] = b"PYCRUCI"; // 7 bytes

#[derive(Debug)]
pub struct PayloadInfo {
    pub offset: u64,
    pub extraction_flag: bool,
}

pub fn create_footer(extract_to_temp: bool, offset: u64) -> Vec<u8> {
    let mut footer = Vec::with_capacity(FOOTER_SIZE);

    // Add offset first (8 bytes)
    footer.extend_from_slice(&offset.to_le_bytes());

    // Add extraction flag (1 byte)
    footer.push(if extract_to_temp { 1 } else { 0 });

    // Add magic bytes (7 bytes)
    footer.extend_from_slice(MAGIC_BYTES);

    footer
}

pub fn read_footer() -> io::Result<PayloadInfo> {
    let exe_path = std::env::current_exe()?;
    let mut file = fs::File::open(exe_path)?;
    let file_size = file.metadata()?.len();

    if file_size < FOOTER_SIZE as u64 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "File too small to contain footer"));
    }

    // Seek to the start of the footer
    file.seek(SeekFrom::End(-(FOOTER_SIZE as i64)))?;
    let mut footer = [0u8; FOOTER_SIZE];
    file.read_exact(&mut footer)?;

    // Extract fields from footer
    let offset = u64::from_le_bytes(footer[0..8].try_into().unwrap());
    let extraction_flag = footer[8] == 1;
    let magic = &footer[9..];

    if magic != MAGIC_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid magic bytes in footer"));
    }

    Ok(PayloadInfo { offset, extraction_flag })
}
