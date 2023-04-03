use tuig::{io::text, Agent};

use crate::{app::CliState, event::Event};

use super::{AutocompleteType, FixedOutput, Tool};

/// Implementation of [`Tool`] for the `touch` command, to create an empty file.
pub struct Touch;

impl Tool for Touch {
    fn name(&self) -> &'static str {
        "touch"
    }

    fn autocomplete(&self, line: &str, state: &CliState) -> String {
        if line.ends_with(char::is_whitespace) {
            AutocompleteType::LocalFile.complete("", state)
        } else if let Some(last) = line.rsplit(char::is_whitespace).next() {
            AutocompleteType::LocalFile.complete(last, state)
        } else {
            String::new()
        }
    }

    fn run(&self, line: &str, state: &CliState) -> Box<dyn Agent<Event>> {
        let mut lines = vec![];
        for file in line.split_whitespace() {
            let path = if file.starts_with('/') {
                file.into()
            } else {
                format!("{}{}", state.cwd, file)
            };
            if let Err(e) = state.machine.write(&path, String::new()) {
                lines.push(text![bright_red "ERROR", ": failed to write {}: {}\n"(file, e)]);
            }
        }
        Box::new(FixedOutput(lines))
    }
}
