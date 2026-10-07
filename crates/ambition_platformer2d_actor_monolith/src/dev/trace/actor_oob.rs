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
    last_step: Option<&ae::SweepSample>,
    world: &ae::World,
    margin: f32,
) -> BodyTraceSnapshot {
    // The box the body has: turned as its last step turned it.
    let aabb = kin.collision_box(last_step);
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
/// not recorded. Runs in `Platformer2dSimulationPhase::Trace` (after
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
        // The record of the body's last step: it has the DOWN of the body.
        Option<&ae::SweepSample>,
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
    for (entity, kin, last_step, identity, faction, is_player) in &bodies_q {
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
        let mut snapshot =
            body_snapshot(id, name, body_kind(is_player, faction), *kin, last_step, world, OOB_MARGIN);
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
            None,
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
            None,
            &world_960x768(),
            OOB_MARGIN,
        );
        let reason = snap.oob.expect("a body far outside the room must be OOB");
        assert!(
            reason.contains("envelope"),
            "expected an out-of-envelope reason, got {reason:?}"
        );
    }

    /// A 24x40 body at the centre of the arena, with one thin solid at
    /// `offset` from its centre. `down` is the DOWN of its last step.
    fn beside_a_solid(offset: ae::Vec2, down: ae::Vec2) -> Option<String> {
        let at = ae::Vec2::new(480.0, 384.0);
        let body = ae::BodyKinematics {
            pos: at,
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(24.0, 40.0),
            facing: 1.0,
        };
        let solid = ae::Aabb::new(at + offset, ae::Vec2::splat(2.0));
        let world = ae::World::new(
            "arena",
            ae::Vec2::new(960.0, 768.0),
            ae::Vec2::ZERO,
            vec![ae::Block::solid("thin", solid.min, solid.max - solid.min)],
        );
        let record = ae::SweepSample::at_rest(body, down);
        body_snapshot(
            "body".into(),
            "Body".into(),
            "body".into(),
            body,
            Some(&record),
            &world,
            OOB_MARGIN,
        )
        .oob
    }

    /// 16 from the centre on world y: inside the half of a body that stands
    /// (20), past the half of a body that lies along sideways gravity (12).
    const BESIDE_ON_Y: ae::Vec2 = ae::Vec2::new(0.0, 16.0);
    /// 16 from the centre on world x: past the half of a body that stands
    /// (12), inside the half of a body that lies along sideways gravity (20).
    const BESIDE_ON_X: ae::Vec2 = ae::Vec2::new(16.0, 0.0);
    const DOWN_Y: ae::Vec2 = ae::Vec2::new(0.0, 1.0);
    const DOWN_X: ae::Vec2 = ae::Vec2::new(1.0, 0.0);

    /// The control of the two arms below: a body that stands in normal
    /// gravity is 20 deep on y and 12 on x.
    #[test]
    fn a_standing_body_is_inside_the_solid_its_box_touches() {
        assert!(beside_a_solid(BESIDE_ON_Y, DOWN_Y).is_some_and(|reason| reason.contains("solid")));
        assert_eq!(beside_a_solid(BESIDE_ON_X, DOWN_Y), None);
    }

    /// The false alarm: a body that lies along sideways gravity is 12 deep on
    /// world y, and a solid 16 away on y does not touch it.
    #[test]
    fn a_body_that_lies_along_sideways_gravity_is_not_inside_a_solid_beside_it() {
        assert_eq!(
            beside_a_solid(BESIDE_ON_Y, DOWN_X),
            None,
            "the solid is clear of the box the body has"
        );
    }

    /// And the alarm that was missed: it is 20 deep on world x.
    #[test]
    fn a_body_that_lies_along_sideways_gravity_is_inside_the_solid_its_own_box_touches() {
        let reason = beside_a_solid(BESIDE_ON_X, DOWN_X).expect("the solid is inside the box the body has");
        assert!(reason.contains("solid"), "{reason:?}");
    }
}
