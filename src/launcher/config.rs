use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct PackageConfig {
    pub entrypoint: String,
    #[serde(default)]
    pub patterns: FilePatterns,
}

#[derive(Debug, Deserialize)]
pub struct UVConfig {
    pub args: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct EnvConfig {
    #[serde(flatten)]
    pub variables: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct Hooks {
    pub pre_run: Option<String>,
    pub post_run: Option<String>,
}

#[derive(Debug, Deserialize)]
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
