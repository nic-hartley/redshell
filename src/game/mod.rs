//! Contains the [`tuig::Game`] implementation and "main function" for the game itself

use std::time::Duration;

use crate::{
    app::{App, ChatApp, CliApp},
    event::Event,
    state::GameState,
};

use tuig::{
    io::{Action, Key, Screen},
    Agent, ControlFlow, Game, Replies, Response, Runner,
};

/// A single step in the conversation tree of an [`NPC`]
struct ChatState {
    messages: Vec<(String, usize)>,
    options: Vec<(String, usize)>,
}

/// Extremely temporary NPC implementation. Very simplistic, can only do basic conversation trees.
#[derive(Default)]
struct NPC {
    /// The name of the NPC
    name: String,
    /// All of the states it could possibly be in
    all_states: Vec<ChatState>,
    /// Which state it's currently in
    state: usize,
    /// Which message in the state it's currently in
    message: usize,
}

impl NPC {
    /// Get the current state
    fn state(&self) -> &ChatState {
        &self.all_states[*&self.state]
    }

    /// Get the current message/delay tuple
    fn message(&self) -> &(String, usize) {
        &self.state().messages[*&self.message]
    }

    /// Advance to the next message/state
    fn advance(&mut self, replies: &mut Replies<Event>) -> ControlFlow {
        if self.state >= self.all_states.len() {
            return ControlFlow::Kill;
        }
        let (text, delay) = self.message().clone();
        // advance to the next message (or beyond the end, to indicate to wait for replies)
        self.message += 1;
        if self.message != self.state().messages.len() {
            // if it's not the last message, we can send now (and then just ignore events until the next mssage)
            replies.queue(Event::NPCChatMessage {
                from: self.name.clone(),
                text,
                options: vec![],
            });
            ControlFlow::sleep_for(Duration::from_millis(delay as u64))
        } else {
            // otherwise we send the replies and `Continue`, to make sure we don't miss a thing
            let options = self
                .state()
                .options
                .iter()
                .map(|(s, _)| s.clone())
                .collect();

            replies.queue(Event::NPCChatMessage {
                from: self.name.clone(),
                text,
                options,
            });
            ControlFlow::Continue
        }
    }
}

impl Agent<Event> for NPC {
    fn start(&mut self, replies: &mut Replies<Event>) -> ControlFlow {
        self.advance(replies)
    }

    fn react(&mut self, event: &Event, replies: &mut Replies<Event>) -> ControlFlow {
        if self.state >= self.all_states.len() {
            // reached the end of the conversation tree
            ControlFlow::Kill
        } else if self.message >= self.all_states[self.state].messages.len() {
            // look for a reply
            let (dest, text) = match event {
                Event::PlayerChatMessage { to, text } => (to, text),
                _ => return ControlFlow::Continue,
            };
            if dest != &self.name {
                return ControlFlow::Continue;
            }
            let options = &self.all_states[self.state].options;
            let new_state = match options.iter().find(|(opt, _)| opt == text) {
                Some((_, s)) => s,
                None => return ControlFlow::Continue,
            };
            self.state = *new_state;
            self.message = 0;
            self.advance(replies)
        } else {
            // send the next message
            self.advance(replies)
        }
    }
}

/// Create an NPC with kinda grody but mostly functional syntax
macro_rules! npc {
    ( $name:literal, $(
        [
            $( say $msg:literal : $delay:literal ),* ,
            ask $( $option:literal => $state:literal ),* $(,)?
        ]
    ),* $(,)? ) => {
        NPC {
            name: $name.into(),
            all_states: vec![ $(
                ChatState {
                    messages: vec![ $(
                        ( $msg.into(), $delay )
                    ),* ],
                    options: vec![
                        $( ( $option.into(), $state ) ),*
                    ],
                }
            ),* ],
            ..Default::default()
        }
    };
}

struct Redshell {
    apps: Vec<(Box<dyn App>, usize)>,
    sel_app: usize,
    state: GameState,
}

impl Redshell {
    pub fn new() -> Self {
        Self {
            apps: vec![
                (Box::new(ChatApp::default()), 0),
                (Box::new(CliApp::default()), 0),
            ],
            sel_app: 0,
            state: Default::default(),
        }
    }
}

impl Game for Redshell {
    type Message = Event;

    fn input(&mut self, input: Action, replies: &mut Replies<Event>) -> Response {
        match input {
            Action::KeyPress { key: Key::F(num) } => {
                if num <= self.apps.len() {
                    self.sel_app = num as usize - 1;
                    Response::Redraw
                } else {
                    Response::Nothing
                }
            }
            other => {
                let app_taint = self.apps[self.sel_app].0.input(other, replies);
                if app_taint {
                    Response::Redraw
                } else {
                    Response::Nothing
                }
            }
        }
    }

    fn message(&mut self, event: &Event) -> Response {
        match event {
            Event::AddTab(b) => {
                let app = b
                    .take()
                    .expect("app bundle taken before sole consumer got it");
                let notifs = app.notifs();
                self.apps.push((app, notifs));
                Response::Redraw
            }
            event => {
                let mut tainted = false;
                for (i, (app, old_notifs)) in self.apps.iter_mut().enumerate() {
                    let ev_taint = app.on_event(event, i == self.sel_app);
                    if i == self.sel_app {
                        tainted |= ev_taint;
                    }
                    let new_notifs = app.notifs();
                    if new_notifs != *old_notifs {
                        tainted = true;
                        *old_notifs = new_notifs;
                    }
                }
                if tainted {
                    Response::Redraw
                } else {
                    Response::Nothing
                }
            }
        }
    }

    fn render(&self, onto: &mut Screen) {
        self.apps[self.sel_app].0.render(&self.state, onto);
        let mut header = onto
            .header()
            .profile(&self.state.player_name)
            .selected(self.sel_app)
            .time("right now >:3");
        for (app, notifs) in &self.apps {
            header = header.tab(app.name(), *notifs);
        }
    }
}

pub fn run(mut _args: impl Iterator<Item = String>) {
    let game = Redshell::new();
    Runner::new(game)
        .queue(Event::install(crate::tools::Ls))
        .queue(Event::install(crate::tools::Touch))
        .queue(Event::install(crate::tools::Mkdir))
        .queue(Event::install(crate::tools::Cd))
        .spawn(npc!(
            "admin",
            [
                say "hi": 500,
                ask "controls?" => 1, "hi" => 0,
            ],
            [
                say "sure!": 250,
                say "Press F1, F2, etc. to switch to tab 1, tab 2, etc.": 250,
                say "Tab #1 is chat. There's only two people to chat with and neither is a great conversationalist.": 250,
                say "Tab #2 is your CLI. There's only, like, four commands, and none of them do anything cool.": 250,
                say "And that's it for now!": 250,
                ask "oh ok. hi." => 0,
            ],
        ))
        .spawn(npc!(
            "yotie",
            [
                say "hey": 500,
                say "hello": 500,
                say "hi": 500,
                say "my close personal friend": 1000,
                say "whose name I do not need to say": 1000,
                say "because we're so close and all": 1000,
                say "how you doin?": 1500,
                ask "good" => 1, "bad" => 2,
            ],
            [
                say "ey that's nice": 2000,
                say "glad you're doing well": 500,
                ask "thanks" => 3,
            ],
            [
                say "ey that's bad": 2000,
                say "sucks you're doing meh": 500,
                ask "thanks?" => 3,
            ],
            [
                say "anyway bye": 500,
                ask "uh ok" => 100,
            ],
        ))
        .load_run();
}
