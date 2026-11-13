//! HUD: crosshair, hotbar, vitals, depth meter, Geiger counter, debug overlay
//! and the objective list.

use bevy::prelude::*;
use bevy::text::FontSize;
use bevy::ui::widget::Text;
use deepcore_shared::hazards::{DOSE_DAMAGE, DOSE_GRAIN, DOSE_NO_REGEN};
use deepcore_shared::inventory::HOTBAR_SLOTS;
use deepcore_shared::protocol::InventorySnapshot;
use deepcore_shared::worldgen::depth_below_surface;

use crate::player::LocalPlayer;
use crate::world::ClientWorld;

/// Root marker for HUD elements that follow the player.
#[derive(Component)]
pub struct HudRoot;

/// Crosshair element.
#[derive(Component)]
pub struct Crosshair;

/// Depth meter text.
#[derive(Component)]
pub struct DepthText;

/// Vitals text.
#[derive(Component)]
pub struct VitalsText;

/// Hotbar label text.
#[derive(Component)]
pub struct HotbarText;

/// Message line under the crosshair.
#[derive(Component)]
pub struct MessageText;

/// Debug overlay text.
#[derive(Component)]
pub struct DebugText;

/// Objective list text.
#[derive(Component)]
pub struct ObjectiveText;

/// Local copy of the player's inventory for the HUD.
#[derive(Resource, Default)]
pub struct HudInventory {
    /// Slots.
    pub slots: Vec<Option<deepcore_shared::inventory::ItemStack>>,
    /// Selected hotbar slot.
    pub selected: u8,
    /// Vitals snapshot.
    pub health: f32,
    /// Oxygen.
    pub oxygen: f32,
    /// Dose.
    pub dose: f32,
    /// Body heat.
    pub heat: f32,
    /// Stamina.
    pub stamina: f32,
}

/// Debug counters.
#[derive(Resource, Default)]
pub struct DebugStats {
    /// Frames per second.
    pub fps: f32,
    /// Loaded chunks.
    pub chunks: usize,
    /// Ping in milliseconds.
    pub ping: u32,
    /// Prediction corrections in the last second.
    pub corrections: u32,
    /// Kilometres... no: metres below the surface.
    pub depth: i32,
}

/// Builds the HUD on startup.
pub fn setup_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            HudRoot,
        ))
        .with_children(|parent| {
            // Crosshair.
            parent.spawn((
                Node {
                    width: Val::Px(4.0),
                    height: Val::Px(4.0),
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0),
                    margin: UiRect::all(Val::Px(-2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.9)),
                Crosshair,
            ));
            // Depth meter.
            parent.spawn((
                Text::new("Depth 0 m"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(20.0),
                    top: Val::Px(20.0),
                    ..default()
                },
                DepthText,
            ));
            // Vitals.
            parent.spawn((
                Text::new("HP 100  O2 100"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(20.0),
                    bottom: Val::Px(80.0),
                    ..default()
                },
                VitalsText,
            ));
            // Hotbar.
            parent.spawn((
                Text::new("[1] -"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Px(24.0),
                    margin: UiRect::left(Val::Px(-140.0)),
                    ..default()
                },
                HotbarText,
            ));
            // Message.
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.6, 0.4)),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(56.0),
                    margin: UiRect::left(Val::Px(-100.0)),
                    ..default()
                },
                MessageText,
            ));
            // Objectives.
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.85, 1.0)),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(20.0),
                    top: Val::Px(20.0),
                    ..default()
                },
                ObjectiveText,
            ));
            // Debug overlay, hidden until F3.
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 1.0, 0.7)),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(20.0),
                    bottom: Val::Px(20.0),
                    ..default()
                },
                Visibility::Hidden,
                DebugText,
            ));
        });
}

/// Applies an inventory snapshot to the HUD.
pub fn apply_inventory_snapshot(
    mut snapshots: MessageReader<InventorySnapshot>,
    mut hud: ResMut<HudInventory>,
) {
    for snap in snapshots.read() {
        hud.slots = snap.slots.clone();
        hud.selected = snap.selected;
    }
}

/// Updates the crosshair colour.
pub fn update_crosshair(
    players: Query<&LocalPlayer>,
    mut crosshairs: Query<&mut BackgroundColor, With<Crosshair>>,
) {
    let Ok(player) = players.single() else { return };
    let color = if player.too_hard {
        Color::srgb(1.0, 0.25, 0.25)
    } else {
        Color::srgba(1.0, 1.0, 1.0, 0.9)
    };
    for mut bg in crosshairs.iter_mut() {
        bg.0 = color;
    }
}

/// Updates the depth meter and Geiger readout.
pub fn update_depth(
    world: Res<ClientWorld>,
    players: Query<&deepcore_shared::movement::PlayerBody>,
    mut texts: Query<&mut Text, With<DepthText>>,
) {
    let Ok(body) = players.single() else { return };
    let pos = BlockPosLike::from(body.pos);
    let depth = depth_below_surface(pos.y);
    let reg = world.registry();
    let layer = deepcore_shared::worldgen::layer_def_at(reg.layers(), pos.y)
        .map(|l| l.name.clone())
        .unwrap_or_else(|| "Surface".to_string());
    let dose = crate::player::dose_estimate(&world, body.pos);
    let clicks = if dose > 0.05 {
        format!("  Geiger {:.1} µSv/s", dose * 10.0)
    } else {
        String::new()
    };
    for mut text in texts.iter_mut() {
        **text = format!("{layer}  —  {} m below surface{clicks}", depth.max(0));
    }
}

/// Helper to turn a float position into a block position.
pub struct BlockPosLike;

impl BlockPosLike {
    /// Converts.
    pub fn from(p: [f32; 3]) -> deepcore_shared::voxel::BlockPos {
        deepcore_shared::voxel::BlockPos::new(
            p[0].floor() as i32,
            p[1].floor() as i32,
            p[2].floor() as i32,
        )
    }
}

/// Updates vitals text with colour warnings.
pub fn update_vitals(hud: Res<HudInventory>, mut texts: Query<&mut Text, With<VitalsText>>) {
    let dose_warning = if hud.dose >= DOSE_DAMAGE {
        "  ☢ CRITICAL"
    } else if hud.dose >= DOSE_NO_REGEN {
        "  ☢ no regen"
    } else if hud.dose >= DOSE_GRAIN {
        "  ☢"
    } else {
        ""
    };
    let heat_warning = if hud.heat > 42.0 { "  HEAT" } else { "" };
    let line = format!(
        "HP {:.0}   O2 {:.0}   Stamina {:.0}   Dose {:.0}{dose_warning}{heat_warning}",
        hud.health, hud.oxygen, hud.stamina, hud.dose
    );
    for mut text in texts.iter_mut() {
        **text = line.clone();
    }
}

/// Updates the hotbar text from the inventory snapshot.
pub fn update_hotbar(
    world: Res<ClientWorld>,
    hud: Res<HudInventory>,
    mut texts: Query<&mut Text, With<HotbarText>>,
) {
    let reg = world.registry();
    let mut parts = Vec::new();
    for i in 0..HOTBAR_SLOTS {
        let label = match hud.slots.get(i).copied().flatten() {
            Some(stack) => {
                let name = reg.item_name(stack.item);
                let short: String = name.chars().take(10).collect();
                format!("{short} x{}", stack.count)
            }
            None => "-".to_string(),
        };
        if i as u8 == hud.selected {
            parts.push(format!("[{i}] {label}"));
        } else {
            parts.push(format!(" {i}  {label}"));
        }
    }
    let line = parts.join("   ");
    for mut text in texts.iter_mut() {
        **text = line.clone();
    }
}

/// Updates the message line.
pub fn update_message(
    players: Query<&LocalPlayer>,
    mut texts: Query<&mut Text, With<MessageText>>,
) {
    let Ok(player) = players.single() else { return };
    for mut text in texts.iter_mut() {
        **text = player.message.clone();
    }
}

/// Toggles the debug overlay with F3 and fills it in.
pub fn update_debug(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    world: Res<ClientWorld>,
    mut stats: ResMut<DebugStats>,
    mut texts: Query<(&mut Text, &mut Visibility), With<DebugText>>,
) {
    let dt = time.delta_secs();
    if dt > 0.0 {
        stats.fps = stats.fps * 0.9 + (1.0 / dt) * 0.1;
    }
    stats.chunks = world.loaded_chunks();
    for (mut text, mut vis) in texts.iter_mut() {
        if keys.just_pressed(KeyCode::F3) {
            *vis = match *vis {
                Visibility::Hidden => Visibility::Visible,
                _ => Visibility::Hidden,
            };
        }
        **text = format!(
            "FPS {:.0}\nchunks {}\nping {} ms\ncorrections/s {}\ndepth {} m",
            stats.fps, stats.chunks, stats.ping, stats.corrections, stats.depth
        );
    }
}

/// Renders the first-hour objective list.
pub fn update_objectives(
    tech: Res<crate::net_client::ClientTech>,
    mut texts: Query<&mut Text, With<ObjectiveText>>,
) {
    let line = tech.objective_line();
    for mut text in texts.iter_mut() {
        **text = line.clone();
    }
}
