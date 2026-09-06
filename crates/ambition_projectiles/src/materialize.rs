//! Projectile request → ECS materialization.
//!
//! Every producer writes [`crate::ProjectileSpawnRequest`]. Two schedule-facing
//! systems drain the same request type at the two historically meaningful
//! first-step boundaries: immediate shots before the unified projectile step,
//! and delayed named body-fire shots after it. The entity-construction code itself
//! is one implementation.

use bevy::prelude::{Commands, Entity, MessageReader, Name, Query, Res, ResMut};

use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, RoomScopedEntity, SessionSpawnScope,
};

use crate::{
    LiveProjectile, ProjectileOwner, ProjectilePresentation, ProjectileSeqCounter,
    ProjectileSpawnRequest, ProjectileStart, ProjectileVisualId,
};

/// Materialize requests whose projectile begins advancing on this tick.
pub fn materialize_projectiles_for_this_tick(
    commands: Commands,
    seq: ResMut<ProjectileSeqCounter>,
    requests: MessageReader<ProjectileSpawnRequest>,
    active_session: Option<Res<ActiveSessionScope>>,
    active_round: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveRoundScope>>,
    sources: Query<&ambition_sfx::BodyPresentationSource>,
) {
    materialize_matching(
        ProjectileStart::StepThisTick,
        commands,
        seq,
        requests,
        active_session,
        active_round,
        sources,
    );
}

/// Materialize requests whose first advance belongs to the next tick.
pub fn materialize_projectiles_for_next_tick(
    commands: Commands,
    seq: ResMut<ProjectileSeqCounter>,
    requests: MessageReader<ProjectileSpawnRequest>,
    active_session: Option<Res<ActiveSessionScope>>,
    active_round: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveRoundScope>>,
    sources: Query<&ambition_sfx::BodyPresentationSource>,
) {
    materialize_matching(
        ProjectileStart::StepNextTick,
        commands,
        seq,
        requests,
        active_session,
        active_round,
        sources,
    );
}

fn materialize_matching(
    wanted: ProjectileStart,
    mut commands: Commands,
    mut seq: ResMut<ProjectileSeqCounter>,
    mut requests: MessageReader<ProjectileSpawnRequest>,
    active_session: Option<Res<ActiveSessionScope>>,
    active_round: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveRoundScope>>,
    sources: Query<&ambition_sfx::BodyPresentationSource>,
) {
    let Some(scope) = SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        requests.clear();
        return;
    };
    let round_scope = active_round
        .as_deref()
        .map(|round| round.spawn_scope())
        .unwrap_or_default();

    for request in requests.read() {
        if request.start != wanted {
            continue;
        }
        let body = &request.projectile.body;
        let mut entity = commands.spawn((
            body.kin,
            body.game,
            seq.next(),
            LiveProjectile,
            RoomScopedEntity,
            // ⭐ The victim ledger arrives with `LiveProjectile`, which
            // `#[require]`s it — see that marker. Listing it here as well would
            // be the fourth copy of a fact one place should own.
        ));
        scope.apply_to(&mut entity);
        round_scope.apply_to(&mut entity);

        if request.owner != Entity::PLACEHOLDER {
            entity.insert(ProjectileOwner(request.owner));
            if let Ok(source) = sources.get(request.owner) {
                entity.insert(source.clone());
            }
        }

        match &request.presentation {
            ProjectilePresentation::NamedKind(kind) => {
                entity.insert((
                    *kind,
                    ProjectileVisualId(kind.visual_id().to_string()),
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
    }
}

#[cfg(test)]
mod tests {
    use ambition_platformer2d_core as ae;
    use bevy::prelude::{App, Entity, IntoScheduleConfigs, Update, With};

    use crate::{
        materialize_projectiles_for_next_tick, materialize_projectiles_for_this_tick,
        LiveProjectile, ProjectileKind, ProjectileOwner, ProjectileSeq, ProjectileSeqCounter,
        ProjectileSpawnRequest, ProjectileStart, ProjectileVisualId,
    };

    fn open_request(owner: Entity, start: ProjectileStart) -> ProjectileSpawnRequest {
        ProjectileSpawnRequest::open(
            owner,
            ambition_projectile_spec::ProjectileSpawn {
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
        app.add_message::<ProjectileSpawnRequest>();
        app.init_resource::<ProjectileSeqCounter>();
        app
    }

    #[test]
    fn immediate_open_request_spawns_one_live_projectile_with_identity() {
        let mut app = app();
        app.add_systems(Update, materialize_projectiles_for_this_tick);
        app.world_mut().write_message(open_request(
            Entity::PLACEHOLDER,
            ProjectileStart::StepThisTick,
        ));
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
        immediate.world_mut().write_message(open_request(
            Entity::PLACEHOLDER,
            ProjectileStart::StepNextTick,
        ));
        immediate.update();
        let count = immediate
            .world_mut()
            .query_filtered::<(), With<LiveProjectile>>()
            .iter(immediate.world())
            .count();
        assert_eq!(count, 0);

        let mut deferred = app();
        deferred.add_systems(Update, materialize_projectiles_for_next_tick);
        deferred.world_mut().write_message(open_request(
            Entity::PLACEHOLDER,
            ProjectileStart::StepThisTick,
        ));
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
        app.world_mut().write_message(open_request(
            Entity::PLACEHOLDER,
            ProjectileStart::StepThisTick,
        ));
        app.world_mut().write_message(open_request(
            Entity::PLACEHOLDER,
            ProjectileStart::StepNextTick,
        ));
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
            .write_message(open_request(owner, ProjectileStart::StepThisTick));
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
        app.world_mut().write_message(open_request(
            Entity::PLACEHOLDER,
            ProjectileStart::StepThisTick,
        ));
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
        let projectile = crate::InFlightProjectile {
            body: crate::ProjectileBody::from_spec(kind.spec(ae::Vec2::ZERO, ae::Vec2::X, 1.0)),
        };
        app.world_mut().write_message(ProjectileSpawnRequest::named(
            owner,
            projectile,
            kind,
            ProjectileStart::StepNextTick,
        ));
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
