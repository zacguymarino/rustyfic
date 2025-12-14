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
