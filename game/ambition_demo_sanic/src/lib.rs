//! Sanic-style demo content home.
//!
//! This crate intentionally depends only on the `ambition` facade crate. It is
//! the E9 engine-for-other-games ORACLE: a second platformer's content is
//! authored entirely through the umbrella surface, never by reaching into a
//! lower `ambition_*` crate or copying `game/ambition_app`'s dependency wall.
//! If authoring a room here needs a type the umbrella does not re-export, that
//! is a real engine leak — and it fails to compile HERE, which is the point.
//!
//! What lives here is the SHOWCASE GEOMETRY (a long momentum speedway with a
//! rideable Sonic loop). The movement-identity FEEL (momentum tuning, the
//! playable binary, character art) is a separate interactive build — it cannot
//! be dialed in headlessly — but the room a Sanic demo runs on is authored and
//! verified here through the engine's public surface.

use ambition::engine_core as ae;
use ambition::prelude::*;
use ambition::world::rooms::RoomSpec;

/// Stable room id for the momentum speedway.
pub const SPEEDWAY_ROOM_ID: &str = "sanic_speedway";

/// Number of segments in the generated Sonic loop polygon.
const LOOP_SEGMENTS: usize = 24;

/// Build the Sanic momentum showcase room through the `ambition` umbrella
/// surface ONLY: a wide room with a long solid floor and a rideable full LOOP
/// authored as a `SurfaceChain` (the momentum-locomotion geometry — a fast body
/// rides up the inside of the loop, across the top, and back down).
///
/// The loop winds with DECREASING angle so each segment's `(t.y, -t.x)` normal
/// points toward the loop center (interior-rideable), matching the engine's
/// `SurfaceLoop` marker convention.
pub fn sanic_speedway() -> RoomSpec {
    let width = 4000.0;
    let height = 720.0;
    let floor_top = height - 48.0;

    // A single long solid floor spanning the room, plus a spawn just above it.
    let floor = ae::Block::solid(
        "speedway_floor",
        ae::Vec2::new(0.0, floor_top),
        ae::Vec2::new(width, 48.0),
    );
    let spawn = ae::Vec2::new(160.0, floor_top - 64.0);

    // The Sonic loop: a closed polygon centered over the floor, interior-rideable.
    let loop_center = ae::Vec2::new(width * 0.5, floor_top - 200.0);
    let loop_radius = 180.0;
    let loop_points: Vec<ae::Vec2> = (0..LOOP_SEGMENTS)
        .map(|k| {
            let theta = -std::f32::consts::TAU * (k as f32) / (LOOP_SEGMENTS as f32);
            loop_center + ae::Vec2::new(theta.cos(), theta.sin()) * loop_radius
        })
        .collect();
    let sonic_loop = ae::SurfaceChain::closed_loop("sanic_loop", loop_points);

    let world = ae::World::new(
        "Sanic Speedway",
        ae::Vec2::new(width, height),
        spawn,
        vec![floor],
    )
    .with_chains(vec![sonic_loop]);

    RoomSpec::new(SPEEDWAY_ROOM_ID, world)
}

/// First-cut content plugin for the Sanic movement demo home. Room authoring is
/// exposed as [`sanic_speedway`]; wiring it into a running app (the playable
/// binary + momentum-feel tuning) is the separate interactive build.
pub struct SanicDemoContentPlugin;

impl Plugin for SanicDemoContentPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Install the Sanic demo content layer into an engine app.
pub fn add_demo_content(app: &mut App) {
    app.add_plugins(SanicDemoContentPlugin);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanic_demo_content_plugin_installs() {
        let mut app = App::new();
        add_demo_content(&mut app);
    }

    /// The oracle: the momentum showcase room composes through the umbrella
    /// surface alone — floor geometry present, the Sonic loop validates, and the
    /// spawn sits inside the room bounds.
    #[test]
    fn sanic_speedway_composes_through_the_umbrella() {
        let room = sanic_speedway();
        assert_eq!(room.id, SPEEDWAY_ROOM_ID);

        // Solid floor geometry made it into the world.
        assert!(
            room.world.blocks.iter().any(|b| b.name == "speedway_floor"),
            "the speedway floor block is present"
        );

        // The Sonic loop is a valid rideable closed chain (the engine's own
        // validator runs here, so a degenerate/self-intersecting loop fails).
        let loop_chain = room
            .world
            .chains
            .iter()
            .find(|c| c.name == "sanic_loop")
            .expect("the sanic loop chain is present");
        assert_eq!(loop_chain.points.len(), LOOP_SEGMENTS);
        assert!(
            loop_chain.validate().is_empty(),
            "the generated Sonic loop is a valid rideable chain: {:?}",
            loop_chain.validate()
        );

        // Spawn is inside the room bounds (not floating/falling on load).
        let s = room.world.spawn;
        assert!(
            s.x >= 0.0 && s.x <= room.world.size.x && s.y >= 0.0 && s.y <= room.world.size.y,
            "spawn {s:?} is inside room bounds {:?}",
            room.world.size
        );
    }
}
