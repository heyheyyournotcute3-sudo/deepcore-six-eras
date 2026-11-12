//! Deepcore dedicated server binary.
//!
//! ```text
//! deepcore-server --port 7788 --seed 1 --bots 0 --simulate-latency 0
//! ```

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use deepcore_server::{ServerConfig, ServerPlugin};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cfg = ServerConfig::from_args(&args);

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::log::LogPlugin {
            level: tracing::Level::INFO,
            ..default()
        })
        .add_plugins(bevy::state::app::StatesPlugin)
        .insert_resource(cfg.clone())
        .insert_resource(Time::<Fixed>::from_hz(deepcore_shared::TICK_HZ as f64))
        .add_plugins(ServerPlugin::default());

    // Renet server resources are created once replicon has registered all
    // channels, which happens in `ServerPlugin::build`.
    let channels = app.world().resource::<RepliconChannels>().clone();
    let mut commands = app.world_mut().commands();
    deepcore_server::net::start_server(&mut commands, &channels, &cfg)?;

    tracing::info!(
        "listening on 0.0.0.0:{} (protocol {}), seed {}",
        cfg.port,
        deepcore_shared::protocol::PROTOCOL_VERSION,
        cfg.seed
    );
    app.run();
    Ok(())
}
