use tuig::{
    io::{
        fmt::{FormattedExt, Text},
        text, text1,
    },
    Agent,
};

use crate::{app::CliState, event::Event, machine::Entry};

use super::{AutocompleteType, BsdArgs, FixedOutput, Tool};

lazy_static::lazy_static! {
    /// Completes `ls` arguments
    static ref COMPLETER: BsdArgs = BsdArgs::new()
        .flag('l').argument('d', AutocompleteType::LocalFile);
}

/// The entries in the directory, on the state's [`Machine`]
fn entries<'cs>(dir: &str, state: &'cs CliState) -> Result<Vec<(String, Entry)>, String> {
    let prefix = if dir.is_empty() {
        format!("{}", state.cwd)
    } else if dir.ends_with('/') {
        format!("{}{}", state.cwd, dir)
    } else {
        format!("{}{}/", state.cwd, dir)
    };
    let entries = match state.machine.readdir(&prefix) {
        Ok(e) => e,
        Err(e) => return Err(e),
    };
    let mut entries: Vec<_> = entries.collect();
    // instability is fine: keys are unique anyway
    entries.sort_unstable_by(|l, r| l.0.cmp(&r.0));
    Ok(entries)
}

/// Produces a short listing of the entries, with just their names, color-coded by type
///
/// See [`Self::entries`].
fn list_short(entries: Vec<(String, Entry)>) -> Vec<Vec<Text>> {
    let mut line: Vec<_> = entries
        .into_iter()
        .map(|(name, entry)| {
            let text = if name.chars().any(char::is_whitespace) {
                text1!["'{}' "(name)]
            } else {
                text1![" {}  "(name)]
            };
            if entry.is_dir() {
                text.cyan().bold()
            } else {
                text
            }
        })
        .collect();
    if !line.is_empty() {
        line.push(text1!["\n"]);
    }
    vec![line]
}

/// Produces a long listing of the entries, with names, sizes, etc. and color-coded by type.
///
/// See [`Self::entries`].
fn list_long(entries: Vec<(String, Entry)>) -> Vec<Vec<Text>> {
    vec![text!["total {}\n"(entries.len())]]
        .into_iter()
        .chain(entries.into_iter().map(|(name, entry)| {
            let mut res = vec![Text::plain(""); 3];
            res[0] = match &entry {
                Entry::File(f) => text1!["{} "(f.contents.len())],
                Entry::Directory(d) => text1!["{} "(d.len())],
            };
            res[1] = if entry.is_dir() {
                text1![cyan bold "{}"(name)]
            } else {
                text1!["{}"(name)]
            };
            res[2] = text1!["\n"];
            res
        }))
        .collect()
}

/// Implementation of [`Tool`] for the `ls` command, to list the current or specified directory.
pub struct Ls;

impl Tool for Ls {
    fn name(&self) -> &'static str {
        "ls"
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
        let dir = args
            .get(&'d')
            .unwrap_or(&Some(""))
            .expect("None despite arg having value");
        let entries = match entries(dir, state) {
            Ok(e) => e,
            Err(e) => {
                return Box::new(FixedOutput(vec![text![bright_red "ERROR", ": {}\n"(e)]]));
            }
        };
        let rows = if args.get(&'l').is_some() {
            list_long(entries)
        } else {
            list_short(entries)
        };
        Box::new(FixedOutput(rows))
    }
}
