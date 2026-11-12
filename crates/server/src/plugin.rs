//! Server plugin: builds the whole authoritative game on a Bevy `App`.
//!
//! `ServerPlugin` is added by the dedicated server binary and by the client
//! when hosting, so hosting is literally "the same plugins in this process".

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use deepcore_shared::content::ContentRegistry;
use deepcore_shared::movement::PlayerBody;
use deepcore_shared::protocol::*;
use deepcore_shared::worldgen::spawn_pos;

use crate::net::*;
use crate::sim::intents::*;
use crate::sim::pickups::Pickups;
use crate::sim::resources::*;
use crate::world::World;

/// Adds the whole server simulation, with or without networking.
pub struct ServerPlugin {
    /// Skip renet/replicon setup (used by headless tests).
    pub networking: bool,
}

impl Default for ServerPlugin {
    fn default() -> Self {
        Self { networking: true }
    }
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        let cfg = app
            .world()
            .get_resource::<ServerConfig>()
            .cloned()
            .unwrap_or_default();

        let registry = match ContentRegistry::load_dir(deepcore_shared::data_dir()) {
            Ok(r) => r,
            Err(e) => {
                error!("failed to load content: {e}");
                panic!("content load failed: {e}");
            }
        };
        info!(
            "loaded content: {} blocks, {} items, {} recipes, {} machines, {} tech nodes",
            registry.blocks().len(),
            registry.items().len(),
            registry.recipes().len(),
            registry.machines().len(),
            registry.tech_nodes().len()
        );

        app.insert_resource(WorldRes(World::new(registry, cfg.seed)))
            .init_resource::<Grid>()
            .init_resource::<TechTree>()
            .init_resource::<PendingChanges>()
            .init_resource::<PendingChunks>()
            .init_resource::<ClientChunks>()
            .init_resource::<Pickups>()
            .init_resource::<LastInputs>()
            .init_resource::<TechDirty>()
            .insert_resource(ServerClock::default());

        if self.networking {
            app.add_plugins((RepliconPlugins, bevy_replicon_renet::RepliconRenetPlugins));
            register_protocol(app);
        }

        app.add_systems(Startup, log_startup)
            .add_systems(
                FixedUpdate,
                (
                    crate::sim::pickups::flush_pickup_queue,
                    crate::sim::mining::tick_mining,
                    crate::sim::machines::tick_machines,
                    crate::sim::hazards::tick_hazards,
                    crate::sim::pickups::collect_pickups,
                )
                    .chain(),
            );

        if self.networking {
            app.add_systems(
                PreUpdate,
                (
                    handle_hello,
                    handle_move_input,
                    handle_start_mining,
                    handle_stop_mining,
                    handle_place_block,
                    handle_move_items,
                    handle_craft,
                    handle_chat,
                    handle_respawn,
                )
                    .after(ServerSystems::Receive),
            )
            .add_systems(
                FixedUpdate,
                (
                    broadcast_block_changes,
                    stream_chunks,
                    sync_replicated_players,
                    send_inventory,
                    send_tech_tree,
                )
                    .chain(),
            );
        }
    }
}

/// Tick counter for diagnostics.
#[derive(Resource, Default)]
pub struct ServerClock {
    /// Ticks elapsed.
    pub ticks: u64,
}

fn log_startup(clock: Res<ServerClock>, world: Res<WorldRes>) {
    info!(
        "deepcore server ready: seed {}, tick {} Hz, world {}x{}x{}",
        world.0.seed(),
        deepcore_shared::TICK_HZ,
        deepcore_shared::WORLD_SIZE,
        deepcore_shared::WORLD_HEIGHT,
        deepcore_shared::WORLD_SIZE
    );
    let _ = clock;
}

/// Spawns a player entity with the starting kit.
pub fn spawn_player(
    commands: &mut Commands,
    world: &WorldRes,
    name: &str,
) -> Entity {
    let reg = world.0.registry();
    let mut inventory = deepcore_shared::inventory::Inventory::new();
    give_starting_kit(reg, &mut inventory);
    let spawn = spawn_pos();
    
    commands
        .spawn((
            Player {
                name: name.to_string(),
                inventory,
                vitals: deepcore_shared::hazards::Vitals::default(),
                last_seq: 0,
                mining: None,
                open_machine: None,
                dead: false,
                look_yaw: 0.0,
                look_pitch: 0.0,
            },
            PlayerBody {
                pos: [spawn.x as f32 + 0.5, spawn.y as f32 + 0.2, spawn.z as f32 + 0.5],
                ..Default::default()
            },
            PlayerReplicated {
                pos: [spawn.x as f32 + 0.5, spawn.y as f32 + 0.2, spawn.z as f32 + 0.5],
                yaw: 0.0,
                pitch: 0.0,
                held: None,
                health: 1.0,
            },
            PlayerName(name.to_string()),
            InventoryDirty(true),
            Replicated,
        ))
        .id()
}
