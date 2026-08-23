//! The capability, mounted by a real composition through the public SDK.
//!
//! The unit tests prove each contribution in isolation: a schema the compiler
//! accepts, an action the registry holds, rollback state a host can install,
//! facts the log records. What none of them shows is the thing an author
//! actually does — write ONE module that mounts the capability and declares
//! what it brings, and have the composition install all of it.
//!
//! `ambition_platformer2d` is a DEV-dependency. A mechanic must not depend on the
//! facade that mounts it; the capability's real closure is still EIGHT
//! foundation crates. This file is the consumer, not the capability.

use ambition_platformer2d::app::{GameModule, ModuleDraft, ModuleManifest, PlatformerApp};
use ambition_platformer2d::participant::{InstalledActions, SemanticActionId, GAMEPLAY_CONTEXT};
use bevy::prelude::*;

/// One character, so the composition has somebody to start as.
///
/// a `playable(..)` whose starting character names nobody is refused with
/// *"the host would prepare NOTHING and wait forever"* — which is the assembly
/// check doing its job and is why this constant exists rather than an empty
/// roster and a hopeful string.
const ROSTER: &str = r#"(
    brain_presets: { "still": StandStill },
    action_set_presets: {
        "walk_only": (
            move_style: Walk,
            melee: None,
            ranged: None,
            special: None,
        ),
    },
    characters: {
        "pulse_walker": (
            display_name: "Walker",
            spritesheet: "minimal_walker.png",
            manifest: "minimal_walker_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "still",
            default_action_set: "walk_only",
            tags: ["player"],
        ),
    },
)"#;

/// A room to stand in, so the module below is a GAME rather than a fragment.
fn a_room_to_stand_in() -> ambition_platformer2d::world::rooms::RoomSpec {
    use ambition_platformer2d::world::prelude::*;
    let size = Vec2::new(640.0, 360.0);
    let floor_top = 320.0;
    let world = AuthoredWorld::new(
        "Pulse Room",
        size,
        Vec2::new(64.0, floor_top - 64.0),
        // `Block::solid(name, MIN, size)` takes a MIN CORNER, not a centre —
        // the reference fixture shipped a centre here for months and its walker
        // fell through the floor forever while the host reported Running.
        vec![Block::solid(
            "floor",
            Vec2::new(0.0, floor_top),
            Vec2::new(size.x, 40.0),
        )],
    );
    RoomSpec::new("pulse_room", world)
}

/// A game that wants the pulse. This is the whole integration an author writes.
struct GameWithPulse;

impl GameModule for GameWithPulse {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest::new("game_with_pulse")
    }

    fn define(&self, module: &mut ModuleDraft) {
        module
            .experience("game_with_pulse")
            .launcher_route("home")
            .gameplay_route("game_with_pulse/play")
            .characters(ROSTER)
            .no_audio()
            .playable(
                "Game With Pulse",
                "A room and a shockwave",
                "pulse_walker",
                "pulse_room",
                vec![a_room_to_stand_in()],
            );
        module
            // behaviour
            .capability(capability_demo::PulsePlugin::default())
            // + the semantic action it contributes
            .actions(&[capability_demo::PULSE_ACTION])
            // + what it needs rewound
            .requires_rollback(capability_demo::REQUIRED_ROLLBACK)
            // + AND the registration that satisfies it. Without this half the
            //   module could say what must rewind and had no supported way to
            //   supply it, so a rollback game mounting this capability could not
            //   be composed at all.
            .provides_rollback::<capability_demo::PulseCooldown>(
                capability_demo::PULSE_CAPABILITY,
                capability_demo::ROLLBACK_STATE,
                |cooldown| u64::from(cooldown.remaining_ticks),
            )
            // and the BODY. The mechanic's whole observable effect is a
            // velocity change, so a rewind that restored only the cooldown
            // would resimulate from a body still carrying the old push.
            .provides_rollback::<capability_demo::PulseBody>(
                capability_demo::PULSE_CAPABILITY,
                capability_demo::BODY_ROLLBACK_STATE,
                |body| body.vel.x.to_bits() as u64,
            );
    }
}

/// All FOUR contributions arrive from one declaration, and the composition
/// SUCCEEDS.
///
/// It could not do better, because there was no supported way for a module to
/// PROVIDE the registration it declared as required. `provides_rollback` is that
/// way, and this is now a game that composes.
#[test]
fn one_module_mounts_the_capability_and_the_composition_installs_everything() {
    let mut app = App::new();
    let outcome = PlatformerApp::headless()
        .rollback(2)
        .mount(GameWithPulse)
        .install_into(&mut app);

    assert!(
        outcome.is_ok(),
        "a rollback-enabled game that mounts the capability AND provides its \
         rewind registration must compose: {outcome:?}"
    );

    // The ACTION reached the composition's vocabulary, beside the engine's.
    let actions = app
        .world()
        .get_resource::<InstalledActions>()
        .expect("the composition builds an action registry");
    assert_eq!(
        actions.get(SemanticActionId("pulse")).map(|d| d.capability),
        Some(capability_demo::PULSE_CAPABILITY),
        "the capability's action is in the game's vocabulary"
    );
    assert!(
        actions
            .for_context(GAMEPLAY_CONTEXT)
            .any(|d| d.id.0 == "jump"),
        "and the engine's own is there unasked, so one query answers both"
    );

    // The BEHAVIOUR is installed: the plugin's own resources exist.
    assert!(
        app.world()
            .get_resource::<capability_demo::PulseProfiles>()
            .is_some(),
        "the mechanic's tuning resource is present, so `capability(..)` ran"
    );

    // And the ROLLBACK state is really registered — not merely required.
    let registry = app
        .world()
        .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
        .expect("a rollback composition builds a registry");
    assert!(
        registry
            .missing_required_state(capability_demo::REQUIRED_ROLLBACK)
            .is_empty(),
        "the capability's rewind state is registered, so a rewind restores the \
         cooldown rather than letting the action fire twice from one charge"
    );
}

/// And the refusal still works — the useful half of the old test, kept as
/// its own negative case rather than doing duty as the positive one.
///
/// A module that declares what must rewind and does NOT provide it is refused
/// at assembly, naming the state and the capability's own reason. That is the
/// composition catching a desync instead of a player discovering it.
#[test]
fn declaring_required_rollback_without_providing_it_is_refused() {
    struct ForgetfulGame;

    impl GameModule for ForgetfulGame {
        fn manifest(&self) -> ModuleManifest {
            ModuleManifest::new("forgetful_game")
        }

        fn define(&self, module: &mut ModuleDraft) {
            module
                .experience("forgetful_game")
                .launcher_route("home")
                .gameplay_route("forgetful_game/play")
                .characters(ROSTER)
                .no_audio()
                .playable(
                    "Forgetful Game",
                    "Declares a rewind requirement and never supplies it",
                    "pulse_walker",
                    "pulse_room",
                    vec![a_room_to_stand_in()],
                );
            // requires, and deliberately does not provide.
            module
                .capability(capability_demo::PulsePlugin::default())
                .requires_rollback(capability_demo::REQUIRED_ROLLBACK);
        }
    }

    let mut app = App::new();
    let outcome = PlatformerApp::headless()
        .rollback(2)
        .mount(ForgetfulGame)
        .install_into(&mut app);

    let message = format!("{outcome:?}");
    assert!(outcome.is_err(), "the omission must refuse: {message}");
    assert!(
        message.contains(capability_demo::ROLLBACK_STATE),
        "the missing rewind state is named by the refusal: {message}"
    );
    assert!(
        message.contains("twice from one charge"),
        "and carries the capability's own reason, not a generic complaint: {message}"
    );
}

/// The content half, through the facade a game actually uses.
///
/// The capability offers a schema; the game adds it to the engine's registry and
/// validates its own authored file. No `ambition_content_pack` import here —
/// only `ambition_platformer2d::content`.
#[test]
fn a_game_validates_the_capabilitys_authored_content_through_the_facade() {
    use ambition_platformer2d::content::{
        compile, engine_schemas, AssetsUnchecked, ContentPackDraft, ContentPackManifest,
        ModuleNamespace, PackId, PackVersion, SchemaId, SchemaVersion, SourceDeclaration,
    };

    let root = std::env::temp_dir().join("capability_demo_sdk/pack");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp pack");
    std::fs::write(
        root.join("pulse.ron"),
        r#"(profiles: [(name: "heavy", radius: 180.0, force: 800.0, cooldown_ticks: 60)])"#,
    )
    .expect("write");

    let mut registry = engine_schemas();
    registry
        .register(capability_demo::pulse_schema())
        .expect("the capability's schema is new to the engine's registry");

    let draft = ContentPackDraft::read_manifest(
        root,
        ContentPackManifest {
            id: PackId("game_with_pulse".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("game_with_pulse".into()),
            requires: Vec::new(),
            sources: vec![SourceDeclaration {
                path: "pulse.ron".into(),
                schema: SchemaId::new(capability_demo::PULSE_SCHEMA),
                version: SchemaVersion(1),
            }],
        },
    )
    .expect("draft reads");

    let pack = compile(&draft, &registry, &AssetsUnchecked).expect("the game's tuning compiles");
    let profiles = pack
        .lowered::<Vec<capability_demo::PulseProfile>>(&SchemaId::new(
            capability_demo::PULSE_SCHEMA,
        ))
        .expect("a Runtime schema lowers what the game will run");
    assert_eq!(profiles[0].name, "heavy");
    assert_eq!(profiles[0].radius, 180.0);
}
