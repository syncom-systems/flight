// SPDX-License-Identifier: MIT

use std::io::{BufRead, Write};

/// The person's answer to a question: only an explicit yes is a yes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    Yes,
    No,
}

/// Where setup talks to the person. A trait so tests can script the answers.
pub trait Console {
    fn say(&mut self, text: &str);
    /// Ask a yes/no question; the default, an empty answer and end of input are all no.
    fn confirm(&mut self, question: &str) -> Answer;
    /// Ask for a line of free text (`None` at end of input).
    fn line(&mut self, question: &str) -> Option<String>;
}

/// Standard input and output.
pub struct TerminalConsole;

impl TerminalConsole {
    fn read(&mut self, question: &str) -> Option<String> {
        print!("{question} ");
        let _ = std::io::stdout().flush();
        let mut buf = String::new();
        match std::io::stdin().lock().read_line(&mut buf) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(buf.trim().to_owned()),
        }
    }
}

impl Console for TerminalConsole {
    fn say(&mut self, text: &str) {
        println!("{text}");
    }

    fn confirm(&mut self, question: &str) -> Answer {
        match self.read(&format!("{question} [y/N]")) {
            Some(a) if a.eq_ignore_ascii_case("y") || a.eq_ignore_ascii_case("yes") => Answer::Yes,
            _ => Answer::No,
        }
    }

    fn line(&mut self, question: &str) -> Option<String> {
        self.read(question)
    }
}
