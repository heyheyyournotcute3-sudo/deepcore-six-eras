//! Power and steam networks.
//!
//! Networks are recomputed only when a node is added or removed; each tick we
//! just sum numbers. Two flavours share the same solver:
//!
//! * **Steam**: boilers push steam units per second into a pipe network, and
//!   consumers draw from it. A boiler throttles to match demand.
//! * **Electric**: generators push kW into a cable network. If demand exceeds
//!   supply every consumer runs at `supply / demand`. Battery banks absorb the
//!   surplus and cover the deficit.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Kind of network.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum NetworkKind {
    /// Steam pipes.
    Steam,
    /// Electric cables.
    Electric,
}

/// Node identifier, stable for the lifetime of a server session.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId(pub u32);

/// A power producer.
#[derive(Clone, Copy, Debug)]
pub struct Producer {
    /// Node.
    pub node: NodeId,
    /// Output in kW (electric) or units/s (steam).
    pub output: f32,
}

/// A power consumer.
#[derive(Clone, Copy, Debug)]
pub struct Consumer {
    /// Node.
    pub node: NodeId,
    /// Demand in kW (electric) or units/s (steam).
    pub demand: f32,
}

/// A connected network of pipes or cables.
#[derive(Clone, Debug, Default)]
pub struct Network {
    /// Nodes in the network.
    pub nodes: Vec<NodeId>,
    /// Producers.
    pub producers: Vec<Producer>,
    /// Consumers.
    pub consumers: Vec<Consumer>,
    /// Battery storage in kJ (electric only).
    pub stored_kj: f32,
    /// Battery capacity in kJ.
    pub capacity_kj: f32,
}

/// Per-tick numbers for one network, used by the overlay and the machines.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NetworkStats {
    /// Total supply.
    pub supply: f32,
    /// Total demand.
    pub demand: f32,
    /// Fraction of demand that can be met, `0..=1`.
    pub satisfaction: f32,
    /// Energy charged into batteries this tick (kW).
    pub charging: f32,
    /// Energy drawn from batteries this tick (kW).
    pub discharging: f32,
    /// Stored energy after the tick (kJ).
    pub stored_kj: f32,
}

/// The full set of networks on a server.
#[derive(Clone, Debug, Default)]
pub struct PowerGrid {
    /// Every network by index.
    pub networks: Vec<Network>,
    /// Which network a node belongs to.
    pub membership: HashMap<NodeId, usize>,
}

impl PowerGrid {
    /// Creates an empty grid.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a node into the network with the given index.
    pub fn add_node(&mut self, node: NodeId, network: usize) {
        if self.networks.len() <= network {
            self.networks.resize_with(network + 1, Network::default);
        }
        self.networks[network].nodes.push(node);
        self.membership.insert(node, network);
    }

    /// Adds a producer to the network containing `node`.
    pub fn add_producer(&mut self, node: NodeId, output: f32) {
        if let Some(&net) = self.membership.get(&node) {
            self.networks[net].producers.push(Producer { node, output });
        }
    }

    /// Adds a consumer to the network containing `node`.
    pub fn add_consumer(&mut self, node: NodeId, demand: f32) {
        if let Some(&net) = self.membership.get(&node) {
            self.networks[net].consumers.push(Consumer { node, demand });
        }
    }

    /// Advances every network by `dt` seconds and returns per-network stats.
    pub fn tick(&mut self, dt: f32) -> Vec<NetworkStats> {
        self.networks.iter_mut().map(|n| solve_network(n, dt)).collect()
    }

    /// Satisfaction of the network containing `node`.
    pub fn satisfaction_of(&self, stats: &[NetworkStats], node: NodeId) -> f32 {
        self.membership
            .get(&node)
            .and_then(|&i| stats.get(i))
            .map(|s| s.satisfaction)
            .unwrap_or(1.0)
    }
}

/// Solves a single network for one tick.
pub fn solve_network(net: &mut Network, dt: f32) -> NetworkStats {
    let supply: f32 = net.producers.iter().map(|p| p.output.max(0.0)).sum();
    let demand: f32 = net.consumers.iter().map(|c| c.demand.max(0.0)).sum();
    let mut stats = NetworkStats {
        supply,
        demand,
        ..Default::default()
    };

    if demand <= f32::EPSILON {
        // No load: charge batteries with whatever is produced.
        if supply > 0.0 && net.capacity_kj > 0.0 {
            let room = net.capacity_kj - net.stored_kj;
            let charge = (supply * dt).min(room.max(0.0));
            net.stored_kj += charge;
            stats.charging = charge / dt.max(1e-6);
        }
        stats.satisfaction = 1.0;
        stats.stored_kj = net.stored_kj;
        return stats;
    }

    if supply >= demand {
        stats.satisfaction = 1.0;
        let surplus = supply - demand;
        if net.capacity_kj > 0.0 {
            let room = net.capacity_kj - net.stored_kj;
            let charge = (surplus * dt).min(room.max(0.0));
            net.stored_kj += charge;
            stats.charging = charge / dt.max(1e-6);
        }
    } else {
        let deficit = demand - supply;
        // Batteries cover the deficit up to what they hold.
        let available_kw = if dt > 0.0 { net.stored_kj / dt } else { 0.0 };
        let from_battery = deficit.min(available_kw).min(net.stored_kj / dt.max(1e-6));
        let drained = (from_battery * dt).min(net.stored_kj);
        net.stored_kj -= drained;
        stats.discharging = drained / dt.max(1e-6);
        let effective = supply + stats.discharging;
        stats.satisfaction = (effective / demand).clamp(0.0, 1.0);
    }
    stats.stored_kj = net.stored_kj;
    stats
}

/// Adds battery capacity to a network.
pub fn add_battery(net: &mut Network, capacity_kj: f32, stored_kj: f32) {
    net.capacity_kj += capacity_kj;
    net.stored_kj = (net.stored_kj + stored_kj).min(net.capacity_kj);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn net(supply: f32, demand: f32) -> Network {
        Network {
            producers: vec![Producer { node: NodeId(0), output: supply }],
            consumers: vec![Consumer { node: NodeId(1), demand }],
            ..Default::default()
        }
    }

    #[test]
    fn full_supply_satisfies_demand() {
        let mut n = net(100.0, 60.0);
        let s = solve_network(&mut n, 1.0 / 30.0);
        assert!((s.satisfaction - 1.0).abs() < 1e-6);
    }

    #[test]
    fn brownout_scales_by_ratio() {
        let mut n = net(50.0, 100.0);
        let s = solve_network(&mut n, 1.0 / 30.0);
        assert!((s.satisfaction - 0.5).abs() < 1e-5);
    }

    #[test]
    fn battery_covers_deficit() {
        let mut n = net(50.0, 100.0);
        n.capacity_kj = 1000.0;
        n.stored_kj = 1000.0;
        let s = solve_network(&mut n, 1.0 / 30.0);
        assert!((s.satisfaction - 1.0).abs() < 1e-3);
        assert!(n.stored_kj < 1000.0);
    }

    #[test]
    fn battery_charges_on_surplus() {
        let mut n = net(100.0, 40.0);
        n.capacity_kj = 1000.0;
        n.stored_kj = 0.0;
        let s = solve_network(&mut n, 1.0);
        assert!((s.charging - 60.0).abs() < 1e-3);
        assert!((n.stored_kj - 60.0).abs() < 1e-3);
    }

    #[test]
    fn empty_network_is_satisfied() {
        let mut n = Network::default();
        let s = solve_network(&mut n, 0.1);
        assert_eq!(s.satisfaction, 1.0);
    }
}
