use super::*;
use crate::authoring::PlatformerExperienceAuthoring;
use crate::catalogs::AuthoredCatalogFragments;

/// ⛔⛤ **WHERE THE CANDIDATE IS BUILT IS AN ASSERTION NOW, BECAUSE A GUARD
/// ONE CRATE UP LOST ITS SUBJECT WHEN IT MOVED.**
///
/// `reload_publication_is_installed::the_commit_sits_between_the_activation_and_session_adoption`
/// asserts `commit → GameplaySessionSet::Providers` and read that as *"the
/// world is not built from the previous cast"*. A10.5 moved construction
/// out of `Providers` into [`prepare_candidate_platformer_session`], and
/// that arm stayed GREEN: the set still exists, the edge still holds, and
/// the work it was ordered against had left. ⇒ **An assertion that a SET
/// exists cannot see work leaving the set.** Its predecessor went vacuous
/// the other way — one system doing two jobs — and both look identical from
/// the assertion.
///
/// ⭐ So the location is pinned HERE, in the crate that owns both systems,
/// where they are private and nameable by type rather than by set:
///
/// * construction runs `.before(AmbitionGameShellSet::Pending)`, because
///   `advance_pending_route` evaluates the gate there and a session that
///   cannot be built must never retire the one that is playing;
/// * construction runs `.after(PlatformerPreparationSet)`, because without
///   it the shipped handoff took the fallback road every time;
/// * `Providers` holds ADOPTION and not construction, which is the fact the
///   arm upstream now depends on and could not check.
///
/// ⚠ **THIS DOES NOT ANSWER THE OPEN QUESTION IT SITS NEXT TO.** A
/// candidate is prepared from the generation current BEFORE its own
/// activation commits, because the commit must run `.after(Pending)` to see
/// `RouteActivated` while construction runs `.before` it. Whether that is
/// correct by design is a maintainer call —
/// `docs/planning/queue.md`'s `CANDIDATE-GENERATION-ORDER`. This arm pins
/// today's shape so the answer cannot be made moot by a silent move.
#[test]
fn the_candidate_is_built_before_the_router_advances_and_providers_only_adopts() {
    use bevy::ecs::schedule::{NodeId, ScheduleGraph, SystemKey};

    fn key_of<M>(graph: &ScheduleGraph, f: impl IntoSystem<(), (), M>) -> SystemKey {
        let wanted = IntoSystem::into_system(f).system_type();
        graph
            .systems
            .iter()
            .find(|(_, system, _)| system.system_type() == wanted)
            .map(|(key, _, _)| key)
            .expect("the system is registered in this schedule")
    }

    // ⛔⛤ **OPTION, NOT `expect` — AND THE FIRST VERSION OF THIS ARM GOT
    // IT WRONG.** A set node exists only while something references it, so
    // deleting the very edge an assertion is about DELETES THE SET, and an
    // `expect` here then panics one line before the assertion that was
    // supposed to speak. Two of the four poisons below failed through
    // `"the set is registered in this schedule"` and told me nothing. ⇒ An
    // absent set is the SAME failure as a missing edge and must say so in
    // the same sentence.
    fn set_key<S: bevy::ecs::schedule::SystemSet>(
        graph: &ScheduleGraph,
        set: S,
    ) -> Option<bevy::ecs::schedule::SystemSetKey> {
        graph
            .system_sets
            .get_key(bevy::ecs::schedule::SystemSet::intern(&set))
    }

    // ⚠ NOT UPDATED. Running a schedule moves its systems out of the graph
    // into the executor, and the graph then answers "no systems".
    let mut app = App::new();
    app.add_plugins(PlatformerProviderRuntimePlugin);
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules
        .get(Update)
        .expect("the provider registers into `Update`")
        .graph();

    let prepare = key_of(graph, prepare_candidate_platformer_session);
    let adopt = key_of(graph, adopt_candidate_platformer_session);
    let pending = set_key(graph, ambition_game_shell::AmbitionGameShellSet::Pending);
    let preparation = set_key(graph, PlatformerPreparationSet);
    let providers = set_key(graph, GameplaySessionSet::Providers);
    let dependencies = graph.dependency().graph();
    let hierarchy = graph.hierarchy().graph();

    assert!(
        pending.is_some_and(
            |set| dependencies.contains_edge(NodeId::System(prepare), NodeId::Set(set))
        ),
        "candidate construction is not ordered BEFORE \
             `AmbitionGameShellSet::Pending` (set present: {}), so the router \
             can evaluate the activation gate for a candidate nobody has built \
             — and the last-good-world guarantee A10.5 bought is gone",
        pending.is_some()
    );
    assert!(
        preparation.is_some_and(
            |set| dependencies.contains_edge(NodeId::Set(set), NodeId::System(prepare))
        ),
        "candidate construction is not ordered AFTER \
             `PlatformerPreparationSet` (set present: {}); measured when this \
             edge was missing, the shipped handoff took the FALLBACK road \
             every time",
        preparation.is_some()
    );
    assert!(
        providers
            .is_some_and(|set| hierarchy.contains_edge(NodeId::Set(set), NodeId::System(adopt))),
        "`GameplaySessionSet::Providers` does not hold adoption (set \
             present: {}), so the commit's edge to that set names nothing",
        providers.is_some()
    );
    assert!(
        !providers
            .is_some_and(|set| hierarchy.contains_edge(NodeId::Set(set), NodeId::System(prepare))),
        "candidate CONSTRUCTION is back inside `GameplaySessionSet::Providers`. \
             That is the shape the guard upstream assumed and stopped checking — \
             re-read `the_commit_sits_between_the_activation_and_session_adoption` \
             before deciding which of the two is now right"
    );
}

/// The session gate must hold while a candidate is pending, admit
/// an accepted candidate, and refuse when publication cannot finish.
/// A refused activation must not retire the live session.
#[test]
fn the_candidate_gate_refuses_rather_than_retiring_a_playing_session() {
    use ambition_game_shell::{
        ReservedGameplayScopes, ShellActivationGates, ShellActivationId, ShellGateVerdict,
        ShellRouteHolds, ShellRouteId,
    };
    use ambition_platformer2d_publication::{PublicationHandle, PublicationVerdict};
    use ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId;

    const ACTIVATION: ShellActivationId = ShellActivationId(7);

    /// A world holding one prepared candidate whose first room's receipt is
    /// `verdict`, with the route HELD and the scope RESERVED exactly as
    /// `prepare_candidate_platformer_session` leaves them.
    fn world_with_a_candidate(verdict: Option<bool>) -> (bevy::prelude::World, ShellRouteId) {
        let mut world = bevy::prelude::World::new();
        let route = ShellRouteId::new("gated");
        let publication = PublicationHandle(world.spawn_empty().id());
        if let Some(published) = verdict {
            world
                .entity_mut(publication.0)
                .insert(PublicationVerdict { published });
        }
        let root = world.spawn_empty().id();

        let mut reserved = ReservedGameplayScopes::default();
        reserved.reserve(ACTIVATION, SessionScopeId(1));
        world.insert_resource(reserved);
        let mut holds = ShellRouteHolds::default();
        holds.hold(route.clone(), candidate_session_hold(ACTIVATION));
        world.insert_resource(holds);
        world.insert_resource(ShellActivationGates::default());

        world.insert_resource(CandidateSessionSlot(
            Some(PreparedCandidateSession {
                scope: SessionScopeId(1),
                root,
                activation_id: ACTIVATION,
                experience: "gated_experience".into(),
                publication,
                route: route.clone(),
                horizon: Default::default(),
                prepared_from: Default::default(),
                starting_bag: Default::default(),
                mechanics: Default::default(),
                cast: Default::default(),
            }),
            0,
        ));
        (world, route)
    }

    // ── 1. NO CANDIDATE. The arm that used to say `Admit`.
    let mut empty = bevy::prelude::World::new();
    empty.insert_resource(CandidateSessionSlot(None, 0));
    assert_eq!(
        candidate_session_gate(&mut empty),
        ShellGateVerdict::Refuse,
        "an activation this provider is HOLDING has no prepared candidate, and the \
             gate admitted it. The route activates, the shell retires the session that \
             is playing, and nothing was ever built to replace it",
    );

    // ── 2. THE CONTROL: in flight. A gate that refuses everything passes
    //       arms 1 and 4 and wedges every route in the game.
    let (mut in_flight, _) = world_with_a_candidate(None);
    assert_eq!(
        candidate_session_gate(&mut in_flight),
        ShellGateVerdict::Hold,
        "the first room's transaction has not closed and the gate gave a final \
             answer. `Hold` is what lets it be asked again next frame",
    );
    assert!(
        in_flight.resource::<CandidateSessionSlot>().0.is_some(),
        "a `Hold` consumed the candidate it is still waiting on",
    );

    // ── 3. THE POSITIVE CONTROL: published.
    let (mut good, _) = world_with_a_candidate(Some(true));
    assert_eq!(
        candidate_session_gate(&mut good),
        ShellGateVerdict::Admit,
        "the first room published and the gate refused it anyway, so no verified \
             candidate could ever activate",
    );

    // ── 4. REFUSED, AND EVERY RESERVATION RELEASED WITH IT.
    let (mut refused, route) = world_with_a_candidate(Some(false));
    assert_eq!(
        candidate_session_gate(&mut refused),
        ShellGateVerdict::Refuse,
        "the first room FAILED to publish and the gate admitted it: the shell \
             retires the playing session for a world that did not build",
    );
    // ⛔ REVIEW FINDING 3, 2026-09-15: this exit used to release neither the
    // reserved SCOPE nor the route HOLD, and the reservation leaked
    // permanently — the route never activates, so nothing calls `take`.
    assert!(
        refused.resource::<CandidateSessionSlot>().0.is_none(),
        "the refused candidate is still in the slot",
    );
    assert!(
        !refused.resource::<ShellRouteHolds>().is_held(&route),
        "the route is still HELD by a candidate that will never activate, so it \
             is wedged forever rather than free to be asked again",
    );
    assert!(
        refused
            .resource_mut::<ReservedGameplayScopes>()
            .take(ACTIVATION)
            .is_none(),
        "the reserved session scope outlived the candidate that reserved it; \
             nothing calls `take` for a route that never activates, so it leaks",
    );

    // ── 5. STALE: the save changed after the candidate was built. The
    //       candidate is discarded and the route is asked again.
    use ambition_platformer2d_session::durable_horizon::{AmbitionGameSave, PreparedFromSave};
    let live = AmbitionGameSave::default();
    // The control: built from the save as it is. A gate that calls each
    // candidate stale passes the stale assertions and adopts nothing.
    let (mut same, _) = world_with_a_candidate(Some(true));
    same.resource_mut::<CandidateSessionSlot>()
        .0
        .as_mut()
        .unwrap()
        .prepared_from = PreparedFromSave::of(Some(live.data().clone()));
    same.insert_resource(live.clone());
    assert_eq!(
        candidate_session_gate(&mut same),
        ShellGateVerdict::Admit,
        "control: a candidate built from the save as it is was not admitted",
    );
    // Built with no save (the default of the fixture), and now there is
    // one: the two values are not equal.
    let (mut stale, route) = world_with_a_candidate(Some(true));
    stale.insert_resource(live);
    assert_eq!(
        candidate_session_gate(&mut stale),
        ShellGateVerdict::Hold,
        "a candidate built from a save that has changed was not held back: `Admit` \
             adopts a world built from the earlier save beside the later save, and \
             `Refuse` cancels a route that is good",
    );
    assert!(
        stale.resource::<CandidateSessionSlot>().0.is_none(),
        "the stale candidate is still in the slot, so no new one is prepared",
    );
    assert_eq!(
        (
            stale.resource::<CandidateSessionSlot>().stale_discards(),
            same.resource::<CandidateSessionSlot>().stale_discards()
        ),
        (1, 0),
        "the discard of the stale candidate was not recorded one time, or the \
             control recorded one",
    );
    assert!(
        !stale.resource::<ShellRouteHolds>().is_held(&route),
        "the stale candidate still holds the route",
    );
    assert!(
        stale
            .resource_mut::<ReservedGameplayScopes>()
            .take(ACTIVATION)
            .is_none(),
        "the scope that the stale candidate reserved was not released",
    );
}

fn active(line: &str) -> ambition_platformer2d_runtime::SelectedContentIdentity {
    ambition_platformer2d_runtime::SelectedContentIdentity(line.to_string())
}

fn claim(load: &str, line: &str) -> ambition_platformer2d_runtime::PendingGenerationInputs {
    ambition_platformer2d_runtime::PendingGenerationInputs {
        load_id: load.to_string(),
        identity: line.to_string(),
        characters: None,
        bosses: None,
        catalog: None,
        audio: None,
        adaptive_cues: None,
    }
}

/// ⛔⛔ **A CANDIDATE IDENTITY BELONGS TO ONE TRANSACTION AND NOBODY ELSE.**
///
/// ⛤ THIS ARM EXISTS BECAUSE ITS ABSENCE WAS MEASURED. Poisoning the
/// resolution to ignore the claim entirely left 868 tests green across this
/// crate and `ambition_app`: the consumer half of the per-transaction
/// identity had no witness anywhere in the tree.
#[test]
fn a_pending_identity_is_read_only_by_the_transaction_that_claimed_it() {
    // ⭐ THE CONTROL FIRST: with no claim at all, the App's selection is the
    // answer — or "a stranger reads the active identity" is satisfied by a
    // resolution that never returns a candidate.
    assert_eq!(
        content_identity_for(Some(&active("pack 1 cfp1:aa")), None, "shell.game.1"),
        Some("pack 1 cfp1:aa".to_string()),
        "with no pending claim the active selection must be the answer"
    );
    // The owner reads the candidate.
    assert_eq!(
        content_identity_for(
            Some(&active("pack 1 cfp1:aa")),
            Some(&claim("shell.game.7", "pack 2 cfp1:bb")),
            "shell.game.7",
        ),
        Some("pack 2 cfp1:bb".to_string()),
        "the claiming transaction did not read its own candidate identity"
    );
    // ⛔ THE ASSERTION THE WHOLE ARM IS FOR — and both loads name the SAME
    // ROUTE, because two generations can target one route and a route
    // comparison would pass this.
    assert_eq!(
        content_identity_for(
            Some(&active("pack 1 cfp1:aa")),
            Some(&claim("shell.game.7", "pack 2 cfp1:bb")),
            "shell.game.8",
        ),
        Some("pack 1 cfp1:aa".to_string()),
        "an unrelated transaction was fingerprinted against a candidate it \
             never prepared — the rollback timeline contract compares exactly \
             this identity"
    );
    // ⚠ ABSENT IS A REAL ANSWER: a composition with no content pack selects
    // none, and a stranger's claim must not invent one for it.
    assert_eq!(
        content_identity_for(
            None,
            Some(&claim("shell.game.7", "pack 2 cfp1:bb")),
            "shell.game.8"
        ),
        None,
        "a stranger's claim became the answer for an App that selected no pack"
    );
}

#[test]
fn authoring_installation_registers_preparation_resources_synchronously() {
    let mut app = App::new();
    PlatformerExperienceAuthoring::new(
        "fixture",
        "fixture_gameplay",
        "Fixture",
        "Provider fixture",
        "Prepare fixture",
        AuthoredCatalogFragments::new("fixture_character", "fixture"),
    )
    .install(&mut app, || -> PreparedPlatformerSource {
        unreachable!("the fixture never receives a preparation request")
    });

    assert!(
        app.world()
            .contains_resource::<PlatformerAuthoredCatalogRegistry>()
    );
    assert!(
        app.world()
            .contains_resource::<PlatformerStreamingReadiness>()
    );
    assert!(
        app.world()
            .contains_resource::<PreparedPlatformerSessions>()
    );
}

#[cfg(feature = "audio")]
#[test]
fn cue_only_preparation_does_not_report_deliberate_silence() {
    use ambition_game_shell::{
        ProviderPreparationPlan, ShellCommand, ShellHostConfiguration, ShellRouteCatalog,
        ShellRouteId, ShellRouteSpec, ShellRouter,
    };
    use bevy::ecs::system::SystemState;

    let authored = ron::from_str(
        r#"(
            cues: [(
                id: "ambient", relative_volume: 1.0,
                media: Inline(asset_root: "audio", bpm: 120.0, beats_per_bar: 4.0,
                    sections: [(id: "song", duration_beats: 4.0, looped: true,
                        sources: [(layer_id: "full", path: "ambient.ogg")])]),
                layers: [(id: "full", slot: 0)],
                states: [(id: "song", section_id: "song", gains: [(layer_id: "full", gain: 1.0)])],
            )],
        )"#,
    )
    .unwrap();
    let cues =
        ambition_audio::cue_program::PreparedMusicCues::prepare(authored, &Default::default())
            .unwrap();
    for (has_cues, expects_cues) in [(true, true), (true, false), (false, false)] {
        let mut app = App::new();
        let characters = character_registry(false, CHARACTER_B);
        app.insert_resource(characters.assemble().unwrap().catalog)
            .insert_resource(characters)
            .init_resource::<ambition_sprite_sheet::SpriteContent>()
            .init_resource::<ContentEpochSequence>()
            .init_resource::<PreparedPlatformerSessions>()
            .init_resource::<PlatformerStreamingReadiness>()
            .add_message::<ambition_load::LoadCommand>();
        let mut fragments = PlatformerAuthoredCatalogRegistry::default();
        let mut declaration = AuthoredCatalogFragments::new("alpha", "same-provider");
        declaration.expects_adaptive_cues = expects_cues;
        fragments.try_register("fixture", declaration).unwrap();
        app.insert_resource(fragments);
        let mut audio = ambition_audio::catalog::AudioCatalogRegistry::default();
        audio
            .register(
                ambition_audio::catalog::AudioCatalogFragment::new("same-provider", None, None)
                    .unwrap(),
            )
            .unwrap();
        app.insert_resource(audio);
        let mut adaptive = ambition_audio::music::AdaptiveMusicCatalogRegistry::default();
        if has_cues {
            adaptive.register("same-provider", cues.clone()).unwrap();
        }
        app.insert_resource(adaptive);

        let mut routes = ShellRouteCatalog::default();
        routes.register(
            ShellRouteSpec::new("game", "fixture").preparing_with(
                ProviderPreparationPlan::new("Prepare", "ready", "Ready")
                    .required("publish", "Publish"),
            ),
        );
        let mut prepared = PreparedSessionRegistry::default();
        let transaction = ShellRouter::default()
            .apply(
                ShellCommand::GoTo(ShellRouteId::new("game")),
                &routes,
                &ShellHostConfiguration::default(),
                &mut ambition_load::LoadCoordinator::default(),
                &mut prepared,
            )
            .into_iter()
            .find_map(|event| match event {
                ShellEvent::PreparationRequested(transaction) => Some(transaction),
                _ => None,
            })
            .unwrap();
        app.insert_resource(prepared);
        let mut preparation = SystemState::<PlatformerPreparation>::new(app.world_mut());
        let identity = preparation
            .get_mut(app.world_mut())
            .unwrap()
            .prepare(&transaction, fixture_source(128.0))
            .expect("the cue-only session prepares");
        let session = app
            .world()
            .resource::<PreparedPlatformerSessions>()
            .peek(&identity)
            .unwrap();
        assert!(!session.report.music_ready);
        assert!(!session.report.procedural_sfx_ready);
        assert_eq!(session.report.adaptive_cues_ready, has_cues);
        assert_eq!(session.report.deliberate_silence, !has_cues);
        assert_eq!(
            session
                .music
                .cues
                .as_ref()
                .and_then(|cues| cues.cue("ambient"))
                .is_some(),
            has_cues
        );
    }
}

const CHARACTER_A: &str = r#"(
        brain_presets: { "idle": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "alpha": (
                display_name: "Alpha", manifest: "alpha.ron",
                tier: MainHall, body_kind: Standard, composition: None,
                default_brain: "idle", default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;

const CHARACTER_B: &str = r#"(
        brain_presets: { "idle": StandStill },
        action_set_presets: { "peaceful": (move_style: Float) },
        characters: {
            "beta": (
                display_name: "Beta", manifest: "beta.ron",
                tier: MainHall, body_kind: Standard, composition: None,
                default_brain: "idle", default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;

fn fixture_source(width: f32) -> PreparedPlatformerSource {
    let room = ambition_platformer2d_world::rooms::RoomSpec::new(
        "same-room",
        ambition_platformer2d_core::World::new(
            "same-room",
            ambition_platformer2d_core::Vec2::new(width, 128.0),
            ambition_platformer2d_core::Vec2::new(16.0, 16.0),
            Vec::new(),
        ),
    );
    let room_set = ambition_platformer2d_world::rooms::RoomSet::from_parts_or_panic(
        "same-room",
        vec![room],
        Vec::new(),
    );
    PreparedPlatformerSource::new(
        "same-provider",
        room_set.clone(),
        ambition_platformer2d_core::RoomGeometry(room_set.activation_spec().world.clone()),
        ambition_platformer2d_session::initial_body::StartingCharacter::new("alpha"),
    )
}

/// A DIRECT ROOT REFUSES TO BE THE SECOND ONE.
///
/// ⭐ The refusal is what makes [`install_direct_session_root`] a ROAD
/// rather than a shorthand. Three compositions hand-built this spawn and
/// none of them could have said no: a plugin that spawns a root
/// unconditionally at build time is exactly the *"build-time root
/// coexisting with an activation's"* coexistence
/// `live_session_world_root`'s doc records as having panicked once.
///
/// ⚠ BOTH ARMS ABORT BEFORE PREPARATION, which is why they need no
/// registries: a composition error is not something a content author can
/// fix, so it cannot wait for a `ContentDiagnostic`.
#[test]
#[should_panic(expected = "already holds a SessionRoot")]
fn a_second_direct_root_is_refused_rather_than_spawned_beside_the_first() {
    let mut app = App::new();
    app.world_mut()
        .spawn(ambition_platformer2d_shared_tangle::lifecycle::SessionRoot(
            SessionScopeId(0),
        ));
    let _ = install_direct_session_root(
        &mut app,
        fixture_source(128.0),
        &AuthoredCatalogFragments::new("alpha", "same-provider"),
    );
}

/// A SHELL-GATED APP HAS NO DIRECT ROOT TO INSTALL.
///
/// `SessionGatedSimulation` IS the declaration that gameplay belongs to
/// shell-routed activations. A direct root at scope 0 standing beside the
/// activation's is two canonical roots for one world, and every
/// `SessionWorldRef` in the tree is a `Single` that matches neither.
#[test]
#[should_panic(expected = "shell-gated App")]
fn a_shell_gated_app_refuses_a_direct_root() {
    let mut app = App::new();
    app.init_resource::<ambition_platformer2d_shared_tangle::lifecycle::SessionGatedSimulation>();
    let _ = install_direct_session_root(
        &mut app,
        fixture_source(128.0),
        &AuthoredCatalogFragments::new("alpha", "same-provider"),
    );
}

fn isolated_room_fixture_source(room_id: &str) -> PreparedPlatformerSource {
    let room = ambition_platformer2d_world::rooms::RoomSpec::new(
        room_id,
        ambition_platformer2d_core::World::new(
            room_id,
            ambition_platformer2d_core::Vec2::new(128.0, 128.0),
            ambition_platformer2d_core::Vec2::new(16.0, 16.0),
            Vec::new(),
        ),
    );
    let room_set = ambition_platformer2d_world::rooms::RoomSet::from_parts_or_panic(
        room_id,
        vec![room],
        Vec::new(),
    );
    PreparedPlatformerSource::new(
        "same-provider",
        room_set.clone(),
        ambition_platformer2d_core::RoomGeometry(room_set.activation_spec().world.clone()),
        ambition_platformer2d_session::initial_body::StartingCharacter::new("alpha"),
    )
}

fn two_room_fixture_source(active_room: &str) -> PreparedPlatformerSource {
    let first = ambition_platformer2d_world::rooms::RoomSpec::new(
        "same-room",
        ambition_platformer2d_core::World::new(
            "same-room",
            ambition_platformer2d_core::Vec2::new(128.0, 128.0),
            ambition_platformer2d_core::Vec2::new(16.0, 16.0),
            Vec::new(),
        ),
    );
    let second = ambition_platformer2d_world::rooms::RoomSpec::new(
        "second-room",
        ambition_platformer2d_core::World::new(
            "second-room",
            ambition_platformer2d_core::Vec2::new(160.0, 128.0),
            ambition_platformer2d_core::Vec2::new(16.0, 16.0),
            Vec::new(),
        ),
    );
    let mut room_set = ambition_platformer2d_world::rooms::RoomSet::from_parts_or_panic(
        "same-room",
        vec![first, second],
        Vec::new(),
    );
    assert!(
        room_set.set_activation_by_id(active_room).is_some(),
        "fixture asked for room `{active_room}`, which this set does not hold"
    );
    PreparedPlatformerSource::new(
        "same-provider",
        room_set.clone(),
        ambition_platformer2d_core::RoomGeometry(room_set.activation_spec().world.clone()),
        ambition_platformer2d_session::initial_body::StartingCharacter::new("alpha"),
    )
}

fn character_registry(
    reverse: bool,
    beta_ron: &str,
) -> ambition_characters::actor::character_catalog::CharacterCatalogRegistry {
    use ambition_characters::actor::character_catalog::{
        CharacterCatalogFragment, CharacterCatalogRegistry,
    };
    let a = CharacterCatalogFragment::from_ron("provider-a", Some("alpha"), CHARACTER_A).unwrap();
    let b = CharacterCatalogFragment::from_ron("provider-b", Some("beta"), beta_ron).unwrap();
    let mut registry = CharacterCatalogRegistry::default();
    if reverse {
        registry.register(b).unwrap();
        registry.register(a).unwrap();
    } else {
        registry.register(a).unwrap();
        registry.register(b).unwrap();
    }
    registry
}

fn staging_registry(
    reverse: bool,
) -> ambition_platformer2d_publication::construction::RoomContentStagingRegistry {
    let mut registry =
        ambition_platformer2d_publication::construction::RoomContentStagingRegistry::default();
    let register_a = |registry: &mut ambition_platformer2d_publication::construction::RoomContentStagingRegistry| {
            registry
                .register(
                    "same-room",
                    "provider-a",
                    "fixture-a",
                    "fixture-a.v1",
                    |_| Vec::new(),
                )
                .unwrap();
        };
    let register_b = |registry: &mut ambition_platformer2d_publication::construction::RoomContentStagingRegistry| {
            registry
                .register(
                    "same-room",
                    "provider-b",
                    "fixture-b",
                    "fixture-b.v1",
                    |_| Vec::new(),
                )
                .unwrap();
        };
    if reverse {
        register_b(&mut registry);
        register_a(&mut registry);
    } else {
        register_a(&mut registry);
        register_b(&mut registry);
    }
    registry
}

fn fixture_content(
    source: PreparedPlatformerSource,
    characters: &ambition_characters::actor::character_catalog::CharacterCatalogRegistry,
    staging: &ambition_platformer2d_publication::construction::RoomContentStagingRegistry,
) -> PreparedContent {
    fixture_content_with_recipes(source, characters, staging, None)
}

fn fixture_content_with_recipes(
    source: PreparedPlatformerSource,
    characters: &ambition_characters::actor::character_catalog::CharacterCatalogRegistry,
    staging: &ambition_platformer2d_publication::construction::RoomContentStagingRegistry,
    construction_recipes: Option<String>,
) -> PreparedContent {
    let authored = AuthoredCatalogFragments::new("alpha", "same-provider");
    let snapshot_schema =
        ambition_platformer2d_runtime::rollback::RollbackRegistry::default().schema_fingerprint();
    let mut epochs = ContentEpochSequence::default();
    prepare_platformer_content(
        source,
        &authored,
        Some(characters),
        None,
        Some(staging),
        MechanicalRegistries {
            construction_recipes,
            ..Default::default()
        },
        snapshot_schema,
        &mut epochs,
    )
    .unwrap()
}

/// Choosing a character other than the default is not a content error.
///
/// `capture_scene --character <id>` has a usage example in its own header — `--character
/// npc_pirate_admiral` — and it had never worked for any id, that one included (found by
/// trying to photograph the player wearing an older incarnation of itself).
///
/// What preparation legitimately owns is that the selection RESOLVES, which
/// the second half of this asserts.
#[test]
fn a_starting_character_other_than_the_default_prepares() {
    let characters = character_registry(false, CHARACTER_B);
    let staging = staging_registry(false);
    let authored = AuthoredCatalogFragments::new("alpha", "same-provider");
    let snapshot_schema =
        ambition_platformer2d_runtime::rollback::RollbackRegistry::default().schema_fingerprint();

    let prepare = |selected: &str| {
        let base = fixture_source(128.0);
        let source = PreparedPlatformerSource::new(
            "same-provider",
            base.room_set().clone(),
            ambition_platformer2d_core::RoomGeometry(
                base.room_set().activation_spec().world.clone(),
            ),
            ambition_platformer2d_session::initial_body::StartingCharacter::new(
                selected.to_string(),
            ),
        );
        let mut epochs = ContentEpochSequence::default();
        prepare_platformer_content(
            source,
            &authored,
            Some(&characters),
            None,
            Some(&staging),
            MechanicalRegistries::default(),
            snapshot_schema,
            &mut epochs,
        )
    };

    assert!(
        prepare("beta").is_ok(),
        "selecting a real character that is not the provider default must \
             prepare — the whole point of a playable cast is that it has members \
             other than the first one"
    );
    let rejected = prepare("no_such_character")
        .expect_err("a selection that names nothing must still be fatal");
    assert_eq!(rejected.section, "provider.defaults");
}

#[test]
fn prepared_content_is_order_independent_and_dump_stable() {
    let first_characters = character_registry(false, CHARACTER_B);
    let second_characters = character_registry(true, CHARACTER_B);
    let first_staging = staging_registry(false);
    let second_staging = staging_registry(true);
    let first = fixture_content(fixture_source(128.0), &first_characters, &first_staging);
    let second = fixture_content(fixture_source(128.0), &second_characters, &second_staging);
    assert_eq!(first.fingerprint(), second.fingerprint());
    assert_eq!(first.deterministic_dump(), second.deterministic_dump());
}

#[test]
fn prepared_content_detects_geometry_and_character_action_changes() {
    let characters = character_registry(false, CHARACTER_B);
    let staging = staging_registry(false);
    let baseline = fixture_content(fixture_source(128.0), &characters, &staging);
    let changed_geometry = fixture_content(fixture_source(192.0), &characters, &staging);
    assert_ne!(baseline.fingerprint(), changed_geometry.fingerprint());

    let changed_character_ron = CHARACTER_B.replace("move_style: Float", "move_style: Slither");
    let changed_characters = character_registry(false, &changed_character_ron);
    let changed_character = fixture_content(fixture_source(128.0), &changed_characters, &staging);
    assert_ne!(baseline.fingerprint(), changed_character.fingerprint());
}

/// ⛔⛤ **THE TWO DERIVED REGISTRIES ARE BOUND TRANSITIVELY, AND I CLAIMED
/// OTHERWISE.** `b3839b28f`'s message said `CharacterCatalog` and
/// `BrainProfileRegistry` *"are not in `PreparedContentIdentity` either"*.
/// **That is wrong, and measurement is what says so.**
///
/// ⭐ Both are built by ONE assembly from `CharacterCatalogRegistry`'s
/// fragments — `registry.rs` builds `CharacterCatalog::from_data(..)` and
/// `BrainProfileRegistry { profiles: autonomous_profiles_for_registry }` from
/// the same parse, with the comment *"same values, same keys, so the two
/// cannot disagree"* — and the production road never inserts either as a
/// standalone authority (`prepare_declared_cast`: *"Not
/// `insert_resource(CharacterCatalog)`: that would be a second authority on
/// what the cast is"*). The identity binds
/// `CharacterCatalogRegistry::canonical_fragments()`, which is the SOURCE
/// RON, so anything derived from it moves when it moves.
///
/// ⚠ **SO THIS ARM PINS A TRANSITIVE FACT THAT WAS ONLY REASONED.**
/// `prepared_content_detects_geometry_and_character_action_changes` already
/// covers an ACTION-SET change; the controller-policy half — the part
/// `BrainProfileRegistry` actually carries — had no arm, so the reasoning
/// above was the only thing holding it.
#[test]
fn a_controller_policy_change_moves_the_fingerprint() {
    let staging = staging_registry(false);
    let baseline = fixture_content(
        fixture_source(128.0),
        &character_registry(false, CHARACTER_B),
        &staging,
    );

    // ⛔ THE PREMISE: the substitution must actually match. A fixture edit
    // that hits nothing and a passing test look identical, and this
    // repository has been bitten by a zero-match substitution twice.
    let changed_ron = CHARACTER_B.replace(
        r#""idle": StandStill"#,
        r#""idle": Wanderer(speed: 40.0, aggressiveness: 0.5)"#,
    );
    assert_ne!(
        changed_ron, CHARACTER_B,
        "the brain-preset substitution matched nothing, so this arm compares \
             a fixture with itself"
    );

    let changed = fixture_content(
        fixture_source(128.0),
        &character_registry(false, &changed_ron),
        &staging,
    );
    assert_ne!(
        baseline.fingerprint(),
        changed.fingerprint(),
        "two compositions whose controller policy differs share one \
             PreparedContentIdentity — and that identity is what the rollback \
             timeline contract compares to decide whether a snapshot from one may \
             be restored into the other",
    );
}

/// The hand a character's art makes a gesture with is simulation input:
/// it moves where a shot is born and where a petter stands
/// (`BodyLandmarks::gesture_hand`). Two casts that differ only in one
/// row's `gesture_hand` are different content.
#[test]
fn a_gesture_hand_change_moves_the_fingerprint() {
    use ambition_characters::actor::GestureHand;
    let staging = staging_registry(false);
    let near = character_registry(false, CHARACTER_B);
    let far_ron = CHARACTER_B.replace(
        r#"display_name: "Beta","#,
        r#"display_name: "Beta", gesture_hand: Far,"#,
    );
    let far = character_registry(false, &far_ron);
    // The premise: the two casts state two hands for the one row. A
    // substitution that matched nothing compares a fixture with itself.
    let hand_of =
        |registry: &ambition_characters::actor::character_catalog::CharacterCatalogRegistry| {
            registry
                .assemble()
                .expect("the fixture cast assembles")
                .catalog
                .get("beta")
                .expect("the row")
                .gesture_hand
        };
    assert_eq!(
        (hand_of(&near), hand_of(&far)),
        (GestureHand::Near, GestureHand::Far)
    );

    let baseline = fixture_content(fixture_source(128.0), &near, &staging);
    let changed = fixture_content(fixture_source(128.0), &far, &staging);
    assert_ne!(
        baseline.fingerprint(),
        changed.fingerprint(),
        "two casts whose row states a different gesture hand share one PreparedContentIdentity",
    );
}

/// The construction recipe table decides how authoritative entities are
/// built, so a change to it is a change to the content. This was DOCUMENTED
/// as contributing to the fingerprint long before it did — `prepare_platformer_content`
/// did not take the registry at all.
#[test]
fn a_construction_recipe_schema_change_moves_the_fingerprint() {
    let characters = character_registry(false, CHARACTER_B);
    let staging = staging_registry(false);

    let baseline = fixture_content_with_recipes(
        fixture_source(128.0),
        &characters,
        &staging,
        Some(construction_dump("actor-construction-v1")),
    );
    let bumped_schema = fixture_content_with_recipes(
        fixture_source(128.0),
        &characters,
        &staging,
        Some(construction_dump("actor-construction-v2")),
    );
    assert_ne!(
        baseline.fingerprint(),
        bumped_schema.fingerprint(),
        "a recipe schema bump is a content change"
    );

    let absent = fixture_content_with_recipes(fixture_source(128.0), &characters, &staging, None);
    assert_ne!(
        baseline.fingerprint(),
        absent.fingerprint(),
        "having a recipe table at all differs from having none"
    );
}

/// ...but registration ORDER does not, because the registry is ordered
/// storage. Both halves matter: a fingerprint that ignores real changes is
/// useless, and one that reacts to plugin insertion order is unusable.
#[test]
fn construction_registration_order_does_not_move_the_fingerprint() {
    let characters = character_registry(false, CHARACTER_B);
    let staging = staging_registry(false);

    let forward = fixture_content_with_recipes(
        fixture_source(128.0),
        &characters,
        &staging,
        Some(construction_dump_ordered(false)),
    );
    let reversed = fixture_content_with_recipes(
        fixture_source(128.0),
        &characters,
        &staging,
        Some(construction_dump_ordered(true)),
    );
    assert_eq!(forward.fingerprint(), reversed.fingerprint());
}

/// A relation's wiring behaviour can change while its kind and owner stay
/// the same. The schema id is what makes that visible to content identity,
/// so it must reach the fingerprint just as a recipe's does.
#[test]
fn a_relation_schema_change_moves_the_fingerprint() {
    let characters = character_registry(false, CHARACTER_B);
    let staging = staging_registry(false);

    let baseline = fixture_content_with_recipes(
        fixture_source(128.0),
        &characters,
        &staging,
        Some(relation_dump("v1")),
    );
    let bumped = fixture_content_with_recipes(
        fixture_source(128.0),
        &characters,
        &staging,
        Some(relation_dump("v2")),
    );
    assert_ne!(baseline.fingerprint(), bumped.fingerprint());
}

fn relation_dump(schema: &str) -> String {
    let mut registry = ambition_platformer2d_construction::ActorConstructionRegistry::default();
    registry
        .try_register_relation(
            ambition_platformer2d_construction::relation_grudge(),
            "ambition_platformer2d_actor_monolith",
            "aggression",
            schema,
        )
        .unwrap();
    registry.deterministic_dump()
}

/// A real registry, dumped — the same value the app path contributes.
fn construction_dump(schema: &str) -> String {
    let mut registry = ambition_platformer2d_construction::ActorConstructionRegistry::default();
    registry
        .try_register_recipe(
            ambition_platformer2d_construction::recipe_staged_actor(),
            "ambition_platformer2d_actor_monolith",
            "content-staging",
            schema,
        )
        .unwrap();
    registry.deterministic_dump()
}

fn construction_dump_ordered(reverse: bool) -> String {
    let mut registry = ambition_platformer2d_construction::ActorConstructionRegistry::default();
    let ids = [
        ambition_platformer2d_construction::recipe_staged_actor(),
        ambition_platformer2d_construction::recipe_summoned_minion(),
    ];
    let ids: Vec<_> = if reverse {
        ids.into_iter().rev().collect()
    } else {
        ids.into_iter().collect()
    };
    for id in ids {
        registry
            .try_register_recipe(id, "ambition_platformer2d_actor_monolith", "src", "v1")
            .unwrap();
    }
    registry.deterministic_dump()
}

/// ⛔⛤ **TWO SESSIONS PREPARED UNDER DIFFERENT CONTENT PACKS ARE DIFFERENT
/// CONTENT GENERATIONS, AND UNTIL 2026-09-11 THEY WERE NOT.**
///
/// Every other section of a `PreparedContent` is an App REGISTRY, so the
/// authored content PACK — move tables, item catalog, encounter waves —
/// reached the game without reaching this fingerprint. The consequence is
/// not academic: `RollbackTimelineContract` stores a `PreparedContentIdentity`
/// and the GGRS session refuses *"prepared content changed while the session
/// was active"* by comparing exactly that, so the guard could not see the
/// content most likely to change during development.
///
/// ⚠ THE SIBLING ARM BELOW IS WHAT MAKES THIS ONE MEAN SOMETHING: two
/// preparations that agree on everything INCLUDING the pack still share a
/// fingerprint, so this is not "any two preparations differ".
#[test]
fn two_preparations_under_different_content_packs_are_different_generations() {
    let characters = character_registry(false, CHARACTER_B);
    let staging = staging_registry(false);
    let authored = AuthoredCatalogFragments::new("alpha", "same-provider");
    let snapshot_schema =
        ambition_platformer2d_runtime::rollback::RollbackRegistry::default().schema_fingerprint();
    let mut epochs = ContentEpochSequence::default();
    let prepare = |pack: Option<&str>, epochs: &mut ContentEpochSequence| {
        prepare_platformer_content(
            fixture_source(128.0),
            &authored,
            Some(&characters),
            None,
            Some(&staging),
            MechanicalRegistries {
                content_pack: pack.map(str::to_string),
                ..Default::default()
            },
            snapshot_schema,
            epochs,
        )
        .unwrap()
    };

    let first = prepare(Some("ambition 1.0.0 aaaaaaaaaaaaaaaa"), &mut epochs);
    let second = prepare(Some("ambition 1.0.0 bbbbbbbbbbbbbbbb"), &mut epochs);
    assert_ne!(
        first.fingerprint(),
        second.fingerprint(),
        "a different content pack did not reach the prepared content's identity"
    );

    // ⛔ AND ABSENT IS ITS OWN ANSWER, distinct from either pack: a
    // composition that selected no pack is not the same generation as one
    // that selected one.
    let none = prepare(None, &mut epochs);
    assert_ne!(none.fingerprint(), first.fingerprint());
    assert_ne!(none.fingerprint(), second.fingerprint());
}

/// ⭐ THE CONTROL. The same pack twice is the same definition, so the arm
/// above is about the PACK and not about preparation being nondeterministic.
#[test]
fn two_preparations_under_one_content_pack_share_a_definition_identity() {
    let characters = character_registry(false, CHARACTER_B);
    let staging = staging_registry(false);
    let authored = AuthoredCatalogFragments::new("alpha", "same-provider");
    let snapshot_schema =
        ambition_platformer2d_runtime::rollback::RollbackRegistry::default().schema_fingerprint();
    let mut epochs = ContentEpochSequence::default();
    let prepare = |epochs: &mut ContentEpochSequence| {
        prepare_platformer_content(
            fixture_source(128.0),
            &authored,
            Some(&characters),
            None,
            Some(&staging),
            MechanicalRegistries {
                content_pack: Some("ambition 1.0.0 aaaaaaaaaaaaaaaa".to_string()),
                ..Default::default()
            },
            snapshot_schema,
            epochs,
        )
        .unwrap()
    };
    assert_eq!(
        prepare(&mut epochs).fingerprint(),
        prepare(&mut epochs).fingerprint()
    );
}

/// A landmark is a simulation input that is compiled in: two preparations
/// whose landmark material differs are different generations, and two that
/// agree are one. `the_provider_road_carries_this_builds_landmark_digest`
/// shows that the material is the tables this build embeds.
#[test]
fn two_preparations_under_different_landmark_tables_are_different_generations() {
    let characters = character_registry(false, CHARACTER_B);
    let staging = staging_registry(false);
    let authored = AuthoredCatalogFragments::new("alpha", "same-provider");
    let snapshot_schema =
        ambition_platformer2d_runtime::rollback::RollbackRegistry::default().schema_fingerprint();
    let mut epochs = ContentEpochSequence::default();
    let mut prepare = |landmarks: &str| {
        prepare_platformer_content(
            fixture_source(128.0),
            &authored,
            Some(&characters),
            None,
            Some(&staging),
            MechanicalRegistries {
                sprite_content: Some(landmarks.to_string()),
                ..Default::default()
            },
            snapshot_schema,
            &mut epochs,
        )
        .unwrap()
        .fingerprint()
    };
    assert_eq!(
        prepare("one"),
        prepare("one"),
        "control: the same tables, one generation"
    );
    assert_ne!(
        prepare("one"),
        prepare("other"),
        "two builds whose characters keep their hands in different places shared an identity",
    );
}

#[test]
fn sequential_preparations_share_definition_identity_but_not_epoch() {
    let characters = character_registry(false, CHARACTER_B);
    let staging = staging_registry(false);
    let authored = AuthoredCatalogFragments::new("alpha", "same-provider");
    let snapshot_schema =
        ambition_platformer2d_runtime::rollback::RollbackRegistry::default().schema_fingerprint();
    let mut epochs = ContentEpochSequence::default();
    let first = prepare_platformer_content(
        fixture_source(128.0),
        &authored,
        Some(&characters),
        None,
        Some(&staging),
        MechanicalRegistries::default(),
        snapshot_schema,
        &mut epochs,
    )
    .unwrap();
    let second = prepare_platformer_content(
        fixture_source(128.0),
        &authored,
        Some(&characters),
        None,
        Some(&staging),
        MechanicalRegistries::default(),
        snapshot_schema,
        &mut epochs,
    )
    .unwrap();

    assert_eq!(first.fingerprint(), second.fingerprint());
    assert_ne!(first.epoch(), second.epoch());
    assert_eq!(
        first.epoch(),
        ambition_platformer2d_runtime::ContentEpoch(1)
    );
    assert_eq!(
        second.epoch(),
        ambition_platformer2d_runtime::ContentEpoch(2)
    );
}

#[test]
fn reload_candidate_is_detached_and_equivalent_reload_preserves_epoch() {
    let characters = character_registry(false, CHARACTER_B);
    let staging = staging_registry(false);
    let active = fixture_content(fixture_source(128.0), &characters, &staging);
    let schema = active.snapshot_schema();

    let equivalent =
        prepare_world_replacement_candidate(&active, fixture_source(128.0), schema).unwrap();
    assert_eq!(equivalent.fingerprint(), active.fingerprint());
    assert_eq!(equivalent.epoch(), active.epoch());

    let changed =
        prepare_world_replacement_candidate(&active, fixture_source(192.0), schema).unwrap();
    assert_ne!(changed.fingerprint(), active.fingerprint());
    assert_eq!(
        changed.epoch(),
        active.epoch(),
        "candidate is not committed yet"
    );
    assert_eq!(active.source().geometry().0.size.x, 128.0);

    let committed = changed.with_epoch(ambition_platformer2d_runtime::ContentEpoch(9));
    assert_eq!(
        committed.epoch(),
        ambition_platformer2d_runtime::ContentEpoch(9)
    );
    assert_eq!(
        active.epoch(),
        ambition_platformer2d_runtime::ContentEpoch(1)
    );
}

#[test]
fn replacement_ignores_the_live_active_room_cursor() {
    let characters = character_registry(false, CHARACTER_B);
    let staging = staging_registry(false);
    let active = fixture_content(two_room_fixture_source("same-room"), &characters, &staging);
    let live_room_candidate = two_room_fixture_source("second-room");
    let candidate =
        prepare_world_replacement_candidate(&active, live_room_candidate, active.snapshot_schema())
            .unwrap();

    assert_eq!(candidate.fingerprint(), active.fingerprint());
    assert_eq!(candidate.epoch(), active.epoch());
    assert_eq!(candidate.source().active_room_id(), "same-room");
}

#[test]
fn failed_reload_candidate_leaves_active_content_unchanged() {
    let characters = character_registry(false, CHARACTER_B);
    let staging = staging_registry(false);
    let active = fixture_content(two_room_fixture_source("same-room"), &characters, &staging);
    let before = active.identity();

    let error = prepare_world_replacement_candidate(
        &active,
        isolated_room_fixture_source("second-room"),
        active.snapshot_schema(),
    )
    .unwrap_err();

    assert_eq!(error.section, "world.definition-active-room");
    assert_eq!(active.identity(), before);
    assert_eq!(active.source().active_room_id(), "same-room");
}

#[test]
fn ordinary_room_movement_does_not_change_prepared_identity() {
    let characters = character_registry(false, CHARACTER_B);
    let staging = staging_registry(false);
    let content = fixture_content(two_room_fixture_source("same-room"), &characters, &staging);
    let before = content.identity();
    // Room movement is the live room root's definition (OW1 cut 5e),
    // which the prepared source does not hold.
    let mut live_room = content.source().instantiate_live_room();
    live_room.definition = content
        .source()
        .room_set()
        .definition_by_id("second-room")
        .expect("the fixture set holds `second-room`");

    assert_eq!(
        content.source().room_set().spec(live_room.definition).id,
        "second-room"
    );
    assert_eq!(content.identity(), before);
    assert_eq!(content.source().active_room_id(), "same-room");
}

#[test]
fn mutable_live_requests_do_not_change_prepared_identity() {
    let characters = character_registry(false, CHARACTER_B);
    let staging = staging_registry(false);
    let content = fixture_content(fixture_source(128.0), &characters, &staging);
    let before = content.identity();
    let mut live = content.source().instantiate_live();
    live.requests
        .encounter_music
        .claim_priority(None, "test", "runtime-boss", 0);
    assert_eq!(content.identity(), before);
}

#[test]
fn packed_bank_readiness_completes_only_the_matching_streamable_transaction() {
    let load_id = ambition_load::LoadId::new("provider-load");
    let other_load_id = ambition_load::LoadId::new("other-load");
    let mut loads = ambition_load::LoadCoordinator::default();
    loads.apply(ambition_load::LoadCommand::Begin(
        ambition_load::LoadPlanSpec::new(load_id.clone(), "Provider load"),
    ));
    loads.apply(ambition_load::LoadCommand::Begin(
        ambition_load::LoadPlanSpec::new(other_load_id.clone(), "Other load"),
    ));

    let mut pending = PlatformerStreamingReadiness::default();
    pending.pending_packed_sfx.insert(
        load_id.clone(),
        PendingPackedSfxReadiness {
            provider_id: "provider".to_owned(),
        },
    );
    pending.pending_packed_sfx.insert(
        other_load_id.clone(),
        PendingPackedSfxReadiness {
            provider_id: "other".to_owned(),
        },
    );

    let mut app = App::new();
    app.add_message::<ambition_load::LoadCommand>()
        .insert_resource(loads)
        .insert_resource(pending)
        .insert_resource(ambition_audio::catalog::SfxBankRegistry::default())
        .add_systems(Update, update_streamable_packed_sfx);
    app.update();
    assert!(
        app.world_mut()
            .resource_mut::<Messages<ambition_load::LoadCommand>>()
            .drain()
            .next()
            .is_none()
    );

    app.world_mut()
        .resource_mut::<ambition_audio::catalog::SfxBankRegistry>()
        .register(
            "provider",
            BTreeMap::from([(ambition_sfx::SfxId::from_static("provider.ready"), 7)]),
        )
        .expect("fixture bank fragment registers");
    app.update();
    let commands = app
        .world_mut()
        .resource_mut::<Messages<ambition_load::LoadCommand>>()
        .drain()
        .collect::<Vec<_>>();
    assert_eq!(commands.len(), 1);
    assert!(matches!(
        &commands[0],
        ambition_load::LoadCommand::SetWorkState {
            load_id: completed_load,
            work_id,
            state: ambition_load::LoadWorkState::Complete,
        } if completed_load == &load_id && work_id.as_str() == PREPARE_PACKED_SFX_WORK_ID
    ));
    let readiness = app.world().resource::<PlatformerStreamingReadiness>();
    assert!(!readiness.pending_packed_sfx.contains_key(&load_id));
    assert!(readiness.pending_packed_sfx.contains_key(&other_load_id));
}
