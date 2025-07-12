use std::fs;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use shared::config::load_project_config;
use shared::footer::PayloadInfo;
use crate::repository::RepositoryHandler;


fn extract_from_binary(info: &PayloadInfo) -> Option<Vec<u8>> {
    let exe_path = std::env::current_exe().ok()?;
    let mut file = fs::File::open(exe_path).ok()?;
    
    let file_size = file.metadata().ok()?.len();
    let payload_offset = info.offset;
    let payload_size = file_size - shared::footer::FOOTER_SIZE as u64 - payload_offset;

    // Read payload
    file.seek(SeekFrom::Start(payload_offset)).ok()?;
    let mut payload_data = vec![0u8; payload_size as usize];
    file.read_exact(&mut payload_data).ok()?;
    Some(payload_data)
}

fn extract_from_archive(target_dir: &Path, payload_data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let reader = std::io::Cursor::new(payload_data);
    let mut archive = zip::ZipArchive::new(reader)?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = target_dir.join(file.name());
        
        if let Some(parent) = outpath.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let mut outfile = fs::File::create(&outpath)?;
        std::io::copy(&mut file, &mut outfile)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&outpath)?.permissions();
            // Set execute permission for UV binary
            if file.name().contains("uv") && !file.name().ends_with("/") {
                perms.set_mode(0o755);
            } else {
                // Set read/write permissions for Python files
                perms.set_mode(0o644);
            }
            fs::set_permissions(&outpath, perms)?;
        }
    }
    Ok(())
}

fn extract_payload(info: &PayloadInfo, target_dir: &Path) -> io::Result<()> {
    let payload_data = extract_from_binary(info);
    if payload_data.is_none() {
        return Err(io::Error::new(io::ErrorKind::Other, "Failed to extract payload from binary"));
    }
    let payload_data = payload_data.unwrap();

    let _ = extract_from_archive(target_dir, payload_data);

    Ok(())
}


pub fn prepare_and_extract_payload(create_temp_dir: bool) -> Option<PathBuf> {
    const PAYLOAD_NAME: &str = "pycrucible_payload";
    let payload_info = shared::footer::read_footer();
    if let Err(e) = payload_info {
        eprintln!("Error reading footer: {:?}", e);
        return None;
    }

    let project_dir = if create_temp_dir {
        // Creating temp directory
        let temp_dir = std::env::temp_dir().join(PAYLOAD_NAME);
        fs::create_dir_all(&temp_dir).ok()?;
        temp_dir
    } else {
        let exe_path = std::env::current_exe().ok()?;
        let current_dir = exe_path.parent().unwrap().join(PAYLOAD_NAME);
        fs::create_dir_all(&current_dir).ok()?;
        current_dir
    };

    // Extracting payload
    let footer_info = payload_info.unwrap();
    extract_payload(&footer_info, &project_dir).ok()?;

    // Check for source configuration and update if necessary
    let pycrucibletoml_path = project_dir.join("pycrucible.toml");
    if pycrucibletoml_path.exists() {
        let project_config = load_project_config(&project_dir.to_path_buf());
        if let Some(source_config) = &project_config.source {
            let sp = shared::spinner::create_spinner_with_message("Updating source code from repository...");
            let mut repo_handler = RepositoryHandler::new(source_config.clone());
            
            match repo_handler.init_or_open(&project_dir) {
                Ok(_) => {
                    if let Err(e) = repo_handler.update() {
                        shared::spinner::stop_and_persist_spinner_with_message(sp, "Failed to update repository");
                        eprintln!("Error updating repository: {:?}", e);
                        std::process::exit(1);
                    }
                    
                    shared::spinner::stop_and_persist_spinner_with_message(sp, "Repository updated successfully");
                }
                Err(e) => {
                    shared::spinner::stop_and_persist_spinner_with_message(sp, "Failed to initialize repository");
                    eprintln!("Error initializing repository: {:?}", e);
                }
            }
        }
    }
    
    Some(project_dir)
}