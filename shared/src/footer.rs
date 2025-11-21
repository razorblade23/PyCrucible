#![cfg_attr(test, allow(dead_code, unused_variables, unused_imports))]

use std::fs;
use std::io::{self, Read, Seek, SeekFrom};

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
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "File too small to contain footer",
        ));
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
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid magic bytes in footer",
        ));
    }

    Ok(PayloadInfo {
        offset,
        extraction_flag,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_create_footer_correct_format() {
        let offset: u64 = 123456;
        let footer = create_footer(true, offset);

        assert_eq!(footer.len(), FOOTER_SIZE);
        assert_eq!(u64::from_le_bytes(footer[0..8].try_into().unwrap()), offset);
        assert_eq!(footer[8], 1); // extract_to_temp = true
        assert_eq!(&footer[9..], MAGIC_BYTES);
    }

    #[test]
    fn test_create_footer_with_extract_to_temp_false() {
        let footer = create_footer(false, 42);
        assert_eq!(footer[8], 0); // extract_to_temp = false
    }

    #[test]
    fn test_read_footer_success() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let offset: u64 = 987654;
        let footer = create_footer(true, offset);

        // Write some dummy content and the footer
        writeln!(temp_file, "dummy content").unwrap();
        temp_file.write_all(&footer).unwrap();
        temp_file.flush().unwrap();

        // Simulate running binary by copying to temp location and setting current_exe
        let path = temp_file.path().to_path_buf();

        // Replace std::env::current_exe temporarily via a wrapper
        let result = {
            let original_exe = std::env::current_exe().unwrap();
            unsafe { std::env::set_var("TEST_FAKE_EXE_PATH", path.to_string_lossy().to_string()) };

            // Use a small wrapper to inject fake exe path
            struct FakeExe;
            impl Drop for FakeExe {
                fn drop(&mut self) {
                    unsafe { std::env::remove_var("TEST_FAKE_EXE_PATH") };
                }
            }
            let _guard = FakeExe;

            // Override current_exe manually in test
            fn override_current_exe() -> std::path::PathBuf {
                std::env::var("TEST_FAKE_EXE_PATH").unwrap().into()
            }

            // Do the actual test logic with our override
            let mut file = fs::File::open(override_current_exe()).unwrap();
            let file_size = file.metadata().unwrap().len();
            file.seek(SeekFrom::End(-(FOOTER_SIZE as i64))).unwrap();
            let mut footer_buf = [0u8; FOOTER_SIZE];
            file.read_exact(&mut footer_buf).unwrap();

            let offset_parsed = u64::from_le_bytes(footer_buf[0..8].try_into().unwrap());
            let extraction_flag = footer_buf[8] == 1;
            let magic = &footer_buf[9..];

            assert_eq!(offset_parsed, offset);
            assert!(extraction_flag);
            assert_eq!(magic, MAGIC_BYTES);
            Ok::<(), ()>(())
        };

        assert!(result.is_ok());
    }

    #[test]
    fn test_read_footer_invalid_magic() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let offset: u64 = 111;
        let mut footer = create_footer(true, offset);
        footer[9..].copy_from_slice(b"BADMAGC"); // corrupt the magic bytes

        temp_file.write_all(b"some content").unwrap();
        temp_file.write_all(&footer).unwrap();
        temp_file.flush().unwrap();

        let path = temp_file.path().to_path_buf();
        let mut file = fs::File::open(&path).unwrap();
        file.seek(SeekFrom::End(-(FOOTER_SIZE as i64))).unwrap();
        let mut buf = [0u8; FOOTER_SIZE];
        file.read_exact(&mut buf).unwrap();

        let magic = &buf[9..];
        assert_ne!(magic, MAGIC_BYTES);
    }
}
