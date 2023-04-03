use core::fmt;
use std::sync::{Arc, Mutex};

use tuig::{io::fmt::Text, Message};

use crate::{app::App, tools::Tool};

/// Convenience for the things that pass trait objects around, but only one of them.
pub struct Bundle<T>(Arc<Mutex<Option<T>>>);

impl<T> Bundle<T> {
    pub fn of(contents: T) -> Self {
        Self(Arc::new(Mutex::new(Some(contents))))
    }

    pub fn take(&self) -> Option<T> {
        self.0.lock().unwrap().take()
    }
}

impl<T> fmt::Debug for Bundle<T> {
    #[cfg_attr(coverage, no_coverage)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bundle<{}>(..)", std::any::type_name::<T>())
    }
}

impl<T> PartialEq for Bundle<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Eq for Bundle<T> {}

impl<T> Clone for Bundle<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

macro_rules! trait_bundle {
    ( $(
        $fn:ident($trait:ident $($extra:tt)*) => $enum:ident
    ),* $(,)? ) => { $(
        paste::paste! {
            pub type [< Bundled $trait >] = Bundle<Box<dyn $trait $($extra)*>>;
            impl [< Bundled $trait >] {
                #[cfg_attr(coverage, no_coverage)]
                pub fn new(contents: impl $trait $($extra)* + 'static) -> Self {
                    Bundle::of(Box::new(contents))
                }
            }
            impl Event {
                #[cfg_attr(coverage, no_coverage)]
                pub fn $fn(item: impl $trait $($extra)* + 'static) -> Self {
                    Self::$enum([< Bundled $trait >]::new(item))
                }
            }
        }
    )* };
}
trait_bundle! {
    install(Tool) => InstallTool,
    add_tab(App) => AddTab,
}

/// A single thing which has happened, which an [`Agent`] may or may not want to respond to.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Event {
    /// See [`Message::tick`].
    Tick,

    /// Create a new tab on the player's UI
    AddTab(BundledApp),

    /// Something added a command to the player's CLI
    InstallTool(BundledTool),
    /// A line of output from a running command
    CommandOutput(Vec<Text>),
    /// Command has changed the CLI's directory to the given (absolute) one
    ChangeDir(String),
    /// The command that was running is done and the prompt can reappear.
    ///
    /// Note this doesn't kill the agent or stop more output from coming; it just tells the console to display the
    /// prompt for the next command. (This allows commands to run in the 'background'.)
    CommandDone,

    /// The player has sent a chat message to some NPC
    PlayerChatMessage { to: String, text: String },
    /// Some NPC has sent a chat message to the player
    NPCChatMessage {
        from: String,
        text: String,
        options: Vec<String>,
    },
}

impl Event {
    #[cfg_attr(coverage, no_coverage)]
    pub fn output(line: Vec<Text>) -> Self {
        Self::CommandOutput(line)
    }

    #[cfg_attr(coverage, no_coverage)]
    pub fn player_chat(to: &str, text: &str) -> Event {
        Event::PlayerChatMessage {
            to: to.into(),
            text: text.into(),
        }
    }

    #[cfg_attr(coverage, no_coverage)]
    pub fn npc_chat(from: &str, text: &str, options: &[&str]) -> Event {
        Event::NPCChatMessage {
            from: from.into(),
            text: text.into(),
            options: options.iter().map(|&s| s.to_owned()).collect(),
        }
    }

    #[cfg_attr(coverage, no_coverage)]
    pub fn cd(to: &str) -> Event {
        Event::ChangeDir(to.into())
    }
}

impl Message for Event {
    fn tick() -> Self {
        Self::Tick
    }
}

#[cfg(test)]
mod bundle_test {
    use super::*;

    #[test]
    fn item_taken_once() {
        let bundle = Bundle::of(1);
        assert_eq!(bundle.take(), Some(1));
        assert_eq!(bundle.take(), None);
    }

    #[test]
    fn bundle_eq_compares_ptrs() {
        let b1 = Bundle::of(1);
        let b2 = b1.clone();
        assert_eq!(b1, b2);
    }

    #[test]
    fn bundle_eq_doesnt_compare_contents() {
        let b1 = Bundle::of(1);
        let b2 = Bundle::of(1);
        assert_ne!(b1, b2);
    }
}
