mod cli;
mod payload;
mod project;
mod runner;
mod spinner_utils;
mod uv_handler;
mod config;

use clap::Parser;
use cli::Cli;
use spinner_utils::{create_spinner_with_message, stop_and_persist_spinner_with_message};
use std::fs;
use std::io;
use std::path::Path;

fn embed_source(source_dir: &Path, output_path: &Path) -> io::Result<()> {
    // Collect source files
    let sp = create_spinner_with_message("Collecting source files ...");
    let source_files = project::collect_source_files(source_dir)?;
    if source_files.is_empty() {
        eprintln!("No Python source files found in the specified directory");
        std::process::exit(1);
    }

    // Check manifest
    let manifest_path = source_dir.join("pyproject.toml");
    if !manifest_path.exists() {
        eprintln!("No pyproject.toml found in the source directory");
        std::process::exit(1);
    }

    // Create ProjectConfig based on pycrucible-tom or default if there is no such file
    let pycrucibletoml_path = source_dir.join("pycrucible.toml");
    let project_config = if pycrucibletoml_path.exists() {
        config::load_project_config(&source_dir.to_path_buf())
    } else {
        config::ProjectConfig::default()
    };


    stop_and_persist_spinner_with_message(sp, "Source files collected");

    // Embed Python project into the binary
    let source_paths: Vec<_> = source_files.iter().map(|sf| sf.absolute_path.clone()).collect();
    payload::embed_payload(&source_paths, &manifest_path, project_config, output_path)
}

fn extract_and_run() -> io::Result<()> {
    let payload_info = payload::read_footer()?;
    
    // Creating temp directory
    let temp_dir = std::env::temp_dir().join("python_app_payload");
    fs::create_dir_all(&temp_dir)?;

    // Extracting payload
    payload::extract_payload(&payload_info, &temp_dir)?;

    // Running application
    runner::run_extracted_project(&temp_dir)
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    if let Some(source_dir) = cli.embed {
        // Embedding mode - add Python project to current binary
        let output_path = cli.output.ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Output path (-o) required when embedding")
        })?;

        // Check if the manifest file exists
        let manifest_path = source_dir.join("pyproject.toml");
        if !manifest_path.exists() {
            eprintln!("No pyproject.toml found in the source directory");
            std::process::exit(1);
        }

        // Embed the project and create new binary
        embed_source(&source_dir, &output_path)?;
        println!("Successfully embedded Python project into new binary: {}", output_path.display());
    } else {
        // Try to run embedded payload
        match payload::read_footer() {
            Ok(_) => extract_and_run()?,
            Err(e) => {
                eprintln!("No Python project embedded in this binary");
                eprintln!("Use --embed <project_dir> -o <output_binary> to create a new binary with embedded code");
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
