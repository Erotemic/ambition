//! Feature collision-world overlay rebuilt from ECS-owned world geometry.
//!
//! The overlay is the bridge between the static ECS world (loaded
//! from LDtk) and dynamic feature state that explicitly contributes world
//! collision geometry (broken breakables, moving rebound surfaces). Engine code
//! that needs the augmented collision world calls `world_with_sandbox_solids`
//! with this resource;
//! rebuilding it once per frame keeps the augment cheap.

use ambition_platformer2d_shared_tangle::feature_overlay::{FeatureEcsWorldOverlay};
use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_combat::*;


/// The set `rebuild_feature_ecs_world_overlay` runs in.
///
/// ⭐ MOVED DOWN to `shared_tangle::schedule` (2026-09-03, the encounter seam
/// design): five ordering edges outside this crate already named it, and two
/// more crates described it in prose because they could not. Its rationale —
/// including WHY it stays a one-member set — lives with the definition now.
pub use ambition_platformer2d_shared_tangle::schedule::FeatureWorldOverlaySet;

pub fn rebuild_feature_ecs_world_overlay(
    mut overlay: ResMut<FeatureEcsWorldOverlay>,
    breakables: Query<(&FeatureName, &CenteredAabb, &BreakableFeature), With<FeatureSimEntity>>,
    // Only entities that explicitly contribute WORLD pogo geometry are lowered
    // into collision blocks. Combat bodies also publish `PogoTargetVolumes`, but
    // those are entity-side affordance geometry and must retain their identity.
    pogo_targets: Query<
        (&FeatureId, &CenteredAabb, Option<&PogoTargetVolumes>),
        (With<FeatureSimEntity>, With<PogoTargetContributor>),
    >,
) {
    overlay.blocks.clear();
    // Gate contributors (encounter / intro lock walls, gnu_ton arena gate)
    // re-extend these after we run; clearing them here gives them the same
    // clean-slate-per-frame contract the breakable blocks above have. (Portal
    // carves are owned + cleared by the portal subsystem, so not touched here.)
    overlay.gate_solids.clear();
    overlay.removed_block_names.clear();
    overlay.climbable_carves.clear();
    overlay.water_regions.clear();
    for (name, aabb, feature) in &breakables {
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
        overlay.blocks.push(ae::Block {
            id: ae::GeoId::anon(),
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
    for (id, centered, pogo) in &pogo_targets {
        let published = pogo.filter(|pogo| !pogo.volumes.is_empty());
        if let Some(pogo) = published {
            for (idx, aabb) in pogo.volumes.iter().copied().enumerate() {
                overlay.blocks.push(ae::Block {
                    id: ae::GeoId::anon(),
                    name: format!("ecs-pogo-target {} {}", id.as_str(), idx),
                    aabb,
                    kind: ae::BlockKind::PogoOrb,
                    velocity: ae::Vec2::ZERO,
                    art_color: None,
                });
            }
        } else {
            overlay.blocks.push(ae::Block {
                id: ae::GeoId::anon(),
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
