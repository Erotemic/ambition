//! Participant → frame populate systems: the schedule-anchored input vocabulary.
//!
//! Bridges the persistent participant's leafwing `ActionState<Platformer2dInputActionMonolith>`
//! into the sim-side `ControlFrame` ([`populate_control_frame_from_actions`])
//! and the menu-side [`MenuControlFrame`]
//! ([`populate_menu_control_frame_from_actions`]), the device-agnostic seam
//! the sim/menu read instead of raw devices (ADR 0012). Also:
//! [`MenuNavConsume`] (the set menu-nav consumers join so touch/joystick
//! writers can pin `.before` it), cutscene advance/skip routing,
//! [`spawn_primary_input_participant`] (the boot-time participant spawn), and
//! [`declare_gameplay_input_context`] (the session lifecycle's context
//! claim). All gated behind the `input` feature except the context claim.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::ActionState;

use ambition_input::participant::{context_priority, ContextClaim};
use ambition_input::{
    analog_to_dir, ControlFrame, InputParticipant, KeyboardPreset, MenuControlFrame,
    MenuInputState, ParticipantContexts, PlayerDashTriggerState, SeatInputContexts,
    CUTSCENE_CONTEXT, DIALOGUE_CONTEXT, GAMEPLAY_CONTEXT,
};
#[cfg(feature = "input")]
use ambition_input::{
    read_gameplay_control_frame_with_settings, read_menu_control_frame, Platformer2dInputActionMonolith,
};
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionGatedSimulation, SessionRoot,
};
use ambition_platformer2d_shared_tangle::schedule::GameMode;

/// Item 3 (optional guard): whether input should be SUPPRESSED this frame because
/// the "Pause input when window unfocused" setting is ON and the OS window is not
/// focused. Default OFF, so this returns `false` and nothing changes unless the
/// player opts in. When ON, it returns `true` while no window is focused, and the
/// input population systems clear their frames (same shape as the existing
/// pause/dialogue/cutscene suppression). Reading `Window.focused` keeps the gate
/// minimal — it never touches the leafwing `ActionState`, so the input abstraction
/// is untouched; only the device-agnostic frames are zeroed.
#[cfg(feature = "input")]
fn input_suppressed_by_unfocus(
    settings: &ambition_persistence::settings::UserSettings,
    window_focus: impl IntoIterator<Item = bool>,
) -> bool {
    if !settings.gameplay.pause_input_when_unfocused {
        return false;
    }
    // Suppress when NO window reports focus. A missing window (headless / between
    // frames) is treated as "not focused" only when the guard is enabled, which is
    // the safe direction for this opt-in.
    !window_focus.into_iter().any(|focused| focused)
}

/// The menu-nav CONSUMERS of [`MenuControlFrame`].
///
/// Both inventory backends' directional nav — the bevy_ui Grid
/// (`grid_menu_nav`) and the 3D cube (`kaleidoscope_focus_nav`) — join
/// this set so any writer that must land in the frame BEFORE it is
/// consumed (notably the remaining pointer-gesture scroll adapter) can pin
/// `.before(MenuNavConsume)` without naming each backend's private system.
/// Touch stick navigation itself now arrives through the participant's
/// virtual-device binding before `MenuControlFrame` is populated.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MenuNavConsume;

/// Spawn the persistent primary input participant at boot.
///
/// The participant is the person in front of the controller: it owns the
/// leafwing `ActionState`/`InputMap` and the declared input contexts, exists
/// before any gameplay session (startup cards, launcher), and survives every
/// session teardown/relaunch — device state is never attached to actors or
/// presentation entities. Idempotent by the primary-participant id guard.
#[cfg(feature = "input")]
pub fn spawn_primary_input_participant(
    mut commands: Commands,
    // The persisted setting is the ONE preset authority (`Option` so headless
    // fixtures without a settings resource fall back to preset 0).
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    existing: Query<&InputParticipant>,
) {
    // A future secondary/local participant must not suppress the engine-owned
    // primary seat. Only an existing PRIMARY participant satisfies this boot
    // invariant; multi-participant work will add the other seats explicitly.
    if existing
        .iter()
        .any(|participant| participant.id == ambition_input::ParticipantId::PRIMARY)
    {
        return;
    }
    let preset = KeyboardPreset::by_index(settings.map_or(0, |s| s.controls.keyboard_preset_index));
    commands.spawn((
        InputParticipant::primary(),
        ParticipantContexts::default(),
        ActionState::<Platformer2dInputActionMonolith>::default(),
        preset.input_map(),
    ));
}

/// Give every HUMAN seat the roster declares an input participant, and take
/// away the ones it no longer declares.
///
/// The roster is the authority on who is playing, so it is the authority on how
/// many seats exist. Deriving seats from connected HARDWARE instead would mean
/// a controller left plugged into a machine silently becomes a second player in
/// every game on it.
///
/// The primary seat is not managed here — it is spawned at boot by
/// [`spawn_primary_input_participant`] and outlives every session, because the
/// launcher needs somebody to drive it before any roster exists.
///
/// Extra seats get [`KeyboardPreset::gamepad_only_map`]. A second player on the
/// same keyboard as the first is not a second player.
#[cfg(feature = "input")]
pub fn seat_input_participants_for_roster(
    mut commands: Commands,
    roster: Option<Res<crate::character_runtime::MatchParticipantRoster>>,
    lobby: Option<Res<ambition_input::DeclaredInputSeats>>,
    existing: Query<(Entity, &InputParticipant)>,
) {
    let mut wanted: Vec<u8> = roster
        .map(|roster| {
            roster
                .participants
                .iter()
                .filter_map(|participant| match participant.controller {
                    crate::character_runtime::ControllerBinding::Human { device_slot } => {
                        Some(device_slot)
                    }
                    _ => None,
                })
                .filter(|slot| *slot != ambition_input::ParticipantId::PRIMARY.slot())
                .collect()
        })
        .unwrap_or_default();
    // **AND the seats a LOBBY is offering.** A character select produces the
    // roster, so it cannot be seated from one: without this, only the primary
    // participant exists while the screen is up and every other panel is a chair
    // nobody can reach. The declaration is a frontend surface's, held only while
    // that surface is up, and the sweep below retires these exactly like a
    // match's.
    if let Some(lobby) = lobby {
        for slot in 0..lobby.0 {
            if slot != ambition_input::ParticipantId::PRIMARY.slot() && !wanted.contains(&slot) {
                wanted.push(slot);
            }
        }
    }

    for (entity, participant) in &existing {
        if participant.id != ambition_input::ParticipantId::PRIMARY
            && !wanted.contains(&participant.id.slot())
        {
            // The match is over, or this seat left it. A participant with no
            // seat still holds an `ActionState` that keeps writing its slot.
            commands.entity(entity).despawn();
        }
    }

    for slot in wanted {
        let id = ambition_input::ParticipantId(slot);
        if existing.iter().any(|(_, participant)| participant.id == id) {
            continue;
        }
        commands.spawn((
            InputParticipant::with_id(id),
            ParticipantContexts::default(),
            ActionState::<Platformer2dInputActionMonolith>::default(),
            KeyboardPreset::gamepad_only_map(),
            SeatDashTriggerState::default(),
        ));
    }
}

/// The session lifecycle's context claim: a live gameplay session owns the
/// participant's actions.
///
/// Mirrors `session_world_exists` (the canonical [`SessionRoot`] must exist
/// and, on shell-gated hosts, match the active scope). The SESSION is the
/// surface that owns gameplay input, so the claim follows the session —
/// never `GameMode`, never controlled-body presence.
pub fn declare_gameplay_input_context(
    gate: Option<Res<SessionGatedSimulation>>,
    active_scope: Option<Res<ActiveSessionScope>>,
    roots: Query<&SessionRoot>,
    mut participants: Query<&mut ParticipantContexts, With<InputParticipant>>,
) {
    let session_live = roots.single().is_ok_and(|root| {
        gate.is_none()
            || active_scope
                .as_deref()
                .and_then(ActiveSessionScope::current)
                == Some(root.0)
    });
    for mut contexts in &mut participants {
        // Touch the component only when the claim actually moves.
        if contexts.is_declared(GAMEPLAY_CONTEXT) != session_live {
            contexts.sync(
                ContextClaim::capturing(GAMEPLAY_CONTEXT, context_priority::GAMEPLAY),
                session_live,
            );
        }
    }
}

/// **Declare the in-session UI surfaces as context claims.**
///
/// `participant.rs` has always stated the rule — *"nothing derives input
/// ownership from `GameMode` or from the presence of a controlled body"* — and
/// until now this crate derived exactly that: `populate_control_frame_from_
/// actions` matched `GameMode::Dialogue` and asked `ActiveCutscene` directly,
/// and every other router would have had to match them again. Two authorities
/// for one question, and the doc was the one that lost.
///
/// So the surfaces DECLARE, and the routers read one resolved answer.
///
/// ⚠ **this is behaviour-identical today, on purpose.** It claims on every
/// participant, exactly as the global gates did, so nothing observable moves in
/// this commit. What changes is that the per-seat version — one player reading
/// a dialogue box while another keeps running — becomes a change at THIS
/// function instead of a rewrite at every router. That is the whole point of
/// moving it.
///
/// ⚠ **pause is deliberately NOT here.** `GameMode::Paused` stops the world,
/// which is not a per-seat fact, and the paused path does something a context
/// claim cannot express: it writes a MENU frame into `ControlFrame` rather than
/// a neutral one, so a paused seat can still navigate. Folding it in would
/// silently delete that.
#[cfg(feature = "input")]
pub fn declare_in_session_input_contexts(
    mode: Res<State<GameMode>>,
    cutscene: Res<ambition_cutscene::ActiveCutscene>,
    mut participants: Query<&mut ParticipantContexts, With<InputParticipant>>,
) {
    let in_dialogue = matches!(mode.get(), GameMode::Dialogue);
    let in_cutscene = cutscene.is_playing();
    for mut contexts in &mut participants {
        // Touch the component only when a claim actually moves, so a quiet
        // frame is not a change-detection event for every reader downstream.
        if contexts.is_declared(DIALOGUE_CONTEXT) != in_dialogue {
            contexts.sync(
                ContextClaim::capturing(DIALOGUE_CONTEXT, context_priority::DIALOGUE),
                in_dialogue,
            );
        }
        if contexts.is_declared(CUTSCENE_CONTEXT) != in_cutscene {
            contexts.sync(
                ContextClaim::capturing(CUTSCENE_CONTEXT, context_priority::CUTSCENE),
                in_cutscene,
            );
        }
    }
}

/// Toggle player-trail emission from the logical input action.
///
/// The physical key or button belongs to `KeyboardPreset::input_map`; this bridge
/// only consumes the semantic `Platformer2dInputActionMonolith` and flips the simulation resource
/// that the trail system reads.
#[cfg(feature = "input")]
pub fn toggle_player_trail_emission_from_actions(
    mode: Res<State<GameMode>>,
    active_context: Res<SeatInputContexts>,
    player_input: Query<(&InputParticipant, &ActionState<Platformer2dInputActionMonolith>)>,
    enabled: Option<ResMut<crate::avatar::trail::PlayerTrailEnabled>>,
) {
    // The participant exists at the launcher too; only a session that owns
    // input (and is actually in a gameplay mode) may consume the toggle.
    if !active_context.primary().gameplay_owned() || !mode.get().allows_gameplay() {
        return;
    }
    let Some(mut enabled) = enabled else {
        return;
    };
    // The primary seat, by id — `single()` here had the same defect as the menu
    // frame below: a second player joining silently disabled the toggle.
    let Some(actions) = player_input
        .iter()
        .find(|(participant, _)| participant.id == ambition_input::ParticipantId::PRIMARY)
        .map(|(_, actions)| actions)
    else {
        return;
    };
    if actions.just_pressed(&Platformer2dInputActionMonolith::TrailToggle) {
        enabled.enabled = !enabled.enabled;
    }
}

/// Bridge leafwing's `ActionState` into the sim-side `ControlFrame` resource.
///
/// This is the visible-binary half of the ADR 0012 input seam. The sim
/// reads `Res<ControlFrame>` only — it never queries `ActionState` —
/// which means headless / RL drivers can populate the resource directly
/// without an `InputManagerPlugin` in scope.
///
/// Non-gameplay modes suppress the sim-side `ControlFrame` without
/// mutating leafwing's `ActionState`. Menu systems read their own
/// semantic `MenuControlFrame`, so clearing gameplay here must not
/// make held keyboard/menu buttons look newly pressed every frame.
#[cfg(feature = "input")]
pub fn populate_control_frame_from_actions(
    mode: Res<State<GameMode>>,
    active_context: Res<SeatInputContexts>,
    player_input: Query<(&InputParticipant, &ActionState<Platformer2dInputActionMonolith>)>,
    mut frame: ResMut<ControlFrame>,
    user_settings: Res<ambition_persistence::settings::UserSettings>,
    mut dash_state: ResMut<PlayerDashTriggerState>,
    windows: Query<&Window>,
) {
    // The participant persists across the whole app lifetime, so "no player
    // spawned yet" no longer implies "no ActionState". The resolved input
    // context is the gate: while the launcher/startup (or nothing) owns the
    // participant's actions, gameplay input stays neutral. In-session UI
    // states (pause/dialogue/cutscene) keep their own suppressions below —
    // the session still owns input there.
    // This system authors the PRIMARY seat's frame (see the comment at the
    // `find` below), so it asks the primary seat's context. Every other seat is
    // gated individually in `populate_secondary_slot_controls`.
    if !active_context.primary().gameplay_owned() {
        dash_state.edge = crate::persistence::settings::TriggerEdgeState::default();
        *frame = ControlFrame::default();
        return;
    }

    // Optional unfocus guard: clear gameplay input while the window is unfocused
    // (and the setting is on). Reset the dash edge too so the post-refocus re-press
    // starts clean, mirroring the pause path.
    if input_suppressed_by_unfocus(&user_settings, windows.iter().map(|w| w.focused)) {
        dash_state.edge = crate::persistence::settings::TriggerEdgeState::default();
        *frame = ControlFrame::default();
        return;
    }
    // ⚠ dialogue and cutscene used to be matched HERE, off `GameMode` and
    // `ActiveCutscene`. They are `ContextClaim`s now
    // (`declare_in_session_input_contexts`), so the `gameplay_owned()` check
    // above already covers both and this router has one gate instead of three.
    //
    // Two things the move preserves and one it changes:
    //
    // * the underlying `ActionState` is still untouched, so a held arrow does
    //   not become `just_pressed` again every frame when the surface closes;
    // * the semantic MENU frame is still the sole producer of cutscene
    //   advance/skip, so this bridge only neutralises the simulation packet and
    //   cannot double-count a held confirm;
    // * the dash edge is now reset on a CUTSCENE too, which it was not before.
    //   That is the pause and dialogue rule applied to the third case rather
    //   than a fourth policy — a trigger held across a cutscene needs a
    //   re-press, like one held across a pause.
    // THE PRIMARY seat authors the global `ControlFrame`, by id.
    //
    // This used to take "the only participant" and go NEUTRAL when a second one
    // existed, warning that two would compete to author one frame. The warning
    // was right about the hazard and the remedy was the wrong way round: it made
    // adding a second local player break the FIRST one, silently, with the
    // symptom "gameplay input stopped working". Its own comment named the fix —
    // *real multi-participant support keys frames by ParticipantId → slot* — so
    // that is what this does. Seat 0 owns this resource; every other seat writes
    // its own slot (`populate_secondary_slot_controls`) and cannot touch this one.
    let action_state = player_input
        .iter()
        .find(|(participant, _)| participant.id == ambition_input::ParticipantId::PRIMARY)
        .map(|(_, actions)| actions);
    *frame = match action_state {
        Some(action_state) => {
            if mode.get().allows_gameplay() {
                let (next_frame, next_state) = read_gameplay_control_frame_with_settings(
                    action_state,
                    &user_settings.controls,
                    dash_state.edge,
                );
                dash_state.edge = next_state;
                next_frame
            } else {
                // While paused, suppress gameplay input AND reset the
                // dash trigger state so the post-pause re-press starts
                // from a clean Released edge.
                dash_state.edge = crate::persistence::settings::TriggerEdgeState::default();
                read_menu_control_frame(action_state)
            }
        }
        // No participant exists only in minimal fixtures that never ran the
        // boot spawn. Neutral input is the contract there, not a warning.
        None => ControlFrame::default(),
    };
}

/// Per-seat dash edge state.
///
/// The primary seat's lives in the `PlayerDashTriggerState` RESOURCE, which is
/// correct for exactly one seat and wrong for two: a shared edge means player
/// one's dash release cancels player two's press. A second seat carries its own
/// on its participant entity.
#[cfg(feature = "input")]
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct SeatDashTriggerState(pub crate::persistence::settings::TriggerEdgeState);

/// **Every non-primary seat writes its OWN slot.** (C4 couch versus)
///
/// `populate_slot_controls` fills slot 0 from the global `ControlFrame` and says
/// in its own docs that co-op adds writers for higher slots without touching it.
/// This is that writer.
///
/// It deliberately does not go through `ControlFrame`: that resource is the
/// primary seat's, everything downstream of it is allowlisted as single-frame,
/// and routing a second player through it is what the old "two participants
/// compete" guard was protecting against. Seat N reads its own `ActionState` and
/// publishes straight into `SlotControls[N]`, which is where
/// `tick_player_brains` already looks for it.
///
/// ✔ **Latched now (queue Y2).** This used to end with a known limit: the primary
/// seat's frame passes through `ControlFrameLatch`, which ORs sub-tick press
/// edges together so a tap between two ticks is never swallowed, and a secondary
/// seat had none — so on a fixed-tick host a very short player-two tap could be
/// missed. Bounded and named rather than hidden, which was right, and still a
/// FAIRNESS asymmetry: two people on two pads and only one of them forgiving.
///
/// Under a fixed-tick host this now folds into [`SlotControlLatches`] on the
/// FEEL clock and [`publish_latched_slot_controls`] drains it on the TICK clock —
/// the primary seat's two-system shape, for the primary seat's reason. Under a
/// frame-stepped host one frame IS one tick, no latch is installed, and this
/// writes `SlotControls` directly exactly as before.
#[cfg(feature = "input")]
pub fn populate_secondary_slot_controls(
    mode: Res<State<GameMode>>,
    active_context: Res<SeatInputContexts>,
    user_settings: Res<ambition_persistence::settings::UserSettings>,
    mut seats: Query<(
        &InputParticipant,
        &ActionState<Platformer2dInputActionMonolith>,
        &mut SeatDashTriggerState,
    )>,
    mut slots: ResMut<ambition_characters::brain::SlotControls>,
    // Present only under a fixed-tick host, mirroring `ControlFrameLatch`.
    // Absent, a frame IS a tick and there is nothing to bridge.
    mut latches: Option<ResMut<ambition_characters::brain::SlotControlLatches>>,
) {
    // ⚠ THIS SEAT'S context, not the primary's. Reading one folded answer here
    // is what made a per-seat surface inexpressible: seat N declaring a claim
    // could not reach this router, and seat 0 declaring one silently took
    // gameplay away from everybody else. `mode` stays global on purpose — the
    // world being paused is not a per-seat fact.
    let world_running = mode.get().allows_gameplay();
    for (participant, actions, mut dash) in &mut seats {
        if participant.id == ambition_input::ParticipantId::PRIMARY {
            continue;
        }
        let gameplay = world_running && active_context.gameplay_owned(participant.id.slot());
        let slot = ambition_characters::brain::PlayerSlot(participant.id.slot());
        if !gameplay {
            // Neutral, and RESET the edge, so the post-pause re-press starts from
            // a clean Released state — the same rule the primary seat follows.
            dash.0 = crate::persistence::settings::TriggerEdgeState::default();
            slots.set(slot, ControlFrame::default());
            // The latch is CLEARED rather than drained: a seat that has stopped
            // being driven must not hand a held direction to the tick after the
            // pause, and an edge accumulated before it must not survive it.
            if let Some(latches) = latches.as_deref_mut() {
                latches.reset(slot);
            }
            continue;
        }
        let (frame, next) =
            read_gameplay_control_frame_with_settings(actions, &user_settings.controls, dash.0);
        dash.0 = next;
        match latches.as_deref_mut() {
            // Fixed tick: fold this device sample in and let the tick drain it.
            // Writing `SlotControls` here as well would be the sample racing its
            // own latch — the tick would see whichever ran last.
            Some(latches) => latches.accumulate(slot, frame),
            // Frame-stepped: a frame IS a tick, so publish straight through.
            None => slots.set(slot, frame),
        }
    }
}

/// TICK clock: publish each secondary seat's latched frame. (queue Y2)
///
/// The twin of `publish_latched_control_frame`, and it runs in the same place
/// for the same reason: at the head of the sim's input phase, before any reader.
/// Slot 0 is skipped — it is the primary seat, it already drains
/// `ControlFrameLatch`, and latching it twice would hold one press across two
/// ticks.
///
/// ⚠ **Not during a REPLAY pass.** Under a rollback host the sim schedule is the
/// GGRS schedule, and a resimulated tick re-runs it. Draining a latch there
/// would CONSUME fresh device input on a frame that is supposed to be replaying
/// history — the second drain finds it empty and the seat goes neutral on the
/// replayed tick but not the original, which is a desync the sim itself
/// manufactures.
///
/// The primary seat has no equivalent hazard because GGRS overwrites
/// `ControlFrame` from the session's confirmed inputs after it drains.
///
/// ⚠ this used to add "seats 1-3 are not in the session at all (queue Y1)". That
/// stopped being true on 2026-07-28, when the visible host started sizing its
/// GGRS session from the local seat topology and `publish_seat_controls_from_ggrs`
/// began writing every handle's confirmed input into its seat. The guard below is
/// still right, and now for a plainer reason: this system runs OUTSIDE the GGRS
/// schedule, so on a replayed tick it would still consume live device input that
/// the replay is not supposed to be reading.
#[cfg(feature = "input")]
pub fn publish_latched_slot_controls(
    replay: Option<Res<ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>>,
    mut latches: ResMut<ambition_characters::brain::SlotControlLatches>,
    mut slots: ResMut<ambition_characters::brain::SlotControls>,
) {
    if replay.is_some_and(|replay| replay.replaying_history) {
        return;
    }
    for slot in 1..ambition_characters::brain::SlotControls::MAX_SLOTS {
        let slot = ambition_characters::brain::PlayerSlot(slot as u8);
        slots.set(slot, latches.take(slot));
    }
}

/// Bridge keyboard/gamepad/menu-wheel input into the device-agnostic menu frame.
///
/// Menu systems should read this resource instead of reading raw
/// `ActionState<Platformer2dInputActionMonolith>`. Keyboard, gamepad, and virtual touch controls
/// are already unified in the participant's action state; mouse-wheel and
/// pointer-drag gestures add their scroll contribution before consumers run.
#[cfg(feature = "input")]
pub fn populate_menu_control_frame_from_actions(
    world_time: Option<Res<ambition_time::WorldTime>>,
    player_input: Query<(&InputParticipant, &ActionState<Platformer2dInputActionMonolith>)>,
    mut menu_frame: ResMut<MenuControlFrame>,
    mut menu_input_state: ResMut<MenuInputState>,
    user_settings: Res<ambition_persistence::settings::UserSettings>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    windows: Query<&Window>,
) {
    let wall_dt = world_time.as_deref().map_or(0.0, |time| time.wall_dt());
    let mut next = MenuControlFrame::default();

    // Optional unfocus guard: leave the menu frame cleared while the window is
    // unfocused (and the setting is on). Drain the wheel so a buffered scroll
    // doesn't fire on refocus.
    if input_suppressed_by_unfocus(&user_settings, windows.iter().map(|w| w.focused)) {
        mouse_wheel.clear();
        *menu_frame = next;
        return;
    }

    // ⛔ **THE PRIMARY SEAT, by id — this used to be `single()`.**
    //
    // `single()` returns `Err` the moment a SECOND participant exists, so the
    // global menu frame went neutral for everybody: two people at a couch, one
    // presses Start, and the pause menu does not open. The comment above claimed
    // this folds every participant; it did not fold them, it discarded all of
    // them whenever the count was not exactly one. (GPT 5.6 review, finding 4.)
    //
    // The policy is now stated rather than emergent: **the primary seat owns
    // the global shell controls.** `SeatMenuFrames` is where a per-seat surface
    // (a select screen, four cursors) reads instead, and this resource stays the
    // one global answer a pause menu wants.
    if let Some(actions) = player_input
        .iter()
        .find(|(participant, _)| participant.id == ambition_input::ParticipantId::PRIMARY)
        .map(|(_, actions)| actions)
    {
        next = decode_menu_frame(actions, &mut menu_input_state, &user_settings, wall_dt);
    }

    for ev in mouse_wheel.read() {
        next.scroll_y += ev.y;
    }

    *menu_frame = next;
}

/// **One decode, so the global menu frame and every seat's frame agree.**
///
/// Extracted 2026-07-31 when the seat-keyed frames landed: two implementations
/// of "what does this controller mean in a menu" would drift the first time one
/// of them learned about a new binding, and the drift would show up as one
/// screen where the shoulder buttons work and another where they do not.
#[cfg(feature = "input")]
pub fn decode_menu_frame(
    actions: &ActionState<Platformer2dInputActionMonolith>,
    menu_input_state: &mut MenuInputState,
    user_settings: &ambition_persistence::settings::UserSettings,
    wall_dt: f32,
) -> MenuControlFrame {
    let edge_up = actions.just_pressed(&Platformer2dInputActionMonolith::MenuNavigateUp);
    let edge_down = actions.just_pressed(&Platformer2dInputActionMonolith::MenuNavigateDown);
    let edge_left = actions.just_pressed(&Platformer2dInputActionMonolith::MenuNavigateLeft);
    let edge_right = actions.just_pressed(&Platformer2dInputActionMonolith::MenuNavigateRight);

    let raw = actions.clamped_axis_pair(&Platformer2dInputActionMonolith::MenuStick);
    let (sx, sy) = ambition_persistence::settings::ControlSettings::apply_deadzone(
        raw.x,
        raw.y,
        user_settings.controls.left_stick_deadzone,
    );
    let analog_dir = analog_to_dir(sx, sy, 0.5);

    let input = menu_input_state.step(
        edge_up,
        edge_down,
        edge_left,
        edge_right,
        analog_dir,
        actions.just_pressed(&Platformer2dInputActionMonolith::MenuSelect),
        actions.just_pressed(&Platformer2dInputActionMonolith::MenuBack),
        actions.just_pressed(&Platformer2dInputActionMonolith::Start),
        wall_dt,
        user_settings.controls.menu_repeat_initial_delay,
        user_settings.controls.menu_repeat_interval,
    );
    let mut next = MenuControlFrame::from_menu_input(input);
    next.select_held = actions.pressed(&Platformer2dInputActionMonolith::MenuSelect)
        || actions.pressed(&Platformer2dInputActionMonolith::Jump)
        || actions.pressed(&Platformer2dInputActionMonolith::Interact);
    next.back_held =
        actions.pressed(&Platformer2dInputActionMonolith::MenuBack) || actions.pressed(&Platformer2dInputActionMonolith::Reset);
    next.inventory = actions.just_pressed(&Platformer2dInputActionMonolith::Inventory);
    next.map = actions.just_pressed(&Platformer2dInputActionMonolith::Map);
    // Paged-menu page-turn bumpers (Fix 2): just-pressed edge so one bumper tap
    // turns exactly one page, independent of the arrow/d-pad item cursor.
    next.page_left = actions.just_pressed(&Platformer2dInputActionMonolith::MenuPageLeft);
    next.page_right = actions.just_pressed(&Platformer2dInputActionMonolith::MenuPageRight);
    next
}

/// **Fill one menu frame PER SEAT.**
///
/// The global [`MenuControlFrame`] folds every participant into one answer via
/// `single()`, which is right for a pause menu and useless for a character
/// select screen: four people navigating four cursors need four frames, and the
/// question "who pressed lock-in" has no answer in a folded one.
///
/// Repeat state is per seat too. One shared [`MenuInputState`] would make seat 2
/// holding a direction reset seat 1's repeat clock — a bug that reads as
/// "the menu is laggy when someone else is scrolling".
#[cfg(feature = "input")]
pub fn populate_seat_menu_frames(
    world_time: Option<Res<ambition_time::WorldTime>>,
    participants: Query<(&InputParticipant, &ActionState<Platformer2dInputActionMonolith>)>,
    mut frames: ResMut<ambition_input::SeatMenuFrames>,
    mut states: Local<std::collections::BTreeMap<u8, MenuInputState>>,
    user_settings: Res<ambition_persistence::settings::UserSettings>,
    windows: Query<&Window>,
) {
    frames.clear();
    if input_suppressed_by_unfocus(&user_settings, windows.iter().map(|w| w.focused)) {
        return;
    }
    let wall_dt = world_time.as_deref().map_or(0.0, |time| time.wall_dt());
    // Sorted by slot so the order this writes in is the order it reads in — a
    // menu whose seats resolve in query order is a menu that resolves
    // differently between runs (ADR 0023).
    let mut rows: Vec<(u8, &ActionState<Platformer2dInputActionMonolith>)> = participants
        .iter()
        .map(|(participant, actions)| (participant.id.slot(), actions))
        .collect();
    rows.sort_by_key(|(slot, _)| *slot);
    for (slot, actions) in rows {
        let state = states.entry(slot).or_default();
        let frame = decode_menu_frame(actions, state, &user_settings, wall_dt);
        frames.set(slot, frame);
    }
}

/// Cutscene controls are UI/menu intent, not gameplay movement. Keep this
/// small bridge beside the menu frame so touch Confirm/Back can advance or
/// skip cutscenes without teaching the gameplay `ControlFrame` about menu
/// gestures.
#[cfg(feature = "input")]
pub fn apply_menu_frame_to_cutscene_request(
    world_time: Option<Res<ambition_time::WorldTime>>,
    menu_frame: Res<MenuControlFrame>,
    cutscene: Res<ambition_cutscene::ActiveCutscene>,
    mut cutscene_request: ResMut<ambition_cutscene::CutsceneAdvanceRequest>,
) {
    let wall_dt = world_time.as_deref().map_or(0.0, |time| time.wall_dt());
    update_cutscene_request_from_menu(
        &menu_frame,
        wall_dt,
        cutscene.is_playing(),
        &mut cutscene_request,
    );
}

fn update_cutscene_request_from_menu(
    menu_frame: &MenuControlFrame,
    wall_dt: f32,
    is_playing: bool,
    request: &mut ambition_cutscene::CutsceneAdvanceRequest,
) {
    if !is_playing {
        // A partial hold belongs to the cutscene that accumulated it; never
        // let it leak into the next script.
        request.skip_hold_seconds = 0.0;
        return;
    }
    // Advance is an EDGE. A held confirm must not burn through several beats
    // while the request is consumed on consecutive simulation ticks.
    if menu_frame.select {
        request.dismiss_dialogue = true;
    }
    if menu_frame.back_held {
        request.skip_hold_seconds += wall_dt;
        if request.skip_hold_seconds >= ambition_cutscene::SKIP_HOLD_THRESHOLD_SECS {
            request.skip_cutscene = true;
            request.skip_hold_seconds = 0.0;
        }
    } else {
        request.skip_hold_seconds = 0.0;
    }
}

#[cfg(all(test, feature = "input"))]
mod focus_gate_tests {
    use super::{
        declare_gameplay_input_context, declare_in_session_input_contexts,
        input_suppressed_by_unfocus,
        spawn_primary_input_participant, update_cutscene_request_from_menu,
    };
    use ambition_input::{
        resolve_active_input_context, InputParticipant, MenuControlFrame, ParticipantContexts,
        ParticipantId, Platformer2dInputActionMonolith, SeatInputContexts,
    };
    use ambition_persistence::settings::UserSettings;
    use ambition_platformer2d_shared_tangle::lifecycle::{SessionRoot, SessionScopeId};
    use bevy::prelude::*;
    use leafwing_input_manager::prelude::{ActionState, InputMap};

    #[test]
    fn the_participant_spawns_once_and_owns_device_state() {
        let mut app = App::new();
        app.add_systems(Update, spawn_primary_input_participant);

        app.update();
        app.update();

        let mut participants = app
            .world_mut()
            .query_filtered::<Entity, With<InputParticipant>>();
        let all: Vec<Entity> = participants.iter(app.world()).collect();
        assert_eq!(all.len(), 1, "the spawn is idempotent across frames");
        let participant = all[0];
        assert!(
            app.world()
                .entity(participant)
                .contains::<ActionState<Platformer2dInputActionMonolith>>(),
            "the participant owns the leafwing action state"
        );
        assert!(
            app.world()
                .entity(participant)
                .contains::<InputMap<Platformer2dInputActionMonolith>>(),
            "the participant owns the active input map"
        );
    }

    /// The roster is the authority on how many people are playing.
    ///
    /// Not connected hardware: a controller left plugged into a machine must not
    /// silently become a second player in every game on it.
    #[test]
    fn declaring_a_human_seat_creates_it_and_undeclaring_it_takes_it_away() {
        use crate::character_runtime::{
            ControllerBinding, MatchParticipant, MatchParticipantRoster,
        };

        let mut app = App::new();
        app.add_systems(
            Update,
            (
                spawn_primary_input_participant,
                super::seat_input_participants_for_roster,
            )
                .chain(),
        );
        app.update();
        assert_eq!(seat_slots(&mut app), vec![0], "boot seats player one only");

        app.world_mut().insert_resource(MatchParticipantRoster {
            participants: vec![
                MatchParticipant::new("mary_o")
                    .driven_by(ControllerBinding::Human { device_slot: 0 }),
                MatchParticipant::new("sanic")
                    .driven_by(ControllerBinding::Human { device_slot: 1 }),
            ],
            ..Default::default()
        });
        app.update();
        assert_eq!(seat_slots(&mut app), vec![0, 1]);

        // A CPU opponent is not a seat: nobody is holding a controller for it.
        app.world_mut().insert_resource(MatchParticipantRoster {
            participants: vec![
                MatchParticipant::new("mary_o")
                    .driven_by(ControllerBinding::Human { device_slot: 0 }),
                MatchParticipant::new("sanic").driven_by(ControllerBinding::Cpu {
                    brain_profile: Some("medium_striker".into()),
                }),
            ],
            ..Default::default()
        });
        app.update();
        assert_eq!(seat_slots(&mut app), vec![0]);

        // And the match ending takes the seat with it. A participant with no
        // seat still holds an `ActionState` that keeps writing its slot, so the
        // next game would start with a ghost second player in it.
        app.world_mut().remove_resource::<MatchParticipantRoster>();
        app.update();
        assert_eq!(
            seat_slots(&mut app),
            vec![0],
            "the primary seat outlives every match; it is the launcher's driver"
        );
    }

    /// **A LOBBY seats its pads before any roster exists.**
    ///
    /// The roster is the authority on who is PLAYING, and a character select is
    /// what produces the roster — so the screen that asks the question cannot be
    /// seated by the answer. Four people at four pads found one cursor between
    /// them: three panels said "press confirm to join" at chairs no controller
    /// could reach.
    #[test]
    fn a_declared_lobby_seats_its_pads_and_closing_it_takes_them_back() {
        let mut app = App::new();
        app.add_systems(
            Update,
            (
                spawn_primary_input_participant,
                super::seat_input_participants_for_roster,
            )
                .chain(),
        );
        app.update();
        assert_eq!(seat_slots(&mut app), vec![0], "boot seats player one only");

        // The select screen opens with three pads plugged in.
        app.world_mut()
            .insert_resource(ambition_input::DeclaredInputSeats(3));
        app.update();
        assert_eq!(
            seat_slots(&mut app),
            vec![0, 1, 2],
            "a pad at an offered seat has nothing driving it, so nobody can join"
        );

        // …and leaving the screen takes them back. A participant with no surface
        // still holds an `ActionState` that keeps writing its slot.
        app.world_mut()
            .insert_resource(ambition_input::DeclaredInputSeats(0));
        app.update();
        assert_eq!(seat_slots(&mut app), vec![0]);
    }

    /// Player two's bindings must not include player one's keyboard.
    #[test]
    fn a_declared_seat_is_bound_to_a_controller_and_not_the_keyboard() {
        use crate::character_runtime::{
            ControllerBinding, MatchParticipant, MatchParticipantRoster,
        };

        let mut app = App::new();
        app.world_mut().insert_resource(MatchParticipantRoster {
            participants: vec![MatchParticipant::new("sanic")
                .driven_by(ControllerBinding::Human { device_slot: 1 })],
            ..Default::default()
        });
        app.add_systems(Update, super::seat_input_participants_for_roster);
        app.update();

        let world = app.world_mut();
        let mut seats = world.query::<(&InputParticipant, &InputMap<Platformer2dInputActionMonolith>)>();
        let (_, map) = seats
            .iter(world)
            .find(|(seat, _)| seat.id == ParticipantId::SECONDARY)
            .expect("the declared seat exists");
        for (action, binding) in map.buttonlike_bindings() {
            assert!(
                !binding.as_reflect().reflect_type_path().contains("KeyCode"),
                "{action:?} is bound to a key on the second seat — player one is \
                 typing on that keyboard"
            );
        }
    }

    /// **Each seat routes through ITS OWN context.**
    ///
    /// The claims were always per-participant; the resolved answer was one
    /// global fold of `ParticipantId::PRIMARY`, and every router read the fold.
    /// So a surface could not give seat 1 a context of its own — seat 1's claim
    /// reached nothing — and seat 0 declaring one silently took gameplay away
    /// from every other seat. That is the shape a character-select screen needs
    /// and could not have.
    ///
    /// ⚠ this is not the pause case. Pausing is a `GameMode` transition and
    /// stays global; `world_running` below is what expresses it, and the second
    /// half of this test pins that the two gates are independent.
    #[test]
    fn a_seat_browsing_a_menu_stops_driving_its_slot_and_the_others_keep_playing() {
        use ambition_characters::brain::{PlayerSlot, SlotControls};
        use ambition_input::participant::context_priority;
        use ambition_input::{ContextClaim, GAMEPLAY_CONTEXT, LAUNCHER_CONTEXT};
        use ambition_platformer2d_shared_tangle::schedule::GameMode;

        fn seat(slot: u8, context: ambition_input::InputContextId, priority: i32) -> impl Bundle {
            let mut contexts = ParticipantContexts::default();
            contexts.declare(ContextClaim::capturing(context, priority));
            let mut actions = ActionState::<Platformer2dInputActionMonolith>::default();
            // Hold JUMP on every seat, so the only thing that can differ
            // downstream is the context — not what the player is doing. (Held
            // rather than an edge: `just_pressed` needs a tick this bare
            // `ActionState` never gets, and the question here is routing.)
            actions.press(&Platformer2dInputActionMonolith::Jump);
            (
                InputParticipant {
                    id: ParticipantId(slot),
                },
                contexts,
                actions,
                super::SeatDashTriggerState::default(),
            )
        }

        let mut app = App::new();
        app.init_resource::<SeatInputContexts>();
        app.init_resource::<SlotControls>();
        app.init_resource::<ambition_persistence::settings::UserSettings>();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.insert_state(GameMode::Playing);
        app.world_mut()
            .spawn(seat(0, GAMEPLAY_CONTEXT, context_priority::GAMEPLAY));
        // Seat 1 is at a menu: its OWN claim captures above gameplay.
        app.world_mut()
            .spawn(seat(1, LAUNCHER_CONTEXT, context_priority::LAUNCHER));
        // Seat 2 is playing, like seat 0.
        app.world_mut()
            .spawn(seat(2, GAMEPLAY_CONTEXT, context_priority::GAMEPLAY));
        app.add_systems(
            Update,
            (
                resolve_active_input_context,
                super::populate_secondary_slot_controls,
            )
                .chain(),
        );
        app.update();

        let slots = app.world().resource::<SlotControls>();
        assert_eq!(
            slots.get(PlayerSlot(1)).jump_held,
            false,
            "the seat holding a menu claim must not drive its body — under the \
             global fold seat 1's own claim reached no router at all"
        );
        assert_ne!(
            slots.get(PlayerSlot(2)).jump_held,
            false,
            "a seat that IS playing keeps playing while another seat browses"
        );

        // Pausing is global and independent: it stops the seats that were
        // playing, without needing anybody's context to move.
        app.insert_state(GameMode::Paused);
        app.update();
        assert_eq!(
            app.world()
                .resource::<SlotControls>()
                .get(PlayerSlot(2))
                .jump_held,
            false,
            "a paused world stops every seat, whatever context each one owns"
        );
    }

    /// **A UI surface is a CLAIM now, not a `GameMode` match in every router.**
    ///
    /// `participant.rs` has always said *"nothing derives input ownership from
    /// `GameMode`"*, and this crate derived exactly that in two places. The
    /// first half of this test pins that moving them changed nothing; the
    /// second pins what the move BUYS, which is the reason to make it.
    #[test]
    fn an_in_session_surface_claims_input_and_can_claim_it_for_one_seat() {
        use ambition_characters::brain::{PlayerSlot, SlotControls};
        use ambition_input::participant::context_priority;
        use ambition_input::{ContextClaim, DIALOGUE_CONTEXT, GAMEPLAY_CONTEXT};
        use ambition_platformer2d_shared_tangle::schedule::GameMode;
        use bevy::ecs::system::RunSystemOnce;

        fn seat(slot: u8) -> impl Bundle {
            let mut contexts = ParticipantContexts::default();
            contexts.declare(ContextClaim::capturing(
                GAMEPLAY_CONTEXT,
                context_priority::GAMEPLAY,
            ));
            let mut actions = ActionState::<Platformer2dInputActionMonolith>::default();
            actions.press(&Platformer2dInputActionMonolith::Jump);
            (
                InputParticipant {
                    id: ParticipantId(slot),
                },
                contexts,
                actions,
                super::SeatDashTriggerState::default(),
            )
        }

        let mut app = App::new();
        app.init_resource::<SeatInputContexts>();
        app.init_resource::<SlotControls>();
        app.init_resource::<ambition_persistence::settings::UserSettings>();
        app.init_resource::<ambition_cutscene::ActiveCutscene>();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.insert_state(GameMode::Playing);
        app.world_mut().spawn(seat(0));
        app.world_mut().spawn(seat(1));
        app.add_systems(
            Update,
            (
                declare_in_session_input_contexts,
                resolve_active_input_context,
                super::populate_secondary_slot_controls,
            )
                .chain(),
        );
        app.update();
        assert!(
            app.world().resource::<SlotControls>().get(PlayerSlot(1)).jump_held,
            "baseline: a seat that owns gameplay drives its body"
        );

        // 1. Entering dialogue suppresses input — through a declared claim, and
        //    with no router matching `GameMode` any more.
        app.insert_state(GameMode::Dialogue);
        app.update();
        let seats = app.world().resource::<SeatInputContexts>();
        assert_eq!(
            seats.for_seat(1).owner(),
            Some(DIALOGUE_CONTEXT),
            "the surface OWNS the seat's input; it does not merely stop the world"
        );
        assert!(!seats.gameplay_owned(1));
        assert!(
            !app.world().resource::<SlotControls>().get(PlayerSlot(1)).jump_held,
            "and the router honours it without knowing what dialogue is"
        );

        // 2. **What the move buys, proved on a running world.** A surface can
        //    now own ONE seat's input: seat 0 is in the conversation, seat 1 is
        //    not, and seat 1 keeps driving its body. Under the old `GameMode`
        //    match this was not a thing a surface could ask for at all.
        //
        //    ⚠ done by claiming directly, because nothing declares per-seat
        //    YET: `declare_in_session_input_contexts` still claims on every
        //    participant, exactly as the global gate did. This asserts the seam
        //    supports it, which is what makes the per-seat declarer a change at
        //    one function instead of a rewrite at every router.
        app.insert_state(GameMode::Playing);
        app.update();
        {
            let world = app.world_mut();
            let mut seats = world.query::<(&InputParticipant, &mut ParticipantContexts)>();
            let mut claims: Vec<_> = seats
                .iter_mut(world)
                .map(|(participant, contexts)| (participant.id, contexts))
                .collect();
            for (id, contexts) in &mut claims {
                if *id == ParticipantId(0) {
                    contexts.declare(ContextClaim::capturing(
                        DIALOGUE_CONTEXT,
                        context_priority::DIALOGUE,
                    ));
                }
            }
        }
        // Re-run only the resolver and the router: re-running the declarer
        // would retract what we just claimed, which is the point.
        app.world_mut()
            .run_system_once(resolve_active_input_context)
            .expect("resolver runs");
        app.world_mut()
            .run_system_once(super::populate_secondary_slot_controls)
            .expect("router runs");

        let seats = app.world().resource::<SeatInputContexts>();
        assert_eq!(
            seats.for_seat(0).owner(),
            Some(DIALOGUE_CONTEXT),
            "seat 0 is in the conversation"
        );
        assert!(seats.gameplay_owned(1), "seat 1 is not");
        assert!(
            app.world().resource::<SlotControls>().get(PlayerSlot(1)).jump_held,
            "ONE PLAYER READS A DIALOGUE BOX WHILE THE OTHER KEEPS RUNNING — the thing the \
             GameMode gate could not express, and the reason this moved"
        );
    }

    /// ⛔ **The split is HALF done, and this is the half that is left.**
    ///
    /// `GameMode::Dialogue` still answers two questions with one switch: *the
    /// world is stopped* and *this seat's input belongs to a surface*. The
    /// context claim now carries the second. The first is still
    /// `allows_gameplay()`, and it OVERRIDES — so a per-seat dialogue declarer,
    /// which is the next step, would still find every seat suppressed.
    ///
    /// Pinned as a test rather than left as prose because the failure mode is
    /// somebody building the per-seat declarer, seeing no effect, and concluding
    /// the context seam does not work. It does; this does not, yet.
    ///
    /// ⚠ the repair is NOT "delete the `allows_gameplay` gate". Pausing must
    /// keep stopping everybody. It is to stop `Dialogue` claiming to stop the
    /// world when what it wants is one participant's attention.
    #[test]
    fn dialogue_still_stops_the_world_as_well_as_claiming_the_input() {
        use ambition_characters::brain::{PlayerSlot, SlotControls};
        use ambition_input::participant::context_priority;
        use ambition_input::{ContextClaim, GAMEPLAY_CONTEXT};
        use ambition_platformer2d_shared_tangle::schedule::GameMode;

        let mut contexts = ParticipantContexts::default();
        contexts.declare(ContextClaim::capturing(
            GAMEPLAY_CONTEXT,
            context_priority::GAMEPLAY,
        ));
        let mut actions = ActionState::<Platformer2dInputActionMonolith>::default();
        actions.press(&Platformer2dInputActionMonolith::Jump);

        let mut app = App::new();
        app.init_resource::<SeatInputContexts>();
        app.init_resource::<SlotControls>();
        app.init_resource::<ambition_persistence::settings::UserSettings>();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.insert_state(GameMode::Dialogue);
        app.world_mut().spawn((
            InputParticipant {
                id: ParticipantId(1),
            },
            contexts,
            actions,
            super::SeatDashTriggerState::default(),
        ));
        app.add_systems(
            Update,
            (
                resolve_active_input_context,
                super::populate_secondary_slot_controls,
            )
                .chain(),
        );
        app.update();

        assert!(
            app.world().resource::<SeatInputContexts>().gameplay_owned(1),
            "this seat's CONTEXT is gameplay — no surface claimed it"
        );
        assert!(
            !app.world().resource::<SlotControls>().get(PlayerSlot(1)).jump_held,
            "and it is suppressed anyway, because `GameMode::Dialogue` still says the world is \
             stopped. That is the remaining half of the split, not a bug in the context seam."
        );
    }

    /// ⛔ **A SECOND PLAYER MUST NOT SILENCE THE PAUSE MENU.**
    ///
    /// `populate_menu_control_frame_from_actions` folded participants with
    /// `single()`, which returns `Err` the moment there are two — so the global
    /// `MenuControlFrame` went neutral for everybody and pressing Start opened
    /// nothing. The comment claimed it folded every participant; it discarded
    /// all of them. (GPT 5.6 review, finding 4.)
    ///
    /// The policy is explicit now: the primary seat owns the global shell
    /// controls. A per-seat surface reads `SeatMenuFrames` instead.
    #[test]
    fn a_second_participant_does_not_silence_the_global_menu_frame() {
        fn seat(slot: u8, press: Platformer2dInputActionMonolith) -> impl Bundle {
            let mut actions = ActionState::<Platformer2dInputActionMonolith>::default();
            actions.press(&press);
            (
                InputParticipant {
                    id: ParticipantId(slot),
                },
                ParticipantContexts::default(),
                actions,
            )
        }

        for seats in [1usize, 2, 4] {
            let mut app = App::new();
            app.init_resource::<MenuControlFrame>();
            app.init_resource::<ambition_input::MenuInputState>();
            app.init_resource::<ambition_persistence::settings::UserSettings>();
            app.add_message::<bevy::input::mouse::MouseWheel>();
            app.world_mut().spawn(seat(0, Platformer2dInputActionMonolith::MenuSelect));
            // Every other seat holds something DIFFERENT, so a fold that mixed
            // them would be visible rather than accidentally agreeing.
            for slot in 1..seats as u8 {
                app.world_mut().spawn(seat(slot, Platformer2dInputActionMonolith::MenuBack));
            }
            app.add_systems(Update, super::populate_menu_control_frame_from_actions);
            app.update();

            let frame = *app.world().resource::<MenuControlFrame>();
            assert!(
                frame.select,
                "with {seats} participant(s), the primary's Select must reach the global menu \
                 frame — a couch game whose pause menu stops working when player two joins is \
                 the defect this pins"
            );
            assert!(
                !frame.back,
                "and seat 1's Back must NOT: the primary OWNS this resource, it is not a fold"
            );
        }
    }

    fn seat_slots(app: &mut App) -> Vec<u8> {
        let world = app.world_mut();
        let mut seats = world.query::<&InputParticipant>();
        let mut slots: Vec<u8> = seats.iter(world).map(|seat| seat.id.slot()).collect();
        slots.sort();
        slots
    }

    #[test]
    fn a_secondary_participant_does_not_suppress_the_primary_boot_seat() {
        let mut app = App::new();
        app.world_mut().spawn((
            InputParticipant {
                id: ParticipantId(1),
            },
            ParticipantContexts::default(),
        ));
        app.add_systems(Update, spawn_primary_input_participant);
        app.update();

        let mut participants = app.world_mut().query::<&InputParticipant>();
        let mut ids: Vec<u8> = participants.iter(app.world()).map(|p| p.id.0).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![0, 1]);
    }

    #[test]
    fn cutscene_confirm_is_edge_driven_and_skip_hold_resets_on_release() {
        let mut request = ambition_cutscene::CutsceneAdvanceRequest::default();

        update_cutscene_request_from_menu(
            &MenuControlFrame {
                select_held: true,
                ..Default::default()
            },
            0.25,
            true,
            &mut request,
        );
        assert!(
            !request.dismiss_dialogue,
            "holding confirm without a new edge must not burn through beats"
        );

        update_cutscene_request_from_menu(
            &MenuControlFrame {
                select: true,
                back_held: true,
                ..Default::default()
            },
            0.25,
            true,
            &mut request,
        );
        assert!(request.dismiss_dialogue);
        assert_eq!(request.skip_hold_seconds, 0.25);

        update_cutscene_request_from_menu(&MenuControlFrame::default(), 0.25, true, &mut request);
        assert_eq!(
            request.skip_hold_seconds, 0.0,
            "releasing back resets the hold instead of banking it"
        );
    }

    #[test]
    fn the_session_lifecycle_claims_and_releases_the_gameplay_context() {
        let mut app = App::new();
        app.init_resource::<SeatInputContexts>();
        app.add_systems(
            Update,
            (
                spawn_primary_input_participant,
                declare_gameplay_input_context,
                resolve_active_input_context,
            )
                .chain(),
        );

        // Before any session (startup cards, launcher): nothing claims
        // gameplay, so the participant's actions do not route to the sim.
        app.update();
        assert!(
            !app.world()
                .resource::<SeatInputContexts>()
                .primary()
                .gameplay_owned(),
            "no session -> gameplay context is not owned"
        );

        // A live session claims the context; teardown releases it. The
        // participant entity itself is untouched either way.
        let root = app.world_mut().spawn(SessionRoot(SessionScopeId(7))).id();
        app.update();
        assert!(app
            .world()
            .resource::<SeatInputContexts>()
            .primary()
            .gameplay_owned());
        let participant = {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, With<InputParticipant>>();
            q.single(app.world()).expect("participant exists")
        };
        app.world_mut().despawn(root);
        app.update();
        assert!(
            !app.world()
                .resource::<SeatInputContexts>()
                .primary()
                .gameplay_owned(),
            "session teardown retracts the gameplay claim"
        );
        assert!(
            app.world().get_entity(participant).is_ok(),
            "destroying the session does not destroy the participant"
        );
        assert!(
            app.world()
                .entity(participant)
                .contains::<ActionState<Platformer2dInputActionMonolith>>(),
            "participant device state survives session teardown"
        );
    }

    #[test]
    fn unfocus_gate_is_off_by_default() {
        let settings = UserSettings::default();
        assert!(!settings.gameplay.pause_input_when_unfocused);
        // With the setting OFF, input is never suppressed regardless of focus.
        assert!(!input_suppressed_by_unfocus(&settings, [false]));
        assert!(!input_suppressed_by_unfocus(&settings, [true]));
        assert!(!input_suppressed_by_unfocus(&settings, std::iter::empty()));
    }

    #[test]
    fn unfocus_gate_suppresses_only_when_on_and_no_window_focused() {
        let mut settings = UserSettings::default();
        settings.gameplay.pause_input_when_unfocused = true;
        // Some window focused → not suppressed.
        assert!(!input_suppressed_by_unfocus(&settings, [false, true]));
        assert!(!input_suppressed_by_unfocus(&settings, [true]));
        // No window focused → suppressed.
        assert!(input_suppressed_by_unfocus(&settings, [false, false]));
        // No window at all (headless) → suppressed (safe direction for the opt-in).
        assert!(input_suppressed_by_unfocus(&settings, std::iter::empty()));
    }
}

/// **Freeze the local seating once a MATCH has been decided.**
///
/// ⛔ Nothing in a shipped build has ever created
/// [`ambition_input::LocalSeatTopology`]. The only non-test caller is the
/// rollback observatory, behind `#[cfg(feature = "dev_tools")]` — a feature the
/// `android` persona omits and desktop only exercises when somebody presses F9
/// (queue S35). Every consumer takes `Option<Res<..>>` and returns early without
/// it, so `reconcile_roster_with_frozen_topology` returned on its first line
/// every frame and `assign_local_seat_devices` always used live discovery. The
/// fix for that landed in July; the mechanism that makes it apply did not ship.
///
/// ⚠ **the lifetime is the ROSTER's, not the session's.** An earlier attempt froze
/// at gameplay-session start and broke the Smash flow: the topology existed
/// before any roster did, so the reconciler rebuilt a roster from a device count
/// nobody had declared. Jon's brief says when — *"Before the match starts,
/// freeze: participant, session seat, control channel, input sources"* — and a
/// match starts when a roster says who is in it.
///
/// ⚠ and it declares the ROSTER'S seat count (`capture_for_roster`), not the
/// device count, because those are two different authorities and the device one
/// was wrong in both directions (queue S34).
#[cfg(feature = "input")]
pub fn freeze_local_seating_for_the_decided_match(
    mut commands: Commands,
    roster: Option<Res<crate::character_runtime::MatchParticipantRoster>>,
    order: Res<ambition_input::LocalDeviceOrder>,
    existing: Option<Res<ambition_input::LocalSeatTopology>>,
) {
    let Some(roster) = roster else {
        // No match. A topology that outlives the roster it describes is the
        // previous match's seating presented to the next one as a frozen fact.
        if existing.is_some() {
            commands.remove_resource::<ambition_input::LocalSeatTopology>();
        }
        return;
    };
    let seats = roster.participants.len();
    if seats == 0 {
        return;
    }
    // Already frozen for THIS roster's shape: leave it exactly as it is. A
    // recapture would advance the generation and every consumer that keys off it
    // would rebuild for no reason.
    if existing
        .as_deref()
        .is_some_and(|topology| topology.is_frozen() && topology.declared_seats() == Some(seats))
    {
        return;
    }
    let mut topology = ambition_input::LocalSeatTopology::default();
    topology.capture_for_roster(&order, seats);
    commands.insert_resource(topology);
}
