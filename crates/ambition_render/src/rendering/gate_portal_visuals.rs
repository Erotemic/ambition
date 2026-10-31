//! Gate-portal presentation: sprite visibility, animation row, and ring charge,
//! driven by the sim's `GatePortalPhases` and keyed by the zone ids in the
//! authored `GatePortalRegistry`. These render systems match props by the
//! render-local [`PropVisual::name`].

use bevy::prelude::*;

use super::actors::{draw_animator_frame, StanceSquash};
use super::primitives::{LoadingZoneVisual, PortalSprite, PropVisual};
use ambition_sprite_sheet::character::{CharacterAnim, CharacterAnimator};
use ambition_time::PresentationTime;
use ambition_platformer2d_world::rooms::{GatePortalPhase, GatePortalPhases, GatePortalRegistry};

/// The loading zones a game draws no picture for, by zone id: a landing pad
/// that is a place to arrive at and not a door (the wake room of the intro:
/// the door of the hub is a one-way trip to it).
#[derive(Resource, Default, Clone, Debug)]
pub struct HiddenLoadingZones(pub std::collections::HashSet<String>);

/// Register a zone with no picture from a content plugin's `build`.
pub trait HiddenLoadingZonesAppExt {
    fn hide_loading_zone(&mut self, zone_id: impl Into<String>) -> &mut Self;
}

impl HiddenLoadingZonesAppExt for App {
    fn hide_loading_zone(&mut self, zone_id: impl Into<String>) -> &mut Self {
        self.world_mut().get_resource_or_init::<HiddenLoadingZones>().0.insert(zone_id.into());
        self
    }
}

/// Hide the debug door-zone visual that `spawn_loading_zone` spawns for a
/// LoadingZone registered as a portal. The gate sprites are the visual.
///
/// Runs each frame, so visuals re-spawned after a room reload are hidden
/// too.
pub fn hide_portal_loading_zone_visuals(
    portals: Res<GatePortalRegistry>,
    hidden: Option<Res<HiddenLoadingZones>>,
    mut visuals: Query<(&LoadingZoneVisual, &mut Visibility)>,
) {
    for (visual, mut vis) in &mut visuals {
        let no_picture = portals.is_portal(&visual.id) || hidden.as_ref().is_some_and(|hidden| hidden.0.contains(&visual.id));
        if no_picture && *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
}

/// Hide the portal sprite while its phase is `Off`; show it
/// otherwise. Matches the prop entity by [`PropVisual::name`] against
/// `GatePortalConfig::portal_sprite_name`.
pub fn sync_portal_sprite_visibility(
    mut commands: Commands,
    portals: Res<GatePortalRegistry>,
    phases: Res<GatePortalPhases>,
    mut sprites: Query<(Entity, &PropVisual, &mut Visibility, Option<&PortalSprite>)>,
) {
    for (zone_id, config) in portals.iter() {
        let target_visibility = if phases.phase(zone_id).portal_sprite_visible() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        for (entity, prop, mut vis, marker) in &mut sprites {
            if prop.name != config.portal_sprite_name {
                continue;
            }
            if marker.is_none() {
                // `try_insert`: these are room-scoped prop visuals, and room
                // teardown can despawn them before this deferred write lands.
                // Covered by `deferred_write_safety::production_passes`.
                commands.entity(entity).try_insert(PortalSprite);
            }
            if *vis != target_visibility {
                *vis = target_visibility;
            }
        }
    }
}

/// Drive gate-portal animation from its phase.
///
/// `opening` and `closing` are drawn from the phase's own progress
/// ([`GatePortalPhase::sequence_progress`]), so the membrane is full size on
/// the tick traversal opens and gone on the tick it closes. `stable` loops on
/// the sheet clock. `PortalSprite` excludes these entities from generic
/// character/prop animation, so this system exclusively owns their animator and
/// atlas frame.
pub fn sync_portal_sprite_animation(
    presentation_time: PresentationTime,
    portals: Res<GatePortalRegistry>,
    phases: Res<GatePortalPhases>,
    // A body going in or coming out ripples the surface (`swallow`, `emit`).
    passages: Option<Res<ambition_sim_view::PassageViews>>,
    mut sprites: Query<(
        Entity,
        &PropVisual,
        &mut Sprite,
        &mut CharacterAnimator,
        Option<&mut bevy::sprite::Anchor>,
    )>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    let dt = presentation_time.scaled_dt();
    for (zone_id, config) in portals.iter() {
        let phase = phases.phase(zone_id);
        for (entity, prop, mut sprite, mut animator, mut anchor) in &mut sprites {
            if prop.name != config.portal_sprite_name {
                continue;
            }
            // The ripple of the gate in this sprite's own room.
            let ripple = passages.as_ref().and_then(|passages| passages.gate_ripple(rooms.of(entity), zone_id));
            match (phase, ripple) {
                (GatePortalPhase::Off, _) => continue,
                (GatePortalPhase::Opening { .. }, _) => {
                    animator.request_clip(["opening"], CharacterAnim::Idle);
                }
                (GatePortalPhase::On, Some((row, _))) => animator.request_clip([row], CharacterAnim::Walk),
                (GatePortalPhase::On, None) => animator.request(CharacterAnim::Walk),
                (GatePortalPhase::Closing { .. }, _) => {
                    animator.request_clip(["closing"], CharacterAnim::Run);
                }
            }
            match (phase, ripple) {
                (GatePortalPhase::On, Some((_, progress))) => animator.slave_clip_to(Some(progress)),
                _ => animator.slave_clip_to(phase.sequence_progress()),
            }
            // `&mut` through the `Mut` marks the components already.
            let _ = draw_animator_frame(
                &mut sprite,
                &mut animator,
                anchor.as_deref_mut(),
                dt,
                false,
                StanceSquash::NONE,
            );
        }
    }
}

/// Charge the gate ring while the portal opens.
///
/// The `charge` row runs a pulse of light round the plaques, first to last,
/// drawn from the opening's progress, so it reaches the last plaque on the
/// tick the portal opens. The sprite itself does not rotate: the ring stands
/// in its base. At all other times the ring plays its slow `idle` row.
pub fn sync_portal_ring_animation(
    mut commands: Commands,
    presentation_time: PresentationTime,
    portals: Res<GatePortalRegistry>,
    phases: Res<GatePortalPhases>,
    mut rings: Query<(
        Entity,
        &PropVisual,
        &mut Sprite,
        &mut CharacterAnimator,
        Option<&mut bevy::sprite::Anchor>,
        Option<&PortalSprite>,
    )>,
) {
    let dt = presentation_time.scaled_dt();
    for (zone_id, config) in portals.iter() {
        let phase = phases.phase(zone_id);
        let opening = match phase {
            GatePortalPhase::Opening { .. } => phase.sequence_progress(),
            _ => None,
        };
        for (entity, prop, mut sprite, mut animator, mut anchor, marker) in &mut rings {
            if prop.name != config.ring_sprite_name {
                continue;
            }
            if marker.is_none() {
                // `try_insert`: these are room-scoped prop visuals, and room
                // teardown can despawn them before this deferred write lands.
                // Covered by `deferred_write_safety::production_passes`.
                commands.entity(entity).try_insert(PortalSprite);
            }
            match opening {
                Some(_) => animator.request_clip(["charge"], CharacterAnim::Walk),
                None => animator.request(CharacterAnim::Idle),
            }
            animator.slave_clip_to(opening);
            // `&mut` through the `Mut` marks the components already.
            let _ = draw_animator_frame(
                &mut sprite,
                &mut animator,
                anchor.as_deref_mut(),
                dt,
                false,
                StanceSquash::NONE,
            );
        }
    }
}

/// The prop kind of the round ring's lit glyphs (`interdimensional_gate_glyphs`):
/// its `lit` row has one frame per chevron, that chevron's glyph ablaze where
/// the ring draws it, and its `bit` and `used` rows one frame per cell of each
/// chevron's position grid (`chevron * 16 + row * 4 + column`), all on the
/// ring's own canvas.
pub const GATE_GLYPHS_PROP_KIND: &str = "gate_glyphs";

/// The prop kind of the lit glyphs of a ring of prop kind `ring_kind`: its
/// kind with the `ring` at its end made `glyphs` (`gate_ring` lights
/// [`GATE_GLYPHS_PROP_KIND`], `triangle_gate_ring` lights
/// `triangle_gate_glyphs`), so each shape of ring lights its own chevrons.
/// `None` for a kind that is no ring.
pub fn ring_glyphs_kind(ring_kind: &str) -> Option<String> {
    ring_kind.strip_suffix("ring").map(|stem| format!("{stem}glyphs"))
}

/// The chevrons a terminal's user has entered and not yet dialed, by the
/// ring whose terminal is open. Presentation state a terminal writes: the
/// ring lights them as they are entered.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct GateDialPreview {
    pub ring: Option<String>,
    pub chevrons: Vec<ambition_platformer2d_world::rooms::Chevron>,
}

/// What lights on a ring's plaque: its glyph, or one cell of its position
/// grid, a set bit or a used row's clear bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GateRingMark {
    Glyph(ambition_platformer2d_world::rooms::Chevron),
    Bit { chevron: ambition_platformer2d_world::rooms::Chevron, cell: u8 },
    Used { chevron: ambition_platformer2d_world::rooms::Chevron, cell: u8 },
}

impl GateRingMark {
    /// The row of the `gate_glyphs` sheet, and the middle of the mark's
    /// frame as a fraction of the row, for a sheet of `chevrons` glyphs.
    fn frame(self, chevrons: usize) -> (&'static str, f32) {
        let cells = (ambition_platformer2d_world::rooms::MAX_CHEVRON_USES * ambition_platformer2d_world::rooms::GRID_BITS) as f32;
        let (row, index, count) = match self {
            Self::Glyph(chevron) => ("lit", f32::from(chevron), chevrons as f32),
            Self::Bit { chevron, cell } => ("bit", f32::from(chevron) * cells + f32::from(cell), chevrons as f32 * cells),
            Self::Used { chevron, cell } => ("used", f32::from(chevron) * cells + f32::from(cell), chevrons as f32 * cells),
        };
        (row, (index + 0.5) / count)
    }
}

/// One lit mark over a ring: a child of the ring's sprite.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateRingLitGlyph(pub GateRingMark);

/// The address a ring shows lit: the one its terminal's user is entering,
/// else the one of the link it sends, else none.
pub fn ring_lit_address(
    zone: &str,
    preview: Option<&GateDialPreview>,
    network: Option<&ambition_platformer2d_world::rooms::GateNetworkState>,
) -> Vec<ambition_platformer2d_world::rooms::Chevron> {
    let entering = preview.filter(|preview| preview.ring.as_deref() == Some(zone)).map(|preview| &preview.chevrons);
    let sending = network.and_then(|network| network.route_from(zone)).map(|link| &link.address);
    entering.filter(|chevrons| !chevrons.is_empty()).or(sending).cloned().unwrap_or_default()
}

/// The marks `address` lights: each chevron it uses once, and each cell of
/// that chevron's used rows (`position_grid`), so the lit ring spells the
/// whole address, its order too.
pub fn ring_lit_marks(address: &[ambition_platformer2d_world::rooms::Chevron]) -> Vec<GateRingMark> {
    use ambition_platformer2d_world::rooms::{position_grid, GRID_BITS, MAX_CHEVRON_USES};
    let mut chevrons = address.to_vec();
    chevrons.sort_unstable();
    chevrons.dedup();
    let mut marks = Vec::new();
    for chevron in chevrons {
        marks.push(GateRingMark::Glyph(chevron));
        let uses = address.iter().filter(|used| **used == chevron).count().min(MAX_CHEVRON_USES);
        for (row, bits) in position_grid(address, chevron).iter().enumerate().take(uses) {
            for (column, set) in bits.iter().enumerate() {
                let cell = (row * GRID_BITS + column) as u8;
                marks.push(if *set { GateRingMark::Bit { chevron, cell } } else { GateRingMark::Used { chevron, cell } });
            }
        }
    }
    marks
}

/// Light the dialed address on each ring: one child sprite per lit mark,
/// drawing that mark's frame of the `gate_glyphs` sheet over the ring.
#[allow(clippy::too_many_arguments)]
pub fn sync_ring_lit_glyphs(
    mut commands: Commands,
    portals: Res<GatePortalRegistry>,
    preview: Option<Res<GateDialPreview>>,
    network: Option<Res<ambition_platformer2d_world::rooms::GateNetworkState>>,
    catalog: Option<Res<ambition_platformer2d_world::rooms::GateNetworkCatalog>>,
    assets: Option<Res<ambition_sprite_sheet::game_assets::GameAssets>>,
    rings: Query<(Entity, &PropVisual, Option<&Children>)>,
    mut glyphs: Query<(&GateRingLitGlyph, &mut Sprite, &mut CharacterAnimator, &mut bevy::sprite::Anchor)>,
) {
    let Some(assets) = assets.as_deref() else {
        return;
    };
    // The rows have frames per chevron, in the inscription's order.
    let Some(count) = catalog.as_deref().map(|catalog| catalog.chevrons()).filter(|count| *count > 0) else {
        return;
    };
    for (zone_id, config) in portals.iter() {
        // A travel portal is no chevron ring: it lights no address (its own
        // frame sheet's `charge` row is its opening).
        let portal = catalog.as_deref().and_then(|catalog| catalog.ring(zone_id)).is_some_and(|ring| {
            ring.kind == ambition_platformer2d_world::rooms::RingKind::Portal
        });
        let lit = if portal {
            Vec::new()
        } else {
            ring_lit_marks(&ring_lit_address(zone_id, preview.as_deref(), network.as_deref()))
        };
        for (ring, prop, children) in &rings {
            if prop.name != config.ring_sprite_name {
                continue;
            }
            // The glyph sheet of this ring's own shape.
            let Some(asset) = ring_glyphs_kind(&prop.kind)
                .and_then(|kind| assets.characters.prop_asset_for_kind(&kind))
                .filter(|asset| asset.spec.clip_slot(["lit"]).is_some())
            else {
                continue;
            };
            let mut lit = lit.clone();
            // A sheet without the grid rows still lights its glyphs.
            if asset.spec.clip_slot(["bit"]).is_none() || asset.spec.clip_slot(["used"]).is_none() {
                lit.retain(|mark| matches!(mark, GateRingMark::Glyph(_)));
            }
            let mut shown = Vec::new();
            for child in children.into_iter().flatten() {
                let Ok((glyph, mut sprite, mut animator, mut anchor)) = glyphs.get_mut(*child) else {
                    continue;
                };
                if !lit.contains(&glyph.0) {
                    commands.entity(*child).try_despawn();
                    continue;
                }
                shown.push(glyph.0);
                let (row, at) = glyph.0.frame(count);
                animator.request_clip([row], CharacterAnim::Idle);
                animator.slave_clip_to(Some(at));
                let _ = draw_animator_frame(&mut sprite, &mut animator, Some(&mut anchor), 0.0, false, StanceSquash::NONE);
            }
            for mark in lit.iter().filter(|mark| !shown.contains(mark)) {
                let (sprite, anchor, animator) =
                    super::world::prop_sprite_bundle(prop.draw, prop.flip_y, asset, prop.size);
                let glyph = commands
                    .spawn((
                        sprite,
                        anchor,
                        animator,
                        // Over the ring, by less than the membrane's step.
                        Transform::from_xyz(0.0, 0.0, 0.002),
                        GateRingLitGlyph(*mark),
                        PortalSprite,
                        Name::new(format!("Gate lit {mark:?}")),
                    ))
                    .id();
                commands.entity(ring).add_child(glyph);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_sprite_sheet::character::{
        try_load_spec_for_target, CharacterSpriteAsset, SheetTuning,
    };

    const ZONE: &str = "gate_zone";
    const RING: &str = "gate_ring";
    const RING_SHEET: &str = "interdimensional_gate_ring";

    #[test]
    fn each_shape_of_ring_lights_its_own_glyph_sheet() {
        assert_eq!(ring_glyphs_kind("gate_ring").as_deref(), Some(GATE_GLYPHS_PROP_KIND));
        assert_eq!(ring_glyphs_kind("triangle_gate_ring").as_deref(), Some("triangle_gate_glyphs"));
        assert_eq!(ring_glyphs_kind("gate_portal"), None);
    }

    fn ring_asset(sprite_content: &ambition_sprite_sheet::SpriteContent) -> CharacterSpriteAsset {
        CharacterSpriteAsset {
            texture: Default::default(),
            layout: Default::default(),
            spec: try_load_spec_for_target(&sprite_content, RING_SHEET, &SheetTuning::default())
                .expect("the shipped ring sheet is baked"),
            pages: Vec::new(),
            requested_tier: Default::default(),
            resolved_tier: Default::default(),
            rigged: None,
        }
    }

    /// The ring, and the flat atlas index of each `charge` frame.
    fn app_with_a_ring(sprite_content: &ambition_sprite_sheet::SpriteContent) -> (App, Entity, Vec<usize>) {
        let asset = ring_asset(&sprite_content);
        let slot = asset.spec.clip_slot(["charge"]).expect("the ring has a charge row");
        let charge: Vec<usize> = (0..30).map(|f| asset.spec.flat_index_at(slot, f)).collect();
        let mut app = App::new();
        app.init_resource::<Time>();
        app.init_resource::<ambition_time::ClockState>();
        let mut portals = GatePortalRegistry::default();
        portals
            .try_register(ZONE, "gate_switch", "gate_portal", RING)
            .expect("one portal registers");
        app.insert_resource(portals);
        app.init_resource::<GatePortalPhases>();
        app.add_systems(Update, sync_portal_ring_animation);
        let ring = app
            .world_mut()
            .spawn((
                PropVisual {
                    id: "ring".into(),
                    kind: RING_SHEET.into(),
                    name: RING.into(),
                    size: Vec2::splat(192.0),
                    draw: Default::default(),
                    flip_y: false,
                },
                Transform::default(),
                Sprite {
                    texture_atlas: Some(TextureAtlas::default()),
                    ..default()
                },
                CharacterAnimator::new(&asset),
            ))
            .id();
        (app, ring, charge)
    }

    fn step(app: &mut App, phase: GatePortalPhase) {
        *app.world_mut()
            .resource_mut::<GatePortalPhases>()
            .phase_mut(ZONE) = phase;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(16));
        app.update();
    }

    fn drawn(app: &App, ring: Entity) -> (usize, Quat) {
        let world = app.world();
        let index = world
            .get::<Sprite>(ring)
            .and_then(|sprite| sprite.texture_atlas.as_ref())
            .map_or(usize::MAX, |atlas| atlas.index);
        (index, world.get::<Transform>(ring).unwrap().rotation)
    }

    /// One opening runs the charge exactly once, in step with the phase,
    /// and the sprite (the ring in its base) never rotates.
    #[test]
    fn the_ring_charges_once_per_opening_and_never_turns() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
        let (mut app, ring, charge) = app_with_a_ring(&sprite_content);
        for (progress, frame) in [(0.0, 0), (0.25, 7), (0.5, 15), (0.95, 28)] {
            step(
                &mut app,
                GatePortalPhase::Opening {
                    elapsed: progress * ambition_platformer2d_world::rooms::PORTAL_OPENING_DURATION_SECS,
                },
            );
            let (index, rotation) = drawn(&app, ring);
            assert_eq!(index, charge[frame], "opening at {progress}: charge frame {frame}");
            assert_eq!(rotation, Quat::IDENTITY, "the sprite turned at {progress}");
        }
        step(&mut app, GatePortalPhase::On);
        let (index, rotation) = drawn(&app, ring);
        assert!(!charge.contains(&index), "an open portal's ring is back on its idle row");
        assert_eq!(rotation, Quat::IDENTITY);
    }

    /// The membrane spawns on `opening` frame 0, which the packed sheet trims to
    /// a 1 px point. Once the portal is open, the quad is the size of the frame
    /// it draws, not the point it was spawned on.
    #[test]
    fn an_open_membrane_is_drawn_at_its_own_frames_size() {

    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
        let mut asset = ring_asset(&sprite_content);
        asset.spec = try_load_spec_for_target(&sprite_content, "interdimensional_gate_portal", &SheetTuning::default())
            .expect("the shipped portal sheet is baked");
        let collision = Vec2::splat(96.0);
        let (sprite, anchor, animator) = crate::rendering::world::prop_sprite_bundle(
            Default::default(),
            false,
            &asset,
            collision,
        );
        let spawned = sprite.custom_size.expect("a packed sheet sizes its quad");
        let mut app = App::new();
        app.init_resource::<Time>();
        app.init_resource::<ambition_time::ClockState>();
        let mut portals = GatePortalRegistry::default();
        portals
            .try_register(ZONE, "gate_switch", "gate_portal", RING)
            .expect("one portal registers");
        app.insert_resource(portals);
        app.init_resource::<GatePortalPhases>();
        app.add_systems(Update, sync_portal_sprite_animation);
        let membrane = app
            .world_mut()
            .spawn((
                PropVisual {
                    id: "membrane".into(),
                    kind: "interdimensional_gate_portal".into(),
                    name: "gate_portal".into(),
                    size: collision,
                    draw: Default::default(),
                    flip_y: false,
                },
                Transform::default(),
                sprite,
                anchor,
                animator,
            ))
            .id();
        step(&mut app, GatePortalPhase::On);
        let world = app.world();
        let drawn = world.get::<Sprite>(membrane).unwrap().custom_size.unwrap();
        let (frame_size, _) = world
            .get::<CharacterAnimator>(membrane)
            .unwrap()
            .current_render()
            .expect("the portal sheet is trimmed");
        assert_eq!(drawn, frame_size, "the quad follows the drawn frame");
        // The spawn frame (the opening's first) trims to its own ink, so the
        // open membrane's quad is not the one it spawned with.
        assert_ne!(drawn, spawned, "the open membrane is drawn in the spawn frame's quad {spawned}");
    }

    /// An address lights each chevron's glyph once and a row of its grid
    /// per use: the place's bits bright, the rest of the row dim.
    #[test]
    fn a_lit_address_spells_its_order_in_the_grids() {
        // Chevron 3 at places 0 and 5 (0b0101), chevron 1 at place 1 (0b0001).
        let marks = ring_lit_marks(&[3, 1, 9, 9, 9, 3]);
        let of = |chevron| marks.iter().filter(|mark| match mark {
            GateRingMark::Glyph(c) => *c == chevron,
            GateRingMark::Bit { chevron: c, .. } | GateRingMark::Used { chevron: c, .. } => *c == chevron,
        }).copied().collect::<Vec<_>>();
        let three = of(3);
        assert_eq!(three.len(), 1 + 2 * 4, "{three:?}");
        assert!(three.contains(&GateRingMark::Glyph(3)));
        for cell in [0, 1, 2, 3, 4, 6] {
            assert!(three.contains(&GateRingMark::Used { chevron: 3, cell }), "{cell}");
        }
        for cell in [5, 7] {
            assert!(three.contains(&GateRingMark::Bit { chevron: 3, cell }), "{cell}");
        }
        assert!(of(1).contains(&GateRingMark::Bit { chevron: 1, cell: 3 }));
        assert_eq!(of(9).len(), 1 + 3 * 4);
        assert!(of(0).is_empty());
    }
}
