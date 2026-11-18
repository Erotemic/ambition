//! Native facts from committed projectile operations, before entity removal.

pub mod extension;

use crate::HitIdentity;
pub use ambition_combat_port::projectile_outcome::ProjectileOutcomeKind;
use ambition_platformer2d_core::{BodyKinematics, Vec2};
use ambition_platformer2d_shared_tangle::{
    lifecycle::{InRoomInstance, LiveRoomInstance, SessionScopedEntity, live_room_of},
    sim_id::SimId,
};
use ambition_projectiles::ownership::ProjectileCredit;
use bevy::prelude::*;

#[derive(Message, Clone, Debug, PartialEq)]
pub struct ProjectileOutcome {
    pub observed_at: u64,
    pub sequence: u64,
    pub projectile: HitIdentity,
    pub credit: Option<HitIdentity>,
    pub credit_body: Option<Entity>,
    pub previous_credit_body: Option<Entity>,
    pub interceptor_body: Option<Entity>,
    pub previous_credit: Option<HitIdentity>,
    pub interceptor: Option<HitIdentity>,
    pub kind: ProjectileOutcomeKind,
    pub terminal: bool,
    pub position: Vec2,
}

fn credited(
    credit: &ProjectileCredit,
    session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
) -> Option<HitIdentity> {
    credit.body.as_ref().map(|id| HitIdentity {
        id: Some(id.clone()),
        session,
        room: credit.room,
    })
}

/// Queue a fact after preceding construction and ownership commands commit.
/// The operation captures identity before a following removal can erase it.
pub fn record(
    commands: &mut Commands,
    shot: Entity,
    kind: ProjectileOutcomeKind,
    interceptor: Option<(Entity, ProjectileCredit)>,
    position: Option<Vec2>,
) {
    commands.queue(move |world: &mut World| {
        if !world.contains_resource::<Messages<ProjectileOutcome>>() {
            return;
        }
        if world
            .get::<ambition_projectiles::LiveProjectile>(shot)
            .is_none()
        {
            return;
        }
        let session = world.get::<SessionScopedEntity>(shot).map(|scope| scope.0);
        let projectile = HitIdentity {
            id: world.get::<SimId>(shot).cloned(),
            session,
            room: live_room_of(
                world.get::<InRoomInstance>(shot),
                world.get::<LiveRoomInstance>(shot),
            ),
        };
        let before = world
            .get::<ProjectileCredit>(shot)
            .and_then(|credit| credited(credit, session));
        let before_body = world
            .get::<ambition_projectiles::ProjectileOwner>(shot)
            .map(|owner| owner.0);
        let interceptor_body = interceptor.as_ref().map(|(body, _)| *body);
        let interceptor = interceptor
            .as_ref()
            .and_then(|(_, credit)| credited(credit, session));
        let reflected = kind == ProjectileOutcomeKind::Reflected;
        let fact = ProjectileOutcome {
            observed_at: world
                .get_resource::<ambition_time::SimTick>()
                .map_or(0, |tick| tick.0),
            sequence: world
                .get::<ambition_projectiles::ProjectileSeq>(shot)
                .map_or(0, |seq| seq.0),
            projectile,
            credit: if reflected {
                interceptor.clone()
            } else {
                before.clone()
            },
            credit_body: if reflected {
                interceptor_body
            } else {
                before_body
            },
            previous_credit_body: if reflected { before_body } else { None },
            interceptor_body,
            previous_credit: if reflected { before } else { None },
            interceptor,
            terminal: !matches!(
                kind,
                ProjectileOutcomeKind::Created
                    | ProjectileOutcomeKind::Reflected
                    | ProjectileOutcomeKind::SpentByClash
            ),
            kind,
            position: position
                .or_else(|| world.get::<BodyKinematics>(shot).map(|kin| kin.pos))
                .unwrap_or_default(),
        };
        extension::enqueue(world, shot, &fact);
        world.write_message(fact);
    });
}

/// Retire through the flight owner. Interception can also name its body.
pub fn retire(
    commands: &mut Commands,
    shot: Entity,
    kind: ProjectileOutcomeKind,
    position: Option<Vec2>,
) {
    assert!(!matches!(
        kind,
        ProjectileOutcomeKind::Created
            | ProjectileOutcomeKind::Reflected
            | ProjectileOutcomeKind::SpentByClash
    ));
    record(commands, shot, kind, None, position);
    commands.entity(shot).despawn();
}
