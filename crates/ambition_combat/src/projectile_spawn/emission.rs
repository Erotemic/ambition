//! Capture projectile source facts when the producer emits its request.

use super::{PendingProjectileEmissions, ProjectileSpawnRequest};
use crate::{allegiance::AttackAllegiance, targeting::MatchTeam};
use ambition_characters::{actor::ActorFaction, control::DrivingParticipant};
use ambition_platformer2d_shared_tangle::{
    construction::SpawnOrigin,
    lifecycle::{
        ActiveRoundScope, ActiveSessionScope, LiveRooms, RoundScopedEntity, RoundSpawnScope,
        SessionScopedEntity, SessionSpawnScope,
    },
    sim_id::{SimId, SimIdCounter},
};
use ambition_projectiles::ownership::ProjectileCredit;
use bevy::{ecs::system::SystemParam, prelude::*};

/// A request with its source facts fixed. Only the native writer can create it.
#[derive(Message, Clone, Debug)]
pub struct ProjectileEmission {
    pub request: ProjectileSpawnRequest,
    pub(super) source: ProjectileSource,
}

#[derive(Clone, Debug)]
pub(super) struct ProjectileSource {
    pub scope: SessionSpawnScope,
    pub round: RoundSpawnScope,
    pub allegiance: Option<AttackAllegiance>,
    pub credit: ProjectileCredit,
    pub presentation: Option<ambition_sfx::BodyPresentationSource>,
    pub identity: Option<(SimId, SpawnOrigin)>,
}

/// The common emission path for body, room, item, and script producers.
#[derive(SystemParam)]
pub struct ProjectileWriter<'w, 's> {
    sources: Query<
        'w,
        's,
        (
            Option<&'static ActorFaction>,
            Option<&'static DrivingParticipant>,
            Option<&'static MatchTeam>,
            Option<&'static SessionScopedEntity>,
            Option<&'static RoundScopedEntity>,
            Option<&'static ambition_sfx::BodyPresentationSource>,
            Option<&'static AttackAllegiance>,
        ),
    >,
    identities: Query<'w, 's, (&'static SimId, &'static mut SimIdCounter)>,
    rooms: LiveRooms<'w, 's>,
    active_session: Option<Res<'w, ActiveSessionScope>>,
    active_round: Option<Res<'w, ActiveRoundScope>>,
    // Some small simulation hosts do not install projectiles.
    requests: Option<MessageWriter<'w, ProjectileEmission>>,
    pending: Option<ResMut<'w, PendingProjectileEmissions>>,
}

impl ProjectileWriter<'_, '_> {
    /// Capture a body's own emission before its lifetime or control can change.
    pub fn write(&mut self, request: ProjectileSpawnRequest) -> bool {
        self.write_from(request.owner, request)
    }

    /// Separate the emitter's birth context from the body's combat credit.
    pub fn write_from(&mut self, emitter: Entity, request: ProjectileSpawnRequest) -> bool {
        if self.requests.is_none() || self.pending.is_none() {
            return false;
        }
        if emitter == Entity::PLACEHOLDER && request.owner != Entity::PLACEHOLDER {
            warn!("projectile emission refused: credited work needs an emitter");
            return false;
        }
        let source = if emitter == Entity::PLACEHOLDER {
            let Some(scope) =
                SessionSpawnScope::for_optional_active_session(self.active_session.as_deref())
            else {
                return false;
            };
            ProjectileSource {
                scope,
                round: self
                    .active_round
                    .as_deref()
                    .map_or(RoundSpawnScope::UNSCOPED, ActiveRoundScope::spawn_scope),
                allegiance: None,
                credit: ProjectileCredit::default(),
                presentation: None,
                identity: None,
            }
        } else {
            let Ok((_, _, _, session, round, presentation, captured)) = self.sources.get(emitter)
            else {
                warn!(
                    ?emitter,
                    "projectile emission refused: its emitter has left"
                );
                return false;
            };
            let room = self.rooms.of(emitter);
            let scope = SessionSpawnScope::new(session.map(|value| value.0)).in_room(room);
            let owner = self.sources.get(request.owner).ok();
            if request.owner != Entity::PLACEHOLDER && owner.is_none() {
                warn!("projectile emission refused: its credit body has left");
                return false;
            }
            if owner.is_some_and(|(_, _, _, session, _, _, _)| {
                session.map(|value| value.0) != scope.id()
            }) {
                warn!(
                    ?emitter,
                    "projectile emission refused: credit belongs to another session"
                );
                return false;
            }
            let allegiance = captured.cloned().or_else(|| {
                owner.and_then(|(faction, driver, team, _, _, _, _)| {
                    faction.map(|faction| AttackAllegiance::from_body(*faction, team, driver))
                })
            });
            let credit = ProjectileCredit::capture(
                self.identities.get(request.owner).ok().map(|(id, _)| id),
                self.rooms.of(request.owner),
            );
            let identity = self
                .identities
                .get_mut(emitter)
                .ok()
                .map(|(parent, mut counter)| {
                    let sequence = counter.next();
                    (
                        SimId::spawned(parent, sequence),
                        SpawnOrigin::Dynamic {
                            parent: parent.clone(),
                            sequence,
                        },
                    )
                });
            ProjectileSource {
                scope,
                round: round.map(|value| value.0.into()).unwrap_or_else(|| {
                    let active = SessionSpawnScope::for_optional_active_session(
                        self.active_session.as_deref(),
                    );
                    if active.is_some_and(|active| active.id() == scope.id()) {
                        self.active_round
                            .as_deref()
                            .map_or(RoundSpawnScope::UNSCOPED, ActiveRoundScope::spawn_scope)
                    } else {
                        RoundSpawnScope::UNSCOPED
                    }
                }),
                allegiance,
                credit,
                presentation: presentation.cloned(),
                identity,
            }
        };
        let emission = ProjectileEmission { request, source };
        self.pending
            .as_mut()
            .expect("checked above")
            .0
            .push(emission.clone());
        self.requests
            .as_mut()
            .expect("checked above")
            .write(emission);
        true
    }
}

impl std::ops::Deref for ProjectileEmission {
    type Target = ProjectileSpawnRequest;
    fn deref(&self) -> &Self::Target {
        &self.request
    }
}
