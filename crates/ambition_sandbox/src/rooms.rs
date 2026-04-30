//! Sandbox room-set and loading-zone definitions.
//!
//! The engine still models one `World` at a time. This Bevy adapter owns the
//! first multi-room shell: a larger scrolling movement lab plus a disconnected
//! chamber reached through loading zones. Keeping transitions sandbox-side for
//! now lets us test camera/room feel before promoting room graphs into the
//! pure engine crate.

use ambition_engine as ae;
use bevy::prelude::Resource;

/// A non-colliding rectangular trigger that swaps the active room.
#[derive(Clone, Debug)]
pub struct LoadingZone {
    pub name: &'static str,
    pub aabb: ae::Aabb,
    pub target_room: usize,
    pub target_spawn: ae::Vec2,
}

/// Complete room data used by the Bevy sandbox.
#[derive(Clone, Debug)]
pub struct RoomSpec {
    pub world: ae::World,
    pub loading_zones: Vec<LoadingZone>,
}

/// Small room graph for early loading-zone tests.
#[derive(Resource, Clone, Debug)]
pub struct RoomSet {
    pub rooms: Vec<RoomSpec>,
    pub active: usize,
}

impl RoomSet {
    pub fn new() -> Self {
        Self {
            rooms: vec![build_scroll_lab(), build_disconnected_chamber()],
            active: 0,
        }
    }

    pub fn active_spec(&self) -> &RoomSpec {
        &self.rooms[self.active]
    }

    pub fn active_world(&self) -> &ae::World {
        &self.active_spec().world
    }

    pub fn active_loading_zones(&self) -> &[LoadingZone] {
        &self.active_spec().loading_zones
    }

    pub fn set_active(&mut self, index: usize) -> &RoomSpec {
        self.active = index.min(self.rooms.len().saturating_sub(1));
        self.active_spec()
    }

    pub fn transition_for_player(&self, player: &ae::Player) -> Option<LoadingZone> {
        let body = player.aabb();
        self.active_loading_zones()
            .iter()
            .find(|zone| body.intersects(zone.aabb))
            .cloned()
    }
}

fn build_scroll_lab() -> RoomSpec {
    let world = ae::build_endgame_sandbox();
    let loading_zones = vec![LoadingZone {
        name: "loading zone: archive chamber",
        aabb: ae::Aabb::from_min_size(
            ae::Vec2::new(world.size.x - 300.0, world.size.y - 170.0),
            ae::Vec2::new(92.0, 122.0),
        ),
        target_room: 1,
        target_spawn: ae::Vec2::new(170.0, 760.0),
    }];
    RoomSpec { world, loading_zones }
}

fn build_disconnected_chamber() -> RoomSpec {
    let w = 1800.0;
    let h = 900.0;
    let mut blocks = Vec::new();

    blocks.push(ae::Block::solid("floor", ae::Vec2::new(0.0, h - 48.0), ae::Vec2::new(w, 48.0)));
    blocks.push(ae::Block::solid("left wall", ae::Vec2::new(0.0, 0.0), ae::Vec2::new(36.0, h)));
    blocks.push(ae::Block::solid("right wall", ae::Vec2::new(w - 36.0, 0.0), ae::Vec2::new(36.0, h)));
    blocks.push(ae::Block::solid("ceiling", ae::Vec2::new(0.0, 0.0), ae::Vec2::new(w, 24.0)));

    // A deliberately sparse room: disconnected from the main lab and focused
    // on proving transitions, camera recentering, blink walls, and return flow.
    blocks.push(ae::Block::one_way("archive lower shelf", ae::Vec2::new(260.0, 690.0), ae::Vec2::new(300.0, 18.0)));
    blocks.push(ae::Block::blink_wall("archive soft partition", ae::Vec2::new(660.0, 555.0), ae::Vec2::new(32.0, 250.0), ae::BlinkWallTier::Soft));
    blocks.push(ae::Block::solid("archive hard theorem wall", ae::Vec2::new(960.0, 515.0), ae::Vec2::new(42.0, 337.0)));
    blocks.push(ae::Block::one_way("archive high shelf", ae::Vec2::new(1100.0, 440.0), ae::Vec2::new(280.0, 18.0)));
    blocks.push(ae::Block::pogo_orb("archive pogo", ae::Vec2::new(1230.0, 365.0), 19.0));
    blocks.push(ae::Block::rebound(
        "archive return launcher",
        ae::Vec2::new(1390.0, 795.0),
        ae::Vec2::new(115.0, 24.0),
        ae::Vec2::new(-700.0, -650.0),
    ));
    blocks.push(ae::Block::hazard("archive low hazard", ae::Vec2::new(710.0, 830.0), ae::Vec2::new(210.0, 22.0)));

    let world = ae::World {
        name: "Ambition: Disconnected Archive Chamber",
        size: ae::Vec2::new(w, h),
        spawn: ae::Vec2::new(170.0, h - 95.0),
        blocks,
    };

    let loading_zones = vec![LoadingZone {
        name: "loading zone: scroll lab",
        aabb: ae::Aabb::from_min_size(ae::Vec2::new(42.0, h - 170.0), ae::Vec2::new(92.0, 122.0)),
        target_room: 0,
        target_spawn: ae::Vec2::new(2760.0, 805.0),
    }];

    RoomSpec { world, loading_zones }
}
