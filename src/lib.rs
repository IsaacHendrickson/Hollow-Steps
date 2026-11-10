//! Reusable game state for Hollow Steps.
//!
//! Rendering and keyboard APIs stay in the binary crate. This library owns
//! gameplay state so it can be tested without opening a window.

mod action_parameters;
pub mod action_runtime;
pub mod behavior;
pub mod behavior_values;
pub mod body;
pub mod body_debug;
pub mod boss_definition;
pub mod boss_editor;
pub mod boss_special;
pub mod camera;
pub mod coin;
pub mod combat;
pub mod damage;
pub mod definition_editor;
pub mod editor;
pub mod editor_guard;
pub mod editor_history;
pub mod editor_view;
pub mod enemy;
pub mod enemy_attack;
pub mod game;
pub mod health;
pub mod item;
pub mod player;
pub mod pose;
pub mod preset;
pub mod room;
pub mod room_size;
pub mod search;
pub mod terrain;
pub mod timestep;
