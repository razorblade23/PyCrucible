mod project;
mod payload;
mod runner;
mod cli;

use std::format;
use cli::Cli;
use shared::spinner::{create_spinner_with_message, stop_and_persist_spinner_with_message};
use std::io;
use std::path::Path;
use std::path::PathBuf;
use shared::config;
use shared::{debuging, debug_println};
use clap::Parser;

fn embed_source(source_dir: &Path, output_path: &Path, uv_path: PathBuf, no_uv_embed: bool) -> io::Result<()> {
    // Create ProjectConfig based on pycrucible-toml or default if there is no such file
    let project_config = config::load_project_config(&source_dir.to_path_buf());
    debug_println!("[main.embed_source] - Project config: {:?}", project_config);

    let sp = create_spinner_with_message("Collecting source files ...");
    let source_files = project::collect_source_files(source_dir)?;
    if source_files.is_empty() {
        eprintln!("No Python source files found in the specified directory");
        std::process::exit(1);
    }

    // Check manifest
    let manifest_path = payload::find_manifest_file(source_dir);
    if !manifest_path.exists() {
        eprintln!("No manifest file found in the source directory");
        std::process::exit(1);
    }
    debug_println!("[main.embed_source] - Manifest path: {:?}", manifest_path);

    stop_and_persist_spinner_with_message(sp, "Source files collected");

    // Embed Python project into the binary
    let source_paths: Vec<_> = source_files.iter().map(|sf| sf.absolute_path.clone()).collect();
    debug_println!("[main.embed_source] - Starting embedding proccess");
    payload::embed_payload(&source_paths, &manifest_path, project_config, uv_path, output_path, no_uv_embed)
}



fn main() -> io::Result<()> {
    let cli = Cli::parse();
    debuging::set_debug_mode(cli.debug);

    // Determine where we are running from, payload path and output path
    let current_dir = std::env::current_dir()?;

    let payload_path: PathBuf = cli.embed;
    if !payload_path.exists() {
        eprintln!("The specified payload directory does not exist: {}", payload_path.display());
        std::process::exit(1);
    }

    let output_path = if cli.output.is_none() {
        let launcher_name = if cfg!(windows) {
            "launcher.exe"
        } else {
            "launcher"
        };

        current_dir.join(launcher_name)
    } else {
        current_dir.join(cli.output.unwrap())
    };
    
    // Determine manifest file - this file contains requirements for the project
    let manifest_path = payload::find_manifest_file(&payload_path);
    if !manifest_path.exists() {
        eprintln!("No manifest file found in the source directory");
        std::process::exit(1);
    }
    debug_println!("[main] - Payload path: {:?} | Output path: {:?} | Manifest path: {:?}", payload_path, output_path, manifest_path);

    // Embed the project and create new binary
    embed_source(&payload_path, &output_path, cli.uv_path, cli.no_uv_embed)?;
    println!("Successfully embedded Python project into new binary: {}", output_path.display());

    Ok(())
}