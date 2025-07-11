use shared::config;
use shared::payload;
use shared::spinner_utils::{
    create_spinner_with_message,
    stop_and_persist_spinner_with_message
};
use shared::repository::RepositoryHandler;
use std::fs;
use std::path::PathBuf;


pub fn extract_payload(create_temp_dir: bool) -> Option<PathBuf> {
    let payload_info = payload::read_footer().ok()?;
    let project_dir = if create_temp_dir {
        // Creating temp directory
        let temp_dir = std::env::temp_dir().join("python_app_payload");
        fs::create_dir_all(&temp_dir).ok()?;
        temp_dir
    } else {
        let exe_path = std::env::current_exe().ok()?;
        let current_dir = exe_path.parent().unwrap().join("payload");
        fs::create_dir_all(&current_dir).ok()?;
        current_dir
    };

    // Extracting payload
    payload::extract_payload(&payload_info, &project_dir).ok()?;

    // Check for source configuration and update if necessary
    let pycrucibletoml_path = project_dir.join("pycrucible.toml");
    if pycrucibletoml_path.exists() {
        let project_config = config::load_project_config(&project_dir.to_path_buf());
        if let Some(source_config) = &project_config.source {
            let sp = create_spinner_with_message("Updating source code from repository...");
            let mut repo_handler = RepositoryHandler::new(source_config.clone());
            
            match repo_handler.init_or_open(&project_dir) {
                Ok(_) => {
                    if let Err(e) = repo_handler.update() {
                        stop_and_persist_spinner_with_message(sp, "Failed to update repository");
                        eprintln!("Error updating repository: {:?}", e);
                        std::process::exit(1);
                    }
                    
                    stop_and_persist_spinner_with_message(sp, "Repository updated successfully");
                }
                Err(e) => {
                    stop_and_persist_spinner_with_message(sp, "Failed to initialize repository");
                    eprintln!("Error initializing repository: {:?}", e);
                }
            }
        }
    }
    
    Some(project_dir)
}