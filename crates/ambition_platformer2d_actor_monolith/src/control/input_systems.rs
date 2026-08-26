//! Per-frame player input/timer systems.
//!
//! These publish the primary controller's slot gestures from the local device
//! and tick the home/player body's own reaction + presentation timers. They are
//! body-generic gameplay-sim logic (no render, no host-only types), so they live
//! beside the player state they mutate; the host schedule (`register_player_input_systems`)
//! owns their ordering + `run_if` gates and references these `pub fn`s.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

/// The set this tick's input timers advance in.
///
/// A reset that must be seen by the timers (the app's player-reset input) lands
/// before it.
///
/// Nested inside `PlayerInputSet::Device`. The parent also holds the slot
/// publish and the frame commit, both of which the reset must NOT precede — it
/// needs to beat the timer decrement, not the whole device phase.
///
/// ⭐ THE SET IS THE DEPENDENCY, not any one system in it. It held one member
/// for a while, and the sandbox reset's `.before(..)` was written against that
/// member's NAME. Depending on the set instead is what let that member become
/// three without touching the reset.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputTimersAdvanced;

/// Decay the cooldown that keeps a body from re-entering the door it just used.
///
/// A whole-world timer with one owner, sharing nothing with the two systems
/// below but the clock.
pub fn tick_room_transition_cooldown(
    world_time: Res<ambition_time::WorldTime>,
    mut sim_state: ResMut<crate::RoomTransitionCooldown>,
) {
    sim_state.remaining = (sim_state.remaining - world_time.wall_dt()).max(0.0);
}

/// Decay the home body's own reaction timers (`hitstun` / `hitstop` /
/// `damage-invuln` / `recoil`).
///
/// The home/player body is not in the actor tick, so it ticks its OWN timers
/// here. This is the home body's state, NOT authority over the controlled
/// subject — a possessed actor ticks its own timers in the actor path. Every
/// player body iterates, so a co-op or clone body ticks its own.
///
/// ⭐ i-frames are a promise to the PLAYER in real seconds — a bullet-time
/// moment must not hand out longer invulnerability — which is the same reason
/// the double-tap windows below are unscaled. What was once wrong here is the
/// WAIVER, not the clock: the `Res<Time>` allowlist entry for this file claimed
/// "the reaction timers still compute their own scaled dt manually", and no such
/// scaling exists or should.
pub fn tick_home_body_reaction_timers(
    world_time: Res<ambition_time::WorldTime>,
    mut home_feel_q: Query<
        &mut ambition_characters::actor::BodyCombat,
        With<crate::actor::PlayerEntity>,
    >,
) {
    //  ONE decay, called — not a fourth spelling of it (AC3.3). Two lists for
    // one rule, disagreeing in both directions.
    let frame_dt = world_time.wall_dt();
    for mut combat in &mut home_feel_q {
        combat.decay_reaction_timers(frame_dt);
    }
}

/// Derive each seat's DIRECTION gestures — double-tap down (fast-fall) and
/// double-tap up (doors) — from that seat's own input row.
///
/// Derived from each seat's row of `SlotControls` into `SlotInteractionState`,
/// for EVERY slot. Body mode / interaction consume that keyed by the acting
/// body's slot, never a per-body component.
///
/// The host registers this with `run_if(gameplay_allowed)` so it only runs in
/// `GameMode::Playing`. Writes `fast_fall_pressed` back into each seat's row.
///
///  it is NOT in `InputSet::Route` any more, and that is by the set's own
/// definition: Route is every system that writes the global `ControlFrame`,
/// and nothing in this file holds that resource now. It runs after the
/// publication boundary instead, on the table the bodies actually read.
///
/// ⛔ THIS WAS ONE SYSTEM WITH THE TWO ABOVE, at thirteen parameters and
/// climbing. They share a clock and nothing else: a room cooldown, a body's
/// reaction timers, and a seat's gesture history have different owners and
/// different reasons to run. This repo has reached Bevy's parameter ceiling
/// before and answered it by packing unrelated resources into tuples; splitting
/// on the ownership seam is the answer that does not end there.
pub fn derive_slot_direction_gestures(
    // ⭐ the UNSCALED SIM step, by its name. These windows are a promise to the
    // player in real seconds, and `WorldTime::wall_dt` is exactly that value —
    // the same number `Res<Time>` gave here, with the authority stated. Ordered
    // `.after(refresh_world_time)` where it is scheduled; a snapshot resource
    // has an ordering dependency that Bevy's `Time` does not.
    world_time: Res<ambition_time::WorldTime>,
    feel_tuning: Res<ambition_combat::feel::Platformer2dFeelTuningMonolith>,
    // WHO IS DRIVING WHAT, so each seat's gesture resolves against the
    // gravity of the body that seat is actually steering.
    drivers: Query<(Entity, &crate::control::DrivingParticipant)>,
    frames: Query<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    primary_q: Query<Entity, crate::actor::PrimaryPlayerOnly>,
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    //  the SLOT TABLE, not the global frame. The derivation refines the
    // frame each body is about to read, and every body reads its own slot.
    mut slots: ResMut<ambition_characters::control::SlotControls>,
    // ⛔⛔ **WRITE to BOTH, because the global `ControlFrame` used to be both.**
    // The derived flag reached the body this tick (the frame→slot copy read that
    // resource after this system ran) AND the encoded rollback input (the latch
    // folded the same resource). Writing only the slot loses the second; writing
    // only the raw row loses the first on a latch host, where the drain has
    // already happened by the time this runs.
    mut raw: ResMut<ambition_characters::control::SeatRawFrames>,
    // Which of those two is THIS TICK's input depends on the clock — see
    // `seat_frame_this_tick`.
    latches: Option<Res<ambition_characters::control::SlotControlLatches>>,
    rollback: Option<Res<ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>>,
    mut slot_gestures: ResMut<ambition_characters::control::SlotInteractionState>,
) {
    let frame_dt = world_time.wall_dt();
    let feel = *feel_tuning;
    let movement_mode = user_settings
        .as_deref()
        .map_or(ae::InputFrameMode::DEFAULT_MOVEMENT, |s| {
            s.gameplay.resolved_movement_frame_mode()
        });
    // ⛔⛔ **EVERY SEAT, AND IT USED TO BE `slot_gestures.primary_mut()`.** The
    // table, the accessor and the consumer were all per-slot already — body mode
    // reads `get_mut(slot).double_tap_down_pending` keyed by the acting body's
    // seat — and the PRODUCER filled row zero. So `fast_fall_pressed` was
    // hardcoded `false` for every other seat and **player two could not
    // fast-fall** (D175). The participant that never joined: nothing was
    // missing but the loop.
    for index in 0..ambition_characters::control::SlotControls::MAX_SLOTS {
        let slot = ambition_characters::control::PlayerSlot(index as u8);
        let Some(interaction) = slot_gestures.get_mut(slot) else {
            continue;
        };
        let frame = crate::control::seat_frame_this_tick(
            latches.as_deref(),
            rollback.as_deref(),
            &slots,
            &raw,
            slot,
        );
        // Fast-fall = double-tap local-down for the body driving THIS seat. Raw
        // cardinal edges are resolved through the same input mapping policy as
        // locomotion, so ScreenDirected sideways gravity can map raw-right /
        // raw-left into local down/up without bespoke cases here.
        //
        //  whose down, asked per seat. A double-tap means *down* relative
        // to the body the person is steering; resolving every seat against the
        // primary's gravity would hand player two player one's idea of down the
        // moment either of them stands on a wall.
        let gravity_dir = crate::control::seat_frame_down(
            &drivers,
            slot,
            &frames,
            (slot == ambition_characters::control::PlayerSlot::PRIMARY)
                .then(|| primary_q.single().ok())
                .flatten(),
        );
        let resolved = ae::AccelerationFrame::new(gravity_dir).resolve_control(
            movement_mode,
            ae::ScreenAxes::new(frame.axis_x, frame.axis_y),
        );
        let raw_edges = frame.raw_direction_edges();
        let descend_pressed = resolved.local_down_pressed(raw_edges);
        let ascend_pressed = resolved.local_up_pressed(raw_edges);
        let double_tap_down =
            interaction.register_down_tap(descend_pressed, frame_dt, feel.down_double_tap_window);
        crate::control::shape_seat_frame(
            latches.as_deref(),
            rollback.as_deref(),
            &mut slots,
            &mut raw,
            slot,
            |frame| frame.fast_fall_pressed = double_tap_down,
        );
        if double_tap_down {
            interaction.double_tap_down_pending = true;
        }
        let door_double_tap_up =
            interaction.register_up_tap(ascend_pressed, frame_dt, feel.up_double_tap_window);
        if door_double_tap_up {
            interaction.double_tap_up_pending = true;
        }
    }
}

/// Ordering point after interaction gestures are buffered and before portal input
/// warping and `PrimarySlotInputCommit`.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InteractionInputBuffered;

/// Fold explicit Interact, double-tap Up, and held Up into each controller slot's
/// buffered interaction, gated by gameplay state and the driven body's hit stun.
/// Fold explicit Interact, double-tap Up, and held Up into each controller slot's
/// buffered interaction, gated by gameplay state and the driven body's hit stun.
pub fn interaction_input_system(
    // Unscaled sim step — see `derive_slot_direction_gestures`.
    world_time: Res<ambition_time::WorldTime>,
    feel_tuning: Res<ambition_combat::feel::Platformer2dFeelTuningMonolith>,
    // Proposal-side input for this frame; confirmed simulation input is in `slots`.
    raw: Res<ambition_characters::control::SeatRawFrames>,
    slots: Res<ambition_characters::control::SlotControls>,
    latches: Option<Res<ambition_characters::control::SlotControlLatches>>,
    rollback: Option<Res<ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>>,
    drivers: Query<(Entity, &crate::control::DrivingParticipant)>,
    frames: Query<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    mut slot_gestures: ResMut<ambition_characters::control::SlotInteractionState>,
    // Hit-stun gate reads the DRIVEN body's reaction state — the body actually
    // being driven by this seat, home avatar or possessed actor.
    combat_q: Query<&ambition_characters::actor::BodyCombat>,
    primary_q: Query<
        Entity,
        (
            With<crate::actor::PlayerEntity>,
            With<crate::actor::PrimaryPlayer>,
        ),
    >,
) {
    let frame_dt = world_time.wall_dt();
    let feel = *feel_tuning;
    let movement_mode = user_settings
        .as_deref()
        .map_or(ae::InputFrameMode::DEFAULT_MOVEMENT, |s| {
            s.gameplay.resolved_movement_frame_mode()
        });
    //  EVERY SEAT, and this was `slot_gestures.primary_mut` too. The interact buffer is
    // what doors and dialogue read, keyed by the acting body's slot — so a second player
    // standing at a door pressed a button that was buffered for nobody.
    for index in 0..ambition_characters::control::SlotControls::MAX_SLOTS {
        let slot = ambition_characters::control::PlayerSlot(index as u8);
        let body = crate::control::body_driving_seat(&drivers, slot).or_else(|| {
            (slot == ambition_characters::control::PlayerSlot::PRIMARY)
                .then(|| primary_q.single().ok())
                .flatten()
        });
        let hitstun = body
            .and_then(|body| combat_q.get(body).ok())
            .map_or(0.0, |combat| combat.hitstun_timer);
        let frame = crate::control::seat_frame_this_tick(
            latches.as_deref(),
            rollback.as_deref(),
            &slots,
            &raw,
            slot,
        );
        let Some(interaction) = slot_gestures.get_mut(slot) else {
            continue;
        };
        let door_double_tap_up = std::mem::take(&mut interaction.double_tap_up_pending);
        // Down + Interact is the possession gesture
        // (`abilities::traversal::possession`), so a held-Down interact is
        // CLAIMED by possession and must NOT also trigger a normal interaction
        // (open a door / start an NPC dialog) — otherwise the press that begins
        // a possession hold also opens whatever's adjacent. Suppress the interact
        // EDGE while Down is held, using the SAME gravity-resolved "down" the
        // possession trigger uses so they agree under any gravity. The
        // double-tap-UP door request is an Up gesture, so it is never suppressed.
        let gravity_dir = crate::control::seat_frame_down(&drivers, slot, &frames, body);
        let down_held = crate::abilities::traversal::possession::holding_descend(
            frame.axis_x,
            frame.axis_y,
            gravity_dir,
            movement_mode,
        );
        // Holding Up is a third way in, beside the press and the double-tap:
        // the same gravity-resolved axis, past the same deflection, held for
        // as long as a possession takes.
        let up_held = crate::abilities::traversal::possession::holding_ascend(
            frame.axis_x,
            frame.axis_y,
            gravity_dir,
            movement_mode,
        );
        let held_up_interact =
            interaction.held_up_interact(up_held, frame_dt, feel.interaction_hold_time);
        let raw_interact_pressed = if hitstun > 0.0 {
            false
        } else {
            (frame.interact_pressed && !down_held) || door_double_tap_up || held_up_interact
        };
        let _live = interaction.buffered_interact(
            raw_interact_pressed,
            frame_dt,
            feel.interaction_buffer_time,
        );
    }
}

/// Decay presentation-only animation and flash timers.
///
/// Runs every frame (including paused/dialogue) so visual flash and
/// animation pose timers wind down continuously, not just during
/// gameplay. Owns: real-time decay of `hit_flash`, `preset_flash`,
/// `slash_anim_timer`, `blink_in_timer`, `camera_snap_timer`. New
/// presentation-flash timers belong here; gameplay timers belong in
/// `derive_slot_direction_gestures`.
pub fn cleanup_timers_system(
    time: Res<Time>,
    mut dev_state: ResMut<ambition_dev_tools::DeveloperRuntimeState>,
    mut player_q: Query<
        (
            &ae::BodyMotionFacts,
            &mut crate::actor::BodyAnimFacts,
            &mut ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState,
        ),
        crate::actor::PrimaryPlayerOnly,
    >,
) {
    let frame_dt = time.delta_secs();
    let Ok((motion_facts, mut anim, mut blink_cam)) = player_q.single_mut() else {
        return;
    };
    //  `hit_flash` is NOT decayed here any more (AC3.3). It is a body-generic
    // reaction timer and it decays with the rest of them in
    // `tick_home_body_reaction_timers`,
    // which iterates every `PlayerEntity` rather than the home avatar alone —
    // this system's query could not see a second player body at all.
    dev_state.preset_flash = (dev_state.preset_flash - frame_dt).max(0.0);
    // Player-specific presentation timers (the blink-camera lerp) decay here; the
    // body-generic anim OVERLAYS advance through the shared helper the actor tick
    // also runs (fable review §A9).
    blink_cam.blink_in_timer = (blink_cam.blink_in_timer - frame_dt).max(0.0);
    blink_cam.camera_snap_timer = (blink_cam.camera_snap_timer - frame_dt).max(0.0);
    crate::features::advance_body_anim_overlays(motion_facts.dashing, &mut anim, frame_dt);
}

#[cfg(test)]
mod per_seat_gesture_tests {
    use super::*;
    use crate::control::DrivingParticipant;
    use ambition_characters::control::{PlayerSlot, SeatRawFrames, SlotControls, SlotInteractionState};
    use ambition_combat::feel::Platformer2dFeelTuningMonolith;
    use ambition_platformer2d_core::ControlFrame;

    /// PLAYER TWO CAN FAST-FALL.
    ///
    ///  they could not, and one line proved it:
    /// `read_gameplay_control_frame_with_settings` hardcoded `fast_fall_pressed: false`, and
    /// the only system that ever set it read `slot_gestures.primary_mut()`. The table was
    /// per-slot, the accessor was per-slot, and body mode consumed it per-slot — the PRODUCER
    /// filled row zero.
    #[test]
    fn every_seat_derives_its_own_fast_fall_double_tap() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        app.insert_resource(Platformer2dFeelTuningMonolith::default());
        app.init_resource::<SlotInteractionState>();
        app.init_resource::<SlotControls>();
        app.init_resource::<SeatRawFrames>();
        for slot in [0u8, 1] {
            app.world_mut().spawn(DrivingParticipant(PlayerSlot(slot)));
        }
        app.init_resource::<ambition_time::WorldTime>();
        app.add_systems(Update, derive_slot_direction_gestures);

        // A tap arms the window; the second tap inside it IS the double-tap. Both
        // seats press on the same two frames, which is the couch case.
        let tap = ControlFrame {
            down_pressed: true,
            axis_y: 1.0,
            ..Default::default()
        };
        let mut fired = [false; 2];
        for _ in 0..2 {
            {
                let mut raw = app.world_mut().resource_mut::<SeatRawFrames>();
                for slot in [0u8, 1] {
                    raw.set(PlayerSlot(slot), tap);
                }
            }
            app.update();
            let slots = app.world().resource::<SlotControls>();
            for slot in [0u8, 1] {
                fired[slot as usize] |= slots.get(PlayerSlot(slot)).fast_fall_pressed;
            }
        }

        assert!(
            fired[0],
            "seat zero double-tapped down and never fast-fell, so the fixture is \
             not exercising the derivation at all and the seat-one claim below \
             proves nothing",
        );
        assert!(
            fired[1],
            "seat ONE double-tapped down on the same two frames as seat zero and \
             never fast-fell: the gesture derivation is still the primary's alone, \
             and a couch match has one player who can fast-fall and one who cannot",
        );
    }

    /// AND ONE SEAT'S TAPS ARE NOT THE OTHER'S. The falsifier for a loop that
    /// derives per seat but shares the window state: two people alternating taps
    /// would each hand the other a double-tap they never pressed.
    #[test]
    fn one_seats_taps_do_not_arm_another_seats_double_tap() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        app.insert_resource(Platformer2dFeelTuningMonolith::default());
        app.init_resource::<SlotInteractionState>();
        app.init_resource::<SlotControls>();
        app.init_resource::<SeatRawFrames>();
        for slot in [0u8, 1] {
            app.world_mut().spawn(DrivingParticipant(PlayerSlot(slot)));
        }
        app.init_resource::<ambition_time::WorldTime>();
        app.add_systems(Update, derive_slot_direction_gestures);

        let tap = ControlFrame {
            down_pressed: true,
            axis_y: 1.0,
            ..Default::default()
        };
        // Seat zero taps, then seat one taps. Neither has tapped twice.
        for slot in [0u8, 1] {
            {
                let mut raw = app.world_mut().resource_mut::<SeatRawFrames>();
                raw.set(PlayerSlot(0), ControlFrame::default());
                raw.set(PlayerSlot(1), ControlFrame::default());
                raw.set(PlayerSlot(slot), tap);
            }
            app.update();
            let slots = app.world().resource::<SlotControls>();
            assert!(
                !slots.get(PlayerSlot(0)).fast_fall_pressed
                    && !slots.get(PlayerSlot(1)).fast_fall_pressed,
                "a single tap from seat {slot} produced a fast-fall somewhere: the \
                 double-tap window is shared between seats",
            );
        }
    }
}

#[cfg(test)]
mod interaction_suppression_tests {
    use super::*;
    use crate::actor::{PlayerEntity, PrimaryPlayer};
    use ambition_characters::actor::BodyCombat;
    use ambition_characters::control::SlotInteractionState;
    use ambition_combat::feel::Platformer2dFeelTuningMonolith;
    use ambition_platformer2d_core::ControlFrame;

    /// Build a minimal app with `interaction_input_system` and one primary
    /// player, set the control frame, run a frame, and report whether the
    /// primary controller's slot interaction buffer went live.
    fn buffered_after(interact: bool, axis_y: f32) -> bool {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        app.insert_resource(Platformer2dFeelTuningMonolith::default());
        app.init_resource::<SlotInteractionState>();
        // The seat's RAW row — the pre-publish device sample this system reads,
        // which used to be the global `ControlFrame` (D175).
        let mut raw = ambition_characters::control::SeatRawFrames::default();
        raw.set(
            ambition_characters::control::PlayerSlot::PRIMARY,
            ControlFrame {
                interact_pressed: interact,
                axis_y,
                ..Default::default()
            },
        );
        app.insert_resource(raw);
        //  no latch: this is a frame-stepped composition, so the RAW row above
        // is this tick's input and the slot table is the empty destination it
        // will be published into. Both exist in any real composition —
        // `BrainPlugin` installs the pair.
        app.init_resource::<ambition_characters::control::SlotControls>();
        app.world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, BodyCombat::default()));
        app.init_resource::<ambition_time::WorldTime>();
        app.add_systems(Update, interaction_input_system);
        app.update();
        app.world()
            .resource::<SlotInteractionState>()
            .primary()
            .buffered()
    }

    /// A TAP THAT ONLY EVER EXISTED IN THE LATCH STILL REACHES THE BUFFER.
    ///
    /// On a LATCH host that is the wrong table: `ControlFrameLatch` OR-accumulates edges across
    /// every sub-tick sample, so a press that opens and closes between two ticks lives in the
    /// drained frame and in NO single raw sample — which is the entire reason the latch exists. The
    /// fixture puts the press only where the latch would have left it.
    #[test]
    fn a_sub_tick_press_survives_on_a_latching_host() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        app.insert_resource(Platformer2dFeelTuningMonolith::default());
        app.init_resource::<SlotInteractionState>();
        // The latch's presence is what says which clock this composition runs on.
        app.init_resource::<ambition_characters::control::SlotControlLatches>();
        // The raw row is NEUTRAL — the tap was never in one sample.
        app.init_resource::<ambition_characters::control::SeatRawFrames>();
        let mut slots = ambition_characters::control::SlotControls::default();
        slots.set(
            ambition_characters::control::PlayerSlot::PRIMARY,
            ControlFrame {
                interact_pressed: true,
                ..Default::default()
            },
        );
        app.insert_resource(slots);
        app.world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, BodyCombat::default()));
        app.init_resource::<ambition_time::WorldTime>();
        app.add_systems(Update, interaction_input_system);
        app.update();

        assert!(
            app.world()
                .resource::<SlotInteractionState>()
                .primary()
                .buffered(),
            "the press was in the drained frame and not in any raw sample, and \
             the buffer never saw it — a sub-tick tap on a fixed-tick or rollback \
             host reaches the sim through the latch or not at all",
        );
    }

    /// A plain Interact (no Down) registers a normal interaction.
    #[test]
    fn plain_interact_registers() {
        assert!(
            buffered_after(true, 0.0),
            "Interact with no Down must trigger a normal interaction"
        );
    }

    /// The Down-held interact edge is suppressed.
    #[test]
    fn down_interact_is_claimed_by_possession_not_a_normal_interact() {
        assert!(
            !buffered_after(true, 1.0),
            "Down+Interact must be claimed by possession, not open a door/NPC"
        );
    }

    /// Sanity: no interact press → nothing buffered, with or without Down.
    #[test]
    fn no_press_buffers_nothing() {
        assert!(!buffered_after(false, 0.0));
        assert!(!buffered_after(false, 1.0));
    }
}
