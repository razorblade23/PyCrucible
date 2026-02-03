mod extract;
mod repository;
mod run;

use std::{ env, io, path::PathBuf};

fn main() -> io::Result<()> {
    let runtime_args: Vec<String> = env::args()
        .skip(1)
        .map(|arg| {
            let path = PathBuf::from(&arg);
            // Check if the argument is a valid path that exists on disk
            if path.exists() {
                // Convert to absolute path (canonicalize handles things like "../" too)
                match path.canonicalize() {
                    Ok(abs_path) => abs_path.to_string_lossy().into_owned(),
                    Err(_) => arg, // Fallback to original if we can't resolve it
                }
            } else {
                arg // If it's not a path (e.g., a flag like --verbose), leave it alone
            }
        })
        .collect();

    let path = extract::prepare_and_extract_payload();
    if path.is_none() {
        eprintln!("Failed to extract payload");
        std::process::exit(1);
    }
    let project_dir = path.unwrap();

    run::run_extracted_project(&project_dir, runtime_args)?;
    Ok(())
}
