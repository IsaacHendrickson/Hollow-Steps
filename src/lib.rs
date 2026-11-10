//! Reusable game state for Hollow Steps.
//!
//! Rendering and keyboard APIs stay in the binary crate. This library owns
//! gameplay state so it can be tested without opening a window.

pub mod behavior;
pub mod behavior_values;
pub mod boss;
pub mod boss_definition;
pub mod boss_editor;
pub mod boss_special;
pub mod coin;
pub mod combat;
pub mod definition_editor;
pub mod editor;
pub mod enemy;
pub mod game;
pub mod health;
pub mod item;
pub mod player;
pub mod preset;
pub mod room;
pub mod search;
