//! Deepcore client: windowed game binary.
//!
//! Runs in one of two modes:
//! * `--host` (default): the authoritative server plugins run inside this
//!   process, so a single player or a host-and-friends session needs no extra
//!   binary.
//! * `--join <ip:port>`: connect to a dedicated server.

use bevy::prelude::*;
use bevy_replicon::prelude::*;

mod host;
mod hud;
mod net_client;
mod player;
mod render;
mod ui;
mod world;

use deepcore_shared::content::ContentRegistry;
use net_client::ClientConfig;
use player::{LocalPlayer, PlayerSettings};
use ui::CaptureRequest;
use world::ClientWorld;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cfg = ClientConfig::from_args(&args);
    let hosting = cfg.server.is_none();

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: if hosting {
                        "Deepcore — Hosting".into()
                    } else {
                        format!("Deepcore — {}", cfg.server.clone().unwrap_or_default())
                    },
                    resolution: (1280, 720).into(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: deepcore_shared::assets_dir().display().to_string(),
                ..default()
            }),
    );

    // Shared content and simulation clock.
    app.add_plugins(deepcore_shared::SharedPlugin);

    // Client world, built from the same content as the server.
    let registry = ContentRegistry::load_dir(deepcore_shared::data_dir())?;
    app.insert_resource(ClientWorld::new(registry, cfg.seed))
        .insert_resource(cfg.clone())
        .init_resource::<PlayerSettings>()
        .init_resource::<hud::HudInventory>()
        .init_resource::<hud::DebugStats>()
        .init_resource::<net_client::ClientTech>()
        .init_resource::<ui::CaptureRequest>()
        .insert_resource(CaptureRequest { want_capture: true })
        .add_message::<deepcore_shared::protocol::MineRefusalMsg>()
        .add_message::<deepcore_shared::protocol::Notification>()
        .add_message::<deepcore_shared::protocol::PlayerDied>()
        .add_message::<deepcore_shared::protocol::TechTreeState>()
        .add_plugins((render::VoxelRenderPlugin, ui::UiPlugin))
        .add_systems(Startup, (setup_scene, hud::setup_hud))
        .add_systems(
            Update,
            (
                player::apply_settings,
                player::mouse_look,
                player::predict_movement,
                player::update_targeting,
                apply_capture,
                hud::update_crosshair,
                hud::update_depth,
                hud::update_vitals,
                hud::update_hotbar,
                hud::update_message,
                hud::update_debug,
                hud::update_objectives,
                draw_target_outline,
                follow_player,
            )
                // Systems that write the same resource are ordered explicitly:
                // Bevy silently skips ambiguous parallel systems, which would
                // disable HUD updates without any error.
                .chain(),
        );

    if hosting {
        // Run the whole authoritative server in this process.
        app.insert_resource(deepcore_server::ServerConfig {
            seed: cfg.seed,
            port: cfg.port,
            ..default()
        })
        .add_plugins(deepcore_server::ServerPlugin { networking: false })
        .add_systems(Startup, spawn_local_host.after(setup_scene))
        .add_systems(
            Update,
            (
                host::stream_local_chunks,
                host::mirror_host_inventory,
                host::mirror_host_body,
                sync_host_seed,
            )
                .chain(),
        );
    } else {
        app.add_plugins((RepliconPlugins, bevy_replicon_renet::RepliconRenetPlugins));
        net_client::register_client_protocol(&mut app);
        let channels = app.world().resource::<RepliconChannels>().clone();
        let mut commands = app.world_mut().commands();
        net_client::start_client(&mut commands, &channels, &cfg)?;
        app.add_systems(
            Update,
            (
                net_client::send_hello,
                net_client::handle_handshake,
                net_client::receive_chunks,
                net_client::apply_block_changes,
                net_client::send_move_inputs,
                net_client::apply_corrections,
                net_client::apply_vitals,
                net_client::apply_tech,
                net_client::apply_refusals,
                net_client::interpolate_remote_players,
                hud::apply_inventory_snapshot,
            ),
        );
    }

    app.run();
    Ok(())
}

/// Spawns the camera and the local player.
fn setup_scene(mut commands: Commands) {
    let spawn = deepcore_shared::worldgen::spawn_pos();
    commands.spawn((
        Camera3d::default(),
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        // Ambient light is a camera component in Bevy 0.20, not a resource:
        // without it, faces pointing away from the sun render pure black.
        AmbientLight {
            color: Color::srgb(0.62, 0.68, 0.82),
            brightness: 900.0,
            affects_lightmapped_meshes: true,
        },
        Projection::Perspective(PerspectiveProjection {
            fov: 75.0_f32.to_radians(),
            ..default()
        }),
        Transform::from_xyz(
            spawn.x as f32 + 0.5,
            spawn.y as f32 + 1.0,
            spawn.z as f32 + 0.5,
        )
        .looking_at(
            Vec3::new(spawn.x as f32 + 0.5, spawn.y as f32, spawn.z as f32 - 10.0),
            Vec3::Y,
        ),
    ));
    commands.spawn((
        LocalPlayer::default(),
        deepcore_shared::movement::PlayerBody {
            pos: [
                spawn.x as f32 + 0.5,
                spawn.y as f32 + 1.0,
                spawn.z as f32 + 0.5,
            ],
            ..default()
        },
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default(),
    ));
    // Daylight for the surface camp.
    commands.spawn((
        DirectionalLight {
            illuminance: 9_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 100.0, 0.0).looking_at(Vec3::new(1.0, 0.0, 1.0), Vec3::Y),
    ));
}

/// Spawns the authoritative player entity for host mode.
fn spawn_local_host(
    mut commands: Commands,
    world: Res<deepcore_server::sim::resources::WorldRes>,
    cfg: Res<ClientConfig>,
) {
    let entity = deepcore_server::spawn_player(&mut commands, &world, &cfg.name);
    commands.entity(entity).insert(host::LocalHostPlayer);
}

/// Keeps the client world's seed in sync with the hosted server.
fn sync_host_seed(world: Res<deepcore_server::sim::resources::WorldRes>, client: Res<ClientWorld>) {
    if client.seed != world.0.seed() {
        warn!(
            "client seed {} differs from server seed {}",
            client.seed,
            world.0.seed()
        );
    }
}

/// Grabs or releases the mouse based on the UI state.
fn apply_capture(
    request: Res<CaptureRequest>,
    mut cursors: Query<&mut bevy::window::CursorOptions>,
    mut players: Query<&mut LocalPlayer>,
) {
    let want = request.want_capture;
    let mut changed = false;
    for mut player in players.iter_mut() {
        if player.captured != want {
            player.captured = want;
            changed = true;
        }
    }
    if changed {
        player::set_capture(&mut cursors, want);
    }
}

/// Keeps the camera at the local player's eye height, looking where the
/// player looks.
fn follow_player(
    players: Query<(&LocalPlayer, &Transform), Without<Camera3d>>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let Ok((player, player_tf)) = players.single() else {
        return;
    };
    let Ok(mut camera_tf) = cameras.single_mut() else {
        return;
    };
    camera_tf.translation = player_tf.translation;
    camera_tf.rotation = player::look_rotation(player.yaw, player.pitch);
}

/// Draws a wireframe box around the targeted block.
fn draw_target_outline(mut gizmos: Gizmos, players: Query<&LocalPlayer>) {
    let Ok(player) = players.single() else { return };
    if let Some((pos, _)) = player.target {
        let center = Vec3::new(pos.x as f32 + 0.5, pos.y as f32 + 0.5, pos.z as f32 + 0.5);
        let color = if player.too_hard {
            Color::srgb(1.0, 0.2, 0.2)
        } else {
            Color::srgb(1.0, 1.0, 1.0)
        };
        gizmos.cube(
            Transform::from_translation(center).with_scale(Vec3::splat(1.02)),
            color,
        );
    }
}
