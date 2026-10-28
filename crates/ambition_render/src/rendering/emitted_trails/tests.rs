use super::*;
use ambition_sim_view::TrailStrip;

#[derive(Resource, Default)]
struct Picture(TrailSnapshot);

fn picture(picture: Res<Picture>) -> TrailSnapshot {
    picture.0.clone()
}

#[test]
fn trail_drawings_keep_their_bands_cache_and_retirement() {
    let mut app = App::new();
    app.init_resource::<Assets<bevy::gizmos::GizmoAsset>>();
    app.insert_resource(Picture(TrailSnapshot(
        [Some(0), Some(1)]
            .into_iter()
            .map(|band| TrailStrip {
                band,
                points: vec![Vec2::ZERO, Vec2::X * 40.0],
                state: TrailStripState::Emitting,
            })
            .collect(),
    )));
    app.add_systems(Update, picture.pipe(draw_emitted_trails));
    app.update();

    let drawings = |world: &mut World| {
        let mut rows: Vec<_> = world
            .query::<(
                Entity,
                &TrailDrawing,
                &bevy::camera::visibility::RenderLayers,
            )>()
            .iter(world)
            .map(|(entity, band, layers)| (band.0, entity, layers.clone()))
            .collect();
        rows.sort_by_key(|(band, ..)| *band);
        rows
    };
    let first = drawings(app.world_mut());
    assert_eq!(first.len(), 2);
    for (position, (band, _, layers)) in first.iter().enumerate() {
        assert_eq!(*band, Some(position));
        assert_eq!(
            *layers,
            bevy::camera::visibility::RenderLayers::none().with(
                ambition_platformer2d_shared_tangle::camera_layers::live_room_render_layer(
                    position
                ),
            )
        );
    }
    let assets_changed = app
        .world()
        .get_resource_ref::<Assets<bevy::gizmos::GizmoAsset>>()
        .unwrap()
        .last_changed();
    app.update();
    assert_eq!(drawings(app.world_mut()), first);
    assert_eq!(
        app.world()
            .get_resource_ref::<Assets<bevy::gizmos::GizmoAsset>>()
            .unwrap()
            .last_changed(),
        assets_changed,
        "a still trail does not rebuild its assets"
    );

    app.world_mut().resource_mut::<Picture>().0 = TrailSnapshot::default();
    app.update();
    assert!(drawings(app.world_mut()).is_empty());
}
