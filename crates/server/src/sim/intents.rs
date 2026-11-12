//! Intent handling: the single place where client messages become validated
//! server actions. Every handler drops invalid input with a log line instead of
//! panicking.

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use deepcore_shared::blocks::{apply_drops, area_blocks, plan_mine};
use deepcore_shared::content::{ItemId, RecipeId};
use deepcore_shared::inventory::Inventory;
use deepcore_shared::movement::PlayerBody;
use deepcore_shared::protocol::*;
use deepcore_shared::voxel::BlockPos;

use crate::net::{InventoryDirty, PendingChanges};
use crate::sim::resources::{Player, TechTree, WorldRes};

/// Resolves the player entity for a sender.
pub fn player_of(
    client_id: ClientId,
    clients: &Query<(Entity, &Player)>,
    by_client: &Query<&PlayerOfClient>,
) -> Option<Entity> {
    match client_id {
        ClientId::Client(c) => by_client.get(c).ok().map(|p| p.0),
        ClientId::Server => clients.iter().next().map(|(e, _)| e),
    }
}

/// Links a connected-client entity to the spawned player entity.
#[derive(Component, Clone, Copy)]
pub struct PlayerOfClient(pub Entity);

/// Handles `MoveInputMsg`: applies the shared movement step and replies with a
/// correction carrying the processed sequence number.
pub fn handle_move_input(
    mut inputs: MessageReader<FromClient<MoveInputMsg>>,
    mut players: Query<(Entity, &mut Player, &mut PlayerBody)>,
    world: Res<WorldRes>,
    mut corrections: MessageWriter<ToClients<MovementCorrection>>,
    by_client: Query<&PlayerOfClient>,
) {
    let dt = 1.0 / deepcore_shared::TICK_HZ as f32;
    for input in inputs.read() {
        let entity = match input.client_id {
            ClientId::Client(c) => by_client.get(c).ok().map(|p| p.0),
            ClientId::Server => players.iter().map(|(e, _, _)| e).next(),
        };
        let Some(entity) = entity else {
            continue;
        };
        let Ok((_, mut player, mut body)) = players.get_mut(entity) else {
            continue;
        };
        if player.dead {
            continue;
        }
        if input.message.seq <= player.last_seq {
            continue;
        }
        player.last_seq = input.message.seq;
        let mut mv = deepcore_shared::movement::MoveInput {
            seq: input.message.seq,
            dir: input.message.dir,
            jump: input.message.jump,
            sprint: input.message.sprint,
            crouch: input.message.crouch,
            yaw: input.message.yaw,
            pitch: input.message.pitch,
        };
        // Clamp inputs so a malicious client cannot fly.
        mv.dir.0 = mv.dir.0.clamp(-1.0, 1.0);
        mv.dir.1 = mv.dir.1.clamp(-1.0, 1.0);
        deepcore_shared::movement::step(&world.0, &mut body, &mv, dt);
        corrections.write(ToClients {
            targets: SendTargets::Single(input.client_id),
            message: MovementCorrection {
                seq: input.message.seq,
                pos: body.pos,
                vel: body.vel,
                grounded: body.grounded,
            },
        });
    }
}

/// Handles `StartMining`.
pub fn handle_start_mining(
    mut msgs: MessageReader<FromClient<StartMining>>,
    mut players: Query<(Entity, &mut Player, &PlayerBody)>,
    mut world: ResMut<WorldRes>,
    _changes: ResMut<PendingChanges>,
    mut refusals: MessageWriter<ToClients<MineRefusalMsg>>,
    by_client: Query<&PlayerOfClient>,
) {
    for msg in msgs.read() {
        let Some(entity) = resolve(msg.client_id, &mut players, &by_client) else {
            continue;
        };
        let Ok((_, mut player, body)) = players.get_mut(entity) else {
            continue;
        };
        if player.dead {
            continue;
        }
        let look = crate::sim::mining::look_dir(player_look_yaw(&player), player_look_pitch(&player));
        let held = player.inventory.held().map(|s| s.item);
        let reach = deepcore_shared::blocks::reach_for(world.0.registry(), held);
        let eye = crate::sim::mining::eye_pos(body);
        let Some((hit, face)) = world.0.raycast(eye, look, reach, false) else {
            continue;
        };
        if hit != msg.message.block {
            refusals.write(ToClients {
                targets: SendTargets::Single(msg.client_id),
                message: MineRefusalMsg::OutOfReach,
            });
            continue;
        }
        let block = world.0.block(hit);
        match plan_mine(world.0.registry(), block, held) {
            Ok(plan) => {
                let candidates = area_blocks(world.0.registry(), hit, face, held);
                let mut targets = Vec::new();
                for p in candidates {
                    let b = world.0.block(p);
                    if plan_mine(world.0.registry(), b, held).is_ok() {
                        targets.push(p);
                    }
                }
                player.mining = Some(crate::sim::resources::MiningState {
                    block: hit,
                    face,
                    ticks_left: plan.ticks,
                    ticks_total: plan.ticks,
                    targets,
                });
            }
            Err(deepcore_shared::blocks::MineRefusal::TooHard { required }) => {
                refusals.write(ToClients {
                    targets: SendTargets::Single(msg.client_id),
                    message: MineRefusalMsg::TooHard { required },
                });
            }
            Err(deepcore_shared::blocks::MineRefusal::Unbreakable) => {
                refusals.write(ToClients {
                    targets: SendTargets::Single(msg.client_id),
                    message: MineRefusalMsg::Unbreakable,
                });
            }
            Err(deepcore_shared::blocks::MineRefusal::Nothing) => {}
        }
    }
}

fn resolve(
    client_id: ClientId,
    players: &mut Query<(Entity, &mut Player, &PlayerBody)>,
    by_client: &Query<&PlayerOfClient>,
) -> Option<Entity> {
    match client_id {
        ClientId::Client(c) => by_client.get(c).ok().map(|p| p.0),
        ClientId::Server => players.iter().map(|(e, _, _)| e).next(),
    }
}

/// Yaw stored on the player for look-direction reconstruction.
pub fn player_look_yaw(p: &Player) -> f32 {
    p.look_yaw
}

/// Pitch stored on the player for look-direction reconstruction.
pub fn player_look_pitch(p: &Player) -> f32 {
    p.look_pitch
}

/// Handles `StopMining`.
pub fn handle_stop_mining(
    mut msgs: MessageReader<FromClient<StopMining>>,
    mut players: Query<(Entity, &mut Player)>,
    by_client: Query<&PlayerOfClient>,
) {
    for msg in msgs.read() {
        let entity = match msg.client_id {
            ClientId::Client(c) => by_client.get(c).ok().map(|p| p.0),
            ClientId::Server => players.iter().map(|(e, _)| e).next(),
        };
        if let Some(e) = entity
            && let Ok((_, mut p)) = players.get_mut(e) {
                p.mining = None;
            }
    }
}

/// Handles `PlaceBlock`.
pub fn handle_place_block(
    mut msgs: MessageReader<FromClient<PlaceBlock>>,
    mut players: Query<(Entity, &mut Player, &PlayerBody, &mut InventoryDirty)>,
    mut world: ResMut<WorldRes>,
    mut changes: ResMut<PendingChanges>,
    by_client: Query<&PlayerOfClient>,
) {
    for msg in msgs.read() {
        let entity = match msg.client_id {
            ClientId::Client(c) => by_client.get(c).ok().map(|p| p.0),
            ClientId::Server => players.iter().map(|(e, _, _, _)| e).next(),
        };
        let Some(entity) = entity else { continue };
        let Ok((_, mut player, body, mut dirty)) = players.get_mut(entity) else {
            continue;
        };
        let look = crate::sim::mining::look_dir(player.look_yaw, player.look_pitch);
        match crate::sim::mining::try_place(&mut world, &mut player, body, &msg.message, look) {
            Ok(change) => {
                changes.batch.changes.push(change);
                dirty.0 = true;
            }
            Err(reason) => {
                trace!("placement refused: {reason:?}");
            }
        }
    }
}

/// Handles `MoveItems`: transfers between the player inventory and a machine.
pub fn handle_move_items(
    mut msgs: MessageReader<FromClient<MoveItems>>,
    mut players: Query<(Entity, &mut Player, &mut InventoryDirty)>,
    world: ResMut<WorldRes>,
    mut machines: Query<&mut crate::sim::resources::Machine>,
    by_client: Query<&PlayerOfClient>,
) {
    for msg in msgs.read() {
        let entity = match msg.client_id {
            ClientId::Client(c) => by_client.get(c).ok().map(|p| p.0),
            ClientId::Server => players.iter().map(|(e, _, _)| e).next(),
        };
        let Some(entity) = entity else { continue };
        let Ok((_, mut player, mut dirty)) = players.get_mut(entity) else {
            continue;
        };
        let reg = world.0.registry();
        let from = msg.message.from;
        let to = msg.message.to;
        let count = if msg.message.count == 0 { u32::MAX } else { msg.message.count };

        if let Some(pos) = msg.message.machine {
            // Machine must be within reach.
            let in_reach = player.open_machine == Some(pos);
            if !in_reach {
                continue;
            }
            let Some(mut machine) = machines.iter_mut().find(|m| m.pos == pos) else {
                continue;
            };
            if msg.message.from_machine {
                // Machine -> player inventory.
                let slot = from.max(0) as usize;
                let stack = if slot < machine.state.outputs.len() {
                    machine.state.outputs.get_mut(slot).and_then(|s| s.take())
                } else {
                    None
                };
                if let Some(stack) = stack {
                    let max = Inventory::max_stack(reg, stack.item);
                    let left = player.inventory.insert(stack.item, stack.count, max);
                    if left > 0
                        && slot < machine.state.outputs.len() && machine.state.outputs[slot].is_none() {
                            machine.state.outputs[slot] = Some(deepcore_shared::inventory::ItemStack::new(
                                stack.item, left,
                            ));
                        }
                }
            } else {
                // Player -> machine input.
                let slot = from.max(0) as usize;
                if let Some(stack) = player.inventory.take_from_slot(slot, count) {
                    let mut placed = false;
                    for input in machine.state.inputs.iter_mut() {
                        if input.is_none() {
                            *input = Some(stack);
                            placed = true;
                            break;
                        } else if let Some(s) = input
                            && s.item == stack.item {
                                s.count += stack.count;
                                placed = true;
                                break;
                            }
                    }
                    if !placed {
                        let max = Inventory::max_stack(reg, stack.item);
                        player.inventory.insert(stack.item, stack.count, max);
                    }
                }
            }
            dirty.0 = true;
            let _ = to;
        }
    }
}

/// Handles `Craft`: hand recipes from the inventory.
pub fn handle_craft(
    mut msgs: MessageReader<FromClient<Craft>>,
    mut players: Query<(Entity, &mut Player, &mut InventoryDirty)>,
    world: Res<WorldRes>,
    tech: Res<TechTree>,
    by_client: Query<&PlayerOfClient>,
) {
    for msg in msgs.read() {
        let entity = match msg.client_id {
            ClientId::Client(c) => by_client.get(c).ok().map(|p| p.0),
            ClientId::Server => players.iter().map(|(e, _, _)| e).next(),
        };
        let Some(entity) = entity else { continue };
        let Ok((_, mut player, mut dirty)) = players.get_mut(entity) else {
            continue;
        };
        let reg = world.0.registry();
        let recipe: RecipeId = msg.message.recipe;
        if !tech.recipe_unlocked(reg, recipe) {
            continue;
        }
        let Some(def) = reg.recipe_def(recipe) else { continue };
        if !def.hand {
            continue;
        }
        let crafts = msg.message.count.clamp(1, 64);
        for _ in 0..crafts {
            let need: Vec<(ItemId, u32)> = def
                .inputs
                .iter()
                .filter_map(|(id, n)| reg.item(id).map(|i| (i, *n)))
                .collect();
            if !player.inventory.has_all(&need) {
                break;
            }
            // Room check before consuming.
            let fits = def.outputs.iter().all(|(id, n)| {
                reg.item(id).map(|i| {
                    let max = Inventory::max_stack(reg, i);
                    let free = player
                        .inventory
                        .slots()
                        .iter()
                        .filter(|s| s.is_none())
                        .count() as u32
                        * max;
                    free + max >= *n
                }).unwrap_or(false)
            });
            if !fits {
                break;
            }
            player.inventory.consume_all(&need);
            for (id, n) in &def.outputs {
                if let Some(item) = reg.item(id) {
                    let max = Inventory::max_stack(reg, item);
                    let left = player.inventory.insert(item, *n, max);
                    if left > 0 {
                        crate::sim::pickups::spawn_pickup(
                            &world,
                            BlockPos::new(0, 0, 0),
                            item,
                            left,
                        );
                    }
                }
            }
        }
        dirty.0 = true;
    }
}

/// Handles `Chat`.
pub fn handle_chat(
    mut msgs: MessageReader<FromClient<Chat>>,
    players: Query<(Entity, &Player)>,
    by_client: Query<&PlayerOfClient>,
    mut out: MessageWriter<ToClients<ChatLine>>,
) {
    for msg in msgs.read() {
        let entity = match msg.client_id {
            ClientId::Client(c) => by_client.get(c).ok().map(|p| p.0),
            ClientId::Server => players.iter().map(|(e, _)| e).next(),
        };
        let Some(entity) = entity else { continue };
        let Ok((_, player)) = players.get(entity) else { continue };
        let text: String = msg.message.text.chars().take(256).collect();
        out.write(ToClients {
            targets: SendTargets::All,
            message: ChatLine {
                from: player.name.clone(),
                text,
            },
        });
    }
}

/// Handles `Respawn`.
pub fn handle_respawn(
    mut msgs: MessageReader<FromClient<Respawn>>,
    mut players: Query<(Entity, &mut Player, &mut PlayerBody, &mut InventoryDirty)>,
    by_client: Query<&PlayerOfClient>,
) {
    for msg in msgs.read() {
        let entity = match msg.client_id {
            ClientId::Client(c) => by_client.get(c).ok().map(|p| p.0),
            ClientId::Server => players.iter().map(|(e, _, _, _)| e).next(),
        };
        let Some(entity) = entity else { continue };
        if let Ok((_, mut player, mut body, mut dirty)) = players.get_mut(entity) {
            let spawn = deepcore_shared::worldgen::spawn_pos();
            player.dead = false;
            player.vitals.health = 100.0;
            player.vitals.oxygen = 100.0;
            body.pos = [spawn.x as f32 + 0.5, spawn.y as f32 + 0.1, spawn.z as f32 + 0.5];
            body.vel = [0.0; 3];
            player.inventory = Inventory::new();
            let reg_pick = ItemId(0);
            let _ = reg_pick;
            dirty.0 = true;
        }
    }
}

/// Grants the starting kit to a fresh player.
pub fn give_starting_kit(reg: &deepcore_shared::content::ContentRegistry, inv: &mut Inventory) {
    if let Some(pick) = reg.item("steel_pickaxe") {
        inv.insert(pick, 1, 1);
    }
    if let Some(lamp) = reg.item("oil_lamp") {
        inv.insert(lamp, 1, 1);
    }
    if let Some(oil) = reg.item("oil") {
        inv.insert(oil, 4, 16);
    }
}

/// Applies drops to an inventory, spawning pickups for overflow.
pub fn grant_drops(
    world: &WorldRes,
    inv: &mut Inventory,
    drops: &[(ItemId, u32)],
    at: BlockPos,
) {
    let reg = world.0.registry();
    for (item, count) in drops {
        let max = Inventory::max_stack(reg, *item);
        let left = inv.insert(*item, *count, max);
        if left > 0 {
            crate::sim::pickups::spawn_pickup(world, at, *item, left);
        }
    }
}

/// Convenience wrapper used by tests: mine one block immediately.
pub fn mine_immediately(
    world: &mut WorldRes,
    player: &mut Player,
    pos: BlockPos,
    changes: &mut PendingChanges,
) -> bool {
    let air = world.0.air();
    let block = world.0.block(pos);
    if block == air {
        return false;
    }
    world.0.set_block(pos, air);
    apply_drops(world.0.registry(), &mut player.inventory, block);
    changes.batch.changes.push(BlockChange { pos, block: air });
    true
}
