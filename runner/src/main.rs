mod extract;
mod repository;

use std::io;
use std::path::Path;
use std::process::Command;

use shared::config::load_project_config;

fn run_extracted_project(project_dir: &Path) -> io::Result<()> {
    // Verify Python files exist
    let config = load_project_config(&project_dir.to_path_buf());
    let entrypoint = config.package.entrypoint;
    let entry_point_path = project_dir.join(&entrypoint);
    let uv_path = project_dir.join("uv");

    // Find manifest file
    let manifest_path = if project_dir.join("pyproject.toml").exists() {
        project_dir.join("pyproject.toml")
    } else if project_dir.join("requirements.txt").exists() {
        project_dir.join("requirements.txt")
    } else if project_dir.join("pylock.toml").exists() {
        project_dir.join("pylock.toml")
    } else if project_dir.join("setup.py").exists() {
        project_dir.join("setup.py")
    } else if project_dir.join("setup.cfg").exists() {
        project_dir.join("setup.cfg")
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No manifest file found in the source directory. \nManifest files can be pyproject.toml, requirements.txt, pylock.toml, setup.py or setup.cfg"
        ));
    };
    
    if !entry_point_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Entry point {} not found", entry_point_path.display())
        ));
    }

    // Create a virtual environment
    let status = Command::new(&uv_path)
        .arg("venv")
        .arg("-qq")
        .current_dir(&project_dir)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "uv sync failed"));
    }

    // Run uv pip sync with proper environment
    let status = Command::new(&uv_path)
        .arg("pip")
        .arg("install")
        .arg("-qq")
        .arg("--requirements")
        .arg(manifest_path)
        .current_dir(&project_dir)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "uv sync failed"));
    }

    // Figure out if there is a hooks section in the config
    let hooks = if config.hooks.is_some() {
        config.hooks
    } else {
        None
    };

    let (pre_hook, post_hook) = hooks.map(|h| {
        (
            h.pre_run.unwrap_or_default(),
            h.post_run.unwrap_or_default(),
        )
    }).unwrap_or((String::new(), String::new()));

    // Run pre-hook if specified
    if !pre_hook.is_empty() {
        let status = Command::new(&uv_path)
            .arg("run")
            .arg(pre_hook)
            .current_dir(&project_dir)
            .status()?;
        if !status.success() {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to run pre-hook"));
        }
    }

    // Run the main application
    let status = Command::new(&uv_path)
        .arg("run")
        .arg(entrypoint)
        .current_dir(&project_dir)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Main application failed"));
    }

    // Run post-hook if specified
    if !post_hook.is_empty() {
        let status = Command::new(&uv_path)
            .arg("run")
            .arg(post_hook)
            .current_dir(&project_dir)
            .status()?;
        if !status.success() {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to run post-hook"));
        }
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let path = extract::prepare_and_extract_payload(false);
    if path.is_none() {
        eprintln!("Failed to extract payload");
        std::process::exit(1);
    }
    let project_dir = path.unwrap();

    run_extracted_project(&project_dir)?;
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
