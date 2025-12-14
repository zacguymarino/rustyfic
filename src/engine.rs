use std::collections::{HashMap, HashSet};

use crate::world;

#[derive(Debug, Clone)]
pub enum OutputBlock {
    Title(String),
    Text(String),
    Event(String),
    Exits(String),
}

#[derive(Default, Debug)]
pub struct Output {
    pub blocks: Vec<OutputBlock>,
}

impl Output {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(&mut self, s: impl Into<String>) {
        let s = s.into();
        if !s.trim().is_empty() {
            self.blocks.push(OutputBlock::Title(s));
        }
    }

    pub fn say(&mut self, s: impl Into<String>) {
        let s = s.into();
        if !s.trim().is_empty() {
            self.blocks.push(OutputBlock::Text(s));
        }
    }

    pub fn event(&mut self, s: impl Into<String>) {
        let s = s.into();
        if !s.trim().is_empty() {
            self.blocks.push(OutputBlock::Event(s));
        }
    }

    pub fn set_exits(&mut self, s: impl Into<String>) {
        let s = s.into();
        if s.trim().is_empty() {
            return;
        }

        // ensure only one Exits block exists, always last
        self.blocks.retain(|b| !matches!(b, OutputBlock::Exits(_)));
        self.blocks.push(OutputBlock::Exits(s));
    }
}

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

/// Try to handle a custom action in a room: e.g. "climb tree", "sleep", "sit chair".
/// Returns true if an action was matched and executed.
pub fn try_handle_action(
    out: &mut Output,
    room: &world::Room,
    input: &str,
    flags: &mut HashSet<String>,
) -> bool {
    let mut parts = input.split_whitespace();
    let verb = match parts.next() {
        Some(v) => v,
        None => return false,
    };
    let noun = parts.next();

    for action in &room.actions {
        // 1) Check conditions first
        if !conditions_met(&action.conditions, flags) {
            continue;
        }

        // 2) Does verb match?
        let verb_match = action
            .verbs
            .iter()
            .any(|v| v.eq_ignore_ascii_case(verb));

        if !verb_match {
            continue;
        }

        // 3) Does noun match (if needed)?
        let noun_match = if action.nouns.is_empty() {
            true
        } else if let Some(user_noun) = noun {
            action
                .nouns
                .iter()
                .any(|n| n.eq_ignore_ascii_case(user_noun))
        } else {
            false
        };

        if verb_match && noun_match {
            out.say(format!("{}", action.response.trim()));

            // 4) Apply effects: add/remove flags
            for eff in &action.effects {
                if let Some(name) = eff.strip_prefix('!') {
                    flags.remove(name);
                } else {
                    flags.insert(eff.clone());
                }
            }

            return true;
        }
    }

    false
}

enum ItemMatch<'a> {
    None,
    One(&'a world::Item),
    Many(Vec<&'a world::Item>),
}

/// Find the *best* matching item by counting full-word overlaps.
/// - Highest score wins
/// - Ties => Many (ambiguity)
/// - Score 0 => None
fn find_item_by_words_scored<'a, F>(
    world: &'a world::World,
    item_locations: &HashMap<String, world::ItemLocation>,
    flags: &HashSet<String>,
    query: &str,
    filter: F,
) -> ItemMatch<'a>
where
    F: Fn(&'a world::Item, &world::ItemLocation) -> bool,
{
    let query_words: Vec<String> = query
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect();

    if query_words.is_empty() {
        return ItemMatch::None;
    }

    // (item, score)
    let mut scored: Vec<(&world::Item, usize)> = Vec::new();

    for item in world.items.values() {
        let loc = match item_locations.get(&item.id) {
            Some(l) => l,
            None => continue,
        };

        if !filter(item, loc) {
            continue;
        }

        // Respect item conditions (same behavior as before)
        if !conditions_met(&item.conditions, flags) {
            continue;
        }

        let name_lower = item.name.to_lowercase();
        let name_words: Vec<&str> = name_lower.split_whitespace().collect();

        // Score = number of query words that appear in the item's name words
        let mut score = 0usize;
        for qw in &query_words {
            if name_words.iter().any(|iw| iw == qw) {
                score += 1;
            }
        }

        if score > 0 {
            scored.push((item, score));
        }
    }

    if scored.is_empty() {
        return ItemMatch::None;
    }

    // Find max score
    let max_score = scored.iter().map(|(_, s)| *s).max().unwrap();

    // All items with max score
    let mut best: Vec<&world::Item> = scored
        .into_iter()
        .filter(|(_, s)| *s == max_score)
        .map(|(i, _)| i)
        .collect();

    match best.len() {
        0 => ItemMatch::None,
        1 => ItemMatch::One(best[0]),
        _ => {
            // Optional: sort to make stable
            best.sort_by(|a, b| a.name.cmp(&b.name));
            ItemMatch::Many(best)
        }
    }
}

pub fn handle_inventory(out: &mut Output, world: &world::World, item_locations: &HashMap<String, world::ItemLocation>) {
    use world::ItemLocation;

    let mut carried: Vec<&world::Item> = world
        .items
        .values()
        .filter(|item| matches!(item_locations.get(&item.id), Some(ItemLocation::Inventory)))
        .collect();

    if carried.is_empty() {
        out.say("You are carrying nothing.");
        return;
    }

    carried.sort_by(|a, b| a.name.cmp(&b.name));

    out.say("You are carrying:");
    for item in carried {
        let txt = item.inventory_text.trim();
        if txt.is_empty() {
            out.say(format!("  {}", item.name));
        } else {
            out.say(format!("  {}", txt));
        }
    }
}

pub fn handle_take(
    out: &mut Output,
    item_locations: &mut HashMap<String, world::ItemLocation>,
    world: &world::World,
    current_room_id: &str,
    target_name: &str,
    flags: &HashSet<String>,
) {
    use world::{ItemLocation};

    let query = target_name.trim().to_lowercase();
    if query.is_empty() {
        out.say("Take what?");
        return;
    }

    let result = find_item_by_words_scored(
        world,
        item_locations,
        flags,
        &query,
        |_item, loc| match loc {
            ItemLocation::Room(room_id) => room_id == current_room_id,
            _ => false,
        },
    );

    let item = match result {
        ItemMatch::None => {
            out.say("You don't see that here.");
            return;
        }
        ItemMatch::Many(_) => {
            out.say("Be more specific.");
            return;
        }
        ItemMatch::One(i) => i,
    };

    if !item.portable {
        out.say(format!("You can't take the {}.", item.name));
        return;
    }

    item_locations.insert(item.id.clone(), ItemLocation::Inventory);
    out.say(format!("You take the {}.", item.name));
}

pub fn handle_take_all_room(
    out: &mut Output,
    item_locations: &mut HashMap<String, world::ItemLocation>,
    world: &world::World,
    current_room_id: &str,
    flags: &HashSet<String>,
) {
    use world::ItemLocation;

    // Collect IDs of items we *can* take.
    let mut to_take: Vec<String> = Vec::new();

    for item in world.items.values() {
        let loc = match item_locations.get(&item.id) {
            Some(l) => l,
            None => continue,
        };

        // Only items in this room, not inside containers
        if let ItemLocation::Room(room_id) = loc {
            if room_id == current_room_id
                && conditions_met(&item.conditions, flags)
                && item.portable
            {
                to_take.push(item.id.clone());
            }
        }
    }

    if to_take.is_empty() {
        out.say("There is nothing here you can take.");
        return;
    }

    for item_id in &to_take {
        if let Some(item) = world.items.get(item_id) {
            item_locations.insert(item_id.clone(), ItemLocation::Inventory);
            out.say(format!("You take the {}.", item.name));
        }
    }
}

pub fn handle_drop(
    out: &mut Output,
    item_locations: &mut HashMap<String, world::ItemLocation>,
    world: &world::World,
    current_room_id: &str,
    target_name: &str,
) {
    use world::ItemLocation;

    let query = target_name.trim().to_lowercase();
    if query.is_empty() {
        out.say("Drop what?");
        return;
    }

    // For dropping, we don't need flags/conditions, so we pass an empty set.
    let dummy_flags = HashSet::new();

    let result = find_item_by_words_scored(
        world,
        item_locations,
        &dummy_flags,
        &query,
        |_item, loc| matches!(loc, ItemLocation::Inventory),
    );

    let item = match result {
        ItemMatch::None => {
            out.say("You aren't carrying that.");
            return;
        }
        ItemMatch::Many(_) => {
            out.say("Be more specific.");
            return;
        }
        ItemMatch::One(i) => i,
    };

    item_locations.insert(
        item.id.clone(),
        ItemLocation::Room(current_room_id.to_string()),
    );
    out.say(format!("You drop the {}.", item.name));
}

pub fn handle_drop_all(
    out: &mut Output,
    item_locations: &mut HashMap<String, world::ItemLocation>,
    world: &world::World,
    current_room_id: &str,
) {
    use world::ItemLocation;

    // Collect all items currently in inventory that we’re allowed to drop.
    let mut to_drop: Vec<String> = Vec::new();

    for item in world.items.values() {
        let loc = match item_locations.get(&item.id) {
            Some(l) => l,
            None => continue,
        };

        if let ItemLocation::Inventory = loc {
            // Respect portable flag: if you somehow have a non-portable item in inventory,
            // we’ll refuse to drop it.
            if item.portable {
                to_drop.push(item.id.clone());
            }
        }
    }

    if to_drop.is_empty() {
        out.say("You aren't carrying anything you can drop.");
        return;
    }

    for item_id in &to_drop {
        if let Some(item) = world.items.get(item_id) {
            item_locations.insert(
                item_id.clone(),
                ItemLocation::Room(current_room_id.to_string()),
            );
            out.say(format!("You drop the {}.", item.name));
        }
    }
}

// pub fn handle_put(
//     out: &mut Output,
//     item_locations: &mut HashMap<String, world::ItemLocation>,
//     world: &world::World,
//     current_room_id: &str,
//     item_name: &str,
//     container_name: &str,
//     flags: &mut HashSet<String>,
// ) {
//     use world::{ItemKind, ItemLocation};

//     let item_query = item_name.trim().to_lowercase();
//     let container_query = container_name.trim().to_lowercase();

//     if item_query.is_empty() {
//         out.say("Put what?");
//         return;
//     }
//     if container_query.is_empty() {
//         out.say("Put it where?");
//         return;
//     }

//     // 1) Find the item in inventory
//     let item_result = find_item_by_words_scored(
//         world,
//         item_locations,
//         &HashSet::new(), // conditions not relevant for inventory membership here
//         &item_query,
//         |_item, loc| matches!(loc, ItemLocation::Inventory),
//     );

//     let item = match item_result {
//         ItemMatch::None => {
//             out.say("You aren't carrying anything like that.");
//             return;
//         }
//         ItemMatch::Many(_) => {
//             out.say("Be more specific about what to put.");
//             return;
//         }
//         ItemMatch::One(i) => i,
//     };

//     if !item.portable {
//         out.say(format!("You can't put the {} into anything.", item.name));
//         return;
//     }

//     // 2) Find the container (room or inventory)
//     let container_result = find_item_by_words_scored(
//         world,
//         item_locations,
//         &HashSet::new(),
//         &container_query,
//         |candidate, loc| {
//             matches!(candidate.kind, ItemKind::Container(_))
//                 && match loc {
//                     ItemLocation::Room(room_id) => room_id == current_room_id,
//                     ItemLocation::Inventory => true,
//                     _ => false,
//                 }
//         },
//     );

//     let (container, props) = match container_result {
//         ItemMatch::None => {
//             out.say("You don't see any container like that here.");
//             return;
//         }
//         ItemMatch::Many(_) => {
//             out.say("Be more specific about which container.");
//             return;
//         }
//         ItemMatch::One(it) => {
//             if let ItemKind::Container(ref props) = it.kind {
//                 (it, props)
//             } else {
//                 out.say("That isn't a container.");
//                 return;
//             }
//         }
//     };

//     // 3) Check container interaction conditions (e.g., open flag)
//     if !props.conditions.is_empty() && !conditions_met(&props.conditions, flags) {
//         out.say(format!("{}", props.closed_text.trim()));
//         return;
//     }

//     // 4) Check capacity
//     if let Some(cap) = props.capacity {
//         let mut count = 0usize;
//         for loc in item_locations.values() {
//             if let ItemLocation::Item(parent_id) = loc {
//                 if parent_id == &container.id {
//                     count += 1;
//                 }
//             }
//         }

//         if count >= cap {
//             out.say(format!("The {} is full.", container.name));
//             return;
//         }
//     }

//     // 5) Move the item
//     item_locations.insert(
//         item.id.clone(),
//         ItemLocation::Item(container.id.clone()),
//     );

//     out.say(format!("You put the {} into the {}.", item.name, container.name));

//     // 6) Check completion logic
//     check_container_completion(out, world, item_locations, flags, &container.id);
// }

pub fn handle_take_from_container(
    out: &mut Output,
    item_locations: &mut HashMap<String, world::ItemLocation>,
    world: &world::World,
    current_room_id: &str,
    item_name: &str,
    container_name: &str,
    flags: &HashSet<String>,
) {
    use world::{ItemKind, ItemLocation};

    let item_query = item_name.trim().to_lowercase();
    let container_query = container_name.trim().to_lowercase();

    if item_query.is_empty() {
        out.say("Take what?");
        return;
    }
    if container_query.is_empty() {
        out.say("Take it from where?");
        return;
    }

    // 1) Find the container
    let container_result = find_item_by_words_scored(
        world,
        item_locations,
        &HashSet::new(),
        &container_query,
        |candidate, loc| {
            matches!(candidate.kind, ItemKind::Container(_))
                && match loc {
                    ItemLocation::Room(room_id) => room_id == current_room_id,
                    ItemLocation::Inventory => true,
                    _ => false,
                }
        },
    );

    let (container, props) = match container_result {
        ItemMatch::None => {
            out.say("You don't see any container like that here.");
            return;
        }
        ItemMatch::Many(_) => {
            out.say("Be more specific about which container.");
            return;
        }
        ItemMatch::One(it) => {
            if let ItemKind::Container(ref props) = it.kind {
                (it, props)
            } else {
                out.say("That isn't a container.");
                return;
            }
        }
    };

    // 2) Check interaction conditions
    if !props.conditions.is_empty() && !conditions_met(&props.conditions, flags) {
        out.say(format!("{}", props.closed_text.trim()));
        return;
    }

    // 3) Find the item inside that container
    let item_result = find_item_by_words_scored(
        world,
        item_locations,
        flags,
        &item_query,
        |_candidate, loc| match loc {
            ItemLocation::Item(parent_id) => parent_id == &container.id,
            _ => false,
        },
    );

    let item = match item_result {
        ItemMatch::None => {
            out.say(format!(
                "You don't see anything like that in the {}.",
                container.name
            ));
            return;
        }
        ItemMatch::Many(_) => {
            out.say("Be more specific about what to take.");
            return;
        }
        ItemMatch::One(i) => i,
    };

    if !item.portable {
        out.say(format!("You can't take the {}.", item.name));
        return;
    }

    item_locations.insert(item.id.clone(), ItemLocation::Inventory);
    out.say(format!("You take the {} from the {}.", item.name, container.name));
}

pub fn handle_take_all_from_container(
    out: &mut Output,
    item_locations: &mut HashMap<String, world::ItemLocation>,
    world: &world::World,
    current_room_id: &str,
    container_name: &str,
    flags: &HashSet<String>,
) {
    use world::{ItemKind, ItemLocation};

    let container_query = container_name.trim().to_lowercase();
    if container_query.is_empty() {
        out.say("Take all from where?");
        return;
    }

    // 1) Find the container (room or inventory), using scored matching
    let container_match = find_item_by_words_scored(
        world,
        item_locations,
        &HashSet::new(),
        &container_query,
        |candidate, loc| {
            let in_scope = match loc {
                ItemLocation::Room(room_id) => room_id == current_room_id,
                ItemLocation::Inventory => true,
                _ => false,
            };

            if !in_scope {
                return false;
            }

            matches!(candidate.kind, ItemKind::Container(_))
        },
    );

    let container = match container_match {
        ItemMatch::None => {
            out.say("You don't see any container like that here.");
            return;
        }
        ItemMatch::Many(_) => {
            out.say("Be more specific about which container.");
            return;
        }
        ItemMatch::One(c) => c,
    };

    let props = match &container.kind {
        ItemKind::Container(p) => p,
        _ => unreachable!(),
    };

    // 2) Check container interaction conditions (open/closed)
    if !props.conditions.is_empty() && !conditions_met(&props.conditions, flags) {
        out.say(props.closed_text.trim());
        return;
    }

    // 3) Collect all portable, visible items inside the container
    let mut to_take: Vec<String> = Vec::new();

    for item in world.items.values() {
        let loc = match item_locations.get(&item.id) {
            Some(l) => l,
            None => continue,
        };

        if let ItemLocation::Item(parent_id) = loc {
            if parent_id == &container.id
                && conditions_met(&item.conditions, flags)
                && item.portable
            {
                to_take.push(item.id.clone());
            }
        }
    }

    if to_take.is_empty() {
        out.say(format!(
            "There is nothing in the {} you can take.",
            container.name
        ));
        return;
    }

    // 4) Move each item to inventory
    for item_id in &to_take {
        if let Some(item) = world.items.get(item_id) {
            item_locations.insert(item_id.clone(), ItemLocation::Inventory);
            out.say(format!(
                "You take the {} from the {}.",
                item.name,
                container.name
            ));
        }
    }
}

pub fn try_handle_container_store(
    out: &mut Output,
    verb: &str,
    rest: &str,
    item_locations: &mut HashMap<String, world::ItemLocation>,
    world: &world::World,
    current_room_id: &str,
    flags: &mut HashSet<String>,
) -> bool {
    use world::{ItemKind, ItemLocation};

    let verb_l = verb.trim().to_lowercase();
    if verb_l.is_empty() {
        return false;
    }

    // 1) Is there ANY visible container in scope that supports this verb?
    let mut any_container_supports = false;

    for c in world.items.values() {
        let loc = match item_locations.get(&c.id) {
            Some(l) => l,
            None => continue,
        };

        let in_scope = match loc {
            ItemLocation::Room(room_id) => room_id == current_room_id,
            ItemLocation::Inventory => true,
            _ => false,
        };

        if !in_scope {
            continue;
        }

        let props = match &c.kind {
            ItemKind::Container(p) => p,
            _ => continue,
        };

        if props.verbs.iter().any(|v| v.eq_ignore_ascii_case(&verb_l)) {
            any_container_supports = true;
            break;
        }
    }

    if !any_container_supports {
        return false; // not a container-store verb in this context; let other systems handle it
    }

    let query = rest.trim().to_lowercase();
    if query.is_empty() {
        out.say(format!("What do you want to {}?", verb_l));
        return true;
    }

    // 2) Find the carried item mentioned anywhere in the rest of the text
    let item_match = find_item_by_words_scored(
        world,
        item_locations,
        &HashSet::new(),
        &query,
        |_it, loc| matches!(loc, ItemLocation::Inventory),
    );

    let item = match item_match {
        ItemMatch::None => {
            out.say("You aren't carrying anything like that.");
            return true;
        }
        ItemMatch::Many(_) => {
            out.say(format!("Be more specific about what you want to {}.", verb_l));
            return true;
        }
        ItemMatch::One(i) => i,
    };

    if !item.portable {
        out.say(format!("You can't {} the {}.", verb_l, item.name));
        return true;
    }

    // 3) Find a container in scope that (a) matches query words AND (b) supports the verb
    let cont_match = find_item_by_words_scored(
        world,
        item_locations,
        &HashSet::new(),
        &query,
        |candidate, loc| {
            let in_scope = match loc {
                ItemLocation::Room(room_id) => room_id == current_room_id,
                ItemLocation::Inventory => true,
                _ => false,
            };

            if !in_scope {
                return false;
            }

            let props = match &candidate.kind {
                ItemKind::Container(p) => p,
                _ => return false,
            };

            props.verbs.iter().any(|v| v.eq_ignore_ascii_case(&verb_l))
        },
    );

    let container = match cont_match {
        ItemMatch::None => {
            out.say(format!("Where do you want to {} the {}?", verb_l, item.name));
            return true;
        }
        ItemMatch::Many(_) => {
            out.say(format!("Be more specific about where you want to {} it.", verb_l));
            return true;
        }
        ItemMatch::One(c) => c,
    };

    let props = match &container.kind {
        ItemKind::Container(p) => p,
        _ => unreachable!(),
    };

    // 4) Check container interaction conditions
    if !props.conditions.is_empty() && !conditions_met(&props.conditions, flags) {
        out.say(format!("{}", props.closed_text.trim()));
        return true;
    }

    // 5) Capacity
    if let Some(cap) = props.capacity {
        let mut count = 0usize;
        for loc in item_locations.values() {
            if let ItemLocation::Item(parent_id) = loc {
                if parent_id == &container.id {
                    count += 1;
                }
            }
        }
        if count >= cap {
            out.say(format!("The {} is full.", container.name));
            return true;
        }
    }

    // 6) Move item into container
    item_locations.insert(item.id.clone(), ItemLocation::Item(container.id.clone()));

    out.say(format!(
        "You {} the {} {} the {}.",
        verb_l,
        item.name,
        props.prep,
        container.name
    ));

    // 7) Completion check
    check_container_completion(out, world, item_locations, flags, &container.id);

    true
}

pub fn check_container_completion(
    out: &mut Output,
    world: &world::World,
    item_locations: &HashMap<String, world::ItemLocation>,
    flags: &mut HashSet<String>,
    container_id: &str,
) {
    use world::{ItemKind, ItemLocation};

    let container = match world.items.get(container_id) {
        Some(i) => i,
        None => return,
    };

    let props = match &container.kind {
        ItemKind::Container(props) => props,
        _ => return,
    };

    // If there's no complete_flag or no items to check, bail.
    let complete_flag = match &props.complete_flag {
        Some(f) => f,
        None => return,
    };

    if props.complete_when.is_empty() {
        return;
    }

    // If flag already set, don't re-check.
    if flags.contains(complete_flag) {
        return;
    }

    // All required items must currently be inside this container.
    for needed_id in &props.complete_when {
        match item_locations.get(needed_id) {
            Some(ItemLocation::Item(parent_id)) if parent_id == container_id => {
                // good
            }
            _ => {
                // missing or elsewhere
                return;
            }
        }
    }

    // All present: set the flag.
    flags.insert(complete_flag.clone());

    if let Some(text) = &props.complete_text {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            out.say(trimmed);
        }
    }
}

pub fn handle_examine(
    out: &mut Output,
    world: &world::World,
    item_locations: &HashMap<String, world::ItemLocation>,
    current_room_id: &str,
    target_name: &str,
    flags: &HashSet<String>,
) {
    use world::{ItemKind, ItemLocation};

    let query = target_name.trim().to_lowercase();
    if query.is_empty() {
        out.say("Examine what?");
        return;
    }

    // 1) Prefer items in inventory
    let inv_match = find_item_by_words_scored(
        world,
        item_locations,
        flags,
        &query,
        |_item, loc| matches!(loc, ItemLocation::Inventory),
    );

    let item = match inv_match {
        ItemMatch::Many(_) => {
            out.say("Be more specific.");
            return;
        }
        ItemMatch::One(i) => Some(i),
        ItemMatch::None => None,
    };

    // 2) If not in inventory, look in the room
    let item = match item {
        Some(i) => i,
        None => {
            let room_match = find_item_by_words_scored(
                world,
                item_locations,
                flags,
                &query,
                |_item, loc| match loc {
                    ItemLocation::Room(room_id) => room_id == current_room_id,
                    _ => false,
                },
            );

            match room_match {
                ItemMatch::None => {
                    out.say("You see nothing like that here.");
                    return;
                }
                ItemMatch::Many(_) => {
                    out.say("Be more specific.");
                    return;
                }
                ItemMatch::One(i) => i,
            }
        }
    };

    // Base examine text
    let txt = item.examine_text.trim();
    if txt.is_empty() {
        out.say(format!("You see nothing special about the {}.", item.name));
    } else {
        out.say(txt);
    }

    // If this item is a container, handle contents / closed logic
    if let ItemKind::Container(props) = &item.kind {
        // Closed?
        if !props.conditions.is_empty() && !conditions_met(&props.conditions, flags) {
            out.say(format!("{}", props.closed_text.trim()));
            return;
        }

        // List contents
        let mut contents: Vec<&world::Item> = Vec::new();

        for other in world.items.values() {
            let loc = match item_locations.get(&other.id) {
                Some(l) => l,
                None => continue,
            };

            if let ItemLocation::Item(parent_id) = loc {
                if parent_id == &item.id && conditions_met(&other.conditions, flags) {
                    contents.push(other);
                }
            }
        }

        if contents.is_empty() {
            out.say("It is currently empty.");
        } else {
            contents.sort_by(|a, b| a.name.cmp(&b.name));
            let list = contents
                .iter()
                .map(|i| i.name.as_str())
                .collect::<Vec<&str>>()
                .join(", ");
            out.say(format!("Inside it you see: {}.", list));
        }
    }
}