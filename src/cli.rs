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
    pub source_dir: PathBuf,

    #[arg(short = 'B', long, help="Set the path to `uv` executable. If not found, it will be downloaded.")]
    #[arg(default_value_os_t = get_output_dir().join(UV_BINARY))]
    pub uv_path: PathBuf,

    #[arg(long, default_value = "true", help="Extract to temporary directory")]
    pub extract_to_temp: Option<String>,

    #[arg(short = 'o', long, default_value = "./pycrucible-launcher", help="Set the output path and launcher name")]
    pub output_path: PathBuf,

    #[arg(short = 't', long, default_value = None, help="Sets target architecture for cross-platform compilation")]
    pub target: Option<String>,
}
