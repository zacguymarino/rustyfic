use std::collections::HashSet;

use crate::world;
use crate::engine::conditions::conditions_met;
use crate::engine::output::Output;

pub fn try_handle_movement(
    out: &mut Output,
    current_room_id: &mut String,
    world: &world::World,
    room: &world::Room,
    cmd: &str,
    flags: &HashSet<String>,
) -> bool {
    let tokens: Vec<String> = cmd
        .split_whitespace()
        .map(|t| t.to_lowercase())
        .collect();

    if tokens.is_empty() {
        return false;
    }

    // Helper: is this exit currently available?
    let exit_available = |e: &world::Exit| conditions_met(&e.conditions, flags);

    // 1) Exact whole-token matches anywhere in the command
    let mut matches: Vec<&world::Exit> = Vec::new();

    for exit in &room.exits {
        if !exit_available(exit) {
            continue;
        }

        let hit = tokens.iter().any(|tok| {
            exit.direction.eq_ignore_ascii_case(tok)
                || exit.verbs.iter().any(|v| v.eq_ignore_ascii_case(tok))
        });

        if hit {
            matches.push(exit);
        }
    }

    if matches.len() == 1 {
        return do_move(out, current_room_id, world, matches[0]);
    } else if matches.len() > 1 {
        let dirs_list = matches
            .iter()
            .map(|e| e.direction.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        out.say(format!("That movement is ambiguous here. Did you mean: {}?", dirs_list));
        return true;
    }

    // 2) Abbreviations: only if a token is EXACTLY one character (e.g. "s")
    let abbrev_chars: Vec<char> = tokens
        .iter()
        .filter_map(|t| {
            let mut it = t.chars();
            let c = it.next()?;
            if it.next().is_some() { None } else { Some(c) }
        })
        .collect();

    if abbrev_chars.is_empty() {
        return false;
    }

    let mut abbrev_matches: Vec<&world::Exit> = Vec::new();

    for exit in &room.exits {
        if !exit_available(exit) {
            continue;
        }

        let hit_dir = exit
            .direction
            .chars()
            .next()
            .map(|c| abbrev_chars.iter().any(|ac| ac.to_ascii_lowercase() == c.to_ascii_lowercase()))
            .unwrap_or(false);

        let hit_verb = exit.verbs.iter().any(|v| {
            v.chars()
                .next()
                .map(|c| abbrev_chars.iter().any(|ac| ac.to_ascii_lowercase() == c.to_ascii_lowercase()))
                .unwrap_or(false)
        });

        if hit_dir || hit_verb {
            abbrev_matches.push(exit);
        }
    }

    match abbrev_matches.len() {
        0 => false,
        1 => do_move(out, current_room_id, world, abbrev_matches[0]),
        _ => {
            let dirs_list = abbrev_matches
                .iter()
                .map(|e| e.direction.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            out.say(format!("That direction is ambiguous here. Did you mean: {}?", dirs_list));
            true
        }
    }
}

fn do_move(
    out: &mut Output,
    current_room_id: &mut String,
    world: &world::World,
    exit: &world::Exit,
) -> bool {
    if !world.rooms.contains_key(&exit.target) {
        out.say(format!(
            "You try to go {}, but something feels wrong (room not found).",
            exit.direction
        ));
        return true;
    }
    out.say(format!("You go {}.", exit.direction));
    *current_room_id = exit.target.clone();
    true
}
