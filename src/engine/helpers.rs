use std::collections::{HashMap, HashSet};

use crate::engine::conditions::conditions_met;
use crate::world;

/// Apply a list of effects to flags.
/// - "flag"  => insert
/// - "!flag" => remove
pub fn apply_effects(flags: &mut HashSet<String>, effects: &[String]) {
    for eff in effects {
        if let Some(name) = eff.strip_prefix('!') {
            flags.remove(name);
        } else {
            flags.insert(eff.clone());
        }
    }
}

/// Returns true if the item's *visibility* conditions are satisfied.
pub fn item_visible(item: &world::Item, flags: &HashSet<String>) -> bool {
    conditions_met(&item.conditions, flags)
}

pub fn item_in_room(
    item_id: &str,
    item_locations: &HashMap<String, world::ItemLocation>,
    room_id: &str,
) -> bool {
    match item_locations.get(item_id) {
        Some(world::ItemLocation::Room(r)) => r == room_id,
        _ => false,
    }
}

pub fn item_in_inventory(
    item_id: &str,
    item_locations: &HashMap<String, world::ItemLocation>,
) -> bool {
    matches!(
        item_locations.get(item_id),
        Some(world::ItemLocation::Inventory)
    )
}
