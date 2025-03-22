mod launcher;
mod cli;

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use cli::Cli;
use clap::Parser;

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

fn collect_source_files(source_dir: &Path) -> io::Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    let source_dir = source_dir.canonicalize()?;

    for entry in WalkDir::new(&source_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() && entry.path().extension().map_or(false, |ext| ext == "py") {
            let relative_path = entry
                .path()
                .strip_prefix(&source_dir)
                .unwrap()
                .to_path_buf();
            let content = fs::read(entry.path())?;
            files.push(SourceFile {
                relative_path,
                content,
            });
        }
    }
    Ok(files)
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

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