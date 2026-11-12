//! Network protocol: replicated components, client intents and server events.
//!
//! Everything here is versioned by [`PROTOCOL_VERSION`]. Bump it whenever the
//! shape of a message or replicated component changes.

use serde::{Deserialize, Serialize};

use crate::content::{BlockId, ItemId, MachineId, RecipeId, TechId};
use crate::hazards::Vitals;
use crate::inventory::ItemStack;
use crate::voxel::{BlockPos, ChunkPos, Face};

/// Bumped whenever a replicated component or message changes shape.
pub const PROTOCOL_VERSION: u32 = 1;

/// Display name length limit.
pub const MAX_NAME_LEN: usize = 24;

/// Chunks sent around the player horizontally.
pub const VIEW_CHUNKS_H: i32 = 6;
/// Chunks sent around the player vertically.
pub const VIEW_CHUNKS_V: i32 = 4;
/// Maximum reach for interacting, in blocks.
pub const REACH: f32 = 5.0;
/// Extra reach granted by tools with `reach_bonus`.
pub const REACH_DRILL: f32 = 6.0;

// ---------------------------------------------------------------------------
// Client -> server intents
// ---------------------------------------------------------------------------

/// A movement input sample. Unreliable, sequenced.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MoveInputMsg {
    /// Sequence number.
    pub seq: u32,
    /// Local move direction.
    pub dir: (f32, f32),
    /// Jump.
    pub jump: bool,
    /// Sprint.
    pub sprint: bool,
    /// Crouch.
    pub crouch: bool,
    /// Yaw.
    pub yaw: f32,
    /// Pitch.
    pub pitch: f32,
}

/// Start mining a block.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct StartMining {
    /// Target block.
    pub block: BlockPos,
    /// Face the player hit.
    pub face: Face,
}

/// Stop mining.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct StopMining;

/// Place a block from an inventory slot.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PlaceBlock {
    /// Target position (the empty cell).
    pub pos: BlockPos,
    /// Face used, for orientation.
    pub face: Face,
    /// Source slot.
    pub slot: u8,
}

/// Interact with a block entity (open UI, use machine).
#[derive(bevy::prelude::Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Interact {
    /// Target block.
    pub target: BlockPos,
}

/// Move items between slots. `to < 0` means "into the machine at `target`".
#[derive(bevy::prelude::Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MoveItems {
    /// Source inventory slot, or a machine slot when `from_machine`.
    pub from: i16,
    /// Destination slot.
    pub to: i16,
    /// Amount, `0` means all.
    pub count: u32,
    /// Machine position if either end is a machine.
    pub machine: Option<BlockPos>,
    /// True when `from` refers to a machine slot.
    pub from_machine: bool,
}

/// Craft a recipe by hand or at a workbench.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Craft {
    /// Recipe.
    pub recipe: RecipeId,
    /// Number of crafts.
    pub count: u32,
}

/// Choose which recipe a machine runs.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SetMachineRecipe {
    /// Machine block position.
    pub machine: BlockPos,
    /// Recipe to run.
    pub recipe: RecipeId,
}

/// Deliver items to a tech node at the research desk.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ResearchDeposit {
    /// Node.
    pub node: TechId,
}

/// Chat message.
#[derive(bevy::prelude::Message, Clone, Debug, Serialize, Deserialize)]
pub struct Chat {
    /// Text, truncated to 256 chars.
    pub text: String,
}

/// Request respawn after death.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Respawn;

/// Toggle a reactor control rod by a delta.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SetControlRods {
    /// Reactor block position.
    pub reactor: BlockPos,
    /// Target insertion `0..=1`.
    pub insertion: f32,
}

// ---------------------------------------------------------------------------
// Server -> client events
// ---------------------------------------------------------------------------

/// Handshake result.
#[derive(bevy::prelude::Message, Clone, Debug, Serialize, Deserialize)]
pub enum HandshakeResult {
    /// Accepted; the server tells the client its seed and content hash.
    Accepted {
        /// World seed.
        seed: u64,
        /// Content hash.
        content_hash: u64,
        /// Server tick rate.
        tick_hz: u32,
    },
    /// Rejected with a human readable reason.
    Rejected {
        /// Reason shown to the player.
        reason: String,
    },
}

/// Client hello, sent before anything else.
#[derive(bevy::prelude::Message, Clone, Debug, Serialize, Deserialize)]
pub struct ClientHello {
    /// Protocol version.
    pub protocol: u32,
    /// Content hash of the client's data files.
    pub content_hash: u64,
    /// Display name.
    pub name: String,
}

/// A single block change.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct BlockChange {
    /// Position.
    pub pos: BlockPos,
    /// New block.
    pub block: BlockId,
}

/// A batch of block changes for one tick.
#[derive(bevy::prelude::Message, Clone, Debug, Default, Serialize, Deserialize)]
pub struct BlockChangeBatch {
    /// The changes.
    pub changes: Vec<BlockChange>,
}

/// Chunk payload sent to a client.
#[derive(bevy::prelude::Message, Clone, Debug, Serialize, Deserialize)]
pub struct ChunkPayload {
    /// Chunk position.
    pub pos: ChunkPos,
    /// zstd-compressed wire bytes.
    pub data: Vec<u8>,
    /// True if this message unloads the chunk instead.
    pub unload: bool,
}

/// A notification for the HUD or chat log.
#[derive(bevy::prelude::Message, Clone, Debug, Serialize, Deserialize)]
pub struct Notification {
    /// Text.
    pub text: String,
}

/// Full inventory snapshot for the owner.
#[derive(bevy::prelude::Message, Clone, Debug, Serialize, Deserialize)]
pub struct InventorySnapshot {
    /// Slots.
    pub slots: Vec<Option<ItemStack>>,
    /// Selected hotbar slot.
    pub selected: u8,
}

/// State of a machine UI the player has open.
#[derive(bevy::prelude::Message, Clone, Debug, Serialize, Deserialize)]
pub struct MachineUiState {
    /// Machine block position.
    pub pos: BlockPos,
    /// Machine type.
    pub kind: MachineId,
    /// Input slots.
    pub inputs: Vec<Option<ItemStack>>,
    /// Output slots.
    pub outputs: Vec<Option<ItemStack>>,
    /// Fuel slot.
    pub fuel: Option<ItemStack>,
    /// Selected recipe.
    pub recipe: Option<RecipeId>,
    /// Progress `0..=1`.
    pub progress: f32,
    /// Idle reason text.
    pub idle: String,
    /// Control rod insertion for reactors.
    pub rod_insertion: f32,
    /// Core temperature for reactors.
    pub temperature: f32,
}

/// Team tech tree state.
#[derive(bevy::prelude::Message, Clone, Debug, Default, Serialize, Deserialize)]
pub struct TechTreeState {
    /// Unlocked node ids.
    pub unlocked: Vec<TechId>,
    /// Delivered cost so far, per node: `(node, item, amount)`.
    pub delivered: Vec<(TechId, ItemId, u32)>,
}

/// Network overlay statistics for one network.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct NetworkStatsMsg {
    /// Network index.
    pub index: u32,
    /// Supply.
    pub supply: f32,
    /// Demand.
    pub demand: f32,
    /// Stored energy.
    pub stored: f32,
}

/// Player death notice.
#[derive(bevy::prelude::Message, Clone, Debug, Serialize, Deserialize)]
pub struct PlayerDied {
    /// Cause shown on the death screen.
    pub cause: String,
}

/// Replicated player state for other clients.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PlayerReplicated {
    /// Position.
    pub pos: [f32; 3],
    /// Yaw.
    pub yaw: f32,
    /// Pitch.
    pub pitch: f32,
    /// Held item.
    pub held: Option<ItemId>,
    /// Current health fraction for the health bar.
    pub health: f32,
}

/// Authoritative movement correction for the owning client.
#[derive(bevy::prelude::Component, bevy::prelude::Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MovementCorrection {
    /// Last processed input sequence.
    pub seq: u32,
    /// Authoritative position.
    pub pos: [f32; 3],
    /// Authoritative velocity.
    pub vel: [f32; 3],
    /// Grounded flag.
    pub grounded: bool,
}

/// Vitals snapshot for the owner.
#[derive(bevy::prelude::Component, bevy::prelude::Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct VitalsMsg(pub Vitals);

/// Chat line broadcast to everyone.
#[derive(bevy::prelude::Message, Clone, Debug, Serialize, Deserialize)]
pub struct ChatLine {
    /// Sender display name.
    pub from: String,
    /// Text.
    pub text: String,
}

/// Why a mining or interaction intent was refused, sent back to the client.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MineRefusalMsg {
    /// The target is out of reach or not looked at.
    OutOfReach,
    /// The tool tier is too low.
    TooHard {
        /// Tier the block requires.
        required: u8,
    },
    /// The block cannot be broken at all.
    Unbreakable,
    /// There was nothing to break.
    Nothing,
}

impl MineRefusalMsg {
    /// Text shown on the HUD.
    pub fn text(self) -> String {
        match self {
            MineRefusalMsg::OutOfReach => "Out of reach".to_string(),
            MineRefusalMsg::TooHard { required } => format!("Too hard — needs Tier {required}"),
            MineRefusalMsg::Unbreakable => "Unbreakable".to_string(),
            MineRefusalMsg::Nothing => String::new(),
        }
    }
}
