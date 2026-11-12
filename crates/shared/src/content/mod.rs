//! Data-driven content: RON definitions, id resolution and validation.

pub mod defs;
pub mod registry;

pub use defs::*;
pub use registry::{
    BlockId, ContentError, ContentRegistry, ItemId, MachineId, RecipeId, TechId,
};
