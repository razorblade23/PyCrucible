// use shared::uv_handler::find_or_download_uv;
use shared::uv_handler::find_or_download_uv;
use shared::{debug_println, debuging};
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::{self, io};

use shared::config::{ProjectConfig, load_project_config};

#[derive(Debug)]
enum RunMode {
    Source,
    Wheel,
    App,
}

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

    debug_println!(
        "[main.run_extracted_project] - Prepared hooks - pre_hook: {}, post_hook: {}",
        pre_hook,
        post_hook
    );
    (pre_hook, post_hook)
}

fn run_hook(
    hook_name: &str,
    hook_cmd: &str,
    uv_path: &Path,
    project_dir: &Path,
) -> io::Result<()> {
    if hook_cmd.is_empty() {
        return Ok(());
    }

    debug_println!("[main.run_extracted_project] - Running {}", hook_name);

    let hook_path = project_dir.join(hook_cmd);

    let path_str = hook_path.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid {} path", hook_name),
        )
    })?;

    run_uv(uv_path, project_dir, &[], &[path_str])
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

/// Builds the argument list passed to `uv run ... -- <args>` for a given run mode.
///
/// This is the fix for #61. Previously only `RunMode::Source` forwarded
/// `runtime_args` - `RunMode::Wheel` and `RunMode::App` entrypoints (console
/// scripts / bare commands) silently discarded any arguments the user passed
/// to the built executable at runtime.
///
/// Source mode resolves to `entry_point_path` (the full path to the .py file
/// on disk, so uv can find it regardless of cwd). Wheel and App mode resolve
/// to the bare `entrypoint` string instead (e.g. "gunicorn"), since that's a
/// console-script/command name, not a real file under the project dir.
fn build_run_args(
    run_mode: &RunMode,
    entrypoint: &str,
    entry_point_path: &Path,
    runtime_args: &[String],
) -> io::Result<Vec<String>> {
    let mut args: Vec<String> = Vec::with_capacity(1 + runtime_args.len());

    match run_mode {
        RunMode::Source => {
            // Use entry_point_path rather than entrypoint to account for indirect project location reference
            let project_entry_point = entry_point_path.to_str().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Could not extract entry point path",
                )
            })?;
            args.push(project_entry_point.to_string());
        }
        RunMode::Wheel | RunMode::App => {
            args.push(entrypoint.to_string());
        }
    }

    args.extend(runtime_args.iter().cloned());

    Ok(args)
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
    let run_mode: RunMode;
    let entrypoint = &config.package.entrypoint;
    let entry_point_path = project_dir.join(entrypoint);

    // Check if the entrypoint path exists in the project directory
    if !entry_point_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Entry point {} not found", entry_point_path.display()),
        ));
    }

    debug_println!(
        "[main.run_extracted_project] - Using entry point: {}",
        entrypoint
    );

    // Determine run mode based on entrypoint extension
    if entrypoint.ends_with(".py") {
        run_mode = RunMode::Source;
    } else if entrypoint.ends_with(".whl") {
        run_mode = RunMode::Wheel;
    } else {
        run_mode = RunMode::App;
    }

    debug_println!(
        "[main.run_extracted_project] - Determined run mode: {:#?}",
        run_mode
    );

    // Grab the hooks from config and unwrap them to a tuple
    let (pre_hook, post_hook) = prepare_hooks(&config);

    // Run pre-hook
    run_hook("pre-hook", &pre_hook, &uv_path, project_dir)?;

    debug_println!("[main.run_extracted_project] - Running main project");

    let args = build_run_args(&run_mode, entrypoint, &entry_point_path, &runtime_args)?;
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    match run_mode {
        RunMode::Source => {
            debug_println!("[main.run_extracted_project] - Running in source mode");
            run_uv(&uv_path, project_dir, &[], &args_refs)?;
        }
        RunMode::Wheel => {
            debug_println!("[main.run_extracted_project] - Running in wheel mode");
            let wheel = find_single_wheel(project_dir)?;
            let wheel_file = wheel.ok_or(io::Error::new(
                io::ErrorKind::NotFound,
                "No .whl file found in the project directory",
            ))?;
            run_uv(
                &uv_path,
                project_dir,
                &[wheel_file.to_str().unwrap()],
                &args_refs,
            )?;
        }
        RunMode::App => {
            debug_println!("[main.run_extracted_project] - Running in app mode");
            run_uv(&uv_path, project_dir, &[], &args_refs)?;
        }
    }

    // Run post-hook
    run_hook("post-hook", &post_hook, &uv_path, project_dir)?;

    // Clean up if delete_after_run is set or extract_to_temp is set
    if (config.options.delete_after_run || config.options.extract_to_temp) && project_dir.exists()
    {
        debug_println!("[main.run_extracted_project] - Cleaning up extracted project");
        std::fs::remove_dir_all(project_dir)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // ---- build_run_args ----------------------------------------------------
    //
    // These pin down the fix for #61: Wheel and App mode must forward
    // runtime_args after the entrypoint, exactly like Source mode already did.

    #[test]
    fn source_mode_uses_entry_point_path_and_forwards_runtime_args() {
        let entry_point_path = Path::new("/tmp/project/main.py");
        let runtime_args = vec!["--flag".to_string(), "value".to_string()];

        let args =
            build_run_args(&RunMode::Source, "main.py", entry_point_path, &runtime_args).unwrap();

        assert_eq!(
            args,
            vec![
                "/tmp/project/main.py".to_string(),
                "--flag".to_string(),
                "value".to_string(),
            ]
        );
    }

    #[test]
    fn source_mode_with_no_runtime_args_is_just_entry_point() {
        let entry_point_path = Path::new("/tmp/project/main.py");
        let args = build_run_args(&RunMode::Source, "main.py", entry_point_path, &[]).unwrap();

        assert_eq!(args, vec!["/tmp/project/main.py".to_string()]);
    }

    #[test]
    fn wheel_mode_forwards_runtime_args_after_entrypoint() {
        // Regression test for #61: a wheel-mode console script (e.g. "gunicorn")
        // used to silently drop any runtime-provided arguments.
        let entry_point_path = Path::new("/tmp/project/gunicorn");
        let runtime_args = vec!["--workers".to_string(), "4".to_string()];

        let args =
            build_run_args(&RunMode::Wheel, "gunicorn", entry_point_path, &runtime_args).unwrap();

        assert_eq!(
            args,
            vec![
                "gunicorn".to_string(),
                "--workers".to_string(),
                "4".to_string(),
            ]
        );
    }

    #[test]
    fn wheel_mode_with_no_runtime_args_is_just_entrypoint() {
        let entry_point_path = Path::new("/tmp/project/gunicorn");
        let args = build_run_args(&RunMode::Wheel, "gunicorn", entry_point_path, &[]).unwrap();

        assert_eq!(args, vec!["gunicorn".to_string()]);
    }

    #[test]
    fn app_mode_forwards_runtime_args_after_entrypoint() {
        // Regression test for #61: same bug as wheel mode, for a bare command
        // entrypoint that isn't a .py or .whl file.
        let entry_point_path = Path::new("/tmp/project/mycommand");
        let runtime_args = vec![
            "serve".to_string(),
            "--port".to_string(),
            "8080".to_string(),
        ];

        let args =
            build_run_args(&RunMode::App, "mycommand", entry_point_path, &runtime_args).unwrap();

        assert_eq!(
            args,
            vec![
                "mycommand".to_string(),
                "serve".to_string(),
                "--port".to_string(),
                "8080".to_string(),
            ]
        );
    }

    #[test]
    fn app_mode_with_no_runtime_args_is_just_entrypoint() {
        let entry_point_path = Path::new("/tmp/project/mycommand");
        let args = build_run_args(&RunMode::App, "mycommand", entry_point_path, &[]).unwrap();

        assert_eq!(args, vec!["mycommand".to_string()]);
    }

    #[test]
    fn runtime_args_preserve_order_and_allow_duplicates() {
        let entry_point_path = Path::new("/tmp/project/mycommand");
        let runtime_args = vec![
            "-v".to_string(),
            "-v".to_string(),
            "--name".to_string(),
            "foo bar".to_string(),
        ];

        let args =
            build_run_args(&RunMode::App, "mycommand", entry_point_path, &runtime_args).unwrap();

        assert_eq!(
            args,
            vec![
                "mycommand".to_string(),
                "-v".to_string(),
                "-v".to_string(),
                "--name".to_string(),
                "foo bar".to_string(),
            ]
        );
    }

    #[test]
    fn wheel_and_app_modes_use_bare_entrypoint_not_full_path() {
        // Wheel/App entrypoints are console-script style commands (e.g.
        // "gunicorn"), not real files under the project dir, so they must NOT
        // be resolved through entry_point_path the way Source mode's .py
        // script is.
        let entry_point_path = Path::new("/some/unrelated/extracted/dir/gunicorn");

        let wheel_args = build_run_args(&RunMode::Wheel, "gunicorn", entry_point_path, &[]).unwrap();
        let app_args = build_run_args(&RunMode::App, "gunicorn", entry_point_path, &[]).unwrap();

        assert_eq!(wheel_args, vec!["gunicorn".to_string()]);
        assert_eq!(app_args, vec!["gunicorn".to_string()]);
    }

    // ---- find_single_wheel --------------------------------------------------

    #[test]
    fn find_single_wheel_returns_none_when_no_wheel_present() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("readme.txt"), b"hi").unwrap();

        let result = find_single_wheel(dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn find_single_wheel_finds_the_only_wheel() {
        let dir = tempdir().unwrap();
        let wheel_path = dir.path().join("mypkg-1.0-py3-none-any.whl");
        fs::write(&wheel_path, b"fake wheel").unwrap();

        let result = find_single_wheel(dir.path()).unwrap();
        assert_eq!(result, Some(wheel_path));
    }

    #[test]
    fn find_single_wheel_errors_on_multiple_wheels() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a-1.0-py3-none-any.whl"), b"a").unwrap();
        fs::write(dir.path().join("b-1.0-py3-none-any.whl"), b"b").unwrap();

        let result = find_single_wheel(dir.path());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }

    // ---- prepare_hooks --------------------------------------------------

    #[test]
    fn prepare_hooks_returns_empty_strings_when_no_hooks_section() {
        let config = ProjectConfig::default();
        let (pre, post) = prepare_hooks(&config);
        assert_eq!(pre, "");
        assert_eq!(post, "");
    }

    #[test]
    fn prepare_hooks_returns_both_when_both_present() {
        let mut config = ProjectConfig::default();
        config.hooks = Some(shared::config::Hooks {
            pre_run: Some("pre.py".to_string()),
            post_run: Some("post.py".to_string()),
        });

        let (pre, post) = prepare_hooks(&config);
        assert_eq!(pre, "pre.py");
        assert_eq!(post, "post.py");
    }

    #[test]
    fn prepare_hooks_defaults_missing_side_to_empty_string() {
        let mut config = ProjectConfig::default();
        config.hooks = Some(shared::config::Hooks {
            pre_run: Some("pre.py".to_string()),
            post_run: None,
        });

        let (pre, post) = prepare_hooks(&config);
        assert_eq!(pre, "pre.py");
        assert_eq!(post, "");
    }
}