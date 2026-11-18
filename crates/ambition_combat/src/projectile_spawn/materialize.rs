//! Construct projectiles from the facts captured at emission.

#[cfg(test)]
use super::ProjectileEmission;
#[cfg(test)]
use bevy::ecs::system::RunSystemOnce;

use super::{PendingProjectileEmissions, ProjectilePresentation, ProjectileStart};
use ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity;
use ambition_projectiles::{
    LiveProjectile, ProjectileOwner, ProjectileSeqCounter, ProjectileVisualId,
};
use bevy::prelude::{Commands, Entity, Name, Query, ResMut};

pub fn materialize_projectiles_for_this_tick(
    commands: Commands,
    seq: ResMut<ProjectileSeqCounter>,
    requests: ResMut<PendingProjectileEmissions>,
    live: Query<Entity>,
) {
    materialize_matching(ProjectileStart::StepThisTick, commands, seq, requests, live);
}
pub fn materialize_projectiles_for_next_tick(
    commands: Commands,
    seq: ResMut<ProjectileSeqCounter>,
    requests: ResMut<PendingProjectileEmissions>,
    live: Query<Entity>,
) {
    materialize_matching(ProjectileStart::StepNextTick, commands, seq, requests, live);
}
fn materialize_matching(
    wanted: ProjectileStart,
    mut commands: Commands,
    mut seq: ResMut<ProjectileSeqCounter>,
    mut requests: ResMut<PendingProjectileEmissions>,
    live: Query<Entity>,
) {
    for emission in requests.take(wanted) {
        let request = &emission.request;
        let source = &emission.source;
        let body = &request.projectile.body;
        let mut entity = commands.spawn((
            body.kin,
            body.game,
            body.acceleration,
            seq.next(),
            LiveProjectile,
            RoomScopedEntity,
            source.credit.clone(),
        ));
        source.scope.apply_to(&mut entity);
        source.round.apply_to(&mut entity);
        if let Some((id, origin)) = &source.identity {
            entity.insert((id.clone(), origin.clone()));
        }
        if let Some(allegiance) = &source.allegiance {
            entity.insert(allegiance.clone());
        }
        if let Some(presentation) = &source.presentation {
            entity.insert(presentation.clone());
        }
        if request.owner != Entity::PLACEHOLDER && live.contains(request.owner) {
            entity.insert(ProjectileOwner(request.owner));
        }
        if let Some(instance) = request.move_instance {
            entity.insert(ambition_projectiles::FiredByMoveInstance(instance));
        }
        if request.weapon_shot {
            entity.insert(ambition_projectiles::WeaponShot);
        }
        if let Some(launch) = request.launch {
            entity.insert(launch);
        }
        match &request.presentation {
            ProjectilePresentation::NamedKind(kind) => {
                entity.insert((
                    *kind,
                    ProjectileVisualId(kind.visual_id().to_string()),
                    Name::new("Named projectile (sim)"),
                ));
            }
            ProjectilePresentation::NamedCharged(kind, tier) => {
                entity.insert((
                    *kind,
                    ProjectileVisualId(kind.visual_id_at(*tier).to_string()),
                    Name::new("Named projectile (sim)"),
                ));
            }
            ProjectilePresentation::OpenVisual(visual_id) => {
                entity.insert((
                    ProjectileVisualId(visual_id.clone()),
                    Name::new("Open projectile (sim)"),
                ));
            }
        }
        if let Some(reply) = request.reply.clone() {
            let subjects = source
                .identity
                .as_ref()
                .map(|(id, _)| id.as_str().to_owned())
                .into_iter()
                .collect();
            commands.queue(move |world: &mut bevy::prelude::World| {
                ambition_platformer2d_shared_tangle::action_results::publish_action_result(
                    world,
                    Some(&reply),
                    ambition_extension_sdk::action::ActionStage::Applied,
                    true,
                    "projectile_created",
                    "",
                    subjects,
                );
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use ambition_platformer2d_core as ae;
    use bevy::prelude::{App, Entity, IntoScheduleConfigs, Update, With};

    use super::*;
    use crate::projectile_spawn::ProjectileSpawnRequest;
    use ambition_projectiles::{
        LiveProjectile, ProjectileKind, ProjectileOwner, ProjectileSeq, ProjectileSeqCounter,
        ProjectileVisualId,
    };

    fn open_request(owner: Entity, start: ProjectileStart) -> ProjectileSpawnRequest {
        ProjectileSpawnRequest::open(
            owner,
            ambition_projectile_spec::ProjectileSpawn {
                reply: None,
                properties: Default::default(),
                guard: Default::default(),
                origin: ae::Vec2::ZERO,
                dir: ae::Vec2::new(1.0, 0.0),
                speed: 100.0,
                damage: 1,
                max_lifetime: 1.0,
                half_extent: ae::Vec2::new(8.0, 8.0),
                gravity: 0.0,
                visual_id: "glider".into(),
                bounces: 0,
                bounce_on_world_contact: false,
                splash_half_extent: 0.0,
                boomerang_return_s: None,
            },
            start,
        )
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<crate::projectile_spawn::PendingProjectileEmissions>()
            .add_message::<crate::projectile_spawn::ProjectileEmission>();
        app.init_resource::<ProjectileSeqCounter>();
        app
    }

    #[test]
    fn immediate_open_request_spawns_one_live_projectile_with_identity() {
        let mut app = app();
        app.add_systems(Update, materialize_projectiles_for_this_tick);
        app.world_mut()
            .run_system_once_with(
                |bevy::prelude::In(request),
                 mut writer: crate::projectile_spawn::ProjectileWriter| {
                    writer.write(request)
                },
                open_request(Entity::PLACEHOLDER, ProjectileStart::StepThisTick),
            )
            .unwrap();
        app.update();

        let mut q = app
            .world_mut()
            .query_filtered::<(&ProjectileVisualId, &ProjectileSeq), With<LiveProjectile>>();
        let rows: Vec<_> = q
            .iter(app.world())
            .map(|(visual_id, seq)| (visual_id.0.clone(), *seq))
            .collect();
        assert_eq!(rows, vec![("glider".to_string(), ProjectileSeq(0))]);
    }

    #[test]
    fn each_materializer_only_consumes_its_own_first_step_class() {
        let mut immediate = app();
        immediate.add_systems(Update, materialize_projectiles_for_this_tick);
        immediate
            .world_mut()
            .run_system_once_with(
                |bevy::prelude::In(request),
                 mut writer: crate::projectile_spawn::ProjectileWriter| {
                    writer.write(request)
                },
                open_request(Entity::PLACEHOLDER, ProjectileStart::StepNextTick),
            )
            .unwrap();
        immediate.update();
        let count = immediate
            .world_mut()
            .query_filtered::<(), With<LiveProjectile>>()
            .iter(immediate.world())
            .count();
        assert_eq!(count, 0);

        let mut deferred = app();
        deferred.add_systems(Update, materialize_projectiles_for_next_tick);
        deferred
            .world_mut()
            .run_system_once_with(
                |bevy::prelude::In(request),
                 mut writer: crate::projectile_spawn::ProjectileWriter| {
                    writer.write(request)
                },
                open_request(Entity::PLACEHOLDER, ProjectileStart::StepThisTick),
            )
            .unwrap();
        deferred.update();
        let count = deferred
            .world_mut()
            .query_filtered::<(), With<LiveProjectile>>()
            .iter(deferred.world())
            .count();
        assert_eq!(count, 0);
    }

    #[test]
    fn both_timing_faces_share_one_channel_without_duplicates() {
        let mut app = app();
        app.add_systems(
            Update,
            (
                materialize_projectiles_for_this_tick,
                materialize_projectiles_for_next_tick,
            )
                .chain(),
        );
        app.world_mut()
            .run_system_once_with(
                |bevy::prelude::In(request),
                 mut writer: crate::projectile_spawn::ProjectileWriter| {
                    writer.write(request)
                },
                open_request(Entity::PLACEHOLDER, ProjectileStart::StepThisTick),
            )
            .unwrap();
        app.world_mut()
            .run_system_once_with(
                |bevy::prelude::In(request),
                 mut writer: crate::projectile_spawn::ProjectileWriter| {
                    writer.write(request)
                },
                open_request(Entity::PLACEHOLDER, ProjectileStart::StepNextTick),
            )
            .unwrap();
        app.update();

        let mut q = app
            .world_mut()
            .query_filtered::<&ProjectileSeq, With<LiveProjectile>>();
        let mut seqs: Vec<_> = q.iter(app.world()).copied().collect();
        seqs.sort();
        assert_eq!(
            seqs,
            vec![ProjectileSeq(0), ProjectileSeq(1)],
            "the two timing cursors must see the same request stream but materialize each request exactly once"
        );
    }

    #[test]
    fn real_owner_and_presentation_source_are_stamped_at_materialization() {
        let mut app = app();
        app.add_systems(Update, materialize_projectiles_for_this_tick);
        let owner = app
            .world_mut()
            .spawn(ambition_sfx::BodyPresentationSource(
                ambition_sfx::PresentationSourceId::new("sanic_demo"),
            ))
            .id();
        app.world_mut()
            .run_system_once_with(
                |bevy::prelude::In(request),
                 mut writer: crate::projectile_spawn::ProjectileWriter| {
                    writer.write(request)
                },
                open_request(owner, ProjectileStart::StepThisTick),
            )
            .unwrap();
        app.update();

        let mut q = app.world_mut().query_filtered::<
            (&ProjectileOwner, &ambition_sfx::BodyPresentationSource),
            With<LiveProjectile>,
        >();
        let rows: Vec<_> = q
            .iter(app.world())
            .map(|(owner, source)| (owner.0, source.id().as_str().to_string()))
            .collect();
        assert_eq!(rows, vec![(owner, "sanic_demo".to_string())]);
    }

    #[test]
    fn placeholder_owner_stays_ownerless_and_has_no_presentation_source() {
        let mut app = app();
        app.add_systems(Update, materialize_projectiles_for_this_tick);
        app.world_mut()
            .run_system_once_with(
                |bevy::prelude::In(request),
                 mut writer: crate::projectile_spawn::ProjectileWriter| {
                    writer.write(request)
                },
                open_request(Entity::PLACEHOLDER, ProjectileStart::StepThisTick),
            )
            .unwrap();
        app.update();

        let mut q = app.world_mut().query_filtered::<(
            Option<&ProjectileOwner>,
            Option<&ambition_sfx::BodyPresentationSource>,
        ), With<LiveProjectile>>();
        let rows: Vec<_> = q
            .iter(app.world())
            .map(|(owner, source)| (owner.is_some(), source.is_some()))
            .collect();
        assert_eq!(rows, vec![(false, false)]);
    }

    #[test]
    fn named_request_stamps_named_kind_and_named_visual() {
        let mut app = app();
        app.add_systems(Update, materialize_projectiles_for_next_tick);
        let owner = app.world_mut().spawn_empty().id();
        let kind = ProjectileKind::Fireball;
        let projectile = ambition_projectiles::InFlightProjectile {
            body: ambition_projectiles::ProjectileBody::from_spec(kind.spec(
                ae::Vec2::ZERO,
                ae::Vec2::X,
                1.0,
            )),
        };
        app.world_mut()
            .run_system_once_with(
                |bevy::prelude::In(request),
                 mut writer: crate::projectile_spawn::ProjectileWriter| {
                    writer.write(request)
                },
                ProjectileSpawnRequest::named(
                    owner,
                    projectile,
                    kind,
                    ProjectileStart::StepNextTick,
                ),
            )
            .unwrap();
        app.update();

        let mut q = app.world_mut().query_filtered::<
            (&ProjectileKind, &ProjectileVisualId, &ProjectileOwner),
            With<LiveProjectile>,
        >();
        let rows: Vec<_> = q
            .iter(app.world())
            .map(|(kind, visual, owner)| (*kind, visual.0.clone(), owner.0))
            .collect();
        assert_eq!(rows, vec![(kind, kind.visual_id().to_string(), owner)]);
    }
}

#[cfg(test)]
mod credit_scope_tests {
    use super::*;
    use crate::projectile_spawn::ProjectileSpawnRequest;
    use ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope;
    use ambition_platformer2d_shared_tangle::{
        lifecycle::{InRoomInstance, LiveRoomInstance, SessionScopedEntity},
        sim_id::SimId,
    };
    use bevy::prelude::{App, Update};
    #[test]
    fn source_session_is_captured_without_using_the_frontend_active_session() {
        for (foreign, root) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut app = App::new();
            app.init_resource::<crate::projectile_spawn::PendingProjectileEmissions>()
                .add_message::<crate::projectile_spawn::ProjectileEmission>()
                .init_resource::<ProjectileSeqCounter>();
            app.add_systems(Update, materialize_projectiles_for_this_tick);
            let mut active = ActiveSessionScope::default();
            let source_scope = active.begin();
            let id = SimId::placement("source");
            let owner = app
                .world_mut()
                .spawn((
                    id.clone(),
                    SessionScopedEntity(source_scope),
                    InRoomInstance(LiveRoomInstance::ACTIVATION),
                ))
                .id();
            if root {
                app.world_mut()
                    .entity_mut(owner)
                    .remove::<InRoomInstance>()
                    .insert((
                        ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot,
                        LiveRoomInstance::ACTIVATION,
                    ));
            }
            if foreign {
                active.begin();
            }
            app.insert_resource(active);
            app.world_mut()
                .run_system_once_with(
                    |bevy::prelude::In(request),
                     mut writer: crate::projectile_spawn::ProjectileWriter| {
                        writer.write(request)
                    },
                    ProjectileSpawnRequest::named(
                        owner,
                        ambition_projectiles::InFlightProjectile {
                            body: ambition_projectiles::ProjectileBody::from_spec(
                                ambition_projectiles::ProjectileKind::Fireball.spec(
                                    ambition_platformer2d_core::Vec2::ZERO,
                                    ambition_platformer2d_core::Vec2::X,
                                    1.0,
                                ),
                            ),
                        },
                        ambition_projectiles::ProjectileKind::Fireball,
                        ProjectileStart::StepThisTick,
                    ),
                )
                .unwrap();
            app.update();
            let credits: Vec<_> = app
                .world_mut()
                .query::<&ambition_projectiles::ownership::ProjectileCredit>()
                .iter(app.world())
                .cloned()
                .collect();
            assert_eq!(
                credits,
                vec![ambition_projectiles::ownership::ProjectileCredit::capture(
                    Some(&id),
                    Some(LiveRoomInstance::ACTIVATION)
                )]
            );
            let sessions: Vec<_> = app
                .world_mut()
                .query_filtered::<&SessionScopedEntity, bevy::prelude::With<LiveProjectile>>()
                .iter(app.world())
                .map(|scope| scope.0)
                .collect();
            assert_eq!(sessions, vec![source_scope]);
        }
    }
}

#[cfg(test)]
mod captured_birth_tests {
    use super::*;
    use crate::projectile_spawn::{ProjectileSpawnRequest, ProjectileWriter};
    use crate::{allegiance::AttackAllegiance, targeting::MatchTeam};
    use ambition_characters::{
        actor::ActorFaction,
        control::{DrivingParticipant, PlayerSlot},
    };
    use ambition_platformer2d_core::Vec2;
    use ambition_platformer2d_shared_tangle::{
        construction::SpawnOrigin,
        lifecycle::{
            ActiveRoundScope, ActiveSessionScope, InRoomInstance, LiveRoomInstance,
            RoundScopedEntity, SessionScopedEntity,
        },
        sim_id::SimId,
    };
    use ambition_projectiles::{
        InFlightProjectile, ProjectileBody, ProjectileKind, ownership::ProjectileCredit,
    };
    use bevy::{ecs::system::RunSystemOnce, prelude::*};

    fn fire(world: &mut World, emitter: Entity, owner: Entity) -> bool {
        world
            .run_system_once_with(
                |In((emitter, owner)), mut writer: ProjectileWriter| {
                    writer.write_from(
                        emitter,
                        ProjectileSpawnRequest::named(
                            owner,
                            InFlightProjectile {
                                body: ProjectileBody::from_spec(ProjectileKind::Fireball.spec(
                                    Vec2::ZERO,
                                    Vec2::X,
                                    1.0,
                                )),
                            },
                            ProjectileKind::Fireball,
                            ProjectileStart::StepNextTick,
                        ),
                    )
                },
                (emitter, owner),
            )
            .unwrap()
    }

    #[test]
    fn deferred_birth_retains_identity_credit_side_presentation_and_lifetime_after_source_retirement()
     {
        for retire in [false, true] {
            let mut app = App::new();
            app.init_resource::<PendingProjectileEmissions>()
                .add_message::<ProjectileEmission>()
                .init_resource::<ProjectileSeqCounter>();
            app.add_systems(Update, materialize_projectiles_for_next_tick);
            let mut session = ActiveSessionScope::default();
            let original_session = session.begin();
            let mut round = ActiveRoundScope::default();
            let original_round = round.begin();
            app.insert_resource(session).insert_resource(round);
            let source_id = SimId::placement("emitter");
            let driver = DrivingParticipant(PlayerSlot(2));
            let emitter = app
                .world_mut()
                .spawn((
                    source_id.clone(),
                    ActorFaction::Enemy,
                    driver,
                    MatchTeam::new("blue"),
                    SessionScopedEntity(original_session),
                    InRoomInstance(LiveRoomInstance::ACTIVATION),
                    ambition_sfx::BodyPresentationSource(ambition_sfx::PresentationSourceId::new(
                        "voice.original",
                    )),
                ))
                .id();
            assert!(fire(app.world_mut(), emitter, emitter));
            let emitted_id = SimId::spawned(&source_id, 0);
            assert!(
                !app.world()
                    .resource::<bevy::ecs::message::Messages<ProjectileEmission>>()
                    .is_empty()
            );
            if retire {
                app.world_mut().despawn(emitter);
            } else {
                app.world_mut()
                    .entity_mut(emitter)
                    .remove::<DrivingParticipant>()
                    .insert((
                        MatchTeam::new("red"),
                        ActorFaction::Boss,
                        InRoomInstance(LiveRoomInstance::from_ordinal(9)),
                        ambition_sfx::BodyPresentationSource(
                            ambition_sfx::PresentationSourceId::new("voice.changed"),
                        ),
                    ));
            }
            app.world_mut().resource_mut::<ActiveSessionScope>().begin();
            app.world_mut().resource_mut::<ActiveRoundScope>().begin();
            app.update();
            let mut shots = app.world_mut().query_filtered::<(
                &SimId,
                &SpawnOrigin,
                &ProjectileCredit,
                &AttackAllegiance,
                &SessionScopedEntity,
                &RoundScopedEntity,
                &InRoomInstance,
                &ambition_sfx::BodyPresentationSource,
                Option<&ProjectileOwner>,
            ), With<LiveProjectile>>();
            let (id, origin, credit, side, session, round, room, voice, live_owner) =
                shots.single(app.world()).unwrap();
            assert_eq!(id, &emitted_id);
            assert_eq!(
                origin,
                &SpawnOrigin::Dynamic {
                    parent: source_id.clone(),
                    sequence: 0
                }
            );
            assert_eq!(
                credit,
                &ProjectileCredit::capture(Some(&source_id), Some(LiveRoomInstance::ACTIVATION))
            );
            assert_eq!(
                side,
                &AttackAllegiance::from_body(
                    ActorFaction::Enemy,
                    Some(&MatchTeam::new("blue")),
                    Some(&driver)
                )
            );
            assert_eq!(session.0, original_session);
            assert_eq!(round.0, original_round);
            assert_eq!(room.0, LiveRoomInstance::ACTIVATION);
            assert_eq!(voice.id().as_str(), "voice.original");
            assert_eq!(
                live_owner.map(|owner| owner.0),
                (!retire).then_some(emitter)
            );
        }
    }

    #[test]
    fn foreign_credit_and_retired_emitters_are_refused_without_spending_a_descendant_id() {
        let mut app = App::new();
        app.init_resource::<PendingProjectileEmissions>()
            .add_message::<ProjectileEmission>();
        let mut session = ActiveSessionScope::default();
        let a = session.begin();
        let b = session.begin();
        let emitter = app
            .world_mut()
            .spawn((SimId::placement("emitter"), SessionScopedEntity(a)))
            .id();
        let foreign = app
            .world_mut()
            .spawn((SimId::placement("foreign"), SessionScopedEntity(b)))
            .id();
        assert!(!fire(app.world_mut(), emitter, foreign));
        assert!(!fire(app.world_mut(), Entity::PLACEHOLDER, emitter));
        assert!(
            app.world()
                .resource::<bevy::ecs::message::Messages<ProjectileEmission>>()
                .is_empty()
        );
        assert!(fire(app.world_mut(), emitter, emitter));
        let emission = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ProjectileEmission>>()
            .drain()
            .next()
            .unwrap();
        assert_eq!(
            emission.source.identity.unwrap().0,
            SimId::spawned(&SimId::placement("emitter"), 0)
        );
        let other_emitter = app.world_mut().spawn(SimId::placement("other")).id();
        app.world_mut().despawn(emitter);
        assert!(!fire(app.world_mut(), emitter, emitter));
        assert!(!fire(app.world_mut(), other_emitter, emitter));
    }

    #[test]
    fn another_sessions_active_round_cannot_supply_a_shots_lifetime() {
        let mut app = App::new();
        app.init_resource::<PendingProjectileEmissions>()
            .add_message::<ProjectileEmission>();
        let mut sessions = ActiveSessionScope::default();
        let source_session = sessions.begin();
        sessions.begin();
        let mut rounds = ActiveRoundScope::default();
        rounds.begin();
        app.insert_resource(sessions).insert_resource(rounds);
        let emitter = app
            .world_mut()
            .spawn((
                SimId::placement("source"),
                SessionScopedEntity(source_session),
            ))
            .id();
        assert!(fire(app.world_mut(), emitter, emitter));
        let emission = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ProjectileEmission>>()
            .drain()
            .next()
            .unwrap();
        assert_eq!(
            emission.source.round,
            ambition_platformer2d_shared_tangle::lifecycle::RoundSpawnScope::UNSCOPED
        );
    }
}

#[cfg(test)]
mod pending_tests {
    use super::*;
    use crate::projectile_spawn::{ProjectileSpawnRequest, ProjectileWriter};
    use ambition_characters::actor::ActorFaction;
    use ambition_platformer2d_core::Vec2;
    use ambition_platformer2d_shared_tangle::{
        lifecycle::{ActiveSessionScope, SessionScopedEntity},
        sim_id::SimId,
    };
    use ambition_projectiles::{InFlightProjectile, ProjectileBody, ProjectileKind};
    use bevy::{ecs::message::Messages, prelude::*};

    fn queued(scope_history: usize, unrelated_entities: usize) -> App {
        let mut app = App::new();
        app.init_resource::<PendingProjectileEmissions>()
            .add_message::<ProjectileEmission>()
            .init_resource::<ProjectileSeqCounter>();
        let mut sessions = ActiveSessionScope::default();
        for _ in 0..scope_history {
            sessions.begin();
        }
        let session = sessions.begin();
        for _ in 0..unrelated_entities {
            app.world_mut().spawn_empty();
        }
        let owner = app
            .world_mut()
            .spawn((
                SimId::placement("source"),
                ActorFaction::Enemy,
                SessionScopedEntity(session),
            ))
            .id();
        assert!(
            app.world_mut()
                .run_system_once_with(
                    |In(owner), mut writer: ProjectileWriter| writer.write(
                        ProjectileSpawnRequest::named(
                            owner,
                            InFlightProjectile {
                                body: ProjectileBody::from_spec(ProjectileKind::Fireball.spec(
                                    Vec2::ZERO,
                                    Vec2::X,
                                    1.0
                                ))
                            },
                            ProjectileKind::Fireball,
                            ProjectileStart::StepThisTick
                        )
                    ),
                    owner
                )
                .unwrap()
        );
        app
    }

    #[test]
    fn transport_clear_and_queue_restore_preserve_late_work_and_consume_it_once() {
        let mut app = queued(0, 0);
        let saved = app.world().resource::<PendingProjectileEmissions>().clone();
        app.world_mut()
            .resource_mut::<Messages<ProjectileEmission>>()
            .clear();
        app.world_mut()
            .resource_mut::<PendingProjectileEmissions>()
            .0
            .clear();
        app.insert_resource(saved);
        app.add_systems(Update, materialize_projectiles_for_next_tick);
        app.update();
        assert!(
            !app.world()
                .resource::<PendingProjectileEmissions>()
                .is_empty()
        );
        app.add_systems(Update, materialize_projectiles_for_this_tick);
        app.update();
        assert!(
            app.world()
                .resource::<PendingProjectileEmissions>()
                .is_empty()
        );
        assert_eq!(app.world().resource::<ProjectileSeqCounter>().0, 1);
        app.update();
        assert_eq!(app.world().resource::<ProjectileSeqCounter>().0, 1);
    }

    #[test]
    fn pending_projection_keeps_gameplay_but_excludes_native_allocation_and_session_history() {
        let a = queued(1, 0);
        let b = queued(17, 8);
        let baseline = a
            .world()
            .resource::<PendingProjectileEmissions>()
            .checksum();
        assert_eq!(
            baseline,
            b.world()
                .resource::<PendingProjectileEmissions>()
                .checksum()
        );
        let mut changed = b.world().resource::<PendingProjectileEmissions>().clone();
        changed.0[0].request.projectile.body.game.damage += 1;
        assert_ne!(baseline, changed.checksum());
        let mut changed = a.world().resource::<PendingProjectileEmissions>().clone();
        changed.0[0].source.credit.body = Some(SimId::placement("another"));
        assert_ne!(baseline, changed.checksum());
    }
}
