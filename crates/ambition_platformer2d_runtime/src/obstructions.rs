//! Crushing C1 (crushing-and-obstruction-response.md): each tick's
//! obstruction facts, one per conflict a body's step reported, in identity
//! order, for the responses (C3 movers, C4 bodies) and diagnostics to read.

use bevy::prelude::*;

use ambition_platformer2d_actor::body::BodyFrameOutput;
use ambition_platformer2d_core::collision_semantics::Obstruction;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// One body's obstruction this tick.
#[derive(Clone, Debug)]
pub struct BodyObstruction {
    pub body: Entity,
    pub obstruction: Obstruction,
}

/// This tick's obstructions, rebuilt every tick after bodies step: ordered by
/// the bodies' `SimId` (bodies without one last), then in the order each
/// body's step reported them. Derived, not state: a rewind rebuilds it.
#[derive(Resource, Clone, Debug, Default)]
pub struct TickObstructions(pub Vec<BodyObstruction>);

impl TickObstructions {
    pub fn of(&self, body: Entity) -> impl Iterator<Item = &Obstruction> {
        self.0.iter().filter(move |row| row.body == body).map(|row| &row.obstruction)
    }
}

pub(crate) fn register_rollback_state(registrar: &mut impl ambition_platformer2d_core::snapshot::RollbackRegistrar) {
    registrar.declare_rollback_derived_resource::<TickObstructions>(
        "ambition_platformer2d_runtime", "derived.tick_obstructions",
        "Cleared and rebuilt every tick from the bodies' step output (crushing C1).");
    registrar.rollback_component_clone::<Squash>("ambition_platformer2d_runtime", "crush.squash");
}

pub use ambition_platformer2d_core::crush::Squash;

/// The game's one-way rule (`CrushPolicy::game.one_way_yields`) on every
/// live room's world, where the step reads it. Written only to a room whose
/// world disagrees, so an unchanged policy changes no geometry.
pub(crate) fn apply_one_way_rule(
    policy: Option<Res<ambition_platformer2d_core::crush::CrushPolicy>>,
    mut rooms: Query<&mut ambition_platformer2d_core::RoomGeometry>,
) {
    let yields = policy.as_deref().map_or(true, |policy| policy.game.one_way_yields);
    for mut room in &mut rooms {
        if room.0.one_way_yields != yields {
            room.0.one_way_yields = yields;
        }
    }
}

/// Collect the conflicts every body's step reported into this tick's list.
pub(crate) fn publish_tick_obstructions(
    bodies: Query<(Entity, &BodyFrameOutput, Option<&SimId>)>,
    mut published: ResMut<TickObstructions>,
) {
    published.0.clear();
    let mut rows: Vec<(Option<&SimId>, Entity, &BodyFrameOutput)> = bodies.iter()
        .filter(|(_, output, _)| output.reset.is_none() && !output.events.constraint_conflicts.is_empty())
        .map(|(entity, output, id)| (id, entity, output))
        .collect();
    // Identity order: by SimId, those without one after, by entity (only a
    // body no snapshot carries has none).
    rows.sort_by(|(a, ea, _), (b, eb, _)| match (a, b) {
        (Some(a), Some(b)) => a.cmp(b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => ea.cmp(eb),
    });
    for (_, body, output) in rows {
        published.0.extend(output.events.constraint_conflicts.iter()
            .map(|conflict| BodyObstruction { body, obstruction: conflict.obstruction() }));
    }
}

/// Crushing C4: each pinched body answers its deepest obstruction this tick
/// by its resistance (the body's own, else the one the policy names its
/// character with, else the game's): `Damage` deals
/// `per_px` per pixel of overconstraint up to `per_tick_cap` through the
/// ordinary hit path; `Destroy` deals a lethal hit there; `Displace` is
/// ejected by the deterministic fallback ([`eject_across`]); `Rigid`, the
/// default, takes nothing, so the default changes nothing. Bodies are
/// answered in the list's identity order.
#[allow(clippy::type_complexity)]
pub(crate) fn respond_to_obstructions(
    published: Res<TickObstructions>,
    policy: Option<Res<ambition_platformer2d_core::crush::CrushPolicy>>,
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    mut bodies: Query<(&mut ambition_platformer2d_core::BodyKinematics, Option<&ambition_platformer2d_core::crush::OwnResistance>,
        Option<&mut Squash>, Option<&ambition_characters::actor::CharacterForm>)>,
    mut hits: MessageWriter<ambition_combat::events::HitEvent>,
    mut commands: Commands,
) {
    use ambition_platformer2d_core::crush::{CompressAxes, CrushPolicy, FailureResponse};
    use ambition_platformer2d_core::collision_semantics::Axis;
    if published.0.is_empty() { return; }
    let default_policy = CrushPolicy::default();
    let policy = policy.as_deref().unwrap_or(&default_policy);
    let mut answered: Vec<Entity> = Vec::new();
    for row in &published.0 {
        if answered.contains(&row.body) { continue; }
        answered.push(row.body);
        // The deepest of its obstructions this tick.
        let deepest = published.of(row.body)
            .max_by(|a, b| a.overconstraint.total_cmp(&b.overconstraint))
            .expect("the row's own obstruction");
        let Ok((mut kin, own, squash, form)) = bodies.get_mut(row.body) else { continue };
        // The policy names a body by the catalog character it wears.
        let resistance = policy.resistance(form.map_or("", |form| form.id()), own.map(|own| own.0));
        let mut over = deepest.overconstraint;
        let room_of = live.of(row.body);
        let hit = |kin: &ambition_platformer2d_core::BodyKinematics, damage: i32| ambition_combat::events::HitEvent {
            origin: None, properties: Default::default(), contact: None, pieces: Vec::new(),
            volume: kin.aabb().into(), damage,
            // A crush is the world's harm, as a hazard's is.
            source: ambition_combat::events::HitSource::Hazard,
            attacker: None, room: room_of,
            target: ambition_combat::events::HitTarget::Body(row.body),
            mode: ambition_combat::events::HitMode::Knockback, knockback: None, status: None,
            ignored_targets: Vec::new(), strike_sfx: None, attacker_move_instance: None,
        };
        // Compression first: the body squashes along the crush axis down to
        // `min_scale` of its rest size, and only what is left reaches its
        // failure response. A body that may squash only along its own up
        // does not squash across.
        if let Some(compress) = resistance.compress.filter(|compress|
            compress.axes == CompressAxes::Crush || deepest.axis == Axis::Y)
        {
            let i = match deepest.axis { Axis::X => 0, Axis::Y => 1 };
            let rest = squash.as_deref().map_or(kin.canonical_size, |squash| squash.rest);
            let scale = kin.material.scale().max(f32::EPSILON);
            let room = (kin.canonical_size[i] - compress.min_scale * rest[i]).max(0.0) * scale;
            let take = over.min(room);
            if take > 0.0 {
                kin.canonical_size[i] -= take / scale;
                match squash {
                    Some(mut squash) => squash.recover_per_s = compress.recover_per_s,
                    None => { commands.entity(row.body).insert(Squash { rest, recover_per_s: compress.recover_per_s }); }
                }
            }
            over -= take;
            if over <= 1.0e-3 { continue; }
            if !compress.survivable {
                // Flattened past what it survives.
                let lethal = hit(&kin, i32::MAX / 4);
                hits.write(lethal);
                continue;
            }
        }
        match resistance.failure {
            FailureResponse::Rigid => {}
            FailureResponse::Damage { per_px, per_tick_cap } => {
                let damage = (per_px * over).min(per_tick_cap).ceil() as i32;
                if damage > 0 { hits.write(hit(&kin, damage)); }
            }
            FailureResponse::Destroy => { hits.write(hit(&kin, i32::MAX / 4)); }
            FailureResponse::Displace => {
                let room = live.of(row.body).map(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance);
                let Some(world) = collision.room(room.as_ref()).and_then(|room| room.solids()) else { continue };
                let solids = ambition_platformer2d_world::platforms::obstruction::full_solids(&world, None);
                if let Some(offset) = eject_across(kin.aabb(), deepest.axis, &solids) {
                    kin.pos += offset;
                }
            }
        }
    }
}

/// A squashed body recovers toward its rest size at its rate once nothing
/// pinches it, only as far as it has room: it grows about its centre, or
/// from whichever side is blocked, and holds when neither fits. Back at
/// rest, the squash is over.
pub(crate) fn recover_squashed_bodies(
    time: Option<Res<ambition_time::WorldTime>>,
    published: Res<TickObstructions>,
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    mut bodies: Query<(Entity, &mut ambition_platformer2d_core::BodyKinematics, &Squash)>,
    mut commands: Commands,
) {
    use ambition_platformer2d_core::{AabbExt, Vec2};
    let dt = time.as_deref().map_or(0.0, |time| time.sim_dt());
    if dt <= 0.0 { return; }
    let mut squashed: Vec<_> = bodies.iter_mut().collect();
    squashed.sort_by_key(|(entity, ..)| *entity);
    for (entity, mut kin, squash) in squashed {
        if published.of(entity).next().is_some() { continue; }
        let step = squash.rest * squash.recover_per_s * dt;
        let grown = kin.canonical_size.min(squash.rest).max(Vec2::ZERO) + step;
        let grown = grown.min(squash.rest);
        let room = live.of(entity).map(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance);
        let Some(world) = collision.room(room.as_ref()).and_then(|room| room.solids()) else { continue };
        let solids = ambition_platformer2d_world::platforms::obstruction::full_solids(&world, None);
        let scale = kin.material.scale();
        let half_growth = (grown - kin.canonical_size) * scale * 0.5;
        let fits = |pos: Vec2| {
            let mut probe = *kin;
            probe.canonical_size = grown;
            probe.pos = pos;
            !solids.iter().any(|solid| probe.aabb().strict_intersects(*solid))
        };
        let Some(pos) = [kin.pos, kin.pos - half_growth, kin.pos + half_growth].into_iter().find(|pos| fits(*pos)) else { continue };
        kin.pos = pos;
        kin.canonical_size = grown;
        if grown == squash.rest { commands.entity(entity).remove::<Squash>(); }
    }
}

/// The deterministic fallback: the nearest offset across `axis` (the pinch's
/// axis) at which `body` overlaps none of `solids`, searched in 1 px steps up
/// to the body's extent across, the negative side before the positive at
/// each distance. `None` when there is none: the body is held.
pub fn eject_across(
    body: ambition_platformer2d_core::Aabb,
    axis: ambition_platformer2d_core::collision_semantics::Axis,
    solids: &[ambition_platformer2d_core::Aabb],
) -> Option<ambition_platformer2d_core::Vec2> {
    use ambition_platformer2d_core::{collision_semantics::Axis, AabbExt, Vec2};
    let (across, extent) = match axis {
        Axis::Y => (Vec2::X, body.width()),
        Axis::X => (Vec2::Y, body.height()),
    };
    let free = |offset: Vec2| !solids.iter().any(|solid| body.translated(offset).strict_intersects(*solid));
    (1..=extent.ceil() as i32).flat_map(|d| [-d, d])
        .map(|d| across * d as f32)
        .find(|offset| free(*offset))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::collision_semantics::{Axis, AxisConstraintConflict, ContactSource};

    fn conflict(min_motion: f32) -> AxisConstraintConflict {
        let block = |name: &str| ContactSource::Block { kind: ambition_platformer2d_core::BlockKind::Solid,
            id: ambition_platformer2d_core::GeoId::placement(ambition_platformer2d_core::PlacementId::new(name), 0) };
        AxisConstraintConflict { axis: Axis::Y, min_center: 10.0, max_center: 6.0, min_source: block("floor"),
            max_source: block("press"), min_motion, max_motion: 0.0 }
    }

    /// The list is in identity order whatever order the bodies spawned in,
    /// and a body with no conflict publishes nothing.
    #[test]
    fn obstructions_are_published_in_identity_order() {
        let mut app = App::new();
        app.init_resource::<TickObstructions>();
        app.add_systems(Update, publish_tick_obstructions);
        let with = |motion: f32| {
            let mut output = BodyFrameOutput::default();
            output.events.constraint_conflicts.push(conflict(motion));
            output
        };
        let late = app.world_mut().spawn((with(4.0), SimId::placement("b"))).id();
        let early = app.world_mut().spawn((with(0.0), SimId::placement("a"))).id();
        app.world_mut().spawn((BodyFrameOutput::default(), SimId::placement("c")));
        app.update();
        let rows = &app.world().resource::<TickObstructions>().0;
        assert_eq!(rows.iter().map(|row| row.body).collect::<Vec<_>>(), vec![early, late]);
        assert!(rows[0].obstruction.driver.is_none(), "a still pinch has no driver");
        assert!(rows[1].obstruction.driver.is_some(), "the closing claimant drives");
        app.update();
        assert_eq!(app.world().resource::<TickObstructions>().0.len(), 2, "rebuilt, not accumulated");
    }

    fn obstruction(over: f32) -> Obstruction {
        let block = ContactSource::Block { kind: ambition_platformer2d_core::BlockKind::Solid,
            id: ambition_platformer2d_core::GeoId::anon() };
        Obstruction { axis: Axis::Y, overconstraint: over, driver: Some((block.clone(), 6.0)), other: block }
    }

    fn respond_app(solids: Vec<ambition_platformer2d_core::Block>) -> App {
        let mut app = App::new();
        app.add_message::<ambition_combat::events::HitEvent>();
        app.init_resource::<TickObstructions>();
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(),
            ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new("crush",
                ambition_platformer2d_core::Vec2::new(1600.0, 900.0), ambition_platformer2d_core::Vec2::ZERO, solids)));
        app.add_systems(Update, respond_to_obstructions);
        app
    }

    fn body(app: &mut App, resistance: Option<ambition_platformer2d_core::crush::BodyResistance>) -> Entity {
        let kin = ambition_platformer2d_core::BodyKinematics { pos: ambition_platformer2d_core::Vec2::new(400.0, 276.0),
            canonical_size: ambition_platformer2d_core::Vec2::new(24.0, 48.0), ..Default::default() };
        let mut entity = app.world_mut().spawn(kin);
        if let Some(resistance) = resistance { entity.insert(ambition_platformer2d_core::crush::OwnResistance(resistance)); }
        entity.id()
    }

    fn hits(app: &mut App) -> Vec<i32> {
        app.world_mut().resource_mut::<Messages<ambition_combat::events::HitEvent>>().drain().map(|hit| hit.damage).collect()
    }

    /// C4: a body whose failure is damage takes `per_px` per pixel of its
    /// deepest pinch, capped per tick, through the hit path; a rigid body
    /// (the default) takes nothing and stays where it is.
    #[test]
    fn a_pinched_body_takes_damage_per_pixel_up_to_its_cap() {
        use ambition_platformer2d_core::crush::{BodyResistance, FailureResponse};
        let mut app = respond_app(Vec::new());
        let soft = body(&mut app, Some(BodyResistance { compress: None, failure: FailureResponse::Damage { per_px: 0.5, per_tick_cap: 3.0 } }));
        let rigid = body(&mut app, None);
        let before = *app.world().get::<ambition_platformer2d_core::BodyKinematics>(rigid).unwrap();
        app.world_mut().resource_mut::<TickObstructions>().0 = vec![
            BodyObstruction { body: soft, obstruction: obstruction(4.0) },
            BodyObstruction { body: soft, obstruction: obstruction(2.0) },
            BodyObstruction { body: rigid, obstruction: obstruction(4.0) },
        ];
        app.update();
        assert_eq!(hits(&mut app), vec![2], "half a point per pixel of the deepest pinch");
        app.world_mut().resource_mut::<TickObstructions>().0 = vec![BodyObstruction { body: soft, obstruction: obstruction(20.0) }];
        app.update();
        assert_eq!(hits(&mut app), vec![3], "capped per tick");
        assert_eq!(*app.world().get::<ambition_platformer2d_core::BodyKinematics>(rigid).unwrap(), before, "a rigid body is held");
    }

    /// C4: the policy names a body by the catalog character it wears; a
    /// body's own resistance still outranks it, and a character the policy
    /// does not name takes the game's (rigid) default.
    #[test]
    fn the_policy_names_a_body_by_the_character_it_wears() {
        use ambition_platformer2d_core::crush::{BodyResistance, CrushPolicy, FailureResponse};
        let mut app = respond_app(Vec::new());
        let soft = BodyResistance { compress: None, failure: FailureResponse::Damage { per_px: 1.0, per_tick_cap: 10.0 } };
        app.insert_resource(CrushPolicy { bodies: [("slime".to_string(), soft)].into(), ..Default::default() });
        let slime = body(&mut app, None);
        let armoured = body(&mut app, Some(BodyResistance::RIGID));
        let knight = body(&mut app, None);
        for (entity, character) in [(slime, "slime"), (armoured, "slime"), (knight, "knight")] {
            app.world_mut().entity_mut(entity).insert(ambition_characters::actor::CharacterForm::new(character));
        }
        app.world_mut().resource_mut::<TickObstructions>().0 = [slime, armoured, knight].into_iter()
            .map(|body| BodyObstruction { body, obstruction: obstruction(4.0) }).collect();
        app.update();
        let struck: Vec<_> = app.world_mut().resource_mut::<Messages<ambition_combat::events::HitEvent>>().drain()
            .map(|hit| (hit.target, hit.damage)).collect();
        assert_eq!(struck, vec![(ambition_combat::events::HitTarget::Body(slime), 4)],
            "only the slime by name; its own rigid resistance spares the second");
    }

    /// C4: the game's one-way rule reaches every live room's world, where the
    /// step reads it.
    #[test]
    fn the_game_one_way_rule_reaches_the_room() {
        use ambition_platformer2d_core::crush::{CrushPolicy, GameCrushPolicy};
        let mut app = respond_app(Vec::new());
        app.add_systems(Update, apply_one_way_rule);
        let yields = |app: &mut App| app.world_mut().query::<&ambition_platformer2d_core::RoomGeometry>()
            .single(app.world()).unwrap().0.one_way_yields;
        app.update();
        assert!(yields(&mut app), "no policy: one-ways yield");
        app.insert_resource(CrushPolicy { game: GameCrushPolicy { one_way_yields: false, ..Default::default() }, ..Default::default() });
        app.update();
        assert!(!yields(&mut app), "the game's rule is on the room");
    }

    /// C4's fallback: a displaceable body pinched with a wall on its left is
    /// ejected to the nearest free place across the pinch (right, by the
    /// wall's overlap), the same every time; boxed in on both sides beyond its
    /// own width, it is held.
    #[test]
    fn a_displaced_body_is_ejected_to_the_nearest_free_place_or_held() {
        use ambition_platformer2d_core::crush::{BodyResistance, FailureResponse};
        use ambition_platformer2d_core::{Block, Vec2};
        let displace = Some(BodyResistance { compress: None, failure: FailureResponse::Displace });
        // The body spans x 388..412; the wall's right face is at x 393.
        let mut app = respond_app(vec![Block::solid("wall", Vec2::new(300.0, 0.0), Vec2::new(93.0, 600.0))]);
        let free = body(&mut app, displace);
        app.world_mut().resource_mut::<TickObstructions>().0 = vec![BodyObstruction { body: free, obstruction: obstruction(4.0) }];
        app.update();
        let pos = app.world().get::<ambition_platformer2d_core::BodyKinematics>(free).unwrap().pos;
        assert_eq!(pos, Vec2::new(405.0, 276.0), "5 px right, clear of the wall");
        let mut app = respond_app(vec![Block::solid("left", Vec2::new(300.0, 0.0), Vec2::new(93.0, 600.0)),
            Block::solid("right", Vec2::new(407.0, 0.0), Vec2::new(93.0, 600.0))]);
        let boxed = body(&mut app, displace);
        app.world_mut().resource_mut::<TickObstructions>().0 = vec![BodyObstruction { body: boxed, obstruction: obstruction(4.0) }];
        app.update();
        assert_eq!(app.world().get::<ambition_platformer2d_core::BodyKinematics>(boxed).unwrap().pos, Vec2::new(400.0, 276.0),
            "no free place within its width: held");
    }

    /// C4 compression: a compressible body squashes along the pinch by the
    /// overconstraint, to `min_scale` of its rest height and no further, and
    /// only what is left reaches its failure response: a survivable one is
    /// held flat, one that is not dies of it. Released, it recovers at its
    /// rate as far as it has room, and is whole again at rest.
    #[test]
    fn a_compressible_body_flattens_to_its_limit_and_recovers_when_released() {
        use ambition_platformer2d_core::crush::{BodyResistance, CompressAxes, Compressibility, FailureResponse};
        use ambition_platformer2d_core::{BodyKinematics, Vec2};
        let soft = |survivable| Some(BodyResistance {
            compress: Some(Compressibility { axes: CompressAxes::Crush, min_scale: 0.5, recover_per_s: 6.0, survivable }),
            failure: FailureResponse::Rigid });
        let mut app = respond_app(Vec::new());
        app.insert_resource(ambition_time::WorldTime::new(1.0 / 60.0, 1.0 / 60.0));
        app.add_systems(Update, recover_squashed_bodies.after(respond_to_obstructions));
        let slime = body(&mut app, soft(true));
        let pinch = |app: &mut App, body: Entity, over: f32| {
            app.world_mut().resource_mut::<TickObstructions>().0 = vec![BodyObstruction { body, obstruction: obstruction(over) }];
            app.update();
        };
        pinch(&mut app, slime, 10.0);
        assert_eq!(app.world().get::<BodyKinematics>(slime).unwrap().canonical_size, Vec2::new(24.0, 38.0));
        assert_eq!(app.world().get::<Squash>(slime).map(|squash| squash.rest), Some(Vec2::new(24.0, 48.0)));
        pinch(&mut app, slime, 30.0);
        assert_eq!(app.world().get::<BodyKinematics>(slime).unwrap().canonical_size, Vec2::new(24.0, 24.0), "half its rest height");
        assert!(hits(&mut app).is_empty(), "a survivable body flattened is not hurt");
        // Released: 6 rest-heights a second is 4.8 px a tick, to 48.
        app.world_mut().resource_mut::<TickObstructions>().0.clear();
        app.update();
        assert!((app.world().get::<BodyKinematics>(slime).unwrap().canonical_size.y - 28.8).abs() < 1.0e-3);
        for _ in 0..10 { app.update(); }
        assert_eq!(app.world().get::<BodyKinematics>(slime).unwrap().canonical_size, Vec2::new(24.0, 48.0));
        assert!(app.world().get::<Squash>(slime).is_none(), "whole again");

        let frail = body(&mut app, soft(false));
        pinch(&mut app, frail, 30.0);
        assert_eq!(hits(&mut app), vec![i32::MAX / 4], "flattened past what it survives");
    }
}
