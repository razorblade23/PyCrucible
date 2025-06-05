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
    #[arg(long, help = "Directory containing Python project to embed. When specified, creates a new binary with the embedded project")]
    pub embed: Option<PathBuf>,

    #[arg(short = 'B', long, help="Path to `uv` executable. If not found, it will be downloaded automatically")]
    #[arg(default_value_os_t = get_output_dir().join(UV_BINARY))]
    pub uv_path: PathBuf,
    
    #[arg(long, help="Target architecture for cross-platform compilation (x86_64-unknown-linux-gnu, x86_64-pc-windows-gnu)")]
    pub target: Option<String>,

    #[arg(long, default_value = "true", help="Extract Python project to a temporary directory when running")]
    pub extract_to_temp: Option<String>,

    #[arg(long, default_value = "false", help="Delete extracted files after running. Note: requires re-downloading dependencies on each run")]
    pub delete_after_run: Option<String>,

    #[arg(short = 'o', long, help="Output path for the new binary when using --embed")]
    pub output: Option<PathBuf>,
}
