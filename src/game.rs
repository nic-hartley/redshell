//! Contains the "main loop" bits of the game. Passes events around agents, renders and handles I/O, etc.
//!
//! This is also the primary split between the "engine" and "game" halves.

use core::fmt;
use std::{fmt::Debug, mem, thread, time::Duration};

use crate::{
    agents::{Agent, ControlFlow},
    io::{
        input::Action,
        output::Screen,
        sys::{self, IoSystem},
    },
    util,
};

pub trait Message: Clone + Send + Sync {}
impl<T: Clone + Send + Sync> Message for T {}

/// Allows a [`Game`] or [`Agent`] to make things happen in the engine in response to events or input.
pub struct Replies<M: Message> {
    agents: Vec<Box<dyn Agent<M>>>,
    messages: Vec<M>,
}

impl<M: Message> Replies<M> {
    #[cfg(test)]
    /// A **test-only** function, so you can ensure your code queues the correct messages.
    pub fn messages(&self) -> &[M] {
        &self.messages
    }

    #[cfg(test)]
    /// A **test-only** function, so you can ensure your code spawns the correct agents.
    pub fn agents(&self) -> &[Box<dyn Agent<M>>] {
        &self.agents
    }
}

impl<M: Message> Default for Replies<M> {
    fn default() -> Self {
        Self {
            agents: Default::default(),
            messages: Default::default(),
        }
    }
}

impl<M: Message> Debug for Replies<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .field("agents", &self.agents.len())
            .field("messages", &self.messages.len())
            .finish()
    }
}

impl<M: Message> Replies<M> {
    pub fn spawn(&mut self, agent: impl Agent<M> + 'static) -> &mut Self {
        self.agents.push(Box::new(agent));
        self
    }
    pub fn spawn_boxed(&mut self, agent: Box<dyn Agent<M>>) -> &mut Self {
        self.agents.push(agent);
        self
    }
    pub fn queue(&mut self, msg: M) -> &mut Self {
        self.messages.push(msg);
        self
    }
    pub fn queue_all(&mut self, msgs: impl IntoIterator<Item = M>) -> &mut Self {
        self.messages.extend(msgs);
        self
    }

    pub fn spawn_len(&self) -> usize {
        self.agents.len()
    }
    pub fn queue_len(&self) -> usize {
        self.messages.len()
    }
}

/// Allows a [`Game`] to control the engine in response to events or input.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Response {
    /// Nothing in particular needs to be done.
    Nothing,
    /// The visual state has updated, and the screen needs to be redrawn.
    Redraw,
    /// The game should be exited, e.g. because the user clicked "Exit" in the menu.
    Quit,
}

/// Represents a game which can be run in the main loop.
///
/// Note that `Game`s don't run the bulk of the game logic; that's the `Agent`'s job. The `Game` trait is the place
/// where user input and rendering happen. The idea here is:
///
/// - When there's relevant user input, you can send [`Event`]s or make new agents, and/or update state for rendering
/// - When an [`Event`] happens (including one you spawned!), you can update internal state for rendering
/// - You *don't* react to events with more events -- that's an `Agent`'s job
/// - Come time to render, you already have all the info you need from previous inputs/events
///
/// This makes the code a bit harder to write, but it clearly separates concerns and encourages you to put your heavy
/// logic somewhere other than the render thread.
pub trait Game: Send {
    type Message: Message;

    /// The user has done some input; update the UI and inform [`Agent`]s accordingly.
    ///
    /// Returns whether the game needs to be redrawn after the user input.
    fn input(&mut self, input: Action, replies: &mut Replies<Self::Message>) -> Response;

    /// An event has happened; update the UI accordingly.
    ///
    /// Returns whether the game needs to be redrawn after the event.
    fn event(&mut self, event: &Self::Message) -> Response;

    /// Render the game onto the provided `Screen`.
    // TODO: Make this take &self instead
    fn render(&mut self, onto: &mut Screen);
}

/// Handles starting up and running a `Game`.
#[must_use]
pub struct Runner<G: Game + 'static> {
    events: Vec<G::Message>,
    agents: Vec<Box<dyn Agent<G::Message>>>,
    game: G,
}

impl<G: Game + 'static> Runner<G> {
    /// Prepare a game to be run
    pub fn new(game: G) -> Self {
        Self {
            game,
            events: vec![],
            agents: vec![],
        }
    }

    /// Set an agent to be running at game startup, to process the first tick of events
    pub fn spawn(mut self, agent: impl Agent<G::Message> + 'static) -> Self {
        self.agents.push(Box::new(agent));
        self
    }

    /// Add a message to be handled on the first tick, by the first crop of [`spawn`][Self::spawn]ed agents.
    pub fn queue(mut self, event: G::Message) -> Self {
        self.events.push(event);
        self
    }

    fn run_game(self, iosys: &mut dyn IoSystem) {
        let mut screen = Screen::new(iosys.size());

        let Self {
            mut game,
            mut events,
            agents: mut new_agents,
        } = self;

        let mut replies = Replies::default();
        let mut agents = vec![];
        let mut tainted = true;
        'mainloop: loop {
            agents.extend(new_agents.drain(..).map(|mut a| (a.start(&mut replies), a)));

            let new_size = iosys.size();
            if new_size != screen.size() {
                tainted = true;
            }

            if tainted {
                screen.resize(new_size);
                game.render(&mut screen);
                iosys.draw(&screen).unwrap();
                tainted = false;
            }

            if let Some(inp) = iosys.input_until(Duration::from_secs_f32(0.25)).unwrap() {
                match inp {
                    Action::Closed => break,
                    Action::Redraw => tainted = true,
                    other => match game.input(other, &mut replies) {
                        Response::Nothing => (),
                        Response::Redraw => tainted = true,
                        // TODO: Clean shutdown
                        Response::Quit => break 'mainloop,
                    },
                }
            }

            for event in &events {
                match game.event(event) {
                    Response::Nothing => (),
                    Response::Redraw => tainted = true,
                    // TODO: Clean shutdown
                    Response::Quit => break 'mainloop,
                }
                for (cf, agent) in &mut agents {
                    if !cf.is_ready() {
                        continue;
                    }
                    *cf = agent.react(event, &mut replies);
                }
            }

            mem::swap(&mut replies.messages, &mut events);
            replies.messages.clear();
            mem::swap(&mut replies.agents, &mut new_agents);

            // filter out agents that will never wake up
            util::retain_unstable(&mut agents, |(cf, _ag)| match cf {
                // never is_ready again
                ControlFlow::Kill => false,
                // if there's only one reference, it's the one in this handle
                ControlFlow::Handle(h) => h.references() > 1,
                // otherwise it might eventually wake up, keep it around
                _ => true,
            });
        }
        iosys.stop();
    }

    /// Start the game running.
    ///
    /// This **must** be run on the main thread. Ideally, you'd run it from `main` directly.
    pub fn run(self) {
        let (mut iosys, mut iorun) = sys::load().expect("failed to load");
        thread::spawn(move || self.run_game(iosys.as_mut()));
        iorun.run();
    }
}
