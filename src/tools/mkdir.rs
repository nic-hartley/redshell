use tuig::{io::text, Agent};

use crate::{app::CliState, event::Event};

use super::{AutocompleteType, BsdArgs, FixedOutput, NoOutput, Tool};

lazy_static::lazy_static! {
    static ref COMPLETER: BsdArgs = BsdArgs::new()
        .flag('p').argument('d', AutocompleteType::LocalFile);
}

/// Implementation of [`Tool`] for the `mkdir` command, to create an empty directory.
pub struct Mkdir;

impl Tool for Mkdir {
    fn name(&self) -> &'static str {
        "mkdir"
    }

    fn autocomplete(&self, line: &str, state: &CliState) -> String {
        COMPLETER.complete(line, state)
    }

    fn run(&self, line: &str, state: &CliState) -> Box<dyn Agent<Event>> {
        let args = match COMPLETER.parse(line) {
            Ok(v) => v,
            Err(msg) => {
                return Box::new(FixedOutput(vec![text![bright_red "ERROR", ": {}\n"(msg)]]))
            }
        };
        let with_parents = args.contains_key(&'p');
        let file = match args.get(&'d') {
            Some(path) => path.expect("no value to option with value"),
            None => {
                return Box::new(FixedOutput(vec![
                    text![bright_red "ERROR", ": provide a directory to make\n"],
                ]))
            }
        };
        let path = if file.starts_with('/') {
            file.into()
        } else {
            format!("{}{}", state.cwd, file)
        };
        if let Err(e) = state.machine.mkdir(&path, with_parents) {
            Box::new(FixedOutput(vec![
                text![bright_red "ERROR", ": failed to write {}: {}\n"(file, e)],
            ]))
        } else {
            Box::new(NoOutput)
        }
    }
}
