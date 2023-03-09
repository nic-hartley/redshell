use std::time::Instant;

use redshell::{
    agents::{Agent, ControlFlow},
    game::{Game, Replies, Response},
    io::{input::Action, output::Screen},
    runner::Runner,
};

const AGENTS: u64 = 10_000;

type TinyMessage = u64;

struct TinyAgent {
    factor: u64,
}

impl Agent<TinyMessage> for TinyAgent {
    fn start(&mut self, replies: &mut Replies<TinyMessage>) -> ControlFlow {
        replies.queue(self.factor);
        ControlFlow::Continue
    }

    fn react(&mut self, event: &TinyMessage, replies: &mut Replies<TinyMessage>) -> ControlFlow {
        if *event <= 1 {
            // ignore it: collatz ended
        } else if *event % AGENTS == self.factor % AGENTS {
            let next = if *event % 2 == 0 {
                *event / 2
            } else {
                *event * 3 + 1
            };
            replies.queue(next);
        }
        ControlFlow::Continue
    }
}

#[derive(Default)]
struct TinyGame {
    count: u64,
    max: TinyMessage,
    complete: u64,
}

impl Game for TinyGame {
    type Message = TinyMessage;
    fn event(&mut self, event: &Self::Message) -> Response {
        if event != &0 {
            self.count += 1;
        }
        if *event == 1 {
            self.complete += 1;
            if self.complete == AGENTS {
                Response::Quit
            } else {
                Response::Redraw
            }
        } else if *event > self.max {
            self.max = *event;
            Response::Redraw
        } else {
            if self.count % (AGENTS / 100) == 0 {
                Response::Redraw
            } else {
                Response::Nothing
            }
        }
    }

    fn input(&mut self, _input: Action, _replies: &mut Replies<Self::Message>) -> Response {
        Response::Nothing
    }

    fn render(&mut self, _onto: &mut Screen) {
        println!(
            "count={}, max={}, complete={}",
            self.count, self.max, self.complete
        );
    }
}

fn main() {
    let mut starter = Runner::new(TinyGame::default()).input_tick(0.0);
    for factor in 1..=AGENTS {
        starter = starter.spawn(TinyAgent { factor });
    }
    let start = Instant::now();
    let TinyGame {
        count,
        max,
        complete,
    } = starter.load_run();
    let dur = Instant::now() - start;
    println!("Completed in {:.02}s", dur.as_secs_f32());
    println!(
        "Final state: count={}, max={}, complete={}",
        count, max, complete
    );
    // Ensure we get the right answers
    assert_eq!(count, 859666);
    assert_eq!(max, 27114424);
    assert_eq!(complete, 10000);
}