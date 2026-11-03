//! What went wrong lately, for the support report (`report.rs`). Each error still goes to stderr too.

use chrono::{DateTime, Local};
use std::collections::VecDeque;
use std::sync::Mutex;

const KEPT: usize = 20;

static RECENT: Mutex<Recent> = Mutex::new(Recent(VecDeque::new()));

struct Recent(VecDeque<(DateTime<Local>, String)>);

impl Recent {
    fn push(&mut self, at: DateTime<Local>, message: String) {
        if self.0.len() == KEPT {
            self.0.pop_front();
        }
        self.0.push_back((at, message));
    }
}

pub fn log(message: impl Into<String>) {
    let message = message.into();
    eprintln!("{message}");
    RECENT.lock().unwrap().push(Local::now(), message);
}

/// The latest errors, oldest first.
pub fn recent() -> Vec<(DateTime<Local>, String)> {
    RECENT.lock().unwrap().0.iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_twenty_most_recent() {
        let mut recent = Recent(VecDeque::new());
        for i in 1..=25 {
            recent.push(Local::now(), format!("error {i}"));
        }
        assert_eq!(recent.0.len(), 20);
        assert_eq!(recent.0[0].1, "error 6");
        assert_eq!(recent.0[19].1, "error 25");
    }
}
