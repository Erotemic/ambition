//! An exploding mite that reaches its foe plants, burns its fuse and goes
//! off: it dies by its own hand, its blast is drawn as big as the area it
//! hurts, and the blast hurts the foe.
//!
//! Jon, 2026-11-11: "exploding mite does not explode". It never detonated by
//! itself (its attack was a swipe), and a kill drew one small burst over a
//! 128 px blast. The room's other bodies are removed: a touch from any of them
//! gives the player the post-hit grace a blast lands in.

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::characters::actor::{BodyHealth, WornCharacter};
use ambition_platformer2d::combat::hitbox::ResolvedBodyHit;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::vfx::{VfxInRoom, VfxMessage};
use bevy::ecs::message::{MessageCursor, Messages};
use bevy::prelude::*;

#[test]
#[ignore = "on demand: the exploding mite's detonation in the assembled game"]
fn an_exploding_mite_in_reach_goes_off_and_its_blast_hurts() {
    let mut sim = fixed_60hz_room_sim("volatile_cache");
    sim.step_n(base(), 30);
    let (mite, at) = {
        let world = sim.world_mut();
        let mites: Vec<(Entity, Vec2)> = world
            .query::<(Entity, &WornCharacter, &BodyKinematics)>()
            .iter(world)
            .filter(|(_, worn, _)| worn.id() == "npc_exploding_mite")
            .map(|(entity, _, kin)| (entity, kin.pos))
            .collect();
        *mites.first().expect("`volatile_cache` places exploding mites")
    };
    let player = {
        let world = sim.world_mut();
        let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).expect("a primary player");
        let others: Vec<Entity> = world
            .query::<(Entity, &WornCharacter)>()
            .iter(world)
            .filter(|(entity, _)| *entity != mite && *entity != player)
            .map(|(entity, _)| entity)
            .collect();
        for other in others {
            world.entity_mut(other).despawn();
        }
        player
    };
    sim.teleport_player((at.x + 120.0, at.y));
    let health = |sim: &mut ambition_app::Platformer2dSimHarness, body| {
        sim.world().get::<BodyHealth>(body).map(|health| health.current())
    };
    let before = health(&mut sim, player).expect("the player has health");

    let mut effects = MessageCursor::<VfxInRoom>::default();
    let mut hits = MessageCursor::<ResolvedBodyHit>::default();
    let (mut self_hit, mut blast_hit, mut drawn) = (false, None, Vec::new());
    for _ in 0..300 {
        sim.step(base());
        let world = sim.world_mut();
        for hit in hits.read(world.resource::<Messages<ResolvedBodyHit>>()) {
            self_hit |= hit.victim == mite && hit.attacker == Some(mite);
            if hit.victim == player && hit.attacker == Some(mite) {
                blast_hit = Some(hit.damage);
            }
        }
        for effect in effects.read(world.resource::<Messages<VfxInRoom>>()) {
            if let VfxMessage::Effect { fx, scale, .. } = effect.vfx {
                drawn.push((ambition_platformer2d::vfx::fx::ids::name_of(fx), scale));
            }
        }
        if blast_hit.is_some() {
            break;
        }
    }
    assert!(self_hit, "the mite reached its foe and never went off");
    assert!(
        drawn.iter().any(|(name, scale)| *name == Some("classic_burst") && *scale > 1.5),
        "the blast was not drawn as big as it hurts: {drawn:?}"
    );
    assert_eq!(blast_hit, Some(3), "the blast did not hurt the foe it went off beside");
    assert_eq!(health(&mut sim, player), Some(before - 3));
}
