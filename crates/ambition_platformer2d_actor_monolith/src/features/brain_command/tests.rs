//! Behavior tests for runtime brain switching: preset replacement, authored-home
//! restoration, deterministic replay, target isolation, and mount ownership.

use super::*;
use ambition_characters::actor::character_catalog::{
    parse_catalog, AuthoredBrainContext, AutonomousSource, BrainBinding, BrainPresetId,
    CharacterCatalog,
};
use ambition_characters::actor::ActorPose;
use ambition_characters::brain::{Brain, StateMachineCfg};
use ambition_characters::control::{DrivingParticipant, PlayerSlot};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::ecs::message::Messages;

const CATALOG: &str = r#"(
    brain_presets: {
        "stand_still": StandStill,
        "wanderer_puppy_slug": Wanderer(speed: 36.0, aggressiveness: 0.0),
        "melee_brute_striker": MeleeBrute(
            aggressiveness: 1.0, aggro_radius: 220.0, attack_range: 36.0, chase_speed: 110.0,
        ),
        "patrol_peaceful": Patrol(
            spawn_local_x: 0.0, radius: 64.0, speed: 28.0,
            aggressiveness: 0.0, aggro_radius: 80.0, attack_range: 0.0,
        ),
    },
    action_set_presets: { "peaceful": (move_style: Walk) },
    characters: {
        "npc_puppy_slug": (
            display_name: "Puppy Slug", spritesheet: "x.png", manifest: "x_spritesheet.ron",
            tier: MainHall, body_kind: Crawler, composition: None,
            default_brain: "wanderer_puppy_slug", default_action_set: "peaceful", tags: [],
        ),
        "npc_patroller": (
            display_name: "Patroller", spritesheet: "x.png", manifest: "x_spritesheet.ron",
            tier: MainHall, body_kind: Standard, composition: None,
            default_brain: "patrol_peaceful", default_action_set: "peaceful", tags: [],
        ),
    },
)"#;

fn catalog() -> CharacterCatalog {
    CharacterCatalog::from_data(parse_catalog(CATALOG))
}

/// Minimal App with the `BrainCommand` channel + its reducer on `Update`.
fn app() -> App {
    let mut app = App::new();
    app.add_message::<BrainCommand>();
    app.insert_resource(catalog());
    app.add_systems(Update, apply_brain_commands);
    app
}

fn send(app: &mut App, cmd: BrainCommand) {
    app.world_mut()
        .resource_mut::<Messages<BrainCommand>>()
        .write(cmd);
}

/// Spawn a catalog NPC carrying its default brain, binding, and authored context.
fn spawn_npc(app: &mut App, sim: &str, character_id: &str, anchor_x: f32) -> Entity {
    let cat = catalog();
    let (binding, brain) = ambition_characters::actor::character_catalog::resolve_initial_brain(
        &cat,
        character_id,
        None,
        // No definition-authored default: this fixture is about the catalog path.
        &ambition_characters::actor::character_catalog::BrainBuildContext::at(anchor_x),
    )
    .expect("catalog default resolves");
    app.world_mut()
        .spawn((
            SimId::placement(sim),
            brain,
            binding,
            AuthoredBrainContext::from_placement(anchor_x, 0.0),
            ActorPose::from_parts(ae::Vec2::new(anchor_x, 0.0), ae::Vec2::new(8.0, 8.0), 1.0),
        ))
        .id()
}

/// #5 — `UsePreset` replaces the active brain with the requested preset and
/// records the override in the binding.
#[test]
fn use_preset_replaces_the_live_brain() {
    let mut app = app();
    let e = spawn_npc(&mut app, "puppy", "npc_puppy_slug", 100.0);
    assert_eq!(app.world().get::<Brain>(e).unwrap().label(), "wanderer");

    send(
        &mut app,
        BrainCommand::use_preset(SimId::placement("puppy"), "melee_brute_striker"),
    );
    app.update();

    assert_eq!(app.world().get::<Brain>(e).unwrap().label(), "melee_brute");
    assert_eq!(
        app.world().get::<BrainBinding>(e).unwrap().source,
        AutonomousSource::CatalogPreset(BrainPresetId::new("melee_brute_striker")),
    );
}

/// #4 — `RestoreDefault` rebuilds a FRESH default brain and clears the override.
#[test]
fn restore_default_rebuilds_a_fresh_default_brain() {
    let mut app = app();
    let e = spawn_npc(&mut app, "puppy", "npc_puppy_slug", 100.0);

    send(
        &mut app,
        BrainCommand::use_preset(SimId::placement("puppy"), "stand_still"),
    );
    app.update();
    assert_eq!(app.world().get::<Brain>(e).unwrap().label(), "stand_still");

    send(
        &mut app,
        BrainCommand::restore_default(SimId::placement("puppy")),
    );
    app.update();

    assert_eq!(app.world().get::<Brain>(e).unwrap().label(), "wanderer");
    assert_eq!(
        app.world().get::<BrainBinding>(e).unwrap().source,
        AutonomousSource::CatalogDefault,
    );
}

/// A `RestoreDefault` rebuilds a patrol brain around its AUTHORED home, not the
/// actor's current pose — the fix for a patroller re-centering wherever it walked.
#[test]
fn restore_default_uses_the_authored_home_not_the_current_pose() {
    let mut app = app();
    let e = spawn_npc(&mut app, "wanderer", "npc_patroller", 100.0);
    // The patroller wandered far from home.
    app.world_mut().get_mut::<ActorPose>(e).unwrap().center.x = 900.0;

    send(
        &mut app,
        BrainCommand::restore_default(SimId::placement("wanderer")),
    );
    app.update();

    match app.world().get::<Brain>(e).unwrap() {
        Brain::StateMachine(StateMachineCfg::Patrol { cfg, .. }) => {
            assert_eq!(
                cfg.lane.center_x, 100.0,
                "the rebuilt patrol lane centers on the AUTHORED anchor, not the current pose"
            );
        }
        other => panic!("expected a Patrol brain, got {other:?}"),
    }
}

/// #14 — the same command replays deterministically.
#[test]
fn a_brain_switch_replays_deterministically() {
    let switch = || {
        let mut app = app();
        let e = spawn_npc(&mut app, "puppy", "npc_puppy_slug", 100.0);
        send(
            &mut app,
            BrainCommand::use_preset(SimId::placement("puppy"), "melee_brute_striker"),
        );
        app.update();
        (
            app.world().get::<Brain>(e).unwrap().label().to_string(),
            app.world().get::<BrainBinding>(e).unwrap().source.clone(),
        )
    };
    assert_eq!(switch(), switch());
}

/// A command targeting a different SimId leaves this actor untouched.
#[test]
fn a_command_only_touches_its_target() {
    let mut app = app();
    let e = spawn_npc(&mut app, "puppy", "npc_puppy_slug", 100.0);
    send(
        &mut app,
        BrainCommand::use_preset(SimId::placement("someone_else"), "stand_still"),
    );
    app.update();
    assert_eq!(app.world().get::<Brain>(e).unwrap().label(), "wanderer");
}

/// An unknown preset is rejected (binding + brain unchanged) — never a silent
/// fall back to the default or StandStill.
#[test]
fn an_unknown_preset_is_rejected() {
    let mut app = app();
    let e = spawn_npc(&mut app, "puppy", "npc_puppy_slug", 100.0);
    send(
        &mut app,
        BrainCommand::use_preset(SimId::placement("puppy"), "no_such_preset"),
    );
    app.update();
    assert_eq!(
        app.world().get::<Brain>(e).unwrap().label(),
        "wanderer",
        "an unknown preset leaves the live brain unchanged"
    );
    assert_eq!(
        app.world().get::<BrainBinding>(e).unwrap().source,
        AutonomousSource::CatalogDefault,
        "an unknown preset leaves the binding unchanged"
    );
}

/// A brain command during POSSESSION applies LIVE, because possession
/// displaces nothing.
///
/// A driven body keeps its own policy now, so the live brain IS the autonomous selection and there
/// is nothing to defer.
#[test]
fn a_driven_body_applies_a_brain_command_live_because_nothing_displaced_its_policy() {
    let mut app = app();
    let binding = BrainBinding::new(
        BrainPresetId::new("wanderer_puppy_slug"),
        AutonomousSource::CatalogDefault,
    );
    let e = app
        .world_mut()
        .spawn((
            SimId::placement("possessed"),
            // Its OWN policy, plus the seat somebody is driving it from.
            Brain::StateMachine(StateMachineCfg::Wanderer {
                cfg: ambition_characters::brain::WandererCfg {
                    speed: 36.0,
                    aggressiveness: 0.0,
                },
            }),
            DrivingParticipant(PlayerSlot::PRIMARY),
            binding,
            AuthoredBrainContext::from_placement(100.0, 0.0),
            ActorPose::from_parts(ae::Vec2::new(100.0, 0.0), ae::Vec2::new(8.0, 8.0), 1.0),
        ))
        .id();

    send(
        &mut app,
        BrainCommand::use_preset(SimId::placement("possessed"), "stand_still"),
    );
    app.update();

    assert_eq!(
        app.world().get::<Brain>(e).unwrap().label(),
        "stand_still",
        "the switch was deferred on a body whose policy nothing displaced — the \
         release would resume a mind that is not the one the command selected"
    );
    assert_eq!(
        app.world().get::<BrainBinding>(e).unwrap().source,
        AutonomousSource::CatalogPreset(BrainPresetId::new("stand_still")),
        "the command updates the autonomous source that resumes on release — never lost"
    );
    assert_eq!(
        app.world().get::<DrivingParticipant>(e).map(|d| d.0),
        Some(PlayerSlot::PRIMARY),
        "a brain command took somebody's seat away"
    );
}

/// A brain command during a MOUNT does not disturb the live (mounted) brain, but
/// it updates the autonomous SOURCE that resumes on dismount — not lost.
#[test]
fn a_mounted_body_updates_its_source_not_its_control() {
    let mut app = app();
    let e = spawn_npc(&mut app, "rider", "npc_puppy_slug", 100.0);
    app.world_mut()
        .entity_mut(e)
        .insert(crate::features::ecs::Mounted);

    send(
        &mut app,
        BrainCommand::use_preset(SimId::placement("rider"), "stand_still"),
    );
    app.update();

    assert_eq!(
        app.world().get::<Brain>(e).unwrap().label(),
        "wanderer",
        "a mounted body's live brain is not switched while it rides"
    );
    assert_eq!(
        app.world().get::<BrainBinding>(e).unwrap().source,
        AutonomousSource::CatalogPreset(BrainPresetId::new("stand_still")),
        "the command updates the source that resumes on dismount — never lost"
    );
}

/// "You are free" (`ReleaseProvocation`) invokes BOTH authorities atomically: it
/// pacifies the actor (peaceful disposition, passive aggression, grudge + target
/// cleared) AND restores the catalog-default source + the live default brain —
/// even though the actor was provoked onto an override brain with a live grudge.
/// A bare `RestoreDefault` would revert only the brain, leaving it hostile.
#[test]
fn release_provocation_pacifies_and_restores_default() {
    use crate::features::{ActorAggression, ActorDisposition, ReleaseProvocation};

    let mut app = App::new();
    app.add_message::<BrainCommand>();
    app.add_message::<ReleaseProvocation>();
    app.insert_resource(catalog());
    app.init_resource::<crate::abilities::traversal::possession::PossessionState>();
    app.add_systems(
        Update,
        (
            apply_release_provocations.before(apply_brain_commands),
            apply_brain_commands,
        ),
    );

    let e = spawn_npc(&mut app, "hall_npc", "npc_puppy_slug", 100.0);
    // Simulate a provoked actor: hostile disposition, a live grudge + target, and
    // (below) an override onto a hostile brain.
    let foe = app.world_mut().spawn_empty().id();
    {
        let mut aggr = ActorAggression::hostile();
        aggr.target = Some(foe);
        aggr.grudge = Some(foe);
        aggr.strikes = 5;
        app.world_mut()
            .entity_mut(e)
            .insert((ActorDisposition::Hostile, aggr));
    }
    send(
        &mut app,
        BrainCommand::use_preset(SimId::placement("hall_npc"), "melee_brute_striker"),
    );
    app.update();
    assert_eq!(app.world().get::<Brain>(e).unwrap().label(), "melee_brute");

    // "You are free".
    app.world_mut()
        .resource_mut::<Messages<ReleaseProvocation>>()
        .write(ReleaseProvocation::new(SimId::placement("hall_npc")));
    app.update();

    assert!(
        app.world()
            .get::<ActorDisposition>(e)
            .unwrap()
            .is_peaceful(),
        "the freed actor is peaceful again"
    );
    let aggr = app.world().get::<ActorAggression>(e).unwrap();
    assert!(!aggr.is_aggressive(), "aggression is pacified to passive");
    assert_eq!(aggr.grudge, None, "the grudge is cleared");
    assert_eq!(aggr.target, None, "the combat target is cleared");
    assert_eq!(
        app.world().get::<BrainBinding>(e).unwrap().source,
        AutonomousSource::CatalogDefault,
        "the catalog-default autonomous source is restored"
    );
    assert_eq!(
        app.world().get::<Brain>(e).unwrap().label(),
        "wanderer",
        "the live default brain is restored (not the hostile override)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// The character-first road: a body whose autonomous default is its own
// character's authored policy rather than a catalog preset.
//
// every fixture above is preset- or catalog-default-based, and that is why
// the whole `CharacterProfile` seam could be wrong in three places at once and
// stay green. A vocabulary with no fixture is a vocabulary with no tests.
// ─────────────────────────────────────────────────────────────────────────────

/// The policy the character AUTHORS — what it does when nobody has provoked it.
fn character_policy() -> ambition_characters::brain::BrainProfile {
    ambition_characters::brain::BrainProfile {
        template: ambition_characters::brain::CharacterBrainTemplate::Wanderer,
        ..Default::default()
    }
}

/// The policy PROVOCATION installs — deliberately a different template, so the
/// two are told apart by `Brain::label()` and not by a float comparison.
///
/// AND DELIBERATELY NOT A `MeleeBrute`, which is what `BrainProfile`'s
/// own `Default` is. A provoked fixture built from the default template makes
/// "the release left the provoked policy in place" and "the release zeroed the
/// policy to one nobody authored" print the identical failure — two different
/// bugs wearing one message. Three distinct templates keep the three outcomes
/// distinguishable.
fn provoked_policy() -> ambition_characters::brain::BrainProfile {
    ambition_characters::brain::BrainProfile {
        template: ambition_characters::brain::CharacterBrainTemplate::Skirmisher,
        aggro_radius: 220.0,
        attack_range: 36.0,
        ..Default::default()
    }
}

/// An app whose world carries a real prepared cast, published through the
/// production registration seam (`register_character` + `finalize`) — not a
/// hand-built registry, so the identity lookup under test is the one production
/// performs.
fn app_with_cast() -> App {
    let mut app = app();
    use crate::character_runtime::CharacterDefinitionAppExt;
    app.register_character(
        crate::character_runtime::CharacterDefinition::new(
            "npc_villager",
            "Villager",
            "brain_command_tests",
        )
        .with_autonomous_profile(character_policy()),
    );
    ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
    app
}

fn character_first_config(brain_profile: ambition_characters::brain::BrainProfile) -> ActorConfig {
    ActorConfig {
        id: "villager".into(),
        name: "Villager".into(),
        tuning: crate::features::ecs::actor_tuning::ActorTuning {
            // deliberately NOT the generic peaceful seed
            // (`max_run_speed: MAX_RUN_SPEED`): this is the body its character
            // built, and a controller change must leave it alone. the pool
            // that stood beside it left with `ActorTuning::max_health` (AC6.2)
            // — it is `BodyHealth`'s, and the entity below carries one.
            max_run_speed: 91.0,
            ..Default::default()
        },
        brain_profile,
        brain: ambition_entity_catalog::placements::CharacterBrain::Passive,
        sprite_override_npc_name: None,
        sprite_character_id: Some("npc_villager".into()),
        // A fixture body, not a seated CPU twin.
        preserves_mirror_symmetry: false,
    }
}

/// A character-first body mid-PROVOCATION: its live policy is the provoked one
/// and its binding says so, while its DEFAULT is still "ask my character".
fn spawn_provoked_character_first(app: &mut App, sim: &str) -> Entity {
    let binding = BrainBinding {
        default_preset:
            ambition_characters::actor::character_catalog::AutonomousDefault::CharacterProfile,
        source: AutonomousSource::ProvokedProfile {
            profile: ambition_entity_catalog::BrainProfileId::new("brain_command_tests::angry"),
        },
    };
    app.world_mut()
        .spawn((
            SimId::placement(sim),
            // The live mind matches the live policy: this body IS fighting.
            crate::features::ecs::character_policy::brain_from_profile(
                &character_first_config(provoked_policy()),
                provoked_policy(),
                Default::default(),
            ),
            binding,
            AuthoredBrainContext::from_placement(0.0, 0.0),
            ActorPose::from_parts(ae::Vec2::ZERO, ae::Vec2::splat(8.0), 1.0),
            character_first_config(provoked_policy()),
            ambition_characters::actor::WornCharacter::new("npc_villager"),
        ))
        .id()
}

/// "YOU ARE FREE" MUST MEAN IT.
#[test]
fn a_released_character_returns_to_its_own_policy_not_the_provoked_one() {
    let mut app = app_with_cast();
    let e = spawn_provoked_character_first(&mut app, "villager");

    // If these two were equal the test would pass without the identity lookup existing at all,
    // and would be proving nothing.
    assert_eq!(
        app.world().get::<Brain>(e).unwrap().label(),
        "skirmisher",
        "precondition: the provoked body is running the provoked mind"
    );
    assert_ne!(
        app.world()
            .get::<ActorConfig>(e)
            .unwrap()
            .brain_profile
            .template,
        character_policy().template,
        "precondition: the body's CURRENT policy is not its character's — \
         the whole point is that reading the field gives the wrong answer"
    );

    send(
        &mut app,
        BrainCommand::restore_default(SimId::placement("villager")),
    );
    app.update();

    assert_eq!(
        app.world().get::<Brain>(e).unwrap().label(),
        "wanderer",
        "the released body is driven by the policy its CHARACTER authors"
    );
    assert_eq!(
        app.world().get::<BrainBinding>(e).unwrap().source,
        AutonomousSource::CharacterProfile,
    );
    assert_eq!(
        app.world()
            .get::<ActorConfig>(e)
            .unwrap()
            .brain_profile
            .template,
        character_policy().template,
        "and the live policy field agrees with the mind, rather than being \
         left holding the provoked one or zeroed to a default nobody authored"
    );
}

/// A CONTROLLER CHANGE IS NOT A BODY CHANGE.
///
/// `apply_catalog_mode` reconstructs the generic peaceful-NPC seed —
/// `MAX_RUN_SPEED`, default capabilities. That is right for a catalog NPC whose
/// body IS that seed, and over a character-authored body it is a silent
/// downgrade wearing a release.
#[test]
fn releasing_a_character_first_body_leaves_the_body_its_character_built() {
    let mut app = app_with_cast();
    let e = spawn_provoked_character_first(&mut app, "villager");
    app.world_mut().entity_mut(e).insert(CombatKit::default());

    send(
        &mut app,
        BrainCommand::restore_default(SimId::placement("villager")),
    );
    app.update();

    // THE POOL HALF OF THIS TEST IS NOW STRUCTURAL (AC6.2). `ActorTuning` states no pool
    // now, so a controller change has nothing to downgrade: the run speed below is the
    // surviving number this road can still get wrong.
    let tuning = &app.world().get::<ActorConfig>(e).unwrap().tuning;
    assert_eq!(
        tuning.max_run_speed, 91.0,
        "and its character's top speed — a released villager does not come \
         back a generic stroller"
    );
}

/// A RELEASE THAT ARRIVES DURING POSSESSION IS STILL A RELEASE.
///
/// `resolve_command_preset` answers *not mine* for a character-first
/// default, and `update_source_only` read that `None` as *unresolvable* and
/// dropped the command. So provoke → possess → release-provocation → release
/// possession resumed the PROVOKED policy: the release was swallowed by the
/// exact state that was supposed to survive it.
#[test]
fn a_release_during_temporary_control_still_changes_the_source() {
    let mut app = app_with_cast();
    let e = spawn_provoked_character_first(&mut app, "villager");
    app.world_mut()
        .entity_mut(e)
        .insert(DrivingParticipant(PlayerSlot::PRIMARY));

    send(
        &mut app,
        BrainCommand::restore_default(SimId::placement("villager")),
    );
    app.update();

    assert_eq!(
        app.world().get::<BrainBinding>(e).unwrap().source,
        AutonomousSource::CharacterProfile,
        "the source a release resumes into is the character's own policy"
    );
    assert_eq!(
        app.world().get::<DrivingParticipant>(e).map(|d| d.0),
        Some(PlayerSlot::PRIMARY),
        "and the SEAT is untouched — a brain command decides what a body does on \
         its own, never who is driving it"
    );
}

/// A CHARACTER-FIRST DEFAULT THAT CANNOT BE RESOLVED IS REJECTED, NOT
/// COVERED FOR.
///
/// `ActorConfig::brain_profile` is the policy the body is running NOW and provocation writes it, so
/// on a body whose `WornCharacter` or prepared cast went missing, "ask the character, and otherwise
/// trust whatever mind is installed" restores the PROVOKED policy and labels it the character's
/// own. Silently. Forever.
///
/// a binding saying `default = CharacterProfile` is a CLAIM that the
/// character can answer. When it cannot, the composition is broken, and the
/// command gets the same answer an unknown preset gets.
///
/// the fixture removes the identity rather than the cast, because that is the
/// half a composition can lose without noticing: a body spawned by a road that
/// forgot to attach `WornCharacter` still has a perfectly good registry sitting
/// beside it.
#[test]
fn a_character_first_default_that_cannot_be_resolved_is_rejected() {
    let mut app = app_with_cast();
    let e = spawn_provoked_character_first(&mut app, "villager");
    app.world_mut()
        .entity_mut(e)
        .remove::<ambition_characters::actor::WornCharacter>();

    send(
        &mut app,
        BrainCommand::restore_default(SimId::placement("villager")),
    );
    app.update();

    assert_eq!(
        app.world().get::<Brain>(e).unwrap().label(),
        "skirmisher",
        "the command was rejected, so nothing changed — the body is still \
         running the mind it had. A fallback would have rebuilt that same \
         provoked mind and CALLED it the character's default, which is the \
         failure this rejection exists to make visible"
    );
    assert!(
        matches!(
            app.world().get::<BrainBinding>(e).unwrap().source,
            AutonomousSource::ProvokedProfile { .. }
        ),
        "and the binding still says what is true: this body is provoked"
    );
}
