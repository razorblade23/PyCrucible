mod extract;
mod repository;
mod run;

use std::io;

fn main() -> io::Result<()> {
    let path = extract::prepare_and_extract_payload();
    if path.is_none() {
        eprintln!("Failed to extract payload");
        std::process::exit(1);
    }
    let project_dir = path.unwrap();

    run::run_extracted_project(&project_dir)?;
    Ok(())
}