use std::fs::{self, OpenOptions};
use std::io::{self, Write, Seek, SeekFrom, Cursor};
use std::path::{Path, PathBuf};
use zip::{write::FileOptions, ZipWriter};
use crate::{config, runner};
use crate::debug_println;
use crate::uv_handler::find_or_download_uv;


pub fn find_manifest_file(source_dir: &Path) -> PathBuf  {
    let manifest_path = if source_dir.join("pyproject.toml").exists() {
        source_dir.join("pyproject.toml")
    } else if source_dir.join("requirements.txt").exists() {
        source_dir.join("requirements.txt")
    } else if source_dir.join("pylock.toml").exists() {
        source_dir.join("pylock.toml")
    } else if source_dir.join("setup.py").exists() {
        source_dir.join("setup.py")
    } else if source_dir.join("setup.cfg").exists() {
        source_dir.join("setup.cfg")
    } else {
        eprintln!("No manifest file found in the source directory. \nManifest files can be pyproject.toml, requirements.txt, pylock.toml, setup.py or setup.cfg");
        source_dir.join("") // Default to empty string if none found;
    };
    manifest_path
}

pub fn embed_payload(source_files: &[PathBuf], manifest_path: &Path, project_config: config::ProjectConfig, uv_path: PathBuf, output_path: &Path) -> io::Result<()> {
    let _ = runner::extract_runner(output_path)?;
    debug_println!("[payload.embed_payload] - Runner extracted to output path");

    // Create a memory buffer for the ZIP
    let mut cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut cursor);
    let options = FileOptions::<()>::default();

    copy_source_to_zip(source_files, manifest_path, &mut zip, options)?;

    create_pycrucible_config_file(&project_config, &mut zip, options)?;

    find_or_download_uv(uv_path, &mut zip, options)?;

    // Finalize ZIP
    zip.finish()?;
    let payload = cursor.into_inner();
    debug_println!("[payload.embed_payload] - Zip finalized");

    // Open output file in append mode (the copied executable)
    let mut file: fs::File = OpenOptions::new()
        .write(true)
        .append(true)
        .open(output_path)?;

    // Get offset where payload will start
    let offset = file.seek(SeekFrom::End(0))?;

    // Write payload
    file.write_all(&payload)?;

    
    // Write footer
    let footer = shared::footer::create_footer(
        project_config.options.extract_to_temp, 
        offset
    );
    file.write_all(&footer)?;

    Ok(())
}

fn create_pycrucible_config_file(project_config: &config::ProjectConfig, zip: &mut ZipWriter<&mut Cursor<Vec<u8>>>, options: FileOptions<'_, ()>) -> Result<(), io::Error> {
    let project_config_toml = toml::to_string(&project_config)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    let mut pycrucible_file = Cursor::new(project_config_toml);
    zip.start_file("pycrucible.toml", options)?;
    io::copy(&mut pycrucible_file, zip)?;
    debug_println!("[payload.embed_payload] - pycrucible.toml copied");
    Ok(())
}

fn copy_source_to_zip(source_files: &[PathBuf], manifest_path: &Path, zip: &mut ZipWriter<&mut Cursor<Vec<u8>>>, options: FileOptions<'_, ()>) -> Result<(), io::Error> {
    debug_println!("[payload.embed_payload] - Starting copy of source files to .zip");
    let source_dir = manifest_path.parent().unwrap().canonicalize()?;
    for source_file in source_files {
        let relative_path = source_file.strip_prefix(&source_dir)
            .unwrap_or(source_file.as_path());
        let relative_path = relative_path.to_string_lossy().replace("\\", "/");
        debug_println!("[payload.embed_payload] - Copied {:?} with relative path {:?} to zip", source_file, relative_path);
        let mut file = fs::File::open(source_file)?;
        zip.start_file(relative_path, options)?;
        io::copy(&mut file, zip)?;
    }
    let mut manifest_file = fs::File::open(manifest_path)?;
    let manifest_file_name = manifest_path.file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid manifest file name"))?;
    zip.start_file(manifest_file_name, options)?;
    io::copy(&mut manifest_file, zip)?;
    debug_println!("[payload.embed_payload] - Copied manifest file");
    Ok(())
}



// #[cfg(test)]
// mod tests {
//     use std::fs::{self, File};
//     use std::io::{Seek, SeekFrom};
//     use tempfile::tempdir;
//     use super::*;

//     // Helper for tests: extract from a specific file, not current_exe
//     fn extract_payload_from_file(info: &PayloadInfo, target_dir: &std::path::Path, exe_path: &std::path::Path) -> std::io::Result<()> {
//         let mut file = File::open(exe_path)?;

//         file.seek(SeekFrom::Start(info.offset))?;
//         let mut payload_data = vec![0u8; info.size as usize];
//         file.read_exact(&mut payload_data)?;

//         let reader = std::io::Cursor::new(payload_data);
//         let mut archive = zip::ZipArchive::new(reader)?;

//         for i in 0..archive.len() {
//             let mut file = archive.by_index(i)?;
//             let outpath = target_dir.join(file.name());

//             if let Some(parent) = outpath.parent() {
//                 fs::create_dir_all(parent)?;
//             }

//             let mut outfile = File::create(&outpath)?;
//             std::io::copy(&mut file, &mut outfile)?;

//             #[cfg(unix)]
//             {
//                 use std::os::unix::fs::PermissionsExt;
//                 let mut perms = fs::metadata(&outpath)?.permissions();
//                 if file.name().contains("uv") && !file.name().ends_with("/") {
//                     perms.set_mode(0o755);
//                 } else {
//                     perms.set_mode(0o644);
//                 }
//                 fs::set_permissions(&outpath, perms)?;
//             }
//         }

//         Ok(())
//     }

//     #[test]
//     fn test_embed_and_extract_payload() {
//         let dir = tempdir().unwrap();
//         let src_dir = dir.path().join("src");
//         fs::create_dir(&src_dir).unwrap();
//         let file1 = src_dir.join("main.py");
//         let file2 = src_dir.join("utils.py");
//         fs::write(&file1, b"print('hello')").unwrap();
//         fs::write(&file2, b"def foo(): pass").unwrap();

//         let manifest = dir.path().join("manifest.toml");
//         fs::write(&manifest, b"[project]\nname = 'test'").unwrap();

//         let project_config = config::ProjectConfig {
//             package: config::PackageConfig {
//                 entrypoint: "src/main.py".to_string(), 
//                 ..Default::default()
//             },
//             ..Default::default()
//         };

//         let uv_path = dir.path().join("uv");
//         fs::write(&uv_path, b"uv-binary").unwrap();

//         let output_path = dir.path().join("output_exe");
//         let source_files = vec![file1.clone(), file2.clone()];

//         let result = embed_payload(
//             &source_files,
//             &manifest,
//             project_config,
//             uv_path.clone(),
//             &output_path,
//         );
//         assert!(result.is_ok());
//         assert!(output_path.exists());

//         let mut file = File::open(&output_path).unwrap();
//         file.seek(SeekFrom::End(-(FOOTER_SIZE as i64))).unwrap();
//         let mut footer = [0u8; FOOTER_SIZE];
//         file.read_exact(&mut footer).unwrap();
//         assert_eq!(&footer[0..4], MAGIC_BYTES);

//         let offset = u64::from_le_bytes(footer[4..12].try_into().unwrap());
//         let size = u32::from_le_bytes(footer[12..16].try_into().unwrap());
//         let info = PayloadInfo { offset, size };

//         let extract_dir = dir.path().join("extract");
//         fs::create_dir(&extract_dir).unwrap();
//         let result = extract_payload_from_file(&info, &extract_dir, &output_path);
//         assert!(result.is_ok());

//         assert!(extract_dir.join("src/main.py").exists());
//         assert!(extract_dir.join("src/utils.py").exists());
//         assert!(extract_dir.join("manifest.toml").exists());
//         assert!(extract_dir.join("pycrucible.toml").exists());
//         assert!(extract_dir.join("uv").exists());
//     }
// }
