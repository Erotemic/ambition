//! The recall-mark beacon visual: a glowing beacon sprite at the player's
//! dropped mark. Render-only; reads the sim-side mark read model.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sim_view::MarkBeaconsView;
use bevy::prelude::*;

/// Marks the persistent beacon sprite shown at a player's dropped recall mark.
#[derive(Component)]
pub struct MarkBeaconVisual;

/// How far above the mark (player center) the beacon's center sits, so it
/// stands up from the spot instead of sinking into the floor.
const BEACON_RISE: f32 = 18.0;
/// In-world display size of the beacon sprite (3:7, matching the rendered prop).
const BEACON_SIZE: ae::Vec2 = ae::Vec2::new(30.0, 70.0);

/// One beacon per mark, removed when the mark is cleared. Visible build only.
///
/// The beacons are spawned again only when the marks changed or a beacon is
/// gone (a room or session teardown took it). They were spawned each frame,
/// and their art looked up by path each time.
pub fn sync_mark_beacon_visual(
    mut commands: Commands,
    // Each drawable is placed by the geometry of its own live room.
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<
        ambition_platformer2d_core::RoomGeometry,
    >,
    asset_server: Res<AssetServer>,
    active_session: Option<Res<ActiveSessionScope>>,
    visuals: Query<Entity, With<MarkBeaconVisual>>,
    marks: Res<MarkBeaconsView>,
    mut drawn: Local<Vec<(ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance, Vec3)>>,
) {
    let session_scope = SessionSpawnScope::for_optional_active_session(active_session.as_deref());
    // Where each beacon stands, in its own room.
    let mut wanted = Vec::new();
    for &(pos, room) in session_scope.iter().flat_map(|_| marks.0.iter()) {
        // A mark whose live room cannot be told is not drawn.
        let Some((room, world)) = room.and_then(|room| Some((room, rooms.in_room(room)?))) else {
            continue;
        };
        // +Y is down in world space, so "up" (toward the ceiling) is -Y.
        let translation = ambition_platformer2d_core::config::world_to_bevy(
            &world.0,
            pos - ae::Vec2::new(0.0, BEACON_RISE),
            7.0,
        );
        wanted.push((room, translation));
    }
    if *drawn == wanted && visuals.iter().count() == wanted.len() {
        return;
    }
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    drawn.clone_from(&wanted);
    let Some(session_scope) = session_scope else {
        return;
    };
    for (room, translation) in wanted {
        let mut sprite = Sprite::from_image(asset_server.load("sprites/props/mark_beacon.png"));
        sprite.custom_size = Some(BEACON_SIZE);
        commands.spawn_session_scoped(
            session_scope.in_room(Some(room)),
            (
                MarkBeaconVisual,
                sprite,
                Transform::from_translation(translation),
                Name::new("Mark beacon visual"),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;

    /// A mark's beacon is kept while the mark stays, and goes when it is
    /// cleared. It was spawned again each frame.
    #[test]
    fn a_beacon_is_kept_while_its_mark_stays() {
        let geometry = ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
            "mark room",
            ae::Vec2::new(800.0, 600.0),
            ae::Vec2::new(16.0, 16.0),
            Vec::new(),
        ));
        let mut app = App::new();
        app.add_plugins((bevy::app::TaskPoolPlugin::default(), bevy::asset::AssetPlugin::default()));
        app.init_asset::<Image>();
        let mut active = ActiveSessionScope::default();
        active.begin();
        app.insert_resource(active);
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), geometry);
        app.insert_resource(MarkBeaconsView(vec![(ae::Vec2::new(100.0, 200.0), Some(LiveRoomInstance::ACTIVATION))]));
        app.add_systems(Update, sync_mark_beacon_visual);
        app.update();
        let beacons = |app: &mut App| {
            app.world_mut().query_filtered::<Entity, With<MarkBeaconVisual>>().iter(app.world()).collect::<Vec<_>>()
        };
        let first = beacons(&mut app);
        assert_eq!(first.len(), 1, "one mark, one beacon");
        app.update();
        assert_eq!(beacons(&mut app), first, "a mark that stays keeps its beacon");
        app.world_mut().resource_mut::<MarkBeaconsView>().0.clear();
        app.update();
        assert!(beacons(&mut app).is_empty(), "a cleared mark takes its beacon");
    }
}
