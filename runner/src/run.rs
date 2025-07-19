use std::path::Path;
use std::process::Command;
use std::path::PathBuf;
use std::io;

use shared::config::load_project_config;

fn find_manifest_file(project_dir: &Path) -> io::Result<PathBuf> {
    let manifest_files = [
        "pyproject.toml",
        "requirements.txt",
        "pylock.toml",
        "setup.py",
        "setup.cfg",
    ];

    for file in &manifest_files {
        let path = project_dir.join(file);
        if path.exists() {
            return Ok(path);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "No manifest file found in the source directory. \nManifest files can be pyproject.toml, requirements.txt, pylock.toml, setup.py or setup.cfg"
    ))
}

pub fn run_extracted_project(project_dir: &Path) -> io::Result<()> {
    // Verify Python files exist
    let config = load_project_config(&project_dir.to_path_buf());
    let entrypoint = config.package.entrypoint;
    let entry_point_path = project_dir.join(&entrypoint);
    let uv_path = project_dir.join("uv");

    // Find manifest file
    let manifest_path = find_manifest_file(project_dir)?;
    
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

    // Clean up if delete_after_run is set or extract_to_temp is true
    if config.options.delete_after_run || config.options.extract_to_temp {
        if project_dir.exists() {
            std::fs::remove_dir_all(project_dir)?;
        }
    }

    Ok(())
}