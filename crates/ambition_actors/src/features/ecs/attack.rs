//! Attack-phase support: brain-output → engine-input translation, the shared
//! post-hit stagger gates, the moveset down-air's world-orb pogo, and the
//! debug-overlay hitbox source.
//!
//! The melee LIFECYCLE lives entirely on the moveset runtime now
//! (`combat::moveset`): a body's swing is a `"attack"`-verb move started by
//! `trigger_moveset_moves`, advanced by `advance_move_playback`, and projected
//! back into `BodyMelee` for the anim/HUD/telegraph read-model by
//! `project_moveset_melee_to_body_melee`. The former flat player+actor melee
//! driver (`start_body_melee`/`advance_body_melee`/`start_attack`/`advance_attack`
//! and the single-hitbox `spawn_melee_strike`) is gone — there is ONE melee path.
//!
//! Pure sim + message emission — it writes `SfxMessage`/`VfxMessage` *facts* and
//! holds no render dependency.

use bevy::prelude::{Entity, Query, Res};

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_characters::actor::control::ActorControlFrame;
use ambition_engine_core::{self as ae, AabbExt};

use crate::combat::BodyMelee;
use crate::combat::{AttackIntent, AttackView};
use crate::world::overlay::FeatureEcsWorldOverlay;

use crate::physics;
use crate::time::feel::SandboxFeelTuning;
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
        movement: ambition_engine_core::ActionEdges::EMPTY
            .with(
                ambition_engine_core::MovementAction::Jump,
                ambition_engine_core::Edge {
                    pressed: actor.jump_pressed,
                    held: actor.jump_held,
                    released: actor.jump_released,
                },
            )
            .with(
                ambition_engine_core::MovementAction::Dash,
                ambition_engine_core::Edge {
                    pressed: actor.dash_pressed,
                    held: false,
                    released: false,
                },
            )
            .with(
                ambition_engine_core::MovementAction::Blink,
                ambition_engine_core::Edge {
                    pressed: actor.blink_pressed,
                    held: actor.blink_held,
                    released: actor.blink_released,
                },
            )
            .with(
                ambition_engine_core::MovementAction::FlyToggle,
                ambition_engine_core::Edge {
                    pressed: actor.fly_toggle_pressed,
                    held: false,
                    released: false,
                },
            )
            .with(
                ambition_engine_core::MovementAction::FastFall,
                ambition_engine_core::Edge {
                    pressed: actor.fast_fall_pressed,
                    held: false,
                    released: false,
                },
            ),
        axes: ae::LocalAxes::from_vec(actor.locomotion),
        blink_quick_dir: ae::WorldVec2(actor.blink_quick_dir),
        blink_aim_step: ae::WorldVec2(actor.blink_aim_step),
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
        // Strip all locomotion authority (fly-toggle is exempt — see above).
        input.movement.set(ae::MovementAction::Jump, ae::Edge::NONE);
        input.movement.set(ae::MovementAction::Dash, ae::Edge::NONE);
        input
            .movement
            .set(ae::MovementAction::FastFall, ae::Edge::NONE);
        input
            .movement
            .set(ae::MovementAction::Blink, ae::Edge::NONE);
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
        // No jump/dash/blink, but PRESERVE an in-progress jump's held/released
        // (only the press EDGE is eaten, matching the pre-re-key behavior).
        input.movement.set_pressed(ae::MovementAction::Jump, false);
        input.movement.set(ae::MovementAction::Dash, ae::Edge::NONE);
        input
            .movement
            .set(ae::MovementAction::FastFall, ae::Edge::NONE);
        input
            .movement
            .set(ae::MovementAction::Blink, ae::Edge::NONE);
        input.interact_pressed = false;
    }
}

/// Tick every body's `BodyMelee` cooldown floors on the sim clock. The melee
/// swing itself lives on the moveset runtime (`advance_move_playback` +
/// `project_moveset_melee_to_body_melee`); what remains on `BodyMelee` is the
/// body-side ranged refire floor (`ranged_cooldown`, invariant I3) armed by
/// `try_fire_ranged`, and the legacy melee-recovery floor (`cooldown`) the AI
/// telegraph reads. Both must keep counting down or a ranged body freezes after
/// one shot. This REPLACES the cooldown-decrement that rode the deleted flat
/// `advance_body_melee`.
pub fn tick_body_melee_cooldowns(
    world_time: Res<ambition_time::WorldTime>,
    mut bodies: Query<&mut BodyMelee>,
) {
    let dt = world_time.sim_dt();
    for mut melee in &mut bodies {
        melee.cooldown = (melee.cooldown - dt).max(0.0);
        melee.ranged_cooldown = (melee.ranged_cooldown - dt).max(0.0);
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
/// gravity — the collision-world orbs the melee down-air reaches, now that the
/// swing runs through the moveset. The ENTITY half (enemies, breakables) rides
/// `dispatch_hitbox_on_hit` + `apply_pogo_bounce`; together they are one pogo
/// (`PogoTarget` entities + `PogoOrb` blocks). `set_jump_velocity` SETS
/// (idempotent), so no per-frame dedup — the owner bounces clear.
pub fn pogo_moveset_off_world_orbs(
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<RoomGeometry>,
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

// `emit_melee_slash` (+ its size curve) lives in `crate::combat::util` (E2): the
// ONE slash-emit shared by the moveset strike path and `spawn_melee_strike`'s
// heirs. Re-exported here for callers that reach it through this module.
pub use crate::combat::util::emit_melee_slash;

/// Source the body's melee hitbox from the sprite manifest — the box authored
/// and shown by `debug-hitboxes` — so the debug overlay draws the visible blade
/// through the same data-driven path bosses use via the App-local
/// `AuthoredAttackVolumeResolver`. Returns `None` when the current swing's
/// animation has no authored hitbox, so the overlay falls back to the hardcoded
/// `AttackSpec` volume. Consumed by `dev/debug_overlay/gizmos.rs`.
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
}
