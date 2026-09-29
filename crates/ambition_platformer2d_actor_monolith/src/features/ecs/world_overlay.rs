//! Feature collision-world overlay rebuilt from ECS-owned world geometry.
//!
//! The overlay is the bridge between the static ECS world (loaded
//! from LDtk) and dynamic feature state that explicitly contributes world
//! collision geometry (broken breakables, moving rebound surfaces). Engine code
//! that needs the augmented collision world calls `world_with_sandbox_solids`
//! with this resource;
//! rebuilding it once per frame keeps the augment cheap.

use ambition_platformer2d_shared_tangle::feature_overlay::RoomOverlays;
use ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance;
use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_combat::*;



pub fn rebuild_feature_ecs_world_overlay(
    // Every live room's overlay; each contribution goes to the room of the
    // entity that makes it.
    mut overlays: RoomOverlays,
    // ⛔⛔ `FeatureId` IS IN THIS QUERY BECAUSE A NAME IS NOT AN IDENTITY. The
    // published block used to carry `GeoId::anon()` and put the breakable's
    // DISPLAY name in `Block.name` — and `FeatureName` documents itself as
    // "human-facing authored name for debug overlays / inspectors". The durable
    // id was one component away on the same entity the whole time
    // (`spawn_breakable_into` inserts `FeatureId::new(authored.id)`).
    breakables: Query<
        (
            &FeatureId,
            &FeatureName,
            &CenteredAabb,
            &BreakableFeature,
            Option<&InRoomInstance>,
        ),
        With<FeatureSimEntity>,
    >,
    // Only entities that explicitly contribute WORLD pogo geometry are lowered
    // into collision blocks. Combat bodies also publish `PogoTargetVolumes`, but
    // those are entity-side affordance geometry and must retain their identity.
    pogo_targets: Query<
        (
            &FeatureId,
            &CenteredAabb,
            Option<&PogoTargetVolumes>,
            Option<&InRoomInstance>,
        ),
        (With<FeatureSimEntity>, With<PogoTargetContributor>),
    >,
) {
    // Gate contributors (encounter / intro lock walls, gnu_ton arena gate)
    // re-extend these after we run; clearing them here gives them the same
    // clean-slate-per-frame contract the breakable blocks below have. (Portal
    // carves are owned + cleared by the portal subsystem, so not touched here.)
    //
    // ⭐ The five clears this replaces were a HAND-KEPT LIST. The method
    // destructures the overlay with no `..`, so a seventh field cannot be added
    // without its author saying which owner clears it.
    for mut overlay in overlays.each() {
        overlay.clear_engine_contributions();
    }
    for (id, name, aabb, feature, room) in &breakables {
        if feature.broken() {
            continue;
        }
        if feature.breakable.pogo_refresh {
            continue;
        }
        let kind = match feature.breakable.collision {
            ambition_interaction::BreakableCollision::None => continue,
            ambition_interaction::BreakableCollision::Solid => ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Hard,
            },
            ambition_interaction::BreakableCollision::OneWayUp => ae::BlockKind::OneWay,
        };
        let Some(mut overlay) = overlays.for_room(room) else {
            continue;
        };
        overlay.blocks.push(ae::Block {
            // ⭐ The OWNING OCCURRENCE, which is what the projectile contact
            // protocol asks a contributed object collider to carry: "tile terrain
            // uses a stable collider/geometry identity; a contributed object
            // collider additionally identifies its owning occurrence."
            // ⛔ `GeoId::anon()` was wrong twice over — it is the source reserved
            // for fixtures ("the authoring pipeline NEVER emits this"), and it
            // left `Block.name` as the only thing telling two breakables apart.
            id: ae::GeoId::placement(ae::PlacementId::new(id.as_str()), 0),
            name: format!("ecs-breakable {}", name.0.as_str()),
            aabb: aabb.aabb(),
            kind,
            velocity: ae::Vec2::ZERO,
            art_color: None,
        });
    }

    // Explicit ECS WORLD-pogo bridge. `PogoTargetContributor` says this entity
    // contributes collision-world rebound geometry. A non-empty published
    // `PogoTargetVolumes` is authoritative; otherwise the contributor's own
    // centered envelope is the deliberate world-surface fallback. Ordinary
    // bodies intentionally lack the contributor and stay entity contacts, so
    // their identity is never flattened into anonymous blocks.
    for (id, centered, pogo, room) in &pogo_targets {
        let Some(mut overlay) = overlays.for_room(room) else {
            continue;
        };
        let published = pogo.filter(|pogo| !pogo.volumes.is_empty());
        if let Some(pogo) = published {
            for (idx, aabb) in pogo.volumes.iter().copied().enumerate() {
                overlay.blocks.push(ae::Block {
                    // Same occurrence identity, with the volume ordinal as the
                    // `GeoId` index — one placement legitimately emits several.
                    id: ae::GeoId::placement(
                        ae::PlacementId::new(id.as_str()),
                        u16::try_from(idx).unwrap_or(u16::MAX),
                    ),
                    name: format!("ecs-pogo-target {} {}", id.as_str(), idx),
                    aabb,
                    kind: ae::BlockKind::PogoOrb,
                    velocity: ae::Vec2::ZERO,
                    art_color: None,
                });
            }
        } else {
            overlay.blocks.push(ae::Block {
                id: ae::GeoId::placement(ae::PlacementId::new(id.as_str()), 0),
                name: format!("ecs-pogo-target-fallback {}", id.as_str()),
                aabb: centered.aabb(),
                kind: ae::BlockKind::PogoOrb,
                velocity: ae::Vec2::ZERO,
                art_color: None,
            });
        }
    }
}

/// The one line that makes five external ordering edges mean anything.
///
/// ⛔ `FeatureWorldOverlaySet` is `shared_tangle` vocabulary now, and three
/// `ambition_content` plugins plus the Mary-O and Sanic demos order `.after()`
/// it. All five of those edges are satisfied by a SINGLE
/// `.in_set(FeatureWorldOverlaySet)` on `rebuild_feature_ecs_world_overlay` in
/// `WorldPrepSchedulePlugin`. Delete that one call and every consumer keeps
/// compiling, keeps its `.after(..)`, and silently waits for an empty set —
/// which is the D33 defect shape exactly: the ordering is gone and nothing is
/// red.
///
/// ⭐ SO THIS ASSERTS MEMBERSHIP, NOT EXISTENCE. A test that checked the system
/// is merely scheduled would stay green through that deletion.
#[cfg(test)]
mod overlay_set_membership {
    use bevy::ecs::schedule::{NodeId, Schedules, SystemSet as _};
    use bevy::prelude::App;

    use ambition_platformer2d_shared_tangle::schedule::FeatureWorldOverlaySet;
    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;

    #[test]
    fn the_overlay_rebuild_is_a_member_of_the_set_its_consumers_order_against() {
        let mut app = App::new();
        app.add_plugins(crate::features::WorldPrepSchedulePlugin);
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        let schedule = schedules
            .get(sim)
            .expect("WorldPrepSchedulePlugin must have created the sim schedule");
        let graph = schedule.graph();

        let set_key = graph
            .system_sets
            .get_key(FeatureWorldOverlaySet.intern())
            .expect(
                "FeatureWorldOverlaySet must be a registered SystemSet — five \
                 ordering edges outside this crate name it",
            );
        let system_key = {
            let mut found = None;
            for (key, system, _) in graph.systems.iter() {
                let name = format!("{}", system.name());
                if name.rsplit("::").next() == Some("rebuild_feature_ecs_world_overlay") {
                    assert!(found.is_none(), "the leaf name must resolve to one system");
                    found = Some(key);
                }
            }
            found.expect("rebuild_feature_ecs_world_overlay must be scheduled")
        };

        assert!(
            graph
                .hierarchy()
                .graph()
                .contains_edge(NodeId::Set(set_key), NodeId::System(system_key)),
            "rebuild_feature_ecs_world_overlay must be a MEMBER of \
             FeatureWorldOverlaySet. Without that membership the set is empty, \
             and three ambition_content plugins plus two demos order .after() \
             nothing — compiling, green, and unordered."
        );
    }
}

#[cfg(test)]
mod breakable_geometry_agreement {
    use ambition_combat::components::{
        BreakableFeature, CenteredAabb, DamageableVolumes, FeatureId, FeatureName,
    };
    use ambition_combat::FeatureSimEntity;
    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay;
    use bevy::prelude::*;

    /// ⭐⭐ A5 ACCEPTANCE, "melee/projectile geometry agreement": THE HURT VOLUME AND
    /// THE CONTRIBUTED SURFACE MUST DESCRIBE THE SAME RECTANGLE.
    ///
    /// A solid breakable is published twice, by two systems in two modules that
    /// never reference each other: `refresh_breakable_damageable_volumes`
    /// (`features/ecs/target_volumes.rs`) writes the volume a hit is tested
    /// against, and `rebuild_feature_ecs_world_overlay` above writes the block a
    /// projectile and a body collide with. They agree today because both read the
    /// same `CenteredAabb` — which is agreement by construction and therefore
    /// exactly the kind that is true by accident until somebody offsets one.
    ///
    /// ⛔ THE FAILURE IS SILENT AND ASYMMETRIC. A surface wider than the hurt
    /// volume is a crate that stops a shot in a band where it cannot be damaged;
    /// narrower, and a shot damages it through its own corner. Neither raises
    /// anything — both roads are individually correct.
    ///
    /// ⚠ IT PINS WHERE, NOT WHETHER. The two systems deliberately disagree about
    /// ELIGIBILITY — the volume needs `trigger.allows_hit() || pogo_refresh`, the
    /// block needs `collision != None && !pogo_refresh` — so an `OnStand` solid
    /// crate is a surface with no hurt volume ON PURPOSE. That divergence is
    /// documented at both sites; this test asserts only that when both publish,
    /// they publish the same rectangle.
    #[test]
    fn a_breakables_hurt_volume_and_its_contributed_surface_are_the_same_rectangle() {
        let mut app = App::new();
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(),
            FeatureEcsWorldOverlay::default(),
        );

        // Deliberately not centred on the origin and not square: a zero-centred
        // unit box makes an offset bug and a correct build agree.
        let centre = ae::Vec2::new(137.0, -64.5);
        let half = ae::Vec2::new(19.0, 7.5);
        let mut breakable = ambition_interaction::Breakable::new("crate", 3);
        breakable.collision = ambition_interaction::BreakableCollision::Solid;
        breakable.trigger = ambition_interaction::BreakableTrigger::OnHit;

        app.world_mut().spawn((
            FeatureSimEntity,
            FeatureId::new("crate_17"),
            FeatureName("A Crate".to_string()),
            CenteredAabb {
                center: centre,
                half_size: half,
            },
            BreakableFeature::new(breakable),
            DamageableVolumes::default(),
        ));

        app.add_systems(
            Update,
            (
                crate::features::ecs::refresh_breakable_damageable_volumes,
                super::rebuild_feature_ecs_world_overlay,
            ),
        );
        app.update();

        let blocks: Vec<ae::Aabb> = ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<
            FeatureEcsWorldOverlay,
        >(app.world())
        .expect("the live room has an overlay")
        .blocks
            .iter()
            .filter(|b| b.name.starts_with("ecs-breakable"))
            .map(|b| b.aabb)
            .collect();
        let mut volumes = app.world_mut().query::<&DamageableVolumes>();
        let published: Vec<ae::Aabb> = volumes
            .iter(app.world())
            .flat_map(|v| v.volumes.iter().map(|volume| volume.bounds()))
            .collect();

        // ⛔ ANTI-VACUITY, AND IT IS THE WHOLE TEST. "Both empty" satisfies any
        // equality check, and both roads are one predicate away from publishing
        // nothing — `collision: None` silences the block, `OnStand` silences the
        // volume, and `Breakable::new` defaults `collision` to `None`, which is
        // why every other projectile-vs-breakable fixture in this repository
        // uses a crate that contributes no surface at all.
        assert_eq!(
            blocks.len(),
            1,
            "the solid breakable contributed no surface, so this compares nothing"
        );
        assert_eq!(
            published.len(),
            1,
            "the hittable breakable published no damageable volume, so this \
             compares nothing"
        );

        assert_eq!(
            blocks[0], published[0],
            "a breakable's contributed collision surface and its damageable \
             volume describe different rectangles: a shot would stop where it \
             cannot damage, or damage where it does not stop"
        );
    }
}

#[cfg(test)]
mod room_keyed_contributions {
    use ambition_combat::components::{BreakableFeature, CenteredAabb, FeatureId, FeatureName};
    use ambition_combat::FeatureSimEntity;
    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay;
    use ambition_platformer2d_shared_tangle::lifecycle::{
        activation_room_root, InRoomInstance, LiveRoomInstance, RoomInstanceRoot, SessionScopeId,
    };
    use bevy::prelude::*;

    fn a_solid_crate_in(app: &mut App, id: &str, room: LiveRoomInstance) {
        let mut breakable = ambition_interaction::Breakable::new(id, 3);
        breakable.collision = ambition_interaction::BreakableCollision::Solid;
        app.world_mut().spawn((
            FeatureSimEntity,
            FeatureId::new(id),
            FeatureName(id.to_string()),
            CenteredAabb {
                center: ae::Vec2::new(40.0, 40.0),
                half_size: ae::Vec2::new(8.0, 8.0),
            },
            BreakableFeature::new(breakable),
            InRoomInstance(room),
        ));
    }

    /// OW1 cut 3c: a contribution goes to the overlay of its own live room.
    ///
    /// Two live rooms, #0 and #7, each with one solid crate. The control is
    /// #0's crate in #0's overlay, as it was when the overlay was one
    /// resource. The subject is that #7's crate is in #7's overlay and NOT in
    /// #0's, so a body in #0 does not collide with a wall of #7.
    #[test]
    fn a_breakable_contributes_to_its_own_live_rooms_overlay() {
        let mut app = App::new();
        let here = LiveRoomInstance::ACTIVATION;
        let elsewhere = (0..7).fold(here, |room, _| room.next());
        app.world_mut().spawn(activation_room_root(SessionScopeId(0)));
        app.world_mut()
            .spawn(activation_room_root(SessionScopeId(0)))
            .insert(elsewhere);
        a_solid_crate_in(&mut app, "crate_here", here);
        a_solid_crate_in(&mut app, "crate_elsewhere", elsewhere);
        app.add_systems(Update, super::rebuild_feature_ecs_world_overlay);
        app.update();

        let world = app.world_mut();
        let mut rooms: Vec<(String, Vec<String>)> = world
            .query_filtered::<(&LiveRoomInstance, &FeatureEcsWorldOverlay), With<RoomInstanceRoot>>()
            .iter(world)
            .map(|(live, overlay)| {
                (
                    live.to_string(),
                    overlay.blocks.iter().map(|block| block.name.clone()).collect(),
                )
            })
            .collect();
        rooms.sort();
        assert_eq!(
            rooms,
            vec![
                (here.to_string(), vec!["ecs-breakable crate_here".to_string()]),
                (elsewhere.to_string(), vec!["ecs-breakable crate_elsewhere".to_string()]),
            ],
            "a contribution did not go to its own live room's overlay"
        );
    }
}
