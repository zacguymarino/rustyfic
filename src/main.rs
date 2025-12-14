// mod world;
// mod engine;

// use std::env;
// use std::io::{self, Write};
// use std::path::PathBuf;
// use std::collections::{HashMap, HashSet};

// fn flush_output(out: engine::Output) {
//     use engine::OutputBlock;

//     let mut printed_anything = false;
//     let mut started_events = false;

//     for block in out.blocks {
//         match block {
//             OutputBlock::Title(t) => {
//                 println!("\n{}", t);
//                 printed_anything = true;
//             }
//             OutputBlock::Text(line) => {
//                 println!("{}", line);
//                 printed_anything = true;
//             }
//             OutputBlock::Event(ev) => {
//                 if !started_events {
//                     if printed_anything {
//                         println!(); // visual separation before first event
//                     }
//                     started_events = true;
//                 }
//                 println!("{}", ev);
//                 printed_anything = true;
//             }
//             OutputBlock::Exits(exits) => {
//                 println!("\n{}", exits);
//                 printed_anything = true;
//             }
//         }
//     }
// }

// fn main() -> io::Result<()> {
//     let world_path: PathBuf = env::args()
//         .nth(1)
//         .map(PathBuf::from)
//         .unwrap_or_else(|| PathBuf::from("public/domus.toml"));

//     let world = match world::load_world_from_file(&world_path) {
//         Ok(w) => {
//             println!("Using world file: {}", world_path.display());
//             w
//         }
//         Err(e) => {
//             eprintln!("Failed to load world file '{}': {e}", world_path.display());
//             std::process::exit(1);
//         }
//     };

//     println!("Welcome to {}!", world.name);
//     if !world.desc.trim().is_empty() {
//         println!("{}", world.desc.trim());
//     }
//     println!();
//     println!("Type 'look' to look around, 'quit' to exit.\n");

//     let mut current_room_id = world.start_room.clone();
//     let mut flags: HashSet<String> = HashSet::new();
//     let mut fired_global_conditions: HashSet<String> = HashSet::new();

//     let mut item_locations: HashMap<String, world::ItemLocation> = HashMap::new();
//     for (id, item) in &world.items {
//         item_locations.insert(id.clone(), item.start_location.clone());
//     }

//     // Initial room render
//     if let Some(room) = world.rooms.get(&current_room_id) {
//         let mut out = engine::Output::new();
//         engine::render_room(&mut out, room, &flags, &world, &item_locations);
//         flush_output(out);
//     } else {
//         eprintln!("Error: start_room '{}' not found.", current_room_id);
//         return Ok(());
//     }

//     let stdin = io::stdin();

//     loop {
//         print!("> ");
//         io::stdout().flush()?;

//         let mut input = String::new();
//         let bytes_read = stdin.read_line(&mut input)?;
//         if bytes_read == 0 {
//             println!("\nGoodbye.");
//             break;
//         }

//         let input = input.trim();
//         if input.is_empty() {
//             continue;
//         }

//         let mut out = engine::Output::new();
//         let lower = input.to_lowercase();
//         let mut quit = false;

//         if lower == "quit" || lower == "exit" {
//             out.say("Goodbye.");
//             quit = true;

//         } else if lower == "inventory" || lower == "i" {
//             engine::handle_inventory(&mut out, &world, &item_locations);

//         } else {
//             let mut parts = input.split_whitespace();
//             let verb = parts.next().unwrap();
//             let rest = parts.collect::<Vec<&str>>().join(" ");
//             let rest_lower = rest.to_lowercase();

//             if verb.eq_ignore_ascii_case("take") || verb.eq_ignore_ascii_case("get") {
//                 if rest.is_empty() {
//                     out.say("Take what?");
//                 } else if rest_lower == "all" {
//                     engine::handle_take_all_room(&mut out, &mut item_locations, &world, &current_room_id, &flags);
//                 } else if let Some(idx) = rest_lower.find(" from ") {
//                     let item_part = rest_lower[..idx].trim();
//                     let container_part = rest_lower[idx + " from ".len()..].trim();

//                     if item_part.is_empty() || container_part.is_empty() {
//                         out.say("I don't understand what you want to take from where.");
//                     } else if item_part == "all" {
//                         engine::handle_take_all_from_container(&mut out, &mut item_locations, &world, &current_room_id, container_part, &flags);
//                     } else {
//                         engine::handle_take_from_container(&mut out, &mut item_locations, &world, &current_room_id, item_part, container_part, &flags);
//                     }
//                 } else {
//                     engine::handle_take(&mut out, &mut item_locations, &world, &current_room_id, &rest_lower, &flags);
//                 }

//             } else if verb.eq_ignore_ascii_case("drop") {
//                 if rest.is_empty() {
//                     out.say("Drop what?");
//                 } else if rest_lower == "all" {
//                     engine::handle_drop_all(&mut out, &mut item_locations, &world, &current_room_id);
//                 } else {
//                     engine::handle_drop(&mut out, &mut item_locations, &world, &current_room_id, &rest_lower);
//                 }

//             } else if verb.eq_ignore_ascii_case("examine")
//                 || verb.eq_ignore_ascii_case("x")
//                 || (verb.eq_ignore_ascii_case("look") && rest_lower.starts_with("at "))
//             {
//                 let target = if verb.eq_ignore_ascii_case("look") {
//                     rest_lower.trim_start_matches("at").trim()
//                 } else {
//                     rest_lower.trim()
//                 };

//                 if target.is_empty() {
//                     out.say("Examine what?");
//                 } else {
//                     engine::handle_examine(&mut out, &world, &item_locations, &current_room_id, target, &flags);
//                 }

//             } else if engine::try_handle_container_store(
//                 &mut out, verb, &rest_lower, &mut item_locations, &world, &current_room_id, &mut flags
//             ) {
//                 // handled

//             } else if let Some(current_room) = world.rooms.get(&current_room_id) {
//                 if lower == "look" || lower == "l" {
//                     engine::render_room(&mut out, current_room, &flags, &world, &item_locations);

//                 } else if engine::try_handle_movement(&mut out, &mut current_room_id, &world, current_room, &lower, &flags) {
//                     if let Some(room) = world.rooms.get(&current_room_id) {
//                         engine::render_room(&mut out, room, &flags, &world, &item_locations);
//                     }

//                 } else if engine::try_handle_action(
//                     &mut out,
//                     current_room,
//                     input,
//                     &world,
//                     &item_locations,
//                     &current_room_id,
//                     &mut flags,
//                 ) {
//                     // handled

//                 } else if engine::try_handle_global_action(
//                     &mut out,
//                     input,
//                     &world,
//                     &item_locations,
//                     &current_room_id,
//                     &mut flags,
//                 ) {
//                     // handled

//                 } else {
//                     out.say("I don't understand that command.");
//                 }

//             } else {
//                 out.say(format!("Error: you are in an unknown room '{}'", current_room_id));
//                 quit = true;
//             }
//         }

//         engine::evaluate_global_conditions(&mut out, &world, &mut flags, &current_room_id, &mut fired_global_conditions);

//         flush_output(out);

//         if quit {
//             break;
//         }
//     }

//     Ok(())
// }


mod world;
mod engine;

use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::collections::{HashMap, HashSet};

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
        .unwrap_or_else(|| PathBuf::from("public/domus.toml"));

    let world = match world::load_world_from_file(&world_path) {
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

    let mut current_room_id = world.start_room.clone();
    let mut flags: HashSet<String> = HashSet::new();
    let mut fired_global_conditions: HashSet<String> = HashSet::new();

    let mut item_locations: HashMap<String, world::ItemLocation> = HashMap::new();
    for (id, item) in &world.items {
        item_locations.insert(id.clone(), item.start_location.clone());
    }

    // Initial room render
    if let Some(room) = world.rooms.get(&current_room_id) {
        let mut out = engine::Output::new();
        engine::render_room(&mut out, room, &flags, &world, &item_locations);
        flush_output(out);
    } else {
        eprintln!("Error: start_room '{}' not found.", current_room_id);
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

        let mut out = engine::Output::new();
        let lower = input.to_lowercase();
        let mut quit = false;

        if lower == "quit" || lower == "exit" {
            out.say("Goodbye.");
            quit = true;

        } else if lower == "inventory" || lower == "i" {
            engine::handle_inventory(&mut out, &world, &item_locations);

        } else {
            let mut parts = input.split_whitespace();
            let verb = parts.next().unwrap();
            let rest = parts.collect::<Vec<&str>>().join(" ");
            let rest_lower = rest.to_lowercase();

            if verb.eq_ignore_ascii_case("take") || verb.eq_ignore_ascii_case("get") {
                if rest.is_empty() {
                    out.say("Take what?");
                } else if rest_lower == "all" {
                    engine::handle_take_all_room(&mut out, &mut item_locations, &world, &current_room_id, &flags);
                } else if let Some(idx) = rest_lower.find(" from ") {
                    let item_part = rest_lower[..idx].trim();
                    let container_part = rest_lower[idx + " from ".len()..].trim();

                    if item_part.is_empty() || container_part.is_empty() {
                        out.say("I don't understand what you want to take from where.");
                    } else if item_part == "all" {
                        engine::handle_take_all_from_container(&mut out, &mut item_locations, &world, &current_room_id, container_part, &flags);
                    } else {
                        engine::handle_take_from_container(&mut out, &mut item_locations, &world, &current_room_id, item_part, container_part, &flags);
                    }
                } else {
                    engine::handle_take(&mut out, &mut item_locations, &world, &current_room_id, &rest_lower, &flags);
                }

            } else if verb.eq_ignore_ascii_case("drop") {
                if rest.is_empty() {
                    out.say("Drop what?");
                } else if rest_lower == "all" {
                    engine::handle_drop_all(&mut out, &mut item_locations, &world, &current_room_id);
                } else {
                    engine::handle_drop(&mut out, &mut item_locations, &world, &current_room_id, &rest_lower);
                }

            } else if verb.eq_ignore_ascii_case("examine")
                || verb.eq_ignore_ascii_case("x")
                || (verb.eq_ignore_ascii_case("look") && rest_lower.starts_with("at "))
            {
                let target = if verb.eq_ignore_ascii_case("look") {
                    rest_lower.trim_start_matches("at").trim()
                } else {
                    rest_lower.trim()
                };

                if target.is_empty() {
                    out.say("Examine what?");
                } else {
                    engine::handle_examine(&mut out, &world, &item_locations, &current_room_id, target, &flags);
                }

            } else if engine::try_handle_container_store(
                &mut out, verb, &rest_lower, &mut item_locations, &world, &current_room_id, &mut flags
            ) {
                // handled

            } else if let Some(current_room) = world.rooms.get(&current_room_id) {
                if lower == "look" || lower == "l" {
                    engine::render_room(&mut out, current_room, &flags, &world, &item_locations);

                } else if engine::try_handle_movement(&mut out, &mut current_room_id, &world, current_room, &lower, &flags) {
                    if let Some(room) = world.rooms.get(&current_room_id) {
                        engine::render_room(&mut out, room, &flags, &world, &item_locations);
                    }

                } else if engine::try_handle_action(
                    &mut out,
                    current_room,
                    input,
                    &world,
                    &item_locations,
                    &current_room_id,
                    &mut flags,
                ) {
                    // handled

                } else if engine::try_handle_global_action(
                    &mut out,
                    input,
                    &world,
                    &item_locations,
                    &current_room_id,
                    &mut flags,
                ) {
                    // handled

                } else {
                    out.say("I don't understand that command.");
                }

            } else {
                out.say(format!("Error: you are in an unknown room '{}'", current_room_id));
                quit = true;
            }
        }

        engine::evaluate_global_conditions(&mut out, &world, &mut flags, &current_room_id, &mut fired_global_conditions);

        flush_output(out);

        if quit {
            break;
        }
    }

    Ok(())
}
