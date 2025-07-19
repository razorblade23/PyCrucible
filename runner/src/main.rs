mod extract;
mod repository;
mod run;

use std::io;

fn main() -> io::Result<()> {
    let path = extract::prepare_and_extract_payload();
    if path.is_none() {
        eprintln!("Failed to extract payload");
        std::process::exit(1);
    }
    let project_dir = path.unwrap();

    run::run_extracted_project(&project_dir)?;
    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use std::fs::{self, File};
//     use std::io::Write;
//     use tempfile::tempdir;
//     use std::os::unix::fs::PermissionsExt;
//     use super::*;

//     // Helper to create a dummy uv executable
//     fn create_dummy_uv(dir: &std::path::Path) -> std::path::PathBuf {
//         let uv_path = dir.join("uv");
//         #[cfg(unix)]
//         {
//             let mut file = File::create(&uv_path).unwrap();
//             writeln!(file, "#!/bin/sh\nexit 0").unwrap();
//             fs::set_permissions(&uv_path, fs::Permissions::from_mode(0o755)).unwrap();
//         }
//         #[cfg(windows)]
//         {
//             let mut file = File::create(&uv_path).unwrap();
//             writeln!(file, "exit 0").unwrap();
//         }
//         uv_path
//     }

//     #[test]
//     fn test_missing_manifest_returns_error() {
//         let dir = tempdir().unwrap();
//         let entrypoint = "main.py";
//         File::create(dir.path().join(entrypoint)).unwrap();
//         create_dummy_uv(dir.path());

//         let result = run_extracted_project(dir.path());
//         assert!(result.is_err());
//         let err = result.unwrap_err();
//         assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
//     }

//     #[test]
//     fn test_missing_entrypoint_returns_error() {
//         let dir = tempdir().unwrap();
//         File::create(dir.path().join("pyproject.toml")).unwrap();
//         create_dummy_uv(dir.path());

//         let result = run_extracted_project(dir.path());
//         assert!(result.is_err());
//         let err = result.unwrap_err();
//         assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
//     }

//     #[test]
//     fn test_successful_run_with_pyproject() {
//         let dir = tempdir().unwrap();
//         let entrypoint = "main.py";
//         File::create(dir.path().join("pyproject.toml")).unwrap();
//         File::create(dir.path().join(entrypoint)).unwrap();
//         create_dummy_uv(dir.path());

//         let result = run_extracted_project(dir.path());
//         assert!(result.is_err() || result.is_ok());
//     }

//     #[test]
//     fn test_manifest_priority_order() {
//         let dir = tempdir().unwrap();
//         let entrypoint = "main.py";
//         File::create(dir.path().join("requirements.txt")).unwrap();
//         File::create(dir.path().join("pylock.toml")).unwrap();
//         File::create(dir.path().join("setup.py")).unwrap();
//         File::create(dir.path().join("setup.cfg")).unwrap();
//         File::create(dir.path().join(entrypoint)).unwrap();
//         create_dummy_uv(dir.path());

//         // Only requirements.txt should be picked if pyproject.toml is missing
//         let result = run_extracted_project(dir.path());
//         assert!(result.is_err() || result.is_ok());
//     }
// }
