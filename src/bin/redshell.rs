use std::{mem, thread, time::Duration};

use redshell::{
    agents::{Agent, ControlFlow, Event},
    app::{ChatApp, CliApp},
    cutscenes,
    io::{
        input::{Action, Key},
        output::Screen,
        sys::{self, IoSystem},
    },
    tools, GameState,
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
    fn advance(&mut self, replies: &mut Vec<Event>) -> ControlFlow {
        if self.state >= self.all_states.len() {
            return ControlFlow::Kill;
        }
        let (text, delay) = self.message().clone();
        // advance to the next message (or beyond the end, to indicate to wait for replies)
        self.message += 1;
        if self.message != self.state().messages.len() {
            // if it's not the last message, we can send now (and then just ignore events until the next mssage)
            replies.push(Event::NPCChatMessage {
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

            replies.push(Event::NPCChatMessage {
                from: self.name.clone(),
                text,
                options,
            });
            ControlFlow::Continue
        }
    }
}

impl Agent for NPC {
    fn start(&mut self, replies: &mut Vec<Event>) -> ControlFlow {
        self.advance(replies)
    }

    fn react(&mut self, events: &[Event], replies: &mut Vec<Event>) -> ControlFlow {
        if self.state >= self.all_states.len() {
            // reached the end of the conversation tree
            ControlFlow::Kill
        } else if self.message >= self.all_states[self.state].messages.len() {
            // look for a reply
            let mut cf = ControlFlow::Continue;
            for event in events {
                let (dest, text) = match event {
                    Event::PlayerChatMessage { to, text } => (to, text),
                    _ => continue,
                };
                if dest != &self.name {
                    continue;
                }
                let options = &self.all_states[self.state].options;
                let new_state = match options.iter().find(|(opt, _)| opt == text) {
                    Some((_, s)) => s,
                    None => continue,
                };
                self.state = *new_state;
                self.message = 0;
                cf = self.advance(replies);
                break;
            }
            cf
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

/// Main game loop
fn run(iosys: &mut dyn IoSystem) {
    let mut screen = Screen::new(iosys.size());

    // get the state, optionally from running the intro cutscene
    let mut state = if let Some(name) = std::env::args().skip(1).next() {
        GameState {
            player_name: name,
            apps: vec![Box::new(ChatApp::default()), Box::new(CliApp::default())],
            machine: Default::default(),
        }
    } else {
        cutscenes::intro(iosys, &mut screen).expect("couldn't run intro cutscene")
    };

    let mut prev_notifs = vec![0; state.apps.len()];
    let mut sel = 0;
    let mut events = vec![
        Event::install(tools::Ls),
        Event::install(tools::Touch),
        Event::install(tools::Mkdir),
        Event::install(tools::Cd),
        Event::spawn(npc!(
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
        )),
        Event::spawn(npc!(
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
        )),
    ];
    let mut agents: Vec<(Box<dyn Agent>, ControlFlow)> = vec![];

    let mut replies = vec![];
    // Whether or not the screen needs to be redrawn since it was last rendered
    let mut tainted = true;
    loop {
        // feed all the agents this round of events
        for (agent, cf) in agents.iter_mut() {
            if !cf.is_ready() {
                // skip it until it *is* ready
                continue;
            }
            *cf = agent.react(&events, &mut replies);
        }
        // delete agents who asked to die
        agents.retain(|(_, cf)| cf != &ControlFlow::Kill);
        // feed events to the apps, update notifications accordingly
        for (i, (app, old_notifs)) in state
            .apps
            .iter_mut()
            .zip(prev_notifs.iter_mut())
            .enumerate()
        {
            for ev in &events {
                let ev_taint = app.on_event(ev);
                if i == sel {
                    tainted |= ev_taint;
                }
            }
            let new_notifs = app.notifs();
            tainted |= new_notifs != *old_notifs;
            *old_notifs = new_notifs;
        }
        // handle system events
        for ev in &events {
            match ev {
                Event::SpawnAgent(b) => {
                    let mut ag = b
                        .take()
                        .expect("agent bundle taken before sole consumer got it");
                    let cf = ag.start(&mut replies);
                    agents.push((ag, cf));
                }
                Event::AddTab(b) => {
                    let app = b
                        .take()
                        .expect("app bundle taken before sole consumer got it");
                    state.apps.push(app);
                }
                _ => (),
            }
        }

        // wait for 25ms or the next input (whichever is sooner) before redrawing
        if let Some(inp) = iosys.input_until(Duration::from_secs_f32(0.25)).unwrap() {
            match inp {
                Action::KeyPress { key: Key::F(num) } => {
                    if num <= state.apps.len() {
                        sel = num as usize - 1;
                        tainted = true;
                    }
                }
                Action::Closed => break,
                Action::Redraw => tainted = true,

                other => tainted |= state.apps[sel].input(other, &mut replies),
            }
        }

        // ensure the screen size is up to date
        let new_size = iosys.size();
        if new_size != screen.size() {
            tainted = true;
        }

        // redraw, if necessary
        if tainted {
            screen.resize(new_size);
            state.apps[sel].render(&Default::default(), &mut screen);
            {
                let mut header = screen
                    .header()
                    .profile(&state.player_name)
                    .selected(sel)
                    .time("right now >:3");
                for app in &state.apps {
                    header = header.tab(app.name(), app.notifs());
                }
            }
            iosys.draw(&screen).unwrap();
            tainted = false;
        }

        // and get ready for the next round of event processing
        mem::swap(&mut events, &mut replies);
        replies.clear();
    }
    iosys.stop();
}

fn main() {
    let (mut iosys, mut iorun) = sys::load().expect("failed to load");
    thread::spawn(move || run(iosys.as_mut()));
    iorun.run()
}
