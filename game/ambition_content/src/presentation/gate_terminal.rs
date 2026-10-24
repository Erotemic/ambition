//! Ambition's gate terminal: the panel a traveller dials from.
//!
//! Draws [`TerminalModel`] as a Bevy UI panel: the ring's name and its upkeep
//! note, the address entered as chevron glyphs, the status line, the tabs
//! (Known, Recent, Manual) and their rows. Reads the shared menu input
//! (`MenuControlFrame`: keyboard, gamepad, touch buttons), the letter keys of
//! the chevron alphabet, and presses on the panel's buttons (mouse and touch),
//! and sends what they ask through the host intent ledger. While the pause
//! menu or another surface owns the seat, the terminal reads nothing.

use ambition_platformer2d::actors::session::host_intents::HostIntentWriter;
use ambition_platformer2d::world::rooms::{GateNetworkCatalog, GateNetworkState, GateTerminalIntent};
use ambition_render::ui_fonts::{UiFontWeight, UiFonts};
use bevy::prelude::*;

use crate::gate_terminal::{
    chevron_glyph, intent, known_addresses, TerminalCell, TerminalEffect, TerminalModel, TerminalTab, TerminalUi,
    GATE_TERMINAL_CONTEXT,
};

/// The panel's root. Rebuilt whole when the model changes.
#[derive(Component)]
struct GateTerminalRoot;

/// A button of the panel, by its place in the model's rows.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct GateTerminalButton {
    row: usize,
    col: usize,
}

/// The model the panel shows now, to rebuild only on a change.
#[derive(Resource, Default)]
struct ShownTerminal(Option<TerminalModel>);

const PANEL: Color = Color::srgb(0.08, 0.07, 0.06);
const PANEL_EDGE: Color = Color::srgb(0.46, 0.37, 0.25);
const INK: Color = Color::srgb(0.96, 0.92, 0.82);
const DIM_INK: Color = Color::srgb(0.70, 0.64, 0.54);
const AMBER: Color = Color::srgb(1.0, 0.62, 0.24);
const REFUSED: Color = Color::srgb(1.0, 0.42, 0.36);
const CELL: Color = Color::srgb(0.20, 0.15, 0.10);
const CELL_SELECTED: Color = Color::srgb(0.46, 0.30, 0.12);
const CELL_EDGE: Color = Color::srgb(0.59, 0.47, 0.31);

/// Install the terminal panel. Visible builds only.
pub(super) fn install(app: &mut App) {
    // Both sleep while no terminal is open. Each runs once more after a
    // terminal closes: the drive to forget its state, the draw to take its
    // panel down.
    app.init_resource::<ShownTerminal>().add_systems(
        Update,
        (
            drive_gate_terminal.run_if(
                |state: Option<Res<GateNetworkState>>, ui: Res<TerminalUi>| {
                    state.is_some_and(|state| state.terminal.is_some()) || ui.ring.is_some()
                },
            ),
            draw_gate_terminal.run_if(|state: Option<Res<GateNetworkState>>, shown: Res<ShownTerminal>| {
                state.is_some_and(|state| state.terminal.is_some()) || shown.0.is_some()
            }),
        )
            .chain()
            .after(ambition_input::InputSet::ResolveContext),
    );
}

/// Whether the terminal owns the primary seat's input this frame.
fn terminal_owns_input(seats: Option<&ambition_input::participant::SeatInputContexts>) -> bool {
    seats.is_none_or(|seats| seats.for_seat(0).owner() == Some(GATE_TERMINAL_CONTEXT))
}

/// The letter keys of the alphabet, as the symbols they type.
const LETTER_KEYS: [(KeyCode, char); 11] = [
    (KeyCode::KeyA, 'A'),
    (KeyCode::KeyC, 'C'),
    (KeyCode::KeyD, 'D'),
    (KeyCode::KeyE, 'E'),
    (KeyCode::KeyG, 'G'),
    (KeyCode::KeyI, 'I'),
    (KeyCode::KeyL, 'L'),
    (KeyCode::KeyN, 'N'),
    (KeyCode::KeyS, 'S'),
    (KeyCode::KeyT, 'T'),
    (KeyCode::KeyY, 'Y'),
];

/// Read every input road into the terminal's state, and send what a press
/// asks. One press is one action: a letter key that also navigates a menu
/// (W, A, S, D, Q, E) types its symbol and does not move the cursor.
#[allow(clippy::too_many_arguments)]
fn drive_gate_terminal(
    catalog: Option<Res<GateNetworkCatalog>>,
    state: Option<Res<GateNetworkState>>,
    save: Option<Res<ambition_platformer2d::persistence::save::AmbitionGameSave>>,
    mut ui: ResMut<TerminalUi>,
    mut menu: Option<ResMut<ambition_input::MenuControlFrame>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    seats: Option<Res<ambition_input::participant::SeatInputContexts>>,
    buttons: Query<(&Interaction, &GateTerminalButton), Changed<Interaction>>,
    mut intents: HostIntentWriter<GateTerminalIntent>,
) {
    let (Some(catalog), Some(state)) = (catalog, state) else {
        return;
    };
    let Some(ring) = state.terminal.clone() else {
        if ui.ring.is_some() {
            *ui = TerminalUi::default();
        }
        return;
    };
    let knows = |flag: &str| save.as_ref().is_some_and(|save| save.data().flag(flag));
    let known = known_addresses(&catalog, &ring, knows);
    if ui.ring.as_deref() != Some(ring.as_str()) {
        let seen = state.reports.get(&ring).map(|report| report.serial);
        ui.open(&ring, known.len(), seen);
    }
    // A dial that answered with a link leaves the terminal, so the traveller
    // can walk into the ring.
    if let Some(report) = state.reports.get(&ring) {
        if ui.seen_serial != Some(report.serial) {
            ui.seen_serial = Some(report.serial);
            if report.outcome.is_ok() {
                intents.write(intent(&ring, ambition_platformer2d::world::rooms::GateTerminalAction::Close));
                return;
            }
        }
    }
    if !terminal_owns_input(seats.as_deref()) {
        return;
    }
    let rows = ui.rows(&catalog, &state, &known);
    ui.clamp(&rows);
    let mut effects = Vec::new();
    // Mouse and touch: a press on a button is a press on its cell.
    for (interaction, button) in &buttons {
        if *interaction == Interaction::Pressed {
            if let Some(cell) = rows.get(button.row).and_then(|row| row.get(button.col)) {
                ui.cursor = (button.row, button.col);
                effects.push(ui.activate(cell));
            }
        }
    }
    // The letter keys type their symbol.
    let mut typed = false;
    if let Some(keys) = keys.as_deref() {
        for (key, symbol) in LETTER_KEYS {
            if keys.just_pressed(key) {
                ui.type_symbol(symbol);
                typed = true;
            }
        }
    }
    if let Some(menu) = menu.as_deref_mut() {
        if typed {
            menu.consume_nav_edges();
        }
        let dx = i32::from(menu.right) - i32::from(menu.left);
        let dy = i32::from(menu.down) - i32::from(menu.up);
        if dx != 0 || dy != 0 {
            ui.navigate(&rows, dx, dy);
        }
        if menu.page_left || menu.page_right {
            let index = TerminalTab::ALL.iter().position(|tab| *tab == ui.tab).unwrap_or(0) as i32;
            let step = i32::from(menu.page_right) - i32::from(menu.page_left);
            ui.tab = TerminalTab::ALL[(index + step).rem_euclid(TerminalTab::ALL.len() as i32) as usize];
        }
        if menu.select {
            let (row, col) = ui.cursor;
            if let Some(cell) = rows.get(row).and_then(|row| row.get(col)).cloned() {
                effects.push(ui.activate(&cell));
            }
        }
        if menu.back {
            effects.push(ui.back());
        }
        menu.consume_nav_edges();
    }
    for effect in effects {
        if let TerminalEffect::Send(action) = effect {
            intents.write(intent(&ring, action));
        }
    }
}

/// Draw the terminal panel from the model, rebuilt only when it changed.
#[allow(clippy::too_many_arguments)]
fn draw_gate_terminal(
    mut commands: Commands,
    catalog: Option<Res<GateNetworkCatalog>>,
    state: Option<Res<GateNetworkState>>,
    save: Option<Res<ambition_platformer2d::persistence::save::AmbitionGameSave>>,
    ui: Res<TerminalUi>,
    mut shown: ResMut<ShownTerminal>,
    roots: Query<Entity, With<GateTerminalRoot>>,
    ui_fonts: Option<Res<UiFonts>>,
) {
    let model = match (catalog, state, ui.ring.as_deref()) {
        (Some(catalog), Some(state), Some(ring)) if state.terminal.as_deref() == Some(ring) => {
            let knows = |flag: &str| save.as_ref().is_some_and(|save| save.data().flag(flag));
            let known = known_addresses(&catalog, ring, knows);
            Some(ui.model(&catalog, &state, &known))
        }
        _ => None,
    };
    if shown.0 == model {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    shown.0 = model.clone();
    let Some(model) = model else {
        return;
    };
    let font = |size: f32, weight: UiFontWeight| {
        ui_fonts.as_deref().map(|fonts| fonts.text_font(size, weight)).unwrap_or(TextFont {
            font_size: FontSize::Px(size),
            ..default()
        })
    };
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                padding: UiRect::all(Val::Px(12.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ZIndex(46),
            GateTerminalRoot,
            Name::new("Gate Terminal Root"),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(96.0),
                    max_width: Val::Px(760.0),
                    padding: UiRect::all(Val::Px(14.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    border: UiRect::all(Val::Px(4.0)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderColor::all(PANEL_EDGE),
                Name::new("Gate Terminal Panel"),
            ))
            .with_children(|panel| {
                panel.spawn((Text::new(model.title.clone()), font(22.0, UiFontWeight::Semibold), TextColor(AMBER)));
                panel.spawn((
                    Text::new(format!("{} — {}", model.facility, model.note)),
                    font(14.0, UiFontWeight::Regular),
                    TextColor(DIM_INK),
                ));
                let chevrons: String = if model.address.is_empty() {
                    "· · · · · · ·".to_string()
                } else {
                    model.address.chars().map(chevron_glyph).map(String::from).collect::<Vec<_>>().join(" ")
                };
                panel.spawn((
                    Text::new(format!("{chevrons}    {}", model.address)),
                    font(28.0, UiFontWeight::Semibold),
                    TextColor(INK),
                ));
                panel.spawn((
                    Text::new(model.status.clone()),
                    font(16.0, UiFontWeight::Regular),
                    TextColor(if model.refused { REFUSED } else { AMBER }),
                ));
                for (row_index, row) in model.rows.iter().enumerate() {
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: Val::Px(6.0),
                            row_gap: Val::Px(6.0),
                            ..default()
                        })
                        .with_children(|line| {
                            for (col, cell) in row.iter().enumerate() {
                                let selected = model.cursor == (row_index, col);
                                let active_tab = matches!(cell, TerminalCell::Tab(tab) if *tab == model.tab);
                                spawn_cell(line, cell, row_index, col, selected, active_tab, &font);
                            }
                        });
                }
                panel.spawn((
                    Text::new("Select: press  ·  Back: erase / leave  ·  Q / E: tabs  ·  letters type chevrons"),
                    font(12.0, UiFontWeight::Regular),
                    TextColor(DIM_INK),
                ));
            });
        });
}

fn spawn_cell(
    parent: &mut ChildSpawnerCommands,
    cell: &TerminalCell,
    row: usize,
    col: usize,
    selected: bool,
    active_tab: bool,
    font: &impl Fn(f32, UiFontWeight) -> TextFont,
) {
    let (label, wide) = match cell {
        TerminalCell::Tab(tab) => (tab.label().to_string(), false),
        TerminalCell::Entry { address, label } => (format!("{address}   {label}"), true),
        TerminalCell::Symbol(symbol) => (format!("{}\n{symbol}", chevron_glyph(*symbol)), false),
        TerminalCell::Erase => ("Erase".to_string(), false),
        TerminalCell::Dial => ("DIAL".to_string(), false),
        TerminalCell::Hangup => ("Hang up".to_string(), false),
        TerminalCell::Close => ("Leave".to_string(), false),
    };
    let fill = if selected {
        CELL_SELECTED
    } else if active_tab {
        Color::srgb(0.30, 0.22, 0.12)
    } else {
        CELL
    };
    parent
        .spawn((
            Button,
            Node {
                min_width: Val::Px(if wide { 420.0 } else { 52.0 }),
                min_height: Val::Px(40.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(if selected { AMBER } else { CELL_EDGE }),
            GateTerminalButton { row, col },
            Name::new(format!("Gate terminal cell {row},{col}")),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                font(if matches!(cell, TerminalCell::Symbol(_)) { 18.0 } else { 16.0 }, UiFontWeight::Semibold),
                TextColor(INK),
                TextLayout::new(Justify::Center, LineBreak::WordBoundary),
            ));
        });
}
