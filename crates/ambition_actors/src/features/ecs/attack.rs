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

use bevy::prelude::{Entity, Has, MessageReader, MessageWriter, Query, Res};

use crate::combat::moveset::MovesetMelee;

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_characters::actor::control::ActorControlFrame;
use ambition_characters::brain::{ActorActionMessage, ActorControl, MeleeActionSpec};
use ambition_engine_core::{self as ae, AabbExt};
use ambition_vfx::vfx::{SlashKind, VfxMessage};

use crate::combat::BodyMelee;
use crate::combat::{
    attack_hitbox_from_view, attack_spec_from_view, resolve_attack_intent_from_view, AttackIntent,
    AttackPhase, AttackView,
};
use crate::world::overlay::FeatureEcsWorldOverlay;
use ambition_dev_tools::dev_tools::EditableMovementTuning;

/// Baseline seconds between enemy contact attacks (scaled per-actor by
/// `attack_cooldown_mult`). Combat-owned pacing tuning (E2).
pub(crate) const ENEMY_ATTACK_COOLDOWN: f32 = 1.05;
use crate::actor::BodyAnimFacts;
use crate::time::feel::SandboxFeelTuning;
use crate::world::platforms::MovingPlatformState;
use crate::{physics, MeleeSwing};
use ambition_characters::actor::BodyCombat;
use ambition_engine_core::RoomGeometry;
use ambition_sfx::{SfxMessage, SfxWriter};
use ambition_world::collision::MovingPlatformSet;

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
        axes: ae::LocalAxes::from_vec(actor.locomotion),
        jump_pressed: actor.jump_pressed,
        jump_held: actor.jump_held,
        jump_released: actor.jump_released,
        dash_pressed: actor.dash_pressed,
        fly_toggle_pressed: actor.fly_toggle_pressed,
        blink_pressed: actor.blink_pressed,
        blink_held: actor.blink_held,
        blink_released: actor.blink_released,
        blink_quick_dir: ae::WorldVec2(actor.blink_quick_dir),
        blink_aim_step: ae::WorldVec2(actor.blink_aim_step),
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
        input.axes = ae::LocalAxes::ZERO;
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
        input.axes = ae::LocalAxes::new(input.axes.x * scale, input.axes.y * scale);
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

/// World-ORB pogo for the MOVESET down-air (fable review R2.5, the block half of
/// the unified pogo). When a body playing a `pogo_bounce` on-hit move
/// (`attack_air_down`) overlaps a world `PogoOrb` block, rebound it away from
/// gravity — the collision-world orbs the flat player pogo used, now that the
/// melee fold routes the down-air through the moveset. The ENTITY half (enemies,
/// breakables) rides `dispatch_hitbox_on_hit` + `apply_pogo_bounce`; together
/// they are one pogo (`PogoTarget` entities + `PogoOrb` blocks). `set_jump_
/// velocity` SETS (idempotent), so no per-frame dedup — the owner bounces clear.
pub fn pogo_moveset_off_world_orbs(
    world: Res<RoomGeometry>,
    moving_platforms: Res<MovingPlatformSet>,
    feature_ecs_overlay: Res<FeatureEcsWorldOverlay>,
    mut hitboxes: Query<(
        Entity,
        &ambition_vfx::Hitbox,
        &mut crate::combat::on_hit::HitboxOnHit,
    )>,
    boxes: Query<&ae::CenteredAabb>,
    mut owners: Query<(
        &physics::ResolvedMotionFrame,
        &mut ae::BodyKinematics,
        &mut ambition_engine_core::BodyGroundState,
    )>,
    mut sfx: SfxWriter,
) {
    // The pogo hitboxes live this frame + where their volume covers. A hitbox that
    // has ALREADY world-bounced this strike is skipped: the world-orb pogo carries
    // no victim ENTITY to record in `HitboxOnHit.fired` (an orb is a collision-world
    // block, not an entity), so — like the entity pogo dedups by victim — this
    // dedups the whole strike with the OWNER as the sentinel key. Without it the
    // bounce + `Pogo` sfx re-fired every frame the box overlapped the orb.
    let pogo: Vec<(Entity, Entity, ae::Aabb, f32)> = hitboxes
        .iter()
        .filter(|(_, _, on_hit)| on_hit.effect.key == crate::combat::on_hit::POGO_BOUNCE_KEY)
        .filter(|(_, hitbox, on_hit)| !on_hit.has_fired(hitbox.owner))
        .filter_map(|(hb_entity, hitbox, on_hit)| {
            let owner_box = boxes.get(hitbox.owner).ok()?;
            let world_box = hitbox.world_volume(owner_box.center).bounds();
            Some((
                hb_entity,
                hitbox.owner,
                world_box,
                crate::combat::on_hit::pogo_rise_from(&on_hit.effect),
            ))
        })
        .collect();
    if pogo.is_empty() {
        return;
    }
    let assembled = ambition_world::collision::world_with_sandbox_solids(
        &world.0,
        &moving_platforms.0,
        &feature_ecs_overlay,
    );
    for (hb_entity, owner, world_box, rise) in pogo {
        if pogo_target_for_attack_hitbox(&assembled, world_box).is_none() {
            continue;
        }
        let Ok((resolved_frame, mut kin, mut ground)) = owners.get_mut(owner) else {
            continue;
        };
        // The owner's per-tick resolved frame: the pogo launches opposite ITS
        // down, the same value its movement integrated under.
        let gdir = resolved_frame.down();
        let pos = kin.pos;
        ae::movement::set_jump_velocity(&mut kin.vel, gdir, rise);
        ground.on_ground = false;
        sfx.write(SfxMessage::Pogo { pos });
        // One bounce per strike: mark this hitbox as having world-bounced so a
        // sustained overlap doesn't re-pogo every frame (the entity pogo's
        // `HitboxOnHit.fired` dedup, extended to the entity-less world orb).
        if let Ok((_, _, mut on_hit)) = hitboxes.get_mut(hb_entity) {
            on_hit.mark_fired(owner);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn start_attack(
    sfx: &mut SfxWriter,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::BodyClustersMut<'_>,
    // The body's published maneuver facts (ADR 0024): the view's
    // wall-cling / dashing reads are semantic, not policy internals.
    facts: &ae::BodyMotionFacts,
    attack: &mut Option<MeleeSwing>,
    // Presentation-only slash-anim timer. `None` for a body with no
    // `BodyAnimFacts` (an actor) — the swing lifecycle is gameplay, the anim is
    // presentation and rides only where authored.
    anim: Option<&mut BodyAnimFacts>,
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
        wall_clinging: facts.wall_clinging,
        dashing: facts.dashing,
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

// `emit_melee_slash` (+ its size curve) moved to `crate::combat::util`
// (E2): the ONE slash-emit is shared by the moveset/hitbox strike paths
// (combat) and this shared body-melee path.
pub use crate::combat::util::emit_melee_slash;

/// Source the player's melee hitbox from the sprite manifest — the box authored
/// and shown by `debug-hitboxes` — so the gameplay damage volume matches the
/// visible blade, the same data-driven path bosses use
/// through the App-local `AuthoredAttackVolumeResolver`. Returns `None` when the
/// current swing's animation has no authored hitbox, so callers fall back to the
/// hardcoded `AttackSpec` volume.
pub fn player_attack_hitbox(
    character_catalog: &CharacterCatalog,
    authored_volumes: &crate::combat::authored_volumes::AuthoredAttackVolumeResolver,
    sprite_character_id: Option<&str>,
    view: &AttackView,
    intent: AttackIntent,
    gravity_dir: ae::Vec2,
) -> Option<ae::CombatVolume> {
    let animation = attack_intent_animation(intent);
    authored_volumes.resolve(
        character_catalog,
        sprite_character_id,
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
pub fn advance_attack(
    character_catalog: &CharacterCatalog,
    authored_volumes: &crate::combat::authored_volumes::AuthoredAttackVolumeResolver,
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
    faction: ambition_characters::actor::pose::ActorFaction,
    // The body's sprite catalog id, if any, so its authored per-animation melee
    // box drives the strike (the same data-driven lookup for player + actor).
    // `None` → the player's hardcoded manifest root.
    sprite_cid: Option<&str>,
    sfx: &mut SfxWriter,
    vfx: &mut MessageWriter<VfxMessage>,
    world: &ae::World,
    moving_platforms: &[MovingPlatformState],
    clusters: &mut ae::BodyClustersMut<'_>,
    // Published maneuver facts for the active-phase strike view (ADR 0024).
    facts: &ae::BodyMotionFacts,
    attack: &mut Option<MeleeSwing>,
    anim: Option<&mut BodyAnimFacts>,
    tuning: ae::MovementTuning,
    // The body's frame down direction, resolved by the environment at the
    // body's position (never reconstructed from tuning).
    gravity_dir: ae::Vec2,
    frame_dt: f32,
    feature_ecs_overlay: &FeatureEcsWorldOverlay,
    hit_events: &mut MessageWriter<crate::combat::events::HitEvent>,
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
            wall_clinging: facts.wall_clinging,
            dashing: facts.dashing,
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
        let manifest = authored_volumes.resolve(
            character_catalog,
            sprite_cid,
            animation,
            view.pos,
            view.size,
            view.facing,
            gravity_dir,
        );
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
            let (knock_x, knockback_strength) = if matches!(
                faction,
                ambition_characters::actor::pose::ActorFaction::Player
            ) {
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
                gravity_dir,
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
            let attack_world = ambition_world::collision::world_with_sandbox_solids(
                world,
                moving_platforms,
                feature_ecs_overlay,
            );
            if let Some(orb_aabb) = pogo_target_for_attack_hitbox(&attack_world, world_box) {
                ae::movement::set_jump_velocity(
                    &mut clusters.kinematics.vel,
                    gravity_dir,
                    tuning.pogo_speed,
                );
                ae::refresh_movement_resources_clusters(
                    clusters.abilities,
                    &mut *clusters.dash,
                    &mut *clusters.jump,
                    tuning.air_jumps,
                );
                clusters.ground.on_ground = false;
                attack_state.pogo_applied = true;
                sfx.write(SfxMessage::Pogo { pos: player_pos });
                hit_events.write(crate::combat::events::HitEvent {
                    volume: orb_aabb.into(),
                    damage: 1,
                    source: crate::combat::HitSource::PogoBounce,
                    attacker: Some(actor),
                    target: crate::combat::events::HitTarget::OrbMatch,
                    mode: crate::combat::HitMode::Knockback,
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
    mut brain_actions: MessageReader<ActorActionMessage>,
    mut sfx_writer: SfxWriter,
    mut vfx_writer: MessageWriter<VfxMessage>,
    mut bodies: Query<(
        ae::BodyClusterQueryData,
        &ae::BodyMotionFacts,
        &physics::ResolvedMotionFrame,
        &mut BodyMelee,
        &BodyCombat,
        &ActorControl,
        Option<&crate::combat::held_items::HeldItem>,
        Option<&super::components::CombatTuning>,
        Option<&mut BodyAnimFacts>,
        Has<MovesetMelee>,
    )>,
) {
    // The resolver can emit the same body once per frame; collect the requesters
    // ONCE EACH, in message order.
    //
    // DETERMINISM (N0.3): this loop spawns strike entities and writes sfx / vfx /
    // hit messages, so the order it visits bodies in is observable — in entity
    // ids, in message order, and therefore in every replay and state hash. It used
    // to iterate a `std::collections::HashSet<Entity>`, whose order is seeded per
    // PROCESS: two runs of the same binary on the same inputs could swing two
    // bodies in opposite orders. Message arrival order is deterministic; the set
    // is now a membership filter and is never iterated.
    let mut requested = std::collections::HashSet::new();
    let melee_actors: Vec<Entity> = brain_actions
        .read()
        .filter(|m| m.is_melee())
        .map(|m| m.actor)
        .filter(|actor| requested.insert(*actor))
        .collect();
    for actor in melee_actors {
        let Ok((
            mut cq,
            facts,
            resolved_frame,
            mut melee,
            combat,
            control,
            held,
            config,
            mut anim,
            moveset_melee,
        )) = bodies.get_mut(actor)
        else {
            continue;
        };
        // A body whose melee is a moveset `"attack"` move is driven by
        // `trigger_moveset_moves` → `advance_move_playback`; the flat swing must not
        // ALSO start (double-fire). Its `BodyMelee` read-model is projected from the
        // live move instead (`project_moveset_melee_to_body_melee`).
        if moveset_melee {
            continue;
        }
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
        // (projected onto the combat-owned `CombatTuning` at spawn) paces its
        // brain's next swing; the player carries no `CombatTuning`, so it has
        // none (its re-swing is gated by the swing duration + recoil lock).
        let cooldown = config
            .map(|c| ENEMY_ATTACK_COOLDOWN * c.attack_cooldown_mult)
            .unwrap_or(0.0);
        let gravity_dir = resolved_frame.down();
        let mut clusters = cq.as_clusters_mut();
        start_attack(
            &mut sfx_writer,
            &mut vfx_writer,
            &mut clusters,
            facts,
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
    character_catalog: Res<CharacterCatalog>,
    authored_volumes: Res<crate::combat::authored_volumes::AuthoredAttackVolumeResolver>,
    world_time: Res<ambition_time::WorldTime>,
    world: Res<RoomGeometry>,
    moving_platforms: Res<MovingPlatformSet>,
    editable_tuning: Res<EditableMovementTuning>,
    feature_ecs_overlay: Res<FeatureEcsWorldOverlay>,
    mut commands: bevy::prelude::Commands,
    mut sfx_writer: SfxWriter,
    mut vfx_writer: MessageWriter<VfxMessage>,
    mut hit_events: MessageWriter<crate::combat::events::HitEvent>,
    mut bodies: Query<(
        Entity,
        ae::BodyClusterQueryData,
        &ae::BodyMotionFacts,
        &physics::ResolvedMotionFrame,
        &mut BodyMelee,
        &ambition_characters::actor::pose::ActorFaction,
        Option<&ambition_characters::brain::Brain>,
        Option<&super::components::CombatTuning>,
        Option<&ambition_characters::actor::WornCharacter>,
        Option<&mut BodyAnimFacts>,
        Has<MovesetMelee>,
    )>,
) {
    let dt = world_time.sim_dt();
    for (
        entity,
        mut cq,
        facts,
        resolved_frame,
        mut melee,
        faction,
        brain,
        config,
        worn,
        mut anim,
        moveset_melee,
    ) in &mut bodies
    {
        // The recovery / refire floors tick every frame regardless of a live swing
        // (`advance_attack` advances the swing's own `elapsed`).
        melee.cooldown = (melee.cooldown - dt).max(0.0);
        melee.ranged_cooldown = (melee.ranged_cooldown - dt).max(0.0);
        // A body whose melee is a moveset `"attack"` move owns its swing through the
        // moveset runtime; its `BodyMelee.swing` is a PROJECTION written after this
        // system (`project_moveset_melee_to_body_melee`), not a flat swing to
        // advance/strike here. Its ranged refire floor still ticks above, so ranged
        // bodies (the PCA) keep their fire-rate; only the melee swing logic skips.
        if moveset_melee {
            continue;
        }
        if melee.swing.is_none() {
            if let Some(anim) = anim.as_deref_mut() {
                anim.slash_anim_timer = 0.0;
            }
            continue;
        }
        let tuning = editable_tuning.as_engine();
        // The body's per-tick resolved frame, so the pogo bounce launches
        // OPPOSITE the same down its movement integrated under.
        let gravity_dir = resolved_frame.down();
        // The strike's effective allegiance picks the damage channel. Actors use
        // their spawn-projected catalog id; controllable bodies use their worn id.
        let strike_faction = crate::combat::targeting::effective_faction(*faction, brain);
        let sprite_cid = config
            .and_then(|c| c.sprite_character_id.as_deref())
            .or_else(|| worn.map(ambition_characters::actor::WornCharacter::id));
        let mut clusters = cq.as_clusters_mut();
        advance_attack(
            &character_catalog,
            &authored_volumes,
            &mut commands,
            entity,
            strike_faction,
            sprite_cid,
            &mut sfx_writer,
            &mut vfx_writer,
            &world.0,
            &moving_platforms.0,
            &mut clusters,
            facts,
            &mut melee.swing,
            anim.as_deref_mut(),
            tuning,
            gravity_dir,
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
                dashing: false,
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
