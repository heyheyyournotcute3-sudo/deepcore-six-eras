//! Raw RON content definitions. These structs mirror the files in
//! `assets/data/` one-to-one; string ids are resolved to numeric ids by
//! [`crate::content::ContentRegistry`].

use serde::{Deserialize, Serialize};

/// How a block face is drawn by the procedural atlas generator.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    /// Flat colour.
    Solid,
    /// Random darker/lighter speckles, used for stone and dirt.
    Speckle,
    /// Bright flecks in a second colour, used for ores.
    OreFleck((f32, f32, f32)),
    /// Emissive glow, used for uranium and lamps.
    Glow,
    /// Horizontal bands, used for sedimentary rock.
    Bands,
}

/// A block definition from `blocks.ron`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockDef {
    /// Stable string id.
    pub id: String,
    /// Seconds to break with a tier-1 tool of speed 1.
    pub hardness: f32,
    /// Minimum tool tier required.
    pub min_tier: u8,
    /// Items dropped when mined.
    #[serde(default)]
    pub drops: Vec<(String, u32)>,
    /// Base colour, linear-ish RGB in `0..1`.
    pub color: (f32, f32, f32),
    /// Texture pattern.
    pub pattern: Pattern,
    /// Emissive strength, `0` for none.
    #[serde(default)]
    pub emissive: f32,
    /// Radiation emitted by the block in the world.
    #[serde(default)]
    pub radiation: Option<RadiationDef>,
    /// Which layer the block is native to, for the atlas and for hints.
    #[serde(default)]
    pub layer_hint: Option<u8>,
    /// True if the block is a liquid that spreads.
    #[serde(default)]
    pub liquid: bool,
    /// True if the block is unbreakable (bedrock, world walls).
    #[serde(default)]
    pub unbreakable: bool,
    /// True if the player can walk through it (air, water, gas).
    #[serde(default)]
    pub passable: bool,
    /// Optional block entity this block represents, e.g. `"stone_furnace"`.
    #[serde(default)]
    pub machine: Option<String>,
}

/// Radiation field of a block.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RadiationDef {
    /// Radius in blocks.
    pub radius: f32,
    /// Dose per second at the block, falling off linearly to the radius.
    pub dose_per_sec: f32,
}

/// Energy requirement of a tool.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ToolEnergy {
    /// No energy needed.
    None,
    /// Burns oil, refilled at a boiler.
    Oil { capacity: u32 },
    /// Steam canister, refilled at the boiler.
    Steam { capacity: u32 },
    /// Battery charge, refilled at a charging station.
    Battery { capacity: u32 },
    /// Nuclear core cell, consumed per break.
    CoreCell { breaks: u32 },
}

/// Tool behaviour of an item.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDef {
    /// Mining tier.
    pub tier: u8,
    /// Speed multiplier in the break-time formula.
    pub speed: f32,
    /// Area mined per action, `(width, height)` of the face.
    pub area: (u8, u8),
    /// Energy source.
    pub energy: ToolEnergy,
    /// Extra reach in blocks.
    #[serde(default)]
    pub reach_bonus: f32,
}

/// Item category, used by the UI to group items.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemCategory {
    /// Raw ore or harvested material.
    Raw,
    /// Smelted ingot or refined material.
    Material,
    /// Intermediate chemical or part.
    Component,
    /// Tool.
    Tool(ToolDef),
    /// Placeable block.
    Block,
    /// Wearable hazard gear.
    Gear,
}

/// An item definition from `items.ron`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemDef {
    /// Stable string id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Maximum stack size.
    pub stack: u32,
    /// Category.
    pub category: ItemCategory,
    /// Optional long description shown in the UI.
    #[serde(default)]
    pub description: String,
}

/// What a machine consumes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MachineEnergy {
    /// Burns fuel items from a fuel slot.
    Fuel { slots: u8, fuel: String },
    /// Consumes steam units per second from a pipe network.
    Steam { per_sec: f32 },
    /// Draws kilowatts from a cable network.
    Electric { kw: f32 },
    /// Hand cranked: no external energy, just slow.
    Manual,
}

/// A machine definition from `machines.ron`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MachineDef {
    /// Stable string id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Footprint in blocks `(x, y, z)`.
    pub size: (u8, u8, u8),
    /// Energy requirement.
    pub energy: MachineEnergy,
    /// Number of input slots.
    pub input_slots: u8,
    /// Number of output slots.
    pub output_slots: u8,
    /// Items needed to build it in the world.
    pub build_cost: Vec<(String, u32)>,
    /// Tech node that unlocks it.
    #[serde(default)]
    pub tech: Option<String>,
    /// Cascade configuration for centrifuges.
    #[serde(default)]
    pub cascade: Option<CascadeDef>,
    /// Maximum upgrade modules the machine accepts.
    #[serde(default)]
    pub upgrade_slots: u8,
    /// True if the machine is a power producer (boiler, generator, reactor).
    #[serde(default)]
    pub producer: bool,
    /// Reactor-specific tuning.
    #[serde(default)]
    pub reactor: Option<ReactorDef>,
}

/// Centrifuge cascade rules.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CascadeDef {
    /// Minimum number of linked centrifuges.
    pub min: u8,
    /// Maximum number of linked centrifuges.
    pub max: u8,
}

/// Reactor tuning from `machines.ron`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ReactorDef {
    /// Nominal output in kW.
    pub output_kw: f32,
    /// Temperature at which the alarm sounds.
    pub warn_temp: f32,
    /// Temperature at which the reactor scrams or melts down.
    pub limit_temp: f32,
    /// Degrees per second added per withdrawn rod at full coolant.
    pub heat_per_rod: f32,
    /// Degrees per second removed per coolant unit.
    pub cooling_rate: f32,
}

/// A recipe from `recipes.ron`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecipeDef {
    /// Stable string id.
    pub id: String,
    /// Machine id, or `"hand"` for workbench crafting.
    pub machine: String,
    /// Seconds to process.
    pub time: f32,
    /// Consumed items.
    pub inputs: Vec<(String, u32)>,
    /// Produced items.
    pub outputs: Vec<(String, u32)>,
    /// Tech node required.
    #[serde(default)]
    pub tech: Option<String>,
    /// Power draw override in kW, else the machine's own value is used.
    #[serde(default)]
    pub power_kw: Option<f32>,
    /// True if the recipe is hand-crafted at the workbench.
    #[serde(default)]
    pub hand: bool,
}

/// A tech tree node from `tech_tree.ron`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TechNodeDef {
    /// Stable string id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Era number, 0..=5.
    pub era: u8,
    /// Parent node ids that must be unlocked first.
    #[serde(default)]
    pub parents: Vec<String>,
    /// Items that must be delivered to the Research Desk.
    #[serde(default)]
    pub cost: Vec<(String, u32)>,
    /// Content ids this node unlocks (machines, recipes, blocks, items).
    #[serde(default)]
    pub unlocks: Vec<String>,
    /// One-line description for the tree UI.
    #[serde(default)]
    pub description: String,
}

/// An ore spawn rule inside a layer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OreSpawnDef {
    /// Block id of the ore block.
    pub block: String,
    /// Chance per candidate blob position, `0..1`.
    pub frequency: f32,
    /// Vein size range in blocks.
    pub vein_size: (u32, u32),
}

/// A hazard active in a layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum HazardDef {
    /// Water pockets that release on breach.
    Flooding {
        /// Chance per chunk.
        pocket_chance: f32,
        /// Maximum blocks released.
        max_volume: u32,
    },
    /// Methane pockets that drain oxygen and explode on ignition.
    Gas {
        /// Chance per chunk.
        pocket_chance: f32,
        /// Oxygen drained per second while inside.
        oxygen_per_sec: f32,
    },
    /// Ambient heat that raises body heat.
    Heat {
        /// Heat added per second.
        per_sec: f32,
    },
    /// Ceiling collapses when mined beneath.
    CaveIn {
        /// Maximum unsupported span before collapse.
        max_span: u32,
    },
    /// Ambient radiation dose per second.
    Radiation {
        /// Dose per second.
        per_sec: f32,
    },
}

/// A world layer from `layers.ron`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayerDef {
    /// Stable id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Depth range below the surface `(top, bottom)` in blocks.
    pub depth: (u32, u32),
    /// Base block id.
    pub base_block: String,
    /// Secondary blocks mixed into the base, with their share `0..1`.
    #[serde(default)]
    pub mix: Vec<(String, f32)>,
    /// Ore spawns.
    #[serde(default)]
    pub ores: Vec<OreSpawnDef>,
    /// Hazards.
    #[serde(default)]
    pub hazards: Vec<HazardDef>,
    /// Ambient temperature in degrees.
    #[serde(default)]
    pub ambient_temp: f32,
    /// Cave density multiplier.
    #[serde(default = "one")]
    pub cave_density: f32,
}

fn one() -> f32 {
    1.0
}
