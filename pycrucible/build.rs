use std::env;
use std::fs;
use std::path::PathBuf;

fn resolve_runner_path(manifest_dir: &PathBuf, target: &str, profile: &str) -> PathBuf {
    let bin_name = if cfg!(windows) {
        "pycrucible_runner.exe"
    } else {
        "pycrucible_runner"
    };

    // First attempt: target/{target}/{profile}/bin
    let primary_path = manifest_dir
        .join("..")
        .join("target")
        .join(target)
        .join(profile)
        .join(bin_name);

    if primary_path.exists() {
        return primary_path;
    }

    // Fallback: target/{profile}/bin (local dev)
    let fallback_path = manifest_dir
        .join("..")
        .join("target")
        .join(profile)
        .join(bin_name);

    if fallback_path.exists() {
        return fallback_path;
    }

    panic!(
        "Runner binary not found at either path:\n  {}\n  {}",
        primary_path.display(),
        fallback_path.display()
    );
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let profile = env::var("PROFILE").unwrap(); // "release" or "debug"

    let target_env = env::var("TARGET").ok();
    let target = target_env.as_deref().unwrap_or("");

    let runner_bin_path = resolve_runner_path(&manifest_dir, target, &profile);

    let runner_path_str = runner_bin_path.to_str().expect("Path not UTF-8");

    let dest_file = out_dir.join("runner_bin.rs");
    fs::write(
        &dest_file,
        format!(
            r#"pub const RUNNER_BIN: &[u8] = include_bytes!(r"{path}");"#,
            path = runner_path_str,
        ),
    )
    .expect("Failed to write runner_bin.rs");

    println!("cargo:rerun-if-changed={}", runner_bin_path.display());
}
