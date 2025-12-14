mod model;
mod loader;

pub use loader::load_world_from_file;

// Minimal, intentional surface area: re-export only what the game/engine uses.
pub use model::{
    World,
    Room,
    StateDesc,
    Exit,
    Action,
    ItemLocation,
    ItemKind,
    Item,
    ContainerProps,
    GlobalCondition,
};
