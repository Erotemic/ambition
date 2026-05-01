//! Sandbox room-set and loading-zone definitions.
//!
//! The pure engine still simulates one `World` at a time. The Bevy adapter owns
//! this early room graph so we can iterate on camera, loading zones, and test
//! room shapes before deciding what belongs in `ambition_engine` permanently.

use ambition_engine as ae;
use bevy::prelude::Resource;

/// How a loading zone should be activated.
///
/// Edge exits are meant to feel like walking out of one room and into the next;
/// doors are explicit interact points and require pressing up while overlapping
/// the zone. Keeping this distinction in the data model avoids surprising
/// transitions from non-edge trigger volumes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadingZoneActivation {
    EdgeExit,
    Door,
}

impl LoadingZoneActivation {
    pub fn label(self) -> &'static str {
        match self {
            Self::EdgeExit => "edge exit",
            Self::Door => "door",
        }
    }
}

/// A non-colliding rectangular trigger that swaps the active room.
#[derive(Clone, Debug)]
pub struct LoadingZone {
    pub name: &'static str,
    pub activation: LoadingZoneActivation,
    pub aabb: ae::Aabb,
    pub target_room: usize,
    pub target_spawn: ae::Vec2,
}

impl LoadingZone {
    pub fn edge_exit(name: &'static str, min: ae::Vec2, size: ae::Vec2, target_room: usize, target_spawn: ae::Vec2) -> Self {
        Self {
            name,
            activation: LoadingZoneActivation::EdgeExit,
            aabb: ae::Aabb::from_min_size(min, size),
            target_room,
            target_spawn,
        }
    }

    pub fn door(name: &'static str, min: ae::Vec2, size: ae::Vec2, target_room: usize, target_spawn: ae::Vec2) -> Self {
        Self {
            name,
            activation: LoadingZoneActivation::Door,
            aabb: ae::Aabb::from_min_size(min, size),
            target_room,
            target_spawn,
        }
    }

    pub fn is_ready(&self, wants_interact: bool) -> bool {
        match self.activation {
            LoadingZoneActivation::EdgeExit => true,
            LoadingZoneActivation::Door => wants_interact,
        }
    }

    pub fn hint(&self) -> String {
        match self.activation {
            LoadingZoneActivation::EdgeExit => format!("{}: {}", self.activation.label(), self.name),
            LoadingZoneActivation::Door => format!("{}: {} (press up)", self.activation.label(), self.name),
        }
    }
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
            rooms: vec![
                build_central_hub(),
                build_scroll_lab(),
                build_vertical_shaft(),
                build_square_arena(),
                build_tiny_chamber(),
            ],
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

    pub fn transition_for_player(&self, player: &ae::Player, wants_interact: bool) -> Option<LoadingZone> {
        let body = player.aabb();
        self.active_loading_zones()
            .iter()
            .find(|zone| body.intersects(zone.aabb) && zone.is_ready(wants_interact))
            .cloned()
    }

    pub fn nearby_zone_hints(&self, player: &ae::Player) -> Vec<String> {
        let body = player.aabb();
        self.active_loading_zones()
            .iter()
            .filter(|zone| body.intersects(zone.aabb))
            .map(LoadingZone::hint)
            .collect()
    }
}

fn shell(blocks: &mut Vec<ae::Block>, w: f32, h: f32) {
    blocks.push(ae::Block::solid("floor", ae::Vec2::new(0.0, h - 48.0), ae::Vec2::new(w, 48.0)));
    blocks.push(ae::Block::solid("left wall", ae::Vec2::new(0.0, 0.0), ae::Vec2::new(36.0, h)));
    blocks.push(ae::Block::solid("right wall", ae::Vec2::new(w - 36.0, 0.0), ae::Vec2::new(36.0, h)));
    blocks.push(ae::Block::solid("ceiling", ae::Vec2::new(0.0, 0.0), ae::Vec2::new(w, 24.0)));
}

fn build_central_hub() -> RoomSpec {
    let w = 1800.0;
    let h = 1000.0;
    let mut blocks = Vec::new();
    shell(&mut blocks, w, h);

    blocks.push(ae::Block::one_way("hub center shelf", ae::Vec2::new(650.0, 720.0), ae::Vec2::new(500.0, 18.0)));
    blocks.push(ae::Block::one_way("hub upper shelf", ae::Vec2::new(745.0, 430.0), ae::Vec2::new(310.0, 18.0)));
    blocks.push(ae::Block::blink_wall("hub soft column", ae::Vec2::new(565.0, 615.0), ae::Vec2::new(32.0, 230.0), ae::BlinkWallTier::Soft));
    blocks.push(ae::Block::blink_wall("hub right soft column", ae::Vec2::new(1210.0, 610.0), ae::Vec2::new(32.0, 235.0), ae::BlinkWallTier::Soft));
    blocks.push(ae::Block::pogo_orb("hub routing note", ae::Vec2::new(900.0, 600.0), 18.0));
    blocks.push(ae::Block::rebound(
        "hub launcher",
        ae::Vec2::new(860.0, 905.0),
        ae::Vec2::new(120.0, 24.0),
        ae::Vec2::new(0.0, -760.0),
    ));

    let world = ae::World {
        name: "Ambition: Central Hub",
        size: ae::Vec2::new(w, h),
        spawn: ae::Vec2::new(900.0, h - 95.0),
        blocks,
    };

    let loading_zones = vec![
        LoadingZone::edge_exit("to scroll lab", ae::Vec2::new(w - 150.0, h - 178.0), ae::Vec2::new(105.0, 130.0), 1, ae::Vec2::new(210.0, 805.0)),
        LoadingZone::door("to vertical shaft", ae::Vec2::new(840.0, 36.0), ae::Vec2::new(120.0, 126.0), 2, ae::Vec2::new(700.0, 2295.0)),
        LoadingZone::edge_exit("to square arena", ae::Vec2::new(45.0, h - 178.0), ae::Vec2::new(105.0, 130.0), 3, ae::Vec2::new(1570.0, 1695.0)),
        LoadingZone::door("to tiny chamber", ae::Vec2::new(840.0, h - 178.0), ae::Vec2::new(120.0, 130.0), 4, ae::Vec2::new(150.0, 425.0)),
    ];
    RoomSpec { world, loading_zones }
}

fn build_scroll_lab() -> RoomSpec {
    let world = ae::build_endgame_sandbox();
    let loading_zones = vec![LoadingZone::edge_exit(
        "to central hub",
        ae::Vec2::new(42.0, world.size.y - 178.0),
        ae::Vec2::new(104.0, 130.0),
        0,
        ae::Vec2::new(1540.0, 905.0),
    )];
    RoomSpec { world, loading_zones }
}

fn build_vertical_shaft() -> RoomSpec {
    let w = 1000.0;
    let h = 2400.0;
    let mut blocks = Vec::new();
    shell(&mut blocks, w, h);

    blocks.push(ae::Block::one_way("shaft shelf 1", ae::Vec2::new(130.0, 2090.0), ae::Vec2::new(290.0, 18.0)));
    blocks.push(ae::Block::one_way("shaft shelf 2", ae::Vec2::new(570.0, 1810.0), ae::Vec2::new(270.0, 18.0)));
    blocks.push(ae::Block::one_way("shaft shelf 3", ae::Vec2::new(150.0, 1510.0), ae::Vec2::new(250.0, 18.0)));
    blocks.push(ae::Block::one_way("shaft shelf 4", ae::Vec2::new(610.0, 1225.0), ae::Vec2::new(250.0, 18.0)));
    blocks.push(ae::Block::one_way("shaft shelf 5", ae::Vec2::new(155.0, 940.0), ae::Vec2::new(260.0, 18.0)));
    blocks.push(ae::Block::one_way("shaft shelf 6", ae::Vec2::new(600.0, 645.0), ae::Vec2::new(260.0, 18.0)));
    blocks.push(ae::Block::blink_wall("shaft soft divider", ae::Vec2::new(485.0, 760.0), ae::Vec2::new(32.0, 1180.0), ae::BlinkWallTier::Soft));
    blocks.push(ae::Block::pogo_orb("shaft pogo 1", ae::Vec2::new(505.0, 1660.0), 18.0));
    blocks.push(ae::Block::pogo_orb("shaft pogo 2", ae::Vec2::new(500.0, 1085.0), 18.0));
    blocks.push(ae::Block::rebound("shaft bottom launcher", ae::Vec2::new(435.0, 2308.0), ae::Vec2::new(130.0, 24.0), ae::Vec2::new(0.0, -920.0)));
    blocks.push(ae::Block::hazard("shaft low hazard", ae::Vec2::new(560.0, 2328.0), ae::Vec2::new(260.0, 24.0)));

    let world = ae::World {
        name: "Ambition: Vertical Shaft",
        size: ae::Vec2::new(w, h),
        spawn: ae::Vec2::new(500.0, h - 95.0),
        blocks,
    };
    let loading_zones = vec![LoadingZone::edge_exit(
        "to central hub",
        ae::Vec2::new(42.0, h - 178.0),
        ae::Vec2::new(104.0, 130.0),
        0,
        ae::Vec2::new(900.0, 275.0),
    )];
    RoomSpec { world, loading_zones }
}

fn build_square_arena() -> RoomSpec {
    let w = 1800.0;
    let h = 1800.0;
    let mut blocks = Vec::new();
    shell(&mut blocks, w, h);

    blocks.push(ae::Block::solid("square center hard wall", ae::Vec2::new(872.0, 720.0), ae::Vec2::new(56.0, 520.0)));
    blocks.push(ae::Block::one_way("square lower left", ae::Vec2::new(240.0, 1410.0), ae::Vec2::new(360.0, 18.0)));
    blocks.push(ae::Block::one_way("square lower right", ae::Vec2::new(1200.0, 1410.0), ae::Vec2::new(360.0, 18.0)));
    blocks.push(ae::Block::one_way("square mid left", ae::Vec2::new(235.0, 990.0), ae::Vec2::new(330.0, 18.0)));
    blocks.push(ae::Block::one_way("square mid right", ae::Vec2::new(1235.0, 990.0), ae::Vec2::new(330.0, 18.0)));
    blocks.push(ae::Block::one_way("square high bridge", ae::Vec2::new(670.0, 470.0), ae::Vec2::new(460.0, 18.0)));
    blocks.push(ae::Block::blink_wall("square soft left veil", ae::Vec2::new(650.0, 970.0), ae::Vec2::new(34.0, 360.0), ae::BlinkWallTier::Soft));
    blocks.push(ae::Block::blink_wall("square soft right veil", ae::Vec2::new(1115.0, 970.0), ae::Vec2::new(34.0, 360.0), ae::BlinkWallTier::Soft));
    blocks.push(ae::Block::pogo_orb("square pogo left", ae::Vec2::new(555.0, 1220.0), 18.0));
    blocks.push(ae::Block::pogo_orb("square pogo right", ae::Vec2::new(1245.0, 1220.0), 18.0));
    blocks.push(ae::Block::rebound("square diagonal launcher", ae::Vec2::new(1460.0, 1650.0), ae::Vec2::new(130.0, 24.0), ae::Vec2::new(-700.0, -720.0)));
    blocks.push(ae::Block::hazard("square bottom hazard", ae::Vec2::new(720.0, 1730.0), ae::Vec2::new(360.0, 24.0)));

    let world = ae::World {
        name: "Ambition: Square Arena",
        size: ae::Vec2::new(w, h),
        spawn: ae::Vec2::new(170.0, h - 105.0),
        blocks,
    };
    let loading_zones = vec![LoadingZone::edge_exit(
        "to central hub",
        ae::Vec2::new(w - 160.0, h - 184.0),
        ae::Vec2::new(116.0, 136.0),
        0,
        ae::Vec2::new(260.0, 905.0),
    )];
    RoomSpec { world, loading_zones }
}

fn build_tiny_chamber() -> RoomSpec {
    let w = 900.0;
    let h = 520.0;
    let mut blocks = Vec::new();
    shell(&mut blocks, w, h);

    blocks.push(ae::Block::solid("tiny hard center", ae::Vec2::new(420.0, 300.0), ae::Vec2::new(42.0, 172.0)));
    blocks.push(ae::Block::one_way("tiny shelf", ae::Vec2::new(190.0, 315.0), ae::Vec2::new(220.0, 18.0)));
    blocks.push(ae::Block::blink_wall("tiny soft gate", ae::Vec2::new(590.0, 250.0), ae::Vec2::new(30.0, 190.0), ae::BlinkWallTier::Soft));
    blocks.push(ae::Block::pogo_orb("tiny pogo", ae::Vec2::new(515.0, 240.0), 16.0));

    let world = ae::World {
        name: "Ambition: Tiny Chamber",
        size: ae::Vec2::new(w, h),
        spawn: ae::Vec2::new(150.0, h - 95.0),
        blocks,
    };
    let loading_zones = vec![LoadingZone::edge_exit(
        "to central hub",
        ae::Vec2::new(w - 158.0, h - 176.0),
        ae::Vec2::new(112.0, 128.0),
        0,
        ae::Vec2::new(1040.0, 905.0),
    )];
    RoomSpec { world, loading_zones }
}
