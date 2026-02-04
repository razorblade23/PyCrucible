// use shared::uv_handler::find_or_download_uv;
use shared::uv_handler::find_or_download_uv;
use shared::{debug_println, debuging};
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::{self, io};

use shared::config::{ProjectConfig, load_project_config};

fn apply_env_from_config(config: &ProjectConfig) {
    if let Some(env_config) = &config.env
        && let Some(vars) = &env_config.variables
    {
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

fn run_uv(uv_path: &Path, project_dir: &Path, with: &[&str], args: &[&str]) -> io::Result<()> {
    let mut cmd = Command::new(uv_path);
    cmd.arg("run").arg("-q");

    for w in with {
        cmd.arg("--with").arg(w);
    }
    cmd.arg("--project");
    cmd.arg(project_dir);

    cmd.args(args);

    let status = cmd.status()?;

    if !status.success() {
        return Err(io::Error::other("uv run failed"));
    }

    Ok(())
}

fn find_single_wheel(project_dir: &Path) -> io::Result<Option<PathBuf>> {
    let mut wheel: Option<PathBuf> = None;

    for entry in std::fs::read_dir(project_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("whl") {
            match &wheel {
                None => wheel = Some(path),
                Some(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Multiple .whl files found in the project directory",
                    ));
                }
            }
        }
    }

    Ok(wheel)
}

pub fn run_extracted_project(project_dir: &Path, runtime_args: Vec<String>) -> io::Result<()> {
    // Load project configuration and determine entrypoint
    let config = load_project_config(&project_dir.to_path_buf());
    debug_println!("[main.run_extracted_project] - Loaded project configuration");

    // Enable debug mode if specified in config
    if config.options.debug {
        debuging::set_debug_mode(true);
        debug_println!("[main.run_extracted_project] - Debug mode enabled");
    }

    // Ensure UV is available
    debug_println!("[main.run_extracted_project] - Ensuring UV is available");
    let uv_path =
        find_or_download_uv(None, config.options.uv_version.as_str()).ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            "Could not find or download uv binary",
        ))?;

    // Apply environment variables from config (unsafe but we are single-threaded so it should be fine)
    apply_env_from_config(&config);
    debug_println!(
        "[main.run_extracted_project] - Applied environment variables from configuration"
    );

    // Determine entrypoint
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

    let wheel = find_single_wheel(project_dir)?;

    let has_wheel = wheel.is_some();

    // Check if entrypoint ends in .py and handle accordingly
    let run_mode = if has_wheel { "wheel" } else { "source" };

    debug_println!(
        "[main.run_extracted_project] - Determined run mode: {}",
        run_mode
    );

    // Run pre-hook if specified
    if !pre_hook.is_empty() {
        debug_println!("[main.run_extracted_project] - Running pre-hook");
        run_uv(&uv_path, project_dir, &[], &[pre_hook.as_str()])?;
    }

    debug_println!("[main.run_extracted_project] - Running main project");
    match run_mode {
        "source" => {
            debug_println!("[main.run_extracted_project] - Running in source mode");
            let mut args_vec: Vec<String> = Vec::with_capacity(1 + runtime_args.len());
            // Use entry_point_path rather than entry point to account for indirect project location reference
            let project_entry_point: String;
            match entry_point_path.to_str(){
            None => return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "Could not extract entry point path",
                        )),
            Some(s) => project_entry_point = String::from(s)
}
            args_vec.push(project_entry_point);
            args_vec.extend(runtime_args);

            let args_refs: Vec<&str> = args_vec.iter().map(|s| s.as_str()).collect();
            run_uv(&uv_path, project_dir, &[], &args_refs)?;
        }
        "wheel" => {
            debug_println!("[main.run_extracted_project] - Running in wheel mode");
            let wheel_file = wheel.ok_or(io::Error::new(
                io::ErrorKind::NotFound,
                "No .whl file found in the project directory",
            ))?;
            run_uv(
                &uv_path,
                project_dir,
                &[wheel_file.to_str().unwrap()],
                &[config.package.entrypoint.as_str()],
            )?;
        }
        _ => unreachable!(),
    }

    // Run post-hook if specified
    if !post_hook.is_empty() {
        debug_println!("[main.run_extracted_project] - Running post-hook");
        run_uv(&uv_path, project_dir, &[], &[post_hook.as_str()])?;
    }

    // Clean up if delete_after_run is set or extract_to_temp is set
    if (config.options.delete_after_run || config.options.extract_to_temp) && project_dir.exists() {
        debug_println!("[main.run_extracted_project] - Cleaning up extracted project");
        std::fs::remove_dir_all(project_dir)?;
    }

    Ok(())
}
