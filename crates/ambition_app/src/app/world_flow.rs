#![allow(unused_imports)]
use ambition_engine_core::AabbExt;

#[allow(unused_imports)]
use super::cli::*;
#[allow(unused_imports)]
use super::dev_runtime::*;
#[allow(unused_imports)]
use super::feedback::*;
#[allow(unused_imports)]
use super::hud::*;
#[allow(unused_imports)]
use super::phases::*;
#[allow(unused_imports)]
use super::player_tick::*;
#[allow(unused_imports)]
use super::plugins::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::setup_systems::*;
#[allow(unused_imports)]
use super::*;
#[allow(unused_imports)]
use ambition_gameplay_core::schedule::*;

/// Bundle of the two room-reset clock/sim resources, so systems that
/// already sit near Bevy's 16-SystemParam limit (e.g.
/// [`apply_room_transition_system`]) can take both in one slot. The
/// sim-clock `time_scale` (time-owned [`ambition_gameplay_core::time::clock_state::ClockState`])
/// and the room-transition cooldown (sim-owned [`ambition_gameplay_core::SandboxSimState`])
/// are reset together on every room load / death / respawn.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct RoomClock<'w> {
    pub sim_state: ResMut<'w, ambition_gameplay_core::SandboxSimState>,
    pub clock: ResMut<'w, ambition_gameplay_core::time::clock_state::ClockState>,
}

pub(super) fn sandbox_dt(hitstop_timer: f32, time_scale: f32, frame_dt: f32) -> f32 {
    if hitstop_timer > 0.0 {
        0.0
    } else {
        frame_dt * time_scale
    }
}

fn pogo_target_for_attack_hitbox(world: &ae::World, attack: ae::Aabb) -> Option<ae::Aabb> {
    world
        .blocks
        .iter()
        .find(|block| block.kind.is_pogo_target() && attack.strict_intersects(block.aabb))
        .map(|block| block.aabb)
}

mod room_flow;
pub use room_flow::*;

/// Probe straight down from the player's feet for the nearest block
/// top (within 256 px). Returns `(distance, source)` where `source` is
/// `"world"`, `"overlay"`, or `"both"`. `None` means nothing — the
/// player is over a pit (real bug) or `to_room_set()` / overlay
/// rebuild hasn't materialised the floor yet (the race we're hunting).
fn ground_gap_below_feet(
    feet_y: f32,
    body: &ae::Aabb,
    world: &ae::World,
    feature_overlay: &ambition_gameplay_core::features::FeatureEcsWorldOverlay,
) -> Option<(f32, &'static str)> {
    const MAX_PROBE_PX: f32 = 256.0;
    let probe = |blocks: &[ae::Block]| {
        let mut best: Option<f32> = None;
        for block in blocks {
            // X must overlap the player body.
            if block.aabb.right() <= body.left() || block.aabb.left() >= body.right() {
                continue;
            }
            // Only consider blocks whose top is below feet.
            let top = block.aabb.top();
            if top < feet_y {
                continue;
            }
            let gap = top - feet_y;
            if gap > MAX_PROBE_PX {
                continue;
            }
            best = Some(best.map_or(gap, |b| b.min(gap)));
        }
        best
    };
    let world_gap = probe(&world.blocks);
    let overlay_gap = probe(&feature_overlay.blocks);
    match (world_gap, overlay_gap) {
        (Some(a), Some(b)) if (a - b).abs() < 0.5 => Some((a.min(b), "both")),
        (Some(a), Some(b)) if a <= b => Some((a, "world")),
        (Some(_), Some(b)) => Some((b, "overlay")),
        (Some(a), None) => Some((a, "world")),
        (None, Some(b)) => Some((b, "overlay")),
        (None, None) => None,
    }
}

pub(super) fn handle_player_events(
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &ae::PlayerClustersMut<'_>,
    combat: &mut ambition_gameplay_core::player::PlayerCombatState,
    blink_cam: &mut ambition_gameplay_core::player::PlayerBlinkCameraState,
    anim: &mut ambition_gameplay_core::player::PlayerAnimState,
    events: ae::FrameEvents,
    was_grounded: Option<bool>,
) {
    /// How long the wall-jump push-off pose holds after the WallJump op
    /// fires. Short enough to clear before the apex of the jump arc so
    /// the regular `Jump` row picks back up; long enough that the kick
    /// reads at typical playback rates.
    const WALL_JUMP_ANIM_HOLD_SECS: f32 = 0.18;
    let pos = clusters.kinematics.pos;
    let facing = clusters.kinematics.facing;
    let size = clusters.kinematics.size;
    let on_ground = clusters.ground.on_ground;
    for op in &events.operations {
        match op {
            ae::MovementOp::Jump => {
                sfx.write(SfxMessage::Jump { pos });
                vfx.write(VfxMessage::Dust { pos, facing });
            }
            ae::MovementOp::WallJump => {
                sfx.write(SfxMessage::Jump { pos });
                vfx.write(VfxMessage::Dust { pos, facing });
                // Arm the push-off pose. Held briefly so the kick
                // reads even after the regular jump arc takes over.
                anim.wall_jump_anim_timer = WALL_JUMP_ANIM_HOLD_SECS;
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
        blink_cam.blink_in_duration = ambition_gameplay_core::BLINK_IN_ANIM_TIME;
        blink_cam.blink_in_timer = blink_cam.blink_in_duration;
        blink_cam.blink_camera_from = blink.from;
        blink_cam.blink_camera_to = blink.to;
        vfx.write(VfxMessage::BlinkEffects {
            from: blink.from,
            to: blink.to,
            precision: blink.precision,
        });
    }
    if events.hazard || !events.operations.is_empty() {
        combat.flash_timer = 0.12;
    }
    if let Some(was_grounded) = was_grounded {
        if !was_grounded && on_ground {
            vfx.write(VfxMessage::Dust {
                pos: pos + ae::Vec2::new(0.0, size.y * 0.5),
                facing,
            });
        }
    }
}

pub(crate) mod attack;
pub use attack::*;

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
        use ambition_gameplay_core::combat::{
            attack_hitbox_from_view, attack_spec_from_view, AttackIntent, AttackView,
        };
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
