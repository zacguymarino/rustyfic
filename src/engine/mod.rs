mod actions;
mod conditions;
mod helpers;
mod items;
mod movement;
mod npcs;
mod output;
mod render;

pub use actions::{try_handle_action, try_handle_global_action};

pub use conditions::{conditions_met, evaluate_global_conditions};

pub use items::{
    check_container_completion, handle_drop, handle_drop_all, handle_examine, handle_give_to_npc,
    handle_inventory, handle_take, handle_take_all_from_container, handle_take_all_room,
    handle_take_from_container, handle_take_from_npc, try_handle_container_store,
};

pub use movement::try_handle_movement;
pub use output::{Output, OutputBlock};
pub use render::{render_room, room_depends_on_any_flag};

pub use helpers::{apply_effects, item_in_inventory, item_in_room, item_visible};

pub use npcs::{
    handle_talk_to_npc, roam_npcs_after_player_move, try_handle_examine_npc, try_handle_npc_action,
};
