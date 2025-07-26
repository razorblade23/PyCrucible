#![cfg_attr(test, allow(dead_code, unused_variables, unused_imports))]

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

#[derive(serde::Serialize, Debug, Deserialize, Clone)]
pub struct ToolOptions {
    #[serde(default)]
    pub debug: bool,
    #[serde(default)]
    pub extract_to_temp: bool,
    #[serde(default)]
    pub delete_after_run: bool,
    #[serde(default)]
    pub offline_mode: bool,
}
impl Default for ToolOptions {
    fn default() -> Self {
        ToolOptions {
            debug: false,
            extract_to_temp: false,
            delete_after_run: false,
            offline_mode: false,
        }
    }
}


#[derive(serde::Serialize, Debug, Deserialize)]
pub struct ProjectConfig {
    #[serde(flatten)]
    pub package: PackageConfig,
    #[serde(default)]
    pub options: ToolOptions,
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
        let config = toml::from_str(&slice).map_err(|e| e.to_string());
        config
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
            options: ToolOptions::default(),
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


#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;
    use super::*;

    #[test]
    fn test_file_patterns_default() {
        let patterns = FilePatterns::default();
        assert!(patterns.include.contains(&"**/*.py".to_string()));
        assert!(patterns.exclude.contains(&".venv/**/*".to_string()));
    }

    #[test]
    fn test_package_config_default() {
        let pkg = PackageConfig::default();
        assert_eq!(pkg.entrypoint, "main.py");
        assert!(pkg.patterns.include.contains(&"**/*.py".to_string()));
    }

    #[test]
    fn test_project_config_default() {
        let config = ProjectConfig::default();
        assert_eq!(config.package.entrypoint, "main.py");
        assert!(config.package.patterns.include.contains(&"**/*.py".to_string()));
        assert!(config.source.is_none());
        assert!(config.uv.is_none());
        assert!(config.env.is_none());
        assert!(config.hooks.is_none());
    }

    #[test]
    fn test_project_config_from_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("pycrucible.toml");
        let toml_content = r#"
            entry = "app.py"
            [patterns]
            include = ["src/**/*.py"]
            exclude = ["tests/**/*"]
        "#;
        let mut file = File::create(&file_path).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();

        let config = ProjectConfig::from_file(&file_path).unwrap();
        assert_eq!(config.package.entrypoint, "app.py");
        assert!(config.package.patterns.include.contains(&"src/**/*.py".to_string()));
        assert!(config.package.patterns.exclude.contains(&"tests/**/*".to_string()));
    }

    #[test]
    fn test_project_config_from_pyproject() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("pyproject.toml");
        let toml_content = r#"
            [tool.pycrucible]
            entry = "main2.py"
            [tool.pycrucible.patterns]
            include = ["lib/**/*.py"]
        "#;
        let mut file = File::create(&file_path).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();

        let config = ProjectConfig::from_pyproject(&file_path).unwrap();
        assert_eq!(config.package.entrypoint, "main2.py");
        assert!(config.package.patterns.include.contains(&"lib/**/*.py".to_string()));
    }

    #[test]
    fn test_load_project_config_with_pycrucible_toml() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("pycrucible.toml");
        let toml_content = r#"
            entry = "run.py"
        "#;
        let mut file = File::create(&config_path).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();

        let config = load_project_config(&dir.path().to_path_buf());
        assert_eq!(config.package.entrypoint, "run.py");
    }

    #[test]
    fn test_load_project_config_with_pyproject_toml() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("pyproject.toml");
        let toml_content = r#"
            [tool.pycrucible]
            entry = "main3.py"
        "#;
        let mut file = File::create(&config_path).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();

        let config = load_project_config(&dir.path().to_path_buf());
        assert_eq!(config.package.entrypoint, "main3.py");
    }

    #[test]
    fn test_load_project_config_defaults_when_no_config() {
        let dir = tempdir().unwrap();
        let config = load_project_config(&dir.path().to_path_buf());
        assert_eq!(config.package.entrypoint, "main.py");
    }

    #[test]
    fn test_source_config_default() {
        let source = SourceConfig::default();
        assert_eq!(source.repository, "");
        assert_eq!(source.update_strategy, Some("pull".to_string()));
    }

    #[test]
    fn test_env_config_default() {
        let env = EnvConfig::default();
        assert!(env.variables.is_none());
    }

    #[test]
    fn test_hooks_default() {
        let hooks = Hooks::default();
        assert!(hooks.pre_run.is_none());
        assert!(hooks.post_run.is_none());
    }

    #[test]
    fn test_uv_config_default() {
        let uv = UVConfig::default();
        assert!(uv.args.is_none());
    }
}
