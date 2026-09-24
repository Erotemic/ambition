//! The renderer for a game's declared HUD readouts.
//!
//! This module draws whatever the active route declared, and knows nothing
//! about what it means. It spawns one text node per
//! [`HudSlotSpec`](ambition_platformer2d_shared_tangle::gameplay_presentation::HudSlotSpec)
//! and mirrors the matching
//! [`HudReadouts`](ambition_platformer2d_shared_tangle::gameplay_presentation::HudReadouts)
//! entry into it every frame. Labels such as "RINGS" or "SCORE" are strings a
//! game writes; none appear here.
//!
//! Placement uses the same ladder as the built-in HUD: ask [`hud_region`] for
//! the slot's region, use it when the active profile reserves a surround big
//! enough, otherwise overlay gameplay. A readout knows its own size.
//!
//! [`hud_region`]:
//!     ambition_platformer2d_shared_tangle::gameplay_presentation::ResolvedGameplayPresentation::hud_region

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::{
    gameplay_presentation::{
        ActiveHudDeclaration, HudFigure, HudReadouts, HudSlotId, HudSlotSpec,
        ResolvedGameplayPresentation, ScreenOccluder, ScreenRect, SurroundRegion,
    },
    lifecycle::{ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt},
};

use super::{HUD_MARGIN, OVERLAY_ANCHOR};

/// Root of a game's declared HUD.
///
/// Public on purpose: a demo's OV1 guard uses it to tell UI the engine
/// dragged in (forbidden) from UI this game declared (allowed).
#[derive(Component)]
pub struct DeclaredHudRoot;

/// Does the declared HUD own this node — as its root, or at any depth beneath?
///
/// Ownership is the whole subtree. `DeclaredHudRoot` is on the panel and the
/// slot; the portrait, stock row, pips, and count are unmarked children. A
/// guard that asks only `Without<DeclaredHudRoot>` would call a demo's allowed
/// HUD an engine violation.
///
/// This lives beside the marker so every demo's `ov1_draws_the_world` guard
/// uses one definition instead of copies that drift.
///
/// Walks to the top, not one level: the stock pips are grandchildren
/// (root, panel, stock row, pip).
pub fn declared_hud_owns(world: &World, entity: Entity) -> bool {
    let mut cursor = entity;
    loop {
        if world.get::<DeclaredHudRoot>(cursor).is_some() {
            return true;
        }
        match world.get::<bevy::prelude::ChildOf>(cursor) {
            Some(parent) => cursor = parent.0,
            None => return false,
        }
    }
}

/// One declared readout's text node, tagged with its slot id and its declared
/// font size.
///
/// The declared size is stored, not read back from the live `TextFont`,
/// because the emphasis scale is applied every frame. Scaling the current size
/// would compound and grow the readout without limit.
#[derive(Component)]
pub struct DeclaredHudSlot(pub HudSlotId, pub f32);

/// Slot ids are stable identities, not cache keys for appearance. Retaining the
/// full spec lets a route update font, colour, centering, order, or region while
/// keeping the same id and still receive a rebuilt node.
#[derive(Component, Clone, Debug)]
pub struct DeclaredHudSpec(HudSlotSpec);

/// Gap between stacked readouts in the same region.
const SLOT_GAP: f32 = 6.0;

/// Bevy's default line height, `LineHeight::RelativeToFont(1.2)`.
///
/// Mirrored, not read: the spawned nodes use the default. If a slot ever
/// declares its own line height, use that value instead.
const LINE_HEIGHT_FACTOR: f32 = 1.2;

/// How much vertical room one slot's published readout needs, including
/// multi-line (`\n`) text.
///
/// A slot with no published readout still reserves one line. Otherwise a
/// conditional card would shift everything below it each time it appears.
fn slot_extent(spec: &HudSlotSpec, readouts: &HudReadouts, measured: Option<f32>) -> f32 {
    // `ComputedNode` has the real laid-out height, including line breaks. It
    // is last frame's value, because UI layout runs in `PostUpdate` and this
    // is an `Update` system. Moving `top` does not change height, so there is
    // no oscillation; the lag shows only when the line count changes.
    if let Some(height) = measured.filter(|height| *height > 0.0) {
        return height + SLOT_GAP;
    }
    let lines = readouts
        .get(&spec.id)
        .map(|readout| readout.text().lines().count())
        .unwrap_or(1)
        .max(1);
    spec.font_size * LINE_HEIGHT_FACTOR * lines as f32 + SLOT_GAP
}

fn declaration_matches_live_specs<'a>(
    declared: &[HudSlotSpec],
    existing: impl Iterator<Item = Option<&'a DeclaredHudSpec>>,
) -> bool {
    let collected: Option<Vec<&HudSlotSpec>> =
        existing.map(|spec| spec.map(|spec| &spec.0)).collect();
    let Some(mut live) = collected else {
        // A node from an older renderer has no cached spec, so rebuild it.
        return false;
    };
    if live.len() != declared.len() {
        return false;
    }
    let mut wanted: Vec<&HudSlotSpec> = declared.iter().collect();
    live.sort_by(|a, b| a.id.cmp(&b.id));
    wanted.sort_by(|a, b| a.id.cmp(&b.id));
    live == wanted
}

fn select_hud_region(
    presentation: &ResolvedGameplayPresentation,
    spec: &HudSlotSpec,
) -> Option<(SurroundRegion, ScreenRect)> {
    if !presentation.prefers_surround_hud() {
        return None;
    }
    let fits = |rect: &ScreenRect| rect.width() >= spec.min_px.x && rect.height() >= spec.min_px.y;
    std::iter::once(spec.region)
        .chain(
            [
                SurroundRegion::Left,
                SurroundRegion::Right,
                SurroundRegion::Top,
                SurroundRegion::Bottom,
            ]
            .into_iter()
            .filter(|region| *region != spec.region),
        )
        .find_map(|region| {
            presentation
                .hud_region(region)
                .filter(fits)
                .map(|rect| (region, rect))
        })
}

/// Spawn one text node per declared slot, once, while a session owns them.
///
/// Rebuilds from scratch when the active declaration changes, so switching
/// experiences in a shared host never leaves the old game's readouts.
pub fn spawn_declared_hud(
    mut commands: Commands,
    active: Res<ActiveHudDeclaration>,
    active_session: Option<Res<ActiveSessionScope>>,
    fonts: Option<Res<crate::ui_fonts::UiFonts>>,
    existing: Query<(Entity, &DeclaredHudSlot, Option<&DeclaredHudSpec>)>,
    // Every root this pass owns, including sibling gauge roots, so a rebuild
    // removes the whole previous HUD.
    owned: Query<Entity, With<DeclaredHudRoot>>,
) {
    let declared = active.slots();

    if declared.is_empty() {
        for entity in &owned {
            commands.entity(entity).despawn();
        }
        return;
    }

    // Already showing this exact declaration, identity and appearance. An
    // id-only check would miss a restyled slot that kept its id.
    let exact = declaration_matches_live_specs(declared, existing.iter().map(|(_, _, spec)| spec));
    if exact {
        return;
    }
    for entity in &owned {
        commands.entity(entity).despawn();
    }

    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        // A shell host can keep a session for one deferred teardown frame. Do
        // not create gameplay UI without a live session owner.
        return;
    };

    // Stack within each region, in the declaration's stable laid-out order.
    let mut offset_in_region: std::collections::BTreeMap<u8, f32> = Default::default();
    let ordered = active
        .0
        .as_ref()
        .map(|declaration| declaration.laid_out())
        .unwrap_or_default();

    for spec in ordered {
        let slot_offset = offset_in_region.entry(spec.region as u8).or_insert(0.0);
        let [r, g, b, a] = spec.color;
        let font = fonts
            .as_deref()
            .map(|fonts| fonts.text_font(spec.font_size, crate::ui_fonts::UiFontWeight::Semibold))
            .unwrap_or(TextFont {
                font_size: FontSize::Px(spec.font_size),
                ..default()
            });
        commands.spawn_session_scoped(
            session_scope,
            (
                DeclaredHudRoot,
                DeclaredHudBar(spec.id.clone()),
                Node {
                    position_type: PositionType::Absolute,
                    // Under the slot text, across its declared minimum width.
                    left: Val::Px(0.0),
                    top: Val::Px(spec.font_size + 2.0),
                    width: Val::Px(0.0),
                    height: Val::Px(0.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(r, g, b, a)),
                Name::new(format!("Declared HUD gauge ({})", spec.id.as_str())),
            ),
        );
        // The fighter panel: spawned for every slot, shown only for a slot
        // that publishes a `Standing`. Spawned once with a fixed number of
        // stock icons, hidden per frame, so losing a life does not churn
        // entities. `DeclaredHudRoot`'s retire sweep expects one spawn per
        // declaration.
        commands
            .spawn_session_scoped(
                session_scope,
                (
                    DeclaredHudRoot,
                    DeclaredHudPanel(spec.id.clone()),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(OVERLAY_ANCHOR.x),
                        top: Val::Px(OVERLAY_ANCHOR.y),
                        width: Val::Px(PANEL_W),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    Visibility::Hidden,
                    Name::new(format!("Declared HUD panel ({})", spec.id.as_str())),
                ),
            )
            .with_children(|panel| {
                panel.spawn((
                    DeclaredHudPortrait(spec.id.clone()),
                    ImageNode::default(),
                    Node {
                        width: Val::Px(PORTRAIT_PX),
                        height: Val::Px(PORTRAIT_PX),
                        ..default()
                    },
                    Visibility::Hidden,
                    Name::new(format!("Declared HUD portrait ({})", spec.id.as_str())),
                ));
                // The stock row sits under the percent (the slot's own text node),
                // so it is spaced down past it.
                panel
                    .spawn((
                        Node {
                            margin: UiRect::top(Val::Px(spec.font_size * LINE_HEIGHT_FACTOR + 4.0)),
                            column_gap: Val::Px(STOCK_ICON_GAP),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        Name::new(format!("Declared HUD stocks ({})", spec.id.as_str())),
                    ))
                    .with_children(|row| {
                        for index in 0..MAX_DRAWN_STOCKS {
                            row.spawn((
                                DeclaredHudStock(spec.id.clone(), index),
                                ImageNode::default(),
                                Node {
                                    width: Val::Px(STOCK_ICON_PX),
                                    height: Val::Px(STOCK_ICON_PX),
                                    ..default()
                                },
                                Visibility::Hidden,
                                Name::new(format!(
                                    "Declared HUD stock {index} ({})",
                                    spec.id.as_str()
                                )),
                            ));
                        }
                        row.spawn((
                            DeclaredHudStockCount(spec.id.clone()),
                            Text::new(String::new()),
                            TextFont {
                                font_size: FontSize::Px(STOCK_ICON_PX),
                                ..default()
                            },
                            TextColor(Color::srgba(r, g, b, a)),
                            Visibility::Hidden,
                            Name::new(format!("Declared HUD stock count ({})", spec.id.as_str())),
                        ));
                    });
            });
        commands.spawn_session_scoped(
            session_scope,
            (
                DeclaredHudRoot,
                DeclaredHudSlot(spec.id.clone(), spec.font_size),
                DeclaredHudSpec(spec.clone()),
                Text::new(String::new()),
                bevy::text::TextLayout::justify(if spec.centered {
                    bevy::text::Justify::Center
                } else {
                    bevy::text::Justify::Left
                }),
                font,
                TextColor(Color::srgba(r, g, b, a)),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(OVERLAY_ANCHOR.x),
                    top: Val::Px(OVERLAY_ANCHOR.y + *slot_offset),
                    ..default()
                },
                Name::new(format!("Declared HUD slot ({})", spec.id.as_str())),
                // Generic screen occupancy from this node's computed layout: the
                // HUD says what it is, the host derives where it is.
                ScreenOccluder::hud(),
            ),
        );
        *slot_offset += spec.font_size + SLOT_GAP;
    }
}

/// Move each declared readout into the region it asked for, when the active
/// profile leaves one big enough; otherwise leave it overlaying gameplay.
///
/// Same ladder as `place_player_hud`, per slot instead of per widget, because
/// each slot declares its own region and minimum.
pub fn place_declared_hud(
    presentation: Res<ResolvedGameplayPresentation>,
    active: Res<ActiveHudDeclaration>,
    // Current readouts: a slot's height depends on how many lines the game
    // published this frame. See [`slot_extent`].
    readouts: Res<HudReadouts>,
    mut slots: Query<(&DeclaredHudSlot, &mut Node, Option<&ComputedNode>)>,
) {
    let mut offset_in_region: std::collections::BTreeMap<u8, f32> = Default::default();
    let mut overlay_offset = 0.0_f32;

    let ordered = active
        .0
        .as_ref()
        .map(|declaration| declaration.laid_out())
        .unwrap_or_default();

    for spec in ordered {
        // A centred card ignores the region ladder: it goes over the gameplay
        // rectangle.
        if spec.centered {
            let gameplay = presentation.gameplay_rect;
            for (slot, mut node, _) in &mut slots {
                if slot.0 != spec.id {
                    continue;
                }
                // Span the gameplay rect and let the text centre itself
                // (`JustifyText::Center`). `left: 50%` would put the node's left
                // edge at the middle, so the card would run off to the right.
                let left = Val::Px(gameplay.min.x);
                let width = Val::Px(gameplay.width());
                if node.left != left {
                    node.left = left;
                }
                if node.width != width {
                    node.width = width;
                }
                let y = gameplay.min.y + gameplay.height() * 0.38;
                if node.top != Val::Px(y) {
                    node.top = Val::Px(y);
                }
            }
            continue;
        }
        // Prefer the declared region; fall back to any other reserved region
        // before overlaying. Otherwise `Top` readouts find nothing on an
        // ordinary monitor.
        let region = select_hud_region(&presentation, spec);

        // What this slot's text used last frame, in logical px. `ComputedNode`
        // is physical; `Node::top` is logical.
        let measured = slots
            .iter()
            .find(|(slot, ..)| slot.0 == spec.id)
            .and_then(|(_, _, computed)| computed)
            .map(|computed| computed.size().y * computed.inverse_scale_factor());

        let anchor = match region {
            Some((actual_region, rect)) => {
                // Two preferences can fall back to the same region. Stack by the
                // region chosen, or both start at its origin and overlap.
                let stacked = offset_in_region.entry(actual_region as u8).or_insert(0.0);
                let anchor = rect.min + Vec2::splat(HUD_MARGIN) + Vec2::new(0.0, *stacked);
                *stacked += slot_extent(spec, &readouts, measured);
                anchor
            }
            None => {
                let anchor = OVERLAY_ANCHOR + Vec2::new(0.0, overlay_offset);
                overlay_offset += slot_extent(spec, &readouts, measured);
                anchor
            }
        };

        for (slot, mut node, _) in &mut slots {
            if slot.0 != spec.id {
                continue;
            }
            if node.left != Val::Px(anchor.x) {
                node.left = Val::Px(anchor.x);
            }
            if node.top != Val::Px(anchor.y) {
                node.top = Val::Px(anchor.y);
            }
        }
    }
}

/// The gauge bar belonging to one declared slot.
///
/// A sibling root, not a child. It follows the slot's live `Node` every frame
/// (see [`update_declared_hud_gauges`]), because the slot moves between
/// regions when the presentation profile changes. Both sweeps in
/// [`spawn_declared_hud`] key on [`DeclaredHudRoot`].
#[derive(bevy::prelude::Component, Debug)]
pub struct DeclaredHudBar(pub HudSlotId);

/// Size each slot's gauge from its published fill.
///
/// A slot whose readout has no `fill` collapses to zero size, so a game can
/// publish a gauge conditionally (a boss bar) without a second slot.
pub fn update_declared_hud_gauges(
    readouts: Res<HudReadouts>,
    // The slot's live node, so the bar follows it when `place_declared_hud`
    // moves the slot to another region.
    specs: Query<(&DeclaredHudSlot, &DeclaredHudSpec, &Node), Without<DeclaredHudBar>>,
    mut bars: Query<(&DeclaredHudBar, &mut Node)>,
) {
    for (bar, mut node) in &mut bars {
        let slot = specs.iter().find(|(slot, ..)| slot.0 == bar.0);
        if let Some((_, spec, slot_node)) = slot {
            let left = slot_node.left;
            let text_height = slot_extent(&spec.0, &readouts, None) - SLOT_GAP;
            let top = match slot_node.top {
                Val::Px(px) => Val::Px(px + text_height + 2.0),
                other => other,
            };
            if node.left != left {
                node.left = left;
            }
            if node.top != top {
                node.top = top;
            }
        }
        let figure = readouts.get(&bar.0).map(|readout| readout.figure.clone());
        let fill = match figure.flatten() {
            Some(HudFigure::Gauge(fill)) => fill,
            // A standing draws a panel, not a bar (`update_declared_hud_panels`).
            // Collapse here so a slot never shows both.
            Some(HudFigure::Standing(_)) | None => {
                if node.height != Val::Px(0.0) {
                    node.height = Val::Px(0.0);
                    node.width = Val::Px(0.0);
                }
                continue;
            }
        };
        // The slot's declared minimum width is the bar's full extent, so a game
        // sizes its gauge by declaring room, not pixels.
        let full = slot
            .map(|(_, spec, _)| spec.0.min_px.x.max(120.0))
            .unwrap_or(120.0);
        let width = Val::Px(full * fill);
        let height = Val::Px(6.0);
        if node.width != width {
            node.width = width;
        }
        if node.height != height {
            node.height = height;
        }
    }
}

/// Mirror the game's published readouts into the spawned text nodes.
///
/// A slot with no published readout draws an empty string rather than stale
/// text, so a game may publish conditionally without the declaration changing.
pub fn update_declared_hud(
    readouts: Res<HudReadouts>,
    mut slots: Query<(&DeclaredHudSlot, &mut Text, &mut TextFont)>,
) {
    for (slot, mut text, mut font) in &mut slots {
        let readout = readouts.get(&slot.0);
        let next = readout.map(|readout| readout.text()).unwrap_or_default();
        if text.0 != next {
            text.0 = next;
        }
        // The punch: a readout that was just hit draws bigger for the same beat
        // as the freeze, because both come from the same number.
        //
        // The base size comes from the declaration, not the current font, so the
        // scale does not compound.
        let emphasis = readout
            .and_then(|readout| readout.standing_of())
            .map(|standing| standing.emphasis.clamp(0.0, 1.0))
            .unwrap_or(0.0);
        let base = slot.1;
        let wanted = base * (1.0 + emphasis * HUD_PUNCH_GAIN);
        // Read-compare-write: a `Mut` deref marks the component changed, so the
        // settled case must not touch it. This pass is the only writer of HUD
        // sizes, so every value is a `Px`.
        if !matches!(font.font_size, FontSize::Px(px) if (px - wanted).abs() <= 0.01) {
            font.font_size = FontSize::Px(wanted);
        }
    }
}

/// How much bigger a freshly-hit readout draws, at full emphasis.
///
/// A quarter again: the eye catches it during a fight, and a 132px panel
/// still holds the text.
const HUD_PUNCH_GAIN: f32 = 0.25;

// ---------------------------------------------------------------------------
// The fighter panel: a portrait, the percent under it, and the stocks as icons
// ---------------------------------------------------------------------------

/// How many stocks are drawn one-icon-each before it becomes a count.
///
/// A platform fighter draws a row of heads while there are few, and switches
/// to `xN` when counting would take longer than reading a number.
pub const MAX_DRAWN_STOCKS: u32 = 5;

/// How wide one fighter panel is, and how big the pieces in it are.
const PANEL_W: f32 = 132.0;
const PORTRAIT_PX: f32 = 56.0;
const STOCK_ICON_PX: f32 = 14.0;
const STOCK_ICON_GAP: f32 = 3.0;

/// One fighter panel's root, tagged with the slot it belongs to.
///
/// A sibling root, not a child of the slot's text node, like the gauge bar:
/// the text node moves between regions, and this follows it every frame.
#[derive(Component, Debug)]
pub struct DeclaredHudPanel(pub HudSlotId);

/// The portrait inside one panel.
#[derive(Component, Debug)]
pub struct DeclaredHudPortrait(pub HudSlotId);

/// One stock icon inside one panel, by its index in the row.
#[derive(Component, Debug)]
pub struct DeclaredHudStock(pub HudSlotId, pub u32);

/// The `xN` beside a single icon, when there are too many to draw.
#[derive(Component, Debug)]
pub struct DeclaredHudStockCount(pub HudSlotId);

/// Which slots are drawing a fighter panel this frame, in laid-out order.
///
/// The count comes from the readouts, not the declaration. The smash stage
/// declares four slots and a 1v1 publishes two; counting the declaration
/// would space two panels as four and leave gaps.
fn panelled_slots(active: &ActiveHudDeclaration, readouts: &HudReadouts) -> Vec<HudSlotId> {
    active
        .0
        .as_ref()
        .map(|declaration| declaration.laid_out())
        .unwrap_or_default()
        .into_iter()
        .filter(|spec| {
            readouts
                .get(&spec.id)
                .is_some_and(|readout| readout.standing_of().is_some())
        })
        .map(|spec| spec.id.clone())
        .collect()
}

/// How a stock count is DRAWN: how many icons, and the count beside them.
///
/// `(icons, count)`: `count` is `Some(n)` only when there are too many to draw
/// one each; then one icon is drawn and the number says the rest. Zero stocks
/// draw nothing: the fighter is out.
fn drawn_stocks(remaining: u32) -> (u32, Option<u32>) {
    if remaining > MAX_DRAWN_STOCKS {
        (1, Some(remaining))
    } else {
        (remaining, None)
    }
}

/// Width occupied by a row of player panels within the available gameplay area.
fn panel_row_span(available: f32, count: usize) -> f32 {
    // Based on available width, but never less than the total panel width,
    // so panels do not overlap.
    (available * ROW_FRACTION).max(PANEL_W * count.max(1) as f32)
}

/// How much of the gameplay width a full panel row uses. It stops short of
/// the edges because a host draws its own buttons in the corners.
const ROW_FRACTION: f32 = 0.72;

/// Where one panel's LEFT edge sits, given its place in the row.
///
/// Centred as a group about `centre_x`, so two panels sit either side of the
/// middle and four spread evenly across it.
fn panel_left(centre_x: f32, available: f32, index: usize, count: usize) -> f32 {
    let span = panel_row_span(available, count);
    let pitch = span / count.max(1) as f32;
    let first_centre = centre_x - span * 0.5 + pitch * 0.5;
    first_centre + pitch * index as f32 - PANEL_W * 0.5
}

/// HUD image handles already loaded by this process, kept alive on purpose.
///
/// The HUD holds the only handle to a portrait. Without this cache, the image
/// drops when the entity despawns and every select-screen visit decodes the
/// same portraits again.
///
/// Bounded: one entry per portrait actually shown (~1.3-2.0 MP each), not
/// every baked portrait. A cast-sized set of small images needs no eviction
/// policy, unlike the character sheet table.
#[derive(Resource, Default)]
pub struct RetainedHudImages {
    by_path: std::collections::HashMap<String, Handle<Image>>,
    /// Requests answered from the cache, and requests that had to load.
    ///
    /// Cache hits are the signal for this cache: `loads` rising while `hits`
    /// stays flat means the re-decode bug is back.
    served: u64,
    loaded: u64,
}

impl RetainedHudImages {
    /// How many requests were answered without loading, and how many loaded.
    pub fn hits_and_loads(&self) -> (u64, u64) {
        (self.served, self.loaded)
    }

    /// The handle for `path`, loading it once and keeping it thereafter.
    fn handle(&mut self, asset_server: &AssetServer, path: String) -> Handle<Image> {
        if self.by_path.contains_key(&path) {
            self.served += 1;
        } else {
            self.loaded += 1;
        }
        self.by_path
            .entry(path)
            .or_insert_with_key(|path| {
                // Stamped like every other portrait load, so the ledger records its
                // demand (a bare `load` shows as `demand=unknown`).
                ambition_sprite_sheet::game_assets::load_sheet_image(
                    asset_server,
                    "portrait",
                    path.clone(),
                )
            })
            .clone()
    }
}

/// Lay the fighter panels out across their region and hang each one's pieces
/// off its slot's live text node.
///
/// Runs after the placer, like the gauges: a panel follows the position that
/// frame settled on.
pub fn update_declared_hud_panels(
    readouts: Res<HudReadouts>,
    active: Res<ActiveHudDeclaration>,
    presentation: Res<ResolvedGameplayPresentation>,
    asset_server: Res<AssetServer>,
    mut retained_hud_images: ResMut<RetainedHudImages>,
    mut slots: Query<
        (&DeclaredHudSlot, &DeclaredHudSpec, &mut Node),
        (
            Without<DeclaredHudPanel>,
            Without<DeclaredHudPortrait>,
            Without<DeclaredHudStock>,
            Without<DeclaredHudStockCount>,
        ),
    >,
    mut panels: Query<
        (&DeclaredHudPanel, &mut Node, &mut Visibility),
        (
            Without<DeclaredHudPortrait>,
            Without<DeclaredHudStock>,
            Without<DeclaredHudStockCount>,
        ),
    >,
    mut portraits: Query<
        (&DeclaredHudPortrait, &mut ImageNode, &mut Visibility),
        (Without<DeclaredHudStock>, Without<DeclaredHudStockCount>),
    >,
    mut stocks: Query<
        (&DeclaredHudStock, &mut ImageNode, &mut Visibility),
        (Without<DeclaredHudPortrait>, Without<DeclaredHudStockCount>),
    >,
    mut counts: Query<(&DeclaredHudStockCount, &mut Text, &mut Visibility)>,
) {
    let panelled = panelled_slots(&active, &readouts);
    let count = panelled.len();
    // The gameplay rectangle's centre, not the window's; they differ under
    // letterboxing.
    let centre_x = presentation.gameplay_rect.min.x + presentation.gameplay_rect.width() * 0.5;

    // ── the panel roots, and the slot text they carry ────────────────────
    for (panel, mut node, mut visibility) in &mut panels {
        let Some(index) = panelled.iter().position(|id| *id == panel.0) else {
            set_hidden(&mut visibility);
            continue;
        };
        set_shown(&mut visibility);
        // The slot's own declared font size: the percent uses it, and panel
        // height is portrait + that line + the stock row.
        let panel_font = slots
            .iter()
            .find(|(slot, ..)| slot.0 == panel.0)
            .map(|(_, spec, _)| spec.0.font_size)
            .unwrap_or(22.0);
        let left = panel_left(centre_x, presentation.gameplay_rect.width(), index, count);
        set_px(&mut node.left, left);
        set_px(&mut node.width, PANEL_W);

        // The slot's own text is the percent, under the portrait. A panelled
        // slot overrides `place_declared_hud`'s position; this is the only place
        // the renderer takes a position back from the stacker.
        let top = panel_top(&presentation, panel_font);
        set_px(&mut node.top, top);
        for (slot, _, mut slot_node) in &mut slots {
            if slot.0 != panel.0 {
                continue;
            }
            set_px(&mut slot_node.left, left);
            set_px(&mut slot_node.width, PANEL_W);
            set_px(&mut slot_node.top, top + PORTRAIT_PX + 2.0);
        }
    }

    // ── the portraits ────────────────────────────────────────────────────
    for (portrait, mut image, mut visibility) in &mut portraits {
        let standing = readouts
            .get(&portrait.0)
            .and_then(|readout| readout.standing_of());
        match standing.and_then(|standing| standing.portrait.clone()) {
            // No portrait draws nothing, not a blank box (which reads as failed
            // art).
            None => set_hidden(&mut visibility),
            Some(path) => {
                set_shown(&mut visibility);
                // Through the retained cache, so a second visit does not re-decode.
                let handle = retained_hud_images.handle(&asset_server, path);
                if image.image != handle {
                    image.image = handle;
                }
                // The face out of the page. A portrait sheet holds every clip, so the
                // whole image would squeeze them all into this box. `None` is the whole
                // image, for a single-frame portrait.
                let frame = standing.and_then(|standing| standing.portrait_frame);
                if image.rect != frame {
                    image.rect = frame;
                }
            }
        }
    }

    // ── the stock icons, and the count that replaces them ────────────────
    for (stock, mut image, mut visibility) in &mut stocks {
        let standing = readouts
            .get(&stock.0)
            .and_then(|readout| readout.standing_of());
        let Some(standing) = standing else {
            set_hidden(&mut visibility);
            continue;
        };
        let (drawn, _) = drawn_stocks(standing.remaining);
        let Some(path) = standing.stock_icon.clone() else {
            set_hidden(&mut visibility);
            continue;
        };
        if stock.1 >= drawn {
            set_hidden(&mut visibility);
            continue;
        }
        set_shown(&mut visibility);
        // Same cache as the portraits, for the same reason: the HUD holds the
        // only handle.
        let handle = retained_hud_images.handle(&asset_server, path);
        if image.image != handle {
            image.image = handle;
        }
    }
    for (count_of, mut text, mut visibility) in &mut counts {
        let standing = readouts
            .get(&count_of.0)
            .and_then(|readout| readout.standing_of());
        match standing {
            Some(standing) if drawn_stocks(standing.remaining).1.is_some() => {
                set_shown(&mut visibility);
                let next = format!("x{}", standing.remaining);
                if text.0 != next {
                    text.0 = next;
                }
            }
            _ => set_hidden(&mut visibility),
        }
    }
}

/// How tall a whole panel is — portrait, the percent under it, the stock
/// row under that.
fn panel_height(font_size: f32) -> f32 {
    PORTRAIT_PX + font_size * LINE_HEIGHT_FACTOR + 4.0 + STOCK_ICON_PX + HUD_MARGIN
}

/// The top of the panel row, positioned so the WHOLE panel is on screen.
fn panel_top(presentation: &ResolvedGameplayPresentation, font_size: f32) -> f32 {
    let height = panel_height(font_size);
    match presentation.hud_region(SurroundRegion::Bottom) {
        // The reserved strip, against its top edge, and pulled up if the
        // strip is shorter than the panel, so a thin letterbox clips the
        // background rather than the numbers.
        Some(rect) => rect.min.y.min(rect.max.y - height).max(0.0) + HUD_MARGIN * 0.5,
        // No reserved surround: overlay inside the gameplay rectangle, on its
        // bottom edge.
        None => (presentation.gameplay_rect.max.y - height - HUD_MARGIN).max(0.0),
    }
}

fn set_px(value: &mut Val, px: f32) {
    let next = Val::Px(px);
    if *value != next {
        *value = next;
    }
}

fn set_shown(visibility: &mut Visibility) {
    if *visibility != Visibility::Inherited {
        *visibility = Visibility::Inherited;
    }
}

fn set_hidden(visibility: &mut Visibility) {
    if *visibility != Visibility::Hidden {
        *visibility = Visibility::Hidden;
    }
}

/// Installs the declared-HUD surface.
///
/// Part of the presentation face, not any one app: a game gets a HUD by
/// declaring one, with no per-game app wiring. A route that declares nothing
/// spawns nothing.
pub struct DeclaredHudPlugin;

impl Plugin for DeclaredHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveHudDeclaration>()
            .init_resource::<HudReadouts>();
        app.add_systems(
            Update,
            (
                spawn_declared_hud,
                update_declared_hud,
                // Uses this frame's resolved HUD regions, so a profile that reserves
                // surround gets the readouts there.
                place_declared_hud.after(
                    ambition_platformer2d_shared_tangle::gameplay_presentation::GameplayPresentationSet,
                ),
                // After the placer: a gauge follows its slot's position this frame.
                update_declared_hud_gauges.after(place_declared_hud),
                // After the placer for the same reason; it also takes the position
                // back for a panelled slot.
                update_declared_hud_panels.after(place_declared_hud),
            )
                .chain()
                .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
        );
        // Outlives any one session, so returning to the select screen does not
        // decode the portraits again.
        app.init_resource::<RetainedHudImages>();
    }
}

#[cfg(test)]
mod punch_tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::gameplay_presentation::{HudReadout, HudStanding};

    fn standing(emphasis: f32) -> HudReadout {
        HudReadout::standing(
            String::new(),
            "88%".to_string(),
            HudStanding {
                portrait: None,
                portrait_frame: None,
                stock_icon: None,
                remaining: 2,
                started: 3,
                emphasis,
            },
        )
    }

    fn drawn_size(emphasis: f32, frames: usize) -> f32 {
        let mut app = App::new();
        let mut readouts = HudReadouts::default();
        readouts.set(HudSlotId::new("p1"), standing(emphasis));
        app.insert_resource(readouts);
        app.add_systems(Update, update_declared_hud);
        let node = app
            .world_mut()
            .spawn((
                DeclaredHudSlot(HudSlotId::new("p1"), 16.0),
                Text::new(String::new()),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..Default::default()
                },
            ))
            .id();
        for _ in 0..frames {
            app.update();
        }
        // The assertions compare pixel counts, so unwrap the unit here. This
        // pass only writes `Px`.
        match app
            .world()
            .get::<TextFont>(node)
            .expect("still a node")
            .font_size
        {
            FontSize::Px(px) => px,
            other => panic!("the HUD punch wrote a non-pixel font size: {other:?}"),
        }
    }

    /// The punch must not compound, which is why the node stores its declared
    /// size.
    ///
    /// A scale of the current size multiplies every frame. A single-tick test
    /// cannot see that, so this one runs sixty ticks.
    #[test]
    fn a_held_punch_does_not_grow_the_readout_every_frame() {
        let one = drawn_size(1.0, 1);
        let sixty = drawn_size(1.0, 60);
        assert!(
            (one - sixty).abs() < 0.01,
            "the readout grew from {one} to {sixty} while the emphasis was held \
             — the scale is compounding on itself"
        );
        assert!(
            (sixty - 16.0 * (1.0 + HUD_PUNCH_GAIN)).abs() < 0.01,
            "a full punch is not the declared size plus the gain: {sixty}"
        );
    }

    /// It returns: with no emphasis, the readout draws at the declared size.
    #[test]
    fn no_emphasis_draws_exactly_the_declared_size() {
        assert!((drawn_size(0.0, 4) - 16.0).abs() < 0.01);
        // Half a punch is half the gain: proportional, not a latch.
        let half = drawn_size(0.5, 2);
        assert!(
            (half - 16.0 * (1.0 + HUD_PUNCH_GAIN * 0.5)).abs() < 0.01,
            "a half punch drew {half}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::gameplay_presentation::{
        HudDeclaration, HudLayoutPolicy, HudReadout, NamedScreenRect,
    };

    /// The panels are centred as a group for any player count: two panels sit
    /// either side of the middle, four spread across it. Packing from the left
    /// would put a 1v1 in the corner.
    #[test]
    fn a_panel_row_is_centred_on_the_screen_for_any_player_count() {
        let centre = 640.0;
        for count in 1..=4usize {
            let lefts: Vec<f32> = (0..count)
                .map(|i| panel_left(centre, 1280.0, i, count))
                .collect();
            let first = lefts[0];
            let last = lefts[count - 1] + PANEL_W;
            let row_centre = (first + last) * 0.5;
            assert!(
                (row_centre - centre).abs() < 0.001,
                "{count} panels centred at {row_centre}, not {centre}: {lefts:?}"
            );
        }
    }

    /// Panels in a row never overlap.
    #[test]
    fn panels_in_a_row_never_overlap() {
        for count in 1..=4usize {
            for index in 1..count {
                let previous = panel_left(640.0, 1280.0, index - 1, count) + PANEL_W;
                let current = panel_left(640.0, 1280.0, index, count);
                assert!(
                    current >= previous - 0.001,
                    "panel {index} of {count} starts at {current}, inside the one ending at {previous}"
                );
            }
        }
    }

    /// Few stocks are icons; many are a count. The boundary is tested from both
    /// sides, so the threshold cannot drift by one.
    #[test]
    fn stocks_draw_as_icons_until_there_are_too_many() {
        assert_eq!(
            drawn_stocks(0),
            (0, None),
            "a fighter who is out draws icons"
        );
        assert_eq!(drawn_stocks(1), (1, None));
        assert_eq!(
            drawn_stocks(MAX_DRAWN_STOCKS),
            (MAX_DRAWN_STOCKS, None),
            "the threshold itself still draws one icon each"
        );
        assert_eq!(
            drawn_stocks(MAX_DRAWN_STOCKS + 1),
            (1, Some(MAX_DRAWN_STOCKS + 1)),
            "one past the threshold must collapse to a single icon and a count"
        );
        assert_eq!(drawn_stocks(99), (1, Some(99)));
    }

    #[test]
    fn same_slot_id_with_changed_style_forces_a_rebuild() {
        let old = DeclaredHudSpec(HudSlotSpec::new("score").with_font_size(18.0));
        let new = HudSlotSpec::new("score").with_font_size(30.0);
        assert!(!declaration_matches_live_specs(
            &[new],
            [Some(&old)].into_iter()
        ));
    }

    #[test]
    fn a_live_node_without_a_cached_spec_forces_a_rebuild() {
        let spec = HudSlotSpec::new("score");
        assert!(!declaration_matches_live_specs(&[spec], [None].into_iter(),));
    }

    #[test]
    fn an_identical_slot_spec_keeps_the_existing_node() {
        let spec = HudSlotSpec::new("score").with_font_size(18.0);
        let live = DeclaredHudSpec(spec.clone());
        assert!(declaration_matches_live_specs(
            &[spec],
            [Some(&live)].into_iter(),
        ));
    }

    /// Rebuilding a declaration leaves exactly one gauge root per slot.
    #[test]
    fn restyling_a_slot_does_not_accumulate_gauge_bars() {
        let mut app = App::new();
        app.insert_resource(ResolvedGameplayPresentation::default());
        app.insert_resource(ActiveHudDeclaration(Some(
            HudDeclaration::new().slot(HudSlotSpec::new("health").with_font_size(18.0)),
        )));
        app.add_systems(Update, spawn_declared_hud);
        app.update();

        let bars = |app: &mut App| {
            let world = app.world_mut();
            let mut query = world.query::<&DeclaredHudBar>();
            query.iter(world).count()
        };
        assert_eq!(bars(&mut app), 1, "the first build spawned no gauge at all");

        // Same id, different style: forces a rebuild without a route change.
        for size in [24.0_f32, 30.0, 36.0] {
            *app.world_mut().resource_mut::<ActiveHudDeclaration>() = ActiveHudDeclaration(Some(
                HudDeclaration::new().slot(HudSlotSpec::new("health").with_font_size(size)),
            ));
            app.update();
        }
        assert_eq!(
            bars(&mut app),
            1,
            "three restyles left {} gauge bars for one slot — every rebuild \
             despawned the text and abandoned its bar, and they draw on top of \
             each other",
            bars(&mut app)
        );
    }

    #[test]
    fn slots_falling_back_to_the_same_region_stack_instead_of_overlapping() {
        let mut presentation = ResolvedGameplayPresentation {
            hud: HudLayoutPolicy::PreferSurround,
            ..Default::default()
        };
        presentation.controls.hud = vec![NamedScreenRect {
            region: SurroundRegion::Left,
            rect: ScreenRect::from_min_size(Vec2::ZERO, Vec2::new(200.0, 200.0)),
        }];
        let declaration = HudDeclaration::new()
            .slot(
                HudSlotSpec::new("top_preference")
                    .with_region(SurroundRegion::Top)
                    .with_min_px(Vec2::new(20.0, 20.0)),
            )
            .slot(
                HudSlotSpec::new("bottom_preference")
                    .with_region(SurroundRegion::Bottom)
                    .with_min_px(Vec2::new(20.0, 20.0)),
            );

        let mut app = App::new();
        app.insert_resource(presentation);
        app.insert_resource(ActiveHudDeclaration(Some(declaration)));
        app.init_resource::<HudReadouts>();
        app.add_systems(Update, place_declared_hud);
        let top = app
            .world_mut()
            .spawn((
                DeclaredHudSlot(HudSlotId::new("top_preference"), 16.0),
                Node::default(),
            ))
            .id();
        let bottom = app
            .world_mut()
            .spawn((
                DeclaredHudSlot(HudSlotId::new("bottom_preference"), 16.0),
                Node::default(),
            ))
            .id();

        app.update();
        let top_y = match app.world().get::<Node>(top).expect("top node").top {
            Val::Px(y) => y,
            ref other => panic!("top slot must use a pixel anchor, got {other:?}"),
        };
        let bottom_y = match app.world().get::<Node>(bottom).expect("bottom node").top {
            Val::Px(y) => y,
            ref other => panic!("bottom slot must use a pixel anchor, got {other:?}"),
        };
        assert!(
            bottom_y > top_y,
            "two preferences that fall back to Left must share its stack: {top_y} vs {bottom_y}",
        );
    }

    /// A slot is as tall as what it published, not one line. Otherwise a
    /// three-line card has the next slot drawn over its second and third lines
    /// (TwinTrack publishes four such slots).
    #[test]
    fn a_multi_line_readout_pushes_the_next_slot_below_all_of_its_lines() {
        const SIZE: f32 = 20.0;
        let declaration = HudDeclaration::new()
            .slot(HudSlotSpec::new("tall").with_font_size(SIZE))
            .slot(HudSlotSpec::new("after").with_font_size(SIZE));

        let mut app = App::new();
        // No reserved surround: both slots use the overlay stack, like an
        // ordinary window.
        app.insert_resource(ResolvedGameplayPresentation::default());
        app.insert_resource(ActiveHudDeclaration(Some(declaration)));
        let mut readouts = HudReadouts::default();
        readouts.set("tall", HudReadout::bare("one\ntwo\nthree"));
        app.insert_resource(readouts);
        app.add_systems(Update, place_declared_hud);
        let tall = app
            .world_mut()
            .spawn((
                DeclaredHudSlot(HudSlotId::new("tall"), 16.0),
                Node::default(),
            ))
            .id();
        let after = app
            .world_mut()
            .spawn((
                DeclaredHudSlot(HudSlotId::new("after"), 16.0),
                Node::default(),
            ))
            .id();
        app.update();

        let y = |entity| match app.world().get::<Node>(entity).expect("node").top {
            Val::Px(y) => y,
            ref other => panic!("expected a pixel anchor, got {other:?}"),
        };
        let gap = y(after) - y(tall);
        assert!(
            gap >= SIZE * 3.0,
            "a three-line card was allotted {gap}px — the next slot is drawn              through its own text",
        );
    }
}

#[cfg(test)]
mod retained_hud_image_tests {
    use super::RetainedHudImages;
    use bevy::prelude::*;

    fn asset_app() -> App {
        let mut app = App::new();
        // `TaskPoolPlugin` first: `AssetServer::load` dispatches onto the IO
        // pool and panics without it.
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::asset::AssetPlugin::default(),
        ));
        app.init_asset::<Image>();
        app
    }

    /// The property is retention, not handle identity.
    ///
    /// "Asking twice returns the same handle" cannot fail: `AssetServer::load`
    /// dedupes by path while the asset is alive. The fix is that this map holds
    /// its own strong handle, so the image survives the HUD entity despawning.
    #[test]
    fn the_cache_keeps_a_handle_after_the_caller_drops_theirs() {
        let app = asset_app();
        let server = app.world().resource::<AssetServer>().clone();
        let mut retained = RetainedHudImages::default();

        let handle = retained.handle(&server, "sprites/noether_portraits.png".to_string());
        let id = handle.id();
        drop(handle);

        let held = retained
            .by_path
            .get("sprites/noether_portraits.png")
            .expect("the cache must still hold the portrait after the caller drops it");
        assert_eq!(
            held.id(),
            id,
            "the cache holds a handle to a different asset than it handed out"
        );
    }

    /// Control: two different portraits must not share one entry.
    #[test]
    fn different_portraits_keep_separate_entries() {
        let app = asset_app();
        let server = app.world().resource::<AssetServer>().clone();
        let mut retained = RetainedHudImages::default();

        retained.handle(&server, "sprites/noether_portraits.png".to_string());
        retained.handle(&server, "sprites/officer_portraits.png".to_string());
        assert_eq!(retained.by_path.len(), 2, "two portraits shared one entry");
    }
}
