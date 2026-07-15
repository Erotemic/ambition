//! Player ECS systems.

use bevy::prelude::*;

use super::components::{PlayerEntity, PrimaryPlayer};
use super::events::PlayerHealRequested;
use super::movement_components::{BodyGroundState, BodyKinematics};
use crate::actor::BodyMelee;
use crate::features::ActorPose;
use ambition_characters::actor::{BodyCombat, BodyHealth};
use ambition_characters::brain::{ActorControl, Brain, BrainSnapshot, SlotControls};
use ambition_engine_core as ae;

/// Mirror authoritative player body state into the generic gameplay
/// [`ActorPose`] used by the brain/action resolver.
///
/// The player, NPCs, enemies, and bosses should all expose action origins
/// through gameplay pose data rather than presentation `Transform`s.
pub fn sync_player_actor_poses(
    mut players: Query<(&BodyKinematics, &mut ActorPose), With<PlayerEntity>>,
) {
    for (kin, mut pose) in &mut players {
        *pose = ActorPose::from_parts(kin.pos, kin.size * 0.5, kin.facing);
    }
}

/// Translate each controlled home body's slot frame into its `ActorControl`
/// frame.
///
/// This is the producer for the universal-brain seam on the home/player side —
/// the direct analogue of the `Brain::Player` branch in `tick_actor_brains`.
/// The INPUT AUTHORITY is [`SlotControls`] read by the body's own
/// `Brain::Player(slot)`, NOT `PlayerInputFrame`: the home body is controlled
/// because it carries the player brain for that slot, exactly like a possessed
/// actor. `PlayerInputFrame` is now only a compatibility mirror for player-
/// flavoured ability/UI systems (held item, heal shrine, portal gun) written by
/// `sync_local_player_input_frame`; gameplay brain input no longer depends on it.
///
/// The query requires `&mut Brain`, so a vacated home avatar (its player brain
/// transferred to a possessed actor by `possession`) carries no `Brain` and is
/// skipped — it stays inert with a neutral `ActorControl`. Iterates every home
/// body carrying a player brain; multi-player ready even though only one slot
/// exists today.
pub fn tick_player_brains(
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    slots: Res<SlotControls>,
    mut players: Query<(
        &BodyKinematics,
        &BodyGroundState,
        &ambition_platformer_primitives::frame_env::ResolvedMotionFrame,
        &mut Brain,
        &mut ActorControl,
    )>,
) {
    let control_frame_modes = user_settings
        .as_deref()
        .map_or(ae::ControlFrameModes::default(), |s| {
            s.gameplay.control_frame_modes()
        });

    for (kin, ground, resolved_frame, mut brain, mut control) in &mut players {
        // The body's OWN per-tick resolved frame (ADR 0024): the same value
        // this tick's integration moves the body under, so controller
        // interpretation and physics can never disagree at a zone boundary.
        let control_down = resolved_frame.down();
        // INPUT AUTHORITY: this body's OWN slot frame, keyed by the brain it
        // carries — the SAME `Brain::Player(slot)` → `SlotControls` path a
        // possessed actor reads. A body whose brain isn't a player brain is
        // skipped (its `ActorControl` is owned by an AI tick, not this one).
        let Some(slot) = brain.player_slot() else {
            continue;
        };
        let input = slots.get(slot);
        // Build the snapshot from the player's cluster components plus
        // the per-tick slot frame. The input is what makes
        // Brain::Player's translation deterministic: same input +
        // same body snapshot → same ActorControlFrame.
        let snapshot = BrainSnapshot {
            actor_pos: kin.pos,
            actor_vel: kin.vel,
            actor_facing: kin.facing,
            control_down,
            movement_frame_mode: control_frame_modes.movement,
            aim_frame_mode: control_frame_modes.aim,
            actor_on_ground: ground.on_ground,
            // The player brain reads input, not the Smash aerial path; grounded
            // locomotion semantics regardless of fly mode.
            actor_aerial: false,
            alive: true,
            target_pos: kin.pos,
            target_alive: true,
            // The player brain doesn't regroup on damage; full-health is inert here.
            health_fraction: 1.0,
            sim_time: 0.0,
            dt: 0.0,
            // Player brain emits an already-normalized stick; capability is
            // applied on the player integration side, so this is don't-care here.
            max_run_speed: 0.0,
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
            // Player brain reads its own air-jump state via the
            // PlayerInputFrame / engine path, not via the snapshot.
            air_jumps_remaining: 0,
        };
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        brain.tick(&snapshot, &mut frame);
        control.0 = frame;
    }
}

/// Write the player's read-model fields on [`BodyCombat`] each frame — the
/// symmetric counterpart to the actor's `sync_actor_components_from_cluster`.
///
/// - `attacking` mirrors `BodyMelee::is_swinging()` (a swing in flight, any phase).
/// - `alive` mirrors the body's liveness AUTHORITY, `BodyHealth`. For actors this
///   field is owned by the per-frame sync from their cluster `status.alive`; the
///   player has no such cluster, so without this it kept its spawn default
///   (`false`) forever — a silent "the player is dead" that made every enemy's
///   `target_alive` read false and idle their brain. Owning it here keeps the
///   field correct for every `BodyCombat` reader (HUD / nameplate / health bar /
///   perception / damage gates), so none of them are a footgun for the player.
///   (Liveness-CRITICAL gameplay should still read `BodyHealth` directly — the
///   authority — rather than this once-per-frame mirror, to avoid a tick of lag.)
pub fn write_player_ecs_components(
    mut players: Query<(&BodyMelee, &BodyHealth, &mut BodyCombat), With<PlayerEntity>>,
) {
    for (attack, health, mut combat) in &mut players {
        combat.attacking = attack.is_swinging();
        combat.alive = health.current() > 0;
    }
}

/// Apply heal messages to the authoritative `BodyHealth` ECS component.
///
/// A heal targets either a specific player entity (`heal.target ==
/// Some(entity)`) or the primary player as a fallback (`None`). The
/// fallback path keeps existing call sites — cutscene heals, dev-tool
/// heals — working with no change. Per-player producers like pickup
/// collection should set the target explicitly so a non-primary
/// player who walked into the heart actually gets healed.
pub fn apply_player_heal_requests(
    mut heals: MessageReader<PlayerHealRequested>,
    mut players: Query<&mut BodyHealth, With<PlayerEntity>>,
    primary_q: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    let primary = primary_q.single().ok();
    for heal in heals.read() {
        if heal.amount <= 0 {
            continue;
        }
        let target = heal.target.or(primary);
        let Some(target) = target else {
            // No player entity yet (startup or headless): drop the
            // heal silently so the queue still drains.
            continue;
        };
        if let Ok(mut health) = players.get_mut(target) {
            health.heal(heal.amount);
        }
    }
}

/// Mana regenerated per second (clamped to the meter max).
const MANA_REGEN_PER_SEC: f32 = 14.0;

/// Mana slowly regenerates so it's a genuine spendable resource. Uses
/// `ResourceMeter::refill` (clamped) rather than the meter's own `regen_rate`
/// field so we don't change `BodyMana::default` (and any test that relies on
/// it). Scaled by sim dt, so bullet-time / pause slow it with the world.
///
/// Refills the *controlled subject's* mana — the body actually spending it on
/// charge attacks / the fireball — so possessing an actor regenerates that
/// actor's meter, not the vacated home avatar's. (Moved from the render HUD
/// module, E4: a sim mutator never lives in presentation.)
pub fn regen_player_mana(
    time: Res<ambition_time::WorldTime>,
    controlled: Option<Res<ambition_platformer_primitives::markers::ControlledSubject>>,
    mut manas: Query<&mut crate::actor::BodyMana>,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    let Some(subject) = controlled
        .as_deref()
        .and_then(|subject| subject.0)
        .or_else(|| primary.single().ok())
    else {
        return;
    };
    if let Ok(mut mana) = manas.get_mut(subject) {
        mana.meter.refill(MANA_REGEN_PER_SEC * dt);
    }
}

#[cfg(test)]
mod tests;
