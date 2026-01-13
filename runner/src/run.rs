// use shared::uv_handler::find_or_download_uv;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::{self, io};
use shared::uv_handler_v2::find_or_download_uv;
use shared::{debug_println, debuging};

use shared::config::{ProjectConfig, load_project_config};

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
        "No manifest file found in the source directory. \nManifest files can be pyproject.toml, requirements.txt, pylock.toml, setup.py or setup.cfg",
    ))
}

fn apply_env_from_config(config: &ProjectConfig) {
    if let Some(env_config) = &config.env
        && let Some(vars) = &env_config.variables {
            for (k, v) in vars {
                unsafe { std::env::set_var(k, v) }; // Set env variables - not thread safe
            }
        }
}

fn prepare_hooks(config: &ProjectConfig) -> (String, String) {
    // Figure out if there is a hooks section in the config
    // Borrow the hooks if present
    let hooks = config.hooks.as_ref();

    let (pre_hook, post_hook) = hooks
        .map(|h| {
            (
                h.pre_run.clone().unwrap_or_default(),
                h.post_run.clone().unwrap_or_default(),
            )
        })
        .unwrap_or((String::new(), String::new()));

    (pre_hook, post_hook)
}

fn run_uv_command(uv_path: &PathBuf, project_dir: &Path, command: &str, args: &[&str]) -> io::Result<()> {
    debug_println!(
        "[main.run_uv_command] - Running `uv {}` in {:?}",
        command,
        project_dir
    );
    let status = Command::new(uv_path)
        .arg(command)
        .arg("-q")
        .args(args)
        .current_dir(project_dir)
        .status()?;

    if !status.success() {
        return Err(io::Error::other(format!("Command `uv {}` failed", command)));
    }

    Ok(())
}

fn run_uv_with(uv_path: &PathBuf, project_dir: &Path, with_args: &[&str], command: &[&str]) -> io::Result<()> {
    let mut cmd = Command::new(&uv_path);
    cmd.arg("run").arg("-q");

    for w in with_args {
        cmd.arg("--with").arg(w);
    }

    for c in command {
        cmd.arg(c);
    }

    let status = cmd.current_dir(project_dir).status()?;

    if !status.success() {
        return Err(io::Error::other("uv run failed"));
    }

    Ok(())
}

pub fn run_extracted_project(project_dir: &Path, runtime_args: Vec<String>) -> io::Result<()> {
    // Load project configuration and determine entrypoint
    let config = load_project_config(&project_dir.to_path_buf());

    // Enable debug mode if specified in config
    if config.options.debug {
        debuging::set_debug_mode(true);
        debug_println!("[main.run_extracted_project] - Debug mode enabled");
    }

    // Ensure UV is available
    debug_println!("[main.run_extracted_project] - Ensuring UV is available");
    let uv_path = find_or_download_uv(None).ok_or(io::Error::new(
        io::ErrorKind::NotFound,
        "Could not find or download uv binary",
    ))?;

    // Apply environment variables from config (unsafe but we are single-threaded so it should be fine)
    apply_env_from_config(&config);
    debug_println!(
        "[main.run_extracted_project] - Applied environment variables from configuration"
    );

    // Determine entrypoint
    debug_println!("[main.run_extracted_project] - Loaded project configuration");
    let entrypoint = &config.package.entrypoint;
    let entry_point_path = project_dir.join(entrypoint);
    if entrypoint.ends_with(".py") {
        if !entry_point_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Entry point {} not found", entry_point_path.display()),
            ));
        }
    }
    debug_println!(
        "[main.run_extracted_project] - Using entry point: {}",
        entrypoint
    );

    // Grab the hooks from config and unwrap them to a tuple
    debug_println!(
        "[main.run_extracted_project] - Preparing pre and post hooks from configuration"
    );
    let (pre_hook, post_hook) = prepare_hooks(&config);


    // Check if entrypoint ends in .py and handle accordingly
    let mut run_mode: &str = "source";
    if !entrypoint.ends_with(".py") {
        debug_println!("[main.run_extracted_project] - Entry point is a wheel file");
        run_mode = "wheel";
    }

    debug_println!(
        "[main.run_extracted_project] - Determined run mode: {}",
        run_mode
    );

    // Run pre-hook if specified
    debug_println!("[main.run_extracted_project] - Running pre-hook if specified");
    if !pre_hook.is_empty() {
        run_uv_command(&uv_path, project_dir, "run", &[pre_hook.as_str()])?;
    }

    // // Create virtual environment
    // debug_println!("[main.run_extracted_project] - Creating virtual environment");
    // run_uv_command(&uv_path, project_dir, "venv", &[])?;

    match run_mode {
        "source" => {
            debug_println!("[main.run_extracted_project] - Running in source mode");
            // Find manifest file
            let manifest_path = find_manifest_file(project_dir)?;
            debug_println!(
                "[main.run_extracted_project] - Found manifest file at {:?}",
                manifest_path
            );

            debug_println!("[main.run_extracted_project] - Running main application");
            let mut args_vec: Vec<String> = Vec::with_capacity(1 + runtime_args.len());
            args_vec.push(entrypoint.clone());
            args_vec.extend(runtime_args);

            let args_refs: Vec<&str> = args_vec.iter().map(|s| s.as_str()).collect();
            run_uv_command(&uv_path, project_dir, "run", &args_refs)?;

            // // Sincronize the virtual environment with the manifest file
            // debug_println!("[main.run_extracted_project] - Installing dependencies from manifest file");
            
            // run_uv_command(
            //     &uv_path,
            //     project_dir,
            //     "pip",
            //     &["install", "--requirements", manifest_path.to_str().unwrap()],
            // )?;


        },
        "wheel" => {
            debug_println!("[main.run_extracted_project] - Running in wheel mode");
            // Find the .whl file in the project_dir
            let wheel_file = std::fs::read_dir(project_dir)?
                .filter_map(|entry| entry.ok())
                .find_map(|entry| {
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "whl") {
                        Some(path)
                    } else {
                        None
                    }
                })
                .ok_or(io::Error::new(
                    io::ErrorKind::NotFound,
                    "No .whl file found in the project directory",
                ))?;
            debug_println!(
                "[main.run_extracted_project] - Found wheel file at {:?}",
                wheel_file
            );
            run_uv_with(&uv_path, project_dir, &[wheel_file.to_str().unwrap()], &[config.package.entrypoint.as_str()])?;
        },
        _ => unreachable!(),
    }


    // Run post-hook if specified
    debug_println!("[main.run_extracted_project] - Running post-hook if specified");
    if !post_hook.is_empty() {
        run_uv_command(&uv_path, project_dir, "run", &[post_hook.as_str()])?;
    }

    // Clean up if delete_after_run is set or extract_to_temp is set
    debug_println!(
        "[main.run_extracted_project] - Cleaning up extracted project if configured to do so"
    );
    if (config.options.delete_after_run || config.options.extract_to_temp)
        && project_dir.exists() {
            std::fs::remove_dir_all(project_dir)?;
        }

    Ok(())
}
