mod loader;
mod model;

pub use loader::load_world_from_file;

// Minimal, intentional surface area: re-export only what the game/engine uses.
pub use model::{
    Action, ContainerProps, Exit, GlobalCondition, Item, ItemKind, ItemLocation, Npc, NpcRoam,
    Room, StateDesc, World,
};
