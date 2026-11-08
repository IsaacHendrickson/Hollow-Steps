//! Reusable game state for Hollow Steps.
//!
//! Rendering and keyboard APIs stay in the binary crate. This library owns
//! gameplay state so it can be tested without opening a window.

pub mod boss;
pub mod boss_special;
pub mod coin;
pub mod combat;
pub mod enemy;
pub mod game;
pub mod health;
pub mod item;
pub mod player;
pub mod room;
