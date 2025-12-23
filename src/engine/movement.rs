use std::collections::{HashMap, HashSet};

use crate::engine::conditions::conditions_met;
use crate::engine::helpers::apply_effects;
use crate::engine::output::Output;
use crate::world;

pub fn try_handle_movement(
    out: &mut Output,
    current_room_id: &mut String,
    world: &world::World,
    room: &world::Room,
    cmd: &str,
    npc_locations: &HashMap<String, String>,
    flags: &mut HashSet<String>,
    attempt_seed: u64,
) -> bool {
    let tokens: Vec<String> = cmd.split_whitespace().map(|t| t.to_lowercase()).collect();

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
        if let Some(block) = movement_blocked_by_npc(
            world,
            npc_locations,
            flags,
            current_room_id,
            matches[0],
            attempt_seed,
        ) {
            out.say(block.message);
            if let Some(text) = block.attack_text {
                out.say(text);
            }
            if !block.attack_effects.is_empty() {
                apply_effects(flags, &block.attack_effects);
            }
            return true;
        }
        return do_move(out, current_room_id, world, matches[0]);
    } else if matches.len() > 1 {
        let dirs_list = matches
            .iter()
            .map(|e| e.direction.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        out.say(format!(
            "That movement is ambiguous here. Did you mean: {}?",
            dirs_list
        ));
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
            .map(|c| {
                abbrev_chars
                    .iter()
                    .any(|ac| ac.to_ascii_lowercase() == c.to_ascii_lowercase())
            })
            .unwrap_or(false);

        let hit_verb = exit.verbs.iter().any(|v| {
            v.chars()
                .next()
                .map(|c| {
                    abbrev_chars
                        .iter()
                        .any(|ac| ac.to_ascii_lowercase() == c.to_ascii_lowercase())
                })
                .unwrap_or(false)
        });

        if hit_dir || hit_verb {
            abbrev_matches.push(exit);
        }
    }

    match abbrev_matches.len() {
        0 => false,
        1 => {
            if let Some(block) = movement_blocked_by_npc(
                world,
                npc_locations,
                flags,
                current_room_id,
                abbrev_matches[0],
                attempt_seed,
            ) {
                out.say(block.message);
                if let Some(text) = block.attack_text {
                    out.say(text);
                }
                if !block.attack_effects.is_empty() {
                    apply_effects(flags, &block.attack_effects);
                }
                true
            } else {
                do_move(out, current_room_id, world, abbrev_matches[0])
            }
        }
        _ => {
            let dirs_list = abbrev_matches
                .iter()
                .map(|e| e.direction.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            out.say(format!(
                "That direction is ambiguous here. Did you mean: {}?",
                dirs_list
            ));
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

struct BlockOutcome {
    message: String,
    attack_text: Option<String>,
    attack_effects: Vec<String>,
}

fn movement_blocked_by_npc(
    world: &world::World,
    npc_locations: &HashMap<String, String>,
    flags: &HashSet<String>,
    current_room_id: &str,
    attempted_exit: &world::Exit,
    attempt_seed: u64,
) -> Option<BlockOutcome> {
    for npc in world.npcs.values() {
        if !npc.block_movement {
            continue;
        }

        let room_id = match npc_locations.get(&npc.id) {
            Some(r) => r,
            None => continue,
        };

        if room_id != current_room_id {
            continue;
        }

        // NPC must be visible and any block-specific conditions must be satisfied.
        if !conditions_met(&npc.conditions, flags) {
            continue;
        }

        if !npc.block_conditions.is_empty() && !conditions_met(&npc.block_conditions, flags) {
            continue;
        }

        // If block_exits is non-empty, require match with attempted exit direction or verbs.
        if !npc.block_exits.is_empty() {
            let dir = attempted_exit.direction.to_ascii_lowercase();
            let mut matches_exit = npc.block_exits.iter().any(|b| b.eq_ignore_ascii_case(&dir));

            if !matches_exit {
                matches_exit = attempted_exit
                    .verbs
                    .iter()
                    .any(|v| npc.block_exits.iter().any(|b| b.eq_ignore_ascii_case(v)));
            }

            if !matches_exit {
                continue;
            }
        }

        let message = match &npc.block_text {
            Some(t) if !t.trim().is_empty() => t.trim().to_string(),
            _ => format!("{} blocks your way.", npc.name),
        };

        // Optional attack
        let (attack_text, attack_effects) = if npc.foe && npc.attack_chance_percent > 0 {
            let roll = stable_roll_percent(attempt_seed, &npc.id);
            if roll < npc.attack_chance_percent as u64 {
                let text = npc
                    .attack_text
                    .as_deref()
                    .and_then(|t| {
                        let trimmed = t.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_string())
                        }
                    })
                    .unwrap_or_else(|| format!("{} strikes at you!", npc.name));
                (Some(text), npc.attack_effects.clone())
            } else {
                (None, Vec::new())
            }
        } else {
            (None, Vec::new())
        };

        return Some(BlockOutcome {
            message,
            attack_text,
            attack_effects,
        });
    }

    None
}

fn stable_roll_percent(turn_index: u64, npc_id: &str) -> u64 {
    // 0..=99 deterministic per turn/NPC; not cryptographic.
    stable_hash_u64(turn_index, npc_id) % 100
}

fn stable_hash_u64(turn_index: u64, s: &str) -> u64 {
    let mut h = 1469598103934665603u64 ^ turn_index;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(1099511628211u64);
    }
    h
}
