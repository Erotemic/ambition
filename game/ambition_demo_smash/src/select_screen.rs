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

/// The screen's UI root. Its own marker, so teardown despawns only this
/// screen's nodes.
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
    /// The stage cycle beside START — a match decision, not a per-seat one.
    Stage,
    /// The stocks cycle, left of the stage: also a match decision.
    Stocks,
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
/// One component and one placing system, so no widget is missed on resize.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchored {
    Title,
    Prompt,
    Portrait(usize),
    Card(usize),
    RoleButton(usize),
    /// The stage cycle beside START — a match decision, not a per-seat one.
    Stage,
    /// The stocks cycle, left of the stage.
    Stocks,
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

/// The stage button's text, so a sync system can find it.
#[derive(Component)]
pub struct StageButtonLabel;

/// What the stage button reads. Prefixed so it reads as a control, not only a
/// state.
pub fn stage_button_text(choice: crate::SmashStageChoice) -> String {
    format!("Stage: {}", choice.label())
}

/// The stocks button's own label marker.
#[derive(Component, Clone, Copy)]
pub struct StocksButtonLabel;

/// What the stocks button reads. Prefixed like the stage button's.
pub fn stocks_button_text(choice: crate::SmashStockChoice) -> String {
    format!("Stocks: {}", choice.label())
}

/// The chosen fighter's portrait on a card.
#[derive(Component, Clone, Copy)]
pub struct CardPortrait(pub usize);

/// The chosen fighter's name on a card.
#[derive(Component, Clone, Copy)]
pub struct CardName(pub usize);

/// The initials drawn under a portrait, shown only when the art never arrives.
///
/// It carries the handle, not a boolean: the path resolves by convention and
/// the load can fail later or never finish, so it asks the asset server each
/// frame.
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

/// The BACK button's frame. It never dims: leaving is always allowed.
#[derive(Component)]
pub struct BackButton;

/// Somebody asked to leave the lobby.
///
/// A request, like [`StartRequested`]: this module does not write
/// `ShellCommand`, so `leave_the_select_screen_when_asked` stays the one place
/// that decides where BACK goes.
///
/// Tap-B belongs to the select state machine (it recalls the owner's token);
/// leaving is an explicit Back control or a held-B gesture.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct LeaveRequested(pub bool);

/// Slot colours, in the order a couch fills up. Far apart in hue, so a token
/// and its card match at a glance.
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

/// What a slot's role button says: the full statement of what that card is.
///
/// It names the device, not the slot index. `devices`/`policy` are optional
/// because fixtures and the walkthrough binary render it without an input
/// world; then it falls back to the index.
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
        // "RANDOM", not a fighter's name: the draw has not happened. A slot on
        // random is decided, so `ready()` counts it.
        Some(SlotPick::Random) => "RANDOM".to_string(),
        Some(SlotPick::Fighter(index)) => match fighters.get(index) {
            Some(id) => display_name(catalog, id),
            None => "— no fighter —".to_string(),
        },
        None => "— no fighter —".to_string(),
    }
}

/// The random square's icon, for the grid cell and for the card of a slot that
/// took it. One accessor, so the two cannot disagree.
///
/// It is a 32x32 world tile standing in for a portrait. Replace it here when a
/// drawn random icon exists.
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
        // Not a panic and not an empty string: a catalog miss shows on screen.
        .unwrap_or_else(|| id.to_string())
}

/// Initials, for a cell whose art did not arrive (as the dialogue box does
/// for speakers with no portrait).
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
    /// What providers registered, so a character that named a portrait target
    /// in Rust gets that face. See [`portrait_art`].
    pub declared: Option<Res<'w, PreparedCharacterRegistry>>,
    pub asset_server: Option<Res<'w, AssetServer>>,
    /// The decoded entity art, for the random square's interrobang. Loaded by
    /// the same asset pass as every other sprite.
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

/// A character's face, as an image and the rectangle to take out of it.
///
/// The registry is `Option`: the standalone app may not install
/// `PortraitSheetRegistryPlugin`. Without it the whole image is used, which is
/// right for single-frame sheets and wrong for the rest.
fn portrait_art(
    catalog: &CharacterCatalog,
    portraits: Option<&PortraitSheetRegistry>,
    declared: Option<&PreparedCharacterRegistry>,
    asset_server: Option<&AssetServer>,
    id: &str,
) -> Option<(Handle<Image>, Option<Rect>)> {
    // Through the engine's resolver, not the catalog directly. A character
    // registered in Rust may name a portrait target; others keep the
    // catalog's derived answer.
    let target = declared
        .and_then(|registry| registry.get(id))
        .and_then(|prepared| prepared.portrait.as_deref());
    let reference = portrait_for_declared_character(portraits, catalog, target, id)?;
    let handle = ambition_platformer2d::sprite_sheet::game_assets::load_sheet_image(
        asset_server?,
        "portrait",
        reference.image.clone(),
    );
    // A still: this grid never ticks a frame.
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
/// Not in every composition. The standalone demo uses the select screen as its
/// home route (`ShellComposition::new(.., SMASH_SELECT_ROUTE, ..)`), so
/// `QuitToHome` would re-enter the same route.
///
/// Read from the host spec. Absent (a bare unit fixture) means no exit.
pub fn exit_leads_somewhere(
    host: Option<&ambition_platformer2d::game_shell::ShellHostConfiguration>,
) -> bool {
    host.and_then(|host| host.spec.as_ref())
        .is_some_and(|spec| spec.home_route.as_str() != crate::SMASH_SELECT_ROUTE)
}

/// Character-select interaction policy.
///
/// Token ownership and selection stay one state machine; the policy only says
/// which otherwise-valid token grabs are permitted. During development one
/// human may move another human's token, which speeds up testing. `false`
/// gives Ultimate's protected-human-token behavior.
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
/// A resource, not a field on [`SelectLayout`]: the layout is a pure function
/// of the viewport, and the page is a decision. A resize that re-pages the
/// grid keeps the page. `SelectLayout::paged` clamps it into range, so this
/// resource does not need the roster size.
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
/// Built once and positioned every frame by [`layout`]. Respawning per frame
/// would drop resolved image handles and restart every load.
pub fn spawn_select_screen(
    mut commands: Commands,
    existing: Query<(), With<SmashSelectUiRoot>>,
    fighters: Res<SmashRoster>,
    // The catalog inside is required, not `Option`
    // (`engine.character-authority-is-app-local`). Without it the grid would
    // show nameless plates that look like missing art. This demo registers
    // its own fragment, so every composition on this route has one.
    art: ScreenArt,
    // Whether to draw the way out; see [`exit_leads_somewhere`].
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
        font: font.clone().into(),
        font_size: FontSize::Px(size),
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
            // Above the world, below the pause menu.
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
            // A button, not only a binding: a mouse has no Back control. It
            // never dims; leaving is always allowed.
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

            // One more cell than there are fighters. The random square is
            // drawn after the loop, so fighters keep their cell indices.
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
                    // A monogram under every portrait, so missing art reads as
                    // missing art, not a broken grid (for example `mary_o`).
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
                        TextLayout::justify(Justify::Center),
                    ));
                });
            }

            // ── THE RANDOM SQUARE, last cell of the grid ─────────────────
            //
            // The glyph from Mary-O's bonus block. It is a cell like any other
            // (tokens rest on it, it lights up, `ready()` counts it), but not
            // a character, so the pick is `SlotPick::Random`.
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
                        // As for portraits: no asset server means the label
                        // and no art.
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
                        TextLayout::justify(Justify::Center),
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

            let mut stage = anchored(Anchored::Stage);
            stage.1.justify_content = JustifyContent::Center;
            stage.1.align_items = AlignItems::Center;
            stage.1.border = UiRect::all(Val::Px(2.0));
            stage.1.border_radius = BorderRadius::all(Val::Px(6.0));
            root.spawn((
                stage,
                BackgroundColor(Color::srgb(0.11, 0.12, 0.18)),
                BorderColor::all(PANEL_EDGE),
                GlobalZIndex(610),
                Name::new("stage button"),
            ))
            .with_children(|node| {
                node.spawn((
                    StageButtonLabel,
                    // Seeded with the default, so it is not blank before its
                    // sync system first runs.
                    Text::new(stage_button_text(crate::SmashStageChoice::default())),
                    text_font(14.0),
                    TextColor(INK),
                ));
            });

            let mut stocks = anchored(Anchored::Stocks);
            stocks.1.justify_content = JustifyContent::Center;
            stocks.1.align_items = AlignItems::Center;
            stocks.1.border = UiRect::all(Val::Px(2.0));
            stocks.1.border_radius = BorderRadius::all(Val::Px(6.0));
            root.spawn((
                stocks,
                BackgroundColor(Color::srgb(0.11, 0.12, 0.18)),
                BorderColor::all(PANEL_EDGE),
                GlobalZIndex(610),
                Name::new("stocks button"),
            ))
            .with_children(|node| {
                node.spawn((
                    StocksButtonLabel,
                    // Seeded with the default, like the stage button.
                    Text::new(stocks_button_text(crate::SmashStockChoice::default())),
                    text_font(14.0),
                    TextColor(INK),
                ));
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
                            TextLayout::justify(Justify::Center),
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
            // One hand per seat, in the seat's `SLOT_COLORS`, so the hand
            // matches the token it grabs.
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
/// A coherent `SystemParam`: it groups input and environment readers. The
/// state machine below still names its own model resources.
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
/// A mouse or finger writes a position, a held stick roams, and arrows and
/// d-pad snap between targets. All write the same field; see [`cursor`] for
/// why there is one position per seat and no separate focus.
pub(crate) fn drive_the_cursor(
    mut select: ResMut<SmashSelect>,
    mut cursors: ResMut<SelectCursors>,
    mut start: ResMut<StartRequested>,
    mut leave: ResMut<LeaveRequested>,
    fighters: Res<SmashRoster>,
    mut page: ResMut<SelectPage>,
    // The stage the START below plays on. Outside `SmashSelect`, like the
    // cursor: no seat decided it.
    mut stage: ResMut<crate::SmashStageChoice>,
    // A required `ResMut`, like `stage`; fixtures register it. An `Option`
    // would silently no-op the button in production if registration dropped.
    mut stocks: ResMut<crate::SmashStockChoice>,
    policy: Res<SelectInteractionPolicy>,
    inputs: SelectScreenInputs,
    mut local: Local<SelectDriverLocal>,
) {
    let layout = current_layout(&inputs.windows, &fighters, &page);
    let exit = exit_leads_somewhere(inputs.host.as_deref());
    // The layout says where things are; the screen says what is reachable
    // (as `token_rect` does for an absent slot's token).
    let targets: Vec<(SelectTarget, HitRect)> = layout
        .targets()
        .into_iter()
        .filter(|(kind, _)| exit || *kind != SelectTarget::Back)
        .collect();
    // The cursor's target is an index into this list. The layout already
    // names each target, and its order breaks ties.
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
    // Directional navigation can land on a token, but tokens are not static
    // layout targets. Keep them out of `rects`: hover belongs to the portrait
    // beneath, and an ineligible human token must be transparent to A.
    // `snap_rects` only gives navigation the token's current centre.
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

    // What each seat asked for this frame, folded from every device for it.
    // Gathered for all four before acting, so no seat sees another's change.
    #[derive(Default, Clone, Copy)]
    struct SeatDrive {
        moved_to: Option<Vec2>,
        analog: Vec2,
        direction: Vec2,
        pressed: bool,
        released: bool,
        back: bool,
        back_held: bool,
        page_back: bool,
        page_forward: bool,
    }
    let mut drives = [SeatDrive::default(); MAX_SMASH_SEATS];

    // The mouse, keyboard and global frame speak for seat 0 (a keyboard on a
    // route with no seats reports on the global frame). Pads speak for their
    // own seats.
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
            drive.analog = frame.analog;
            drive.pressed |= frame.select;
            drive.back |= frame.back && !frame.start;
            drive.back_held |= frame.back_held && !frame.start;
            drive.page_back |= frame.page_left;
            drive.page_forward |= frame.page_right;
            // `back` is ignored when `start` is set: Escape is bound to both
            // (`presets.rs`, `rebind.rs`), and the pause menu opens on `start`
            // in the same unordered set. Checked per frame, so another seat's
            // pad can still leave on the same tick.
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
        if global.analog.length_squared() > drive.analog.length_squared() {
            drive.analog = global.analog;
        }
        drive.pressed |= global.select;
        drive.back |= global.back && !global.start;
        drive.back_held |= global.back_held && !global.start;
        drive.page_back |= global.page_left;
        drive.page_forward |= global.page_right;
    }

    // ── the mouse ────────────────────────────────────────────────────────
    // Only a mouse move counts, so a stationary mouse does not fight the
    // arrow keys (the snap-back bug `SeatActiveDevices` addresses). Local,
    // because this screen needs the move's position.
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

    // No move gate for touch: a touch position exists only while a finger is
    // down, and a new touch has zero delta, so a gate would skip the tap.
    if let Some(touches) = inputs.touches.as_deref() {
        // A lifted finger stops driving after this frame, so the release edge
        // lands where the finger left.
        for touch in touches.iter_just_pressed() {
            if local.fingers.contains_key(&touch.id()) {
                continue;
            }
            // A finger that lands on a token drives that seat, if free, so a
            // second finger is a second player.
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
            // A seat mid-carry claims the next finger. Tap-token, tap-fighter
            // uses two fingers, because the first lifted; without this the
            // second tap would drive seat 0. Lowest seat wins a tie.
            let carrying = (0..MAX_SMASH_SEATS).find(|seat| {
                !taken.contains(seat)
                    && cursors
                        .seat(*seat)
                        .is_some_and(|cursor| cursor.carrying.is_some())
            });
            // Otherwise seat 0, if free: one person on a phone taps portraits
            // and buttons without touching a token.
            let seat = on_token.or(carrying).or(if taken.contains(&DESKTOP_SEAT) {
                None
            } else {
                Some(DESKTOP_SEAT)
            });
            if let Some(seat) = seat {
                local.fingers.insert(touch.id(), seat);
            }
        }
        // Android recycles pointer ids. Seats are claimed only on the
        // just-pressed edge above, never reassigned to a finger already down.
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

    // Which input participants are present, derived from the per-seat menu
    // frames (one per `InputParticipant`), not from device order.
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
    // Headless/touch-only compositions may have no per-seat producer. Seat 0
    // is still the desktop/touch cursor, and active fingers own cursors while
    // they are down.
    if connected_sources.is_empty() {
        connected_sources.push(DESKTOP_SEAT);
    }
    connected_sources.extend(local.fingers.values().copied());
    connected_sources.sort_unstable();
    connected_sources.dedup();

    // ── the page, which is the one part of the grid every seat shares ────
    // Clamped against the layout's page count, so a resize never shows an
    // empty page.
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
        // Which card is this person's? Not `seat`: see
        // [`SmashSelect::slot_driven_by`]. The cursor is seat-keyed; the card
        // is the one naming this seat's input source.
        let own_slot = select.slot_driven_by(seat);

        // Movement is one short mutable borrow; token arbitration below needs
        // the whole cursor table.
        {
            let pointer = cursors
                .seat_mut(seat)
                .expect("`seat` is bounded by the loop over 0..MAX_SMASH_SEATS");

            // Start on a portrait, not the origin, spread per seat so four
            // cursors do not stack.
            if !pointer.placed {
                if let Some(rect) = layout.portrait(seat.min(layout.characters.saturating_sub(1))) {
                    pointer.move_to(rect.center());
                }
            }

            if let Some(position) = drive.moved_to {
                pointer.move_to(position);
            }

            // The stick roams; the d-pad snaps. `MenuControlFrame::analog` is
            // only the stick, so a stick flick does not also fire a snap. A hand
            // on the stick is a pointer and never snaps; d-pad, arrows and the
            // analog edges from the repeat machinery walk target to target. The
            // stick wins when both are live.
            //
            // Advance the ramp every frame the stick is read, including at
            // rest, so a released stick resets its speed.
            let dt = inputs.time.delta_secs();
            let ramp = pointer.ramp.advance(drive.analog, dt);
            if drive.analog != Vec2::ZERO {
                let travel = cursor::cursor_travel(drive.analog, layout.cell(), dt, ramp);
                let roamed = pointer.position + travel;
                pointer.move_to(Vec2::new(
                    roamed.x.clamp(0.0, layout.viewport.x),
                    roamed.y.clamp(0.0, layout.viewport.y),
                ));
            } else if drive.direction != Vec2::ZERO {
                if let Some(entity) = cursor::snap(pointer.position, drive.direction, &snap_rects) {
                    if let Some(target) = snap_rects.iter().find(|target| target.entity == entity) {
                        pointer.move_to(target.rect.center());
                    }
                }
            }
        }

        // Tap Back is token manipulation; holding Back leaves the screen. The
        // hold timer is per seat.
        if drive.back_held {
            local.back_hold_seconds[seat] += inputs.time.delta_secs();
            if exit && local.back_hold_seconds[seat] >= BACK_HOLD_TO_LEAVE_SECONDS {
                leave.0 = true;
            }
        } else {
            local.back_hold_seconds[seat] = 0.0;
        }

        if drive.back {
            // Ultimate's tap-B: with an empty hand, the hand returns to its
            // own placed token and picks it up. B while carrying is a no-op. If
            // another cursor carries this token (allowed by the development
            // policy), the owner does not steal it.
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
            // Tokens sit over portraits, so check token eligibility first. Own
            // and CPU tokens are always grabbable; other humans' tokens depend
            // on the policy (default on). A non-grabbable token is transparent.
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
                // Token hits win over the portrait beneath. `try_grab` enforces
                // exclusivity, so two hands on one token give one carrier.
                (None, Some(slot), _) => {
                    cursors.try_grab(seat, slot);
                }
                // Pressing a fighter with an empty hand chooses it. If this
                // source has no slot yet, it claims the first absent card and
                // chooses in one step. Never write card `seat` just because the
                // cursor is seat-keyed.
                (None, _, Some(SelectTarget::Portrait(cell))) => {
                    if let Some(pick) = fighters.cell(cell) {
                        if let Some(own) = select.slot_for_or_claim(seat) {
                            select.set_pick(own, pick);
                        }
                    }
                }
                // Page turns are allowed while carrying: the target fighter
                // may be on another page.
                (_, _, Some(SelectTarget::PagePrev)) => {
                    page.0 = page.0.saturating_sub(1);
                }
                (_, _, Some(SelectTarget::PageNext)) => {
                    page.0 = (page.0 + 1).min(last_page);
                }
                // Placing, by cell (not fighter index): `SmashRoster::cell`
                // knows the last square is random, and a click past the grid
                // chooses nothing.
                (Some(slot), _, Some(SelectTarget::Portrait(cell))) => {
                    if let Some(pick) = fighters.cell(cell) {
                        select.set_pick(slot, pick);
                        cursors
                            .seat_mut(seat)
                            .expect("`seat` is bounded by the loop over 0..MAX_SMASH_SEATS")
                            .drop_it();
                    }
                }
                // Empty space and unrelated controls do nothing: a token is
                // either carried or placed.
                (Some(_), _, _) => {}
                (None, None, Some(SelectTarget::RoleButton(slot))) => {
                    select.cycle_role(slot, seat, &connected_sources);
                }
                // Any seat may cycle the stage, as for roles; the match is not
                // player one's.
                (None, None, Some(SelectTarget::Stage)) => {
                    *stage = stage.next();
                }
                // Any seat may cycle stocks, as for the stage.
                (None, None, Some(SelectTarget::Stocks)) => {
                    *stocks = stocks.next();
                }
                (None, None, Some(SelectTarget::Start)) => {
                    if select.ready() {
                        start.0 = true;
                    }
                }
                // No readiness check: leaving is always allowed.
                (None, None, Some(SelectTarget::Back)) => {
                    leave.0 = true;
                }
                (None, None, _) => {}
            }
        } else if drive.released && release_should_drop {
            // A pointer drag commits only on a legal destination. Releasing
            // over open space keeps the token in hand.
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
/// Offset per slot, so two players on the same fighter are both visible.
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
            Anchored::Stage => Some(layout.stage_button()),
            Anchored::Stocks => Some(layout.stocks_button()),
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
    stage: Res<crate::SmashStageChoice>,
    mut stage_label: Query<
        &mut Text,
        (
            With<StageButtonLabel>,
            Without<RoleButtonLabel>,
            Without<CardName>,
            Without<StocksButtonLabel>,
        ),
    >,
    stocks: Res<crate::SmashStockChoice>,
    mut stocks_label: Query<
        &mut Text,
        (
            With<StocksButtonLabel>,
            Without<RoleButtonLabel>,
            Without<CardName>,
            Without<StageButtonLabel>,
        ),
    >,
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

    // Keep the label in step with the stage the match will prepare.
    for mut text in &mut stage_label {
        let next = stage_button_text(*stage);
        if text.0 != next {
            text.0 = next;
        }
    }
    for mut text in &mut stocks_label {
        let next = stocks_button_text(*stocks);
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
    refusal: Option<Res<ambition_platformer2d::versus_match::MatchPreparationProblems>>,
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
/// The two roles share one system because a carried token follows a cursor.
/// Their mutual `Without` filters are a local proof, not a screen-wide
/// exclusion matrix.
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
            // The selected fighter can be on another page; this page does not
            // draw the token.
            None => set_visibility(&mut visibility, Visibility::Hidden),
        }
    }

    for (marker, mut node, mut visibility) in &mut cursor_nodes {
        let seat = marker.0;
        // Cursors are indexed by input seat, not match slot. Using
        // `select.slot(seat)` would show a phantom hand for a CPU hole in a
        // sparse roster (human / CPU / human). `LocalSeatOffer` is the
        // authority for how many local participants are offered.
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

    /// The screen, driven headlessly, with the real touch path in front of it.
    ///
    /// The test sends the `TouchInput` messages winit emits and lets Bevy's
    /// `touch_screen_input_system` fold them (`Touches` is private), so the
    /// fixture stays close to Android.
    ///
    /// No window and no `UiPlugin`: the rectangles come from [`layout`], which
    /// uses `HEADLESS_VIEWPORT` when there is no window.
    fn screen() -> App {
        let mut app = App::new();
        app.init_resource::<SmashSelect>();
        app.init_resource::<SmashRoster>();
        app.init_resource::<SelectCursors>();
        app.init_resource::<StartRequested>();
        app.init_resource::<LeaveRequested>();
        app.init_resource::<SelectPage>();
        app.init_resource::<SelectInteractionPolicy>();
        // The stage the START press plays on. The driver writes it, so every
        // touch test fails without it.
        app.init_resource::<crate::SmashStageChoice>();
        app.init_resource::<crate::SmashStockChoice>();
        // See the note in `lib.rs`'s fixture: the cursor integrates a clock.
        app.init_resource::<Time>();
        app.init_resource::<Touches>();
        app.add_message::<TouchInput>();
        app.add_systems(PreUpdate, bevy::input::touch::touch_screen_input_system);
        app.add_systems(Update, drive_the_cursor);
        app
    }

    /// Two fingers are two players.
    ///
    /// The second finger lands on the second cursor's token, which assigns the
    /// gesture to that cursor. A finger landing elsewhere while cursor 0 is
    /// busy claims no cursor (tested below).
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

        // Each lands on a different fighter, so neither is the other's press
        // arriving twice.
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

    /// Pressing a fighter with an empty hand chooses it, in the default policy.
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

    /// A placed token can still be picked back up.
    ///
    /// A placed token sits on the portrait it chose, so a press there matches
    /// both "pick up" and "choose". The pick-up must win.
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

    /// Two taps by a seat that is not seat zero: a seat mid-carry claims the
    /// next finger.
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

        // Tap one: pick the token up. The finger lifts.
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

        // Tap two: a new finger, on a face.
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
        // `Random`, not `None`: joining seats a slot on the random square.
        // This asserts seat 1's tap did not change seat 0's pick.
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
            // `move_to`, not `.position = …`: an unplaced cursor is moved to
            // the first portrait by the screen's opening move.
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
            // `move_to`, not `.position = …`: an unplaced cursor is moved to
            // the first portrait by the screen's opening move.
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

    /// A held stick roams; it does not snap.
    ///
    /// The cursor stops wherever the stick left it, rarely a target's centre.
    /// This checks that it travelled and did not land on a target.
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
            .analog = Vec2::X;
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

    /// A d-pad edge lands on a portrait. A d-pad press arrives with `analog` at
    /// rest.
    #[test]
    fn a_d_pad_edge_snaps_to_the_next_portrait() {
        let mut app = screen();
        app.init_resource::<ambition_platformer2d::input::MenuControlFrame>();
        app.update();

        app.world_mut()
            .resource_mut::<ambition_platformer2d::input::MenuControlFrame>()
            .right = true;
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
            "a d-pad press left the cursor at {landed:?}, {nearest:.1}px from the \
             nearest target centre — a digital direction must walk the grid \
             target to target"
        );
    }

    /// A stick flick never snaps, even on the edge frame.
    ///
    /// The analog repeat machinery emits a direction edge from a deflected
    /// stick, so on the flick frame both `right` and `analog` are live. A hand
    /// on the stick is a pointer.
    #[test]
    fn a_stick_flick_does_not_snap_even_though_it_also_fires_a_direction_edge() {
        let mut app = screen();
        app.init_resource::<ambition_platformer2d::input::MenuControlFrame>();
        app.update();

        let start = app
            .world()
            .resource::<SelectCursors>()
            .seat(0)
            .expect("seat 0")
            .position;

        // What a real stick sends on the frame it crosses the edge threshold.
        {
            let mut frame = app
                .world_mut()
                .resource_mut::<ambition_platformer2d::input::MenuControlFrame>();
            frame.right = true;
            frame.analog = Vec2::X;
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
            nearest >= 0.5,
            "a stick flick snapped the cursor onto a target centre at {landed:?} \
             — the next frame of the same hold then roams away from it, which is \
             the whole complaint"
        );
        assert!(
            landed.x > start.x,
            "the flick moved the cursor nowhere at all, so this arm would pass \
             for a cursor that is simply dead"
        );
    }

    /// A stick keeps moving while held: a second frame travels as far again,
    /// with no repeat timer.
    #[test]
    fn a_stick_held_for_two_frames_travels_twice_as_far() {
        let mut app = screen();
        app.init_resource::<ambition_platformer2d::input::MenuControlFrame>();
        app.update();
        app.world_mut()
            .resource_mut::<ambition_platformer2d::input::MenuControlFrame>()
            .analog = Vec2::X;

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

    /// A held stick builds speed, through the real screen.
    ///
    /// `cursor_ramp_tests` pins the curve; this pins the wiring. A ramp
    /// advanced in the wrong place, or reset each frame, would pass those.
    ///
    /// The position is reset each frame so the measurement is per-frame
    /// travel, not limited by the viewport clamp.
    #[test]
    fn a_stick_held_a_long_time_covers_more_ground_per_frame_than_it_did_at_first() {
        const HOME: f32 = 100.0;
        let mut app = screen();
        app.init_resource::<ambition_platformer2d::input::MenuControlFrame>();
        app.update();
        app.world_mut()
            .resource_mut::<ambition_platformer2d::input::MenuControlFrame>()
            .analog = Vec2::X;

        let mut travel_per_frame = Vec::new();
        for _ in 0..60 {
            {
                let mut cursors = app.world_mut().resource_mut::<SelectCursors>();
                let pointer = cursors.seat_mut(0).expect("seat 0");
                pointer.move_to(Vec2::new(HOME, pointer.position.y));
            }
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(16));
            app.update();
            let x = app
                .world()
                .resource::<SelectCursors>()
                .seat(0)
                .expect("seat 0")
                .position
                .x;
            travel_per_frame.push(x - HOME);
        }

        let early = travel_per_frame[1];
        let late = travel_per_frame[travel_per_frame.len() - 1];
        assert!(early > 0.0, "the cursor never moved at all");
        assert!(
            late > early * 1.5,
            "after a second of holding, a frame still travels {late:.2}px against \
             the opening {early:.2}px — the ramp is not reaching the screen"
        );

        // Letting go resets the ramp; otherwise the next push starts at speed.
        app.world_mut()
            .resource_mut::<ambition_platformer2d::input::MenuControlFrame>()
            .analog = Vec2::ZERO;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(16));
        app.update();
        app.world_mut()
            .resource_mut::<ambition_platformer2d::input::MenuControlFrame>()
            .analog = Vec2::X;
        {
            let mut cursors = app.world_mut().resource_mut::<SelectCursors>();
            let pointer = cursors.seat_mut(0).expect("seat 0");
            pointer.move_to(Vec2::new(HOME, pointer.position.y));
        }
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(16));
        app.update();
        let after_release = app
            .world()
            .resource::<SelectCursors>()
            .seat(0)
            .expect("seat 0")
            .position
            .x
            - HOME;
        assert!(
            (after_release - early).abs() < early * 0.05,
            "the push after a release travelled {after_release:.2}px against an \
             opening {early:.2}px — it inherited the speed of the last sweep"
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

    /// A finger plays this screen: tap the token, tap a portrait. A finger
    /// has no hover state.
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
        // done before the finger arrives.
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

        // Lifting without travelling is the first half of a two-tap place,
        // not a drop (as for a pad's pick-up).
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

    /// Dragging needs the release edge. When it fires, the touch is gone from
    /// `Touches::iter`, so a driver that reads only fingers still down misses it.
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

    /// A second finger on nobody's token is not a second cursor.
    ///
    /// One person drags a token and a stray finger lands on a portrait. The
    /// cursor stays with the driving finger and the stray press does nothing.
    ///
    /// The intruder gets the lower id on purpose: Android recycles pointer ids,
    /// so "lowest id wins" would fail here.
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
