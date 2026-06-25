//! Composite mount/rider spawn helpers.
//!
//! The public spawn facade keeps room orchestration in `spawn.rs`; this
//! module owns the fan-out from a composite authored enemy into separate
//! mount and rider ECS actors.

use super::super::enemies::{spec_for_brain, EnemyArchetypeSpec};
use super::brain_builders::{
    enemy_combat_kit_for_spec, enemy_default_action_set, held_item_for_spec,
    mounted_rider_brain_and_action_set, skirmisher_brain_for_enemy,
};
use super::spawn_actors::EnemyActorSpawnPlan;
use super::*;

/// Fan a composite "X on Shark" spawn into a mount entity + a rider
/// entity. Both are spawned at the authored position; the per-tick
/// [`super::sync_riders_to_mounts`] system snaps the rider to the
/// mount's offset from frame one.
///
/// Mount: `BurningFlyingShark` archetype with an explicit orbiting
/// Skirmisher-style mount brain so the fused shark+pirate encounter
/// keeps its aerial height changes and spreads out instead of
/// clumping. Health comes from the composite spec (PirateOnShark = 6,
/// PirateHeavyOnShark = 7) so the body HP pool stays as authored. The
/// riderless shark now has its own dedicated Shark brain via
/// `enemy_default_brain`.
///
/// Rider: `PirateRaider` for the light composite, `PirateHeavy` for
/// the heavy composite. Brain is explicitly built as Skirmisher and the
/// composite/rider held item carries the ranged weapon capability. When the
/// shark dies, mount dissolution keeps the held item and derives the new
/// dismounted action set from it, so a rider that still has a gun-sword can
/// keep firing on foot. Aggressiveness is forced ON regardless of the rider
/// archetype's `attacks_player()` default so a dismounted PirateHeavy (which
/// is normally peaceful Cove crew) keeps fighting after the shark is killed
/// under her.
pub(super) fn spawn_composite_mount_rider(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<ambition_characters::actor::EnemyBrain>,
    paths: &[(String, ambition_characters::actor::KinematicPath)],
    composite_spec: &EnemyArchetypeSpec,
) {
    // The whole fan-out is driven by the composite's authored
    // `composite_visual` row (mount/rider brain keys + names): no named
    // roster branch decides the mount or rider here.
    let cv = composite_spec
        .composite_visual
        .as_ref()
        .expect("spawn_composite_mount_rider called on a non-composite spec");

    // Spawn both at the authored center. The mount's standalone
    // size is its `default_size`; the rider rides at
    // `pirate_on_shark_rider_offset(mount.size, rider.size)`.
    let center = authored.aabb.center();
    let mount_brain_payload = ambition_characters::actor::EnemyBrain::Custom(cv.mount_brain.clone());
    let mount_spec = spec_for_brain(&mount_brain_payload);
    let mount_size = mount_spec
        .default_size
        .expect("composite mount has a default_size in enemy_archetypes.ron");
    let mount_aabb = ae::Aabb::new(center, mount_size * 0.5);

    // Mount HP: take the composite spec's body HP rather than the
    // standalone mount's default, so tuning the composite's HP in the
    // RON works as expected.
    let composite_hp = composite_spec.max_health;
    // Mount keeps the authored id so the room-side FeatureVisual
    // entity (spawned by `spawn_room_visuals`) matches and resolves
    // its sprite via the standard upgrade path. The rider takes a
    // suffixed id (`<authored>:rider`) and gets its own FeatureVisual
    // entity from `spawn_composite_visuals` in
    // `presentation::rendering::world`.
    let mount_id = authored.id.clone();
    let mount_name = cv.mount_name.clone();
    let mut mount_enemy = super::actor_clusters::ActorClusterSeed::new(
        mount_id.clone(),
        mount_name.clone(),
        mount_aabb,
        mount_brain_payload,
        paths,
    );
    mount_enemy.status.health = ambition_characters::actor::Health::new(composite_hp);

    // Rider variant name. `rider_name_from_spawn` heavy variants parse the
    // authored spawn name (e.g. "Iron Mary on Shark" → "Iron Mary"),
    // falling back to the authored `rider_fallback_name`; light variants
    // always use the fallback.
    let rider_brain_payload = ambition_characters::actor::EnemyBrain::Custom(cv.rider_brain.clone());
    let rider_spec = spec_for_brain(&rider_brain_payload);
    let rider_variant_name = if cv.rider_name_from_spawn {
        authored
            .name
            .strip_suffix(" on Shark")
            .unwrap_or(&cv.rider_fallback_name)
            .to_string()
    } else {
        cv.rider_fallback_name.clone()
    };
    // Standalone size = the full cove-pirate hitbox (44x78 for the raider;
    // 72x110 for a heavy). Shark-rider size = the authored sky-rider scale.
    //
    // Important: mount status must not resize a character by default. A shark
    // rider begins at the compact sky-fight scale so she fits ON the shark, and
    // she keeps that same size after dismount. Larger cove PirateRaider /
    // PirateHeavy actors are separate authored spawns that use their full
    // standalone archetype sizes.
    let standalone_size = rider_spec.default_size.expect("rider has a default_size");
    let mounted_size = standalone_size * 0.5;
    let dismounted_size = mounted_size;
    // Rider starts at the same footprint it will keep after dismount, so
    // presentation / hitbox scale is invariant across mount state.
    let rider_offset = super::mount::pirate_on_shark_rider_offset(mount_size, mounted_size);
    let rider_pos = center + rider_offset;
    let rider_aabb = ae::Aabb::new(rider_pos, mounted_size * 0.5);
    let rider_id = format!("{}:rider", authored.id);
    let mut rider_enemy = super::actor_clusters::ActorClusterSeed::new(
        rider_id.clone(),
        rider_variant_name.clone(),
        rider_aabb,
        rider_brain_payload,
        paths,
    );
    // Override `spawn_size` so `reset_to_spawn` / mount-dissolve restore the
    // intended dismounted footprint rather than the temporary spawn AABB. For
    // PirateHeavy-on-shark this is deliberately the compact mounted-size body;
    // the cove PirateHeavy keeps the full-size body through normal spawns.
    rider_enemy.config.spawn.size = dismounted_size;
    rider_enemy.kin.size = mounted_size;
    // Rider HP from the composite spec's `rider_max_health`.
    if let Some(rider_hp) = composite_spec.rider_max_health {
        rider_enemy.status.health = ambition_characters::actor::Health::new(rider_hp);
    }
    rider_enemy.surface.gravity_scale = 0.0;

    // Build the rider's MOUNTED brain/action set through the shared
    // enemy brain-builder policy. The builder keeps composite ranged
    // behavior, forced mounted hostility, and per-rider jitter in one
    // place instead of hand-rolling a parallel setup here.
    let (rider_brain, rider_action_set) =
        mounted_rider_brain_and_action_set(&rider_id, &rider_spec, composite_spec);
    let rider_held_item =
        held_item_for_spec(composite_spec).or_else(|| held_item_for_spec(&rider_spec));

    // Build mount-side bundles and reserve the entity. We need both
    // entity IDs to link MountSlot ↔ RidingOn, so we spawn each and
    // then attach the link components. The mount should keep the
    // orbiting aerial brain so the shark still changes height while
    // the rider stays visually welded to it.
    let mount_brain = skirmisher_brain_for_enemy(&mount_enemy.config);
    let mount_action_set = enemy_default_action_set(&mount_enemy.spec);
    let mount_combat_kit = enemy_combat_kit_for_spec(&mount_enemy.spec);
    let mount_feature_aabb = CenteredAabb::from_aabb(mount_aabb);
    let mount_entity = EnemyActorSpawnPlan::hostile(
        format!("Feature actor mount: {mount_name}"),
        mount_id.clone(),
        mount_name.clone(),
        mount_feature_aabb,
        mount_enemy,
    )
    .with_brain(mount_brain)
    .with_action_set(mount_action_set)
    .with_combat_kit(mount_combat_kit)
    .without_held_item()
    .spawn(commands);
    commands.entity(mount_entity).insert((
        super::Mountable { rider_offset },
        super::MountSlot::default(),
        super::Mass(mount_spec.mass),
    ));

    // Rider-side bundles, with the RidingOn link pointing at the
    // mount we just spawned.
    let rider_combat_kit = enemy_combat_kit_for_spec(&rider_enemy.spec);
    let rider_feature_aabb = CenteredAabb::from_aabb(rider_aabb);
    // Cache the mounted brain on the rider so the same-room reset
    // path can restore it after a mount-death-then-reset cycle
    // (without the cache, the rider would keep their solo brain
    // after reset and the gun-sword would go silent).
    let mounted_brain_cache = super::MountedBrainCache {
        brain: rider_brain.clone(),
        action_set: rider_action_set.clone(),
    };
    let rider_entity = EnemyActorSpawnPlan::hostile(
        format!("Feature actor rider: {rider_variant_name}"),
        rider_id.clone(),
        rider_variant_name.clone(),
        rider_feature_aabb,
        rider_enemy,
    )
    .with_brain(rider_brain)
    .with_action_set(rider_action_set)
    .with_combat_kit(rider_combat_kit)
    .with_held_item(rider_held_item)
    .spawn(commands);
    commands.entity(rider_entity).insert((
        mounted_brain_cache,
        super::Mounted,
        super::MountedSize(mounted_size),
        super::RidingOn {
            mount: mount_entity,
        },
        super::Mass(rider_spec.mass),
    ));

    // Wire MountSlot.rider on the mount so death-side dissolution
    // can reach back from mount → rider.
    commands.entity(mount_entity).insert(super::MountSlot {
        rider: Some(rider_entity),
    });
}
