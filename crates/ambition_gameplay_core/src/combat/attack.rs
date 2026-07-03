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
//! because the component vocabulary (`MeleeSwing`, `ae::BodyClustersMut`,
//! `BodyMelee`) does. The relativity-principle fix is the actor-
//! unification rename of those types, tracked separately.

use bevy::prelude::{Entity, MessageReader, MessageWriter, Query, Res};

use ambition_characters::actor::control::ActorControlFrame;
use ambition_characters::brain::{ActorActionMessage, ActorControl, MeleeActionSpec};
use ambition_engine_core::{self as ae, AabbExt};
use ambition_vfx::vfx::{SlashKind, VfxMessage};

use ambition_characters::actor::BodyCombat;
use ambition_sfx::SfxMessage;
use crate::combat::{
    attack_hitbox_from_view, attack_spec_from_view, resolve_attack_intent_from_view, AttackIntent,
    AttackPhase, AttackView,
};
use crate::dev::dev_tools::EditableMovementTuning;
use crate::features::{self, FeatureEcsWorldOverlay};
use crate::player::{BodyMelee, PlayerAnimState};
use crate::time::feel::SandboxFeelTuning;
use crate::world::platforms::MovingPlatformState;
use crate::{physics, MeleeSwing, MovingPlatformSet, RoomGeometry};

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
    apply_post_hit_input_gates(&mut input, feel, hitstun_timer, recoil_lock_timer);
    input
}

/// The two post-hit gates applied to ANY body's FINAL [`ae::InputState`]
/// (fable review §A2 step 7): the ONE stagger rule, so a knocked actor loses
/// authority exactly the way the knocked player does — the player's input
/// bridge and the actor's `integrate_body` both call this.
pub fn apply_post_hit_input_gates(
    input: &mut ae::InputState,
    feel: SandboxFeelTuning,
    hitstun_timer: f32,
    recoil_lock_timer: f32,
) {
    // The FLY TOGGLE is exempt from both gates: it is a mode-switch INTENT, not
    // movement authority (the axes are still stripped, so a toggled flyer can't
    // steer until the stagger clears). Eating an edge-triggered toggle corrupts
    // an open-loop brain's mode state (it believes it toggled), and toggling
    // flight to arrest a launch is a legitimate recovery tech for every body.
    if recoil_lock_timer > 0.0 {
        // Recoil throw: NO authority. Zero everything (including the movement /
        // flight steering axis) so the knockback carries the body out and it
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
        input.interact_pressed = false;
    } else if hitstun_timer > 0.0 {
        // Post-recoil hitstun: reduced movement authority and no
        // jump/dash/blink, but the attack verb (and its pogo sibling) is
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
        input.interact_pressed = false;
    }
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
    attack: &mut Option<MeleeSwing>,
    // Presentation-only slash-anim timer. `None` for a body with no
    // `PlayerAnimState` (an actor) — the swing lifecycle is gameplay, the anim is
    // presentation and rides only where authored.
    anim: Option<&mut PlayerAnimState>,
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
    if !actor.pogo_pressed && intent == AttackIntent::AirDown && descend >= 0.0 && descend < 80.0 {
        frame.ensure_descend_speed(&mut clusters.kinematics.vel, 80.0);
    }

    let player_pos = clusters.kinematics.pos;
    sfx.write(SfxMessage::Slash { pos: player_pos });
    if let Some(anim) = anim {
        anim.slash_anim_timer = spec.total_seconds().max(0.20);
    }
    *attack = Some(MeleeSwing::new(spec));
    // The slash VFX is NO LONGER emitted here. It is spawned at the active edge by
    // the ONE shared `spawn_melee_strike` (in `advance_attack`), from the SAME
    // gravity-resolved box as the damage hitbox — so the slash and the hitbox can
    // never point in different directions under rotated gravity. `vfx` is unused
    // by `start_attack` now; the swing-start whoosh stays on the `sfx` channel.
    let _ = vfx;
}

/// Pick the slash ART for an attack: down-tilt is a grounded horizontal poke;
/// every other swing (forward, up, and the down-AIR sweep) is the arc.
/// Direction is handled separately from the hitbox, so these point correctly
/// under any gravity.
pub fn slash_kind(intent: AttackIntent) -> SlashKind {
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
/// two melee STATE MACHINES that call it — `MeleeSwing` here and
/// `BodyMelee` in `update_ecs_actors` — are the next fork to collapse; see
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
pub fn player_attack_hitbox(
    view: &AttackView,
    intent: AttackIntent,
    gravity_dir: ae::Vec2,
) -> Option<ae::CombatVolume> {
    let animation = attack_intent_animation(intent);
    crate::character_sprites::player_attack_hitbox_world(
        animation,
        view.pos,
        view.size,
        view.facing,
        gravity_dir,
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
#[allow(clippy::too_many_arguments)]
pub fn advance_attack(
    commands: &mut bevy::prelude::Commands,
    // The body that owns the swing — the source of truth for the strike's owner
    // and effective allegiance. NOT rediscovered as "the player" in here.
    actor: Entity,
    // The strike's EFFECTIVE faction (`effective_faction(authored, brain)`): a
    // player / possessed body strikes as `Player` (FollowOwner Volume, signed
    // `knock_x`, hits its foes); an autonomous hostile strikes as `Enemy`/`Boss`
    // (position-derived `knockback_strength`, hits the player). This is the ONLY
    // branch — a faction distinction the damage resolver already makes, not a
    // player-shaped one.
    faction: features::ActorFaction,
    // The body's sprite catalog id, if any, so its authored per-animation melee
    // box drives the strike (the same data-driven lookup for player + actor).
    // `None` → the player's hardcoded manifest root.
    sprite_cid: Option<&str>,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    world: &ae::World,
    moving_platforms: &[MovingPlatformState],
    clusters: &mut ae::BodyClustersMut<'_>,
    attack: &mut Option<MeleeSwing>,
    anim: Option<&mut PlayerAnimState>,
    tuning: ae::MovementTuning,
    frame_dt: f32,
    feature_ecs_overlay: &FeatureEcsWorldOverlay,
    hit_events: &mut MessageWriter<features::HitEvent>,
) {
    let Some(mut attack_state) = attack.take() else {
        return;
    };

    attack_state.elapsed += frame_dt.max(0.0);
    let Some(phase) = attack_state.phase() else {
        if let Some(anim) = anim {
            anim.slash_anim_timer = 0.0;
        }
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
        // THE gravity-resolved strike box. The authored sprite-manifest box is
        // gravity-aware (it rotates the screen-axis hull into the body's frame), so
        // it is correct under ANY gravity. The manifest is resolved by the body's
        // sprite catalog id when it has one (an actor), else the player's hardcoded
        // manifest root — the same `manifest_attack_hitbox_world` lookup either way.
        // Falls back to the hardcoded `AttackSpec` box when the animation authors
        // none. ONE box drives BOTH the damage hitbox AND the slash VFX (via
        // `spawn_melee_strike`), so they can never diverge.
        let spec_box = attack_hitbox_from_view(&view, attack_state.spec);
        let animation = attack_intent_animation(attack_state.spec.intent);
        let manifest = match sprite_cid {
            Some(cid) => crate::character_sprites::actor_attack_hitbox_world(
                cid,
                animation,
                view.pos,
                view.size,
                view.facing,
                tuning.gravity_dir,
            ),
            None => crate::character_sprites::player_attack_hitbox_world(
                animation,
                view.pos,
                view.size,
                view.facing,
                tuning.gravity_dir,
            ),
        };
        let world_box = manifest.map(|v| v.bounds()).unwrap_or(spec_box);

        let first_active_frame = !attack_state.active_started;
        if first_active_frame {
            attack_state.active_started = true;
            // ONE strike spawn for EVERY body through `spawn_melee_strike` (damage
            // hitbox + slash VFX from one box). The strike's damage rides the swing
            // spec (or the body's offense multiplier); its knockback CHANNEL is
            // chosen by EFFECTIVE FACTION — the only branch, and one the damage
            // resolver already makes:
            //   * Player/possessed  → FollowOwner Volume with the signed slash
            //     `knock_x` (hits the body's foes), no aggressor push.
            //   * Enemy/Boss        → position-derived `knockback_strength` aggressor
            //     push (hits the player), no slash `knock_x`.
            let slash_damage = attack_state
                .spec
                .damage_override
                .unwrap_or_else(|| clusters.offense.damage_multiplier.max(1))
                .max(1);
            let (knock_x, knockback_strength) = if matches!(faction, features::ActorFaction::Player)
            {
                let kx = if attack_state.spec.knockback.x.abs() > 0.0 {
                    attack_state.spec.knockback.x
                } else {
                    clusters.kinematics.facing * 300.0
                };
                (kx, 0.0)
            } else {
                (0.0, 1.0)
            };
            super::hitbox::spawn_melee_strike(
                commands,
                vfx,
                actor,
                faction,
                clusters.kinematics.pos,
                world_box,
                slash_damage,
                knockback_strength,
                knock_x,
                attack_state.spec.active_seconds,
                slash_kind(attack_state.spec.intent),
                tuning.gravity_dir,
            );
        }

        let player_pos = clusters.kinematics.pos;
        // Pogo stays in the player path (player-only physics): an active down-spike
        // that reaches an authored pogo target bounces the player + damages the orb.
        // Checked every active frame (like the strike's per-tick hit resolution).
        if clusters.abilities.abilities.pogo
            && attack_state.spec.can_pogo
            && !attack_state.pogo_applied
        {
            let attack_world =
                features::world_with_sandbox_solids(world, moving_platforms, feature_ecs_overlay);
            if let Some(orb_aabb) = pogo_target_for_attack_hitbox(&attack_world, world_box) {
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
                sfx.write(SfxMessage::Pogo { pos: player_pos });
                hit_events.write(features::HitEvent {
                    volume: orb_aabb.into(),
                    damage: 1,
                    source: features::HitSource::PogoBounce,
                    attacker: Some(actor),
                    target: features::HitTarget::OrbMatch,
                    mode: features::HitMode::Knockback,
                    knockback: None,
                    ignored_targets: Vec::new(),
                });
            }
        }
    }

    if attack_state.done() {
        if let Some(anim) = anim {
            anim.slash_anim_timer = 0.0;
        }
    } else {
        *attack = Some(attack_state);
    }
}

/// **Phase — START body melee.** Turn every `ActorActionMessage::Melee` into a
/// swing on the body that requested it. ONE lifecycle for EVERY body — the human
/// player, a possessed actor, an autonomous hostile — keyed by `msg.actor`, with
/// no `PlayerEntity` filter, no `ControlledSubject` lookup, and no
/// per-controller-kind driver. The upstream `emit_brain_action_messages` resolver
/// already gated on the body's `ActionSet.melee` (the capability seam); here the
/// BODY enforces its own physical gates: not already swinging, off the recovery
/// floor, past the brief post-hit recoil lock, and the `abilities.attack`
/// capability (inside `start_attack`).
///
/// Replaces BOTH the player-only `attack_advance_system` start and the actor-only
/// `start_enemy_melee_from_brain_actions` — one body-action START phase.
pub fn start_body_melee(
    gravity: physics::GravityCtx,
    mut brain_actions: MessageReader<ActorActionMessage>,
    mut sfx_writer: MessageWriter<SfxMessage>,
    mut vfx_writer: MessageWriter<VfxMessage>,
    mut bodies: Query<(
        ae::BodyClusterQueryData,
        &mut BodyMelee,
        &BodyCombat,
        &ActorControl,
        Option<&features::HeldItem>,
        Option<&features::ActorConfig>,
        Option<&mut PlayerAnimState>,
    )>,
) {
    // The resolver can emit the same body once per frame; collect the requesters.
    let melee_actors: std::collections::HashSet<Entity> = brain_actions
        .read()
        .filter(|m| m.is_melee())
        .map(|m| m.actor)
        .collect();
    for actor in melee_actors {
        let Ok((mut cq, mut melee, combat, control, held, config, mut anim)) =
            bodies.get_mut(actor)
        else {
            continue;
        };
        // Body enforces: no double-swing, off the recovery floor, past the recoil
        // lock (the same re-swing gate the player used, now shared by all bodies).
        if melee.swing.is_some() || melee.on_cooldown() || combat.recoil_lock_timer > 0.0 {
            continue;
        }
        // Only an actually-held weapon (axe etc.) re-tunes the swing; the default
        // ActionSet melee keeps the directional `attack_spec_from_view` feel.
        let held_melee = held.and_then(|item| item.spec.melee);
        let frame = control.0;
        // The recovery/AI pacing floor is BODY DATA: an actor's authored cooldown
        // (the same value the deleted `begin_melee_attack` set) paces its brain's
        // next swing; the player carries no `ActorConfig`, so it has none (its
        // re-swing is gated by the swing duration + recoil lock).
        let cooldown = config
            .map(|c| features::ENEMY_ATTACK_COOLDOWN * c.tuning.attack_cooldown_mult)
            .unwrap_or(0.0);
        let gravity_dir = gravity.dir_at(cq.kinematics.pos);
        let mut clusters = cq.as_clusters_mut();
        start_attack(
            &mut sfx_writer,
            &mut vfx_writer,
            &mut clusters,
            &mut melee.swing,
            anim.as_deref_mut(),
            frame,
            held_melee,
            gravity_dir,
        );
        // If a swing actually began (capability + not-already-swinging passed
        // inside `start_attack`), arm the body's recovery floor.
        if melee.swing.is_some() {
            melee.cooldown = cooldown.max(0.0);
        }
    }
}

/// **Phase — ADVANCE body melee.** Tick every body's in-flight swing + its
/// recovery/refire floors, and at the windup→active edge spawn the ONE
/// gravity-resolved strike (damage hitbox + slash VFX) through
/// `spawn_melee_strike`. Runs for EVERY body — the human player, a possessed
/// actor, an autonomous hostile, a peaceful NPC with a kit — so there is exactly
/// one melee ADVANCE lifecycle, not a player driver and an actor driver.
///
/// The strike's OWNER is the body itself and its EFFECTIVE FACTION
/// (`effective_faction`) selects the damage channel the resolver already
/// distinguishes (Player/possessed → FollowOwner slash hitting its foes;
/// Enemy/Boss → aggressor push hitting the player). Melee advances on the SIM
/// clock (`WorldTime::sim_dt`) so it composes with bullet-time / pause for every
/// body.
///
/// Replaces BOTH the player-only `advance_attack` call in `attack_advance_system`
/// and the actor-only active-edge strike spawn inside `update_ecs_actors` (and the
/// `self.attack.tick` that used to ride the actor movement integration).
#[allow(clippy::too_many_arguments)]
pub fn advance_body_melee(
    world_time: Res<ambition_time::WorldTime>,
    world: Res<RoomGeometry>,
    moving_platforms: Res<MovingPlatformSet>,
    editable_tuning: Res<EditableMovementTuning>,
    feature_ecs_overlay: Res<FeatureEcsWorldOverlay>,
    gravity: physics::GravityCtx,
    mut commands: bevy::prelude::Commands,
    mut sfx_writer: MessageWriter<SfxMessage>,
    mut vfx_writer: MessageWriter<VfxMessage>,
    mut hit_events: MessageWriter<features::HitEvent>,
    mut bodies: Query<(
        Entity,
        ae::BodyClusterQueryData,
        &mut BodyMelee,
        &features::ActorFaction,
        Option<&ambition_characters::brain::Brain>,
        Option<&features::ActorConfig>,
        Option<&mut PlayerAnimState>,
    )>,
) {
    let dt = world_time.sim_dt();
    for (entity, mut cq, mut melee, faction, brain, config, mut anim) in &mut bodies {
        // The recovery / refire floors tick every frame regardless of a live swing
        // (`advance_attack` advances the swing's own `elapsed`).
        melee.cooldown = (melee.cooldown - dt).max(0.0);
        melee.ranged_cooldown = (melee.ranged_cooldown - dt).max(0.0);
        if melee.swing.is_none() {
            if let Some(anim) = anim.as_deref_mut() {
                anim.slash_anim_timer = 0.0;
            }
            continue;
        }
        let pos = cq.kinematics.pos;
        let mut tuning = editable_tuning.as_engine();
        // Gravity into the tuning so the pogo bounce launches OPPOSITE live gravity.
        physics::apply_gravity_dir(&mut tuning, gravity.dir_at(pos));
        // The strike's effective allegiance picks the damage channel; the sprite id
        // (actors) resolves the authored per-animation box, else the player root.
        let strike_faction = crate::combat::targeting::effective_faction(*faction, brain);
        let sprite_cid = config.and_then(|c| c.sprite_character_id.as_deref());
        let mut clusters = cq.as_clusters_mut();
        advance_attack(
            &mut commands,
            entity,
            strike_faction,
            sprite_cid,
            &mut sfx_writer,
            &mut vfx_writer,
            &world.0,
            &moving_platforms.0,
            &mut clusters,
            &mut melee.swing,
            anim.as_deref_mut(),
            tuning,
            dt,
            &feature_ecs_overlay,
            &mut hit_events,
        );
    }
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
