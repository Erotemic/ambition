//! A hit's status follows that hit's reception on the HOME-body route too.
//!
//! The actor road and the bare reaction have their own proofs
//! (`an_actor_installs_a_hits_status_only_when_it_accepts_the_hit`,
//! `hit_status_tests`). The player is resolved by a different receiver: hits
//! are staged into `PendingPlayerHitEvents` and drained next frame by
//! `ambition_damage`, whose resolution decides whether the reaction runs at all.
//! So the staged hit here is the real carrier, drained by the real resolver.

use crate::common::{base, fixed_60hz_sim};

use ambition_platformer2d::characters::actor::BodyCombat;
use ambition_platformer2d::combat::events::{
    HitEvent, HitMode, HitSource, HitTarget, PendingPlayerHitEvents, StagedPlayerHit,
};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::{HitStatus, HitStatusKind};
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

fn the_player(world: &mut World) -> Entity {
    let mut query = world.query_filtered::<Entity, With<PrimaryPlayer>>();
    query
        .iter(world)
        .next()
        .expect("the hosted app has a primary player")
}

/// Stage one status hit on the settled player and let the resolver drain it.
/// `invulnerable` arms the player's i-frames first, so the same hit is Ignored.
fn after_a_status_hit(kind: HitStatusKind, invulnerable: bool) -> BodyCombat {
    let mut sim = fixed_60hz_sim();
    sim.step_n(base(), 120);
    let player = the_player(sim.world_mut());
    if invulnerable {
        sim.world_mut()
            .get_mut::<BodyCombat>(player)
            .expect("the player carries combat state")
            .damage_invuln_timer = 1.0;
    }
    sim.world_mut()
        .resource_mut::<PendingPlayerHitEvents>()
        .0
        .push(StagedPlayerHit {
            event: HitEvent {
                status: Some(HitStatus {
                    kind,
                    duration_s: 1.0,
                }),
                origin: None,
                properties: Default::default(),
                contact: None,
                pieces: Vec::new(),
                strike_sfx: None,
                volume: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(8.0)).into(),
                damage: 1,
                source: HitSource::Melee,
                attacker: None,
                room: None,
                target: HitTarget::Body(player),
                mode: HitMode::Knockback,
                knockback: None,
                ignored_targets: Vec::new(),
                attacker_move_instance: None,
            },
            attacker_id: None,
            victim_id: None,
        });
    sim.step_n(base(), 2);
    let player = the_player(sim.world_mut());
    sim.world()
        .get::<BodyCombat>(player)
        .expect("the player carries combat state")
        .clone()
}

#[test]
fn the_home_receiver_installs_a_status_only_for_a_hit_it_accepts() {
    let paralyzed = after_a_status_hit(HitStatusKind::Paralyze, false);
    assert!(
        paralyzed.paralysis_timer > 0.5,
        "an accepted paralyzing hit holds the player (timer {})",
        paralyzed.paralysis_timer
    );
    // The settled player stands on the floor, so a burial replaces the hit.
    let buried = after_a_status_hit(HitStatusKind::Bury, false);
    assert!(
        buried.bury_timer > 0.5,
        "an accepted burying hit buries a standing player (timer {})",
        buried.bury_timer
    );

    let refused = after_a_status_hit(HitStatusKind::Paralyze, true);
    assert_eq!(
        refused.paralysis_timer, 0.0,
        "a hit the i-frames ignored installed its status anyway"
    );
}
