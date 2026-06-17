#![allow(unused_imports)]
use ambition_gameplay_core::engine_core::AabbExt;

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
pub(super) struct RoomClock<'w> {
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

pub(super) fn death_respawn_player(
    world: &ae::World,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    died: &mut MessageWriter<PlayerDiedMessage>,
    clusters: &mut ae::PlayerClustersMut<'_>,
    sim_state: &mut ambition_gameplay_core::SandboxSimState,
    clock: &mut ambition_gameplay_core::time::clock_state::ClockState,
    safety: &mut ambition_gameplay_core::player::PlayerSafetyState,
    banner: &mut features::GameplayBanner,
    player_health: Option<&mut ambition_gameplay_core::player::PlayerHealth>,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    from: ae::Vec2,
    anim: &mut ambition_gameplay_core::player::PlayerAnimState,
    combat: &mut ambition_gameplay_core::player::PlayerCombatState,
) {
    let to = world.spawn;
    ae::reset_player_clusters(clusters, world.spawn);
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning,
    );
    clusters.mana.meter.refill_full();
    safety.last_safe_pos = world.spawn;
    clock.time_scale = 1.0;
    sim_state.room_transition_cooldown = 0.0;
    anim.reset();
    combat.reset();
    if let Some(health) = player_health {
        health.reset();
    }
    combat.damage_invuln_timer = feel.hazard_respawn_invulnerability_time;
    combat.flash_timer = feel.reset_flash_time.max(0.35);
    banner.show("PLAYER DOWN: respawned at room start with full HP", 2.4);
    sfx.write(SfxMessage::Death { pos: from });
    vfx.write(VfxMessage::ResetEffects { from, to });
    died.write(PlayerDiedMessage { pos: from });
}

/// Whether a held shield blocks a hit coming from `hit_x`: you can only guard the
/// side you face (a hit from behind still lands). A facing of exactly 0 (neutral)
/// guards either side. Pure so the directional rule is unit-tested directly.
pub fn shield_blocks_hit(shield_held: bool, facing: f32, player_x: f32, hit_x: f32) -> bool {
    if !shield_held {
        return false;
    }
    if facing == 0.0 {
        return true;
    }
    // Same sign => the hit is on the side the player is facing.
    (hit_x - player_x).signum() == facing.signum()
}

pub(super) fn handle_player_damage_events(
    world: &ae::World,
    shield_held: bool,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    died: &mut MessageWriter<PlayerDiedMessage>,
    clusters: &mut ae::PlayerClustersMut<'_>,
    sim_state: &mut ambition_gameplay_core::SandboxSimState,
    clock: &mut ambition_gameplay_core::time::clock_state::ClockState,
    safety: &mut ambition_gameplay_core::player::PlayerSafetyState,
    banner: &mut features::GameplayBanner,
    mut player_health: Option<&mut ambition_gameplay_core::player::PlayerHealth>,
    damage_events: &[features::HitEvent],
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    difficulty_multiplier: f32,
    anim: &mut ambition_gameplay_core::player::PlayerAnimState,
    combat: &mut ambition_gameplay_core::player::PlayerCombatState,
) {
    let Some(mut damage) = damage_events.first().cloned() else {
        return;
    };
    // Invincibility (debug toggle): drop the damage event entirely
    // before any state mutates so testing systems that consume HP
    // (boss phases, encounter pacing, music) can run uninterrupted.
    if clusters.offense.invincible {
        return;
    }
    // Shield block: a held shield fully negates a hit coming from the side the
    // player faces (you can't guard your back). Costs nothing but a short guard
    // i-frame; a defensive verb to complement the offensive/movement abilities.
    let guard_impact = damage
        .knockback
        .as_ref()
        .map(|k| k.impact_pos)
        .unwrap_or_else(|| damage.volume.center());
    if shield_blocks_hit(
        shield_held,
        clusters.kinematics.facing,
        clusters.kinematics.pos.x,
        guard_impact.x,
    ) {
        sfx.write(SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_ROCK_HIT,
            pos: clusters.kinematics.pos,
        });
        combat.damage_invuln_timer = combat.damage_invuln_timer.max(0.12);
        banner.show("blocked", 1.0);
        return;
    }
    // Difficulty / assist scaling. Easy halves incoming damage, hard
    // doubles it; the menu setting also exposes a fine-grained
    // gameplay damage multiplier. The minimum is one HP so a damage
    // event always lands somewhere.
    let scaled = ((damage.damage as f32) * difficulty_multiplier).round() as i32;
    damage.damage = scaled.max(1);
    let died_from_damage = if let Some(health) = player_health.as_deref_mut() {
        health.damage(damage.damage)
    } else {
        false
    };
    let impact_pos = damage
        .knockback
        .as_ref()
        .map(|k| k.impact_pos)
        .unwrap_or_else(|| damage.volume.center());
    if died_from_damage {
        death_respawn_player(
            world,
            sfx,
            vfx,
            died,
            clusters,
            sim_state,
            clock,
            safety,
            banner,
            player_health,
            tuning,
            feel,
            impact_pos,
            anim,
            combat,
        );
        return;
    }
    match damage.mode {
        features::HitMode::SafeRespawn => {
            safe_respawn_player(
                sfx, vfx, clusters, clock, safety, combat, tuning, feel, impact_pos,
            );
        }
        features::HitMode::Knockback => {
            // Getting hit knocks you off a ledge grab — you fall with the
            // knockback instead of hanging there immune.
            clusters.ledge.knock_off_on_hit();
            apply_player_knockback(sfx, vfx, clusters, combat, tuning, feel, &damage);
        }
    }
}

pub(super) fn safe_respawn_player(
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::PlayerClustersMut<'_>,
    clock: &mut ambition_gameplay_core::time::clock_state::ClockState,
    safety: &ambition_gameplay_core::player::PlayerSafetyState,
    combat: &mut ambition_gameplay_core::player::PlayerCombatState,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    from: ae::Vec2,
) {
    let to = safety.last_safe_pos;
    ae::reset_player_clusters(clusters, to);
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning,
    );
    combat.damage_invuln_timer = feel.hazard_respawn_invulnerability_time;
    combat.hitstun_timer = 0.0;
    combat.hitstop_timer = 0.0;
    combat.flash_timer = feel.reset_flash_time;
    clock.time_scale = 1.0;
    sfx.write(SfxMessage::Reset { pos: to });
    vfx.write(VfxMessage::ResetEffects { from, to });
}

pub(super) fn apply_player_knockback(
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::PlayerClustersMut<'_>,
    combat: &mut ambition_gameplay_core::player::PlayerCombatState,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    damage: &features::HitEvent,
) {
    let boss_hit = matches!(
        damage.source,
        features::HitSource::BossBody | features::HitSource::BossAttack
    );
    let knockback = damage.knockback.as_ref();
    let impact_pos = knockback
        .map(|k| k.impact_pos)
        .unwrap_or_else(|| damage.volume.center());
    let knockback_dir = knockback.map(|k| k.dir).unwrap_or(0.0);
    let dir = if knockback_dir.abs() <= 0.001 {
        -clusters.kinematics.facing
    } else {
        knockback_dir.signum()
    };
    let strength = knockback.map(|k| k.strength.max(0.0)).unwrap_or(0.0);
    let knock_x = if boss_hit {
        feel.boss_knockback_x
    } else {
        feel.enemy_knockback_x
    };
    let knock_y = if boss_hit {
        feel.boss_knockback_y
    } else {
        feel.enemy_knockback_y
    };
    clusters.kinematics.vel.x = dir * knock_x * strength;
    clusters.kinematics.vel.y = -knock_y * strength;
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning,
    );
    combat.hitstun_timer = if boss_hit {
        feel.boss_hitstun_time
    } else {
        feel.enemy_hitstun_time
    } * strength.max(0.35);
    combat.damage_invuln_timer = feel.knockback_invulnerability_time;
    combat.hitstop_timer = feel.player_damage_hitstop_time;
    combat.flash_timer = 0.20;
    sfx.write(SfxMessage::Hit { pos: impact_pos });
    vfx.write(VfxMessage::Impact { pos: impact_pos });
}

mod attack;
pub use attack::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shield_blocks_only_hits_from_the_faced_side() {
        // Player at x=100 facing right (+1).
        assert!(
            shield_blocks_hit(true, 1.0, 100.0, 150.0),
            "guards a hit from the right"
        );
        assert!(
            !shield_blocks_hit(true, 1.0, 100.0, 50.0),
            "a hit from behind (left) lands"
        );
        // Facing left (-1) flips it.
        assert!(
            shield_blocks_hit(true, -1.0, 100.0, 50.0),
            "guards a hit from the left"
        );
        assert!(
            !shield_blocks_hit(true, -1.0, 100.0, 150.0),
            "a hit from behind (right) lands"
        );
        // No shield held -> never blocks; neutral facing -> guards either side.
        assert!(
            !shield_blocks_hit(false, 1.0, 100.0, 150.0),
            "no shield, no block"
        );
        assert!(
            shield_blocks_hit(true, 0.0, 100.0, 50.0),
            "neutral facing guards either side"
        );
    }

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
