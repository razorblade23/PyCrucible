use std::io;
use std::path::Path;
use std::process::Command;
use crate::config::load_project_config;
use crate::debug_println;


pub fn run_extracted_project(project_dir: &Path) -> io::Result<()> {
    // Verify Python files exist
    let config = load_project_config(&project_dir.to_path_buf());
    debug_println!("Loaded project config");
    let entrypoint = config.package.entrypoint;
    debug_println!("Loaded project entrypopoint");
    let entry_point_path = project_dir.join(&entrypoint);
    debug_println!("Loaded project entrypoint path");
    let uv_path = project_dir.join("uv");
    debug_println!("Loaded project uv");
    
    if !entry_point_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Entry point {} not found", entry_point_path.display())
        ));
    }
    // Run uv pip sync with proper environment
    let status = Command::new(&uv_path)
        .arg("sync")
        .arg("-qq")
        .current_dir(&project_dir)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "uv sync failed"));
    }

    let hooks = config.hooks.unwrap();
    let pre_hook = hooks.pre_run.unwrap();
    let post_hook = hooks.post_run.unwrap();


    if !pre_hook.is_empty() {
        let status = Command::new(&uv_path)
            .arg("run")
            .arg(pre_hook)
            .current_dir(&project_dir)
            .status()?;
        if !status.success() {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to run pre-hook"));
        }
    }

    let status = Command::new(&uv_path)
        .arg("run")
        .arg(entrypoint)
        .current_dir(&project_dir)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Main application failed"));
    }

    if !post_hook.is_empty() {
        let status = Command::new(&uv_path)
            .arg("run")
            .arg(post_hook)
            .current_dir(&project_dir)
            .status()?;
        if !status.success() {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to run post-hook"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_run_project_missing_entrypoint() {
        let dir = tempdir().unwrap();
        let result = run_extracted_project(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_run_project_with_entrypoint() {
        let dir = tempdir().unwrap();
        
        // Create minimal project structure
        fs::write(dir.path().join("main.py"), "print('test')").unwrap();
        fs::write(dir.path().join("pyproject.toml"), r#"
            [project]
            name = "test-project"
            version = "0.1.0"
        "#).unwrap();
        fs::write(dir.path().join("pycrucible.toml"), r#"
            [package]
            entrypoint = "main.py"

            [hooks]
            pre_run = ""
            post_run = ""
        "#).unwrap();
        
        // Create uv executable mock (just a script that does nothing)
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            fs::OpenOptions::new()
                .create(true)
                .write(true)
                .mode(0o755)
                .open(dir.path().join("uv"))
                .unwrap();
        }
        #[cfg(windows)]
        {
            fs::write(dir.path().join("uv.exe"), "").unwrap();
        }
        
        let result = run_extracted_project(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_project_with_hooks() {
        let dir = tempdir().unwrap();
        
        // Create project structure with hooks
        fs::write(dir.path().join("main.py"), "print('main')").unwrap();
        fs::write(dir.path().join("pre_hook.py"), "print('pre-hook')").unwrap();
        fs::write(dir.path().join("post_hook.py"), "print('post-hook')").unwrap();
        fs::write(dir.path().join("pyproject.toml"), r#"
            [project]
            name = "test-project"
            version = "0.1.0"
        "#).unwrap();
        fs::write(dir.path().join("pycrucible.toml"), r#"
            [package]
            entrypoint = "main.py"

            [hooks]
            pre_run = "pre_hook.py"
            post_run = "post_hook.py"
        "#).unwrap();
        
        // Create uv executable mock
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            fs::OpenOptions::new()
                .create(true)
                .write(true)
                .mode(0o755)
                .open(dir.path().join("uv"))
                .unwrap();
        }
        #[cfg(windows)]
        {
            fs::write(dir.path().join("uv.exe"), "").unwrap();
        }
        
        let result = run_extracted_project(dir.path());
        assert!(result.is_ok());
    }
}
