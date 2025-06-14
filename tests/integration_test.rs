// #[cfg(test)]
// mod integration {
//     use std::path::Path;
//     use tempfile::tempdir;

//     #[test]
//     fn test_full_workflow() {
//         // Create a temporary project
//         let dir = tempdir().unwrap();
//         let project_dir = dir.path();
        
//         // Create a simple Python project
//         std::fs::write(
//             project_dir.join("main.py"),
//             "print('Hello from PyCrucible!')"
//         ).unwrap();
        
//         std::fs::write(
//             project_dir.join("pyproject.toml"),
//             r#"[project]
//             name = "test-project"
//             version = "0.1.0"
//             "#
//         ).unwrap();
        
//         std::fs::write(
//             project_dir.join("pycrucible.toml"),
//             r#"[package]
//             entrypoint = "main.py"
            
//             [package.patterns]
//             include = ["**/*.py"]
//             exclude = ["**/__pycache__/**"]
//             "#
//         ).unwrap();
        
//         // Create output path
//         let output_path = project_dir.join("test_app");
        
//         // Test the full workflow:
//         // 1. Collect source files
//         let source_files = pycrucible::project::collect_source_files(project_dir).unwrap();
//         assert!(!source_files.is_empty());
        
//         // 2. Load config
//         let config = pycrucible::config::load_project_config(&project_dir.to_path_buf());
//         assert_eq!(config.package.entrypoint, "main.py");
        
//         // 3. Download UV
//         let uv_path = pycrucible::uv_handler::download_binary_and_unpack(None).unwrap();
//         assert!(uv_path.exists());
        
//         // 4. Embed payload
//         let result = pycrucible::payload::embed_payload(
//             &source_files.iter().map(|f| f.absolute_path.clone()).collect::<Vec<_>>(),
//             &project_dir.join("pyproject.toml"),
//             config,
//             uv_path,
//             &output_path
//         );
//         assert!(result.is_ok());
//         assert!(output_path.exists());
        
//         // 5. Verify embedded payload
//         let footer_result = pycrucible::payload::read_footer();
//         assert!(footer_result.is_ok());
//     }

//     #[test]
//     fn test_error_handling() {
//         // Test with missing project directory
//         let nonexistent_dir = Path::new("/nonexistent/directory");
//         let result = pycrucible::project::collect_source_files(nonexistent_dir);
//         assert!(result.is_err());
        
//         // Test with invalid config
//         let dir = tempdir().unwrap();
//         std::fs::write(
//             dir.path().join("pycrucible.toml"),
//             "invalid toml content"
//         ).unwrap();
//         let config = pycrucible::config::ProjectConfig::from_file(
//             &dir.path().join("pycrucible.toml")
//         );
//         assert!(config.is_err());
        
//         // Test with missing entrypoint
//         let result = pycrucible::runner::run_extracted_project(dir.path());
//         assert!(result.is_err());
//     }
// }
