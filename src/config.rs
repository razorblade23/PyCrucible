use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

use std::path::{Path, PathBuf};



#[derive(serde::Serialize, Debug, Deserialize)]
pub struct FilePatterns {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl Default for FilePatterns {
    fn default() -> Self {
        FilePatterns {
            include: vec![],
            exclude: vec![],
        }
    }
}

#[derive(serde::Serialize, Debug, Deserialize)]
pub struct PackageConfig {
    pub entrypoint: String,
    #[serde(default)]
    pub patterns: FilePatterns,
}

#[derive(serde::Serialize, Debug, Deserialize)]
pub struct UVConfig {
    pub args: Option<Vec<String>>,
}

#[derive(serde::Serialize, Debug, Deserialize)]
pub struct EnvConfig {
    #[serde(flatten)]
    pub variables: HashMap<String, String>,
}

#[derive(serde::Serialize, Debug, Deserialize)]
pub struct Hooks {
    pub pre_run: Option<String>,
    pub post_run: Option<String>,
}

#[derive(serde::Serialize, Debug, Deserialize, Clone)]
pub struct SourceConfig {
    pub repository: String,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub commit: Option<String>,
    pub update_strategy: Option<String>, // "pull" or "fetch"
}

#[derive(serde::Serialize, Debug, Deserialize)]
pub struct ProjectConfig {
    pub package: PackageConfig,
    pub source: Option<SourceConfig>,
    pub uv: Option<UVConfig>,
    pub env: Option<EnvConfig>,
    pub hooks: Option<Hooks>,
}

impl ProjectConfig {
    /// Load configuration from the specified file path.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        toml::from_str(&content).map_err(|e| e.to_string())
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        ProjectConfig {
            package: PackageConfig {
                entrypoint: "main.py".into(),
                patterns: FilePatterns {
                    include: vec!["**/*.py".to_string()],
                    exclude: vec![
                        ".venv/**/*".to_string(),
                        "**/__pycache__/**".to_string(),
                        ".git/**/*".to_string(),
                        "**/*.pyc".to_string(),
                        "**/*.pyo".to_string(),
                        "**/*.pyd".to_string(),
                    ],
                },
            },
            source: None,
            uv: None,
            env: None,
            hooks: Some(Hooks {
                pre_run: Some("".to_string()),
                post_run: Some("".to_string()),
            }),
        }
    }
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

pub fn load_project_config(source_dir: &PathBuf) -> ProjectConfig {
    // Load config with default Python-specific patterns
    let project_config = match source_dir.join("pycrucible.toml").canonicalize() {
        Ok(config_path) if config_path.exists() => {
            println!("Loading project config from project directory - (pycrucible.toml found)");
            load_config(&config_path)
        }
        _ => {
            println!("Loading project config defaults - (pycrucible.toml not found)");
            ProjectConfig::default()
        }
    };
    project_config
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = ProjectConfig::default();
        
        assert_eq!(config.package.entrypoint, "main.py");
        assert!(config.source.is_none());
        assert!(config.uv.is_none());
        assert!(config.env.is_none());
        
        // Check default patterns
        assert_eq!(config.package.patterns.include, vec!["**/*.py"]);
        assert!(config.package.patterns.exclude.contains(&"**/__pycache__/**".to_string()));
        assert!(config.package.patterns.exclude.contains(&".venv/**/*".to_string()));
        assert!(config.package.patterns.exclude.contains(&".git/**/*".to_string()));
    }

    #[test]
    fn test_load_config_from_file() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("pycrucible.toml");
        
        let config_content = r#"
            [package]
            entrypoint = "src/app.py"

            [package.patterns]
            include = ["**/*.py", "**/*.pyi"]
            exclude = ["tests/**/*"]

            [source]
            repository = "https://github.com/user/repo"
            branch = "main"
        "#;

        std::fs::write(&config_path, config_content).unwrap();

        let config = ProjectConfig::from_file(config_path.as_path()).unwrap();

        assert_eq!(config.package.entrypoint, "src/app.py");
        assert_eq!(config.package.patterns.include, vec!["**/*.py", "**/*.pyi"]);
        assert_eq!(config.package.patterns.exclude, vec!["tests/**/*"]);
        assert!(config.source.is_some());
        let source = config.source.unwrap();
        assert_eq!(source.repository, "https://github.com/user/repo");
        assert_eq!(source.branch.unwrap(), "main");
    }

    #[test]
    fn test_invalid_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("pycrucible.toml");
        
        let invalid_content = r#"
            [package
            invalid toml content
        "#;

        std::fs::write(&config_path, invalid_content).unwrap();

        assert!(ProjectConfig::from_file(config_path.as_path()).is_err());
    }

    #[test]
    fn test_load_project_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("pycrucible.toml");
        let source_dir = dir.path().to_path_buf();

        // Test with no config file (should use defaults)
        let default_config = load_project_config(&source_dir);
        assert_eq!(default_config.package.entrypoint, "main.py");

        // Test with config file
        let config_content = r#"
            [package]
            entrypoint = "src/main.py"

            [package.patterns]
            include = ["**/*.py"]
            exclude = ["tests/**/*"]
        "#;

        std::fs::write(&config_path, config_content).unwrap();
        let loaded_config = load_project_config(&source_dir);
        assert_eq!(loaded_config.package.entrypoint, "src/main.py");
    }
}
