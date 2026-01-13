mod cli;
mod payload;
mod project;
mod runner;

use clap::Parser;
use cli::Cli;
use shared::config;
use shared::spinner::{create_spinner_with_message, stop_and_persist_spinner_with_message};
use shared::{debug_println, debuging};
use std::format;
use std::io;
use std::path::PathBuf;

pub struct CLIOptions {
    source_dir: PathBuf,
    output_path: PathBuf,
    uv_path: PathBuf,
    uv_version: String,
    no_uv_embed: bool,
    extract_to_temp: bool,
    delete_after_run: bool,
    force_uv_download: bool,
    debug: bool,
}

fn embed_source(cli_options: CLIOptions) -> io::Result<()> {
    // Create ProjectConfig based on pycrucible-toml or default if there is no such file
    let mut project_config = config::load_project_config(&cli_options.source_dir.to_path_buf());
    debug_println!("[main.embed_source] - Project config: {:?}", project_config);

    let sp = create_spinner_with_message("Collecting source files ...");

    let collected_sources = project::collect_source_files(&cli_options.source_dir)?;

    payload::embed_payload(
        &collected_sources,
        &payload::find_manifest_file(&cli_options.source_dir),
        &mut project_config,
        cli_options,
    )?;

    stop_and_persist_spinner_with_message(sp, "Source files embedded successfully.");
    Ok(())
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    debuging::set_debug_mode(cli.debug);

    // Determine where we are running from, payload path and output path
    let current_dir = std::env::current_dir()?;

    let payload_path: PathBuf = cli.embed;
    if !payload_path.exists() {
        eprintln!(
            "The specified payload directory does not exist: {}",
            payload_path.display()
        );
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

    // // Determine manifest file - this file contains requirements for the project
    // let manifest_path = payload::find_manifest_file(&payload_path);
    // if !manifest_path.exists() {
    //     eprintln!("No manifest file found in the source directory");
    //     std::process::exit(1);
    // }
    // debug_println!(
    //     "[main] - Payload path: {:?} | Output path: {:?} | Manifest path: {:?}",
    //     payload_path,
    //     output_path,
    //     manifest_path
    // );

    let cli_options = CLIOptions {
        source_dir: payload_path.clone(),
        output_path: output_path.clone(),
        uv_path: cli.uv_path,
        uv_version: cli.uv_version,
        no_uv_embed: cli.no_uv_embed,
        extract_to_temp: cli.extract_to_temp,
        delete_after_run: cli.delete_after_run,
        force_uv_download: cli.force_uv_download,
        debug: cli.debug,
    };
    // Embed the project and create new binary
    embed_source(cli_options)?;
    println!(
        "Successfully embedded Python project into new binary: {}",
        output_path.display()
    );

    Ok(())
}
