//! A rider whose mount died gets its own controller back.
//!
//! Answers the `MountDied` the mount crate announces by rebuilding the rider's
//! brain and action set on the LIVE entity. Kernel policy about a running actor;
//! the brain builder it calls stays in the spawn capability because a spawn and a
//! dismount build the same skirmisher/brute brain from the same config.

use ambition_combat::actor_tuning::ActorConfig;
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
/// It reads `ActorConfig` and `HeldItem` — character-runtime facts, both of
/// them — so a mount crate that called it would have to import the character
/// runtime to dissolve a mount.
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
    riders: bevy::prelude::Query<(
        &ActorConfig,
        &ambition_combat::components::ActorIdentity,
        Option<&HeldItem>,
        Option<&ambition_boss_encounter::BossConfig>,
        // ⛔⛤ IS THIS BODY A MATCH SEAT? See the skip below: a seat's brain is
        // the MATCH's decision, never a derivation from a kit.
        Option<&ambition_match::MatchSeat>,
    )>,
) {
    for dismount in dismounts.read() {
        let Ok((config, identity, held_item, boss_config, match_seat)) =
            riders.get(dismount.rider)
        else {
            continue;
        };
        if boss_config.is_some() {
            continue;
        }
        // ⛔⛔ A MATCH SEAT'S BRAIN IS THE MATCH'S DECISION, NEVER A DERIVATION
        // FROM A KIT — AND BOTH KINDS OF SEAT WERE BEING OVERWRITTEN.
        //
        // MEASURED 2026-09-10, twice, on the same character:
        //
        // - A CPU SEAT. An `npc_pirate_admiral` mirror duel at rung 9 ended with
        //   seat 1 running a `melee_brute` after summoning and losing its shark —
        //   born `fighter`, and its `ActorConfig.brain_profile.template` still
        //   `Fighter` at that moment, so the match's policy had landed and this
        //   rebuild threw it away for the rest of the bout.
        // - A HUMAN SEAT. `killing_the_shark_puts_the_admiral_down_and_frees_the_
        //   up_b` seats a player at seat 0: it rode as `stand_still` and came
        //   down a `melee_brute`. ⚠ That placeholder is DELIBERATE and
        //   `prepared.rs` says why — *"a local-input seat authors `Passive`
        //   because its real driver is attached afterwards; a passive placeholder
        //   rather than a wandering one, so a body whose writer never arrives
        //   stands still instead of strolling off looking possessed"*. Replacing
        //   it with a hostile brute is exactly the failure that placeholder
        //   exists to prevent, waiting for the player's input to lapse.
        //
        // `dismounted_rider_brain` chooses between a skirmisher
        // and a forced brute and consults no template at all. That is not a bug
        // in it — the module doc above says so in as many words, *"a spawn and a
        // dismount build the same skirmisher/brute brain from the same config"*.
        // It was written when those were the only two brains a rider could have.
        // A seat is neither, and the comment is an accurate specification of a
        // world with two templates that now has twelve.
        //
        // ⇒ THE SEAT IS THE DISCRIMINATOR, not the brain it happens to hold. A
        // human seat and a room-spawned NPC can both sit on `stand_still` and
        // they mean opposite things: one is a placeholder the match chose, the
        // other is a body whose brain this builder is FOR. Nothing about the
        // brain's own value can tell them apart — only who assigned it.
        //
        // ⛔ TWO DISCRIMINATORS THAT LOOKED RIGHT AND WERE NOT, both caught by
        // tests rather than by reasoning:
        //
        // 1. `MountedBrainCache`. The mount crate draws exactly this distinction
        //    for `ControlClaims` — *"the claim is the BRAIN SWAP, not a
        //    ride"* — so keying on the cache read as the same rule stated twice.
        //    **Nothing in the tree ever CONSTRUCTS that component**: it is
        //    declared, rollback-registered, read in three places and documented
        //    as "attached at composite spawn", and every construction is in a
        //    test. Keying on it disabled the rebuild for every production rider,
        //    and a fallen pirate raider stopped getting the ranged brain that
        //    keeps its gun-sword live.
        // 2. `Brain::Fighter | Brain::Smash`. True of the CPU seat and blind to
        //    the human one, whose placeholder is `StandStill` — the same value a
        //    room NPC carries when it genuinely wants rebuilding.
        // ⭐ AND THIS ONE WAS CHECKED FOR A WRITER BEFORE IT WAS TRUSTED, which
        // is the whole difference from (1). `MatchSeat` has exactly ONE
        // production construction site — `character_runtime::match_activation`,
        // where a seat is realized — with every other occurrence a read, a
        // snapshot decode, or a test. ⇒ *"this body's brain was assigned by the
        // match"* is true by construction rather than by hope. **Grep for a
        // CONSTRUCTION, not a mention: reads are not writers.**
        //
        // ⚠ SCOPE, so the next reader can re-ask it rather than assume: this is
        // right while the only seated bodies are match participants. If a mode
        // ever seats a rider whose brain IS meant to be kit-derived, this skips
        // it and the symptom looks exactly like the raider's. The arm below is
        // the fixture that would say so.
        if match_seat.is_some() {
            continue;
        }
        // ONLY THE BRAIN. A rider's repertoire is the projection of its
        // identity, its worn equipment and its hand
        // (`ambition_characters::repertoire`), and a mount dying moves none of
        // those — so the set it lands with is the set it was riding with. This
        // used to rebuild an action set from the identity baseline alone, which
        // dropped any granted verb the rider was wearing.
        let brain = ambition_platformer2d_actor_spawn::brain_builders::dismounted_rider_brain(
            config,
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

    /// The rider's config and identity, spawned together as the cluster
    /// bundle would.
    fn rider_config() -> (ActorConfig, ambition_combat::components::ActorIdentity) {
        let config = ActorConfig {
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
            sprite_character_id: Some("npc_pirate_admiral".into()),
            preserves_mirror_symmetry: false,
        };
        let identity =
            ambition_combat::components::ActorIdentity::new("seat_fighter#seat1", "Seat Fighter");
        (config, identity)
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
