pub mod config;
pub mod spinner;
pub mod debuging;
pub mod footer;

pub use config::*;
pub use spinner::*;
pub use debuging::*;
pub use footer::{PayloadInfo, FOOTER_SIZE};
pub use cli::Cli;

pub static PYCRUCIBLE_RUNNER_NAME: &str = if cfg!(target_os = "windows") {
    "pycrucible_runner.exe"
} else {
    "pycrucible_runner"
};