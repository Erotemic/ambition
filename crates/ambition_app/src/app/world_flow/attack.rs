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
/// Two post-hit gates apply to the FINAL `InputState`:
/// - `recoil_lock_timer` (the brief recoil throw): a HARD lock — every verb,
///   including the movement/flight steering axis, is zeroed so the knockback
///   ejects the player and they can't act.
/// - `hitstun_timer` (the longer, softer window once recoil clears): movement
///   authority is reduced and jump/dash/blink are suppressed, but the ATTACK
///   verb is preserved so the player can swing back the instant recoil ends —
///   even while still inside a boss and flashing (Hollow-Knight feel).
pub(crate) fn engine_input_from_actor_control(
    actor: ambition_gameplay_core::actor::control::ActorControlFrame,
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
    // Classify the swing in the controlled body's local frame. The player brain
    // resolves raw input into `attack_axis`; non-directional brains can leave it
    // zero and the combat resolver falls back to facing.
    let attack_axis = actor.attack_axis;
    let intent = ambition_gameplay_core::combat::resolve_attack_intent_from_view(
        &view,
        attack_axis.x,
        attack_axis.y,
        actor.pogo_pressed,
    );
    let mut spec = ambition_gameplay_core::combat::attack_spec_from_view(&view, intent);
    // A held melee weapon re-tunes the swing to its own feel (axe = slow,
    // long-reach, heavier). Pogo (AirDown) keeps its spike timing.
    if let Some(melee) = held_melee {
        if !matches!(
            intent,
            ambition_gameplay_core::combat::AttackIntent::AirDown
        ) {
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
    if matches!(
        intent,
        ambition_gameplay_core::combat::AttackIntent::AirUp
            | ambition_gameplay_core::combat::AttackIntent::Up
    ) && descend > -40.0
    {
        clusters.kinematics.vel += frame.down * (-40.0 - descend);
    }
    // Force the toward-feet commit ONLY for the aerial down spike. The grounded
    // `Down` is a kneeling forward poke rooted to the floor, so committing it would
    // punch through one-way platforms. Skip when the body was already pogo-bounced
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
    // Slash effect, oriented + placed in the PLAYER'S reference frame. `spec`
    // is already `into_world_frame`d, so `spec.hitbox_offset` is the
    // gravity-rotated player→strike vector — feed THAT to the effect (NOT the
    // manifest `player_attack_hitbox`, which is screen-axis and so points the
    // wrong way under rotated C4 gravity). The renderer rotates the art along
    // `dir`; only the art KIND comes from the intent (down-tilt pokes,
    // everything else arcs). A shared starting point; each attack can graduate
    // to a bespoke effect later.
    let slash_dir = spec.hitbox_offset;
    vfx.write(VfxMessage::Slash {
        center: view.pos + slash_dir,
        size: slash_effect_size(spec.hitbox_half_size),
        kind: slash_kind(spec.intent),
        dir: slash_dir,
    });
}

/// Pick the slash ART for an attack: down-tilt is a grounded horizontal poke;
/// every other swing (forward, up, and the down-AIR sweep) is the arc.
/// Direction is handled separately from the hitbox, so these point correctly
/// under any gravity.
fn slash_kind(intent: ambition_gameplay_core::combat::AttackIntent) -> SlashKind {
    use ambition_gameplay_core::combat::AttackIntent as I;
    match intent {
        I::Down => SlashKind::Poke,
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

/// Source the player's melee hitbox from the sprite manifest — the box authored
/// and shown by `debug-hitboxes` — so the gameplay damage volume matches the
/// visible blade, the same data-driven path bosses use
/// (`character_sprites::player_attack_hitbox_world`). Returns `None` when the
/// current swing's animation has no authored hitbox, so callers fall back to the
/// hardcoded `AttackSpec` volume.
fn player_attack_hitbox(
    view: &ambition_gameplay_core::combat::AttackView,
    intent: ambition_gameplay_core::combat::AttackIntent,
) -> Option<ae::CombatVolume> {
    let animation = attack_intent_animation(intent);
    ambition_gameplay_core::character_sprites::player_attack_hitbox_world(
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
fn attack_intent_animation(intent: ambition_gameplay_core::combat::AttackIntent) -> &'static str {
    use ambition_gameplay_core::combat::AttackIntent as I;
    match intent {
        I::Up => "attack_up",
        I::Down => "attack_down",
        I::AirUp => "air_up",
        I::AirDown => "air_down",
        I::AirForward => "air_forward",
        I::AirBack => "air_back",
        _ => "attack_side",
    }
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
        let attack = player_attack_hitbox(&view, attack_state.spec.intent).unwrap_or_else(|| {
            ambition_gameplay_core::combat::attack_hitbox_from_view(&view, attack_state.spec).into()
        });
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
