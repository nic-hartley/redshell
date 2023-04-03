use std::mem;

use tuig::{io::text, Agent, ControlFlow, Replies};

use crate::{app::CliState, event::Event};

use super::{AutocompleteType, FixedOutput, Tool};

/// The agent which actually does the changing of directories
struct CdAgent(String);
impl Agent<Event> for CdAgent {
    fn start(&mut self, replies: &mut Replies<Event>) -> ControlFlow {
        replies.queue(Event::ChangeDir(mem::take(&mut self.0)));
        replies.queue(Event::CommandDone);
        ControlFlow::Kill
    }
}

/// Implementation of [`Tool`] for the `cd` command, to change the current working directory
pub struct Cd;

impl Tool for Cd {
    fn name(&self) -> &'static str {
        "cd"
    }

    fn autocomplete(&self, line: &str, state: &CliState) -> String {
        AutocompleteType::LocalFile.complete(line, state)
    }

    fn run(&self, line: &str, state: &CliState) -> Box<dyn Agent<Event>> {
        let line = line.trim();
        let mut target_comps = if line.starts_with('/') {
            vec![]
        } else {
            state.cwd.split('/').filter(|s| !s.is_empty()).collect()
        };
        for component in line.split('/') {
            if component.is_empty() || component == "." {
                continue;
            } else if component == ".." {
                target_comps.pop();
            } else {
                target_comps.push(component);
            }
        }
        let res = if target_comps.is_empty() {
            "/".into()
        } else {
            format!("/{}/", target_comps.join("/"))
        };
        match state.machine.readdir(&res) {
            Ok(_) => Box::new(CdAgent(res)),
            Err(e) => Box::new(FixedOutput(vec![text![bright_red "ERROR", ": {}\n"(e)]])),
        }
    }
}
