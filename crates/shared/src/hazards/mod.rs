//! Hazards: body heat, oxygen, radiation dose, gas and flooding helpers.
//!
//! Pure functions only. The server samples the world and feeds these.

use serde::{Deserialize, Serialize};

/// Player vitals tracked by the server.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vitals {
    /// Health, 0..=100.
    pub health: f32,
    /// Stamina, 0..=100.
    pub stamina: f32,
    /// Oxygen, 0..=100.
    pub oxygen: f32,
    /// Body heat, degrees Celsius.
    pub heat: f32,
    /// Radiation dose, `0..` where 100 starts screen grain.
    pub dose: f32,
}

impl Default for Vitals {
    fn default() -> Self {
        Self {
            health: 100.0,
            stamina: 100.0,
            oxygen: 100.0,
            heat: 36.6,
            dose: 0.0,
        }
    }
}

/// Thresholds for the radiation dose ladder.
pub const DOSE_GRAIN: f32 = 100.0;
/// Above this dose health regeneration stops.
pub const DOSE_NO_REGEN: f32 = 300.0;
/// Above this dose the player takes damage.
pub const DOSE_DAMAGE: f32 = 600.0;
/// Dose per second of damage above [`DOSE_DAMAGE`], scaled by excess dose.
pub const DOSE_DAMAGE_RATE: f32 = 0.02;

/// Thresholds for body heat.
pub const HEAT_COMFORT: f32 = 39.0;
/// Above this body heat the player loses health.
pub const HEAT_LIMIT: f32 = 42.0;
/// Health lost per second at twice the limit.
pub const HEAT_DAMAGE_RATE: f32 = 4.0;

/// Gear modifiers applied to hazard maths.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GearModifiers {
    /// Radiation dose multiplier, e.g. `0.2` for a hazmat suit.
    pub radiation_mult: f32,
    /// Heat gain multiplier.
    pub heat_mult: f32,
    /// Oxygen drain multiplier.
    pub oxygen_mult: f32,
    /// True if the player has a respirator (immune to gas damage).
    pub respirator: bool,
}

impl GearModifiers {
    /// No gear.
    pub const NONE: Self = Self {
        radiation_mult: 1.0,
        heat_mult: 1.0,
        oxygen_mult: 1.0,
        respirator: false,
    };
}

/// Radiation dose gained per second from a source of `dose_per_sec` at
/// `distance` blocks, falling off linearly to `radius`.
pub fn radiation_falloff(dose_per_sec: f32, distance: f32, radius: f32) -> f32 {
    if radius <= 0.0 || distance >= radius {
        return 0.0;
    }
    let t = 1.0 - (distance / radius).clamp(0.0, 1.0);
    dose_per_sec * t
}

/// Health change per second from the current dose.
pub fn radiation_health_delta(dose: f32) -> f32 {
    if dose > DOSE_DAMAGE {
        -(dose - DOSE_DAMAGE) * DOSE_DAMAGE_RATE
    } else {
        0.0
    }
}

/// True if the dose blocks natural regeneration.
pub fn dose_blocks_regen(dose: f32) -> bool {
    dose >= DOSE_NO_REGEN
}

/// Health change per second from body heat.
pub fn heat_health_delta(heat: f32) -> f32 {
    if heat > HEAT_LIMIT {
        -(heat - HEAT_LIMIT) * HEAT_DAMAGE_RATE
    } else {
        0.0
    }
}

/// Advances vitals by `dt` seconds given the environment.
pub fn tick_vitals(
    v: &mut Vitals,
    gear: GearModifiers,
    ambient_heat: f32,
    oxygen_drain_per_sec: f32,
    dose_per_sec: f32,
    dt: f32,
) {
    // Heat drifts toward ambient, scaled by gear.
    let target = 36.6 + (ambient_heat - 18.0).max(0.0) * 0.06 * gear.heat_mult;
    let rate = if target > v.heat { 0.02 } else { 0.05 };
    v.heat += (target - v.heat) * rate * dt * 10.0;
    v.heat = v.heat.clamp(30.0, 60.0);

    v.oxygen = (v.oxygen - oxygen_drain_per_sec * gear.oxygen_mult * dt).clamp(0.0, 100.0);
    if v.oxygen <= 0.0 {
        v.health -= 8.0 * dt;
    }

    v.dose = (v.dose + dose_per_sec * gear.radiation_mult * dt).max(0.0);
    v.health += radiation_health_delta(v.dose) * dt;
    v.health += heat_health_delta(v.heat) * dt;

    if v.oxygen > 20.0 && !dose_blocks_regen(v.dose) && v.health < 100.0 && v.health > 0.0 {
        v.health = (v.health + 0.5 * dt).min(100.0);
    }
    v.health = v.health.clamp(0.0, 100.0);
}

/// Removes dose, as a Decon Shower does.
pub fn decontaminate(v: &mut Vitals, amount: f32) {
    v.dose = (v.dose - amount).max(0.0);
}

/// Maximum span of unsupported ceiling before a cave-in, in blocks.
pub const MAX_UNSUPPORTED_SPAN: i32 = 7;

/// True if a ceiling span of the given width will collapse.
pub fn cave_in(span: i32) -> bool {
    span > MAX_UNSUPPORTED_SPAN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radiation_falls_off_linearly() {
        assert!((radiation_falloff(1.0, 0.0, 4.0) - 1.0).abs() < 1e-6);
        assert!((radiation_falloff(1.0, 2.0, 4.0) - 0.5).abs() < 1e-6);
        assert_eq!(radiation_falloff(1.0, 4.0, 4.0), 0.0);
    }

    #[test]
    fn dose_damage_thresholds() {
        assert_eq!(radiation_health_delta(100.0), 0.0);
        assert_eq!(radiation_health_delta(300.0), 0.0);
        assert!(radiation_health_delta(700.0) < 0.0);
        assert!(dose_blocks_regen(300.0));
        assert!(!dose_blocks_regen(299.0));
    }

    #[test]
    fn heat_damage_only_above_limit() {
        assert_eq!(heat_health_delta(41.0), 0.0);
        assert!(heat_health_delta(45.0) < 0.0);
    }

    #[test]
    fn hazmat_suit_reduces_dose() {
        let mut a = Vitals::default();
        let mut b = Vitals::default();
        tick_vitals(&mut a, GearModifiers::NONE, 18.0, 0.0, 10.0, 1.0);
        tick_vitals(
            &mut b,
            GearModifiers { radiation_mult: 0.2, ..GearModifiers::NONE },
            18.0,
            0.0,
            10.0,
            1.0,
        );
        assert!((a.dose - 10.0).abs() < 1e-4);
        assert!((b.dose - 2.0).abs() < 1e-4);
    }

    #[test]
    fn oxygen_runs_out_and_damages() {
        let mut v = Vitals::default();
        v.oxygen = 0.5;
        tick_vitals(&mut v, GearModifiers::NONE, 18.0, 5.0, 0.0, 1.0);
        assert!(v.health < 100.0);
    }

    #[test]
    fn cave_in_span_rule() {
        assert!(!cave_in(7));
        assert!(cave_in(8));
    }

    #[test]
    fn decon_shower_clears_dose() {
        let mut v = Vitals::default();
        v.dose = 200.0;
        decontaminate(&mut v, 500.0);
        assert_eq!(v.dose, 0.0);
    }
}
