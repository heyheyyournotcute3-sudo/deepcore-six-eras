//! Deepcore client: windowed game binary.

use bevy::prelude::*;

fn main() -> anyhow::Result<()> {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Deepcore".into(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(deepcore_shared::SharedPlugin);
    app.run();
    Ok(())
}
