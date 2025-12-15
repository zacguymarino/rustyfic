use std::collections::{HashMap, HashSet};

use crate::engine::conditions::conditions_met;
use crate::engine::output::Output;
use crate::world;

enum ItemMatch<'a> {
    None,
    One(&'a world::Item),
    Many(Vec<&'a world::Item>),
}

/// Find the *best* matching item by counting full-word overlaps.
/// - Highest score wins
/// - Ties => Many (ambiguity)
/// - Score 0 => None
///
/// `respect_conditions` controls whether `item.conditions` are enforced during matching.
/// - true  => item must satisfy its visibility/interaction conditions
/// - false => ignore item.conditions (useful for inventory-only operations like drop)
fn find_item_by_words_scored<'a, F>(
    world: &'a world::World,
    item_locations: &HashMap<String, world::ItemLocation>,
    flags: &HashSet<String>,
    query: &str,
    filter: F,
    respect_conditions: bool,
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

        // Optionally respect item visibility/interaction conditions
        if respect_conditions && !conditions_met(&item.conditions, flags) {
            continue;
        }

        // Build matchable words from: primary name + aliases
        let mut all_words: Vec<String> = Vec::new();

        // primary display name
        all_words.extend(
            item.name
                .split_whitespace()
                .filter(|w| !w.is_empty())
                .map(|w| w.to_lowercase()),
        );

        // extra aliases (if you added them in world/model + loader)
        for alias in &item.aliases {
            all_words.extend(
                alias
                    .split_whitespace()
                    .filter(|w| !w.is_empty())
                    .map(|w| w.to_lowercase()),
            );
        }

        // Score = number of query words that appear in the item's name/alias words
        let mut score = 0usize;
        for qw in &query_words {
            if all_words.iter().any(|iw| iw == qw) {
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

/// Convenience wrapper: default behavior (respect item.conditions)
fn find_item<'a, F>(
    world: &'a world::World,
    item_locations: &HashMap<String, world::ItemLocation>,
    flags: &HashSet<String>,
    query: &str,
    filter: F,
) -> ItemMatch<'a>
where
    F: Fn(&'a world::Item, &world::ItemLocation) -> bool,
{
    find_item_by_words_scored(world, item_locations, flags, query, filter, true)
}

/// Convenience wrapper: ignore item.conditions
fn find_item_ignore_conditions<'a, F>(
    world: &'a world::World,
    item_locations: &HashMap<String, world::ItemLocation>,
    flags: &HashSet<String>,
    query: &str,
    filter: F,
) -> ItemMatch<'a>
where
    F: Fn(&'a world::Item, &world::ItemLocation) -> bool,
{
    find_item_by_words_scored(world, item_locations, flags, query, filter, false)
}

pub fn handle_inventory(
    out: &mut Output,
    world: &world::World,
    item_locations: &HashMap<String, world::ItemLocation>,
) {
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
    use world::ItemLocation;

    let query = target_name.trim().to_lowercase();
    if query.is_empty() {
        out.say("Take what?");
        return;
    }

    let result = find_item(
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

    let mut to_take: Vec<String> = Vec::new();

    for item in world.items.values() {
        let loc = match item_locations.get(&item.id) {
            Some(l) => l,
            None => continue,
        };

        if let ItemLocation::Room(room_id) = loc {
            if room_id == current_room_id && conditions_met(&item.conditions, flags) && item.portable
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

    // Drop should NOT be blocked by item.conditions (visibility flags, etc.)
    // We pass an empty set for flags because we're ignoring conditions anyway.
    let dummy_flags = HashSet::new();

    let result = find_item_ignore_conditions(
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

    let mut to_drop: Vec<String> = Vec::new();

    for item in world.items.values() {
        let loc = match item_locations.get(&item.id) {
            Some(l) => l,
            None => continue,
        };

        if let ItemLocation::Inventory = loc {
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

    // Find the container (must be visible)
    let container_result = find_item(
        world,
        item_locations,
        flags,
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

    if !props.conditions.is_empty() && !conditions_met(&props.conditions, flags) {
        out.say(format!("{}", props.closed_text.trim()));
        return;
    }

    // Find the item inside (must be visible)
    let item_result = find_item(
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

    let container_match = find_item(
        world,
        item_locations,
        flags,
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

    if !props.conditions.is_empty() && !conditions_met(&props.conditions, flags) {
        out.say(props.closed_text.trim());
        return;
    }

    let mut to_take: Vec<String> = Vec::new();

    for item in world.items.values() {
        let loc = match item_locations.get(&item.id) {
            Some(l) => l,
            None => continue,
        };

        if let ItemLocation::Item(parent_id) = loc {
            if parent_id == &container.id && conditions_met(&item.conditions, flags) && item.portable {
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

    for item_id in &to_take {
        if let Some(item) = world.items.get(item_id) {
            item_locations.insert(item_id.clone(), ItemLocation::Inventory);
            out.say(format!("You take the {} from the {}.", item.name, container.name));
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

        // Container itself must be visible
        if !conditions_met(&c.conditions, flags) {
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
        return false;
    }

    let query = rest.trim().to_lowercase();
    if query.is_empty() {
        out.say(format!("What do you want to {}?", verb_l));
        return true;
    }

    // 2) Find carried item mentioned in rest (ignore conditions for inventory matching)
    let item_match = find_item_ignore_conditions(
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

    // 3) Find a container in scope that matches query and supports verb (must be visible)
    let cont_match = find_item(
        world,
        item_locations,
        flags,
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
        verb_l, item.name, props.prep, container.name
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

    let complete_flag = match &props.complete_flag {
        Some(f) => f,
        None => return,
    };

    if props.complete_when.is_empty() {
        return;
    }

    if flags.contains(complete_flag) {
        return;
    }

    for needed_id in &props.complete_when {
        match item_locations.get(needed_id) {
            Some(ItemLocation::Item(parent_id)) if parent_id == container_id => {}
            _ => return,
        }
    }

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

    // Prefer items in inventory (respect conditions)
    let inv_match = find_item_ignore_conditions(
        world,
        item_locations,
        &HashSet::new(),
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

    // If not in inventory, look in the room
    let item = match item {
        Some(i) => i,
        None => {
            let room_match = find_item(
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

    let txt = item.examine_text.trim();
    if txt.is_empty() {
        out.say(format!("You see nothing special about the {}.", item.name));
    } else {
        out.say(txt);
    }

    if let ItemKind::Container(props) = &item.kind {
        if !props.conditions.is_empty() && !conditions_met(&props.conditions, flags) {
            out.say(format!("{}", props.closed_text.trim()));
            return;
        }

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
