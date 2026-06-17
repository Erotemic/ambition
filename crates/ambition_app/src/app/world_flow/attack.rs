//! Attack-phase flow: actor-control -> engine input translation and the
//! start/advance attack-phase state machine.
//!
//! Split out of the former 1211-line `world_flow.rs` (2026-06-15).

use super::*;

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
/// The hitstun gate is applied to the FINAL `InputState` so every
/// verb is zeroed uniformly.
pub(crate) fn engine_input_from_actor_control(
    actor: ambition_gameplay_core::actor::control::ActorControlFrame,
    feel: SandboxFeelTuning,
    hitstun_timer: f32,
    control_dt: f32,
) -> ae::InputState {
    let mut input = ae::InputState {
        axis_x: actor.desired_vel.x,
        axis_y: actor.desired_vel.y,
        jump_pressed: actor.jump_pressed,
        jump_held: actor.jump_held,
        jump_released: actor.jump_released,
        dash_pressed: actor.dash_pressed,
        fly_toggle_pressed: actor.fly_toggle_pressed,
        blink_pressed: actor.blink_pressed,
        blink_held: actor.blink_held,
        blink_released: actor.blink_released,
        fast_fall_pressed: actor.fast_fall_pressed,
        attack_pressed: actor.melee_pressed,
        pogo_pressed: actor.pogo_pressed,
        interact_pressed: actor.interact_pressed,
        reset_pressed: false,
        shield_held: actor.shield_held,
        control_dt,
    };
    if hitstun_timer > 0.0 {
        let scale = feel.hitstun_control_scale.clamp(0.0, 1.0);
        input.axis_x *= scale;
        input.axis_y *= scale;
        input.jump_pressed = false;
        input.dash_pressed = false;
        input.fast_fall_pressed = false;
        input.blink_pressed = false;
        input.blink_held = false;
        input.blink_released = false;
        input.attack_pressed = false;
        input.pogo_pressed = false;
        input.fly_toggle_pressed = false;
        input.interact_pressed = false;
    }
    input
}

pub(crate) fn start_attack(
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::PlayerClustersMut<'_>,
    attack: &mut Option<ambition_gameplay_core::PlayerAttackState>,
    anim: &mut ambition_gameplay_core::player::PlayerAnimState,
    actor: ambition_gameplay_core::actor::control::ActorControlFrame,
    // When the player is holding a melee weapon (axe etc.), its `ActionSet`
    // melee spec re-tunes the swing (timing / reach / damage) so the held item
    // *replaces* the default attack instead of merely gating it.
    held_melee: Option<ambition_gameplay_core::brain::MeleeActionSpec>,
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
    let view = ambition_gameplay_core::combat::AttackView {
        pos: clusters.kinematics.pos,
        size: clusters.kinematics.size,
        facing: clusters.kinematics.facing,
        on_ground: clusters.ground.on_ground,
        wall_clinging: clusters.wall.wall_clinging,
        dash_timer: clusters.dash.timer,
        abilities_directional_primary: clusters.abilities.abilities.directional_primary,
    };
    let frame = ae::AccelerationFrame::new(gravity_dir);
    // INPUT → PLAYER: classify the swing in the player's frame. `descend` is the
    // toward-feet intent, so a down-attack reads as `AirDown` (pogoable) under any
    // gravity — raw `desired_vel.y` would mis-read it as `AirUp` past ±90°.
    let intent = ambition_gameplay_core::combat::resolve_attack_intent_from_view(
        &view,
        actor.desired_vel.x,
        frame.descend(actor.desired_vel.y),
        actor.pogo_pressed,
    );
    let mut spec = ambition_gameplay_core::combat::attack_spec_from_view(&view, intent);
    // A held melee weapon re-tunes the swing to its own feel (axe = slow,
    // long-reach, heavier). Pogo (AirDown) keeps its spike timing.
    if let Some(melee) = held_melee {
        if !matches!(intent, ambition_gameplay_core::combat::AttackIntent::AirDown) {
            spec = spec.with_held_melee(melee);
        }
    }
    // PLAYER → WORLD: the spec's hitbox + self-impulse are authored for an upright
    // player; rotate them into the live gravity so a down-attack's box lands toward
    // the feet (and overlaps the pogo orb there). Identity under normal gravity.
    spec.hitbox_offset = frame.to_world(spec.hitbox_offset);
    spec.hitbox_half_size = frame.to_world_half(spec.hitbox_half_size);
    spec.self_impulse = frame.to_world(spec.self_impulse);

    // Directional attacks get small self-motion so the hitbox feels connected
    // to the controller. Keep these impulses modest; the engine control path
    // still owns the canonical slash/pogo op + recoil bookkeeping.
    clusters.kinematics.vel += spec.self_impulse;
    // Vertical commit, expressed in the player frame: up-attacks guarantee a
    // minimum ASCEND (away from feet); the air down-spike a minimum DESCEND
    // (toward feet). Identity under normal gravity (descend == +vel.y).
    let descend = frame.descend_speed(clusters.kinematics.vel);
    if matches!(
        intent,
        ambition_gameplay_core::combat::AttackIntent::AirUp | ambition_gameplay_core::combat::AttackIntent::Up
    ) && descend > -40.0
    {
        clusters.kinematics.vel += frame.down * (-40.0 - descend);
    }
    // Force the toward-feet commit ONLY for the aerial down spike. The grounded
    // `Down` is a kneeling forward poke rooted to the floor, so committing it would
    // punch through one-way platforms. Skip when the player was already pogo-bounced
    // this frame (the bounce is real even when a 1hp orb shatters instantly, so
    // startup must not overwrite the away-from-feet velocity).
    if !actor.pogo_pressed
        && intent == ambition_gameplay_core::combat::AttackIntent::AirDown
        && descend >= 0.0
        && descend < 80.0
    {
        frame.ensure_descend_speed(&mut clusters.kinematics.vel, 80.0);
    }

    let player_pos = clusters.kinematics.pos;
    sfx.write(SfxMessage::Slash { pos: player_pos });
    anim.slash_anim_timer = spec.total_seconds().max(0.20);
    *attack = Some(ambition_gameplay_core::PlayerAttackState::new(spec));
    vfx.write(VfxMessage::SlashPreview {
        hitbox: ambition_gameplay_core::combat::attack_hitbox_from_view(&view, spec),
    });
}

pub(crate) fn advance_attack(
    player_entity: bevy::prelude::Entity,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    world: &ae::World,
    moving_platforms: &[ambition_gameplay_core::world::platforms::MovingPlatformState],
    clusters: &mut ae::PlayerClustersMut<'_>,
    attack: &mut Option<ambition_gameplay_core::PlayerAttackState>,
    anim: &mut ambition_gameplay_core::player::PlayerAnimState,
    combat: &mut ambition_gameplay_core::player::PlayerCombatState,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    frame_dt: f32,
    feature_ecs_overlay: &features::FeatureEcsWorldOverlay,
    hit_events: &mut MessageWriter<features::HitEvent>,
) {
    let Some(mut attack_state) = attack.take() else {
        return;
    };

    attack_state.elapsed += frame_dt.max(0.0);
    let Some(phase) = attack_state.phase() else {
        anim.slash_anim_timer = 0.0;
        return;
    };

    if phase == ambition_gameplay_core::combat::AttackPhase::Active {
        let view = ambition_gameplay_core::combat::AttackView {
            pos: clusters.kinematics.pos,
            size: clusters.kinematics.size,
            facing: clusters.kinematics.facing,
            on_ground: clusters.ground.on_ground,
            wall_clinging: clusters.wall.wall_clinging,
            dash_timer: clusters.dash.timer,
            abilities_directional_primary: clusters.abilities.abilities.directional_primary,
        };
        let attack = ambition_gameplay_core::combat::attack_hitbox_from_view(&view, attack_state.spec);
        let first_active_frame = !attack_state.active_started;
        if first_active_frame {
            attack_state.active_started = true;
            vfx.write(VfxMessage::SlashPreview { hitbox: attack });
        }

        let player_pos = clusters.kinematics.pos;
        let mut pogo_landed = false;
        if clusters.abilities.abilities.pogo
            && attack_state.spec.can_pogo
            && !attack_state.pogo_applied
        {
            let attack_world =
                features::world_with_sandbox_solids(world, moving_platforms, feature_ecs_overlay);
            if let Some(orb_aabb) = pogo_target_for_attack_hitbox(&attack_world, attack) {
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
                    volume: orb_aabb,
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
        if first_active_frame {
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
        }
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
            combat.flash_timer = 0.16;
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
