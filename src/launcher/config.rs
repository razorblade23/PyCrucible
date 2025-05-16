use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

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

#[derive(serde::Serialize, Debug, Deserialize)]
pub struct ProjectConfig {
    pub package: PackageConfig,
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
            uv: None,
            env: None,
            hooks: None,
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
            load_config(&config_path)
        }
        _ => {
            ProjectConfig::default()
        }
    };
    project_config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_config_default() {
        let default_config = ProjectConfig::default();
        assert_eq!(default_config.package.entrypoint, "main.py");
        assert!(
            default_config
                .package
                .patterns
                .include
                .contains(&"**/*.py".to_string())
        );
        assert!(
            default_config
                .package
                .patterns
                .exclude
                .contains(&".venv/**/*".to_string())
        );
    }

    #[test]
    fn test_project_config_from_file_valid() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("pycrucible.toml");
        let mut file = File::create(&config_path).unwrap();

        let toml_content = r#"
            [package]
            entrypoint = "app.py"
            [package.patterns]
            include = ["src/**/*.py"]
            exclude = ["tests/**/*"]

            [uv]
            args = ["--debug"]

            [env]
            VAR1 = "value1"
            VAR2 = "value2"

            [hooks]
            pre_run = "echo Pre-run"
            post_run = "echo Post-run"
        "#;

        file.write_all(toml_content.as_bytes()).unwrap();

        let config = ProjectConfig::from_file(&config_path).unwrap();
        assert_eq!(config.package.entrypoint, "app.py");
        assert!(
            config
                .package
                .patterns
                .include
                .contains(&"src/**/*.py".to_string())
        );
        assert!(
            config
                .package
                .patterns
                .exclude
                .contains(&"tests/**/*".to_string())
        );
        assert_eq!(config.uv.unwrap().args.unwrap(), vec!["--debug"]);
        assert_eq!(config.env.unwrap().variables.get("VAR1").unwrap(), "value1");
        assert_eq!(config.hooks.unwrap().pre_run.unwrap(), "echo Pre-run");
    }

    #[test]
    fn test_project_config_from_file_invalid() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("pycrucible.toml");
        let mut file = File::create(&config_path).unwrap();

        let invalid_toml_content = r#"
            [package]
            entrypoint = 123  # Invalid type
        "#;

        file.write_all(invalid_toml_content.as_bytes()).unwrap();

        let result = ProjectConfig::from_file(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_project_config_with_existing_file() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("pycrucible.toml");
        let mut file = File::create(&config_path).unwrap();

        let toml_content = r#"
            [package]
            entrypoint = "app.py"
        "#;

        file.write_all(toml_content.as_bytes()).unwrap();

        let config = load_project_config(&temp_dir.into_path());
        assert_eq!(config.package.entrypoint, "app.py");
    }

    #[test]
    fn test_load_project_config_with_missing_file() {
        let temp_dir = tempdir().unwrap();
        let config = load_project_config(&temp_dir.into_path());
        assert_eq!(config.package.entrypoint, "main.py");
    }
}
