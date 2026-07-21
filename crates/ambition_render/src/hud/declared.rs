//! The renderer for a game's **declared** HUD readouts.
//!
//! The sibling module draws Ambition's fixed HP/MP/$ widget — three readouts
//! hardcoded into the engine, which is exactly why no other game could have a
//! HUD without a core edit. This module draws whatever the ACTIVE ROUTE
//! declared instead, and knows nothing about what any of it means: it spawns
//! one text node per
//! [`HudSlotSpec`](ambition_platformer_primitives::gameplay_presentation::HudSlotSpec)
//! and mirrors the matching
//! [`HudReadouts`](ambition_platformer_primitives::gameplay_presentation::HudReadouts)
//! entry into it every frame. "RINGS", "SCORE", "TIME" are strings a game
//! writes; none of them appear here.
//!
//! Placement reuses the ladder the built-in HUD already walks — ask
//! [`hud_region`] for the region the slot asked for, take it when the active
//! profile reserves a surround and it is big enough, otherwise overlay
//! gameplay. No layout negotiation: a readout knows its own size.
//!
//! [`hud_region`]:
//!     ambition_platformer_primitives::gameplay_presentation::ResolvedGameplayPresentation::hud_region

use bevy::prelude::*;

use ambition_platformer_primitives::{
    gameplay_presentation::{
        ActiveHudDeclaration, HudReadouts, HudSlotId, HudSlotSpec, ResolvedGameplayPresentation,
        ScreenOccluder, ScreenRect, SurroundRegion,
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

/// One declared readout's text node, tagged with the slot it mirrors.
#[derive(Component)]
pub struct DeclaredHudSlot(pub HudSlotId);

/// The exact declaration used to build a node.
///
/// Slot ids are stable identities, not cache keys for appearance. Retaining the
/// full spec lets a route update font, colour, centering, order, or region while
/// keeping the same id and still receive a rebuilt node.
#[derive(Component, Clone, Debug)]
pub struct DeclaredHudSpec(HudSlotSpec);

/// Gap between stacked readouts in the same region.
const SLOT_GAP: f32 = 6.0;

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
    let fits =
        |rect: &ScreenRect| rect.width() >= spec.min_px.x && rect.height() >= spec.min_px.y;
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
) {
    let declared = active.slots();

    // Nothing declared: retire anything a previous route left behind.
    if declared.is_empty() {
        for (entity, _, _) in &existing {
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
    for (entity, _, _) in &existing {
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
                DeclaredHudSlot(spec.id.clone()),
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
    mut slots: Query<(&DeclaredHudSlot, &mut Node)>,
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
            for (slot, mut node) in &mut slots {
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
        // A game declares where it would LIKE its readouts, but which surround
        // a profile actually reserves depends on the display: Mary-O's fixed
        // 4:3 pillarboxes on a widescreen (Left/Right) and letterboxes on a
        // tall one (Top/Bottom). Honouring only the declared region meant its
        // `Top` readouts found nothing on every ordinary monitor and fell
        // through to the overlay corner — landing somewhere reasonable purely
        // by luck rather than by placement.
        let region = select_hud_region(&presentation, spec);

        let anchor = match region {
            Some((actual_region, rect)) => {
                // Two differently authored preferences may fall back to the
                // same physical region. Stack by the region actually chosen,
                // or both start at its origin and overlap.
                let stacked = offset_in_region.entry(actual_region as u8).or_insert(0.0);
                let anchor = rect.min + Vec2::splat(HUD_MARGIN) + Vec2::new(0.0, *stacked);
                *stacked += spec.font_size + SLOT_GAP;
                anchor
            }
            None => {
                let anchor = OVERLAY_ANCHOR + Vec2::new(0.0, overlay_offset);
                overlay_offset += spec.font_size + SLOT_GAP;
                anchor
            }
        };

        for (slot, mut node) in &mut slots {
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

/// Mirror the game's published readouts into the spawned text nodes.
///
/// A slot with no published readout draws an empty string rather than stale
/// text, so a game may publish conditionally without the declaration changing.
pub fn update_declared_hud(
    readouts: Res<HudReadouts>,
    mut slots: Query<(&DeclaredHudSlot, &mut Text)>,
) {
    for (slot, mut text) in &mut slots {
        let next = readouts
            .get(&slot.0)
            .map(|readout| readout.text())
            .unwrap_or_default();
        if text.0 != next {
            text.0 = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer_primitives::gameplay_presentation::{
        HudDeclaration, HudLayoutPolicy, NamedScreenRect,
    };

    #[test]
    fn same_slot_id_with_changed_style_forces_a_rebuild() {
        let old = DeclaredHudSpec(HudSlotSpec::new("score").with_font_size(18.0));
        let new = HudSlotSpec::new("score").with_font_size(30.0);
        assert!(!declaration_matches_live_specs(&[new], [Some(&old)].into_iter()));
    }

    #[test]
    fn a_live_node_without_a_cached_spec_forces_a_rebuild() {
        let spec = HudSlotSpec::new("score");
        assert!(!declaration_matches_live_specs(
            &[spec],
            [None].into_iter(),
        ));
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

    #[test]
    fn slots_falling_back_to_the_same_region_stack_instead_of_overlapping() {
        let mut presentation = ResolvedGameplayPresentation::default();
        presentation.hud = HudLayoutPolicy::PreferSurround;
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
        app.add_systems(Update, place_declared_hud);
        let top = app
            .world_mut()
            .spawn((
                DeclaredHudSlot(HudSlotId::new("top_preference")),
                Node::default(),
            ))
            .id();
        let bottom = app
            .world_mut()
            .spawn((
                DeclaredHudSlot(HudSlotId::new("bottom_preference")),
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
                    ambition_platformer_primitives::gameplay_presentation::GameplayPresentationSet,
                ),
            )
                .chain()
                .run_if(ambition_platformer_primitives::lifecycle::session_world_exists),
        );
    }
}
