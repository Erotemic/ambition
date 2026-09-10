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
        // ⛔⛔ THE RECORD THAT A BRAIN WAS EVER TAKEN AWAY. A rider only gets a
        // controller BACK if boarding took one: `board()` swaps a brain exactly
        // when there is a `MountedBrainCache` to swap in, and a rider without
        // one drove itself from the saddle the whole ride.
        Option<&ambition_mount::MountedBrainCache>,
    )>,
) {
    for dismount in dismounts.read() {
        let Ok((config, held_item, combat_kit, boss_config, mounted_cache)) =
            riders.get(dismount.rider)
        else {
            continue;
        };
        if boss_config.is_some() {
            continue;
        }
        // ⛔⛔ A RIDER WHOSE BRAIN WAS NEVER SWAPPED HAS NOTHING TO GET BACK, AND
        // REBUILDING IT DESTROYS THE BRAIN IT WAS ACTUALLY RUNNING.
        //
        // MEASURED 2026-09-10: a `npc_pirate_admiral` mirror duel at rung 9 ends
        // with seat 1 running a `melee_brute` after summoning and losing its
        // shark — born `fighter`, and its `ActorConfig.brain_profile.template` is
        // still `Fighter` at that moment, so the match's policy landed and this
        // rebuild threw it away. `dismounted_rider_brain_and_action_set` chooses
        // between a skirmisher and a forced brute and consults no template at
        // all: it was written when those were the only two brains a rider could
        // have, and its module doc still says so — *"a spawn and a dismount build
        // the same skirmisher/brute brain from the same config"*. A seated
        // fighter is neither.
        //
        // ⇒ THE DISCRIMINATOR IS NOT A NEW FLAG, IT IS THE ONE THE MOUNT CRATE
        // ALREADY USES FOR EXACTLY THIS DISTINCTION. `enforce_mount_rider_link`
        // files a `TemporaryControl` claim only when a brain was swapped, and its
        // comment names this same rider: *"a seated fighter — the Smash Admiral,
        // every runtime `board()` customer — keeps driving itself from the
        // saddle"*. The claim is the BRAIN SWAP, not the ride; so is the rebuild.
        // The cache survives mount death on purpose, so it is still readable here.
        if mounted_cache.is_none() {
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

#[cfg(test)]
mod a_rider_that_drove_itself_keeps_its_brain {
    use super::*;
    use ambition_characters::brain::{
        ActionSet, Brain, BrainProfile, StateMachineCfg,
    };
    use ambition_combat::actor_tuning::ActorTuning;
    use ambition_platformer2d_shared_tangle::body::MountDied;
    use bevy::prelude::*;

    fn rider_config() -> ActorConfig {
        ActorConfig {
            id: "seat_fighter#seat1".into(),
            name: "Seat Fighter".into(),
            tuning: ActorTuning::default(),
            // ⭐ THE MATCH'S POLICY, which is what a smash seat carries: the
            // duel measured `template: Fighter` on the admiral's `ActorConfig`
            // at the very tick its `Brain` had already become a brute.
            brain_profile: BrainProfile {
                template: ambition_characters::brain::CharacterBrainTemplate::Fighter,
                ..Default::default()
            },
            brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                "smash_duelist_l9".into(),
            ),
            sprite_override_npc_name: None,
            sprite_character_id: Some("npc_pirate_admiral".into()),
            preserves_mirror_symmetry: false,
        }
    }

    fn fighter_brain() -> Brain {
        let cfg = ambition_characters::brain::fighter::FighterCfg::new(
            ambition_characters::brain::fighter::FighterBrainProfile::for_level(9),
        );
        let state = ambition_characters::brain::fighter::FighterState::new(&cfg, 0xABCD_1234);
        Brain::StateMachine(StateMachineCfg::Fighter {
            cfg: Box::new(cfg),
            state: Box::new(state),
        })
    }

    /// Spawn a rider, announce that its mount died, run the rebuild once, and
    /// report the brain label the rider is left holding.
    fn label_after_dismount(cached: bool) -> &'static str {
        let mut app = App::new();
        app.add_message::<MountDied>();
        app.add_systems(Update, rebuild_dismounted_rider_brains);

        let mut rider = app.world_mut().spawn((
            rider_config(),
            CombatKit::default(),
            fighter_brain(),
        ));
        if cached {
            rider.insert(ambition_mount::MountedBrainCache {
                brain: Brain::stand_still(),
                action_set: ActionSet::default(),
            });
        }
        let rider = rider.id();
        let mount = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(MountDied { mount, rider });
        app.update();

        app.world().entity(rider).get::<Brain>().expect("the rider still has a brain").label()
    }

    /// **A SEATED FIGHTER THAT LOSES ITS SHARK IS STILL A FIGHTER.**
    ///
    /// MEASURED 2026-09-10 in `smash_cpus_damage_each_other`: an
    /// `npc_pirate_admiral` mirror duel ends with seat 1 running a
    /// `melee_brute` — born `fighter`, `ActorConfig.brain_profile.template`
    /// still `Fighter`, five presses of `call_the_shark` in between. The match
    /// assigned a brain and a dismount threw it away, for the rest of the bout.
    ///
    /// ⚠ THE EDIT THAT MAKES THIS FALSE is deleting the `mounted_cache.is_none()`
    /// skip in [`rebuild_dismounted_rider_brains`] — which is exactly the state
    /// this was written against, and it fails naming `melee_brute`.
    #[test]
    fn a_rider_whose_brain_was_never_swapped_is_not_rebuilt() {
        assert_eq!(
            label_after_dismount(false),
            "fighter",
            "a `board()` customer with no `MountedBrainCache` drove itself from \
             the saddle, so losing the mount takes nothing away from it — but \
             the rebuild handed it a brain derived from its kit instead"
        );
    }

    /// **AND THE SYSTEM STILL DOES ITS JOB**, which is the arm that stops the
    /// one above from passing on a rebuild that never runs at all.
    ///
    /// A cached rider IS one whose brain was swapped on boarding, and it is the
    /// case the system was written for: it must come back with a rebuilt solo
    /// brain, not the fighter brain it happens to be holding.
    ///
    /// ⚠ THE EDIT THAT MAKES THIS FALSE is `return`ing from the system, or
    /// widening the skip above to every rider.
    #[test]
    fn a_rider_whose_brain_was_swapped_still_gets_one_back() {
        assert_eq!(
            label_after_dismount(true),
            "melee_brute",
            "a rider that gave up its controller on boarding must be handed one \
             back; if this says `fighter`, the rebuild is not running at all and \
             the guard beside it proves nothing"
        );
    }
}
