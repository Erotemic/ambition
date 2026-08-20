//! **What the select screen LOOKS like, and how a cursor works it.**
//!
//! Jon, 2026-08-05 — the whole spec, kept verbatim in
//! `docs/planning/JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`:
//!
//! > a grid of portraits for each of the selectable characters on the top 65% of
//! > the screen. The bottom 35% of the screen should be 4 participant slot
//! > cards. In this UI the arrows or game stick or mouse should move a cursor
//! > that can click on elements. Each participant slot will have a button to
//! > toggle it between a controller player (which must have a corresponding
//! > attached controller), a CPU player, or not participating. Each
//! > participating card gets a corresponding sphere icon on the character grid
//! > that the cursor can pick up and drag to select a character. The selected
//! > portrait should appear on the participants bottom card.
//!
//! ## Three pieces, and why they are three
//!
//! * [`layout`] is where things ARE — a pure function of the viewport.
//! * [`cursor`] is a pointer over rectangles and knows nothing about fighters.
//! * [`crate::select::SmashSelect`] is the decision and knows nothing about
//!   pixels.
//!
//! This module is the only thing that knows all three, and it is thin because of
//! it: it writes the layout's rectangles into Bevy nodes, asks the cursor what
//! it is over, and tells the decision what happened.
//!
//! ⭐ **the layout is the same object the nodes are drawn from and the cursor
//! hit-tests against**, so "what you clicked" and "what you saw" are the same
//! numbers rather than two derivations that agree until they do not.
//!
//! ## ⚠ The portraits were already there
//!
//! Every character in this repo ships a generated `<stem>_portraits.png` beside
//! its spritesheet — 141 of them — and `CharacterCatalog::portrait_ref` already
//! derives the path from the sheet's own name. So the "engine helper for
//! portraits" Jon expected to need turns out to be a method that has existed the
//! whole time and that nothing outside the dialogue box ever called. This screen
//! is its second consumer, which is the only real test of whether that was a
//! seam or a private detail.

pub mod cursor;
pub mod layout;

use ambition_platformer2d::character::{
    portrait_for_declared_character, CharacterCatalog, PortraitSheetRegistry,
    PreparedCharacterRegistry,
};
use bevy::input::touch::Touches;
use bevy::prelude::*;

use crate::select::{SlotOccupant, SlotPick, SmashRoster, SmashSelect, MAX_SMASH_SEATS};
use cursor::{CursorTarget, HitRect, SelectCursor};
use layout::{SelectLayout, CURSOR_PX, TOKEN_PX};

/// The screen's UI root. One marker, so teardown is `despawn` on a query
/// filtered by THIS owner rather than a sweep of every node — a shared marker's
/// teardown clobbers whatever else happened to carry it.
#[derive(Component)]
pub struct SmashSelectUiRoot;

/// **Everything the cursor can act on.**
///
/// One type over four kinds rather than four marker components, because the
/// cursor's central question — *what am I over* — has to be answered once. Four
/// answers in some order is the shape that lets a press land on two things.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectTarget {
    /// A portrait in the grid. Indexes [`SmashRoster`].
    Portrait(usize),
    /// The button that cycles one card between controller / CPU / absent.
    RoleButton(usize),
    /// A slot's draggable token, at rest in the pool.
    Token(usize),
    /// Begin the match.
    Start,
    /// Leave the lobby — see [`LeaveRequested`].
    Back,
}

/// **Which rectangle in the layout a node wears.**
///
/// One component and one placing system, so a widget that moves when the window
/// resizes cannot be the one somebody forgot.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchored {
    Title,
    Prompt,
    Portrait(usize),
    Card(usize),
    RoleButton(usize),
    CardPortrait(usize),
    Start,
    Back,
}

/// A slot's token. Positioned from the DECISION rather than from an anchor — it
/// is over a portrait, in the cursor's hand, or resting in the pool.
#[derive(Component, Clone, Copy)]
pub struct SlotToken(pub usize);

/// The cursor's own node.
#[derive(Component)]
pub struct CursorNode;

/// The frame around one portrait, tinted by who is on it.
#[derive(Component, Clone, Copy)]
pub struct PortraitCell(pub usize);

/// One slot card's outer frame.
#[derive(Component, Clone, Copy)]
pub struct SlotCardFrame(pub usize);

/// The text inside a card's role button.
#[derive(Component, Clone, Copy)]
pub struct RoleButtonLabel(pub usize);

/// The chosen fighter's portrait on a card.
#[derive(Component, Clone, Copy)]
pub struct CardPortrait(pub usize);

/// The chosen fighter's name on a card.
#[derive(Component, Clone, Copy)]
pub struct CardName(pub usize);

/// The initials drawn under a portrait, shown only when the art never arrives.
///
/// ⚠ it carries the HANDLE rather than a boolean, because "is there art" cannot
/// be answered when the cell is built: the path resolves by convention and the
/// load fails later, asynchronously, or never finishes. Asking the asset server
/// each frame is the only honest form of the question.
#[derive(Component)]
pub struct PortraitMonogram(pub Option<Handle<Image>>);

/// The line that says what the screen is waiting for.
#[derive(Component)]
pub struct SelectPrompt;

/// The start button's frame, which dims until the match can start.
#[derive(Component)]
pub struct StartButton;

/// **Somebody clicked START.**
///
/// ⚠ a rung the old screen did not have, and it is here because Jon asked for
/// the real thing: *"work just like how the real smash character select screen
/// works"*, and that screen does not launch the instant the last token lands.
/// Auto-starting on readiness also made the screen impossible to LOOK at — every
/// attempt to photograph a decided lobby photographed the match instead.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct StartRequested(pub bool);

/// The BACK button's frame. Never dims: leaving is always allowed, where
/// starting waits on a decided lobby.
#[derive(Component)]
pub struct BackButton;

/// **Somebody asked to leave the lobby.**
///
/// ⭐ **a REQUEST, exactly like [`StartRequested`], and for the same reason.**
/// This module draws the screen and arbitrates presses; it does not name the
/// shell. Writing `ShellCommand` from here would give the screen a second
/// opinion about routing, and the one place that decides where BACK goes
/// (`leave_the_select_screen_when_asked`) would no longer be the only one.
///
/// ⚠ **and it has to be one flag rather than two systems reading the same
/// edge.** BACK already means "put the token down" while the cursor is
/// carrying one, and that undo is decided in [`drive_the_cursor`] — a separate
/// system reading `SeatMenuFrames` for the quit would see the same frame and
/// leave the lobby on the press that was meant to drop a fighter.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct LeaveRequested(pub bool);

/// Slot colours, in the order a couch fills up. Far enough apart in hue that a
/// token and its card are matched at a glance across the 65% gap, which is the
/// only thing tying them together on screen.
const SLOT_COLORS: [Color; MAX_SMASH_SEATS] = [
    Color::srgb(0.98, 0.36, 0.36),
    Color::srgb(0.36, 0.62, 0.99),
    Color::srgb(0.99, 0.82, 0.30),
    Color::srgb(0.44, 0.90, 0.52),
];

const INK: Color = Color::srgb(0.94, 0.96, 1.0);
const DIM_INK: Color = Color::srgb(0.55, 0.60, 0.72);
const PANEL: Color = Color::srgb(0.07, 0.08, 0.13);
const PANEL_EDGE: Color = Color::srgb(0.20, 0.23, 0.33);
const BACKDROP: Color = Color::srgb(0.03, 0.035, 0.06);

/// What a card's button says, which is also the full statement of what that card
/// IS. Public because a test that checks entities EXIST is a test that passes
/// over an empty box.
/// **What a slot's role button says.**
///
/// ⭐ **it names the DEVICE, not an index.** It read `CONTROLLER 1` /
/// `CONTROLLER 2`, which is the slot's own numbering said back to it and tells
/// nobody which thing in the room drives that card. Jon, debugging a couch match
/// on 2026-08-07: *"the UI has no way to indicate which player is connected to
/// which input device, so idk if that is the problem or not."* It says
/// `KEYBOARD` and `PAD 1` now, from the same authority that decided the index —
/// see `select::source_name_under`.
///
/// `devices`/`policy` are optional because a fixture (and the walkthrough
/// binary) renders this text without a live input world; absent, the button
/// falls back to the index it always showed rather than claiming a device it
/// cannot see.
pub fn role_button_text(
    occupant: SlotOccupant,
    naming: Option<(
        &ambition_platformer2d::input::LocalDeviceOrder,
        ambition_platformer2d::input::sources::InputAssignmentPolicy,
    )>,
) -> String {
    match occupant {
        SlotOccupant::Absent => "NOT PLAYING".to_string(),
        SlotOccupant::Controller { device } => match naming {
            Some((devices, policy)) => crate::select::source_name_under(device, devices, policy),
            None => format!("CONTROLLER {}", device + 1),
        },
        SlotOccupant::Cpu => "CPU".to_string(),
    }
}

/// The fighter one card has chosen, in words.
pub fn card_name_text(
    catalog: Option<&CharacterCatalog>,
    fighters: &SmashRoster,
    pick: Option<SlotPick>,
) -> String {
    match pick {
        // ⚠ **"RANDOM", not a fighter's name.** The card must not name somebody
        // before the draw happens, and it must not read as undecided either —
        // a slot on random IS decided, which is why `ready()` counts it.
        Some(SlotPick::Random) => "RANDOM".to_string(),
        Some(SlotPick::Fighter(index)) => match fighters.get(index) {
            Some(id) => display_name(catalog, id),
            None => "— no fighter —".to_string(),
        },
        None => "— no fighter —".to_string(),
    }
}

/// **The random square's icon**, for the grid cell AND for the card of a slot
/// that took it — one accessor, because two spellings of "what random looks
/// like" is how a screen ends up disagreeing with itself.
///
/// ⚠ **PLACEHOLDER ART.** This is `BonusBlockTile` — the interrobang plate from
/// Mary-O's bonus blocks — reused because the glyph already means "you do not
/// know what is in here" (Jon, 2026-08-07: *"We can reuse the interobang sprite
/// for this"*). It is a 32x32 world TILE standing in for a portrait, so it reads
/// heavier than the faces beside it and its plate/rivets belong to a different
/// game. Replace it with a drawn random icon when one exists; nothing else has
/// to change, because this function is the only place that names it.
fn random_icon(art: &ScreenArt<'_>) -> Option<Handle<Image>> {
    art.entities.as_deref().and_then(|assets| {
        assets
            .entities
            .get(ambition_platformer2d::actors::assets::game_assets::EntitySprite::BonusBlockTile)
            .cloned()
    })
}

/// Which grid cell a pick occupies. The random square is a cell like any other,
/// so a token can rest on it and it can light up when somebody takes it.
fn cell_of(pick: SlotPick, fighters: &SmashRoster) -> usize {
    match pick {
        SlotPick::Fighter(index) => index,
        SlotPick::Random => fighters.random_cell(),
    }
}

fn display_name(catalog: Option<&CharacterCatalog>, id: &str) -> String {
    catalog
        .and_then(|catalog| catalog.get(id))
        .map(|entry| entry.display_name.clone())
        // ⚠ NOT a panic and not an empty string. A grid cell with no name is a
        // cell nobody can talk about, and a catalog miss is exactly the kind of
        // authoring slip that should be visible on the screen rather than fatal
        // in a demo somebody is showing to a room.
        .unwrap_or_else(|| id.to_string())
}

/// Initials, for a cell whose art did not arrive. The dialogue box has done this
/// for speakers with no portrait since it was written; this is the same idea one
/// screen over.
fn monogram(label: &str) -> String {
    let mut words = label
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .filter_map(|word| word.chars().next())
        .map(|ch| ch.to_uppercase().collect::<String>());
    let first = words.next().unwrap_or_else(|| "?".to_string());
    match words.next() {
        Some(second) => format!("{first}{second}"),
        None => first,
    }
}

/// **Everything needed to draw a character**, as one parameter.
///
/// ⚠ four separate `Res` arguments pushed `present_the_select_screen` past
/// Bevy's system-parameter tuple ceiling. Grouping them is not only a workaround:
/// a portrait is *catalog + manifest + asset server*, and the three arriving
/// together is what stops one of them being forgotten at a second call site.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ScreenArt<'w> {
    pub catalog: Res<'w, CharacterCatalog>,
    pub portraits: Option<Res<'w, PortraitSheetRegistry>>,
    /// What providers REGISTERED, so a character that named a portrait target in
    /// Rust gets the face it asked for rather than the one its sheet name
    /// happens to derive. See [`portrait_art`].
    pub declared: Option<Res<'w, PreparedCharacterRegistry>>,
    pub asset_server: Option<Res<'w, AssetServer>>,
    /// The decoded entity art, for the RANDOM square's interrobang. Loaded by
    /// the same asset pass every other sprite comes from, so the square is
    /// absent for exactly the reasons any other sprite would be.
    pub entities: Option<Res<'w, ambition_platformer2d::view::GameAssets>>,
    pub menu_font: Option<Res<'w, ambition_platformer2d::menu::render::bevy_ui::MenuFont>>,
}

impl ScreenArt<'_> {
    /// This character's face and the rectangle to take out of it.
    pub fn portrait(&self, id: &str) -> Option<(Handle<Image>, Option<Rect>)> {
        portrait_art(
            &self.catalog,
            self.portraits.as_deref(),
            self.declared.as_deref(),
            self.asset_server.as_deref(),
            id,
        )
    }

    pub fn display_name(&self, id: &str) -> String {
        display_name(Some(&self.catalog), id)
    }
}

/// **A character's face, as an image AND the rectangle to take out of it.**
///
/// ⛔ **found by LOOKING, 2026-08-05.** The first version loaded the derived PNG
/// whole. Most portrait sheets are one 256x320 frame, so most cells were right —
/// and `alice_portraits.png` and `oiler_portraits.png` are **2048x320**, eight
/// frames of a `default` / `speaking` / `focused` clip set, which drew as a
/// strip of eight tiny Alices. **A portrait sheet is a SHEET**, and the manifest
/// beside it has said so all along.
///
/// ⚠ the registry is `Option` because the standalone smash app does not install
/// `PortraitSheetRegistryPlugin` unless this screen asks for it, and a
/// composition without one should still draw a face rather than nothing. With no
/// registry the whole image is used, which is exactly right for the single-frame
/// sheets that are the majority and visibly wrong for the rest — so the fallback
/// reports the missing plugin instead of hiding it.
fn portrait_art(
    catalog: &CharacterCatalog,
    portraits: Option<&PortraitSheetRegistry>,
    declared: Option<&PreparedCharacterRegistry>,
    asset_server: Option<&AssetServer>,
    id: &str,
) -> Option<(Handle<Image>, Option<Rect>)> {
    // ⭐ **through the ENGINE's resolver, not straight to the catalog.** A
    // character registered in Rust may name a portrait TARGET; everything that
    // names nothing keeps the catalog's derived answer, which is how all 144 of
    // today's portraits resolve. This screen is the first consumer of that road
    // — Jon decided it on 2026-07-29 and nothing had reason to call it until a
    // grid of faces existed.
    let target = declared
        .and_then(|registry| registry.get(id))
        .and_then(|prepared| prepared.portrait.as_deref());
    let reference = portrait_for_declared_character(portraits, catalog, target, id)?;
    let handle = asset_server?.load::<Image>(reference.image.clone());
    let rect = portraits
        .and_then(|registry| {
            registry.resolve_clip(&reference.manifest, None, &reference.default_clip)
        })
        .and_then(|(_, clip)| clip.frames.first().copied())
        .map(|frame| {
            Rect::new(
                frame.x as f32,
                frame.y as f32,
                (frame.x + frame.w) as f32,
                (frame.y + frame.h) as f32,
            )
        });
    Some((handle, rect))
}

/// The viewport this screen is laid out for, or `None` where there is no window.
fn viewport(windows: &Query<&Window>) -> Option<Vec2> {
    windows
        .iter()
        .next()
        .map(|window| Vec2::new(window.width(), window.height()))
}

/// **Does backing out of this lobby lead anywhere?**
///
/// ⛔ **it does not in every composition, and pretending otherwise is a dead
/// button.** The standalone smash demo names the select screen as its OWN home
/// route (`ShellComposition::new(.., SMASH_SELECT_ROUTE, ..)`), because leaving
/// a match there should return to the screen that chose it rather than to a
/// launcher listing one game. `QuitToHome` in that app therefore re-enters the
/// route it is already on: a churn with nothing to see. The multi-game host
/// names its launcher, and there this is the title screen Jon asked for.
///
/// ⚠ **a fact about the COMPOSITION, so it is read from the host spec rather
/// than assumed by either app.** Absent (a bare unit fixture with no host
/// configured) reads as NO exit: a screen with no shell to leave through has
/// nowhere to go, and drawing an exit for one would be the same lie.
pub fn exit_leads_somewhere(
    host: Option<&ambition_platformer2d::game_shell::ShellHostConfiguration>,
) -> bool {
    host.and_then(|host| host.spec.as_ref())
        .is_some_and(|spec| spec.home_route.as_str() != crate::SMASH_SELECT_ROUTE)
}

/// The layout this frame, from the window if there is one.
pub fn current_layout(windows: &Query<&Window>, fighters: &SmashRoster) -> SelectLayout {
    SelectLayout::for_viewport(viewport(windows), fighters.cell_count())
}

/// Build the screen.
///
/// ⚠ **built once and positioned every frame.** Nothing here is rebuilt: a tree
/// respawned per frame throws away the handles its images resolved to and
/// restarts every load, and the positions come from [`layout`] anyway — so a
/// resize costs four numbers rather than a rebuild.
pub fn spawn_select_screen(
    mut commands: Commands,
    existing: Query<(), With<SmashSelectUiRoot>>,
    fighters: Res<SmashRoster>,
    // ⛔ **the catalog inside is REQUIRED, not `Option`.**
    // `engine.character-authority-is-app-local` forbids making it optional by
    // name, and the reason is this screen's exact failure mode: an absent
    // catalog would draw a grid of nameless plates that looks like missing ART
    // rather than like a composition with no cast. Every composition that
    // reaches this route has one, because this demo registers its own fragment.
    art: ScreenArt,
    // **WHETHER TO DRAW THE WAY OUT** — see [`exit_leads_somewhere`]. A plain
    // `bool` rather than the resource, because the caller already holds it and
    // this is a drawing decision, not a second reading of the host spec.
    exit: bool,
) {
    if !existing.is_empty() {
        return;
    }
    let catalog = Some(&*art.catalog);
    let font = art
        .menu_font
        .as_deref()
        .and_then(|font| font.0.clone())
        .unwrap_or_default();
    let text_font = |size: f32| TextFont {
        font: font.clone(),
        font_size: size,
        ..default()
    };
    let portrait = |id: &str| art.portrait(id);
    // Everything the layout places is absolute; `place_the_screen` fills in the
    // numbers on the same frame, before anything is presented.
    let anchored = |anchor: Anchored| {
        (
            anchor,
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
        )
    };

    commands
        .spawn((
            SmashSelectUiRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(BACKDROP),
            // Above the world, below the pause menu — a frontend route drawn
            // over whatever the shell had already put on screen.
            GlobalZIndex(600),
            Name::new("smash select screen"),
        ))
        .with_children(|root| {
            let mut title = anchored(Anchored::Title);
            title.1.justify_content = JustifyContent::Center;
            title.1.align_items = AlignItems::Center;
            root.spawn(title).with_children(|node| {
                node.spawn((
                    Text::new("CHOOSE YOUR FIGHTER"),
                    text_font(26.0),
                    TextColor(INK),
                ));
            });

            // ── THE WAY OUT ──────────────────────────────────────────────
            //
            // ⭐ **a BUTTON, not only a binding.** Jon, 2026-08-16: *"in the
            // smash character select, there is no way to quit to title, you can
            // only do this if you start a match."* The `back` intent alone would
            // not have answered that: a mouse has no Back control at all, so a
            // player at a desk with no pad and no keyboard hand on Escape would
            // still have been stuck in the lobby — and an unlabelled press is a
            // feature only the person who wrote it knows about.
            //
            // ⚠ it never dims. START waits on a decided lobby; leaving is always
            // allowed, which is the whole complaint.
            if exit {
                let mut back = anchored(Anchored::Back);
                back.1.justify_content = JustifyContent::Center;
                back.1.align_items = AlignItems::Center;
                back.1.border = UiRect::all(Val::Px(2.0));
                back.1.border_radius = BorderRadius::all(Val::Px(6.0));
                root.spawn((
                    back,
                    BackButton,
                    BackgroundColor(PANEL),
                    BorderColor::all(PANEL_EDGE),
                    Name::new("back button"),
                ))
                .with_children(|node| {
                    node.spawn((Text::new("BACK"), text_font(15.0), TextColor(DIM_INK)));
                });
            }

            // ── THE GRID: Jon's top 65% ──────────────────────────────────
            //
            // ⚠ **one more cell than there are fighters.** The last square is
            // RANDOM (Jon, 2026-08-07), drawn below the loop so every fighter
            // keeps the cell index it already had.
            for (index, id) in fighters.ids().enumerate() {
                let mut cell = anchored(Anchored::Portrait(index));
                cell.1.flex_direction = FlexDirection::Column;
                cell.1.align_items = AlignItems::Center;
                cell.1.justify_content = JustifyContent::SpaceBetween;
                cell.1.border = UiRect::all(Val::Px(3.0));
                cell.1.border_radius = BorderRadius::all(Val::Px(8.0));
                cell.1.padding = UiRect::all(Val::Px(4.0));
                cell.1.overflow = Overflow::clip();
                root.spawn((
                    cell,
                    PortraitCell(index),
                    BackgroundColor(PANEL),
                    BorderColor::all(PANEL_EDGE),
                    Name::new(format!("portrait cell {index}")),
                ))
                .with_children(|cell| {
                    // **A MONOGRAM UNDER EVERY PORTRAIT.**
                    //
                    // ⛔ found by LOOKING: `mary_o` draws a hole. Her catalog row
                    // names `mary_o_v2_spritesheet.png`, `portrait_ref` derives
                    // `mary_o_v2_portraits.png` by convention, and that file was
                    // never generated — so the path resolves, the load fails,
                    // and the `ImageNode` renders nothing. **A derived path is
                    // not a promise that the art exists**, and the failure is
                    // silent all the way down.
                    //
                    // So the cell says who it is underneath, always. A missing
                    // portrait then reads as missing ART rather than as a broken
                    // grid, and it costs nothing when the art is there.
                    let art = portrait(id);
                    cell.spawn((
                        PortraitMonogram(art.as_ref().map(|(handle, _)| handle.clone())),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            right: Val::Px(0.0),
                            top: Val::Percent(24.0),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        Visibility::Hidden,
                        Name::new(format!("portrait {index} monogram")),
                    ))
                    .with_children(|slate| {
                        slate.spawn((
                            Text::new(monogram(&display_name(catalog, id))),
                            text_font(46.0),
                            TextColor(Color::srgb(0.28, 0.31, 0.42)),
                        ));
                    });
                    match art {
                        Some((handle, rect)) => {
                            let mut image = ImageNode::new(handle);
                            image.rect = rect;
                            cell.spawn((
                                image,
                                Node {
                                    flex_grow: 1.0,
                                    width: Val::Percent(100.0),
                                    min_height: Val::Px(0.0),
                                    ..default()
                                },
                            ));
                        }
                        None => {
                            cell.spawn((Node {
                                flex_grow: 1.0,
                                width: Val::Percent(100.0),
                                min_height: Val::Px(0.0),
                                ..default()
                            },));
                        }
                    }
                    cell.spawn((
                        Text::new(display_name(catalog, id)),
                        text_font(13.0),
                        TextColor(INK),
                        TextLayout::new_with_justify(Justify::Center),
                    ));
                });
            }

            // ── THE RANDOM SQUARE, last cell of the grid ─────────────────
            //
            // ⭐ **the interrobang, reused deliberately** (Jon: *"We can reuse
            // the interobang sprite for this"*). It is the glyph on Mary-O's
            // bonus block — the thing you hit without knowing what comes out —
            // which is the same promise this square makes.
            //
            // ⚠ it is a cell like any other: a token rests on it, it lights up
            // for whoever took it, and `SmashSelect::ready()` counts it as
            // decided. What it is NOT is a character, which is why the pick is
            // `SlotPick::Random` and not an index.
            {
                let index = fighters.random_cell();
                let mut cell = anchored(Anchored::Portrait(index));
                cell.1.flex_direction = FlexDirection::Column;
                cell.1.align_items = AlignItems::Center;
                cell.1.justify_content = JustifyContent::SpaceBetween;
                cell.1.border = UiRect::all(Val::Px(3.0));
                cell.1.border_radius = BorderRadius::all(Val::Px(8.0));
                cell.1.padding = UiRect::all(Val::Px(4.0));
                cell.1.overflow = Overflow::clip();
                root.spawn((
                    cell,
                    PortraitCell(index),
                    BackgroundColor(PANEL),
                    BorderColor::all(PANEL_EDGE),
                    Name::new("portrait cell random"),
                ))
                .with_children(|cell| {
                    match random_icon(&art) {
                        Some(handle) => {
                            cell.spawn((
                                ImageNode::new(handle),
                                Node {
                                    flex_grow: 1.0,
                                    width: Val::Percent(100.0),
                                    min_height: Val::Px(0.0),
                                    ..default()
                                },
                            ));
                        }
                        // ⚠ the same rule the portraits follow: a composition
                        // with no asset server draws the LABEL and no art,
                        // rather than a hole nobody can explain.
                        None => {
                            cell.spawn((Node {
                                flex_grow: 1.0,
                                width: Val::Percent(100.0),
                                min_height: Val::Px(0.0),
                                ..default()
                            },));
                        }
                    }
                    cell.spawn((
                        Text::new("RANDOM"),
                        text_font(13.0),
                        TextColor(INK),
                        TextLayout::new_with_justify(Justify::Center),
                    ));
                });
            }

            // ── THE POOL STRIP: prompt, resting tokens, START ────────────
            let mut prompt = anchored(Anchored::Prompt);
            prompt.1.align_items = AlignItems::Center;
            root.spawn(prompt).with_children(|node| {
                node.spawn((
                    SelectPrompt,
                    Text::new(String::new()),
                    text_font(14.0),
                    TextColor(DIM_INK),
                ));
            });

            let mut start = anchored(Anchored::Start);
            start.1.justify_content = JustifyContent::Center;
            start.1.align_items = AlignItems::Center;
            start.1.border = UiRect::all(Val::Px(2.0));
            start.1.border_radius = BorderRadius::all(Val::Px(6.0));
            root.spawn((
                start,
                StartButton,
                BackgroundColor(PANEL),
                BorderColor::all(PANEL_EDGE),
                Name::new("start button"),
            ))
            .with_children(|node| {
                node.spawn((Text::new("START"), text_font(17.0), TextColor(INK)));
            });

            // ── THE CARDS: Jon's bottom 35% ──────────────────────────────
            for slot in 0..MAX_SMASH_SEATS {
                let mut card = anchored(Anchored::Card(slot));
                card.1.flex_direction = FlexDirection::Column;
                card.1.align_items = AlignItems::Center;
                card.1.border = UiRect::all(Val::Px(3.0));
                card.1.border_radius = BorderRadius::all(Val::Px(8.0));
                card.1.padding = UiRect::all(Val::Px(6.0));
                card.1.overflow = Overflow::clip();
                root.spawn((
                    card,
                    SlotCardFrame(slot),
                    BackgroundColor(PANEL),
                    BorderColor::all(PANEL_EDGE),
                    Name::new(format!("slot card {}", slot + 1)),
                ))
                .with_children(|card| {
                    card.spawn((
                        Text::new(format!("P{}", slot + 1)),
                        text_font(18.0),
                        TextColor(SLOT_COLORS[slot]),
                    ));
                    // The fighter's name sits at the BOTTOM of the card, under
                    // the portrait the layout places over it.
                    card.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(2.0),
                            right: Val::Px(2.0),
                            bottom: Val::Px(4.0),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        Name::new(format!("slot {} name row", slot + 1)),
                    ))
                    .with_children(|row| {
                        row.spawn((
                            CardName(slot),
                            Text::new(card_name_text(catalog, &fighters, None)),
                            text_font(13.0),
                            TextColor(DIM_INK),
                            TextLayout::new_with_justify(Justify::Center),
                        ));
                    });
                });

                let mut button = anchored(Anchored::RoleButton(slot));
                button.1.justify_content = JustifyContent::Center;
                button.1.align_items = AlignItems::Center;
                button.1.border = UiRect::all(Val::Px(2.0));
                button.1.border_radius = BorderRadius::all(Val::Px(6.0));
                root.spawn((
                    button,
                    BackgroundColor(Color::srgb(0.11, 0.12, 0.18)),
                    BorderColor::all(PANEL_EDGE),
                    GlobalZIndex(610),
                    Name::new(format!("slot {} role button", slot + 1)),
                ))
                .with_children(|node| {
                    node.spawn((
                        RoleButtonLabel(slot),
                        Text::new(role_button_text(SlotOccupant::Absent, None)),
                        text_font(14.0),
                        TextColor(INK),
                    ));
                });

                root.spawn((
                    anchored(Anchored::CardPortrait(slot)),
                    CardPortrait(slot),
                    ImageNode::default(),
                    GlobalZIndex(610),
                    // Hidden until something is picked; an empty `ImageNode`
                    // draws a white plate otherwise.
                    Visibility::Hidden,
                    Name::new(format!("slot {} chosen portrait", slot + 1)),
                ));
            }

            // ── THE TOKENS AND THE CURSOR, over everything ───────────────
            for slot in 0..MAX_SMASH_SEATS {
                root.spawn((
                    SlotToken(slot),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(-999.0),
                        top: Val::Px(-999.0),
                        width: Val::Px(TOKEN_PX),
                        height: Val::Px(TOKEN_PX),
                        border: UiRect::all(Val::Px(3.0)),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(SLOT_COLORS[slot]),
                    BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.85)),
                    GlobalZIndex(620),
                    Visibility::Hidden,
                    Name::new(format!("slot {} token", slot + 1)),
                ));
            }
            root.spawn((
                CursorNode,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(-999.0),
                    top: Val::Px(-999.0),
                    width: Val::Px(CURSOR_PX),
                    height: Val::Px(CURSOR_PX),
                    border: UiRect::all(Val::Px(3.0)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 0.96, 0.62, 0.35)),
                BorderColor::all(Color::srgb(1.0, 0.93, 0.35)),
                GlobalZIndex(640),
                Name::new("select cursor"),
            ));
        });
}

pub fn despawn_select_screen(
    mut commands: Commands,
    roots: Query<Entity, With<SmashSelectUiRoot>>,
) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

/// **Move the cursor, and act on what it is over.**
///
/// The mouse writes a position; the arrows, d-pad and stick snap between
/// targets. Both write the same field — see [`cursor`] for why there is one
/// position and no separate focus.
#[allow(clippy::too_many_arguments)]
pub fn drive_the_cursor(
    mut select: ResMut<SmashSelect>,
    mut pointer: ResMut<SelectCursor>,
    mut start: ResMut<StartRequested>,
    mut leave: ResMut<LeaveRequested>,
    fighters: Res<SmashRoster>,
    windows: Query<&Window>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    touches: Option<Res<Touches>>,
    seat_frames: Option<Res<ambition_platformer2d::input::SeatMenuFrames>>,
    global_frame: Option<Res<ambition_platformer2d::input::MenuControlFrame>>,
    devices: Option<Res<ambition_platformer2d::input::LocalDeviceOrder>>,
    // **IS THERE ANYWHERE TO GO?** See [`exit_leads_somewhere`] — a composition
    // whose HOME is this very screen has no "out", and a cursor that snapped to
    // an exit nobody drew would be a stop on empty air.
    host: Option<Res<ambition_platformer2d::game_shell::ShellHostConfiguration>>,
    mut last_mouse: Local<Option<Vec2>>,
    mut driving_finger: Local<Option<u64>>,
) {
    let layout = current_layout(&windows, &fighters);
    let exit = exit_leads_somewhere(host.as_deref());
    // ⭐ **the LAYOUT says where things are; the SCREEN says what is
    // REACHABLE.** Same split `token_rect` already makes for an absent slot's
    // token: the rectangle exists, and nobody may press it.
    let targets: Vec<(SelectTarget, HitRect)> = layout
        .targets()
        .into_iter()
        .filter(|(kind, _)| exit || *kind != SelectTarget::Back)
        .collect();
    // The cursor's "entity" is an INDEX into this list. Entity ids would be a
    // second identity for something the layout already names, and the layout's
    // order is the one that decides ties.
    let rects: Vec<CursorTarget> = targets
        .iter()
        .enumerate()
        .filter_map(|(index, (_, rect))| {
            Some(CursorTarget {
                entity: Entity::from_raw_u32(index as u32)?,
                rect: *rect,
            })
        })
        .collect();
    let kind_of = |entity: Entity| {
        rects
            .iter()
            .position(|target| target.entity == entity)
            .and_then(|index| targets.get(index))
            .map(|(kind, _)| *kind)
    };

    // The cursor starts on the first portrait rather than at the origin. A
    // pointer parked in a corner makes the first press cross the whole screen,
    // and there is no way to tell that from "the cursor is broken".
    if !pointer.placed {
        if let Some(rect) = layout.portrait(0) {
            pointer.move_to(rect.center());
        }
    }

    // ── the mouse ────────────────────────────────────────────────────────
    // ⚠ only a MOVE counts. A stationary mouse reporting the same position
    // every frame must not fight the arrow keys for the cursor — that is the
    // snap-back bug `SeatActiveDevices` exists for. Local rather than read
    // from that resource because this screen needs the POSITION of the move,
    // not just the fact of it.
    if let Some(position) = windows.iter().next().and_then(Window::cursor_position) {
        if last_mouse.is_none_or(|previous| previous.distance_squared(position) > 0.01) {
            pointer.move_to(position);
        }
        *last_mouse = Some(position);
    }

    // ── the finger ───────────────────────────────────────────────────────
    // A touch reports a POSITION, so it is the mouse's arm and not the pad's:
    // `Touches` already speaks logical window pixels with a top-left origin,
    // which is the space `HitRect` is measured in, so there is nothing to
    // convert. This is the fifth driver of one `move_to`, not a touch path.
    //
    // ⭐ **no move-gate, unlike the mouse.** A stationary mouse reports the same
    // position forever and would fight the arrows for the cursor; a touch
    // position exists only while a finger is on the glass, so there is no
    // stale report to suppress — and gating on travel would skip the frame a
    // tap ARRIVES on (a fresh touch's delta is zero), arbitrating the press at
    // wherever the cursor used to be.
    //
    // ⚠ **ONE finger drives it, and it keeps driving until it lifts.** Four
    // people share one cursor here, exactly as they share one mouse, and a
    // second button does not relocate a mouse — so a second finger neither
    // moves the cursor nor clicks. Re-choosing the driver every frame is not
    // enough to promise that: `Touches::iter` walks a `HashMap`, so "the first
    // one" is arbitrary, and Android RECYCLES pointer ids, so a finger that
    // lands after another lifts can be handed an id BELOW one still down —
    // "lowest id wins" would hand the cursor over mid-drag and drop somebody's
    // token on the wrong fighter. Lowest id only breaks the tie when there is
    // no driver yet, where it is a deterministic answer instead of a lucky one.
    let finger = touches.as_deref().and_then(|touches| {
        let driving = driving_finger
            .and_then(|id| {
                // A lift is still this finger's frame: the release edge that
                // ends a drag has to land where the finger actually left.
                touches
                    .get_pressed(id)
                    .or_else(|| touches.iter_just_released().find(|touch| touch.id() == id))
            })
            .or_else(|| touches.iter().min_by_key(|touch| touch.id()))
            .or_else(|| touches.iter_just_released().min_by_key(|touch| touch.id()))
            .copied();
        *driving_finger = driving
            .map(|touch| touch.id())
            .filter(|id| touches.get_pressed(*id).is_some());
        driving
    });
    if let Some(finger) = finger {
        pointer.move_to(finger.position());
    }

    // ── the arrows, d-pad and stick ──────────────────────────────────────
    // The union over every seat: four people share one cursor, so any of them
    // may move it. `MenuControlFrame`'s directions are just-pressed EDGES, and
    // the global frame is included because a keyboard on a route that declared
    // no seats still reports there.
    let mut direction = Vec2::ZERO;
    let mut clicked = false;
    let mut back = false;
    let mut back_out = false;
    let mut frames: Vec<ambition_platformer2d::input::MenuControlFrame> = Vec::new();
    if let Some(seat_frames) = seat_frames.as_deref() {
        for seat in 0..MAX_SMASH_SEATS as u8 {
            frames.push(seat_frames.for_seat(seat));
        }
    }
    if let Some(global) = global_frame.as_deref() {
        frames.push(*global);
    }
    for frame in frames {
        if frame.left {
            direction.x -= 1.0;
        }
        if frame.right {
            direction.x += 1.0;
        }
        if frame.up {
            direction.y -= 1.0;
        }
        if frame.down {
            direction.y += 1.0;
        }
        clicked |= frame.select;
        back |= frame.back;
        // ⛔⛔ **ESCAPE IS BOTH `Start` AND `MenuBack`** — one key, two semantic
        // actions, and `presets.rs` binds it to both on purpose (`rebind.rs`
        // documents it and tests it). The shell's pause menu opens on `start`
        // and this screen's chain runs in the SAME set with no order between
        // them, so a bare `back` here would have Escape open the pause menu AND
        // quit the lobby out from under it, deterministically wrong either way
        // the set happened to schedule.
        //
        // ⚠ **per FRAME, not over the union.** The pair is a property of one
        // seat's press — the seat holding a pad sends East with `start` clear
        // and still leaves, on the same tick somebody else opens the menu.
        //
        // ⭐ **the answer is in the frame, not in the shell.** Asking
        // `ShellPauseMenu` whether it is open would need a feature edge this
        // demo is not allowed to have (`basic_shell_presentation` is not in
        // `all_capabilities`), and the frame already carries the fact.
        back_out |= frame.back && !frame.start;
    }
    if direction != Vec2::ZERO {
        if let Some(entity) = cursor::snap(pointer.position, direction, &rects) {
            if let Some(target) = rects.iter().find(|target| target.entity == entity) {
                pointer.move_to(target.rect.center());
            }
        }
    }

    // ── clicks ───────────────────────────────────────────────────────────
    let mut pressed = clicked;
    let mut released = false;
    if let Some(mouse) = mouse.as_deref() {
        pressed |= mouse.just_pressed(MouseButton::Left);
        released |= mouse.just_released(MouseButton::Left);
    }
    // Touching the glass IS the button — and only the driving finger's edges
    // count, so a second finger landing during a drag cannot arbitrate a press
    // at the first one's position.
    if let (Some(touches), Some(finger)) = (touches.as_deref(), finger) {
        pressed |= touches.just_pressed(finger.id());
        released |= touches.just_released(finger.id());
    }

    // Back puts a carried token down where it came from, which is the only undo
    // this screen needs: the pick it would have replaced is untouched.
    if back && pointer.carrying.is_some() {
        pointer.drop_it();
        return;
    }

    // **AND WITH AN EMPTY HAND, BACK LEAVES.**
    //
    // ⭐ **graded, and the order above is the grading.** This is the same rung
    // every fighting game's select screen has: the first Back puts your token
    // down, the second backs you out. Reversing them would make one press on a
    // held token cost the whole lobby.
    //
    // ⚠ **ANY seat may press it**, which is this screen's own rule and not a new
    // one. Four people share ONE cursor here — the directions, the clicks and
    // the token-drop above are all a union over every seat's frame, because the
    // screen is a single shared surface rather than four private ones. A quit
    // that only seat 0 could reach would be the one decision on this screen with
    // a different arbitration, and on a couch the person who wants out is as
    // often on pad 2. Nothing is lost by being wrong: arriving here resets the
    // lobby anyway, so re-entering costs the picks that a `Random` default
    // re-seeds in one frame.
    if exit && back_out && !leave.0 {
        leave.0 = true;
    }

    let sources = devices
        .as_deref()
        .map(|devices| {
            crate::select::seats_offered_under(
                devices,
                ambition_platformer2d::input::sources::InputAssignmentPolicy::JoinToClaim,
            )
        })
        .unwrap_or(1);

    if pressed {
        let over = cursor::hovered(pointer.position, &rects).and_then(kind_of);
        // A PLACED token is checked first: it sits on top of the portrait it
        // chose, and a press there means "pick this up", not "choose this
        // again". Its resting home is in the target list; where it currently
        // sits is the decision's answer, not the layout's.
        let on_token = (0..MAX_SMASH_SEATS).find(|slot| {
            token_rect(&layout, &select, *slot).is_some_and(|rect| rect.contains(pointer.position))
        });
        match (pointer.carrying, on_token, over) {
            // Picking up.
            (None, Some(slot), _) => pointer.grab(slot),
            // Placing.
            // ⚠ a CELL, not a fighter index. `SmashRoster::cell` is the one
            // place that knows the grid's last square is RANDOM; a click that
            // lands past the end of the grid chooses nothing rather than
            // clamping onto whoever is last.
            (Some(slot), _, Some(SelectTarget::Portrait(cell))) => {
                if let Some(pick) = fighters.cell(cell) {
                    select.set_pick(slot, pick);
                }
                pointer.drop_it();
            }
            // Anywhere else with something in hand: put it back. Dropping a
            // token on empty space returns it rather than clearing the slot —
            // losing a fighter to a misclick is the one thing a select screen
            // must not do to somebody holding a controller.
            (Some(_), _, _) => {
                pointer.drop_it();
            }
            (None, None, Some(SelectTarget::RoleButton(slot))) => {
                select.cycle_occupant(slot, sources);
            }
            (None, None, Some(SelectTarget::Start)) => {
                if select.ready() {
                    start.0 = true;
                }
            }
            // ⚠ **no readiness term.** START is refused on an undecided lobby;
            // BACK is exactly what an undecided lobby is for.
            (None, None, Some(SelectTarget::Back)) => {
                leave.0 = true;
            }
            (None, None, _) => {}
        }
    } else if released && pointer.release_should_drop() {
        // A mouse DRAG: press on the token, move, let go over a portrait.
        let over = cursor::hovered(pointer.position, &rects).and_then(kind_of);
        if let (Some(slot), Some(SelectTarget::Portrait(cell))) = (pointer.carrying, over) {
            if let Some(pick) = fighters.cell(cell) {
                select.set_pick(slot, pick);
            }
        }
        pointer.drop_it();
    }
}

/// **Where a slot's token is right now**, ignoring the cursor's hand.
///
/// `None` for a slot nobody is at — an absent slot has no token to grab, and
/// returning its pool rect anyway would let a click on empty space pick up a
/// player who is not there.
pub fn token_rect(layout: &SelectLayout, select: &SmashSelect, slot: usize) -> Option<HitRect> {
    let card = select.slot(slot);
    if !card.occupant.participates() {
        return None;
    }
    // **A TOKEN RESTS ON A PORTRAIT, OR AT HOME — NEVER ON RANDOM.**
    //
    // ⛔ deriving the resting place from the pick moved a token NOBODY MOVED:
    // joining a slot seats it on random (Jon, 2026-08-07), and a token that
    // jumps onto the random square by itself takes the drag affordance with it —
    // the player's own card is then empty and there is nothing to pick up.
    //
    // ⚠ so random is not a resting place. The square still lights up in the
    // owner's colour and the card says `RANDOM` with the random icon, which is
    // the feedback; what does not happen is the screen rearranging itself
    // around a choice the player has not made yet. Dragging ONTO random is the
    // same: the token goes home, because there is no portrait under it.
    match card
        .pick
        .and_then(SlotPick::fighter)
        .and_then(|index| layout.portrait(index))
    {
        Some(cell) => Some(token_rect_over(cell, slot)),
        None => Some(layout.token_home(slot)),
    }
}

/// Where a slot's token sits once it is ON a portrait.
///
/// Offset per slot so two players who chose the same fighter are both visible;
/// two on one character is legal, and a stack of one would read as a lost token.
fn token_rect_over(cell: HitRect, slot: usize) -> HitRect {
    let spread = TOKEN_PX * 0.62;
    let offset = Vec2::new(
        (slot as f32 - 1.5) * spread,
        cell.size().y * 0.5 - TOKEN_PX * 0.9,
    );
    HitRect::from_center_size(cell.center() + offset, Vec2::splat(TOKEN_PX))
}

/// **Put every anchored node where the layout says it goes.**
pub fn place_the_screen(
    fighters: Res<SmashRoster>,
    windows: Query<&Window>,
    mut nodes: Query<(&Anchored, &mut Node)>,
) {
    let layout = current_layout(&windows, &fighters);
    for (anchor, mut node) in &mut nodes {
        let rect = match *anchor {
            Anchored::Title => Some(layout.title()),
            Anchored::Prompt => Some(layout.prompt()),
            Anchored::Portrait(index) => layout.portrait(index),
            Anchored::Card(slot) => Some(layout.card(slot)),
            Anchored::RoleButton(slot) => Some(layout.role_button(slot)),
            Anchored::CardPortrait(slot) => Some(layout.card_portrait(slot)),
            Anchored::Start => Some(layout.start_button()),
            Anchored::Back => Some(layout.back_button()),
        };
        if let Some(rect) = rect {
            set_rect(&mut node, rect);
        }
    }
}

/// **Draw the decision.**
///
/// In place and change-gated. Nothing here spawns or despawns: see
/// [`spawn_select_screen`].
#[allow(clippy::too_many_arguments)]
pub fn update_the_select_screen(
    select: Res<SmashSelect>,
    pointer: Res<SelectCursor>,
    fighters: Res<SmashRoster>,
    // **THE ENGINE'S REFUSAL, if it has one.** See the prompt below: this is the
    // only surface in the product that can say a decided roster could not be
    // built, and until it read this the answer to that was an empty stage.
    // ⚠ ONE param, three resources: this system is at Bevy's 16-param ceiling
    // and a fourth resource here is a compile error, not a style choice.
    //   `.0` the engine's refusal, if it has one — see the prompt below.
    //   `.1`/`.2` WHICH device each seated slot holds, for the role button.
    lobby_facts: (
        Option<Res<ambition_platformer2d::actors::character_runtime::MatchPreparationProblems>>,
        Option<Res<ambition_platformer2d::input::LocalDeviceOrder>>,
        Option<Res<ambition_platformer2d::input::sources::InputAssignmentPolicy>>,
    ),
    // Required for the same reason `spawn_select_screen`'s is; see there.
    art: ScreenArt,
    windows: Query<&Window>,
    mut cells: Query<
        (&PortraitCell, &mut BorderColor),
        (Without<SlotCardFrame>, Without<StartButton>),
    >,
    mut cards: Query<
        (&SlotCardFrame, &mut BorderColor),
        (Without<PortraitCell>, Without<StartButton>),
    >,
    mut start_button: Query<
        &mut BorderColor,
        (
            With<StartButton>,
            Without<PortraitCell>,
            Without<SlotCardFrame>,
        ),
    >,
    mut role_labels: Query<
        (&RoleButtonLabel, &mut Text),
        (Without<CardName>, Without<SelectPrompt>),
    >,
    mut card_names: Query<
        (&CardName, &mut Text, &mut TextColor),
        (Without<RoleButtonLabel>, Without<SelectPrompt>),
    >,
    mut card_portraits: Query<(&CardPortrait, &mut ImageNode, &mut Visibility), Without<SlotToken>>,
    mut prompt: Query<
        &mut Text,
        (
            With<SelectPrompt>,
            Without<RoleButtonLabel>,
            Without<CardName>,
        ),
    >,
    mut monograms: Query<
        (&PortraitMonogram, &mut Visibility),
        (Without<SlotToken>, Without<CardPortrait>),
    >,
    mut tokens: Query<(&SlotToken, &mut Node, &mut Visibility), Without<CardPortrait>>,
    mut cursor_node: Query<&mut Node, (With<CursorNode>, Without<SlotToken>)>,
) {
    let (refusal, devices, assignment) = lobby_facts;
    let catalog = Some(&*art.catalog);
    let layout = current_layout(&windows, &fighters);

    // A cell wears the colour of whoever chose it, so the grid answers "who
    // took Sanic" without reading four cards.
    for (cell, mut border) in &mut cells {
        let owner = (0..MAX_SMASH_SEATS).find(|slot| {
            let card = select.slot(*slot);
            card.occupant.participates()
                && card.pick.map(|pick| cell_of(pick, &fighters)) == Some(cell.0)
        });
        set_border(
            &mut border,
            owner.map_or(PANEL_EDGE, |slot| SLOT_COLORS[slot]),
        );
    }

    for (card, mut border) in &mut cards {
        let lit = select.slot(card.0).occupant.participates();
        set_border(
            &mut border,
            if lit { SLOT_COLORS[card.0] } else { PANEL_EDGE },
        );
    }

    for mut border in &mut start_button {
        set_border(&mut border, if select.ready() { INK } else { PANEL_EDGE });
    }

    // The LIVE policy, not a second copy of the constant: this demo claims
    // `JoinToClaim` on its own routes (`lib.rs`), and the screen only runs on
    // one of them, so reading the resource is the same answer without being a
    // second statement of it.
    let naming = devices
        .as_deref()
        .map(|devices| (devices, assignment.as_deref().copied().unwrap_or_default()));
    for (label, mut text) in &mut role_labels {
        let next = role_button_text(select.slot(label.0).occupant, naming);
        if text.0 != next {
            text.0 = next;
        }
    }

    for (name, mut text, mut color) in &mut card_names {
        let card = select.slot(name.0);
        let shown = card.occupant.participates().then_some(card.pick).flatten();
        let next = card_name_text(catalog, &fighters, shown);
        if text.0 != next {
            text.0 = next;
        }
        let next_color = if shown.is_some() { INK } else { DIM_INK };
        if color.0 != next_color {
            color.0 = next_color;
        }
    }

    for (portrait, mut image, mut visibility) in &mut card_portraits {
        let card = select.slot(portrait.0);
        let shown = card
            .occupant
            .participates()
            .then_some(card.pick)
            .flatten()
            .and_then(|pick| match pick {
                SlotPick::Fighter(index) => fighters.get(index).and_then(|id| art.portrait(id)),
                // ⚠ **the random ICON, never a fighter's face.** The draw has
                // not happened — it happens when the match starts — so any
                // portrait here would be the screen inventing an answer it does
                // not have. The square's own art is the honest one: it says
                // "this seat is a surprise", which is exactly what the seat is.
                SlotPick::Random => random_icon(&art).map(|handle| (handle, None)),
            });
        match shown {
            Some((handle, rect)) => {
                if image.image != handle {
                    image.image = handle;
                }
                if image.rect != rect {
                    image.rect = rect;
                }
                set_visibility(&mut visibility, Visibility::Inherited);
            }
            None => set_visibility(&mut visibility, Visibility::Hidden),
        }
    }

    // **THE INITIALS SHOW ONLY WHERE THE ART DID NOT ARRIVE.**
    //
    // ⛔ the first draft drew them under every cell unconditionally and they
    // showed THROUGH the portraits, which have transparent backgrounds — a
    // ghostly "DB" behind Duelist B. Found by looking at the capture.
    for (mono, mut visibility) in &mut monograms {
        let missing = match (&mono.0, art.asset_server.as_deref()) {
            (None, _) => true,
            (Some(handle), Some(server)) => server
                .get_load_state(handle.id())
                .is_none_or(|state| state.is_failed()),
            // No asset server at all is a headless fixture; claiming the art is
            // missing would be true and useless.
            (Some(_), None) => false,
        };
        set_visibility(
            &mut visibility,
            if missing {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
        );
    }

    for mut text in &mut prompt {
        // ⛔ **A REFUSAL OUTRANKS BOTH**, and having nowhere to say it was the
        // last half of "a permanent failure must never present as a wait".
        // Preparation names every reason a roster cannot become a match — an
        // unregistered fighter, a CPU with no brain profile, a replay seat this
        // build cannot drive — before one entity exists. Nothing in the product
        // read it, so the screen kept offering START, the match never opened,
        // and the player was looking at a deadlock wearing an invitation.
        //
        // ⚠ the screen's own `blocker()` answers a DIFFERENT question — what
        // this person still has to do — and stays first when it applies. This is
        // what the engine could not do with what they already chose.
        let next = if let Some(refusal) = refusal.as_deref() {
            format!("This match cannot start — {refusal}")
        } else {
            select
                .blocker()
                .map(str::to_string)
                .unwrap_or_else(|| "Ready — click START".to_string())
        };
        if text.0 != next {
            text.0 = next;
        }
    }

    // A token is over its chosen portrait, in the cursor's hand, or resting in
    // the pool. Written every frame from the layout rather than remembered, so a
    // resized window carries the tokens with it.
    for (token, mut node, mut visibility) in &mut tokens {
        let Some(resting) = token_rect(&layout, &select, token.0) else {
            set_visibility(&mut visibility, Visibility::Hidden);
            continue;
        };
        set_visibility(&mut visibility, Visibility::Inherited);
        let rect = if pointer.carrying == Some(token.0) {
            HitRect::from_center_size(pointer.position, Vec2::splat(TOKEN_PX))
        } else {
            resting
        };
        set_rect(&mut node, rect);
    }

    // ⚠ **the cursor is placed HERE, not only by `drive_the_cursor`.** Driving
    // is gated on this screen owning its input — correct, because a pause menu
    // over the top must take the presses — but a cursor that stops being DRAWN
    // when something covers the screen is a cursor that has vanished. This
    // always runs, and it is also what puts the pointer somewhere real on the
    // very first frame.
    let home = layout.portrait(0).map(HitRect::center);
    for mut node in &mut cursor_node {
        let at = if pointer.placed {
            pointer.position
        } else {
            home.unwrap_or(layout.viewport * 0.5)
        };
        set_rect(
            &mut node,
            HitRect::from_center_size(at, Vec2::splat(CURSOR_PX)),
        );
    }
}

fn set_border(border: &mut BorderColor, color: Color) {
    let next = BorderColor::all(color);
    if *border != next {
        *border = next;
    }
}

fn set_visibility(visibility: &mut Visibility, next: Visibility) {
    if *visibility != next {
        *visibility = next;
    }
}

fn set_rect(node: &mut Node, rect: HitRect) {
    let size = rect.size();
    for (field, value) in [
        (&mut node.left, Val::Px(rect.min.x)),
        (&mut node.top, Val::Px(rect.min.y)),
        (&mut node.width, Val::Px(size.x)),
        (&mut node.height, Val::Px(size.y)),
    ] {
        if *field != value {
            *field = value;
        }
    }
}

#[cfg(test)]
mod touch_tests {
    use super::*;
    use crate::select::SlotOccupant;
    use bevy::input::touch::{TouchInput, TouchPhase, Touches};

    /// The screen, driven headlessly, with the REAL touch path in front of it.
    ///
    /// ⚠ nothing here writes `Touches` directly — it cannot, the collections are
    /// private, and that is a mercy: the test sends the `TouchInput` messages
    /// winit emits and lets Bevy's own `touch_screen_input_system` fold them, so
    /// a fixture that stopped resembling Android would stop compiling rather
    /// than stay green.
    ///
    /// No window and no `UiPlugin`: the rectangles come from [`layout`], which
    /// lays out against `HEADLESS_VIEWPORT` when there is none.
    fn screen() -> App {
        let mut app = App::new();
        app.init_resource::<SmashSelect>();
        app.init_resource::<SmashRoster>();
        app.init_resource::<SelectCursor>();
        app.init_resource::<StartRequested>();
        app.init_resource::<LeaveRequested>();
        app.init_resource::<Touches>();
        app.add_message::<TouchInput>();
        app.add_systems(PreUpdate, bevy::input::touch::touch_screen_input_system);
        app.add_systems(Update, drive_the_cursor);
        app
    }

    fn finger(app: &mut App, id: u64, phase: TouchPhase, at: Vec2) {
        app.world_mut()
            .resource_mut::<Messages<TouchInput>>()
            .write(TouchInput {
                phase,
                position: at,
                window: Entity::PLACEHOLDER,
                force: None,
                id,
            });
    }

    fn headless_layout() -> SelectLayout {
        SelectLayout::for_viewport(None, SmashRoster::default().cell_count())
    }

    /// Where slot 0's token is sitting, asked of the same function the screen
    /// hit-tests with.
    fn token_of_slot_zero(app: &App, layout: &SelectLayout) -> HitRect {
        token_rect(layout, app.world().resource::<SmashSelect>(), 0)
            .expect("slot 0 is participating, so it owns a token")
    }

    /// **A FINGER PLAYS THIS SCREEN.**
    ///
    /// Tap the token, tap a portrait — the two-tap idiom a pad already uses,
    /// which is the one a finger can perform without a hover state. Every
    /// assertion below is a seam a touch has to cross: the cursor moved to the
    /// finger, the press edge arrived, the lift did NOT undo the pick-up, and
    /// the second tap committed the choice.
    #[test]
    fn a_finger_moves_the_cursor_and_chooses_a_fighter() {
        let mut app = screen();
        app.world_mut()
            .resource_mut::<SmashSelect>()
            .set_occupant(0, SlotOccupant::Controller { device: 0 });

        let layout = headless_layout();
        let token = token_of_slot_zero(&app, &layout);
        let portrait = layout.portrait(1).expect("the default roster draws a grid");

        // A frame with nothing touching, so the cursor's initial placement is
        // spent before the finger arrives and cannot be mistaken for its work.
        app.update();
        assert_ne!(
            app.world().resource::<SelectCursor>().position,
            token.center(),
            "the cursor already sat on the token, so this test cannot see a \
             finger move it"
        );

        finger(&mut app, 7, TouchPhase::Started, token.center());
        app.update();
        assert_eq!(
            app.world().resource::<SelectCursor>().position,
            token.center(),
            "a finger on slot 0's token did not move the cursor to it"
        );
        assert_eq!(
            app.world().resource::<SelectCursor>().carrying,
            Some(0),
            "the touch press never reached the screen's click arbitration"
        );

        // Lifting without travelling is the first half of a two-tap place, not
        // a drop — the same rule that keeps a pad's pick-up in hand.
        finger(&mut app, 7, TouchPhase::Ended, token.center());
        app.update();
        assert_eq!(
            app.world().resource::<SelectCursor>().carrying,
            Some(0),
            "lifting the finger put the token straight back down"
        );

        finger(&mut app, 8, TouchPhase::Started, portrait.center());
        app.update();
        assert_eq!(
            app.world().resource::<SelectCursor>().position,
            portrait.center(),
            "the second tap did not move the cursor onto the portrait"
        );
        assert_eq!(
            app.world().resource::<SmashSelect>().slot(0).pick,
            Some(SlotPick::Fighter(1)),
            "a finger tapped a portrait and the slot did not take that fighter"
        );
    }

    /// **AND A FINGER CAN DRAG**, which is the idiom Jon's spec actually names:
    /// *"the cursor can pick up and drag to select a character"*.
    ///
    /// ⚠ this is the half that needs the RELEASE edge, and the release edge is
    /// the one a lifted finger nearly loses: by the time it fires the touch is
    /// gone from `Touches::iter`, so a driver that only ever looks at what is
    /// still down reports nothing and the token never lands.
    #[test]
    fn a_finger_can_drag_a_token_onto_a_portrait_in_one_stroke() {
        let mut app = screen();
        app.world_mut()
            .resource_mut::<SmashSelect>()
            .set_occupant(0, SlotOccupant::Controller { device: 0 });

        let layout = headless_layout();
        let token = token_of_slot_zero(&app, &layout);
        let portrait = layout.portrait(0).expect("the default roster draws a grid");

        finger(&mut app, 3, TouchPhase::Started, token.center());
        app.update();
        finger(&mut app, 3, TouchPhase::Moved, portrait.center());
        app.update();
        assert_eq!(
            app.world().resource::<SelectCursor>().carrying,
            Some(0),
            "the token came out of the cursor's hand part-way through the drag"
        );

        finger(&mut app, 3, TouchPhase::Ended, portrait.center());
        app.update();
        assert_eq!(
            app.world().resource::<SmashSelect>().slot(0).pick,
            Some(SlotPick::Fighter(0)),
            "the finger let go over a portrait and the token did not land on it"
        );
        assert_eq!(
            app.world().resource::<SelectCursor>().carrying,
            None,
            "the drag ended with the token still in hand"
        );
    }

    /// **A SECOND FINGER IS NOT A SECOND CURSOR.**
    ///
    /// One person drags a token; somebody else's finger — or the same person's
    /// palm — lands on a portrait. The cursor must stay where the driving finger
    /// is and the stray press must not arbitrate, or the drag ends by dropping
    /// the token wherever the intruder touched.
    ///
    /// ⚠ the intruder is given the LOWER id on purpose. Android recycles pointer
    /// ids, so a finger that lands after another lifts really can be handed an id
    /// below one still down; "the lowest id wins" would hand the cursor over here
    /// and this is the only assertion that can tell the two rules apart.
    #[test]
    fn a_second_finger_neither_moves_the_cursor_nor_clicks() {
        let mut app = screen();
        app.world_mut()
            .resource_mut::<SmashSelect>()
            .set_occupant(0, SlotOccupant::Controller { device: 0 });

        let layout = headless_layout();
        let token = token_of_slot_zero(&app, &layout);
        let portrait = layout.portrait(1).expect("the default roster draws a grid");

        finger(&mut app, 5, TouchPhase::Started, token.center());
        app.update();
        assert_eq!(
            app.world().resource::<SelectCursor>().carrying,
            Some(0),
            "the driving finger never picked the token up"
        );

        finger(&mut app, 2, TouchPhase::Started, portrait.center());
        app.update();
        assert_eq!(
            app.world().resource::<SelectCursor>().position,
            token.center(),
            "a second finger stole the cursor from the one that was dragging"
        );
        assert_eq!(
            app.world().resource::<SelectCursor>().carrying,
            Some(0),
            "the second finger's press arbitrated, so the drag let go of the token"
        );
        assert_eq!(
            app.world().resource::<SmashSelect>().slot(0).pick,
            Some(SlotPick::Random),
            "a stray finger committed a fighter nobody chose"
        );
    }
}
