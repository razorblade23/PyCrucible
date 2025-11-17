use std::path::Path;
use std::process::Command;
use std::path::PathBuf;
use std::{self, io};

use shared::config::{load_project_config, ProjectConfig};

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

fn apply_env_from_config(config: &ProjectConfig) {
    if let Some(env_config) = &config.env {
        if let Some(vars) = &env_config.variables {
            for (k, v) in vars {
                unsafe { std::env::set_var(k, v) }; // Set env variables - not thread safe
            }
        }
    }
}

fn prepare_hooks (config: &ProjectConfig) -> (String, String) {
    // Figure out if there is a hooks section in the config
    // Borrow the hooks if present
    let hooks = config.hooks.as_ref();

    let (pre_hook, post_hook) = hooks
        .map(|h| (
            h.pre_run.clone().unwrap_or_default(),
            h.post_run.clone().unwrap_or_default(),
        ))
        .unwrap_or((String::new(), String::new()));

    (pre_hook, post_hook)
}

fn run_uv_command(
    project_dir: &Path,
    command: &str,
    args: &[&str],
) -> io::Result<()> {
    let uv_path = project_dir.join("uv");
    let status = Command::new(&uv_path)
        .arg(command)
        .arg("-q")
        .args(args)
        .current_dir(project_dir)
        .status()?;
    
    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Command `uv {}` failed", command),
        ));
    }
    
    Ok(())
}


pub fn run_extracted_project(project_dir: &Path, runtime_args: Vec<String>) -> io::Result<()> {
    // Verify Python files exist
    let config = load_project_config(&project_dir.to_path_buf());
    let entrypoint = &config.package.entrypoint;
    let entry_point_path = project_dir.join(&entrypoint);

    // Find manifest file
    let manifest_path = find_manifest_file(project_dir)?;
    
    if !entry_point_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Entry point {} not found", entry_point_path.display())
        ));
    }

    // Apply environment variables from config (unsafe but we are single-threaded so it should be fine)
    apply_env_from_config(&config);

    // Create virtual environment
    let _venv = run_uv_command(project_dir, "venv", &[])?;

    // Sincronize the virtual environment with the manifest file
    let _pip_sync = run_uv_command(project_dir, "pip", &["install", "--requirements", manifest_path.to_str().unwrap()])?;
    
    // Grab the hooks from config and unwrap them to a tuple
    let (pre_hook, post_hook) = prepare_hooks(&config);
    

    // Run pre-hook if specified
    if !pre_hook.is_empty() {
        let _prehook = run_uv_command(project_dir, "run", &[pre_hook.as_str()])?;
    }

    // Run the main application
    let mut args_vec: Vec<String> = Vec::with_capacity(1 + runtime_args.len());
    args_vec.push(entrypoint.clone());
    args_vec.extend(runtime_args);


    let args_refs: Vec<&str> = args_vec.iter().map(|s| s.as_str()).collect();
    let _main = run_uv_command(project_dir, "run", &args_refs)?;
    

    // Run post-hook if specified
    if !post_hook.is_empty() {
        let _prehook = run_uv_command(project_dir, "run", &[post_hook.as_str()])?;
    }

    // Clean up if delete_after_run is set or extract_to_temp is set
    if config.options.delete_after_run || config.options.extract_to_temp {
        if project_dir.exists() {
            std::fs::remove_dir_all(project_dir)?;
        }
    }

    Ok(())
}