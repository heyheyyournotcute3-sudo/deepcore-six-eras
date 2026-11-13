//! egui screens: inventory, crafting, machine UI, tech tree, pause and menu.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use deepcore_shared::content::ContentRegistry;
use deepcore_shared::inventory::{HOTBAR_SLOTS, INVENTORY_SLOTS};
use deepcore_shared::protocol::*;

use crate::hud::HudInventory;
use crate::world::ClientWorld;

/// Which screen is open.
#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
pub enum UiScreen {
    /// Playing, no overlay.
    #[default]
    None,
    /// Inventory and backpack.
    Inventory,
    /// Machine slots.
    Machine,
    /// Tech tree graph.
    Tech,
    /// Pause menu.
    Pause,
}

/// UI state shared between systems.
#[derive(Resource, Default)]
pub struct UiState {
    /// Currently open screen.
    pub screen: UiScreen,
    /// Machine whose UI is open.
    pub machine: Option<BlockPos>,
    /// Latest machine state from the server.
    pub machine_state: Option<MachineUiState>,
    /// Chat log lines.
    pub chat: Vec<String>,
    /// Chat input buffer.
    pub chat_input: String,
    /// True while typing chat.
    pub typing: bool,
}

/// Block position alias for the UI module.
pub use deepcore_shared::voxel::BlockPos;

/// Registers the egui plugin.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // Register the intent and event types this module reads or writes so
        // the systems validate even when no network layer is present (host
        // mode, or a client that has not connected yet).
        app.add_message::<MoveItems>()
            .add_message::<Craft>()
            .add_message::<Chat>()
            .add_message::<ResearchDeposit>()
            .add_message::<SetMachineRecipe>()
            .add_message::<MachineUiState>()
            .add_message::<ChatLine>()
            .add_message::<InventorySnapshot>()
            .add_plugins(EguiPlugin::default())
            .init_resource::<UiState>()
            // UI drawing must run in `EguiPrimaryContextPass`: that is the
            // schedule bevy_egui runs between `begin_pass` and `end_pass`.
            .add_systems(
                Update,
                (collect_machine_state, collect_chat, toggle_screens).chain(),
            )
            .add_systems(EguiPrimaryContextPass, draw_ui);
    }
}

fn toggle_screens(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<UiState>,
    mut capture: ResMut<CaptureRequest>,
) {
    if keys.just_pressed(KeyCode::KeyE) || keys.just_pressed(KeyCode::Tab) {
        state.screen = match state.screen {
            UiScreen::Inventory => UiScreen::None,
            _ => UiScreen::Inventory,
        };
    }
    if keys.just_pressed(KeyCode::KeyT) {
        state.screen = match state.screen {
            UiScreen::Tech => UiScreen::None,
            _ => UiScreen::Tech,
        };
    }
    if keys.just_pressed(KeyCode::Escape) {
        state.screen = match state.screen {
            UiScreen::Pause => UiScreen::None,
            _ => UiScreen::Pause,
        };
    }
    if keys.just_pressed(KeyCode::KeyY) {
        state.typing = !state.typing;
        if !state.typing {
            state.chat_input.clear();
        }
    }
    capture.want_capture = state.screen == UiScreen::None && !state.typing;
}

/// Set by the UI to tell the player controller whether to grab the mouse.
#[derive(Resource)]
pub struct CaptureRequest {
    /// True when the game should capture the mouse.
    pub want_capture: bool,
}

impl Default for CaptureRequest {
    fn default() -> Self {
        Self { want_capture: true }
    }
}

/// Runs exactly one egui pass per frame and draws whichever screen is open.
///
/// A single pass is required: `EguiContexts::ctx_mut()` starts and ends an
/// egui frame, and starting a second one in the same frame drops the previous
/// frame's texture deltas.
fn draw_ui(
    mut contexts: EguiContexts,
    state: Res<UiState>,
    hud: Res<HudInventory>,
    world: Res<ClientWorld>,
    tech: Res<crate::net_client::ClientTech>,
    mut intents: IntentWriters,
) -> Result {
    if state.screen == UiScreen::None && !state.typing && state.chat.is_empty() {
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    let reg = world.registry();
    match state.screen {
        UiScreen::Inventory => {
            inventory_window(ctx, reg, &hud, &mut intents.moves, &mut intents.craft)
        }
        UiScreen::Machine => {
            if let Some(ms) = &state.machine_state {
                machine_window(ctx, reg, ms, &mut intents.moves);
            }
        }
        UiScreen::Tech => tech_window(ctx, reg, &tech, &mut intents.deposits),
        UiScreen::Pause => pause_window(ctx),
        UiScreen::None => {}
    }
    if state.typing || !state.chat.is_empty() {
        let mut input = std::mem::take(&mut state.chat_input.clone());
        chat_window(ctx, &state.chat, &mut input, state.typing, &mut intents.chat);
    }
    Ok(())
}

/// The intent writers the UI needs, grouped so the entry point stays small.
#[derive(bevy::ecs::system::SystemParam)]
pub struct IntentWriters<'w> {
    /// Inventory moves.
    pub moves: MessageWriter<'w, MoveItems>,
    /// Hand crafting.
    pub craft: MessageWriter<'w, Craft>,
    /// Research deliveries.
    pub deposits: MessageWriter<'w, ResearchDeposit>,
    /// Chat lines.
    pub chat: MessageWriter<'w, Chat>,
}

fn inventory_window(
    ctx: &egui::Context,
    reg: &ContentRegistry,
    hud: &HudInventory,
    moves: &mut MessageWriter<MoveItems>,
    craft: &mut MessageWriter<Craft>,
) {
    egui::Window::new("Backpack")
        .default_pos([40.0, 60.0])
        .default_width(420.0)
        .show(ctx, |ui| {
            ui.label(format!(
                "Slots: {} of {INVENTORY_SLOTS}",
                hud.slots.iter().flatten().count()
            ));
            ui.horizontal_wrapped(|ui| {
                for (i, slot) in hud.slots.iter().enumerate() {
                    let label = match slot {
                        Some(s) => format!("{}\n×{}", short_name(reg, s.item), s.count),
                        None => format!("{i}\n-"),
                    };
                    let mut button = ui.add_sized([64.0, 48.0], egui::Button::new(label));
                    if i as u8 == hud.selected {
                        button = button.highlight();
                    }
                    if button.clicked() {
                        let to_hotbar = i >= HOTBAR_SLOTS;
                        moves.write(MoveItems {
                            from: i as i16,
                            to: if to_hotbar { 0 } else { HOTBAR_SLOTS as i16 },
                            count: 0,
                            machine: None,
                            from_machine: false,
                        });
                    }
                }
            });
            ui.separator();
            ui.heading("Hand crafting");
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| {
                    for r in reg.recipes().iter().filter(|r| r.hand) {
                        let can = r.inputs.iter().all(|(id, n)| {
                            reg.item(id)
                                .map(|i| count_item(&hud.slots, i) >= *n)
                                .unwrap_or(false)
                        });
                        let inputs: Vec<String> = r
                            .inputs
                            .iter()
                            .map(|(id, n)| format!("{n}× {}", id.replace('_', " ")))
                            .collect();
                        ui.horizontal(|ui| {
                            ui.label(format!(
                                "{}  ({})",
                                r.id.replace('_', " "),
                                inputs.join(", ")
                            ));
                            if ui.add_enabled(can, egui::Button::new("Craft")).clicked()
                                && let Some(rid) = reg.recipe(&r.id) {
                                    craft.write(Craft {
                                        recipe: rid,
                                        count: 1,
                                    });
                                }
                        });
                    }
                });
        });
}

fn machine_window(
    ctx: &egui::Context,
    reg: &ContentRegistry,
    ms: &MachineUiState,
    moves: &mut MessageWriter<MoveItems>,
) {
    let title = reg
        .machine_def(ms.kind)
        .map(|d| d.name.clone())
        .unwrap_or_else(|| "Machine".into());
    egui::Window::new(title)
        .default_pos([300.0, 120.0])
        .show(ctx, |ui| {
            ui.label(format!("Status: {}", ms.idle));
            ui.add(egui::ProgressBar::new(ms.progress).show_percentage());
            ui.separator();
            ui.label("Inputs");
            ui.horizontal_wrapped(|ui| {
                for (i, slot) in ms.inputs.iter().enumerate() {
                    if ui
                        .add_sized([56.0, 40.0], egui::Button::new(slot_label(reg, *slot)))
                        .clicked()
                    {
                        moves.write(MoveItems {
                            from: i as i16,
                            to: 0,
                            count: 0,
                            machine: Some(ms.pos),
                            from_machine: true,
                        });
                    }
                }
            });
            if let Some(fuel) = ms.fuel {
                ui.label(format!("Fuel: {}", slot_label(reg, Some(fuel))));
            }
            ui.separator();
            ui.label("Outputs");
            ui.horizontal_wrapped(|ui| {
                for (i, slot) in ms.outputs.iter().enumerate() {
                    if ui
                        .add_sized([56.0, 40.0], egui::Button::new(slot_label(reg, *slot)))
                        .clicked()
                    {
                        moves.write(MoveItems {
                            from: i as i16,
                            to: 0,
                            count: 0,
                            machine: Some(ms.pos),
                            from_machine: true,
                        });
                    }
                }
            });
            if ms.temperature > 0.0 {
                ui.separator();
                ui.label(format!("Core temperature: {:.0} °C", ms.temperature));
                ui.label(format!(
                    "Control rods: {:.0}% inserted",
                    ms.rod_insertion * 100.0
                ));
            }
        });
}

fn tech_window(
    ctx: &egui::Context,
    reg: &ContentRegistry,
    tech: &crate::net_client::ClientTech,
    deposits: &mut MessageWriter<ResearchDeposit>,
) {
    egui::Window::new("Tech Tree")
        .default_width(560.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for era in 0..=5u8 {
                    ui.heading(format!("Era {era}"));
                    for node in reg.tech_nodes().iter().filter(|t| t.era == era) {
                        let id = reg.tech(&node.id);
                        let unlocked = id.map(|i| tech.unlocked.contains(&i)).unwrap_or(false);
                        let parents_ok = node.parents.iter().all(|p| {
                            reg.tech(p)
                                .map(|i| tech.unlocked.contains(&i))
                                .unwrap_or(true)
                        });
                        let label = if unlocked {
                            format!("✔ {}", node.name)
                        } else if parents_ok {
                            format!("▸ {}", node.name)
                        } else {
                            format!("🔒 {}", node.name)
                        };
                        ui.horizontal(|ui| {
                            ui.label(label);
                            if !unlocked && parents_ok {
                                let cost: Vec<String> = node
                                    .cost
                                    .iter()
                                    .map(|(i, n)| format!("{n}× {}", i.replace('_', " ")))
                                    .collect();
                                ui.label(format!("needs {}", cost.join(", ")));
                                if ui.button("Research").clicked()
                                    && let Some(tid) = id {
                                        deposits.write(ResearchDeposit { node: tid });
                                    }
                            }
                        });
                        ui.label(egui::RichText::new(&node.description).small().weak());
                    }
                    ui.separator();
                }
            });
        });
}

fn pause_window(ctx: &egui::Context) {
    egui::Window::new("Paused").show(ctx, |ui| {
        ui.label("E or Tab — backpack");
        ui.label("T — tech tree");
        ui.label("Y — chat");
        ui.label("F3 — debug overlay");
        ui.label("Esc — resume");
    });
}

fn chat_window(
    ctx: &egui::Context,
    lines: &[String],
    input: &mut String,
    typing: bool,
    chat: &mut MessageWriter<Chat>,
) {
    egui::Window::new("Chat")
        .default_pos([40.0, 400.0])
        .show(ctx, |ui| {
            for line in lines.iter().rev().take(8) {
                ui.label(line);
            }
            if typing {
                let response = ui.text_edit_singleline(input);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let text = std::mem::take(input);
                    if !text.trim().is_empty() {
                        chat.write(Chat { text });
                    }
                }
            }
        });
}

fn collect_machine_state(mut states: MessageReader<MachineUiState>, mut ui: ResMut<UiState>) {
    for s in states.read() {
        ui.machine = Some(s.pos);
        ui.machine_state = Some(s.clone());
        ui.screen = UiScreen::Machine;
    }
}

fn collect_chat(mut lines: MessageReader<ChatLine>, mut ui: ResMut<UiState>) {
    for line in lines.read() {
        ui.chat.push(format!("{}: {}", line.from, line.text));
    }
    let len = ui.chat.len();
    if len > 64 {
        ui.chat.drain(0..len - 64);
    }
}

fn short_name(reg: &ContentRegistry, item: deepcore_shared::content::ItemId) -> String {
    reg.item_name(item).chars().take(9).collect()
}

fn slot_label(
    reg: &ContentRegistry,
    slot: Option<deepcore_shared::inventory::ItemStack>,
) -> String {
    match slot {
        Some(s) => format!("{}\n×{}", short_name(reg, s.item), s.count),
        None => "-".to_string(),
    }
}

fn count_item(
    slots: &[Option<deepcore_shared::inventory::ItemStack>],
    item: deepcore_shared::content::ItemId,
) -> u32 {
    slots
        .iter()
        .flatten()
        .filter(|s| s.item == item)
        .map(|s| s.count)
        .sum()
}
