use super::*;
use crate::test_support::test_boss_status;
use crate::BossEncounter;
use crate::BossEncounterPhase;
use ambition_combat::GameplayBanner;
use ambition_encounter::{
    EncounterBeat, EncounterEffect, EncounterGate, EncounterMusicRequest, EncounterParticipant,
    EncounterParticipants, EncounterRole, EncounterScript, EncounterTrigger,
};
use ambition_time::WorldTime;

fn member(hp: i32) -> (BossEncounter, ambition_characters::actor::BodyHealth) {
    test_boss_status(hp, BossEncounterPhase::Phase1)
}

fn member_health(
    app: &App,
    boss: bevy::prelude::Entity,
) -> &ambition_characters::actor::BodyHealth {
    app.world().entity(boss).get().unwrap()
}

fn test_app() -> App {
    let mut app = App::new();
    app.insert_resource(WorldTime::new(1.0 / 60.0, 1.0 / 60.0));
    app.add_message::<EncounterGate>();
    app.init_resource::<GameplayBanner>();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        <EncounterMusicRequest>::default(),
    );
    app.add_systems(Update, tick_encounter_scripts);
    app
}

/// A gate-triggered ForceKill beat kills the named member when the gate fires.
#[test]
fn gate_beat_force_kills_its_member() {
    let mut app = test_app();
    let boss = app.world_mut().spawn(member(9999)).id();
    app.world_mut().spawn((
        EncounterParticipants::new(vec![EncounterParticipant::adopted(
            "cut_rope_boss",
            boss,
            EncounterRole::PrimaryTarget,
        )]),
        EncounterScript::new(vec![EncounterBeat::new(
            EncounterTrigger::Gate("impact".into()),
            vec![EncounterEffect::ForceKill(0)],
        )]),
    ));

    // No gate yet → the boss lives.
    app.update();
    assert!(member_health(&app, boss).alive());

    // Fire the gate → the script force-kills the member.
    app.world_mut().write_message(EncounterGate::new("impact"));
    app.update();
    let status = app.world().entity(boss).get::<BossEncounter>().unwrap();
    assert!(!member_health(&app, boss).alive());
    assert_eq!(member_health(&app, boss).current(), 0);
    assert_eq!(
        status.encounter.as_ref().unwrap().phase,
        BossEncounterPhase::Death
    );
}

/// Beats advance one per fired trigger; a Timer beat fires after its delay,
/// and effects (Banner) apply.
#[test]
fn beats_advance_in_order_with_timer_and_banner() {
    let mut app = test_app();
    let boss = app.world_mut().spawn(member(10)).id();
    app.world_mut().spawn((
        EncounterParticipants::new(vec![EncounterParticipant::adopted(
            "enc_boss",
            boss,
            EncounterRole::PrimaryTarget,
        )]),
        EncounterScript::new(vec![
            EncounterBeat::new(
                EncounterTrigger::Gate("go".into()),
                vec![EncounterEffect::Banner {
                    text: "BEAT 1".into(),
                    secs: 1.0,
                }],
            ),
            EncounterBeat::new(
                EncounterTrigger::Timer(0.1),
                vec![EncounterEffect::ForceKill(0)],
            ),
        ]),
    ));

    // Fire the first gate → beat 0 applies, cursor advances to beat 1.
    app.world_mut().write_message(EncounterGate::new("go"));
    app.update();
    {
        let mut q = app.world_mut().query::<&EncounterScript>();
        assert_eq!(q.single(app.world()).unwrap().cursor(), 1);
    }
    assert!(member_health(&app, boss).alive());

    // Tick past the 0.1s timer (1/60 per frame) → beat 1 fires the kill.
    for _ in 0..10 {
        app.update();
    }
    assert!(!member_health(&app, boss).alive());
    let mut q = app.world_mut().query::<&EncounterScript>();
    assert!(q.single(app.world()).unwrap().done());
}

/// An aligned `FallingHazard` falls onto its target and fires its impact gate.
#[test]
fn falling_hazard_drops_when_aligned_and_fires_impact_gate() {
    let mut app = App::new();
    app.insert_resource(WorldTime::new(1.0 / 60.0, 1.0 / 60.0));
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ae::World::new(
            "t",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(50.0, 50.0),
            Vec::new(),
        )),
    );
    app.add_message::<EncounterGate>();
    app.add_systems(Update, tick_falling_hazards);

    // Target sits directly below the hazard anchor (aligned in x).
    let target = app
        .world_mut()
        .spawn(CenteredAabb::from_center_size(
            ae::Vec2::new(500.0, 500.0),
            ae::Vec2::splat(40.0),
        ))
        .id();
    app.world_mut().spawn((
        CenteredAabb::from_center_size(ae::Vec2::new(500.0, 100.0), ae::Vec2::splat(60.0)),
        FallingHazard {
            size: ae::Vec2::splat(60.0),
            gravity: 1400.0,
            terminal: 920.0,
            align_tolerance: 50.0,
            target,
            impact_gate: "boom".into(),
            vel_y: 0.0,
            dropping: false,
        },
    ));

    let mut fired = false;
    for _ in 0..180 {
        app.update();
        let msgs = app
            .world()
            .resource::<bevy::ecs::message::Messages<EncounterGate>>();
        if msgs
            .iter_current_update_messages()
            .any(|g| g.gate == "boom")
        {
            fired = true;
            break;
        }
    }
    assert!(
        fired,
        "an aligned hazard falls onto its target and fires its impact gate"
    );
    // The hazard despawns on impact.
    let mut q = app.world_mut().query::<&FallingHazard>();
    assert_eq!(q.iter(app.world()).count(), 0, "hazard retires on impact");
}


/// A despawned encounter takes its music claim with it.
///
/// `SetMusic` is an effect, fired once when a beat reaches it, so
/// `release_priority` is reached only while a live script emits
/// `SetMusic(None)`. An encounter that ends without that beat, or despawns
/// when the player leaves its room, would leave the claim standing, and the
/// priority tier outranks room music everywhere.
///
/// The premise is asserted first: while the script is alive, the claim must
/// survive. Otherwise a release that always fires would pass this test.
#[test]
fn a_scripts_music_claim_does_not_outlive_the_script() {
    use ambition_platformer2d_shared_tangle::lifecycle::session_world_component_mut;

    let mut app = test_app();
    let script = app
        .world_mut()
        .spawn((
            EncounterParticipants::new(Vec::new()),
            EncounterScript::new(vec![EncounterBeat::new(
                EncounterTrigger::Gate("never".into()),
                Vec::new(),
            )]),
        ))
        .id();

    // What a `SetMusic(Some(..))` beat leaves behind.
    session_world_component_mut::<EncounterMusicRequest>(app.world_mut())
        .expect("the fixture inserts one")
        .claim_priority(None, super::SCRIPT_MUSIC_OWNER, "smirking_behemoth_intro", 0);

    app.update();
    assert_eq!(
        session_world_component_mut::<EncounterMusicRequest>(app.world_mut())
            .expect("present")
            .desired_track(None),
        Some("smirking_behemoth_intro"),
        "premise: a LIVE script keeps its music claim"
    );

    // The player leaves the room: the encounter is gone.
    app.world_mut().entity_mut(script).despawn();
    app.update();
    assert_eq!(
        session_world_component_mut::<EncounterMusicRequest>(app.world_mut())
            .expect("present")
            .desired_track(None),
        None,
        "the script is gone but its claim still wins, so its track beats room \
         music in every room the player visits"
    );
}

/// Review of OW1 cut 7e: a gate fired in one live room advances only that
/// room's script. Two live rooms each have a boss and a script that
/// force-kills it on `impact`; the gate is fired in #1. The subject: #1's
/// boss dies and #2's lives. With every script hearing every gate, both died.
#[test]
fn a_gate_fired_in_one_live_room_advances_only_that_rooms_script() {
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
    let first = LiveRoomInstance::ACTIVATION.next();
    let second = first.next();
    let mut app = test_app();
    let bosses: Vec<_> = [first, second]
        .into_iter()
        .map(|room| {
            app.world_mut().spawn((RoomInstanceRoot, room));
            let boss = app.world_mut().spawn((member(9999), InRoomInstance(room))).id();
            app.world_mut().spawn((
                InRoomInstance(room),
                EncounterParticipants::new(vec![EncounterParticipant::adopted(
                    "cut_rope_boss",
                    boss,
                    EncounterRole::PrimaryTarget,
                )]),
                EncounterScript::new(vec![EncounterBeat::new(
                    EncounterTrigger::Gate("impact".into()),
                    vec![EncounterEffect::ForceKill(0)],
                )]),
            ));
            boss
        })
        .collect();
    app.world_mut().write_message(EncounterGate::new("impact").in_room(Some(first)));
    app.update();
    assert_eq!(
        bosses.iter().map(|boss| member_health(&app, *boss).alive()).collect::<Vec<_>>(),
        vec![false, true],
        "the gate fired in #1 did not kill only #1's boss"
    );
}

/// ⭐ MUSIC-CANDIDATES: TWO SCRIPTS IN ONE ROOM ARE TWO MUSIC SOURCES. Each
/// script's `SetMusic` is the candidate of that script (its encounter's
/// `SimId`), so a script that ends takes only its own track with it.
///
/// The first script claims `first_theme` on tick 1, and the second claims
/// `second_theme` on tick 4. While both live, the later claim plays. When the
/// second ends, the first's track plays again with no new claim. When both
/// end, nothing plays. With one source for every script, the second's claim
/// replaced the first's, and the ended script's track kept playing.
#[test]
fn two_scripts_in_one_room_are_two_music_candidates() {
    use ambition_platformer2d_shared_tangle::lifecycle::session_world_component;
    use ambition_platformer2d_shared_tangle::sim_id::SimId;
    use ambition_time::SimTick;

    let mut app = test_app();
    app.insert_resource(SimTick(0));
    let script = |id: &str, after: f32, theme: &str| {
        (
            SimId::singleton("encounter", id),
            EncounterParticipants::new(Vec::new()),
            EncounterScript::new(vec![EncounterBeat::new(
                EncounterTrigger::Timer(after),
                vec![EncounterEffect::SetMusic(Some(theme.into()))],
            )]),
        )
    };
    let first = app.world_mut().spawn(script("first", 0.0, "first_theme")).id();
    let second = app.world_mut().spawn(script("second", 0.05, "second_theme")).id();
    let step = |app: &mut App, ticks: usize| {
        for _ in 0..ticks {
            app.world_mut().resource_mut::<SimTick>().0 += 1;
            app.update();
        }
    };
    let playing = |app: &App| {
        session_world_component::<EncounterMusicRequest>(app.world())
            .expect("the fixture inserts one")
            .desired_track(None)
            .map(str::to_owned)
    };

    step(&mut app, 10);
    let both_live = playing(&app);
    app.world_mut().entity_mut(second).despawn();
    step(&mut app, 1);
    let the_second_ended = playing(&app);
    app.world_mut().entity_mut(first).despawn();
    step(&mut app, 1);
    let both_ended = playing(&app);
    assert_eq!(
        (both_live.as_deref(), the_second_ended.as_deref(), both_ended.as_deref()),
        (Some("second_theme"), Some("first_theme"), None),
        "(both scripts live, the second ended, both ended): the track that plays"
    );
}
