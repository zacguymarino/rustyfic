use std::collections::{HashMap, HashSet};

use crate::world;
use crate::engine::conditions::conditions_met;
use crate::engine::output::Output;

pub fn render_room(
    out: &mut Output,
    room: &world::Room,
    flags: &HashSet<String>,
    world: &world::World,
    item_locations: &HashMap<String, world::ItemLocation>,
) {
    use world::ItemLocation;

    let mut room_desc = String::new();

    out.title(room.name.clone());

    room_desc.push_str(room.desc.trim());

    for state_desc in &room.state_descs {
        if conditions_met(&state_desc.conditions, flags) {
            let txt = state_desc.text.trim();
            if !txt.is_empty() {
                if !room_desc.is_empty() {
                    room_desc.push(' ');
                }
                room_desc.push_str(txt);
            }
        }
    }

    for item in world.items.values() {
        let loc = match item_locations.get(&item.id) {
            Some(l) => l,
            None => continue,
        };

        if let ItemLocation::Room(room_id) = loc {
            if room_id == &room.id && conditions_met(&item.conditions, flags) {
                let txt = item.room_text.trim();
                if !txt.is_empty() {
                    if !room_desc.is_empty() {
                        room_desc.push(' ');
                    }
                    room_desc.push_str(txt);
                }
            }
        }
    }

    out.say(room_desc);

    let visible_exits: Vec<&world::Exit> = room
        .exits
        .iter()
        .filter(|e| conditions_met(&e.conditions, flags))
        .collect();

    if visible_exits.is_empty() {
        out.set_exits("Exits: (none)");
    } else {
        let mut dirs: Vec<&String> = visible_exits.iter().map(|e| &e.direction).collect();
        dirs.sort();
        dirs.dedup();
        let list = dirs
            .into_iter()
            .map(|d| d.as_str())
            .collect::<Vec<&str>>()
            .join(", ");
        out.set_exits(format!("Exits: {}", list));
    }
}

pub fn room_depends_on_any_flag(
    room: &world::Room,
    world: &world::World,
    item_locations: &HashMap<String, world::ItemLocation>,
    flags_changed: &HashSet<String>,
) -> bool {
    use world::{ItemKind, ItemLocation};

    // Helper: does any condition string mention a changed flag (with or without '!')?
    fn conds_touch_changed(conds: &[String], changed: &HashSet<String>) -> bool {
        conds.iter().any(|c| {
            let t = c.trim();
            if t.is_empty() {
                return false;
            }
            let name = t.trim_start_matches('!').trim();
            !name.is_empty() && changed.contains(name)
        })
    }

    // room.state_desc conditions
    for sd in &room.state_descs {
        if conds_touch_changed(&sd.conditions, flags_changed) {
            return true;
        }
    }

    // exit conditions
    for ex in &room.exits {
        if conds_touch_changed(&ex.conditions, flags_changed) {
            return true;
        }
    }

    // Items in THIS room:
    // - direct room items whose own conditions touch changed flags
    // - containers in the room whose container-conditions touch changed flags
    // - items inside those containers whose own conditions touch changed flags
    for item in world.items.values() {
        match item_locations.get(&item.id) {
            Some(ItemLocation::Room(room_id)) if room_id == &room.id => {
                // direct item visibility
                if conds_touch_changed(&item.conditions, flags_changed) {
                    return true;
                }

                // if it's a container in this room, its "open/closed" gating may depend on flags
                if let ItemKind::Container(props) = &item.kind {
                    if conds_touch_changed(&props.conditions, flags_changed) {
                        return true;
                    }

                    // contents whose visibility conditions depend on changed flags
                    for inner in world.items.values() {
                        match item_locations.get(&inner.id) {
                            Some(ItemLocation::Item(parent_id)) if parent_id == &item.id => {
                                if conds_touch_changed(&inner.conditions, flags_changed) {
                                    return true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    false
}
