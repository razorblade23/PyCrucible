pub mod config;
pub mod debuging;
pub mod footer;
pub mod spinner;
// pub mod uv_handler_v2;
pub mod uv_handler;

pub use config::*;
pub use debuging::*;
pub use footer::{FOOTER_SIZE, PayloadInfo};
pub use spinner::*;
// pub use uv_handler_v2::{find_or_download_uv, download_and_install_uv_v2};
pub use uv_handler::uv_handler_core::find_or_download_uv;

pub static PYCRUCIBLE_RUNNER_NAME: &str = if cfg!(target_os = "windows") {
    "pycrucible_runner.exe"
} else {
    "pycrucible_runner"
};
