//! Test double for collectors: answers commands from a script and records every call.

use crate::ports::{CommandOutput, CommandRunner};
use std::collections::HashMap;
use std::io;
use std::sync::Mutex;

#[derive(Default)]
pub struct Scripted {
    answers: Mutex<HashMap<String, CommandOutput>>,
    calls: Mutex<Vec<String>>,
}

impl Scripted {
    /// `command` is the full command line, e.g. `docker version --format json`.
    pub fn answer(self, command: &str, status: i32, stdout: &str, stderr: &str) -> Self {
        self.answers.lock().unwrap().insert(
            command.to_owned(),
            CommandOutput {
                status,
                stdout: stdout.to_owned(),
                stderr: stderr.to_owned(),
            },
        );
        self
    }

    pub fn ok(self, command: &str, stdout: &str) -> Self {
        self.answer(command, 0, stdout, "")
    }

    /// Replaces the answer of a command already scripted (state changed in between).
    pub fn set(&self, command: &str, stdout: &str) {
        self.answers.lock().unwrap().insert(
            command.to_owned(),
            CommandOutput {
                status: 0,
                stdout: stdout.to_owned(),
                stderr: String::new(),
            },
        );
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl CommandRunner for Scripted {
    fn run(&self, program: &str, args: &[&str]) -> io::Result<CommandOutput> {
        let line = std::iter::once(program)
            .chain(args.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        self.calls.lock().unwrap().push(line.clone());
        self.answers
            .lock()
            .unwrap()
            .get(&line)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("not scripted: {line}")))
    }
}
