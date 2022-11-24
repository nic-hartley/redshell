use std::{
    collections::HashMap,
    env::args,
    future::Future,
    io::{stdout, Write},
    pin::Pin,
    time::Duration,
};

use redshell::{
    app::{App, ChatApp},
    event::Event,
    io::{
        input::{Action, Key},
        output::{Color, FormattedExt, Screen, Text},
        sys::{self, IoSystem},
        XY,
    },
    text, GameState,
};
use tokio::time::sleep;

pub fn load_or_die() -> Box<dyn IoSystem> {
    let errs = match sys::load() {
        Ok(io) => return io,
        Err(e) => e,
    };

    if errs.is_empty() {
        println!("No renderers enabled! Someone compiled this wrong.")
    } else {
        println!("{} renderers tried to load:", errs.len());
        for (name, err) in errs {
            println!("- {}: {:?}", name, err);
        }
    }

    std::process::exit(1);
}

async fn render_demo(io: &mut dyn IoSystem) {
    let mut s = Screen::new(io.size());
    s.horizontal(1);
    s.vertical(0);
    let mut texts = Vec::new();
    for fg in Color::all() {
        texts.push(Text::of(format!("{} on:\n", fg.name())));
        let amt = Color::all().len();
        const LINES: usize = 2;
        for (i, bg) in IntoIterator::into_iter(Color::all()).enumerate() {
            let text = Text::of(format!("{}", bg.name())).fg(fg).bg(bg);
            texts.push(text);
            if i % (amt / LINES) == amt / LINES - 1 {
                texts.push(Text::plain("\n"));
            } else if i < amt - 1 {
                texts.push(Text::plain(" "));
            }
        }
    }

    texts.extend(text!("\n", underline "underline", " ", bold "bold", " ", invert "invert", " "));

    s.textbox(texts).pos(1, 2);
    s.header()
        .tab("tab", 1)
        .tab("tab", 2)
        .selected(1)
        .profile("watching the render concept")
        .time("the time is now");
    io.draw(&s).await.unwrap();
}

async fn intro_demo(io: &mut dyn IoSystem) {
    redshell::cutscenes::intro(io)
        .await
        .expect("Failed to run intro");
}

async fn chat_demo(io: &mut dyn IoSystem) {
    let mut s = Screen::new(io.size());

    let mut app = ChatApp::default();
    let state = GameState {
        player_name: "player".into(),
        apps: vec![],
    };
    let frames: Vec<(_, &[Action])> = vec![
        (
            vec![Event::npc_chat(
                "alice",
                "hello there",
                &["hi", "hello", "sup"],
            )],
            &[],
        ),
        (
            vec![Event::npc_chat("bob", "so", &[])],
            &[Action::KeyPress { key: Key::Right }],
        ),
        (
            vec![Event::npc_chat("alice", "buddy", &["hi", "hello", "sup"])],
            &[Action::KeyPress { key: Key::Right }],
        ),
        (vec![], &[Action::KeyPress { key: Key::Enter }]),
        (
            vec![
                Event::npc_chat("bob", "hi friend", &[]),
                Event::npc_chat("charlie", "asdfasdfasdfadsf", &[]),
                Event::npc_chat("charlie", "adskfljalksdjasldkf", &[]),
                Event::npc_chat("bob", "u up?", &["yes", "no"]),
            ],
            &[],
        ),
        (
            vec![Event::npc_chat("alice", "so", &[])],
            &[Action::KeyPress { key: Key::Down }],
        ),
        (
            vec![
                Event::npc_chat("alice", "uh", &[]),
                Event::npc_chat("bob", "hello?", &["yes hello", "no goodbye"]),
                Event::npc_chat("alice", "what's the deal with airline tickets", &[]),
            ],
            &[],
        ),
        (vec![], &[Action::KeyPress { key: Key::Up }]),
    ];
    for (chats, inputs) in frames.into_iter() {
        s.resize(io.size());
        app.on_event(&chats);
        for input in inputs.into_iter() {
            let mut _events = vec![];
            app.input(input.clone(), &mut _events);
        }
        app.render(&state, &mut s);
        s.textbox(text!(
            "This is a ", bold red "demo", " of the chatbox. No input necessary."
        ))
        .pos(0, 0)
        .height(1);
        io.draw(&s).await.unwrap();
        sleep(Duration::from_millis(1000)).await;
    }
    sleep(Duration::from_secs(1)).await;
}

async fn mouse_demo(io: &mut dyn IoSystem) {
    let mut s = Screen::new(io.size());
    s.textbox(text!(invert "Press any keyboard button to exit"));
    io.draw(&s).await.unwrap();
    loop {
        let text;
        let at;
        match io.input().await.unwrap() {
            Action::KeyPress { .. } | Action::KeyRelease { .. } => break,
            Action::MousePress { button, pos } => {
                text = format!("{:?} button pressed at {:?}", button, pos);
                at = pos;
            }
            Action::MouseRelease { button, pos } => {
                text = format!("{:?} button released at {:?}", button, pos);
                at = pos;
            }
            Action::MouseMove {
                button: Some(b),
                pos,
            } => {
                text = format!("Moved to {:?} holding {:?}", pos, b);
                at = pos;
            }
            Action::MouseMove { button: None, pos } => {
                text = format!("Moved to {:?} holding nothing", pos);
                at = pos;
            }
            Action::Resized => {
                text = format!("Window resized");
                at = XY(0, 0);
            }
            Action::Unknown(desc) => {
                text = format!("Unknown input: {}", desc);
                at = XY(0, 0);
            }
            Action::Error(msg) => {
                text = format!("Error: {}", msg);
                at = XY(0, 0);
            }
        };
        s.resize(io.size());
        s.textbox(text!(invert "Press any keyboard button to exit"));
        s.textbox(text!("{}"(text))).xy(at);
        io.draw(&s).await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    let concepts = {
        type ConceptFn = for<'a> fn(&'a mut dyn IoSystem) -> Pin<Box<dyn Future<Output = ()> + 'a>>;
        let mut map: HashMap<&'static str, ConceptFn> = HashMap::new();
        map.insert("render", |s| Box::pin(render_demo(s)));
        map.insert("intro", |s| Box::pin(intro_demo(s)));
        map.insert("chat", |s| Box::pin(chat_demo(s)));
        map.insert("mouse", |s| Box::pin(mouse_demo(s)));
        map
    };

    let mut args = args();
    let arg0 = args.next().expect("how did you have no argv[0]");
    if let Some(name) = args.next() {
        if let Some(func) = concepts.get(name.as_str()) {
            print!("Playing {}... ", name);
            stdout().flush().unwrap();
            {
                let mut iosys = load_or_die();
                func(iosys.as_mut()).await;
                let XY(width, height) = iosys.size();
                let msg = "fin.";
                write!(
                    stdout(),
                    "\x1b[{};{}H\x1b[107;30m{}\x1b[0m",
                    height,
                    width - msg.len(),
                    msg
                )
                .unwrap();
                stdout().flush().unwrap();
                sleep(Duration::from_secs(2)).await;
            }
            println!(" Done.");
            return;
        }
    }
    println!("Show off some concept art, built on the actual UI toolkit of the game.");
    println!("Pass the name as the first parameter, i.e.:");
    println!("  {} <name>", arg0);
    println!();
    println!("Available concept art is:");
    for (k, _) in concepts {
        println!("- {}", k);
    }
}
