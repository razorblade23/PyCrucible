mod cli;
mod payload;
mod project;
mod runner;
mod spinner_utils;
mod uv_handler;
mod config;
mod debuging;
mod repository;

use clap::Parser;
use cli::Cli;
use spinner_utils::{create_spinner_with_message, stop_and_persist_spinner_with_message};
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use repository::RepositoryHandler;
use uv_handler::find_manifest_file;

fn embed_source(source_dir: &Path, output_path: &Path, uv_path: PathBuf) -> io::Result<()> {
    // Create ProjectConfig based on pycrucible-toml or default if there is no such file
    let pycrucibletoml_path = source_dir.join("pycrucible.toml");
    let project_config = if pycrucibletoml_path.exists() {
        config::load_project_config(&source_dir.to_path_buf())
    } else {
        config::ProjectConfig::default()
    };
    debug_println!("[main.embed_source] - Project config: {:?}", project_config);

    // Repository handling moved to extract_and_run function

    // Collect source files
    debug_println!("[main.embed_source] - Source dir: {:?}", source_dir);
    debug_println!("[main.embed_source] - Output dir: {:?}", source_dir);

    let sp = create_spinner_with_message("Collecting source files ...");
    let source_files = project::collect_source_files(source_dir)?;
    if source_files.is_empty() {
        eprintln!("No Python source files found in the specified directory");
        std::process::exit(1);
    }

    // Check manifest
    let manifest_path = find_manifest_file(source_dir);
    if !manifest_path.exists() {
        eprintln!("No manifest file found in the source directory");
        std::process::exit(1);
    }
    debug_println!("[main.embed_source] - Manifest path: {:?}", manifest_path);

    stop_and_persist_spinner_with_message(sp, "Source files collected");

    // Embed Python project into the binary
    let source_paths: Vec<_> = source_files.iter().map(|sf| sf.absolute_path.clone()).collect();
    debug_println!("[main.embed_source] - Starting embedding proccess");
    payload::embed_payload(&source_paths, &manifest_path, project_config, uv_path, output_path)
}

fn extract_and_run(create_temp_dir: bool) -> io::Result<()> {
    let payload_info = payload::read_footer()?;
    
    let project_dir = if create_temp_dir {
        // Creating temp directory
        let temp_dir = std::env::temp_dir().join("python_app_payload");
        fs::create_dir_all(&temp_dir)?;
        temp_dir
    } else {
        let exe_path = std::env::current_exe()?;
        let current_dir = exe_path.parent().unwrap().join("payload");
        fs::create_dir_all(&current_dir)?;
        current_dir
    };

    // Extracting payload
    payload::extract_payload(&payload_info, &project_dir)?;

    // Check for source configuration and update if necessary
    let pycrucibletoml_path = project_dir.join("pycrucible.toml");
    if pycrucibletoml_path.exists() {
        let project_config = config::load_project_config(&project_dir.to_path_buf());
        if let Some(source_config) = &project_config.source {
            let sp = create_spinner_with_message("Updating source code from repository...");
            let mut repo_handler = RepositoryHandler::new(source_config.clone());
            
            match repo_handler.init_or_open(&project_dir) {
                Ok(_) => {
                    if let Err(e) = repo_handler.update() {
                        stop_and_persist_spinner_with_message(sp, "Failed to update repository");
                        eprintln!("Error updating repository: {:?}", e);
                        std::process::exit(1);
                    }
                    
                    stop_and_persist_spinner_with_message(sp, "Repository updated successfully");
                }
                Err(e) => {
                    stop_and_persist_spinner_with_message(sp, "Failed to initialize repository");
                    eprintln!("Error initializing repository: {:?}", e);
                }
            }
        }
    }

    // Running application
    runner::run_extracted_project(&project_dir)
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    debuging::set_debug_mode(cli.debug);

    let has_embedded_project = payload::read_footer().is_ok();

    if let Some(source_dir) = cli.embed {
        if has_embedded_project {
            eprintln!("This binary already has an embedded Python project.");
            std::process::exit(1);
        }
        let source_dir = if source_dir.is_relative() {
            std::env::current_dir()?.join(source_dir)
        } else {
            source_dir
        };

        // Embedding mode - add Python project to current binary
        let output_path = cli.output.ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Output path (-o) required when embedding")
        })?;

        // Check if the manifest file exists
        // Check manifest
        let manifest_path = find_manifest_file(&source_dir);
        if !manifest_path.exists() {
            eprintln!("No manifest file found in the source directory");
            std::process::exit(1);
        }

        // Embed the project and create new binary
        embed_source(&source_dir, &output_path, cli.uv_path)?;
        println!("Successfully embedded Python project into new binary: {}", output_path.display());
    } else {
        // Try to run embedded payload
        if has_embedded_project {
            extract_and_run(cli.extract_to_temp)?;
        } else {
            eprintln!("No Python project embedded in this binary");
            eprintln!("Use --embed <project_dir> -o <output_binary> to create a new binary with embedded code");
            std::process::exit(1);
        }
    }

    Ok(())
}
