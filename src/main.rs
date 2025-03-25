mod cli;
mod launcher;

use clap::Parser;
use cli::Cli;
use glob::Pattern;
use launcher::config::ProjectConfig;
use spinners::{Spinner, Spinners};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use launcher::generator::LauncherGenerator;

#[derive(Debug)]
struct SourceFile {
    relative_path: PathBuf,
    content: Vec<u8>,
}

struct BuilderConfig {
    source_files: Vec<SourceFile>,
    manifest: Vec<u8>,
    uv_binary: Vec<u8>,
    output_path: String,
}

fn load_config(config_path: &PathBuf) -> ProjectConfig {
    match ProjectConfig::from_file(&config_path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!(
                "Warning: Failed to load config, using defaults. Error: {}",
                e
            );
            ProjectConfig::default()
        }
    }
}

fn should_include_file(
    file_path: &Path,
    source_dir: &Path,
    include_patterns: &[String],
    exclude_patterns: &[String],
) -> bool {
    let relative_path = file_path
        .strip_prefix(source_dir)
        .unwrap()
        .to_string_lossy()
        .to_string();

    // Check exclude patterns first
    for pattern in exclude_patterns {
        if Pattern::new(pattern).unwrap().matches(&relative_path) {
            return false;
        }
    }

    // If include patterns are specified, file must match at least one
    include_patterns
        .iter()
        .any(|pattern| Pattern::new(pattern).unwrap().matches(&relative_path))
}

fn collect_source_files(source_dir: &Path) -> io::Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    let mut seen_paths = HashSet::new();
    let source_dir = source_dir.canonicalize()?;

    // Load config with default Python-specific patterns
    let project_config = match source_dir.join("pycrucible.toml").canonicalize() {
        Ok(config_path) if config_path.exists() => {
            println!("Loading config from: {:?}", config_path);
            load_config(&config_path)
        }
        _ => {
            println!("Using default Python-specific configuration");
            ProjectConfig::default()
        }
    };

    let include_patterns = project_config.package.patterns.include;
    let exclude_patterns = project_config.package.patterns.exclude;

    // Collect files based on patterns
    for entry in WalkDir::new(&source_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if should_include_file(
                entry.path(),
                &source_dir,
                &include_patterns,
                &exclude_patterns,
            ) {
                let relative_path = entry
                    .path()
                    .strip_prefix(&source_dir)
                    .unwrap()
                    .to_path_buf();

                if seen_paths.contains(&relative_path) {
                    eprintln!("Warning: Skipping duplicate file: {:?}", relative_path);
                    continue;
                }

                println!("Found source file: {:?}", relative_path);
                seen_paths.insert(relative_path.clone());
                let content = fs::read(entry.path())?;
                files.push(SourceFile {
                    relative_path,
                    content,
                });
            }
        }
    }
    Ok(files)
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let mut sp = Spinner::new(Spinners::Dots9, "Collecting source files ...".into());
    let source_files = collect_source_files(&cli.source_dir)?;
    if source_files.is_empty() {
        eprintln!("No Python source files found in the specified directory");
        std::process::exit(1);
    }

    let manifest_path = cli.source_dir.join("pyproject.toml");
    if !manifest_path.exists() {
        eprintln!("No pyproject.toml found in the source directory");
        std::process::exit(1);
    }

    sp.stop_and_persist("✔", "Source files collected".into());

    let config = BuilderConfig {
        source_files,
        manifest: fs::read(manifest_path)?,
        uv_binary: fs::read(&cli.uv_path)?,
        output_path: cli.output_path.to_string_lossy().to_string(),
    };

    let generator = LauncherGenerator::new(config);
    generator.generate_and_compile()?;
    Ok(())
}
