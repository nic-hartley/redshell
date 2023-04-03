//! Common structural code for apps.

use tuig::{
    io::{Action, Screen},
    Replies,
};

use crate::{event::Event, state::GameState};

/// Each app is a single tab in the game's window view, e.g. chat. They exclusively handle IO: Processing user input
/// and rendering (part of) game state.
///
/// Apps are only given input and rendered when they're on-screen, but they receive all events. Note, though, that
/// events may be batched when the app is offscreen, so that systems and the onscreen app can be updated on time.
pub trait App: Send + Sync + 'static {
    /// The name of this app's tab in the header. (should be constant, hence &'static)
    fn name(&self) -> &'static str;

    /// Take a single input action, returning any new events generated as a result.
    ///
    /// Returns whether this item was tainted, i.e. true if it needs to be redrawn.
    fn input(&mut self, a: Action, events: &mut Replies<Event>) -> bool;
    /// Receive an event, in case the app needs to care to render it.
    ///
    /// `focused` indicates whether this app is the current "foreground" one.
    ///
    /// Returns whether this item was tainted, i.e. true if it needs to be redrawn.
    fn on_event(&mut self, ev: &Event, focused: bool) -> bool;

    /// The number of notifications this app has.
    fn notifs(&self) -> usize;
    /// Display the game state on screen.
    ///
    /// You can be sure that this will never be called except when the module is the active one; feel free to use it
    /// for e.g. clearing notifications.
    fn render(&self, state: &GameState, screen: &mut Screen);
}

/// Assert things about the outcomes of an `App` receiving input
#[allow(unused)]
#[cfg(test)]
macro_rules! assert_input {
    (
        $app:ident .input ( $($arg:expr),* $(,)? )
        $( clean $( @ $clean:ident )? )? $( taints $( @ $taint:ident )? )?,
        $( $test:tt )*
    ) => {
        {
            let mut evs = tuig::Replies::default();
            let taint = $app.input($( $arg ),* , &mut evs);
            $( assert!(!taint, "app tainted unexpectedly"); $( $clean )? )?
            $( assert!(taint, "app didn't taint when expected"); $( $taint )? )?
            assert_input!(@cmp evs._messages() $( $test )*);
        }
    };
    (@cmp $evs:ident == $other:expr) => { assert_eq!($evs, $other) };
    (@cmp $evs:ident != $other:expr) => { assert_ne!($evs, $other) };
    (@cmp $test:expr) => { assert!($test) };
}

mod chat;
pub use chat::ChatApp;
mod cli;
pub use cli::{CliApp, CliState};
