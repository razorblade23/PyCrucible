use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

use std::path::{Path, PathBuf};
use crate::debug_println;


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
            debug_println!("Loading project config from project directory - (pycrucible.toml found)");
            load_config(&config_path)
        }
        _ => {
            debug_println!("Loading project config defaults - (pycrucible.toml not found)");
            ProjectConfig::default()
        }
    };
    project_config
}
