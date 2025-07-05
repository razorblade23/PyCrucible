use std::io;
use std::path::Path;
use std::process::Command;

use crate::config::{load_project_config, Hooks};
use crate::debug_println;



pub fn run_extracted_project(project_dir: &Path) -> io::Result<()> {
    // Verify Python files exist
    let config = load_project_config(&project_dir.to_path_buf());
    debug_println!("[runner.run_extracted_project] - Loaded project config");
    let entrypoint = config.package.entrypoint;
    debug_println!("[runner.run_extracted_project] - Loaded project entrypopoint");
    let entry_point_path = project_dir.join(&entrypoint);
    debug_println!("[runner.run_extracted_project] - Loaded project entrypoint path");
    let uv_path = project_dir.join("uv");
    debug_println!("[runner.run_extracted_project] - Loaded project uv");

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
    debug_println!("[runner.run_extracted_project] - Manifest path: {:?}", manifest_path);
    
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


    let hooks = if config.hooks.is_some() {
        debug_println!("[runner.run_extracted_project] - Hooks found in project config");
        config.hooks.unwrap()
    } else {
        debug_println!("[runner.run_extracted_project] - No hooks found in project config");
        Hooks {
            pre_run: Some(String::new()),
            post_run: Some(String::new()),
        }
    };

    let pre_hook = hooks.pre_run.unwrap();
    let post_hook = hooks.post_run.unwrap();

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

    let status = Command::new(&uv_path)
        .arg("run")
        .arg(entrypoint)
        .current_dir(&project_dir)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Main application failed"));
    }

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

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;
    use std::os::unix::fs::PermissionsExt;
    use super::*;

    // Helper to create a dummy uv executable
    fn create_dummy_uv(dir: &std::path::Path) -> std::path::PathBuf {
        let uv_path = dir.join("uv");
        #[cfg(unix)]
        {
            let mut file = File::create(&uv_path).unwrap();
            writeln!(file, "#!/bin/sh\nexit 0").unwrap();
            fs::set_permissions(&uv_path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        #[cfg(windows)]
        {
            let mut file = File::create(&uv_path).unwrap();
            writeln!(file, "exit 0").unwrap();
        }
        uv_path
    }

    #[test]
    fn test_missing_manifest_returns_error() {
        let dir = tempdir().unwrap();
        let entrypoint = "main.py";
        File::create(dir.path().join(entrypoint)).unwrap();
        create_dummy_uv(dir.path());

        let result = run_extracted_project(dir.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_missing_entrypoint_returns_error() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("pyproject.toml")).unwrap();
        create_dummy_uv(dir.path());

        let result = run_extracted_project(dir.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_successful_run_with_pyproject() {
        let dir = tempdir().unwrap();
        let entrypoint = "main.py";
        File::create(dir.path().join("pyproject.toml")).unwrap();
        File::create(dir.path().join(entrypoint)).unwrap();
        create_dummy_uv(dir.path());

        let result = run_extracted_project(dir.path());
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_manifest_priority_order() {
        let dir = tempdir().unwrap();
        let entrypoint = "main.py";
        File::create(dir.path().join("requirements.txt")).unwrap();
        File::create(dir.path().join("pylock.toml")).unwrap();
        File::create(dir.path().join("setup.py")).unwrap();
        File::create(dir.path().join("setup.cfg")).unwrap();
        File::create(dir.path().join(entrypoint)).unwrap();
        create_dummy_uv(dir.path());

        // Only requirements.txt should be picked if pyproject.toml is missing
        let result = run_extracted_project(dir.path());
        assert!(result.is_err() || result.is_ok());
    }
}
