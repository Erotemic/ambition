//! Non-player-centric OOB recorder.
//!
//! The relativity-respecting counterpart to [`super::record_frame_system`]
//! (which records the rich, input-driven PLAYER feel timeline). This system
//! samples EVERY simulated body's kinematics each frame — player, boss,
//! enemy, NPC, all through the one shared [`ae::BodyKinematics`] component —
//! and auto-dumps the offender's recent trajectory the moment ANY character
//! leaves the world envelope. There is no privileged observer: the player is
//! just one body in the `bodies_q` iteration.
//!
//! The OOB predicate ([`super::detect_oob_from_kinematics`]) and the world
//! envelope are exactly the ones the player recorder uses, so a flying boss
//! that escapes its arena is caught by the same definition that catches a
//! player tunnelling a one-way platform.

use super::*;
use crate::actor::PlayerEntity;
use crate::combat::components::{ActorFaction, ActorIdentity};

fn body_kind(is_player: bool, faction: Option<&ActorFaction>) -> String {
    if is_player {
        return "player".into();
    }
    match faction {
        Some(ActorFaction::Boss) => "boss",
        Some(ActorFaction::Enemy) => "enemy",
        Some(ActorFaction::Npc) => "npc",
        Some(ActorFaction::Neutral) => "neutral",
        Some(ActorFaction::Player) => "player",
        None => "body",
    }
    .into()
}

/// Build one body's snapshot, running the shared OOB predicate against the
/// augmented world. Pure so the classification is unit-testable without a
/// Bevy `App`.
pub fn body_snapshot(
    actor_id: String,
    name: String,
    kind: String,
    kin: ae::BodyKinematics,
    world: &ae::World,
    margin: f32,
) -> BodyTraceSnapshot {
    let aabb = kin.aabb();
    let oob =
        detect_oob_from_kinematics(kin.pos, kin.vel, aabb, world, margin).map(|r| r.short_label());
    BodyTraceSnapshot {
        actor_id,
        name,
        kind,
        pos: kin.pos.into(),
        vel: kin.vel.into(),
        size: kin.size.into(),
        aabb: aabb.into(),
        facing: kin.facing,
        oob,
    }
}

/// Records one [`ActorTraceFrame`] per Update tick: a snapshot of every body
/// with a [`ae::BodyKinematics`], each classified for OOB against the same
/// augmented world the player tick uses. Runs in `SandboxSet::Trace` (after
/// `CoreSimulation`) so it captures resolved post-integration positions.
#[allow(clippy::too_many_arguments)]
pub fn record_actor_oob_frame_system(
    mut buffer: ResMut<ActorTraceBuffer>,
    boundary: Option<Res<ae::ConfirmedFrameBoundary>>,
    world_time: Res<ambition_time::WorldTime>,
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<RoomGeometry>,
    platform_set: Res<ambition_world::collision::MovingPlatformSet>,
    feature_ecs_overlay: Res<crate::features::FeatureEcsWorldOverlay>,
    rooms: Option<
        ambition_platformer_primitives::lifecycle::SessionWorldRef<crate::rooms::RoomSet>,
    >,
    mode: Res<State<ambition_platformer_primitives::schedule::GameMode>>,
    bodies_q: Query<(
        Entity,
        &ae::BodyKinematics,
        Option<&ActorIdentity>,
        Option<&ActorFaction>,
        Has<PlayerEntity>,
    )>,
) {
    let augmented_world = ambition_world::collision::world_with_sandbox_solids(
        &world.0,
        &platform_set.0,
        &feature_ecs_overlay,
    );
    // A flight recorder wants wall-clock timing (so a dump reads in real
    // seconds), plus the scaled dt so bullet-time / pause is visible in the
    // trace. `WorldTime` exposes both — no `Res<Time>` discipline exception.
    let real_dt = world_time.wall_dt();
    let sim_dt = world_time.sim_dt();
    let time_scale = if real_dt > 0.0 { sim_dt / real_dt } else { 0.0 };
    let active_area = rooms
        .as_ref()
        .map(|r| r.active_spec().id.clone())
        .unwrap_or_else(|| "<unknown>".into());
    let mode_label = format!("{:?}", mode.get());

    let mut bodies = Vec::new();
    for (entity, kin, identity, faction, is_player) in &bodies_q {
        let (id, name) = match identity {
            Some(idn) => (idn.id.clone(), idn.name.clone()),
            None if is_player => ("player".to_string(), "Player".to_string()),
            None => (format!("entity-{}", entity.index()), "<body>".to_string()),
        };
        bodies.push(body_snapshot(
            id,
            name,
            body_kind(is_player, faction),
            *kin,
            &augmented_world,
            OOB_MARGIN,
        ));
    }

    // The augmented world's solid geometry, so a dump is self-contained:
    // cross-referenced with a body's pre-anomaly trajectory it shows the exact
    // wall/floor it was jammed into before leaving bounds.
    let solids: Vec<CollisionTraceShape> = augmented_world
        .blocks
        .iter()
        .filter(|b| matches!(b.kind, ae::BlockKind::Solid))
        .take(64)
        .map(|b| CollisionTraceShape {
            kind: format!("{:?}", b.kind),
            name: b.name.clone(),
            aabb: b.aabb.into(),
            distance: 0.0,
        })
        .collect();

    let timeline = boundary.as_deref().copied();
    let frame = ActorTraceFrame {
        seq: buffer.sequence,
        tick: buffer.tick,
        sim_session: timeline.map(|boundary| boundary.session),
        sim_frame: timeline.map(|boundary| boundary.current),
        real_dt,
        sim_dt,
        time_scale,
        game_mode: mode_label,
        active_area,
        world_size: augmented_world.size.into(),
        world_spawn: augmented_world.spawn.into(),
        bodies,
        solids,
    };
    buffer.record(frame, timeline.map(|boundary| boundary.confirmed));
}

/// Flush a pending actor-trace dump to disk. Disk writes are unavailable on
/// wasm, so there we just clear the request.
#[cfg(not(target_arch = "wasm32"))]
pub fn flush_actor_dump(
    mut buffer: ResMut<ActorTraceBuffer>,
    policy: Res<ambition_gameplay_trace::TraceDumpPolicy>,
) {
    let Some(reason) = buffer.dump_request.take() else {
        return;
    };
    // See `flush_pending_dump`: consume the request even when suppressed.
    if !policy.allows(reason.is_automatic()) {
        buffer.last_dump_status = Some(format!(
            "skipped: automatic dumps are off (set {}=1 to enable)",
            ambition_gameplay_trace::AUTO_DUMP_ENV
        ));
        return;
    }
    let dir = default_dump_dir();
    match write_actor_dump(&buffer, &reason, &dir) {
        Ok(path) => {
            info!("actor OOB trace dumped: {}", path.display());
            buffer.last_dump_path = Some(path.display().to_string());
            buffer.last_dump_status = Some("ok".into());
        }
        Err(err) => {
            warn!("actor OOB trace dump failed: {err}");
            buffer.last_dump_status = Some(format!("error: {err}"));
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn flush_actor_dump(mut buffer: ResMut<ActorTraceBuffer>) {
    buffer.dump_request = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world_960x768() -> ae::World {
        ae::World::new("arena", ae::Vec2::new(960.0, 768.0), ae::Vec2::ZERO, vec![])
    }

    fn kin(pos: ae::Vec2) -> ae::BodyKinematics {
        ae::BodyKinematics {
            pos,
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(40.0, 40.0),
            facing: 1.0,
        }
    }

    #[test]
    fn body_inside_room_is_not_oob() {
        let snap = body_snapshot(
            "boss".into(),
            "Mockingbird".into(),
            "boss".into(),
            kin(ae::Vec2::new(430.0, 400.0)),
            &world_960x768(),
            OOB_MARGIN,
        );
        assert!(snap.oob.is_none(), "a body mid-arena is in bounds");
    }

    #[test]
    fn body_far_above_room_is_flagged_oob() {
        // Far above the 768-tall room, well past the 96px margin — the exact
        // "boss hovering above the arena" symptom this tooling is built for.
        let snap = body_snapshot(
            "boss".into(),
            "Mockingbird".into(),
            "boss".into(),
            kin(ae::Vec2::new(430.0, -400.0)),
            &world_960x768(),
            OOB_MARGIN,
        );
        let reason = snap.oob.expect("a body far outside the room must be OOB");
        assert!(
            reason.contains("envelope"),
            "expected an out-of-envelope reason, got {reason:?}"
        );
    }
}
