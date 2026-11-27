//! A ray can start a second strike after a body takes damage.

use std::collections::BTreeSet;

use ambition_characters::{actor::ActorFaction, control::DrivingParticipant};
use ambition_combat::{
    damage_box::DamageBoxSpawner,
    extension::{blast::credit_of, targets::foe_bodies_of},
    hit_origin::RayJumpOnDamage,
    strike::{DamageBox, HitboxKnockback, HitboxTarget, RayStrike},
};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{ExecutionScopes, InRoomInstance, LiveRooms};
use ambition_platformer2d_world::collision::CollisionWorld;
use bevy::prelude::*;

use crate::BodyHitResolved;

const JUMP_LIFETIME_S: f32 = 0.1;

/// Make each jump after the primary damage decisions for this tick.
pub fn spawn_ray_jumps(
    world: &World,
    mut reports: ae::replay_messages::ReplayMessageReader<BodyHitResolved>,
    rooms: LiveRooms,
    scopes: ExecutionScopes,
    collision: CollisionWorld,
    mut strikes: DamageBoxSpawner,
) {
    let mut hits: Vec<_> = reports
        .read()
        .filter_map(|hit| {
            let attacker = hit.attacker?;
            let source = hit.attacker_identity.as_ref()?.id.as_ref()?;
            let victim = hit.identity.id.as_ref()?;
            let ray = hit.ray_jump.as_ref()?;
            (hit.resolution.damage_applied() > 0).then_some((hit, attacker, source, victim, ray))
        })
        .collect();
    hits.sort_by_key(|(hit, _, source, victim, _)| {
        (hit.observed_at, hit.identity.room, *source, *victim)
    });
    hits.dedup_by_key(|(hit, _, source, victim, _)| {
        (hit.observed_at, hit.identity.room, *source, *victim)
    });
    // Keep the primary victims and the selected jump targets out of subsequent jumps.
    let mut reached: BTreeSet<_> = hits
        .iter()
        .map(|(hit, attacker, ..)| (hit.observed_at, *attacker, hit.body))
        .collect();
    for (hit, attacker, source, victim, ray) in hits {
        if !permits_jump(
            ray.jump.chance,
            hit.observed_at,
            credit_of(source),
            credit_of(victim),
        ) {
            continue;
        }
        let room = rooms.of(hit.body);
        if room != hit.identity.room || !scopes.share(attacker, hit.body) {
            continue;
        }
        let (Some(kin), Some(faction), Some(foes)) = (
            world.get::<ae::BodyKinematics>(hit.body),
            world.get::<ActorFaction>(attacker),
            foe_bodies_of(world, attacker),
        ) else {
            continue;
        };
        let stamp = room.map(InRoomInstance);
        let Some(walls) = collision
            .room(stamp.as_ref())
            .and_then(|room| room.solids())
        else {
            continue;
        };
        let target = foes
            .into_iter()
            .filter(|foe| {
                foe.id.is_some()
                    && scopes.share(attacker, foe.entity)
                    && !reached.contains(&(hit.observed_at, attacker, foe.entity))
            })
            .filter_map(|foe| clear_ray(&walls, kin.pos, foe.at, ray).map(|ray| (foe.entity, ray)))
            .min_by(|(_, a), (_, b)| a.length.total_cmp(&b.length));
        let Some((target, arc)) = target else {
            continue;
        };
        let faction = ambition_combat::targeting::effective_faction(
            *faction,
            world.get::<DrivingParticipant>(attacker),
        );
        let shape = arc.shape();
        let bounds = shape.place_at(ae::Vec2::ZERO, 1.0, ae::Vec2::Y).bounds();
        let Some(mut strike) = strikes.spawn(
            attacker,
            attacker,
            ambition_combat::hit_side_from_actor_faction(faction),
            kin.pos,
            DamageBox {
                cause: hit.source,
                move_instance: None,
                half_extent: bounds.max.abs().max(bounds.min.abs()),
                shape: Some(shape),
                damage: ray.jump.damage,
                knockback: HitboxKnockback::FeelScale(0.0),
                lifetime_s: JUMP_LIFETIME_S,
                name: None,
            },
        ) else {
            continue;
        };
        // Only the primary strike carries jump settings.
        strike.insert((arc, HitboxTarget(target)));
        let damage_types = hit.damage_types.clone();
        strike.entry::<ambition_combat::HitOrigin>().and_modify(move |mut origin| {
            origin.damage_types = damage_types;
            origin.ray_jump = None;
        });
        reached.insert((hit.observed_at, attacker, target));
    }
}

fn permits_jump(chance: f32, tick: u64, attacker: u64, victim: u64) -> bool {
    let seed = ae::navigation::mix(tick ^ attacker.rotate_left(17) ^ victim);
    let fraction = (seed >> 40) as f32 / (1_u32 << 24) as f32;
    fraction < chance
}

fn clear_ray(
    walls: &ae::World,
    from: ae::Vec2,
    to: ae::Vec2,
    ray: &RayJumpOnDamage,
) -> Option<RayStrike> {
    let delta = to - from;
    let length = delta.length();
    if length <= 0.0 || length > ray.jump.range || !length.is_finite() {
        return None;
    }
    let dir = delta / length;
    let trace = ae::cast::trace_through_apertures(
        walls,
        &[],
        ae::cast::PortalRay {
            origin: from,
            direction: dir,
            reach: length,
            width: ray.width,
        },
        false,
        ae::frame::thread::PieceBudget::SINGLE_HOP.into(),
        Default::default(),
    )
    .ok()?;
    trace.hit.is_none().then(|| RayStrike {
        dir,
        length,
        width: ray.width,
        look: ray.look.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat::{HitIdentity, HitOrigin, HitSource};
    use ambition_platformer2d_shared_tangle::{
        lifecycle::{LiveRoomInstance, RoomInstanceRoot},
        sim_id::SimId,
    };

    #[test]
    fn a_jump_requires_damage_and_selects_one_reachable_enemy() {
        let mut app = App::new();
        ambition_combat::targeting::init_targeting_resources(&mut app);
        app.add_message::<BodyHitResolved>()
            .add_systems(Update, spawn_ray_jumps);
        let world = app.world_mut();
        let room = LiveRoomInstance::ACTIVATION;
        let wall = ae::Block::solid("wall", ae::Vec2::new(60.0, -5.0), ae::Vec2::new(5.0, 10.0));
        world.spawn((
            RoomInstanceRoot,
            room,
            ae::RoomGeometry(ae::World::new(
                "jump",
                ae::Vec2::splat(300.0),
                ae::Vec2::ZERO,
                vec![wall],
            )),
        ));

        let mut body = |name: &str, faction, x, y| {
            let id = SimId::placement(name);
            let identity = HitIdentity {
                id: Some(id.clone()),
                session: None,
                room: Some(room),
            };
            let entity = world
                .spawn((
                    id,
                    faction,
                    InRoomInstance(room),
                    ae::BodyKinematics {
                        pos: ae::Vec2::new(x, y),
                        ..Default::default()
                    },
                ))
                .id();
            (entity, identity)
        };
        let (attacker, credit) = body("caster", ActorFaction::Player, 0.0, 0.0);
        let (victim, identity) = body("primary", ActorFaction::Enemy, 48.0, 0.0);
        body("nearer but behind a wall", ActorFaction::Enemy, 70.0, 0.0);
        body("ally", ActorFaction::Player, 48.0, 10.0);
        let (target, _) = body("clear", ActorFaction::Enemy, 48.0, 30.0);
        body("farther", ActorFaction::Enemy, 48.0, 75.0);
        let mut report = BodyHitResolved {
            projectile: None,
            body: victim,
            attacker: Some(attacker),
            identity,
            attacker_identity: Some(credit),
            observed_at: 17,
            resolution: crate::BodyHitResolution::Blocked,
            source: HitSource::Melee,
            raw_damage: 1,
            damage_types: ambition_combat::damage_types::DamageTypes::from_names(
                vec!["example.fire".into(), "example.electric".into()],
            ).unwrap(),
            attacker_move_instance: None,
            ray_jump: Some(RayJumpOnDamage {
                jump: ambition_combat_port::ray::RayJump {
                    chance: 1.0,
                    range: 96.0,
                    damage: 1,
                },
                width: 5.0,
                look: "force".into(),
            }),
        };
        world.write_message(report.clone());
        app.update();
        let world = app.world_mut();
        let mut arcs = world.query::<(&RayStrike, &HitboxTarget, &HitOrigin)>();
        assert_eq!(arcs.iter(world).count(), 0);

        report.resolution = crate::BodyHitResolution::Damaged {
            damage: 1,
            died: false,
        };
        world.write_message(report.clone());
        world.write_message(report);
        app.update();
        let (ray, selected, origin) = arcs.single(app.world()).unwrap();
        assert_eq!(selected.0, target);
        assert_eq!(ray.length, 30.0);
        assert!(origin.ray_jump.is_none());
        assert_eq!(origin.damage_types.names().collect::<Vec<_>>(), ["example.electric", "example.fire"]);
    }
}
