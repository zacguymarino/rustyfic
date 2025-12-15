use std::collections::HashMap;

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
    pub capacity: Option<usize>,        // number of items that can fit
    pub conditions: Vec<String>,        // flags required to interact
    pub complete_when: Vec<String>,     // item IDs
    pub complete_flag: Option<String>,  // flag to set
    pub closed_text: String,            // message when conditions not met
    pub complete_text: Option<String>,  // message when completion triggers
    pub verbs: Vec<String>,
    pub prep: String,
}

pub struct GlobalCondition {
    pub id: String,
    pub conditions: Vec<String>,        // flag conditions like everywhere else
    pub allowed_rooms: Vec<String>,     // optional whitelist of room IDs
    pub disallowed_rooms: Vec<String>,  // optional blacklist of room IDs
    pub response: String,               // text printed when it fires
    pub effects: Vec<String>,           // flags to add/remove
    pub one_shot: bool,                 // if true, only fires once ever
}
