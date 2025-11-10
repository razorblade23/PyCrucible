use clap::Parser;
use std::path::PathBuf;
use std::env;

const AUTHOR: &str = "razorblade23";
const ABOUT: &str = "Tool to generate python executable by melding UV and python source code in crucible of one binary";
const UV_BINARY: &str = if cfg!(windows) { "uv.exe" } else { "uv" };

fn get_output_dir() -> PathBuf {
    let exe_path = env::current_exe().expect("Failed to get current exe path");
    exe_path.parent().unwrap().to_path_buf()
}

fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Parser, Debug)]
#[command(author = AUTHOR, version = get_version(), about = ABOUT, long_about = None)]
pub struct Cli {
    #[arg(short = 'e', long, help = "Directory containing Python project to embed.", value_name = "SOURCE_DIR")]
    pub embed: PathBuf,

    #[arg(short = 'o', long, help="Output path for the new binary. If not specified, defaults to `./launcher`.")]
    pub output: Option<PathBuf>,
    
    #[arg(long, help="Path to `uv` executable. If not found, it will be downloaded automatically")]
    #[arg(default_value_os_t = get_output_dir().join(UV_BINARY))]
    pub uv_path: PathBuf,

    #[arg(long, help="Disable embedding `uv` binary into the output executable. This will require `uv` to be present alongside (or downloaded) the output binary at runtime.")]
    pub no_uv_embed: bool,

    #[arg(long, help="Enable debug output")]
    pub debug: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_output_dir_contains_exe() {
        let output_dir = get_output_dir();
        let exe = env::current_exe().unwrap();
        let expected_dir = exe.parent().unwrap();
        assert_eq!(output_dir, expected_dir);
    }

    #[test]
    fn test_get_version_matches_env() {
        let version = get_version();
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
    }
}