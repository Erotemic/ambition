use super::*;

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
    ambition_characters::actor::character_catalog::CharacterCatalog(data)
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
fn wearing_sanic_inserts_momentum_then_unwearing_removes_it() {
    // Q16 test (c): wearing a momentum character makes the box ride
    // surfaces; re-wearing a non-momentum character REMOVES the model so a
    // stale MotionModel never rides a chain the new character can't (the
    // render-refresh clobber gotcha in reverse). Removal restores the
    // axis-swept path byte-for-byte — the absence of the component IS the
    // default.
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

    // Re-wear the protagonist (axis-swept) → the model is removed entirely.
    let mut queue = bevy::ecs::world::CommandQueue::default();
    {
        let mut commands = Commands::new(&mut queue, app.world());
        apply_worn_motion_model(&catalog, &mut commands, entity, "player");
    }
    queue.apply(app.world_mut());
    assert!(
        app.world().get::<MotionModel>(entity).is_none(),
        "unwearing a momentum character restores the axis-swept path (no MotionModel)"
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
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            // The persisted capability set the overlay rebuilds a HostCode /
            // unknown kit from — every real player body carries it.
            crate::actor::BodyAbilities::new(ambition_engine_core::AbilitySet::sandbox_all()),
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
    // removed and the name follows.
    *app.world_mut().get_mut::<WornCharacter>(e).unwrap() = WornCharacter::new("player");
    app.update();
    assert!(
        app.world().get::<MotionModel>(e).is_none(),
        "re-wearing a non-momentum character removes the stale movement model"
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
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            crate::actor::BodyAbilities::new(ambition_engine_core::AbilitySet::sandbox_all()),
        ))
        .id();
    app.update();

    let expected = ambition_engine_core::surface::SurfaceMotion::Riding {
        on: ambition_engine_core::surface::SurfaceRef::Chain(0),
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
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            // The persisted capability set the overlay rebuilds a HostCode /
            // unknown kit from — every real player body carries it.
            crate::actor::BodyAbilities::new(ambition_engine_core::AbilitySet::sandbox_all()),
        ))
        .id();
    app.update(); // Added → derives SurfaceMomentum for sanic.
    assert!(app.world().get::<MotionModel>(e).is_some());

    // No identity change: subsequent frames must not re-run the wear. Prove it
    // by clobbering the model and confirming the un-changed system leaves it.
    app.world_mut().entity_mut(e).remove::<MotionModel>();
    app.update();
    assert!(
        app.world().get::<MotionModel>(e).is_none(),
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
    use ambition_characters::brain::{ActionSet, RangedActionSpec};
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install_test_catalog(&mut app);
    app.add_systems(Update, apply_worn_character_gameplay);
    let e = app
        .world_mut()
        .spawn((
            WornCharacter::new("npc_pirate_admiral"),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            // The persisted capability set the overlay rebuilds a HostCode /
            // unknown kit from — every real player body carries it.
            crate::actor::BodyAbilities::new(ambition_engine_core::AbilitySet::sandbox_all()),
        ))
        .id();
    app.update();
    assert!(
        matches!(
            app.world().get::<ActionSet>(e).unwrap().ranged,
            Some(RangedActionSpec::Pistol { .. })
        ),
        "wearing the pirate derives its authored pistol into the ActionSet"
    );

    // Re-wear a DIFFERENT known character: the kit fully swaps — no stale pistol.
    *app.world_mut().get_mut::<WornCharacter>(e).unwrap() = WornCharacter::new("goblin");
    app.update();
    assert!(
        !matches!(
            app.world().get::<ActionSet>(e).unwrap().ranged,
            Some(RangedActionSpec::Pistol { .. })
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
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            // The persisted capability set the overlay rebuilds a HostCode /
            // unknown kit from — every real player body carries it.
            crate::actor::BodyAbilities::new(ambition_engine_core::AbilitySet::sandbox_all()),
        ))
        .id();
    app.update();
    assert!(
        matches!(
            app.world().get::<ActionSet>(e).unwrap().ranged,
            Some(RangedActionSpec::Pistol { .. })
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
        matches!(set.ranged, Some(RangedActionSpec::Bolt { .. })),
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
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            crate::actor::BodyAbilities::new(ambition_engine_core::AbilitySet::sandbox_all()),
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
            && matches!(set.ranged, Some(RangedActionSpec::Bolt { .. })),
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
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            crate::actor::BodyAbilities::new(ambition_engine_core::AbilitySet::sandbox_all()),
        ))
        .id();
    app.update();

    let initial = app.world().get::<ActionSet>(entity).unwrap();
    assert!(matches!(initial.melee, Some(MeleeActionSpec::Swipe(_))));
    assert!(matches!(
        initial.ranged,
        Some(RangedActionSpec::Bolt { .. })
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
        matches!(refreshed.ranged, Some(RangedActionSpec::Bolt { .. })),
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
