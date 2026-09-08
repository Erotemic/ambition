//! A rider whose mount died gets its own controller back.
//!
//! Answers the `MountDied` the mount crate announces by rebuilding the rider's
//! brain and action set on the LIVE entity. Kernel policy about a running actor;
//! the brain builder it calls stays in the spawn capability because a spawn and a
//! dismount build the same skirmisher/brute brain from the same config.

use ambition_combat::actor_tuning::ActorConfig;
use ambition_combat::components::CombatKit;
use ambition_combat::held_items::HeldItem;

/// Rebuild a fallen rider's solo brain, on the dissolution the mount ANNOUNCES.
///
/// ⭐ THE MOUNT MODULE DOES NOT CALL THE BUILDER ANY MORE. It writes
/// [`MountDied`](ambition_platformer2d_shared_tangle::body::MountDied) — which it
/// already did, for the boss bridge — and this system answers it. That is the
/// same road `ambition_boss_encounter` takes to turn the dissolution into a
/// rider boss's `External("mount_died")` phase: mount announces, the domain that
/// owns the reaction reacts.
///
/// ⛔ THE REBUILD CANNOT TRAVEL WITH A MOUNT CARVE and that is why it moved.
/// It reads `ActorConfig`, `CombatKit`, `HeldItem` and the prepared cast —
/// character-runtime facts, every one — so a mount crate that called it would
/// have to import the character runtime to dissolve a mount.
///
/// ⛔ A BOSS RIDER IS SKIPPED, unchanged: its identity is AUTHORED, not derived
/// from a kit, so re-deriving a brain for it would be wrong (ADR 0020; Q19b).
/// The component IS the marker — no new flag.
///
/// ⚠ ORDER, not shape, is what this had to preserve: the insert must land
/// before the dismounted body is simulated. It runs after
/// `ambition_mount::MountRiderLinkEnforced` in `CombatSet::Settle`, so its
/// commands flush at the same barrier the direct call's did.
/// ⚠ This said "chained straight after `enforce_mount_rider_link`" until
/// 2026-09-07, describing an arrangement the composition had already replaced
/// with the published-set anchor — a doc comment outliving the wiring it
/// describes. `install_dismounted_rider_rebuild` above is now the wiring.
/// Install the dismounted-rider rebuild against the mount crate's published set.
///
/// ⭐ BOTH ANCHORS ARE NAMEABLE HERE, which is the whole reason this moved.
/// `ambition_mount` is a dependency of this crate and `CombatSet` lives in
/// `shared_tangle`, so "run after the mount link is enforced, inside Settle" needs
/// no composition to know it. The rebuild ANSWERS the `MountDied` the mount crate
/// announces; ordering itself against that crate's PUBLISHED set is the system's
/// own business.
///
/// ⛔ THE REFERENCE DEFECT THIS REPLACED, kept because it is the lesson rather
/// than history: the composition used to write
/// `(ambition_mount::enforce_mount_rider_link, rebuild_dismounted_rider_brains).chain()`
/// — one crate fixing the relative order of two OTHER crates' private systems,
/// which the architecture program names as its example of private cross-domain
/// ordering authority. Anchoring on the published set fixed the reference; moving
/// the statement here fixes who says it.
///
/// ⚠ The SCHEDULE stays the caller's: `app.sim_schedule()` differs by host.
pub fn install_dismounted_rider_rebuild(
    app: &mut bevy::prelude::App,
    schedule: impl bevy::ecs::schedule::ScheduleLabel,
) {
    use bevy::prelude::IntoScheduleConfigs;
    app.add_systems(
        schedule,
        rebuild_dismounted_rider_brains
            .after(ambition_mount::MountRiderLinkEnforced)
            .in_set(ambition_platformer2d_shared_tangle::schedule::CombatSet::Settle),
    );
}

pub fn rebuild_dismounted_rider_brains(
    mut commands: bevy::prelude::Commands,
    mut dismounts: bevy::prelude::MessageReader<
        ambition_platformer2d_shared_tangle::body::MountDied,
    >,
    // The prepared cast, so a dismounted rider swings its own weapon rather
    // than borrowing an archetype's.
    prepared: Option<bevy::prelude::Res<ambition_characters::prepared::PreparedCharacterRegistry>>,
    riders: bevy::prelude::Query<(
        &ActorConfig,
        Option<&HeldItem>,
        Option<&CombatKit>,
        Option<&ambition_boss_encounter::BossConfig>,
    )>,
) {
    for dismount in dismounts.read() {
        let Ok((config, held_item, combat_kit, boss_config)) = riders.get(dismount.rider) else {
            continue;
        };
        if boss_config.is_some() {
            continue;
        }
        // A rider always carries a CombatKit; fall back defensively.
        let kit = combat_kit.cloned().unwrap_or_default();
        let (brain, action_set) = ambition_platformer2d_actor_spawn::brain_builders::dismounted_rider_brain_and_action_set(
            config,
            &kit,
            held_item.map(|item| &item.spec),
            prepared.as_deref(),
        );
        commands.entity(dismount.rider).insert((brain, action_set));
    }
}
