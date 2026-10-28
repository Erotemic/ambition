//! Player ECS systems.

use bevy::prelude::*;

use ambition_characters::brain::{tick_player_brain, BrainSnapshot};
use ambition_characters::control::ActorControl;
use ambition_characters::control::{DrivingParticipant, SlotControls};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::{BodyGroundState, BodyKinematics};
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

/// The frame a held movement stick was pressed in, kept while it stays held
/// (Jon, 2026-10-19: in ceiling_cross the player-relative camera "used to
/// just flip the screen once and let me continue moving in that direction").
///
/// A body-relative stick (every mode, under the player-relative camera) is
/// read in the body's frame. When gravity turns while the stick is held, the
/// same hold would point the other way in the world: held right into an
/// upward field became world-left, walked the body back out, turned gravity
/// back, and so on (30 turns in 6 s, measured). So a held stick keeps the
/// world direction it had when it was pressed; letting go, or turning the
/// stick, reads it in the body's frame again. Rollback state: the frame of a
/// hold is part of the body's input history.
///
/// A hold belongs to one seat's stick, read under one frame mode. Another
/// seat that drives the body, or a change of the mode, is a new hold, and a
/// body that no seat drives keeps none.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct HeldControlFrame {
    /// The body's `down` when the hold began.
    pub down: ae::Vec2,
    /// The raw stick of the hold, to tell when it turns.
    pub raw: ae::Vec2,
    /// The seat whose stick this is.
    pub slot: ambition_characters::control::PlayerSlot,
    /// The movement frame mode the stick was read under.
    pub mode: ae::InputFrameMode,
}

/// A stick this long is held.
const HELD: f32 = 0.2;
/// A stick turned further than this from its hold (cosine) is a new hold.
const SAME_HOLD_COS: f32 = 0.7;

/// The raw stick to read this tick in the body's frame `down` under `mode`,
/// and the hold to keep: a held stick points where it pointed in the frame it
/// was pressed in ([`HeldControlFrame`]).
pub fn hold_stick_through_turns(
    hold: Option<HeldControlFrame>,
    raw: ae::Vec2,
    down: ae::Vec2,
    mode: ae::InputFrameMode,
    slot: ambition_characters::control::PlayerSlot,
) -> (Option<HeldControlFrame>, ae::Vec2) {
    if raw.length() < HELD {
        return (None, raw);
    }
    let held = hold.filter(|hold| {
        hold.slot == slot && hold.mode == mode && raw.normalize().dot(hold.raw.normalize_or_zero()) >= SAME_HOLD_COS
    });
    let Some(held) = held else {
        return (Some(HeldControlFrame { down, raw, slot, mode }), raw);
    };
    if held.down == down {
        return (Some(held), raw);
    }
    let then = ae::AccelerationFrame::new(held.down);
    let now = ae::AccelerationFrame::new(down);
    let world = then.to_world(then.resolve_input(mode, ae::ScreenAxes::from_vec(raw)).vec());
    let stick = now.raw_axis_for_resolved_input(mode, ae::LocalAxes::from_vec(now.to_local(world)));
    (Some(held), stick.vec())
}





/// Ordering seam meaning participant input has been translated to `ActorControl`.
///
/// Keep this a single-member leaf set; brain-adjacent work belongs in the parent
/// phase, while consumers may order directly after this translation.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ControlledBrainTick;

/// Translate `SlotControls` into `ActorControl` for any participant-driven body.
///
/// `DrivingParticipant` selects the slot; body motion policy supplies movement
/// scale. Vacated or autonomous bodies are skipped. Dormancy does not suppress
/// human-driven bodies because it sleeps AI brain work, not body integration.
///
/// ⛔⛔ **"WHICH BODY DOES SEAT N DRIVE" WAS ANSWERED IN TWO PLACES, AND ONE OF
/// THEM COULD NOT SEE AN AMBIGUITY THE OTHER REFUSED.**
/// [`crate::control::body_driving_seat`] resolves the UNIQUE holder of
/// `DrivingParticipant(slot)` and, when there are two, logs an `error!` and
/// returns `None` — *"refusing ambiguous authority, so this seat drives nothing
/// until one of them vacates."* This translation asked a different question: it
/// iterated BODIES and read `slots.get(driver.0)` off each one, so it could not
/// tell one holder from two. The refusal never reached the road that moves
/// anything.
///
/// ⇒ MEASURED 2026-09-10 before the fix, driving the real headless sim with two
/// bodies holding `DrivingParticipant(PRIMARY)` and one held right press: the
/// player's body travelled 180.00px (175.17px unambiguously) **and the rival
/// travelled 91.67px**. Not "the seat drives nothing" — *the seat drove BOTH*,
/// each at its own locomotion capability. The behaviour was the exact inverse of
/// the documented one, and `control/authority.rs` calls that state "the exact
/// two-writer state this whole component exists to make impossible".
///
/// ⇒ ONE AUTHORITY. The seat is resolved through the same helper every other
/// reader uses, and a body that is not the one it names is skipped. Nothing
/// changes when a seat has exactly one holder, which is every unambiguous tick.
#[allow(clippy::too_many_arguments)]
pub fn tick_controlled_brains(
    mut commands: Commands,
    // ⛔⛤ THE SEAT'S RESOLVED POLICY, not `Res<UserSettings>`. This system is
    // registered into the simulation schedule by
    // `install_avatar_player_input(app, sim)`, so reading the persisted,
    // App-local, menu-mutable settings here meant a rollback resimulation
    // derived frame N's `ActorControl` from `old ControlFrame + the current
    // machine's settings`. Found by the GPT architecture review 2026-09-14, on
    // the main controlled-player brain path, after the census had reported zero.
    //
    // ⚠ **AND THE TABLE IS A SEAM, NOT THE DESTINATION.** It is still read DURING
    // simulation; the stated architecture is that capture resolves the semantic
    slots: Res<SlotControls>,
    drivers: Query<(bevy::prelude::Entity, &DrivingParticipant)>,
    // A body that no seat drives keeps no hold: the next seat to drive it
    // presses its own.
    undriven: Query<bevy::prelude::Entity, (With<HeldControlFrame>, Without<DrivingParticipant>)>,
    mut controlled: Query<(
        bevy::prelude::Entity,
        &BodyKinematics,
        &BodyGroundState,
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        Option<&ambition_platformer2d_core::MotionModel>,
        &DrivingParticipant,
        &mut ActorControl,
        Option<&HeldControlFrame>,
    )>,
) {
    for entity in &undriven {
        commands.entity(entity).remove::<HeldControlFrame>();
    }
    for (entity, kin, ground, resolved_frame, motion_model, driver, mut control, hold) in &mut controlled
    {
        // Input interpretation uses the same resolved frame as this tick's physics.
        let control_down = resolved_frame.down();
        // Input authority is the body's driving slot.
        let slot = driver.0;
        // ⛔ AND THE SEAT'S OWN ANSWER TO WHO HOLDS IT. A body carrying
        // `DrivingParticipant(slot)` is a CLAIM on that seat, not a licence: two
        // claims are ambiguous authority, and the resolver refuses rather than
        // picking by query order. Skipping here is what makes that refusal real
        // instead of advisory.
        if crate::control::body_driving_seat(&drivers, slot) != Some(entity) {
            // ⛔⛔ **NEUTRALISED, NOT SKIPPED — AND SKIPPING WAS THE FIRST FIX,
            // WHICH THE FIXTURE CAUGHT.** `ActorControl` is LATCHED state: it
            // holds whatever was written into it last tick until something
            // writes again. So declining to write it leaves the body running on
            // the last frame it was given, and under a HELD press that is
            // indistinguishable from driving — measured, the player kept
            // travelling its full 180px with the seat refused. "Refuse" has to
            // mean the stick reaches nothing, not "carry on with what you had".
            // ⭐ `neutral()`, THE SPELLING THIS TYPE DOCUMENTS — "construction goes
            // through `ActorControlFrame::neutral`" — and the one
            // `blank_scripted_control_frames` twelve lines up already uses for
            // the same job. `default()` is equal to it and is a second name for
            // one fact.
            control.0 = ambition_characters::actor::control::ActorControlFrame::neutral();
            // And the seat's hold leaves with its authority.
            if hold.is_some() {
                commands.entity(entity).remove::<HeldControlFrame>();
            }
            continue;
        }
        let mut input = slots.get(slot);
        // ⭐ ASKED PER SEAT, beside the seat's own control frame. A frame mode is
        // the comfort preference of the human in that chair; resolving every body
        // against one machine-wide answer is what this migration removed.
        // From the seat's own replayed frame, not from an `Update`-written table.
        let control_frame_modes = input.control_frame_modes;
        // A held stick keeps the world direction it was pressed in through a
        // turn of gravity (`HeldControlFrame`).
        let (next_hold, stick) = hold_stick_through_turns(
            hold.copied(),
            ae::Vec2::new(input.axis_x, input.axis_y),
            control_down,
            control_frame_modes.movement,
            slot,
        );
        if next_hold.as_ref() != hold {
            match next_hold {
                Some(next) => {
                    commands.entity(entity).insert(next);
                }
                None => {
                    commands.entity(entity).remove::<HeldControlFrame>();
                }
            }
        }
        input.axis_x = stick.x;
        input.axis_y = stick.y;
        // Same slot frame plus same body snapshot produces the same control frame.
        let snapshot = BrainSnapshot {
            // A possessed body's brain drives a body a person is steering; it is
            // never in a capture on this road, and saying so beats inheriting a
            // default nobody chose.
            captured: false,
            captured_for: 0.0,
            holding_captive: false,
            pummels_landed: 0,
            // The avatar's own body; the fighter brain is not on this path,
            // but the field is the snapshot's and every builder fills it.
            subject: None,
            actor_pos: kin.pos,
            actor_vel: kin.vel,
            actor_facing: kin.facing,
            // The player brain faces by the stick, never by `face_toward`.
            actor_half_width: kin.size.x * 0.5,
            control_down,
            movement_frame_mode: control_frame_modes.movement,
            aim_frame_mode: control_frame_modes.aim,
            actor_on_ground: ground.on_ground,
            // Human input owns facing policy. Collision facts do not implicitly
            // reverse a controlled body.
            side_contact_normal: None,
            turns_at_walls: false,
            ground_ends_ahead: false,
            // This translation path does not carry an ActorMoveset; fighter attack
            // generation is therefore inactive here.
            attack_kit: Vec::new(),
            melee_reach: None,
            // The player brain reads input, not the Smash aerial path; grounded
            // locomotion semantics regardless of fly mode.
            actor_aerial: false,
            alive: true,
            target_pos: kin.pos,
            target_alive: true,
            // The player brain doesn't regroup on damage; full-health is inert here.
            health_fraction: 1.0,
            // A possessed body's own guards answer nothing on this road.
            guard: Default::default(),
            sim_time: 0.0,
            dt: 0.0,
            // Free-mover velocity targets use the body's own commanded top speed.
            max_run_speed: motion_model.map_or(0.0, |model| model.commanded_top_speed()),
            // The player brain does not predict; it translates a stick. Nothing
            // on this path reads a movement law, and claiming one would be a
            // fact the avatar path never resolved.
            movement_tuning: None,
            // Same reason as the law above: the player brain translates a stick
            // and predicts nothing, so it never asks the kernel a question that
            // would need the kit.
            abilities: None,
            attack_cooldown_remaining: 0.0,
            attack_windup_remaining: 0.0,
            attack_active_remaining: 0.0,
            attack_recover_remaining: 0.0,
            stun_remaining: 0.0,
            // BossPattern-only inputs — inert for the player body.
            boss_encounter_phase: None,
            world_size: ae::Vec2::ZERO,
            front_wall_clearance: None,
            player_input: Some(input),
            // Player brain doesn't consult these fields; leave them
            // None so the snapshot builder doesn't pay for queries
            // the brain ignores.
            crowding: None,
            terrain: None,
            navigation: Default::default(),
            // A person learns from their own eyes whether their move hit; the
            // player brain does not read this field.
            own_move: None,
            // Player brain does not consult this snapshot field; air-jump
            // acceptance remains body-side movement state.
            air_jumps_remaining: 0,
        };
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        // the player-input translator, called DIRECTLY rather than reached
        // through an enum arm. It was only ever a `Brain` variant because the
        // seat was; with the seat named on the body there is nothing left to
        // dispatch on.
        tick_player_brain(slot, &snapshot, &mut frame);
        control.0 = frame;
    }
}

/// **How fast a driven body's Mana refills, as the COMPOSITION's statement.**
///
/// ⛔⛔ ABSENT MEANS NO REFILL. This once defaulted to the platformer's own
/// 14/s when a composition stated nothing, and a Smash stage had to insert a
/// zero to keep that invented rate out of its Limit, which then shared the one
/// meter. Mana is now a named resource a body holds only when its experience
/// declared the pool, and the rate belongs to the same declaration: the
/// Ambition provider states `ambition_abilities::mana::REGEN_PER_SEC` beside
/// the pool it gives the home body.
///
/// ⭐⭐ **AND IT IS THE ONLY ONE OF ITS KIND — swept 2026-09-05, so nobody has to
/// wonder.** Two questions were asked of the whole tree:
///
/// 1. *Which rollback-registered component does more than one CRATE write?*
///    **66 of 256** — which is normal for an ECS and far too noisy to guard.
///    `BodyKinematics` alone has sixteen writers and every one is legitimate.
///    ⇒ Multiple writers was never the hazard. **Two POLICIES for one value
///    was**, and no static shape distinguishes those.
/// 2. *Is there a sibling — a continuous per-tick platformer rule writing a
///    meter a ruleset also owns?* **No.** There is no health regen, and
///    `regen_player_mana` is the only `* dt` writer in this module.
///
/// ⇒ So this resource closes the case rather than opening a family, and the
/// negative result is recorded because "is this the only one" is exactly what
/// the next reader will ask.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, PartialEq)]
pub struct PlayerManaRegen(pub f32);

/// Mana slowly regenerates, at the composition's stated rate, so it is a
/// genuine spendable resource. Clamped at the pool's max and scaled by sim dt,
/// so bullet-time / pause slow it with the world.
///
/// Refills every DRIVEN body's mana — the bodies actually spending it on charge
/// attacks and held abilities — so possessing an actor regenerates that actor's
/// meter, not the vacated home avatar's, and a couch's second seat regenerates
/// at all.
///
/// ⛔⛔ IT REFILLED ONE `ControlledSubject`, which is one entity by construction.
/// Seat one spent mana on a gauntlet it could never get back — a slow leak
/// rather than a dead verb, which is why it outlived the verbs' own fix.
pub fn regen_player_mana(
    time: Res<ambition_time::WorldTime>,
    driven: ambition_held_items::DrivenBodies,
    mut banks: Query<&mut ambition_platformer2d_core::resources::ActorResources>,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    policy: Option<Res<PlayerManaRegen>>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    // The composition's rate; a composition that states none refills nothing.
    let Some(per_second) = policy.map(|p| p.0) else {
        return;
    };
    if per_second <= 0.0 {
        return;
    }
    // ⚠ THE FALLBACK IS THE STARTUP FRAME and nothing else.
    let mut subjects = driven.entities();
    if subjects.is_empty() {
        subjects.extend(primary.single().ok());
    }
    for subject in subjects {
        if let Some(mana) = banks
            .get_mut(subject)
            .ok()
            .and_then(|bank| bank.into_inner().level_of_mut(&ambition_abilities::mana::MANA))
        {
            mana.refill(per_second * dt);
        }
    }
}

#[cfg(test)]
mod tests;
