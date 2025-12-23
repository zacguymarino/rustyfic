use std::collections::HashSet;

use super::model::{Action, ItemKind, ItemLocation, World};

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
}

impl ValidationError {
    fn new(msg: impl Into<String>) -> Self {
        ValidationError {
            message: msg.into(),
        }
    }
}

pub fn validate_world(world: &World) -> Vec<ValidationError> {
    let mut errors: Vec<ValidationError> = Vec::new();

    // Rooms must not be empty
    if world.rooms.is_empty() {
        errors.push(ValidationError::new("world has no rooms"));
    }

    // start_room must exist
    if !world.rooms.contains_key(&world.start_room) {
        errors.push(ValidationError::new(format!(
            "start_room '{}' not found among rooms",
            world.start_room
        )));
    }

    // Validate exits
    for (room_id, room) in &world.rooms {
        for exit in &room.exits {
            if !world.rooms.contains_key(&exit.target) {
                errors.push(ValidationError::new(format!(
                    "room '{}' exit '{}' targets missing room '{}'",
                    room_id, exit.direction, exit.target
                )));
            }
        }
    }

    // Index helpers
    let all_items: HashSet<String> = world.items.keys().cloned().collect();
    let all_rooms: HashSet<String> = world.rooms.keys().cloned().collect();

    // Validate item start locations and container completeness
    for item in world.items.values() {
        match &item.start_location {
            ItemLocation::Room(r) => {
                if !world.rooms.contains_key(r) {
                    errors.push(ValidationError::new(format!(
                        "item '{}' start_location room '{}' not found",
                        item.id, r
                    )));
                }
            }
            ItemLocation::Item(parent) => {
                if parent == &item.id {
                    errors.push(ValidationError::new(format!(
                        "item '{}' cannot start inside itself",
                        item.id
                    )));
                }
                if !world.items.contains_key(parent) {
                    errors.push(ValidationError::new(format!(
                        "item '{}' start_location item '{}' not found",
                        item.id, parent
                    )));
                }
            }
            ItemLocation::Npc(npc_id) => {
                if !world.npcs.contains_key(npc_id) {
                    errors.push(ValidationError::new(format!(
                        "item '{}' start_location npc '{}' not found",
                        item.id, npc_id
                    )));
                }
            }
            ItemLocation::Inventory => {}
        }

        if let ItemKind::Container(props) = &item.kind {
            for needed in &props.complete_when {
                if !world.items.contains_key(needed) {
                    errors.push(ValidationError::new(format!(
                        "container '{}' complete_when references missing item '{}'",
                        item.id, needed
                    )));
                }
            }
        }
    }

    // Validate NPCs
    for (npc_id, npc) in &world.npcs {
        if !world.rooms.contains_key(&npc.start_room) {
            errors.push(ValidationError::new(format!(
                "npc '{}' start_room '{}' not found",
                npc_id, npc.start_room
            )));
        }

        // block_exits are free-form; just ensure not empty strings
        for ex in &npc.block_exits {
            if ex.trim().is_empty() {
                errors.push(ValidationError::new(format!(
                    "npc '{}' has an empty block_exits entry",
                    npc_id
                )));
            }
        }

        validate_actions(
            &npc.actions,
            &all_items,
            &all_rooms,
            &mut errors,
            Some(format!("npc '{}'", npc_id)),
        );
    }

    // Validate room actions
    for (room_id, room) in &world.rooms {
        validate_actions(
            &room.actions,
            &all_items,
            &all_rooms,
            &mut errors,
            Some(format!("room '{}'", room_id)),
        );
    }

    // Validate global actions
    validate_actions(
        &world.global_actions,
        &all_items,
        &all_rooms,
        &mut errors,
        Some("global actions".to_string()),
    );

    // Validate global conditions
    for gc in &world.global_conditions {
        for r in &gc.allowed_rooms {
            if !all_rooms.contains(r) {
                errors.push(ValidationError::new(format!(
                    "global_condition '{}' allowed_rooms references missing room '{}'",
                    gc.id, r
                )));
            }
        }
        for r in &gc.disallowed_rooms {
            if !all_rooms.contains(r) {
                errors.push(ValidationError::new(format!(
                    "global_condition '{}' disallowed_rooms references missing room '{}'",
                    gc.id, r
                )));
            }
        }
    }

    errors
}

fn validate_actions(
    actions: &[Action],
    all_items: &HashSet<String>,
    _all_rooms: &HashSet<String>,
    errors: &mut Vec<ValidationError>,
    scope_label: Option<String>,
) {
    let label = scope_label.unwrap_or_else(|| "actions".to_string());

    for action in actions {
        for req in &action.requires_inventory {
            if !all_items.contains(req) {
                errors.push(ValidationError::new(format!(
                    "{} action '{}' requires missing item '{}'",
                    label, action.id, req
                )));
            }
        }

        for req in &action.scope_requirements {
            if !all_items.contains(req) {
                errors.push(ValidationError::new(format!(
                    "{} action '{}' scope_requirements references missing item '{}'",
                    label, action.id, req
                )));
            }
        }

        for cond_room in &action.conditions {
            // Conditions are flag strings; no validation here.
            let _ = cond_room;
        }

        for verb in &action.verbs {
            if verb.trim().is_empty() {
                errors.push(ValidationError::new(format!(
                    "{} action '{}' has an empty verb entry",
                    label, action.id
                )));
            }
        }

        for noun in &action.nouns {
            if noun.trim().is_empty() {
                errors.push(ValidationError::new(format!(
                    "{} action '{}' has an empty noun entry",
                    label, action.id
                )));
            }
        }
    }
}
