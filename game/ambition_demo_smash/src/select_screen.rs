//! Smash character-select presentation and cursor interaction.
//!
//! [`layout`] owns screen geometry, [`cursor`] performs rectangle hit-testing,
//! and [`crate::select::SmashSelect`] owns selection state without pixel
//! knowledge. This module connects the three so rendered and clickable geometry
//! share one layout calculation. Character portraits resolve through the
//! catalog's existing portrait reference.

pub mod cursor;
pub mod layout;

use ambition_platformer2d::character::{
    portrait_for_declared_character, CharacterCatalog, PortraitSheetRegistry,
    PreparedCharacterRegistry,
};
use bevy::input::touch::Touches;
use bevy::prelude::*;

use crate::select::{SlotOccupant, SlotPick, SmashRoster, SmashSelect, MAX_SMASH_SEATS};
use cursor::{CursorTarget, HitRect, SelectCursors};
use layout::SelectLayout;

/// The screen's UI root. One marker, so teardown is `despawn` on a query
/// filtered by THIS owner rather than a sweep of every node — a shared marker's
/// teardown clobbers whatever else happened to carry it.
#[derive(Component)]
pub struct SmashSelectUiRoot;

/// Everything the cursor can act on through the static layout.
///
/// Tokens are deliberately not variants here: their rectangles come from
/// selection state (or from a carrier's hand), while these targets are pure
/// layout. `drive_the_cursor` arbitrates token hits before the portrait beneath
/// them.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectTarget {
    /// A portrait in the grid. Indexes [`SmashRoster`].
    Portrait(usize),
    /// The button that cycles one card between controller / CPU / absent.
    RoleButton(usize),
    /// Begin the match.
    Start,
    /// Leave the lobby — see [`LeaveRequested`].
    Back,
    /// Turn the grid back a page. Only present when the roster needs more than
    /// one — see [`SelectLayout::pages`].
    PagePrev,
    /// Turn the grid on a page.
    PageNext,
}

/// Which rectangle in the layout a node wears.
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

/// A slot's token. Its resting position is derived from the slot's selection;
/// while carried, it follows the carrier's hand.
#[derive(Component, Clone, Copy)]
pub struct SlotToken(pub usize);

/// One seat's cursor node, by seat.
#[derive(Component)]
pub struct CursorNode(pub usize);

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
/// it carries the HANDLE rather than a boolean, because "is there art" cannot
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

/// Somebody clicked START.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct StartRequested(pub bool);

/// The BACK button's frame. Never dims: leaving is always allowed, where
/// starting waits on a decided lobby.
#[derive(Component)]
pub struct BackButton;

/// Somebody asked to leave the lobby.
///
/// a REQUEST, exactly like [`StartRequested`], and for the same reason.
/// This module draws the screen and arbitrates presses; it does not name the
/// shell. Writing `ShellCommand` from here would give the screen a second
/// opinion about routing, and the one place that decides where BACK goes
/// (`leave_the_select_screen_when_asked`) would no longer be the only one.
///
/// Tap-B belongs to the select interaction state machine (it recalls the
/// owner's token); navigation out is an explicit Back control or a held-B
/// gesture. Keeping the request here lets one system arbitrate those meanings
/// before the shell sees a route change.
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
/// What a slot's role button says.
///
/// it names the DEVICE, not an index. It read `CONTROLLER 1` / `CONTROLLER 2`, which is the
/// slot's own numbering said back to it and tells nobody which thing in the room drives that card.
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
        // "RANDOM", not a fighter's name. The card must not name somebody
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

/// The random square's icon, for the grid cell AND for the card of a slot
/// that took it — one accessor, because two spellings of "what random looks
/// like" is how a screen ends up disagreeing with itself.
///
/// It is a 32x32 world TILE standing in for a portrait, so it reads heavier than the faces
/// beside it and its plate/rivets belong to a different game. Replace it with a drawn random
/// icon when one exists; nothing else has to change, because this function is the only place
/// that names it.
fn random_icon(art: &ScreenArt<'_>) -> Option<Handle<Image>> {
    art.entities.as_deref().and_then(|assets| {
        assets
            .entities
            .get(ambition_platformer2d::content::EntitySprite::BonusBlockTile)
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
        // NOT a panic and not an empty string. A grid cell with no name is a
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

/// The read-only art context needed to present a character.
///
/// This is a coherent `SystemParam`, not parameter packing: portrait resolution
/// is defined by the catalog, prepared declarations, loaded sheets/assets and
/// menu font together. Spawn and projection systems consume the same context so
/// they cannot grow separate portrait-resolution rules.
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

/// A character's face, as an image AND the rectangle to take out of it.
///
/// the registry is `Option` because the standalone smash app does not install
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
    // through the ENGINE's resolver, not straight to the catalog. A
    // character registered in Rust may name a portrait TARGET; everything that
    // names nothing keeps the catalog's derived answer, which is how all 144 of
    // today's portraits resolve. This screen is the first consumer of that road
    // grid of faces existed.
    let target = declared
        .and_then(|registry| registry.get(id))
        .and_then(|prepared| prepared.portrait.as_deref());
    let reference = portrait_for_declared_character(portraits, catalog, target, id)?;
    let handle = asset_server?.load::<Image>(reference.image.clone());
    // STILL, said out loud. This grid never ticks a frame, so it asks for the one
    // frame it draws instead of taking a clip and keeping its first — which is
    // what it used to do, and what left the choice invisible at the call site.
    let rect = portraits
        .and_then(|registry| {
            registry.resolve_still(&reference.manifest, None, Some(&reference.still_clip))
        })
        .map(|(_, frame)| Rect::from(frame));
    Some((handle, rect))
}

/// The viewport this screen is laid out for, or `None` where there is no window.
fn viewport(windows: &Query<&Window>) -> Option<Vec2> {
    windows
        .iter()
        .next()
        .map(|window| Vec2::new(window.width(), window.height()))
}

/// Does backing out of this lobby lead anywhere?
///
/// it does not in every composition, and pretending otherwise is a dead button. The standalone
/// smash demo names the select screen as its OWN home route (`ShellComposition::new(..,
/// SMASH_SELECT_ROUTE, ..)`), because leaving a match there should return to the screen that chose
/// it rather than to a launcher listing one game. `QuitToHome` in that app therefore re-enters the
/// route it is already on: a churn with nothing to see.
///
/// a fact about the COMPOSITION, so it is read from the host spec rather
/// than assumed by either app. Absent (a bare unit fixture with no host
/// configured) reads as NO exit: a screen with no shell to leave through has
/// nowhere to go, and drawing an exit for one would be the same lie.
pub fn exit_leads_somewhere(
    host: Option<&ambition_platformer2d::game_shell::ShellHostConfiguration>,
) -> bool {
    host.and_then(|host| host.spec.as_ref())
        .is_some_and(|spec| spec.home_route.as_str() != crate::SMASH_SELECT_ROUTE)
}

/// The layout this frame, from the window if there is one.
/// How much of the screen's WIDTH a fully deflected stick crosses per second.
///
/// a fraction rather than a pixel rate, so the cursor takes the same TIME to
/// cross a phone and a monitor. `1.15` puts a corner-to-corner sweep just under
/// a second on a 16:9 screen, which is about where Smash's own cursor sits.
const CURSOR_SPEED_PER_SECOND: f32 = 1.15;

/// Character-select interaction policy.
///
/// Token ownership and selection stay one state machine; policy only answers
/// which otherwise-valid token grabs this build permits. During development we
/// intentionally allow one human to move another human's token because it makes
/// controller and CPU setup much faster to test. Setting this to `false` gives
/// Ultimate's protected-human-token behavior without changing any state shape.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectInteractionPolicy {
    pub allow_other_human_token_grab: bool,
}

impl Default for SelectInteractionPolicy {
    fn default() -> Self {
        Self {
            allow_other_human_token_grab: true,
        }
    }
}

/// Which page of the grid is showing.
///
/// a resource rather than a field on [`SelectLayout`], because the layout is
/// a pure function of the viewport and must stay one — the page is a DECISION
/// somebody made, and the layout is where things are. Persisting it here also
/// means a window resize that re-pages the grid does not lose which page the
/// player was on.
///
/// clamped by the layout, not here. `SelectLayout::paged` takes whatever
/// this holds and pins it into range, so a resize from a phone to a monitor
/// (three pages down to one) shows page 0 rather than an empty grid, and this
/// resource never has to know the roster size.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SelectPage(pub usize);

pub fn current_layout(
    windows: &Query<&Window>,
    fighters: &SmashRoster,
    page: &SelectPage,
) -> SelectLayout {
    SelectLayout::paged(
        viewport(windows).unwrap_or(layout::HEADLESS_VIEWPORT),
        fighters.cell_count(),
        page.0,
    )
}

/// Build the screen.
///
/// built once and positioned every frame. Nothing here is rebuilt: a tree
/// respawned per frame throws away the handles its images resolved to and
/// restarts every load, and the positions come from [`layout`] anyway — so a
/// resize costs four numbers rather than a rebuild.
pub fn spawn_select_screen(
    mut commands: Commands,
    existing: Query<(), With<SmashSelectUiRoot>>,
    fighters: Res<SmashRoster>,
    // the catalog inside is REQUIRED, not `Option`.
    // `engine.character-authority-is-app-local` forbids making it optional by
    // name, and the reason is this screen's exact failure mode: an absent
    // catalog would draw a grid of nameless plates that looks like missing ART
    // rather than like a composition with no cast. Every composition that
    // reaches this route has one, because this demo registers its own fragment.
    art: ScreenArt,
    // WHETHER TO DRAW THE WAY OUT — see [`exit_leads_somewhere`]. A plain
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
            // a BUTTON, not only a binding. The `back` intent alone would not have answered
            // that: a mouse has no Back control at all, so a player at a desk with no pad and
            // no keyboard hand on Escape would still have been stuck in the lobby — and an
            // unlabelled press is a feature only the person who wrote it knows about.
            //
            // it never dims. START waits on a decided lobby; leaving is always
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

            // one more cell than there are fighters. The last square is
            // RANDOM, drawn below the loop so every fighter
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
                    // A MONOGRAM UNDER EVERY PORTRAIT.
                    //
                    // found by LOOKING: `mary_o` draws a hole.
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
            // It is the glyph on Mary-O's bonus block — the thing you hit without knowing what
            // comes out — which is the same promise this square makes.
            //
            // it is a cell like any other: a token rests on it, it lights up
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
                        // the same rule the portraits follow: a composition
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

            // ── THE CONTROL STRIP: prompt, page controls, START ─────────
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
                        width: Val::Px(layout::TOKEN_PX),
                        height: Val::Px(layout::TOKEN_PX),
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
            // ONE HAND PER SEAT, IN THE SEAT'S OWN COLOUR. Four identical
            // cursors would be four people asking each other which one is
            // theirs; the tokens already carry `SLOT_COLORS`, so the hand that
            // grabs one wears the same paint.
            for seat in 0..MAX_SMASH_SEATS {
                let tint = SLOT_COLORS[seat];
                root.spawn((
                    CursorNode(seat),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(-999.0),
                        top: Val::Px(-999.0),
                        width: Val::Px(layout::CURSOR_PX),
                        height: Val::Px(layout::CURSOR_PX),
                        border: UiRect::all(Val::Px(3.0)),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(tint.with_alpha(0.35)),
                    BorderColor::all(tint),
                    GlobalZIndex(640),
                    Visibility::Hidden,
                    Name::new(format!("seat {} cursor", seat + 1)),
                ));
            }
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

/// Raw sources that can drive this one frontend.
///
/// This is a coherent `SystemParam`: it groups input/environment readers, not
/// unrelated UI projections. The state machine below still names its own model
/// resources explicitly.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct SelectScreenInputs<'w, 's> {
    windows: Query<'w, 's, &'static Window>,
    mouse: Option<Res<'w, ButtonInput<MouseButton>>>,
    touches: Option<Res<'w, Touches>>,
    seat_frames: Option<Res<'w, ambition_platformer2d::input::SeatMenuFrames>>,
    global_frame: Option<Res<'w, ambition_platformer2d::input::MenuControlFrame>>,
    host: Option<Res<'w, ambition_platformer2d::game_shell::ShellHostConfiguration>>,
    time: Res<'w, Time>,
}

#[derive(Default)]
pub(crate) struct SelectDriverLocal {
    last_mouse: Option<Vec2>,
    fingers: std::collections::HashMap<u64, usize>,
    back_hold_seconds: [f32; MAX_SMASH_SEATS],
}

/// Holding Back is navigation; tapping Back is token manipulation.
const BACK_HOLD_TO_LEAVE_SECONDS: f32 = 0.55;

/// Move every seat's cursor, and act on what each one is over.
///
/// What stayed shared is the screen's own verbs — see the union at the bottom.
///
/// A mouse or a finger writes a position; a held stick roams; the arrows and
/// d-pad snap between targets. All of them write the same field — see [`cursor`]
/// for why there is one position per seat and no separate focus.
pub(crate) fn drive_the_cursor(
    mut select: ResMut<SmashSelect>,
    mut cursors: ResMut<SelectCursors>,
    mut start: ResMut<StartRequested>,
    mut leave: ResMut<LeaveRequested>,
    fighters: Res<SmashRoster>,
    mut page: ResMut<SelectPage>,
    policy: Res<SelectInteractionPolicy>,
    inputs: SelectScreenInputs,
    mut local: Local<SelectDriverLocal>,
) {
    let layout = current_layout(&inputs.windows, &fighters, &page);
    let exit = exit_leads_somewhere(inputs.host.as_deref());
    // the LAYOUT says where things are; the SCREEN says what is
    // REACHABLE. Same split `token_rect` already makes for an absent slot's
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
    // D-pad/keyboard navigation must be able to LAND on a token even though a
    // token is not a static layout target. Keep those dynamic rectangles out of
    // `rects`: hover semantics still belong to the underlying portrait, and an
    // ineligible human token must be transparent to an A press. `snap_rects`
    // exists only to give directional navigation the token's current centre.
    let mut snap_rects = rects.clone();
    for slot in 0..MAX_SMASH_SEATS {
        if cursors.carrier_of(slot).is_some() {
            continue;
        }
        let Some(rect) = token_rect(&layout, &select, &fighters, slot) else {
            continue;
        };
        let Some(entity) = Entity::from_raw_u32(snap_rects.len() as u32) else {
            continue;
        };
        snap_rects.push(CursorTarget {
            entity,
            rect: SelectLayout::touchable(rect),
        });
    }
    let kind_of = |entity: Entity| {
        rects
            .iter()
            .position(|target| target.entity == entity)
            .and_then(|index| targets.get(index))
            .map(|(kind, _)| *kind)
    };

    // WHAT ONE SEAT ASKED FOR THIS FRAME, folded from every device that
    // speaks for it. Gathered for all four before any of them is acted on, so a
    // seat cannot see a screen another seat has already changed.
    #[derive(Default, Clone, Copy)]
    struct SeatDrive {
        moved_to: Option<Vec2>,
        nav: Vec2,
        direction: Vec2,
        pressed: bool,
        released: bool,
        back: bool,
        back_held: bool,
        page_back: bool,
        page_forward: bool,
    }
    let mut drives = [SeatDrive::default(); MAX_SMASH_SEATS];

    // the mouse, the keyboard and the global frame all speak for SEAT 0.
    // A machine has one mouse and one keyboard; they are the first seat's
    // devices, and the global frame is where a keyboard on a route that
    // declared no seats reports. Pads speak for their own seats below.
    const DESKTOP_SEAT: usize = 0;

    // ── the pads ─────────────────────────────────────────────────────────
    if let Some(seat_frames) = inputs.seat_frames.as_deref() {
        for seat in 0..MAX_SMASH_SEATS {
            let frame = seat_frames.for_seat(seat as u8);
            let drive = &mut drives[seat];
            if frame.left {
                drive.direction.x -= 1.0;
            }
            if frame.right {
                drive.direction.x += 1.0;
            }
            if frame.up {
                drive.direction.y -= 1.0;
            }
            if frame.down {
                drive.direction.y += 1.0;
            }
            drive.nav = frame.nav;
            drive.pressed |= frame.select;
            drive.back |= frame.back && !frame.start;
            drive.back_held |= frame.back_held && !frame.start;
            drive.page_back |= frame.page_left;
            drive.page_forward |= frame.page_right;
            // ESCAPE IS BOTH `Start` AND `MenuBack` — one key, two
            // semantic actions, and `presets.rs` binds it to both on purpose
            // (`rebind.rs` documents it and tests it). The shell's pause menu
            // opens on `start` and this screen's chain runs in the SAME set with
            // no order between them, so a bare `back` here would have Escape
            // open the pause menu AND quit the lobby out from under it,
            // deterministically wrong either way the set happened to schedule.
            //
            // per FRAME, not over the union. The pair is a property of one
            // seat's press — the seat holding a pad sends East with `start`
            // clear and still leaves, on the same tick somebody else opens the
            // menu.
        }
    }
    if let Some(global) = inputs.global_frame.as_deref() {
        let drive = &mut drives[DESKTOP_SEAT];
        if global.left {
            drive.direction.x -= 1.0;
        }
        if global.right {
            drive.direction.x += 1.0;
        }
        if global.up {
            drive.direction.y -= 1.0;
        }
        if global.down {
            drive.direction.y += 1.0;
        }
        if global.nav.length_squared() > drive.nav.length_squared() {
            drive.nav = global.nav;
        }
        drive.pressed |= global.select;
        drive.back |= global.back && !global.start;
        drive.back_held |= global.back_held && !global.start;
        drive.page_back |= global.page_left;
        drive.page_forward |= global.page_right;
    }

    // ── the mouse ────────────────────────────────────────────────────────
    // ⚠ only a MOVE counts. A stationary mouse reporting the same position
    // every frame must not fight the arrow keys for the cursor — that is the
    // snap-back bug `SeatActiveDevices` exists for. Local rather than read
    // from that resource because this screen needs the POSITION of the move,
    // not just the fact of it.
    if let Some(position) = inputs
        .windows
        .iter()
        .next()
        .and_then(Window::cursor_position)
    {
        if local
            .last_mouse
            .is_none_or(|previous| previous.distance_squared(position) > 0.01)
        {
            drives[DESKTOP_SEAT].moved_to = Some(position);
        }
        local.last_mouse = Some(position);
    }
    if let Some(mouse) = inputs.mouse.as_deref() {
        drives[DESKTOP_SEAT].pressed |= mouse.just_pressed(MouseButton::Left);
        drives[DESKTOP_SEAT].released |= mouse.just_released(MouseButton::Left);
    }

    // no move-gate, unlike the mouse. A stationary mouse reports the same
    // position forever and would fight the arrows for the cursor; a touch
    // position exists only while a finger is on the glass, so there is no
    // stale report to suppress — and gating on travel would skip the frame a
    // tap ARRIVES on (a fresh touch's delta is zero), arbitrating the press at
    // wherever the cursor used to be.
    if let Some(touches) = inputs.touches.as_deref() {
        // A finger that has lifted stops driving — but only AFTER this frame,
        // because the release edge that ends a drag has to land where the
        // finger actually left.
        for touch in touches.iter_just_pressed() {
            if local.fingers.contains_key(&touch.id()) {
                continue;
            }
            // WHOSE TOKEN DID IT LAND ON? That seat, if nobody is already
            // driving it. This is what makes a second finger a second player
            // rather than an interruption.
            let taken: Vec<usize> = local.fingers.values().copied().collect();
            let on_token = (0..MAX_SMASH_SEATS)
                .filter(|slot| !taken.contains(slot))
                .filter_map(|slot| {
                    let rect =
                        SelectLayout::touchable(token_rect(&layout, &select, &fighters, slot)?);
                    rect.contains(touch.position())
                        .then_some((slot, rect.center().distance_squared(touch.position())))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(slot, _)| slot);
            // A SEAT MID-CARRY CLAIMS THE NEXT FINGER, and without this
            // the two-tap idiom is broken on a phone. Tap your token, tap a
            // fighter: those are two DIFFERENT fingers, because the first one
            // lifted. The second lands on a portrait, matches no free token,
            // and would fall through to seat 0 — so seat 1's tap-to-place put
            // seat 0's token down instead, and seat 1's own token went home,
            // which reads on screen as "it chose Random".
            //
            // lowest seat wins if two are somehow carrying with no finger
            // between them. A deterministic answer beats a lucky one, and the
            // case needs two people to put two tokens down and then have one of
            // them tap.
            let carrying = (0..MAX_SMASH_SEATS).find(|seat| {
                !taken.contains(seat)
                    && cursors
                        .seat(*seat)
                        .is_some_and(|cursor| cursor.carrying.is_some())
            });
            // otherwise it drives seat 0, and only if seat 0 is free. One
            // person on a phone taps portraits and buttons without ever
            // touching a token, and that has to work; a stray thumb during
            // somebody else's drag has to not.
            let seat = on_token.or(carrying).or(if taken.contains(&DESKTOP_SEAT) {
                None
            } else {
                Some(DESKTOP_SEAT)
            });
            if let Some(seat) = seat {
                local.fingers.insert(touch.id(), seat);
            }
        }
        // Android RECYCLES pointer ids, so a finger that lands after
        // another lifts can be handed an id that was in this map a moment ago.
        // Claiming happens once, above, on the just-pressed edge only — a seat
        // is never reassigned to a finger already down.
        let mut lifted: Vec<u64> = Vec::new();
        for (id, seat) in local.fingers.iter() {
            if let Some(touch) = touches.get_pressed(*id) {
                drives[*seat].moved_to = Some(touch.position());
                if touches.just_pressed(*id) {
                    drives[*seat].pressed = true;
                }
            } else if let Some(touch) = touches.iter_just_released().find(|t| t.id() == *id) {
                drives[*seat].moved_to = Some(touch.position());
                drives[*seat].released = true;
                lifted.push(*id);
            } else {
                lifted.push(*id);
            }
        }
        for id in lifted {
            local.fingers.remove(&id);
        }
    }

    // Which input participants are actually present on this surface. This is
    // deliberately derived from the per-seat menu frames: that table is already
    // produced once per `InputParticipant`, so consulting device order here would
    // create a second answer to "who can use a cursor?".
    let mut connected_sources: Vec<usize> = inputs
        .seat_frames
        .as_deref()
        .map(|frames| {
            frames
                .seats()
                .map(|(seat, _)| seat as usize)
                .filter(|seat| *seat < MAX_SMASH_SEATS)
                .collect()
        })
        .unwrap_or_default();
    // Headless/touch-only compositions may have no per-seat producer. Seat zero
    // is still the desktop/touch cursor, and active fingers below are real cursor
    // owners for the duration of their gesture.
    if connected_sources.is_empty() {
        connected_sources.push(DESKTOP_SEAT);
    }
    connected_sources.extend(local.fingers.values().copied());
    connected_sources.sort_unstable();
    connected_sources.dedup();

    // ── the page, which is the ONE part of the grid every seat shares ────
    // clamped against the LAYOUT's count, not against a remembered one.
    // How many pages exist is a fact about the viewport and the roster, and the
    // layout is the one thing that derives it; a page number that outran a
    // resize would show an empty grid until somebody pressed something.
    let last_page = layout.pages.saturating_sub(1);
    if drives.iter().any(|drive| drive.page_back) {
        page.0 = page.0.saturating_sub(1);
    }
    if drives.iter().any(|drive| drive.page_forward) {
        page.0 = (page.0 + 1).min(last_page);
    }

    // ── each seat, in seat order ─────────────────────────────────────────
    for seat in 0..MAX_SMASH_SEATS {
        let drive = drives[seat];
        // WHICH CARD IS THIS PERSON'S? Not `seat` — see
        // [`SmashSelect::slot_driven_by`]. The cursor stays seat-keyed because a
        // hand belongs to a person; the CARD is whichever one names this seat's
        // input source, and with a CPU sitting between two people those two
        // indices come apart.
        let own_slot = select.slot_driven_by(seat);

        // Movement is one short mutable borrow. Token arbitration below needs
        // the whole cursor table, so holding `seat_mut` across the state machine
        // would make ownership harder to express than it is.
        {
            let pointer = cursors
                .seat_mut(seat)
                .expect("`seat` is bounded by the loop over 0..MAX_SMASH_SEATS");

            // The cursor starts on the first portrait rather than at the origin. A
            // pointer parked in a corner makes the first press cross the whole
            // screen, and there is no way to tell that from "the cursor is broken".
            //
            // spread per seat, so four cursors do not begin as one cursor:
            // four hands stacked on cell zero look like a bug in the drawing.
            if !pointer.placed {
                if let Some(rect) = layout.portrait(seat.min(layout.characters.saturating_sub(1))) {
                    pointer.move_to(rect.center());
                }
            }

            if let Some(position) = drive.moved_to {
                pointer.move_to(position);
            }

            // A HELD STICK ROAMS; A TAP STILL SNAPS.
            //
            // ⛔⛔ AND FOR MONTHS IT DID NOT — THE SNAP BRANCH WAS UNREACHABLE ON
            // EVERY REAL DEVICE, which is what *"the controls don't feel good,
            // they are very hard to use with a gamepad"* was. `nav` is not just
            // the stick: `decode_menu_frame` folds the HELD d-pad and the held
            // arrow keys into it too (`held_x`/`held_y`), and a direction EDGE
            // implies the same direction is held on that very frame. So
            // `nav != ZERO` was true on every frame any edge could fire, the
            // roam always won, and nothing ever snapped — on a stick, a d-pad or
            // a keyboard. The comment above stated the rule; the code could not
            // reach it.
            //
            // ⭐ THE EDGE GOES FIRST NOW, which is the rule as written: a flick
            // lands on the next portrait's centre, and a deflection that is
            // still held between repeats roams freely, so Smash's hand survives
            // for the player who wants to sweep the grid.
            if drive.direction != Vec2::ZERO {
                if let Some(entity) = cursor::snap(pointer.position, drive.direction, &snap_rects) {
                    if let Some(target) = snap_rects.iter().find(|target| target.entity == entity) {
                        pointer.move_to(target.rect.center());
                    }
                }
            } else if drive.nav != Vec2::ZERO {
                let travel = drive.nav
                    * layout.viewport.x
                    * CURSOR_SPEED_PER_SECOND
                    * inputs.time.delta_secs();
                let roamed = pointer.position + travel;
                pointer.move_to(Vec2::new(
                    roamed.x.clamp(0.0, layout.viewport.x),
                    roamed.y.clamp(0.0, layout.viewport.y),
                ));
            }
        }

        // Tap Back is token manipulation; holding Back is navigation out of
        // the screen. The hold timer is per input seat so one player's held B
        // cannot borrow another player's edge.
        if drive.back_held {
            local.back_hold_seconds[seat] += inputs.time.delta_secs();
            if exit && local.back_hold_seconds[seat] >= BACK_HOLD_TO_LEAVE_SECONDS {
                leave.0 = true;
            }
        } else {
            local.back_hold_seconds[seat] = 0.0;
        }

        if drive.back {
            // Ultimate's tap-B behavior: with an empty hand, the HAND returns to
            // its own placed token and picks that token up. B while carrying any
            // token is a no-op. If another cursor currently carries this token
            // (possible while the development policy permits it), the owner does
            // not steal it mid-drag.
            if cursors
                .seat(seat)
                .expect("`seat` is bounded by the loop over 0..MAX_SMASH_SEATS")
                .carrying
                .is_none()
            {
                if let Some(own) = own_slot.filter(|slot| cursors.carrier_of(*slot).is_none()) {
                    let card = select.slot(own);
                    if let Some(pick) = card.pick {
                        let cell = cell_of(pick, &fighters);
                        let target_page = cell / layout.per_page();
                        if page.0 != target_page {
                            page.0 = target_page;
                        }
                        let token_layout = SelectLayout::paged(
                            layout.viewport,
                            fighters.cell_count(),
                            target_page,
                        );
                        if let Some(rect) = token_rect(&token_layout, &select, &fighters, own) {
                            let pointer = cursors
                                .seat_mut(seat)
                                .expect("`seat` is bounded by the loop over 0..MAX_SMASH_SEATS");
                            pointer.move_to(rect.center());
                            cursors.try_grab(seat, own);
                        }
                    }
                }
            }
            continue;
        }

        let position = cursors
            .seat(seat)
            .expect("`seat` is bounded by the loop over 0..MAX_SMASH_SEATS")
            .position;
        let carrying = cursors
            .seat(seat)
            .expect("`seat` is bounded by the loop over 0..MAX_SMASH_SEATS")
            .carrying;
        let release_should_drop = cursors
            .seat(seat)
            .expect("`seat` is bounded by the loop over 0..MAX_SMASH_SEATS")
            .release_should_drop();

        if drive.pressed {
            let over = cursor::hovered(position, &rects).and_then(kind_of);
            // Tokens sit over portraits, so token eligibility is checked before
            // the underlying portrait. Own and CPU tokens are always grabbable;
            // other-human tokens are a policy knob that defaults ON for testing.
            // A non-grabbable human token is transparent: the portrait beneath it
            // remains the A target.
            let may_grab = |slot: usize| match select.slot(slot).occupant {
                SlotOccupant::Absent => false,
                SlotOccupant::Cpu => true,
                SlotOccupant::Controller { device } if device == seat => true,
                SlotOccupant::Controller { .. } => policy.allow_other_human_token_grab,
            };
            let on_token = (0..MAX_SMASH_SEATS)
                .filter(|slot| may_grab(*slot))
                .filter_map(|slot| {
                    let rect =
                        SelectLayout::touchable(token_rect(&layout, &select, &fighters, slot)?);
                    rect.contains(position)
                        .then_some((slot, rect.center().distance_squared(position)))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(slot, _)| slot);
            match (carrying, on_token, over) {
                // Token hit-testing wins over the portrait beneath it. Mechanical
                // exclusivity is enforced again by `try_grab`, so two hands
                // converging on one CPU/testing token still produce one carrier.
                (None, Some(slot), _) => {
                    cursors.try_grab(seat, slot);
                }
                // PRESSING A FIGHTER WITH AN EMPTY HAND CHOOSES IT. A
                // connected participant does not need to visit a role button
                // first: if this source has no match slot yet, the same action
                // atomically claims the first absent card and chooses the fighter.
                // The source→slot relation remains explicit; we never write card
                // `seat` merely because the cursor happens to be seat-keyed.
                (None, _, Some(SelectTarget::Portrait(cell))) => {
                    if let Some(pick) = fighters.cell(cell) {
                        if let Some(own) = select.slot_for_or_claim(seat) {
                            select.set_pick(own, pick);
                        }
                    }
                }
                // TURNING THE PAGE IS LEGAL WITH A TOKEN IN HAND, and it has
                // to be: the fighter you are carrying it to may be on another
                // page, and having to put the token down to go looking would
                // make a paged grid worse than an unpaged one.
                (_, _, Some(SelectTarget::PagePrev)) => {
                    page.0 = page.0.saturating_sub(1);
                }
                (_, _, Some(SelectTarget::PageNext)) => {
                    page.0 = (page.0 + 1).min(last_page);
                }
                // Placing.
                // a CELL, not a fighter index. `SmashRoster::cell` is the one
                // place that knows the grid's last square is RANDOM; a click that
                // lands past the end of the grid chooses nothing rather than
                // clamping onto whoever is last.
                (Some(slot), _, Some(SelectTarget::Portrait(cell))) => {
                    if let Some(pick) = fighters.cell(cell) {
                        select.set_pick(slot, pick);
                        cursors
                            .seat_mut(seat)
                            .expect("`seat` is bounded by the loop over 0..MAX_SMASH_SEATS")
                            .drop_it();
                    }
                }
                // Empty space and unrelated controls do not invent a third token
                // state. A token is either carried or placed on its selection.
                (Some(_), _, _) => {}
                (None, None, Some(SelectTarget::RoleButton(slot))) => {
                    select.cycle_role(slot, seat, &connected_sources);
                }
                (None, None, Some(SelectTarget::Start)) => {
                    if select.ready() {
                        start.0 = true;
                    }
                }
                // no readiness term. START is refused on an undecided
                // lobby; BACK is exactly what an undecided lobby is for.
                (None, None, Some(SelectTarget::Back)) => {
                    leave.0 = true;
                }
                (None, None, _) => {}
            }
        } else if drive.released && release_should_drop {
            // A pointer drag commits only when it ends on a legal destination.
            // Releasing over open space leaves the token in hand.
            let over = cursor::hovered(position, &rects).and_then(kind_of);
            if let (Some(slot), Some(SelectTarget::Portrait(cell))) = (carrying, over) {
                if let Some(pick) = fighters.cell(cell) {
                    select.set_pick(slot, pick);
                    cursors
                        .seat_mut(seat)
                        .expect("`seat` is bounded by the loop over 0..MAX_SMASH_SEATS")
                        .drop_it();
                }
            }
        }
    }
}

/// Where a placed token is right now, ignoring a carrier's hand.
///
/// The token has no independent resting coordinate. Its owner slot chooses a
/// grid cell (fighter or Random), and the token is drawn on that cell. `None`
/// means the slot is absent or its selected cell is on another page.
pub fn token_rect(
    layout: &SelectLayout,
    select: &SmashSelect,
    fighters: &SmashRoster,
    slot: usize,
) -> Option<HitRect> {
    let card = select.slot(slot);
    if !card.occupant.participates() {
        return None;
    }
    let cell = card.pick.map(|pick| cell_of(pick, fighters))?;
    layout
        .portrait(cell)
        .map(|rect| token_rect_over(layout, rect, slot))
}

/// Where a slot's token sits once it is ON a portrait.
///
/// Offset per slot so two players who chose the same fighter are both visible;
/// two on one character is legal, and a stack of one would read as a lost token.
fn token_rect_over(layout: &SelectLayout, cell: HitRect, slot: usize) -> HitRect {
    let token = layout.token_px();
    let spread = token * 0.62;
    let offset = Vec2::new(
        (slot as f32 - 1.5) * spread,
        cell.size().y * 0.5 - token * 0.9,
    );
    HitRect::from_center_size(cell.center() + offset, Vec2::splat(token))
}

/// Put every anchored node where the layout says it goes.
pub fn place_the_screen(
    fighters: Res<SmashRoster>,
    page: Res<SelectPage>,
    windows: Query<&Window>,
    mut nodes: Query<(&Anchored, &mut Node)>,
) {
    let layout = current_layout(&windows, &fighters, &page);
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

/// Project selection state onto the portrait grid.
pub fn sync_select_grid(
    select: Res<SmashSelect>,
    fighters: Res<SmashRoster>,
    art: ScreenArt,
    mut cells: Query<(&PortraitCell, &mut BorderColor)>,
    mut monograms: Query<(&PortraitMonogram, &mut Visibility)>,
) {
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

    // Initials are a fallback for portrait art that never arrived.
    for (mono, mut visibility) in &mut monograms {
        let missing = match (&mono.0, art.asset_server.as_deref()) {
            (None, _) => true,
            (Some(handle), Some(server)) => server
                .get_load_state(handle.id())
                .is_none_or(|state| state.is_failed()),
            // No asset server is a headless fixture, not a failed load.
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
}

/// Project participant-slot state onto the four bottom cards.
pub fn sync_select_cards(
    select: Res<SmashSelect>,
    fighters: Res<SmashRoster>,
    art: ScreenArt,
    devices: Option<Res<ambition_platformer2d::input::LocalDeviceOrder>>,
    assignment: Option<Res<ambition_platformer2d::input::LocalSeatOffer>>,
    mut cards: Query<(&SlotCardFrame, &mut BorderColor)>,
    mut role_labels: Query<(&RoleButtonLabel, &mut Text), Without<CardName>>,
    mut card_names: Query<(&CardName, &mut Text, &mut TextColor), Without<RoleButtonLabel>>,
    mut card_portraits: Query<(&CardPortrait, &mut ImageNode, &mut Visibility)>,
) {
    let catalog = Some(&*art.catalog);
    let naming = devices.as_deref().map(|devices| {
        (
            devices,
            assignment
                .as_deref()
                .map(|offer| offer.policy())
                .unwrap_or_default(),
        )
    });

    for (card, mut border) in &mut cards {
        set_border(
            &mut border,
            if select.slot(card.0).occupant.participates() {
                SLOT_COLORS[card.0]
            } else {
                PANEL_EDGE
            },
        );
    }

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
}

/// Project readiness/refusal state onto START and the prompt.
pub fn sync_select_chrome(
    select: Res<SmashSelect>,
    refusal: Option<
        Res<ambition_platformer2d::actors::character_runtime::MatchPreparationProblems>,
    >,
    mut start_button: Query<&mut BorderColor, With<StartButton>>,
    mut prompt: Query<&mut Text, With<SelectPrompt>>,
) {
    for mut border in &mut start_button {
        set_border(&mut border, if select.ready() { INK } else { PANEL_EDGE });
    }

    for mut text in &mut prompt {
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
}

/// Project movable select-screen pieces: tokens and human hand cursors.
///
/// These are the only two UI roles kept in one system because they share a
/// direct visual relation: a carried token follows a cursor. Their mutual
/// `Without` filters are therefore a local proof, not a screen-wide exclusion
/// matrix.
pub fn sync_select_tokens_and_cursors(
    select: Res<SmashSelect>,
    cursors: Res<SelectCursors>,
    fighters: Res<SmashRoster>,
    page: Res<SelectPage>,
    windows: Query<&Window>,
    offer: Option<Res<ambition_platformer2d::input::LocalSeatOffer>>,
    mut tokens: Query<(&SlotToken, &mut Node, &mut Visibility), Without<CursorNode>>,
    mut cursor_nodes: Query<(&CursorNode, &mut Node, &mut Visibility), Without<SlotToken>>,
) {
    let layout = current_layout(&windows, &fighters, &page);
    let offered_seats = offer
        .as_deref()
        .map(|offer| offer.seats() as usize)
        .unwrap_or(1);

    for (token, mut node, mut visibility) in &mut tokens {
        let card = select.slot(token.0);
        if !card.occupant.participates() {
            set_visibility(&mut visibility, Visibility::Hidden);
            continue;
        }

        let rect = if let Some(seat) = cursors.carrier_of(token.0) {
            Some(HitRect::from_center_size(
                cursors
                    .seat(seat)
                    .expect("`seat` is bounded by the loop over 0..MAX_SMASH_SEATS")
                    .position,
                Vec2::splat(layout.token_px()),
            ))
        } else {
            token_rect(&layout, &select, &fighters, token.0)
        };

        match rect {
            Some(rect) => {
                set_visibility(&mut visibility, Visibility::Inherited);
                set_rect(&mut node, rect);
            }
            // A selected fighter can be on another page. The token still has a
            // well-defined placement; this page simply does not draw it.
            None => set_visibility(&mut visibility, Visibility::Hidden),
        }
    }

    for (marker, mut node, mut visibility) in &mut cursor_nodes {
        let seat = marker.0;
        // Cursors are indexed by INPUT seat, not by match-roster slot. Looking
        // at `select.slot(seat)` here creates a phantom hand for a CPU hole in a
        // sparse roster (human / CPU / human) and can hide the actual second
        // person's hand. `LocalSeatOffer` is the authority for how many local
        // input participants this frontend is offering, so presentation does not
        // re-derive the same count from connected devices.
        if seat >= offered_seats {
            set_visibility(&mut visibility, Visibility::Hidden);
            continue;
        }
        set_visibility(&mut visibility, Visibility::Inherited);
        let pointer = cursors
            .seat(seat)
            .expect("`seat` is bounded by the loop over 0..MAX_SMASH_SEATS");
        let at = if pointer.placed {
            pointer.position
        } else {
            layout
                .portrait(seat.min(layout.characters.saturating_sub(1)))
                .map(HitRect::center)
                .unwrap_or(layout.viewport * 0.5)
        };
        set_rect(
            &mut node,
            HitRect::from_center_size(at, Vec2::splat(layout.cursor_px())),
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
    /// nothing here writes `Touches` directly — it cannot, the collections are
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
        app.init_resource::<SelectCursors>();
        app.init_resource::<StartRequested>();
        app.init_resource::<LeaveRequested>();
        app.init_resource::<SelectPage>();
        app.init_resource::<SelectInteractionPolicy>();
        // See the note in `lib.rs`'s fixture: the cursor integrates a clock.
        app.init_resource::<Time>();
        app.init_resource::<Touches>();
        app.add_message::<TouchInput>();
        app.add_systems(PreUpdate, bevy::input::touch::touch_screen_input_system);
        app.add_systems(Update, drive_the_cursor);
        app
    }

    /// TWO FINGERS ARE TWO PLAYERS, which is the whole point of per-seat
    /// cursors and the thing one shared pointer could never do.
    ///
    /// the second finger has to land on the SECOND CURSOR'S TOKEN. That
    /// assigns the gesture to that cursor; a finger landing anywhere else while
    /// cursor 0 is busy still claims no cursor, which keeps a stray thumb from
    /// hijacking somebody's drag (the test below).
    #[test]
    fn two_fingers_drag_two_seats_tokens_at_once() {
        let mut app = screen();
        {
            let mut select = app.world_mut().resource_mut::<SmashSelect>();
            select.set_occupant(0, SlotOccupant::Controller { device: 0 });
            select.set_occupant(1, SlotOccupant::Controller { device: 1 });
        }

        let layout = headless_layout();
        let token_zero = placed_token(&app, &layout, 0);
        let token_one = placed_token(&app, &layout, 1);

        finger(&mut app, 10, TouchPhase::Started, token_zero.center());
        finger(&mut app, 11, TouchPhase::Started, token_one.center());
        app.update();

        let cursors = *app.world().resource::<SelectCursors>();
        assert_eq!(
            (
                cursors.seat(0).expect("seat 0").carrying,
                cursors.seat(1).expect("seat 1").carrying
            ),
            (Some(0), Some(1)),
            "two fingers on two tokens did not put one in each seat's hand"
        );

        // Each lands on a DIFFERENT fighter, so neither could have been the
        // other's press arriving twice.
        let first = layout.portrait(0).expect("a grid");
        let second = layout.portrait(1).expect("a grid with two cells");
        finger(&mut app, 10, TouchPhase::Moved, first.center());
        finger(&mut app, 11, TouchPhase::Moved, second.center());
        app.update();
        finger(&mut app, 10, TouchPhase::Ended, first.center());
        finger(&mut app, 11, TouchPhase::Ended, second.center());
        app.update();

        let select = app.world().resource::<SmashSelect>();
        assert_eq!(
            (select.slot(0).pick, select.slot(1).pick),
            (Some(SlotPick::Fighter(0)), Some(SlotPick::Fighter(1))),
            "two simultaneous drags did not land on two different fighters"
        );
    }

    /// Releasing a carried token over empty space does not invent a third
    /// resting state. The token remains in the hand until it reaches a legal
    /// fighter/Random destination.
    #[test]
    fn a_token_released_in_open_space_remains_carried() {
        let mut app = screen();
        app.world_mut()
            .resource_mut::<SmashSelect>()
            .set_occupant(0, SlotOccupant::Controller { device: 0 });

        let layout = headless_layout();
        let token = placed_token(&app, &layout, 0);
        let empty = Vec2::new(layout.viewport.x * 0.5, layout.viewport.y * 0.60);

        finger(&mut app, 50, TouchPhase::Started, token.center());
        app.update();
        finger(&mut app, 50, TouchPhase::Moved, empty);
        app.update();
        finger(&mut app, 50, TouchPhase::Ended, empty);
        app.update();

        assert_eq!(
            app.world()
                .resource::<SelectCursors>()
                .seat(0)
                .expect("seat 0")
                .carrying,
            Some(0),
            "open-space release created a resting token instead of keeping it in hand"
        );
        assert_eq!(
            app.world().resource::<SmashSelect>().slot(0).pick,
            Some(SlotPick::Random),
            "moving a carried token through empty space changed its owner's selection"
        );
    }

    /// PRESSING A FIGHTER WITH AN EMPTY HAND CHOOSES IT — IN THE DEFAULT.
    ///
    /// This is the ordinary Ultimate-style fast path: a free hand selects the
    /// portrait directly and the owner's placed token follows the new pick.
    #[test]
    fn a_tap_on_a_fighter_chooses_it_without_switching_any_mode() {
        let mut app = screen();
        app.world_mut()
            .resource_mut::<SmashSelect>()
            .set_occupant(0, SlotOccupant::Controller { device: 0 });

        let layout = headless_layout();
        let portrait = layout.portrait(1).expect("a grid with two cells");
        finger(&mut app, 51, TouchPhase::Started, portrait.center());
        app.update();

        assert_eq!(
            app.world().resource::<SmashSelect>().slot(0).pick,
            Some(SlotPick::Fighter(1)),
            "a plain tap on a face did not choose that fighter"
        );
    }

    /// AND A PLACED TOKEN CAN STILL BE PICKED BACK UP.
    ///
    /// the arm-ordering trap: a placed token sits ON the portrait it chose,
    /// so a press there matches both "pick this up" and "choose this". If the
    /// tap won, a token once placed could never be moved again.
    #[test]
    fn a_token_resting_on_a_face_is_picked_up_rather_than_re_choosing_it() {
        let mut app = screen();
        app.world_mut()
            .resource_mut::<SmashSelect>()
            .set_occupant(0, SlotOccupant::Controller { device: 0 });

        let layout = headless_layout();
        let portrait = layout.portrait(1).expect("a grid with two cells");
        finger(&mut app, 52, TouchPhase::Started, portrait.center());
        app.update();
        finger(&mut app, 52, TouchPhase::Ended, portrait.center());
        app.update();

        // The token now rests on that face. Press it again.
        let on_face = placed_token(&app, &layout, 0);
        finger(&mut app, 53, TouchPhase::Started, on_face.center());
        app.update();

        assert_eq!(
            app.world()
                .resource::<SelectCursors>()
                .seat(0)
                .expect("seat 0")
                .carrying,
            Some(0),
            "pressing a placed token re-chose the fighter under it instead of \
             picking the token up"
        );
    }

    /// TWO TAPS BY A SEAT THAT IS NOT SEAT ZERO.
    ///
    /// A seat mid-carry now claims the next finger.
    #[test]
    fn a_second_seat_can_tap_its_token_then_tap_a_fighter() {
        let mut app = screen();
        {
            let mut select = app.world_mut().resource_mut::<SmashSelect>();
            select.set_occupant(0, SlotOccupant::Controller { device: 0 });
            select.set_occupant(1, SlotOccupant::Controller { device: 1 });
        }
        let layout = headless_layout();
        let token_one = placed_token(&app, &layout, 1);
        let portrait = layout.portrait(1).expect("a grid with two cells");

        // Tap one: pick the token up. The finger LIFTS.
        finger(&mut app, 40, TouchPhase::Started, token_one.center());
        app.update();
        finger(&mut app, 40, TouchPhase::Ended, token_one.center());
        app.update();
        assert_eq!(
            app.world()
                .resource::<SelectCursors>()
                .seat(1)
                .expect("seat 1")
                .carrying,
            Some(1),
            "the first tap did not leave seat 1 holding its token"
        );

        // Tap two: a NEW finger, on a face.
        finger(&mut app, 41, TouchPhase::Started, portrait.center());
        app.update();
        finger(&mut app, 41, TouchPhase::Ended, portrait.center());
        app.update();

        let select = app.world().resource::<SmashSelect>();
        assert_eq!(
            select.slot(1).pick,
            Some(SlotPick::Fighter(1)),
            "seat 1's second tap did not choose the fighter it landed on"
        );
        // `Random`, not `None` — joining a slot seats it on the random
        // square, so an untouched seat 0 already has a pick. What this asserts
        // is that seat 1's tap did not CHANGE it.
        assert_eq!(
            select.slot(0).pick,
            Some(SlotPick::Random),
            "seat 1's tap moved seat 0's pick, so the finger drove the wrong seat"
        );
    }

    /// Development defaults permit one human hand to manipulate another
    /// human's token. This is useful for exercising rosters without reaching
    /// for every controller, while the policy remains switchable to Ultimate's
    /// stricter rule.
    #[test]
    fn another_human_token_is_grabbable_by_default() {
        let mut app = screen();
        app.init_resource::<ambition_platformer2d::input::SeatMenuFrames>();
        {
            let mut select = app.world_mut().resource_mut::<SmashSelect>();
            select.set_occupant(0, SlotOccupant::Controller { device: 0 });
            select.set_occupant(1, SlotOccupant::Controller { device: 1 });
            select.set_pick(0, 0);
            select.set_pick(1, 1);
        }
        let layout = headless_layout();
        let token_zero = placed_token(&app, &layout, 0);
        app.world_mut()
            .resource_mut::<SelectCursors>()
            .seat_mut(1)
            .expect("seat 1")
            // ⛔ `move_to`, NOT `.position = …`. A cursor is PLACED or it is
            // not, and an unplaced one is relocated to the first portrait by
            // the screen's own opening move — so assigning the field alone
            // parked this finger on a portrait and the token was never hit.
            .move_to(token_zero.center());
        app.world_mut()
            .resource_mut::<ambition_platformer2d::input::SeatMenuFrames>()
            .set(
                1,
                ambition_platformer2d::input::MenuControlFrame {
                    select: true,
                    ..Default::default()
                },
            );

        app.update();

        assert_eq!(
            app.world()
                .resource::<SelectCursors>()
                .seat(1)
                .expect("seat 1")
                .carrying,
            Some(0),
            "the default testing policy refused another human's token"
        );
    }

    /// A connected hand is still the lobby's manipulation tool when every
    /// match slot is CPU. It needs no owned match card to move a CPU token.
    #[test]
    fn an_unseated_hand_can_move_a_cpu_token() {
        let mut app = screen();
        app.init_resource::<ambition_platformer2d::input::SeatMenuFrames>();
        app.world_mut()
            .resource_mut::<SmashSelect>()
            .set_occupant(1, SlotOccupant::Cpu);

        let layout = headless_layout();
        let cpu_token = placed_token(&app, &layout, 1);
        app.world_mut()
            .resource_mut::<SelectCursors>()
            .seat_mut(0)
            .expect("seat 0")
            .move_to(cpu_token.center());
        app.world_mut()
            .resource_mut::<ambition_platformer2d::input::SeatMenuFrames>()
            .set(
                0,
                ambition_platformer2d::input::MenuControlFrame {
                    select: true,
                    ..Default::default()
                },
            );

        app.update();

        assert_eq!(
            app.world()
                .resource::<SelectCursors>()
                .seat(0)
                .expect("seat 0")
                .carrying,
            Some(1),
            "an unseated human hand could not configure a CPU token"
        );
    }

    /// With the optional cross-human grab disabled, another person's token is
    /// pointer-transparent: A reaches the fighter portrait beneath it instead.
    #[test]
    fn another_human_token_can_be_made_pointer_transparent() {
        let mut app = screen();
        app.init_resource::<ambition_platformer2d::input::SeatMenuFrames>();
        app.world_mut()
            .resource_mut::<SelectInteractionPolicy>()
            .allow_other_human_token_grab = false;
        {
            let mut select = app.world_mut().resource_mut::<SmashSelect>();
            select.set_occupant(0, SlotOccupant::Controller { device: 0 });
            select.set_occupant(1, SlotOccupant::Controller { device: 1 });
            select.set_pick(0, 0);
            select.set_pick(1, 1);
        }
        let layout = headless_layout();
        let token_zero = placed_token(&app, &layout, 0);
        app.world_mut()
            .resource_mut::<SelectCursors>()
            .seat_mut(1)
            .expect("seat 1")
            // ⛔ `move_to`, NOT `.position = …`. A cursor is PLACED or it is
            // not, and an unplaced one is relocated to the first portrait by
            // the screen's own opening move — so assigning the field alone
            // parked this finger on a portrait and the token was never hit.
            .move_to(token_zero.center());
        app.world_mut()
            .resource_mut::<ambition_platformer2d::input::SeatMenuFrames>()
            .set(
                1,
                ambition_platformer2d::input::MenuControlFrame {
                    select: true,
                    ..Default::default()
                },
            );

        app.update();

        assert_eq!(
            app.world()
                .resource::<SelectCursors>()
                .seat(1)
                .expect("seat 1")
                .carrying,
            None,
            "Ultimate-style policy still let seat 1 grab seat 0's token"
        );
        assert_eq!(
            app.world().resource::<SmashSelect>().slot(1).pick,
            Some(SlotPick::Fighter(0)),
            "the non-grabbable token blocked the fighter portrait beneath it"
        );
    }

    /// A HELD STICK ROAMS; IT DOES NOT SNAP.
    ///
    /// the whole point of `MenuControlFrame::nav`, and the thing a d-pad cannot express: the
    /// cursor lands wherever the stick left it, which will almost never be a target's centre.
    /// So this checks BOTH: it travelled, and it did not arrive anywhere in particular.
    #[test]
    fn a_held_stick_roams_the_cursor_instead_of_snapping_to_a_target() {
        let mut app = screen();
        app.init_resource::<ambition_platformer2d::input::MenuControlFrame>();
        app.update();

        let start = app
            .world()
            .resource::<SelectCursors>()
            .seat(0)
            .expect("seat 0")
            .position;
        // A tenth of a second of full-right deflection.
        app.world_mut()
            .resource_mut::<ambition_platformer2d::input::MenuControlFrame>()
            .nav = Vec2::X;
        let step = std::time::Duration::from_millis(100);
        app.world_mut().resource_mut::<Time>().advance_by(step);
        app.update();

        let moved = app
            .world()
            .resource::<SelectCursors>()
            .seat(0)
            .expect("seat 0")
            .position;
        assert!(
            moved.x > start.x,
            "a held stick left the cursor at {moved:?}, where it started"
        );
        assert!(
            (moved.y - start.y).abs() < 0.001,
            "pushing sideways moved the cursor vertically, to {moved:?}"
        );

        let layout = headless_layout();
        let landed_on_a_centre = layout
            .targets()
            .into_iter()
            .any(|(_, rect)| rect.center().distance(moved) < 0.5);
        assert!(
            !landed_on_a_centre,
            "the cursor snapped to a target centre at {moved:?} instead of roaming"
        );
    }

    /// AND A FLICK LANDS ON A PORTRAIT, WHICH IS THE HALF THAT WAS BROKEN.
    ///
    /// ⛔⛔ THE FIXTURE IS THE FINDING. A real device never sends a direction
    /// EDGE with an idle deflection: `decode_menu_frame` builds `nav` from the
    /// held d-pad and held arrow keys as well as the stick, so on the frame any
    /// edge fires, that same direction is held and `nav` is non-zero. The
    /// screen took the roam branch first, so the snap branch was unreachable on
    /// a stick, a d-pad AND a keyboard — and Jon's report was *"very hard to use
    /// with a gamepad"*.
    ///
    /// ⭐ so this drives BOTH TOGETHER, which is the shape production sends, and
    /// asserts the cursor arrived somewhere in particular. Its sibling above
    /// keeps the other half honest: deflection with no edge still roams.
    #[test]
    fn a_flick_snaps_even_though_the_same_direction_is_held() {
        let mut app = screen();
        app.init_resource::<ambition_platformer2d::input::MenuControlFrame>();
        app.update();

        // A press edge AND the deflection that necessarily accompanies it.
        {
            let mut frame = app
                .world_mut()
                .resource_mut::<ambition_platformer2d::input::MenuControlFrame>();
            frame.right = true;
            frame.nav = Vec2::X;
        }
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(16));
        app.update();

        let landed = app
            .world()
            .resource::<SelectCursors>()
            .seat(0)
            .expect("seat 0")
            .position;
        let layout = headless_layout();
        let nearest = layout
            .targets()
            .into_iter()
            .map(|(_, rect)| rect.center().distance(landed))
            .fold(f32::INFINITY, f32::min);
        assert!(
            nearest < 0.5,
            "a flick left the cursor at {landed:?}, {nearest:.1}px from the \
             nearest target centre — it roamed instead of snapping, which is the \
             branch order that made this screen hard to drive with a pad"
        );
    }

    /// AND IT KEEPS GOING WHILE THE STICK IS HELD, which is the half an
    /// edge-driven cursor could never do: a second frame of the same press
    /// travels as far again, rather than waiting for a repeat timer.
    #[test]
    fn a_stick_held_for_two_frames_travels_twice_as_far() {
        let mut app = screen();
        app.init_resource::<ambition_platformer2d::input::MenuControlFrame>();
        app.update();
        app.world_mut()
            .resource_mut::<ambition_platformer2d::input::MenuControlFrame>()
            .nav = Vec2::X;

        let step = std::time::Duration::from_millis(50);
        let start = app
            .world()
            .resource::<SelectCursors>()
            .seat(0)
            .expect("seat 0")
            .position
            .x;
        app.world_mut().resource_mut::<Time>().advance_by(step);
        app.update();
        let after_one = app
            .world()
            .resource::<SelectCursors>()
            .seat(0)
            .expect("seat 0")
            .position
            .x;
        app.world_mut().resource_mut::<Time>().advance_by(step);
        app.update();
        let after_two = app
            .world()
            .resource::<SelectCursors>()
            .seat(0)
            .expect("seat 0")
            .position
            .x;

        let first = after_one - start;
        let second = after_two - after_one;
        assert!(first > 0.0, "the first frame did not move the cursor");
        assert!(
            (second - first).abs() < first * 0.05,
            "two equal frames travelled {first} then {second} — the cursor is \
             not integrating a held stick"
        );
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

    /// Where one placed token is sitting, asked of the same derivation the
    /// screen draws and hit-tests with.
    fn placed_token(app: &App, layout: &SelectLayout, slot: usize) -> HitRect {
        token_rect(
            layout,
            app.world().resource::<SmashSelect>(),
            app.world().resource::<SmashRoster>(),
            slot,
        )
        .expect("a participating slot on this page owns a token")
    }

    fn token_of_slot_zero(app: &App, layout: &SelectLayout) -> HitRect {
        placed_token(app, layout, 0)
    }

    /// A FINGER PLAYS THIS SCREEN.
    ///
    /// Tap the token, tap a portrait — the two-tap idiom a pad already uses, which is the one a
    /// finger can perform without a hover state.
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
            app.world()
                .resource::<SelectCursors>()
                .seat(0)
                .expect("seat 0")
                .position,
            token.center(),
            "the cursor already sat on the token, so this test cannot see a \
             finger move it"
        );

        finger(&mut app, 7, TouchPhase::Started, token.center());
        app.update();
        assert_eq!(
            app.world()
                .resource::<SelectCursors>()
                .seat(0)
                .expect("seat 0")
                .position,
            token.center(),
            "a finger on slot 0's token did not move the cursor to it"
        );
        assert_eq!(
            app.world()
                .resource::<SelectCursors>()
                .seat(0)
                .expect("seat 0")
                .carrying,
            Some(0),
            "the touch press never reached the screen's click arbitration"
        );

        // Lifting without travelling is the first half of a two-tap place, not
        // a drop — the same rule that keeps a pad's pick-up in hand.
        finger(&mut app, 7, TouchPhase::Ended, token.center());
        app.update();
        assert_eq!(
            app.world()
                .resource::<SelectCursors>()
                .seat(0)
                .expect("seat 0")
                .carrying,
            Some(0),
            "lifting the finger put the token straight back down"
        );

        finger(&mut app, 8, TouchPhase::Started, portrait.center());
        app.update();
        assert_eq!(
            app.world()
                .resource::<SelectCursors>()
                .seat(0)
                .expect("seat 0")
                .position,
            portrait.center(),
            "the second tap did not move the cursor onto the portrait"
        );
        assert_eq!(
            app.world().resource::<SmashSelect>().slot(0).pick,
            Some(SlotPick::Fighter(1)),
            "a finger tapped a portrait and the slot did not take that fighter"
        );
    }

    /// this is the half that needs the RELEASE edge, and the release edge is
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
            app.world()
                .resource::<SelectCursors>()
                .seat(0)
                .expect("seat 0")
                .carrying,
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
            app.world()
                .resource::<SelectCursors>()
                .seat(0)
                .expect("seat 0")
                .carrying,
            None,
            "the drag ended with the token still in hand"
        );
    }

    /// A SECOND FINGER ON NOBODY'S TOKEN IS NOT A SECOND CURSOR.
    ///
    /// One person drags a token; somebody else's finger — or the same person's
    /// palm — lands on a portrait. That seat's cursor must stay where the
    /// driving finger is and the stray press must not arbitrate, or the drag
    /// ends by dropping the token wherever the intruder touched.
    ///
    /// the intruder is given the LOWER id on purpose. Android recycles pointer
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
            app.world()
                .resource::<SelectCursors>()
                .seat(0)
                .expect("seat 0")
                .carrying,
            Some(0),
            "the driving finger never picked the token up"
        );

        finger(&mut app, 2, TouchPhase::Started, portrait.center());
        app.update();
        assert_eq!(
            app.world()
                .resource::<SelectCursors>()
                .seat(0)
                .expect("seat 0")
                .position,
            token.center(),
            "a second finger stole the cursor from the one that was dragging"
        );
        assert_eq!(
            app.world()
                .resource::<SelectCursors>()
                .seat(0)
                .expect("seat 0")
                .carrying,
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
