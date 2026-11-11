//! Ambition-specific clock requests and world-log reporting.
//!
//! Clock arbitration lives in `ambition_time::time_control`; this module emits
//! game-policy requests such as hitstop, bullet-time, and developer slow-motion.

use bevy::prelude::*;

use ambition_characters::actor::BodyCombat;
use ambition_combat::feel::Platformer2dFeelTuningMonolith;
use ambition_characters::control::{DrivingParticipant, PlayerSlot};
use ambition_time::time_control::{
    ClockRequester, ClockResetRequest, ClockScaleRequest, RequestedClockScale,
};
use ambition_time::{ClockDomain, ClockObserver, ClockState};

/// Emit time requests for the primary local observer's driven body.
/// Match impact holds use their own tick expiry and take precedence.
/// The current observer policy requests the shared simulation clock.
///
/// A FREEZE IS FOR ONE ROOM'S FIGHT. The shared clock runs every live room,
/// so with more than one live, a hit in one would stop the others: players in
/// different rooms are independent (ruling 2026-10-19), as a conversation is
/// not modal there either. So with several live rooms neither the impact hold
/// nor Player 1's hitlag freezes the shared clock: the struck room's own
/// bodies hold instead (`ImpactHitstop::rooms`, read through `RoomFreeze`).
/// And bullet time slows Player 1's room, not every room: the shared clock
/// runs at 1 and Player 1's room eases toward the bullet scale in
/// `RoomClockScales`, at the shared clock's ramp rates (ISL-1b,
/// `docs/planning/engine/island-rollback-and-compact-peer-input.md`).
pub fn emit_primary_observer_time_intent(
    drivers: Query<(Entity, &DrivingParticipant)>,
    bodies: Query<(&ambition_platformer2d_core::BodyMotionFacts, &BodyCombat)>,
    feel: Res<Platformer2dFeelTuningMonolith>,
    // The match hold is independent of the local observer.
    impact: Option<Res<ambition_combat::impact_hitstop::ImpactHitstop>>,
    tick: Option<Res<ambition_time::SimTick>>,
    // A room hidden for another room's replay (`Disabled`) is still live.
    live_rooms: Query<
        (),
        (
            With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
            bevy::ecs::query::Allow<bevy::ecs::entity_disabling::Disabled>,
        ),
    >,
    mut writer: MessageWriter<ClockScaleRequest>,
    // Each room's eased scale while several rooms are live, the room of
    // Player 1's body, and the real seconds the ramp is measured in.
    (room_scales, rooms_of, world_time): (
        Option<ResMut<ambition_combat::impact_hitstop::RoomClockScales>>,
        ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
        Option<Res<ambition_time::WorldTime>>,
    ),
) {
    let wall_dt = world_time.as_deref().map_or(0.0, ambition_time::WorldTime::wall_dt);
    let one_room = live_rooms.iter().count() <= 1;
    // The match hold expires on SimTick, which advances during a freeze.
    if let (true, Some(impact), Some(tick)) = (one_room, impact, tick) {
        if impact.is_freezing(*tick) {
            writer.write(ClockScaleRequest {
                domain: ClockDomain::SimClock,
                scale: 0.0,
                requester: ClockRequester::Engine,
                reason: "impact_hitstop",
            });
            return;
        }
    }
    let observed_body = ambition_platformer2d_actor_monolith::control::body_driving_seat(
        &drivers, PlayerSlot::PRIMARY,
    );
    let observed = observed_body.and_then(|body| bodies.get(body).ok());
    // No unique observed body means no retained body may hold the world clock.
    let Some((facts, combat)) = observed else {
        // Nor any room's: every room eases home.
        if let Some(mut room_scales) = room_scales.filter(|scales| !scales.rooms.is_empty()) {
            room_scales.rooms = ease_room_clocks(&room_scales.rooms, None, &feel, wall_dt);
        }
        writer.write(ClockScaleRequest {
            domain: ClockDomain::SimClock,
            scale: 1.0,
            requester: ClockRequester::Engine,
            reason: "default",
        });
        return;
    };
    let (scale, requester, reason) = if one_room && combat.hitstop_timer > 0.0 {
        (0.0, ClockRequester::Engine, "hitstop")
    } else if facts.blink_aiming {
        (
            feel.bullet_time_scale,
            ClockRequester::Player(ClockObserver::PRIMARY),
            "bullet_blink",
        )
    } else if facts.blink_telegraph {
        // The aiming arm above claimed precision aim, so a telegraph here is
        // the charge HOLD specifically.
        (
            feel.blink_hold_slow_scale,
            ClockRequester::Player(ClockObserver::PRIMARY),
            "blink_hold_slow",
        )
    } else {
        (1.0, ClockRequester::Engine, "default")
    };
    // Several live rooms: the scale is Player 1's room's, and the shared
    // clock runs at 1.
    if let Some(mut room_scales) = room_scales {
        let room = (!one_room).then(|| observed_body.and_then(|body| rooms_of.of(body))).flatten();
        let eased = ease_room_clocks(&room_scales.rooms, room.map(|room| (room, scale)), &feel, wall_dt);
        if eased != room_scales.rooms {
            room_scales.rooms = eased;
        }
    }
    let (scale, reason) = if one_room { (scale, reason) } else { (1.0, "room_clock") };
    writer.write(ClockScaleRequest {
        domain: ClockDomain::SimClock,
        scale,
        requester,
        reason,
    });
}

/// Each room's clock this tick: the `wanted` room eases toward its scale, and
/// every other room toward `1.0`, at the shared clock's ramp rates; a room at
/// `1.0` is forgotten. With no room wanted (one live room, or no Player 1),
/// every room eases home.
fn ease_room_clocks(
    rooms: &std::collections::BTreeMap<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance, f32>,
    wanted: Option<(ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance, f32)>,
    feel: &Platformer2dFeelTuningMonolith,
    frame_dt: f32,
) -> std::collections::BTreeMap<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance, f32> {
    let mut eased = std::collections::BTreeMap::new();
    let current = rooms.iter().map(|(room, scale)| (*room, *scale)).chain(
        wanted.filter(|(room, _)| !rooms.contains_key(room)).map(|(room, _)| (room, 1.0)),
    );
    for (room, now) in current {
        let target = wanted.filter(|(at, _)| *at == room).map_or(1.0, |(_, scale)| scale);
        let rate = if target < now { feel.time_ramp_down_rate } else { feel.time_ramp_up_rate };
        let next = crate::sim_clock::move_toward(now, target, rate * frame_dt);
        if next != 1.0 {
            eased.insert(room, next);
        }
    }
    eased
}

/// Smooth [`ClockState::time_scale`] toward
/// [`RequestedClockScale::sim_clock`] at feel-tuned rates.
///
/// Replaces the imperative `crate::update_time_scale` helper. The
/// asymmetric ramp (`time_ramp_down_rate` when decelerating,
/// `time_ramp_up_rate` when accelerating) preserves the "snap into
/// bullet-time, breathe back to normal" feel the imperative version
/// shipped.
pub fn smooth_sim_clock_toward_target_system(
    target: Res<RequestedClockScale>,
    feel: Res<Platformer2dFeelTuningMonolith>,
    world_time: Res<ambition_time::WorldTime>,
    mut clock: ResMut<ClockState>,
) {
    // Unscaled: the ramp is in real seconds, whatever scale it is ramping.
    let frame_dt = world_time.wall_dt();
    // ⛔⛔ A FREEZE IS NOT A RAMP. A hit's freeze asks for 0 and ends by
    // asking for the pace again; ramped at 5/s down and 14/s up, the clock
    // took 12 frames to reach the stop a 8–16 frame hitlag asked for and 4 to
    // come back, so a hit played as a slow-motion dip rather than a stop. A
    // freeze snaps on and snaps off; a slow-down (bullet time) still eases.
    if target.sim_clock == 0.0 || clock.time_scale == 0.0 {
        clock.time_scale = target.sim_clock;
        return;
    }
    let rate = if target.sim_clock < clock.time_scale {
        feel.time_ramp_down_rate
    } else {
        feel.time_ramp_up_rate
    };
    clock.time_scale = crate::sim_clock::move_toward(clock.time_scale, target.sim_clock, rate * frame_dt);
}

/// Report sim-clock target changes and frozen/running transitions.
///
/// Reports are edge-triggered rather than per-frame. Request metadata identifies
/// the requester and reason; changes without a request come from suspended-gameplay
/// clock control. Frozen detection uses an epsilon because the live scale is smoothed.
const SIM_CLOCK_FROZEN_EPS: f32 = 1e-4;

pub fn report_sim_clock_changes(
    clock: Res<ClockState>,
    target: Res<RequestedClockScale>,
    mut scale_requests: MessageReader<ClockScaleRequest>,
    mut reset_requests: MessageReader<ClockResetRequest>,
    mut last: Local<Option<(f32, f32)>>,
) {
    let live = clock.time_scale;
    let want = target.sim_clock;
    let previous = last.replace((live, want));

    let report = match previous {
        None => true,
        Some((previous_live, previous_want)) => {
            frozen_label(previous_live) != frozen_label(live)
                || (want - previous_want).abs() > SIM_CLOCK_FROZEN_EPS
        }
    };
    if !report {
        // The cursors still advance every frame. `emit_primary_observer_time_intent`
        // writes a request on EVERY playing frame (the ladder always resolves,
        // even to "default"), so a cursor left to lag would eventually staple a
        // stale reason onto an unrelated change.
        scale_requests.clear();
        reset_requests.clear();
        return;
    }

    let mut causes = String::new();
    for request in scale_requests.read() {
        if !causes.is_empty() {
            causes.push_str(", ");
        }
        causes.push_str(&format!(
            "{}@{:.3} by {:?}",
            request.reason, request.scale, request.requester
        ));
    }
    for request in reset_requests.read() {
        if !causes.is_empty() {
            causes.push_str(", ");
        }
        causes.push_str(&format!(
            "{} reset by {:?}",
            request.reason, request.requester
        ));
    }
    if causes.is_empty() {
        causes.push_str("no clock request this frame (see [game-mode])");
    }

    let Some((previous_live, previous_want)) = previous else {
        ambition_platformer2d_shared_tangle::world_log::sim_clock(format_args!(
            "initial {:<7} live={live:.3} target={want:.3}",
            frozen_label(live)
        ));
        return;
    };
    ambition_platformer2d_shared_tangle::world_log::sim_clock(format_args!(
        "{:<7} live {previous_live:.3} -> {live:.3}  target {previous_want:.3} -> {want:.3}  \
         cause: {causes}",
        frozen_label(live)
    ));
}

fn frozen_label(scale: f32) -> &'static str {
    if scale.abs() <= SIM_CLOCK_FROZEN_EPS {
        "FROZEN"
    } else {
        "running"
    }
}

#[cfg(test)]
mod tests;

/// Install this module's diagnostic reporting into `app`.
///
/// ⭐ THE CRATE INSTALLS ITS OWN SYSTEM, and the composition keeps the decision
/// that is actually the composition's: `sim_core_resources.rs` records WHY this
/// belongs to the engine group rather than the windowed host — *"an Android
/// freeze and a headless repro must produce the same log, and only the engine
/// group is common to both"* — and that reason is unchanged. What moves is the
/// NAME of a private system out of a file that has no business knowing it.
///
/// ⚠ `PostUpdate` IS THIS CRATE'S OWN STATEMENT, not a phase the host chose. The
/// report is edge-triggered off a clock the simulation has already settled, so it
/// reads state after the frame's writers rather than competing with them. A host
/// that wanted it elsewhere would be asking for a different fact.
pub fn install_sim_clock_reporting(app: &mut App) {
    app.add_systems(PostUpdate, report_sim_clock_changes);
}

#[cfg(test)]
mod install_tests {
    use super::*;

    /// ⛔⛔ THE POISON FOR THIS CARVE COULD NOT FAIL, AND THIS IS WHY THE GUARD
    /// EXISTS. Removing the installer from the composition left `app_it` at
    /// 578/578 — not because nothing depends on the system, but because what it
    /// produces is a `world_log::sim_clock` LINE, and no test asserts on the log.
    ///
    /// ⇒ Third outcome of the carve rule: not "load-bearing and covered", not
    /// "dormant or dead", but **a real consumer the suite cannot see**. A
    /// diagnostic is exactly the kind of output that vanishes without a single
    /// test noticing, and an Android freeze is where somebody finds out.
    ///
    /// ⛔⛔ THIS GUARD USED TO COUNT, AND THE REASON GIVEN WAS FALSE WHERE IT SAT.
    /// The comment here said *"Bevy strips system names without its `debug` feature,
    /// so every row reads `<Enable the debug feature to see the name>` and a
    /// name-based assertion passes against anything."* True of Bevy; false of the
    /// build this test runs in. MEASURED 2026-09-07
    /// (`cargo tree -e features -i bevy_utils@0.19.1`): the one dependent that turns
    /// `bevy_utils/debug` on is `bevy_egui 0.40.1`, reached through the OPTIONAL
    /// `bevy-inspector-egui` behind the `dev_tools` feature — which this crate's own
    /// `default = ["desktop_dev"]` includes. So the names are here, and the guard was
    /// counting for a reason that did not apply.
    ///
    /// ⚠ A COUNT CANNOT BE WRONG, ONLY UNDER-POWERED: it passes if the installer
    /// registers ANY one system, so an installer that registered the wrong system
    /// would satisfy it. Naming the subject is what closes that.
    ///
    /// ⭐ AND THE NAME IS NOT FREE EITHER, so the precondition is stated rather than
    /// assumed. `android = [.., "rl_sim", ..]` is a real feature set in this tree that
    /// carries no `dev_tools`; under it every row IS the placeholder, and a bare
    /// `contains` would then fail claiming the installer registered nothing.
    #[test]
    fn the_installer_registers_the_clock_reporter_by_name() {
        let mut app = App::new();
        let before = post_update_system_names(&mut app);
        install_sim_clock_reporting(&mut app);
        let after = post_update_system_names(&mut app);

        let added: Vec<&String> = after.iter().filter(|name| !before.contains(name)).collect();
        assert_eq!(
            added.len(),
            1,
            "the sim-clock installer registered {} systems, not 1: {added:?}",
            added.len()
        );
        assert!(
            !added[0].contains(NAMES_STRIPPED),
            "this build renders no system names, so the assertion below cannot \
             identify its subject. Names come from `bevy_utils/debug`, supplied only \
             by `bevy_egui` via the optional `bevy-inspector-egui` behind `dev_tools` \
             — someone trimmed that feature out of this build."
        );
        assert!(
            added[0].contains("report_sim_clock_changes"),
            "the sim-clock installer registered `{}`, which is not the clock \
             reporter. A count would have accepted it.",
            added[0]
        );
    }

    /// What Bevy renders instead of a system name when `bevy_utils/debug` is OFF.
    const NAMES_STRIPPED: &str = "<Enable the debug feature to see the name>";

    fn post_update_system_names(app: &mut App) -> Vec<String> {
        app.world_mut().resource_scope(
            |world, mut schedules: Mut<bevy::ecs::schedule::Schedules>| {
                let Some(schedule) = schedules.get_mut(PostUpdate) else {
                    return Vec::new();
                };
                let _ = schedule.initialize(world);
                schedule
                    .systems()
                    .map(|systems| {
                        systems
                            .map(|(_, system)| format!("{}", system.name()))
                            .collect()
                    })
                    .unwrap_or_default()
            },
        )
    }
}
