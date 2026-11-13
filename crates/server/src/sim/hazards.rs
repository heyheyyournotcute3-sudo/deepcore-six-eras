//! Hazard simulation: radiation sampling, heat, gas, oxygen and flooding.
//!
//! The server samples the world around each player once per tick and applies
//! the shared hazard maths from `deepcore_shared::hazards`.

use bevy::prelude::*;
use deepcore_shared::content::{HazardDef, ItemCategory};
use deepcore_shared::hazards::{GearModifiers, Vitals, tick_vitals};
use deepcore_shared::voxel::BlockPos;
use deepcore_shared::worldgen::{depth_below_surface, layer_def_at};

use crate::sim::resources::{Player, WorldRes};

/// Maximum number of blocks sampled around a player for radiation.
const RADIATION_SAMPLE_RADIUS: i32 = 5;

/// Gear a player currently wears, derived from their inventory.
pub fn gear_of(world: &WorldRes, player: &Player) -> GearModifiers {
    let reg = world.0.registry();
    let mut gear = GearModifiers::NONE;
    for stack in player.inventory.slots().iter().flatten() {
        let Some(def) = reg.item_def(stack.item) else {
            continue;
        };
        if def.category != ItemCategory::Gear {
            continue;
        }
        match def.id.as_str() {
            "hazmat_suit" => gear.radiation_mult = gear.radiation_mult.min(0.2),
            "cooling_suit" => gear.heat_mult = gear.heat_mult.min(0.35),
            "respirator" => {
                gear.respirator = true;
                gear.oxygen_mult = gear.oxygen_mult.min(0.5);
            }
            _ => {}
        }
    }
    gear
}

/// Radiation dose per second at a position from nearby radioactive blocks.
pub fn radiation_at(world: &mut WorldRes, pos: BlockPos) -> f32 {
    let mut dose = 0.0;
    for x in -RADIATION_SAMPLE_RADIUS..=RADIATION_SAMPLE_RADIUS {
        for y in -RADIATION_SAMPLE_RADIUS..=RADIATION_SAMPLE_RADIUS {
            for z in -RADIATION_SAMPLE_RADIUS..=RADIATION_SAMPLE_RADIUS {
                let p = pos.offset([x, y, z]);
                if !p.in_world() {
                    continue;
                }
                let block = world.0.block(p);
                let Some(def) = world.0.block_def(block) else {
                    continue;
                };
                if let Some(rad) = def.radiation {
                    let dist = ((x * x + y * y + z * z) as f32).sqrt();
                    dose += deepcore_shared::hazards::radiation_falloff(
                        rad.dose_per_sec,
                        dist,
                        rad.radius,
                    );
                }
            }
        }
    }
    dose
}

/// Layer hazards at a position.
pub fn hazards_at(world: &WorldRes, pos: BlockPos) -> (f32, f32, f32, Option<HazardDef>) {
    let layers = world.0.layers();
    let Some(layer) = layer_def_at(&layers, pos.y) else {
        return (18.0, 0.0, 0.0, None);
    };
    let mut ambient = layer.ambient_temp;
    let mut gas = 0.0f32;
    let mut radiation = 0.0f32;
    let mut notable = None;
    for h in &layer.hazards {
        match h {
            HazardDef::Heat { per_sec } => {
                ambient += per_sec * 10.0;
                notable = Some(h.clone());
            }
            HazardDef::Gas { oxygen_per_sec, .. } => {
                gas = gas.max(*oxygen_per_sec);
                notable = Some(h.clone());
            }
            HazardDef::Radiation { per_sec } => {
                radiation = radiation.max(*per_sec);
                notable = Some(h.clone());
            }
            HazardDef::Flooding { .. } | HazardDef::CaveIn { .. } => {}
        }
    }
    let _ = depth_below_surface(pos.y);
    (ambient, gas, radiation, notable)
}

/// True if the block at the player's head is water.
pub fn underwater(world: &mut WorldRes, body_pos: [f32; 3]) -> bool {
    let Some(water) = world.0.registry().block("water") else {
        return false;
    };
    let head = BlockPos::new(
        body_pos[0].floor() as i32,
        (body_pos[1] + 0.6).floor() as i32,
        body_pos[2].floor() as i32,
    );
    world.0.block(head) == water
}

/// System: advances every player's vitals by one fixed step.
pub fn tick_hazards(
    mut world: ResMut<WorldRes>,
    mut players: Query<(&mut Player, &deepcore_shared::movement::PlayerBody)>,
) {
    let dt = 1.0 / deepcore_shared::TICK_HZ as f32;
    for (mut player, body) in players.iter_mut() {
        if player.dead {
            continue;
        }
        let gear = gear_of(&world, &player);
        let pos = BlockPos::new(
            body.pos[0].floor() as i32,
            body.pos[1].floor() as i32,
            body.pos[2].floor() as i32,
        );
        let (ambient, gas, layer_rad, _) = hazards_at(&world, pos);
        let block_rad = radiation_at(&mut world, pos);
        let in_water = underwater(&mut world, body.pos);
        let oxygen_drain = if in_water { 25.0 } else { gas };
        let mut vitals: Vitals = player.vitals;
        tick_vitals(
            &mut vitals,
            gear,
            ambient,
            oxygen_drain,
            block_rad + layer_rad,
            dt,
        );
        // The respirator makes gas harmless but not water.
        if gear.respirator && !in_water {
            vitals.oxygen = (vitals.oxygen + 20.0 * dt).min(100.0);
        }
        player.vitals = vitals;
        if player.vitals.health <= 0.0 && !player.dead {
            player.dead = true;
        }
    }
}

/// Applies decontamination, used by the Decon Shower.
pub fn decontaminate(player: &mut Player, amount: f32) {
    deepcore_shared::hazards::decontaminate(&mut player.vitals, amount);
}

/// Health bar fraction for the replicated player component.
pub fn health_fraction(v: &Vitals) -> f32 {
    (v.health / 100.0).clamp(0.0, 1.0)
}

/// Detects cave-in risk: an unsupported ceiling span wider than the limit.
///
/// Returns the blocks that collapse when the block at `mined` is removed.
pub fn cave_in_blocks(world: &mut WorldRes, mined: BlockPos) -> Vec<BlockPos> {
    let layers = world.0.layers();
    let Some(layer) = layer_def_at(&layers, mined.y) else {
        return Vec::new();
    };
    let max_span = layer
        .hazards
        .iter()
        .filter_map(|h| match h {
            HazardDef::CaveIn { max_span } => Some(*max_span as i32),
            _ => None,
        })
        .max();
    let Some(max_span) = max_span else {
        return Vec::new();
    };
    let air = world.0.air();

    // Count the open horizontal run at the mined block's level, in both X and Z.
    let mut span_x = 1;
    for dir in [-1, 1] {
        let mut d = 1;
        while d <= max_span + 1 {
            let p = mined.offset([dir * d, 0, 0]);
            if world.0.block(p) == air {
                d += 1;
            } else {
                break;
            }
        }
        span_x += d - 1;
    }
    let mut span_z = 1;
    for dir in [-1, 1] {
        let mut d = 1;
        while d <= max_span + 1 {
            let p = mined.offset([0, 0, dir * d]);
            if world.0.block(p) == air {
                d += 1;
            } else {
                break;
            }
        }
        span_z += d - 1;
    }
    let span = span_x.max(span_z);
    if !deepcore_shared::hazards::cave_in(span) {
        return Vec::new();
    }
    // Collapse the ceiling above the open span.
    let mut out = Vec::new();
    for x in -(span_x / 2)..=(span_x / 2) {
        for z in -(span_z / 2)..=(span_z / 2) {
            let p = mined.offset([x, 3, z]);
            let b = world.0.block(p);
            let solid = world
                .0
                .block_def(b)
                .map(|d| !(d.passable || d.liquid))
                .unwrap_or(false);
            if solid {
                out.push(p);
            }
        }
    }
    out
}
