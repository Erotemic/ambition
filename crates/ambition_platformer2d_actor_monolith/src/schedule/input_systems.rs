//! Participant → frame populate systems: the schedule-anchored input vocabulary.
//!
//! Bridges the persistent participant's leafwing `ActionState<Platformer2dInputActionMonolith>`
//! into the sim-side `ControlFrame` ([`populate_seat_control_frames`])
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
    MenuInputState, ParticipantContexts, SeatInputContexts, CUTSCENE_CONTEXT, DIALOGUE_CONTEXT,
    GAMEPLAY_CONTEXT,
};
#[cfg(feature = "input")]
use ambition_input::{
    read_gameplay_control_frame_with_settings, read_menu_control_frame,
    Platformer2dInputActionMonolith,
};
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionGatedSimulation, SessionRoot,
};
use ambition_platformer2d_shared_tangle::schedule::{DialogueStopsTheWorld, GameMode};

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

/// The device→frame WRITERS of [`MenuControlFrame`] and its per-seat companion.
///
/// The counterpart of [`MenuNavConsume`], and it existed only as prose until
/// now: an adapter that must add to the frame after it is rebuilt had to name
/// `populate_menu_control_frame_from_actions` directly, from another crate.
///
/// TWO members, and the second one is the argument. `populate_seat_menu_frames`
/// is the per-seat companion, chained immediately after the global populate, and
/// it writes `SeatMenuFrames` and NOTHING else — not `MenuControlFrame`, not
/// `SeatActiveDevices`. A gesture adapter pinning `.after` this set therefore lands
/// in exactly the same observable position as pinning the global populate alone;
/// it shares no mutable state with the extra member it now waits for. That was
/// checked at both signatures rather than assumed from the names.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MenuFramePopulate;

/// The cutscene-skip CONSUMER of [`MenuControlFrame`].
///
/// ONE member, and it is deliberately NOT folded into [`MenuNavConsume`]
/// despite both being consumers of the same frame. That set is documented as
/// the directional-NAV consumers and is pinned `.after` by the menu-backend
/// switch; adding a cutscene system to it would silently make that switch wait
/// on something unrelated to nav.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MenuFrameCutsceneSkip;

/// Umbrella for all [`MenuControlFrame`] consumers in this schedule.
///
/// Bevy set ordering is schedule-local, so all member sets must live here.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MenuFrameConsume;

/// Directional navigation consumers of [`MenuControlFrame`].
///
/// Writers that must precede navigation can order against this set without
/// naming backend-private systems. Use [`MenuFrameConsume`] for all consumers.
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
    // The persisted RECIPE, not just its preset: a player who remapped Jump
    // last session must not spend the first frames of this one — the ones
    // before any settings edit marks the resource changed — on the preset's
    // binding.
    let (preset_index, overrides) = settings.map_or_else(
        || (0, Vec::new()),
        |s| {
            (
                s.controls.keyboard_preset_index,
                s.controls.binding_overrides.clone(),
            )
        },
    );
    let preset = KeyboardPreset::by_index(preset_index);
    let recipe = ambition_input::BindingRecipe::preset(preset.id).with_overrides(overrides);
    commands.spawn((
        InputParticipant::primary(),
        ParticipantContexts::default(),
        ActionState::<Platformer2dInputActionMonolith>::default(),
        recipe.build(),
        recipe,
        // Burst edge state is participant-local so every seat contributes to
        // the merged control-frame producer.
        SeatBurstTriggerState::default(),
    ));
}

/// Keep the PRIMARY participant's [`ambition_input::BindingRecipe`] in step
/// with the persisted keyboard preset AND its binding overrides.
///
/// The settings menu writes `UserSettings.controls` — the ONE binding
/// authority — and this is the engine-owned bridge from that data to the
/// seat's declared recipe; `ambition_input::rebuild_maps_from_recipes` then
/// rebuilds the map. The app-side system this replaces (`sync_preset_input_map`)
/// read its participant with `single_mut()`, so a second seat made a preset
/// change reach nobody — and it shipped only in Ambition's own app, so no demo
/// composition had a rebuild path at all.
///
/// One path for both halves. Changing a preset and changing a single
/// binding are the same operation on the recipe, so a remap reaches behaviour,
/// glyphs (`SeatBindings` projects the rebuilt map) and the touch overlay
/// (its `Changed<InputMap>` hook) in the same frame, through the machinery a
/// preset change already proved.
///
/// Only the primary: couch seats are gamepad-only recipes, and what preset
/// the keyboard player picked is not a fact about their pad. Per-seat presets
/// and per-seat overrides wait on the product question of whether couch seats
/// get their own persisted profiles at all.
#[cfg(feature = "input")]
pub fn sync_primary_recipe_from_settings(
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    mut participants: Query<(&InputParticipant, &mut ambition_input::BindingRecipe)>,
) {
    let Some(settings) = settings else { return };
    if !settings.is_changed() {
        return;
    }
    let id = KeyboardPreset::by_index(settings.controls.keyboard_preset_index).id;
    for (participant, mut recipe) in &mut participants {
        if participant.id != ambition_input::ParticipantId::PRIMARY {
            continue;
        }
        // The layout is not a setting; it is
        // `ambition_input::apply_active_binding_layout_to_recipes`'s to own.
        // ⭐ THE SCOPE IS NOT A SETTING EITHER. A settings edit changes the
        // PROFILE — which preset, which remaps — and must not quietly hand the
        // primary seat back the keyboard-and-pad shape it had before a match
        // froze its seating. Rebuilding the whole recipe from settings is exactly
        // how the old assumption reasserted itself one frame after being fixed.
        let wanted = ambition_input::BindingRecipe::preset(id)
            .with_sources(recipe.sources)
            .with_layout(recipe.layout)
            .with_overrides(settings.controls.binding_overrides.clone());
        // Write only on a real change: `Res<UserSettings>` is marked changed
        // by every settings edit, and most of them are not this field.
        if *recipe != wanted {
            *recipe = wanted;
        }
    }
}

/// One physical source's device scope. Split out so the tests pin THIS rule
/// rather than a second copy of it.
#[cfg(feature = "input")]
fn scope_for_source(
    source: Option<ambition_input::LocalInputSource>,
) -> ambition_input::BindingSources {
    match source {
        Some(ambition_input::LocalInputSource::Keyboard) => {
            ambition_input::BindingSources::KeyboardOnly
        }
        Some(ambition_input::LocalInputSource::Pad(_)) => {
            ambition_input::BindingSources::GamepadOnly
        }
        None => ambition_input::BindingSources::GamepadOnly,
    }
}

/// Which devices the seat on `channel` is entitled to hear.
///
/// ⛔ THE FROZEN PLAN OUTRANKS THE GENERIC POLICY, the same rule
/// `assign_local_seat_devices` already applies one layer down. A surface with no
/// roster (launcher, menus, a headless fixture) has declared nothing, and
/// `Unified` is the right answer there — it is only a DECIDED MATCH that knows
/// who claimed what.
#[cfg(feature = "input")]
fn sources_for_channel(
    roster: Option<&crate::character_runtime::MatchParticipantRoster>,
    channel: ambition_input::ParticipantId,
) -> ambition_input::BindingSources {
    match roster.and_then(|r| r.local_channel_plan().source_for(channel)) {
        Some(source) => scope_for_source(Some(source)),
        // ⛔⛔ NO PLAN YET (a lobby seat nobody has claimed, a menu, a headless
        // fixture). The OLD RULE STILL HOLDS HERE and it is not the same answer
        // for every channel: the primary seat is the one the keyboard drives
        // before anyone declares anything, and an extra seat sharing that one
        // keyboard is not a second player. Returning `Unified` for every
        // unclaimed seat put the keyboard on all of them at once — which broke
        // the lobby exactly the way the frozen-plan bug broke the match, and the
        // existing forward test caught it within a minute.
        None if channel == ambition_input::ParticipantId::PRIMARY => {
            ambition_input::BindingSources::Unified
        }
        None => ambition_input::BindingSources::GamepadOnly,
    }
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
/// ⛔ EXTRA SEATS ARE NOT "THE PAD SEATS". Each seat hears whatever the roster's
/// `LocalChannelPlan` says claimed it — see [`sources_for_channel`]. The older
/// rule ("a second player on the same keyboard as the first is not a second
/// player") is still true, and the plan is what enforces it: the keyboard
/// appears in the plan at most once.
#[cfg(feature = "input")]
pub fn seat_input_participants_for_roster(
    mut commands: Commands,
    roster: Option<Res<crate::character_runtime::MatchParticipantRoster>>,
    offer: Option<Res<ambition_input::LocalSeatOffer>>,
    existing: Query<(Entity, &InputParticipant)>,
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
) {
    // The persisted preset, so a seat claimed with the KEYBOARD gets the layout
    // its player actually chose rather than preset zero.
    let preset_id = KeyboardPreset::by_index(
        settings
            .as_ref()
            .map(|s| s.controls.keyboard_preset_index)
            .unwrap_or(0),
    )
    .id;
    // CHANNELS, not the SOURCES the roster names. This collected each
    // human seat's `device_slot`, which is a lobby source number and is
    // deliberately sparse — so a couch of two people on pads 1 and 2 spawned
    // participants 1 and 2 beside the boot-time primary, three seats for two
    // channels, and the fighter reading `PlayerSlot(2)` sat in a session that
    // opened handles 0 and 1. The plan is the roster's own
    // dense answer, and it is the same one the session is sized from.
    let mut wanted: Vec<u8> = roster
        .as_ref()
        .map(|roster| {
            roster
                .local_channel_plan()
                .channels_with_sources()
                .map(|(channel, _)| channel.slot())
                .filter(|slot| *slot != ambition_input::ParticipantId::PRIMARY.slot())
                .collect()
        })
        .unwrap_or_default();
    // AND the seats a LOBBY is offering. A character select produces the
    // roster, so it cannot be seated from one: without this, only the primary
    // participant exists while the screen is up and every other panel is a chair
    // nobody can reach. The declaration is a frontend surface's, held only while
    // that surface is up, and the sweep below retires these exactly like a
    // match's.
    if let Some(offer) = offer {
        for slot in 0..offer.seats() {
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

    // ⛔⛔ AND A SEAT THAT ALREADY EXISTS MUST FOLLOW THE PLAN TOO. Seats are
    // spawned while the LOBBY is up, before any roster has declared anything, so
    // every one of them starts on the no-plan default. Setting the scope only at
    // spawn meant the match's frozen plan never reached them: the keyboard player
    // who claimed card two kept the gamepad-only seat the lobby gave them and
    // could not move at all. (`a_pad_claiming_the_first_card...` fails on exactly
    // that, at exactly 0.00px.)
    //
    // ⛔ THE PRIMARY IS NOT EXEMPT EITHER. It is `Unified` before a match decides
    // anything, but a match where BOTH fighters are on pads must leave the
    // keyboard driving NEITHER.
    // ⛔⛔ UNCONDITIONALLY, INCLUDING WHEN THE ROSTER IS GONE. A scope applied
    // only while a roster exists is never RETRACTED: quitting a match left the
    // primary holding the seating that match declared, so a host whose smash
    // seat was on the keyboard came back to the launcher with no pad at all.
    // `sources_for_channel` answers "no plan" with the pre-match default, so
    // this same loop both applies and undoes.
    {
        for (entity, participant) in &existing {
            // ⛔⛔ NOT THE ONES JUST DESPAWNED. The sweep above retires seats the
            // roster no longer declares, and queueing a command against a
            // despawned entity trips Bevy's error handler — which surfaces as a
            // panic inside `bevy_ecs`, nowhere near this line.
            if participant.id != ambition_input::ParticipantId::PRIMARY
                && !wanted.contains(&participant.id.slot())
            {
                continue;
            }
            let sources = sources_for_channel(roster.as_deref(), participant.id);
            commands
                .entity(entity)
                .queue(move |mut entity: EntityWorldMut| {
                    let Some(recipe) = entity.get::<ambition_input::BindingRecipe>() else {
                        return;
                    };
                    if recipe.sources == sources {
                        return;
                    }
                    let recipe = recipe.clone().with_sources(sources);
                    let map = recipe.build();
                    entity.insert((recipe, map));
                });
        }
    }

    for slot in wanted {
        let id = ambition_input::ParticipantId(slot);
        if existing.iter().any(|(_, participant)| participant.id == id) {
            continue;
        }
        // ⭐⭐ THE ROSTER'S OWN PLAN DECIDES WHICH DEVICE THIS SEAT HEARS. It was
        // `gamepad_only()` for every non-primary seat, which encoded "player 1
        // is on the keyboard" — so a couch where a PAD claimed card 0 and the
        // KEYBOARD claimed card 1 gave seat 1 no controls at all, while the
        // keyboard went on driving seat 0 as an unintended second controller.
        // `LocalChannelPlan` has recorded the truth since character select; this
        // is the layer that had stopped reading it.
        let recipe = ambition_input::BindingRecipe::preset(preset_id)
            .with_sources(sources_for_channel(roster.as_deref(), id));
        commands.spawn((
            InputParticipant::with_id(id),
            ParticipantContexts::default(),
            ActionState::<Platformer2dInputActionMonolith>::default(),
            recipe.build(),
            recipe,
            SeatBurstTriggerState::default(),
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

/// Declare dialogue and cutscene input contexts from their authoritative owners.
/// Conversation capture is attributed by `ConversationInputOwner`; absent attribution
/// captures nobody. Pause remains separate because it routes menu input rather than a
/// neutral gameplay frame.
#[cfg(feature = "input")]
pub fn declare_in_session_input_contexts(
    cutscene: Res<ambition_cutscene::ActiveCutscene>,
    // `Option` because a composition without the conversation feature still has
    // cutscenes, and a missing authority means no conversation rather than an
    // error.
    conversation: Option<Res<ambition_conversation::ActiveConversation>>,
    mut participants: Query<(&InputParticipant, &mut ParticipantContexts)>,
) {
    let owner = conversation
        .as_deref()
        .and_then(ambition_conversation::ActiveConversation::input_owner);
    let in_cutscene = cutscene.is_playing();
    for (participant, mut contexts) in &mut participants {
        let in_dialogue = match owner {
            Some(ambition_conversation::ConversationInputOwner::Participant(id)) => {
                participant.id == id
            }
            Some(ambition_conversation::ConversationInputOwner::Primary) => {
                participant.id == ambition_input::ParticipantId::PRIMARY
            }
            Some(ambition_conversation::ConversationInputOwner::AllParticipants) => true,
            None => false,
        };
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
    player_input: Query<(
        &InputParticipant,
        &ActionState<Platformer2dInputActionMonolith>,
    )>,
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

/// Per-seat burst edge state.
///
/// Every seat carries its own on its participant entity now — including seat zero, which has a
/// participant entity like everybody else — and that resource is deleted.
#[cfg(feature = "input")]
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct SeatBurstTriggerState(pub ambition_persistence::settings::TriggerEdgeState);

/// EVERY SEAT'S CONTROL FRAME, DECIDED IN ONE PLACE. (C4 couch versus)
///
/// there were TWO of these and they had drifted six ways — seat zero's own producer and
/// this one, registered adjacently under a comment claiming they were interchangeable.
///
/// the FILTERS row moved seat zero, deliberately. Filtering is per PAD and
/// bindings are shared — a deadzone is a fact about the stick in somebody's
/// hands — and seat zero predated that rule. `filters_for_seat` falls back to
/// the machine-wide sliders when no device detector exists, so a headless
/// fixture is unchanged.
///
/// the WORLD-STOPPED row is preserved rather than harmonised, because the
/// field it turns on — `start_pressed` — is read by the trace codec and by
/// nothing in gameplay. Changing it would change the recorded input stream for
/// no gameplay reason.
///
/// ✔ Latched. Every seat's frame folds into `SlotControlLatches` on the FEEL clock and drains
/// on the TICK clock, so a tap that opens and closes between two ticks reaches the sim.
pub fn populate_seat_control_frames(
    mode: Res<State<GameMode>>,
    // Whether a conversation stops the world for everybody. Default `false`
    // ; an experience that wants the modal beat sets it.
    dialogue_policy: Option<Res<DialogueStopsTheWorld>>,
    active_context: Res<SeatInputContexts>,
    user_settings: Res<ambition_persistence::settings::UserSettings>,
    // Which pad each seat is holding, so its FILTERING is that pad's. Optional
    // for the same reason every device resource here is: a headless fixture
    // installs no detector, and absent, every seat falls back to the
    // machine-wide values — which is what it did for all of them before.
    devices: Option<Res<ambition_input::SeatActiveDevices>>,
    mut seats: Query<(
        &InputParticipant,
        &ActionState<Platformer2dInputActionMonolith>,
        &mut SeatBurstTriggerState,
    )>,
    // EVERY SEAT'S DESTINATION, and there is only one now. A raw frame goes here to be shaped;
    // the commit stage folds it into that seat's latch after every shaping stage has run.
    mut raw: ResMut<ambition_characters::control::SeatRawFrames>,
    mut latches: Option<ResMut<ambition_characters::control::SlotControlLatches>>,
    // That is the shape this fork kept producing: not two implementations, one implementation
    // with one caller.
    windows: Query<&Window>,
) {
    // THIS SEAT'S context, not the primary's. Reading one folded answer here is what made a
    // per-seat surface inexpressible: seat N declaring a claim could not reach this router, and
    // seat 0 declaring one silently took gameplay away from everybody else. `mode` stays global on
    // purpose — the world being paused is not a per-seat fact. `stops_the_world`, not
    // `allows_gameplay`. Whether this seat's input routes is the CONTEXT's answer, checked per
    // seat below; this asks only whether the world is running at all.
    let world_running = !mode
        .get()
        .stops_the_world(dialogue_policy.map(|p| *p).unwrap_or_default())
        && !input_suppressed_by_unfocus(&user_settings, windows.iter().map(|w| w.focused));
    for (participant, actions, mut burst) in &mut seats {
        let primary = participant.id == ambition_input::ParticipantId::PRIMARY;
        let gameplay = world_running && active_context.gameplay_owned(participant.id.slot());
        // through the SEAM rather than by arithmetic (R5). This line is the
        // exact shape the reviewer asked new code to stop writing — a bare
        // `PlayerSlot(id.slot())` asserts the two numberings are the same thing,
        // and they are two lifecycles that happen to agree today.
        let slot = crate::participant_seat::player_slot_of(participant.id);
        if !gameplay {
            // Neutral, and RESET the edge, so the post-pause re-press starts from
            // a clean Released state.
            burst.0 = ambition_persistence::settings::TriggerEdgeState::default();
            // seat zero is handed `read_menu_control_frame`, which sets
            // exactly one field: `start_pressed`. Nothing in gameplay reads it
            // — `brain/player.rs` destructures it away and says why: *"pause and
            // reset belong to the session, and a body that could read them could
            // act on somebody else's menu."* Its only readers are the trace codec
            // and the harness's action encoder, so this is what the RECORDED
            // stream contains and changing it would change the wire for no
            // gameplay reason. Every other seat is handed neutral.
            raw.set(
                slot,
                if primary {
                    read_menu_control_frame(actions)
                } else {
                    ControlFrame::default()
                },
            );
            // The latch is CLEARED rather than drained: a seat that has stopped
            // being driven must not hand a held direction to the tick after the
            // pause, and an edge accumulated before it must not survive it.
            if let Some(latches) = latches.as_deref_mut() {
                latches.reset(slot);
            }
            continue;
        }
        // NOT the machine-wide sliders. A couch seat cannot reach the settings screen — it
        // belongs to the primary — so reading their hand-tuned deadzone here meant player two's
        // drifty 360 pad ran on whatever suited player one's DualSense. A deadzone is a fact
        // about the stick in somebody's hands.
        let filters = filters_for_seat(&user_settings, devices.as_deref(), participant.id.slot());
        let (next_frame, next) =
            read_gameplay_control_frame_with_settings(actions, filters, burst.0);
        burst.0 = next;
        // THE ASYMMETRY IS GONE: every seat's raw frame lands in one table, and the shaping
        // stages run over it before anything is committed.
        raw.set(slot, next_frame);
    }
}

/// COMMIT EVERY SEAT'S SHAPED FRAME, once the shaping stages have all run.
///
/// Anything that ran after it would be shaping the frame the latch has already taken, which
/// under GGRS means each peer deriving a flag from its own wall clock.
///
/// it replaces `accumulate_control_frame_latch`, which folded exactly one
/// seat's shaped frame in — because exactly one seat had anywhere for shaping to
/// happen.
#[cfg(feature = "input")]
pub fn commit_seat_raw_frames(
    raw: Res<ambition_characters::control::SeatRawFrames>,
    latches: Option<ResMut<ambition_characters::control::SlotControlLatches>>,
) {
    // `Option`, because the host registers this unconditionally and a
    // frame-stepped composition installs no latch. There it is
    // `publish_seat_controls_when_nobody_else_does` in the sim that commits.
    let Some(mut latches) = latches else {
        return;
    };
    for (slot, frame) in raw.seats() {
        latches.accumulate(slot, frame);
    }
}

/// THE COMMIT FOR A COMPOSITION NOBODY ELSE PUBLISHES FOR.
///
/// THREE hosts publish a seat's frame and only one of them is this.
///
/// ```text
/// fixed-tick  a latch bridges frame→tick; publish_latched_slot_controls drains it
/// rollback    the SESSION publishes, from the input GGRS confirmed
/// frame-step  neither exists — a frame IS a tick — so this copies raw → slots
/// ```
///
/// guarding on the latch ALONE is not enough, and that cost 22 app_it tests. A rollback
/// harness has no latch (it drives `PendingSeatInputs` directly), so a latch-only guard let
/// this run there and overwrite the session's confirmed input with a neutral raw row — both
/// seats went silent through a rewind.
///
/// the predecessor got this for free and that is why it never noticed.
/// `populate_slot_controls` copied the global `ControlFrame`, which under both
/// other hosts had ALREADY been written by whichever authority owned
/// publication — so running it a second time copied the right answer twice. This
/// copies a table that authority does not write, so it has to ask.
#[cfg(feature = "input")]
pub fn publish_seat_controls_when_nobody_else_does(
    latches: Option<Res<ambition_characters::control::SlotControlLatches>>,
    rollback: Option<Res<ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>>,
    raw: Res<ambition_characters::control::SeatRawFrames>,
    mut slots: ResMut<ambition_characters::control::SlotControls>,
) {
    if crate::control::another_authority_publishes(latches.as_deref(), rollback.as_deref()) {
        return;
    }
    for (slot, frame) in raw.seats() {
        slots.set(slot, frame);
    }
}

/// TICK clock: publish EVERY seat's latched frame.
///
/// Not during a REPLAY pass. Under a rollback host the sim schedule is the
/// GGRS schedule, and a resimulated tick re-runs it. Draining a latch there
/// would CONSUME fresh device input on a frame that is supposed to be replaying
/// history — the second drain finds it empty and the seat goes neutral on the
/// replayed tick but not the original, which is a desync the sim itself
/// manufactures.
///
/// The primary seat has no equivalent hazard because GGRS overwrites
/// `ControlFrame` from the session's confirmed inputs after it drains.
#[cfg(feature = "input")]
pub fn publish_latched_slot_controls(
    replay: Option<Res<ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>>,
    mut latches: ResMut<ambition_characters::control::SlotControlLatches>,
    mut slots: ResMut<ambition_characters::control::SlotControls>,
) {
    if replay.is_some_and(|replay| replay.replaying_history) {
        return;
    }
    for slot in 0..ambition_characters::control::SlotControls::MAX_SLOTS {
        let slot = ambition_characters::control::PlayerSlot(slot as u8);
        slots.set(slot, latches.take(slot));
    }
}

/// MIRROR SEAT ZERO'S CONFIRMED FRAME INTO THE GLOBAL `ControlFrame`.
///
/// Shaping happens in `SeatRawFrames` now; what is left of this resource is a READ-ONLY MIRROR of
/// what seat zero actually received, for the consumers that legitimately want it: the forensic
/// trace codec, the harness's action encoder, and the diagnostics that ask whether a driver wrote
/// to the wrong seam.
///
/// Drive input with `drive_slot_frame`.
#[cfg(feature = "input")]
pub fn mirror_primary_slot_to_control_frame(
    slots: Res<ambition_characters::control::SlotControls>,
    mut frame: ResMut<ControlFrame>,
) {
    *frame = slots.get(ambition_characters::control::PlayerSlot::PRIMARY);
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
    // The primary seat owns this resource, so its pad decides the calibration.
    devices: Option<Res<ambition_input::SeatActiveDevices>>,
    player_input: Query<(
        &InputParticipant,
        &ActionState<Platformer2dInputActionMonolith>,
    )>,
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

    // `single()` returns `Err` the moment a SECOND participant exists, so the global menu frame
    // went neutral for everybody: two people at a couch, one presses Start, and the pause menu does
    // not open.
    //
    // The policy is now stated rather than emergent: the primary seat owns
    // the global shell controls. `SeatMenuFrames` is where a per-seat surface
    // (a select screen, four cursors) reads instead, and this resource stays the
    // one global answer a pause menu wants.
    if let Some(actions) = player_input
        .iter()
        .find(|(participant, _)| participant.id == ambition_input::ParticipantId::PRIMARY)
        .map(|(_, actions)| actions)
    {
        let filters = filters_for_seat(
            &user_settings,
            devices.as_deref(),
            ambition_input::ParticipantId::PRIMARY.slot(),
        );
        next = decode_menu_frame(
            actions,
            &mut menu_input_state,
            &user_settings,
            filters,
            wall_dt,
        );
    }

    for ev in mouse_wheel.read() {
        next.scroll_y += ev.y;
    }

    *menu_frame = next;
}

/// Which filters apply to the stick in THIS seat's hands.
///
/// `None` — a headless fixture with no device tracking — reads the machine-wide
/// sliders, which is the honest answer when nothing is known about the device.
#[cfg(feature = "input")]
fn filters_for_seat(
    settings: &ambition_persistence::settings::UserSettings,
    devices: Option<&ambition_input::SeatActiveDevices>,
    slot: u8,
) -> ambition_input::ControlFilters {
    match devices {
        Some(devices) => ambition_input::ControlFilters::for_pad(
            &settings.controls,
            devices.gamepad_style_for(slot),
        ),
        None => ambition_input::ControlFilters::from_settings(&settings.controls),
    }
}

/// One decode, so the global menu frame and every seat's frame agree.
#[cfg(feature = "input")]
pub fn decode_menu_frame(
    actions: &ActionState<Platformer2dInputActionMonolith>,
    menu_input_state: &mut MenuInputState,
    user_settings: &ambition_persistence::settings::UserSettings,
    // RESOLVED by the caller, which is the only place that knows the seat.
    // Passing `UserSettings` alone is what made every seat share one deadzone:
    // this function cannot ask "whose stick is this" and should not try.
    filters: ambition_input::ControlFilters,
    wall_dt: f32,
) -> MenuControlFrame {
    let edge_up = actions.just_pressed(&Platformer2dInputActionMonolith::MenuNavigateUp);
    let edge_down = actions.just_pressed(&Platformer2dInputActionMonolith::MenuNavigateDown);
    let edge_left = actions.just_pressed(&Platformer2dInputActionMonolith::MenuNavigateLeft);
    let edge_right = actions.just_pressed(&Platformer2dInputActionMonolith::MenuNavigateRight);

    let raw = actions.clamped_axis_pair(&Platformer2dInputActionMonolith::MenuStick);
    // the seat's OWN deadzone, not the machine-wide slider. A couch seat
    // cannot reach the settings screen — it belongs to the primary — so reading
    // that slider here meant player two's pad was filtered by whatever suited
    // player one's.
    let (sx, sy) = ambition_persistence::settings::ControlSettings::apply_deadzone(
        raw.x,
        raw.y,
        filters.left_stick_deadzone,
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
    next.back_held = actions.pressed(&Platformer2dInputActionMonolith::MenuBack)
        || actions.pressed(&Platformer2dInputActionMonolith::Reset);
    next.inventory = actions.just_pressed(&Platformer2dInputActionMonolith::Inventory);
    next.map = actions.just_pressed(&Platformer2dInputActionMonolith::Map);
    // Paged-menu page-turn bumpers (Fix 2): just-pressed edge so one bumper tap
    // turns exactly one page, independent of the arrow/d-pad item cursor.
    next.page_left = actions.just_pressed(&Platformer2dInputActionMonolith::MenuPageLeft);
    next.page_right = actions.just_pressed(&Platformer2dInputActionMonolith::MenuPageRight);
    // THE HELD NAVIGATION VECTOR — see `MenuControlFrame::nav`. Everything
    // above is an EDGE, which is what a list wants and what a free cursor
    // cannot be built from; this is the same stick, undecided, so a screen that
    // wants to roam can integrate it.
    //
    // the deadzoned pair, not the raw one. `sx`/`sy` have already been
    // through the SEAT's own deadzone a few lines up, so a drifting stick that
    // cannot pick a `MenuDir` cannot creep a cursor either — one filter, one
    // answer, and the two can never disagree about whether the stick is idle.
    //
    // `sy` is negated. A stick reports `+y` UP and `nav` is screen space,
    // where `+y` is DOWN.
    let held_x = f32::from(actions.pressed(&Platformer2dInputActionMonolith::MenuNavigateRight))
        - f32::from(actions.pressed(&Platformer2dInputActionMonolith::MenuNavigateLeft));
    let held_y = f32::from(actions.pressed(&Platformer2dInputActionMonolith::MenuNavigateDown))
        - f32::from(actions.pressed(&Platformer2dInputActionMonolith::MenuNavigateUp));
    // A d-pad and a stick both held would otherwise sum past full deflection.
    next.nav = bevy::math::Vec2::new(sx + held_x, -sy + held_y).clamp_length_max(1.0);
    next
}

/// Fill one menu frame PER SEAT.
///
/// The global [`MenuControlFrame`] folds every participant into one answer via
/// `single()`, which is right for a pause menu and useless for a character
/// select screen: four people navigating four cursors need four frames, and the
/// question "who pressed lock-in" has no answer in a folded one.
///
/// Repeat state is per seat too.
#[cfg(feature = "input")]
pub fn populate_seat_menu_frames(
    world_time: Option<Res<ambition_time::WorldTime>>,
    // the whole point of a per-seat frame: each seat's stick is filtered by
    // the pad actually in that seat's hands.
    devices: Option<Res<ambition_input::SeatActiveDevices>>,
    participants: Query<(
        &InputParticipant,
        &ActionState<Platformer2dInputActionMonolith>,
    )>,
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
        let filters = filters_for_seat(&user_settings, devices.as_deref(), slot);
        let frame = decode_menu_frame(actions, state, &user_settings, filters, wall_dt);
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
    mut skip_hold: ResMut<ambition_cutscene::CutsceneSkipHold>,
) {
    let wall_dt = world_time.as_deref().map_or(0.0, |time| time.wall_dt());
    update_cutscene_request_from_menu(
        &menu_frame,
        wall_dt,
        cutscene.is_playing(),
        &mut cutscene_request,
        &mut skip_hold,
    );
}

fn update_cutscene_request_from_menu(
    menu_frame: &MenuControlFrame,
    wall_dt: f32,
    is_playing: bool,
    request: &mut ambition_cutscene::CutsceneAdvanceRequest,
    // the accumulator is INPUT-LOCAL and the request is the crossing. Only
    // the completed edge (`skip_cutscene`) reaches the sim; the partial hold
    // stays here and is drawn by the HUD. See `CutsceneSkipHold`.
    hold: &mut ambition_cutscene::CutsceneSkipHold,
) {
    if !is_playing {
        // A partial hold belongs to the cutscene that accumulated it; never
        // let it leak into the next script.
        hold.seconds = 0.0;
        return;
    }
    // Advance is an EDGE. A held confirm must not burn through several beats
    // while the request is consumed on consecutive simulation ticks.
    if menu_frame.select {
        request.dismiss_dialogue = true;
    }
    if menu_frame.back_held {
        hold.seconds += wall_dt;
        if hold.seconds >= ambition_cutscene::SKIP_HOLD_THRESHOLD_SECS {
            request.skip_cutscene = true;
            hold.seconds = 0.0;
        }
    } else {
        hold.seconds = 0.0;
    }
}

#[cfg(all(test, feature = "input"))]
mod focus_gate_tests {
    use super::{
        declare_gameplay_input_context, declare_in_session_input_contexts,
        input_suppressed_by_unfocus, spawn_primary_input_participant,
        update_cutscene_request_from_menu,
    };
    use ambition_input::{
        resolve_active_input_context, InputParticipant, MenuControlFrame, ParticipantContexts,
        ParticipantId, Platformer2dInputActionMonolith, SeatInputContexts,
    };
    use ambition_persistence::settings::UserSettings;
    use ambition_platformer2d_shared_tangle::lifecycle::{SessionRoot, SessionScopeId};
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

    /// A preset change reaches EVERY seat's recipe-derived map, not "the" seat.
    ///
    /// The app-side resync this replaced read its participant with
    /// `single_mut()`, so the moment a second seat existed a preset change
    /// silently reached nobody. The engine path is per-recipe: the primary's
    /// map follows the persisted preset, and a couch seat's gamepad-only map
    /// is not touched by what the keyboard player picked.
    #[test]
    fn a_preset_change_reaches_the_primary_beside_a_second_seat() {
        use crate::character_runtime::{
            ControllerBinding, MatchParticipant, MatchParticipantRoster,
        };
        use ambition_input::{ActionBindings, PhysicalControl};

        let mut app = App::new();
        app.insert_resource(UserSettings::default());
        app.add_systems(
            Update,
            (
                spawn_primary_input_participant,
                super::seat_input_participants_for_roster,
                super::sync_primary_recipe_from_settings,
                ambition_input::rebuild_maps_from_recipes,
            )
                .chain(),
        );
        app.world_mut().insert_resource(MatchParticipantRoster {
            participants: vec![
                MatchParticipant::new("mary_o").driven_by(ControllerBinding::Human {
                    // ⭐ THE KEYBOARD, because this fixture is about a KEYBOARD
                    // preset reaching the primary's map. It declared `Pad(0)`
                    // here until 2026-08-25, when a seat's device scope started
                    // following the roster — under which an all-pad roster means
                    // the primary has no keyboard bindings at all, and a keyboard
                    // preset has nowhere to land. That is correct behaviour, and
                    // this fixture was quietly asking for the opposite.
                    source: ambition_input::LocalInputSource::Keyboard,
                }),
                MatchParticipant::new("sanic").driven_by(ControllerBinding::Human {
                    source: ambition_input::LocalInputSource::Pad(1),
                }),
            ],
            ..Default::default()
        });
        app.update();
        app.update();
        assert_eq!(seat_slots(&mut app), vec![0, 1], "two seats exist");

        app.world_mut()
            .resource_mut::<UserSettings>()
            .controls
            .keyboard_preset_index = 1; // wasd_jkl
        app.update();

        let mut maps = app.world_mut().query::<(
            &InputParticipant,
            &InputMap<Platformer2dInputActionMonolith>,
        )>();
        for (participant, map) in maps.iter(app.world()) {
            let bindings = ActionBindings::from_map(map);
            if participant.id == ParticipantId::PRIMARY {
                assert_eq!(
                    bindings
                        .label(&Platformer2dInputActionMonolith::Jump)
                        .as_deref(),
                    Some("Space"),
                    "the primary's live map follows the preset (WASD binds Jump to Space)"
                );
            } else {
                assert!(
                    !bindings
                        .controls(&Platformer2dInputActionMonolith::Jump)
                        .iter()
                        .any(|control| matches!(control, PhysicalControl::Key(_))),
                    "a couch seat's map binds no key, whatever preset the keyboard player picked"
                );
            }
        }
    }

    /// Changing ONE binding reaches behaviour and glyphs in the same frame,
    /// and only the seat that changed it.
    ///
    /// `F13` is deliberate: no preset binds it, so this cannot pass on a map
    /// nobody overrode.
    #[test]
    fn a_binding_override_reaches_the_primarys_map_and_labels_beside_a_second_seat() {
        use crate::character_runtime::{
            ControllerBinding, MatchParticipant, MatchParticipantRoster,
        };
        use ambition_input::{ActionBindings, BindingOverride, PhysicalControl, SeatBindings};

        let mut app = App::new();
        app.insert_resource(UserSettings::default());
        app.init_resource::<SeatBindings>();
        app.add_systems(
            Update,
            (
                spawn_primary_input_participant,
                super::seat_input_participants_for_roster,
                super::sync_primary_recipe_from_settings,
                ambition_input::rebuild_maps_from_recipes,
                ambition_input::publish_seat_bindings,
            )
                .chain(),
        );
        app.world_mut().insert_resource(MatchParticipantRoster {
            participants: vec![
                MatchParticipant::new("mary_o").driven_by(ControllerBinding::Human {
                    // ⭐ THE KEYBOARD, because this fixture is about a KEYBOARD
                    // preset reaching the primary's map. It declared `Pad(0)`
                    // here until 2026-08-25, when a seat's device scope started
                    // following the roster — under which an all-pad roster means
                    // the primary has no keyboard bindings at all, and a keyboard
                    // preset has nowhere to land. That is correct behaviour, and
                    // this fixture was quietly asking for the opposite.
                    source: ambition_input::LocalInputSource::Keyboard,
                }),
                MatchParticipant::new("sanic").driven_by(ControllerBinding::Human {
                    source: ambition_input::LocalInputSource::Pad(1),
                }),
            ],
            ..Default::default()
        });
        app.update();
        app.update();
        assert_eq!(seat_slots(&mut app), vec![0, 1], "two seats exist");
        let before = app
            .world()
            .resource::<SeatBindings>()
            .label(0, &Platformer2dInputActionMonolith::Jump);
        assert_ne!(
            before.as_deref(),
            Some("F13"),
            "no preset binds F13 — the assertion below cannot pass by accident"
        );

        app.world_mut()
            .resource_mut::<UserSettings>()
            .controls
            .set_binding_override(BindingOverride::key("Jump", KeyCode::F13));
        app.update();

        // Glyphs: the projection every prompt reads names the new key FIRST,
        // which is what a prompt actually prints.
        assert_eq!(
            app.world()
                .resource::<SeatBindings>()
                .label(0, &Platformer2dInputActionMonolith::Jump)
                .as_deref(),
            Some("F13"),
            "the primary's published label is the key the player just bound"
        );

        let mut maps = app.world_mut().query::<(
            &InputParticipant,
            &InputMap<Platformer2dInputActionMonolith>,
        )>();
        for (participant, map) in maps.iter(app.world()) {
            let bindings = ActionBindings::from_map(map);
            let jump = bindings.controls(&Platformer2dInputActionMonolith::Jump);
            if participant.id == ParticipantId::PRIMARY {
                // Behaviour: the map the router reads, not just the read-model.
                assert_eq!(
                    jump.iter()
                        .filter(|control| matches!(control, PhysicalControl::Key(_)))
                        .collect::<Vec<_>>(),
                    vec![&PhysicalControl::Key(KeyCode::F13)],
                    "the live map binds Jump to the override and to no other key"
                );
                // ⭐ AND THE OVERRIDE DISPLACED ONLY THE KEYBOARD HALF. This
                // arm used to assert the PRIMARY still had a pad Jump, which
                // said "a keyboard remap is not a pad remap" only while every
                // primary seat was keyboard-AND-pad by construction. This
                // roster declares the primary on the KEYBOARD, so it has no pad
                // bindings to keep — and the claim now lives where it is real:
                // the pad seat below, whose button Jump must be untouched.
                assert_eq!(
                    jump.iter()
                        .filter(|control| matches!(control, PhysicalControl::Button(_)))
                        .count(),
                    0,
                    "a seat the roster puts on the KEYBOARD is still hearing a pad"
                );
            } else {
                assert!(
                    !jump.contains(&PhysicalControl::Key(KeyCode::F13)),
                    "the couch seat's bindings are its own"
                );
                // ⛔ A KEYBOARD REMAP IS NOT A PAD REMAP — the claim the primary
                // arm above used to carry. This seat is on a pad, and its button
                // Jump must survive somebody else rebinding a key.
                assert!(
                    jump.iter()
                        .any(|control| matches!(control, PhysicalControl::Button(_))),
                    "the pad seat lost its button Jump to a KEYBOARD override"
                );
            }
        }
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
                MatchParticipant::new("mary_o").driven_by(ControllerBinding::Human {
                    source: ambition_input::LocalInputSource::Pad(0),
                }),
                MatchParticipant::new("sanic").driven_by(ControllerBinding::Human {
                    source: ambition_input::LocalInputSource::Pad(1),
                }),
            ],
            ..Default::default()
        });
        app.update();
        assert_eq!(seat_slots(&mut app), vec![0, 1]);

        // A CPU opponent is not a seat: nobody is holding a controller for it.
        app.world_mut().insert_resource(MatchParticipantRoster {
            participants: vec![
                MatchParticipant::new("mary_o").driven_by(ControllerBinding::Human {
                    source: ambition_input::LocalInputSource::Pad(0),
                }),
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

    /// A LOBBY seats its pads before any roster exists.
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
            .insert_resource(ambition_input::LocalSeatOffer::offered(
                "a test lobby",
                3,
                Default::default(),
            ));
        app.update();
        assert_eq!(
            seat_slots(&mut app),
            vec![0, 1, 2],
            "a pad at an offered seat has nothing driving it, so nobody can join"
        );

        // …and leaving the screen takes them back. A participant with no surface
        // still holds an `ActionState` that keeps writing its slot.
        app.world_mut()
            .insert_resource(ambition_input::LocalSeatOffer::default());
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
        // Channels are dense now, so one person is one seat however many controllers are in the
        // room, and asking for `SECONDARY` from a one-person roster would be asking for a chair
        // nobody is sitting in.
        app.world_mut().insert_resource(MatchParticipantRoster {
            participants: vec![
                MatchParticipant::new("mary_o").driven_by(ControllerBinding::Human {
                    source: ambition_input::LocalInputSource::Pad(0),
                }),
                MatchParticipant::new("sanic").driven_by(ControllerBinding::Human {
                    source: ambition_input::LocalInputSource::Pad(1),
                }),
            ],
            ..Default::default()
        });
        app.add_systems(Update, super::seat_input_participants_for_roster);
        app.update();

        let world = app.world_mut();
        let mut seats = world.query::<(
            &InputParticipant,
            &InputMap<Platformer2dInputActionMonolith>,
        )>();
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

    /// A COUCH SEATS ONE PARTICIPANT PER PERSON, not one per controller
    /// NUMBER.
    ///
    /// the shipped Smash couch, exactly: under `JoinToClaim` the select
    /// screen offers the keyboard as source 0 and each pad after it, so two
    /// people on two pads publish a roster of `Pad(0)` and `Pad(1)`… and this
    /// collected the SOURCE numbers. Add the boot-time primary and that is three
    /// participants for a two-handle session — with the third writing a
    /// `PlayerSlot` no GGRS handle ever publishes.
    ///
    /// the roster's channel plan is dense by construction, and it is the same
    /// plan the session is sized from.
    #[test]
    fn a_sparse_couch_seats_one_participant_per_person() {
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
        // Two people, holding the second and fourth controllers in the room.
        app.world_mut().insert_resource(MatchParticipantRoster {
            participants: vec![
                MatchParticipant::new("mary_o").driven_by(ControllerBinding::Human {
                    source: ambition_input::LocalInputSource::Pad(1),
                }),
                MatchParticipant::new("sanic").driven_by(ControllerBinding::Human {
                    source: ambition_input::LocalInputSource::Pad(3),
                }),
            ],
            ..Default::default()
        });
        app.update();
        assert_eq!(
            seat_slots(&mut app),
            vec![0, 1],
            "two people are two participants — seats 1 and 3 would be two chairs \
             nobody is in, plus a session handle that never arrives"
        );
    }

    /// Each seat routes through ITS OWN context.
    ///
    /// The claims were always per-participant; the resolved answer was one
    /// global fold of `ParticipantId::PRIMARY`, and every router read the fold.
    /// So a surface could not give seat 1 a context of its own — seat 1's claim
    /// reached nothing — and seat 0 declaring one silently took gameplay away
    /// from every other seat. That is the shape a character-select screen needs
    /// and could not have.
    ///
    /// this is not the pause case. Pausing is a `GameMode` transition and
    /// stays global; `world_running` below is what expresses it, and the second
    /// half of this test pins that the two gates are independent.
    #[test]
    fn a_seat_browsing_a_menu_stops_driving_its_slot_and_the_others_keep_playing() {
        use ambition_characters::control::{PlayerSlot, SlotControls};
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
                super::SeatBurstTriggerState::default(),
            )
        }

        let mut app = App::new();
        app.init_resource::<SeatInputContexts>();
        app.init_resource::<SlotControls>();
        // and the table it is committed FROM. These are one model:
        // `BrainPlugin` installs both, and a hand-built fixture that takes
        // only the destination is describing a composition that cannot exist.
        app.init_resource::<ambition_characters::control::SeatRawFrames>();
        // Seat zero's destination: every real composition has it, and the merged
        // producer writes row zero there. See its delivery branch.
        app.init_resource::<ambition_input::ControlFrame>();
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
                super::populate_seat_control_frames,
                // the COMMIT, because the producer no longer publishes.
                // It fills every seat's raw row and one stage commits them;
                // a fixture that ran only the producer and then read
                // `SlotControls` would be asserting against a table nothing
                // had written this frame.
                super::publish_seat_controls_when_nobody_else_does,
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

    /// ALT-TAB STOPS EVERY SEAT, NOT JUST SEAT ZERO.
    ///
    /// `pause_input_when_unfocused` is an opt-in that clears the device-agnostic
    /// frames while no window reports focus. Seat zero's producer applied it;
    /// the one every other seat used did not — it never took a
    /// `Query<&Window>` at all, so it could not have. Two people on a couch, the
    /// window loses focus, and player one freezes while player two keeps walking
    /// on a held stick.
    ///
    /// the existing unfocus tests could not see this and were right not to: they exercise
    /// `input_suppressed_by_unfocus` as a pure predicate, which was always correct.
    ///
    /// the control is inside the test: with the setting OFF, the same
    /// unfocused window must leave the seat driving, or this would pass against a
    /// build that had simply stopped seat one for some other reason.
    #[test]
    fn an_unfocused_window_stops_a_secondary_seat_too() {
        use ambition_characters::control::{PlayerSlot, SlotControls};
        use ambition_input::{ContextClaim, GAMEPLAY_CONTEXT};

        fn app_with(pause_when_unfocused: bool) -> App {
            let mut app = App::new();
            app.init_resource::<SeatInputContexts>();
            app.init_resource::<SlotControls>();
            // and the table it is committed FROM. These are one model:
            // `BrainPlugin` installs both, and a hand-built fixture that takes
            // only the destination is describing a composition that cannot exist.
            app.init_resource::<ambition_characters::control::SeatRawFrames>();
            app.init_resource::<ambition_input::ControlFrame>();
            // Seat zero's destination: every real composition has it, and the merged
            // producer writes row zero there. See its delivery branch.
            app.init_resource::<ambition_input::ControlFrame>();
            let mut settings = ambition_persistence::settings::UserSettings::default();
            settings.gameplay.pause_input_when_unfocused = pause_when_unfocused;
            app.insert_resource(settings);
            app.add_plugins(bevy::state::app::StatesPlugin);
            app.insert_state(GameMode::Playing);
            for slot in [0u8, 1] {
                let mut contexts = ParticipantContexts::default();
                contexts.declare(ContextClaim::capturing(
                    GAMEPLAY_CONTEXT,
                    context_priority::GAMEPLAY,
                ));
                let mut actions = ActionState::<Platformer2dInputActionMonolith>::default();
                actions.press(&Platformer2dInputActionMonolith::Jump);
                app.world_mut().spawn((
                    InputParticipant {
                        id: ParticipantId(slot),
                    },
                    contexts,
                    actions,
                    super::SeatBurstTriggerState::default(),
                ));
            }
            // A window that is NOT focused — the whole subject.
            app.world_mut().spawn(bevy::prelude::Window {
                focused: false,
                ..Default::default()
            });
            app.add_systems(
                Update,
                (
                    resolve_active_input_context,
                    super::populate_seat_control_frames,
                    super::publish_seat_controls_when_nobody_else_does,
                    // the COMMIT, because the producer no longer publishes.
                    // It fills every seat's raw row and one stage commits them;
                    // a fixture that ran only the producer and then read
                    // `SlotControls` would be asserting against a table nothing
                    // had written this frame.
                    super::publish_seat_controls_when_nobody_else_does,
                )
                    .chain(),
            );
            app
        }

        let mut off = app_with(false);
        off.update();
        assert!(
            off.world()
                .resource::<SlotControls>()
                .get(PlayerSlot(1))
                .jump_held,
            "precondition: with the guard OFF an unfocused window changes nothing, \
             so seat one is driving and the assertion below is about the guard",
        );

        let mut on = app_with(true);
        on.update();
        assert!(
            !on.world()
                .resource::<SlotControls>()
                .get(PlayerSlot(1))
                .jump_held,
            "the window is unfocused and `pause_input_when_unfocused` is on, and \
             seat one is still holding jump. Seat zero stopped; a couch game just \
             kept walking player two into a pit while the player was reading their \
             email",
        );
    }

    /// A UI surface is a CLAIM now, not a `GameMode` match in every router.
    ///
    /// `participant.rs` has always said *"nothing derives input ownership from
    /// `GameMode`"*, and this crate derived exactly that in two places. The
    /// first half of this test pins that moving them changed nothing; the
    /// second pins what the move BUYS, which is the reason to make it.
    #[test]
    fn an_in_session_surface_claims_input_and_can_claim_it_for_one_seat() {
        use ambition_characters::control::{PlayerSlot, SlotControls};
        use ambition_input::{ContextClaim, DIALOGUE_CONTEXT, GAMEPLAY_CONTEXT};

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
                super::SeatBurstTriggerState::default(),
            )
        }

        let mut app = App::new();
        app.init_resource::<SeatInputContexts>();
        app.init_resource::<SlotControls>();
        // and the table it is committed FROM. These are one model:
        // `BrainPlugin` installs both, and a hand-built fixture that takes
        // only the destination is describing a composition that cannot exist.
        app.init_resource::<ambition_characters::control::SeatRawFrames>();
        // Seat zero's destination: every real composition has it, and the merged
        // producer writes row zero there. See its delivery branch.
        app.init_resource::<ambition_input::ControlFrame>();
        app.init_resource::<ambition_persistence::settings::UserSettings>();
        app.init_resource::<ambition_cutscene::ActiveCutscene>();
        app.init_resource::<ambition_conversation::ActiveConversation>();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.insert_state(GameMode::Playing);
        app.world_mut().spawn(seat(0));
        app.world_mut().spawn(seat(1));
        app.add_systems(
            Update,
            (
                declare_in_session_input_contexts,
                resolve_active_input_context,
                super::populate_seat_control_frames,
                // the COMMIT, because the producer no longer publishes.
                // It fills every seat's raw row and one stage commits them;
                // a fixture that ran only the producer and then read
                // `SlotControls` would be asserting against a table nothing
                // had written this frame.
                super::publish_seat_controls_when_nobody_else_does,
            )
                .chain(),
        );
        app.update();
        assert!(
            app.world()
                .resource::<SlotControls>()
                .get(PlayerSlot(1))
                .jump_held,
            "baseline: a seat that owns gameplay drives its body"
        );

        // 1. A conversation this seat is in suppresses its input — through a
        //    declared claim, and with no router matching `GameMode` any more.
        app.world_mut()
            .resource_mut::<ambition_conversation::ActiveConversation>()
            .open(ambition_conversation::LiveConversation::for_test(
                None,
                None,
                "chat",
                ambition_conversation::ConversationInputOwner::Participant(ParticipantId(1)),
            ));
        app.update();
        let seats = app.world().resource::<SeatInputContexts>();
        assert_eq!(
            seats.for_seat(1).owner(),
            Some(DIALOGUE_CONTEXT),
            "the surface OWNS the seat's input; it does not merely stop the world"
        );
        assert!(!seats.gameplay_owned(1));
        assert!(
            !app.world()
                .resource::<SlotControls>()
                .get(PlayerSlot(1))
                .jump_held,
            "and the router honours it without knowing what dialogue is"
        );

        // 2. What the move buys, proved on a running world through the
        //    PRODUCTION declarer. Seat 0 is in the conversation, seat 1 is not,
        //    and seat 1 keeps driving its body.
        //
        // The whole chain runs now, and the conversation's own owner is what makes seat 1 keep
        // playing.
        app.world_mut()
            .resource_mut::<ambition_conversation::ActiveConversation>()
            .open(ambition_conversation::LiveConversation::for_test(
                None,
                None,
                "chat",
                ambition_conversation::ConversationInputOwner::Participant(ParticipantId(0)),
            ));
        app.update();

        let seats = app.world().resource::<SeatInputContexts>();
        assert_eq!(
            seats.for_seat(0).owner(),
            Some(DIALOGUE_CONTEXT),
            "seat 0 is in the conversation"
        );
        assert!(seats.gameplay_owned(1), "seat 1 is not");
        assert!(
            app.world()
                .resource::<SlotControls>()
                .get(PlayerSlot(1))
                .jump_held,
            "ONE PLAYER READS A DIALOGUE BOX WHILE THE OTHER KEEPS RUNNING — the thing the \
             GameMode gate could not express, and the reason this moved"
        );
    }

    /// ✔ The split is DONE, and this is the half that was left.
    ///
    /// The context claim carries ownership; `stops_the_world` carries the clock; and dialogue now
    /// says only the first.
    ///
    /// the repair was NOT "delete the mode gate". Pausing must keep stopping
    /// everybody, and it does — `Paused`, `RoomTransition` and `Cutscene` still
    /// answer `stops_the_world`, which the second half of this test pins.
    #[test]
    fn dialogue_claims_the_talker_while_a_pause_still_stops_everybody() {
        use ambition_characters::control::{PlayerSlot, SlotControls};
        use ambition_input::{ContextClaim, GAMEPLAY_CONTEXT};

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
        // and the table it is committed FROM. These are one model:
        // `BrainPlugin` installs both, and a hand-built fixture that takes
        // only the destination is describing a composition that cannot exist.
        app.init_resource::<ambition_characters::control::SeatRawFrames>();
        // Seat zero's destination: every real composition has it, and the merged
        // producer writes row zero there. See its delivery branch.
        app.init_resource::<ambition_input::ControlFrame>();
        app.init_resource::<ambition_persistence::settings::UserSettings>();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.insert_state(GameMode::Dialogue);
        app.world_mut().spawn((
            InputParticipant {
                id: ParticipantId(1),
            },
            contexts,
            actions,
            super::SeatBurstTriggerState::default(),
        ));
        app.add_systems(
            Update,
            (
                resolve_active_input_context,
                super::populate_seat_control_frames,
                // the COMMIT, because the producer no longer publishes.
                // It fills every seat's raw row and one stage commits them;
                // a fixture that ran only the producer and then read
                // `SlotControls` would be asserting against a table nothing
                // had written this frame.
                super::publish_seat_controls_when_nobody_else_does,
            )
                .chain(),
        );
        app.update();

        assert!(
            app.world()
                .resource::<SeatInputContexts>()
                .gameplay_owned(1),
            "this seat's CONTEXT is gameplay — no surface claimed it"
        );
        assert!(
            app.world()
                .resource::<SlotControls>()
                .get(PlayerSlot(1))
                .jump_held,
            "and it KEEPS PLAYING: a conversation claims the talker's attention, not the \
             world's clock. This assertion was inverted until 2026-08-06."
        );

        // the half that must NOT move. A pause stops everybody, including a
        // seat with an untouched gameplay context — otherwise this change would
        // have deleted the pause instead of narrowing dialogue.
        app.world_mut()
            .resource_mut::<NextState<GameMode>>()
            .set(GameMode::Paused);
        app.update();
        assert!(
            !app.world()
                .resource::<SlotControls>()
                .get(PlayerSlot(1))
                .jump_held,
            "a PAUSE still stops the world for every seat"
        );

        // And the per-experience opt-in restores the modal beat exactly.
        app.world_mut()
            .resource_mut::<NextState<GameMode>>()
            .set(GameMode::Dialogue);
        app.insert_resource(
            ambition_platformer2d_shared_tangle::schedule::DialogueStopsTheWorld(true),
        );
        app.update();
        assert!(
            !app.world()
                .resource::<SlotControls>()
                .get(PlayerSlot(1))
                .jump_held,
            "an experience that asks for a world-stopping conversation gets the old behaviour \
             back — Jon's 2026-08-03 ruling was that BOTH must be expressible"
        );
    }

    /// A SECOND PLAYER MUST NOT SILENCE THE PAUSE MENU.
    ///
    /// `populate_menu_control_frame_from_actions` folded participants with `single()`, which
    /// returns `Err` the moment there are two — so the global `MenuControlFrame` went neutral
    /// for everybody and pressing Start opened nothing.
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
            app.world_mut()
                .spawn(seat(0, Platformer2dInputActionMonolith::MenuSelect));
            // Every other seat holds something DIFFERENT, so a fold that mixed
            // them would be visible rather than accidentally agreeing.
            for slot in 1..seats as u8 {
                app.world_mut()
                    .spawn(seat(slot, Platformer2dInputActionMonolith::MenuBack));
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

    /// A seat's MENU deadzone comes from the pad in that seat's hands.
    ///
    /// So one couch seat could have its own calibration while driving and player one's while
    /// navigating — the same stick answering two ways depending on which screen is up.
    ///
    /// the magnitude is CHOSEN, not guessed. After the deadzone rescale
    /// `(m - d) / (1 - d)`, a direction needs to clear 0.5. At `m = 0.58`:
    /// PlayStation (`d = 0.14`) gives 0.512 and registers; the Xbox/baseline
    /// table (`d = 0.18`) gives 0.488 and does not. Both sides sit off the
    /// threshold by more than 1%, so this is a real discrimination rather than a
    /// float coin-flip.
    ///
    /// `controller_profile` MUST stay `Default` or this test proves nothing.
    /// `ControlFilters::for_pad` returns the machine values unchanged when somebody has
    /// explicitly picked a profile — an explicit choice is a decision, and detection does not
    /// overrule it.
    #[test]
    fn a_couch_seats_menu_reads_its_own_pads_calibration() {
        use ambition_input::{ActiveDevice, GamepadStyle, SeatActiveDevices, SeatMenuFrames};

        fn seat(slot: u8) -> impl Bundle {
            let mut actions = ActionState::<Platformer2dInputActionMonolith>::default();
            // The SAME physical push for both seats — so any difference in the
            // frames is a difference in calibration and nothing else.
            actions.set_axis_pair(
                &Platformer2dInputActionMonolith::MenuStick,
                bevy::math::Vec2::new(0.0, 0.58),
            );
            (
                InputParticipant {
                    id: ParticipantId(slot),
                },
                ParticipantContexts::default(),
                actions,
            )
        }

        let settings = ambition_persistence::settings::UserSettings::default();
        assert_eq!(
            settings.controls.controller_profile,
            ambition_persistence::settings::controls::ControllerProfileId::Default,
            "precondition: an explicitly chosen profile makes `for_pad` a no-op, \
             which would make this test green without testing anything"
        );

        let mut app = App::new();
        app.insert_resource(settings);
        app.init_resource::<SeatMenuFrames>();
        let mut devices = SeatActiveDevices::default();
        devices.mark(0, ActiveDevice::Gamepad(GamepadStyle::XboxLike));
        devices.mark(1, ActiveDevice::Gamepad(GamepadStyle::PlayStation));
        app.insert_resource(devices);
        app.world_mut().spawn(seat(0));
        app.world_mut().spawn(seat(1));
        app.add_systems(Update, super::populate_seat_menu_frames);
        app.update();

        let frames = app.world().resource::<SeatMenuFrames>();
        let dualsense = frames.for_seat(1);
        assert!(
            dualsense.up || dualsense.down,
            "the DualSense seat's tighter stick cleared its own deadzone and the \
             menu did not notice — it was filtered by whatever the machine-wide \
             slider says, which belongs to a different person's controller"
        );
        let xbox = frames.for_seat(0);
        assert!(
            !xbox.up && !xbox.down,
            "and the seat on the wider table must NOT register the same push — \
             otherwise this passes because both seats got the tight deadzone, \
             which is the same bug facing the other way"
        );
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
        // the accumulator is a SEPARATE resource now: only the completed edge
        // crosses into the sim, and a half-held button is input-local state the
        // HUD draws.
        let mut hold = ambition_cutscene::CutsceneSkipHold::default();

        update_cutscene_request_from_menu(
            &MenuControlFrame {
                select_held: true,
                ..Default::default()
            },
            0.25,
            true,
            &mut request,
            &mut hold,
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
            &mut hold,
        );
        assert!(request.dismiss_dialogue);
        assert_eq!(hold.seconds, 0.25);
        assert!(
            !request.skip_cutscene,
            "a quarter second is not the completed edge, and only the edge crosses"
        );

        update_cutscene_request_from_menu(
            &MenuControlFrame::default(),
            0.25,
            true,
            &mut request,
            &mut hold,
        );
        assert_eq!(
            hold.seconds, 0.0,
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

/// Freeze the local seating once a MATCH has been decided.
///
/// Nothing in a shipped build has ever created [`ambition_input::LocalSeatTopology`]. The only
/// non-test caller is the rollback observatory, behind `#[cfg(feature = "dev_tools")]` — a feature
/// the `android` persona omits and desktop only exercises when somebody presses F9 . Every consumer
/// takes `Option<Res<..>>` and returns early without it, so `reconcile_roster_with_frozen_topology`
/// returned on its first line every frame and `assign_local_seat_devices` always used live
/// discovery.
///
/// and it declares the ROSTER'S seat count (`capture_for_roster`), not the
/// device count, because those are two different authorities and the device one
/// was wrong in both directions.
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
    // HUMANS, not participants. This counted every seat in the roster,
    // CPUs included, and `declared_seats` is read for two things that both mean
    // "how many people are playing on this machine":
    //
    // - it sizes the ggrs session's LOCAL HANDLES, so a one-human-one-CPU smash
    //   match built a two-handle session whose second handle nothing ever wrote;
    // - it picks solo-vs-couch in `assign_local_seat_devices`, where `players
    //   < 2` means "leave leafwing's any-pad behaviour alone". A solo player
    //   against a CPU was taking the COUCH branch, which assigns pads
    //   positionally — fine while their pad is at index 0, and nothing at all
    //   the moment it is not.
    //
    // A CPU seat needs a body and a brain. It does not need a device or a
    // rollback handle, and counting it as though it did is what made the two
    // authorities that size a session disagree.
    // through the roster's own accessor, so this and the session's handle
    // count are ONE definition rather than two that agree by inspection. They
    // did not agree: this counted humans and `SessionSeatingSource::decided` was
    // handed `participants.len()`.
    let plan = roster.local_channel_plan();
    if plan.is_empty() {
        return;
    }
    // Already frozen for THIS roster's shape: leave it exactly as it is. A
    // recapture would advance the generation and every consumer that keys off it
    // would rebuild for no reason.
    //
    // the whole PLAN, not its size. Two rosters can want the same number
    // of channels and a different controller behind each: flip seat one from a
    // pad to a CPU and seat two from a CPU to a pad and the count never moves,
    // while every seat's device does.
    if existing
        .as_deref()
        .is_some_and(|topology| topology.is_frozen() && topology.declared_channels() == Some(&plan))
    {
        return;
    }
    // CARRY THE GENERATION FORWARD, do not restart it. This built a fresh
    // `LocalSeatTopology::default()`, so its counter began at 0 and reached 1 on
    // capture — colliding with the generation the session maintainer had already
    // published from its own device-derived capture. `generation` exists so a
    // consumer can notice a rebuild *"rather than compare vectors"*, and two
    // independent topologies both calling themselves generation 1 is exactly the
    // thing it cannot then notice.
    //
    // It means HUMANS now — which is what `versus_roster_from`'s own parameter is called — so
    // the rebuild it was suppressing produces the right answer, and suppressing it is no longer
    // doing anyone a favour.
    let mut topology = existing.as_deref().cloned().unwrap_or_default();
    topology.capture_for_roster(&order, plan);
    commands.insert_resource(topology);
}

#[cfg(all(test, feature = "input"))]
mod binding_scope_tests {
    use super::*;
    use ambition_input::{BindingSources, LocalInputSource, ParticipantId};

    /// ⭐ THE THREE ANSWERS, and the middle one is the whole bug: a keyboard that
    /// claimed a NON-PRIMARY card must give that seat keyboard bindings.
    #[test]
    fn a_seats_devices_come_from_the_plan_that_seated_it() {
        let plan = ambition_input::LocalChannelPlan::from_sources([
            LocalInputSource::Pad(0),
            LocalInputSource::Keyboard,
        ]);
        assert_eq!(
            plan.source_for(ParticipantId(0)),
            Some(LocalInputSource::Pad(0)),
            "the fixture is not the reversed arrangement it exists to test"
        );
        assert_eq!(
            scope_for_source(plan.source_for(ParticipantId(0))),
            BindingSources::GamepadOnly
        );
        assert_eq!(
            scope_for_source(plan.source_for(ParticipantId(1))),
            BindingSources::KeyboardOnly,
            "a keyboard player in seat two was given a pad-only seat, which is no \
             controls at all"
        );
    }

    /// ⛔⛔ TWO PADS AND NOBODY ON THE KEYBOARD. The primary seat must LOSE its
    /// keyboard here: leaving it is an unintended second controller for player
    /// one, and it is invisible until someone rests a hand on the keys.
    ///
    /// ⚠ WHAT THIS ARM ACTUALLY PINS is that a PAD source scopes a seat to pads
    /// even when that seat is channel ZERO — the primary's `Unified` default is
    /// overridden by any source the plan names for it. A poison on the no-plan
    /// arm passes here, because in an all-pad match every channel IS named.
    #[test]
    fn a_two_pad_match_leaves_the_keyboard_driving_nobody() {
        let plan = ambition_input::LocalChannelPlan::from_sources([
            LocalInputSource::Pad(0),
            LocalInputSource::Pad(1),
        ]);
        assert_eq!(
            plan.keyboard_channel(),
            None,
            "the fixture seats a keyboard"
        );
        for channel in [ParticipantId(0), ParticipantId(1)] {
            assert_eq!(
                scope_for_source(plan.source_for(channel)),
                BindingSources::GamepadOnly,
                "channel {channel:?} still hears the keyboard in an all-pad match"
            );
        }
    }
}
