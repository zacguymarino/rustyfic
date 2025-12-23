use std::collections::{HashMap, HashSet};

use crate::engine::actions::evaluate_actions_for_input;
use crate::engine::conditions::conditions_met;
use crate::engine::helpers::apply_effects;
use crate::engine::output::Output;
use crate::world;
use crate::world::ItemLocation;

pub enum NpcMatch<'a> {
    None,
    One(&'a world::Npc),
    Many(Vec<&'a world::Npc>),
}

fn tokenize(input: &str) -> Vec<String> {
    input.split_whitespace().map(|t| t.to_lowercase()).collect()
}

fn npc_visible(npc: &world::Npc, flags: &HashSet<String>) -> bool {
    conditions_met(&npc.conditions, flags)
}

/// Basic full-word overlap scoring on name + aliases (same spirit as items)
pub(crate) fn find_npc_by_words_scored<'a>(
    world: &'a world::World,
    npc_locations: &HashMap<String, String>,
    flags: &HashSet<String>,
    current_room_id: &str,
    query: &str,
) -> NpcMatch<'a> {
    let query_words: Vec<String> = query
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect();

    if query_words.is_empty() {
        return NpcMatch::None;
    }

    let mut scored: Vec<(&world::Npc, usize)> = Vec::new();

    for npc in world.npcs.values() {
        let room_id = match npc_locations.get(&npc.id) {
            Some(r) => r,
            None => continue,
        };

        if room_id != current_room_id {
            continue;
        }

        if !npc_visible(npc, flags) {
            continue;
        }

        let mut all_words: Vec<String> = Vec::new();
        all_words.extend(
            npc.name
                .split_whitespace()
                .filter(|w| !w.is_empty())
                .map(|w| w.to_lowercase()),
        );
        for alias in &npc.aliases {
            all_words.extend(
                alias
                    .split_whitespace()
                    .filter(|w| !w.is_empty())
                    .map(|w| w.to_lowercase()),
            );
        }

        let mut score = 0usize;
        for qw in &query_words {
            if all_words.iter().any(|nw| nw == qw) {
                score += 1;
            }
        }

        if score > 0 {
            scored.push((npc, score));
        }
    }

    if scored.is_empty() {
        return NpcMatch::None;
    }

    let max_score = scored.iter().map(|(_, s)| *s).max().unwrap();
    let mut best: Vec<&world::Npc> = scored
        .into_iter()
        .filter(|(_, s)| *s == max_score)
        .map(|(n, _)| n)
        .collect();

    match best.len() {
        1 => NpcMatch::One(best[0]),
        _ => {
            best.sort_by(|a, b| a.name.cmp(&b.name));
            NpcMatch::Many(best)
        }
    }
}

/// Try to handle NPC interactions using the existing Action evaluator.
/// This triggers only when the input mentions the NPC (via name word overlap).
pub fn try_handle_npc_action(
    out: &mut Output,
    input: &str,
    world: &world::World,
    item_locations: &mut HashMap<String, world::ItemLocation>,
    npc_locations: &HashMap<String, String>,
    current_room_id: &str,
    flags: &mut HashSet<String>,
) -> bool {
    let tokens = tokenize(input);
    if tokens.is_empty() {
        return false;
    }

    // Find which NPC the player is addressing in this room.
    let npc_match = find_npc_by_words_scored(world, npc_locations, flags, current_room_id, input);

    let npc = match npc_match {
        NpcMatch::None => return false,
        NpcMatch::Many(_) => {
            out.say("Be more specific.");
            return true;
        }
        NpcMatch::One(n) => n,
    };

    // Evaluate that NPC's actions using the existing engine evaluator
    let (exec, msg, handled) = evaluate_actions_for_input(
        &npc.actions,
        input,
        world,
        item_locations,
        current_room_id,
        flags,
    );

    if let Some(action) = exec {
        let txt = action.response.trim();
        if !txt.is_empty() {
            out.say(txt);
        }
        apply_effects(flags, &action.effects);

        // Consume required inventory items by removing their location entries entirely.
        // This prevents taking them back after a successful NPC action (e.g., bribe).
        for req in &action.requires_inventory {
            item_locations.remove(req);
        }

        return true;
    }

    if let Some(m) = msg {
        out.say(m);
        return true;
    }

    handled
}

/// Examine NPCs in the current room.
pub fn try_handle_examine_npc(
    out: &mut Output,
    item_locations: &HashMap<String, ItemLocation>,
    world: &world::World,
    npc_locations: &HashMap<String, String>,
    current_room_id: &str,
    target_name: &str,
    flags: &HashSet<String>,
) -> bool {
    let query = target_name.trim().to_lowercase();
    if query.is_empty() {
        return false;
    }

    let npc_match = find_npc_by_words_scored(world, npc_locations, flags, current_room_id, &query);

    let npc = match npc_match {
        NpcMatch::None => return false,
        NpcMatch::Many(_) => {
            out.say("Be more specific.");
            return true;
        }
        NpcMatch::One(n) => n,
    };

    let txt = npc.examine_text.trim();
    if txt.is_empty() {
        out.say(format!("You see nothing special about {}.", npc.name));
    } else {
        out.say(txt);
    }

    // List visible items held by this NPC.
    let mut held: Vec<&world::Item> = Vec::new();
    for item in world.items.values() {
        if let Some(ItemLocation::Npc(holder)) = item_locations.get(&item.id) {
            if holder == &npc.id && conditions_met(&item.conditions, flags) {
                held.push(item);
            }
        }
    }

    if !held.is_empty() {
        held.sort_by(|a, b| a.name.cmp(&b.name));
        let list = held
            .iter()
            .map(|i| i.name.as_str())
            .collect::<Vec<&str>>()
            .join(", ");
        out.say(format!("{} is holding: {}.", npc.name, list));
    }

    true
}

/// Simple dialogue handler: triggers the first matching dialogue entry for the NPC.
/// Returns true if handled (even if no dialogue available), false if no NPC match.
pub fn handle_talk_to_npc(
    out: &mut Output,
    world: &world::World,
    npc_locations: &HashMap<String, String>,
    current_room_id: &str,
    target_name: &str,
    flags: &mut HashSet<String>,
    fired_dialogues: &mut HashSet<String>,
) -> bool {
    let query = target_name.trim().to_lowercase();
    if query.is_empty() {
        out.say("Talk to whom?");
        return true;
    }

    let npc_match = find_npc_by_words_scored(world, npc_locations, flags, current_room_id, &query);

    let npc = match npc_match {
        NpcMatch::None => return false,
        NpcMatch::Many(_) => {
            out.say("Be more specific.");
            return true;
        }
        NpcMatch::One(n) => n,
    };

    if npc.dialogue.is_empty() {
        out.say(format!("{} has nothing to say.", npc.name));
        return true;
    }

    for dlg in &npc.dialogue {
        if !conditions_met(&dlg.conditions, flags) {
            continue;
        }

        let key = format!("{}::{}", npc.id, dlg.id);
        if dlg.one_shot && fired_dialogues.contains(&key) {
            continue;
        }

        let txt = dlg.response.trim();
        if !txt.is_empty() {
            out.say(txt);
        }
        apply_effects(flags, &dlg.effects);

        if dlg.one_shot {
            fired_dialogues.insert(key);
        }

        return true;
    }

    out.say(format!("{} has nothing new to say.", npc.name));
    true
}

/// Deterministic roaming after a successful player move.
/// - Called ONLY when the player actually changes rooms.
/// - Uses (turn_index, npc_id) to pick whether the NPC moves and to which allowed room.
/// - No output; appearance is handled naturally by room rendering.
pub fn roam_npcs_after_player_move(
    world: &world::World,
    npc_locations: &mut HashMap<String, String>,
    flags: &HashSet<String>,
    turn_index: u64,
) {
    for npc in world.npcs.values() {
        let roam = match &npc.roam {
            Some(r) if r.enabled && !r.allowed_rooms.is_empty() && r.chance_percent > 0 => r,
            _ => continue,
        };

        // If NPC is not currently visible due to flags, we still allow it to roam;
        // visibility is handled at render time.
        let _ = flags; // explicit: we don't need flags here today

        let roll = deterministic_roll_percent(turn_index, &npc.id);
        if roll >= roam.chance_percent as u64 {
            continue;
        }

        let idx = deterministic_index(turn_index, &npc.id, roam.allowed_rooms.len());
        let target_room = roam.allowed_rooms[idx].clone();

        // Only move if target exists (author error safe-guard)
        if world.rooms.contains_key(&target_room) {
            npc_locations.insert(npc.id.clone(), target_room);
        }
    }
}

fn deterministic_roll_percent(turn_index: u64, npc_id: &str) -> u64 {
    // 0..=99
    (stable_hash_u64(turn_index, npc_id) % 100) as u64
}

fn deterministic_index(turn_index: u64, npc_id: &str, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (stable_hash_u64(turn_index.wrapping_add(999), npc_id) % (len as u64)) as usize
}

fn stable_hash_u64(turn_index: u64, s: &str) -> u64 {
    // Simple stable hash: not cryptographic, just deterministic.
    let mut h = 1469598103934665603u64 ^ turn_index;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(1099511628211u64);
    }
    h
}
