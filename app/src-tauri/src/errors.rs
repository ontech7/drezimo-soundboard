//! What went wrong lately, for the support report (`report.rs`). Each error still goes to stderr too.

use chrono::{DateTime, Local};
use std::collections::VecDeque;
use std::sync::Mutex;

const KEPT: usize = 20;

static RECENT: Mutex<Recent> = Mutex::new(Recent(VecDeque::new()));

/// An error, and how often it came in a row: a failure retried every few seconds takes one line
#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    pub message: String,
    pub first: DateTime<Local>,
    pub last: DateTime<Local>,
    pub count: u32,
}

struct Recent(VecDeque<Error>);

impl Recent {
    fn push(&mut self, at: DateTime<Local>, message: String) {
        if let Some(latest) = self.0.back_mut().filter(|latest| latest.message == message) {
            latest.last = at;
            latest.count += 1;
            return;
        }
        if self.0.len() == KEPT {
            self.0.pop_front();
        }
        self.0.push_back(Error {
            message,
            first: at,
            last: at,
            count: 1,
        });
    }
}

pub fn log(message: impl Into<String>) {
    let message = message.into();
    eprintln!("{message}");
    RECENT.lock().unwrap().push(Local::now(), message);
}

/// The latest errors, oldest first.
pub fn recent() -> Vec<Error> {
    RECENT.lock().unwrap().0.iter().cloned().collect()
}

/// A system error in English, whatever the system's language: its kind and the system's own number.
pub fn in_english(error: &std::io::Error) -> String {
    match error.raw_os_error() {
        Some(code) => format!("{} (os error {code})", error.kind()),
        None => error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn keeps_the_twenty_most_recent() {
        let mut recent = Recent(VecDeque::new());
        for i in 1..=25 {
            recent.push(Local::now(), format!("error {i}"));
        }
        assert_eq!(recent.0.len(), 20);
        assert_eq!(recent.0[0].message, "error 6");
        assert_eq!(recent.0[19].message, "error 25");
    }

    #[test]
    fn a_repeated_error_takes_one_line() {
        let at = |second| Local.with_ymd_and_hms(2026, 11, 3, 13, 0, second).unwrap();
        let mut recent = Recent(VecDeque::new());
        recent.push(at(1), "a".into());
        for second in 2..=4 {
            recent.push(at(second), "b".into());
        }
        let lines: Vec<_> = recent
            .0
            .iter()
            .map(|error| (error.message.as_str(), error.count, error.first, error.last))
            .collect();
        assert_eq!(lines, [("a", 1, at(1), at(1)), ("b", 3, at(2), at(4))]);
    }

    #[test]
    fn writes_a_system_error_in_english() {
        let reset = std::io::Error::from(std::io::ErrorKind::ConnectionReset);
        assert_eq!(in_english(&reset), "connection reset");
        #[cfg(windows)]
        assert_eq!(
            in_english(&std::io::Error::from_raw_os_error(10054)),
            "connection reset (os error 10054)"
        );
    }
}
