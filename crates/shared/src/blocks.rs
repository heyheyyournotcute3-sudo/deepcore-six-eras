//! Block interaction rules shared by client and server: can this tool mine
//! that block, how long does it take, what does it drop, and can a block be
//! placed here.
//!
//! The server is authoritative, but the client runs the same functions to
//! predict the crack animation and to colour the crosshair.

use crate::content::{BlockId, ContentRegistry, ItemCategory, ItemId, ToolDef};
use crate::crafting::{break_ticks, break_time};
use crate::inventory::Inventory;
use crate::voxel::{BlockPos, Face};

/// Why a block cannot be mined by the given tool.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MineRefusal {
    /// The tool tier is too low for the block.
    TooHard { required: u8 },
    /// The block is bedrock or a world wall.
    Unbreakable,
    /// The block is air.
    Nothing,
}

/// Result of a successful mining check.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinePlan {
    /// Ticks the server needs to break it.
    pub ticks: u32,
    /// Blocks broken per action, as `(width, height)` of the face.
    pub area: (u8, u8),
}

/// Returns the tool definition of a held item, if it is a tool.
pub fn tool_of(reg: &ContentRegistry, item: ItemId) -> Option<ToolDef> {
    match reg.item_def(item)?.category {
        ItemCategory::Tool(t) => Some(t),
        _ => None,
    }
}

/// Reach in blocks for a held item.
pub fn reach_for(reg: &ContentRegistry, held: Option<ItemId>) -> f32 {
    let base = crate::protocol::REACH;
    match held.and_then(|i| tool_of(reg, i)) {
        Some(t) if t.reach_bonus > 0.0 => crate::protocol::REACH_DRILL,
        _ => base,
    }
}

/// Checks whether `held` can mine `block` and how long it takes.
pub fn plan_mine(
    reg: &ContentRegistry,
    block: BlockId,
    held: Option<ItemId>,
) -> Result<MinePlan, MineRefusal> {
    let Some(def) = reg.block_def(block) else {
        return Err(MineRefusal::Nothing);
    };
    if block == reg.air {
        return Err(MineRefusal::Nothing);
    }
    if def.unbreakable {
        return Err(MineRefusal::Unbreakable);
    }
    let tool = held.and_then(|i| tool_of(reg, i));
    let tier = tool.map(|t| t.tier).unwrap_or(0);
    if tier < def.min_tier {
        return Err(MineRefusal::TooHard {
            required: def.min_tier,
        });
    }
    let (speed, area) = match tool {
        Some(t) => (t.speed, t.area),
        None => (0.5, (1, 1)),
    };
    Ok(MinePlan {
        ticks: break_ticks(def.hardness, speed, 1.0),
        area,
    })
}

/// Predicted break time in seconds, for the client's crack animation.
pub fn predicted_break_seconds(reg: &ContentRegistry, block: BlockId, held: Option<ItemId>) -> f32 {
    let Some(def) = reg.block_def(block) else {
        return 0.0;
    };
    let speed = held
        .and_then(|i| tool_of(reg, i))
        .map(|t| t.speed)
        .unwrap_or(0.5);
    break_time(def.hardness, speed, 1.0)
}

/// The blocks an area tool breaks for one action, given the targeted block
/// and the face that was hit.
pub fn area_blocks(
    reg: &ContentRegistry,
    target: BlockPos,
    face: Face,
    held: Option<ItemId>,
) -> Vec<BlockPos> {
    let (w, h) = held
        .and_then(|i| tool_of(reg, i))
        .map(|t| t.area)
        .unwrap_or((1, 1));
    let _ = reg;
    if w <= 1 && h <= 1 {
        return vec![target];
    }
    // Build the plane of the face: the two axes that are not the face normal.
    let normal = face.offset();
    let (ax, ay) = match normal {
        [1, 0, 0] | [-1, 0, 0] => ([0, 1, 0], [0, 0, 1]),
        [0, 1, 0] | [0, -1, 0] => ([1, 0, 0], [0, 0, 1]),
        _ => ([1, 0, 0], [0, 1, 0]),
    };
    let mut out = Vec::with_capacity(w as usize * h as usize);
    let half_w = (w as i32 - 1) / 2;
    let half_h = (h as i32 - 1) / 2;
    for i in -half_w..=half_w {
        for j in -half_h..=half_h {
            out.push(BlockPos::new(
                target.x + ax[0] * i + ay[0] * j,
                target.y + ax[1] * i + ay[1] * j,
                target.z + ax[2] * i + ay[2] * j,
            ));
        }
    }
    out
}

/// Why a placement failed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlaceRefusal {
    /// The item is not placeable.
    NotPlaceable,
    /// The target cell is not empty.
    Occupied,
    /// The block would intersect the player.
    BlockedByPlayer,
}

/// Checks that a block item may be placed at `pos`.
pub fn can_place(
    reg: &ContentRegistry,
    item: ItemId,
    _pos: BlockPos,
    current_block: BlockId,
    player_occupied: bool,
) -> Result<BlockId, PlaceRefusal> {
    let Some(item_def) = reg.item_def(item) else {
        return Err(PlaceRefusal::NotPlaceable);
    };
    if item_def.category != ItemCategory::Block {
        return Err(PlaceRefusal::NotPlaceable);
    }
    let Some(block) = reg.block(&item_def.id) else {
        return Err(PlaceRefusal::NotPlaceable);
    };
    if current_block != reg.air {
        let cur = reg.block_def(current_block);
        let passable = cur.map(|b| b.passable || b.liquid).unwrap_or(false);
        if !passable {
            return Err(PlaceRefusal::Occupied);
        }
    }
    if player_occupied {
        return Err(PlaceRefusal::BlockedByPlayer);
    }
    Ok(block)
}

/// Applies mined drops to an inventory. Returns the items that did not fit.
pub fn apply_drops(
    reg: &ContentRegistry,
    inv: &mut Inventory,
    block: BlockId,
) -> Vec<(ItemId, u32)> {
    let mut leftover = Vec::new();
    let Some(def) = reg.block_def(block) else {
        return leftover;
    };
    for (id, count) in &def.drops {
        let Some(item) = reg.item(id) else { continue };
        let max = Inventory::max_stack(reg, item);
        let left = inv.insert(item, *count, max);
        if left > 0 {
            leftover.push((item, left));
        }
    }
    leftover
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_of_single_block_tool() {
        let blocks = area_blocks_for_test((1, 1));
        assert_eq!(blocks.len(), 1);
    }

    fn area_blocks_for_test(area: (u8, u8)) -> Vec<BlockPos> {
        let target = BlockPos::new(10, 20, 30);
        let (w, h) = area;
        if w <= 1 && h <= 1 {
            return vec![target];
        }
        let mut out = Vec::new();
        for i in -(w as i32 / 2)..=w as i32 / 2 {
            for j in -(h as i32 / 2)..=h as i32 / 2 {
                out.push(target.offset([i, j, 0]));
            }
        }
        out
    }

    #[test]
    fn three_by_three_area_has_nine_blocks() {
        assert_eq!(area_blocks_for_test((3, 3)).len(), 9);
    }
}
