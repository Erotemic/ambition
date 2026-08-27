use super::*;
use crate::features::FeatureSimEntity;
use crate::features::NPC_HOSTILE_STRIKE_THRESHOLD;
use ambition_combat::components::{CenteredAabb, FeatureId};
use ambition_platformer2d_core::{self as ae, AabbExt};
use bevy::prelude::{App, Update};

fn spawn_npc_with_strikes(app: &mut App, strikes: i32) -> bevy::prelude::Entity {
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    let interactable = ambition_interaction::Interactable::new(
        "alice",
        "Talk",
        aabb,
        ambition_interaction::InteractionKind::Npc {
            character_id: None,
            dialogue_id: None,
            patrol_radius: 0.0,
            patrol_path_id: None,
            brain_override: None,
        },
    );
    // Peaceful actor = the unified enemy cluster with peaceful tuning.
    let (seed, _render) = super::super::actor_clusters::ActorClusterSeed::new_peaceful_npc(
        "alice",
        "Alice",
        aabb,
        &interactable,
        &[],
    );
    spawn_actor_from_seed(app, seed, "alice", aabb, interactable, strikes)
}

/// The spawn half of [`spawn_npc_with_strikes`], shared with the flight fixture
/// below so both bodies reach the world through the same components.
fn spawn_actor_from_seed(
    app: &mut App,
    seed: super::super::actor_clusters::ActorClusterSeed,
    id: &str,
    aabb: ae::Aabb,
    interactable: ambition_interaction::Interactable,
    strikes: i32,
) -> bevy::prelude::Entity {
    let (identity, disposition, combat) =
        super::super::actors::actor_component_snapshot(&seed, ActorDisposition::Peaceful);
    // Provoke accumulator lives on `ActorAggression` now.
    let aggression = ActorAggression {
        mode: AggressionMode::RetaliatesWhenHit {
            strike_threshold: NPC_HOSTILE_STRIKE_THRESHOLD as u8,
        },
        target: None,
        strikes,
        grudge: None,
    };
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new(id),
            CenteredAabb::from_center_size(aabb.center(), aabb.half_size() * 2.0),
            aggression,
            CombatKit::default(),
            seed.into_components(),
            ActorInteraction {
                interactable,
                talk_radius: crate::features::NPC_TALK_RADIUS,
            },
            identity,
            disposition,
            combat,
        ))
        .id()
}

fn run(app: &mut App, actor: bevy::prelude::Entity) {
    app.world_mut().write_message(ActorStimulus::DamagedBy {
        actor,
        source: None,
        damage: 1,
    });
    app.update();
}

#[test]
fn npc_flips_hostile_with_a_grudge_against_its_attacker() {
    let mut app = App::new();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_actor_stimuli);
    // Already at the strike threshold (the damage system increments
    // strikes; this stimulus is the provocation that re-evaluates).
    let npc = spawn_npc_with_strikes(&mut app, NPC_HOSTILE_STRIKE_THRESHOLD);
    let attacker = app.world_mut().spawn_empty().id();
    app.world_mut().write_message(ActorStimulus::DamagedBy {
        actor: npc,
        source: Some(attacker),
        damage: 1,
    });
    app.update();
    assert_eq!(
        *app.world().get::<ActorDisposition>(npc).unwrap(),
        ActorDisposition::Hostile,
        "an NPC at the strike threshold should flip hostile when provoked"
    );
    // It hunts its attacker through a per-actor GRUDGE — NOT by mutating its
    // faction identity (the old in-place flip to Enemy is gone). Targeting
    // treats the grudge entity as a foe; the victim-side damage gate is
    // different-faction (`can_damage`), which an Npc→Player hit already passes.
    assert_eq!(
        app.world().get::<ActorAggression>(npc).unwrap().grudge,
        Some(attacker),
        "a provoked NPC holds a grudge against the entity that struck it"
    );
    assert!(
        app.world()
            .get::<ambition_combat::components::ActorFaction>(npc)
            .is_none(),
        "provoke must NOT insert an Enemy faction — identity is preserved, the grudge does the work"
    );
}

#[test]
fn a_pending_challenge_defers_the_flip_until_its_grace_elapses() {
    // `<<challenge>>` arms a `PendingChallenge`; the hostile flip must NOT fire
    // until the grace (counted only in `Playing`, i.e. after the dialog box
    // closes) elapses — so the player isn't attacked point-blank mid-dialog.
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime {
        scaled_dt: 1.0,
        ..Default::default()
    });
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, tick_pending_challenges);
    let actor = app
        .world_mut()
        .spawn(PendingChallenge {
            challenger: None,
            grace: CHALLENGE_GRACE_S, // 2.0
        })
        .id();

    // One 1.0 s tick (grace 2.0 → 1.0): still armed, no stimulus yet.
    app.update();
    assert!(
        app.world().get::<PendingChallenge>(actor).is_some(),
        "still armed before the grace elapses"
    );
    assert!(
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorStimulus>>()
            .drain()
            .next()
            .is_none(),
        "no Challenged stimulus before the grace elapses"
    );

    // Second 1.0 s tick (grace 1.0 → 0.0): fires + the armed marker is consumed.
    app.update();
    assert!(
        app.world().get::<PendingChallenge>(actor).is_none(),
        "the armed challenge is consumed once it fires"
    );
    let fired: Vec<_> = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ActorStimulus>>()
        .drain()
        .collect();
    assert!(
        matches!(fired.as_slice(), [ActorStimulus::Challenged { actor: a, .. }] if *a == actor),
        "the deferred challenge emits exactly one Challenged for the actor"
    );
}

#[test]
fn npc_below_the_threshold_stays_peaceful() {
    let mut app = App::new();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_actor_stimuli);
    let npc = spawn_npc_with_strikes(&mut app, NPC_HOSTILE_STRIKE_THRESHOLD - 1);
    run(&mut app, npc);
    assert_eq!(
        *app.world().get::<ActorDisposition>(npc).unwrap(),
        ActorDisposition::Peaceful,
        "an NPC below the strike threshold should stay peaceful"
    );
}

#[test]
fn a_challenge_flips_a_peaceful_npc_hostile_with_zero_strikes() {
    // The dialogue-gated combat trigger: an explicit `Challenged`
    // stimulus provokes the actor unconditionally — no strikes, no
    // threshold — because picking "challenge" IS consent to fight. This
    // is the gate the Perfect Cell-ular Automaton encounter rides on.
    let mut app = App::new();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_actor_stimuli);
    let npc = spawn_npc_with_strikes(&mut app, 0);
    app.world_mut().write_message(ActorStimulus::Challenged {
        actor: npc,
        challenger: None,
    });
    app.update();
    assert_eq!(
        *app.world().get::<ActorDisposition>(npc).unwrap(),
        ActorDisposition::Hostile,
        "a challenged NPC must flip hostile even with zero strikes"
    );
    // The flip swaps in a hostile combat brain (the generic provoked NPC
    // resolves to the `combatant` Smash brawler). Pin that it's now a
    // reactive fighter, not the peaceful stand-still brain.
    let brain = app
        .world()
        .get::<ambition_characters::brain::Brain>(npc)
        .expect("provoke inserts a Brain");
    assert!(
        brain.is_hostile(),
        "the post-challenge brain should be hostile, got {}",
        brain.label()
    );
}

/// Re-deriving the brain on every stimulus zeroed all of its `SmashState` cadences (ranged / dash /
/// blink / footsies timers, mode-dwell hysteresis) each hit — which is what turned the Perfect
/// Cell-ular Automaton into a per-tick glider spammer that never got to duel. The live brain (and
/// its accumulated state) must persist across repeat stimuli.
#[test]
fn a_repeat_stimulus_preserves_an_already_hostile_brain_state() {
    use ambition_characters::brain::{Brain, StateMachineCfg};
    let mut app = App::new();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_actor_stimuli);
    let npc = spawn_npc_with_strikes(&mut app, 0);
    // First stimulus: the peaceful→hostile flip builds the (combatant Smash)
    // brain exactly once.
    app.world_mut().write_message(ActorStimulus::Challenged {
        actor: npc,
        challenger: None,
    });
    app.update();
    // Advance a cadence on the LIVE brain, as a mid-duel shot would.
    const SENTINEL: f32 = 0.9;
    {
        let mut brain = app
            .world_mut()
            .get_mut::<Brain>(npc)
            .expect("the flip inserts a Brain");
        let Brain::StateMachine(StateMachineCfg::Smash { state, .. }) = &mut *brain else {
            panic!("the provoked combatant should be a Smash brain");
        };
        state.sprint_cooldown_remaining = SENTINEL;
        state.mode_dwell_s = SENTINEL;
    }
    // A second stimulus on the now-hostile actor must leave the brain intact.
    app.world_mut().write_message(ActorStimulus::DamagedBy {
        actor: npc,
        source: None,
        damage: 1,
    });
    app.update();
    let brain = app.world().get::<Brain>(npc).unwrap();
    let Brain::StateMachine(StateMachineCfg::Smash { state, .. }) = brain else {
        panic!("the brain should still be a Smash brain");
    };
    assert_eq!(
        state.sprint_cooldown_remaining, SENTINEL,
        "a repeat stimulus must not reset the brain's dash cadence (no brain rebuild)"
    );
    assert_eq!(
        state.mode_dwell_s, SENTINEL,
        "a repeat stimulus must not reset mode-dwell hysteresis"
    );
}

/// One character whose locomotion authors free flight — the parrot / burning-shark
/// case, built through `prepare_and_finalize_for_test` so the body that reaches
/// the world is the one production construction produces.
fn npc_cast(
    flight: Option<bool>,
    max_health: Option<i32>,
) -> crate::character_runtime::PreparedCharacterRegistry {
    let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
    let mut definition = crate::character_runtime::CharacterDefinition::new(
        "npc_test_parrot",
        "Test Parrot",
        "test",
    )
    .with_locomotion(ambition_characters::actor::CharacterLocomotion {
        run_speed: 120.0,
        baseline_free_flight: flight,
        ..Default::default()
    });
    definition.vitals.max_health = max_health;
    let finalized = crate::character_runtime::prepare_and_finalize_for_test(
        definition,
        &crate::character_runtime::CharacterBindings::default(),
    );
    registry.insert_prepared(finalized.prepared);
    registry
}

/// A peaceful NPC placement naming `npc_test_parrot`, built through the
/// production character-first seed so the body is the one the game constructs.
fn spawn_character_npc(
    app: &mut App,
    cast: &crate::character_runtime::PreparedCharacterRegistry,
) -> bevy::prelude::Entity {
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    let interactable = ambition_interaction::Interactable::new(
        "parrot",
        "Talk",
        aabb,
        ambition_interaction::InteractionKind::Npc {
            character_id: Some("npc_test_parrot".into()),
            dialogue_id: None,
            patrol_radius: 0.0,
            patrol_path_id: None,
            brain_override: None,
        },
    );
    let (seed, _render) = super::super::actor_clusters::ActorClusterSeed::new_peaceful_npc_in(
        &Default::default(),
        &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        Some(cast),
        "parrot",
        "Parrot",
        aabb,
        &interactable,
        &[],
    );
    spawn_actor_from_seed(app, seed, "parrot", aabb, interactable, 0)
}

fn spawn_flying_npc(app: &mut App) -> bevy::prelude::Entity {
    spawn_character_npc(app, &npc_cast(Some(true), None))
}

/// A FLYING BODY STAYS FLYING WHEN IT IS PROVOKED.
///
/// But the engine's default provoked policy is `CharacterBrainTemplate::Smash`, and the Smash brain
/// branches on `obs.self_aerial` with no `can_fly` gate (`smash/mod.rs`: *"Flyer: the grounded
/// motor outputs don't apply — discard them and steer a 2D velocity"*). `cfg.can_fly` gates only
/// the hybrid TAKE-OFF/LANDING toggle, which is exactly right: a baseline flyer never toggles, it
/// simply flies. And `can_fly` itself is read off the body's own `AbilitySet`
/// (`smash_cfg_from_spec`), which `ActorBody::from_kit` forces true for an aerial body.
///
/// The brain read that half-body as aerial (`gravity_scale <= 0.001 || fly_enabled`) while the
/// integrator read it as grounded (`fly_enabled` alone) — so it really did freeze, from the
/// fixture's own disagreement rather than from provocation.
#[test]
fn a_flying_npc_stays_flying_when_it_is_provoked() {
    use crate::features::enemies::ActorSurfaceState;
    use ambition_characters::brain::{Brain, StateMachineCfg};

    let mut app = App::new();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_actor_stimuli);
    let npc = spawn_flying_npc(&mut app);

    //  THE REALISM GUARD. A body that does not actually fly would satisfy
    // "still flying afterwards" trivially.
    assert_eq!(
        app.world()
            .get::<ActorSurfaceState>(npc)
            .unwrap()
            .gravity_scale,
        0.0,
        "the fixture must genuinely be a flight-authored body"
    );
    assert!(
        app.world()
            .get::<ambition_platformer2d_core::BodyFlightState>(npc)
            .unwrap()
            .fly_enabled,
        "and it must agree with itself: the integrator's flight predicate is \
         `fly_enabled`, not gravity"
    );
    assert!(
        app.world()
            .get::<ambition_platformer2d_core::BodyAbilities>(npc)
            .unwrap()
            .abilities
            .fly,
        "and the body must carry the fly VERB, since that is what the driver asks"
    );

    app.world_mut().write_message(ActorStimulus::Challenged {
        actor: npc,
        challenger: None,
    });
    app.update();

    assert_eq!(
        *app.world().get::<ActorDisposition>(npc).unwrap(),
        ActorDisposition::Hostile,
        "the provocation must actually land"
    );
    assert_eq!(
        app.world()
            .get::<ActorSurfaceState>(npc)
            .unwrap()
            .gravity_scale,
        0.0,
        "a provoked parrot is an angry parrot, not a grounded one — provocation \
         changes the mind and the relationship, never the body"
    );
    assert!(
        app.world()
            .get::<ambition_platformer2d_core::BodyFlightState>(npc)
            .unwrap()
            .fly_enabled,
        "and its flight mode survives too, or the body and the brain would \
         disagree about what it is"
    );

    //  THE POISON: "it changed nothing" satisfies every assertion above while
    // describing a provocation that does not provoke. The driver it was handed
    // must be a real hostile mind that KNOWS this body flies.
    let brain = app
        .world()
        .get::<Brain>(npc)
        .expect("the flip inserts a Brain");
    let Brain::StateMachine(StateMachineCfg::Smash { cfg, .. }) = brain else {
        panic!("a provoked body is driven by the engine's default provoked policy");
    };
    assert!(
        cfg.can_fly,
        "and that policy was lowered against THIS body's verbs — a driver that \
         thought it was grounded is how the old body-mutation got justified"
    );
}

/// A PROVOKED BODY KEEPS THE HEALTH POOL ITS CHARACTER AUTHORED — AND THE
/// DAMAGE IT HAD ALREADY TAKEN.
///
/// It existed because a peaceful placement spawned at `max_health: 1` and a provoked one that kept
/// its own pool died to the next hit.
///
/// The value did not change, so nothing about how tough a provoked NPC is changed either —
/// which is what makes this a repair of the authority rather than a rebalance.
///
///  the DAMAGE half is the sharper assertion: a pool the right SIZE would satisfy a max-only
/// check while still having healed a wounded creature mid- fight, which is the same divergence
/// class the reconciler would have shipped.
#[test]
fn a_provoked_body_keeps_the_health_pool_its_character_authored() {
    use ambition_characters::actor::BodyHealth;

    let mut app = App::new();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_actor_stimuli);
    let npc = spawn_character_npc(&mut app, &npc_cast(Some(false), Some(9)));

    //  THE REALISM GUARD: 9 is nothing the engine would pick, so this pool can
    // only have come from the character.
    assert_eq!(
        app.world().get::<BodyHealth>(npc).unwrap().max(),
        9,
        "the fixture must be a body whose character states its own vitals"
    );
    assert_ne!(
        9,
        ambition_characters::actor::DEFAULT_UNAUTHORED_BODY_HEALTH,
        "or the assertion below would pass on a build that had thrown the \
         authored pool away and installed the engine default"
    );

    // A body mid-fight, not a pristine one.
    app.world_mut()
        .get_mut::<BodyHealth>(npc)
        .unwrap()
        .damage(4);
    app.world_mut().write_message(ActorStimulus::Challenged {
        actor: npc,
        challenger: None,
    });
    app.update();

    assert_eq!(
        *app.world().get::<ActorDisposition>(npc).unwrap(),
        ActorDisposition::Hostile,
        "the provocation must actually land"
    );
    let health = *app.world().get::<BodyHealth>(npc).unwrap();
    assert_eq!(
        health.max(),
        9,
        "provocation must not resize a body it did not author"
    );
    assert_eq!(
        health.current(),
        5,
        "and it must not heal one either — the old flip replaced the whole \
         `BodyHealth`, so being provoked was also a free full heal"
    );
}

/// PROVOKING A BODY SOMEBODY IS DRIVING DOES NOT TAKE IT AWAY FROM THEM.
///
/// the flip inserted the provoked `Brain` unconditionally, and for a body under player control that
/// is a silent seizure: the first hit a SEATED FIGHTER took replaced its own policy with the Smash
/// state machine, in place and permanently — activation is one-shot and never rebinds — so a
/// human's fighter became a CPU mid-fight and the couch test read it as input crosstalk.
///
///  what a provocation may do to a driven body: change its RELATIONSHIP, land
/// its action set (what a body fights with is part of what it is), and record
/// the autonomous source that will resume when control is released
/// (`a_released_character_returns_to_its_own_policy_not_the_provoked_one` is the
/// other end of that thread).
#[test]
fn provoking_a_player_driven_body_changes_its_mood_and_not_its_driver() {
    use ambition_characters::actor::character_catalog::{
        AutonomousSource, BrainBinding, BrainPresetId,
    };
    use ambition_characters::brain::{ActionSet, Brain, StateMachineCfg};
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};

    let mut app = App::new();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_actor_stimuli);

    let cast = npc_cast(Some(false), None);
    let driven = spawn_character_npc(&mut app, &cast);
    let free = spawn_character_npc(&mut app, &cast);
    for body in [driven, free] {
        app.world_mut().entity_mut(body).insert(BrainBinding::new(
            BrainPresetId::new("stroll"),
            AutonomousSource::CatalogDefault,
        ));
    }
    app.world_mut()
        .entity_mut(driven)
        .insert(DrivingParticipant(PlayerSlot::PRIMARY));

    for body in [driven, free] {
        app.world_mut().write_message(ActorStimulus::Challenged {
            actor: body,
            challenger: None,
        });
    }
    app.update();

    //  THE POISON, and it runs first because it is what proves the assertion
    // below is about the DRIVER rather than about provocation doing nothing. The
    // same stimulus on the same body with nobody at the controls installs a
    // hostile mind.
    assert!(
        matches!(
            app.world().get::<Brain>(free),
            Some(Brain::StateMachine(StateMachineCfg::Smash { .. }))
        ),
        "an undriven body must actually receive the provoked mind, or this test \
         would pass on a build where provocation had stopped working entirely"
    );

    assert_eq!(
        app.world().get::<DrivingParticipant>(driven).map(|d| d.0),
        Some(PlayerSlot::PRIMARY),
        "a body under player control must still be under player control — \
         provocation changes what a body IS, never who drives it"
    );
    assert!(
        !matches!(
            app.world().get::<Brain>(driven),
            Some(Brain::StateMachine(StateMachineCfg::Smash { .. }))
        ),
        "the driven body's own policy was seized — the provoked mind must not be \
         installed over the one a person is currently playing with"
    );
    assert_eq!(
        *app.world().get::<ActorDisposition>(driven).unwrap(),
        ActorDisposition::Hostile,
        "the relationship still changes; leaving the driver alone is not the \
         same as ignoring the provocation"
    );
    assert!(
        app.world().get::<ActionSet>(driven).is_some(),
        "and the provoked kit still lands — what a body fights with is part of \
         what it is, and only the driver is left alone"
    );
    assert_eq!(
        app.world().get::<BrainBinding>(driven).map(|b| &b.source),
        Some(&AutonomousSource::ProvokedDefault),
        "and the SOURCE that resumes on release is the provoked one — otherwise \
         letting go of a body you angered would hand back a peaceful stroller"
    );
}

/// The poison for the pool above: a character that authors NOTHING gets the
/// undescribed-body default, and provocation leaves that alone too.
///
///  without this, moving the constant could have gone wrong in the quiet
/// direction — an unauthored body left at `1` would still satisfy every
/// assertion in the test above, and the first villager to turn hostile would die
/// to one hit. That is the gameplay change this refactor exists NOT to make.
#[test]
fn an_unauthored_body_gets_the_undescribed_pool_before_anybody_hits_it() {
    use ambition_characters::actor::{BodyHealth, DEFAULT_UNAUTHORED_BODY_HEALTH};

    let mut app = App::new();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_actor_stimuli);
    let npc = spawn_character_npc(&mut app, &npc_cast(Some(false), None));

    assert_eq!(
        app.world().get::<BodyHealth>(npc).unwrap().max(),
        DEFAULT_UNAUTHORED_BODY_HEALTH,
        "a body nobody described is as tough as any other body nobody described \
         — being peaceful is not a claim about its toughness"
    );

    app.world_mut().write_message(ActorStimulus::Challenged {
        actor: npc,
        challenger: None,
    });
    app.update();

    assert_eq!(
        *app.world().get::<ActorDisposition>(npc).unwrap(),
        ActorDisposition::Hostile,
        "the provocation must actually land"
    );
    assert_eq!(
        app.world().get::<BodyHealth>(npc).unwrap().max(),
        DEFAULT_UNAUTHORED_BODY_HEALTH,
        "and it is the same pool afterwards, from construction rather than from \
         the flip — which is the whole of D101's health half"
    );
}

#[test]
fn an_un_challenged_passive_npc_ignores_damage() {
    // Symmetric negative: without the explicit challenge, a passive
    // actor stays peaceful when merely damaged — only the challenge (or
    // crossing the retaliation threshold) arms the fight.
    let mut app = App::new();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_actor_stimuli);
    let npc = spawn_npc_with_strikes(&mut app, 0);
    // Force passive so DamagedBy is a no-op.
    app.world_mut()
        .get_mut::<ActorAggression>(npc)
        .unwrap()
        .mode = AggressionMode::Passive;
    run(&mut app, npc);
    assert_eq!(
        *app.world().get::<ActorDisposition>(npc).unwrap(),
        ActorDisposition::Peaceful,
        "a passive, un-challenged NPC stays peaceful under damage"
    );
}
