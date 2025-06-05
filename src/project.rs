use glob::Pattern;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use walkdir;

#[derive(Deserialize, Debug, Default)]
pub struct FilePatterns {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Deserialize, Debug, Default)]
pub struct PackageConfig {
    #[serde(default = "default_entrypoint")]
    pub entrypoint: String,
    #[serde(default)]
    pub patterns: FilePatterns,
}

#[derive(Deserialize, Debug, Default)]
pub struct ProjectConfig {
    #[serde(default)]
    pub package: PackageConfig,
}

fn default_entrypoint() -> String {
    "main.py".to_string()
}

pub fn load_project_config(source_dir: &Path) -> ProjectConfig {
    match source_dir.join("pycrucible.toml").canonicalize() {
        Ok(config_path) if config_path.exists() => {
            match fs::read_to_string(&config_path)
                .ok()
                .and_then(|content| toml::from_str(&content).ok())
            {
                Some(config) => config,
                None => ProjectConfig::default(),
            }
        }
        _ => ProjectConfig::default(),
    }
}

#[derive(Debug)]
pub struct SourceFile {
    pub relative_path: PathBuf,
    pub content: Vec<u8>,
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

pub fn collect_source_files(source_dir: &Path) -> io::Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    let mut seen_paths = HashSet::new();
    let source_dir = source_dir.canonicalize()?;

    let project_config = load_project_config(&source_dir);

    let include_patterns = project_config.package.patterns.include;
    let exclude_patterns = project_config.package.patterns.exclude;

    // Collect files based on patterns
    for entry in walkdir::WalkDir::new(&source_dir)
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
                    continue;
                }

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
