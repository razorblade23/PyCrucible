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
            include: vec!["**/*.py".to_string()],
            exclude: vec![
                ".venv/**/*".to_string(),
                "**/__pycache__/**".to_string(),
                ".git/**/*".to_string(),
                "**/*.pyc".to_string(),
                "**/*.pyo".to_string(),
                "**/*.pyd".to_string(),
            ],
        }
    }
}

#[derive(serde::Serialize, Debug, Deserialize)]
pub struct PackageConfig {
    #[serde(alias = "entry")]
    pub entrypoint: String,
    #[serde(default)]
    pub patterns: FilePatterns,
}

impl Default for PackageConfig {
    fn default() -> Self {
        PackageConfig {
            entrypoint: "main.py".into(),
            patterns: FilePatterns::default(),
        }
    }
}

#[derive(serde::Serialize, Debug, Deserialize)]
pub struct UVConfig {
    pub args: Option<Vec<String>>,
}
impl Default for UVConfig {
    fn default() -> Self {
        UVConfig {
            args: None,
        }
    }
}

#[derive(serde::Serialize, Debug, Deserialize)]
pub struct EnvConfig {
    #[serde(flatten)]
    pub variables: Option<HashMap<String, String>>,
}
impl Default for EnvConfig {
    fn default() -> Self {
        EnvConfig {
            variables: None,
        }
    }
}

#[derive(serde::Serialize, Debug, Deserialize)]
pub struct Hooks {
    pub pre_run: Option<String>,
    pub post_run: Option<String>,
}
impl Default for Hooks {
    fn default() -> Self {
        Hooks {
            pre_run: None,
            post_run: None,
        }
    }
}

#[derive(serde::Serialize, Debug, Deserialize, Clone)]
pub struct SourceConfig {
    pub repository: String,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub commit: Option<String>,
    pub update_strategy: Option<String>, // "pull" or "fetch"
}
impl Default for SourceConfig {
    fn default() -> Self {
        SourceConfig {
            repository: String::new(),
            branch: None,
            tag: None,
            commit: None,
            update_strategy: Some("pull".to_string()), // Default to "pull"
        }
    }
}


#[derive(serde::Serialize, Debug, Deserialize)]
pub struct ProjectConfig {
    #[serde(flatten)]
    pub package: PackageConfig,
    #[serde(default)]
    pub source: Option<SourceConfig>,
    #[serde(default)]
    pub uv: Option<UVConfig>,
    #[serde(default)]
    pub env: Option<EnvConfig>,
    #[serde(default)]
    pub hooks: Option<Hooks>,
}

impl ProjectConfig {
    /// Load configuration from the specified file path.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        toml::from_str(&content).map_err(|e| e.to_string())
    }

    fn from_pyproject(path: &Path) -> Result<Self, String> {
        let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let doc: toml::Value = toml::from_str(&raw).map_err(|e| e.to_string())?;

        let tbl = doc.get("tool")
            .and_then(|t| t.get("pycrucible"))
            .ok_or("no [tool.pycrucible] section")?;

        // Re-serialize just that sub-table so we can leverage
        // the existing `ProjectConfig` derive.
        let slice = toml::to_string(tbl).map_err(|e| e.to_string())?;
        toml::from_str(&slice).map_err(|e| e.to_string())
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
    // Is there pycrucible.toml in the source directory?
    if let Ok(path) = source_dir.join("pycrucible.toml").canonicalize() {
        if path.exists() {
            debug_println!("[config] using pycrucible.toml");
            return load_config(&path);
        }
    }


    let pyproject = source_dir.join("pyproject.toml").canonicalize();
    if let Ok(pyproject) = pyproject {
        if pyproject.exists() {
            match ProjectConfig::from_pyproject(&pyproject) {
                Ok(cfg) => {
                    debug_println!("[config] using [tool.pycrucible] in pyproject.toml");
                    return cfg;
                }
                Err(e) => {
                    debug_println!(
                        "[config] pyproject.toml found but no usable [tool.pycrucible] - {}",
                        e
                    );
                }
            }
        }
    }

    // No config file found, use built-in defaults
    debug_println!("[config] using built-in defaults");
    ProjectConfig::default()
}

