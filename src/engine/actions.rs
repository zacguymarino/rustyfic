use std::collections::{HashMap, HashSet};

use crate::world;
use crate::engine::conditions::conditions_met;
use crate::engine::output::Output;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionBlockReason {
    MissingInventory,
    MissingScope,
    BlockedByConditions,
    Ambiguous,
}

pub fn evaluate_global_conditions(
    out: &mut Output,
    world: &world::World,
    flags: &mut HashSet<String>,
    current_room_id: &str,
    fired: &mut HashSet<String>,
) {
    for gc in &world.global_conditions {
        if gc.one_shot && fired.contains(&gc.id) {
            continue;
        }

        if !conditions_met(&gc.conditions, flags) {
            continue;
        }

        if !gc.allowed_rooms.is_empty() && !gc.allowed_rooms.iter().any(|r| r == current_room_id) {
            continue;
        }

        if gc.disallowed_rooms.iter().any(|r| r == current_room_id) {
            continue;
        }

        let txt = gc.response.trim();
        if !txt.is_empty() {
            out.event(txt.to_string());
        }

        for eff in &gc.effects {
            if let Some(name) = eff.strip_prefix('!') {
                flags.remove(name);
            } else {
                flags.insert(eff.clone());
            }
        }

        if gc.one_shot {
            fired.insert(gc.id.clone());
        }
    }
}

pub fn try_handle_action(
    out: &mut Output,
    room: &world::Room,
    input: &str,
    world: &world::World,
    item_locations: &HashMap<String, world::ItemLocation>,
    current_room_id: &str,
    flags: &mut HashSet<String>,
) -> bool {
    let (exec, msg, handled) = evaluate_actions_for_input(
        &room.actions,
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

        for eff in &action.effects {
            if let Some(name) = eff.strip_prefix('!') {
                flags.remove(name);
            } else {
                flags.insert(eff.clone());
            }
        }
        return true;
    }

    if let Some(m) = msg {
        out.say(m);
        return true;
    }

    handled
}

pub fn try_handle_global_action(
    out: &mut Output,
    input: &str,
    world: &world::World,
    item_locations: &HashMap<String, world::ItemLocation>,
    current_room_id: &str,
    flags: &mut HashSet<String>,
) -> bool {
    let (exec, msg, handled) = evaluate_actions_for_input(
        &world.global_actions,
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

        for eff in &action.effects {
            if let Some(name) = eff.strip_prefix('!') {
                flags.remove(name);
            } else {
                flags.insert(eff.clone());
            }
        }
        return true;
    }

    if let Some(m) = msg {
        out.say(m);
        return true;
    }

    handled
}

fn tokenize(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .map(|t| t.to_lowercase())
        .collect()
}

/// Phrase matches if ALL words in phrase appear as full tokens (order-independent).
fn phrase_matches_tokens(phrase: &str, tokens: &[String]) -> bool {
    let words: Vec<String> = phrase
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect();

    if words.is_empty() {
        return false;
    }

    words.iter().all(|w| tokens.iter().any(|t| t == w))
}

/// Returns how many words matched (for scoring), or 0 if phrase doesn't match.
fn phrase_match_score(phrase: &str, tokens: &[String]) -> usize {
    if phrase_matches_tokens(phrase, tokens) {
        phrase.split_whitespace().filter(|w| !w.is_empty()).count()
    } else {
        0
    }
}

fn item_in_room(
    item_id: &str,
    item_locations: &HashMap<String, world::ItemLocation>,
    room_id: &str,
) -> bool {
    match item_locations.get(item_id) {
        Some(world::ItemLocation::Room(r)) => r == room_id,
        _ => false,
    }
}

fn item_in_inventory(
    item_id: &str,
    item_locations: &HashMap<String, world::ItemLocation>,
) -> bool {
    matches!(item_locations.get(item_id), Some(world::ItemLocation::Inventory))
}

/// Require that the player's input mentions the required item (weakly) by default name words.
/// Rule: at least ONE word from item.name must appear as a token.
fn input_mentions_item_name(item: &world::Item, tokens: &[String]) -> bool {
    let name_words: Vec<String> = item
        .name
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect();

    name_words
        .iter()
        .any(|nw| tokens.iter().any(|t| t == nw))
}

fn missing_inventory_message(action: &world::Action, world: &world::World) -> String {
    if action.requires_inventory.is_empty() {
        return "You don't have what you need.".to_string();
    }
    let mut names: Vec<String> = Vec::new();
    for id in &action.requires_inventory {
        if let Some(it) = world.items.get(id) {
            names.push(it.name.clone());
        } else {
            names.push(id.clone());
        }
    }
    if names.len() == 1 {
        format!("You need the {}.", names[0])
    } else {
        format!("You need: {}.", names.join(", "))
    }
}

fn missing_scope_message(action: &world::Action, world: &world::World) -> String {
    if action.scope_requirements.is_empty() {
        return "You don't see that here.".to_string();
    }

    let mut names: Vec<String> = Vec::new();
    for id in &action.scope_requirements {
        if let Some(it) = world.items.get(id) {
            names.push(it.name.clone());
        } else {
            names.push(id.clone());
        }
    }

    if names.len() == 1 {
        format!("You don't see the {} here.", names[0])
    } else {
        format!("You don't see those here: {}.", names.join(", "))
    }
}

fn evaluate_actions_for_input<'a>(
    actions: &'a [world::Action],
    input: &str,
    world: &'a world::World,
    item_locations: &HashMap<String, world::ItemLocation>,
    current_room_id: &str,
    flags: &HashSet<String>,
) -> (Option<&'a world::Action>, Option<String>, bool) {
    let tokens = tokenize(input);
    if tokens.is_empty() {
        return (None, None, false);
    }

    // Track best executable actions
    let mut best_exec_score = 0usize;
    let mut best_exec: Vec<&world::Action> = Vec::new();

    // Track best blocked attempt (only if intent is strong)
    let mut best_blocked: Option<(usize, ActionBlockReason, String)> = None;

    for action in actions {
        // --- Verb match ---
        let verb_score = action
            .verbs
            .iter()
            .map(|v| phrase_match_score(v, &tokens))
            .max()
            .unwrap_or(0);

        if verb_score == 0 {
            continue;
        }

        // --- Noun match (optional) ---
        let noun_score = if action.nouns.is_empty() {
            0
        } else {
            let best = action
                .nouns
                .iter()
                .map(|n| phrase_match_score(n, &tokens))
                .max()
                .unwrap_or(0);

            if best == 0 {
                // If nouns exist and none match, intent is weak; don't "catch" the command.
                continue;
            }
            best
        };

        // --- Scope requirements (optional) ---
        let mut scope_ok = true;
        let mut scope_mentioned_ok = true;
        let mut scope_score = 0usize;

        for req_id in &action.scope_requirements {
            let item = match world.items.get(req_id) {
                Some(i) => i,
                None => {
                    scope_ok = false;
                    scope_mentioned_ok = false;
                    break;
                }
            };

            if !item_in_room(req_id, item_locations, current_room_id) {
                scope_ok = false;
                // still may be "attempting", but scope isn't satisfied
            }

            // Require at least ONE name-word in input to avoid hijacking
            if !input_mentions_item_name(item, &tokens) {
                scope_mentioned_ok = false;
            } else {
                scope_score += 3;
            }
        }

        // --- Inventory requirements (optional) ---
        let mut inv_ok = true;
        let mut inv_score = 0usize;

        for inv_id in &action.requires_inventory {
            if !item_in_inventory(inv_id, item_locations) {
                inv_ok = false;
            } else {
                inv_score += 2;
            }
        }

        // --- Conditions ---
        let cond_ok = conditions_met(&action.conditions, flags);

        // Strong intent definition:
        // verb matched + (nouns ok if required) + (if scope reqs exist, the player mentioned them)
        let intent_strong = if action.scope_requirements.is_empty() {
            true
        } else {
            scope_mentioned_ok
        };

        // Total score (for selecting best candidate)
        let total_score = verb_score + noun_score + scope_score + inv_score;

        // If fully executable, consider it for execution
        if intent_strong && scope_ok && inv_ok && cond_ok {
            if total_score > best_exec_score {
                best_exec_score = total_score;
                best_exec.clear();
                best_exec.push(action);
            } else if total_score == best_exec_score {
                best_exec.push(action);
            }
            continue;
        }

        // If not executable, consider giving feedback — but only if intent is strong
        if intent_strong {
            let (reason, msg) = if !inv_ok {
                (
                    ActionBlockReason::MissingInventory,
                    missing_inventory_message(action, world),
                )
            } else if !scope_ok {
                (
                    ActionBlockReason::MissingScope,
                    missing_scope_message(action, world),
                )
            } else if !cond_ok {
                (
                    ActionBlockReason::BlockedByConditions,
                    "You can't do that right now.".to_string(),
                )
            } else {
                // Shouldn't happen often, but keep a safe default
                (
                    ActionBlockReason::BlockedByConditions,
                    "You can't do that.".to_string(),
                )
            };

            // Prefer: higher score; tie-break by "more specific" reasons
            let reason_rank = match reason {
                ActionBlockReason::MissingInventory => 3,
                ActionBlockReason::MissingScope => 2,
                ActionBlockReason::BlockedByConditions => 1,
                ActionBlockReason::Ambiguous => 0,
            };

            let candidate = (total_score * 10 + reason_rank, reason, msg);

            match &best_blocked {
                None => best_blocked = Some(candidate),
                Some((best_key, _, _)) => {
                    if candidate.0 > *best_key {
                        best_blocked = Some(candidate);
                    }
                }
            }
        }
    }

    // Resolve execution vs ambiguity
    if best_exec.len() == 1 {
        return (Some(best_exec[0]), None, true);
    } else if best_exec.len() > 1 {
        return (None, Some("Be more specific.".to_string()), true);
    }

    // No executable: return best blocked message if present
    if let Some((_key, _reason, msg)) = best_blocked {
        return (None, Some(msg), true);
    }

    // No match at all
    (None, None, false)
}
