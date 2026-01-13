#![cfg_attr(test, allow(dead_code, unused_variables, unused_imports))]

use crate::{config, runner};
use crate::{debug_println, project};
use shared::uv_handler::find_or_download_uv;
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::io::{self, Cursor, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use zip::ZipArchive;
use zip::{ZipWriter, write::FileOptions};

pub fn find_manifest_file(source_dir: &Path) -> Option<PathBuf> {
    if source_dir.join("pyproject.toml").exists() {
        Some(source_dir.join("pyproject.toml"))
    } else if source_dir.join("requirements.txt").exists() {
        Some(source_dir.join("requirements.txt"))
    } else if source_dir.join("pylock.toml").exists() {
        Some(source_dir.join("pylock.toml"))
    } else if source_dir.join("setup.py").exists() {
        Some(source_dir.join("setup.py"))
    } else if source_dir.join("setup.cfg").exists() {
        Some(source_dir.join("setup.cfg"))
    } else {
        eprintln!(
            "No manifest file found in the source directory. \nManifest files can be pyproject.toml, requirements.txt, pylock.toml, setup.py or setup.cfg"
        );
        None // Default to empty string if none found;
    }
}

fn embed_uv(
    cli_options: &crate::CLIOptions,
    zip: &mut ZipWriter<&mut Cursor<Vec<u8>>>,
    options: FileOptions<'_, ()>,
) -> io::Result<Option<()>> {
    debug_println!("[payload.embed_uv] - Embedding uv binary into payload");
    let uv_path = find_or_download_uv(Some(cli_options.uv_path.clone()), &cli_options.uv_version);
    match uv_path {
        None => {
            eprintln!("Could not find or download uv binary. uv will be required at runtime.");
            Ok(None)
        }
        Some(path) => {
            debug_println!("[payload.embed_payload] - uv binary found at {:?}", path);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&path, perms)?;
                debug_println!("[payload.embed_payload] - Set permissions for uv on linux");
            }
            zip.start_file("uv", options)?;
            // let uv_file = fs::File::open(&uv_path)?;
            let _ = zip.write(&fs::read(path)?);
            // io::copy(&mut uv_file, zip)?;
            debug_println!("[payload.embed_payload] - Added uv to zip");
            Ok(Some(()))
        }
    }
}

fn write_to_zip(
    name: &str,
    file: PathBuf,
    zip: &mut ZipWriter<&mut Cursor<Vec<u8>>>,
    options: FileOptions<'_, ()>,
) -> Result<(), io::Error> {
    zip.start_file(name, options)?;
    let _ = zip.write(&fs::read(file)?);
    Ok(())
}

fn read_wheel_name(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut zip = ZipArchive::new(file)?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry.name();

        if name.ends_with(".dist-info/METADATA") {
            let mut contents = String::new();
            entry.read_to_string(&mut contents)?;

            for line in contents.lines() {
                if let Some(value) = line.strip_prefix("Name: ") {
                    return Ok(value.trim().to_string());
                }
            }
        }
    }

    Err("No Name field found in METADATA".into())
}

pub fn embed_payload(
    source_files: &project::CollectedSources,
    manifest_path: &Option<PathBuf>,
    project_config: &mut config::ProjectConfig,
    cli_options: crate::CLIOptions,
) -> io::Result<()> {
    runner::extract_runner(&cli_options.output_path)?;
    debug_println!("[payload.embed_payload] - Runner extracted to output path");

    // Create a memory buffer for the ZIP
    let mut cursor = Cursor::new(Vec::new());
    let mut zip: ZipWriter<&mut Cursor<Vec<u8>>> = ZipWriter::new(&mut cursor);
    let options: FileOptions<'_, ()> = FileOptions::<()>::default();

    // Update project config with CLI options as we do not use any other file to store these in wheel mode
    project_config.options.debug = cli_options.debug;
    project_config.options.uv_version = cli_options.uv_version.to_string();

    // Check to see if we have a wheel or source files and handle accordingly
    match source_files {
        project::CollectedSources::Wheel(wheel) => {
            project_config.options.extract_to_temp = cli_options.extract_to_temp;
            project_config.options.delete_after_run = cli_options.delete_after_run;

            let wheel_path = &wheel.absolute_path;
            let wheel_file_name =
                wheel_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidInput, "Invalid wheel file name")
                    })?;
            project_config.package.entrypoint = read_wheel_name(wheel_path.to_str().unwrap())
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            debug_println!(
                "[payload.embed_payload] - Embedding wheel file: {:?}",
                wheel_file_name
            );
            write_to_zip(wheel_file_name, wheel_path.clone(), &mut zip, options)?;
            debug_println!("[payload.embed_payload] - Wheel file added to zip");
        }
        project::CollectedSources::Files(files) => {
            if let Some(manifest) = manifest_path {
                copy_source_to_zip(
                    &files
                        .iter()
                        .map(|sf| sf.absolute_path.clone())
                        .collect::<Vec<_>>(),
                    manifest,
                    &mut zip,
                    options,
                )?;
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Manifest path not provided for source files",
                ));
            }
        }
    }

    create_pycrucible_config_file(&project_config, &mut zip, options)?;

    if cli_options.no_uv_embed {
        debug_println!("[payload.embed_payload] - Skipping uv embedding as per no_uv_embed flag");
    } else {
        if cli_options.force_uv_download {
            debug_println!(
                "[payload.embed_payload] - Force uv download flag is set, re-downloading uv"
            );
            let uv_path = if cli_options.uv_path.exists() {
                Some(cli_options.uv_path.clone())
            } else {
                None
            };
            find_or_download_uv(uv_path, cli_options.uv_version.as_str());
        }
        debug_println!("[payload.embed_payload] - Looking for uv binary to embed");
        if let Some(_path) = embed_uv(&cli_options, &mut zip, options)? {
            debug_println!("[payload.embed_payload] - uv binary embedded successfully");
        } else {
            eprintln!("Could not find or download uv binary. uv will be required at runtime.");
        }
    }

    // Finalize ZIP
    zip.finish()?;
    let payload = cursor.into_inner();
    debug_println!("[payload.embed_payload] - Zip finalized");

    // Open output file in append mode (the copied executable)
    let mut file: fs::File = OpenOptions::new()
        .append(true)
        .open(cli_options.output_path)?;

    // Get offset where payload will start
    let offset = file.seek(SeekFrom::End(0))?;

    // Write payload
    file.write_all(&payload)?;

    // Write footer
    let footer = shared::footer::create_footer(project_config.options.extract_to_temp, offset);
    file.write_all(&footer)?;

    Ok(())
}

fn create_pycrucible_config_file(
    project_config: &config::ProjectConfig,
    zip: &mut ZipWriter<&mut Cursor<Vec<u8>>>,
    options: FileOptions<'_, ()>,
) -> Result<(), io::Error> {
    let project_config_toml =
        toml::to_string(&project_config).map_err(|e| io::Error::other(e.to_string()))?;
    let mut pycrucible_file = Cursor::new(project_config_toml);
    zip.start_file("pycrucible.toml", options)?;
    io::copy(&mut pycrucible_file, zip)?;
    debug_println!("[payload.embed_payload] - pycrucible.toml copied");
    Ok(())
}

fn copy_source_to_zip(
    source_files: &[PathBuf],
    manifest_path: &Path,
    zip: &mut ZipWriter<&mut Cursor<Vec<u8>>>,
    options: FileOptions<'_, ()>,
) -> Result<(), io::Error> {
    debug_println!("[payload.embed_payload] - Starting copy of source files to .zip");
    let source_dir = manifest_path.parent().unwrap().canonicalize()?;
    for source_file in source_files {
        let relative_path = source_file
            .strip_prefix(&source_dir)
            .unwrap_or(source_file.as_path());
        let relative_path = relative_path.to_string_lossy().replace("\\", "/");
        debug_println!(
            "[payload.embed_payload] - Copied {:?} with relative path {:?} to zip",
            source_file,
            relative_path
        );
        let mut file = fs::File::open(source_file)?;
        zip.start_file(relative_path, options)?;
        io::copy(&mut file, zip)?;
    }
    let mut manifest_file = fs::File::open(manifest_path)?;
    let manifest_file_name = manifest_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid manifest file name"))?;
    zip.start_file(manifest_file_name, options)?;
    io::copy(&mut manifest_file, zip)?;
    debug_println!("[payload.embed_payload] - Copied manifest file");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::footer::{FOOTER_SIZE, MAGIC_BYTES, PayloadInfo};
    use std::fs::{self, File};
    use std::io::{Read, Seek, SeekFrom};
    use tempfile::tempdir;

    fn extract_payload_from_file(
        info: &PayloadInfo,
        target_dir: &std::path::Path,
        exe_path: &std::path::Path,
    ) -> std::io::Result<()> {
        let mut file = File::open(exe_path)?;
        file.seek(SeekFrom::Start(info.offset))?;

        let mut payload_data = Vec::new();
        file.read_to_end(&mut payload_data)?;

        let reader = std::io::Cursor::new(payload_data);
        let mut archive = zip::ZipArchive::new(reader)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = target_dir.join(file.name());

            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&outpath)?.permissions();
                if file.name().contains("uv") && !file.name().ends_with("/") {
                    perms.set_mode(0o755);
                } else {
                    perms.set_mode(0o644);
                }
                fs::set_permissions(&outpath, perms)?;
            }
        }

        Ok(())
    }

    #[test]
    fn test_embed_and_extract_payload() {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();
        let file1 = src_dir.join("main.py");
        let file2 = src_dir.join("utils.py");
        fs::write(&file1, b"print('hello')").unwrap();
        fs::write(&file2, b"def foo(): pass").unwrap();

        let manifest = dir.path().join("requirements.txt");
        fs::write(&manifest, b"requests").unwrap();

        let mut project_config = config::ProjectConfig {
            package: config::PackageConfig {
                entrypoint: "src/main.py".to_string(),
                ..Default::default()
            },
            options: config::ToolOptions {
                extract_to_temp: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let uv_path = dir.path().join("uv");
        fs::write(&uv_path, b"uv-binary").unwrap();

        let output_path = dir.path().join("output_exe");
        fs::write(&output_path, b"stub-runner").unwrap(); // dummy base exe

        // Build the collected sources expected by embed_payload (Files variant contains items
        // with an `absolute_path` field).
        let collected_files = vec![
            project::SourceFile {
                absolute_path: file1.clone(),
            },
            project::SourceFile {
                absolute_path: file2.clone(),
            },
        ];
        let source_files = project::CollectedSources::Files(collected_files);

        // Manifest path as an Option
        let manifest_option = Some(manifest.clone());

        // Build CLIOptions expected by embed_payload
        let cli_options = crate::CLIOptions {
            source_dir: src_dir.clone(),
            output_path: output_path.clone(),
            uv_path: uv_path.clone(),
            uv_version: "0.9.21".to_string(),
            no_uv_embed: false,
            extract_to_temp: true,
            delete_after_run: false,
            force_uv_download: false,
            debug: false,
        };

        let result = embed_payload(
            &source_files,
            &manifest_option,
            &mut project_config,
            cli_options,
        );
        assert!(result.is_ok(), "embed_payload should succeed");
        assert!(output_path.exists());

        // Read and verify footer
        let mut file = File::open(&output_path).unwrap();
        file.seek(SeekFrom::End(-(FOOTER_SIZE as i64))).unwrap();
        let mut footer = [0u8; FOOTER_SIZE];
        file.read_exact(&mut footer).unwrap();

        let offset = u64::from_le_bytes(footer[0..8].try_into().unwrap());
        let extraction_flag = footer[8] == 1;
        let magic = &footer[9..];

        assert_eq!(magic, MAGIC_BYTES, "Magic bytes mismatch");
        assert!(extraction_flag, "Expected extract_to_temp flag to be true");

        let info = PayloadInfo {
            offset,
            extraction_flag,
        };

        let extract_dir = dir.path().join("extract");
        fs::create_dir(&extract_dir).unwrap();
        let result = extract_payload_from_file(&info, &extract_dir, &output_path);
        assert!(result.is_ok(), "Payload extraction failed");

        assert!(extract_dir.join("src/main.py").exists());
        assert!(extract_dir.join("src/utils.py").exists());
        assert!(extract_dir.join("requirements.txt").exists());
        assert!(extract_dir.join("pycrucible.toml").exists());
    }
}
