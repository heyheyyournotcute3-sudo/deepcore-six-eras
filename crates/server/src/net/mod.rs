//! Networking: replicon setup, channels, handshake, chunk streaming and the
//! per-tick broadcast of block changes, inventory snapshots and events.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use deepcore_shared::content::{BlockId, ContentRegistry};
use deepcore_shared::protocol::*;
use deepcore_shared::voxel::{BlockPos, ChunkPos};

use crate::sim::resources::{Player, ServerConfig, WorldRes};

/// Chunk data queued for sending to one client.
#[derive(Resource, Default)]
pub struct PendingChunks {
    /// Per-client chunk payloads.
    pub queues: HashMap<Entity, Vec<ChunkPayload>>,
}

/// Block changes accumulated during one tick.
#[derive(Resource, Default)]
pub struct PendingChanges {
    /// The batch.
    pub batch: BlockChangeBatch,
}

/// Which chunks each client currently has.
#[derive(Resource, Default)]
pub struct ClientChunks {
    /// Per-client loaded chunk set.
    pub loaded: HashMap<Entity, std::collections::HashSet<ChunkPos>>,
}

/// Marker on the replicated player entity on the server.
#[derive(Component)]
pub struct ServerPlayerRoot;

/// Registers every replicated component and remote message.
pub fn register_protocol(app: &mut App) {
    app.replicate::<PlayerReplicated>();
    app.replicate::<PlayerName>();

    app.add_client_message::<ClientHello>(Channel::Ordered)
        .add_client_message::<MoveInputMsg>(Channel::Unreliable)
        .add_client_message::<StartMining>(Channel::Ordered)
        .add_client_message::<StopMining>(Channel::Ordered)
        .add_client_message::<PlaceBlock>(Channel::Ordered)
        .add_client_message::<Interact>(Channel::Ordered)
        .add_client_message::<MoveItems>(Channel::Ordered)
        .add_client_message::<Craft>(Channel::Ordered)
        .add_client_message::<SetMachineRecipe>(Channel::Ordered)
        .add_client_message::<ResearchDeposit>(Channel::Ordered)
        .add_client_message::<Chat>(Channel::Ordered)
        .add_client_message::<Respawn>(Channel::Ordered)
        .add_client_message::<SetControlRods>(Channel::Ordered);

    app.add_server_message::<HandshakeResult>(Channel::Ordered)
        .add_server_message::<BlockChangeBatch>(Channel::Ordered)
        .add_server_message::<ChunkPayload>(Channel::Ordered)
        .add_server_message::<Notification>(Channel::Ordered)
        .add_server_message::<InventorySnapshot>(Channel::Ordered)
        .add_server_message::<MachineUiState>(Channel::Ordered)
        .add_server_message::<TechTreeState>(Channel::Ordered)
        .add_server_message::<ChatLine>(Channel::Ordered)
        .add_server_message::<PlayerDied>(Channel::Ordered)
        .add_server_message::<MineRefusalMsg>(Channel::Unordered)
        .add_server_message::<MovementCorrection>(Channel::Unreliable)
        .add_server_message::<VitalsMsg>(Channel::Unordered)
        .add_server_message::<NetworkStatsMsg>(Channel::Unreliable);
}

/// Replicated display name of a player.
#[derive(Component, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerName(pub String);

/// Builds the renet server and inserts the transport resources.
pub fn start_server(
    commands: &mut Commands,
    channels: &RepliconChannels,
    cfg: &ServerConfig,
) -> anyhow::Result<()> {
    use bevy_replicon_renet::renet::ConnectionConfig;
    use bevy_replicon_renet::{RenetChannelsExt, RenetServer};
    use bevy_replicon_renet::netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig as NetcodeConfig};
    use std::net::{Ipv4Addr, UdpSocket};
    use std::time::SystemTime;

    let server = RenetServer(bevy_replicon_renet::renet::RenetServer::new(ConnectionConfig {
        server_channels_config: channels.server_configs(),
        client_channels_config: channels.client_configs(),
        ..Default::default()
    }));

    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, cfg.port))?;
    let netcode = NetcodeConfig {
        current_time,
        max_clients: 16,
        protocol_id: cfg.protocol_id,
        authentication: ServerAuthentication::Unsecure,
        public_addresses: Default::default(),
    };
    let transport = NetcodeServerTransport::new(netcode, socket)?;

    commands.insert_resource(server);
    commands.insert_resource(transport);
    Ok(())
}

/// Handles the `ClientHello` handshake: checks protocol and content hash and
/// tells the client the seed, or rejects it.
pub fn handle_hello(
    world: Res<WorldRes>,
    mut hellos: MessageReader<FromClient<ClientHello>>,
    mut results: MessageWriter<ToClients<HandshakeResult>>,
) {
    let hash = world.0.registry().hash;
    for hello in hellos.read() {
        let reason = if hello.message.protocol != PROTOCOL_VERSION {
            Some(format!(
                "Protocol mismatch: server {}, client {}",
                PROTOCOL_VERSION, hello.message.protocol
            ))
        } else if hello.message.content_hash != hash {
            Some("Content mismatch".to_string())
        } else if hello.message.name.trim().is_empty() {
            Some("Empty name".to_string())
        } else {
            None
        };
        let target = hello.client_id;
        let result = match reason {
            Some(reason) => HandshakeResult::Rejected { reason },
            None => HandshakeResult::Accepted {
                seed: world.0.seed(),
                content_hash: hash,
                tick_hz: deepcore_shared::TICK_HZ,
            },
        };
        results.write(ToClients {
            targets: SendTargets::Single(target),
            message: result,
        });
    }
}

/// Compresses a chunk for the wire.
pub fn encode_chunk(chunk: &deepcore_shared::voxel::Chunk) -> Option<Vec<u8>> {
    let raw = chunk.to_wire();
    zstd::encode_all(&raw[..], 3).ok()
}

/// Decompresses a chunk payload.
pub fn decode_chunk(data: &[u8]) -> Option<deepcore_shared::voxel::Chunk> {
    let raw = zstd::decode_all(data).ok()?;
    deepcore_shared::voxel::Chunk::from_wire(&raw)
}

/// Sends chunks around each player, nearest first, and unloads chunks that
/// left the view radius.
pub fn stream_chunks(
    mut world: ResMut<WorldRes>,
    mut pending: ResMut<PendingChunks>,
    mut client_chunks: ResMut<ClientChunks>,
    players: Query<(Entity, &Player, &deepcore_shared::movement::PlayerBody)>,
    mut sender: MessageWriter<ToClients<ChunkPayload>>,
) {
    for (entity, _player, body) in players.iter() {
        let center = BlockPos::new(
            body.pos[0].floor() as i32,
            body.pos[1].floor() as i32,
            body.pos[2].floor() as i32,
        )
        .chunk();
        let loaded = client_chunks.loaded.entry(entity).or_default();

        // Build the wanted set, ordered by distance.
        let mut wanted: Vec<ChunkPos> = Vec::new();
        for dx in -VIEW_CHUNKS_H..=VIEW_CHUNKS_H {
            for dy in -VIEW_CHUNKS_V..=VIEW_CHUNKS_V {
                for dz in -VIEW_CHUNKS_H..=VIEW_CHUNKS_H {
                    let pos = center.offset([dx, dy, dz]);
                    if pos.y < 0 || pos.y * deepcore_shared::CHUNK_SIZE >= deepcore_shared::WORLD_HEIGHT {
                        continue;
                    }
                    wanted.push(pos);
                }
            }
        }
        wanted.sort_by_key(|p| p.distance_to(center));

        let queue = pending.queues.entry(entity).or_default();
        let mut sent = 0;
        for pos in wanted.iter().take(512) {
            if loaded.contains(pos) || queue.iter().any(|q| q.pos == *pos && !q.unload) {
                continue;
            }
            let chunk = world.0.chunk(*pos);
            if let Some(data) = encode_chunk(chunk) {
                queue.push(ChunkPayload { pos: *pos, data, unload: false });
                sent += 1;
            }
            if sent >= 8 {
                break;
            }
        }

        // Unload chunks that are now out of range.
        let wanted_set: std::collections::HashSet<ChunkPos> = wanted.into_iter().collect();
        let to_unload: Vec<ChunkPos> = loaded.difference(&wanted_set).copied().collect();
        for pos in to_unload {
            queue.push(ChunkPayload { pos, data: Vec::new(), unload: true });
        }
    }

    for (entity, payloads) in pending.queues.drain() {
        let loaded = client_chunks.loaded.entry(entity).or_default();
        for p in payloads {
            if p.unload {
                loaded.remove(&p.pos);
            } else {
                loaded.insert(p.pos);
            }
            sender.write(ToClients {
                targets: SendTargets::Single(ClientId::Client(entity)),
                message: p,
            });
        }
    }
}

/// Broadcasts the block changes accumulated this tick.
pub fn broadcast_block_changes(
    mut changes: ResMut<PendingChanges>,
    clients: Query<Entity, With<ConnectedClient>>,
    mut sender: MessageWriter<ToClients<BlockChangeBatch>>,
) {
    if changes.batch.changes.is_empty() {
        return;
    }
    let batch = std::mem::take(&mut changes.batch);
    for client in clients.iter() {
        sender.write(ToClients {
            targets: SendTargets::Single(ClientId::Client(client)),
            message: batch.clone(),
        });
    }
}

/// Replicates the authoritative player position each tick.
pub fn sync_replicated_players(
    mut players: Query<(&Player, &deepcore_shared::movement::PlayerBody, &mut PlayerReplicated)>,
) {
    for (player, body, mut rep) in players.iter_mut() {
        rep.pos = body.pos;
        rep.held = player.inventory.held().map(|s| s.item);
        rep.health = crate::sim::hazards::health_fraction(&player.vitals);
    }
}

/// Sends an inventory snapshot to the owner whenever it changes.
pub fn send_inventory(
    mut players: Query<(Entity, &Player, &mut InventoryDirty)>,
    mut sender: MessageWriter<ToClients<InventorySnapshot>>,
) {
    for (entity, player, mut dirty) in players.iter_mut() {
        if !dirty.0 {
            continue;
        }
        dirty.0 = false;
        sender.write(ToClients {
            targets: SendTargets::Single(ClientId::Client(entity)),
            message: InventorySnapshot {
                slots: player.inventory.slots().to_vec(),
                selected: player.inventory.selected,
            },
        });
    }
}

/// Marks a player whose inventory must be re-sent.
#[derive(Component, Default)]
pub struct InventoryDirty(pub bool);

/// Sends the tech tree to everyone when it changes.
pub fn send_tech_tree(
    tech: Res<crate::sim::resources::TechTree>,
    mut dirty: ResMut<TechDirty>,
    clients: Query<Entity, With<ConnectedClient>>,
    mut sender: MessageWriter<ToClients<TechTreeState>>,
) {
    if !dirty.0 {
        return;
    }
    dirty.0 = false;
    let state = TechTreeState {
        unlocked: tech.unlocked.iter().copied().collect(),
        delivered: tech
            .delivered
            .iter()
            .map(|((node, item), n)| (*node, *item, *n))
            .collect(),
    };
    for client in clients.iter() {
        sender.write(ToClients {
            targets: SendTargets::Single(ClientId::Client(client)),
            message: state.clone(),
        });
    }
}

/// Set when the tech tree changed and must be broadcast.
#[derive(Resource, Default)]
pub struct TechDirty(pub bool);

/// Encodes a block change batch for tests.
pub fn encode_batch(batch: &BlockChangeBatch) -> Vec<u8> {
    postcard::to_allocvec(batch).unwrap_or_default()
}

/// Helper used by tests: the air block id.
pub fn air(reg: &ContentRegistry) -> BlockId {
    reg.air
}
