use std::collections::HashMap;

//////////////////////////////
/// GAME STRUCTS AND ENUMS ///
//////////////////////////////

/// Runtime world type used by the game loop.
pub struct World {
    #[allow(dead_code)]
    pub id: String,
    pub name: String,
    pub desc: String,
    pub start_room: String,
    pub rooms: HashMap<String, Room>,
    pub items: HashMap<String, Item>,
    pub npcs: HashMap<String, Npc>,
    pub global_conditions: Vec<GlobalCondition>,
    pub global_actions: Vec<Action>,
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
    #[allow(dead_code)]
    pub id: String,
    pub verbs: Vec<String>,
    pub nouns: Vec<String>,
    pub response: String,
    pub effects: Vec<String>,
    pub conditions: Vec<String>,
    pub scope_requirements: Vec<String>,
    pub requires_inventory: Vec<String>,
    pub missing_inventory_text: Option<String>,
    pub missing_scope_text: Option<String>,
}

#[derive(Clone)]
pub enum ItemLocation {
    Room(String),
    Inventory,
    Item(String), // inside another item (container) later
    Npc(String),  // held by an NPC
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
    pub aliases: Vec<String>,
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
    pub complete_text: Option<String>, // message when completion triggers
    pub verbs: Vec<String>,
    pub prep: String,
}

pub struct GlobalCondition {
    pub id: String,
    pub conditions: Vec<String>, // flag conditions like everywhere else
    pub allowed_rooms: Vec<String>, // optional whitelist of room IDs
    pub disallowed_rooms: Vec<String>, // optional blacklist of room IDs
    pub response: String,        // text printed when it fires
    pub effects: Vec<String>,    // flags to add/remove
    pub one_shot: bool,          // if true, only fires once ever
}

pub struct Npc {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub start_room: String,
    pub room_text: String,
    pub examine_text: String,
    pub conditions: Vec<String>,
    pub actions: Vec<Action>,          // reuse existing Action struct
    pub roam: Option<NpcRoam>,         // optional roaming behavior
    pub block_movement: bool,          // if true, can block movement while present/visible
    pub block_conditions: Vec<String>, // additional conditions for blocking
    pub block_text: Option<String>,    // custom message when blocking movement
    pub block_exits: Vec<String>, // optional list of exit directions/verbs to block (empty = all)
    pub foe: bool,                // if true, may attack when blocking
    pub attack_chance_percent: u8, // 0..=100 chance when blocking
    pub attack_text: Option<String>, // message when attack triggers
    pub attack_effects: Vec<String>, // effects applied on attack
    pub dialogue: Vec<NpcDialogue>, // optional dialogue entries
}

pub struct NpcRoam {
    pub enabled: bool,
    pub allowed_rooms: Vec<String>,
    pub chance_percent: u8, // 0..=100
}

pub struct NpcDialogue {
    pub id: String,
    pub conditions: Vec<String>,
    pub response: String,
    pub effects: Vec<String>,
    pub one_shot: bool,
}
