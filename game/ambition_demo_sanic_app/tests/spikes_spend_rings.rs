//! **The spikes cost rings, not the run** — driven through the real headless
//! demo, on the real speedway geometry.
//!
//! Jon, from play: *"Sanic hitting the spikes should not be an insta kill. It
//! should hurt him and knock out his rings. Sanic should only die after getting
//! hit with 0 rings."*
//!
//! ⛔ **the ring shield was never the broken part.** It is implemented and
//! pinned (`ambition_demo_sanic::tests` — a hit with rings never reaches HP, it
//! costs every ring, and they scatter as real collectible pickups). What did not
//! happen was the HIT: the mid-course strip was authored as a `HazardBlock`,
//! which has exactly one outcome — teleport to spawn — so a runner carrying 40
//! rings was returned to the start line with his purse intact and the shield
//! never heard about it. The strip is a `DamageVolume` now.
//!
//! ⚠ **this asserts the four cases as a SET, and each one poisons a different
//! wrong fix.** Making the pit hurt instead of reset passes the first case and
//! fails the third. Removing the strip passes the first two and fails all four
//! by making them vacuous, which is why every case also proves the strip was
//! reached. Dropping the super form's exemption passes three and fails the last.

use ambition_demo_sanic::{PIT_LEFT_X, SUPER_SANIC_CHARACTER_ID};
use ambition_demo_sanic_app::build_demo_app;
use ambition_platformer2d::characters::actor::{BodyHealth, BodyWallet, WornCharacter};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

/// Left edge of the authored strip (`mid_spikes`, 96px wide, sitting ON the
/// floor at y 656..672 — it is not over a void and never was).
const SPIKES_LEFT_X: f32 = 5648.0;
/// Where a walk-in starts: on the floor, clear of the strip, close enough that
/// a few dozen frames of held Right reach it.
const RUN_UP_X: f32 = 5600.0;
/// The floor's top edge (`FLOOR_TOP`), the y a standing body's feet rest on.
const FLOOR_TOP: f32 = 672.0;

/// What one scripted walk-in produced.
#[derive(Debug)]
struct Outcome {
    hp: i32,
    rings: i32,
    scattered: usize,
    /// True once the body has been put back at the act's spawn — the observable
    /// a reset and a death share, and the one the spikes must NOT produce while
    /// the player still has rings.
    sent_home: bool,
    /// How many times the engine published its authoritative *"the local
    /// player's attempt ended"* fact. ⚠ this is what separates the two ways of
    /// being sent home: the pit's reset and a lethal hit both move the body, and
    /// only [`ActorDiedMessage`] says the difference out loud.
    deaths: usize,
    /// Furthest right the body's leading EDGE got, so a case cannot pass by
    /// never arriving. ⚠ the edge, not the centre: a 30px-wide body touches the
    /// strip with 15px still to go, and a centre-based check called the
    /// zero-ring death "vacuous" at max_x=5640 of a strip starting at 5648.
    max_right: f32,
}

/// Every `ActorDiedMessage` observed this run.
#[derive(Resource, Default)]
struct DeathsSeen(usize);

fn count_deaths(
    mut seen: ResMut<DeathsSeen>,
    mut deaths: MessageReader<ambition_platformer2d::actors::ActorDiedMessage>,
) {
    seen.0 += deaths.read().count();
}

fn boot() -> App {
    let mut app = build_demo_app();
    app.init_resource::<DeathsSeen>();
    app.add_systems(Last, count_deaths);
    // A fixed-tick host without a pinned clock runs a machine-speed-dependent
    // number of ticks per update, and the same script would then cover a
    // different distance on every run.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    for _ in 0..600 {
        app.update();
        if player(&mut app).is_some() {
            return app;
        }
    }
    panic!("the demo never activated a playable body");
}

fn player(app: &mut App) -> Option<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryPlayer>>();
    q.iter(app.world()).next()
}

fn pos(app: &mut App) -> Vec2 {
    let mut q = app
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    q.iter(app.world())
        .next()
        .map(|k| k.pos)
        .unwrap_or(Vec2::NAN)
}

/// Relocate through the engine's discrete-transit authority (ADR 0024) rather
/// than poking `BodyKinematics.pos`. Sanic rides the momentum kernel, so his
/// motion model carries surface state a raw position write would leave
/// describing the old place.
fn displace(app: &mut App, to: Vec2) {
    let mut query = app.world_mut().query_filtered::<(
        ae::BodyClusterQueryData,
        &mut ambition_platformer2d::actors::features::MotionModel,
    ), With<PrimaryPlayer>>();
    let world = app.world_mut();
    let (mut cluster_item, mut motion_model) = query
        .iter_mut(world)
        .next()
        .expect("gameplay has a primary player");
    let mut clusters = cluster_item.as_clusters_mut();
    ae::movement::transit_body(
        &mut motion_model,
        &mut clusters,
        to,
        ae::movement::TransitVelocity::Zero,
    );
}

/// **Hold Right, in a way that does not depend on which composition this is.**
///
/// ⛔⛔ **THIS WAS A TWO-ARM `#[cfg(feature = "input")]` FORK, AND THE CFG READ
/// THE WRONG CRATE'S FEATURE.** The arm that writes `ControlFrame` directly was
/// selected by THIS crate's `input` flag, while the thing that overwrites such a
/// write is `ambition_platformer2d/input` — the participant pipeline in the
/// dependency, which workspace feature unification turns on regardless. So the
/// file compiled either way and the two arms did not correspond to the two
/// compositions at all: under `-p ambition_demo_sanic_app -p ambition_app`, all
/// four cases walked to x=5615 of the strip at x=5648 and reported themselves
/// vacuous. The prescribed per-crate test command never selected that
/// composition, so the gate had reported them green since 2026-08-09.
///
/// ⭐ **there is one seam now and it is composition-independent by
/// construction**: `scripted_input` writes after the pipeline's routing stage
/// and before the frame→tick latch, so it wins under a build that HAS a
/// device bridge and works unchanged under one that does not. No cfg, no arms,
/// nothing for a feature flag to select wrongly.
fn hold_right(app: &mut App) {
    ambition_platformer2d::scripted_input::hold(
        app,
        ambition_platformer2d::input::ControlFrame {
            axis_x: 1.0,
            right_pressed: true,
            ..Default::default()
        },
    );
}

/// Park Sanic on the floor at `from_x` holding `rings`, then hold Right until
/// something happens to him or `frames` elapse.
fn walk_right_into(from_x: f32, rings: i32, super_form: bool, frames: usize) -> Outcome {
    let mut app = boot();
    let body = player(&mut app).expect("a playable body");
    let size = app.world().get::<ae::BodyKinematics>(body).unwrap().size;
    if let Some(mut wallet) = app.world_mut().get_mut::<BodyWallet>(body) {
        wallet.balance = rings;
    }
    if super_form {
        let mut worn = app
            .world_mut()
            .get_mut::<WornCharacter>(body)
            .expect("the player wears a character");
        *worn = WornCharacter::new(SUPER_SANIC_CHARACTER_ID);
    }
    displace(&mut app, Vec2::new(from_x, FLOOR_TOP - size.y * 0.5));
    app.update();

    let spawn_x = {
        let mut q = app.world_mut().query_filtered::<&ae::RoomGeometry, With<
            ambition_platformer2d::platformer::lifecycle::SessionRoot,
        >>();
        q.iter(app.world())
            .next()
            .expect("an active session publishes its room geometry")
            .0
            .spawn
            .x
    };

    let half_width = size.x * 0.5;
    let hp0 = app
        .world()
        .get::<BodyHealth>(body)
        .map(|h| h.health.current)
        .unwrap_or(0);
    // The displacement itself must not be read as a death by the counter.
    app.world_mut().resource_mut::<DeathsSeen>().0 = 0;
    let mut out = Outcome {
        hp: hp0,
        rings,
        scattered: 0,
        sent_home: false,
        deaths: 0,
        max_right: from_x + half_width,
    };
    for _ in 0..frames {
        hold_right(&mut app);
        app.update();
        let at = pos(&mut app);
        out.max_right = out.max_right.max(at.x + half_width);
        out.hp = app
            .world()
            .get::<BodyHealth>(body)
            .map(|h| h.health.current)
            .unwrap_or(0);
        out.rings = app
            .world()
            .get::<BodyWallet>(body)
            .map(|w| w.balance)
            .unwrap_or(0);
        out.scattered = {
            let mut q = app
                .world_mut()
                .query::<&ambition_demo_sanic::ScatteredRing>();
            q.iter(app.world()).count()
        };
        out.deaths = app.world().resource::<DeathsSeen>().0;
        if (at.x - spawn_x).abs() < 64.0 {
            out.sent_home = true;
            break;
        }
        // ⚠ **stop at the FIRST thing that happens to him.** Running on past a
        // successful hit re-collects the rings he just dropped (they are real
        // pickups and he is still holding Right), and the first draft of this
        // read 14 rings back at the finish line and called the spend a failure.
        // ⚠ **unless he DIED, in which case keep going** (ADR 0033). The reset
        // is a CONSEQUENCE now rather than a same-frame reflex inside the hit
        // resolver, so the frame his HP changes is no longer the frame he
        // arrives back at the start line — the two used to coincide, and the
        // `sent_home` check above happened to win the race. The ring-recollection
        // hazard this break exists for cannot bite on a death: he is out of play
        // and the level is about to be put back.
        if out.deaths == 0 && (out.rings != rings || out.hp != hp0) {
            break;
        }
        // Clear of the strip and unharmed: nothing is going to happen now.
        if at.x - half_width > SPIKES_LEFT_X + 200.0 {
            break;
        }
    }
    out
}

/// Every case must actually reach the strip, or it proves nothing about it.
///
/// ⚠ **`sent_home` also counts as arriving.** The last position sampled before
/// a reset is the frame BEFORE it, so a body teleported on contact records a
/// leading edge two pixels short of the strip it just touched — measured at
/// 5646 of 5648 during the poison run that re-authored the strip as a
/// `HazardBlock`. Requiring the edge unconditionally would have reported the
/// real regression as "this case is vacuous", which is a true sentence about
/// the wrong thing.
fn assert_reached_the_strip(what: &str, out: &Outcome) {
    assert!(
        out.max_right >= SPIKES_LEFT_X || out.sent_home,
        "{what}: nothing happened to him and the run never reached the spikes \
         at x={SPIKES_LEFT_X} (leading edge got to {:.0}), so this case is \
         vacuous — check the walk-up, not the hazard",
        out.max_right
    );
}

#[test]
fn spikes_with_rings_cost_the_rings_and_nothing_else() {
    let out = walk_right_into(RUN_UP_X, 12, false, 240);
    assert_reached_the_strip("with rings", &out);
    assert!(
        !out.sent_home,
        "a hit with rings must not end the run — that is the whole bug. \
         Authored as a `HazardBlock` this teleported him to spawn with all 12 \
         rings still in the purse. {out:?}"
    );
    assert!(
        out.hp > 0 && out.deaths == 0,
        "the rings absorbed the hit, so HP is untouched and no attempt was \
         lost: {out:?}"
    );
    assert_eq!(
        out.rings, 0,
        "and it costs EVERY ring — the classic price: {out:?}"
    );
    assert_eq!(
        out.scattered, 12,
        "which burst out as real pickups he can run back down: {out:?}"
    );
}

#[test]
fn spikes_at_zero_rings_are_lethal() {
    let out = walk_right_into(RUN_UP_X, 0, false, 600);
    assert_reached_the_strip("at zero rings", &out);
    assert!(
        out.sent_home && out.deaths > 0,
        "with nothing left to spend the hit lands, and Sanic's authored \
         `max_health: 1` makes it FATAL — the engine publishes the attempt-lost \
         fact and he is back at the start line. On the host's default 20-point \
         pool this took 1 HP and he walked on with 19, which is an RPG rule \
         wearing a Sonic sprite. {out:?}"
    );
}

#[test]
fn a_super_runner_crosses_the_spikes_untouched() {
    let out = walk_right_into(RUN_UP_X, 12, true, 240);
    assert_reached_the_strip("as super", &out);
    assert!(
        !out.sent_home && out.deaths == 0 && out.rings >= 12 && out.scattered == 0,
        "the super form is invulnerable, and a damage volume asks \
         `body_vulnerable` like every other emitter — so he keeps his rings and \
         his run: {out:?}"
    );
}

#[test]
fn the_pit_still_swallows_him_at_any_ring_count() {
    for rings in [0, 12] {
        // Start on the west floor just short of the gap and run in.
        let out = walk_right_into(PIT_LEFT_X - 40.0, rings, false, 240);
        assert!(
            out.sent_home,
            "the pit resets, and it resets whatever you are carrying — falling \
             out is not something that HIT you, so no wallet buys you out of it \
             (rings={rings}): {out:?}"
        );
        assert_eq!(
            out.scattered, 0,
            "and it is not a hit, so nothing scatters (rings={rings}): {out:?}"
        );
    }
}
