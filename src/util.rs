// Note: transfer sends now go through `secure::send_secure_to_endpoints`, which
// returns the same `Vec<(Endpoint, io::Error)>` error shape consumed by the
// `Reportable` impl and `stringify_sendall_errors` below.

// split messages to fit the width of the ui panel
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
pub fn split_each(input: String, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![input];
    }
    let mut splitted = Vec::with_capacity(input.width() / width);
    let mut row = String::new();

    let mut index = 0;

    for current_char in input.chars() {
        if (index != 0 && index == width) || index + current_char.width().unwrap_or(0) > width {
            splitted.push(std::mem::take(&mut row));
            index = 0;
        }

        row.push(current_char);
        index += current_char.width().unwrap_or(0);
    }
    // leftover
    if !row.is_empty() {
        splitted.push(std::mem::take(&mut row));
    }
    splitted
}

// Errors
pub type Error = anyhow::Error;
pub type Result<T> = anyhow::Result<T>;

pub fn stringify_sendall_errors(e: Vec<(message_io::network::Endpoint, std::io::Error)>) -> String {
    let mut out = String::new();
    for (endpoint, error) in e {
        let msg = format!("Failed to connect to {}, error: {}", endpoint, error);
        out.push_str(&msg);
        out.push('\n');
    }
    // remove last new line
    if !out.is_empty() {
        out.pop();
    }
    out
}

use crate::state::State;
/// Trait for reporting Recoverable errors/ Infos to the user
pub trait Reportable: Sized {
    fn report_if_err(self, _state: &mut State) {
        unimplemented!()
    }
    fn report_err(self, _state: &mut State) {
        unimplemented!()
    }
    fn report_info(self, _state: &mut State) {
        unimplemented!()
    }
}

impl Reportable for Result<()> {
    fn report_if_err(self, state: &mut State) {
        if let Err(e) = self {
            state.add_system_error_message(e.to_string());
        }
    }
}

impl Reportable for std::result::Result<(), Vec<(message_io::network::Endpoint, std::io::Error)>> {
    fn report_if_err(self, state: &mut State) {
        if let Err(e) = self {
            state.add_system_error_message(crate::util::stringify_sendall_errors(e));
        }
    }
}

impl Reportable for anyhow::Error {
    fn report_err(self, state: &mut State) {
        self.to_string().report_err(state);
    }
}

impl Reportable for String {
    fn report_err(self, state: &mut State) {
        state.add_system_error_message(self);
    }

    fn report_info(self, state: &mut State) {
        state.add_system_info_message(self);
    }
}

#[cfg(test)]
mod tests {
    use super::split_each;

    #[test]
    fn split_each_zero_width_does_not_divide_by_zero() {
        assert_eq!(split_each("hello".to_string(), 0), vec!["hello".to_string()]);
    }

    #[test]
    fn split_each_wraps_to_width() {
        assert_eq!(split_each("abcd".to_string(), 2), vec!["ab".to_string(), "cd".to_string()]);
    }
}
