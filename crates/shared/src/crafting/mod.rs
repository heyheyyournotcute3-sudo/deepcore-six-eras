//! Crafting and machine processing: recipe matching and the per-machine state
//! machine. All of it is pure and deterministic so the server can run it in
//! `FixedUpdate` and tests can step it directly.

use serde::{Deserialize, Serialize};

use crate::content::{ContentRegistry, ItemId, MachineDef, MachineEnergy, RecipeId};
use crate::inventory::{Inventory, ItemStack};

/// Why a machine is not running, shown in the machine UI.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum IdleReason {
    /// Running normally.
    Working,
    /// No recipe selected.
    NoRecipe,
    /// Inputs missing.
    MissingInputs,
    /// Output slots are full.
    OutputFull,
    /// No fuel in the fuel slot.
    NoFuel,
    /// The power network cannot supply the machine.
    NoPower,
    /// Blocked by a hazard (flooded machine, etc).
    Blocked,
}

impl IdleReason {
    /// Human readable text for the UI.
    pub fn text(self) -> &'static str {
        match self {
            IdleReason::Working => "Running",
            IdleReason::NoRecipe => "No recipe",
            IdleReason::MissingInputs => "Missing inputs",
            IdleReason::OutputFull => "Output full",
            IdleReason::NoFuel => "No fuel",
            IdleReason::NoPower => "No power",
            IdleReason::Blocked => "Blocked",
        }
    }
}

/// Upgrade modules installed in a machine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Upgrades {
    /// Speed modules installed (0..=2).
    pub speed: u8,
    /// Efficiency modules installed (0..=2).
    pub efficiency: u8,
}

impl Upgrades {
    /// Speed multiplier: +50% per speed module.
    pub fn speed_mult(&self) -> f32 {
        1.0 + 0.5 * self.speed as f32
    }
    /// Power multiplier: +80% per speed module, -30% per efficiency module.
    pub fn power_mult(&self) -> f32 {
        (1.0 + 0.8 * self.speed as f32) * (1.0 - 0.3 * self.efficiency as f32)
    }
}

/// State of one placed machine.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MachineState {
    /// Machine type.
    pub kind: crate::content::MachineId,
    /// Input slots.
    pub inputs: Vec<Option<ItemStack>>,
    /// Output slots.
    pub outputs: Vec<Option<ItemStack>>,
    /// Fuel slot, when the machine burns fuel.
    pub fuel: Option<ItemStack>,
    /// Currently selected recipe.
    pub recipe: Option<RecipeId>,
    /// Seconds of progress on the current recipe.
    pub progress: f32,
    /// Seconds of remaining burn time in the fuel slot.
    pub burn_remaining: f32,
    /// Last computed idle reason.
    pub idle: IdleReason,
    /// Installed upgrades.
    pub upgrades: Upgrades,
    /// Reactor control rod insertion, `0.0` fully withdrawn .. `1.0` fully in.
    pub rod_insertion: f32,
}

impl MachineState {
    /// Creates an empty machine of the given type.
    pub fn new(def: &MachineDef) -> Self {
        Self {
            kind: crate::content::MachineId(0),
            inputs: vec![None; def.input_slots as usize],
            outputs: vec![None; def.output_slots as usize],
            fuel: if matches!(def.energy, MachineEnergy::Fuel { .. }) {
                Some(ItemStack::new(ItemId(0), 0))
            } else {
                None
            },
            recipe: None,
            progress: 0.0,
            burn_remaining: 0.0,
            idle: IdleReason::NoRecipe,
            upgrades: Upgrades::default(),
            rod_insertion: 1.0,
        }
    }

    /// Creates a machine with the given id and slot counts.
    pub fn with_kind(
        kind: crate::content::MachineId,
        def: &MachineDef,
    ) -> Self {
        let mut s = Self::new(def);
        s.kind = kind;
        s
    }

    /// Total items of a type across the input slots.
    pub fn input_count(&self, item: ItemId) -> u32 {
        self.inputs.iter().flatten().filter(|s| s.item == item).map(|s| s.count).sum()
    }

    /// Free room in the output slots for a given item and amount.
    pub fn output_room(&self, reg: &ContentRegistry, item: ItemId, amount: u32) -> u32 {
        let max = Inventory::max_stack(reg, item);
        let mut room = 0;
        for slot in &self.outputs {
            match slot {
                None => room += max,
                Some(s) if s.item == item => room += max.saturating_sub(s.count),
                Some(_) => {}
            }
        }
        room.min(amount)
    }

    /// Adds items to the output slots, returning the leftover.
    pub fn push_output(&mut self, reg: &ContentRegistry, item: ItemId, amount: u32) -> u32 {
        let max = Inventory::max_stack(reg, item);
        let mut left = amount;
        for slot in self.outputs.iter_mut() {
            if left == 0 {
                break;
            }
            match slot {
                None => {
                    let m = max.min(left);
                    *slot = Some(ItemStack::new(item, m));
                    left -= m;
                }
                Some(s) if s.item == item && s.count < max => {
                    let m = (max - s.count).min(left);
                    s.count += m;
                    left -= m;
                }
                Some(_) => {}
            }
        }
        left
    }

    fn consume_inputs(&mut self, need: &[(ItemId, u32)]) {
        for (item, n) in need {
            let mut left = *n;
            for slot in self.inputs.iter_mut() {
                if left == 0 {
                    break;
                }
                if let Some(s) = slot
                    && s.item == *item {
                        let t = s.count.min(left);
                        s.count -= t;
                        left -= t;
                        if s.count == 0 {
                            *slot = None;
                        }
                    }
            }
        }
    }
}

/// Result of one machine tick, reported to the server for effects and events.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TickResult {
    /// True if a recipe completed this tick.
    pub completed: bool,
    /// True if the machine consumed fuel this tick.
    pub consumed_fuel: bool,
    /// Power actually drawn this tick (kW), after brownout scaling.
    pub power_drawn: f32,
}

/// Finds a recipe that can run given the current input contents.
pub fn match_recipe(
    reg: &ContentRegistry,
    state: &MachineState,
    unlocked: &dyn Fn(crate::content::RecipeId) -> bool,
) -> Option<RecipeId> {
    if let Some(r) = state.recipe {
        let def = reg.recipe_def(r)?;
        let ok = def
            .inputs
            .iter()
            .all(|(id, n)| reg.item(id).map(|i| state.input_count(i) >= *n).unwrap_or(false));
        if ok && unlocked(r) {
            return Some(r);
        }
    }
    reg.recipes_for_machine(state.kind).find(|r| {
        if !unlocked(*r) {
            return false;
        }
        let Some(def) = reg.recipe_def(*r) else {
            return false;
        };
        let has_inputs = def
            .inputs
            .iter()
            .all(|(id, n)| reg.item(id).map(|i| state.input_count(i) >= *n).unwrap_or(false));
        let has_room = def.outputs.iter().all(|(id, n)| {
            reg.item(id).map(|i| state.output_room(reg, i, *n) >= *n).unwrap_or(false)
        });
        has_inputs && has_room
    })
}

/// Advances a machine by `dt` seconds.
///
/// `power_ratio` is the fraction of requested power the network can supply
/// (`1.0` when the machine is unpowered or fully supplied). Fuel burning and
/// steam/electric draw are handled here; the caller owns the network.
pub fn tick_machine(
    reg: &ContentRegistry,
    state: &mut MachineState,
    dt: f32,
    power_ratio: f32,
    unlocked: &dyn Fn(crate::content::RecipeId) -> bool,
) -> TickResult {
    let mut result = TickResult::default();
    let Some(def) = reg.machine_def(state.kind) else {
        state.idle = IdleReason::Blocked;
        return result;
    };

    // --- energy -----------------------------------------------------------
    match &def.energy {
        MachineEnergy::Fuel { fuel, .. } => {
            let fuel_item = reg.item(fuel).unwrap_or(ItemId(0));
            if state.burn_remaining <= 0.0 {
                // Pull one fuel item from the fuel slot.
                if let Some(slot) = state.fuel.as_mut() {
                    if slot.item == fuel_item && slot.count > 0 {
                        slot.count -= 1;
                        state.burn_remaining = FUEL_BURN_SECONDS;
                        result.consumed_fuel = true;
                    } else if let Some(inv_fuel) = state.inputs.iter().flatten().find(|s| s.item == fuel_item) {
                        let _ = inv_fuel;
                    }
                }
                if state.burn_remaining <= 0.0 {
                    state.idle = IdleReason::NoFuel;
                    state.progress = 0.0;
                    return result;
                }
            }
            state.burn_remaining = (state.burn_remaining - dt).max(0.0);
        }
        MachineEnergy::Steam { per_sec } => {
            if power_ratio <= 0.0 {
                state.idle = IdleReason::NoPower;
                return result;
            }
            result.power_drawn = per_sec * power_ratio;
        }
        MachineEnergy::Electric { kw } => {
            if power_ratio <= 0.0 {
                state.idle = IdleReason::NoPower;
                return result;
            }
            result.power_drawn = kw * state.upgrades.power_mult() * power_ratio;
        }
        MachineEnergy::Manual => {}
    }

    // --- recipe -----------------------------------------------------------
    let Some(recipe) = match_recipe(reg, state, unlocked) else {
        state.idle = if state.recipe.is_none() {
            IdleReason::NoRecipe
        } else {
            IdleReason::MissingInputs
        };
        return result;
    };
    let Some(rdef) = reg.recipe_def(recipe) else {
        state.idle = IdleReason::NoRecipe;
        return result;
    };
    state.recipe = Some(recipe);

    // Output must have room before we spend inputs.
    for (id, n) in &rdef.outputs {
        let Some(item) = reg.item(id) else { continue };
        if state.output_room(reg, item, *n) < *n {
            state.idle = IdleReason::OutputFull;
            return result;
        }
    }

    let speed = state.upgrades.speed_mult() * power_ratio.clamp(0.05, 1.0);
    state.progress += dt * speed;
    state.idle = IdleReason::Working;

    if state.progress >= rdef.time {
        state.progress = 0.0;
        let need: Vec<(ItemId, u32)> = rdef
            .inputs
            .iter()
            .filter_map(|(id, n)| reg.item(id).map(|i| (i, *n)))
            .collect();
        state.consume_inputs(&need);
        for (id, n) in &rdef.outputs {
            if let Some(item) = reg.item(id) {
                let left = state.push_output(reg, item, *n);
                debug_assert_eq!(left, 0, "output room was checked before completing");
            }
        }
        result.completed = true;
    }
    result
}

/// Seconds one unit of coal burns in a fuel machine.
pub const FUEL_BURN_SECONDS: f32 = 80.0;

/// Break time in seconds for a block, per the formula in the design:
/// `hardness / (tool_speed * power_multiplier)`.
pub fn break_time(hardness: f32, tool_speed: f32, power_multiplier: f32) -> f32 {
    let denom = (tool_speed * power_multiplier).max(0.01);
    (hardness / denom).max(0.0)
}

/// Break time rounded up to whole server ticks.
pub fn break_ticks(hardness: f32, tool_speed: f32, power_multiplier: f32) -> u32 {
    let secs = break_time(hardness, tool_speed, power_multiplier);
    (secs * crate::TICK_HZ as f32).ceil().max(1.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn break_time_formula() {
        assert!((break_time(2.0, 1.0, 1.0) - 2.0).abs() < 1e-6);
        assert!((break_time(2.0, 2.0, 1.0) - 1.0).abs() < 1e-6);
        assert!((break_time(6.0, 3.0, 2.0) - 1.0).abs() < 1e-6);
        assert_eq!(break_ticks(1.0, 1.0, 1.0), 30);
    }

    #[test]
    fn upgrade_multipliers() {
        let u = Upgrades { speed: 2, efficiency: 0 };
        assert!((u.speed_mult() - 2.0).abs() < 1e-6);
        assert!((u.power_mult() - 2.6).abs() < 1e-6);
        let e = Upgrades { speed: 0, efficiency: 2 };
        assert!((e.power_mult() - 0.4).abs() < 1e-6);
    }
}
