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
use ambition_combat::components::ActorIdentity;
use ambition_characters::actor::ActorFaction;
use ambition_gameplay_trace::default_dump_dir;
use ambition_gameplay_trace::write_actor_dump;
use ambition_gameplay_trace::ActorTraceBuffer;
use ambition_gameplay_trace::ActorTraceFrame;
use ambition_gameplay_trace::BodyTraceSnapshot;
use ambition_gameplay_trace::CollisionTraceShape;
use ambition_gameplay_trace::RoomTraceSnapshot;
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;

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
        room: None,
        oob,
    }
}

/// A live room's solid blocks, the dump's self-contained geometry.
fn solid_shapes(world: &ae::World) -> Vec<CollisionTraceShape> {
    world
        .blocks
        .iter()
        .filter(|b| matches!(b.kind, ae::BlockKind::Solid))
        .take(64)
        .map(|b| CollisionTraceShape {
            // ⛔ NOT `format!("{:?}", b.kind)`. The filter one line up admits
            // ONLY `Solid`, so that formatted a compile-time constant into a
            // fresh `String` up to 64 times per frame, forever, to write the
            // same six characters. The filter is the spec; this follows it.
            kind: "Solid".to_string(),
            name: b.name.clone(),
            aabb: b.aabb.into(),
            distance: 0.0,
        })
        .collect()
}

/// Records one [`ActorTraceFrame`] per Update tick: a snapshot of every body
/// with a [`ae::BodyKinematics`], each classified for OOB against the same
/// augmented world the player tick uses, of the body's own live room (OW1).
/// The frame holds each live room a body was in: its area, envelope and
/// solids. A body whose room cannot be told (two rooms live, no stamp) is
/// not recorded. Runs in `Platformer2dSimulationPhaseMonolith::Trace` (after
/// `CoreSimulation`) so it captures resolved post-integration positions.
#[allow(clippy::too_many_arguments)]
pub fn record_actor_oob_frame_system(
    mut buffer: ResMut<ActorTraceBuffer>,
    boundary: Option<Res<ae::ConfirmedFrameBoundary>>,
    world_time: Res<ambition_time::WorldTime>,
    // The composed collision read-API rather than its three ingredients — a
    // trace must see exactly the world the simulation collided against.
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    mode: Res<State<ambition_platformer2d_shared_tangle::schedule::GameMode>>,
    bodies_q: Query<(
        Entity,
        &ae::BodyKinematics,
        Option<&ActorIdentity>,
        Option<&ActorFaction>,
        Has<PlayerEntity>,
    )>,
) {
    // A flight recorder wants wall-clock timing (so a dump reads in real
    // seconds), plus the scaled dt so bullet-time / pause is visible in the
    // trace. `WorldTime` exposes both — no `Res<Time>` discipline exception.
    let real_dt = world_time.wall_dt();
    let sim_dt = world_time.sim_dt();
    let time_scale = world_time.time_scale();
    let mode_label = format!("{:?}", mode.get());

    // Each live room's composed world, built once: (room, area, world).
    let mut worlds: Vec<(Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>, String, Option<std::borrow::Cow<'_, ae::World>>)> = Vec::new();
    let mut bodies = Vec::new();
    for (entity, kin, identity, faction, is_player) in &bodies_q {
        let room = rooms.live().of(entity);
        let index = match worlds.iter().position(|(seen, ..)| *seen == room) {
            Some(index) => index,
            None => {
                let world = collision
                    .room(room.map(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance).as_ref())
                    .and_then(|room| room.solids());
                let area = rooms
                    .definition_named(room)
                    .map(|definition| rooms.rooms().spec(definition).id.clone())
                    .unwrap_or_else(|| "<unknown>".into());
                worlds.push((room, area, world));
                worlds.len() - 1
            }
        };
        let Some(world) = worlds[index].2.as_deref() else {
            continue;
        };
        let (id, name) = match identity {
            Some(idn) => (idn.id.clone(), idn.name.clone()),
            None if is_player => ("player".to_string(), "Player".to_string()),
            None => (format!("entity-{}", entity.index()), "<body>".to_string()),
        };
        let mut snapshot = body_snapshot(id, name, body_kind(is_player, faction), *kin, world, OOB_MARGIN);
        snapshot.room = room.map(|room| room.ordinal());
        bodies.push(snapshot);
    }
    // No live room at all (nothing loaded): nothing to record, as before.
    if bodies.is_empty() && collision.room(None).is_none() && worlds.is_empty() {
        return;
    }

    // Each room's world envelope and solid geometry, so a dump is
    // self-contained: cross-referenced with a body's pre-anomaly trajectory it
    // shows the exact wall/floor it was jammed into before leaving bounds.
    worlds.sort_by_key(|(room, ..)| *room);
    let rooms: Vec<RoomTraceSnapshot> = worlds
        .iter()
        .filter_map(|(room, area, world)| {
            let world = world.as_deref()?;
            Some(RoomTraceSnapshot {
                room: room.map(|room| room.ordinal()),
                area: area.clone(),
                world_size: world.size.into(),
                world_spawn: world.spawn.into(),
                solids: solid_shapes(world),
            })
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
        bodies,
        rooms,
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
