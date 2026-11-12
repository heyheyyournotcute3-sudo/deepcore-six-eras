//! Deepcore shared game logic: content registries, voxels, world generation,
//! movement, inventory, crafting, power and hazards. No rendering, no sockets.

use bevy::prelude::*;

pub mod blocks;
pub mod content;
pub mod crafting;
pub mod hazards;
pub mod inventory;
pub mod movement;
pub mod power;
pub mod protocol;
pub mod voxel;
pub mod worldgen;

/// Simulation rate of the authoritative server, in Hz.
pub const TICK_HZ: u32 = 30;
/// World width in blocks (X and Z).
pub const WORLD_SIZE: i32 = 1024;
/// World height in blocks (Y).
pub const WORLD_HEIGHT: i32 = 512;
/// Chunk edge length in blocks.
pub const CHUNK_SIZE: i32 = 32;
/// Y coordinate of the flat camp surface.
pub const SURFACE_Y: i32 = 504;
/// Fixed seed used when none is supplied (hosting defaults).
pub const DEFAULT_SEED: u64 = 0x0DEE_C0DE_5EED_1234;

/// Shared plugin: content registry plus the fixed 30 Hz simulation clock that
/// client and server both agree on.
pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(TICK_HZ as f64));
    }
}

/// Path to the `assets/` directory, resolved at build time.
pub fn assets_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("DEEPCORE_ASSETS"))
}

/// Path to the content data directory.
pub fn data_dir() -> std::path::PathBuf {
    assets_dir().join("data")
}

/// Commonly used shared types.
pub mod prelude {
    pub use crate::content::{BlockId, ContentRegistry, ItemId, RecipeId, TechId};
    pub use crate::inventory::{Inventory, ItemStack, Slot};
    pub use crate::voxel::{BlockPos, Chunk, ChunkPos};
}
