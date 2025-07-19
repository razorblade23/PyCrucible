use crate::debug_println;
use std::path::Path;
use std::io;

#[cfg(not(rust_analyzer))]
include!(concat!(env!("OUT_DIR"), "/runner_bin.rs"));

// fallback stub for VSCode
#[cfg(rust_analyzer)]
pub const RUNNER_BIN: &[u8] = b"";

pub fn extract_runner(output_path: &Path) -> io::Result<()> {
    std::fs::write(&output_path, RUNNER_BIN)?;
    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&output_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&output_path, perms)?;
    }
    debug_println!("[runner_handler] - Extracted runner to {:?}", output_path);
    Ok(())
}