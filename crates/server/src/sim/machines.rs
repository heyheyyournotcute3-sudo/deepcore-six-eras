//! Machine simulation: ticking every placed machine, the power and steam
//! grids, and the reactor.

use bevy::prelude::*;
use deepcore_shared::content::{MachineEnergy, RecipeId};
use deepcore_shared::crafting::{MachineState, tick_machine};
use deepcore_shared::power::{Network, NetworkStats};

use crate::sim::resources::{Grid, Machine, Player, TechTree, WorldRes};

/// System: advances every machine by one fixed step.
pub fn tick_machines(
    world: Res<WorldRes>,
    tech: Res<TechTree>,
    mut grid: ResMut<Grid>,
    mut machines: Query<&mut Machine>,
    _players: Query<&Player>,
) {
    let reg = world.0.registry();
    let dt = 1.0 / deepcore_shared::TICK_HZ as f32;

    // 1. Refresh network totals from the current machine set.
    let mut stats = grid.grid.tick(dt);

    // 2. Ask the grid how much of its demand each machine can actually draw.
    let mut ratios: Vec<f32> = Vec::new();
    for machine in machines.iter() {
        let ratio = grid
            .grid
            .membership
            .get(&machine.node)
            .and_then(|&i| stats.get(i))
            .map(|s| s.satisfaction)
            .unwrap_or(1.0);
        ratios.push(ratio);
    }
    for (i, mut machine) in machines.iter_mut().enumerate() {
        let ratio = ratios.get(i).copied().unwrap_or(1.0);
        let unlocked = |r: RecipeId| tech.recipe_unlocked(reg, r);
        let def = reg.machine_def(machine.kind);
        let mut state = machine.state.clone();
        let result = tick_machine(reg, &mut state, dt, ratio, &unlocked);
        machine.state = state;

        // Reactor temperature model.
        if let Some(rdef) = def.and_then(|d| d.reactor) {
            let withdrawn = 1.0 - machine.state.rod_insertion.clamp(0.0, 1.0);
            let heat = withdrawn * rdef.heat_per_rod;
            let cooling = rdef.cooling_rate * ratio;
            machine.temperature += (heat - cooling) * dt;
            machine.temperature = machine.temperature.max(20.0);
        }

        if result.completed || result.consumed_fuel {
            // Producers publish their output into the grid.
            if def.map(|d| d.producer).unwrap_or(false) {
                publish_producer_output(&mut grid, machine.node, reg, machine.kind);
            }
        }
    }

    // 3. Re-solve so the overlay numbers match what was actually consumed.
    stats = grid.grid.tick(0.0);
    grid.stats = stats;
}

fn publish_producer_output(
    grid: &mut Grid,
    node: deepcore_shared::power::NodeId,
    reg: &deepcore_shared::content::ContentRegistry,
    kind: deepcore_shared::content::MachineId,
) {
    let Some(def) = reg.machine_def(kind) else {
        return;
    };
    let output = match &def.energy {
        MachineEnergy::Fuel { .. } => 60.0,
        MachineEnergy::Steam { per_sec } => per_sec * 25.0,
        _ => def.reactor.map(|r| r.output_kw).unwrap_or(0.0),
    };
    if let Some(&net) = grid.grid.membership.get(&node)
        && let Some(n) = grid.grid.networks.get_mut(net)
    {
        if let Some(p) = n.producers.iter_mut().find(|p| p.node == node) {
            p.output = output;
        } else {
            n.producers
                .push(deepcore_shared::power::Producer { node, output });
        }
    }
}

/// Creates a fresh machine state for a kind.
pub fn new_machine_state(
    kind: deepcore_shared::content::MachineId,
    def: &deepcore_shared::content::MachineDef,
) -> MachineState {
    MachineState::with_kind(kind, def)
}

/// Registers a machine on the grid: a new single-machine network, or the
/// network of an adjacent machine.
pub fn attach_to_grid(
    grid: &mut Grid,
    node: deepcore_shared::power::NodeId,
    neighbour: Option<deepcore_shared::power::NodeId>,
    demand: f32,
) {
    let net_index = neighbour
        .and_then(|n| grid.grid.membership.get(&n).copied())
        .unwrap_or_else(|| {
            grid.grid.networks.push(Network::default());
            grid.grid.networks.len() - 1
        });
    grid.grid.add_node(node, net_index);
    if demand > 0.0 {
        grid.grid.add_consumer(node, demand);
    }
}

/// Runs one network solve and returns the stats (used by tests and the
/// overlay).
pub fn solve(grid: &mut Grid, dt: f32) -> Vec<NetworkStats> {
    let stats = grid.grid.tick(dt);
    grid.stats = stats.clone();
    stats
}
