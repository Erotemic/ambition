//! Ambition's gate terminal: the dial a traveller uses.
//!
//! Draws [`TerminalModel`] as a Bevy UI panel. The terminal opens on its
//! LIST: the ring's name, the status line, and the known and recent
//! addresses, a tap or a press dialing one. The WHEEL is a view behind the
//! list's "Dial an address" row (Jon, 2026-10-25: the dial "should be
//! secondary"): the ring's fifteen glyphs round a bronze ring laid out as the
//! ring is (the first at the lower left, Δ at the top), the engage press (a
//! big solid Δ) in the slice at the bottom, and the address entered in the
//! middle (Jon's concept, 2026-10-24).
//!
//! Input, one road for every device:
//!
//! - in the list, up and down move the cursor, select dials, back leaves; a
//!   mouse or a finger taps a row (a tap dials, a drag scrolls the list, a
//!   mouse wheel scrolls it); the list shows as many rows as the screen holds;
//! - in the dial view, a gamepad points its left stick at a chevron (or down
//!   at the Δ) and presses select to enter it; the arrow keys and the d-pad
//!   step round the wheel; a moving mouse or a finger points and presses;
//!   back takes the last chevron back, then returns to the list;
//! - the page buttons swap between the list and the dial view.
//!
//! There is no typing: a letter can stand on several chevrons (Λ, Λ₂, Λ₃), and
//! only the wheel tells them apart. While the pause menu or another surface
//! owns the seat, the terminal reads nothing. The chevrons being entered light
//! on the ring (`GateDialPreview`).

use ambition_platformer2d::render::rendering::gate_portal_visuals::GateDialPreview;
use ambition_platformer2d::world::rooms::{
    position_grid, Chevron, GateNetworkCatalog, GateNetworkState, GateRingPositions, GateTerminalIntent, GRID_BITS,
};
use ambition_platformer2d_shared_tangle::host_intents::HostIntentWriter;
use ambition_platformer2d_shared_tangle::lifecycle::SpawnSessionScopedExt;
use ambition_render::ui_fonts::{UiFontWeight, UiFonts};
use bevy::prelude::*;

use crate::gate_terminal::{
    chevron_angle, intent, known_addresses, ListRow, TerminalEffect, TerminalFocus, TerminalModel, TerminalUi,
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
struct GateTerminalRoot { owner: Entity }

/// A place of the wheel, as a button.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct WheelButton(WheelSlot);

/// A row of the list, as a button, by its place in the list.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct ListButton(usize);

/// The dial view's way back to the list, for a finger.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct DialBackButton;

/// A row's height and the gap under it: a finger's target.
const ROW_TALL: f32 = 52.0;
const ROW_PITCH: f32 = ROW_TALL + 6.0;
/// The list panel's height besides its rows: the title and status, the
/// "more" marks, the hint, and the paddings; and on a short screen (a phone
/// held sideways), where the heading is one line and the hint is left out.
const LIST_CHROME: f32 = 230.0;
const COMPACT_CHROME: f32 = 120.0;
/// A screen shorter than this draws the compact list.
const COMPACT_BELOW: f32 = 560.0;

/// The model the panel shows now, to rebuild only on a change.
#[derive(Component, Default)]
struct ShownTerminal(Option<(TerminalModel, Rect)>);

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
    app.init_resource::<GateDialPreview>()
        .init_resource::<ambition_input::SeatMenuFrames>()
        .init_resource::<ambition_input::participant::SeatInputContexts>()
        .add_systems(Startup, load_glyph_icons)
        .add_systems(Update,
            (initialize_terminal_surfaces,
                drive_gate_terminal.run_if(|state: Option<Res<GateNetworkState>>, users: Query<&TerminalUi>, preview: Res<GateDialPreview>| {
                    state.is_some_and(|state| !state.terminals.is_empty()) || users.iter().any(|ui| ui.ring.is_some()) || !preview.by_ring.is_empty()
                }),
                draw_gate_terminal.run_if(|state: Option<Res<GateNetworkState>>, users: Query<&ShownTerminal>| {
                    state.is_some_and(|state| !state.terminals.is_empty()) || users.iter().any(|shown| shown.0.is_some())
                }))
                .chain()
                .after(ambition_input::InputSet::ResolveContext)
                .after(ambition_platformer2d::input::MenuFrameConsume));
}

#[derive(bevy::ecs::system::SystemParam)]
struct TerminalParticipants<'w, 's> {
    inputs: Query<'w, 's, (Entity, &'static ambition_input::participant::InputParticipant)>,
    roots: Query<'w, 's, (&'static ambition_platformer2d_shared_tangle::lifecycle::SessionRoot,
        &'static ambition_platformer2d::session::participants::SessionParticipants)>,
    scope: ambition_platformer2d_shared_tangle::lifecycle::LiveSessionScope<'w, 's>,
    local: Option<Res<'w, ambition_platformer2d::rollback::LocalPlayers>>,
}

impl TerminalParticipants<'_, '_> {
    fn contains(&self, owner: Entity) -> bool {
        let Ok((_, participant)) = self.inputs.get(owner) else { return false; };
        let slot = ambition_platformer2d::characters::control::PlayerSlot(participant.id.slot());
        self.local.as_ref().is_none_or(|local| local.0.contains(&usize::from(slot.0)))
            && self.scope.get().is_some_and(|scope| self.roots.iter().any(|(root, members)| root.0 == scope && members.contains(slot)))
    }
}

fn initialize_terminal_surfaces(
    mut commands: Commands,
    participants: TerminalParticipants,
    initialized: Query<(), With<TerminalUi>>,
    roots: Query<(Entity, &GateTerminalRoot)>,
) {
    for (entity, _) in &participants.inputs {
        match (participants.contains(entity), initialized.contains(entity)) {
            (true, false) => { commands.entity(entity).insert((TerminalUi::default(), ShownTerminal::default())); }
            (false, true) => { commands.entity(entity).remove::<(TerminalUi, ShownTerminal)>(); }
            _ => {}
        }
    }
    for (entity, root) in &roots {
        if !participants.contains(root.owner) { commands.entity(entity).despawn(); }
    }
}

#[derive(bevy::ecs::system::SystemParam)]
struct TerminalWidgets<'w, 's> {
    wheel: Query<'w, 's, (Entity, &'static Interaction, &'static WheelButton), Changed<Interaction>>,
    list: Query<'w, 's, (Entity, &'static Interaction, &'static ListButton), Changed<Interaction>>,
    back: Query<'w, 's, (Entity, &'static Interaction), (Changed<Interaction>, With<DialBackButton>)>,
    roots: Query<'w, 's, (Entity, &'static GateTerminalRoot, &'static ComputedNode)>,
    parents: Query<'w, 's, &'static ChildOf>,
}

impl TerminalWidgets<'_, '_> {
    fn owner(&self, mut entity: Entity) -> Option<Entity> {
        loop {
            if let Ok((_, root, _)) = self.roots.get(entity) { return Some(root.owner); }
            entity = self.parents.get(entity).ok()?.parent();
        }
    }
}

fn load_glyph_icons(sprite_content: bevy::prelude::Res<ambition_platformer2d::sprite_sheet::SpriteContent>, mut commands: Commands, asset_server: Res<AssetServer>) {
    use ambition_sprite_sheet::game_assets::load_sheet_image;
    commands.insert_resource(GlyphIcons {
        image: load_sheet_image(Some(&sprite_content), &asset_server, "gate-terminal", GLYPH_ICONS),
        layout: None,
    });
}

/// The pointer's state for one frame: where it is, what came up, how it
/// moved and scrolled.
#[derive(bevy::ecs::system::SystemParam)]
struct PointerInputs<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<bevy::window::PrimaryWindow>>,
    touches: Option<Res<'w, bevy::input::touch::Touches>>,
    mouse_buttons: Option<Res<'w, ButtonInput<MouseButton>>>,
    moves: MessageReader<'w, 's, bevy::window::CursorMoved>,
    scrolls: MessageReader<'w, 's, bevy::input::mouse::MouseWheel>,
}

/// Read every input road into the terminal's state, and send what a press
/// asks. The menu frame is only read: while the terminal owns the seat no
/// other surface acts on it, so there is nothing to consume.
#[allow(clippy::too_many_arguments)]
fn drive_gate_terminal(
    catalog: Option<Res<GateNetworkCatalog>>,
    positions: Option<Res<GateRingPositions>>,
    state: Option<Res<GateNetworkState>>,
    save: Option<Res<ambition_platformer2d::persistence::save::AmbitionGameSave>>,
    mut users: Query<(Entity, &ambition_input::participant::InputParticipant, &mut TerminalUi)>,
    drivers: Query<(Entity, &ambition_platformer2d::characters::control::DrivingParticipant)>,
    bodies: ambition_platformer2d_shared_tangle::lifecycle::LiveBodies,
    mut preview: ResMut<GateDialPreview>,
    frames: Res<ambition_input::SeatMenuFrames>,
    seats: Res<ambition_input::participant::SeatInputContexts>,
    widgets: TerminalWidgets,
    participants: TerminalParticipants,
    surface: ambition_render::ui_surface::ParticipantSurface,
    pointer_frame: Option<Res<ambition_input::MenuControlFrame>>,
    mut pointer: PointerInputs,
    mut intents: HostIntentWriter<GateTerminalIntent>,
) {
    // A rebuilt panel puts a new button under a still pointer, and its
    // `Interaction` changes to `Hovered` although nothing moved: only a
    // pointer that moved this frame points the wheel.
    let pointer_moved = pointer.moves.read().count() > 0;
    let wheel_steps: i32 = pointer.scrolls.read().map(|wheel| -wheel.y.signum() as i32).sum();
    let windows = &pointer.windows;
    let (touches, mouse_buttons) = (pointer.touches.as_deref(), pointer.mouse_buttons.as_deref());
    let (Some(catalog), Some(state)) = (catalog, state) else {
        return;
    };
    let mut previews = std::collections::BTreeMap::new();
    let owners: Vec<_> = participants.inputs.iter().filter(|(entity, _)| participants.contains(*entity)).map(|(_, participant)| participant.id).collect();
    let pointer_position = ambition_ui_nav::pointer_position(touches, windows.single().ok().and_then(Window::cursor_position));
    let mut ordered: Vec<_> = users.iter_mut().collect();
    ordered.sort_by_key(|(_, participant, _)| participant.id.slot());
    for (owner, participant, mut ui) in ordered {
        let subject = crate::gate_terminal::terminal_subject(participant.id, &drivers, &bodies);
        let ring = subject.as_ref().and_then(|subject| state.terminals.get(subject)).cloned();
        let Some((subject, ring)) = subject.zip(ring) else {
            *ui = TerminalUi::default();
            continue;
        };
        if ui.subject.as_ref() != Some(&subject) {
            *ui = TerminalUi { subject: Some(subject.clone()), ..default() };
        }
        let menu = Some(frames.for_seat(participant.id.slot()));
        let over_surface = pointer_position.is_some_and(|position| surface.menu_rect(participant.id, &owners).is_some_and(|region| region.contains(position)));
        let scrolled = if over_surface { wheel_steps } else { 0 };
        if ui.ring.as_deref() != Some(ring.as_str()) {
            let seen = state.report_for(&subject, &ring).map(|report| report.serial);
            ui.open(&ring, seen);
            // A portal arch is a list of places, not an address to spell: it opens
            // on its list.
            if catalog.ring(&ring).is_some_and(|ring| ring.kind == ambition_platformer2d::world::rooms::RingKind::Portal) {
                ui.focus = crate::gate_terminal::TerminalFocus::List;
            }
        }
        // A dial that answered with a link leaves the terminal, so the traveller
        // can walk into the ring.
        if let Some(report) = state.report_for(&subject, &ring) {
            if ui.seen_serial != Some(report.serial) {
                ui.seen_serial = Some(report.serial);
                if report.outcome.is_ok() {
                    intents.write(intent(&subject, &ring, ambition_platformer2d::world::rooms::GateTerminalAction::Close));
                    continue;
                }
            }
        }
        if seats.for_seat(participant.id.slot()).owner() == Some(GATE_TERMINAL_CONTEXT) {
            let knows = |flag: &str| save.as_ref().is_some_and(|save| save.data().flag(flag));
            let none = GateRingPositions::default();
            let known = known_addresses(&catalog, positions.as_deref().unwrap_or(&none), &ring, knows);
            let rows = ui.list(&catalog, &state, &known);
            let count = catalog.chevrons();
            let mut effects = Vec::new();
            // Mouse and touch: on the wheel a moving pointer points and a press
            // presses. On the list a hover is drawn and never moves the cursor
            // (Q70: it would undo every arrow key, since the list is rebuilt
            // under the pointer); a press selects and activates.
            for (entity, interaction, button) in &widgets.wheel {
                if widgets.owner(entity) != Some(owner) { continue; }
                match interaction {
                    Interaction::Hovered if pointer_moved => ui.point(button.0),
                    Interaction::Pressed => effects.push(ui.press_slot(button.0)),
                    _ => {}
                }
            }
            // As many rows as the screen holds: a phone shows fewer, and scrolls.
            // The screen is the size the panel's root was laid out at (not the
            // window's, which an offscreen target or a UI scale does not share).
            if let Some(height) = widgets.roots.iter().find(|(_, root, _)| root.owner == owner).map(|(_, _, node)| node.size().y * node.inverse_scale_factor()) {
                let compact = height < COMPACT_BELOW;
                let chrome = if compact { COMPACT_CHROME } else { LIST_CHROME };
                let fit = (((height - chrome) / ROW_PITCH).floor() as usize).clamp(2, 12);
                if ui.list_rows != fit || ui.compact != compact {
                    ui.list_rows = fit;
                    ui.compact = compact;
                    ui.list_window = ambition_ui_nav::scroll_into_view(ui.list_window, ui.list_cursor, rows.len(), fit);
                }
            }
            // A row activates when the pointer comes up on the row it went down
            // on, unless it travelled (a finger scrolling the list): the shared
            // press guard of the menus' flat lists (`ambition_ui_nav::RowPress`).
            let cursor = windows.single().ok().and_then(Window::cursor_position);
            let position = ambition_ui_nav::pointer_position(touches, cursor);
            let pointer_up = ambition_ui_nav::pointer_up(touches, mouse_buttons);
            if ui.row_press.is_armed() {
                ui.row_press.moved(position);
            }
            let mut tapped = None;
            for (entity, interaction, button) in &widgets.list {
                if widgets.owner(entity) != Some(owner) { continue; }
                let index = button.0;
                if index >= rows.len() {
                    continue;
                }
                match interaction {
                    Interaction::Hovered => {
                        ui.list_hover = Some(index);
                        // A mouse released over the row it pressed reports `Hovered`.
                        if pointer_up.came_up() && ui.row_press.release(index, position) == Some(index) {
                            tapped = Some(index);
                        }
                    }
                    Interaction::Pressed => {
                        ui.row_press.press(index, position);
                        // A whole tap can fit in one frame.
                        if pointer_up.came_up() {
                            ui.row_press.clear();
                            tapped = Some(index);
                        }
                    }
                    // `None` is a finger lifting AND a pointer leaving the row: only
                    // a touch that came up this frame is a release.
                    Interaction::None => {
                        if ui.list_hover == Some(index) {
                            ui.list_hover = None;
                        }
                        if pointer_up != ambition_ui_nav::PointerUp::Touch {
                            if ui.row_press.armed() == Some(&index) {
                                ui.row_press.left(index);
                            }
                        } else if ui.row_press.release(index, position) == Some(index) {
                            tapped = Some(index);
                        }
                    }
                }
            }
            if let Some(row) = tapped.and_then(|index| rows.get(index).cloned()) {
                ui.focus = TerminalFocus::List;
                ui.list_cursor = tapped.unwrap_or_default();
                effects.push(ui.press_row(&row));
            }
            for (entity, interaction) in &widgets.back {
                if widgets.owner(entity) != Some(owner) { continue; }
                if *interaction == Interaction::Pressed {
                    ui.focus = TerminalFocus::List;
                }
            }
            // A mouse wheel or a drag over the screen scrolls the list's window,
            // not its cursor.
            let drag = if over_surface { pointer_frame.as_ref().map_or(0, |menu| -(menu.drag_scroll_y.round() as i32)) } else { 0 };
            if scrolled + drag != 0 {
                ui.scroll_list(scrolled + drag, rows.len());
            }
            // The stick points the wheel while the wheel has the focus.
            let leaning = ui.focus == TerminalFocus::Wheel && {
                let stick = menu.as_ref().map_or(Vec2::ZERO, |menu| menu.analog);
                let angle = (stick.length() >= STICK_POINTS).then(|| stick.y.atan2(stick.x).to_degrees());
                if let Some(angle) = angle {
                    ui.point(WheelSlot::at_angle(angle, count));
                }
                angle.is_some()
            };
            if let Some(menu) = menu.as_ref() {
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
                    intents.write(intent(&subject, &ring, action));
                }
            }
        }
        previews.entry(ring).or_insert_with(|| ui.entered.clone());
    }
    if preview.by_ring != previews { preview.by_ring = previews; }

}

/// Draw the terminal panel from the model, rebuilt only when it changed.
#[allow(clippy::too_many_arguments)]
fn draw_gate_terminal(
    mut commands: Commands,
    catalog: Option<Res<GateNetworkCatalog>>,
    positions: Option<Res<GateRingPositions>>,
    state: Option<Res<GateNetworkState>>,
    save: Option<Res<ambition_platformer2d::persistence::save::AmbitionGameSave>>,
    mut users: Query<(Entity, &ambition_input::participant::InputParticipant, &TerminalUi, &mut ShownTerminal)>,
    participants: TerminalParticipants,
    surface: ambition_render::ui_surface::ParticipantSurface,
    roots: Query<(Entity, &GateTerminalRoot, &ComputedNode)>,
    ui_fonts: Option<Res<UiFonts>>,
    mut icons: Option<ResMut<GlyphIcons>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let Some(scope) = participants.scope.get() else { return; };
    let owners: Vec<_> = participants.inputs.iter().filter(|(entity, _)| participants.contains(*entity)).map(|(_, participant)| participant.id).collect();
    for (owner, participant, ui, mut shown) in &mut users {
        let region = surface.menu_rect(participant.id, &owners);
        let model = match (catalog.as_deref(), state.as_deref(), ui.ring.as_deref()) {
            (Some(catalog), Some(state), Some(ring)) if ui.subject.as_ref().and_then(|subject| state.terminals.get(subject)).map(String::as_str) == Some(ring) => {
                let knows = |flag: &str| save.as_ref().is_some_and(|save| save.data().flag(flag));
                let none = GateRingPositions::default();
                let known = known_addresses(catalog, positions.as_deref().unwrap_or(&none), ring, knows);
                Some(ui.model(catalog, &state, &known))
            }
            _ => None,
        };
        let wanted = model.clone().zip(region);
        if shown.0 == wanted { continue; }
        // The height the last panel was laid out at, to fit the wheel.
        let height = region.map_or(0.0, |region| region.height());
        let width = region.map_or(0.0, |region| region.width());
        let stacked = model.as_ref().is_some_and(|model| model.focus == TerminalFocus::Wheel) && width < WHEEL + 360.0;
        for (entity, root, _) in &roots {
            if root.owner == owner { commands.entity(entity).despawn(); }
        }
        shown.0 = wanted;
        let (Some(model), Some(region), Some(icons)) = (model, region, icons.as_mut()) else { continue; };
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
        let mut node = Node {
            padding: UiRect::all(Val::Px(if model.compact { 4.0 } else { 12.0 })),
            justify_content: JustifyContent::Center, align_items: AlignItems::Center,
            ..default()
        };
        surface.place(&mut node, region);
        commands
            .spawn_in_session(scope, (
                node,
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
                ZIndex(46),
                GateTerminalRoot { owner },
                Name::new("Gate Terminal Root"),
            ))
            .with_children(|root| {
                root.spawn((
                    Node {
                        max_width: Val::Percent(100.0),
                        max_height: Val::Percent(100.0),
                        // A misfit clips the bottom, never the heading.
                        overflow: Overflow::clip(),
                        padding: UiRect::all(Val::Px(if model.compact { 8.0 } else { 14.0 })),
                        flex_direction: if stacked { FlexDirection::Column } else { FlexDirection::Row },
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        column_gap: Val::Px(20.0),
                        border: UiRect::all(Val::Px(4.0)),
                        border_radius: BorderRadius::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(PANEL),
                    BorderColor::all(PANEL_EDGE),
                    Name::new("Gate Terminal Panel"),
                ))
                .with_children(|panel| match model.focus {
                    TerminalFocus::List => spawn_list(panel, &model, &glyphs, &font),
                    TerminalFocus::Wheel => {
                        // The wheel shrinks to fit a short screen (a phone held
                        // sideways); its layout stays the ring's.
                        let available_height = height - if stacked { 300.0 } else { 60.0 };
                        let scale = (available_height.min(width - 60.0) / WHEEL).clamp(0.2, 1.0);
                        spawn_wheel(panel, &model, &glyphs, &font, scale);
                        spawn_dial_side(panel, &model, &font);
                    }
                });
            });
    }
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
    scale: f32,
) {
    let count = model.inscription.len();
    let c = WHEEL / 2.0;
    let wheel_focus = model.focus == TerminalFocus::Wheel;
    // Laid out at full size inside a box of the scaled size, then scaled: a
    // transform does not change layout, so the box keeps the panel's size right.
    panel
        .spawn((Node { width: Val::Px(WHEEL * scale), height: Val::Px(WHEEL * scale), flex_shrink: 0.0, ..default() }, Name::new("Gate dial wheel box")))
        .with_children(|holder| {
            holder
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px((WHEEL * scale - WHEEL) / 2.0),
                top: Val::Px((WHEEL * scale - WHEEL) / 2.0),
                width: Val::Px(WHEEL),
                height: Val::Px(WHEEL),
                ..default()
            },
            UiTransform::from_scale(Vec2::splat(scale)),
            Name::new("Gate dial wheel"),
        ))
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
        });
}

/// The terminal's main view: the ring, the status, and the destinations,
/// as many rows as the screen holds, big enough for a finger.
fn spawn_list(
    panel: &mut ChildSpawnerCommands,
    model: &TerminalModel,
    glyphs: &Glyphs,
    font: &impl Fn(f32, UiFontWeight) -> TextFont,
) {
    panel
        .spawn((
            Node { width: Val::Px(620.0), max_width: Val::Percent(100.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), ..default() },
            Name::new("Gate terminal list"),
        ))
        .with_children(|side| {
            if model.compact {
                spawn_compact_heading(side, model, font);
            } else {
                spawn_heading(side, model, font);
            }
            let shown = model.list_window..(model.list_window + model.list_rows).min(model.list.len());
            let more = |count: usize, up: bool| {
                (
                    Text::new(format!("{} {count} more", if up { "\u{25B2}" } else { "\u{25BC}" })),
                    font(13.0, UiFontWeight::Semibold),
                    TextColor(DIM_INK),
                )
            };
            if shown.start > 0 {
                side.spawn(more(shown.start, true));
            }
            for (index, row) in model.list.iter().enumerate().skip(shown.start).take(shown.len()) {
                let selected = model.focus == TerminalFocus::List && model.list_cursor == index;
                let hovered = model.list_hover == Some(index);
                side.spawn((
                    Button,
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(12.0),
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                        min_height: Val::Px(ROW_TALL),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
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
                    ListRow::Address { address, word, name, recent } => {
                        line.spawn((
                            Node { flex_direction: FlexDirection::Column, flex_grow: 1.0, ..default() },
                        ))
                        .with_children(|text| {
                            text.spawn((Text::new(name.clone()), font(19.0, UiFontWeight::Semibold), TextColor(INK)));
                            let detail = match (word.is_empty(), recent) {
                                (true, true) => "recent".to_string(),
                                (true, false) => String::new(),
                                (false, true) => format!("{word}  ·  recent"),
                                (false, false) => word.clone(),
                            };
                            if !detail.is_empty() {
                                text.spawn((Text::new(detail), font(13.0, UiFontWeight::Regular), TextColor(DIM_INK)));
                            }
                        });
                        line.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(1.0), ..default() }).with_children(
                            |chevrons| {
                                for chevron in address {
                                    chevrons.spawn(glyphs.node(*chevron, 20.0, CYAN_DIM, 0.0));
                                }
                            },
                        );
                    }
                    ListRow::Dial => {
                        line.spawn((Text::new("Dial an address\u{2026}"), font(17.0, UiFontWeight::Regular), TextColor(CYAN)));
                    }
                    ListRow::Hangup => {
                        line.spawn((Text::new("Hang up"), font(17.0, UiFontWeight::Semibold), TextColor(INK)));
                    }
                    ListRow::Close => {
                        line.spawn((Text::new("Leave"), font(17.0, UiFontWeight::Semibold), TextColor(INK)));
                    }
                });
            }
            if shown.end < model.list.len() {
                side.spawn(more(model.list.len() - shown.end, false));
            }
            if model.compact {
                return;
            }
            side.spawn((
                Text::new("Tap or press a destination to dial it  \u{00B7}  drag or scroll for more  \u{00B7}  Back leaves"),
                font(12.0, UiFontWeight::Regular),
                TextColor(DIM_INK),
            ));
        });
}

/// The ring's name, its facility, and the status line.
fn spawn_heading(side: &mut ChildSpawnerCommands, model: &TerminalModel, font: &impl Fn(f32, UiFontWeight) -> TextFont) {
    side.spawn((Text::new(model.title.clone()), font(22.0, UiFontWeight::Semibold), TextColor(CYAN)));
    side.spawn((Text::new(format!("{} \u{2014} {}", model.facility, model.note)), font(13.0, UiFontWeight::Regular), TextColor(DIM_INK)));
    side.spawn((
        Text::new(model.status.clone()),
        font(16.0, UiFontWeight::Semibold),
        TextColor(if model.refused { REFUSED } else { INK }),
    ));
}

/// The heading of a short screen, on one line: the ring's name and the status.
fn spawn_compact_heading(side: &mut ChildSpawnerCommands, model: &TerminalModel, font: &impl Fn(f32, UiFontWeight) -> TextFont) {
    side.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(12.0), align_items: AlignItems::Baseline, ..default() })
        .with_children(|line| {
            line.spawn((Text::new(model.title.clone()), font(19.0, UiFontWeight::Semibold), TextColor(CYAN)));
            line.spawn((
                Text::new(model.status.clone()),
                font(14.0, UiFontWeight::Semibold),
                TextColor(if model.refused { REFUSED } else { INK }),
            ));
        });
}

/// Beside the wheel: the heading, the way back to the list, and the hint.
fn spawn_dial_side(panel: &mut ChildSpawnerCommands, model: &TerminalModel, font: &impl Fn(f32, UiFontWeight) -> TextFont) {
    panel
        .spawn((
            Node { width: Val::Px(300.0), flex_shrink: 1.0, flex_direction: FlexDirection::Column, row_gap: Val::Px(8.0), ..default() },
            Name::new("Gate dial side"),
        ))
        .with_children(|side| {
            spawn_heading(side, model, font);
            side.spawn((
                Button,
                Node {
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    min_height: Val::Px(ROW_TALL),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(ROW),
                BorderColor::all(CYAN_DIM),
                DialBackButton,
                Name::new("Gate dial back to the list"),
            ))
            .with_children(|button| {
                button.spawn((Text::new("\u{25C0}  Back to the list"), font(17.0, UiFontWeight::Semibold), TextColor(INK)));
            });
            side.spawn((
                Text::new("Back erases a chevron, then returns to the list"),
                font(12.0, UiFontWeight::Regular),
                TextColor(DIM_INK),
            ));
        });
}
