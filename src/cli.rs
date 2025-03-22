use clap::Parser;
use std::path::PathBuf;

const about: &str = "Tool to generate python executable by melding UV and python source code in crusable of one binary";

#[derive(Parser, Debug)]
#[command(author, version, about = about, long_about = None)]
pub struct Cli {
    pub source_dir: PathBuf,

    #[arg(short = 'B', long, default_value = "./uv")]
    pub uv_path: String,
    
    #[arg(short = 'o', long, default_value = "./PyCrucible")]
    pub output_path: PathBuf,

    #[arg(long, default_value = "release", value_parser = ["debug", "release"])]
    pub profile: String,

}