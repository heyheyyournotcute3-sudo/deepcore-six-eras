//! Items that did not fit in an inventory are dropped into the world as
//! pickups. They are stored per block position and handed to the nearest
//! player within reach on the next tick.

use bevy::prelude::*;
use deepcore_shared::content::ItemId;
use deepcore_shared::inventory::Inventory;
use deepcore_shared::voxel::BlockPos;

use crate::sim::resources::{Player, WorldRes};

/// A pile of dropped items at a position.
#[derive(Clone, Copy, Debug)]
pub struct Pickup {
    /// Position in the world.
    pub pos: BlockPos,
    /// Item type.
    pub item: ItemId,
    /// Amount.
    pub count: u32,
    /// Ticks until it can be collected, so it is visible first.
    pub delay: u32,
}

/// All pickups currently in the world.
#[derive(Resource, Default)]
pub struct Pickups {
    /// The piles.
    pub piles: Vec<Pickup>,
}

/// Adds a pickup at a block position.
pub fn spawn_pickup(world: &WorldRes, pos: BlockPos, item: ItemId, count: u32) {
    let _ = world;
    PICKUP_QUEUE.with(|q| {
        q.borrow_mut().push(Pickup {
            pos,
            item,
            count,
            delay: 15,
        });
    });
}

thread_local! {
    static PICKUP_QUEUE: std::cell::RefCell<Vec<Pickup>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Moves queued pickups into the resource. Called at the start of each tick.
pub fn flush_pickup_queue(mut pickups: ResMut<Pickups>) {
    PICKUP_QUEUE.with(|q| {
        let mut q = q.borrow_mut();
        for p in q.drain(..) {
            pickups.piles.push(p);
        }
    });
}

/// System: hands pickups to nearby players and removes collected piles.
pub fn collect_pickups(
    world: Res<WorldRes>,
    mut pickups: ResMut<Pickups>,
    mut players: Query<(&mut Player, &deepcore_shared::movement::PlayerBody)>,
) {
    if pickups.piles.is_empty() {
        return;
    }
    let reg = world.0.registry();
    let mut remaining = Vec::new();
    for mut pile in std::mem::take(&mut pickups.piles) {
        if pile.delay > 0 {
            pile.delay -= 1;
            remaining.push(pile);
            continue;
        }
        let mut consumed = false;
        for (mut player, body) in players.iter_mut() {
            let center = BlockPos::new(
                body.pos[0].floor() as i32,
                body.pos[1].floor() as i32,
                body.pos[2].floor() as i32,
            );
            if center.distance_squared(pile.pos) <= 6.25 {
                let max = Inventory::max_stack(reg, pile.item);
                let left = player.inventory.insert(pile.item, pile.count, max);
                if left > 0 {
                    pile.count = left;
                    remaining.push(pile);
                }
                consumed = true;
                break;
            }
        }
        if !consumed {
            remaining.push(pile);
        }
    }
    pickups.piles = remaining;
}

/// Drops a whole inventory as a death crate at a position.
pub fn drop_inventory(pickups: &mut Pickups, pos: BlockPos, inv: &mut Inventory) {
    for slot in 0..deepcore_shared::inventory::INVENTORY_SLOTS {
        if let Some(stack) = inv.take_from_slot(slot, u32::MAX) {
            pickups.piles.push(Pickup {
                pos,
                item: stack.item,
                count: stack.count,
                delay: 0,
            });
        }
    }
}
