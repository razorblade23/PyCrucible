use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let profile = env::var("PROFILE").unwrap(); // "release" or "debug"

    let target_env = env::var("CARGO_BUILD_TARGET").ok();
    let target = target_env.as_deref().unwrap_or("");

    let bin_name = if cfg!(windows) {
        "pycrucible_runner.exe"
    } else {
        "pycrucible_runner"
    };

    // Determine path to runner binary
    let runner_path = if !target.is_empty() {
        println!("Using target: {}", target);
        manifest_dir
            .join("..")
            .join("target")
            .join(&target)
            .join(&profile)
            .join(bin_name)
    } else {
        // local dev build
        manifest_dir
            .join("..")
            .join("target")
            .join(&profile)
            .join(bin_name)
    };

    if !runner_path.exists() {
        panic!("Please build pycrucible_runner first. Tried path: {}, target: {}", runner_path.display(), target);
    }

    let runner_path_str = runner_path.to_str().expect("Path not UTF-8");

    let dest_file = out_dir.join("runner_bin.rs");
    fs::write(
        &dest_file,
        format!(
            r#"pub const RUNNER_BIN: &[u8] = include_bytes!(r"{path}");"#,
            path = runner_path_str,
        ),
    )
    .expect("Failed to write runner_bin.rs");

    println!("cargo:rerun-if-changed={}", runner_path.display());
}
