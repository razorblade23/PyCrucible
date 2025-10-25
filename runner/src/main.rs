mod extract;
mod repository;
mod run;

use std::io;
use std::env;

fn main() -> io::Result<()> {
    let runtime_args: Vec<String> = env::args().skip(1).collect();
    let path = extract::prepare_and_extract_payload();
    if path.is_none() {
        eprintln!("Failed to extract payload");
        std::process::exit(1);
    }
    let project_dir = path.unwrap();

    run::run_extracted_project(&project_dir, runtime_args)?;
    Ok(())
}