//! A rider whose mount died gets its own controller back.
//!
//! Answers the `MountDied` the mount crate announces by rebuilding the rider's
//! brain and action set on the LIVE entity. Kernel policy about a running actor;
//! the brain builder it calls stays in the spawn capability because a spawn and a
//! dismount build the same skirmisher/brute brain from the same config.

use ambition_combat::actor_tuning::ActorConfig;
use ambition_combat::held_items::HeldItem;

/// Rebuild a non-seat rider's brain after `MountDied`.
///
/// Mount logic emits the event; character runtime restores the brain
/// from actor policy, identity, and equipment. Boss riders retain their
/// authored behavior. The system runs after `MountRiderLinkEnforced` in
/// `CombatSet::Settle` so the restored brain is ready for simulation.
/// The caller selects the simulation schedule.
pub fn install_dismounted_rider_rebuild(
    app: &mut bevy::prelude::App,
    schedule: impl bevy::ecs::schedule::ScheduleLabel,
) {
    use bevy::prelude::IntoScheduleConfigs;
    app.add_systems(
        schedule,
        rebuild_dismounted_rider_brains
            .run_if(ambition_platformer2d_core::a_message_waits::<ambition_platformer2d_shared_tangle::body::MountDied>)
            .after(ambition_mount::MountRiderLinkEnforced)
            .in_set(ambition_platformer2d_shared_tangle::schedule::CombatSet::Settle),
    );
}

pub fn rebuild_dismounted_rider_brains(
    mut commands: bevy::prelude::Commands,
    mut dismounts: bevy::prelude::MessageReader<
        ambition_platformer2d_shared_tangle::body::MountDied,
    >,
    riders: bevy::prelude::Query<(
        &ActorConfig,
        &ambition_combat::actor_tuning::ActorPolicy,
        &ambition_combat::components::ActorIdentity,
        Option<&HeldItem>,
        Option<&ambition_boss_encounter::BossConfig>,
        // ⛔⛤ IS THIS BODY A MATCH SEAT? See the skip below: a seat's brain is
        // the MATCH's decision, never a derivation from a kit.
        Option<&ambition_match::MatchSeat>,
    )>,
) {
    for dismount in dismounts.read() {
        let Ok((config, policy, identity, held_item, boss_config, match_seat)) =
            riders.get(dismount.rider)
        else {
            continue;
        };
        if boss_config.is_some() {
            continue;
        }
        // Match seats keep the brain chosen by their match policy.
        // This includes human seats with a passive placeholder; neither
        // that placeholder nor a CPU fighter brain is kit-derived.
        if match_seat.is_some() {
            continue;
        }
        // Rebuild only the brain. Dismounting does not change identity,
        // equipment, or the repertoire derived from them.
        let brain = ambition_platformer2d_actor_spawn::brain_builders::dismounted_rider_brain(
            config,
            &policy.0,
            identity,
            held_item.map(|item| &item.spec),
        );
        commands.entity(dismount.rider).insert(brain);
    }
}

#[cfg(test)]
mod a_seat_keeps_its_brain_and_an_unseated_rider_gets_one_back {
    use super::*;
    use ambition_characters::brain::{Brain, BrainProfile, StateMachineCfg};
    use ambition_combat::actor_tuning::ActorTuning;
    use ambition_platformer2d_shared_tangle::body::MountDied;
    use bevy::prelude::*;

    /// The rider's config, policy and identity, spawned together as the
    /// cluster bundle would.
    fn rider_config() -> (
        ActorConfig,
        ambition_combat::actor_tuning::ActorPolicy,
        ambition_combat::components::ActorIdentity,
    ) {
        // ⭐ THE MATCH'S POLICY, which is what a smash seat carries: the duel
        // measured `template: Fighter` on the admiral's policy at the very tick
        // its `Brain` had already become a brute.
        let policy = ambition_combat::actor_tuning::ActorPolicy(BrainProfile {
            template: ambition_characters::brain::CharacterBrainTemplate::Fighter,
            ..Default::default()
        });
        let config = ActorConfig {
            tuning: ActorTuning::default(),
            brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                "smash_duelist_l9".into(),
            ),
            preserves_mirror_symmetry: false,
        };
        let identity =
            ambition_combat::components::ActorIdentity::new("seat_fighter#seat1", "Seat Fighter");
        (config, policy, identity)
    }

    fn fighter_brain() -> Brain {
        let cfg = ambition_characters::brain::fighter::FighterCfg::new(
            ambition_characters::brain::fighter::FighterBrainProfile::engine_default(9),
        );
        let state = ambition_characters::brain::fighter::FighterState::new(&cfg, 0xABCD_1234);
        Brain::StateMachine(StateMachineCfg::Fighter {
            cfg: Box::new(cfg),
            state: Box::new(state),
        })
    }

    /// Spawn a rider on a fighter brain, optionally SEATED in a match,
    /// announce that its mount died, run the rebuild once, and report the brain
    /// label the rider is left holding.
    fn label_after_dismount(seated: bool) -> &'static str {
        let mut app = App::new();
        app.add_message::<MountDied>();
        app.add_systems(Update, rebuild_dismounted_rider_brains);

        let mut rider =
            app.world_mut()
                .spawn((rider_config(), fighter_brain()));
        if seated {
            rider.insert(ambition_match::MatchSeat(1));
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
    fn a_seated_fighter_that_loses_its_mount_is_still_a_fighter() {
        assert_eq!(
            label_after_dismount(true),
            "fighter",
            "a SEAT's brain is the match's decision — a brain DERIVED FROM ITS \
             KIT is not a worse version of one, it is a different character for \
             the rest of the match"
        );
    }

    /// **AND EVERY OTHER RIDER STILL GETS ITS BRAIN BACK**, which is the arm
    /// that stops the one above from passing on a rebuild that never runs.
    ///
    /// ⛔⛔ THIS ARM IS NOT DECORATION. The first version of this fix keyed the
    /// skip on `MountedBrainCache` — a component the mount crate documents as
    /// *"attached at composite spawn"* and which **nothing in the tree ever
    /// constructs outside a test**. Every production rider read as uncached, the
    /// rebuild was disabled for all of them, and a fallen pirate raider stopped
    /// getting the ranged brain that keeps its gun-sword live. A unit arm with a
    /// hand-inserted cache PASSED, because the fixture supplied what production
    /// does not. ⇒ The app-level
    /// `a_dead_mount_rebuilds_its_riders_brain_through_the_real_schedule` is what
    /// caught it, and this arm is its cheap local echo.
    ///
    /// ⚠ THE EDIT THAT MAKES THIS FALSE is `return`ing from the system, or
    /// widening the skip above past `Fighter`/`Smash`.
    #[test]
    fn an_unseated_rider_still_gets_a_brain_back() {
        assert_eq!(
            label_after_dismount(false),
            "melee_brute",
            "a rider that is NOT a match seat is exactly what this builder is \
             for, and must be handed a rebuilt brain; if this says `fighter`, \
             the rebuild is not running at all and the guard beside it proves \
             nothing. ⚠ The two arms differ ONLY in the `MatchSeat`, and the \
             brain they start on is identical — so neither can pass by accident \
             of what it was holding."
        );
    }

    /// A rider whose repertoire baseline is absent keeps the repertoire it has.
    ///
    /// Defaulting the baseline instead would rebuild the live `ActionSet` from an
    /// empty one, which then falls through `melee.is_none()` into the ruleset's
    /// provoked swipe — so a missing baseline disarms the body and re-arms it
    /// with somebody else's weapon. The brain half still runs: it is chosen from
    /// the config and the held item and reads no baseline.
    #[test]
    fn a_rider_with_no_baseline_is_not_handed_an_empty_one() {
        let mut app = App::new();
        app.add_message::<MountDied>();
        app.add_systems(Update, rebuild_dismounted_rider_brains);

        let live = ambition_characters::brain::ActionSet {
            melee: Some(ambition_characters::brain::MeleeActionSpec::Swipe(
                ambition_characters::brain::SwipeSpec {
                    windup_s: 0.2,
                    active_s: 0.1,
                    recover_s: 0.2,
                    damage: 4,
                    reach_px: 44.0,
                },
            )),
            ..Default::default()
        };
        // A rider carrying a live repertoire and NO baseline to rebuild from.
        let rider = app
            .world_mut()
            .spawn((rider_config(), live.clone(), fighter_brain()))
            .id();
        let mount = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(MountDied { mount, rider });
        app.update();

        let after = app
            .world()
            .entity(rider)
            .get::<ambition_characters::brain::ActionSet>()
            .expect("the rider still has a repertoire");
        assert_eq!(
            after.melee, live.melee,
            "a dismount rebuilt this rider's repertoire from an INVENTED empty \
             baseline and took its swing away; a body whose baseline is absent \
             must be left holding what it had"
        );
        // AND THE BRAIN STILL COMES BACK, or the arm above passes by the system
        // having skipped the rider entirely — which is the same rebuild-never-ran
        // failure `an_unseated_rider_still_gets_a_brain_back` exists to catch.
        assert_eq!(
            app.world()
                .entity(rider)
                .get::<Brain>()
                .expect("the rider still has a brain")
                .label(),
            "melee_brute",
            "the repertoire was preserved by skipping the whole rebuild, so this \
             says nothing about the invention — the brain half must still run"
        );
    }
}
