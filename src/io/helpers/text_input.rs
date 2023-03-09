use std::{collections::VecDeque, mem};

use crate::{
    io::{
        clifmt::Text,
        input::{Action, Key},
    },
    text,
};

use super::ModState;

/// Indicates what the text input needs from its owner
#[derive(Debug, PartialEq, Eq)]
pub enum TextInputRequest {
    /// Action doesn't require any response.
    Nothing,
    /// Autocomplete has been requested, with the given text.
    Autocomplete,
    /// User has submitted a line by pressing Enter.
    Line(String),
}

/// Allows the user to input text, rendering it to a bounded area and offering hooks for tab-based autocomplete.
#[derive(Clone, Default)]
pub struct TextInput {
    /// prompt displayed before the user's text
    prompt: String,

    /// line(s) currently being typed
    line: String,
    /// cursor position in the line being typed
    cursor: usize,
    /// any autocomplete that's been requested
    autocomplete: String,

    /// whether the textbox needs to be redrawn since it was last rendered
    tainted: bool,

    /// the current state of the keyboard modifiers
    modstate: ModState,

    /// previous lines entered, for scrolling through
    history: VecDeque<String>,
    /// where in the history we are. (history.len() = next line.)
    hist_pos: usize,
    /// the number of history items to store
    hist_cap: usize,
}

impl TextInput {
    /// Create a new text input element.
    #[cfg_attr(coverage, no_coverage)]
    pub fn new(prompt: &str, history: usize) -> Self {
        Self {
            prompt: prompt.into(),
            line: String::new(),
            cursor: 0,
            autocomplete: String::new(),
            tainted: true,
            modstate: Default::default(),
            history: Default::default(),
            hist_pos: 0,
            hist_cap: history,
        }
    }

    pub fn completable(&self) -> &str {
        &self.line[..self.cursor]
    }

    pub fn set_complete(&mut self, text: String) {
        self.autocomplete = text;
        self.tainted = true;
    }

    /// Whether this text box needs to be redrawn.
    pub fn tainted(&self) -> bool {
        self.tainted
    }

    fn cur_line(&self) -> &str {
        if self.hist_pos < self.history.len() {
            &self.history[self.hist_pos]
        } else {
            &self.line
        }
    }

    fn pick_hist(&mut self) {
        if self.hist_pos == self.history.len() {
            return;
        }
        self.line = self.history[self.hist_pos].clone();
        self.hist_pos = self.history.len();
    }

    fn keypress(&mut self, key: Key) -> TextInputRequest {
        match key {
            Key::Char(ch) if !self.modstate.hotkeying() => {
                self.pick_hist();
                let chs: String = if self.modstate.shift {
                    ch.to_uppercase().collect()
                } else {
                    ch.to_lowercase().collect()
                };
                self.line.insert_str(self.cursor, &chs);
                self.cursor += 1;
            }
            Key::Backspace if self.cursor > 0 => {
                self.pick_hist();
                self.line.remove(self.cursor - 1);
                self.cursor -= 1;
            }
            Key::Delete if self.cursor < self.cur_line().len() => {
                self.pick_hist();
                self.line.remove(self.cursor);
            }
            Key::Left if self.cursor > 0 => self.cursor -= 1,
            Key::Right if self.cursor < self.cur_line().len() => self.cursor += 1,
            Key::Up if self.hist_pos > 0 => {
                self.hist_pos -= 1;
                self.cursor = self.cur_line().len();
            }
            Key::Down if self.hist_pos < self.history.len() => {
                self.hist_pos += 1;
                self.cursor = self.cur_line().len();
            }
            Key::Tab => {
                if self.autocomplete.is_empty() {
                    return TextInputRequest::Autocomplete;
                } else {
                    self.pick_hist();
                    self.line.insert_str(self.cursor, &self.autocomplete);
                    self.cursor += self.autocomplete.len();
                }
            }
            Key::Enter => {
                self.pick_hist();
                self.cursor = 0;
                self.autocomplete = String::new();
                let old_line = mem::replace(&mut self.line, String::new());
                self.tainted = true;
                if !old_line.trim().is_empty() && self.hist_cap > 0 {
                    if self.history.len() == self.hist_cap {
                        self.history.pop_front();
                    }
                    self.history.push_back(old_line.clone());
                }
                self.hist_pos = self.history.len();
                return TextInputRequest::Line(old_line);
            }
            // return early to skip tainting / clearing autocomplete
            _ => return TextInputRequest::Nothing,
        }
        self.autocomplete = String::new();
        self.tainted = true;
        TextInputRequest::Nothing
    }

    /// Handles an [`Action`] which should go to the textbox, for things like typing and autocompletion.
    ///
    /// The type this returns indicates what needs to be done
    pub fn action(&mut self, action: Action) -> TextInputRequest {
        match action {
            act if self.modstate.action(&act) => TextInputRequest::Nothing,
            Action::KeyPress { key } => self.keypress(key),
            _ => TextInputRequest::Nothing,
        }
    }

    /// Builds a `Vec<Text>` with the prompt line, for rendering.
    pub fn render<'s>(&mut self) -> Vec<Text> {
        self.tainted = false;
        let line = self.cur_line();
        if self.cursor == line.len() {
            if self.autocomplete.is_empty() {
                text![
                    "{}"(self.prompt),
                    bright_white "{}"(line),
                    underline " ",
                ]
            } else {
                text![
                    "{}"(self.prompt),
                    bright_white "{}"(line),
                    bright_black underline "{}"(&self.autocomplete[..1]),
                    bright_black "{}"(&self.autocomplete[1..]),
                ]
            }
        } else {
            if self.autocomplete.is_empty() {
                text![
                    "{}"(self.prompt),
                    bright_white "{}"(&line[..self.cursor]),
                    bright_white underline "{}"(&line[self.cursor..self.cursor+1]),
                    bright_white "{}"(&line[self.cursor+1..]),
                ]
            } else {
                text![
                    "{}"(self.prompt),
                    bright_white "{}"(&line[..self.cursor]),
                    bright_black underline "{}"(&self.autocomplete[..1]),
                    bright_black "{}"(&self.autocomplete[1..]),
                    bright_white "{}"(&line[self.cursor..]),
                ]
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Feed a series of inputs to a TextInput, asserting that it returns `TextInputRequest::Nothing`, and maybe
    /// whether it's tainted afterwards.
    macro_rules! feed {
        ( $ti:ident: $(
            $key:ident $( ( $content:expr ) )? $( $side:ident )?
        ),* $( => $( tainted $( @@ $taint_yes:ident )? )? $( untainted $( @@ $taint_no:ident )? )? )? ) => {
            $(
                assert_eq!(feed!(@@one $ti: $key $( ($content) )?), TextInputRequest::Nothing);
            )*
            $(
                $(
                    assert!($ti.tainted(), "text input not tainted after render-relevant inputs");
                    $( $taint_yes )?
                )?
                $(
                    assert!($ti.tainted(), "text input tainted after render-irrelevant inputs");
                    $( $taint_no )?
                )?
            )?
        };
        ( @@one $ti:ident: String($val:literal) ) => {
            {
                let mut res = None;
                let mut cap = false;
                for ch in $val.chars() {
                    let new_cap = ch.is_uppercase();
                    if new_cap != cap {
                        if new_cap {
                            assert_eq!(feed!(@@one $ti: LeftShift KeyPress), TextInputRequest::Nothing);
                        } else {
                            assert_eq!(feed!(@@one $ti: LeftShift KeyRelease), TextInputRequest::Nothing);
                        }
                        cap = new_cap;
                    }
                    let cur = feed!(@@one $ti: Char(ch));
                    if let Some(prev) = &res {
                        assert_eq!(&cur, prev, "different results across fed String");
                    } else {
                        res = Some(cur);
                    }
                }
                if cap {
                    assert_eq!(feed!(@@one $ti: LeftShift KeyRelease), TextInputRequest::Nothing);
                }
                res.expect("test is broken: tried to feed empty string")
            }
        };
        ( @@one $ti:ident: String($_:tt) $side:ident ) => {
            compile_error!("Cannot KeyPress/KeyRelease a String");
        };
        ( @@one $ti:ident: $key:ident $( ($content:expr) )? ) => {
            {
                let res1 = feed!(@@one $ti: $key $( ( $content ) )? KeyPress );
                let res2 = feed!(@@one $ti: $key $( ( $content ) )? KeyRelease );
                assert_eq!(res1, res2, "press/release fed char differed");
                res1
            }
        };
        ( @@one $ti:ident: $key:ident $( ($content:expr) )? $side:ident ) => {
            $ti.action(Action::$side { key: Key::$key $( ($content) )? })
        };
    }

    #[coverage_helper::test]
    fn blank_renders_to_prompt() {
        let mut ti = TextInput::new("> ", 0);
        assert!(ti.tainted(), "not tainted to force initial draw");
        assert_eq!(ti.render(), text!["> ", bright_white "", underline " "]);
        assert!(!ti.tainted(), "render doesn't untaint");
    }

    #[coverage_helper::test]
    fn text_renders_to_prompt() {
        let mut ti = TextInput::new("> ", 0);
        feed!(ti: String("abcdef") => tainted);
        assert_eq!(
            ti.render(),
            text!["> ", bright_white "abcdef", underline " "]
        );
    }

    #[coverage_helper::test]
    fn blank_renders_to_prompt_with_autocomplete() {
        let mut ti = TextInput::new("> ", 0);
        ti.set_complete("mlem".into());
        assert_eq!(
            ti.render(),
            text!["> ", bright_white "", underline bright_black "m", bright_black "lem"]
        );
        assert!(!ti.tainted(), "render doesn't untaint");
    }

    #[coverage_helper::test]
    fn text_renders_to_prompt_with_autocomplete() {
        let mut ti = TextInput::new("> ", 0);
        feed!(ti: String("abcdef") => tainted);
        ti.set_complete("mlem".into());
        assert!(ti.tainted(), "not tainted after visually important changes");
        assert_eq!(
            ti.render(),
            text!["> ", bright_white "abcdef", bright_black underline "m", bright_black "lem"]
        );
    }

    #[coverage_helper::test]
    fn text_renders_to_prompt_moved_cursor() {
        let mut ti = TextInput::new("> ", 0);
        feed!(ti: String("abcdef"), Left, Left => tainted);
        assert_eq!(
            ti.render(),
            text!["> ", bright_white "abcd", underline bright_white "e", bright_white "f"]
        );
    }

    #[coverage_helper::test]
    fn text_renders_to_prompt_with_autocomplete_moved_cursor() {
        let mut ti = TextInput::new("> ", 0);
        feed!(ti: String("abcdef"), Left, Left, Left, Right => tainted);
        ti.set_complete("mlem".into());
        assert!(ti.tainted(), "not tainted after visually important changes");
        assert_eq!(
            ti.render(),
            text!["> ", bright_white "abcd", underline bright_black "m", bright_black "lem", bright_white "ef"]
        );
    }

    #[coverage_helper::test]
    fn typing_uppercase() {
        let mut ti = TextInput::new("> ", 0);
        feed!(ti: String("abCDef") => tainted);
        assert_eq!(
            ti.render(),
            text!["> ", bright_white "abCDef", underline " "]
        );
    }

    #[coverage_helper::test]
    fn backspacing_chars() {
        let mut ti = TextInput::new("> ", 0);
        feed!(ti: String("abcd"), Backspace, Backspace, String("ef"));
        assert_eq!(ti.render(), text!["> ", bright_white "abef", underline " "]);
    }

    #[coverage_helper::test]
    fn deleting_chars() {
        let mut ti = TextInput::new("> ", 0);
        feed!(ti: String("abcd"), Left, Left, Delete, Delete, String("ef"));
        assert_eq!(ti.render(), text!["> ", bright_white "abef", underline " "]);
    }

    #[coverage_helper::test]
    fn tab_triggers_autocomplete() {
        let mut ti = TextInput::new("> ", 0);
        feed!(ti: String("abc"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Tab }),
            TextInputRequest::Autocomplete
        );
        assert_eq!(ti.completable(), "abc");
        ti.set_complete("mlem".into());
        assert_eq!(
            ti.render(),
            text!["> ", bright_white "abc", underline bright_black "m", bright_black "lem"]
        );
    }

    #[coverage_helper::test]
    fn tab_twice_finishes_autocomplete() {
        let mut ti = TextInput::new("> ", 0);
        feed!(ti: String("abc"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Tab }),
            TextInputRequest::Autocomplete
        );
        assert_eq!(ti.completable(), "abc");
        ti.set_complete("mlem".into());
        feed!(ti: Tab);
        assert_eq!(
            ti.render(),
            text!["> ", bright_white "abcmlem", underline " "]
        );
    }

    #[coverage_helper::test]
    fn enter_sends_line() {
        let mut ti = TextInput::new("> ", 0);
        feed!(ti: String("abc"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Enter }),
            TextInputRequest::Line("abc".into())
        );
        assert_eq!(ti.render(), text!["> ", bright_white "", underline " "]);
    }

    #[coverage_helper::test]
    fn history_scrolls_with_arrows() {
        let mut ti = TextInput::new("> ", 10);
        feed!(ti: String("abc"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Enter }),
            TextInputRequest::Line("abc".into())
        );
        feed!(ti: String("def"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Enter }),
            TextInputRequest::Line("def".into())
        );
        feed!(ti: String("ghi"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Enter }),
            TextInputRequest::Line("ghi".into())
        );
        assert_eq!(ti.render(), text!["> ", bright_white "", underline " "]);
        feed!(ti: Up, Up, Up => tainted);
        assert_eq!(ti.render(), text!["> ", bright_white "abc", underline " "]);
        feed!(ti: Down => tainted);
        assert_eq!(ti.render(), text!["> ", bright_white "def", underline " "]);
    }

    #[coverage_helper::test]
    fn history_scroll_to_bottom_doesnt_reset_line() {
        let mut ti = TextInput::new("> ", 10);
        feed!(ti: String("abc"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Enter }),
            TextInputRequest::Line("abc".into())
        );
        feed!(ti: String("def"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Enter }),
            TextInputRequest::Line("def".into())
        );
        feed!(ti: String("ghi"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Enter }),
            TextInputRequest::Line("ghi".into())
        );
        feed!(ti: String("jkl"));
        assert_eq!(ti.render(), text!["> ", bright_white "jkl", underline " "]);
        feed!(ti: Up => tainted);
        assert_eq!(ti.render(), text!["> ", bright_white "ghi", underline " "]);
        feed!(ti: Down => tainted);
        assert_eq!(ti.render(), text!["> ", bright_white "jkl", underline " "]);
    }

    #[coverage_helper::test]
    fn history_selects_with_typing() {
        let mut ti = TextInput::new("> ", 10);
        feed!(ti: String("abc"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Enter }),
            TextInputRequest::Line("abc".into())
        );
        feed!(ti: String("def"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Enter }),
            TextInputRequest::Line("def".into())
        );
        feed!(ti: String("ghi"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Enter }),
            TextInputRequest::Line("ghi".into())
        );
        feed!(ti: Up);
        assert_eq!(ti.render(), text!["> ", bright_white "ghi", underline " "]);
        feed!(ti: Char('j') => tainted);
        assert_eq!(ti.render(), text!["> ", bright_white "ghij", underline " "]);
        feed!(ti: Up => tainted);
        assert_eq!(ti.render(), text!["> ", bright_white "ghi", underline " "]);
        feed!(ti: Up => tainted);
        assert_eq!(ti.render(), text!["> ", bright_white "def", underline " "]);
    }

    #[coverage_helper::test]
    fn history_selects_with_backspace() {
        let mut ti = TextInput::new("> ", 10);
        feed!(ti: String("abc"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Enter }),
            TextInputRequest::Line("abc".into())
        );
        feed!(ti: String("def"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Enter }),
            TextInputRequest::Line("def".into())
        );
        feed!(ti: String("ghi"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Enter }),
            TextInputRequest::Line("ghi".into())
        );
        feed!(ti: Up);
        assert_eq!(ti.render(), text!["> ", bright_white "ghi", underline " "]);
        feed!(ti: Backspace => tainted);
        assert_eq!(ti.render(), text!["> ", bright_white "gh", underline " "]);
        feed!(ti: Up => tainted);
        assert_eq!(ti.render(), text!["> ", bright_white "ghi", underline " "]);
        feed!(ti: Up => tainted);
        assert_eq!(ti.render(), text!["> ", bright_white "def", underline " "]);
    }

    #[coverage_helper::test]
    fn history_selects_with_enter() {
        let mut ti = TextInput::new("> ", 10);
        feed!(ti: String("abc"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Enter }),
            TextInputRequest::Line("abc".into())
        );
        feed!(ti: String("def"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Enter }),
            TextInputRequest::Line("def".into())
        );
        feed!(ti: String("ghi"));
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Enter }),
            TextInputRequest::Line("ghi".into())
        );
        feed!(ti: Up);
        assert_eq!(ti.render(), text!["> ", bright_white "ghi", underline " "]);
        assert_eq!(
            ti.action(Action::KeyPress { key: Key::Enter }),
            TextInputRequest::Line("ghi".into())
        );
        assert_eq!(ti.render(), text!["> ", bright_white "", underline " "]);
        feed!(ti: Up => tainted);
        assert_eq!(ti.render(), text!["> ", bright_white "ghi", underline " "]);
        feed!(ti: Up => tainted);
        assert_eq!(ti.render(), text!["> ", bright_white "ghi", underline " "]);
        feed!(ti: Up => tainted);
        assert_eq!(ti.render(), text!["> ", bright_white "def", underline " "]);
    }
}