//! Gate-portal presentation: sprite visibility, animation row, and ring spin,
//! driven by the sim's `GatePortalPhases` and keyed by the zone ids in the
//! authored `GatePortalRegistry`. These render systems match props by the
//! render-local [`PropVisual::name`].

use bevy::prelude::*;

use super::actors::{draw_animator_frame, StanceSquash};
use super::primitives::{LoadingZoneVisual, PortalSprite, PropVisual};
use ambition_sprite_sheet::character::{CharacterAnim, CharacterAnimator};
use ambition_time::PresentationTime;
use ambition_platformer2d_world::rooms::{GatePortalPhase, GatePortalPhases, GatePortalRegistry};

/// Hide the debug door-zone visual that `spawn_loading_zone` spawns for a
/// LoadingZone registered as a portal. The gate sprites are the visual.
///
/// Runs each frame, so visuals re-spawned after a room reload are hidden
/// too.
pub fn hide_portal_loading_zone_visuals(
    portals: Res<GatePortalRegistry>,
    mut visuals: Query<(&LoadingZoneVisual, &mut Visibility)>,
) {
    for (visual, mut vis) in &mut visuals {
        if portals.is_portal(&visual.id) && *vis != Visibility::Hidden {
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
    mut sprites: Query<(
        &PropVisual,
        &mut Sprite,
        &mut CharacterAnimator,
        Option<&mut bevy::sprite::Anchor>,
    )>,
) {
    let dt = presentation_time.scaled_dt();
    for (zone_id, config) in portals.iter() {
        let phase = phases.phase(zone_id);
        for (prop, mut sprite, mut animator, mut anchor) in &mut sprites {
            if prop.name != config.portal_sprite_name {
                continue;
            }
            match phase {
                GatePortalPhase::Off => continue,
                GatePortalPhase::Opening { .. } => {
                    animator.request_clip(["opening"], CharacterAnim::Idle);
                }
                GatePortalPhase::On => animator.request(CharacterAnim::Walk),
                GatePortalPhase::Closing { .. } => {
                    animator.request_clip(["closing"], CharacterAnim::Run);
                }
            }
            animator.slave_clip_to(phase.sequence_progress());
            draw_animator_frame(
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

/// Spin the gate ring's inscription while the portal opens.
///
/// The `spin` row is one full turn of the inscription band, drawn from the
/// opening's progress, so the band makes exactly one turn and stops with Λ at
/// 12 o'clock on the tick the portal opens. The sprite itself does not rotate:
/// the stand clamps and the floor shadow are part of it and stay still. At all
/// other times the ring plays its slow `idle` row.
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
                Some(_) => animator.request_clip(["spin"], CharacterAnim::Walk),
                None => animator.request(CharacterAnim::Idle),
            }
            animator.slave_clip_to(opening);
            draw_animator_frame(
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

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_sprite_sheet::character::{
        try_load_spec_for_target, CharacterSpriteAsset, SheetTuning,
    };

    const ZONE: &str = "gate_zone";
    const RING: &str = "gate_ring";
    const RING_SHEET: &str = "interdimensional_gate_ring";

    fn ring_asset() -> CharacterSpriteAsset {
        CharacterSpriteAsset {
            texture: Default::default(),
            layout: Default::default(),
            spec: try_load_spec_for_target(RING_SHEET, &SheetTuning::default())
                .expect("the shipped ring sheet is baked"),
            pages: Vec::new(),
            requested_tier: Default::default(),
            resolved_tier: Default::default(),
            rigged: None,
        }
    }

    /// The ring, and the flat atlas index of each `spin` frame.
    fn app_with_a_ring() -> (App, Entity, Vec<usize>) {
        let asset = ring_asset();
        let slot = asset.spec.clip_slot(["spin"]).expect("the ring has a spin row");
        let spin: Vec<usize> = (0..12).map(|f| asset.spec.flat_index_at(slot, f)).collect();
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
        (app, ring, spin)
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

    /// One opening turns the inscription exactly once, in step with the
    /// phase, and the sprite (with its stand and shadow) never rotates.
    #[test]
    fn the_inscription_turns_once_per_opening_and_the_stand_stays_still() {
        let (mut app, ring, spin) = app_with_a_ring();
        for (progress, frame) in [(0.0, 0), (0.25, 3), (0.5, 6), (0.95, 11)] {
            step(
                &mut app,
                GatePortalPhase::Opening {
                    elapsed: progress * ambition_platformer2d_world::rooms::PORTAL_OPENING_DURATION_SECS,
                },
            );
            let (index, rotation) = drawn(&app, ring);
            assert_eq!(index, spin[frame], "opening at {progress}: spin frame {frame}");
            assert_eq!(rotation, Quat::IDENTITY, "the sprite turned at {progress}");
        }
        step(&mut app, GatePortalPhase::On);
        let (index, rotation) = drawn(&app, ring);
        assert!(!spin.contains(&index), "an open portal's ring is back on its idle row");
        assert_eq!(rotation, Quat::IDENTITY);
    }

    /// The membrane spawns on `opening` frame 0, which the packed sheet trims to
    /// a 1 px point. Once the portal is open, the quad is the size of the frame
    /// it draws, not the point it was spawned on.
    #[test]
    fn an_open_membrane_is_drawn_at_its_own_frames_size() {
        let mut asset = ring_asset();
        asset.spec = try_load_spec_for_target("interdimensional_gate_portal", &SheetTuning::default())
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
        assert!(
            drawn.x > spawned.x * 4.0,
            "the open membrane {drawn} is drawn in the spawn frame's quad {spawned}"
        );
    }
}
