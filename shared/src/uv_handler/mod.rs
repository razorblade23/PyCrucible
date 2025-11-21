pub mod uv_handler_core;

#[cfg(unix)]
pub mod uv_handler_unix;
#[cfg(target_os = "windows")]
pub mod uv_handler_windows;

pub use uv_handler_core::{ find_or_download_uv, download_and_install_uv, uv_exists };