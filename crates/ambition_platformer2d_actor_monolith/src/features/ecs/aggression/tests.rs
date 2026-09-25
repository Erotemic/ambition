use super::*;
use ambition_platformer2d_actor_spawn::npc_policy::NPC_HOSTILE_STRIKE_THRESHOLD;
use ambition_combat::components::{CenteredAabb, FeatureId};
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use bevy::prelude::{App, Update};

/// A peaceful NPC wearing a character that resolves a provoked policy, with
/// `strikes` already counted toward its threshold.
fn spawn_npc_with_strikes(app: &mut App, strikes: i32) -> bevy::prelude::Entity {
    let npc = spawn_character_npc(app, &npc_cast(Some(false), None));
    app.world_mut()
        .get_mut::<ActorAggression>(npc)
        .expect("a spawned NPC carries its aggression")
        .strikes = strikes;
    npc
}

/// A peaceful NPC nobody authored: no character, so no provoked policy.
fn spawn_anonymous_npc(app: &mut App) -> bevy::prelude::Entity {
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
    let (seed, _render) = ambition_body_seed::ActorClusterSeed::new_peaceful_npc(
        "alice",
        "Alice",
        aabb,
        &interactable,
        &[],
    );
    spawn_actor_from_seed(app, seed, "alice", aabb, interactable, 0)
}

/// NO POLICY, NO PROVOCATION. A body whose character and provider state no
/// provoked policy is not handed an engine-invented fighter when struck; it
/// stays as it was.
#[test]
fn a_body_with_no_provoked_policy_cannot_be_provoked() {
    let mut app = App::new();
    app.add_message::<ActorStimulus>();
    app.add_message::<crate::features::NpcProvocationChanged>();
    app.add_systems(Update, apply_actor_stimuli);
    let npc = spawn_anonymous_npc(&mut app);
    let control = spawn_character_npc(&mut app, &npc_cast(Some(false), None));
    for body in [npc, control] {
        app.world_mut().write_message(ActorStimulus::Challenged {
            actor: body,
            challenger: None,
        });
    }
    app.update();
    assert_eq!(
        *app.world().get::<ActorDisposition>(control).unwrap(),
        ActorDisposition::Hostile,
        "the control: a body with a resolved policy is provoked by the same stimulus"
    );
    assert_eq!(
        *app.world().get::<ActorDisposition>(npc).unwrap(),
        ActorDisposition::Peaceful,
        "an anonymous body was provoked into a policy nobody declared"
    );
}

/// The spawn half of [`spawn_npc_with_strikes`], shared with the flight fixture
/// below so both bodies reach the world through the same components.
fn spawn_actor_from_seed(
    app: &mut App,
    seed: ambition_body_seed::ActorClusterSeed,
    id: &str,
    aabb: ae::Aabb,
    interactable: ambition_interaction::Interactable,
    strikes: i32,
) -> bevy::prelude::Entity {
    let (disposition, combat) =
        ambition_platformer2d_actor_spawn::conversion::actor_component_snapshot(&seed, ActorDisposition::Peaceful);
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
            ambition_characters::brain::action_set::IdentityKit::default(),
            seed.into_components(),
            ActorInteraction {
                interactable,
            },
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
    app.add_message::<crate::features::NpcProvocationChanged>();
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
        Some(ambition_combat::components::Grudge::Body(attacker)),
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
    app.add_message::<crate::features::NpcProvocationChanged>();
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
    app.add_message::<crate::features::NpcProvocationChanged>();
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
    app.add_message::<crate::features::NpcProvocationChanged>();
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
    app.add_message::<crate::features::NpcProvocationChanged>();
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
) -> ambition_characters::prepared::PreparedCharacterRegistry {
    let mut registry = ambition_characters::prepared::PreparedCharacterRegistry::default();
    let mut definition = ambition_characters::actor::definition::CharacterDefinition::new(
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
    let mut prepared = crate::character_runtime::prepare_and_finalize_for_test(
        definition,
        &ambition_characters::prepared::CharacterBindings::default(),
    )
    .prepared;
    // What preparation resolves from a provider's declared provoked profile:
    // without one the character is not provokable at all.
    prepared.provoked_profile = Some(ambition_characters::brain::BrainProfile {
        template: ambition_characters::brain::CharacterBrainTemplate::Smash,
        aggro_radius: 460.0,
        attack_range: 150.0,
        ..Default::default()
    });
    prepared.provoked_profile_id = Some(ambition_entity_catalog::BrainProfileId::new(
        "test::combatant",
    ));
    registry.insert_prepared(prepared);
    registry
}

/// A peaceful NPC placement naming `npc_test_parrot`, built through the
/// production character-first seed so the body is the one the game constructs.
fn spawn_character_npc(
    app: &mut App,
    cast: &ambition_characters::prepared::PreparedCharacterRegistry,
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
    // The provocation reads the published cast and the worn character, so the
    // fixture publishes one and wears the other.
    app.insert_resource(cast.clone());
    let (seed, _render) = ambition_body_seed::ActorClusterSeed::new_peaceful_npc_in(
        &Default::default(),
        &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        Some(cast),
        "parrot",
        "Parrot",
        aabb,
        &interactable,
        &[],
    );
    let npc = spawn_actor_from_seed(app, seed, "parrot", aabb, interactable, 0);
    app.world_mut()
        .entity_mut(npc)
        .insert(ambition_characters::actor::WornCharacter::new("npc_test_parrot"));
    npc
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
/// (`smash_cfg_from_spec`), which `ActorBody::from_abilities` sets for an aerial body.
///
/// The brain read that half-body as aerial (`gravity_scale <= 0.001 || fly_enabled`) while the
/// integrator read it as grounded (`fly_enabled` alone) — so it really did freeze, from the
/// fixture's own disagreement rather than from provocation.
#[test]
fn a_flying_npc_stays_flying_when_it_is_provoked() {
    use ambition_characters::brain::{Brain, StateMachineCfg};
    use ambition_platformer2d_core::ActorSurfaceState;

    let mut app = App::new();
    app.add_message::<ActorStimulus>();
    app.add_message::<crate::features::NpcProvocationChanged>();
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
    app.add_message::<crate::features::NpcProvocationChanged>();
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

/// PROVOCATION NEVER CHANGES WHO DRIVES A BODY, BUT IT DOES CHANGE THE
/// AUTONOMOUS POLICY WAITING UNDER THE DRIVER.
///
/// `DrivingParticipant` suppresses the brain's execution; it does not own the
/// `Brain`, and releasing control only removes the driver. So a body provoked
/// while driven must carry the provoked mind already, or the old peaceful one
/// resumes on release under a hostile disposition. The repertoire is untouched
/// either way: it is a projection of identity, worn equipment and the hand.
#[test]
fn provoking_a_driven_body_changes_its_mind_and_not_its_driver() {
    use ambition_characters::actor::character_catalog::{
        AutonomousSource, BrainBinding, BrainPresetId,
    };
    use ambition_characters::brain::{ActionSet, Brain, StateMachineCfg};
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};

    let mut app = App::new();
    app.add_message::<ActorStimulus>();
    app.add_message::<crate::features::NpcProvocationChanged>();
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
    // A repertoire nothing about this body's identity would produce, so an
    // overwrite from the identity baseline is visible as itself. It marks the
    // SPECIAL slot rather than an attack slot: the provoked brain choice reads
    // melee/ranged, and a fixture that changed which brain is installed would
    // be measuring the poison below instead of the overwrite.
    let carried = ActionSet {
        special: Some(ambition_characters::brain::SpecialActionSpec::Special(
            "carried_marker".to_string(),
        )),
        ..ActionSet::default()
    };
    for body in [driven, free] {
        app.world_mut().entity_mut(body).insert(carried.clone());
    }

    for body in [driven, free] {
        app.world_mut().write_message(ActorStimulus::Challenged {
            actor: body,
            challenger: None,
        });
    }
    app.update();

    // The control: the same stimulus on an undriven body installs the mind, so
    // the driven assertion below measures the driver and not a broken provoke.
    assert!(
        matches!(
            app.world().get::<Brain>(free),
            Some(Brain::StateMachine(StateMachineCfg::Smash { .. }))
        ),
        "an undriven body must receive the provoked mind"
    );

    assert_eq!(
        app.world().get::<DrivingParticipant>(driven).map(|d| d.0),
        Some(PlayerSlot::PRIMARY),
        "a body under player control must still be under player control — \
         provocation changes what a body IS, never who drives it"
    );
    assert_eq!(
        app.world().get::<Brain>(driven).map(Brain::label),
        app.world().get::<Brain>(free).map(Brain::label),
        "the driven body kept its peaceful mind, so releasing control resumes \
         a policy its hostile disposition contradicts"
    );
    assert_eq!(
        *app.world().get::<ActorDisposition>(driven).unwrap(),
        ActorDisposition::Hostile,
        "the relationship still changes; leaving the driver alone is not the \
         same as ignoring the provocation"
    );
    for (body, who) in [(driven, "driven"), (free, "undriven")] {
        assert_eq!(
            app.world().get::<ActionSet>(body),
            Some(&carried),
            "provocation rewrote the {who} body's repertoire; it is a projection \
             of identity, worn equipment and the hand, and a stimulus moves none \
             of them"
        );
    }
    assert_eq!(
        app.world().get::<BrainBinding>(driven).map(|b| &b.source),
        Some(&AutonomousSource::ProvokedProfile {
            profile: ambition_entity_catalog::BrainProfileId::new("test::combatant"),
        }),
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
    app.add_message::<crate::features::NpcProvocationChanged>();
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
    app.add_message::<crate::features::NpcProvocationChanged>();
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

/// ⛔ THE SAVE'S PROVOCATION FACT IS WHAT A ROOM REPLAY BUILDS THE PERSON FROM,
/// so every road that changes the live fact must change the durable one.
///
/// A `<<challenge>>` flipped the NPC hostile live and recorded nothing — only the
/// damage road wrote `npc_<id>_hostile` — so a replay rebuilt the person
/// peaceful. A `<<restore_brain>>` release pacified it live and left the flag
/// set, so a replay rebuilt it hostile. And the flag carries no faction while
/// construction rebuilds `Grudge::Faction(Player)`, so a provocation by anyone
/// else must not set it.
#[test]
fn a_provocation_is_durable_exactly_when_the_player_causes_it_and_a_release_clears_it() {
    use ambition_persistence::save::AmbitionGameSave;
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    fn app_with_npc() -> (App, bevy::prelude::Entity) {
        let mut app = App::new();
        app.add_message::<ActorStimulus>();
        app.add_message::<crate::features::NpcProvocationChanged>();
        app.add_message::<crate::features::ReleaseProvocation>();
        app.add_message::<crate::features::BrainCommand>();
        app.insert_resource(AmbitionGameSave::default());
        app.insert_resource(ambition_persistence::quest::QuestRegistry::default());
        app.add_systems(
            Update,
            (
                apply_actor_stimuli,
                crate::features::apply_release_provocations,
                crate::features::record_npc_provocations,
            )
                .chain(),
        );
        let npc = spawn_npc_with_strikes(&mut app, 0);
        app.world_mut()
            .entity_mut(npc)
            .insert(SimId::placement("parrot"));
        (app, npc)
    }
    fn durable(app: &App) -> bool {
        app.world()
            .resource::<AmbitionGameSave>()
            .data()
            .flag(&crate::fate_flags::npc_flag_id("parrot"))
    }

    // The player challenges: live AND durable.
    let (mut app, npc) = app_with_npc();
    let player = app
        .world_mut()
        .spawn(ambition_combat::components::ActorFaction::Player)
        .id();
    app.world_mut().write_message(ActorStimulus::Challenged {
        actor: npc,
        challenger: Some(player),
    });
    app.update();
    assert_eq!(
        *app.world().get::<ActorDisposition>(npc).unwrap(),
        ActorDisposition::Hostile,
        "premise: the challenge flipped the NPC live"
    );
    assert!(
        durable(&app),
        "a challenged NPC turned hostile live and the save does not record it, \
         so the next room replay rebuilds it peaceful"
    );

    // Released: live AND durable.
    app.world_mut()
        .write_message(crate::features::ReleaseProvocation::new(SimId::placement("parrot")));
    app.update();
    assert_eq!(
        *app.world().get::<ActorDisposition>(npc).unwrap(),
        ActorDisposition::Peaceful,
        "premise: the release pacified the NPC live"
    );
    assert!(
        !durable(&app),
        "a released NPC is peaceful live and the save still records it provoked, \
         so the next room replay rebuilds it hostile"
    );

    // Provoked by somebody who is not the player: live, NOT durable.
    let (mut app, npc) = app_with_npc();
    let bandit = app
        .world_mut()
        .spawn(ambition_combat::components::ActorFaction::Enemy)
        .id();
    app.world_mut().write_message(ActorStimulus::Challenged {
        actor: npc,
        challenger: Some(bandit),
    });
    app.update();
    assert_eq!(
        *app.world().get::<ActorDisposition>(npc).unwrap(),
        ActorDisposition::Hostile,
        "premise: the NPC turned on the bandit live"
    );
    assert!(
        !durable(&app),
        "a grudge against a non-player was persisted as the flag construction \
         rebuilds as a grudge against the PLAYER"
    );
}

/// A cast whose one character AUTHORS its provoked policy — a template the
/// engine default (`Smash`) is not, so the two are told apart by label.
fn provoked_profile_cast() -> (
    ambition_characters::prepared::PreparedCharacterRegistry,
    ambition_characters::brain::BrainProfile,
    ambition_entity_catalog::BrainProfileId,
) {
    let mut registry = ambition_characters::prepared::PreparedCharacterRegistry::default();
    let definition = ambition_characters::actor::definition::CharacterDefinition::new(
        "npc_test_parrot",
        "Test Parrot",
        "test",
    );
    let mut finalized = crate::character_runtime::prepare_and_finalize_for_test(
        definition,
        &ambition_characters::prepared::CharacterBindings::default(),
    );
    let profile = ambition_characters::brain::BrainProfile {
        template: ambition_characters::brain::CharacterBrainTemplate::Skirmisher,
        aggro_radius: 220.0,
        attack_range: 36.0,
        ..Default::default()
    };
    let id = ambition_entity_catalog::BrainProfileId::new("test::angry_parrot");
    finalized.prepared.provoked_profile = Some(profile);
    finalized.prepared.provoked_profile_id = Some(id.clone());
    registry.insert_prepared(finalized.prepared);
    (registry, profile, id)
}

fn authored_provocation_app() -> (
    App,
    bevy::prelude::Entity,
    ambition_characters::brain::BrainProfile,
    ambition_entity_catalog::BrainProfileId,
) {
    let (cast, profile, id) = provoked_profile_cast();
    let mut app = App::new();
    app.add_message::<ActorStimulus>();
    app.add_message::<crate::features::NpcProvocationChanged>();
    app.add_systems(Update, apply_actor_stimuli);
    let npc = spawn_character_npc(&mut app, &cast);
    app.insert_resource(cast);
    app.world_mut().entity_mut(npc).insert((
        ambition_characters::actor::WornCharacter::new("npc_test_parrot"),
        ambition_characters::actor::character_catalog::BrainBinding::from_character_profile(),
    ));
    (app, npc, profile, id)
}

/// A CHARACTER'S OWN PROVOKED POLICY IS INSTALLED WHOLE, the way the engine
/// default is: policy, read-model, mind and the binding that records it.
#[test]
fn an_authored_provocation_installs_the_characters_policy_and_records_it() {
    use ambition_characters::actor::character_catalog::{AutonomousSource, BrainBinding};
    use ambition_characters::brain::Brain;
    let (mut app, npc, profile, id) = authored_provocation_app();

    app.world_mut().write_message(ActorStimulus::Challenged {
        actor: npc,
        challenger: None,
    });
    app.update();

    let world = app.world();
    let config = world.get::<ambition_combat::actor_tuning::ActorConfig>(npc).unwrap();
    let brain = world.get::<Brain>(npc).unwrap();
    assert_eq!(config.brain_profile, profile, "the character's policy is the live one");
    assert_ne!(
        brain.label(),
        "smash",
        "the engine default was installed over the character's own answer"
    );
    assert_eq!(
        world.get::<BrainBinding>(npc).unwrap().source,
        AutonomousSource::ProvokedProfile { profile: id },
    );
}

/// AN ALREADY-HOSTILE BODY IS NOT RE-DERIVED, on the authored road as on the
/// default one: a repeat stimulus rebuilt its brain (zeroing its cadence) and
/// re-recorded a policy.
#[test]
fn a_repeat_stimulus_leaves_an_authored_provocation_alone() {
    use ambition_characters::brain::Brain;
    let (mut app, npc, _, _) = authored_provocation_app();
    app.world_mut().write_message(ActorStimulus::Challenged {
        actor: npc,
        challenger: None,
    });
    app.update();
    // A mind no provocation builds, so any rebuild shows.
    *app.world_mut().get_mut::<Brain>(npc).unwrap() = Brain::stand_still();

    app.world_mut().write_message(ActorStimulus::DamagedBy {
        actor: npc,
        source: None,
        damage: 1,
    });
    app.update();

    assert_eq!(
        app.world().get::<Brain>(npc).unwrap().label(),
        "stand_still",
        "a repeat stimulus rebuilt an already-hostile body's brain"
    );
}

/// A BODY THAT WAS NEVER PEACEFUL IS NOT RECORDED AS PROVOKED INTO A POLICY
/// IT DOES NOT RUN. The authored road wrote `ProvokedProfile` for any stimulus
/// while installing that policy only on a peaceful flip.
#[test]
fn an_already_hostile_body_records_no_policy_it_was_not_given() {
    use ambition_characters::actor::character_catalog::{AutonomousSource, BrainBinding};
    let (mut app, npc, profile, _) = authored_provocation_app();
    *app.world_mut().get_mut::<ActorDisposition>(npc).unwrap() = ActorDisposition::Hostile;

    app.world_mut().write_message(ActorStimulus::Challenged {
        actor: npc,
        challenger: None,
    });
    app.update();

    let world = app.world();
    assert_ne!(world.get::<ambition_combat::actor_tuning::ActorConfig>(npc).unwrap().brain_profile, profile);
    assert_eq!(
        world.get::<BrainBinding>(npc).unwrap().source,
        AutonomousSource::CharacterProfile,
        "the binding claims a provoked policy the body is not running"
    );
}

/// A PROVOCATION CHANGES THE MIND AND NOT THE PLACEMENT'S AUTHORED BRAIN KEY.
///
/// `ActorConfig.brain` is the content label the tag and sprite passes read
/// (`Custom("sandbag")`, a Mary-O snake). Provocation used to overwrite it with
/// a two-value projection of the new `Brain`, erasing the label every reader
/// wanted; the mind is `Brain`'s alone.
#[test]
fn a_provocation_keeps_the_placements_authored_brain_key() {
    let mut app = App::new();
    app.add_message::<ActorStimulus>();
    app.add_message::<crate::features::NpcProvocationChanged>();
    app.add_systems(Update, apply_actor_stimuli);
    let npc = spawn_npc_with_strikes(&mut app, 0);
    let label = ambition_entity_catalog::placements::CharacterBrain::Custom("sandbag".into());
    app.world_mut()
        .get_mut::<ambition_combat::actor_tuning::ActorConfig>(npc)
        .unwrap()
        .brain = label.clone();

    app.world_mut().write_message(ActorStimulus::Challenged {
        actor: npc,
        challenger: None,
    });
    app.update();

    assert_eq!(
        *app.world().get::<ActorDisposition>(npc).unwrap(),
        ActorDisposition::Hostile,
        "premise: the provocation landed"
    );
    assert_eq!(
        app.world()
            .get::<ambition_combat::actor_tuning::ActorConfig>(npc)
            .unwrap()
            .brain,
        label,
        "provocation rewrote the placement's authored brain key"
    );
}
