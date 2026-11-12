//! Deepcore server library: the authoritative simulation and its plugins.
//! `main.rs` is a thin wrapper so the same plugins can be hosted inside the
//! client for "Host game".

pub mod net;
pub mod plugin;
pub mod sim;
pub mod world;

pub use net::*;
pub use plugin::{ServerPlugin, spawn_player};
pub use sim::resources::*;
pub use world::World;
