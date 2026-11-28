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
//! [`PresentationSettling`] names each one, each frame, with the live room it
//! is in, or none for what is the whole App's (a pipeline that compiles). A
//! cover over one room lifts on a frame where nothing of that room and
//! nothing global waits ([`PresentationSettling::pending_for`]). The render
//! cache gives new pipelines a short time to compile. Older work stays in
//! the cache and can finish after the cover lifts. A system that knows of more
//! (a room look that dresses the room's doors) adds to it in `Update` with
//! [`PresentationSettling::hold_in`], before the covers read it
//! ([`PresentationSettlingSet`]).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bevy::platform::{collections::HashMap, time::Instant};
use bevy::prelude::*;
use bevy::render::render_resource::{PipelineCache, PipelineDescriptor};

use ambition_sprite_sheet::game_assets::GameAssets;

use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRooms};

use super::features::UnclaimedFeatureViews;
use super::primitives::EntityArt;
use super::terrain_skin::{RoomTerrain, TerrainSkinned, ThemedDoor, ThemedFixture};

/// What is not drawn yet, this frame. Cleared in `First`; each writer adds to
/// it in `Update`, before [`PresentationSettlingSet`] ends.
#[derive(Resource, Default, Debug)]
pub struct PresentationSettling {
    /// What is not drawn, and the live room it is in (`None`: the whole
    /// App's, or a room that cannot be told).
    pending: Vec<(Option<LiveRoomInstance>, String)>,
}

impl PresentationSettling {
    /// Say that `what`, which is the whole App's, is not drawn yet.
    pub fn hold(&mut self, what: impl Into<String>) {
        self.hold_in(None, what);
    }

    /// Say that `what`, in live room `room`, is not drawn yet. `None` holds
    /// every cover.
    pub fn hold_in(&mut self, room: Option<LiveRoomInstance>, what: impl Into<String>) {
        self.pending.push((room, what.into()));
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Everything that is not drawn yet, for a report that has to say which.
    pub fn pending(&self) -> Vec<&str> {
        self.pending.iter().map(|(_, what)| what.as_str()).collect()
    }

    /// What a cover over `rooms` waits on: what is in one of them, and what
    /// is the whole App's. A cover that cannot name its rooms (`rooms`
    /// empty) waits on everything.
    pub fn pending_for(&self, rooms: &[LiveRoomInstance]) -> Vec<&str> {
        self.pending
            .iter()
            .filter(|(room, _)| rooms.is_empty() || room.is_none_or(|room| rooms.contains(&room)))
            .map(|(_, what)| what.as_str())
            .collect()
    }
}

/// Where [`PresentationSettling`] is filled. A cover reads it after this set.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PresentationSettlingSet;

/// The number of pipelines still in their initial compilation period.
/// Shared between the main world and the render world.
#[derive(Resource, Clone, Default, Debug)]
pub struct RenderPipelineWarmup(Arc<AtomicUsize>);

impl RenderPipelineWarmup {
    pub fn count(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}

/// The cache can retry an unavailable shader on every frame. Limit the time
/// one continuous wait can delay a cover.
const PIPELINE_WARMUP_GRACE: Duration = Duration::from_millis(250);

#[derive(Default)]
struct PipelineWaits {
    previous: HashMap<usize, PipelineWait>,
    current: HashMap<usize, PipelineWait>,
}

struct PipelineWait {
    since: Instant,
    reported: bool,
}

impl PipelineWaits {
    fn sample(
        &mut self,
        now: Instant,
        waiting: impl IntoIterator<Item = usize>,
        mut report: impl FnMut(usize, Duration),
    ) -> usize {
        let mut count = 0;
        for id in waiting {
            let mut wait = self.previous.remove(&id).unwrap_or(PipelineWait {
                since: now,
                reported: false,
            });
            let elapsed = now.duration_since(wait.since);
            if elapsed < PIPELINE_WARMUP_GRACE {
                count += 1;
            } else if !wait.reported {
                report(id, elapsed);
                wait.reported = true;
            }
            self.current.insert(id, wait);
        }
        self.previous.clear();
        std::mem::swap(&mut self.previous, &mut self.current);
        count
    }
}

fn sample_pipeline_warmup(
    cache: Res<PipelineCache>,
    warmup: Res<RenderPipelineWarmup>,
    mut waits: Local<PipelineWaits>,
) {
    let count = waits.sample(Instant::now(), cache.waiting_pipelines(), |id, elapsed| {
        if let Some(pipeline) = cache.pipelines().nth(id) {
            let label = match &pipeline.descriptor {
                PipelineDescriptor::RenderPipelineDescriptor(desc) => desc.label.as_deref(),
                PipelineDescriptor::ComputePipelineDescriptor(desc) => desc.label.as_deref(),
            };
            warn!(
                "pipeline {id} ({}) still pending after {:.0} ms. Its cover warm-up period expired. State {:?}",
                label.unwrap_or("unlabeled"),
                elapsed.as_secs_f64() * 1000.0,
                pipeline.state,
            );
        }
    });
    warmup.0.store(count, Ordering::Relaxed);
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
    // The room of an undrawn view: its row says it.
    views: Option<Res<ambition_sim_view::FeatureViewIndex>>,
    pipelines: Option<Res<RenderPipelineWarmup>>,
    terrains: Query<(Entity, &RoomTerrain)>,
    // A door with art of its own (a room look's door) does not take its
    // theme's (`dress_themed_doors`): it is not waited on for it.
    doors: Query<(Entity, &ThemedDoor), (Without<TerrainSkinned>, Without<EntityArt>)>,
    fixtures: Query<(Entity, &ThemedFixture), Without<TerrainSkinned>>,
    rooms: LiveRooms,
) {
    if let Some(unclaimed) = unclaimed {
        for id in unclaimed.ids() {
            let room = views.as_deref().and_then(|views| views.get(id)).and_then(|view| view.room);
            settling.hold_in(room, format!("feature view {id}"));
        }
    }
    // A theme not asked for yet is not waited on: what asks for it (the room's
    // preparation) has not run, and a cover must not wait for what nothing will
    // bring.
    let attempted = |theme| assets.as_ref().is_some_and(|assets| assets.room_dressing.attempted(theme));
    for (entity, terrain) in &terrains {
        if !terrain.is_laid() && attempted(terrain.theme) {
            settling.hold_in(rooms.of(entity), format!("terrain skin {}", terrain.theme.key()));
        }
    }
    for (entity, ThemedDoor(theme)) in &doors {
        if attempted(*theme) {
            settling.hold_in(rooms.of(entity), format!("door art {}", theme.key()));
        }
    }
    for (entity, fixture) in &fixtures {
        if attempted(fixture.theme) {
            settling.hold_in(rooms.of(entity), format!("fixture art {}", fixture.theme.key()));
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
        let warmup = RenderPipelineWarmup::default();
        app.init_resource::<PresentationSettling>()
            .insert_resource(warmup.clone())
            .configure_sets(Update, PresentationSettlingSet.after(Platformer2dSimulationPhase::PresentationVisualSync))
            .add_systems(First, forget_last_frame)
            .add_systems(Update, hold_for_the_renderer.in_set(PresentationSettlingSet));
        if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
            render_app.insert_resource(warmup).add_systems(
                bevy::render::Render,
                sample_pipeline_warmup.after(bevy::render::RenderSystems::Render),
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
    fn an_old_pipeline_stops_holding_reveals_while_a_new_one_can_wait() {
        let mut waits = PipelineWaits::default();
        let start = Instant::now();
        let later = start + PIPELINE_WARMUP_GRACE;
        let mut expired = Vec::new();
        let mut report = |id, _| expired.push(id);
        assert_eq!(waits.sample(start, [1], &mut report), 1);
        assert_eq!(waits.sample(later, [1, 2], &mut report), 1);
        assert_eq!(waits.sample(later, [1], &mut report), 0);
        assert_eq!(waits.sample(later, [], &mut report), 0);
        assert_eq!(waits.sample(later, [1], &mut report), 1);
        assert_eq!(expired, vec![1]);
    }

    /// Review integration check (2026-10-19): a cover over one room waits on
    /// that room and on what is the whole App's, not on another live room.
    /// Control: a cover that cannot name its rooms waits on everything.
    #[test]
    fn a_cover_over_one_room_waits_on_that_room_only() {
        let (here, there) = (LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next());
        let mut settling = PresentationSettling::default();
        settling.hold_in(Some(there), "door art there");
        settling.hold_in(Some(here), "door art here");
        settling.hold("1 render pipeline(s) compiling");
        assert_eq!(settling.pending_for(&[here]), vec!["door art here", "1 render pipeline(s) compiling"]);
        assert_eq!(settling.pending_for(&[]).len(), 3, "control: a cover with no rooms waits on all");
    }

    /// An undrawn feature view is held in the room its row names, and one
    /// whose room cannot be told holds every cover. Poison: hold every view
    /// as the whole App's.
    #[test]
    fn an_undrawn_view_is_held_in_its_own_room() {
        let (here, there) = (LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next());
        let mut app = App::new();
        app.init_resource::<PresentationSettling>();
        app.insert_resource(UnclaimedFeatureViews::of(&["a_there", "b_here", "c_nowhere"]));
        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([
            ("a_there".to_string(), actor_in(Some(there))),
            ("b_here".to_string(), actor_in(Some(here))),
            ("c_nowhere".to_string(), actor_in(None)),
        ]));
        app.add_systems(Update, hold_for_the_renderer);
        app.update();
        let settling = app.world().resource::<PresentationSettling>();
        assert_eq!(settling.pending_for(&[here]), vec!["feature view b_here", "feature view c_nowhere"]);
    }

    fn actor_in(room: Option<LiveRoomInstance>) -> ambition_sim_view::FeatureView {
        use ambition_platformer2d_core as ae;
        ambition_sim_view::FeatureView {
            pos: ae::Vec2::ZERO,
            size: ae::Vec2::new(20.0, 40.0),
            kind: ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind::Actor,
            visible: true,
            submerged: false,
            wire_anchor: None,
            grab_reach: None,
            line_anchor: None,
            limb_host: None,
            depth_plane: ae::DepthPlane::PLAYABLE,
            flash: false,
            breakable_state: None,
            chest_opened: false,
            fighting: false,
            switch_on: false,
            material: Default::default(),
            alive: true,
            hit_flash_secs: 0.0,
            parry_flash_secs: 0.0,
            hp_current: 1,
            hp_max: 1,
            training_dummy: false,
            armored: false,
            hit_strength: 0.0,
            unhittable: false,
            defense_cues: ambition_sim_view::DefenseCueCauses::NONE,
            sprite_offset: None,
            room,
            order: Default::default(),
        }
    }

    #[test]
    fn a_cover_lifts_on_the_second_settled_update_in_a_row() {
        let mut latch = SettledLatch::default();
        assert!(!latch.observe(1, true));
        assert!(!latch.observe(2, false), "a hold resets it");
        assert!(!latch.observe(3, true));
        assert!(latch.observe(4, true));
    }
}
