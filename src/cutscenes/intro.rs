use std::{
    io,
    time::{Duration, Instant},
};

use rand::prelude::*;

use crate::{
    cell,
    io::{
        clifmt::{Color, Formatted, FormattedExt, Text},
        helpers::{TextInput, TextInputRequest},
        input::{Action, Key},
        output::{Cell, Screen},
        sys::IoSystem,
    },
    text, text1, GameState,
};

fn sleep(s: f32) {
    std::thread::sleep(Duration::from_secs_f32(s))
}

fn rngat(seed: u64, x: usize, y: usize, xor: u64) -> SmallRng {
    let pos_seed = seed ^ xor ^ (x as u64) ^ (y as u64).rotate_left(32);
    // SmallRng is the right choice here because for this use we only care about the appearance of randomness, not
    // true unpredictability, and it's massively faster to initialize and generate than the higher-quality RNGs.
    // Further, we're not trying to generate the same values across different platforms, or even across different
    // runs on the same platform.
    SmallRng::seed_from_u64(pos_seed)
}

fn fadeat(seed: u64, x: usize, y: usize) -> f32 {
    rngat(seed, x, y, 0x5CA1AB1E7E1ECA57).gen()
}

fn cellat(seed: u64, x: usize, y: usize) -> Cell {
    const CHARS: [char; 92] = [
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J',
        'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '1', '2',
        '3', '4', '5', '6', '7', '8', '9', '0', '!', '@', '#', '$', '%', '^', '&', '*', '(', ')',
        ',', '.', '<', '>', '[', ']', '{', '}', '`', '~', '/', '?', '\\', '|', '\'', '"', ';', ':',
        '-', '_',
    ];
    let ch = *CHARS
        .choose(&mut rngat(seed, x, y, 0xBE1A7EDDECEA5ED))
        .unwrap();
    let fg = rngat(seed, x, y, 0xCA11AB1ECA55E77E).gen();
    Cell::of(ch).fg(fg)
}

fn leaveat(seed: u64, x: usize, y: usize) -> bool {
    rngat(seed, x, y, 0xBA11AD0FBADA55E5).gen_bool(0.01)
}

/// Play the wave, then return the seed of the last shift (which produces the leftovers)
fn sprinkler_wave(io: &mut dyn IoSystem, screen: &mut Screen) -> io::Result<u64> {
    // the unit of width here is fractions of the screen width; time is in seconds

    // the width of one shift
    const SHIFT_WIDTH: f32 = 0.1;
    // how many shifts there are in the wave
    const SHIFT_COUNT: usize = 3;
    // how fast the wave moves right
    const WAVE_SPEED: f32 = 0.75;
    // the width of the whole wave
    const WAVE_WIDTH: f32 = SHIFT_WIDTH * SHIFT_COUNT as f32;

    let seeds: [u64; SHIFT_COUNT] = thread_rng().gen();
    let start = Instant::now();

    loop {
        let now = Instant::now();
        let since_start = now.duration_since(start).as_secs_f32();
        let wave_lead = since_start * WAVE_SPEED;
        let wave_trail = wave_lead - WAVE_WIDTH;
        if wave_trail > 1.0 {
            break;
        }

        screen.resize(io.size());
        for y in 0..screen.size().y() {
            let pct = y as f32 / screen.size().y() as f32;
            if pct > wave_lead {
                // everything past the leading edge is blank
                for x in 0..screen.size().x() {
                    screen[y][x] = Cell::BLANK;
                }
            } else if pct < wave_trail {
                // almost everything past the trailing edge is blank
                for x in 0..screen.size().x() {
                    if leaveat(seeds[SHIFT_COUNT - 1], x, y) {
                        screen[y][x] = cellat(seeds[SHIFT_COUNT - 1], x, y);
                    } else {
                        // leave it blank
                    }
                }
            } else {
                let shift_pos = (wave_lead - pct) / SHIFT_WIDTH;
                let from_shift = shift_pos as usize;
                let to_shift = from_shift + 1;
                let within = shift_pos.fract();

                for x in 0..screen.size().x() {
                    if within < fadeat(seeds[from_shift], x, y) {
                        if from_shift > 0 {
                            screen[y][x] = cellat(seeds[from_shift], x, y);
                        }
                    } else {
                        if to_shift < SHIFT_COUNT {
                            screen[y][x] = cellat(seeds[to_shift], x, y);
                        } else if leaveat(seeds[from_shift], x, y) {
                            screen[y][x] = cellat(seeds[from_shift], x, y);
                        }
                    }
                }
            }
        }
        io.draw(&screen)?;

        // wait until either the screen is resized, or a brief (randomized) period passes
        sleep(0.01);
    }

    Ok(seeds[SHIFT_COUNT - 1])
}

fn gen_lines() -> Vec<Text> {
    let mut rng = thread_rng();
    let mut verbs = [
        "LOADING",
        "DECRYPTING",
        "SPAWNING",
        "ALLOCATING",
        "CREATING",
        "INITIALIZING",
        "BUILDING",
        "SETTING UP",
        "RETICULATING",
        "COMPILING",
        "ASSEMBLING",
        "GENERATING",
        "VALIDATING",
    ];
    let mut nouns = [
        "ENCRYPTOR",
        "ANONYMIZER",
        "PACKET FILTERS",
        "KERNEL",
        "SERVER",
        "DAEMON",
        "SUBPROCESS",
        "SHELL",
        "SPLINES",
        "PROGRAM CODE",
        "COMPILER",
        "INTERPRETER",
        "NEURAL LINK",
    ];
    assert!(verbs.len() == nouns.len());
    verbs.shuffle(&mut rng);
    nouns.shuffle(&mut rng);
    verbs
        .into_iter()
        .zip(nouns.into_iter())
        .map(|(v, n)| text1!(bold green "\n{} {}..."(v, n)))
        .collect()
}

fn loading_text(io: &mut dyn IoSystem, screen: &mut Screen, seed: u64) -> io::Result<()> {
    const MIN_DELAY: f32 = 0.25;
    const MAX_DELAY: f32 = 0.75;

    let lines = gen_lines();
    let mut scroll = 0;
    let mut show_next_time = Instant::now() + Duration::from_secs_f32(1.0);
    while scroll < io.size().y() + lines.len() + 1 {
        screen.resize(io.size());

        // render the loading lines first so we know where to put the rest
        let mut text = lines.iter().cloned().collect::<Vec<_>>();
        text.resize(scroll, text1!("\n"));
        let textinfo = screen.textbox(text).scroll_bottom(true).render();
        let y_off = textinfo.lines;
        if y_off <= screen.size().y() {
            screen
                .horizontal(screen.size().y() - y_off)
                .fill(cell!(green on_black '='));
        }

        if y_off < screen.size().y() {
            for y_raw in 0..(screen.size().y() - y_off) {
                let y = y_raw + y_off;
                for x in 0..screen.size().x() {
                    if leaveat(seed, x, y) {
                        screen[y_raw][x] = cellat(seed, x, y);
                    }
                }
            }
        }
        io.draw(&screen)?;

        sleep(0.01);
        if Instant::now() > show_next_time {
            scroll += 1;
            let delay = if scroll < lines.len() {
                // while we're still doing lines, pause a bit between each
                thread_rng().gen_range(MIN_DELAY..MAX_DELAY)
            } else {
                // once that's done, pause long enough to scroll the screen off pretty fast
                0.5 / screen.size().y() as f32
            };
            show_next_time = Instant::now() + Duration::from_secs_f32(delay);
        }
    }

    Ok(())
}

fn render(io: &mut dyn IoSystem, screen: &mut Screen, text: &[Text]) -> io::Result<()> {
    screen.resize(io.size());
    let mut text_v: Vec<_> = text.iter().cloned().collect();
    if let Some(last) = text_v.last_mut() {
        // trim trailing newline
        last.text = last.text.trim_end().into();
    }
    screen
        .textbox(text_v)
        .scroll_bottom(true)
        .indent(10 + 2)
        .first_indent(0);
    io.draw(screen)
}

fn render_sleep(
    delay: f32,
    io: &mut dyn IoSystem,
    screen: &mut Screen,
    text: &[Text],
) -> io::Result<()> {
    let end = Instant::now() + Duration::from_secs_f32(delay);
    while Instant::now() < end {
        render(io, screen, &text)?;
        sleep(0.01);
    }
    Ok(())
}

fn name_input(io: &mut dyn IoSystem, screen: &mut Screen, text: &[Text]) -> io::Result<String> {
    let mut ti = TextInput::new("         >  ", 0);
    ti.set_complete("(type now)".into());

    let name = loop {
        let redraw = match io.input()? {
            // loop and redraw
            Action::Redraw => true,
            other => match ti.action(other) {
                // ignore autocomplete
                TextInputRequest::Autocomplete => false,
                TextInputRequest::Nothing => false,
                TextInputRequest::Redraw => true,
                TextInputRequest::Line(name) => break name,
            },
        };
        if redraw {
            let mut last_line = ti.render();
            for elem in &mut last_line {
                if elem.get_fmt().fg == Color::BrightWhite {
                    elem.get_fmt_mut().fg = Color::Cyan;
                }
            }
            let all_text: Vec<_> = text.iter().cloned().chain(last_line.into_iter()).collect();
            render(io, screen, &all_text)?;
        }
    };
    Ok(name)
}

fn do_choice<'a>(
    io: &mut dyn IoSystem,
    screen: &mut Screen,
    text: &[Text],
    opts: &[&'a str],
) -> io::Result<&'a str> {
    let mut selected = 0;
    loop {
        let mut last_line = Vec::with_capacity(opts.len() * 2);
        for (i, opt) in opts.iter().enumerate() {
            if i == 0 {
                last_line.push(text1!("         >  "));
            } else {
                last_line.push(text1!("  "));
            }
            let mut text = text1!(bold bright_white "{}"(opt));
            if i == selected {
                text = text.underline();
            }
            last_line.push(text);
        }
        let text: Vec<_> = text.iter().cloned().chain(last_line).collect();

        render(io, screen, &text)?;

        loop {
            let mut redraw = true;
            match io.input()? {
                Action::KeyPress { key: Key::Enter } => return Ok(opts[selected]),
                Action::KeyPress { key: Key::Left } => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                Action::KeyPress { key: Key::Right } => {
                    if selected < opts.len() - 1 {
                        selected += 1;
                    }
                }
                // we want to redraw on resize but nothing else
                Action::Redraw => (),
                _ => redraw = false,
            }
            if redraw {
                break;
            }
        }
    }
}

fn tutorial(io: &mut dyn IoSystem, screen: &mut Screen) -> io::Result<String> {
    let mut text: Vec<Text> = vec![];
    macro_rules! admin_say {
        ( $( $delay:expr =>
            $(
                $( $_name:ident )* $_fmt:literal $( ( $($arg:expr),* $(,)? ) )?
            ),* $(,)?
        );* $(;)? ) => { $(
            render_sleep($delay, io, screen, &text)?;
            text.extend(text!(
                "     admin: ",
                $( bold bright_white $($_name)* $_fmt $(($($arg),*))? ),*,
                "\n"
            ));
            render(io, screen, &text)?;
        )* };
    }

    admin_say!(
        1.0 => "welcome to the fight";
        1.5 => "what can I call you?";
        0.75 => "not your real name.";
    );
    render_sleep(0.5, io, screen, &text)?;
    let name = name_input(io, screen, &text)?;
    text.extend(text!("         >  ", cyan "{}"(name), "\n"));

    macro_rules! choose {
        ( $( $option:literal => $then:expr $(,)? )* ) => {
            render_sleep(0.25, io, screen, &text)?;
            match do_choice(io, screen, &text, &[$($option),*])? {
                $( $option => {
                    text.extend(text!(
                        cyan "{:>10}: "(name),
                        bold bright_white $option,
                        "\n",
                    ));
                    $then
                }, )*
                _ => unreachable!("selected unavailable choice"),
            };
        };
    }

    admin_say!(
        1.0 => "you're ", cyan "{}"(name), "?";
        1.5 => "good name";
        1.5 => "you ever used redshell?";
    );
    choose! {
        "yes" => {
            admin_say!(
                0.5 => "cool";
                1.0 => "good luck";
            );
            return Ok(name);
        }
        "no" => (), // continues below
    }
    admin_say!(
        0.25 => "...";
        0.25 => "ok, look";
        0.25 => "I haven't written the tutorial yet, so.";
        0.25 => "Press F1 to open the first tab, F2 for the second, etc.";
        0.25 => "Tab #1 is chat. There's only two people to chat with and neither is a great conversationalist.";
        0.25 => "Tab #2 is your CLI. There's only, like, four commands, and none of them do anything cool.";
        0.25 => "And that's it for now!";
        0.25 => "But please do try them and let me know what you think.";
        0.25 => "Thanks!";
        5.0 => "ok good luck";
    );

    Ok(name)
}

/// Run the intro cutscene
pub fn run(io: &mut dyn IoSystem, screen: &mut Screen) -> io::Result<GameState> {
    let seed = sprinkler_wave(io, screen)?;
    loading_text(io, screen, seed)?;

    // screen should now be blank so we can start on the actual intro
    let name = tutorial(io, screen)?;

    Ok(GameState {
        player_name: name,
        machine: Default::default(),
    })
}
