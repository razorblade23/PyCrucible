use spinners::{Spinner, Spinners};

pub fn create_spinner_with_message(msg: &str) -> Spinner {
    let sp = Spinner::new(Spinners::Dots9, msg.into());
    sp
}

pub fn stop_and_persist_spinner_with_message(mut sp: Spinner, msg: &str) {
    sp.stop_and_persist("✔", msg.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_creation() {
        let msg = "Testing spinner";
        // Just verify the function doesn't panic
        let _spinner = create_spinner_with_message(msg);
    }

    #[test]
    fn test_spinner_stop_and_persist() {
        let msg = "Testing spinner";
        let spinner = create_spinner_with_message(msg);
        let final_msg = "Done testing";
        // Just verify the function doesn't panic
        stop_and_persist_spinner_with_message(spinner, final_msg);
    }
}
