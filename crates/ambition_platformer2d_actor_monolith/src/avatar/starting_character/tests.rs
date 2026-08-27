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

/// AN AUTHORED CHARGE SURVIVES ITS AUTHORED KIT.
///
/// Two terms, because one alone proves nothing: a character that authors
/// `ChargedProjectile` charges even with a fully authored kit, and one that
/// authors nothing does NOT — the default has to stay `MovesetVerb` or every
/// content NPC acquires a charge mechanic it never asked for.
#[test]
fn an_authored_character_decides_whether_it_charges() {
    use ambition_characters::actor::definition::CharacterDefinition;
    use ambition_characters::brain::RangedExecution;

    let charged = CharacterDefinition::new("charger", "Charger", "test")
        .with_ranged_execution(RangedExecution::ChargedProjectile);
    assert_eq!(
        charged.ranged_execution,
        RangedExecution::ChargedProjectile,
        "a character that authors a charge did not keep it"
    );
    assert!(
        charged.ranged_execution.charges_projectiles(),
        "the authored charge does not reach the runtime capability, so the \
         marker that installs `ChargesProjectiles` will never be set"
    );

    // the poison: the DEFAULT must remain the ordinary verb.
    let plain = CharacterDefinition::new("plain", "Plain", "test");
    assert_eq!(
        plain.ranged_execution,
        RangedExecution::MovesetVerb,
        "a character that did not opt into charged projectiles acquired the capability"
    );
    assert!(!plain.ranged_execution.charges_projectiles());
}

#[test]
fn default_is_unset_and_is_default() {
    // No override: an empty id routes to the untouched `from_scratch` path.
    // The concrete row is CONTENT's (`effective_id` resolves it at spawn);
    // the engine bakes in no character name.
    let sc = StartingCharacter::default();
    assert!(sc.character_id.as_str().is_empty());
    assert!(sc.is_default());
    // `effective_id` resolves to a real catalog row (the content-installed
    // default, or the first row as fallback) — never empty, never a name
    // the ENGINE baked in.
    let eff = sc.effective_id("player_robot_v3");
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
        apply_worn_motion_model(&catalog, &mut commands, entity, "player_robot_v3");
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

/// S1: gameplay configuration is DERIVED from the worn identity, at spawn
/// (Added) and on any later re-wear (Changed). A body carrying only the
/// `WornCharacter` identity plus the mutable gameplay components has its name
/// and movement identity re-derived by `apply_worn_character_gameplay`.
#[test]
fn gameplay_derives_from_worn_identity_at_add_and_on_change() {
    use ambition_characters::brain::ActionSet;
    use ambition_combat::moveset::ActorMoveset;
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
            ambition_platformer2d_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
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

    *app.world_mut().get_mut::<WornCharacter>(e).unwrap() = WornCharacter::new("player_robot_v3");
    // AND ASK FOR IT. Writing the identity stopped rebuilding the
    // body: a re-wear is an explicit request, the
    // way Mary-O's powerup already made it. A fixture that mutated the id and
    // expected a rebuild was encoding the contract that split.
    app.world_mut()
        .entity_mut(e)
        .insert(ambition_characters::actor::RecharacterizeBody);
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
        "Player Robot v3",
        "the display name follows the new worn identity"
    );
}

#[test]
fn rewearing_an_equivalent_momentum_profile_preserves_live_ride_state() {
    use ambition_characters::brain::ActionSet;
    use ambition_combat::moveset::ActorMoveset;
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
            ambition_platformer2d_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();
    app.update();

    let expected = ambition_platformer2d_core::SurfaceMotion::Riding {
        on: ambition_platformer2d_core::SurfaceRef::Chain(0),
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
    // AND ASK FOR IT. Writing the identity stopped rebuilding the
    // body: a re-wear is an explicit request, the
    // way Mary-O's powerup already made it. A fixture that mutated the id and
    // expected a rebuild was encoding the contract that split.
    app.world_mut()
        .entity_mut(entity)
        .insert(ambition_characters::actor::RecharacterizeBody);
    app.update();

    match app.world().get::<MotionModel>(entity) {
        Some(MotionModel::SurfaceMomentum(momentum)) => {
            assert_eq!(momentum.state, expected);
        }
        other => panic!("expected preserved SurfaceMomentum, got {other:?}"),
    }
}

/// S1 poison / non-vacuity: with no change to either `WornCharacter` or
/// `BodyAbilities`, the derive system does not fire, so a hand-set movement model
/// is left untouched. This proves the assertions above are driven by the two
/// `Changed` edges, not by the system running unconditionally every frame.
#[test]
fn derive_system_only_fires_on_identity_or_ability_change() {
    use ambition_characters::brain::ActionSet;
    use ambition_combat::moveset::ActorMoveset;
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
            ambition_platformer2d_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
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
        ambition_platformer2d_core::movement::AxisSweptMotion::default(),
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

/// WRITING THE IDENTITY IS NOT A RE-TEMPLATE REQUEST.
///
/// Splitting the component was not enough while that comparison stood: an id write still rebuilt
/// the body, so ordinary construction could still be finished by an observation rather than by a
/// constructor.
///
/// Two steps, and the pair is the proof: change the id and NOTHING happens; then ask, and the
/// replacement runs.
#[test]
fn changing_the_worn_identity_alone_does_not_rebuild_the_body() {
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.add_systems(Update, apply_worn_character_gameplay);
    let e = app
        .world_mut()
        .spawn((
            WornCharacter::new("sanic"),
            Name::new("placeholder"),
            ambition_characters::brain::ActionSet::peaceful(),
            ambition_combat::moveset::ActorMoveset(Default::default()),
            ambition_characters::brain::action_set::IdentityKit::default(),
            ambition_platformer2d_core::BodyAbilities::new(
                ambition_platformer2d_core::AbilitySet::sandbox_all(),
            ),
            MotionModel::default(),
            ambition_characters::actor::RecharacterizeBody,
        ))
        .id();
    app.update();
    assert_eq!(
        app.world().get::<Name>(e).unwrap().as_str(),
        "Sanic",
        "the first application did not happen, so neither step below means anything"
    );

    // ⇥ STEP ONE: change the identity, ask nothing.
    *app.world_mut().get_mut::<WornCharacter>(e).unwrap() = WornCharacter::new("player_robot_v3");
    app.update();
    assert_eq!(
        app.world().get::<Name>(e).unwrap().as_str(),
        "Sanic",
        "writing the worn id rebuilt the body, so `WornCharacter` still means \
         'please re-apply me' and the split is nominal"
    );

    // ⇥ STEP TWO: ask.
    app.world_mut()
        .entity_mut(e)
        .insert(ambition_characters::actor::RecharacterizeBody);
    app.update();
    assert_eq!(
        app.world().get::<Name>(e).unwrap().as_str(),
        "Player Robot v3",
        "an explicit re-template request did not perform the replacement, which \
         leaves no way to re-wear a body at all"
    );
}

/// The full KIT (ActionSet + moveset), not just name/movement, follows a
/// re-wear between two KNOWN characters — the reviewer-flagged gap. Wearing
/// the pirate gives its authored pistol; re-wearing the goblin replaces it with
/// the goblin's kit, leaving no stale pirate pistol behind.
#[test]
fn worn_kit_fully_follows_a_known_character_rewear() {
    use ambition_characters::brain::{action_set::RangedStyle, ActionSet, RangedActionSpec};
    use ambition_combat::moveset::ActorMoveset;
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
            ambition_platformer2d_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
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
    // AND ASK FOR IT. Writing the identity stopped rebuilding the
    // body: a re-wear is an explicit request, the
    // way Mary-O's powerup already made it. A fixture that mutated the id and
    // expected a rebuild was encoding the contract that split.
    app.world_mut()
        .entity_mut(e)
        .insert(ambition_characters::actor::RecharacterizeBody);
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

#[test]
fn runtime_rewear_rebuilds_from_the_destination_character() {
    use ambition_characters::brain::{ActionSet, RangedActionSpec};
    use ambition_combat::moveset::ActorMoveset;
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
            ambition_platformer2d_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
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

    // Re-wear the HostCode default ("player_robot_v3"): the code kit (Swipe +
    // Bolt + bubble_shield from sandbox_all abilities) is rebuilt — NO stale
    // pistol.
    *app.world_mut().get_mut::<WornCharacter>(e).unwrap() = WornCharacter::new("player_robot_v3");
    // AND ASK FOR IT. Writing the identity stopped rebuilding the
    // body: a re-wear is an explicit request, the
    // way Mary-O's powerup already made it. A fixture that mutated the id and
    // expected a rebuild was encoding the contract that split.
    app.world_mut()
        .entity_mut(e)
        .insert(ambition_characters::actor::RecharacterizeBody);
    app.update();
    assert_eq!(
        app.world().get::<Name>(e).unwrap().as_str(),
        "Player Robot v3"
    );
    let set = app.world().get::<ActionSet>(e).unwrap();
    // THE INVARIANT, WHICH IS ABOUT STALENESS AND NOT ABOUT THE ROBOT.
    // The pistol must be GONE: a kit is a function of identity plus persisted
    // abilities, never of mutation history, and that is also the snapshot-restore
    // contract — restoring a `WornCharacter` onto a survivor must rebuild rather
    // than inherit.
    //
    // Robot v3 authors its repertoire on its definition in `ambition_content` now, and this crate
    // cannot depend on content to build it. That is the correct outcome, not a gap: the engine does
    // not know the protagonist's moves. What it must still guarantee is that nothing of the
    // PREVIOUS character survives the change.
    assert!(
        !matches!(
            set.ranged,
            Some(RangedActionSpec {
                style: RangedStyle::Pistol,
                ..
            })
        ),
        "the pirate's pistol survived a re-wear, so this body's kit depends on \
         what it used to be — and a restored snapshot would inherit it too"
    );
    assert_eq!(
        *set,
        ActionSet::peaceful(),
        "the re-worn body kept something the destination row does not author"
    );
}

/// Unknown ids are deterministic, not stale. Re-wearing an id the catalog
/// does not know installs a DEFINED fallback (the code kit rebuilt from the
/// body's abilities) and names the body after the id — it never silently keeps
/// the prior character's kit or name.
#[test]
fn runtime_rewear_to_an_unknown_id_is_a_defined_fallback_not_stale_state() {
    use ambition_characters::brain::{ActionSet, MeleeActionSpec, RangedActionSpec};
    use ambition_combat::moveset::ActorMoveset;
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
            ambition_platformer2d_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();
    app.update();

    *app.world_mut().get_mut::<WornCharacter>(e).unwrap() =
        WornCharacter::new("ghost_not_in_catalog");
    // AND ASK FOR IT. Writing the identity stopped rebuilding the
    // body: a re-wear is an explicit request, the
    // way Mary-O's powerup already made it. A fixture that mutated the id and
    // expected a rebuild was encoding the contract that split.
    app.world_mut()
        .entity_mut(e)
        .insert(ambition_characters::actor::RecharacterizeBody);
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
    use ambition_characters::brain::{ActionSet, MeleeActionSpec, RangedActionSpec};
    use ambition_combat::moveset::ActorMoveset;
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
            ambition_platformer2d_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
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
            .get_mut::<ambition_platformer2d_core::BodyAbilities>(entity)
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
    use ambition_characters::brain::ActionSet;
    use ambition_characters::control::ActorControl;
    use bevy::prelude::*;

    let mut frame = ActorControlFrame::neutral();
    frame.melee_pressed = true;
    frame.pogo_pressed = true;
    frame.attack_axis = ambition_platformer2d_core::LocalAxes::new(1.0, -1.0);
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
            ambition_platformer2d_shared_tangle::markers::PlayerEntity,
            WornCharacter::new("sanic"),
            MotionModel::default(),
            // The gate resolves the body's live authorities through the shared
            // scheme; a peaceful `ActionSet` (no melee/ranged/special) + no
            // moveset yields no combat slots, so the gate strips every combat
            // verb below. `BodyAbilities` supplies the movement slots.
            ambition_platformer2d_core::BodyAbilities::new(
                ambition_platformer2d_core::AbilitySet::sandbox_all(),
            ),
            ActionSet::peaceful(),
            ActorControl(frame),
        ))
        .id();
    app.update();

    let gated = &app.world().get::<ActorControl>(entity).unwrap().0;
    assert!(!gated.melee_pressed);
    assert!(!gated.pogo_pressed);
    assert_eq!(
        gated.attack_axis,
        ambition_platformer2d_core::LocalAxes::ZERO
    );
    assert!(gated.fire.is_none());
    // Peaceful is a claim about ATTACKING; a guard is defensive, and this body's `BodyAbilities`
    // grants `shield`, which is what puts the Shield slot on its scheme. A body that lacks the
    // ability still loses the verb — that is
    // `a_body_without_the_shield_ability_loses_its_guard_verb` below, and keeping the strip here
    // would have been the gate asking about the wrong authority again.
    assert!(
        gated.shield_held,
        "a peaceful persona that OWNS the shield ability lost its guard — the \
         persona gate is judging a defensive capability by an offensive kit"
    );
    assert!(!gated.projectile_pressed);
    assert!(!gated.projectile_held);
    assert!(!gated.projectile_released);
}

/// A CHARACTER THAT CHARGES KEEPS ITS PROJECTILE PRESS.
///
/// The hour Robot v3 started authoring its own kit — the row flipping to `Authored` — that gate
/// began silently disabling the Hadouken. Nothing failed: pressing the projectile button as the
/// protagonist was covered nowhere, so a full green suite said the protagonist could still fire.
///
/// the gate asks the CHARACTER how it fires now (`ranged_execution`), which is
/// what §4 made an authored fact for. Two terms, because the permissive half
/// alone would pass with the gate deleted: a charging character keeps the press,
/// and a `MovesetVerb` character still loses it.
#[test]
fn an_authored_charging_character_keeps_its_projectile_press() {
    use crate::character_runtime::CharacterBindings;
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::actor::definition::CharacterDefinition;
    use ambition_characters::brain::ActionSet;
    use ambition_characters::control::ActorControl;
    use bevy::prelude::*;

    let charge_frame = || {
        let mut frame = ActorControlFrame::neutral();
        frame.projectile_pressed = true;
        frame.projectile_held = true;
        frame.projectile_released = true;
        frame
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.add_systems(Update, gate_worn_player_control);

    // Two characters, identical but for how they fire.
    let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
    for (id, execution) in [
        (
            "gunner",
            ambition_characters::brain::RangedExecution::ChargedProjectile,
        ),
        (
            "swordfighter",
            ambition_characters::brain::RangedExecution::MovesetVerb,
        ),
    ] {
        registry.insert_prepared(
            crate::character_runtime::prepare_and_finalize_against_for_test(
                CharacterDefinition::new(id, id, "test")
                    .with_ranged_execution(execution)
                    .with_action_set(ActionSet {
                        ranged: Some(ambition_characters::brain::RangedActionSpec::bolt(600.0, 1)),
                        ..ActionSet::default()
                    }),
                &CharacterBindings::default(),
                None,
            )
            .prepared,
        );
    }
    app.insert_resource(registry);

    let spawn = |app: &mut App, id: &str| {
        app.world_mut()
            .spawn((
                ambition_platformer2d_shared_tangle::markers::PlayerEntity,
                WornCharacter::new(id),
                MotionModel::default(),
                ambition_platformer2d_core::BodyAbilities::new(
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
                ),
                ActionSet {
                    ranged: Some(ambition_characters::brain::RangedActionSpec::bolt(600.0, 1)),
                    ..ActionSet::default()
                },
                ambition_characters::brain::ChargesProjectiles,
                ActorControl(charge_frame()),
            ))
            .id()
    };
    let gunner = spawn(&mut app, "gunner");
    let swordfighter = spawn(&mut app, "swordfighter");
    app.update();

    let read = |app: &App, e: Entity| {
        app.world()
            .get::<ActorControl>(e)
            .unwrap()
            .0
            .projectile_held
    };
    assert!(
        read(&app, gunner),
        "a character that AUTHORS a charged projectile lost its press — this is \
         the Hadouken going quiet the moment the robot stopped saying `HostCode`"
    );
    // the poison: the gate must still strip the press from a character that
    // does not charge, or it is not a gate.
    assert!(
        !read(&app, swordfighter),
        "a character that fires through its MOVESET kept the charge press, so \
         one button would be owned twice"
    );
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
    use ambition_characters::brain::ActionSet;
    use ambition_characters::control::ActorControl;
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
            ambition_platformer2d_shared_tangle::markers::PlayerEntity,
            WornCharacter::new("sanic"),
            ambition_platformer2d_core::BodyAbilities::new(
                ambition_platformer2d_core::AbilitySet::sandbox_all(),
            ),
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

/// A typo in a known catalog row is content corruption, not permission to gain
/// the host protagonist's code kit. Validation reports the bad row; the runtime
/// fallback is deliberately inert.
#[test]
fn malformed_authored_resolution_is_safe_peaceful_not_host_code() {
    let (set, execution) = resolve_playable_action_set(
        // the catalog HAS a row for it; its preset is what does not resolve
        true,
        None,
        ambition_platformer2d_core::AbilitySet::sandbox_all(),
    );
    assert!(set.melee.is_none());
    assert!(set.ranged.is_none());
    assert!(set.special.is_none());
    assert_eq!(execution, RangedExecution::MovesetVerb);
}

/// Gate 1: the canonical player's `Special("bubble_shield")` was
/// a PHANTOM — `default_player_action_set` declared it, but the player's moveset
/// was built melee-only, so `trigger_moveset_moves` (which fires `special_pressed`
/// only when the moveset carries the `"special"` verb) started nothing. This is
/// the end-to-end proof of the fix: build the moveset EXACTLY as the real player
/// bundle does, press `special`, and observe the resulting move.
#[test]
fn pressing_special_starts_the_real_players_folded_bubble_shield_move() {
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::control::ActorControl;
    use bevy::prelude::*;

    // The REAL bundle authorities + the REAL moveset build (bundles.rs:135).
    let action_set = crate::avatar::bundles::default_player_action_set(
        ambition_platformer2d_core::AbilitySet::sandbox_all(),
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
    // The buffer decays on the body's own clock, so the chain needs one.
    app.insert_resource(ambition_time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        raw_dt: 1.0 / 60.0,
        ..Default::default()
    });
    app.add_systems(
        Update,
        (
            ambition_combat::moveset::resolve_attack_gestures,
            // ⛔ THE PRODUCTION CHAIN IS THREE SYSTEMS, and the middle one is
            // what publishes `ResolvedAttackGesture::special` — the value the
            // trigger's special arm matches on. Without it the press was
            // interpreted and then thrown away, and the fixture read "no move"
            // as the phantom it was written to disprove.
            ambition_combat::moveset::buffer_combat_action_presses,
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
            ambition_platformer2d_core::BodyKinematics::default(),
            // ⛔ THE RESOLVER'S REQUIRED COLUMNS, and without them this
            // fixture measured the QUERY rather than the special: a body the
            // system does not match starts no move, which reads exactly like
            // the phantom this test exists to disprove.
            //
            // The press ledger (the spend site for a buffered edge), and the
            // gesture triple `resolve_attack_gestures` reads and publishes.
            ambition_platformer2d_core::BodyActionBuffer::default(),
            ambition_characters::actor::attack_gesture::AttackGestureState::default(),
            ambition_characters::actor::attack_gesture::AttackGestureTuning::default(),
            ambition_characters::actor::attack_gesture::ResolvedAttackGesture::default(),
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
    use ambition_characters::control::ActorControl;
    use bevy::prelude::*;

    let action_set = crate::avatar::bundles::default_player_action_set(
        ambition_platformer2d_core::AbilitySet::sandbox_all(),
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

/// C3: the ONE persona construction consults the prepared registry.
///
/// A worn body's `ActionSet`, moveset and `IdentityKit` are built together here, and
/// `reconcile_equipment_grants` then overlays equipment onto that baseline.
///
/// The fix is that this construction reads the registry itself, so the prepared
/// moves ARE the identity baseline — which is what makes the equipment overlay
/// apply on top of them instead of behind them.
#[test]
fn a_registered_characters_moveset_becomes_the_identity_baseline() {
    use ambition_entity_catalog::{ClipBinding, MoveGates, MoveSpec, MovesetContract};

    let swat = MoveSpec {
        display_name: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        id: "swat".to_string(),
        clip: ClipBinding {
            clip: "swat".to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.2,
        events: vec![],
        windows: vec![],
        gates: MoveGates::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        repeat: None,
    };
    let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
    let prepared = crate::character_runtime::prepare_and_finalize_for_test(
        ambition_characters::actor::definition::CharacterDefinition::new("hero", "Hero", "demo")
            .with_moveset(MovesetContract {
                verbs: std::collections::BTreeMap::from([(
                    "attack".to_string(),
                    "swat".to_string(),
                )]),
                moves: vec![swat],
            }),
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
        None,
        "hero",
        ambition_platformer2d_core::AbilitySet::default(),
        // No match: this fixture is testing the AUTHORED persona.
        None,
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
    // Wearing a quieter character must REPLACE the moves, not merge with them. Pinned HERE because
    // this is the single writer for a worn body, and a guarantee belongs at the authority that
    // provides it.
    let mut registry = registry;
    let unarmed = crate::character_runtime::prepare_and_finalize_for_test(
        ambition_characters::actor::definition::CharacterDefinition::new("monk", "Monk", "demo"),
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
        None,
        "monk",
        ambition_platformer2d_core::AbilitySet::default(),
        // No match: this fixture is testing the AUTHORED persona.
        None,
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
// The moveset already outranked it.

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
        None,
        id,
        // A BODY THAT MAY ACT, and this was `AbilitySet::default()` — which is `basic()`, whose
        // `attack` and `shield` are BOTH false. Every persona these tests derive is a fighter;
        // production gives a fighter a body that fights.
        ambition_platformer2d_core::AbilitySet {
            attack: true,
            shield: true,
            ..ambition_platformer2d_core::AbilitySet::basic()
        },
        // No match: this fixture is testing the AUTHORED persona.
        None,
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
    definition: ambition_characters::actor::definition::CharacterDefinition,
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
    definition: ambition_characters::actor::definition::CharacterDefinition,
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
        ambition_characters::actor::definition::CharacterDefinition::new(
            "duellist", "Duellist", "demo",
        )
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
    // to fill a slot would be authoring against the design.
    //
    // So `Some(ActionSet::default())` has to survive as a DECISION. A resolver
    // that asks "does this set look empty?" instead of "did anybody author one?"
    // falls through to the catalog and hands him the row's melee — and the whole
    // reason this field is an `Option` is to make that unrepresentable.
    let catalog = catalog_granting_melee("speedster");
    let registry = prepared(
        ambition_characters::actor::definition::CharacterDefinition::new(
            "speedster",
            "Speedster",
            "demo",
        )
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
        ambition_characters::actor::definition::CharacterDefinition::new(
            "inheritor",
            "Inheritor",
            "demo",
        ),
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
    // Precedence alone is not enough: if the action set is resolved and the moveset is then derived
    // from the value the resolution DISPLACED, the body reaches for capabilities its own definition
    // removed. Empty authored set + catalog melee is the case that tells the two implementations
    // apart — deriving from the winner yields no moves, deriving from the loser yields the row's
    // swipe.
    let catalog = catalog_granting_melee("minimalist");
    let registry = prepared(
        ambition_characters::actor::definition::CharacterDefinition::new(
            "minimalist",
            "Minimalist",
            "demo",
        )
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

/// An authored RANGED action set must derive a ranged MOVE.
///
/// Precedence put the definition's action set in charge, and then the derivation threw away its
/// ranged payload: `build_actor_moveset(None, melee, None, special)` passed a hard-coded `None`
/// where `set.ranged` lives.
///
/// The result is worse than a missing move, because the body still ADVERTISES
/// the capability — the brain and the input bridge both read `ActionSet` to
/// decide whether the ranged verb may be pressed — so it reaches for a verb
/// with no timeline behind it. A gun the character does not believe in.
///
/// Preparation never caught it: it validates the REVERSE mismatch only (an explicit ranged move
/// whose action set supplies no payload).
#[test]
fn an_authored_ranged_action_set_derives_a_ranged_move() {
    use ambition_characters::brain::action_set::{RangedActionSpec, RangedStyle};

    let catalog = catalog_granting_melee("gunslinger");
    let registry = prepared(
        ambition_characters::actor::definition::CharacterDefinition::new(
            "gunslinger",
            "Gunslinger",
            "demo",
        )
        .with_action_set(ActionSet {
            ranged: Some(RangedActionSpec {
                style: RangedStyle::default(),
                speed: 411.0,
                damage: 7,
                flight: None,
                visual: None,
                charge: None,
                refire_s: ambition_characters::brain::action_set::DEFAULT_RANGED_REFIRE_S,
                aim_assist: None,
                discharge: None,
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
            .contains_key(ambition_combat::moveset::RANGED_VERB),
        "the action set advertises ranged and the derived moveset has no ranged \
         verb, so pressing it does nothing: {:?}",
        moveset.0.verbs.keys().collect::<Vec<_>>()
    );
}

/// An authored SPECIAL derives a special move. (the H2 defect, one field over)
///
/// `ActionSet::special` is a CAPABILITY marker: its own doc says the brain reads
/// `special.is_some()` to decide whether to press it, and that the execution is
/// "a data-driven move in the body's `ActorMoveset`, triggered on the `special`
/// verb". Both derivations passed a hard-coded `None` there, on the reasoning
/// that an authored persona puts its special into its authored MOVES — which is
/// true only when it authored a moveset, and nothing requires one.
///
/// So a character could advertise a signature move with no timeline behind it,
/// which is exactly what H2 closed for `ranged` and exactly as invisible: the
/// press is accepted and nothing happens.
#[test]
fn an_authored_special_action_set_derives_a_special_move() {
    use ambition_characters::brain::SpecialActionSpec;

    let catalog = catalog_granting_melee("mystic");
    let registry = prepared(
        ambition_characters::actor::definition::CharacterDefinition::new(
            "mystic", "Mystic", "demo",
        )
        .with_action_set(ActionSet {
            special: Some(SpecialActionSpec::Special("starfall".into())),
            ..ActionSet::default()
        }),
    );

    let (action_set, moveset) = wear(&catalog, &registry, "mystic");

    assert!(
        action_set.special.is_some(),
        "fixture: the body must ADVERTISE a special, or the mismatch under test \
         cannot exist"
    );
    assert!(
        moveset
            .0
            .verbs
            .contains_key(ambition_combat::moveset::SPECIAL_VERB),
        "the action set advertises a special and the derived moveset has no \
         special verb, so pressing it does nothing: {:?}",
        moveset.0.verbs.keys().collect::<Vec<_>>()
    );
}

/// An UNREGISTERED authored persona derives its ranged move too.
///
/// The test above proves it for a character in the prepared registry, where the
/// fold happens at the preparation barrier. Most of the cast is not registered —
/// it lives in the catalog and nothing else — and that path derives its moves
/// separately, in `derive_persona_moveset`.
#[test]
fn an_unregistered_authored_persona_derives_its_ranged_move() {
    let catalog = catalog_granting_melee_and_ranged("drifter");
    let empty = crate::character_runtime::PreparedCharacterRegistry::default();

    let (action_set, moveset) = wear(&catalog, &empty, "drifter");

    assert!(
        action_set.ranged.is_some(),
        "fixture: the catalog row must ADVERTISE ranged, or the mismatch under \
         test cannot exist"
    );
    assert!(
        moveset
            .0
            .verbs
            .contains_key(ambition_combat::moveset::RANGED_VERB),
        "an unregistered catalog persona advertises ranged and derived no ranged \
         verb, so pressing it does nothing: {:?}",
        moveset.0.verbs.keys().collect::<Vec<_>>()
    );
}

/// A catalog whose one authored row grants BOTH a melee and a ranged preset.
fn catalog_granting_melee_and_ranged(id: &str) -> CharacterCatalog {
    use ambition_characters::actor::character_catalog::parse_catalog;
    let ron = format!(
        r#"(
            brain_presets: {{ "stand_still": StandStill }},
            action_set_presets: {{
                "gunbrawler": (
                    move_style: Walk,
                    melee: Some(Swipe(
                        windup_s: 0.28, active_s: 0.08, recover_s: 0.32,
                        damage: 3, reach_px: 40.0,
                    )),
                    ranged: Some(Pistol(speed: 411.0, damage: 7)),
                ),
            }},
            characters: {{
                "{id}": (
                    display_name: "Catalog Says Gunbrawler",
                    spritesheet: "sprites/robot_spritesheet.png",
                    manifest: "sprites/robot_spritesheet.ron",
                    tier: Basement,
                    body_kind: Standard,
                    composition: None,
                    default_brain: "stand_still",
                    default_action_set: "gunbrawler",
                    tags: [],
                ),
            }},
        )"#
    );
    CharacterCatalog::from_data(parse_catalog(&ron))
}

/// X8: the primary player's kit comes from the RESOLVED identity.
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
    use ambition_characters::brain::action_set::{RangedActionSpec, RangedStyle};
    use ambition_characters::brain::ActionSet;
    use ambition_combat::moveset::ActorMoveset;
    use bevy::prelude::*;

    let authored = ActionSet {
        ranged: Some(RangedActionSpec {
            style: RangedStyle::default(),
            speed: 411.0,
            damage: 7,
            flight: None,
            visual: None,
            charge: None,
            refire_s: ambition_characters::brain::action_set::DEFAULT_RANGED_REFIRE_S,
            aim_assist: None,
            discharge: None,
        }),
        ..ActionSet::default()
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.insert_resource(prepared(
        ambition_characters::actor::definition::CharacterDefinition::new(
            "sanic",
            "Sanic",
            "sanic_demo",
        )
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
            ambition_platformer2d_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
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

/// A worn character brings its BODY, not only its kit.
///
/// Three construction paths, three answers, one character.
///
/// and the damage stays. A re-wear is character REPLACEMENT, not construction
/// — the maximum moves to the new identity's and the current value is clamped
/// under it, because refilling would make swapping characters mid-round a free
/// heal.
#[test]
fn a_re_worn_character_moves_the_bodys_health_pool_without_healing_it() {
    use ambition_characters::brain::ActionSet;
    use ambition_combat::moveset::ActorMoveset;
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    let mut heavy = ambition_characters::actor::definition::CharacterDefinition::new(
        "anvil",
        "Anvil",
        "demo_provider",
    );
    heavy.vitals = ambition_characters::actor::definition::Vitals {
        max_health: Some(40),
        mass: Some(6.5),
        knockback_weight: None,
        canonical_height: None,
    };
    app.insert_resource(prepared(heavy));
    app.add_systems(Update, apply_worn_character_gameplay);

    let body = app
        .world_mut()
        .spawn((
            WornCharacter::new("anvil"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            ambition_characters::brain::action_set::IdentityKit::default(),
            ambition_platformer2d_core::BodyKinematics::default(),
            // A body that has already TAKEN four damage off a ten-point pool.
            {
                let mut health = ambition_characters::actor::BodyHealth::new(
                    ambition_characters::actor::Health::new(10),
                );
                health.health.current = 6;
                health
            },
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();

    app.update();

    let health = app
        .world()
        .get::<ambition_characters::actor::BodyHealth>(body)
        .expect("the body kept its health");
    assert_eq!(
        health.health.max, 40,
        "the worn character authored a 40-point pool and the body kept the one \
         its construction happened to give it"
    );
    assert_eq!(
        health.health.current, 6,
        "wearing a character HEALED the body — a re-wear is replacement, not \
         construction, so accumulated damage is the body's and survives"
    );
    assert_eq!(
        app.world()
            .get::<ambition_platformer2d_shared_tangle::body::Mass>(body)
            .map(|m| m.0),
        Some(6.5),
        "the authored mass reached a seated fighter and not a worn one, which is \
         the same character weighing two different amounts"
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
    use ambition_platformer2d_core::{MomentumParams, MotionModelSpec};

    let catalog = test_catalog();
    let momentum = MotionModelSpec::SurfaceMomentum(MomentumParams {
        ground_accel: 1234.0,
        ..Default::default()
    });
    let registry = prepared(
        ambition_characters::actor::definition::CharacterDefinition::new(
            "mary_o", "Mary-O", "demo",
        )
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

/// A from-scratch player and a re-worn one get the SAME host kit.
///
/// `PlayerSimulationBundle::from_scratch` built the host code kit's moves inline
/// and the persona derive built them again in `build_host_code_moveset`. They
/// agreed only because a comment in the derive said they did — *"so this agrees
/// with `PlayerSimulationBundle::from_scratch`, which builds the same code kit and
/// applies the same overlay."*
///
/// An agreement asserted in prose diverges the first time either side learns
/// something: a ranged rule, a second cue family, a new slot. Both call the one
/// constructor now, and this pins that they produce the same moves so the shared
/// call cannot be quietly un-shared later.
#[test]
fn the_spawned_and_the_rewarn_host_kit_are_one_construction() {
    // An ability set that actually grants ATTACK — `basic()` does not, and a
    // probe caught this test comparing two empty contracts because of it.
    let mut abilities = ambition_platformer2d_core::AbilitySet::basic();
    abilities.attack = true;
    let action_set = crate::avatar::bundles::default_player_action_set(abilities);

    let spawned = crate::avatar::PlayerSimulationBundle::from_scratch(
        crate::avatar::primary_player_scratch(
            ambition_platformer2d_core::Vec2::new(0.0, 0.0),
            abilities,
        ),
        ambition_characters::actor::Health::new(10),
    );
    let rewarn = crate::avatar::starting_character::derive_persona_moveset(
        &action_set,
        ambition_characters::brain::RangedExecution::ChargedProjectile,
        None,
    );

    assert!(
        !spawned.moveset.0.moves.is_empty(),
        "the host kit produced NO moves, so comparing the two constructions \
         compares two empty contracts and proves nothing"
    );

    // Compare the whole moveset contract, including cue metadata, not only move
    // IDs and verbs.
    assert_eq!(
        spawned.moveset.0, rewarn,
        "a player that SPAWNED with the host kit and one that RE-WORE it have \
         different moves, so which one you are depends on how you got here"
    );
}

/// A character that authors no physical override RETRACTS the last one, it
/// does not inherit it.
///
/// `apply_to_body` wrote maximum health and mass only when the incoming
/// definition supplied `Some(_)`, so absence meant "keep whatever is there" —
/// which is the outgoing persona's contribution, not the body's. Wear a heavy,
/// high-health duelist and then a persona that authors neither and the body kept
/// 2.0 / 60 forever, with nothing in the world claiming to have decided that
/// . The same shape appeared when a hot reload dropped an
/// authored override from `Some` to `None`.
///
/// The retraction target is the body's OWN answer, captured once on the first
/// projection — never the previous character's, or two swaps would land on
/// character one.
#[test]
fn a_silent_character_gives_back_the_bodys_own_mass_and_health() {
    use ambition_characters::actor::character_catalog::CharacterCatalog;
    use ambition_characters::brain::ActionSet;
    use ambition_combat::moveset::ActorMoveset;
    use bevy::prelude::*;

    const BODY_MAX_HEALTH: i32 = 100;
    const BODY_MASS: f32 = 1.0;
    const DUELIST_MAX_HEALTH: i32 = 60;
    const DUELIST_MASS: f32 = 2.0;

    let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
    for (id, vitals) in [
        (
            "heavy_duelist",
            ambition_characters::actor::definition::Vitals {
                max_health: Some(DUELIST_MAX_HEALTH),
                mass: Some(DUELIST_MASS),
                knockback_weight: None,
                canonical_height: None,
            },
        ),
        // Authors NOTHING physical. This is the ordinary case — most characters
        // never think about mass — which is why silence had to mean the right
        // thing rather than the convenient thing.
        (
            "silent_persona",
            ambition_characters::actor::definition::Vitals::default(),
        ),
    ] {
        let mut definition =
            ambition_characters::actor::definition::CharacterDefinition::new(id, id, "demo");
        definition.vitals = vitals;
        let prepared = crate::character_runtime::prepare_and_finalize_for_test(
            definition,
            &crate::character_runtime::CharacterBindings::default(),
        );
        registry.insert_prepared(prepared.prepared);
    }

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(CharacterCatalog::empty());
    app.insert_resource(registry);
    app.add_systems(Update, apply_worn_character_gameplay);

    let body = app
        .world_mut()
        .spawn((
            WornCharacter::new("heavy_duelist"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(
                BODY_MAX_HEALTH,
            )),
            ambition_platformer2d_shared_tangle::body::Mass(BODY_MASS),
            ambition_platformer2d_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();
    app.update();

    let health_max = |app: &App| {
        app.world()
            .get::<ambition_characters::actor::BodyHealth>(body)
            .unwrap()
            .max()
    };
    let mass = |app: &App| {
        app.world()
            .get::<ambition_platformer2d_shared_tangle::body::Mass>(body)
            .map(|m| m.0)
    };

    assert_eq!(health_max(&app), DUELIST_MAX_HEALTH);
    assert_eq!(mass(&app), Some(DUELIST_MASS));
    assert_eq!(
        app.world().get::<PersonaBaseline>(body).unwrap().displaced,
        crate::character_runtime::DisplacedPhysicals {
            max_health: Some(BODY_MAX_HEALTH),
            mass: Some(Some(BODY_MASS)),
            // Neither persona in this fixture authors a weight, so no persona
            // has displaced one and there is nothing to put back.
            knockback_weight: None,
        }
    );

    *app.world_mut().get_mut::<WornCharacter>(body).unwrap() = WornCharacter::new("silent_persona");
    // AND ASK FOR IT. Writing the identity stopped rebuilding the
    // body: a re-wear is an explicit request, the
    // way Mary-O's powerup already made it. A fixture that mutated the id and
    // expected a rebuild was encoding the contract that split.
    app.world_mut()
        .entity_mut(body)
        .insert(ambition_characters::actor::RecharacterizeBody);
    app.update();

    assert_eq!(
        health_max(&app),
        BODY_MAX_HEALTH,
        "the silent persona inherited the duelist's health pool — absence read \
         as 'keep', so every swap accumulated instead of replacing"
    );
    assert_eq!(
        mass(&app),
        Some(BODY_MASS),
        "same for mass: the body kept weighing what the character it no longer \
         wears said it should"
    );

    // And the record is unchanged, so a THIRD wear still retracts to the body
    // rather than to the persona that just left.
    assert_eq!(
        app.world().get::<PersonaBaseline>(body).unwrap().displaced,
        crate::character_runtime::DisplacedPhysicals {
            max_health: Some(BODY_MAX_HEALTH),
            mass: Some(Some(BODY_MASS)),
            // Neither persona in this fixture authors a weight, so no persona
            // has displaced one and there is nothing to put back.
            knockback_weight: None,
        }
    );
}

/// The other half of retraction: a body that never carried a `Mass` must not
/// acquire one permanently from a character that only briefly authored it.
#[test]
fn a_body_with_no_mass_of_its_own_loses_the_component_again() {
    use ambition_characters::actor::character_catalog::CharacterCatalog;
    use ambition_characters::brain::ActionSet;
    use ambition_combat::moveset::ActorMoveset;
    use bevy::prelude::*;

    let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
    for (id, mass) in [("heavy_duelist", Some(2.0)), ("silent_persona", None)] {
        let mut definition =
            ambition_characters::actor::definition::CharacterDefinition::new(id, id, "demo");
        definition.vitals = ambition_characters::actor::definition::Vitals {
            max_health: None,
            mass,
            knockback_weight: None,
            canonical_height: None,
        };
        let prepared = crate::character_runtime::prepare_and_finalize_for_test(
            definition,
            &crate::character_runtime::CharacterBindings::default(),
        );
        registry.insert_prepared(prepared.prepared);
    }

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(CharacterCatalog::empty());
    app.insert_resource(registry);
    app.add_systems(Update, apply_worn_character_gameplay);

    let body = app
        .world_mut()
        .spawn((
            WornCharacter::new("heavy_duelist"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            ambition_platformer2d_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<ambition_platformer2d_shared_tangle::body::Mass>(body)
            .map(|m| m.0),
        Some(2.0)
    );

    *app.world_mut().get_mut::<WornCharacter>(body).unwrap() = WornCharacter::new("silent_persona");
    // AND ASK FOR IT. Writing the identity stopped rebuilding the
    // body: a re-wear is an explicit request, the
    // way Mary-O's powerup already made it. A fixture that mutated the id and
    // expected a rebuild was encoding the contract that split.
    app.world_mut()
        .entity_mut(body)
        .insert(ambition_characters::actor::RecharacterizeBody);
    app.update();
    assert!(
        app.world()
            .get::<ambition_platformer2d_shared_tangle::body::Mass>(body)
            .is_none(),
        "the body never had a mass of its own, so retraction REMOVES the \
         component rather than inventing an ambient 1.0 — the same distinction \
         `Vitals::mass` documents between authoring 1.0 and saying nothing"
    );
}

/// Fields never authored by any persona are not managed by persona retraction.
/// This prevents the derive from becoming a second authority over unrelated body
/// state such as `max_health`, including during rollback resimulation.
#[test]
fn a_field_no_persona_authored_is_left_to_whoever_else_writes_it() {
    use ambition_characters::actor::character_catalog::CharacterCatalog;
    use ambition_characters::brain::ActionSet;
    use ambition_combat::moveset::ActorMoveset;
    use bevy::prelude::*;

    let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
    for id in ["quiet_one", "quiet_two"] {
        // Neither authors health or mass — the ordinary case for most of the cast.
        let definition =
            ambition_characters::actor::definition::CharacterDefinition::new(id, id, "demo");
        let prepared = crate::character_runtime::prepare_and_finalize_for_test(
            definition,
            &crate::character_runtime::CharacterBindings::default(),
        );
        registry.insert_prepared(prepared.prepared);
    }

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(CharacterCatalog::empty());
    app.insert_resource(registry);
    app.add_systems(Update, apply_worn_character_gameplay);

    let body = app
        .world_mut()
        .spawn((
            WornCharacter::new("quiet_one"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(
                100,
            )),
            ambition_platformer2d_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();
    app.update();

    // SOMEBODY ELSE re-pools the body — a staged fixture, a difficulty rule, an
    // upgrade. Exactly what `rollback_lifecycle_reset::stage_on_floor` does.
    {
        let mut health = app
            .world_mut()
            .get_mut::<ambition_characters::actor::BodyHealth>(body)
            .unwrap();
        health.health.max = 3;
        health.health.current = 3;
    }

    *app.world_mut().get_mut::<WornCharacter>(body).unwrap() = WornCharacter::new("quiet_two");
    // AND ASK FOR IT. Writing the identity stopped rebuilding the
    // body: a re-wear is an explicit request, the
    // way Mary-O's powerup already made it. A fixture that mutated the id and
    // expected a rebuild was encoding the contract that split.
    app.world_mut()
        .entity_mut(body)
        .insert(ambition_characters::actor::RecharacterizeBody);
    app.update();

    assert_eq!(
        app.world()
            .get::<ambition_characters::actor::BodyHealth>(body)
            .unwrap()
            .max(),
        3,
        "the persona derive clobbered a health pool no character ever claimed. \
         It is not this path's field to write, and writing it makes a derive \
         gated on non-rewinding change ticks into a rollback divergence"
    );
    assert_eq!(
        app.world()
            .get::<PersonaBaseline>(body)
            .unwrap()
            .displaced
            .max_health,
        None,
        "and nothing was recorded as displaced, because nothing displaced it"
    );
}

/// A HOT RELOAD that drops an override from `Some` to `None` retracts it.
///
/// The record is what makes this work: `displaced` was captured at the first projection, from
/// the BODY, and it does not move when the definition behind the id does.
#[test]
fn deleting_an_override_in_a_hot_reload_gives_the_body_its_own_numbers_back() {
    use ambition_characters::actor::character_catalog::CharacterCatalog;
    use ambition_characters::brain::ActionSet;
    use ambition_combat::moveset::ActorMoveset;
    use bevy::prelude::*;

    const BODY_MAX_HEALTH: i32 = 100;
    const BODY_MASS: f32 = 1.0;

    /// One publication of `heavy_duelist`, with whatever physicals it authors
    /// this time round.
    fn prepared_duelist(
        vitals: ambition_characters::actor::definition::Vitals,
    ) -> crate::character_runtime::PreparedCharacterDefinition {
        let mut definition = ambition_characters::actor::definition::CharacterDefinition::new(
            "heavy_duelist",
            "heavy_duelist",
            "demo",
        );
        definition.vitals = vitals;
        crate::character_runtime::prepare_and_finalize_for_test(
            definition,
            &crate::character_runtime::CharacterBindings::default(),
        )
        .prepared
    }

    let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
    registry.insert_prepared(prepared_duelist(
        ambition_characters::actor::definition::Vitals {
            max_health: Some(60),
            mass: Some(2.0),
            knockback_weight: None,
            canonical_height: None,
        },
    ));

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(CharacterCatalog::empty());
    app.insert_resource(registry);
    app.add_systems(Update, apply_worn_character_gameplay);

    let body = app
        .world_mut()
        .spawn((
            WornCharacter::new("heavy_duelist"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(
                BODY_MAX_HEALTH,
            )),
            ambition_platformer2d_shared_tangle::body::Mass(BODY_MASS),
            ambition_platformer2d_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();
    app.update();

    let health_max = |app: &App| {
        app.world()
            .get::<ambition_characters::actor::BodyHealth>(body)
            .unwrap()
            .max()
    };
    let mass = |app: &App| {
        app.world()
            .get::<ambition_platformer2d_shared_tangle::body::Mass>(body)
            .map(|m| m.0)
    };
    assert_eq!((health_max(&app), mass(&app)), (60, Some(2.0)));

    // The reload: the SAME character, re-prepared with both lines removed.
    // `insert_prepared` publishes, which moves the generation the derive
    // compares against — the body's `WornCharacter` is never touched.
    app.world_mut()
        .resource_mut::<crate::character_runtime::PreparedCharacterRegistry>()
        .insert_prepared(prepared_duelist(
            ambition_characters::actor::definition::Vitals::default(),
        ));
    app.update();

    assert_eq!(
        (health_max(&app), mass(&app)),
        (BODY_MAX_HEALTH, Some(BODY_MASS)),
        "the reloaded definition authors neither, and the body kept the values \
         the deleted lines had given it — an author editing a RON file would \
         see nothing happen and conclude hot reload does not work"
    );
}

/// A worn body with NO moveset must still get its persona — and get a moveset.
#[test]
fn a_worn_body_carrying_no_moveset_is_still_given_its_persona() {
    use ambition_characters::brain::ActionSet;
    use ambition_combat::moveset::ActorMoveset;
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.add_systems(Update, apply_worn_character_gameplay);

    // The same full body as the derive tests above, MINUS the moveset.
    let e = app
        .world_mut()
        .spawn((
            WornCharacter::new("sanic"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ambition_platformer2d_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();
    assert!(
        app.world().get::<ActorMoveset>(e).is_none(),
        "the fixture's whole point is a body that carries none",
    );
    app.update();

    assert_eq!(
        app.world().get::<Name>(e).unwrap().as_str(),
        "Sanic",
        "the persona must be applied; 'unset' here means the missing moveset \
         filtered the whole body out of the derive",
    );
    assert!(
        matches!(
            app.world().get::<MotionModel>(e),
            Some(MotionModel::SurfaceMomentum(_))
        ),
        "and its movement identity with it",
    );
    assert!(
        app.world().get::<ActorMoveset>(e).is_some(),
        "absence means BUILD one, not skip: the overlay states a repertoire \
         unconditionally, so the body must end up somewhere to put it",
    );
}

/// The minted moveset carries the character's ACTUAL repertoire.
///
/// A reviewer flagged a double-mint hazard in the repair above: both branches of
/// `apply_worn_character_gameplay` minted their own moveset when the body
/// carried none, and `Commands` insertion is DEFERRED — so two branches running
/// in one update would each observe `None`, queue two inserts, and the second
/// would silently discard the first's derivation.
///
/// Two mints are now unrepresentable however the control flow is later rearranged.
///
/// and this test does NOT prove that, which is worth stating plainly. I
/// wrote it as a falsifier and it failed to falsify twice. A goblin cannot
/// express the scenario at all — an ordinary authored character fails the
/// `HostCode`-or-unknown gate, so the ability branch never runs for it. And for
/// the protagonist, which does pass that gate, BOTH branches derive the same
/// non-empty kit from the same persisted `AbilitySet`, so a discarded first
/// derivation is indistinguishable from a kept one by any assertion on the
/// result. A falsifier that passes under its own poison is not a falsifier, and
/// naming that here is cheaper than the next reader re-deriving it.
///
/// What it DOES pin: the minted component carries a real derived repertoire
/// rather than the empty default, and a later ability change refines that same
/// component in place. "An `ActorMoveset` exists" — the first test's assertion —
/// is satisfied by an empty one, and an empty one is exactly what a clobber
/// would leave.
#[test]
fn a_minted_moveset_is_singular_and_carries_the_real_repertoire() {
    use ambition_characters::brain::ActionSet;
    use ambition_combat::moveset::ActorMoveset;
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.add_systems(Update, apply_worn_character_gameplay);

    let e = app
        .world_mut()
        .spawn((
            WornCharacter::new("player"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ambition_platformer2d_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();
    app.update();

    let minted = app
        .world()
        .get::<ActorMoveset>(e)
        .expect("the body must have been given one");
    assert!(
        !minted.0.moves.is_empty(),
        "the protagonist's sandbox kit derives a non-empty repertoire — an \
         EMPTY one here means the component was minted and then replaced by a \
         second empty mint, which is what `is_some()` cannot see",
    );

    // And a later ability change refreshes the SAME component rather than
    // minting beside it — the body carries one now, so there is nothing to mint.
    let before = minted.0.moves.len();
    {
        let mut abilities = app
            .world_mut()
            .get_mut::<ambition_platformer2d_core::BodyAbilities>(e)
            .expect("the movement bundle carries one");
        abilities.abilities.attack = false;
    }
    app.update();
    assert!(
        app.world().get::<ActorMoveset>(e).unwrap().0.moves.len() < before,
        "disabling the melee must shrink the repertoire in place; an unchanged \
         count means the ability branch wrote somewhere other than the body",
    );
}

/// THE PROBE FOR — A BODY'S SHIELD IS ITS OWN CAPABILITY, NOT A PROPERTY OF ITS SPECIAL.
///
/// So *which special do you carry* stood where *can you shield at all* belongs, and any persona
/// holding `AbilitySet::shield` alongside an ordinary special had its guard erased every single
/// frame. It is an independent participant control/action."*
///
/// Three bodies, because one proves nothing on its own:
/// * a shield-capable body with an ORDINARY special keeps the verb (the case the
///   old gate refused);
/// * a shield-capable body with the BUBBLE special keeps it too (the behaviour
///   that must not regress);
/// * the poison — a body with NO shield ability loses it, so this is a gate and
///   not a deletion.
#[test]
fn the_shield_verb_follows_the_ability_not_the_special() {
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::brain::{ActionSet, SpecialActionSpec};
    use ambition_characters::control::ActorControl;
    use bevy::prelude::*;

    let guarding = || {
        let mut frame = ActorControlFrame::neutral();
        frame.shield_held = true;
        frame
    };
    let kit = |special: Option<&str>, shield: bool| {
        let mut abilities = ambition_platformer2d_core::AbilitySet::sandbox_all();
        abilities.shield = shield;
        let actions = ActionSet {
            special: special.map(|key| SpecialActionSpec::Special(key.to_owned())),
            ..ActionSet::default()
        };
        (abilities, actions)
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.add_systems(Update, gate_worn_player_control);

    let mut spawn = |abilities, actions| {
        app.world_mut()
            .spawn((
                ambition_platformer2d_shared_tangle::markers::PlayerEntity,
                WornCharacter::new("sanic"),
                MotionModel::default(),
                ambition_platformer2d_core::BodyAbilities::new(abilities),
                actions,
                ActorControl(guarding()),
            ))
            .id()
    };
    let (abilities, actions) = kit(Some("meteor_kick"), true);
    let ordinary_special = spawn(abilities, actions);
    let (abilities, actions) = kit(Some("bubble_shield"), true);
    let bubble = spawn(abilities, actions);
    let (abilities, actions) = kit(Some("meteor_kick"), false);
    let no_shield_ability = spawn(abilities, actions);

    app.update();

    let guard = |app: &App, e: Entity| app.world().get::<ActorControl>(e).unwrap().0.shield_held;
    assert!(
        guard(&app, ordinary_special),
        "a body that OWNS the shield ability lost its guard because its special \
         is not the bubble shield. Shield is its own participant action — a gate \
         that reads the special key is asking Special whether Shield may happen"
    );
    assert!(
        guard(&app, bubble),
        "the bubble-shield persona lost the guard the old exception gave it"
    );
    assert!(
        !guard(&app, no_shield_ability),
        "a body with NO shield ability kept the guard verb, so nothing is gating \
         it and the assertions above prove nothing"
    );
}

/// The held-item exception survives the move: shield+attack is the universal
/// "throw the held item" gesture, so a body holding an item keeps the shield verb
/// even with no shield ability of its own.
#[test]
fn a_held_item_keeps_the_shield_verb_alive_without_the_ability() {
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::brain::ActionSet;
    use ambition_characters::control::ActorControl;
    use bevy::prelude::*;

    let mut frame = ActorControlFrame::neutral();
    frame.shield_held = true;
    let mut abilities = ambition_platformer2d_core::AbilitySet::sandbox_all();
    abilities.shield = false;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.add_systems(Update, gate_worn_player_control);
    let body = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::markers::PlayerEntity,
            WornCharacter::new("sanic"),
            MotionModel::default(),
            ambition_platformer2d_core::BodyAbilities::new(abilities),
            ActionSet::peaceful(),
            ambition_combat::held_items::HeldItem::new(ambition_characters::brain::HeldItemSpec {
                id: "rock".to_owned(),
                melee: None,
                ranged: None,
                use_behavior: Default::default(),
            }),
            ActorControl(frame),
        ))
        .id();
    app.update();

    assert!(
        app.world().get::<ActorControl>(body).unwrap().0.shield_held,
        "a body holding an item lost the shield half of the throw gesture"
    );
}
