use lazy_static::lazy_static;
use std::sync::atomic::{AtomicBool, Ordering};

lazy_static! {
    static ref DEBUG_MODE: AtomicBool = AtomicBool::new(false);
}

pub fn set_debug_mode(debug: bool) {
    DEBUG_MODE.store(debug, Ordering::Relaxed);
}

pub fn is_debug_mode() -> bool {
    DEBUG_MODE.load(Ordering::Relaxed)
}

// Helper macro for debug printing
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        if $crate::debuging::is_debug_mode() {
            println!("Debug: {}", format!($($arg)*));
        }
    };
}