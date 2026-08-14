//! **When does the world reset, relative to the death beat?**
//!
//! Jon, from play: *"When maryo-dies the enemies seem to reset before the death
//! animation is finished. The level reset needs to happen all at once at a time
//! that is easy to express in the game code."*
//!
//! Reading the chain says that should not happen: `ResetRoomFeaturesEvent` has
//! one production writer, reached only by draining `RoomReplayRequested`, and
//! Mary-O emits that from `restart_level_after_death`, which returns early while
//! `sequence.active()`. So the reset is already gated behind the beat.
//!
//! ⚠ **which makes this a REPRODUCTION question, not a design one**, and a
//! reproduction question is settled by stepping the sim rather than by reading
//! more of it. This records, per frame, the beat's `remaining` against the frame
//! the world actually resets, and asserts the ordering the code claims. A red
//! here is the real row; a green says the symptom is something else wearing the
//! same clothes — most likely the PLAYER resetting mid-animation, which
//! `death_respawn_player` does do on the fatal hit and which the death module's
//! own doc calls out.
//!
//! ⛔ **it kills her by dropping her in a pit, not by writing
//! `ActorDiedMessage`.** Writing the message would prove the beat gates the
//! reset when the beat is armed by hand — which is the one case nobody doubted.
//! The pit is the path a player takes.
//!
//! ## Two deaths, and the second one is here because the first cannot see
//!
//! The pit death drops her `y = 4000`, which is ≥3500 px from every enemy in the
//! level — and both Mary-O enemies carry `AwakeNearObservers { radius: 720 }`,
//! so **every enemy is dormant for the whole beat.** `enemy_signature` sits
//! constant across 94% of it, and it would do that whether or not the world was
//! frozen. ⛔ **a dormant world looks exactly like a frozen one**, so that
//! fixture reports *"the world holds still"* unconditionally: if the open
//! *freeze it?* decision is ever answered yes, this is the thing that would
//! confirm an implementation that never ran.
//!
//! ⚠ **the 4000 stays.** It is how she dies *in a pit*, which is the death the
//! first fixture is about; moving her within 720 px of an enemy would change the
//! cause of death. So the repair is the SECOND fixture below, which kills her
//! where she stands — beside four woken enemies — and asserts that the
//! instrument can see the world move before it reports anything about the beat.

use ambition_demo_mary_o_app::build_demo_app;
use ambition_platformer2d::actors::features::ecs::dormancy::{DormancyPolicy, Dormant};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

/// The frames on which the world was told to put its features back.
#[derive(Resource, Default)]
struct ResetFrames(Vec<usize>);

#[derive(Resource, Default)]
struct FrameCounter(usize);

fn record_resets(
    frame: Res<FrameCounter>,
    mut seen: ResMut<ResetFrames>,
    mut resets: MessageReader<ambition_platformer2d::combat::ResetRoomFeaturesEvent>,
) {
    for _ in resets.read() {
        seen.0.push(frame.0);
    }
}

fn advance_frame(mut frame: ResMut<FrameCounter>) {
    frame.0 += 1;
}

fn boot() -> App {
    let mut app = build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    app.init_resource::<ResetFrames>();
    app.init_resource::<FrameCounter>();
    app.add_systems(Last, (record_resets, advance_frame).chain());
    app
}

fn settle_until_playable(app: &mut App) {
    for _ in 0..600 {
        app.update();
        let mut query = app
            .world_mut()
            .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
        if query.iter(app.world()).next().is_some() {
            return;
        }
    }
    panic!("the demo never activated a playable body");
}

/// Relocate through the engine's authority (ADR 0024) rather than poking `pos`,
/// which would leave the motion model's attachment state describing the old
/// position — the same reason `room_replay.rs` does it this way.
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

/// Seconds left on the death window, or `None` when nobody is dying.
///
/// ⚠ **it rides the BODY now** (ADR 0033), not the level owner — the beat is the
/// participant's, and the component is absent entirely when no window is open.
/// `None` therefore means "nobody is dying", where the old level-owned component
/// answered `Some(0.0)` for the same state.
pub(crate) fn beat_remaining(app: &mut App) -> Option<f32> {
    let mut query = app
        .world_mut()
        .query::<&ambition_platformer2d::combat::death_rules::DeathInterlude>();
    query.iter(app.world()).next().map(|s| s.remaining)
}

/// **Kill her with a real hit, and keep swinging until the beat says she died.**
///
/// Returns how many frames it took, so a caller can print what it cost rather
/// than trusting a magic number.
///
/// ⛔ **`BodyHealth.health.current = 0` does NOT kill her, and it is the obvious
/// wrong route.** Nothing polls the controlled body's health for death: the only
/// two writers of `ActorDiedMessage` are `death_respawn_player` — reached from
/// `handle_player_damage_events`, i.e. from a HIT — and
/// `publish_kernel_reset_death`, which needs a kernel reset (a pit, a hazard).
/// Measured here 2026-08-09: a hand-zeroed body walked this room at `hp = 0` for
/// 120 frames and the beat never armed. `versus_stage.rs` recorded the same
/// thing about the versus stage: *"a hand-zeroed body never invokes it and the
/// test passes whether or not the fix exists"*.
///
/// ⚠ **the first swing is routinely voided**, which is why this loops rather
/// than throwing one hit and hoping. A victim-side hit is staged into
/// `PendingPlayerHitEvents` at the end of one frame's Combat phase and applied
/// on the next, and `void_pending_player_hits_at_lifecycle_boundaries` clears
/// that FIFO on every `RoomLoaded` / `ResetRoomFeaturesEvent` — so a hit thrown
/// in the frames around a room load lands on nothing. Swinging until the beat
/// arms means no caller has to know which frame that boundary fell on.
pub(crate) fn deal_a_lethal_hit(app: &mut App) -> usize {
    use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};

    for frame in 0..600 {
        if beat_remaining(app).unwrap_or(0.0) > 0.0 {
            return frame;
        }
        let (her, at, hp) = {
            let mut query = app.world_mut().query_filtered::<(
                Entity,
                &ae::BodyKinematics,
                &ambition_platformer2d::characters::actor::BodyHealth,
            ), With<PrimaryPlayer>>();
            let (her, kin, health) = query
                .iter(app.world())
                .next()
                .expect("gameplay has a primary player to kill");
            (her, kin.pos, health.current())
        };
        let volume: ae::CombatVolume = ae::Aabb::new(at, Vec2::new(40.0, 40.0)).into();
        app.world_mut().write_message(HitEvent {
            strike_sfx: None,
            volume,
            // Enough to finish her whatever she is wearing: the classic armor
            // ladder absorbs a hit before HP, and this fixture is about the
            // death, not about how many hits it takes to get there.
            damage: hp.max(1) + 10,
            // Contact with an enemy body — the death a player actually dies in
            // this level, and a victim-side source, so it reaches the player
            // damage pass rather than the attacker-side one.
            source: HitSource::Contact,
            attacker: None,
            target: HitTarget::Body(her),
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });
        app.update();
    }
    panic!(
        "600 frames of lethal hits and the death beat never armed — the fixture \
         killed nothing, so whatever it measures next is vacuous"
    );
}

/// How many bodies that declare a dormancy policy are AWAKE right now, out of
/// how many exist.
///
/// The gate this counts is the whole reason the second fixture exists: a
/// sleeping brain writes nothing, so a body behind a shut gate contributes a
/// constant to [`enemy_signature`] forever.
fn awake_bodies(app: &mut App) -> (usize, usize) {
    let mut query = app
        .world_mut()
        .query_filtered::<Has<Dormant>, (Without<PrimaryPlayer>, With<DormancyPolicy>)>();
    let mut total = 0usize;
    let mut awake = 0usize;
    for dormant in query.iter(app.world()) {
        total += 1;
        if !dormant {
            awake += 1;
        }
    }
    (awake, total)
}

/// A cheap signature of every non-player body's placement, so "the enemies moved
/// back" is observable without naming any particular enemy.
fn enemy_signature(app: &mut App) -> (usize, i64) {
    let mut query = app
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, Without<PrimaryPlayer>>();
    let mut count = 0usize;
    let mut sum = 0i64;
    for kin in query.iter(app.world()) {
        count += 1;
        sum += kin.pos.x.round() as i64 + kin.pos.y.round() as i64;
    }
    (count, sum)
}

/// **The world does not come back before the beat is over.**
#[test]
fn the_room_resets_no_earlier_than_the_death_beat_ends() {
    let mut app = boot();
    settle_until_playable(&mut app);

    // Drop her far below the room. The pit rule is what kills her, so the death
    // arrives by the door a player uses.
    displace(&mut app, Vec2::new(200.0, 4000.0));

    let mut beat_started: Option<usize> = None;
    let mut beat_ended: Option<usize> = None;
    let mut enemy_moved: Option<usize> = None;
    let mut previous_signature: Option<(usize, i64)> = None;
    let mut log: Vec<String> = Vec::new();

    for _ in 0..600 {
        app.update();
        let frame = app.world().resource::<FrameCounter>().0;
        let remaining = beat_remaining(&mut app);
        let signature = enemy_signature(&mut app);

        // ⚠ **the window is REMOVED when it closes** (ADR 0033), it does not sit
        // at zero — so "ended" is `None` after having been `Some`, not
        // `Some(0.0)`. A fixture that only watched for `<= 0.0` would wait
        // forever and report the beat as never having run down.
        match remaining {
            Some(r) if r > 0.0 => {
                if beat_started.is_none() {
                    beat_started = Some(frame);
                }
            }
            _ if beat_started.is_some() && beat_ended.is_none() => {
                beat_ended = Some(frame);
            }
            _ => {}
        }
        // Only interesting once the beat is running: before that she is walking
        // around a live room and everything in it is legitimately moving.
        if beat_started.is_some() && enemy_moved.is_none() {
            if let Some(previous) = previous_signature {
                if previous != signature {
                    enemy_moved = Some(frame);
                }
            }
        }
        if beat_started.is_some() && log.len() < 200 {
            log.push(format!(
                "f{frame}: remaining={:?} enemies={:?}",
                remaining, signature
            ));
        }
        previous_signature = Some(signature);
        if beat_ended.is_some() && !app.world().resource::<ResetFrames>().0.is_empty() {
            break;
        }
    }

    let resets = app.world().resource::<ResetFrames>().0.clone();
    let started = beat_started.expect(
        "she never died in a pit 4000 units below the room — the fixture is not \
         reproducing anything, so a green result here would be vacuous",
    );
    let ended = beat_ended.unwrap_or_else(|| {
        panic!("the death beat never ran down within 600 frames (started f{started})")
    });
    let first_reset = *resets.first().unwrap_or_else(|| {
        panic!(
            "the beat ran from f{started} to f{ended} and the room was never \
             reset — the replay the beat owes was never delivered"
        )
    });

    assert!(
        first_reset >= ended,
        "THE WORLD CAME BACK BEFORE THE DEATH BEAT ENDED — reset on f{first_reset}, \
         beat ran f{started}..f{ended}. This is Jon's report reproduced, and it \
         means the reset is NOT gated behind the beat the way the chain reads.\n{}",
        log.join("\n")
    );
    // ⭐ **WHAT THIS MEASURED, 2026-08-03, and it is not what the report said.**
    // The reset is correctly gated — but the world never HOLDS STILL: the
    // non-player bodies move on essentially every frame of the ~0.55 s dwell
    // (signature drifting 37188 → 37131 across f163..f195) and then snap on the
    // single frame after it ends. To a player that reads exactly as "the enemies
    // reset before the death animation finished": they were walking the whole
    // time, and then jumped.
    //
    // ⛔ **deliberately NOT asserted.** Whether the world freezes during a death
    // beat is an unmade design decision, and pinning today's answer either way
    // would be a regression test over unpolished behaviour. The measurement is
    // printed and the row carries it; see D2 in the 08-03 spine.
    println!(
        "[death beat] started f{started}, ended f{ended}, room reset f{first_reset}; \
         first world motion during the beat: {enemy_moved:?}"
    );
    for line in &log {
        println!("{line}");
    }
}

/// **HOW MANY TIMES DOES ONE FALL ASK THE WORLD TO RESET?**
///
/// Jon, from play (2026-08-09): *"I've noticed enemies respawning immediately
/// when she dies even though the animation and music is still playing. That is
/// not correct."*
///
/// The fixture above says the room resets exactly once, after the beat — and it
/// is right about THIS host. `build_demo_app` is the standalone Mary-O binary,
/// and the standalone binary does not carry `apply_home_reset_policy`; the
/// hosted `ambition_app` does (`game/ambition_app/src/app/player_tick.rs:40`).
/// That system writes `ResetRoomFeaturesEvent { reason: PlayerDeath }` on **every
/// frame `PlayerBodyFrameOutput.reset` is `Some`**, with no other condition than
/// `gameplay_allowed`.
///
/// ⭐ **so the fact to measure here is the INPUT that system reads**, which this
/// host produces identically: the beat pins her at the place she died, that place
/// is outside the world, and the kernel's gate is a position test. Every frame of
/// the pin re-flags the reset. The count below is therefore the number of room
/// resets the HOSTED app performs during one death — measured on a host where
/// nothing consumes them.
///
/// ⚠ **anti-vacuity first.** A run where she never died at all would report
/// "no re-flags" by reporting nothing, so the beat must have armed and a life
/// must have been spent before any count below means anything.
#[test]
fn the_pinned_death_pose_reflags_the_world_reset_every_frame_of_the_beat() {
    let mut app = boot();
    settle_until_playable(&mut app);
    // The level owner is staged after the body, and the beat + lives both ride
    // it — seeding the fall before it exists measures nothing.
    let mut staged = false;
    for _ in 0..600 {
        app.update();
        if level_lives(&mut app).is_some() {
            staged = true;
            break;
        }
    }
    assert!(staged, "the level owner never appeared, so there is no run to lose");
    // Let the room settle before the fall is seeded.
    for _ in 0..60 {
        app.update();
    }

    let start_pos = player_pos(&mut app).expect("she is on the level before she falls");
    let start_lives = level_lives(&mut app).expect("the level owner carries the lives counter");

    // One fall, through the door a player uses: below the room is the pit.
    displace(&mut app, Vec2::new(200.0, 4000.0));

    let mut beat_armings: Vec<usize> = Vec::new();
    let mut lives_timeline: Vec<(usize, i8)> = Vec::new();
    let mut reflagged_during_beat = 0usize;
    let mut reflag_causes: Vec<String> = Vec::new();
    let mut previous_active = false;
    let mut previous_lives = start_lives;
    // Two dwells' worth of frames: one whole beat, plus room to see what the
    // frame after it does.
    let frames = ((ambition_demo_mary_o::death::DEATH_DWELL * 2.0 + 2.0) * 60.0) as usize;
    for _ in 0..frames {
        app.update();
        let frame = app.world().resource::<FrameCounter>().0;
        let active = beat_remaining(&mut app).unwrap_or(0.0) > 0.0;
        if active && !previous_active {
            beat_armings.push(frame);
        }
        previous_active = active;
        if let Some(lives) = level_lives(&mut app) {
            if lives != previous_lives {
                lives_timeline.push((frame, lives));
                previous_lives = lives;
            }
        }
        // THE INPUT the hosted app's home-reset policy reads. One `Some` here is
        // one `ResetRoomFeaturesEvent { PlayerDeath }` there.
        if active {
            if let Some(reset) = kernel_reset_flag(&mut app) {
                reflagged_during_beat += 1;
                if reflag_causes.len() < 4 {
                    reflag_causes.push(format!("f{frame}: {reset:?}"));
                }
            }
        }
    }

    let resets = app.world().resource::<ResetFrames>().0.clone();
    let end_pos = player_pos(&mut app);
    println!(
        "[one fall] start {start_pos:?} lives {start_lives}; beats armed on {beat_armings:?}; \
         lives {lives_timeline:?}; room resets seen by THIS host {resets:?}; \
         she ended at {end_pos:?}"
    );
    println!(
        "[one fall] kernel re-flagged the world reset on {reflagged_during_beat} frames \
         DURING the beat — in the hosted app that is {reflagged_during_beat} \
         `ResetRoomFeaturesEvent {{ PlayerDeath }}` while the death music plays. \
         First few: {reflag_causes:?}"
    );

    assert!(
        !beat_armings.is_empty(),
        "she never died falling 4000 units below the room — nothing below this \
         line means anything"
    );
    assert!(
        !lives_timeline.is_empty(),
        "the beat armed on {beat_armings:?} and the lives counter never moved, \
         so this fixture cannot see a life being spent"
    );
    // ⭐ **ONE, and it is the frame she died on.** That frame IS the death: the
    // kernel flags it, `publish_kernel_reset_death` turns it into the fact, and
    // `open_death_interlude` marks her out of play in the same frame's Outcome
    // set — so from the next frame the gate skips her and stays silent for the
    // whole dwell. Before ADR 0033 this was 192 of 192, because the beat pinned
    // her outside the world and the gate is a position test; in the hosted app
    // every one of those frames was a full room-feature reset while the death
    // music played.
    assert_eq!(
        reflagged_during_beat, 1,
        "one fall must flag the world exactly once — the frame it happened. \
         {reflagged_during_beat} says the world is still acting on a body whose \
         attempt is over. Causes: {reflag_causes:?}"
    );
    assert_eq!(
        beat_armings.len(),
        1,
        "and one fall is one death beat: {beat_armings:?}, lives {lives_timeline:?}"
    );
    assert_eq!(
        lives_timeline.len(),
        1,
        "and costs exactly one life: {lives_timeline:?}"
    );
}

/// The kernel's world-reset flag for the controlled body THIS frame — the fact
/// `apply_home_reset_policy` turns into a room-feature reset in the hosted app.
fn kernel_reset_flag(
    app: &mut App,
) -> Option<ambition_platformer2d::actors::avatar::BodyReset> {
    let mut query = app
        .world_mut()
        .query_filtered::<&ambition_platformer2d::actors::avatar::PlayerBodyFrameOutput, With<PrimaryPlayer>>();
    query.iter(app.world()).next().and_then(|out| out.reset)
}

/// The controlled body's position right now, if she has one.
fn player_pos(app: &mut App) -> Option<Vec2> {
    let mut query = app
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    query.iter(app.world()).next().map(|kin| kin.pos)
}

/// The level owner's lives counter, if the level is staged.
fn level_lives(app: &mut App) -> Option<i8> {
    let mut query = app
        .world_mut()
        .query::<&ambition_demo_mary_o::MaryOLevelState>();
    query.iter(app.world()).next().map(|level| level.lives)
}

/// **The same beat, measured somewhere the instrument can see.**
///
/// She dies WHERE SHE STANDS, so no coordinate is authored here at all — her
/// `PlayerStart` has four enemies inside the 720 px wake radius, the nearest at
/// 210 px, which is how the dormancy gate ends up open without the fixture
/// arranging anything.
///
/// ⚠ **it asserts what the instrument CAN SEE and only prints what it saw.**
/// Whether the world should hold still during a death beat is an open design
/// question, and pinning today's answer either way would be a regression test
/// over unpolished behaviour — the same reason the pit fixture prints its
/// measurement instead of asserting it. What is asserted is that this fixture
/// could tell the difference: a fixture that cannot detect motion cannot detect
/// its absence, and that is the entire defect being repaired.
#[test]
fn the_death_beat_is_measured_with_the_world_awake() {
    let mut app = boot();
    settle_until_playable(&mut app);

    // A live window first: she is alive, the room is running, and the question
    // is whether this instrument moves at all.
    let mut alive_window = Vec::new();
    for _ in 0..60 {
        app.update();
        alive_window.push(enemy_signature(&mut app));
    }
    let (awake_alive, policy_bodies) = awake_bodies(&mut app);
    assert!(
        policy_bodies > 0,
        "no body in this room declares a dormancy policy, so this fixture is \
         measuring a gate that does not exist"
    );
    assert!(
        awake_alive > 0,
        "the dormancy gate is SHUT where she stands ({awake_alive}/{policy_bodies} \
         awake) — every enemy is asleep, so the signature below is constant for \
         reasons that have nothing to do with a death beat"
    );
    assert!(
        alive_window.windows(2).any(|a| a[0] != a[1]),
        "the signature never moved across 60 frames of a LIVE room \
         ({:?} throughout) — the instrument cannot see motion, so it cannot see \
         its absence either",
        alive_window.first()
    );

    // Kill her where she stands. The four woken enemies stay woken because the
    // beat pins her exactly here for its whole duration.
    let swings = deal_a_lethal_hit(&mut app);

    let mut beat_log: Vec<String> = Vec::new();
    let mut awake_floor = usize::MAX;
    let mut moved = false;
    let mut previous = enemy_signature(&mut app);
    for _ in 0..600 {
        let Some(remaining) = beat_remaining(&mut app) else {
            break;
        };
        if remaining <= 0.0 {
            break;
        }
        let (awake, total) = awake_bodies(&mut app);
        let signature = enemy_signature(&mut app);
        awake_floor = awake_floor.min(awake);
        moved |= signature != previous;
        previous = signature;
        beat_log.push(format!(
            "remaining={remaining:.3} awake={awake}/{total} enemies={signature:?}"
        ));
        app.update();
    }

    assert!(
        beat_log.len() > 100,
        "the beat was only observed for {} frames — too few to say anything \
         about a 3.2s dwell",
        beat_log.len()
    );
    assert!(
        awake_floor > 0,
        "the world fell asleep DURING the beat ({awake_floor} awake at the \
         floor), which is the exact blindness this fixture exists to remove — \
         the pit fixture already measures a dormant world and cannot tell one \
         from a frozen one"
    );

    println!(
        "[death beat, awake world] {swings} swings to kill her; \
         {awake_alive}/{policy_bodies} bodies awake beside her; \
         {} frames of beat observed, at least {awake_floor} awake throughout; \
         the world MOVED during the beat: {moved}",
        beat_log.len()
    );
    for line in &beat_log {
        println!("{line}");
    }
}
