//! What a boss's slow-motion finish looks like
//! (`ambition_boss_encounter::finish`): while the world hangs the picture
//! loses its colour but the boss, and cinema bars come in from the top and
//! the bottom of the screen. Both follow how deep in its finish the boss is
//! (`BossRenderView::finish`), so they go as the world comes back to speed.
//!
//! The drain is the void field's pass (`crate::void_field`): a wide aura on
//! the boss takes the colour out, and a ward on it keeps the boss's own. With
//! screen shaders off only the bars and the slow motion are left.

use std::collections::HashMap;

use bevy::camera::{CameraUpdateSystems, RenderTarget};
use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, LiveRooms, SessionScopeId, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sim_view::{BossRenderIndex, FeatureViewIndex, LocalView, PresentsView, ViewsOnHand};

use super::retained_entities::RetainedEntities;
use super::{GameplayOutputCamera, PresentedViewFrames};
use crate::void_field::{VoidAura, VoidFieldPlugin, VoidWard};

/// The aura reaches past any screen; its middle drains this much.
const DRAIN_RADIUS: f32 = 8000.0;
const DRAIN: f32 = 0.9;
/// The ward keeps this many of the boss's body sizes in colour.
const WARD_REACH: f32 = 1.0;
/// Each bar comes in this far, as a part of the screen's height.
const BAR: f32 = 0.11;

/// The drain on a visual that this module put there (a void thing's own
/// aura is not this module's to take away).
#[derive(Component)]
pub struct FinishDrain;

/// A cinema bar of a finish.
#[derive(Component)]
pub struct FinishBar;

type BarKey = (Option<SessionScopeId>, Entity, bool);

/// Install the finish's look. The drain needs the void field's pass.
pub fn install(app: &mut App) {
    if !app.is_plugin_added::<VoidFieldPlugin>() && app.get_sub_app(bevy::render::RenderApp).is_some() {
        app.add_plugins(VoidFieldPlugin);
    }
    // A finish is a named boss's: asleep where no live room's game has one,
    // after the frame that takes the drain and the bars away.
    app.add_systems(
        Update,
        drain_around_finishing_bosses
            .run_if(super::boss_stage::bosses_are_shown_or_were)
            .after(ambition_sim_view::PresentedPoseSet)
            .in_set(ambition_platformer2d_shared_tangle::capability::CapabilityPresentationSet(ambition_platformer2d_shared_tangle::capability::Capability::BossEncounters)),
    );
    app.add_systems(
        PostUpdate,
        show_finish_bars
            .run_if(super::boss_stage::bosses_are_shown_or_were)
            .after(CameraUpdateSystems)
            .before(bevy::ui::UiSystems::Prepare)
            .in_set(ambition_platformer2d_shared_tangle::capability::CapabilityPresentationSet(ambition_platformer2d_shared_tangle::capability::Capability::BossEncounters)),
    );
}

/// Give each boss visual in its finish the drain about it, and take it away
/// when the finish is over.
pub fn drain_around_finishing_bosses(
    mut commands: Commands,
    // `Option`: a composition with no sim read model has no boss to finish.
    bosses: Option<Res<BossRenderIndex>>,
    features: Option<Res<FeatureViewIndex>>,
    rooms: LiveRooms,
    visuals: Query<(Entity, &super::FeatureVisual, Has<FinishDrain>)>,
) {
    for (entity, visual, drained) in &visuals {
        let room = rooms.of(entity);
        let finish = bosses.as_deref().and_then(|bosses| bosses.get_in(&visual.id, room)).map_or(0.0, |boss| boss.finish);
        if finish > 0.0 {
            let size = features
                .as_deref()
                .and_then(|features| features.get_in(&visual.id, room))
                .map_or(100.0, |feature| feature.size.max_element());
            commands.entity(entity).try_insert((
                FinishDrain,
                VoidAura { radius: DRAIN_RADIUS, strength: DRAIN * finish },
                VoidWard { radius: size * WARD_REACH, strength: finish },
            ));
        } else if drained {
            commands.entity(entity).try_remove::<(FinishDrain, VoidAura, VoidWard)>();
        }
    }
}

/// Draw the finish bars inside each output of the view.
fn show_finish_bars(
    mut commands: Commands,
    mut bars: Query<&mut Node, With<FinishBar>>,
    mut retained: Local<RetainedEntities<BarKey>>,
    bosses: Option<Res<BossRenderIndex>>,
    frames: Option<Res<PresentedViewFrames>>,
    views: Query<Entity, With<LocalView>>,
    cameras: Query<(Entity, &Camera, &RenderTarget, Option<&PresentsView>), GameplayOutputCamera>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    let scope = SessionSpawnScope::for_optional_active_session(active_session.as_deref());
    let mut rooms = HashMap::new();
    if let Some(bosses) = bosses.as_deref() {
        for (_, boss) in bosses.iter() {
            if let Some(room) = boss.room.filter(|_| boss.finish > 0.0) {
                let height: &mut f32 = rooms.entry(room).or_default();
                *height = height.max(boss.finish);
            }
        }
    }
    let bound = ViewsOnHand::survey(views.iter());
    let mut wanted = HashMap::new();
    if let (Some(scope), Some(frames)) = (scope, frames.as_deref()) {
        for (entity, camera, target, link) in &cameras {
            if !camera.is_active || !matches!(target, RenderTarget::Window(_) | RenderTarget::Image(_)) {
                continue;
            }
            let Some(frame) = bound.presented_by(link.copied()).filter(|view| views.contains(*view))
                .and_then(|view| frames.of(view))
            else {
                continue;
            };
            let Some(finish) = rooms.get(&frame.room) else {
                continue;
            };
            let height = Val::Percent(100.0 * BAR * (1.0 - (1.0 - finish).powi(2)));
            for top in [true, false] {
                wanted.insert((scope.id(), entity, top), height);
            }
        }
    }

    retained.reconcile(&mut commands, |key| wanted.contains_key(key));
    let Some(scope) = scope else {
        return;
    };
    for (key @ (_, camera, top), height) in wanted {
        if let Some(mut bar) = retained.get(&key).and_then(|entity| bars.get_mut(*entity).ok()) {
            if bar.height != height {
                bar.height = height;
            }
            continue;
        }
        let entity = commands.spawn_session_scoped(
            scope,
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: if top { Val::Px(0.0) } else { Val::Auto },
                    bottom: if top { Val::Auto } else { Val::Px(0.0) },
                    height,
                    ..default()
                },
                BackgroundColor(Color::BLACK),
                UiTargetCamera(camera),
                Pickable::IGNORE,
                Name::new("Boss finish bar"),
                FinishBar,
            ),
        ).id();
        retained.insert(key, entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
    use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
    use ambition_sim_view::BossRenderView;
    use crate::rendering::PresentedViewFrame;

    #[test]
    fn the_finish_bars_follow_active_outputs_in_the_bosss_room() {
        let mut app = App::new();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::ZERO));
        app.add_systems(PostUpdate, show_finish_bars);
        let arena = LiveRoomInstance::ACTIVATION;
        let yard = arena.next();
        let world = app.world_mut();
        let frame = |room| PresentedViewFrame {
            room, plane_center: Vec2::ZERO, roll_radians: 0.0, orthographic_scale: 1.0,
        };
        let [(arena_view, plane, arena_frame), (yard_view, yard_camera, yard_frame)] = [arena, yard].map(|room| {
            let view = world.spawn(LocalView).id();
            let plane = world.spawn((Camera2d, MainCamera, PresentsView(view))).id();
            (view, plane, frame(room))
        });
        world.insert_resource(PresentedViewFrames::of_views(vec![(arena_view, arena_frame), (yard_view, yard_frame)]));
        let bosses = |finish| BossRenderIndex::from_rows([(
            "sentinel".into(),
            BossRenderView {
                room: Some(arena),
                name: "Sentinel".into(),
                behavior_id: "sentinel".into(),
                title: None,
                stage: Default::default(),
                awake: true,
                fighting: false,
                finish,
            },
        )]);
        world.insert_resource(bosses(1.0));

        let bars = |app: &mut App| {
            let world = app.world_mut();
            let mut rows = world.query_filtered::<(Entity, &UiTargetCamera, &Node), With<FinishBar>>()
                .iter(world).map(|(id, camera, node)| (id, camera.entity(), node.height)).collect::<Vec<_>>();
            rows.sort_unstable_by_key(|(id, _, _)| *id);
            rows
        };
        app.update();
        let first = bars(&mut app);
        assert_eq!(first.len(), 2);
        assert!(first.iter().all(|(_, camera, height)| *camera == plane && *height == Val::Percent(100.0 * BAR)));
        assert!(!first.iter().any(|(_, camera, _)| *camera == yard_camera));

        app.insert_resource(bosses(0.5));
        app.update();
        assert_eq!(bars(&mut app), first.iter().map(|(id, camera, _)| (*id, *camera, Val::Percent(75.0 * BAR))).collect::<Vec<_>>());

        #[cfg(feature = "card_view")]
        {
            use crate::rendering::card_view::CardViewCamera;
            let card = app.world_mut().spawn((Camera3d::default(), CardViewCamera, PresentsView(arena_view))).id();
            app.update();
            assert_eq!(bars(&mut app).iter().filter(|(_, camera, _)| *camera == card).count(), 2);
            app.world_mut().get_mut::<Camera>(plane).unwrap().is_active = false;
            app.update();
            let rows = bars(&mut app);
            assert_eq!(rows.len(), 2);
            assert!(rows.iter().all(|(_, camera, _)| *camera == card));
        }

        app.world_mut().resource_mut::<PresentedViewFrames>().of_mut(arena_view).unwrap().room = yard;
        app.update();
        assert!(bars(&mut app).is_empty());
    }
}
