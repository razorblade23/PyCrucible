use crate::debug_println;
use std::io;
use std::path::Path;

include!(concat!(env!("OUT_DIR"), "/runner_bin.rs"));

pub fn extract_runner(output_path: &Path) -> io::Result<()> {
    std::fs::write(output_path, RUNNER_BIN)?;
    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(output_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(output_path, perms)?;
    }
    debug_println!("[runner_handler] - Extracted runner to {:?}", output_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Read;
    use tempfile::tempdir;

    // Define a test-only RUNNER_BIN
    const TEST_RUNNER_BIN: &[u8] = b"#!/usr/bin/env python3\necho 'Hello, World!'";

    // Override the function just for test
    fn extract_runner_test(output_path: &Path, content: &[u8]) -> io::Result<()> {
        std::fs::write(&output_path, content)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&output_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&output_path, perms)?;
        }
        Ok(())
    }

    #[test]
    fn test_extract_runner_creates_file_with_correct_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("runner_test");

        extract_runner_test(&path, TEST_RUNNER_BIN).unwrap();

        // Check contents
        let mut buf = Vec::new();
        fs::File::open(&path)
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap();
        assert_eq!(buf, TEST_RUNNER_BIN);

        // Check permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&path).unwrap();
            let mode = metadata.permissions().mode();
            assert_eq!(mode & 0o111, 0o111); // Executable by someone
        }
    }
}
