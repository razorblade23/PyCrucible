use std::path::Path;
use git2::{Repository, FetchOptions};
use crate::config::SourceConfig;
use crate::debug_println;

pub struct RepositoryHandler {
    repo: Option<Repository>,
    config: SourceConfig,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum RepositoryError {
    GitError(git2::Error),
    InvalidConfiguration(&'static str),
}

impl From<git2::Error> for RepositoryError {
    fn from(err: git2::Error) -> Self {
        RepositoryError::GitError(err)
    }
}

impl RepositoryHandler {
    pub fn new(config: SourceConfig) -> Self {
        RepositoryHandler {
            repo: None,
            config,
        }
    }

    pub fn init_or_open(&mut self, path: &Path) -> Result<(), RepositoryError> {
        self.repo = if path.join(".git").exists() {
            debug_println!("Found existing repository at {}, opening ...", path.display());
            Some(Repository::open(path)?)
        } else {
            debug_println!("No existing repository found at {}, cloning from {}", path.display(), self.config.repository);
            if path.exists() {
                let uv_name = if cfg!(windows) { "uv.exe" } else { "uv" };
                let uv_path = path.join(uv_name);
                let temp_uv_path = std::env::temp_dir().join(uv_name);

                // Move uv to temp if it exists
                if uv_path.exists() {
                    std::fs::rename(&uv_path, &temp_uv_path).expect("Failed to move uv to temp");
                }

                // Clear directory
                let entries = std::fs::read_dir(path).expect("Failed to read directory");
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_dir() {
                            std::fs::remove_dir_all(path).expect("Failed to remove directory");
                        } else {
                            std::fs::remove_file(path).expect("Failed to remove file");
                        }
                    }
                }
            }
            let repo = Some(Repository::clone(&self.config.repository, path)?);

            // Move uv back if it was temporarily moved
            let temp_uv_path = std::env::temp_dir().join(if cfg!(windows) { "uv.exe" } else { "uv" });
            if temp_uv_path.exists() {
                std::fs::rename(&temp_uv_path, path.join(if cfg!(windows) { "uv.exe" } else { "uv" }))
                    .expect("Failed to move uv back");
            }
            
            repo
        };
        Ok(())
    }

    pub fn update(&self) -> Result<(), RepositoryError> {
        let repo = self.repo.as_ref().ok_or(RepositoryError::InvalidConfiguration("Repository not initialized"))?;
        let mut remote = repo.find_remote("origin")?;
        
        let strategy = self.config.update_strategy.as_deref().unwrap_or("pull");
        
        match strategy {
            "pull" => {
                let mut fetch_opts = FetchOptions::new();
                remote.fetch(&[] as &[&str], Some(&mut fetch_opts), None)?;
                
                // Get the latest commit from the remote branch
                let branch_name = self.config.branch.as_deref().unwrap_or("master");
                let remote_branch = repo.find_branch(&format!("origin/{}", branch_name), git2::BranchType::Remote)?;
                let remote_commit = remote_branch.get().peel_to_commit()?;
                
                // Get the current HEAD and set it to the remote commit
                let mut head = repo.head()?;
                head.set_target(remote_commit.id(), "pull: Fast-forward update")?;
            }
            "fetch" => {
                let mut fetch_opts = FetchOptions::new();
                remote.fetch(&[] as &[&str], Some(&mut fetch_opts), None)?;
            }
            _ => return Err(RepositoryError::InvalidConfiguration("Invalid update strategy")),
        }
        
        // If specific tag or commit is specified, check it out
        if let Some(tag) = &self.config.tag {
            let tag_oid = git2::Oid::from_str(tag)?;
            let tag_ref = repo.find_tag(tag_oid)?;
            repo.set_head_detached(tag_ref.target_id())?;
        } else if let Some(commit) = &self.config.commit {
            let oid = git2::Oid::from_str(commit)?;
            let commit_obj = repo.find_commit(oid)?;
            repo.set_head_detached(commit_obj.id())?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_repository_handler_creation() {
        let config = SourceConfig {
            repository: "https://github.com/example/repo".to_string(),
            branch: Some("main".to_string()),
            tag: None,
            commit: None,
            update_strategy: Some("pull".to_string()),
        };

        let handler = RepositoryHandler::new(config);
        assert!(handler.update().is_err()); // Should error because repo is not initialized
    }

    #[test]
    fn test_repository_invalid_config() {
        let config = SourceConfig {
            repository: "invalid-url".to_string(),
            branch: Some("main".to_string()),
            tag: None,
            commit: None,
            update_strategy: Some("invalid".to_string()),
        };

        let mut handler = RepositoryHandler::new(config);
        let dir = tempdir().unwrap();
        assert!(handler.init_or_open(dir.path()).is_err());
    }

    #[test]
    fn test_repository_clone() {
        let config = SourceConfig {
            repository: "file:///tmp/test-repo".to_string(), // Use local path instead of HTTP
            branch: Some("main".to_string()),
            tag: None,
            commit: None,
            update_strategy: Some("pull".to_string()),
        };

        let mut handler = RepositoryHandler::new(config);
        let dir = tempdir().unwrap();
        
        // Create a test git repo
        let repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("test", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();
        
        // Test opening
        assert!(handler.init_or_open(dir.path()).is_ok());
        assert!(dir.path().join(".git").exists());
    }

    #[test]
    fn test_update_strategy() {
        let config = SourceConfig {
            repository: "file:///tmp/test-repo".to_string(),
            branch: Some("main".to_string()),
            tag: None,
            commit: None,
            update_strategy: Some("fetch".to_string()),
        };

        let mut handler = RepositoryHandler::new(config);
        let dir = tempdir().unwrap();
        
        // Create a test git repo
        let repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("test", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();
        
        // Initialize repo
        handler.init_or_open(dir.path()).unwrap();

        // Test invalid strategy
        let config = SourceConfig {
            repository: "file:///tmp/test-repo".to_string(),
            branch: Some("main".to_string()),
            tag: None,
            commit: None,
            update_strategy: Some("invalid".to_string()),
        };

        let handler = RepositoryHandler::new(config);
        assert!(matches!(handler.update(), Err(RepositoryError::InvalidConfiguration(_))));
    }
}
