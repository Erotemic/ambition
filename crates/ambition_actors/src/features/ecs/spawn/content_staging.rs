//! **`RoomContentStagingRegistry` — the open seam for content-staged room
//! occupants** (N3.2b closeout).
//!
//! Some room occupants are not authored `RoomSpec` placements: the spectator
//! duel's fighters, a demo level's walkers. Content used to stage them from
//! systems consuming the `RoomLoaded` notification — which had two structural
//! faults:
//!
//! 1. **The occupants were invisible to room construction.** A snapshot restore
//!    that stages a room (netcode.md N3.2b) rebuilds exactly what construction
//!    builds; an occupant created only by a future-frame notification consumer
//!    came back as a bare identity with no authored components.
//! 2. **The staging tick was not a sim fact.** The consumers ran on the
//!    presentation schedule (`Update`), so *when* the occupants appeared,
//!    relative to sim ticks, was a function of frame rate.
//!
//! This registry closes both: providers/content register a **pure** stager —
//! `RoomSpec` in, [`SpawnActorRequest`]s out — and room construction
//! ([`RoomFeatureConstructionPlan`](super::RoomFeatureConstructionPlan))
//! drains every registered stager for the room being staged, on the sim side,
//! in both the normal load path and the restore staging path. `RoomLoaded`
//! remains a pure downstream notification (resource re-arms, presentation
//! beats); it no longer creates snapshot-authoritative entities.
//!
//! Purity is what makes the seam preflightable: a stager must be a function of
//! the `RoomSpec` alone, so a mutation-free caller (`RoomConstructionPlan::prepare`, a
//! roster preflight) can ask "what WOULD this room stage?" without staging it.

use std::sync::Arc;

use bevy::ecs::resource::Resource;

use super::super::spawn_actors::SpawnActorRequest;
use crate::rooms::RoomSpec;

/// A registered content stager: a pure function from the authored room to the
/// actors content stages into it.
type Stager = Arc<dyn Fn(&RoomSpec) -> Vec<SpawnActorRequest> + Send + Sync>;

/// A malformed set of content-staged room occupants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomContentStagingError {
    EmptyId { room: String },
    DuplicateId { room: String, id: String },
    AuthoredIdCollision { room: String, id: String },
}

impl std::fmt::Display for RoomContentStagingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyId { room } => {
                write!(
                    f,
                    "content staging for room `{room}` produced an empty actor id"
                )
            }
            Self::DuplicateId { room, id } => write!(
                f,
                "content staging for room `{room}` produced actor id `{id}` more than once"
            ),
            Self::AuthoredIdCollision { room, id } => write!(
                f,
                "content staging for room `{room}` produced `{id}`, which collides with an \
                 authored placement/enemy/boss id"
            ),
        }
    }
}

impl std::error::Error for RoomContentStagingError {}

/// App-installed registry of per-room content stagers. Clone-cheap (the
/// stagers are `Arc`s), like the placement-lowering registry it mirrors.
///
/// Registration is normalized by room/provider/source/schema identity, so
/// equivalent plugin insertion orders produce the same staging and fingerprint
/// contribution. Conflicting duplicate ownership is rejected transactionally.
#[derive(Resource, Clone, Default)]
pub struct RoomContentStagingRegistry {
    stagers: Vec<RoomContentStager>,
}

#[derive(Clone)]
struct RoomContentStager {
    room_id: String,
    owner: String,
    source: String,
    schema_id: String,
    stager: Stager,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomContentStagingRegistrationError {
    EmptyIdentity {
        field: &'static str,
    },
    DuplicateSource {
        room_id: String,
        owner: String,
        source: String,
    },
}

impl std::fmt::Display for RoomContentStagingRegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyIdentity { field } => write!(f, "room content staging {field} must not be empty"),
            Self::DuplicateSource { room_id, owner, source } => write!(f, "room content staging source '{owner}/{source}' registered twice for room '{room_id}'"),
        }
    }
}
impl std::error::Error for RoomContentStagingRegistrationError {}

impl RoomContentStagingRegistry {
    /// Register a pure content stager for `room_id`. The stager runs every
    /// time that room's contents are staged: activation, transition, reset,
    /// hot-reload, and snapshot-restore staging alike.
    pub fn register(
        &mut self,
        room_id: impl Into<String>,
        owner: impl Into<String>,
        source: impl Into<String>,
        schema_id: impl Into<String>,
        stager: impl Fn(&RoomSpec) -> Vec<SpawnActorRequest> + Send + Sync + 'static,
    ) -> Result<(), RoomContentStagingRegistrationError> {
        let room_id = room_id.into();
        let owner = owner.into();
        let source = source.into();
        let schema_id = schema_id.into();
        for (field, value) in [
            ("room id", room_id.as_str()),
            ("owner", owner.as_str()),
            ("source", source.as_str()),
            ("schema id", schema_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(RoomContentStagingRegistrationError::EmptyIdentity { field });
            }
        }
        if self
            .stagers
            .iter()
            .any(|entry| entry.room_id == room_id && entry.owner == owner && entry.source == source)
        {
            return Err(RoomContentStagingRegistrationError::DuplicateSource {
                room_id,
                owner,
                source,
            });
        }
        self.stagers.push(RoomContentStager {
            room_id,
            owner,
            source,
            schema_id,
            stager: Arc::new(stager),
        });
        self.stagers.sort_by(|a, b| {
            (&a.room_id, &a.owner, &a.source, &a.schema_id).cmp(&(
                &b.room_id,
                &b.owner,
                &b.source,
                &b.schema_id,
            ))
        });
        Ok(())
    }

    pub fn schema_descriptors(&self) -> Vec<(String, String, String, String)> {
        self.stagers
            .iter()
            .map(|entry| {
                (
                    entry.room_id.clone(),
                    entry.owner.clone(),
                    entry.source.clone(),
                    entry.schema_id.clone(),
                )
            })
            .collect()
    }

    pub fn deterministic_dump(&self) -> String {
        self.schema_descriptors()
            .into_iter()
            .map(|(room, owner, source, schema)| format!("{room}\t{owner}\t{source}\t{schema}\n"))
            .collect()
    }

    /// Every request content stages into `room`, in registration order, after
    /// validating stable-id uniqueness against both the other stagers and the
    /// room's authored placement/enemy/boss roster.
    ///
    /// The validation happens before any content-staged spawn requests are
    /// emitted, so a duplicate content id cannot get as far as the later
    /// global `SimId` invariant.
    pub fn try_requests_for(
        &self,
        room: &RoomSpec,
    ) -> Result<Vec<SpawnActorRequest>, RoomContentStagingError> {
        Ok(self
            .try_owned_requests_for(room)?
            .into_iter()
            .map(|(_, request)| request)
            .collect())
    }

    /// As [`Self::try_requests_for`], but each request is paired with the id of
    /// the provider that staged it.
    ///
    /// Construction planning needs the owner because a staged occupant's
    /// provenance is *which provider put it here* — the fact
    /// [`SpawnOrigin::ProviderStaged`](ambition_platformer_primitives::construction::SpawnOrigin::ProviderStaged)
    /// records. It is not recoverable from the request, which carries only
    /// content fields.
    pub fn try_owned_requests_for(
        &self,
        room: &RoomSpec,
    ) -> Result<Vec<(String, SpawnActorRequest)>, RoomContentStagingError> {
        let requests = self
            .stagers
            .iter()
            .filter(|entry| entry.room_id == room.id)
            .flat_map(|entry| {
                (entry.stager)(room)
                    .into_iter()
                    .map(|request| (entry.owner.clone(), request))
            })
            .collect::<Vec<_>>();

        let authored = room
            .placements
            .iter()
            .map(|placement| placement.id.0.as_str())
            .chain(room.enemy_spawns.iter().map(|enemy| enemy.id.as_str()))
            .chain(room.boss_spawns.iter().map(|boss| boss.id.as_str()))
            .chain(room.ground_items.iter().map(|item| item.id.as_str()))
            .collect::<std::collections::BTreeSet<_>>();
        let mut staged = std::collections::BTreeSet::new();
        for (_, request) in &requests {
            if request.id.trim().is_empty() {
                return Err(RoomContentStagingError::EmptyId {
                    room: room.id.clone(),
                });
            }
            if authored.contains(request.id.as_str()) {
                return Err(RoomContentStagingError::AuthoredIdCollision {
                    room: room.id.clone(),
                    id: request.id.clone(),
                });
            }
            if !staged.insert(request.id.as_str()) {
                return Err(RoomContentStagingError::DuplicateId {
                    room: room.id.clone(),
                    id: request.id.clone(),
                });
            }
        }
        Ok(requests)
    }

    /// Infallible construction-side convenience. Invalid content staging is an
    /// authored/plugin bug, so normal room construction fails loudly before it
    /// emits any content-staged spawn requests. Snapshot room staging uses `try_requests_for`
    /// during its mutation-free prepare phase and returns a controlled refusal.
    pub fn requests_for(&self, room: &RoomSpec) -> Vec<SpawnActorRequest> {
        self.try_requests_for(room)
            .unwrap_or_else(|err| panic!("invalid room content staging: {err}"))
    }

    /// The feature ids `requests_for` would stage — the mutation-free roster
    /// prediction a restore preflight needs.
    pub fn staged_ids_for(&self, room: &RoomSpec) -> Vec<String> {
        self.requests_for(room)
            .into_iter()
            .map(|request| request.id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat::components::ActorFaction;
    use ambition_engine_core as ae;
    use ambition_entity_catalog::placements::CharacterBrain;

    fn request(id: &str) -> SpawnActorRequest {
        SpawnActorRequest {
            id: id.to_string(),
            name: id.to_string(),
            pos: ae::Vec2::ZERO,
            half_size: ae::Vec2::ONE,
            faction: ActorFaction::Npc,
            grudge_against: None,
            kind: crate::features::SpawnActorKind::Enemy {
                brain: CharacterBrain::Custom("fixture".to_string()),
            },
        }
    }

    #[test]
    fn duplicate_staged_ids_are_rejected_before_spawning() {
        let mut registry = RoomContentStagingRegistry::default();
        registry
            .register("room", "test", "one", "fixture.v1", |_| {
                vec![request("duplicate")]
            })
            .unwrap();
        registry
            .register("room", "test", "two", "fixture.v1", |_| {
                vec![request("duplicate")]
            })
            .unwrap();
        let room = RoomSpec::new(
            "room",
            ae::World::new("room", ae::Vec2::new(128.0, 128.0), ae::Vec2::ZERO, vec![]),
        );
        match registry.try_requests_for(&room) {
            Err(RoomContentStagingError::DuplicateId { room, id }) => {
                assert_eq!(room, "room");
                assert_eq!(id, "duplicate");
            }
            Err(other) => panic!("expected DuplicateId, got {other:?}"),
            Ok(_) => panic!("expected duplicate staged ids to be rejected"),
        }
    }

    #[test]
    fn registration_order_does_not_change_dump_or_request_order() {
        let mut first = RoomContentStagingRegistry::default();
        first
            .register("room", "provider-b", "second", "fixture.v1", |_| {
                vec![request("b")]
            })
            .unwrap();
        first
            .register("room", "provider-a", "first", "fixture.v1", |_| {
                vec![request("a")]
            })
            .unwrap();

        let mut second = RoomContentStagingRegistry::default();
        second
            .register("room", "provider-a", "first", "fixture.v1", |_| {
                vec![request("a")]
            })
            .unwrap();
        second
            .register("room", "provider-b", "second", "fixture.v1", |_| {
                vec![request("b")]
            })
            .unwrap();

        let room = RoomSpec::new(
            "room",
            ae::World::new("room", ae::Vec2::new(128.0, 128.0), ae::Vec2::ZERO, vec![]),
        );
        assert_eq!(first.deterministic_dump(), second.deterministic_dump());
        let first_ids = first
            .try_requests_for(&room)
            .unwrap()
            .into_iter()
            .map(|request| request.id)
            .collect::<Vec<_>>();
        let second_ids = second
            .try_requests_for(&room)
            .unwrap()
            .into_iter()
            .map(|request| request.id)
            .collect::<Vec<_>>();
        assert_eq!(first_ids, second_ids);
    }

    #[test]
    fn duplicate_owner_source_is_structured_and_transactional() {
        let mut registry = RoomContentStagingRegistry::default();
        registry
            .register("room", "provider", "source", "fixture.v1", |_| {
                vec![request("a")]
            })
            .unwrap();
        let before = registry.deterministic_dump();
        let error = registry
            .register("room", "provider", "source", "fixture.v2", |_| {
                vec![request("b")]
            })
            .expect_err("duplicate ownership must be rejected");
        assert!(matches!(
            error,
            RoomContentStagingRegistrationError::DuplicateSource { .. }
        ));
        assert_eq!(registry.deterministic_dump(), before);
    }
}
