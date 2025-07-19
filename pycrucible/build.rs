use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Target triple, e.g., aarch64-apple-darwin
    let target = env::var("TARGET").expect("Missing TARGET env var");
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("Missing CARGO_MANIFEST_DIR"));

    // Determine correct filename based on OS
    let bin_name = if target.contains("windows") {
        "pycrucible_runner.exe"
    } else {
        "pycrucible_runner"
    };

    // Construct absolute path to runner binary
    let runner_path = manifest_dir
        .join("..")               // into workspace root
        .join("target")
        .join(&target)
        .join("release")
        .join(bin_name);

    // Ensure it exists before continuing
    if !runner_path.exists() {
        panic!(
            "[build.rs] Expected runner binary not found at: {}.\nMake sure 'pycrucible_runner' is built first.",
            runner_path.display()
        );
    }

    // Escape the full path for `include_bytes!` (we use raw string syntax)
    let runner_path_str = runner_path.to_str().expect("Path contains invalid UTF-8");

    // Write the generated file into OUT_DIR
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("Missing OUT_DIR env var"));
    let dest = out_dir.join("runner_bin.rs");

    fs::write(
        &dest,
        format!(
            r#"pub const RUNNER_BIN: &[u8] = include_bytes!(r"{runner_path}");"#,
            runner_path = runner_path_str
        ),
    ).expect("Failed to write runner_bin.rs");

    // Tell Cargo to re-run this script if the runner binary changes
    println!("cargo:rerun-if-changed={}", runner_path_str);
}
