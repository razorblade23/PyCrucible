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
