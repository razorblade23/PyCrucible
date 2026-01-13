pub mod config;
pub mod debuging;
pub mod footer;
pub mod spinner;
pub mod uv_handler;
// pub mod uv_handler;

pub use config::*;
pub use debuging::*;
pub use footer::{FOOTER_SIZE, PayloadInfo};
pub use spinner::*;
pub use uv_handler::install_uv;
// pub use uv_handler::uv_handler_core::find_or_download_uv;

pub static PYCRUCIBLE_RUNNER_NAME: &str = if cfg!(target_os = "windows") {
    "pycrucible_runner.exe"
} else {
    "pycrucible_runner"
};
