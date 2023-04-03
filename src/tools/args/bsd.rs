use std::{collections::HashMap, iter::repeat};

use crate::app::CliState;

use super::AutocompleteType;

/// Allows for easy completion and parsing of BSD-style command options, e.g.:
///
/// ## Completion
///
/// - `tar |`: tries to complete x, f, etc. (the various option characters)
/// - `tar x|`: the same thing
/// - `tar x |`: doesn't try to complete anything; as far as it cares the options are done
/// - `tar xf |`: tries to autocomplete a filename
/// - `tar xf abc|`: tries to autocomplete a filename starting with `abc`
///
/// ## Parsing
///
/// - `tar`: `{}`
/// - `tar x`: `{ 'x': None }`
/// - `tar xf filename`: `{ 'x': None, f: Some("filename") }`
/// - `tar f abc`: `{ 'f': Some("abc") }`
pub struct BsdArgs {
    /// The options this completer can complete
    options: HashMap<char, Option<AutocompleteType>>,
}

impl BsdArgs {
    /// Create a new BsdCompleter with no options
    pub fn new() -> Self {
        Self {
            options: Default::default(),
        }
    }

    /// Add a valueless option. This will appear in the option string
    pub fn flag(mut self, ch: char) -> Self {
        self.options.insert(ch, None);
        self
    }

    /// Add an option which takes a value
    pub fn argument(mut self, ch: char, kind: AutocompleteType) -> Self {
        self.options.insert(ch, Some(kind));
        self
    }

    /// Perform autocompletion
    pub fn complete(&self, line: &str, state: &CliState) -> String {
        if let Some((opts, rest)) = line.split_once(' ') {
            // autocomplete arguments
            let mut kinds = opts
                .chars()
                .filter_map(|c| self.options.get(&c).map(|o| o.as_ref()).flatten())
                .chain(repeat(&AutocompleteType::None));
            let vals = rest.trim_start().split_whitespace();
            if rest.ends_with(char::is_whitespace) || rest.is_empty() {
                // correct for split_whitespace trimming the end
                for _ in 0..vals.count() {
                    kinds.next();
                }
                let last_kind = kinds
                    .next()
                    .expect("iter chained with infinite repeat ran out");
                last_kind.complete("", state)
            } else {
                match vals.zip(&mut kinds).last() {
                    Some((last_val, last_kind)) => last_kind.complete(last_val, state),
                    None => String::new(),
                }
            }
        } else {
            // autocomplete remaining options
            let mut remaining = self.options.keys().filter(|&&c| !line.contains(c)).fuse();
            let maybe_last = remaining.next();
            let maybe_after = remaining.next();
            match (maybe_last, maybe_after) {
                // there's exactly one remaining option
                (Some(last), None) => String::from(*last),
                // there's more than one, we can't complete
                (Some(_), Some(_)) => String::new(),
                // there's none left, we can't complete
                (None, None) => String::new(),
                // the iterator is fused, it shouldn't ever do this
                (None, Some(_)) => unreachable!(),
            }
        }
    }

    /// Parse a line for options
    pub fn parse<'l>(&self, line: &'l str) -> Result<HashMap<char, Option<&'l str>>, String> {
        let (opts, rest) = line.split_once(' ').unwrap_or((line, ""));
        let mut args = rest.split_whitespace();
        let mut res = HashMap::with_capacity(opts.len());
        for opt in opts.chars() {
            match self.options.get(&opt) {
                None => Err(format!("Unknown option: {}", opt))?,
                Some(Some(_)) => {
                    // real argument, takes a value
                    match args.next() {
                        Some(v) => {
                            res.insert(opt, Some(v));
                        }
                        None => Err(format!("Option requires argument: {}", opt))?,
                    }
                }
                Some(None) => {
                    // real argument, takes no value
                    res.insert(opt, None);
                }
            }
        }
        Ok(res)
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use crate::machine::Machine;

    use super::*;

    #[test]
    fn empty_completer_doesnt_complete_no_options() {
        let completer = BsdArgs::new();
        let machine = Machine::default();
        machine
            .write("/moo", "".into())
            .expect("Failed to write test file");
        machine
            .write("/maggot", "".into())
            .expect("Failed to write test file");
        machine
            .write("/abyss", "".into())
            .expect("Failed to write test file");
        let clis = CliState {
            machine: Arc::new(machine),
            cwd: "/".into(),
        };
        assert_eq!(completer.complete("", &clis), "");
        assert_eq!(completer.complete("m", &clis), "");
        assert_eq!(completer.complete("f", &clis), "");
        assert_eq!(completer.complete("f m", &clis), "");
        assert_eq!(completer.complete("z", &clis), "");
        assert_eq!(completer.complete("z c", &clis), "");
        assert_eq!(completer.complete("vzf", &clis), "");
    }

    #[test]
    fn bsd_completer_completes_options() {
        let completer = BsdArgs::new()
            .flag('v')
            .flag('q')
            .argument('z', AutocompleteType::choices(["compress", "decompress"]))
            .argument('f', AutocompleteType::LocalFile);
        let machine = Machine::default();
        machine
            .write("/moo", "".into())
            .expect("Failed to write test file");
        machine
            .write("/maggot", "".into())
            .expect("Failed to write test file");
        machine
            .write("/abyss", "".into())
            .expect("Failed to write test file");
        let clis = CliState {
            machine: Arc::new(machine),
            cwd: "/".into(),
        };
        assert_eq!(completer.complete("", &clis), "");
        assert_eq!(completer.complete("v", &clis), "");
        assert_eq!(completer.complete("vf", &clis), "");
        assert_eq!(completer.complete("vfz", &clis), "q");
        assert_eq!(completer.complete("vfq", &clis), "z");
        assert_eq!(completer.complete("qvz", &clis), "f");
    }

    #[test]
    fn bsd_completer_completes_values() {
        let completer = BsdArgs::new()
            .flag('v')
            .flag('q')
            .argument('z', AutocompleteType::choices(["compress", "decompress"]))
            .argument('f', AutocompleteType::LocalFile);
        let machine = Machine::default();
        machine
            .write("/moo", "".into())
            .expect("Failed to write test file");
        machine
            .write("/maggot", "".into())
            .expect("Failed to write test file");
        machine
            .write("/abyss", "".into())
            .expect("Failed to write test file");
        let clis = CliState {
            machine: Arc::new(machine),
            cwd: "/".into(),
        };
        assert_eq!(completer.complete("", &clis), "");
        assert_eq!(completer.complete("f", &clis), "");
        assert_eq!(completer.complete("f ", &clis), "");
        assert_eq!(completer.complete("f a", &clis), "byss");
        assert_eq!(completer.complete("f ma", &clis), "ggot");
        assert_eq!(completer.complete("z", &clis), "");
        assert_eq!(completer.complete("z ", &clis), "");
        assert_eq!(completer.complete("z d", &clis), "ecompress");
        assert_eq!(completer.complete("z comp", &clis), "ress");
    }

    #[test]
    fn bsd_completer_skips_flag_values() {
        let completer = BsdArgs::new()
            .flag('v')
            .flag('q')
            .argument('z', AutocompleteType::choices(["compress", "decompress"]))
            .argument('f', AutocompleteType::LocalFile);
        let machine = Machine::default();
        machine
            .write("/moo", "".into())
            .expect("Failed to write test file");
        machine
            .write("/maggot", "".into())
            .expect("Failed to write test file");
        machine
            .write("/abyss", "".into())
            .expect("Failed to write test file");
        let clis = CliState {
            machine: Arc::new(machine),
            cwd: "/".into(),
        };
        assert_eq!(completer.complete("qf", &clis), "");
        assert_eq!(completer.complete("fv ", &clis), "");
        assert_eq!(completer.complete("vf a", &clis), "byss");
        assert_eq!(completer.complete("fq ma", &clis), "ggot");
        assert_eq!(completer.complete("vqz ", &clis), "");
        assert_eq!(completer.complete("zqv d", &clis), "ecompress");
        assert_eq!(completer.complete("qvz comp", &clis), "ress");
    }

    #[test]
    fn bsd_completer_respects_cwd() {
        let completer = BsdArgs::new()
            .flag('v')
            .flag('q')
            .argument('z', AutocompleteType::choices(["compress", "decompress"]))
            .argument('f', AutocompleteType::LocalFile);
        let machine = Machine::default();
        machine
            .write("/moo", "".into())
            .expect("Failed to write test file");
        machine
            .write("/maggot", "".into())
            .expect("Failed to write test file");
        machine
            .write("/abyss", "".into())
            .expect("Failed to write test file");
        machine
            .mkdir("/stuff/", true)
            .expect("Failed to create test dir");
        machine
            .write("/stuff/violin", "".into())
            .expect("Failed to write test file");
        machine
            .write("/stuff/cello", "".into())
            .expect("Failed to write test file");
        machine
            .write("/stuff/flute", "".into())
            .expect("Failed to write test file");
        let clis = CliState {
            machine: Arc::new(machine),
            cwd: "/stuff/".into(),
        };
        assert_eq!(completer.complete("qf", &clis), "");
        assert_eq!(completer.complete("fv ", &clis), "");
        assert_eq!(completer.complete("vf a", &clis), "");
        assert_eq!(completer.complete("fq v", &clis), "iolin");
        assert_eq!(completer.complete("vqz ", &clis), "");
        assert_eq!(completer.complete("zqv d", &clis), "ecompress");
        assert_eq!(completer.complete("qvz comp", &clis), "ress");
    }
}
