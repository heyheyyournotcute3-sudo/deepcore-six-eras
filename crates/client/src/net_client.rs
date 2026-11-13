//! Networking on the client: connection, handshake, chunk decoding, remote
//! players and message senders.

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::SystemTime;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use deepcore_shared::content::TechId;
use deepcore_shared::protocol::*;

use crate::hud::{DebugStats, HudInventory};
use crate::player::LocalPlayer;
use crate::world::ClientWorld;

/// Connection settings from the command line or the main menu.
#[derive(Resource, Clone, Debug)]
pub struct ClientConfig {
    /// Server address to join, or `None` to host locally.
    pub server: Option<String>,
    /// UDP port for local hosting.
    pub port: u16,
    /// World seed used when hosting.
    pub seed: u64,
    /// Display name.
    pub name: String,
    /// Simulated latency in milliseconds.
    pub latency_ms: u64,
    /// Simulated packet loss percent.
    pub loss_percent: f64,
    /// Protocol id, must match the server.
    pub protocol_id: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server: None,
            port: 7788,
            seed: deepcore_shared::DEFAULT_SEED,
            name: "Miner".to_string(),
            latency_ms: 0,
            loss_percent: 0.0,
            protocol_id: 0x0DEE_C0DE,
        }
    }
}

impl ClientConfig {
    /// Parses `--join ip:port`, `--host`, `--name`, latency and loss flags.
    pub fn from_args(args: &[String]) -> Self {
        let mut cfg = Self::default();
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--join" => {
                    if let Some(v) = args.get(i + 1) {
                        cfg.server = Some(v.clone());
                    }
                    i += 2;
                }
                "--host" => {
                    cfg.server = None;
                    i += 1;
                }
                "--port" => {
                    if let Some(v) = args.get(i + 1).and_then(|v| v.parse().ok()) {
                        cfg.port = v;
                    }
                    i += 2;
                }
                "--seed" => {
                    if let Some(v) = args.get(i + 1).and_then(|v| v.parse().ok()) {
                        cfg.seed = v;
                    }
                    i += 2;
                }
                "--name" => {
                    if let Some(v) = args.get(i + 1) {
                        cfg.name = v.chars().take(MAX_NAME_LEN).collect();
                    }
                    i += 2;
                }
                "--simulate-latency" => {
                    if let Some(v) = args.get(i + 1).and_then(|v| v.parse().ok()) {
                        cfg.latency_ms = v;
                    }
                    i += 2;
                }
                "--simulate-loss" => {
                    if let Some(v) = args.get(i + 1).and_then(|v| v.parse().ok()) {
                        cfg.loss_percent = v;
                    }
                    i += 2;
                }
                _ => i += 1,
            }
        }
        cfg
    }
}

/// Client-side tech tree mirror.
#[derive(Resource, Default)]
pub struct ClientTech {
    /// Unlocked node ids.
    pub unlocked: Vec<TechId>,
    /// Delivered amounts.
    pub delivered: Vec<(TechId, deepcore_shared::content::ItemId, u32)>,
}

impl ClientTech {
    /// One-line objective shown on the HUD.
    pub fn objective_line(&self) -> String {
        if self.unlocked.is_empty() {
            "Objective: build a Workbench (timber from the camp trees)".to_string()
        } else {
            format!("Research unlocked: {} nodes", self.unlocked.len())
        }
    }
}

/// Chunks received but not yet meshed.
#[derive(Resource, Default)]
pub struct IncomingChunks {
    /// Decoded chunks waiting to be inserted.
    pub queue: Vec<deepcore_shared::voxel::Chunk>,
}

/// Registers client-side replication rules and messages.
pub fn register_client_protocol(app: &mut App) {
    app.replicate::<PlayerReplicated>();
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

/// Creates the renet client and transport.
pub fn start_client(
    commands: &mut Commands,
    channels: &RepliconChannels,
    cfg: &ClientConfig,
) -> anyhow::Result<()> {
    use bevy_replicon_renet::netcode::{ClientAuthentication, NetcodeClientTransport};
    use bevy_replicon_renet::renet::{ConnectionConfig, RenetClient};
    use bevy_replicon_renet::{RenetChannelsExt, RenetClient as RenetClientRes};

    let addr_str = cfg
        .server
        .clone()
        .unwrap_or_else(|| format!("127.0.0.1:{}", cfg.port));
    let server_addr: SocketAddr = if addr_str.contains(':') {
        addr_str.parse()?
    } else {
        format!("{addr_str}:{}", cfg.port).parse()?
    };

    let client = RenetClientRes(RenetClient::new(ConnectionConfig {
        server_channels_config: channels.server_configs(),
        client_channels_config: channels.client_configs(),
        ..Default::default()
    }));

    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let client_id = current_time.as_millis() as u64;
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: cfg.protocol_id,
        server_addr: SocketAddr::new(server_addr.ip().to_canonical(), server_addr.port()),
        user_data: None,
    };
    let transport = NetcodeClientTransport::new(current_time, authentication, socket)?;

    commands.insert_resource(client);
    commands.insert_resource(transport);
    Ok(())
}

/// Sends the handshake once the client is connected.
pub fn send_hello(
    mut state: Local<HelloState>,
    mut writer: MessageWriter<ClientHello>,
    world: Res<ClientWorld>,
    cfg: Res<ClientConfig>,
    client_state: Res<State<ClientState>>,
) {
    if *client_state.get() != ClientState::Connected || state.sent {
        return;
    }
    state.sent = true;
    writer.write(ClientHello {
        protocol: PROTOCOL_VERSION,
        content_hash: world.registry().hash,
        name: cfg.name.clone(),
    });
}

#[derive(Default)]
pub struct HelloState {
    sent: bool,
}

/// Applies the server's handshake answer.
pub fn handle_handshake(mut results: MessageReader<HandshakeResult>) {
    for r in results.read() {
        match r {
            HandshakeResult::Accepted { seed, .. } => {
                info!("handshake accepted, world seed {seed}");
            }
            HandshakeResult::Rejected { reason } => {
                error!("server rejected connection: {reason}");
            }
        }
    }
}

/// Decodes chunk payloads into the client world.
pub fn receive_chunks(
    mut payloads: MessageReader<ChunkPayload>,
    mut world: ResMut<ClientWorld>,
    mut incoming: ResMut<IncomingChunks>,
) {
    for p in payloads.read() {
        if p.unload {
            world.unload_chunk(p.pos);
            continue;
        }
        if let Some(chunk) = crate::net_client::decode_chunk(&p.data) {
            incoming.queue.push(chunk);
        }
    }
    for chunk in incoming.queue.drain(..) {
        world.insert_chunk(chunk);
    }
}

/// Decompresses a chunk payload.
pub fn decode_chunk(data: &[u8]) -> Option<deepcore_shared::voxel::Chunk> {
    let raw = zstd::decode_all(data).ok()?;
    deepcore_shared::voxel::Chunk::from_wire(&raw)
}

/// Applies block change batches.
pub fn apply_block_changes(
    mut batches: MessageReader<BlockChangeBatch>,
    mut world: ResMut<ClientWorld>,
) {
    for batch in batches.read() {
        for change in &batch.changes {
            world.set_block(change.pos, change.block);
        }
    }
}

/// Sends local movement inputs to the server.
pub fn send_move_inputs(
    mut writer: MessageWriter<MoveInputMsg>,
    mut players: Query<&mut LocalPlayer>,
    client_state: Res<State<ClientState>>,
) {
    if *client_state.get() != ClientState::Connected {
        return;
    }
    for player in players.iter_mut() {
        // Send the newest input only: unreliable channel, sequenced.
        let Some(input) = player.pending.last().copied() else {
            continue;
        };
        writer.write(MoveInputMsg {
            seq: input.seq,
            dir: input.dir,
            jump: input.jump,
            sprint: input.sprint,
            crouch: input.crouch,
            yaw: input.yaw,
            pitch: input.pitch,
        });
    }
}

/// Applies authoritative movement corrections.
pub fn apply_corrections(
    mut corrections: MessageReader<MovementCorrection>,
    mut players: Query<(&mut LocalPlayer, &mut deepcore_shared::movement::PlayerBody)>,
    mut stats: ResMut<DebugStats>,
) {
    let Ok((mut player, mut body)) = players.single_mut() else {
        return;
    };
    for c in corrections.read() {
        player.pending.retain(|i| i.seq > c.seq);
        let delta = Vec3::new(
            c.pos[0] - body.pos[0],
            c.pos[1] - body.pos[1],
            c.pos[2] - body.pos[2],
        );
        if delta.length() > 0.3 {
            // Snap for large corrections, smooth small ones.
            body.pos = c.pos;
            body.vel = c.vel;
            body.grounded = c.grounded;
            stats.corrections += 1;
        } else if delta.length() > 0.01 {
            player.correction += delta;
            body.vel = c.vel;
            body.grounded = c.grounded;
            stats.corrections += 1;
        }
    }
}

/// Applies vitals sent by the server.
pub fn apply_vitals(mut vitals: MessageReader<VitalsMsg>, mut hud: ResMut<HudInventory>) {
    for v in vitals.read() {
        hud.health = v.0.health;
        hud.oxygen = v.0.oxygen;
        hud.dose = v.0.dose;
        hud.heat = v.0.heat;
        hud.stamina = v.0.stamina;
    }
}

/// Applies tech tree broadcasts.
pub fn apply_tech(mut states: MessageReader<TechTreeState>, mut tech: ResMut<ClientTech>) {
    for s in states.read() {
        tech.unlocked = s.unlocked.clone();
        tech.delivered = s.delivered.clone();
    }
}

/// Applies mine refusals to the HUD message line.
pub fn apply_refusals(
    mut refusals: MessageReader<MineRefusalMsg>,
    mut players: Query<&mut LocalPlayer>,
) {
    let Ok(mut player) = players.single_mut() else {
        return;
    };
    for r in refusals.read() {
        player.message = r.text();
    }
}

/// Interpolates remote player transforms from replicated state.
type RemotePlayerQuery<'w, 's> =
    Query<'w, 's, (&'static PlayerReplicated, &'static mut Transform), (Without<LocalPlayer>, With<Remote>)>;

pub fn interpolate_remote_players(time: Res<Time>, mut remotes: RemotePlayerQuery) {
    let dt = time.delta_secs();
    for (rep, mut transform) in remotes.iter_mut() {
        let target = Vec3::new(rep.pos[0], rep.pos[1], rep.pos[2]);
        let lerp = (dt * 12.0).min(1.0);
        transform.translation = transform.translation.lerp(target, lerp);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, rep.yaw, rep.pitch, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_from_str(s: &str) -> ClientConfig {
        ClientConfig::from_args(&s.split_whitespace().map(String::from).collect::<Vec<_>>())
    }

    #[test]
    fn config_parses_join() {
        let cfg = config_from_str("--join 1.2.3.4:9000 --name Ada");
        assert_eq!(cfg.server.as_deref(), Some("1.2.3.4:9000"));
        assert_eq!(cfg.name, "Ada");
    }

    #[test]
    fn config_defaults_to_hosting() {
        let cfg = config_from_str("");
        assert!(cfg.server.is_none());
    }

    #[test]
    fn name_is_truncated() {
        let long = "a".repeat(100);
        let cfg = config_from_str(&format!("--name {long}"));
        assert!(cfg.name.len() <= MAX_NAME_LEN);
    }
}
