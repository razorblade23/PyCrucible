use spinners::{Spinner, Spinners};

pub fn create_spinner_with_message(msg: &str) -> Spinner {
    Spinner::new(Spinners::Dots9, msg.into())
}

pub fn stop_and_persist_spinner_with_message(mut sp: Spinner, msg: &str) {
    sp.stop_and_persist("✔", msg.into());
}
