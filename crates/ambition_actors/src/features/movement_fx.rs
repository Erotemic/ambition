//! Movement-event presentation facts: translate a frame's engine
//! [`ae::FrameEvents`] (jump/dash/blink/ledge/etc. ops + blink endpoints) into
//! `SfxMessage`/`VfxMessage` *facts*, and arm the short presentation timers the
//! ops imply (wall-jump pose, blink-camera lerp, hit flash).
//!
//! Pure sim + message emission — `VfxMessage` is `ambition_vfx`, not
//! `ambition_render`, so this carries no render dependency. It is called from the
//! host's player-tick control/sim phases; it lived in `ambition_app` only because
//! it was authored beside that glue.
//!
//! Player-centrism note: [`handle_player_events`] is still named "player" and
//! arms `Player*State`, but its SFX/VFX half is the body-generic
//! [`emit_movement_fx`] — the SAME emitter the actor tick runs, so an AI fighter
//! that jumps/dashes/dodges/wall-jumps produces the same dust + SFX the player
//! does (fable review 2026-07-02 §A8).

use bevy::prelude::MessageWriter;

use ambition_engine_core as ae;
use ambition_vfx::vfx::{ParticleKind, VfxMessage};

use crate::actor::BodyAnimFacts;
use crate::avatar::PlayerBlinkCameraState;
use ambition_characters::actor::BodyCombat;
use ambition_sfx::{SfxMessage, SfxWriter};

/// How long the wall-jump push-off pose holds after the WallJump op fires. Short
/// enough to clear before the apex of the jump arc so the regular `Jump` row
/// picks back up; long enough that the kick reads at typical playback rates.
const WALL_JUMP_ANIM_HOLD_SECS: f32 = 0.18;

/// Advance a body's presentation overlay timers ([`crate::actor::BodyAnimFacts`])
/// one frame: decay the op-armed poses (slash / shoot / wall-jump / interact) and
/// arm+decay the edge-derived poses (landing on the air→ground edge, graded hard
/// vs soft by pre-touchdown speed; dash-startup on the dash rising edge). Body-
/// generic — reads only `(on_ground, vel_y, dashing)` body facts, no player
/// specifics — so the player tick AND every actor advance their overlays through
/// the SAME code, and `pick_actor_anim` can show those poses for AI fighters too
/// (fable review §A9). The op-armed timers are set elsewhere (attack / projectile /
/// the movement WallJump op); this only advances them.
pub fn advance_body_anim_overlays(
    on_ground: bool,
    vel_y: f32,
    dashing: bool,
    anim: &mut crate::actor::BodyAnimFacts,
    frame_dt: f32,
) {
    /// Pre-touchdown downward speed (px/s) above which the hard-landing row plays.
    const HARD_LAND_SPEED: f32 = 520.0;
    /// Time the landing pose holds after touchdown (hard vs soft).
    const LAND_HARD_HOLD_SECS: f32 = 0.34;
    const LAND_SOFT_HOLD_SECS: f32 = 0.16;
    /// Brief pre-roll for the dash startup pose (below the dash's own duration so
    /// the streaking dash row still gets airtime).
    const DASH_STARTUP_SECS: f32 = 0.05;

    // Op-armed poses just decay here (armed by attack / projectile / movement ops).
    anim.slash_anim_timer = (anim.slash_anim_timer - frame_dt).max(0.0);
    anim.shoot_anim_timer = (anim.shoot_anim_timer - frame_dt).max(0.0);
    anim.wall_jump_anim_timer = (anim.wall_jump_anim_timer - frame_dt).max(0.0);
    anim.interact_anim_timer = (anim.interact_anim_timer - frame_dt).max(0.0);

    // Landing edge: airborne last frame, grounded this frame.
    if on_ground && !anim.anim_prev_on_ground {
        let hard = anim.anim_prev_vel_y >= HARD_LAND_SPEED;
        anim.land_anim_hard = hard;
        anim.land_anim_timer = if hard {
            LAND_HARD_HOLD_SECS
        } else {
            LAND_SOFT_HOLD_SECS
        };
    } else if !on_ground {
        // The landing pose only plays on the ground.
        anim.land_anim_timer = 0.0;
    } else {
        anim.land_anim_timer = (anim.land_anim_timer - frame_dt).max(0.0);
    }

    // Dash rising edge: no dash last frame, a dash this frame.
    if dashing && !anim.anim_prev_dashing {
        anim.dash_startup_timer = DASH_STARTUP_SECS;
    } else {
        anim.dash_startup_timer = (anim.dash_startup_timer - frame_dt).max(0.0);
    }

    // Snapshot for the next frame's edge detection.
    anim.anim_prev_on_ground = on_ground;
    anim.anim_prev_vel_y = vel_y;
    anim.anim_prev_dashing = dashing;
}

/// Arm the op-driven presentation overlays a movement frame implies on ANY body's
/// [`crate::actor::BodyAnimFacts`]: the wall-jump push-off pose fires on the
/// `WallJump` op. Body-generic so the player tick AND the actor tick arm the SAME
/// pose from the SAME frame data — an AI fighter that wall-jumps shows the kick pose
/// the player does, not just its dust/SFX (fable review §A9 follow-up). The other
/// op-armed overlays (slash / shoot) are armed at their own effect sites — attack
/// (`combat::attack`) and projectile fire (`brain_effects` / `projectile::systems`)
/// — because those aren't movement ops; this covers the movement ops only, and
/// [`advance_body_anim_overlays`] decays every op-armed timer afterward.
pub fn arm_movement_anim_overlays(anim: &mut BodyAnimFacts, events: &ae::FrameEvents) {
    for op in &events.operations {
        if matches!(op, ae::MovementOp::WallJump) {
            anim.wall_jump_anim_timer = WALL_JUMP_ANIM_HOLD_SECS;
        }
    }
}

/// Body-generic movement presentation: translate a frame's [`ae::FrameEvents`]
/// (jump/dash/dodge/wall-jump/pogo/swim/ledge/shield/fly ops + blink endpoints)
/// into `SfxMessage`/`VfxMessage` facts at the body's position, plus the
/// grounded-transition landing dust.
///
/// Carries NO body-specific state — the wall-jump anim pose, the blink-camera
/// lerp, and the action hit-flash stay with each caller ([`handle_player_events`]
/// arms them for the player; the actor tick does not). This is the ONE emit site
/// the actor path and the player path share, retiring the old blink-only actor
/// branch + its hand-copied second blink emit (the "parallel emission site" bug —
/// fable review §A8).
#[allow(clippy::too_many_arguments)]
pub fn emit_movement_fx(
    sfx: &mut SfxWriter,
    vfx: &mut MessageWriter<VfxMessage>,
    events: &ae::FrameEvents,
    pos: ae::Vec2,
    facing: f32,
    size: ae::Vec2,
    on_ground: bool,
    was_grounded: Option<bool>,
) {
    for op in &events.operations {
        match op {
            ae::MovementOp::Jump | ae::MovementOp::WallJump => {
                sfx.write(SfxMessage::Jump { pos });
                vfx.write(VfxMessage::Dust { pos, facing });
            }
            ae::MovementOp::DoubleJump => {
                sfx.write(SfxMessage::DoubleJump { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 14,
                    speed: 210.0,
                    color: [0.70, 1.0, 0.86, 0.82],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::Dash | ae::MovementOp::DoubleDash => {
                sfx.write(SfxMessage::Dash { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 10,
                    speed: 330.0,
                    color: [1.0, 0.86, 0.38, 0.90],
                    kind: ParticleKind::Spark,
                });
            }
            ae::MovementOp::DodgeRoll => {
                sfx.write(SfxMessage::Dash { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 8,
                    speed: 240.0,
                    color: [0.60, 1.0, 0.70, 0.80],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::Blink | ae::MovementOp::PrecisionBlink => {
                // Blink visuals use the explicit `events.blinks` endpoint data below.
            }
            ae::MovementOp::FlyToggle => {
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 12,
                    speed: 180.0,
                    color: [0.45, 0.82, 1.0, 0.72],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::Pogo | ae::MovementOp::Rebound => {
                sfx.write(SfxMessage::Pogo { pos });
            }
            ae::MovementOp::SwimStroke => {
                sfx.write(SfxMessage::Jump { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 8,
                    speed: 150.0,
                    color: [0.50, 0.85, 1.0, 0.70],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::LedgeGrab => {
                vfx.write(VfxMessage::Dust { pos, facing });
            }
            ae::MovementOp::LedgeJump => {
                sfx.write(SfxMessage::Jump { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 8,
                    speed: 180.0,
                    color: [0.70, 1.0, 0.86, 0.82],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::LedgeRoll => {
                // Reuse the dash sfx — the ledge roll IS a dodge-roll
                // semantically (invuln rolling motion). Adds a small
                // dust burst at the platform lip for visual feedback.
                sfx.write(SfxMessage::Dash { pos });
                vfx.write(VfxMessage::Dust { pos, facing });
            }
            ae::MovementOp::LedgeGetupAttack => {
                // The engine pairs this op with MovementOp::Slash on
                // the same frame, so the slash SFX/VFX (and the
                // attack hitbox) fire through the normal slash path.
                // Here we only add the lift-up dust so the swing
                // reads as "coming off the ledge," not "in mid-air."
                // TODO: when a dedicated getup-attack sprite lands,
                // route a distinct VFX/SFX here too.
                vfx.write(VfxMessage::Dust { pos, facing });
            }
            ae::MovementOp::ShieldUp => {
                // Reuse the quick blink tone as a placeholder until a
                // dedicated Shield SoundCue is added to the sfxbank.
                sfx.write(SfxMessage::Blink {
                    pos,
                    precision: false,
                });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 12,
                    speed: 120.0,
                    color: [0.50, 0.80, 1.0, 0.70],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::LedgeClimbStart
            | ae::MovementOp::LedgeClimbFinish
            | ae::MovementOp::LedgeDrop
            | ae::MovementOp::WallCling
            | ae::MovementOp::WallClimb
            | ae::MovementOp::Slash => {}
            ae::MovementOp::Reset => {
                sfx.write(SfxMessage::Reset { pos });
            }
        }
    }
    for blink in &events.blinks {
        sfx.write(SfxMessage::Blink {
            pos: blink.from,
            precision: blink.precision,
        });
        vfx.write(VfxMessage::BlinkEffects {
            from: blink.from,
            to: blink.to,
            precision: blink.precision,
        });
    }
    if let Some(was_grounded) = was_grounded {
        if !was_grounded && on_ground {
            let feet = pos + ae::Vec2::new(0.0, size.y * 0.5);
            // Touchdown footfall. Emitted for every body; provider authority
            // gates it, so a game hears it only by authoring `player.land`.
            sfx.write(SfxMessage::Land { pos: feet });
            vfx.write(VfxMessage::Dust { pos: feet, facing });
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_player_events(
    sfx: &mut SfxWriter,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &ae::BodyClustersMut<'_>,
    combat: &mut BodyCombat,
    blink_cam: &mut PlayerBlinkCameraState,
    anim: &mut BodyAnimFacts,
    events: ae::FrameEvents,
    was_grounded: Option<bool>,
) {
    let pos = clusters.kinematics.pos;
    let facing = clusters.kinematics.facing;
    let size = clusters.kinematics.size;
    let on_ground = clusters.ground.on_ground;
    // Body-generic SFX/VFX — the SAME emitter the actor tick uses.
    emit_movement_fx(
        sfx,
        vfx,
        &events,
        pos,
        facing,
        size,
        on_ground,
        was_grounded,
    );
    // Body-generic op-driven overlay poses (the wall-jump push-off) — the SAME
    // arming the actor tick runs (§A9). Player-specific presentation the shared
    // arming deliberately omits stays inline below: the blink-camera lerp.
    arm_movement_anim_overlays(anim, &events);
    for blink in &events.blinks {
        blink_cam.blink_in_duration = crate::BLINK_IN_ANIM_TIME;
        blink_cam.blink_in_timer = blink_cam.blink_in_duration;
        blink_cam.blink_camera_from = blink.from;
        blink_cam.blink_camera_to = blink.to;
    }
    // The white hit-flash is DAMAGE feedback — a hazard hit reads as being hurt.
    // Movement operations (jump, dash, blink, …) deliberately do NOT flash: an
    // action is not a hit, and flashing the sprite white on every jump reads as
    // taking damage. Real combat/hazard damage arms `hit_flash` through the damage
    // path (`features::ecs::damage_apply`).
    if events.hazard {
        combat.hit_flash = 0.12;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[derive(Resource)]
    struct TestEvents(ae::FrameEvents);

    fn emit_system(
        mut sfx: SfxWriter,
        mut vfx: MessageWriter<VfxMessage>,
        events: Res<TestEvents>,
    ) {
        emit_movement_fx(
            &mut sfx,
            &mut vfx,
            &events.0,
            ae::Vec2::ZERO,
            1.0,
            ae::Vec2::new(20.0, 40.0),
            true,        // on_ground now
            Some(false), // was airborne last frame → the landing dust fires
        );
    }

    /// The body-generic emitter (shared by the player tick AND the actor tick)
    /// turns a frame's ops into movement SFX/VFX: a `Jump` op yields one `Jump`
    /// SFX + a `Dust` VFX, and the air→ground transition adds the landing dust.
    /// Pins that a future edit can't silently drop actor (or player) movement
    /// presentation the way the old blink-only actor branch did (§A8).
    #[test]
    fn emit_movement_fx_emits_jump_sfx_and_dust_plus_landing() {
        let mut events = ae::FrameEvents::default();
        events.operations.push(ae::MovementOp::Jump);
        let mut app = App::new();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<VfxMessage>();
        app.insert_resource(TestEvents(events));
        app.add_systems(Update, emit_system);
        app.update();
        let sfx: Vec<SfxMessage> = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ambition_sfx::OwnedSfxMessage>>()
            .drain()
            .map(|message| message.request)
            .collect();
        let vfx: Vec<VfxMessage> = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<VfxMessage>>()
            .drain()
            .collect();
        assert_eq!(sfx.len(), 1, "a Jump op yields exactly one Jump SFX");
        assert!(matches!(sfx[0], SfxMessage::Jump { .. }));
        assert_eq!(vfx.len(), 2, "the Jump dust + the air→ground landing dust");
        assert!(
            vfx.iter().all(|m| matches!(m, VfxMessage::Dust { .. })),
            "both VFX are Dust bursts"
        );
    }

    /// The body-generic overlay arming (shared by the player tick AND the actor
    /// tick) sets the wall-jump push-off pose timer on a `WallJump` op and leaves
    /// it untouched otherwise. Pins that an actor which wall-jumps arms the SAME
    /// pose the player does (§A9 follow-up) — a future edit can't silently make
    /// this player-only again.
    #[test]
    fn arm_movement_anim_overlays_arms_wall_jump_pose_on_wall_jump_op() {
        let mut anim = BodyAnimFacts::default();

        let mut plain = ae::FrameEvents::default();
        plain.operations.push(ae::MovementOp::Jump);
        arm_movement_anim_overlays(&mut anim, &plain);
        assert_eq!(
            anim.wall_jump_anim_timer, 0.0,
            "a plain Jump op does NOT arm the wall-jump pose"
        );

        let mut wall = ae::FrameEvents::default();
        wall.operations.push(ae::MovementOp::WallJump);
        arm_movement_anim_overlays(&mut anim, &wall);
        assert_eq!(
            anim.wall_jump_anim_timer, WALL_JUMP_ANIM_HOLD_SECS,
            "a WallJump op arms the push-off pose for the hold duration"
        );
    }
}
