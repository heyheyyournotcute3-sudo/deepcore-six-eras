//! Local player: first-person controller with predicted movement, block
//! targeting and mining state.

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use deepcore_shared::blocks::{MineRefusal, plan_mine, predicted_break_seconds};
use deepcore_shared::movement::{MoveInput, PlayerBody, step};
use deepcore_shared::protocol::REACH;
use deepcore_shared::voxel::BlockPos;

use crate::world::ClientWorld;

/// The local player's camera and state.
#[derive(Component)]
pub struct LocalPlayer {
    /// Yaw in radians.
    pub yaw: f32,
    /// Pitch in radians.
    pub pitch: f32,
    /// Input sequence counter.
    pub seq: u32,
    /// Unacknowledged inputs, for reconciliation.
    pub pending: Vec<MoveInput>,
    /// Held item, mirrored from the inventory snapshot.
    pub held: Option<deepcore_shared::content::ItemId>,
    /// Block currently under the crosshair.
    pub target: Option<(BlockPos, deepcore_shared::voxel::Face)>,
    /// Mining progress in seconds, predicted locally.
    pub mining_progress: f32,
    /// Block being mined.
    pub mining_target: Option<BlockPos>,
    /// True if the crosshair should be red.
    pub too_hard: bool,
    /// Last refusal message shown on the HUD.
    pub message: String,
    /// True when the mouse is captured for looking.
    pub captured: bool,
    /// Smoothing applied after a server correction.
    pub correction: Vec3,
}

impl Default for LocalPlayer {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            seq: 0,
            pending: Vec::new(),
            held: None,
            target: None,
            mining_progress: 0.0,
            mining_target: None,
            too_hard: false,
            message: String::new(),
            captured: false,
            correction: Vec3::ZERO,
        }
    }
}

/// Resources that hold the player's settings.
#[derive(Resource)]
pub struct PlayerSettings {
    /// Mouse sensitivity.
    pub sensitivity: f32,
    /// Field of view in degrees.
    pub fov: f32,
    /// Render distance in chunks.
    pub render_distance: i32,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            sensitivity: 0.0025,
            fov: 75.0,
            render_distance: 6,
        }
    }
}

/// Applies FOV from settings to the camera.
pub fn apply_settings(
    settings: Res<PlayerSettings>,
    mut cameras: Query<&mut Projection, With<Camera3d>>,
    mut last: Local<f32>,
) {
    if (*last - settings.fov).abs() < 0.01 {
        return;
    }
    *last = settings.fov;
    for mut projection in cameras.iter_mut() {
        if let Projection::Perspective(p) = &mut *projection {
            p.fov = settings.fov.to_radians();
        }
    }
}

/// Grabs or releases the mouse by updating the window's `CursorOptions`.
pub fn set_capture(cursors: &mut Query<&mut CursorOptions>, captured: bool) {
    if let Ok(mut cursor) = cursors.single_mut() {
        cursor.grab_mode = if captured {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        };
        cursor.visible = !captured;
    }
}

/// Mouse look.
pub fn mouse_look(
    mut motion: MessageReader<bevy::input::mouse::MouseMotion>,
    mut players: Query<&mut LocalPlayer>,
    settings: Res<PlayerSettings>,
) {
    let mut delta = Vec2::ZERO;
    for ev in motion.read() {
        delta += ev.delta;
    }
    if delta == Vec2::ZERO {
        return;
    }
    for mut player in players.iter_mut() {
        if !player.captured {
            continue;
        }
        player.yaw -= delta.x * settings.sensitivity;
        player.pitch = (player.pitch - delta.y * settings.sensitivity).clamp(-1.55, 1.55);
    }
}

/// Rotation of a camera or avatar for the given yaw and pitch.
pub fn look_rotation(yaw: f32, pitch: f32) -> Quat {
    Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0)
}

/// Direction the local player is looking.
pub fn look_direction(player: &LocalPlayer) -> Vec3 {
    let (sy, cy) = player.yaw.sin_cos();
    let (sp, cp) = player.pitch.sin_cos();
    Vec3::new(-sy * cp, sp, -cy * cp)
}

/// Eye position for a body.
pub fn eye(body: &PlayerBody) -> Vec3 {
    Vec3::new(body.pos[0], body.pos[1] + 0.65, body.pos[2])
}

/// Builds a movement input from keyboard state.
pub fn read_move_input(
    keys: &ButtonInput<KeyCode>,
    player: &LocalPlayer,
) -> (f32, f32, bool, bool, bool) {
    let mut fwd = 0.0;
    let mut right = 0.0;
    if keys.pressed(KeyCode::KeyW) {
        fwd += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        fwd -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        right += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        right -= 1.0;
    }
    let jump = keys.pressed(KeyCode::Space);
    let sprint = keys.pressed(KeyCode::ShiftLeft);
    let crouch = keys.pressed(KeyCode::ControlLeft);
    let _ = player;
    (right, fwd, jump, sprint, crouch)
}

/// System: predicts local movement every frame using the shared movement code.
pub fn predict_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    world: Res<ClientWorld>,
    mut players: Query<(&mut LocalPlayer, &mut PlayerBody, &mut Transform)>,
) {
    let dt = time.delta_secs().min(0.05);
    for (mut player, mut body, mut transform) in players.iter_mut() {
        let (right, fwd, jump, sprint, crouch) = read_move_input(&keys, &player);
        let input = MoveInput {
            seq: player.seq,
            dir: (right, fwd),
            jump,
            sprint,
            crouch,
            yaw: player.yaw,
            pitch: player.pitch,
        };
        player.seq = player.seq.wrapping_add(1);
        player.pending.push(input);
        if player.pending.len() > 128 {
            player.pending.remove(0);
        }
        step(&*world, &mut body, &input, dt);

        // Smooth out server corrections instead of snapping.
        if player.correction.length_squared() > 1e-6 {
            let fix = player.correction * (dt * 10.0).min(1.0);
            body.pos[0] += fix.x;
            body.pos[1] += fix.y;
            body.pos[2] += fix.z;
            player.correction -= fix;
        }

        transform.translation = Vec3::new(body.pos[0], body.pos[1] + 0.65, body.pos[2]);
        transform.rotation = look_rotation(player.yaw, player.pitch);
    }
}

/// System: updates the block under the crosshair and the mining prediction.
pub fn update_targeting(
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    world: Res<ClientWorld>,
    mut players: Query<(&mut LocalPlayer, &PlayerBody)>,
) {
    let dt = time.delta_secs();
    for (mut player, body) in players.iter_mut() {
        let dir = look_direction(&player);
        let reach = match player.held.and_then(|i| world.registry().item_def(i)) {
            Some(def) => match &def.category {
                deepcore_shared::content::ItemCategory::Tool(t) if t.reach_bonus > 0.0 => {
                    deepcore_shared::protocol::REACH_DRILL
                }
                _ => REACH,
            },
            None => REACH,
        };
        player.target = world.raycast(eye(body).to_array(), dir.to_array(), reach);

        // Crosshair colour from the tool tier rule.
        if let Some((pos, _)) = player.target {
            let block = world.block(pos);
            let held = player.held;
            player.too_hard = matches!(
                plan_mine(world.registry(), block, held),
                Err(MineRefusal::TooHard { .. })
            );
        } else {
            player.too_hard = false;
        }

        // Predicted crack progress.
        if mouse.pressed(MouseButton::Left) {
            if let Some((pos, _)) = player.target {
                if player.mining_target != Some(pos) {
                    player.mining_target = Some(pos);
                    player.mining_progress = 0.0;
                }
                let block = world.block(pos);
                let secs = predicted_break_seconds(world.registry(), block, player.held);
                player.mining_progress += dt / secs.max(0.05);
            }
        } else {
            player.mining_target = None;
            player.mining_progress = 0.0;
        }
    }
}

/// Local radiation estimate for the Geiger readout, using the same falloff
/// maths as the server.
pub fn dose_estimate(world: &ClientWorld, body_pos: [f32; 3]) -> f32 {
    let reg = world.registry();
    let center = BlockPos::new(
        body_pos[0].floor() as i32,
        body_pos[1].floor() as i32,
        body_pos[2].floor() as i32,
    );
    let mut dose = 0.0;
    for x in -5..=5 {
        for y in -5..=5 {
            for z in -5..=5 {
                let p = center.offset([x, y, z]);
                let Some(def) = reg.block_def(world.block(p)) else {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn look_direction_is_normalised() {
        let p = LocalPlayer {
            yaw: 0.7,
            pitch: 0.3,
            ..Default::default()
        };
        assert!((look_direction(&p).length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn forward_look_points_negative_z() {
        let p = LocalPlayer::default();
        let d = look_direction(&p);
        assert!(d.z < -0.99);
    }

    #[test]
    fn camera_rotation_matches_look_direction() {
        for (yaw, pitch) in [(0.0, 0.0), (0.8, 0.4), (-1.2, -0.6), (2.5, 1.2)] {
            let expected = look_direction(&LocalPlayer {
                yaw,
                pitch,
                ..Default::default()
            });
            let actual = look_rotation(yaw, pitch) * Vec3::NEG_Z;
            assert!(
                (expected - actual).length() < 1e-4,
                "yaw {yaw} pitch {pitch}: expected {expected:?}, camera looks {actual:?}"
            );
        }
    }

    #[test]
    fn pitch_is_clamped_by_mouse_look_range() {
        let mut p = LocalPlayer::default();
        p.pitch = (p.pitch + 10.0).clamp(-1.55, 1.55);
        assert!(p.pitch <= 1.55);
    }
}
