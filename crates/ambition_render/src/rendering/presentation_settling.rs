//! What the room on screen has still to draw: the answer a cover waits on.
//!
//! A room is shown when its images are loaded, but its look comes in over the
//! frames after that: the terrain lays its skin when its pictures are there,
//! a door and a fixture take their theme's art, a feature the sim published
//! waits for its render family, and a material the room draws for the first
//! time has a pipeline that compiles on another thread (the frames before it
//! is ready draw nothing for it). A cover that lifts on loaded images shows
//! each of those coming in: the room in its placeholder art, then its look.
//!
//! [`PresentationSettling`] names each one, each frame. A cover lifts on a
//! frame where it is empty. A system that knows of more (a room look that
//! dresses the room's doors) adds to it in `Update` with
//! [`PresentationSettling::hold`], before the covers read it
//! ([`PresentationSettlingSet`]).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use bevy::prelude::*;

use ambition_sprite_sheet::game_assets::GameAssets;

use super::features::UnclaimedFeatureViews;
use super::primitives::EntityArt;
use super::terrain_skin::{RoomTerrain, TerrainSkinned, ThemedDoor, ThemedFixture};

/// What is not drawn yet, this frame. Cleared in `First`; each writer adds to
/// it in `Update`, before [`PresentationSettlingSet`] ends.
#[derive(Resource, Default, Debug)]
pub struct PresentationSettling {
    pending: Vec<String>,
}

impl PresentationSettling {
    /// Say that `what` is not drawn yet.
    pub fn hold(&mut self, what: impl Into<String>) {
        self.pending.push(what.into());
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// What is not drawn yet, for a report that has to say which.
    pub fn pending(&self) -> &[String] {
        &self.pending
    }
}

/// An opaque cover over the room: while one is up, what is drawn is drawn
/// under it, not in front of the player (the image census reads it).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PresentationCover;

/// Where [`PresentationSettling`] is filled. A cover reads it after this set.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PresentationSettlingSet;

/// How many render pipelines this App's render world has asked for and not
/// yet built: shared between the main world and the render world by the
/// `Arc`, as `AppGpuPreparedImages` is.
#[derive(Resource, Clone, Default, Debug)]
pub struct RenderPipelinesWaiting(Arc<AtomicUsize>);

impl RenderPipelinesWaiting {
    pub fn count(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}

fn count_waiting_pipelines(cache: Res<bevy::render::render_resource::PipelineCache>, waiting: Res<RenderPipelinesWaiting>) {
    waiting.0.store(cache.waiting_pipelines().count(), Ordering::Relaxed);
}

fn forget_last_frame(mut settling: ResMut<PresentationSettling>) {
    settling.pending.clear();
}

/// What the renderer itself knows is not drawn: undrawn feature views, a
/// room's terrain that waits for its pictures, a door or a fixture whose
/// theme's art has not come, and pipelines that are compiling.
#[allow(clippy::too_many_arguments)]
fn hold_for_the_renderer(
    mut settling: ResMut<PresentationSettling>,
    assets: Option<Res<GameAssets>>,
    unclaimed: Option<Res<UnclaimedFeatureViews>>,
    pipelines: Option<Res<RenderPipelinesWaiting>>,
    terrains: Query<&RoomTerrain>,
    // A door with art of its own (a room look's door) does not take its
    // theme's (`dress_themed_doors`): it is not waited on for it.
    doors: Query<&ThemedDoor, (Without<TerrainSkinned>, Without<EntityArt>)>,
    fixtures: Query<&ThemedFixture, Without<TerrainSkinned>>,
) {
    if let Some(unclaimed) = unclaimed {
        settling.pending.extend(unclaimed.ids().map(|id| format!("feature view {id}")));
    }
    // A theme not asked for yet is not waited on: what asks for it (the room's
    // preparation) has not run, and a cover must not wait for what nothing will
    // bring.
    let attempted = |theme| assets.as_ref().is_some_and(|assets| assets.room_dressing.attempted(theme));
    for terrain in &terrains {
        if !terrain.is_laid() && attempted(terrain.theme) {
            settling.hold(format!("terrain skin {}", terrain.theme.key()));
        }
    }
    for ThemedDoor(theme) in &doors {
        if attempted(*theme) {
            settling.hold(format!("door art {}", theme.key()));
        }
    }
    for fixture in &fixtures {
        if attempted(fixture.theme) {
            settling.hold(format!("fixture art {}", fixture.theme.key()));
        }
    }
    let compiling = pipelines.map_or(0, |pipelines| pipelines.count());
    if compiling > 0 {
        settling.hold(format!("{compiling} render pipeline(s) compiling"));
    }
}

/// Installs [`PresentationSettling`] and the renderer's holds on it, and the
/// render-world count of compiling pipelines when the App has a render world.
pub struct PresentationSettlingPlugin;

impl Plugin for PresentationSettlingPlugin {
    fn build(&self, app: &mut App) {
        use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase;
        let waiting = RenderPipelinesWaiting::default();
        app.init_resource::<PresentationSettling>()
            .insert_resource(waiting.clone())
            .configure_sets(Update, PresentationSettlingSet.after(Platformer2dSimulationPhase::PresentationVisualSync))
            .add_systems(First, forget_last_frame)
            .add_systems(Update, hold_for_the_renderer.in_set(PresentationSettlingSet));
        if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
            render_app.insert_resource(waiting).add_systems(
                bevy::render::Render,
                count_waiting_pipelines.after(bevy::render::RenderSystems::Render),
            );
        }
    }
}

/// When a cover may lift: [`PresentationSettling`] was empty on two updates
/// in a row. One is not enough: the count of compiling pipelines is the
/// render world's last frame, and a thing that came into view on that frame
/// asks for its pipeline then.
#[derive(Clone, Copy, Debug, Default)]
pub struct SettledLatch {
    settled_since: Option<u64>,
}

impl SettledLatch {
    /// `settled` is this update's answer, and `update` its number. True when
    /// the cover may lift.
    pub fn observe(&mut self, update: u64, settled: bool) -> bool {
        if !settled {
            self.settled_since = None;
            return false;
        }
        let since = *self.settled_since.get_or_insert(update);
        update > since
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cover_lifts_on_the_second_settled_update_in_a_row() {
        let mut latch = SettledLatch::default();
        assert!(!latch.observe(1, true));
        assert!(!latch.observe(2, false), "a hold resets it");
        assert!(!latch.observe(3, true));
        assert!(latch.observe(4, true));
    }
}
