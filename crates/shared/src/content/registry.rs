//! Content loading, id resolution and validation.
//!
//! Every gameplay rule reads from [`ContentRegistry`]. String ids in the RON
//! files resolve once at load time to compact numeric ids, so the rest of the
//! codebase works with `u16` and never parses strings at runtime.

use std::collections::HashMap;
use std::path::Path;

use thiserror::Error;

use super::defs::*;

/// Numeric block id.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct BlockId(pub u16);

/// Numeric item id.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct ItemId(pub u16);

/// Numeric recipe id.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct RecipeId(pub u16);

/// Numeric tech node id.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct TechId(pub u16);

/// Numeric machine id.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct MachineId(pub u16);

/// Errors produced while loading or validating content.
#[derive(Debug, Error)]
pub enum ContentError {
    /// A file could not be read.
    #[error("could not read {path}: {source}")]
    Io {
        /// Path that failed.
        path: String,
        /// Underlying error.
        source: std::io::Error,
    },
    /// A file was not valid RON.
    #[error("could not parse {path}: {source}")]
    Parse {
        /// Path that failed.
        path: String,
        /// Underlying error, boxed to keep the error type small.
        source: Box<ron::error::SpannedError>,
    },
    /// A definition referenced an id that does not exist.
    #[error("validation: {0}")]
    Validation(String),
}

/// Fully loaded and validated content.
pub struct ContentRegistry {
    blocks: Vec<BlockDef>,
    items: Vec<ItemDef>,
    recipes: Vec<RecipeDef>,
    machines: Vec<MachineDef>,
    tech: Vec<TechNodeDef>,
    layers: Vec<LayerDef>,
    block_ids: HashMap<String, BlockId>,
    item_ids: HashMap<String, ItemId>,
    recipe_ids: HashMap<String, RecipeId>,
    machine_ids: HashMap<String, MachineId>,
    tech_ids: HashMap<String, TechId>,
    /// Stable hash of the raw file bytes, sent during the handshake.
    pub hash: u64,
    /// Air block id.
    pub air: BlockId,
    /// Bedrock block id.
    pub bedrock: BlockId,
}

impl ContentRegistry {
    /// Loads every content file from a directory.
    pub fn load_dir(dir: impl AsRef<Path>) -> Result<Self, ContentError> {
        let dir = dir.as_ref();
        let blocks: Vec<BlockDef> = read_ron(dir, "blocks.ron")?;
        let items: Vec<ItemDef> = read_ron(dir, "items.ron")?;
        let recipes: Vec<RecipeDef> = read_ron(dir, "recipes.ron")?;
        let machines: Vec<MachineDef> = read_ron(dir, "machines.ron")?;
        let tech: Vec<TechNodeDef> = read_ron(dir, "tech_tree.ron")?;
        let layers: Vec<LayerDef> = read_ron(dir, "layers.ron")?;
        Self::from_defs(blocks, items, recipes, machines, tech, layers)
    }

    /// Builds a registry from already-parsed definitions and validates it.
    pub fn from_defs(
        blocks: Vec<BlockDef>,
        items: Vec<ItemDef>,
        recipes: Vec<RecipeDef>,
        machines: Vec<MachineDef>,
        tech: Vec<TechNodeDef>,
        layers: Vec<LayerDef>,
    ) -> Result<Self, ContentError> {
        let mut block_ids = HashMap::new();
        for (i, b) in blocks.iter().enumerate() {
            if block_ids.insert(b.id.clone(), BlockId(i as u16)).is_some() {
                return Err(ContentError::Validation(format!("duplicate block id `{}`", b.id)));
            }
        }
        let mut item_ids = HashMap::new();
        for (i, it) in items.iter().enumerate() {
            if item_ids.insert(it.id.clone(), ItemId(i as u16)).is_some() {
                return Err(ContentError::Validation(format!("duplicate item id `{}`", it.id)));
            }
        }
        let mut recipe_ids = HashMap::new();
        for (i, r) in recipes.iter().enumerate() {
            if recipe_ids.insert(r.id.clone(), RecipeId(i as u16)).is_some() {
                return Err(ContentError::Validation(format!("duplicate recipe id `{}`", r.id)));
            }
        }
        let mut machine_ids = HashMap::new();
        for (i, m) in machines.iter().enumerate() {
            if machine_ids.insert(m.id.clone(), MachineId(i as u16)).is_some() {
                return Err(ContentError::Validation(format!("duplicate machine id `{}`", m.id)));
            }
        }
        let mut tech_ids = HashMap::new();
        for (i, t) in tech.iter().enumerate() {
            if tech_ids.insert(t.id.clone(), TechId(i as u16)).is_some() {
                return Err(ContentError::Validation(format!("duplicate tech id `{}`", t.id)));
            }
        }

        let air = *block_ids
            .get("air")
            .ok_or_else(|| ContentError::Validation("missing block `air`".into()))?;
        let bedrock = *block_ids
            .get("bedrock")
            .ok_or_else(|| ContentError::Validation("missing block `bedrock`".into()))?;

        let hash = content_hash(&blocks, &items, &recipes, &machines, &tech, &layers);

        let reg = Self {
            blocks,
            items,
            recipes,
            machines,
            tech,
            layers,
            block_ids,
            item_ids,
            recipe_ids,
            machine_ids,
            tech_ids,
            hash,
            air,
            bedrock,
        };
        reg.validate()?;
        Ok(reg)
    }

    /// Resolves a block string id.
    pub fn block(&self, id: &str) -> Option<BlockId> {
        self.block_ids.get(id).copied()
    }
    /// Resolves an item string id.
    pub fn item(&self, id: &str) -> Option<ItemId> {
        self.item_ids.get(id).copied()
    }
    /// Resolves a recipe string id.
    pub fn recipe(&self, id: &str) -> Option<RecipeId> {
        self.recipe_ids.get(id).copied()
    }
    /// Resolves a machine string id.
    pub fn machine(&self, id: &str) -> Option<MachineId> {
        self.machine_ids.get(id).copied()
    }
    /// Resolves a tech string id.
    pub fn tech(&self, id: &str) -> Option<TechId> {
        self.tech_ids.get(id).copied()
    }

    /// Block definition by numeric id.
    pub fn block_def(&self, id: BlockId) -> Option<&BlockDef> {
        self.blocks.get(id.0 as usize)
    }
    /// Item definition by numeric id.
    pub fn item_def(&self, id: ItemId) -> Option<&ItemDef> {
        self.items.get(id.0 as usize)
    }
    /// Recipe definition by numeric id.
    pub fn recipe_def(&self, id: RecipeId) -> Option<&RecipeDef> {
        self.recipes.get(id.0 as usize)
    }
    /// Machine definition by numeric id.
    pub fn machine_def(&self, id: MachineId) -> Option<&MachineDef> {
        self.machines.get(id.0 as usize)
    }
    /// Tech definition by numeric id.
    pub fn tech_def(&self, id: TechId) -> Option<&TechNodeDef> {
        self.tech.get(id.0 as usize)
    }
    /// Layer definitions.
    pub fn layers(&self) -> &[LayerDef] {
        &self.layers
    }
    /// All blocks.
    pub fn blocks(&self) -> &[BlockDef] {
        &self.blocks
    }
    /// All items.
    pub fn items(&self) -> &[ItemDef] {
        &self.items
    }
    /// All recipes.
    pub fn recipes(&self) -> &[RecipeDef] {
        &self.recipes
    }
    /// All machines.
    pub fn machines(&self) -> &[MachineDef] {
        &self.machines
    }
    /// All tech nodes.
    pub fn tech_nodes(&self) -> &[TechNodeDef] {
        &self.tech
    }
    /// Recipes a given machine can run.
    pub fn recipes_for_machine(&self, machine: MachineId) -> impl Iterator<Item = RecipeId> + '_ {
        let want = self.machines.get(machine.0 as usize).map(|m| m.id.as_str());
        self.recipes
            .iter()
            .enumerate()
            .filter(move |(_, r)| Some(r.machine.as_str()) == want)
            .map(|(i, _)| RecipeId(i as u16))
    }
    /// Display name of an item.
    pub fn item_name(&self, id: ItemId) -> &str {
        self.item_def(id).map(|d| d.name.as_str()).unwrap_or("?")
    }
    /// String id of an item.
    pub fn item_str(&self, id: ItemId) -> &str {
        self.item_def(id).map(|d| d.id.as_str()).unwrap_or("?")
    }

    /// Validates cross references. Fails fast with a readable message.
    pub fn validate(&self) -> Result<(), ContentError> {
        let mut errs: Vec<String> = Vec::new();

        for b in &self.blocks {
            for (drop, count) in &b.drops {
                if self.item(drop).is_none() {
                    errs.push(format!("block `{}` drops unknown item `{drop}`", b.id));
                }
                if *count == 0 {
                    errs.push(format!("block `{}` drops zero of `{drop}`", b.id));
                }
            }
            if let Some(m) = &b.machine
                && self.machine(m).is_none() {
                    errs.push(format!("block `{}` refers to unknown machine `{m}`", b.id));
                }
            if b.min_tier == 0 && !b.unbreakable && !b.passable {
                errs.push(format!("block `{}` is unbreakable but not marked unbreakable", b.id));
            }
        }

        for m in &self.machines {
            for (item, _) in &m.build_cost {
                if self.item(item).is_none() {
                    errs.push(format!("machine `{}` build cost uses unknown item `{item}`", m.id));
                }
            }
            if let Some(t) = &m.tech
                && self.tech(t).is_none() {
                    errs.push(format!("machine `{}` requires unknown tech `{t}`", m.id));
                }
            if let MachineEnergy::Fuel { fuel, .. } = &m.energy
                && self.item(fuel).is_none() {
                    errs.push(format!("machine `{}` burns unknown fuel `{fuel}`", m.id));
                }
        }

        for r in &self.recipes {
            if r.machine != "hand" && self.machine(&r.machine).is_none() {
                errs.push(format!(
                    "recipe `{}` names machine `{}` which does not exist",
                    r.id, r.machine
                ));
            }
            for (item, _) in r.inputs.iter().chain(r.outputs.iter()) {
                if self.item(item).is_none() {
                    errs.push(format!("recipe `{}` uses unknown item `{item}`", r.id));
                }
            }
            if r.inputs.is_empty() || r.outputs.is_empty() {
                errs.push(format!("recipe `{}` must have inputs and outputs", r.id));
            }
            if let Some(t) = &r.tech
                && self.tech(t).is_none() {
                    errs.push(format!("recipe `{}` requires unknown tech `{t}`", r.id));
                }
        }

        for t in &self.tech {
            for p in &t.parents {
                if self.tech(p).is_none() {
                    errs.push(format!("tech `{}` has missing parent `{p}`", t.id));
                }
            }
            for (item, _) in &t.cost {
                if self.item(item).is_none() {
                    errs.push(format!("tech `{}` costs unknown item `{item}`", t.id));
                }
            }
            for u in &t.unlocks {
                if self.recipe(u).is_none()
                    && self.machine(u).is_none()
                    && self.block(u).is_none()
                    && self.item(u).is_none()
                {
                    errs.push(format!("tech `{}` unlocks unknown content `{u}`", t.id));
                }
            }
        }

        // Tech graph must be acyclic.
        if let Err(cycle) = self.check_acyclic() {
            errs.push(cycle);
        }

        // Every ore block must be used by a layer and drop something used somewhere.
        let used_ores: std::collections::HashSet<&str> = self
            .layers
            .iter()
            .flat_map(|l| l.ores.iter().map(|o| o.block.as_str()))
            .collect();
        for b in &self.blocks {
            if matches!(b.pattern, Pattern::OreFleck(_)) && !used_ores.contains(b.id.as_str()) {
                errs.push(format!("ore block `{}` is not placed by any layer", b.id));
            }
        }

        // Every item must be obtainable: dropped by a block, produced by a recipe,
        // or granted at spawn.
        let produced: std::collections::HashSet<&str> = self
            .recipes
            .iter()
            .flat_map(|r| r.outputs.iter().map(|(i, _)| i.as_str()))
            .chain(self.blocks.iter().flat_map(|b| b.drops.iter().map(|(i, _)| i.as_str())))
            .chain(SPAWN_ITEMS.iter().copied())
            .collect();
        for it in &self.items {
            if !produced.contains(it.id.as_str()) {
                errs.push(format!("item `{}` cannot be obtained by any means", it.id));
            }
        }

        // Every recipe must be reachable from some tech node (or hand-crafted).
        for r in &self.recipes {
            let reachable = r.hand
                || r.tech.is_some()
                || self
                    .tech
                    .iter()
                    .any(|t| t.unlocks.iter().any(|u| u == &r.id));
            if !reachable {
                errs.push(format!("recipe `{}` is not unlocked by any tech node", r.id));
            }
        }

        for l in &self.layers {
            if self.block(&l.base_block).is_none() {
                errs.push(format!("layer `{}` uses unknown base block `{}`", l.id, l.base_block));
            }
            for (b, _) in &l.mix {
                if self.block(b).is_none() {
                    errs.push(format!("layer `{}` mixes unknown block `{b}`", l.id));
                }
            }
            for o in &l.ores {
                if self.block(&o.block).is_none() {
                    errs.push(format!("layer `{}` spawns unknown ore `{}`", l.id, o.block));
                }
            }
        }

        if errs.is_empty() {
            Ok(())
        } else {
            Err(ContentError::Validation(errs.join("\n  - ")))
        }
    }

    fn check_acyclic(&self) -> Result<(), String> {
        #[derive(Clone, Copy, PartialEq)]
        enum Mark {
            New,
            Active,
            Done,
        }
        fn visit(
            reg: &ContentRegistry,
            node: usize,
            marks: &mut [Mark],
            path: &mut Vec<String>,
        ) -> Result<(), String> {
            match marks[node] {
                Mark::Done => return Ok(()),
                Mark::Active => {
                    return Err(format!(
                        "tech tree contains a cycle: {}",
                        path.join(" -> ")
                    ));
                }
                Mark::New => {}
            }
            marks[node] = Mark::Active;
            path.push(reg.tech[node].id.clone());
            for p in &reg.tech[node].parents {
                if let Some(pid) = reg.tech(p) {
                    visit(reg, pid.0 as usize, marks, path)?;
                }
            }
            path.pop();
            marks[node] = Mark::Done;
            Ok(())
        }
        let mut marks = vec![Mark::New; self.tech.len()];
        for i in 0..self.tech.len() {
            let mut path = Vec::new();
            visit(self, i, &mut marks, &mut path)?;
        }
        Ok(())
    }
}

/// Items every player starts with (used by the "obtainable" validation rule).
pub const SPAWN_ITEMS: &[&str] = &["steel_pickaxe", "oil_lamp"];

fn read_ron<T: serde::de::DeserializeOwned>(
    dir: &Path,
    file: &str,
) -> Result<T, ContentError> {
    let path = dir.join(file);
    let text = std::fs::read_to_string(&path).map_err(|source| ContentError::Io {
        path: path.display().to_string(),
        source,
    })?;
    ron::from_str(&text).map_err(|source| ContentError::Parse {
        path: path.display().to_string(),
        source: Box::new(source),
    })
}

fn content_hash(
    blocks: &[BlockDef],
    items: &[ItemDef],
    recipes: &[RecipeDef],
    machines: &[MachineDef],
    tech: &[TechNodeDef],
    layers: &[LayerDef],
) -> u64 {
    // Serialise deterministically (RON of the parsed structures) and hash.
    let mut s = String::new();
    s.push_str(&ron::ser::to_string(blocks).unwrap_or_default());
    s.push_str(&ron::ser::to_string(items).unwrap_or_default());
    s.push_str(&ron::ser::to_string(recipes).unwrap_or_default());
    s.push_str(&ron::ser::to_string(machines).unwrap_or_default());
    s.push_str(&ron::ser::to_string(tech).unwrap_or_default());
    s.push_str(&ron::ser::to_string(layers).unwrap_or_default());
    // FNV-1a 64 bit.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h
}
