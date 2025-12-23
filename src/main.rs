mod engine;
mod world;

use std::collections::{HashMap, HashSet};
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;

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
    let mut fired_dialogues: HashSet<String> = HashSet::new();
    let mut action_index: u64 = 0;

    let mut item_locations: HashMap<String, world::ItemLocation> = HashMap::new();
    let mut npc_locations: HashMap<String, String> = HashMap::new();

    let mut turn_index: u64 = 0;

    for (id, npc) in &world.npcs {
        npc_locations.insert(id.clone(), npc.start_room.clone());
    }

    for (id, item) in &world.items {
        item_locations.insert(id.clone(), item.start_location.clone());
    }

    // Initial room render
    if let Some(room) = world.rooms.get(&current_room_id) {
        let mut out = engine::Output::new();
        engine::render_room(
            &mut out,
            room,
            &flags,
            &world,
            &item_locations,
            &npc_locations,
        );
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
        let mut rendered_room_this_turn = false;
        action_index = action_index.wrapping_add(1);

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

            if verb.eq_ignore_ascii_case("talk") || verb.eq_ignore_ascii_case("speak") {
                if rest_lower.is_empty() {
                    out.say("Talk to whom?");
                } else {
                    engine::handle_talk_to_npc(
                        &mut out,
                        &world,
                        &npc_locations,
                        &current_room_id,
                        &rest_lower,
                        &mut flags,
                        &mut fired_dialogues,
                    );
                }
            } else if verb.eq_ignore_ascii_case("give") {
                if rest_lower.is_empty() {
                    out.say("Give what to whom?");
                } else if let Some(idx) = rest_lower.rfind(" to ") {
                    let item_part = rest_lower[..idx].trim();
                    let npc_part = rest_lower[idx + " to ".len()..].trim();

                    if item_part.is_empty() || npc_part.is_empty() {
                        out.say("I don't understand who you want to give that to.");
                    } else {
                        engine::handle_give_to_npc(
                            &mut out,
                            &mut item_locations,
                            &world,
                            &npc_locations,
                            &current_room_id,
                            item_part,
                            npc_part,
                            &mut flags,
                        );
                    }
                } else {
                    out.say("Give it to whom?");
                }
            } else if verb.eq_ignore_ascii_case("take") || verb.eq_ignore_ascii_case("get") {
                if rest.is_empty() {
                    out.say("Take what?");
                } else if rest_lower == "all" {
                    engine::handle_take_all_room(
                        &mut out,
                        &mut item_locations,
                        &world,
                        &current_room_id,
                        &flags,
                    );
                } else if let Some(idx) = rest_lower.find(" from ") {
                    let item_part = rest_lower[..idx].trim();
                    let container_part = rest_lower[idx + " from ".len()..].trim();

                    if item_part.is_empty() || container_part.is_empty() {
                        out.say("I don't understand what you want to take from where.");
                    } else {
                        let handled_npc = engine::handle_take_from_npc(
                            &mut out,
                            &mut item_locations,
                            &world,
                            &npc_locations,
                            &current_room_id,
                            item_part,
                            container_part,
                            &flags,
                        );

                        if !handled_npc {
                            if item_part == "all" {
                                engine::handle_take_all_from_container(
                                    &mut out,
                                    &mut item_locations,
                                    &world,
                                    &current_room_id,
                                    container_part,
                                    &flags,
                                );
                            } else {
                                engine::handle_take_from_container(
                                    &mut out,
                                    &mut item_locations,
                                    &world,
                                    &current_room_id,
                                    item_part,
                                    container_part,
                                    &flags,
                                );
                            }
                        }
                    }
                } else {
                    engine::handle_take(
                        &mut out,
                        &mut item_locations,
                        &world,
                        &current_room_id,
                        &rest_lower,
                        &flags,
                    );
                }
            } else if verb.eq_ignore_ascii_case("drop") {
                if rest.is_empty() {
                    out.say("Drop what?");
                } else if rest_lower == "all" {
                    engine::handle_drop_all(
                        &mut out,
                        &mut item_locations,
                        &world,
                        &current_room_id,
                    );
                } else {
                    engine::handle_drop(
                        &mut out,
                        &mut item_locations,
                        &world,
                        &current_room_id,
                        &rest_lower,
                    );
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
                    engine::handle_examine(
                        &mut out,
                        &world,
                        &item_locations,
                        &npc_locations,
                        &current_room_id,
                        target,
                        &flags,
                    );
                }
            } else if engine::try_handle_container_store(
                &mut out,
                verb,
                &rest_lower,
                &mut item_locations,
                &world,
                &current_room_id,
                &mut flags,
            ) {
                // handled
            } else if let Some(current_room) = world.rooms.get(&current_room_id) {
                if lower == "look" || lower == "l" {
                    engine::render_room(
                        &mut out,
                        current_room,
                        &flags,
                        &world,
                        &item_locations,
                        &npc_locations,
                    );
                    rendered_room_this_turn = true;
                } else {
                    // We want to detect a *successful* move (room id changes),
                    // and only then run deterministic NPC roaming.
                    let prev_room_id = current_room_id.clone();

                    if engine::try_handle_movement(
                        &mut out,
                        &mut current_room_id,
                        &world,
                        current_room,
                        &lower,
                        &npc_locations,
                        &mut flags,
                        action_index,
                    ) {
                        let moved = current_room_id != prev_room_id;

                        if moved {
                            // Turn advances only on successful player movement
                            turn_index += 1;
                            engine::roam_npcs_after_player_move(
                                &world,
                                &mut npc_locations,
                                &flags,
                                turn_index,
                            );

                            if let Some(room) = world.rooms.get(&current_room_id) {
                                engine::render_room(
                                    &mut out,
                                    room,
                                    &flags,
                                    &world,
                                    &item_locations,
                                    &npc_locations,
                                );
                                rendered_room_this_turn = true;
                            }
                        } else {
                            // Movement was handled but did not change rooms (e.g., blocked)
                            rendered_room_this_turn = true;
                        }
                    } else if engine::try_handle_npc_action(
                        &mut out,
                        input,
                        &world,
                        &mut item_locations,
                        &npc_locations,
                        &current_room_id,
                        &mut flags,
                    ) {
                        // handled
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
                }
            } else {
                out.say(format!(
                    "Error: you are in an unknown room '{}'",
                    current_room_id
                ));
                quit = true;
            }
        }

        // If global conditions change flags, re-render ONLY if it would change what the player sees.
        let flags_before = flags.clone();

        engine::evaluate_global_conditions(
            &mut out,
            &world,
            &mut flags,
            &current_room_id,
            &mut fired_global_conditions,
        );

        // Track added OR removed flags
        let mut changed_flags: HashSet<String> = HashSet::new();
        for f in flags.difference(&flags_before) {
            changed_flags.insert(f.clone());
        }
        for f in flags_before.difference(&flags) {
            changed_flags.insert(f.clone());
        }

        if !changed_flags.is_empty() && !rendered_room_this_turn {
            if let Some(room) = world.rooms.get(&current_room_id) {
                if engine::room_depends_on_any_flag(
                    room,
                    &world,
                    &item_locations,
                    &npc_locations,
                    &changed_flags,
                ) {
                    engine::render_room(
                        &mut out,
                        room,
                        &flags,
                        &world,
                        &item_locations,
                        &npc_locations,
                    );
                }
            }
        }

        flush_output(out);

        if quit {
            break;
        }
    }

    Ok(())
}
