use std::collections::HashSet;

use crate::engine::output::Output;
use crate::world;
use crate::engine::helpers::apply_effects;

/// Returns true if all conditions are satisfied.
/// Condition syntax:
/// - "flag" means the flag must be present
/// - "!flag" means the flag must NOT be present
pub fn conditions_met(conditions: &[String], flags: &HashSet<String>) -> bool {
    for cond in conditions {
        if let Some(name) = cond.strip_prefix('!') {
            // Negated condition: flag must NOT be present
            if flags.contains(name) {
                return false;
            }
        } else {
            // Positive condition: flag must be present
            if !flags.contains(cond) {
                return false;
            }
        }
    }
    true
}

/// Evaluate and fire any global conditions that are satisfied.
/// This may print events and apply effects (flags add/remove).
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

        apply_effects(flags, &gc.effects);

        if gc.one_shot {
            fired.insert(gc.id.clone());
        }
    }
}
