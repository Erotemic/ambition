//! The projectile domain's extension request adapter.
//!
//! A module submits a [`ProjectileSpawnPort`] value; this adapter lowers it
//! through [`ProjectileSpawnRequest::open`], the one spawn-request seam, with
//! the invocation's body as owner and the invocation's move use as credit.
//! The module cannot name another owner or another move use.

use ambition_extension_host::{
    ExtensionAppExt, ExtensionOutbox, InBossConduct, InHitReaction, InModuleEntityTick,
    InTechniqueExecution, InWieldedUse, LowersIn,
};
use ambition_extension_sdk::phases::{
    BOSS_CONDUCT, HIT_REACTION, MODULE_ENTITY_TICK, TECHNIQUE_EXECUTION, WIELDED_USE,
};
use ambition_extension_sdk::action::ActionStage;
use ambition_platformer2d_shared_tangle::{
    action_results::publish_action_result,
    sim_id::{SimId, SimIdCounter},
};
use ambition_projectile_spec::ProjectileSpawnPort;
use bevy::{ecs::system::SystemState, prelude::*};

use super::{ProjectileSpawnRequest, ProjectileStart};

/// Install the request port in `technique_execution`, where a spawn
/// materializes before this tick's projectile step.
pub fn install(app: &mut App) {
    super::install_emission(app);
    app.install_extension_submitted_request::<ProjectileSpawnPort, _>(
        TECHNIQUE_EXECUTION,
        "ambition_combat",
        lower_projectile_spawns::<InTechniqueExecution>,
    );
}

/// Install the same request port in `wielded_use` (a held item's use), which
/// runs before this tick's projectile step too.
pub fn install_for_wielded_use(app: &mut App) {
    super::install_emission(app);
    app.install_extension_submitted_request::<ProjectileSpawnPort, _>(
        WIELDED_USE,
        "ambition_combat",
        lower_projectile_spawns::<InWieldedUse>,
    );
}

/// Install the same request port in `module_entity_tick` (a module-owned
/// entity fires: the entity owns the projectile).
pub fn install_for_module_entity_tick(app: &mut App) {
    super::install_emission(app);
    app.install_extension_submitted_request::<ProjectileSpawnPort, _>(
        MODULE_ENTITY_TICK,
        "ambition_combat",
        lower_projectile_spawns::<InModuleEntityTick>,
    );
}

/// Install the same request port in `boss_conduct` (a conducted boss throws).
pub fn install_for_boss_conduct(app: &mut App) {
    super::install_emission(app);
    app.install_extension_submitted_request::<ProjectileSpawnPort, _>(
        BOSS_CONDUCT,
        "ambition_combat",
        lower_projectile_spawns::<InBossConduct>,
    );
}

/// A hit reaction can emit a counter shot after this tick's materializer.
pub fn install_for_hit_reaction(app: &mut App) {
    super::install_emission(app);
    app.install_extension_submitted_request::<ProjectileSpawnPort, _>(
        HIT_REACTION,
        "ambition_combat",
        lower_projectile_spawns::<InHitReaction>,
    );
}

type SpawnState = SystemState<super::ProjectileWriter<'static, 'static>>;
fn lower_projectile_spawns<L: LowersIn>(world: &mut World, mut state: Local<Option<SpawnState>>) {
    let requests = world
        .resource_mut::<ExtensionOutbox>()
        .drain::<ProjectileSpawnPort>(&L::PHASE);
    if requests.is_empty() {
        return;
    }
    let state = state.get_or_insert_with(|| SystemState::new(world));
    apply_projectile_spawns(world, state, requests);
}

/// Validate, emit and answer each drained projectile request, in submission
/// order. One body for every phase: the phase wrapper above only drains its
/// own port and keeps its own lazily made writer state.
fn apply_projectile_spawns(
    world: &mut World,
    state: &mut SpawnState,
    requests: Vec<ambition_extension_host::Submitted<<ProjectileSpawnPort as ambition_extension_sdk::Port>::Value>>,
) {
    for submitted in requests {
        let dispatch = ambition_platformer2d_shared_tangle::action_results::begin_action_request::<ProjectileSpawnPort>(
            world, submitted.state_scope, &submitted.entry, &submitted.value);
        if !dispatch.execute {
            continue;
        }
        let missing_identity = dispatch.reply.is_some()
            && (world.get::<SimId>(submitted.scope).is_none()
                || world.get::<SimIdCounter>(submitted.scope).is_none());
        let value = submitted.value;
        let invalid_shape = !value.origin.is_finite()
            || !value.dir.is_finite()
            || !value.half_extent.is_finite()
            || value.half_extent.min_element() <= 0.0
            || !value.speed.is_finite()
            || !value.gravity.is_finite()
            || !value.max_lifetime.is_finite()
            || !value.splash_half_extent.is_finite()
            || value
                .boomerang_return_s
                .is_some_and(|time| !time.is_finite());
        if missing_identity || invalid_shape {
            let code = if missing_identity {
                "unidentified_emitter"
            } else {
                "invalid_projectile_shape"
            };
            publish_action_result(
                world,
                dispatch.reply.as_ref(),
                ActionStage::Rejected,
                true,
                code,
                "",
                Vec::new(),
            );
            continue;
        }
        let mut request =
            ProjectileSpawnRequest::open(submitted.scope, value, ProjectileStart::StepThisTick)
                .fired_by_move_if_any(submitted.occurrence);
        request.reply = dispatch.reply.clone();
        let emitted = state
            .get_mut(world)
            .is_ok_and(|mut writer| writer.write(request));
        state.apply(world);
        publish_action_result(
            world,
            dispatch.reply.as_ref(),
            if emitted {
                ActionStage::Admitted
            } else {
                ActionStage::Rejected
            },
            !emitted,
            if emitted {
                "projectile_queued"
            } else {
                "emission_refused"
            },
            "",
            Vec::new(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::SPAWN_DOMAIN;
    use ambition_extension_sdk::action::ReplyTo;
    use ambition_platformer2d_shared_tangle::{
        action_results::{ActionMailbox, begin_action, cancel_action},
        lifecycle::{
            InRoomInstance, LiveRoomInstance, RoomInstanceRoot, SessionScopeId, SessionScopedEntity,
        },
    };
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn creation_is_cancelable_while_queued_and_survives_observer_retirement() {
        for case in 0..3 {
            let mut app = App::new();
            app.init_resource::<super::super::PendingProjectileEmissions>()
                .add_message::<super::super::ProjectileEmission>()
                .init_resource::<ambition_projectiles::ProjectileSeqCounter>();
            super::super::install_emission(&mut app);
            app.add_systems(Update, super::super::materialize_projectiles_for_next_tick);
            let session = SessionScopeId(1);
            let room = LiveRoomInstance::ACTIVATION;
            app.world_mut()
                .spawn((RoomInstanceRoot, room, SessionScopedEntity(session)));
            let emitter = app
                .world_mut()
                .spawn((
                    SimId::placement("emitter"),
                    SessionScopedEntity(session),
                    InRoomInstance(room),
                ))
                .id();
            let reply = begin_action(
                app.world_mut(),
                emitter,
                "test::shots/fire",
                SPAWN_DOMAIN,
                Some(&ReplyTo {
                    request: 1,
                    selector: "result".into(),
                }),
                7,
            )
            .reply
            .unwrap();
            let mut request = ProjectileSpawnRequest::named(
                emitter,
                ambition_projectiles::InFlightProjectile {
                    body: ambition_projectiles::ProjectileBody::from_spec(
                        ambition_projectiles::ProjectileKind::Fireball.spec(
                            bevy::math::Vec2::ZERO,
                            bevy::math::Vec2::X,
                            1.0,
                        ),
                    ),
                },
                ambition_projectiles::ProjectileKind::Fireball,
                ProjectileStart::StepNextTick,
            );
            request.reply = Some(reply.clone());
            assert!(
                app.world_mut()
                    .run_system_once_with(
                        |In(request), mut writer: super::super::ProjectileWriter| writer
                            .write(request),
                        request,
                    )
                    .unwrap()
            );
            assert!(
                !app.world_mut()
                    .query_filtered::<Entity, With<ambition_projectiles::LiveProjectile>>()
                    .iter(app.world())
                    .any(|_| true)
            );
            match case {
                1 => {
                    app.world_mut()
                        .entity_mut(emitter)
                        .insert(InRoomInstance(LiveRoomInstance::from_ordinal(1)));
                    assert_eq!(cancel_action(app.world_mut(), emitter, &reply.ticket), Ok(()));
                }
                2 => {
                    app.world_mut().despawn(emitter);
                }
                _ => {}
            }
            app.update();
            let count = app
                .world_mut()
                .query_filtered::<Entity, With<ambition_projectiles::LiveProjectile>>()
                .iter(app.world())
                .count();
            assert_eq!(count, usize::from(case != 1));
            if case == 0 {
                let result = app
                    .world()
                    .get::<ActionMailbox>(emitter)
                    .unwrap()
                    .history()
                    .entries
                    .pop()
                    .unwrap()
                    .result;
                assert_eq!(result.stage, ActionStage::Applied);
                assert!(result.terminal);
                let world = app.world_mut();
                let id = world
                    .query_filtered::<&SimId, With<ambition_projectiles::LiveProjectile>>()
                    .single(world)
                    .unwrap();
                assert_eq!(result.subjects.len(), 1);
                assert_eq!(result.subjects[0].key, id.as_str());
                assert!(result.subjects[0].object.is_some());
                assert_eq!(cancel_action(app.world_mut(), emitter, &reply.ticket),
                    Err(ambition_platformer2d_shared_tangle::action_results::ActionCancellationRefusal::AlreadyTerminal));
            }
        }
    }
}
