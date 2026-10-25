//! Ambition's gate terminal: the dial a traveller uses.
//!
//! Draws [`TerminalModel`] as a Bevy UI panel: on the left the WHEEL, the
//! ring's fifteen glyphs round a bronze ring laid out as the ring is (the
//! first at the lower left, Δ at the top), the engage press (a big solid Δ)
//! in the slice at the bottom, and the address entered in the middle; on the
//! right the ring's name, the status line, and the known and recent
//! addresses (Jon's concept, 2026-10-24).
//!
//! Input, one road for every device:
//!
//! - a gamepad points its left stick at a chevron (or down at the Δ) and
//!   presses select to enter it; back takes the last chevron back, then
//!   leaves; the page buttons swap to the list and back;
//! - the arrow keys and the d-pad step round the wheel (left and right), jump
//!   to the top (up) or to the Δ (down);
//! - a mouse or a finger points by hovering and presses a chevron, the Δ or a
//!   row of the list.
//!
//! There is no typing: a letter can stand on several chevrons (Λ, Λ₂, Λ₃), and
//! only the wheel tells them apart. While the pause menu or another surface
//! owns the seat, the terminal reads nothing. The chevrons being entered light
//! on the ring (`GateDialPreview`).

use ambition_platformer2d::render::rendering::gate_portal_visuals::GateDialPreview;
use ambition_platformer2d::world::rooms::{
    position_grid, Chevron, GateNetworkCatalog, GateNetworkState, GateTerminalIntent, GRID_BITS,
};
use ambition_platformer2d_shared_tangle::host_intents::HostIntentWriter;
use ambition_render::ui_fonts::{UiFontWeight, UiFonts};
use bevy::prelude::*;

use crate::gate_terminal::{
    chevron_angle, intent, known_addresses, ListRow, TerminalEffect, TerminalFocus, TerminalModel, TerminalUi, LIST_ROWS,
    WheelSlot, GATE_TERMINAL_CONTEXT,
};

/// The glyphs in a row, white, for the wheel to tint (the gate target's
/// `interdimensional_gate_glyph_icons.png`).
const GLYPH_ICONS: &str = "sprites/interdimensional_gate_glyph_icons.png";
/// The side of one glyph's cell in that picture.
const GLYPH_ICON_PX: u32 = 96;

/// The wheel's box, the radius its chevrons stand on, and its ring's band.
const WHEEL: f32 = 440.0;
const WHEEL_R: f32 = 172.0;
const BAND: f32 = 70.0;
const CHEVRON_BOX: f32 = 54.0;
/// A chevron's plaque along its ray: its glyph and its position grid.
const PLAQUE_TALL: f32 = 80.0;
/// A cell of a plaque's position grid.
const GRID_CELL: f32 = 5.0;
/// How far the stick must lean to point.
const STICK_POINTS: f32 = 0.55;

const PANEL: Color = Color::srgb(0.07, 0.10, 0.13);
const PANEL_EDGE: Color = Color::srgb(0.42, 0.31, 0.20);
const BRONZE: Color = Color::srgb(0.36, 0.27, 0.19);
const BRONZE_LIGHT: Color = Color::srgb(0.69, 0.54, 0.36);
const PLAQUE: Color = Color::srgb(0.14, 0.10, 0.07);
const PLAQUE_POINTED: Color = Color::srgb(0.10, 0.27, 0.32);
/// A chevron of the address entered: lit, as the ring lights it.
const PLAQUE_LIT: Color = Color::srgb(0.16, 0.45, 0.52);
const FACE: Color = Color::srgb(0.05, 0.08, 0.10);
const GRID_EMPTY: Color = Color::srgb(0.03, 0.05, 0.06);
const CYAN: Color = Color::srgb(0.50, 0.95, 1.0);
const CYAN_DIM: Color = Color::srgb(0.30, 0.62, 0.70);
const INK: Color = Color::srgb(0.90, 0.96, 0.98);
const DIM_INK: Color = Color::srgb(0.54, 0.64, 0.68);
const REFUSED: Color = Color::srgb(1.0, 0.45, 0.40);
const ROW: Color = Color::srgb(0.10, 0.15, 0.19);
const ROW_SELECTED: Color = Color::srgb(0.13, 0.30, 0.36);
/// A row under the pointer: lighter than a row, never the cursor's colour.
const ROW_HOVERED: Color = Color::srgb(0.14, 0.20, 0.25);

/// The panel's root. Rebuilt whole when the model changes.
#[derive(Component)]
struct GateTerminalRoot;

/// A place of the wheel, as a button.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct WheelButton(WheelSlot);

/// A row of the list, as a button, by its place in the list.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct ListButton(usize);

/// The model the panel shows now, to rebuild only on a change.
#[derive(Resource, Default)]
struct ShownTerminal(Option<TerminalModel>);

/// The glyph picture and its cells.
#[derive(Resource)]
struct GlyphIcons {
    image: Handle<Image>,
    layout: Option<Handle<TextureAtlasLayout>>,
}

/// Install the terminal panel. Visible builds only.
pub(super) fn install(app: &mut App) {
    // Both sleep while no terminal is open. Each runs once more after a
    // terminal closes: the drive to forget its state, the draw to take its
    // panel down.
    app.init_resource::<ShownTerminal>()
        .init_resource::<GateDialPreview>()
        .add_systems(Startup, load_glyph_icons)
        .add_systems(
            Update,
            (
                drive_gate_terminal.run_if(|state: Option<Res<GateNetworkState>>, ui: Res<TerminalUi>| {
                    state.is_some_and(|state| state.terminal.is_some()) || ui.ring.is_some()
                }),
                draw_gate_terminal.run_if(|state: Option<Res<GateNetworkState>>, shown: Res<ShownTerminal>| {
                    state.is_some_and(|state| state.terminal.is_some()) || shown.0.is_some()
                }),
            )
                .chain()
                .after(ambition_input::InputSet::ResolveContext)
                // A READER of the menu frame, after every surface that writes
                // or consumes it: the frame is whole when the terminal reads
                // it, and no other surface's order depends on the terminal's.
                .after(ambition_platformer2d::actors::schedule::MenuFrameConsume),
        );
}

fn load_glyph_icons(mut commands: Commands, asset_server: Res<AssetServer>) {
    use ambition_sprite_sheet::game_assets::load_sheet_image;
    commands.insert_resource(GlyphIcons {
        image: load_sheet_image(&asset_server, "gate-terminal", GLYPH_ICONS),
        layout: None,
    });
}

/// Whether the terminal owns the primary seat's input this frame.
fn terminal_owns_input(seats: Option<&ambition_input::participant::SeatInputContexts>) -> bool {
    seats.is_none_or(|seats| seats.for_seat(0).owner() == Some(GATE_TERMINAL_CONTEXT))
}

/// The direction the most leaned stick points, as an angle of the wheel
/// (degrees clockwise from 3 o'clock, y down), when one leans far enough.
fn stick_angle(gamepads: &Query<&Gamepad>) -> Option<f32> {
    gamepads
        .iter()
        .map(|gamepad| gamepad.left_stick())
        .filter(|stick| stick.length() >= STICK_POINTS)
        .max_by(|a, b| a.length().total_cmp(&b.length()))
        .map(|stick| (-stick.y).atan2(stick.x).to_degrees())
}

/// Read every input road into the terminal's state, and send what a press
/// asks. The menu frame is only read: while the terminal owns the seat no
/// other surface acts on it, so there is nothing to consume.
#[allow(clippy::too_many_arguments)]
fn drive_gate_terminal(
    catalog: Option<Res<GateNetworkCatalog>>,
    state: Option<Res<GateNetworkState>>,
    save: Option<Res<ambition_platformer2d::persistence::save::AmbitionGameSave>>,
    mut ui: ResMut<TerminalUi>,
    mut preview: ResMut<GateDialPreview>,
    menu: Option<Res<ambition_input::MenuControlFrame>>,
    seats: Option<Res<ambition_input::participant::SeatInputContexts>>,
    gamepads: Query<&Gamepad>,
    wheel: Query<(&Interaction, &WheelButton), Changed<Interaction>>,
    list: Query<(&Interaction, &ListButton), Changed<Interaction>>,
    mut pointer_moves: MessageReader<bevy::window::CursorMoved>,
    mut scrolls: MessageReader<bevy::input::mouse::MouseWheel>,
    mut intents: HostIntentWriter<GateTerminalIntent>,
) {
    // A rebuilt panel puts a new button under a still pointer, and its
    // `Interaction` changes to `Hovered` although nothing moved: only a
    // pointer that moved this frame points the wheel.
    let pointer_moved = pointer_moves.read().count() > 0;
    let scrolled: i32 = scrolls.read().map(|wheel| -wheel.y.signum() as i32).sum();
    let (Some(catalog), Some(state)) = (catalog, state) else {
        return;
    };
    let Some(ring) = state.terminal.clone() else {
        if ui.ring.is_some() {
            *ui = TerminalUi::default();
        }
        if *preview != GateDialPreview::default() {
            *preview = GateDialPreview::default();
        }
        return;
    };
    if ui.ring.as_deref() != Some(ring.as_str()) {
        let seen = state.reports.get(&ring).map(|report| report.serial);
        ui.open(&ring, seen);
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
    if terminal_owns_input(seats.as_deref()) {
        let knows = |flag: &str| save.as_ref().is_some_and(|save| save.data().flag(flag));
        let known = known_addresses(&catalog, &ring, knows);
        let rows = ui.list(&catalog, &state, &known);
        let count = catalog.chevrons();
        let mut effects = Vec::new();
        // Mouse and touch: on the wheel a moving pointer points and a press
        // presses. On the list a hover is drawn and never moves the cursor
        // (Q70: it would undo every arrow key, since the list is rebuilt
        // under the pointer); a press selects and activates.
        for (interaction, button) in &wheel {
            match interaction {
                Interaction::Hovered if pointer_moved => ui.point(button.0),
                Interaction::Pressed => effects.push(ui.press_slot(button.0)),
                _ => {}
            }
        }
        for (interaction, button) in &list {
            if let Some(row) = rows.get(button.0) {
                match interaction {
                    Interaction::Hovered => ui.list_hover = Some(button.0),
                    Interaction::Pressed => {
                        ui.focus = TerminalFocus::List;
                        ui.list_cursor = button.0;
                        effects.push(ui.press_row(row));
                    }
                    Interaction::None => {
                        if ui.list_hover == Some(button.0) {
                            ui.list_hover = None;
                        }
                    }
                }
            }
        }
        // A mouse wheel or a drag over the screen scrolls the list's window,
        // not its cursor.
        let drag = menu.as_deref().map_or(0, |menu| -(menu.drag_scroll_y.round() as i32));
        if scrolled + drag != 0 {
            ui.scroll_list(scrolled + drag, rows.len());
        }
        // The stick points the wheel while the wheel has the focus.
        let leaning = ui.focus == TerminalFocus::Wheel && {
            let angle = stick_angle(&gamepads);
            if let Some(angle) = angle {
                ui.point(WheelSlot::at_angle(angle, count));
            }
            angle.is_some()
        };
        if let Some(menu) = menu.as_deref() {
            if menu.page_left || menu.page_right {
                ui.swap_focus();
            } else {
                match ui.focus {
                    TerminalFocus::Wheel => {
                        if !leaning {
                            if menu.left {
                                ui.step_wheel(-1, count);
                            }
                            if menu.right {
                                ui.step_wheel(1, count);
                            }
                            if menu.up {
                                ui.point(WheelSlot::at_angle(270.0, count));
                            }
                            if menu.down {
                                ui.point(WheelSlot::Engage);
                            }
                        }
                        if menu.select {
                            let slot = ui.pointed.unwrap_or(WheelSlot::at_angle(270.0, count));
                            effects.push(ui.press_slot(slot));
                        }
                    }
                    TerminalFocus::List => {
                        let step = i32::from(menu.down) - i32::from(menu.up);
                        if step != 0 {
                            ui.step_list(step, rows.len());
                        }
                        if menu.left {
                            ui.focus = TerminalFocus::Wheel;
                        }
                        if menu.select {
                            if let Some(row) = rows.get(ui.list_cursor).cloned() {
                                effects.push(ui.press_row(&row));
                            }
                        }
                    }
                }
            }
            if menu.back {
                effects.push(ui.back());
            }
        }
        for effect in effects {
            if let TerminalEffect::Send(action) = effect {
                intents.write(intent(&ring, action));
            }
        }
    }
    let shown = GateDialPreview { ring: Some(ring), chevrons: ui.entered.clone() };
    if *preview != shown {
        *preview = shown;
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
    icons: Option<ResMut<GlyphIcons>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let model = match (catalog.as_deref(), state, ui.ring.as_deref()) {
        (Some(catalog), Some(state), Some(ring)) if state.terminal.as_deref() == Some(ring) => {
            let knows = |flag: &str| save.as_ref().is_some_and(|save| save.data().flag(flag));
            let known = known_addresses(catalog, ring, knows);
            Some(ui.model(catalog, &state, &known))
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
    let (Some(model), Some(mut icons)) = (model, icons) else {
        return;
    };
    let count = model.inscription.len();
    let layout = icons
        .layout
        .get_or_insert_with(|| {
            layouts.add(TextureAtlasLayout::from_grid(UVec2::splat(GLYPH_ICON_PX), count as u32, 1, None, None))
        })
        .clone();
    let glyphs = Glyphs { image: icons.image.clone(), layout };
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
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
            ZIndex(46),
            GateTerminalRoot,
            Name::new("Gate Terminal Root"),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    max_width: Val::Px(1060.0),
                    max_height: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(18.0)),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(24.0),
                    row_gap: Val::Px(12.0),
                    border: UiRect::all(Val::Px(4.0)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderColor::all(PANEL_EDGE),
                Name::new("Gate Terminal Panel"),
            ))
            .with_children(|panel| {
                spawn_wheel(panel, &model, &glyphs, &font);
                spawn_side(panel, &model, &glyphs, &font);
            });
        });
}

/// The glyph picture and its cells, for one rebuild.
struct Glyphs {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
}

impl Glyphs {
    /// Chevron `chevron`'s glyph, `size` pixels square, tinted, turned
    /// clockwise by `turn` degrees.
    fn node(&self, chevron: Chevron, size: f32, tint: Color, turn: f32) -> impl Bundle {
        (
            ImageNode::from_atlas_image(self.image.clone(), TextureAtlas { layout: self.layout.clone(), index: chevron.into() })
                .with_color(tint),
            Node { width: Val::Px(size), height: Val::Px(size), ..default() },
            UiTransform::from_rotation(Rot2::degrees(turn)),
        )
    }
}

/// A node of `size` placed with its centre at (x, y) of its parent.
fn placed(x: f32, y: f32, w: f32, h: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(x - w / 2.0),
        top: Val::Px(y - h / 2.0),
        width: Val::Px(w),
        height: Val::Px(h),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    }
}

fn spawn_wheel(
    panel: &mut ChildSpawnerCommands,
    model: &TerminalModel,
    glyphs: &Glyphs,
    font: &impl Fn(f32, UiFontWeight) -> TextFont,
) {
    let count = model.inscription.len();
    let c = WHEEL / 2.0;
    let wheel_focus = model.focus == TerminalFocus::Wheel;
    panel
        .spawn((Node { width: Val::Px(WHEEL), height: Val::Px(WHEEL), flex_shrink: 0.0, ..default() }, Name::new("Gate dial wheel")))
        .with_children(|wheel| {
            // The bronze ring, and the dark face inside it.
            let outer = WHEEL_R + BAND / 2.0;
            wheel.spawn((
                Node { border: UiRect::all(Val::Px(BAND)), border_radius: BorderRadius::MAX, ..placed(c, c, 2.0 * outer, 2.0 * outer) },
                BorderColor::all(BRONZE),
            ));
            let face = WHEEL_R - BAND / 2.0;
            wheel
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        border: UiRect::all(Val::Px(3.0)),
                        border_radius: BorderRadius::MAX,
                        ..placed(c, c, 2.0 * face, 2.0 * face)
                    },
                    BackgroundColor(FACE),
                    BorderColor::all(BRONZE_LIGHT),
                ))
                .with_children(|middle| {
                    middle.spawn((Text::new("ADDRESS"), font(13.0, UiFontWeight::Semibold), TextColor(DIM_INK)));
                    middle
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            justify_content: JustifyContent::Center,
                            column_gap: Val::Px(2.0),
                            max_width: Val::Px(2.0 * face - 30.0),
                            min_height: Val::Px(30.0),
                            ..default()
                        })
                        .with_children(|address| {
                            for chevron in &model.entered {
                                address.spawn(glyphs.node(*chevron, 28.0, CYAN, 0.0));
                            }
                            // The place of the next chevron.
                            address.spawn((
                                Node { width: Val::Px(16.0), height: Val::Px(26.0), border: UiRect::bottom(Val::Px(3.0)), ..default() },
                                BorderColor::all(CYAN_DIM),
                            ));
                        });
                    let word = model.entered_word.clone().unwrap_or_default();
                    middle.spawn((Text::new(word), font(15.0, UiFontWeight::Semibold), TextColor(CYAN)));
                });
            // The chevrons, each foot toward the middle, as on the ring: its
            // plaque stands on its ray, the glyph inward and its position
            // grid outward, lit as the ring will light it.
            for index in 0..count {
                let angle = chevron_angle(index, count);
                let (x, y) = (c + WHEEL_R * angle.to_radians().cos(), c + WHEEL_R * angle.to_radians().sin());
                let chevron = index as Chevron;
                let slot = WheelSlot::Chevron(chevron);
                let pointed = model.pointed == Some(slot) && wheel_focus;
                let uses = model.entered.iter().filter(|used| **used == chevron).count();
                let lit = uses > 0;
                let grid = position_grid(&model.entered, chevron);
                wheel
                    .spawn((
                        Button,
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(3.0),
                            border: UiRect::all(Val::Px(2.0)),
                            border_radius: BorderRadius::all(Val::Px(8.0)),
                            ..placed(x, y, CHEVRON_BOX, PLAQUE_TALL)
                        },
                        UiTransform::from_rotation(Rot2::degrees(angle + 90.0)),
                        BackgroundColor(if lit {
                            PLAQUE_LIT
                        } else if pointed {
                            PLAQUE_POINTED
                        } else {
                            PLAQUE
                        }),
                        BorderColor::all(if pointed { INK } else if lit { CYAN } else { BRONZE_LIGHT }),
                        WheelButton(slot),
                        Name::new(format!("Gate dial chevron {index}")),
                    ))
                    .with_children(|button| {
                        button
                            .spawn(Node {
                                display: Display::Grid,
                                grid_template_columns: RepeatedGridTrack::px(GRID_BITS as u16, GRID_CELL),
                                row_gap: Val::Px(2.0),
                                column_gap: Val::Px(2.0),
                                ..default()
                            })
                            .with_children(|cells| {
                                for (row, bits) in grid.iter().enumerate() {
                                    for set in bits {
                                        let tint = if row >= uses {
                                            GRID_EMPTY
                                        } else if *set {
                                            INK
                                        } else {
                                            CYAN_DIM
                                        };
                                        cells.spawn((
                                            Node { width: Val::Px(GRID_CELL), height: Val::Px(GRID_CELL), ..default() },
                                            BackgroundColor(tint),
                                        ));
                                    }
                                }
                            });
                        button.spawn(glyphs.node(chevron, 34.0, if pointed || lit { INK } else { CYAN }, 0.0));
                    });
            }
            // The engage press, in the slice at the bottom.
            let pointed = model.pointed == Some(WheelSlot::Engage) && wheel_focus;
            let delta = model.inscription.iter().position(|letter| *letter == 'D').unwrap_or(0) as Chevron;
            wheel
                .spawn((
                    Button,
                    Node {
                        border: UiRect::all(Val::Px(3.0)),
                        border_radius: BorderRadius::all(Val::Px(14.0)),
                        ..placed(c, c + WHEEL_R + 6.0, 96.0, 84.0)
                    },
                    BackgroundColor(if pointed { PLAQUE_POINTED } else { PLAQUE }),
                    BorderColor::all(if pointed { INK } else { CYAN }),
                    WheelButton(WheelSlot::Engage),
                    Name::new("Gate dial engage"),
                ))
                .with_children(|button| {
                    button.spawn(glyphs.node(delta, 70.0, if pointed { INK } else { CYAN }, 0.0));
                });
        });
}

fn spawn_side(
    panel: &mut ChildSpawnerCommands,
    model: &TerminalModel,
    glyphs: &Glyphs,
    font: &impl Fn(f32, UiFontWeight) -> TextFont,
) {
    panel
        .spawn((
            Node {
                width: Val::Px(500.0),
                max_height: Val::Px(WHEEL + 120.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                overflow: Overflow::clip_y(),
                ..default()
            },
            Name::new("Gate dial side"),
        ))
        .with_children(|side| {
            side.spawn((Text::new(model.title.clone()), font(22.0, UiFontWeight::Semibold), TextColor(CYAN)));
            side.spawn((
                Text::new(format!("{} — {}", model.facility, model.note)),
                font(13.0, UiFontWeight::Regular),
                TextColor(DIM_INK),
            ));
            side.spawn((
                Text::new(model.status.clone()),
                font(16.0, UiFontWeight::Semibold),
                TextColor(if model.refused { REFUSED } else { INK }),
            ));
            let shown = model.list_window..(model.list_window + LIST_ROWS).min(model.list.len());
            let more = |count: usize, up: bool| {
                (
                    Text::new(format!("{} {count} more", if up { "\u{25B2}" } else { "\u{25BC}" })),
                    font(12.0, UiFontWeight::Semibold),
                    TextColor(DIM_INK),
                )
            };
            if shown.start > 0 {
                side.spawn(more(shown.start, true));
            }
            let mut heading = None;
            for (index, row) in model.list.iter().enumerate().skip(shown.start).take(shown.len()) {
                let this = match row {
                    ListRow::Address { recent: false, .. } => "KNOWN DESTINATIONS",
                    ListRow::Address { recent: true, .. } => "RECENT ADDRESSES",
                    ListRow::Hangup | ListRow::Close => "",
                };
                if heading != Some(this) {
                    heading = Some(this);
                    if !this.is_empty() {
                        side.spawn((
                            Text::new(this),
                            font(13.0, UiFontWeight::Semibold),
                            TextColor(DIM_INK),
                            Node { margin: UiRect::top(Val::Px(8.0)), ..default() },
                        ));
                    }
                }
                let selected = model.focus == TerminalFocus::List && model.list_cursor == index;
                let hovered = model.list_hover == Some(index);
                side.spawn((
                    Button,
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(10.0),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
                        min_height: Val::Px(32.0),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(if selected {
                        ROW_SELECTED
                    } else if hovered {
                        ROW_HOVERED
                    } else {
                        ROW
                    }),
                    BorderColor::all(if selected { CYAN } else { Color::NONE }),
                    ListButton(index),
                    Name::new(format!("Gate dial row {index}")),
                ))
                .with_children(|line| match row {
                    ListRow::Address { address, word, name, .. } => {
                        line.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(1.0), min_width: Val::Px(210.0), ..default() })
                            .with_children(|chevrons| {
                                for chevron in address {
                                    chevrons.spawn(glyphs.node(*chevron, 24.0, CYAN, 0.0));
                                }
                            });
                        let label = if word.is_empty() { name.clone() } else { format!("{name}  ·  {word}") };
                        line.spawn((Text::new(label), font(15.0, UiFontWeight::Regular), TextColor(INK)));
                    }
                    ListRow::Hangup => {
                        line.spawn((Text::new("Hang up"), font(15.0, UiFontWeight::Semibold), TextColor(INK)));
                    }
                    ListRow::Close => {
                        line.spawn((Text::new("Leave"), font(15.0, UiFontWeight::Semibold), TextColor(INK)));
                    }
                });
            }
            if shown.end < model.list.len() {
                side.spawn(more(model.list.len() - shown.end, false));
            }
            side.spawn((
                Text::new("Point at a chevron, press to enter it  ·  \u{0394} dials  ·  Back erases, then leaves  ·  Page: the list"),
                font(12.0, UiFontWeight::Regular),
                TextColor(DIM_INK),
                Node { margin: UiRect::top(Val::Px(8.0)), ..default() },
            ));
        });
}
