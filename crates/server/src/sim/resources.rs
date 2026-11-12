//! Server resources that hold authoritative game state.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use deepcore_shared::content::{ItemId, MachineId, RecipeId, TechId};
use deepcore_shared::crafting::MachineState;
use deepcore_shared::hazards::Vitals;
use deepcore_shared::inventory::Inventory;
use deepcore_shared::power::{NetworkStats, NodeId, PowerGrid};
use deepcore_shared::protocol::MoveInputMsg;
use deepcore_shared::voxel::BlockPos;

/// Wraps the voxel world as a Bevy resource.
#[derive(Resource)]
pub struct WorldRes(pub crate::world::World);

/// A player connected to the server.
#[derive(Component)]
pub struct Player {
    /// Display name.
    pub name: String,
    /// Authoritative inventory.
    pub inventory: Inventory,
    /// Vitals.
    pub vitals: Vitals,
    /// Last processed movement input sequence.
    pub last_seq: u32,
    /// True while the player is mining.
    pub mining: Option<MiningState>,
    /// Machine UI currently open, if any.
    pub open_machine: Option<BlockPos>,
    /// True if the player is dead and waiting to respawn.
    pub dead: bool,
    /// Look yaw, updated from the movement stream.
    pub look_yaw: f32,
    /// Look pitch, updated from the movement stream.
    pub look_pitch: f32,
}

/// State of an in-progress mining action.
#[derive(Clone, Debug)]
pub struct MiningState {
    /// Target block.
    pub block: BlockPos,
    /// Face hit.
    pub face: deepcore_shared::voxel::Face,
    /// Ticks remaining.
    pub ticks_left: u32,
    /// Total ticks the action takes, for progress reporting.
    pub ticks_total: u32,
    /// Blocks this action breaks when it completes.
    pub targets: Vec<BlockPos>,
}

/// A placed machine block entity.
#[derive(Component, Clone)]
pub struct Machine {
    /// Block position of the machine origin (minimum corner).
    pub pos: BlockPos,
    /// Machine type.
    pub kind: MachineId,
    /// Simulation state.
    pub state: MachineState,
    /// Network node id for power/steam.
    pub node: NodeId,
    /// Reactor temperature, only meaningful for reactors.
    pub temperature: f32,
}

/// Team-shared research progress.
#[derive(Resource, Default)]
pub struct TechTree {
    /// Unlocked nodes.
    pub unlocked: HashSet<TechId>,
    /// Items delivered per node so far.
    pub delivered: HashMap<(TechId, ItemId), u32>,
}

impl TechTree {
    /// True if a recipe is available to the team.
    pub fn recipe_unlocked(&self, reg: &deepcore_shared::content::ContentRegistry, r: RecipeId) -> bool {
        let Some(def) = reg.recipe_def(r) else {
            return false;
        };
        match def.tech.as_deref() {
            None => true,
            Some(t) => reg
                .tech(t)
                .map(|id| self.unlocked.contains(&id))
                .unwrap_or(false),
        }
    }
}

/// Power and steam grid.
#[derive(Resource, Default)]
pub struct Grid {
    /// The solver state.
    pub grid: PowerGrid,
    /// Latest per-network stats.
    pub stats: Vec<NetworkStats>,
    /// Next free node id.
    pub next_node: u32,
}

impl Grid {
    /// Allocates a fresh node id.
    pub fn alloc_node(&mut self) -> NodeId {
        self.next_node += 1;
        NodeId(self.next_node)
    }
}

/// Per-player last known movement inputs, for reconciliation.
#[derive(Resource, Default)]
pub struct LastInputs {
    /// Most recent input per player entity.
    pub map: HashMap<Entity, MoveInputMsg>,
}

/// Server configuration from the command line.
#[derive(Resource, Clone, Debug)]
pub struct ServerConfig {
    /// UDP port.
    pub port: u16,
    /// World seed.
    pub seed: u64,
    /// Simulated latency in milliseconds, applied to every packet.
    pub latency_ms: u64,
    /// Simulated packet loss in percent.
    pub loss_percent: f64,
    /// Number of headless bots to spawn.
    pub bots: u32,
    /// Autosave interval in seconds.
    pub autosave_secs: f32,
    /// Directory for save files.
    pub save_dir: String,
    /// Protocol id for the renet transport.
    pub protocol_id: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 7788,
            seed: deepcore_shared::DEFAULT_SEED,
            latency_ms: 0,
            loss_percent: 0.0,
            bots: 0,
            autosave_secs: 300.0,
            save_dir: "saves".to_string(),
            protocol_id: 0x0DEE_C0DE,
        }
    }
}

impl ServerConfig {
    /// Parses `--key value` style arguments.
    pub fn from_args(args: &[String]) -> Self {
        let mut cfg = Self::default();
        let mut i = 0;
        while i < args.len() {
            let key = args[i].as_str();
            let val = args.get(i + 1).map(|s| s.as_str());
            match (key, val) {
                ("--port", Some(v)) => {
                    if let Ok(p) = v.parse() {
                        cfg.port = p;
                    }
                    i += 2;
                }
                ("--seed", Some(v)) => {
                    if let Ok(s) = v.parse() {
                        cfg.seed = s;
                    }
                    i += 2;
                }
                ("--simulate-latency", Some(v)) => {
                    if let Ok(l) = v.parse() {
                        cfg.latency_ms = l;
                    }
                    i += 2;
                }
                ("--simulate-loss", Some(v)) => {
                    if let Ok(l) = v.parse() {
                        cfg.loss_percent = l;
                    }
                    i += 2;
                }
                ("--bots", Some(v)) => {
                    if let Ok(b) = v.parse() {
                        cfg.bots = b;
                    }
                    i += 2;
                }
                ("--autosave", Some(v)) => {
                    if let Ok(a) = v.parse() {
                        cfg.autosave_secs = a;
                    }
                    i += 2;
                }
                ("--save-dir", Some(v)) => {
                    cfg.save_dir = v.to_string();
                    i += 2;
                }
                ("--protocol", Some(v)) => {
                    if let Ok(p) = v.parse() {
                        cfg.protocol_id = p;
                    }
                    i += 2;
                }
                _ => i += 1,
            }
        }
        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_parses_flags() {
        let args: Vec<String> = "--port 9000 --seed 42 --bots 3 --simulate-latency 150"
            .split_whitespace()
            .map(String::from)
            .collect();
        let cfg = ServerConfig::from_args(&args);
        assert_eq!(cfg.port, 9000);
        assert_eq!(cfg.seed, 42);
        assert_eq!(cfg.bots, 3);
        assert_eq!(cfg.latency_ms, 150);
    }

    #[test]
    fn config_ignores_unknown_flags() {
        let args: Vec<String> = "--nope 1 --port 1234".split_whitespace().map(String::from).collect();
        let cfg = ServerConfig::from_args(&args);
        assert_eq!(cfg.port, 1234);
    }
}
