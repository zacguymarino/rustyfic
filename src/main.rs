use std::env;
use std::io::{self, Write};
use std::path::PathBuf;

use int_fic::{GameState, engine, load_world_from_file};

fn flush_output(out: engine::Output) {
    use engine::OutputBlock;

    let mut printed_anything = false;
    let mut started_events = false;

    for block in out.blocks {
        match block {
            OutputBlock::Title(t) => {
                println!("\n{}", t);
                printed_anything = true;
            }
            OutputBlock::Text(line) => {
                println!("{}", line);
                printed_anything = true;
            }
            OutputBlock::Event(ev) => {
                if !started_events {
                    if printed_anything {
                        println!(); // visual separation before first event
                    }
                    started_events = true;
                }
                println!("{}", ev);
                printed_anything = true;
            }
            OutputBlock::Exits(exits) => {
                println!("\n{}", exits);
                printed_anything = true;
            }
        }
    }
}

fn main() -> io::Result<()> {
    let world_path: PathBuf = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("public/default.toml"));

    let world = match load_world_from_file(&world_path) {
        Ok(w) => {
            println!("Using world file: {}", world_path.display());
            w
        }
        Err(e) => {
            eprintln!("Failed to load world file '{}': {e}", world_path.display());
            std::process::exit(1);
        }
    };

    println!("Welcome to {}!", world.name);
    if !world.desc.trim().is_empty() {
        println!("{}", world.desc.trim());
    }
    println!();
    println!("Type 'look' to look around, 'quit' to exit.\n");

    let mut game = GameState::new(world);

    if let Some(out) = game.initialize() {
        flush_output(out);
    } else {
        eprintln!("Error: start_room '{}' not found.", game.world.start_room);
        return Ok(());
    }

    let stdin = io::stdin();

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        let bytes_read = stdin.read_line(&mut input)?;
        if bytes_read == 0 {
            println!("\nGoodbye.");
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        let (out, quit) = game.step(input);
        flush_output(out);
        if quit {
            break;
        }
    }

    Ok(())
}
