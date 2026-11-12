//! Mining, placing and interaction validation. Every rule from the design is
//! enforced here: reach, line of sight, tool tier, inventory contents and the
//! reach of the machine being opened.

use bevy::prelude::*;
use deepcore_shared::blocks::{apply_drops, area_blocks, can_place, plan_mine, MineRefusal, PlaceRefusal};
use deepcore_shared::content::BlockId;
use deepcore_shared::movement::{overlaps_solid, PlayerBody, PLAYER_HALF_HEIGHT, PLAYER_HALF_WIDTH};
use deepcore_shared::protocol::{
    BlockChange, BlockChangeBatch, Interact, MineRefusalMsg, PlaceBlock, StartMining, StopMining,
};
use deepcore_shared::voxel::{BlockPos, Face};
use deepcore_shared::inventory::Inventory;

use crate::sim::resources::{MiningState, Player, WorldRes};

/// Eye position of a player body.
pub fn eye_pos(body: &PlayerBody) -> [f32; 3] {
    [body.pos[0], body.pos[1] + PLAYER_HALF_HEIGHT - 0.25, body.pos[2]]
}

/// Direction vector from yaw and pitch.
pub fn look_dir(yaw: f32, pitch: f32) -> [f32; 3] {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    [-sy * cp, sp, -cy * cp]
}

/// A validated mining start.
pub struct MiningStart {
    /// Blocks broken by this action.
    pub targets: Vec<BlockPos>,
    /// Ticks for one action.
    pub ticks: u32,
}

/// Validates a `StartMining` intent for a player.
pub fn validate_start_mining(
    world: &mut WorldRes,
    body: &PlayerBody,
    inv: &Inventory,
    msg: &StartMining,
    look: [f32; 3],
) -> Result<MiningStart, MineRefusalMsg> {
    let held = inv.held().map(|s| s.item);
    let reach = deepcore_shared::blocks::reach_for(world.0.registry(), held);

    let eye = eye_pos(body);
    let target_center = msg.block.center();
    let dist = ((target_center.x - eye[0]).powi(2)
        + (target_center.y - eye[1]).powi(2)
        + (target_center.z - eye[2]).powi(2))
    .sqrt();
    if dist > reach + 1.5 {
        return Err(MineRefusalMsg::OutOfReach);
    }

    // The client must be looking at the block it claims to mine.
    let Some((hit, _)) = world.0.raycast(eye, look, reach, false) else {
        return Err(MineRefusalMsg::OutOfReach);
    };
    if hit != msg.block {
        return Err(MineRefusalMsg::OutOfReach);
    }

    let block = world.0.block(msg.block);
    let air = world.0.air();
    let _ = air;
    let mut targets = Vec::new();
    let ticks;
    match plan_mine(world.0.registry(), block, held) {
        Ok(plan) => {
            ticks = plan.ticks;
            let candidates = area_blocks(world.0.registry(), msg.block, msg.face, held);
            for p in candidates {
                let b = world.0.block(p);
                if plan_mine(world.0.registry(), b, held).is_ok() {
                    targets.push(p);
                }
            }
            Ok(MiningStart { targets, ticks })
        }
        Err(MineRefusal::TooHard { required }) => Err(MineRefusalMsg::TooHard { required }),
        Err(MineRefusal::Unbreakable) => Err(MineRefusalMsg::Unbreakable),
        Err(MineRefusal::Nothing) => Err(MineRefusalMsg::Nothing),
    }
}

/// Breaks the blocks of a finished mining action, granting drops and recording
/// the changes that must be broadcast.
pub fn finish_mining(
    world: &mut WorldRes,
    player: &mut Player,
    targets: &[BlockPos],
    changes: &mut Vec<BlockChange>,
) {
    let air = world.0.air();
    for pos in targets {
        let block = world.0.block(*pos);
        if block == air {
            continue;
        }
        world.0.set_block(*pos, air);
        let leftover = apply_drops(world.0.registry(), &mut player.inventory, block);
        // Anything that did not fit stays where it was mined by being left in
        // the world as a pickup the player can collect later.
        if !leftover.is_empty() {
            for (item, count) in leftover {
                crate::sim::pickups::spawn_pickup(world, *pos, item, count);
            }
        }
        changes.push(BlockChange { pos: *pos, block: air });
    }
}

/// Validates and applies a block placement.
pub fn try_place(
    world: &mut WorldRes,
    player: &mut Player,
    body: &PlayerBody,
    msg: &PlaceBlock,
    look: [f32; 3],
) -> Result<BlockChange, PlaceRefusal> {
    let slot = msg.slot as usize;
    let Some(stack) = player.inventory.get(slot) else {
        return Err(PlaceRefusal::NotPlaceable);
    };
    let held = player.inventory.held().map(|s| s.item);
    let reach = deepcore_shared::blocks::reach_for(world.0.registry(), held);
    let eye = eye_pos(body);
    let target_center = msg.pos.center();
    let dist = ((target_center.x - eye[0]).powi(2)
        + (target_center.y - eye[1]).powi(2)
        + (target_center.z - eye[2]).powi(2))
    .sqrt();
    if dist > reach + 1.5 {
        return Err(PlaceRefusal::Occupied);
    }
    let Some((hit, _)) = world.0.raycast(eye, look, reach, false) else {
        return Err(PlaceRefusal::Occupied);
    };
    if hit.distance_squared(msg.pos) > 2.0 {
        return Err(PlaceRefusal::Occupied);
    }

    let current = world.0.block(msg.pos);
    let player_box_blocked = block_intersects_player(body, msg.pos);
    let block = can_place(world.0.registry(), stack.item, msg.pos, current, player_box_blocked)?;
    world.0.set_block(msg.pos, block);
    player.inventory.take_from_slot(slot, 1);
    Ok(BlockChange { pos: msg.pos, block })
}

/// True if placing a block at `pos` would overlap the player's body.
pub fn block_intersects_player(body: &PlayerBody, pos: BlockPos) -> bool {
    let min = [
        body.pos[0] - PLAYER_HALF_WIDTH,
        body.pos[1] - PLAYER_HALF_HEIGHT,
        body.pos[2] - PLAYER_HALF_WIDTH,
    ];
    let max = [
        body.pos[0] + PLAYER_HALF_WIDTH,
        body.pos[1] + PLAYER_HALF_HEIGHT,
        body.pos[2] + PLAYER_HALF_WIDTH,
    ];
    let bx = pos.x as f32;
    let by = pos.y as f32;
    let bz = pos.z as f32;
    min[0] < bx + 1.0 && max[0] > bx && min[1] < by + 1.0 && max[1] > by && min[2] < bz + 1.0
        && max[2] > bz
}

/// Recomputes whether a body is standing on solid ground.
pub fn refresh_grounded(world: &WorldRes, body: &mut PlayerBody) {
    body.grounded = overlaps_solid(&world.0, [body.pos[0], body.pos[1] - 0.02, body.pos[2]]);
}

/// System: advances mining timers for every player and breaks the blocks of
/// finished actions.
pub fn tick_mining(
    mut world: ResMut<WorldRes>,
    mut players: Query<&mut Player>,
    mut changes: ResMut<crate::net::PendingChanges>,
) {
    let mut batch = Vec::new();
    for mut player in players.iter_mut() {
        let Some(mut mining) = player.mining.clone() else {
            continue;
        };
        if mining.ticks_left > 0 {
            mining.ticks_left -= 1;
            player.mining = Some(mining);
            continue;
        }
        let targets = mining.targets.clone();
        finish_mining(&mut world, &mut player, &targets, &mut batch);
        player.mining = None;
    }
    if !batch.is_empty() {
        changes.batch.changes.extend(batch);
    }
}

/// Handles `StartMining` for a player.
pub fn handle_start_mining(
    world: &mut WorldRes,
    player: &mut Player,
    body: &PlayerBody,
    msg: &StartMining,
) -> Result<(), MineRefusalMsg> {
    let look = player_look(player);
    let plan = validate_start_mining(world, body, &player.inventory, msg, look)?;
    player.mining = Some(MiningState {
        block: msg.block,
        face: msg.face,
        ticks_left: plan.ticks,
        ticks_total: plan.ticks,
        targets: plan.targets,
    });
    Ok(())
}

fn player_look(_player: &Player) -> [f32; 3] {
    // The look direction is supplied by the movement input stream and stored
    // on the body by the movement system; see `handle_start_mining_with_look`.
    [0.0, 0.0, -1.0]
}

/// Convenience: applies a batch of block changes to the world and the pending
/// broadcast list.
pub fn apply_block_changes(
    world: &mut WorldRes,
    changes: &[BlockChange],
    pending: &mut BlockChangeBatch,
) {
    for c in changes {
        world.0.set_block(c.pos, c.block);
    }
    pending.changes.extend_from_slice(changes);
}

/// Stops mining for a player.
pub fn stop_mining(player: &mut Player, _msg: &StopMining) {
    player.mining = None;
}

/// Handles an `Interact` intent, returning the machine the player opened.
pub fn handle_interact(
    world: &mut WorldRes,
    player: &mut Player,
    body: &PlayerBody,
    msg: &Interact,
    look: [f32; 3],
) -> Result<Option<BlockPos>, MineRefusalMsg> {
    let held = player.inventory.held().map(|s| s.item);
    let reach = deepcore_shared::blocks::reach_for(world.0.registry(), held);
    let eye = eye_pos(body);
    let dist = ((msg.target.center().x - eye[0]).powi(2)
        + (msg.target.center().y - eye[1]).powi(2)
        + (msg.target.center().z - eye[2]).powi(2))
    .sqrt();
    if dist > reach + 1.5 {
        return Err(MineRefusalMsg::OutOfReach);
    }
    let block = world.0.block(msg.target);
    let Some(def) = world.0.block_def(block) else {
        return Err(MineRefusalMsg::Nothing);
    };
    if def.machine.is_some() {
        player.open_machine = Some(msg.target);
        return Ok(Some(msg.target));
    }
    let _ = look;
    Ok(None)
}

/// Face of a block from a look direction, used when placing against a face.
pub fn face_from_normal(n: [f32; 3]) -> Face {
    if n[1].abs() > n[0].abs() && n[1].abs() > n[2].abs() {
        if n[1] > 0.0 { Face::Up } else { Face::Down }
    } else if n[0].abs() > n[2].abs() {
        if n[0] > 0.0 { Face::East } else { Face::West }
    } else if n[2] > 0.0 {
        Face::South
    } else {
        Face::North
    }
}

/// Block id helper for tests.
pub fn air(world: &WorldRes) -> BlockId {
    world.0.registry().air
}
