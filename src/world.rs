use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

//////////////////////////////
/// GAME STRUCTS AND ENUMS ///
//////////////////////////////

/// Runtime world type used by the game loop.
pub struct World {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub start_room: String,
    pub rooms: HashMap<String, Room>,
    pub items: HashMap<String, Item>,
    pub global_conditions: Vec<GlobalCondition>,
}

pub struct Room {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub exits: Vec<Exit>,
    pub actions: Vec<Action>,
    pub state_descs: Vec<StateDesc>,
}

pub struct StateDesc {
    pub conditions: Vec<String>,
    pub text: String,
}

pub struct Exit {
    pub direction: String,
    pub target: String,
    pub verbs: Vec<String>,
    pub conditions: Vec<String>,
}

pub struct Action {
    pub id: String,
    pub verbs: Vec<String>,
    pub nouns: Vec<String>,
    pub response: String,
    pub effects: Vec<String>,
    pub conditions: Vec<String>,
}

#[derive(Clone)]
pub enum ItemLocation {
    Room(String),
    Inventory,
    Item(String), // inside another item (container) later
}

pub enum ItemKind {
    Simple,
    Container(ContainerProps),
    // Weapon(WeaponProps),
    // Armor(ArmorProps),
    // Consumable(ConsumableProps),
}

pub struct Item {
    pub id: String,
    pub name: String,
    pub room_text: String,
    pub inventory_text: String,
    pub examine_text: String,
    pub conditions: Vec<String>,
    pub portable: bool,
    pub kind: ItemKind,
    pub start_location: ItemLocation,
}

pub struct ContainerProps {
    pub capacity: Option<usize>,       // number of items that can fit
    pub conditions: Vec<String>,       // flags required to interact
    pub complete_when: Vec<String>,    // item IDs
    pub complete_flag: Option<String>, // flag to set
    pub closed_text: String,           // message when conditions not met
    pub complete_text: Option<String>,  // message when completion triggers
    pub verbs: Vec<String>,
    pub prep: String,
}

pub struct GlobalCondition {
    pub id: String,
    pub conditions: Vec<String>,       // flag conditions like everywhere else
    pub allowed_rooms: Vec<String>,    // optional whitelist of room IDs
    pub disallowed_rooms: Vec<String>, // optional blacklist of room IDs
    pub response: String,                  // text printed when it fires
    pub effects: Vec<String>,          // flags to add/remove
    pub one_shot: bool,                // if true, only fires once ever
}

////////////////////
/// TOML STRUCTS ///
////////////////////

#[derive(Deserialize)]
struct WorldFile {
    world: WorldHeader,
    #[serde(default)]
    room: Vec<RoomConfig>, // [[room]] blocks
    #[serde(default)]
    item: Vec<ItemConfig>, // [[item]] blocks
    #[serde(default)]
    global_condition: Vec<GlobalConditionConfig>, // [[global_condition]]
}

#[derive(Deserialize)]
struct WorldHeader {
    id: String,
    name: String,
    start_room: String,
    #[serde(default)]
    desc: String,
}

#[derive(Deserialize)]
struct RoomConfig {
    id: String,
    name: String,
    #[serde(default)]
    desc: String,

    #[serde(default)]
    exit: Vec<ExitConfig>, // [[room.exit]]
    #[serde(default)]
    action: Vec<ActionConfig>, // [[room.action]]
    #[serde(default)]
    state_desc: Vec<StateDescConfig>, // [[room.state_desc]]
}

#[derive(Deserialize)]
struct StateDescConfig {
    #[serde(default)]
    conditions: Vec<String>,
    text: String,
}

#[derive(Deserialize)]
struct ExitConfig {
    direction: String,
    target: String,

    #[serde(default)]
    verbs: Vec<String>,

    #[serde(default)]
    conditions: Vec<String>,
}

#[derive(Deserialize)]
struct ActionConfig {
    id: String,
    verbs: Vec<String>,

    #[serde(default)]
    nouns: Vec<String>,

    response: String,

    #[serde(default)]
    effects: Vec<String>,

    #[serde(default)]
    conditions: Vec<String>,
}

#[derive(Deserialize)]
struct ItemConfig {
    id: String,
    name: String,

    /// Where the item starts: "room:house", "inventory", "item:trophy_case", etc.
    start_location: String,

    #[serde(default)]
    room_text: String,

    #[serde(default)]
    inventory_text: String,

    #[serde(default)]
    examine_text: String,

    #[serde(default)]
    conditions: Vec<String>,

    #[serde(default)]
    portable: Option<bool>,

    #[serde(default)]
    kind: Option<String>, // e.g. "simple", "container", "weapon"

    #[serde(default)]
    capacity: Option<usize>,

    #[serde(default)]
    container_conditions: Vec<String>,

    #[serde(default)]
    complete_when: Vec<String>,

    #[serde(default)]
    complete_flag: Option<String>,

    #[serde(default)]
    container_closed_text: Option<String>,

    #[serde(default)]
    complete_text: Option<String>,

    #[serde(default)]
    container_verbs: Vec<String>,

    #[serde(default)]
    container_prep: Option<String>,
}

#[derive(Deserialize)]
struct GlobalConditionConfig {
    id: String,

    #[serde(default)]
    conditions: Vec<String>,

    #[serde(default)]
    allowed_rooms: Vec<String>,

    #[serde(default)]
    disallowed_rooms: Vec<String>,

    #[serde(default)]
    response: String,

    #[serde(default)]
    effects: Vec<String>,

    // default to true if omitted
    #[serde(default = "default_true")]
    one_shot: bool,
}

// Helper for serde default
fn default_true() -> bool {
    true
}

/////////////////////////////
/// TOML PARSER FUNCTIONS ///
/////////////////////////////

/// Public API: load a world from a .toml file on disk.
pub fn load_world_from_file(path: &Path) -> io::Result<World> {
    let contents = fs::read_to_string(path)?;
    let world_file: WorldFile = toml::from_str(&contents)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    // Basic validation
    if world_file.world.id.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "world.id may not be empty",
        ));
    }
    if world_file.world.start_room.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "world.start_room may not be empty",
        ));
    }

    // Build rooms map
    let mut rooms_map: HashMap<String, Room> = HashMap::new();

    for room_cfg in world_file.room {
        if rooms_map.contains_key(&room_cfg.id) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Duplicate room id: {}", room_cfg.id),
            ));
        }

        let exits = room_cfg
            .exit
            .into_iter()
            .map(|e| Exit {
                direction: e.direction,
                target: e.target,
                verbs: e.verbs,
                conditions: e.conditions,
            })
            .collect();

        let actions = room_cfg
            .action
            .into_iter()
            .map(|a| Action {
                id: a.id,
                verbs: a.verbs,
                nouns: a.nouns,
                response: a.response,
                effects: a.effects,
                conditions: a.conditions,
            })
            .collect();

        let state_descs = room_cfg
            .state_desc
            .into_iter()
            .map(|sd| StateDesc {
                conditions: sd.conditions,
                text: normalize_multiline_desc(&sd.text),
            })
            .collect();

        rooms_map.insert(
            room_cfg.id.clone(),
            Room {
                id: room_cfg.id,
                name: room_cfg.name,
                desc: normalize_multiline_desc(&room_cfg.desc),
                exits,
                actions,
                state_descs,
            },
        );
    }

    // Ensure start_room exists
    if !rooms_map.contains_key(&world_file.world.start_room) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "start_room '{}' not found among rooms",
                world_file.world.start_room
            ),
        ));
    }

    // Build items map
    let mut items_map: HashMap<String, Item> = HashMap::new();

    for ic in world_file.item {
        if items_map.contains_key(&ic.id) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Duplicate item id: {}", ic.id),
            ));
        }

        let start_location = parse_item_location(&ic.start_location)
            .map_err(|msg| io::Error::new(io::ErrorKind::InvalidData, msg))?;

        let kind = parse_item_kind(&ic);

        let room_text = normalize_multiline_desc(&ic.room_text);

        let inventory_text = if ic.inventory_text.trim().is_empty() {
            // fall back to name if no custom inventory text
            ic.name.clone()
        } else {
            normalize_multiline_desc(&ic.inventory_text)
        };

        let examine_text = normalize_multiline_desc(&ic.examine_text);

        let portable = ic.portable.unwrap_or(true);

        items_map.insert(
            ic.id.clone(),
            Item {
                id: ic.id,
                name: ic.name,
                room_text,
                inventory_text,
                examine_text,
                conditions: ic.conditions,
                portable,
                kind,
                start_location,
            },
        );
    }

    // Build global conditions
    let mut global_conditions: Vec<GlobalCondition> = Vec::new();

    for gc in world_file.global_condition {
        if gc.id.trim().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "global_condition.id may not be empty",
            ));
        }

        global_conditions.push(GlobalCondition {
            id: gc.id,
            conditions: gc.conditions,
            allowed_rooms: gc.allowed_rooms,
            disallowed_rooms: gc.disallowed_rooms,
            response: normalize_multiline_desc(&gc.response),
            effects: gc.effects,
            one_shot: gc.one_shot,
        });
    }

    Ok(World {
        id: world_file.world.id,
        name: world_file.world.name,
        desc: normalize_multiline_desc(&world_file.world.desc),
        start_room: world_file.world.start_room,
        rooms: rooms_map,
        items: items_map,
        global_conditions,
    })
}

fn normalize_multiline_desc(raw: &str) -> String {
    let mut result = String::new();
    let mut pending_blank_lines = 0usize;
    let mut first_text_seen = false;

    for line in raw.lines() {
        // Strip *all* leading/trailing whitespace so indentation in TOML
        // doesn't affect what the player sees.
        let trimmed = line.trim();

        let is_blank = trimmed.is_empty();

        if is_blank {
            // Count blank lines; we'll decide how to render them when we
            // hit the next non-blank line.
            pending_blank_lines += 1;
            continue;
        }

        // Non-blank line:
        if !first_text_seen {
            // First actual text: just write it
            result.push_str(trimmed);
            first_text_seen = true;
        } else {
            match pending_blank_lines {
                0 => {
                    // Wrapped line: single newline in TOML → space in output
                    result.push(' ');
                    result.push_str(trimmed);
                }
                1 => {
                    // One blank line → one visible newline
                    result.push('\n');
                    result.push_str(trimmed);
                }
                _ => {
                    // Two or more blank lines → paragraph break
                    result.push_str("\n\n");
                    result.push_str(trimmed);
                }
            }
        }

        // Reset pending blanks after we've handled them
        pending_blank_lines = 0;
    }

    result
}

////////////////////////////
/// ITEM PARSE HELPERS   ///
////////////////////////////

fn parse_item_location(s: &str) -> Result<ItemLocation, String> {
    let s = s.trim();

    if s.eq_ignore_ascii_case("inventory") {
        return Ok(ItemLocation::Inventory);
    }

    if let Some(rest) = s.strip_prefix("room:") {
        let room_id = rest.trim();
        if room_id.is_empty() {
            return Err(format!("Invalid start_location '{}': empty room id", s));
        }
        return Ok(ItemLocation::Room(room_id.to_string()));
    }

    if let Some(rest) = s.strip_prefix("item:") {
        let item_id = rest.trim();
        if item_id.is_empty() {
            return Err(format!("Invalid start_location '{}': empty item id", s));
        }
        return Ok(ItemLocation::Item(item_id.to_string()));
    }

    Err(format!(
        "Invalid start_location '{}': expected 'room:<id>', 'item:<id>', or 'inventory'",
        s
    ))
}

fn parse_item_kind(ic: &ItemConfig) -> ItemKind {
    match ic.kind.as_deref().map(|s| s.to_lowercase()) {
        Some(ref k) if k == "container" => {
            ItemKind::Container(ContainerProps {
                capacity: ic.capacity,
                conditions: ic.container_conditions.clone(),
                complete_when: ic.complete_when.clone(),
                complete_flag: ic.complete_flag.clone(),
                closed_text: ic
                    .container_closed_text
                    .clone()
                    .unwrap_or_else(|| "It is currently closed.".to_string()),
                complete_text: ic.complete_text.clone(),
                verbs: if ic.container_verbs.is_empty() {
                    vec!["put".to_string()]
                } else {
                    ic.container_verbs.clone()
                },
                prep: ic.container_prep.clone().unwrap_or_else(|| "in".to_string()),
            })
        }
        Some(ref k) if k == "simple" => ItemKind::Simple,
        Some(ref k) if !k.is_empty() => {
            eprintln!("Warning: unknown item kind '{}', defaulting to Simple", k);
            ItemKind::Simple
        }
        _ => ItemKind::Simple,
    }
}
