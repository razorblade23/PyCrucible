use glob::Pattern;
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use walkdir;

use crate::config::load_project_config;
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

pub fn collect_source_files(source_dir: &Path) -> io::Result<Vec<SourceFile>> {
    debug_println!("Collecting source files from: {:?}", source_dir);
    let mut files = Vec::new();
    let mut seen_paths = HashSet::new();
    let source_dir = source_dir.canonicalize()?;

    let project_config = load_project_config(&source_dir);
    debug_println!("Project config: {:?}", project_config);

    let include_patterns = project_config.package.patterns.include;
    let exclude_patterns = project_config.package.patterns.exclude;

    // Collect files based on patterns
    debug_println!("Collecting project files");
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

                let absolute_path = entry
                    .path()
                    .to_path_buf();

                if seen_paths.contains(&absolute_path) {
                    continue;
                }

                debug_println!("Collected file at path: {:?}", absolute_path);

                seen_paths.insert(absolute_path.clone());
                files.push(SourceFile {
                    absolute_path
                });
            }
        }
    }
    debug_println!("All collected files: {:?}", files);
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_collect_source_files_empty_dir() {
        let dir = tempdir().unwrap();
        let files = collect_source_files(dir.path()).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_collect_source_files_with_python_files() {
        let dir = tempdir().unwrap();
        
        // Create some Python files
        fs::write(dir.path().join("main.py"), "print('hello')").unwrap();
        fs::write(dir.path().join("utils.py"), "def helper(): pass").unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src").join("lib.py"), "class MyClass: pass").unwrap();
        
        let files = collect_source_files(dir.path()).unwrap();
        assert_eq!(files.len(), 3);
        
        let file_names: Vec<String> = files
            .iter()
            .map(|f| f.absolute_path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        
        assert!(file_names.contains(&"main.py".to_string()));
        assert!(file_names.contains(&"utils.py".to_string()));
        assert!(file_names.contains(&"lib.py".to_string()));
    }

    #[test]
    fn test_exclude_patterns() {
        let dir = tempdir().unwrap();
        
        // Create some files
        fs::write(dir.path().join("main.py"), "print('hello')").unwrap();
        fs::create_dir(dir.path().join("__pycache__")).unwrap();
        fs::write(dir.path().join("__pycache__").join("main.pyc"), "").unwrap();
        fs::create_dir(dir.path().join(".venv")).unwrap();
        fs::write(dir.path().join(".venv").join("lib.py"), "").unwrap();
        
        let files = collect_source_files(dir.path()).unwrap();
        assert_eq!(files.len(), 1); // Should only include main.py
        
        let file = &files[0];
        assert_eq!(file.absolute_path.file_name().unwrap().to_string_lossy(), "main.py");
    }

    #[test]
    fn test_custom_include_exclude_patterns() {
        let dir = tempdir().unwrap();
        let config_content = r#"
            [package]
            entrypoint = "main.py"

            [package.patterns]
            include = ["**/*.txt"]
            exclude = ["**/*.py"]
        "#;
        
        fs::write(dir.path().join("pycrucible.toml"), config_content).unwrap();
        fs::write(dir.path().join("data.txt"), "some data").unwrap();
        fs::write(dir.path().join("script.py"), "print('hello')").unwrap();
        
        let files = collect_source_files(dir.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].absolute_path.file_name().unwrap().to_string_lossy(), "data.txt");
    }
}
