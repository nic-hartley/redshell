pub mod app;
pub mod concept;
pub mod constants;
pub mod cutscenes;
pub mod event;
pub mod game;
pub mod machine;
pub mod state;
pub mod tools;

fn run_game(args: &mut dyn Iterator<Item = String>) -> bool {
    if let Some(bin) = args.next() {
        if bin.ends_with("redshell-concept") {
            return false;
        } else if bin.ends_with("redshell") {
            return true;
        }
    }
    if let Some(arg1) = args.next() {
        if arg1 == "concept" || arg1 == "redshell-concept" {
            return false;
        } else if arg1 == "redshell" || arg1 == "game" || arg1 == "play" {
            return true;
        }
    }
    true
}

fn main() {
    let mut args = std::env::args();
    if run_game(&mut args) {
        game::run(args)
    } else {
        concept::run(args)
    }
}
