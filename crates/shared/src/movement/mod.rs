//! Deterministic player movement: AABB versus voxel collision.
//!
//! The same code runs on the client (prediction) and on the server
//! (authority), so it must be pure: no randomness, no float time accumulation
//! beyond the fixed step passed in.

use serde::{Deserialize, Serialize};

use crate::voxel::{BlockPos, world_to_block};

/// Player collision box half extents (width/depth 0.6, height 1.8).
pub const PLAYER_HALF_WIDTH: f32 = 0.3;
/// Player half height.
pub const PLAYER_HALF_HEIGHT: f32 = 0.9;
/// Eye height above the player centre.
pub const EYE_OFFSET: f32 = 0.65;

/// Movement constants. Kept here so client and server cannot diverge.
pub mod consts {
    /// Gravity in blocks per second squared.
    pub const GRAVITY: f32 = 26.0;
    /// Walk speed in blocks per second.
    pub const WALK_SPEED: f32 = 4.6;
    /// Sprint speed.
    pub const SPRINT_SPEED: f32 = 7.4;
    /// Crouch speed.
    pub const CROUCH_SPEED: f32 = 2.0;
    /// Jump impulse.
    pub const JUMP_SPEED: f32 = 8.6;
    /// Terminal fall speed.
    pub const TERMINAL_VELOCITY: f32 = 60.0;
    /// Step height for walking up single blocks.
    pub const STEP_HEIGHT: f32 = 0.6;
    /// Stamina drained per second while sprinting.
    pub const SPRINT_STAMINA: f32 = 12.0;
    /// Stamina regenerated per second while resting.
    pub const STAMINA_REGEN: f32 = 18.0;
}

/// One movement input from the client.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MoveInput {
    /// Sequence number, monotonically increasing per client.
    pub seq: u32,
    /// Local-space move direction, `x` right, `z` forward.
    pub dir: (f32, f32),
    /// Jump requested this tick.
    pub jump: bool,
    /// Sprint requested.
    pub sprint: bool,
    /// Crouch requested.
    pub crouch: bool,
    /// Look yaw in radians.
    pub yaw: f32,
    /// Look pitch in radians.
    pub pitch: f32,
}

/// Physical state of a player.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, bevy::prelude::Component)]
pub struct PlayerBody {
    /// Position of the player centre.
    pub pos: [f32; 3],
    /// Velocity in blocks per second.
    pub vel: [f32; 3],
    /// True if standing on ground last step.
    pub grounded: bool,
    /// Remaining stamina.
    pub stamina: f32,
}

impl Default for PlayerBody {
    fn default() -> Self {
        Self {
            pos: [0.0, 0.0, 0.0],
            vel: [0.0, 0.0, 0.0],
            grounded: false,
            stamina: 100.0,
        }
    }
}

/// Abstraction over the voxel world used by the collision code.
pub trait VoxelView {
    /// True if the block at this position blocks movement.
    fn solid(&self, pos: BlockPos) -> bool;
}

/// Returns true if the player box at `pos` overlaps any solid block.
pub fn overlaps_solid(world: &impl VoxelView, pos: [f32; 3]) -> bool {
    let min = [
        pos[0] - PLAYER_HALF_WIDTH,
        pos[1] - PLAYER_HALF_HEIGHT,
        pos[2] - PLAYER_HALF_WIDTH,
    ];
    let max = [
        pos[0] + PLAYER_HALF_WIDTH,
        pos[1] + PLAYER_HALF_HEIGHT,
        pos[2] + PLAYER_HALF_WIDTH,
    ];
    let lo = world_to_block(bevy::math::Vec3::new(min[0], min[1], min[2]));
    let hi = world_to_block(bevy::math::Vec3::new(max[0] - 1e-4, max[1] - 1e-4, max[2] - 1e-4));
    for x in lo.x..=hi.x {
        for y in lo.y..=hi.y {
            for z in lo.z..=hi.z {
                if world.solid(BlockPos::new(x, y, z)) {
                    return true;
                }
            }
        }
    }
    false
}

/// Advances the player one fixed step.
pub fn step(
    world: &impl VoxelView,
    body: &mut PlayerBody,
    input: &MoveInput,
    dt: f32,
) -> PlayerBody {
    let before = *body;

    // --- horizontal wish velocity -----------------------------------------
    let mut wish = [input.dir.0, input.dir.1];
    let len = (wish[0] * wish[0] + wish[1] * wish[1]).sqrt();
    if len > 1.0 {
        wish[0] /= len;
        wish[1] /= len;
    }
    let wants_sprint = input.sprint && len > 0.1 && body.stamina > 1.0 && !input.crouch;
    let speed = if input.crouch {
        consts::CROUCH_SPEED
    } else if wants_sprint {
        consts::SPRINT_SPEED
    } else {
        consts::WALK_SPEED
    };
    if wants_sprint {
        body.stamina = (body.stamina - consts::SPRINT_STAMINA * dt).max(0.0);
    } else {
        body.stamina = (body.stamina + consts::STAMINA_REGEN * dt).min(100.0);
    }

    let (sin, cos) = input.yaw.sin_cos();
    // Forward is -Z in Bevy's right-handed world with +Y up.
    let fwd = [-sin, -cos];
    let right = [cos, -sin];
    let wish_world = [
        right[0] * wish[0] + fwd[0] * wish[1],
        right[1] * wish[0] + fwd[1] * wish[1],
    ];

    // Ground friction / air control.
    let accel = if body.grounded { 60.0 } else { 18.0 };
    body.vel[0] += (wish_world[0] * speed - body.vel[0]) * (accel * dt).min(1.0);
    body.vel[2] += (wish_world[1] * speed - body.vel[2]) * (accel * dt).min(1.0);

    // --- gravity and jump ---------------------------------------------------
    if input.jump && body.grounded {
        body.vel[1] = consts::JUMP_SPEED;
        body.grounded = false;
    }
    body.vel[1] = (body.vel[1] - consts::GRAVITY * dt).clamp(-consts::TERMINAL_VELOCITY, 60.0);

    // --- integrate with axis-separated collision ---------------------------
    body.grounded = false;
    move_axis(world, body, 1, body.vel[1] * dt, true);
    move_axis(world, body, 0, body.vel[0] * dt, false);
    move_axis(world, body, 2, body.vel[2] * dt, false);

    // Step-up assist: if we are blocked horizontally but a small step is free.
    if before.grounded && body.vel[0].abs() + body.vel[2].abs() > 0.01 {
        let blocked = (body.pos[0] - before.pos[0]).abs() + (body.pos[2] - before.pos[2]).abs() < 1e-3;
        if blocked {
            let mut probe = before;
            probe.pos[1] += consts::STEP_HEIGHT;
            if !overlaps_solid(world, probe.pos) {
                move_axis(world, &mut probe, 0, body.vel[0] * dt, false);
                move_axis(world, &mut probe, 2, body.vel[2] * dt, false);
                if !overlaps_solid(world, probe.pos) {
                    body.pos[0] = probe.pos[0];
                    body.pos[2] = probe.pos[2];
                }
            }
        }
    }
    *body
}

fn move_axis(world: &impl VoxelView, body: &mut PlayerBody, axis: usize, delta: f32, vertical: bool) {
    if delta == 0.0 {
        return;
    }
    let old = body.pos[axis];
    body.pos[axis] += delta;
    if overlaps_solid(world, body.pos) {
        body.pos[axis] = old;
        if vertical {
            if delta < 0.0 {
                body.grounded = true;
            }
            body.vel[1] = 0.0;
        } else {
            body.vel[axis] = 0.0;
        }
    }
}

/// Recomputes `grounded` without moving, used after a teleport.
pub fn refresh_grounded(world: &impl VoxelView, body: &mut PlayerBody) {
    let mut probe = *body;
    probe.pos[1] -= 0.02;
    body.grounded = overlaps_solid(world, probe.pos);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FlatGround {
        floor_y: i32,
    }

    impl VoxelView for FlatGround {
        fn solid(&self, pos: BlockPos) -> bool {
            pos.y <= self.floor_y
        }
    }

    fn input(dir: (f32, f32), jump: bool, sprint: bool) -> MoveInput {
        MoveInput { dir, jump, sprint, ..Default::default() }
    }

    #[test]
    fn falls_until_grounded() {
        let world = FlatGround { floor_y: 0 };
        let mut body = PlayerBody { pos: [0.5, 5.0, 0.5], ..Default::default() };
        for _ in 0..200 {
            step(&world, &mut body, &input((0.0, 0.0), false, false), 1.0 / 30.0);
        }
        assert!(body.grounded);
        assert!((body.pos[1] - (1.0 + PLAYER_HALF_HEIGHT)).abs() < 0.05, "y={}", body.pos[1]);
    }

    #[test]
    fn jump_leaves_ground_and_returns() {
        let world = FlatGround { floor_y: 0 };
        let mut body = PlayerBody { pos: [0.5, 1.9, 0.5], ..Default::default() };
        refresh_grounded(&world, &mut body);
        assert!(body.grounded);
        step(&world, &mut body, &input((0.0, 0.0), true, false), 1.0 / 30.0);
        assert!(body.pos[1] > 1.9);
        for _ in 0..120 {
            step(&world, &mut body, &input((0.0, 0.0), false, false), 1.0 / 30.0);
        }
        assert!(body.grounded);
    }

    #[test]
    fn sprint_is_faster_than_walk() {
        let world = FlatGround { floor_y: 0 };
        let mut walk = PlayerBody { pos: [0.5, 1.9, 0.5], ..Default::default() };
        let mut run = walk;
        refresh_grounded(&world, &mut walk);
        refresh_grounded(&world, &mut run);
        for _ in 0..60 {
            step(&world, &mut walk, &input((0.0, 1.0), false, false), 1.0 / 30.0);
            step(&world, &mut run, &input((0.0, 1.0), false, true), 1.0 / 30.0);
        }
        let walked = (walk.pos[2] - 0.5).abs();
        let ran = (run.pos[2] - 0.5).abs();
        assert!(ran > walked, "ran={ran} walked={walked}");
    }

    #[test]
    fn wall_stops_horizontal_movement() {
        struct Wall;
        impl VoxelView for Wall {
            fn solid(&self, pos: BlockPos) -> bool {
                pos.y <= 0 || pos.x >= 2
            }
        }
        let world = Wall;
        let mut body = PlayerBody { pos: [0.5, 1.9, 0.5], ..Default::default() };
        refresh_grounded(&world, &mut body);
        for _ in 0..120 {
            step(&world, &mut body, &input((1.0, 0.0), false, false), 1.0 / 30.0);
        }
        assert!(body.pos[0] < 2.0, "x={}", body.pos[0]);
    }

    #[test]
    fn movement_is_deterministic() {
        let world = FlatGround { floor_y: 0 };
        let mut a = PlayerBody { pos: [0.5, 3.0, 0.5], ..Default::default() };
        let mut b = a;
        for i in 0..100 {
            let inp = input((0.3, 0.7), i % 17 == 0, i % 5 == 0);
            step(&world, &mut a, &inp, 1.0 / 30.0);
            step(&world, &mut b, &inp, 1.0 / 30.0);
        }
        assert_eq!(a.pos, b.pos);
    }
}
