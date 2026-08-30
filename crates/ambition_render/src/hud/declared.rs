//! The renderer for a game's declared HUD readouts.
//!
//! This module draws whatever the ACTIVE ROUTE declared instead, and knows nothing about what
//! any of it means: it spawns one text node per
//! [`HudSlotSpec`](ambition_platformer2d_shared_tangle::gameplay_presentation::HudSlotSpec) and
//! mirrors the matching
//! [`HudReadouts`](ambition_platformer2d_shared_tangle::gameplay_presentation::HudReadouts)
//! entry into it every frame. "RINGS", "SCORE", "TIME" are strings a game writes; none of them
//! appear here.
//!
//! Placement reuses the ladder the built-in HUD already walks — ask
//! [`hud_region`] for the region the slot asked for, take it when the active
//! profile reserves a surround and it is big enough, otherwise overlay
//! gameplay. No layout negotiation: a readout knows its own size.
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
/// Public and deliberately load-bearing: it is how a demo's OV1 guard tells
/// "UI the engine's presentation face dragged in" (still forbidden) from "UI
/// this game declared" (the point of the seam).
#[derive(Component)]
pub struct DeclaredHudRoot;

/// One declared readout's text node, tagged with the slot it mirrors and the
/// font size it was DECLARED at.
///
/// ⛔ the size is carried rather than read back off the live `TextFont`, because
/// the emphasis scale is applied to it every frame: scaling whatever the font
/// happens to be NOW compounds, and a readout that is hit twice grows until it
/// fills the panel.
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
///  mirrored rather than read, because the spawned nodes take the default and
/// nothing here sets one. If a slot ever declares its own line height, this
/// derivation has to move to that value.
const LINE_HEIGHT_FACTOR: f32 = 1.2;

/// How much vertical room one slot's PUBLISHED readout needs.
///
/// Any game that publishes a `\n` hits it.
///
///  a slot with NO published readout still reserves one line: a conditional
/// card that blinks in and out would otherwise shove everything below it up and
/// down as it appeared, and a stable HUD that reserves a little too much beats
/// one that jumps.
fn slot_extent(spec: &HudSlotSpec, readouts: &HudReadouts, measured: Option<f32>) -> f32 {
    // `ComputedNode` carries what the layout actually produced, which is the only thing that
    // knows where the text broke.
    //
    //  last frame's height, because UI layout runs in `PostUpdate` and this
    // is an `Update` system. Moving a node's `top` does not change its height,
    // so there is no oscillation to converge — the lag shows only on the frame a
    // readout changes line count.
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
        // A node from an older declaration renderer has no cached spec and
        // must be rebuilt rather than silently treated as current.
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
/// Rebuilds from scratch whenever the active declaration changes shape, so
/// switching experiences in a shared host never leaves the previous game's
/// readouts on screen.
pub fn spawn_declared_hud(
    mut commands: Commands,
    active: Res<ActiveHudDeclaration>,
    active_session: Option<Res<ActiveSessionScope>>,
    fonts: Option<Res<crate::ui_fonts::UiFonts>>,
    existing: Query<(Entity, &DeclaredHudSlot, Option<&DeclaredHudSpec>)>,
    // Query every root owned by this pass, including sibling gauge roots, so a
    // declaration rebuild retires the complete previous HUD.
    owned: Query<Entity, With<DeclaredHudRoot>>,
) {
    let declared = active.slots();

    if declared.is_empty() {
        for entity in &owned {
            commands.entity(entity).despawn();
        }
        return;
    }

    // Already showing this declaration exactly — identity AND appearance.
    // Comparing ids alone left stale font/colour/centering/placement whenever a
    // route revised a slot without renaming it.
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
        // A shell host can retain a session for one deferred teardown frame.
        // Never materialize new gameplay UI without a live session owner.
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
                font_size: spec.font_size,
                ..default()
            });
        commands.spawn_session_scoped(
            session_scope,
            (
                DeclaredHudRoot,
                DeclaredHudBar(spec.id.clone()),
                Node {
                    position_type: PositionType::Absolute,
                    // Under the slot's text, spanning the width it declared a minimum for.
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
        // THE FIGHTER PANEL, spawned for every slot and shown only for one
        // publishing a `Standing`.  spawned ONCE with a fixed number of stock
        // icons and hidden per frame rather than spawned per stock: a family
        // that appears and disappears with a number would churn entities every
        // time somebody lost a life, and `DeclaredHudRoot`'s retire sweep is
        // built around one spawn per declaration.
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
                // The stock row sits under the percent, which is the slot's own
                // text node — so this row is spaced down past it.
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
                                font_size: STOCK_ICON_PX,
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
                bevy::text::TextLayout::new_with_justify(if spec.centered {
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
                // Generic screen occupancy, derived from this node's own
                // computed layout — the HUD says what it is, the host derives
                // where it is.
                ScreenOccluder::hud(),
            ),
        );
        *slot_offset += spec.font_size + SLOT_GAP;
    }
}

/// Move each declared readout into the region it asked for, when the active
/// profile leaves one big enough; otherwise leave it overlaying gameplay.
///
/// The same ladder `place_player_hud` walks, per slot instead of per widget,
/// because each slot declares its own region and minimum.
pub fn place_declared_hud(
    presentation: Res<ResolvedGameplayPresentation>,
    active: Res<ActiveHudDeclaration>,
    // What each slot is CURRENTLY showing, because how tall a slot is depends on
    // how many lines the game published into it this frame. See [`slot_extent`].
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
        // A centred card ignores the region ladder entirely: it belongs over
        // the gameplay rectangle, which is the thing the player is looking at.
        if spec.centered {
            let gameplay = presentation.gameplay_rect;
            for (slot, mut node, _) in &mut slots {
                if slot.0 != spec.id {
                    continue;
                }
                // Span the gameplay rect and let the text centre ITSELF inside
                // that span (the node carries `JustifyText::Center`). Setting
                // `left: 50%` instead puts the node's LEFT EDGE at the middle,
                // so the card starts at centre and runs off to the right — it
                // reads as "the HUD is in the middle of the screen" rather than
                // as a centred card, which is exactly how this shipped.
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
        // Prefer the declared region; fall back to any OTHER reserved region
        // before giving up and overlaying.
        //
        // Honouring only the declared region meant its `Top` readouts found nothing on every
        // ordinary monitor and fell through to the overlay corner — landing somewhere
        // reasonable purely by luck rather than by placement.
        let region = select_hud_region(&presentation, spec);

        // What this slot's text actually occupied last frame, in LOGICAL px —
        // `ComputedNode` is physical, and `Node::top` is not.
        let measured = slots
            .iter()
            .find(|(slot, ..)| slot.0 == spec.id)
            .and_then(|(_, _, computed)| computed)
            .map(|computed| computed.size().y * computed.inverse_scale_factor());

        let anchor = match region {
            Some((actual_region, rect)) => {
                // Two differently authored preferences may fall back to the
                // same physical region. Stack by the region actually chosen,
                // or both start at its origin and overlap.
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
/// A SIBLING root, not a child — it is positioned against the slot's live `Node`
/// every frame (see [`update_declared_hud_gauges`]) because the slot itself
/// moves between regions as the active presentation profile changes.
///
/// Both sweeps in [`spawn_declared_hud`] now key on [`DeclaredHudRoot`], which every spawn
/// there carries.
#[derive(bevy::prelude::Component, Debug)]
pub struct DeclaredHudBar(pub HudSlotId);

/// Size each slot's gauge from its published fill.
///
/// A slot whose readout has no `fill` collapses to zero size — so a game may
/// publish a gauge conditionally (a boss bar that appears with the boss)
/// without declaring two slots.
pub fn update_declared_hud_gauges(
    readouts: Res<HudReadouts>,
    // The slot's live node, so the bar FOLLOWS its placement. `place_declared_hud`
    // moves slots between regions as the active presentation profile changes, and
    // a bar pinned to where it spawned would drift away from the number it
    // belongs to the first time that happened.
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
            // A standing draws a PANEL, not a bar — see `update_declared_hud_panels`.
            // Collapsing here is what keeps a slot from wearing both.
            Some(HudFigure::Standing(_)) | None => {
                if node.height != Val::Px(0.0) {
                    node.height = Val::Px(0.0);
                    node.width = Val::Px(0.0);
                }
                continue;
            }
        };
        // The slot's own declared minimum width is the bar's full extent, so a
        // game sizes its gauge by declaring how much room it wants rather than
        // by knowing anything about pixels here.
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
        // ⭐ THE PUNCH. A readout that was just hit is drawn bigger for as long
        // as the hit is being felt — the same beat the freeze lasts, because it
        // is derived from the same number. A HUD that grew on its own schedule
        // would read as a second, laggier hit.
        //
        // ⛔ the BASE font size is recovered from the declaration rather than
        // remembered: a scale applied to whatever the font is now compounds every
        // frame, which is a readout that grows until it fills the screen.
        let emphasis = readout
            .and_then(|readout| readout.standing_of())
            .map(|standing| standing.emphasis.clamp(0.0, 1.0))
            .unwrap_or(0.0);
        let base = slot.1;
        let wanted = base * (1.0 + emphasis * HUD_PUNCH_GAIN);
        if (font.font_size - wanted).abs() > 0.01 {
            font.font_size = wanted;
        }
    }
}

/// How much bigger a freshly-hit readout draws, at full emphasis.
///
/// A quarter again: enough that the eye catches it in peripheral vision during a
/// fight, small enough that a 132px panel still holds the text — the constraint
/// that already decided this panel draws no name beside its number.
const HUD_PUNCH_GAIN: f32 = 0.25;

// ---------------------------------------------------------------------------
// The fighter panel: a portrait, the percent under it, and the stocks as icons
// ---------------------------------------------------------------------------

/// How many stocks are drawn one-icon-each before it becomes a count.
///
/// the genre's own break point, not a guess: a platform fighter draws a row of little heads while
/// there are few enough to read at a glance, and switches to `xN` once counting them would take
/// longer than reading a number.
pub const MAX_DRAWN_STOCKS: u32 = 5;

/// How wide one fighter panel is, and how big the pieces in it are.
const PANEL_W: f32 = 132.0;
const PORTRAIT_PX: f32 = 56.0;
const STOCK_ICON_PX: f32 = 14.0;
const STOCK_ICON_GAP: f32 = 3.0;

/// One fighter panel's root, tagged with the slot it belongs to.
///
/// A SIBLING root rather than a child of the slot's text node, for the reason
/// the gauge bar is one: the text node moves between regions as the active
/// profile changes, and this tracks it every frame.
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
///  the count is what "horizontally distributed depending on the number of
/// players" means, and it is a fact about the READOUTS rather than the
/// declaration: the smash stage declares four slots and a 1v1 publishes two, so
/// asking the declaration would space a two-player match as if four people were
/// playing and leave two gaps.
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
/// `(icons, count)` — `count` is `Some(n)` only when there are too many to draw
/// one each, in which case exactly one icon is drawn and the number says the
/// rest.  zero stocks draw NOTHING and that is not an error: it is a fighter
/// who is out, and an empty row is what says so.
fn drawn_stocks(remaining: u32) -> (u32, Option<u32>) {
    if remaining > MAX_DRAWN_STOCKS {
        (1, Some(remaining))
    } else {
        (remaining, None)
    }
}

/// Width occupied by a row of player panels within the available gameplay area.
fn panel_row_span(available: f32, count: usize) -> f32 {
    // Base span on available screen width, but never below the total panel width
    // needed to avoid overlap.
    (available * ROW_FRACTION).max(PANEL_W * count.max(1) as f32)
}

/// How much of the gameplay width a full panel row occupies. Short of the edges
/// on purpose: a host draws its own buttons in the corners.
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

/// Lay the fighter panels out across their region and hang each one's pieces
/// off its slot's live text node.
///
///  after the placer, like the gauges: a panel tracks a position that
/// frame settled on.
/// HUD image handles this process has already loaded, kept alive on purpose.
///
/// ⛔⛔ WITHOUT THIS, EVERY SELECT-SCREEN VISIT RE-DECODES THE SAME PORTRAITS.
/// The HUD holds the only handle to a portrait; when its entity despawns the last
/// reference goes, Bevy drops the image, and the next visit decodes it again.
/// Measured on hardware 2026-08-29: the select screen's set decoded TWICE in one
/// session (56.2s and 71.9s, the same eight names), and after the phase-scoped
/// analysis those were **15 of the 15 decodes that landed in settled play** —
/// every other decode in the run was boot or a room still arriving.
///
/// ⭐ BOUNDED BY CONSTRUCTION, which is why this is a cache and not the residency
/// service the sheet store forbids: it holds one entry per portrait ACTUALLY
/// SHOWN (~1.3–2.0MP each), not the 163 baked portrait manifests. A cast-sized
/// set of small images is a different object from a 470MB-per-character sheet
/// table, and it needs no eviction policy to stay bounded.
#[derive(Resource, Default)]
pub struct RetainedHudImages {
    by_path: std::collections::HashMap<String, Handle<Image>>,
    /// Requests answered from the cache, and requests that had to load.
    ///
    /// ⭐ THE CAMPAIGN ROW ASKED FOR CACHE HITS, AND THIS IS THE ONLY PLACE THEY
    /// MEAN ANYTHING HERE. A decode count says an image arrived; it cannot say
    /// whether a screen was reopened and served without one. `loads` climbing
    /// while `hits` stays flat is the bug this cache exists to prevent, coming
    /// back.
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
            .or_insert_with_key(|path| asset_server.load(path.clone()))
            .clone()
    }
}

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
    // The gameplay rectangle's centre, so the row is centred on what the player
    // is looking at rather than on the window — they differ under letterboxing.
    let centre_x = presentation.gameplay_rect.min.x + presentation.gameplay_rect.width() * 0.5;

    // ── the panel roots, and the slot text they carry ────────────────────
    for (panel, mut node, mut visibility) in &mut panels {
        let Some(index) = panelled.iter().position(|id| *id == panel.0) else {
            set_hidden(&mut visibility);
            continue;
        };
        set_shown(&mut visibility);
        // The slot's OWN declared font size, because the percent is drawn in it
        // and the panel's height is portrait + that line + the stock row.
        let panel_font = slots
            .iter()
            .find(|(slot, ..)| slot.0 == panel.0)
            .map(|(_, spec, _)| spec.0.font_size)
            .unwrap_or(22.0);
        let left = panel_left(centre_x, presentation.gameplay_rect.width(), index, count);
        set_px(&mut node.left, left);
        set_px(&mut node.width, PANEL_W);

        // The slot's own text is the PERCENT, and it belongs under the
        // portrait. `place_declared_hud` put it wherever the region stacker
        // wanted; a panelled slot overrides that, which is the one place this
        // renderer takes a position back from the stacker.
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
            //  a fighter with no portrait draws none rather than a blank box:
            // an empty rectangle reads as art that failed to load.
            None => set_hidden(&mut visibility),
            Some(path) => {
                set_shown(&mut visibility);
                // Through the retained cache: a second visit must not re-decode.
                let handle = retained_hud_images.handle(&asset_server, path);
                if image.image != handle {
                    image.image = handle;
                }
                // The FACE out of the page. A portrait sheet holds every clip
                // this character can wear, so drawing the whole image squeezes
                // the lot into this box. `None` is the whole image, which is
                // what a single-frame portrait wants.
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
        // only handle, so despawning its entity drops the icon and the next
        // screen decodes it again.
        // ⚠ NOT observed in the hardware run — a stock icon is below the census's
        // 1MP notable threshold, so it would not have appeared either way. Fixed
        // because it is the IDENTICAL defect, not because it was measured.
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
        // The reserved strip, sat against its top edge — and pulled up if the
        // strip is shorter than the panel, so a thin letterbox clips the
        // BACKGROUND rather than the numbers.
        Some(rect) => rect.min.y.min(rect.max.y - height).max(0.0) + HUD_MARGIN * 0.5,
        // No reserved surround: overlay INSIDE the gameplay rectangle, sat on
        // its bottom edge, which is where a fighting game's HUD belongs anyway.
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
/// Belongs to the presentation face rather than any one app, because the whole
/// point of the seam is that a game gets a HUD by DECLARING one — no app-side
/// wiring per game. A route that declared nothing spawns nothing, so hosts
/// whose games have no HUD are unaffected.
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
                // Consumes THIS frame's resolved HUD regions, so a profile
                // that reserves surround actually gets the readouts put there.
                place_declared_hud.after(
                    ambition_platformer2d_shared_tangle::gameplay_presentation::GameplayPresentationSet,
                ),
                // AFTER the placer: a gauge tracks its slot's live position, so
                // it has to read the position this frame settled on.
                update_declared_hud_gauges.after(place_declared_hud),
                // AFTER the placer for the same reason, and it takes the
                // position back for a panelled slot — see the note there.
                update_declared_hud_panels.after(place_declared_hud),
            )
                .chain()
                .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
        );
        // Outlives any one session on purpose: the point is that leaving the
        // select screen and coming back does not decode the portraits again.
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
                    font_size: 16.0,
                    ..Default::default()
                },
            ))
            .id();
        for _ in 0..frames {
            app.update();
        }
        app.world()
            .get::<TextFont>(node)
            .expect("still a node")
            .font_size
    }

    /// ⛔⛔ THE PUNCH MUST NOT COMPOUND, and this is the whole reason the node
    /// carries its DECLARED size.
    ///
    /// A scale applied to whatever the font happens to be NOW multiplies every
    /// frame: at a quarter again, a readout held under emphasis for a second is
    /// drawn about four thousand times its size. The failure is invisible in a
    /// single-tick test, which is why this one runs sixty.
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

    /// ⭐ AND IT COMES BACK. A punch that never returns to the declared size is
    /// a HUD that is permanently bigger after the first hit of the match.
    #[test]
    fn no_emphasis_draws_exactly_the_declared_size() {
        assert!((drawn_size(0.0, 4) - 16.0).abs() < 0.01);
        // Half a punch is half the gain — the scale is proportional rather than
        // a latch, so a light hit reads lighter than a heavy one.
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

    /// THE PANELS ARE CENTRED AS A GROUP, whatever the player count.
    ///
    ///  this is what "horizontally distributed depending on the number of
    /// players" has to mean: a 1v1 sits two panels either side of the middle
    /// and a four-player match spreads four across it, and in BOTH the row's
    /// own centre is the screen's. A layout that packed from the left would put
    /// a 1v1 in the corner.
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

    /// AND THEY DO NOT OVERLAP. Two panels sharing pixels is two percents
    /// on top of each other, which is the failure a HUD cannot have.
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

    /// FEW STOCKS ARE ICONS; MANY ARE A COUNT.
    ///
    ///  the boundary is asserted from both sides. A rule that only checked the
    /// small case would let the threshold drift by one and nobody would see it
    /// until a HUD tried to draw nine little heads.
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

        // Same id, different style — the case that forces a rebuild without a
        // route change, and the one this regressed on.
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

    /// A slot as tall as what it PUBLISHED, not as tall as one line.
    ///
    ///  the stack advanced by `font_size + gap` whatever the readout said, so
    /// a game publishing a three-line card had the next slot drawn across its
    /// second and third lines. TwinTrack does exactly that in four slots at
    /// once, and its top-left corner is unreadable because of it.
    #[test]
    fn a_multi_line_readout_pushes_the_next_slot_below_all_of_its_lines() {
        const SIZE: f32 = 20.0;
        let declaration = HudDeclaration::new()
            .slot(HudSlotSpec::new("tall").with_font_size(SIZE))
            .slot(HudSlotSpec::new("after").with_font_size(SIZE));

        let mut app = App::new();
        // No reserved surround: both slots land in the overlay stack, which is
        // the arrangement every ordinary window produces.
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
        // ⚠ `TaskPoolPlugin` FIRST: `AssetServer::load` dispatches onto the IO
        // pool, so an `App::new()` with only `AssetPlugin` panics inside
        // `bevy_tasks`. The neighbouring asset tests do not hit this because they
        // insert images directly and never call `load`.
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::asset::AssetPlugin::default(),
        ));
        app.init_asset::<Image>();
        app
    }

    /// ⭐ THE PROPERTY IS RETENTION, NOT HANDLE IDENTITY.
    ///
    /// ⛔ "asking twice returns the same handle" is a check that CANNOT FAIL:
    /// `AssetServer::load` dedupes by path and hands back the same handle while
    /// the asset is alive, so a cache that reloaded on every call passes it.
    /// Poison-proven.
    ///
    /// What fixes the bug is that this map holds a STRONG handle of its own, so
    /// the image survives the HUD entity despawning — which is what made the
    /// select screen re-decode its portraits on a second visit.
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

    /// ⛔ The control: two different portraits must not collapse onto one entry.
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
