//! Standardized option completers, which are used by a lot of commands.
//!
//! Demonstrationg completions is also pretty hard, because where the cursor is changes things.
//! The format used in this module's documentation is:
//!
//! - `command text|` -> explanation of what happens
//!
//! where `|` is the cursor's location when you press tab.
//! There'll also be an explanation of the command, or it'll be a common \*nix one.

mod gnu;
// pub use gnu::GnuCompleter;
mod bsd;
pub use bsd::BsdArgs;

use crate::app::CliState;

/// Autocomplete a prefix against any iterable of anything that can be `AsRef`'d into a `str`.
///
/// This returns only the completion part, i.e. excluding the prefix.
///
/// Slightly more formally, this filters for items which start with `prefix`, then returns the longest common prefix
/// among those.
pub fn autocomplete(prefix: &str, options: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    let mut completed: Option<String> = None;
    for opt in options.into_iter() {
        let rest = match opt.as_ref().strip_prefix(prefix) {
            Some(r) => r,
            None => continue,
        };
        if let Some(prev) = &mut completed {
            let eq_find = rest
                .chars()
                .zip(prev.chars())
                .enumerate()
                .find(|(_, (o, r))| o != r);
            let eq_idx = eq_find.map(|(i, _)| i).unwrap_or(prev.len());
            prev.truncate(eq_idx);
        } else {
            completed = Some(rest.into());
        }
    }
    completed.unwrap_or(String::new())
}

/// Describes the various things that can be autocompleted. Used to indicate value types in the various autocompleters
/// and holds a little bit of common logic across all of them.
#[derive(Debug)]
pub enum AutocompleteType {
    /// This cannot be autocompleted
    None,
    /// One of a fixed list of choices
    Choices(Vec<String>),
    /// A file on the local machine
    LocalFile,
    /// A known hostname
    Hostname,
    /// A file somewhere on a remote machine, i.e. `hostname:filepath`
    RemoteFile,
    /// The name of another tool
    Tool,
    /// A full other command, e.g. for chaining commands
    Command,
}

impl AutocompleteType {
    /// Shorthand for creating an [`AutocompleteType::Choices`], mainly for testing
    pub fn choices(vals: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        Self::Choices(vals.into_iter().map(|s| s.as_ref().to_owned()).collect())
    }

    /// Attempt to autocomplete, based on the type.
    pub fn complete(&self, prefix: &str, state: &CliState) -> String {
        match self {
            Self::None => String::new(),
            Self::Choices(opts) => autocomplete(prefix, opts),
            Self::LocalFile => {
                let mut path: Vec<_> = prefix.split('/').collect();
                let file = path.pop().expect("split never returns an empty iterator");
                let cmd_dir = path
                    .into_iter()
                    .map(|s| format!("{}/", s))
                    .collect::<String>();
                let prefix = format!("{}{}", state.cwd, cmd_dir);
                let files = match state.machine.readdir(&prefix) {
                    Ok(f) => f,
                    Err(_) => return String::new(),
                };
                autocomplete(
                    file,
                    files.map(|(f, e)| {
                        if e.is_dir() {
                            format!("{}/", f)
                        } else {
                            format!("{}", f)
                        }
                    }),
                )
            }
            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use crate::machine::Machine;

    use super::*;

    #[test]
    fn find_autocomplete_empty_for_empty_iter() {
        let opts: &[&str] = &[];
        assert_eq!(autocomplete("", opts), "");
        assert_eq!(autocomplete("a", opts), "");
    }

    #[test]
    fn find_autocomplete_presents_option() {
        let opts: &[&str] = &["abyss"];
        assert_eq!(autocomplete("", opts), "abyss");
        assert_eq!(autocomplete("a", opts), "byss");
        assert_eq!(autocomplete("aby", opts), "ss");
        assert_eq!(autocomplete("abyss", opts), "");
    }

    #[test]
    fn find_autocomplete_presents_conflicting_options() {
        let opts: &[&str] = &["abyss", "absolute", "gorgonzola"];
        assert_eq!(autocomplete("", opts), "");
        assert_eq!(autocomplete("a", opts), "b");
        assert_eq!(autocomplete("ab", opts), "");
        assert_eq!(autocomplete("aby", opts), "ss");
        assert_eq!(autocomplete("abs", opts), "olute");
        assert_eq!(autocomplete("g", opts), "orgonzola");
    }

    #[test]
    fn none_doesnt_autocomplete() {
        let machine = Machine::default();
        machine
            .write("/moo", "".into())
            .expect("Failed to write test file");
        machine
            .write("/abyss", "".into())
            .expect("Failed to write test file");
        let clis = CliState {
            machine: Arc::new(machine),
            cwd: "/".into(),
        };
        let ac = AutocompleteType::None;
        assert_eq!(ac.complete("", &clis), "");
        assert_eq!(ac.complete("m", &clis), "");
        assert_eq!(ac.complete("mi", &clis), "");
        assert_eq!(ac.complete("mo", &clis), "");
    }

    #[test]
    fn choices_autocompletes_choices() {
        let machine = Machine::default();
        machine
            .write("/moo", "".into())
            .expect("Failed to write test file");
        machine
            .write("/abyss", "".into())
            .expect("Failed to write test file");
        let clis = CliState {
            machine: Arc::new(machine),
            cwd: "/".into(),
        };
        let ac = AutocompleteType::choices(&["mass", "help", "gorgonzola"]);
        assert_eq!(ac.complete("", &clis), "");
        assert_eq!(ac.complete("m", &clis), "ass");
        assert_eq!(ac.complete("ma", &clis), "ss");
        assert_eq!(ac.complete("mi", &clis), "");
        assert_eq!(ac.complete("mo", &clis), "");
        assert_eq!(ac.complete("a", &clis), "");
        assert_eq!(ac.complete("g", &clis), "orgonzola");
    }

    #[test]
    fn local_file_autocompletes_local_files() {
        let machine = Machine::default();
        machine
            .write("/moo", "".into())
            .expect("Failed to write test file");
        machine
            .write("/abyss", "".into())
            .expect("Failed to write test file");
        let clis = CliState {
            machine: Arc::new(machine),
            cwd: "/".into(),
        };
        let ac = AutocompleteType::LocalFile;
        assert_eq!(ac.complete("", &clis), "");
        assert_eq!(ac.complete("m", &clis), "oo");
        assert_eq!(ac.complete("mi", &clis), "");
        assert_eq!(ac.complete("mo", &clis), "o");
        assert_eq!(ac.complete("a", &clis), "byss");
        assert_eq!(ac.complete("abyss", &clis), "");
    }

    #[test]
    fn local_file_autocompletes_local_files_in_cwd() {
        let machine = Machine::default();
        machine
            .write("/moo", "".into())
            .expect("Failed to write test file");
        machine
            .write("/abyss", "".into())
            .expect("Failed to write test file");
        machine
            .mkdir("/stuff/", true)
            .expect("Failed to create test dir");
        machine
            .write("/stuff/bongos", "".into())
            .expect("Failed to write test file");
        machine
            .write("/stuff/michael_hill".into(), "".into())
            .expect("Failed to write test file");
        machine
            .write("/stuff/neil_baum", "".into())
            .expect("Failed to write test file");
        let clis = CliState {
            machine: Arc::new(machine),
            cwd: "/stuff/".into(),
        };
        let ac = AutocompleteType::LocalFile;
        assert_eq!(ac.complete("", &clis), "");
        assert_eq!(ac.complete("m", &clis), "ichael_hill");
        assert_eq!(ac.complete("mi", &clis), "chael_hill");
        assert_eq!(ac.complete("mo", &clis), "");
        assert_eq!(ac.complete("a", &clis), "");
        assert_eq!(ac.complete("neil_bau", &clis), "m");
        assert_eq!(ac.complete("neil_baum", &clis), "");
    }

    #[test]
    fn local_file_autocompletes_directories_nicely() {
        let machine = Machine::default();
        machine
            .write("/moo", "".into())
            .expect("Failed to write test file");
        machine
            .write("/abyss", "".into())
            .expect("Failed to write test file");
        machine
            .mkdir("/stuff/", true)
            .expect("Failed to create test dir");
        machine
            .write("/stuff/bongos", "".into())
            .expect("Failed to write test file");
        let clis = CliState {
            machine: Arc::new(machine),
            cwd: "/".into(),
        };
        let ac = AutocompleteType::LocalFile;
        assert_eq!(ac.complete("", &clis), "");
        assert_eq!(ac.complete("st", &clis), "uff/");
        assert_eq!(ac.complete("stuff", &clis), "/");
        assert_eq!(ac.complete("stuff/", &clis), "bongos");
        assert_eq!(ac.complete("stuff/bo", &clis), "ngos");
    }
}
