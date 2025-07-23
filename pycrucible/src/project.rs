#![cfg_attr(test, allow(dead_code, unused_variables, unused_imports))]

use glob::Pattern;
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use walkdir;

use crate::config::{load_project_config, ProjectConfig};
use crate::debug_println;

#[derive(Debug)]
pub struct SourceFile {
    pub absolute_path: PathBuf
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
    .replace("\\", "/");
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

pub fn collect_source_files_with_config(
    source_dir: &Path,
    project_config: &ProjectConfig,
) -> io::Result<Vec<SourceFile>> {
    debug_println!("[project.collect_source_files] - Collecting source files from: {:?}", source_dir);
    let mut files = Vec::new();
    let mut seen_paths = HashSet::new();
    let source_dir = source_dir.canonicalize()?;

    let include_patterns = &project_config.package.patterns.include;
    let exclude_patterns = &project_config.package.patterns.exclude;

    for entry in walkdir::WalkDir::new(&source_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if should_include_file(
                entry.path(),
                &source_dir,
                include_patterns,
                exclude_patterns,
            ) {
                let absolute_path = entry.path().to_path_buf();

                if seen_paths.contains(&absolute_path) {
                    continue;
                }

                debug_println!("[project.collect_source_files] - Collected file at path: {:?}", absolute_path);

                seen_paths.insert(absolute_path.clone());
                files.push(SourceFile { absolute_path });
            }
        }
    }
    Ok(files)
}


pub fn collect_source_files(source_dir: &Path) -> io::Result<Vec<SourceFile>> {
    let config = load_project_config(&source_dir.to_path_buf());
    collect_source_files_with_config(source_dir, &config)
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    use crate::config::{ProjectConfig, PackageConfig, FilePatterns};

    // Mock config loader to return fixed patterns
    fn mock_config_with_patterns() -> crate::config::ProjectConfig {
        crate::config::ProjectConfig {
            package: PackageConfig {
                entrypoint: "main.py".to_string(),
                patterns: FilePatterns::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_should_include_file() {
        let source_dir = PathBuf::from("/project");

        let file_py = source_dir.join("main.py");
        let file_txt = source_dir.join("README.txt");
        let file_test = source_dir.join("tests/test_main.py");

        let include = vec!["**/*.py".to_string()];
        let exclude = vec!["tests/*".to_string()];

        assert!(should_include_file(&file_py, &source_dir, &include, &exclude));
        assert!(!should_include_file(&file_txt, &source_dir, &include, &exclude));
        assert!(!should_include_file(&file_test, &source_dir, &include, &exclude));
    }

    #[test]
fn test_collect_source_files_with_mock_config() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();

    fs::write(root.join("src/main.py"), b"print('hi')").unwrap();
    fs::write(root.join("src/ignore.txt"), b"ignore me").unwrap();
    fs::write(root.join("tests/test_main.py"), b"# test").unwrap();

    let mock_config = ProjectConfig {
        package: PackageConfig {
            entrypoint: "main.py".to_string(),
            patterns: FilePatterns {
                include: vec!["**/*.py".to_string()],
                exclude: vec!["tests/*".to_string()],
            },
        },
        ..Default::default()
    };

    let files = collect_source_files_with_config(root, &mock_config).unwrap();

    let collected_paths: Vec<_> = files
        .iter()
        .map(|sf| sf.absolute_path.strip_prefix(root).unwrap().to_path_buf())
        .collect();

    assert_eq!(collected_paths.len(), 1);
    assert_eq!(collected_paths[0], PathBuf::from("src/main.py"));
}

}
