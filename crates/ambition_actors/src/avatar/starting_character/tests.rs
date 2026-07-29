use super::*;
use ambition_characters::brain::action_set::RangedStyle;

/// A self-contained momentum speedster. Ambition's shipped roster no longer
/// authors one — Sanic's identity belongs to the standalone Sanic experience
/// provider now — but these tests exercise the `SurfaceMomentum` derivation, so
/// the momentum row under test is supplied locally. Its `stand_still`/`peaceful`
/// presets already exist in the real roster, so merging just the character entry
/// keeps it resolvable while leaving the authored `goblin`/`npc_pirate_admiral`
/// kits the kit tests rely on untouched.
const SANIC_MOMENTUM_FIXTURE: &str = r#"(
    brain_presets: { "stand_still": StandStill },
    action_set_presets: { "peaceful": (move_style: Walk) },
    characters: {
        "sanic": (
            display_name: "Sanic",
            spritesheet: "sprites/sanic_spritesheet.png",
            manifest: "sprites/sanic_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            tags: ["playable", "speedster", "demo"],
            momentum: Some((
                ground_accel: 900.0,
                top_speed: 1200.0,
                jump_speed: 700.0,
            )),
        ),
    },
)"#;

fn test_catalog() -> ambition_characters::actor::character_catalog::CharacterCatalog {
    use ambition_characters::actor::character_catalog::parse_catalog;
    let mut data = parse_catalog(include_str!(
        "../../../../../game/ambition_content/assets/data/character_catalog.ron"
    ));
    let fixture = parse_catalog(SANIC_MOMENTUM_FIXTURE);
    if let Some(sanic) = fixture.characters.get("sanic") {
        data.characters.insert("sanic".to_string(), sanic.clone());
    }
    ambition_characters::actor::character_catalog::CharacterCatalog::from_data(data)
}

fn install_test_catalog(app: &mut bevy::prelude::App) {
    app.insert_resource(test_catalog());
}

mod live_refresh;

#[test]
fn default_is_unset_and_is_default() {
    // No override: an empty id routes to the untouched `from_scratch` path.
    // The concrete row is CONTENT's (`effective_id` resolves it at spawn);
    // the engine bakes in no character name.
    let sc = StartingCharacter::default();
    assert!(sc.character_id.is_empty());
    assert!(sc.is_default());
    // `effective_id` resolves to a real catalog row (the content-installed
    // default, or the first row as fallback) — never empty, never a name
    // the ENGINE baked in.
    let eff = sc.effective_id("player");
    assert!(!eff.is_empty());
    assert!(test_catalog().get(eff).is_some());
}

#[test]
fn wearing_sanic_selects_momentum_then_unwearing_selects_axis_swept() {
    // Q16 test (c): wearing a momentum character makes the box ride
    // surfaces; re-wearing a non-momentum character explicitly selects the
    // axis-swept model so stale surface-private state cannot survive the swap.
    // Every movable body carries one policy; absence is never a default.
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    let entity = app.world_mut().spawn_empty().id();
    let catalog = app
        .world()
        .resource::<ambition_characters::actor::character_catalog::CharacterCatalog>()
        .clone();

    // Wear Sanic → SurfaceMomentum inserted with the authored fast profile.
    let mut queue = bevy::ecs::world::CommandQueue::default();
    {
        let mut commands = Commands::new(&mut queue, app.world());
        apply_worn_motion_model(&catalog, &mut commands, entity, "sanic");
    }
    queue.apply(app.world_mut());
    match app.world().get::<MotionModel>(entity) {
        Some(MotionModel::SurfaceMomentum(m)) => {
            assert_eq!(m.params.top_speed, 1200.0, "Sanic's authored top speed");
        }
        other => panic!("expected SurfaceMomentum after wearing Sanic, got {other:?}"),
    }

    // Re-wear the protagonist → an explicit axis-swept model replaces momentum.
    let mut queue = bevy::ecs::world::CommandQueue::default();
    {
        let mut commands = Commands::new(&mut queue, app.world());
        apply_worn_motion_model(&catalog, &mut commands, entity, "player");
    }
    queue.apply(app.world_mut());
    assert!(
        matches!(
            app.world().get::<MotionModel>(entity),
            Some(MotionModel::AxisSwept(_))
        ),
        "unwearing a momentum character installs the explicit axis-swept policy"
    );
}

#[test]
fn non_default_id_is_not_default() {
    assert!(!StartingCharacter::new("goblin").is_default());
}

/// **S1: gameplay configuration is DERIVED from the worn identity, at spawn
/// (Added) and on any later re-wear (Changed).** A body carrying only the
/// `WornCharacter` identity plus the mutable gameplay components has its name
/// and movement identity re-derived by `apply_worn_character_gameplay`.
#[test]
fn gameplay_derives_from_worn_identity_at_add_and_on_change() {
    use crate::combat::moveset::ActorMoveset;
    use ambition_characters::brain::ActionSet;
    use bevy::prelude::*;

    // Pin the installed default so the protagonist branch is deterministic.

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.add_systems(Update, apply_worn_character_gameplay);

    // Spawn wearing the momentum speedster.
    let e = app
        .world_mut()
        .spawn((
            WornCharacter::new("sanic"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            // A worn body is a FULL body: kinematics + the movement clusters
            // (which include the persisted capability set the overlay rebuilds
            // a HostCode / unknown kit from).
            ambition_engine_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_engine_core::BodyClusterScratch::new_with_abilities(
                    ambition_engine_core::Vec2::ZERO,
                    ambition_engine_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();
    app.update();

    // Movement identity (SurfaceMomentum) + name are derived from "sanic".
    assert!(
        matches!(
            app.world().get::<MotionModel>(e),
            Some(MotionModel::SurfaceMomentum(_))
        ),
        "wearing the momentum character derives SurfaceMomentum"
    );
    assert_eq!(
        app.world().get::<Name>(e).unwrap().as_str(),
        "Sanic",
        "the display name is derived from the worn identity"
    );

    // Re-wear the protagonist through the supported path (mutate the
    // identity). Downstream observes the change: the stale momentum model is
    // replaced by the explicit axis-swept policy and the name follows.
    *app.world_mut().get_mut::<WornCharacter>(e).unwrap() = WornCharacter::new("player");
    app.update();
    assert!(
        matches!(
            app.world().get::<MotionModel>(e),
            Some(MotionModel::AxisSwept(_))
        ),
        "re-wearing a non-momentum character installs the axis-swept policy"
    );
    assert_eq!(
        app.world().get::<Name>(e).unwrap().as_str(),
        "Player",
        "the display name follows the new worn identity"
    );
}

#[test]
fn rewearing_an_equivalent_momentum_profile_preserves_live_ride_state() {
    use crate::combat::moveset::ActorMoveset;
    use ambition_characters::brain::ActionSet;
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.add_systems(Update, apply_worn_character_gameplay);
    let entity = app
        .world_mut()
        .spawn((
            WornCharacter::new("sanic"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            ambition_engine_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_engine_core::BodyClusterScratch::new_with_abilities(
                    ambition_engine_core::Vec2::ZERO,
                    ambition_engine_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();
    app.update();

    let expected = ambition_engine_core::SurfaceMotion::Riding {
        on: ambition_engine_core::SurfaceRef::Chain(0),
        s: 123.0,
        v_t: 456.0,
    };
    {
        let mut motion = app
            .world_mut()
            .get_mut::<MotionModel>(entity)
            .expect("Sanic must derive a momentum model");
        let MotionModel::SurfaceMomentum(momentum) = &mut *motion else {
            panic!("Sanic must derive a momentum model");
        };
        momentum.state = expected;
    }

    // Assigning the equivalent identity still creates a real Changed edge. The
    // derive must refresh name/kit without replacing the matching motion model.
    *app.world_mut().get_mut::<WornCharacter>(entity).unwrap() = WornCharacter::new("sanic");
    app.update();

    match app.world().get::<MotionModel>(entity) {
        Some(MotionModel::SurfaceMomentum(momentum)) => {
            assert_eq!(momentum.state, expected);
        }
        other => panic!("expected preserved SurfaceMomentum, got {other:?}"),
    }
}

/// **S1 poison / non-vacuity:** with no change to either `WornCharacter` or
/// `BodyAbilities`, the derive system does not fire, so a hand-set movement model
/// is left untouched. This proves the assertions above are driven by the two
/// `Changed` edges, not by the system running unconditionally every frame.
#[test]
fn derive_system_only_fires_on_identity_or_ability_change() {
    use crate::combat::moveset::ActorMoveset;
    use ambition_characters::brain::ActionSet;
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.add_systems(Update, apply_worn_character_gameplay);
    let e = app
        .world_mut()
        .spawn((
            WornCharacter::new("sanic"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            // A worn body is a FULL body: kinematics + the movement clusters
            // (which include the persisted capability set the overlay rebuilds
            // a HostCode / unknown kit from).
            ambition_engine_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_engine_core::BodyClusterScratch::new_with_abilities(
                    ambition_engine_core::Vec2::ZERO,
                    ambition_engine_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();
    app.update(); // Added → derives SurfaceMomentum for sanic.
    assert!(app.world().get::<MotionModel>(e).is_some());

    // No identity change: subsequent frames must not re-run the wear. Prove it
    // by changing to a different explicit policy and confirming the unchanged
    // derive system leaves it alone. Absence is never used as a policy sentinel.
    app.world_mut().entity_mut(e).insert(MotionModel::AxisSwept(
        crate::features::AxisSweptMotion::default(),
    ));
    app.update();
    assert!(
        matches!(
            app.world().get::<MotionModel>(e),
            Some(MotionModel::AxisSwept(_))
        ),
        "with no identity or ability change the derive system must not re-fire"
    );
}

/// **The full KIT (ActionSet + moveset), not just name/movement, follows a
/// re-wear between two KNOWN characters** — the reviewer-flagged gap. Wearing
/// the pirate gives its authored pistol; re-wearing the goblin replaces it with
/// the goblin's kit, leaving no stale pirate pistol behind.
#[test]
fn worn_kit_fully_follows_a_known_character_rewear() {
    use crate::combat::moveset::ActorMoveset;
    use ambition_characters::brain::{action_set::RangedStyle, ActionSet, RangedActionSpec};
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.add_systems(Update, apply_worn_character_gameplay);
    let e = app
        .world_mut()
        .spawn((
            WornCharacter::new("npc_pirate_admiral"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            // A worn body is a FULL body: kinematics + the movement clusters
            // (which include the persisted capability set the overlay rebuilds
            // a HostCode / unknown kit from).
            ambition_engine_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_engine_core::BodyClusterScratch::new_with_abilities(
                    ambition_engine_core::Vec2::ZERO,
                    ambition_engine_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();
    app.update();
    assert!(
        matches!(
            app.world().get::<ActionSet>(e).unwrap().ranged,
            Some(RangedActionSpec {
                style: RangedStyle::Pistol,
                ..
            })
        ),
        "wearing the pirate derives its authored pistol into the ActionSet"
    );

    // Re-wear a DIFFERENT known character: the kit fully swaps — no stale pistol.
    *app.world_mut().get_mut::<WornCharacter>(e).unwrap() = WornCharacter::new("goblin");
    app.update();
    assert!(
        !matches!(
            app.world().get::<ActionSet>(e).unwrap().ranged,
            Some(RangedActionSpec {
                style: RangedStyle::Pistol,
                ..
            })
        ),
        "re-wearing the goblin replaces the pirate's kit — no stale ActionSet"
    );
    assert_eq!(app.world().get::<Name>(e).unwrap().as_str(), "Goblin");
    // The MOVESET (not just the ActionSet) follows too: the goblin authors a
    // melee, so the derived directional moveset is non-empty — the moveset is
    // rebuilt from the new kit, not left as the pirate's (pistol-only) moveset.
    assert!(
        !app.world()
            .get::<ActorMoveset>(e)
            .unwrap()
            .0
            .moves
            .is_empty(),
        "the goblin's melee derives a non-empty directional moveset"
    );
}

/// **Closed gap (reviewer 2026-07-11):** a runtime re-wear FROM a known
/// character TO a `HostCode` protagonist REBUILDS the code kit deterministically
/// from the body's persisted `BodyAbilities` — it does NOT leave the prior
/// character's kit. The kit is a function of identity + persisted abilities, not
/// of mutation history, so this is also the snapshot-restore contract: restoring
/// `WornCharacter("player")` onto a survivor rebuilds the protagonist kit.
#[test]
fn runtime_rewear_to_a_host_code_protagonist_rebuilds_the_code_kit() {
    use crate::combat::moveset::ActorMoveset;
    use ambition_characters::brain::{
        ActionSet, MeleeActionSpec, RangedActionSpec, SpecialActionSpec,
    };
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.add_systems(Update, apply_worn_character_gameplay);
    let e = app
        .world_mut()
        .spawn((
            WornCharacter::new("npc_pirate_admiral"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            // A worn body is a FULL body: kinematics + the movement clusters
            // (which include the persisted capability set the overlay rebuilds
            // a HostCode / unknown kit from).
            ambition_engine_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_engine_core::BodyClusterScratch::new_with_abilities(
                    ambition_engine_core::Vec2::ZERO,
                    ambition_engine_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();
    app.update();
    assert!(
        matches!(
            app.world().get::<ActionSet>(e).unwrap().ranged,
            Some(RangedActionSpec {
                style: RangedStyle::Pistol,
                ..
            })
        ),
        "wearing the pirate first installs its pistol"
    );

    // Re-wear the HostCode default ("player"): the code kit (Swipe + Bolt +
    // bubble_shield from sandbox_all abilities) is rebuilt — NO stale pistol.
    *app.world_mut().get_mut::<WornCharacter>(e).unwrap() = WornCharacter::new("player");
    app.update();
    assert_eq!(app.world().get::<Name>(e).unwrap().as_str(), "Player");
    let set = app.world().get::<ActionSet>(e).unwrap();
    assert!(
        matches!(set.melee, Some(MeleeActionSpec::Swipe(_))),
        "the rebuilt protagonist kit has its Swipe melee"
    );
    assert!(
        matches!(
            set.ranged,
            Some(RangedActionSpec {
                style: RangedStyle::Bolt,
                ..
            })
        ),
        "the pirate's pistol is gone — the code kit's Bolt is rebuilt"
    );
    assert!(
        matches!(set.special, Some(SpecialActionSpec::Special(_))),
        "the code kit's bubble_shield special is rebuilt"
    );
    assert!(
        app.world()
            .get::<ambition_characters::brain::ChargesProjectiles>(e)
            .is_some(),
        "the host charge capability is rebuilt with the host kit"
    );
    assert!(
        app.world()
            .get::<ambition_projectiles::PlayerProjectileState>(e)
            .is_some(),
        "the per-body host charge state is reconstructed when absent"
    );
}

/// **Unknown ids are deterministic, not stale.** Re-wearing an id the catalog
/// does not know installs a DEFINED fallback (the code kit rebuilt from the
/// body's abilities) and names the body after the id — it never silently keeps
/// the prior character's kit or name.
#[test]
fn runtime_rewear_to_an_unknown_id_is_a_defined_fallback_not_stale_state() {
    use crate::combat::moveset::ActorMoveset;
    use ambition_characters::brain::{ActionSet, MeleeActionSpec, RangedActionSpec};
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.add_systems(Update, apply_worn_character_gameplay);
    let e = app
        .world_mut()
        .spawn((
            WornCharacter::new("npc_pirate_admiral"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            ambition_engine_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_engine_core::BodyClusterScratch::new_with_abilities(
                    ambition_engine_core::Vec2::ZERO,
                    ambition_engine_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();
    app.update();

    *app.world_mut().get_mut::<WornCharacter>(e).unwrap() =
        WornCharacter::new("ghost_not_in_catalog");
    app.update();
    // Name is the id itself (a legible diagnostic), NOT the stale "Pirate Admiral".
    assert_eq!(
        app.world().get::<Name>(e).unwrap().as_str(),
        "ghost_not_in_catalog"
    );
    let set = app.world().get::<ActionSet>(e).unwrap();
    assert!(
        matches!(set.melee, Some(MeleeActionSpec::Swipe(_)))
            && matches!(
                set.ranged,
                Some(RangedActionSpec {
                    style: RangedStyle::Bolt,
                    ..
                })
            ),
        "an unknown id falls back to the defined code kit, not the stale pistol"
    );
}

/// A HostCode row is derived from the body's mutable ability source, so changing
/// that source must refresh the effective kit even when the worn identity does
/// not change. This is the live-dev/progression edge the identity-only filter
/// missed.
#[test]
fn host_code_kit_refreshes_when_body_abilities_change() {
    use crate::combat::moveset::ActorMoveset;
    use ambition_characters::brain::{ActionSet, MeleeActionSpec, RangedActionSpec};
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.add_systems(Update, apply_worn_character_gameplay);
    let entity = app
        .world_mut()
        .spawn((
            WornCharacter::new("player"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            ambition_engine_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_engine_core::BodyClusterScratch::new_with_abilities(
                    ambition_engine_core::Vec2::ZERO,
                    ambition_engine_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();
    app.update();

    let initial = app.world().get::<ActionSet>(entity).unwrap();
    assert!(matches!(initial.melee, Some(MeleeActionSpec::Swipe(_))));
    assert!(matches!(
        initial.ranged,
        Some(RangedActionSpec {
            style: RangedStyle::Bolt,
            ..
        })
    ));
    assert!(initial.special.is_some());

    {
        let mut abilities = app
            .world_mut()
            .get_mut::<crate::actor::BodyAbilities>(entity)
            .unwrap();
        abilities.abilities.attack = false;
        abilities.abilities.pogo = false;
        abilities.abilities.shield = false;
    }
    app.update();

    let refreshed = app.world().get::<ActionSet>(entity).unwrap();
    assert!(
        refreshed.melee.is_none(),
        "Changed<BodyAbilities> removes the now-disabled melee"
    );
    assert!(
        matches!(
            refreshed.ranged,
            Some(RangedActionSpec {
                style: RangedStyle::Bolt,
                ..
            })
        ),
        "an unrelated enabled ability remains in the rebuilt host kit"
    );
    assert!(
        refreshed.special.is_none(),
        "Changed<BodyAbilities> removes the now-disabled bubble shield"
    );
    assert!(
        app.world()
            .get::<ActorMoveset>(entity)
            .unwrap()
            .0
            .moves
            .is_empty(),
        "the derived moveset refreshes with the ActionSet"
    );
}

/// A peaceful authored persona must be peaceful at the final body-control seam,
/// not only in its nominal ActionSet. Legacy player mechanics consume these raw
/// fields directly, bypassing the generic action resolver unless they are gated.
#[test]
fn peaceful_worn_kit_gates_direct_player_combat_verbs() {
    use ambition_characters::actor::control::{ActorControlFrame, ActorFireRequest};
    use ambition_characters::brain::{ActionSet, ActorControl};
    use bevy::prelude::*;

    let mut frame = ActorControlFrame::neutral();
    frame.melee_pressed = true;
    frame.pogo_pressed = true;
    frame.attack_axis = Vec2::new(1.0, -1.0);
    frame.fire = Some(ActorFireRequest::world_space(Vec2::X, 123.0));
    frame.shield_held = true;
    frame.projectile_pressed = true;
    frame.projectile_held = true;
    frame.projectile_released = true;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.add_systems(Update, gate_worn_player_control);
    let entity = app
        .world_mut()
        .spawn((
            crate::actor::PlayerEntity,
            WornCharacter::new("sanic"),
            MotionModel::default(),
            // The gate resolves the body's live authorities through the shared
            // scheme; a peaceful `ActionSet` (no melee/ranged/special) + no
            // moveset yields no combat slots, so the gate strips every combat
            // verb below. `BodyAbilities` supplies the movement slots.
            crate::actor::BodyAbilities::new(ambition_engine_core::AbilitySet::sandbox_all()),
            ActionSet::peaceful(),
            ActorControl(frame),
        ))
        .id();
    app.update();

    let gated = &app.world().get::<ActorControl>(entity).unwrap().0;
    assert!(!gated.melee_pressed);
    assert!(!gated.pogo_pressed);
    assert_eq!(gated.attack_axis, Vec2::ZERO);
    assert!(gated.fire.is_none());
    assert!(!gated.shield_held);
    assert!(!gated.projectile_pressed);
    assert!(!gated.projectile_held);
    assert!(!gated.projectile_released);
}

/// The gate CONSUMES the shared resolver: a body whose scheme puts a
/// `Technique` on the Attack slot has its melee device edge ROUTED into the
/// sanctioned `ResolvedTechniqueEdges` (and the raw verb cleared) — proving the
/// resolver drives both gating AND the content-technique seam. A body with a
/// real melee `Move` keeps its verb.
#[test]
fn gate_routes_a_technique_attack_slot_into_the_sanctioned_edge() {
    use ambition_characters::action_scheme::{ActorTechniques, ResolvedTechniqueEdges};
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::brain::{ActionSet, ActorControl};
    use ambition_entity_catalog::action_scheme::{ActionGate, ActionId, ActionSpec, ControlSlot};
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.add_systems(Update, gate_worn_player_control);

    // A Sanic-shaped body: movement abilities, empty ActionSet, and a spin_dash
    // technique claiming the Attack slot. Pressing melee this tick.
    let mut frame = ActorControlFrame::neutral();
    frame.melee_pressed = true;
    let spin = ActionSpec {
        id: ActionId::new("spin_dash"),
        slot: ControlSlot::Attack,
        display_name: Some("Spin Dash".to_owned()),
        visual: None,
        gate: ActionGate::Technique("spin_dash".to_owned()),
    };
    let body = app
        .world_mut()
        .spawn((
            crate::actor::PlayerEntity,
            WornCharacter::new("sanic"),
            crate::actor::BodyAbilities::new(ambition_engine_core::AbilitySet::sandbox_all()),
            ActionSet::peaceful(),
            ActorTechniques(vec![spin]),
            ResolvedTechniqueEdges::default(),
            ActorControl(frame),
        ))
        .id();
    app.update();

    let control = &app.world().get::<ActorControl>(body).unwrap().0;
    assert!(
        !control.melee_pressed,
        "the raw melee verb is cleared — no longer the content API"
    );
    let edges = app.world().get::<ResolvedTechniqueEdges>(body).unwrap();
    assert!(
        edges.pressed("spin_dash"),
        "the Attack device edge is routed into the sanctioned spin_dash technique edge"
    );
}

/// A typo in a known Authored row is content corruption, not permission to gain
/// the host protagonist's code kit. Validation reports the bad row; the runtime
/// fallback is deliberately inert.
#[test]
fn malformed_authored_resolution_is_safe_peaceful_not_host_code() {
    use ambition_characters::actor::character_catalog::PlayableKitSource;

    let (set, charges_projectiles) = resolve_playable_action_set(
        Some(PlayableKitSource::Authored),
        None,
        ambition_engine_core::AbilitySet::sandbox_all(),
    );
    assert!(set.melee.is_none());
    assert!(set.ranged.is_none());
    assert!(set.special.is_none());
    assert!(!charges_projectiles);
}

/// Gate 1 (GPT-5.6 review): the canonical player's `Special("bubble_shield")` was
/// a PHANTOM — `default_player_action_set` declared it, but the player's moveset
/// was built melee-only, so `trigger_moveset_moves` (which fires `special_pressed`
/// only when the moveset carries the `"special"` verb) started nothing. This is
/// the end-to-end proof of the fix: build the moveset EXACTLY as the real player
/// bundle does, press `special`, and observe the resulting move.
#[test]
fn pressing_special_starts_the_real_players_folded_bubble_shield_move() {
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::brain::ActorControl;
    use bevy::prelude::*;

    // The REAL bundle authorities + the REAL moveset build (bundles.rs:135).
    let action_set = crate::avatar::bundles::default_player_action_set(
        ambition_engine_core::AbilitySet::sandbox_all(),
    );
    assert!(
        matches!(
            action_set.special.as_ref(),
            Some(ambition_characters::brain::SpecialActionSpec::Special(k)) if k == "bubble_shield"
        ),
        "the canonical player declares the bubble_shield special capability"
    );
    let moveset = build_actor_moveset(
        None,
        action_set.melee.as_ref(),
        None,
        action_set.special.as_ref(),
    )
    .expect("player moveset");

    let mut app = App::new();
    app.add_systems(
        Update,
        (
            ambition_combat::moveset::resolve_attack_gestures,
            ambition_combat::moveset::trigger_moveset_moves,
        )
            .chain(),
    );
    let mut frame = ActorControlFrame::neutral();
    frame.special_pressed = true;
    let body = app
        .world_mut()
        .spawn((
            ambition_combat::moveset::ActorMoveset(moveset),
            ActorControl(frame),
            ambition_engine_core::BodyKinematics::default(),
        ))
        .id();
    app.update();

    let playback = app
        .world()
        .get::<ambition_combat::moveset::MovePlayback>(body)
        .expect("pressing Special started a real move, not a phantom");
    assert_eq!(
        playback.spec.id, "bubble_shield",
        "the started move IS the folded bubble_shield special"
    );
}

/// The folded special is not a bare animation: while the `bubble_shield` special
/// move plays, [`sustain_bubble_shield`] holds the guard up through the ONE shield
/// input path, so the special deploys the real bubble shield. A DIFFERENT move
/// (or a body whose special isn't bubble_shield) must NOT raise the guard.
#[test]
fn the_bubble_shield_special_move_holds_the_guard_up() {
    use ambition_characters::brain::ActorControl;
    use bevy::prelude::*;

    let action_set = crate::avatar::bundles::default_player_action_set(
        ambition_engine_core::AbilitySet::sandbox_all(),
    );
    let moveset = build_actor_moveset(
        None,
        action_set.melee.as_ref(),
        None,
        action_set.special.as_ref(),
    )
    .expect("player moveset");
    let special = moveset
        .move_for_verb("special")
        .expect("special move")
        .clone();
    let attack = moveset
        .move_for_verb("attack")
        .expect("attack move")
        .clone();

    let mut app = App::new();
    app.add_systems(Update, sustain_bubble_shield);

    // Body A: the bubble_shield special is PLAYING → guard forced up.
    let shielding = app
        .world_mut()
        .spawn((
            action_set.clone(),
            ambition_combat::moveset::MovePlayback::new(special, 1.0),
            ActorControl::default(),
        ))
        .id();
    // Body B: a plain ATTACK move is playing → guard is NOT forced (only the
    // bubble_shield move raises it, by move identity).
    let attacking = app
        .world_mut()
        .spawn((
            action_set.clone(),
            ambition_combat::moveset::MovePlayback::new(attack, 1.0),
            ActorControl::default(),
        ))
        .id();
    app.update();

    assert!(
        app.world()
            .get::<ActorControl>(shielding)
            .unwrap()
            .0
            .shield_held,
        "the bubble_shield special move forces the guard up"
    );
    assert!(
        !app.world()
            .get::<ActorControl>(attacking)
            .unwrap()
            .0
            .shield_held,
        "a plain attack move does not raise the bubble shield"
    );
}

/// **C3: the ONE persona construction consults the prepared registry.**
///
/// A worn body's `ActionSet`, moveset and `IdentityKit` are built together here,
/// and `reconcile_equipment_grants` then overlays equipment onto that baseline. The
/// C3 projection first landed AFTER both and overwrote only the moveset, which
/// erased equipment-granted moves and left an action set that did not authorize the
/// verbs of the moveset beside it (GPT 5.6, 2026-07-27).
///
/// The fix is that this construction reads the registry itself, so the prepared
/// moves ARE the identity baseline — which is what makes the equipment overlay
/// apply on top of them instead of behind them.
#[test]
fn a_registered_characters_moveset_becomes_the_identity_baseline() {
    use ambition_entity_catalog::{ClipBinding, MoveGates, MoveSpec, MovesetContract};

    let swat = MoveSpec {
        id: "swat".to_string(),
        clip: ClipBinding {
            clip: "swat".to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.2,
        events: vec![],
        windows: vec![],
        gates: MoveGates { grounded: None },
        start_impulse: None,
        smash_charge_mult: 1.0,
    };
    let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
    let prepared = crate::character_runtime::prepare_and_finalize_for_test(
        crate::character_runtime::CharacterDefinition::new("hero", "Hero", "demo").with_moveset(
            MovesetContract {
                verbs: std::collections::BTreeMap::from([(
                    "attack".to_string(),
                    "swat".to_string(),
                )]),
                moves: vec![swat],
            },
        ),
        &crate::character_runtime::CharacterBindings::default(),
    );
    registry.insert_prepared(prepared.prepared);

    let catalog = CharacterCatalog::empty();
    let mut name = Name::new("placeholder");
    let mut action_set = ActionSet::default();
    let mut moveset = ActorMoveset(ambition_entity_catalog::MovesetContract::default());
    let mut identity = ambition_characters::brain::action_set::IdentityKit::default();
    crate::avatar::apply_worn_character_overlay(
        &catalog,
        Some(&registry),
        &mut name,
        &mut action_set,
        &mut moveset,
        &mut identity,
        "hero",
        ambition_engine_core::AbilitySet::default(),
    );

    assert!(
        moveset.0.moves.iter().any(|m| m.id == "swat"),
        "the registered character's authored move must be the one on the body"
    );
    assert!(
        identity.moveset.moves.iter().any(|m| m.id == "swat"),
        "and it must be the IDENTITY BASELINE, because that is what \
         `reconcile_equipment_grants` re-derives the live kit from — a baseline \
         that disagreed with the body's moveset is how a granted move gets erased \
         and how a revoked one cannot be taken back"
    );

    // ── The other half of the identity swap ──────────────────────────────────
    //
    // Wearing a quieter character must REPLACE the moves, not merge with them.
    // This is the claim the prepared-character projection used to make by
    // removing `ActorMoveset` — which was worse than useless, because the removal
    // took the body out of this very system's query and cost it its whole persona
    // (GPT 5.6, 2026-07-27). Pinned HERE because this is the single writer for a
    // worn body, and a guarantee belongs at the authority that provides it.
    let mut registry = registry;
    let unarmed = crate::character_runtime::prepare_and_finalize_for_test(
        crate::character_runtime::CharacterDefinition::new("monk", "Monk", "demo"),
        &crate::character_runtime::CharacterBindings::default(),
    );
    registry.insert_prepared(unarmed.prepared);
    crate::avatar::apply_worn_character_overlay(
        &catalog,
        Some(&registry),
        &mut name,
        &mut action_set,
        &mut moveset,
        &mut identity,
        "monk",
        ambition_engine_core::AbilitySet::default(),
    );
    assert!(
        !moveset.0.moves.iter().any(|m| m.id == "swat"),
        "the previous character's attack timeline survived the swap; a form change \
         that keeps the old moves is a body that can still throw a punch it no \
         longer has"
    );
    assert!(
        !identity.moveset.moves.iter().any(|m| m.id == "swat"),
        "and the baseline kept it too, so the next equipment reconcile would put \
         it straight back"
    );
}

// ── C3: the definition's action set outranks the catalog row's ───────────────
//
// The moveset already outranked it. The action set did not, and the split was
// the identity bug: a definition could author what moves EXIST while the catalog
// separately decided what the body and the AI believed the body could reach for
// (GPT 5.6, 2026-07-28).

/// A catalog whose row hands `id` a real melee kit.
fn catalog_granting_melee(id: &str) -> CharacterCatalog {
    use ambition_characters::actor::character_catalog::parse_catalog;
    let ron = format!(
        r#"(
            brain_presets: {{ "stand_still": StandStill }},
            action_set_presets: {{
                "brawler": (
                    move_style: Walk,
                    melee: Some(Swipe(
                        windup_s: 0.28, active_s: 0.08, recover_s: 0.32,
                        damage: 3, reach_px: 40.0,
                    )),
                ),
            }},
            characters: {{
                "{id}": (
                    display_name: "Catalog Says Brawler",
                    spritesheet: "sprites/robot_spritesheet.png",
                    manifest: "sprites/robot_spritesheet.ron",
                    tier: Basement,
                    body_kind: Standard,
                    composition: None,
                    default_brain: "stand_still",
                    default_action_set: "brawler",
                    playable_kit: Authored,
                    tags: [],
                ),
            }},
        )"#
    );
    CharacterCatalog::from_data(parse_catalog(&ron))
}

/// Run the one production writer and hand back what it put on the body.
fn wear(
    catalog: &CharacterCatalog,
    registry: &crate::character_runtime::PreparedCharacterRegistry,
    id: &str,
) -> (ActionSet, ActorMoveset) {
    let mut name = Name::new("placeholder");
    let mut action_set = ActionSet::default();
    let mut moveset = ActorMoveset(ambition_entity_catalog::MovesetContract::default());
    let mut identity = ambition_characters::brain::action_set::IdentityKit::default();
    crate::avatar::apply_worn_character_overlay(
        catalog,
        Some(registry),
        &mut name,
        &mut action_set,
        &mut moveset,
        &mut identity,
        id,
        ambition_engine_core::AbilitySet::default(),
    );
    assert_eq!(
        identity.action_set, action_set,
        "the identity BASELINE and the live set disagreed at publication — \
         `reconcile_equipment_grants` re-derives from the baseline, so this is \
         how a granted verb gets erased and a revoked one cannot be taken back"
    );
    (action_set, moveset)
}

fn prepared(
    definition: crate::character_runtime::CharacterDefinition,
) -> crate::character_runtime::PreparedCharacterRegistry {
    prepared_against(definition, None)
}

/// The same, with the catalog the barrier folds against.
///
/// Which row a character inherits is decided at FINALIZATION now, not when a
/// body wears it — so a fixture proving inheritance has to supply the catalog
/// here rather than only to `wear`. That is not fixture bookkeeping: it is the
/// change itself, visible. A registry finalized without a catalog describes a
/// composition that has none, and a character in it inherits nothing because
/// there was nothing to inherit from.
fn prepared_against(
    definition: crate::character_runtime::CharacterDefinition,
    catalog: Option<&CharacterCatalog>,
) -> crate::character_runtime::PreparedCharacterRegistry {
    let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
    registry.insert_prepared(
        crate::character_runtime::prepare_and_finalize_against_for_test(
            definition,
            &crate::character_runtime::CharacterBindings::default(),
            catalog,
        )
        .prepared,
    );
    registry
}

#[test]
fn an_action_set_authored_on_the_definition_beats_the_catalog_row() {
    use ambition_characters::brain::action_set::{MeleeActionSpec, SwipeSpec};

    let catalog = catalog_granting_melee("duellist");
    let authored = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
            damage: 9,
            reach_px: 999.0,
            ..SwipeSpec::STRIKER_DEFAULT
        })),
        ..ActionSet::default()
    };
    let registry = prepared(
        crate::character_runtime::CharacterDefinition::new("duellist", "Duellist", "demo")
            .with_action_set(authored),
    );

    let (set, _) = wear(&catalog, &registry, "duellist");

    match set.melee {
        Some(MeleeActionSpec::Swipe(swipe)) => assert_eq!(
            swipe.damage, 9,
            "the catalog row's kit won: the definition authored damage 9 and the \
             body is swinging the row's 3"
        ),
        other => panic!("the authored melee did not reach the body: {other:?}"),
    }
}

#[test]
fn an_authored_empty_action_set_is_not_the_same_as_authoring_nothing() {
    // Sanic. His kit IS the momentum ride and the ball dash; giving him a punch
    // to fill a slot would be authoring against the design (queue C3).
    //
    // So `Some(ActionSet::default())` has to survive as a DECISION. A resolver
    // that asks "does this set look empty?" instead of "did anybody author one?"
    // falls through to the catalog and hands him the row's melee — and the whole
    // reason this field is an `Option` is to make that unrepresentable.
    let catalog = catalog_granting_melee("speedster");
    let registry = prepared(
        crate::character_runtime::CharacterDefinition::new("speedster", "Speedster", "demo")
            .with_action_set(ActionSet::default()),
    );

    let (set, moveset) = wear(&catalog, &registry, "speedster");

    assert!(
        set.melee.is_none(),
        "an author who wrote an empty action set was overruled by the catalog \
         row, so 'this character reaches for nothing' and 'this character was \
         never described' resolve identically: {:?}",
        set.melee
    );
    assert!(
        moveset.0.moves.is_empty(),
        "and the moveset derived from the DISPLACED catalog value: {:?}",
        moveset.0.moves.iter().map(|m| &m.id).collect::<Vec<_>>()
    );
}

#[test]
fn a_definition_with_no_action_set_still_falls_through_to_the_catalog() {
    // The other half of the same claim, and the one that keeps this migration
    // safe: every character that has not authored a set must behave exactly as
    // it did before this existed.
    use ambition_characters::brain::action_set::MeleeActionSpec;

    let catalog = catalog_granting_melee("inheritor");
    let registry = prepared_against(
        crate::character_runtime::CharacterDefinition::new("inheritor", "Inheritor", "demo"),
        Some(&catalog),
    );

    let (set, _) = wear(&catalog, &registry, "inheritor");

    match set.melee {
        Some(MeleeActionSpec::Swipe(swipe)) => assert_eq!(swipe.damage, 3),
        other => panic!("an unauthored character stopped inheriting its row: {other:?}"),
    }
}

#[test]
fn a_prepared_action_set_with_no_prepared_moveset_derives_from_the_winning_set() {
    // The precise bug GPT 5.6 named. Precedence alone is not enough: if the
    // action set is resolved and the moveset is then derived from the value the
    // resolution DISPLACED, the body reaches for capabilities its own definition
    // removed. Empty authored set + catalog melee is the case that tells the two
    // implementations apart — deriving from the winner yields no moves, deriving
    // from the loser yields the row's swipe.
    let catalog = catalog_granting_melee("minimalist");
    let registry = prepared(
        crate::character_runtime::CharacterDefinition::new("minimalist", "Minimalist", "demo")
            .with_action_set(ActionSet::default()),
    );

    let (_, moveset) = wear(&catalog, &registry, "minimalist");

    assert!(
        moveset.0.moves.is_empty(),
        "the moveset was derived from the catalog action set the definition had \
         already displaced: {:?}",
        moveset.0.moves.iter().map(|m| &m.id).collect::<Vec<_>>()
    );
}

/// **An authored RANGED action set must derive a ranged MOVE.**
///
/// The mirror of the test above, and the half that was broken. Precedence put
/// the definition's action set in charge, and then the derivation threw away
/// its ranged payload: `build_actor_moveset(None, melee, None, special)` passed
/// a hard-coded `None` where `set.ranged` lives.
///
/// The result is worse than a missing move, because the body still ADVERTISES
/// the capability — the brain and the input bridge both read `ActionSet` to
/// decide whether the ranged verb may be pressed — so it reaches for a verb
/// with no timeline behind it. A gun the character does not believe in.
///
/// Preparation never caught it: it validates the REVERSE mismatch only (an
/// explicit ranged move whose action set supplies no payload). Found by
/// GPT 5.6, 2026-07-28.
#[test]
fn an_authored_ranged_action_set_derives_a_ranged_move() {
    use ambition_characters::brain::action_set::{RangedActionSpec, RangedStyle};

    let catalog = catalog_granting_melee("gunslinger");
    let registry = prepared(
        crate::character_runtime::CharacterDefinition::new("gunslinger", "Gunslinger", "demo")
            .with_action_set(ActionSet {
                ranged: Some(RangedActionSpec {
                    style: RangedStyle::default(),
                    speed: 411.0,
                    damage: 7,
                    flight: None,
                    visual: None,
                }),
                ..ActionSet::default()
            }),
    );

    let (action_set, moveset) = wear(&catalog, &registry, "gunslinger");

    assert!(
        action_set.ranged.is_some(),
        "fixture: the body must ADVERTISE ranged, or the mismatch under test \
         cannot exist"
    );
    assert!(
        moveset
            .0
            .verbs
            .contains_key(crate::combat::moveset::RANGED_VERB),
        "the action set advertises ranged and the derived moveset has no ranged \
         verb, so pressing it does nothing: {:?}",
        moveset.0.verbs.keys().collect::<Vec<_>>()
    );
}

/// **X8: the primary player's kit comes from the RESOLVED identity.**
///
/// `PlayerSimulationBundle::from_scratch_as_character` applies the overlay with
/// `registry: None`, and its comment says why — a from-scratch bundle predates
/// the world it will live in, so there is no registry to consult yet, and "the
/// per-frame derivation reaches the body on its first tick".
///
/// That is a claim about a SYSTEM, made at a call site that cannot verify it, so
/// it is worth a test rather than a comment: if the derive did not reach the
/// spawned body, the player would keep the catalog kit forever and the sentence
/// would still read as true.
///
/// Driven through `apply_worn_character_gameplay` — the one production writer —
/// against a body carrying what a player body carries.
#[test]
fn a_spawned_player_body_receives_the_prepared_action_set_on_its_first_tick() {
    use crate::combat::moveset::ActorMoveset;
    use ambition_characters::brain::action_set::{RangedActionSpec, RangedStyle};
    use ambition_characters::brain::ActionSet;
    use bevy::prelude::*;

    let authored = ActionSet {
        ranged: Some(RangedActionSpec {
            style: RangedStyle::default(),
            speed: 411.0,
            damage: 7,
            flight: None,
            visual: None,
        }),
        ..ActionSet::default()
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.insert_resource(prepared(
        crate::character_runtime::CharacterDefinition::new("sanic", "Sanic", "sanic_demo")
            .with_action_set(authored.clone()),
    ));
    app.add_systems(Update, apply_worn_character_gameplay);

    let body = app
        .world_mut()
        .spawn((
            WornCharacter::new("sanic"),
            MotionModel::default(),
            Name::new("unset"),
            // What the bundle leaves behind: the catalog-derived kit, applied
            // with no registry because there was none yet.
            ActionSet::default(),
            ActorMoveset(Default::default()),
            ambition_characters::brain::action_set::IdentityKit::default(),
            ambition_engine_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_engine_core::BodyClusterScratch::new_with_abilities(
                    ambition_engine_core::Vec2::ZERO,
                    ambition_engine_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();

    app.update();

    let live = app
        .world()
        .get::<ActionSet>(body)
        .expect("the body kept its kit");
    assert_eq!(
        live, &authored,
        "the spawned player never received its character's authored action set — \
         the bundle's `registry: None` is only safe because the derive corrects \
         it on the first tick, and it did not"
    );
    let baseline = app
        .world()
        .get::<ambition_characters::brain::action_set::IdentityKit>(body)
        .expect("the body has an identity baseline");
    assert_eq!(
        &baseline.action_set, &authored,
        "the live kit is right and the BASELINE is not, so the next equipment \
         reconcile re-derives from the wrong thing and takes it away again"
    );
}

/// The worn half of the same claim: a player wearing a character that authored
/// its motion model gets that model, not the catalog row's.
///
/// Both halves land together on purpose. The action set was wired into the worn
/// path alone and seated fighters went without it for a day (X9); doing the
/// third leg of the kit the same way would have been repeating a mistake whose
/// diagnosis is two commits old.
#[test]
fn a_definition_authored_motion_model_beats_the_catalog_row() {
    use ambition_engine_core::{MomentumParams, MotionModelSpec};

    let catalog = test_catalog();
    let momentum = MotionModelSpec::SurfaceMomentum(MomentumParams {
        ground_accel: 1234.0,
        ..Default::default()
    });
    let registry = prepared(
        crate::character_runtime::CharacterDefinition::new("mary_o", "Mary-O", "demo")
            .with_motion_model(momentum),
    );

    let resolved =
        crate::avatar::motion_model_spec_for_character(Some(&registry), &catalog, "mary_o");
    assert_eq!(
        resolved, momentum,
        "the catalog row won over the definition's authored motion model"
    );

    // And a character that authored NOTHING still inherits its row — the
    // migration path, and the half that keeps this safe to put in front of
    // every character at once.
    let untouched =
        crate::avatar::motion_model_spec_for_character(Some(&registry), &catalog, "sanic");
    assert_eq!(
        untouched,
        crate::avatar::motion_model_spec_for_character_id(&catalog, "sanic"),
        "an unauthored character stopped inheriting its catalog row"
    );
}
