//! Attack-phase runtime: brain-output → engine-input translation and the
//! start/advance melee attack-phase state machine, plus the
//! `attack_advance_system` that drives them.
//!
//! Pure sim + message emission — it writes `SfxMessage`/`VfxMessage`/`HitEvent`
//! *facts* (all reachable from this crate; `VfxMessage` is `ambition_vfx`, not
//! `ambition_render`) and holds no render dependency. It lived in `ambition_app`
//! only because it was authored beside the room/transition glue; it belongs here
//! in the combat runtime with the `AttackSpec`/`AttackView` model it consumes.
//!
//! Player-centrism note: the bodies still name the controlled actor "player"
//! because the component vocabulary (`PlayerAttackState`, `ae::BodyClustersMut`,
//! `ActivePlayerAttack`) does. The relativity-principle fix is the actor-
//! unification rename of those types, tracked separately.

use bevy::prelude::{Entity, MessageReader, MessageWriter, Query, Res, Time};

use ambition_characters::actor::control::ActorControlFrame;
use ambition_characters::brain::{ActorActionMessage, ActorControl, MeleeActionSpec};
use ambition_engine_core::{self as ae, AabbExt};
use ambition_vfx::vfx::{SlashKind, VfxMessage};

use crate::audio::SfxMessage;
use crate::combat::{
    attack_hitbox_from_view, attack_spec_from_view, resolve_attack_intent_from_view, AttackIntent,
    AttackPhase, AttackView,
};
use crate::dev::dev_tools::EditableMovementTuning;
use crate::features::{self, FeatureEcsWorldOverlay};
use crate::player::{ActivePlayerAttack, PlayerAnimState};
use crate::actor::BodyCombat;
use crate::actor::{PrimaryPlayerOnly};
use crate::time::feel::SandboxFeelTuning;
use crate::world::platforms::MovingPlatformState;
use crate::{physics, MovingPlatformSet, PlayerAttackState, RoomGeometry};

/// Build the engine's `InputState` purely from `ActorControl` —
/// the player's brain output is the single source of truth for
/// every input verb the simulation consumes. The polarity flip is
/// now complete: raw `ControlFrame` is no longer consulted inside
/// the player simulation phases.
///
/// The drop-through gesture is no longer precomputed here — the engine forms it
/// gravity-relatively (`movement::wants_drop_through`) from `axis_y + jump`, so
/// it flips correctly under inverted gravity.
///
/// Two post-hit gates apply to the FINAL `InputState`:
/// - `recoil_lock_timer` (the brief recoil throw): a HARD lock — every verb,
///   including the movement/flight steering axis, is zeroed so the knockback
///   ejects the player and they can't act.
/// - `hitstun_timer` (the longer, softer window once recoil clears): movement
///   authority is reduced and jump/dash/blink are suppressed, but the ATTACK
///   verb is preserved so the player can swing back the instant recoil ends —
///   even while still inside a boss and flashing (Hollow-Knight feel).
pub fn engine_input_from_actor_control(
    actor: ActorControlFrame,
    feel: SandboxFeelTuning,
    hitstun_timer: f32,
    recoil_lock_timer: f32,
    control_dt: f32,
) -> ae::InputState {
    let mut input = ae::InputState {
        axis_x: actor.locomotion.x,
        axis_y: actor.locomotion.y,
        jump_pressed: actor.jump_pressed,
        jump_held: actor.jump_held,
        jump_released: actor.jump_released,
        dash_pressed: actor.dash_pressed,
        fly_toggle_pressed: actor.fly_toggle_pressed,
        blink_pressed: actor.blink_pressed,
        blink_held: actor.blink_held,
        blink_released: actor.blink_released,
        blink_quick_dir: actor.blink_quick_dir,
        blink_aim_step: actor.blink_aim_step,
        fast_fall_pressed: actor.fast_fall_pressed,
        attack_pressed: actor.melee_pressed,
        pogo_pressed: actor.pogo_pressed,
        interact_pressed: actor.interact_pressed,
        reset_pressed: false,
        shield_held: actor.shield_held,
        control_dt,
    };
    if recoil_lock_timer > 0.0 {
        // Recoil throw: NO authority. Zero everything (including the movement /
        // flight steering axis) so the knockback carries the player out and they
        // can't steer back in or act until it clears.
        input.axis_x = 0.0;
        input.axis_y = 0.0;
        input.jump_pressed = false;
        input.jump_held = false;
        input.jump_released = false;
        input.dash_pressed = false;
        input.fast_fall_pressed = false;
        input.blink_pressed = false;
        input.blink_held = false;
        input.blink_released = false;
        input.attack_pressed = false;
        input.pogo_pressed = false;
        input.fly_toggle_pressed = false;
        input.interact_pressed = false;
    } else if hitstun_timer > 0.0 {
        // Post-recoil hitstun: reduced movement authority and no
        // jump/dash/blink/fly, but the attack verb (and its pogo sibling) is
        // PRESERVED — you can fight back, and damage a boss you're standing in,
        // the instant the recoil lock ends while i-frames are still ticking.
        let scale = feel.hitstun_control_scale.clamp(0.0, 1.0);
        input.axis_x *= scale;
        input.axis_y *= scale;
        input.jump_pressed = false;
        input.dash_pressed = false;
        input.fast_fall_pressed = false;
        input.blink_pressed = false;
        input.blink_held = false;
        input.blink_released = false;
        input.fly_toggle_pressed = false;
        input.interact_pressed = false;
    }
    input
}

fn pogo_target_for_attack_hitbox(world: &ae::World, attack: ae::Aabb) -> Option<ae::Aabb> {
    world
        .blocks
        .iter()
        .find(|block| block.kind.is_pogo_target() && attack.strict_intersects(block.aabb))
        .map(|block| block.aabb)
}

#[allow(clippy::too_many_arguments)]
pub fn start_attack(
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::BodyClustersMut<'_>,
    attack: &mut Option<PlayerAttackState>,
    anim: &mut PlayerAnimState,
    actor: ActorControlFrame,
    // When the player is holding a melee weapon (axe etc.), its `ActionSet`
    // melee spec re-tunes the swing (timing / reach / damage) so the held item
    // *replaces* the default attack instead of merely gating it.
    held_melee: Option<MeleeActionSpec>,
    // The live gravity, so the attack is classified + placed in the player's
    // reference frame (a toward-feet down-attack is `AirDown`/pogoable, and its
    // hitbox lands toward the feet, under ANY gravity).
    gravity_dir: ae::Vec2,
) {
    if !clusters.abilities.abilities.attack || attack.is_some() {
        return;
    }
    // Combat helpers consume a small `AttackView` snapshot — same
    // fields they used to read off `&Player`, but materialized
    // directly from cluster components without going through
    // `to_player`. Read-only; cluster fields stay the source of
    // truth for any state changes below.
    let view = AttackView {
        pos: clusters.kinematics.pos,
        size: clusters.kinematics.size,
        facing: clusters.kinematics.facing,
        on_ground: clusters.ground.on_ground,
        wall_clinging: clusters.wall.wall_clinging,
        dash_timer: clusters.dash.timer,
        abilities_directional_primary: clusters.abilities.abilities.directional_primary,
    };
    let frame = ae::AccelerationFrame::new(gravity_dir);
    // Classify the swing in the controlled body's local frame. The player brain
    // resolves raw input into `attack_axis`; non-directional brains can leave it
    // zero and the combat resolver falls back to facing.
    let attack_axis = actor.attack_axis;
    let intent =
        resolve_attack_intent_from_view(&view, attack_axis.x, attack_axis.y, actor.pogo_pressed);
    let mut spec = attack_spec_from_view(&view, intent);
    // A held melee weapon re-tunes the swing to its own feel (axe = slow,
    // long-reach, heavier). Pogo (AirDown) keeps its spike timing.
    if let Some(melee) = held_melee {
        if !matches!(intent, AttackIntent::AirDown) {
            spec = spec.with_held_melee(melee);
        }
    }
    // LOCAL BODY → WORLD: the spec's hitbox, impulses, and knockback are authored
    // in the controlled body's local frame; rotate them through the single combat
    // conversion seam so a down-attack's box lands toward the feet under any
    // gravity. Identity under normal gravity.
    spec = spec.into_world_frame(frame);

    // Directional attacks get small self-motion so the hitbox feels connected
    // to the controller. Keep these impulses modest; the engine control path
    // still owns the canonical slash/pogo op + recoil bookkeeping.
    clusters.kinematics.vel += spec.self_impulse;
    // Vertical commit, expressed in the controlled body's local frame: up-attacks guarantee a
    // minimum ASCEND (away from feet); the air down-spike a minimum DESCEND
    // (toward feet). Identity under normal gravity (descend == +vel.y).
    let descend = frame.descend_speed(clusters.kinematics.vel);
    if matches!(intent, AttackIntent::AirUp | AttackIntent::Up) && descend > -40.0 {
        clusters.kinematics.vel += frame.down * (-40.0 - descend);
    }
    // Force the toward-feet commit ONLY for the aerial down spike. The grounded
    // `Down` is a kneeling forward poke rooted to the floor, so committing it would
    // punch through one-way platforms. Skip when the body was already pogo-bounced
    // this frame (the bounce is real even when a 1hp orb shatters instantly, so
    // startup must not overwrite the away-from-feet velocity).
    if !actor.pogo_pressed
        && intent == AttackIntent::AirDown
        && descend >= 0.0
        && descend < 80.0
    {
        frame.ensure_descend_speed(&mut clusters.kinematics.vel, 80.0);
    }

    let player_pos = clusters.kinematics.pos;
    sfx.write(SfxMessage::Slash { pos: player_pos });
    anim.slash_anim_timer = spec.total_seconds().max(0.20);
    *attack = Some(PlayerAttackState::new(spec));
    // Slash effect, oriented + placed in the PLAYER'S reference frame. `spec`
    // is already `into_world_frame`d, so `spec.hitbox_offset` is the
    // gravity-rotated player→strike vector — feed THAT to the effect (NOT the
    // manifest `player_attack_hitbox`, which is screen-axis and so points the
    // wrong way under rotated C4 gravity). The renderer rotates the art along
    // `dir`; only the art KIND comes from the intent (down-tilt pokes,
    // everything else arcs). A shared starting point; each attack can graduate
    // to a bespoke effect later.
    let slash_dir = spec.hitbox_offset;
    emit_melee_slash(
        vfx,
        view.pos + slash_dir,
        spec.hitbox_half_size,
        slash_kind(spec.intent),
        slash_dir,
    );
}

/// Pick the slash ART for an attack: down-tilt is a grounded horizontal poke;
/// every other swing (forward, up, and the down-AIR sweep) is the arc.
/// Direction is handled separately from the hitbox, so these point correctly
/// under any gravity.
fn slash_kind(intent: AttackIntent) -> SlashKind {
    match intent {
        AttackIntent::Down => SlashKind::Poke,
        _ => SlashKind::Arc,
    }
}

/// On-screen size for the slash effect: a flourish a bit larger than the
/// hitbox so the swing reads beyond the exact damage box. Takes the world
/// hitbox half-extent. Tunable.
fn slash_effect_size(hitbox_half_size: ae::Vec2) -> f32 {
    const SLASH_EFFECT_SCALE: f32 = 2.0;
    ((hitbox_half_size * 2.0).max_element() * SLASH_EFFECT_SCALE).max(24.0)
}

/// THE single melee-slash effect emit. EVERY body's melee — the player AND any
/// brain-driven actor — draws its swing through this one function, so the slash
/// visual has exactly ONE definition (size curve + message shape). `center` is the
/// world hitbox center, `half_size` its half-extent, `dir` the gravity-relative
/// body→strike offset (the renderer rotates the art along it).
///
/// ONE BODY, ONE PATH: do NOT add another `VfxMessage::Slash` site — call this. (The
/// two melee STATE MACHINES that call it — `PlayerAttackState` here and
/// `ActorAttackState` in `update_ecs_actors` — are the next fork to collapse; see
/// the `BIFURCATION:` note in dev/journals/code_smells.md.)
pub fn emit_melee_slash(
    vfx: &mut MessageWriter<VfxMessage>,
    center: ae::Vec2,
    half_size: ae::Vec2,
    kind: SlashKind,
    dir: ae::Vec2,
) {
    vfx.write(VfxMessage::Slash {
        center,
        size: slash_effect_size(half_size),
        kind,
        dir,
    });
}

/// Source the player's melee hitbox from the sprite manifest — the box authored
/// and shown by `debug-hitboxes` — so the gameplay damage volume matches the
/// visible blade, the same data-driven path bosses use
/// (`character_sprites::player_attack_hitbox_world`). Returns `None` when the
/// current swing's animation has no authored hitbox, so callers fall back to the
/// hardcoded `AttackSpec` volume.
pub fn player_attack_hitbox(view: &AttackView, intent: AttackIntent) -> Option<ae::CombatVolume> {
    let animation = attack_intent_animation(intent);
    crate::character_sprites::player_attack_hitbox_world(
        animation,
        view.pos,
        view.size,
        view.facing,
    )
}

/// Map the attack intent to its sprite animation row (mirrors the renderer's
/// `directional_attack_anim`). Only rows with an authored manifest hitbox
/// resolve to a box; the rest fall back to the spec volume. Today only
/// `attack_side` is authored — the others are placeholders for when their
/// per-row hitboxes land.
fn attack_intent_animation(intent: AttackIntent) -> &'static str {
    match intent {
        AttackIntent::Up => "attack_up",
        AttackIntent::Down => "attack_down",
        AttackIntent::AirUp => "air_up",
        AttackIntent::AirDown => "air_down",
        AttackIntent::AirForward => "air_forward",
        AttackIntent::AirBack => "air_back",
        _ => "attack_side",
    }
}

#[allow(clippy::too_many_arguments)]
pub fn advance_attack(
    player_entity: Entity,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    world: &ae::World,
    moving_platforms: &[MovingPlatformState],
    clusters: &mut ae::BodyClustersMut<'_>,
    attack: &mut Option<PlayerAttackState>,
    anim: &mut PlayerAnimState,
    combat: &mut BodyCombat,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    frame_dt: f32,
    feature_ecs_overlay: &FeatureEcsWorldOverlay,
    hit_events: &mut MessageWriter<features::HitEvent>,
) {
    // `vfx` is unused today — the slash effect is spawned once in `start_attack`;
    // the active phase only drives the damage hitbox. Kept for the callsite shape
    // and a future per-frame attack VFX.
    let _ = vfx;
    let Some(mut attack_state) = attack.take() else {
        return;
    };

    attack_state.elapsed += frame_dt.max(0.0);
    let Some(phase) = attack_state.phase() else {
        anim.slash_anim_timer = 0.0;
        return;
    };

    if phase == AttackPhase::Active {
        let view = AttackView {
            pos: clusters.kinematics.pos,
            size: clusters.kinematics.size,
            facing: clusters.kinematics.facing,
            on_ground: clusters.ground.on_ground,
            wall_clinging: clusters.wall.wall_clinging,
            dash_timer: clusters.dash.timer,
            abilities_directional_primary: clusters.abilities.abilities.directional_primary,
        };
        let attack = player_attack_hitbox(&view, attack_state.spec.intent)
            .unwrap_or_else(|| attack_hitbox_from_view(&view, attack_state.spec).into());
        let first_active_frame = !attack_state.active_started;
        if first_active_frame {
            attack_state.active_started = true;
            // The slash effect is spawned once at swing start (start_attack);
            // the active phase only drives the damage hitbox below.
        }

        let player_pos = clusters.kinematics.pos;
        let mut pogo_landed = false;
        if clusters.abilities.abilities.pogo
            && attack_state.spec.can_pogo
            && !attack_state.pogo_applied
        {
            let attack_world =
                features::world_with_sandbox_solids(world, moving_platforms, feature_ecs_overlay);
            if let Some(orb_aabb) = pogo_target_for_attack_hitbox(&attack_world, attack.bounds()) {
                ae::movement::set_jump_velocity(
                    &mut clusters.kinematics.vel,
                    tuning.gravity_dir,
                    tuning.pogo_speed,
                );
                ae::refresh_movement_resources_clusters(
                    clusters.abilities,
                    &mut *clusters.dash,
                    &mut *clusters.jump,
                    tuning,
                );
                clusters.ground.on_ground = false;
                attack_state.pogo_applied = true;
                pogo_landed = true;
                sfx.write(SfxMessage::Pogo { pos: player_pos });
                hit_events.write(features::HitEvent {
                    volume: orb_aabb.into(),
                    damage: 1,
                    source: features::HitSource::PogoBounce,
                    // Player melee-driven pogo: this player's
                    // downward strike landed on the orb. The hit
                    // belongs to them; stamp for multi-player
                    // attribution.
                    attacker: Some(player_entity),
                    target: features::HitTarget::OrbMatch,
                    mode: features::HitMode::Knockback,
                    knockback: None,
                    ignored_targets: Vec::new(),
                });
            }
        }
        let slash_damage = attack_state
            .spec
            .damage_override
            .unwrap_or_else(|| clusters.offense.damage_multiplier.max(1))
            .max(1);
        let knock_x = if attack_state.spec.knockback.x.abs() > 0.0 {
            attack_state.spec.knockback.x
        } else {
            clusters.kinematics.facing * 300.0
        };
        // Emit the slash hit on EVERY active frame, not just the first — the
        // hitbox tracks the player as it moves, so a strike at the edge of reach
        // (or mid-descent on a pogo) connects on whatever active frame the box
        // actually reaches the target, matching the every-frame pogo-bounce
        // check. `ignored_targets` (the per-swing `hit_targets`, accumulated by
        // `apply_feature_hit_events` as each target is struck) keeps it to one
        // hit per target across the active window.
        let _ = first_active_frame;
        hit_events.write(features::HitEvent {
            volume: attack,
            damage: slash_damage,
            source: features::HitSource::PlayerSlash { knock_x },
            // Slash hits attribute to the player whose attack
            // landed — the feature-side consumer reads this
            // to apply hitstop / flash to the right player
            // rather than always landing it on primary.
            attacker: Some(player_entity),
            target: features::HitTarget::Volume,
            mode: features::HitMode::Knockback,
            knockback: None,
            ignored_targets: attack_state.hit_targets.clone(),
        });
        // Damage is resolved by the ECS damage queue after the player tick.
        // Keep this phase responsible only for spawning the one-frame hitbox
        // and for immediate pogo/world-contact feedback.
        let landed = false;
        let killed = false;

        if landed || pogo_landed {
            if landed {
                sfx.write(SfxMessage::Hit { pos: player_pos });
            }
            combat.hitstop_timer = feel.attack_hitstop_time;
            combat.hit_flash = 0.16;
        }
        if killed {
            sfx.write(SfxMessage::Death { pos: player_pos });
        }
        if landed
            && clusters.abilities.abilities.pogo
            && attack_state.spec.can_pogo
            && !attack_state.pogo_applied
        {
            ae::movement::set_jump_velocity(
                &mut clusters.kinematics.vel,
                tuning.gravity_dir,
                tuning.pogo_speed,
            );
            ae::refresh_movement_resources_clusters(
                clusters.abilities,
                &mut *clusters.dash,
                &mut *clusters.jump,
                tuning,
            );
            attack_state.pogo_applied = true;
            sfx.write(SfxMessage::Pogo { pos: player_pos });
        }
    }

    if attack_state.done() {
        anim.slash_anim_timer = 0.0;
    } else {
        *attack = Some(attack_state);
    }
}

/// Drive the player's slash / pogo attack lifecycle: start a new
/// swing on rising-edge input (gated by hit-stun), then advance any
/// in-flight attack — applying hits, debris, and recoil through the
/// damage / pogo / sfx / vfx message channels.
///
/// Runs after transition detection so ordering remains detect → attack → apply.
#[allow(clippy::too_many_arguments)]
pub fn attack_advance_system(
    time: Res<Time>,
    world: Res<RoomGeometry>,
    moving_platforms: Res<MovingPlatformSet>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    feature_ecs_overlay: Res<FeatureEcsWorldOverlay>,
    gravity_field: Option<Res<physics::GravityField>>,
    mut player_q: Query<
        (
            Entity,
            ae::BodyClusterQueryData,
            &mut PlayerAnimState,
            &mut BodyCombat,
            &mut ActivePlayerAttack,
            &ActorControl,
            Option<&features::HeldItem>,
        ),
        PrimaryPlayerOnly,
    >,
    mut brain_actions: MessageReader<ActorActionMessage>,
    mut hit_events: MessageWriter<features::HitEvent>,
    mut sfx_writer: MessageWriter<SfxMessage>,
    mut vfx_writer: MessageWriter<VfxMessage>,
) {
    let Ok((
        player_entity,
        mut cluster_item,
        mut anim,
        mut combat,
        mut attack,
        actor_control,
        held_item,
    )) = player_q.single_mut()
    else {
        return;
    };
    // Only an actually-held weapon (axe etc.) re-tunes the swing; the default
    // ActionSet melee keeps the directional attack_spec_from_view feel.
    let held_melee = held_item.and_then(|item| item.spec.melee);
    // The brain-driver system populated this `ActorControl` for the
    // current player upstream (PlayerInput set). Every combat verb
    // start_attack needs (pogo, axes for attack-intent resolution)
    // lives on the brain-driven frame — the raw `PlayerInputFrame`
    // is no longer read in this system.
    let actor_frame = actor_control.0;
    let mut tuning = editable_tuning.as_engine();
    // Sync gravity into the tuning so the pogo bounce (and any other
    // gravity-relative impulse this system applies) launches OPPOSITE the live
    // gravity, not a hardcoded world-up. Without this the attack-path pogo used
    // the default `(0,1)` down and bounced the wrong way under inverted gravity.
    let gdir = physics::gravity_dir_or_default(gravity_field.as_deref());
    physics::apply_gravity_dir(&mut tuning, gdir);
    let feel = *feel_tuning;
    let frame_dt = time.delta_secs();

    let mut clusters = cluster_item.as_clusters_mut();
    // Melee comes through the ActionSet-resolved brain message; pogo is a
    // player-specific intent mirrored onto `ActorControlFrame`.
    let melee_requested = brain_actions
        .read()
        .any(|msg| msg.actor == player_entity && msg.is_melee());
    // Attack is gated on the brief recoil lock, NOT the full hitstun window:
    // once the player has been thrown clear (~0.12s) they can swing again even
    // while still in hitstun / i-frames, so face-tanking a boss lets you fight
    // back instead of standing there helpless (Hollow-Knight feel).
    if combat.recoil_lock_timer <= 0.0 && (melee_requested || actor_frame.pogo_pressed) {
        start_attack(
            &mut sfx_writer,
            &mut vfx_writer,
            &mut clusters,
            &mut attack.0,
            &mut anim,
            actor_frame,
            held_melee,
            tuning.gravity_dir,
        );
    }
    advance_attack(
        player_entity,
        &mut sfx_writer,
        &mut vfx_writer,
        &world.0,
        &moving_platforms.0,
        &mut clusters,
        &mut attack.0,
        &mut anim,
        &mut combat,
        tuning,
        feel,
        frame_dt,
        &feature_ecs_overlay,
        &mut hit_events,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_attack_box() -> ae::Aabb {
        ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(16.0, 16.0))
    }

    #[test]
    fn attack_phase_pogo_rejects_ground_and_one_way_targets() {
        let attack = test_attack_box();
        let min = attack.center() - attack.half_size();
        let size = attack.half_size() * 2.0;
        let world = ae::World::new(
            "pogo attack reject test",
            ae::Vec2::new(400.0, 300.0),
            ae::Vec2::ZERO,
            vec![
                ae::Block::solid("floor", min, size),
                ae::Block::one_way("one-way", min, size),
                ae::Block::blink_wall("blink-wall", min, size, ae::BlinkWallTier::Soft),
            ],
        );

        assert_eq!(pogo_target_for_attack_hitbox(&world, attack), None);
    }

    #[test]
    fn attack_phase_pogo_accepts_authored_pogo_targets() {
        let attack = test_attack_box();
        let min = attack.center() - attack.half_size();
        let size = attack.half_size() * 2.0;
        let orb = ae::Block::pogo_orb("orb", attack.center(), 12.0);
        let rebound = ae::Block::rebound(
            "rebound",
            min + ae::Vec2::new(60.0, 0.0),
            size,
            ae::Vec2::new(0.0, 180.0),
        );
        let world = ae::World::new(
            "pogo attack accept test",
            ae::Vec2::new(400.0, 300.0),
            ae::Vec2::ZERO,
            vec![ae::Block::solid("floor", min, size), orb.clone(), rebound],
        );

        assert_eq!(
            pogo_target_for_attack_hitbox(&world, attack),
            Some(orb.aabb)
        );
    }

    /// Pins the geometry behind the "pogo bounces but deals no damage at the
    /// edge" bug (now FIXED): the slash hitbox tracks the player, so frame 1
    /// (player still high / at the edge) misses while a later active frame
    /// reaches the target. The bug was that `advance_attack` emitted the
    /// slash-damage `HitEvent` only on the FIRST active frame but re-checked the
    /// POGO bounce EVERY active frame — so the later frame bounced with no hit.
    /// Fixed by emitting the slash damage every active frame (deduped per target
    /// via `hit_targets`, accumulated in `apply_feature_hit_events`), mirroring
    /// the pogo check. This test keeps the geometry honest: the later-frame
    /// hitbox DOES overlap, so the every-frame emit will land the hit.
    #[test]
    fn pogo_connects_on_a_later_frame_than_the_first_active_frame_damage_check() {
        let hitbox_at = |pos: ae::Vec2| {
            let view = AttackView {
                pos,
                size: ae::Vec2::new(30.0, 48.0),
                facing: 1.0,
                on_ground: false,
                wall_clinging: false,
                dash_timer: 0.0,
                abilities_directional_primary: true,
            };
            attack_hitbox_from_view(&view, attack_spec_from_view(&view, AttackIntent::AirDown))
        };
        // The boss's pogo target — same geometry as its damageable volume
        // (pogo is `FromDamageable`).
        let orb = ae::Block::pogo_orb("boss", ae::Vec2::new(100.0, 200.0), 16.0);
        let world = ae::World::new(
            "pogo-timing repro",
            ae::Vec2::new(400.0, 400.0),
            ae::Vec2::ZERO,
            vec![orb.clone()],
        );

        // First active frame: player still high → the down hitbox misses.
        let first = hitbox_at(ae::Vec2::new(100.0, 80.0));
        // A later active frame: player descended into the boss → hitbox overlaps.
        let later = hitbox_at(ae::Vec2::new(100.0, 120.0));

        // Damage is first-active-frame only → it samples `first`, which misses.
        assert!(
            !first.strict_intersects(orb.aabb),
            "first-frame hitbox misses the boss, so the one-shot slash damage never lands",
        );
        assert_eq!(
            pogo_target_for_attack_hitbox(&world, first),
            None,
            "pogo also misses on the first frame",
        );
        // Pogo is checked every active frame → it connects on `later` and bounces.
        assert_eq!(
            pogo_target_for_attack_hitbox(&world, later),
            Some(orb.aabb),
            "pogo connects on a later frame → bounce with no damage (the bug)",
        );
        // The later-frame hitbox DOES overlap the boss — the only reason damage
        // didn't land is the first-active-frame-only gate. Checking damage every
        // active frame (like pogo) would fix it.
        assert!(
            later.strict_intersects(orb.aabb),
            "later-frame hitbox overlaps the boss; only the first-frame damage gate hid the hit",
        );
    }
}
