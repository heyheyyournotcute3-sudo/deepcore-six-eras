//! Single-player / host mode: runs the authoritative server inside the client
//! process on the same ECS, so "Host game" needs no second binary and the
//! client's local player is simply another player entity.

use bevy::prelude::*;
use deepcore_shared::movement::PlayerBody;

use crate::hud::HudInventory;
use crate::player::LocalPlayer;

/// Marks the local player entity when hosting locally.
#[derive(Component)]
pub struct LocalHostPlayer;

/// When hosting, streams chunks around the player into the client world up to
/// the configured render distance.
pub fn stream_local_chunks(
    settings: Res<crate::player::PlayerSettings>,
    bodies: Query<&PlayerBody, With<LocalPlayer>>,
    mut server_world: ResMut<deepcore_server::sim::resources::WorldRes>,
    mut client_world: ResMut<crate::world::ClientWorld>,
) {
    let Ok(body) = bodies.single() else { return };
    let center = deepcore_shared::voxel::BlockPos::new(
        body.pos[0].floor() as i32,
        body.pos[1].floor() as i32,
        body.pos[2].floor() as i32,
    )
    .chunk();
    client_world.ensure_around(
        deepcore_shared::voxel::BlockPos::new(
            body.pos[0].floor() as i32,
            body.pos[1].floor() as i32,
            body.pos[2].floor() as i32,
        ),
        2,
    );
    let r = settings.render_distance.clamp(1, 12);
    for dx in -r..=r {
        for dz in -r..=r {
            for dy in -2..=2 {
                let pos = center.offset([dx, dy, dz]);
                if pos.y < 0 || pos.y * deepcore_shared::CHUNK_SIZE >= deepcore_shared::WORLD_HEIGHT
                {
                    continue;
                }
                if !client_world.has_chunk(pos) {
                    let chunk = server_world.0.clone_chunk(pos);
                    if let Some(chunk) = chunk {
                        client_world.insert_chunk(chunk);
                    }
                }
            }
        }
    }
}

/// Copies the host's authoritative inventory into the HUD snapshot.
pub fn mirror_host_inventory(
    players: Query<&deepcore_server::sim::resources::Player, With<LocalHostPlayer>>,
    mut hud: ResMut<HudInventory>,
) {
    for player in players.iter() {
        hud.slots = player.inventory.slots().to_vec();
        hud.selected = player.inventory.selected;
        hud.health = player.vitals.health;
        hud.oxygen = player.vitals.oxygen;
        hud.dose = player.vitals.dose;
        hud.heat = player.vitals.heat;
        hud.stamina = player.vitals.stamina;
    }
}

/// Copies the host's authoritative body into the local player for prediction.
pub fn mirror_host_body(
    host: Query<&PlayerBody, (With<LocalHostPlayer>, Without<LocalPlayer>)>,
    mut local: Query<(&mut LocalPlayer, &mut PlayerBody), Without<LocalHostPlayer>>,
) {
    let Ok(host_body) = host.single() else { return };
    let Ok((mut player, mut body)) = local.single_mut() else {
        return;
    };
    // Keep the local body in sync with the authoritative one while still
    // letting prediction run: only correct large divergence.
    let dx = host_body.pos[0] - body.pos[0];
    let dy = host_body.pos[1] - body.pos[1];
    let dz = host_body.pos[2] - body.pos[2];
    if dx * dx + dy * dy + dz * dz > 1.0 {
        body.pos = host_body.pos;
        body.vel = host_body.vel;
        body.grounded = host_body.grounded;
        player.correction = Vec3::ZERO;
    }
}
