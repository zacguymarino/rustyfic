pub mod engine;
pub mod world;

use std::collections::{HashMap, HashSet};

use engine::{
    Output, handle_drop, handle_drop_all, handle_examine, handle_give_to_npc, handle_inventory,
    handle_take, handle_take_all_from_container, handle_take_all_room, handle_take_from_container,
    handle_take_from_npc, handle_talk_to_npc, render_room, roam_npcs_after_player_move,
    room_depends_on_any_flag, try_handle_action, try_handle_container_store,
    try_handle_global_action, try_handle_movement, try_handle_npc_action,
};
use world::{ItemLocation, World};

pub use world::{load_world_from_file, load_world_from_str};

pub struct GameState {
    pub world: World,
    pub current_room_id: String,
    pub flags: HashSet<String>,
    pub fired_global_conditions: HashSet<String>,
    pub fired_dialogues: HashSet<String>,
    pub item_locations: HashMap<String, ItemLocation>,
    pub npc_locations: HashMap<String, String>,
    pub turn_index: u64,
    pub action_index: u64,
}

#[cfg(feature = "wasm")]
mod wasm_bindings {
    use super::*;
    use serde::Serialize;
    use serde_wasm_bindgen::to_value;
    use wasm_bindgen::prelude::*;

    #[derive(Serialize)]
    struct WasmStepResult {
        blocks: Vec<engine::OutputBlock>,
        quit: bool,
    }

    #[wasm_bindgen]
    pub struct WasmGame {
        state: GameState,
        initialized: bool,
    }

    #[wasm_bindgen]
    impl WasmGame {
        /// Create a new game from a TOML world string. Call `init()` to get the initial render.
        #[wasm_bindgen(constructor)]
        pub fn new(world_toml: &str) -> Result<WasmGame, JsValue> {
            let world =
                load_world_from_str(world_toml).map_err(|e| JsValue::from_str(&e.to_string()))?;
            Ok(WasmGame {
                state: GameState::new(world),
                initialized: false,
            })
        }

        /// Initialize the game and return the initial render output.
        #[wasm_bindgen]
        pub fn init(&mut self) -> JsValue {
            if !self.initialized {
                self.initialized = true;
            }
            match self.state.initialize() {
                Some(out) => to_value(&WasmStepResult {
                    blocks: out.blocks,
                    quit: false,
                })
                .unwrap_or(JsValue::NULL),
                None => JsValue::NULL,
            }
        }

        /// Process a player command and return the resulting output blocks and quit flag.
        #[wasm_bindgen]
        pub fn step(&mut self, input: &str) -> JsValue {
            if !self.initialized {
                let _ = self.init();
            }
            let (out, quit) = self.state.step(input);
            to_value(&WasmStepResult {
                blocks: out.blocks,
                quit,
            })
            .unwrap_or(JsValue::NULL)
        }
    }
}

impl GameState {
    pub fn new(world: World) -> Self {
        let mut item_locations: HashMap<String, ItemLocation> = HashMap::new();
        for (id, item) in &world.items {
            item_locations.insert(id.clone(), item.start_location.clone());
        }

        let mut npc_locations: HashMap<String, String> = HashMap::new();
        for (id, npc) in &world.npcs {
            npc_locations.insert(id.clone(), npc.start_room.clone());
        }

        GameState {
            world,
            current_room_id: String::new(),
            flags: HashSet::new(),
            fired_global_conditions: HashSet::new(),
            fired_dialogues: HashSet::new(),
            item_locations,
            npc_locations,
            turn_index: 0,
            action_index: 0,
        }
    }

    pub fn initialize(&mut self) -> Option<Output> {
        self.current_room_id = self.world.start_room.clone();
        if let Some(room) = self.world.rooms.get(&self.current_room_id) {
            let mut out = Output::new();
            render_room(
                &mut out,
                room,
                &self.flags,
                &self.world,
                &self.item_locations,
                &self.npc_locations,
            );
            Some(out)
        } else {
            None
        }
    }

    /// Process a single player input; returns (output, quit?)
    pub fn step(&mut self, input: &str) -> (Output, bool) {
        let mut out = Output::new();
        let lower = input.to_lowercase();
        let mut quit = false;
        let mut rendered_room_this_turn = false;
        self.action_index = self.action_index.wrapping_add(1);

        if lower == "quit" || lower == "exit" {
            out.say("Goodbye.");
            quit = true;
        } else if lower == "inventory" || lower == "i" {
            handle_inventory(&mut out, &self.world, &self.item_locations);
        } else {
            let mut parts = input.split_whitespace();
            let verb = parts.next().unwrap_or("");
            let rest = parts.collect::<Vec<&str>>().join(" ");
            let rest_lower = rest.to_lowercase();

            if verb.eq_ignore_ascii_case("talk") || verb.eq_ignore_ascii_case("speak") {
                if rest_lower.is_empty() {
                    out.say("Talk to whom?");
                } else {
                    handle_talk_to_npc(
                        &mut out,
                        &self.world,
                        &self.npc_locations,
                        &self.current_room_id,
                        &rest_lower,
                        &mut self.flags,
                        &mut self.fired_dialogues,
                    );
                }
            } else if verb.eq_ignore_ascii_case("give") {
                if rest_lower.is_empty() {
                    out.say("Give what to whom?");
                } else if let Some(idx) = rest_lower.rfind(" to ") {
                    let item_part = rest_lower[..idx].trim();
                    let npc_part = rest_lower[idx + " to ".len()..].trim();

                    if item_part.is_empty() || npc_part.is_empty() {
                        out.say("I don't understand who you want to give that to.");
                    } else {
                        handle_give_to_npc(
                            &mut out,
                            &mut self.item_locations,
                            &self.world,
                            &self.npc_locations,
                            &self.current_room_id,
                            item_part,
                            npc_part,
                            &mut self.flags,
                        );
                    }
                } else {
                    out.say("Give it to whom?");
                }
            } else if verb.eq_ignore_ascii_case("take") || verb.eq_ignore_ascii_case("get") {
                if rest.is_empty() {
                    out.say("Take what?");
                } else if rest_lower == "all" {
                    handle_take_all_room(
                        &mut out,
                        &mut self.item_locations,
                        &self.world,
                        &self.current_room_id,
                        &self.flags,
                    );
                } else if let Some(idx) = rest_lower.find(" from ") {
                    let item_part = rest_lower[..idx].trim();
                    let container_part = rest_lower[idx + " from ".len()..].trim();

                    if item_part.is_empty() || container_part.is_empty() {
                        out.say("I don't understand what you want to take from where.");
                    } else {
                        let handled_npc = handle_take_from_npc(
                            &mut out,
                            &mut self.item_locations,
                            &self.world,
                            &self.npc_locations,
                            &self.current_room_id,
                            item_part,
                            container_part,
                            &self.flags,
                        );

                        if !handled_npc {
                            if item_part == "all" {
                                handle_take_all_from_container(
                                    &mut out,
                                    &mut self.item_locations,
                                    &self.world,
                                    &self.current_room_id,
                                    container_part,
                                    &self.flags,
                                );
                            } else {
                                handle_take_from_container(
                                    &mut out,
                                    &mut self.item_locations,
                                    &self.world,
                                    &self.current_room_id,
                                    item_part,
                                    container_part,
                                    &self.flags,
                                );
                            }
                        }
                    }
                } else {
                    handle_take(
                        &mut out,
                        &mut self.item_locations,
                        &self.world,
                        &self.current_room_id,
                        &rest_lower,
                        &self.flags,
                    );
                }
            } else if verb.eq_ignore_ascii_case("drop") {
                if rest.is_empty() {
                    out.say("Drop what?");
                } else if rest_lower == "all" {
                    handle_drop_all(
                        &mut out,
                        &mut self.item_locations,
                        &self.world,
                        &self.current_room_id,
                    );
                } else {
                    handle_drop(
                        &mut out,
                        &mut self.item_locations,
                        &self.world,
                        &self.current_room_id,
                        &rest_lower,
                    );
                }
            } else if verb.eq_ignore_ascii_case("examine")
                || verb.eq_ignore_ascii_case("x")
                || (verb.eq_ignore_ascii_case("look") && rest_lower.starts_with("at "))
            {
                let target = if verb.eq_ignore_ascii_case("look") {
                    rest_lower.trim_start_matches("at").trim()
                } else {
                    rest_lower.trim()
                };

                if target.is_empty() {
                    out.say("Examine what?");
                } else {
                    handle_examine(
                        &mut out,
                        &self.world,
                        &self.item_locations,
                        &self.npc_locations,
                        &self.current_room_id,
                        target,
                        &self.flags,
                    );
                }
            } else if try_handle_container_store(
                &mut out,
                verb,
                &rest_lower,
                &mut self.item_locations,
                &self.world,
                &self.current_room_id,
                &mut self.flags,
            ) {
                // handled
            } else if let Some(current_room) = self.world.rooms.get(&self.current_room_id) {
                if lower == "look" || lower == "l" {
                    render_room(
                        &mut out,
                        current_room,
                        &self.flags,
                        &self.world,
                        &self.item_locations,
                        &self.npc_locations,
                    );
                    rendered_room_this_turn = true;
                } else {
                    let prev_room_id = self.current_room_id.clone();

                    if try_handle_movement(
                        &mut out,
                        &mut self.current_room_id,
                        &self.world,
                        current_room,
                        &lower,
                        &self.npc_locations,
                        &mut self.flags,
                        self.action_index,
                    ) {
                        let moved = self.current_room_id != prev_room_id;

                        if moved {
                            self.turn_index += 1;
                            roam_npcs_after_player_move(
                                &self.world,
                                &mut self.npc_locations,
                                &self.flags,
                                self.turn_index,
                            );

                            if let Some(room) = self.world.rooms.get(&self.current_room_id) {
                                render_room(
                                    &mut out,
                                    room,
                                    &self.flags,
                                    &self.world,
                                    &self.item_locations,
                                    &self.npc_locations,
                                );
                                rendered_room_this_turn = true;
                            }
                        } else {
                            rendered_room_this_turn = true;
                        }
                    } else if try_handle_npc_action(
                        &mut out,
                        input,
                        &self.world,
                        &mut self.item_locations,
                        &self.npc_locations,
                        &self.current_room_id,
                        &mut self.flags,
                    ) {
                        // handled
                    } else if try_handle_action(
                        &mut out,
                        current_room,
                        input,
                        &self.world,
                        &self.item_locations,
                        &self.current_room_id,
                        &mut self.flags,
                    ) {
                        // handled
                    } else if try_handle_global_action(
                        &mut out,
                        input,
                        &self.world,
                        &self.item_locations,
                        &self.current_room_id,
                        &mut self.flags,
                    ) {
                        // handled
                    } else {
                        out.say("I don't understand that command.");
                    }
                }
            } else {
                out.say(format!(
                    "Error: you are in an unknown room '{}'",
                    self.current_room_id
                ));
                quit = true;
            }
        }

        let flags_before = self.flags.clone();

        engine::evaluate_global_conditions(
            &mut out,
            &self.world,
            &mut self.flags,
            &self.current_room_id,
            &mut self.fired_global_conditions,
        );

        let mut changed_flags: HashSet<String> = HashSet::new();
        for f in self.flags.difference(&flags_before) {
            changed_flags.insert(f.clone());
        }
        for f in flags_before.difference(&self.flags) {
            changed_flags.insert(f.clone());
        }

        if !changed_flags.is_empty() && !rendered_room_this_turn {
            if let Some(room) = self.world.rooms.get(&self.current_room_id) {
                if room_depends_on_any_flag(
                    room,
                    &self.world,
                    &self.item_locations,
                    &self.npc_locations,
                    &changed_flags,
                ) {
                    render_room(
                        &mut out,
                        room,
                        &self.flags,
                        &self.world,
                        &self.item_locations,
                        &self.npc_locations,
                    );
                }
            }
        }

        (out, quit)
    }
}
