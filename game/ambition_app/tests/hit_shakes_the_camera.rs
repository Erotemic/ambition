#![cfg(feature = "rl_sim")]
//! nothing here is injected. The hits are the duel's own, at the strength
//! the authored movesets produce; the only edit is the removal, and the test
//! asserts the removal took before it believes anything else.

use ambition_app::AmbitionSim;
use ambition_app::{
    AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::characters::actor::BodyCombat;
use ambition_platformer2d::platformer::camera_ease::CameraShakeState;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;

/// How long to watch the duel. The fighters trade from the first seconds; this
/// is generous enough to see several exchanges without turning the suite slow.
const BOUT_FRAMES: usize = 600;

/// What the bout produced, gathered every frame because hitstop is a few frames
/// wide and the camera decays out from under a once-at-the-end read.
#[derive(Debug, Default)]
struct Bout {
    /// The hardest freeze any body served.
    hardest_hitstop: f32,
    frames_with_a_connect: u32,
    /// The loudest the camera got.
    peak_shake_px: f32,
    /// Home avatars still in the world during the bout. Must stay zero.
    home_avatars_seen: usize,
    /// Frames on which the MATCH clock was held by a connect.
    frozen_frames: u32,
}

fn watch_a_duel_with_no_home_avatar() -> (Bout, f32) {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room("duel_arena"),
    )
    .expect("the sim harness builds in the duel arena");

    // Let the room stage its fighters and its observing player.
    for _ in 0..60 {
        sim.step(AgentAction::default());
    }

    // make it a fight nobody is watching from a home body. Removing the
    // marker is the whole edit: the bodies, the brains, the grudge and the
    // movesets are the room's own.
    let world = sim.world_mut();
    let mut homes =
        world.query_filtered::<bevy::prelude::Entity, bevy::prelude::With<PrimaryPlayer>>();
    let home_bodies: Vec<_> = homes.iter(world).collect();
    assert!(
        !home_bodies.is_empty(),
        "the duel arena staged no home avatar at all, so removing one proves \
         nothing — the fixture stopped modelling the room it is named after"
    );
    for entity in home_bodies {
        world.entity_mut(entity).remove::<PrimaryPlayer>();
    }

    let mut bout = Bout::default();
    for _ in 0..BOUT_FRAMES {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        let mut bodies = world.query::<&BodyCombat>();
        let mut connected = false;
        for combat in bodies.iter(world) {
            bout.hardest_hitstop = bout.hardest_hitstop.max(combat.hitstop_timer);
            connected |= combat.hitstop_timer > 0.0;
        }
        if connected {
            bout.frames_with_a_connect += 1;
        }
        let mut still_home =
            world.query_filtered::<bevy::prelude::Entity, bevy::prelude::With<PrimaryPlayer>>();
        bout.home_avatars_seen = bout.home_avatars_seen.max(still_home.iter(world).count());
        bout.peak_shake_px = bout
            .peak_shake_px
            .max(world.resource::<CameraShakeState>().amplitude_px);
        let tick = *world.resource::<ambition_platformer2d::time::SimTick>();
        if world
            .resource::<ambition_platformer2d::combat::impact_hitstop::ImpactHitstop>()
            .is_freezing(tick)
        {
            bout.frozen_frames += 1;
        }
    }

    // read from the running app rather than restated: a route that retunes its
    // hitlag retunes what counts as a hard hit, and a literal here would be a
    // second number agreeing with the first by coincidence.
    let reference = sim
        .world_mut()
        .get_resource::<ambition_platformer2d::combat::feel::Platformer2dFeelTuningMonolith>()
        .expect("the composed sim installs the monolith's feel tuning")
        .hitlag_time;
    (bout, reference)
}

/// Two fighters nobody is playing shake the screen when they connect.
#[test]
fn a_fight_between_two_bodies_nobody_is_playing_shakes_the_camera() {
    let (bout, reference) = watch_a_duel_with_no_home_avatar();

    assert_eq!(
        bout.home_avatars_seen, 0,
        "a home avatar came back during the bout, so this is no longer the \
         no-`PrimaryPlayer` world the claim rests on: {bout:?}"
    );
    assert!(
        bout.frames_with_a_connect > 0,
        "no body in the duel arena was ever in hitlag across {BOUT_FRAMES} \
         frames — the fixture staged a standoff, so it is measuring nothing: \
         {bout:?}"
    );
    let weakest = reference * ambition_platformer2d::engine_core::hit_response::MIN_HITLAG_SCALE;
    assert!(
        bout.hardest_hitstop > weakest,
        "the hardest connect in the whole duel was {} s, at or under the {weakest} s \
         weakest connect the hitlag law admits, so the dead zone is entitled to \
         swallow it and this test cannot speak to the shake at all: {bout:?}",
        bout.hardest_hitstop
    );
    assert!(
        bout.peak_shake_px > 0.0,
        "two fighters traded real blows in a world with no home avatar and the \
         camera never moved a pixel — the hit shake is gated on `PrimaryPlayer` \
         again, or it has gone back to living in the app's player-presentation \
         system where the standalone smash binary cannot reach it: {bout:?}"
    );
    // ⭐⭐ AND THE MATCH FREEZE IS THE SHAKE'S SIBLING, pinned in the same bout
    // for the same reason: it is the other beat a connect buys, and NOTHING in
    // this suite watched it fire in a real match. It reads `ResolvedBodyHit`
    // now — a channel with two producers on two damage roads, registered in a
    // plugin neither of them owns — so the hand-listed chain wants a fixture
    // that only passes if the whole thing is wired.
    assert!(
        bout.frozen_frames > 0,
        "two fighters connected repeatedly and the match clock never once \
         stopped — the resolved-hit channel is not reaching the freeze: {bout:?}"
    );
}
