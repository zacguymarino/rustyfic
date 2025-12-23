mod loader;
mod model;
mod validator;

pub use loader::{load_world_from_file, load_world_from_str};

// Minimal, intentional surface area: re-export only what the game/engine uses.
pub use model::{Action, Exit, Item, ItemKind, ItemLocation, Npc, Room, World};
pub use validator::{ValidationError, validate_world};
