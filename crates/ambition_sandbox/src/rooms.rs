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

    /// Return non-fatal authoring warnings for room specs.
    ///
    /// This is a tiny first step toward data-driven room validation. It catches
    /// active fixtures such as rebound/bounce pads, pogo orbs, or hazards that
    /// overlap loading zones. Later this can become a hard validator for
    /// generated or hand-authored room specs.
    pub fn layout_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        for (room_index, room) in self.rooms.iter().enumerate() {
            for zone in &room.loading_zones {
                for block in &room.world.blocks {
                    let active_fixture = matches!(
                        block.kind,
                        ae::BlockKind::Rebound { .. } | ae::BlockKind::PogoOrb | ae::BlockKind::Hazard
                    );
                    if active_fixture && block.aabb.intersects(zone.aabb) {
                        warnings.push(format!(
                            "room {room_index} '{}' has {} '{}' overlapping loading zone '{}'",
                            room.world.name,
                            block_kind_label(block.kind),
                            block.name,
                            zone.name,
                        ));
                    }
                }
            }
        }
        warnings
    }
}

fn block_kind_label(kind: ae::BlockKind) -> &'static str {
    match kind {
        ae::BlockKind::Solid => "solid",
        ae::BlockKind::BlinkWall { .. } => "blink wall",
        ae::BlockKind::OneWay => "one-way platform",
        ae::BlockKind::Hazard => "hazard",
        ae::BlockKind::PogoOrb => "pogo orb",
        ae::BlockKind::Rebound { .. } => "rebound pad",
    }
}

/// Side of a room shell where an edge-exit opening should be cut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WallSide {
    Left,
    Right,
}

/// Vertical opening in a side wall.
///
/// Automatic loading zones should read like walking through a hole in the wall,
/// so the solid wall is split around the opening instead of placing a trigger
/// volume in the middle of the room.
#[derive(Clone, Copy, Debug)]
struct WallOpening {
    side: WallSide,
    y: f32,
    height: f32,
}

fn shell(blocks: &mut Vec<ae::Block>, w: f32, h: f32) {
    shell_with_openings(blocks, w, h, &[]);
}

fn shell_with_openings(blocks: &mut Vec<ae::Block>, w: f32, h: f32, openings: &[WallOpening]) {
    const WALL: f32 = 36.0;
    const FLOOR: f32 = 48.0;
    const CEILING: f32 = 24.0;

    blocks.push(ae::Block::solid("floor", ae::Vec2::new(0.0, h - FLOOR), ae::Vec2::new(w, FLOOR)));
    blocks.push(ae::Block::solid("ceiling", ae::Vec2::new(0.0, 0.0), ae::Vec2::new(w, CEILING)));

    for (side, x, name) in [
        (WallSide::Left, 0.0, "left wall"),
        (WallSide::Right, w - WALL, "right wall"),
    ] {
        let mut spans = vec![(0.0, h)];
        for opening in openings.iter().filter(|opening| opening.side == side) {
            let open_min = opening.y.max(CEILING);
            let open_max = (opening.y + opening.height).min(h - FLOOR);
            if open_min >= open_max {
                continue;
            }
            let mut next = Vec::new();
            for (span_min, span_max) in spans {
                if open_max <= span_min || open_min >= span_max {
                    next.push((span_min, span_max));
                } else {
                    if span_min < open_min {
                        next.push((span_min, open_min));
                    }
                    if open_max < span_max {
                        next.push((open_max, span_max));
                    }
                }
            }
            spans = next;
        }
        for (index, (span_min, span_max)) in spans.into_iter().enumerate() {
            let height = span_max - span_min;
            if height > 0.5 {
                let block_name = if index == 0 { name } else { "wall segment" };
                blocks.push(ae::Block::solid(block_name, ae::Vec2::new(x, span_min), ae::Vec2::new(WALL, height)));
            }
        }
    }
}

fn low_side_opening(side: WallSide, h: f32) -> WallOpening {
    WallOpening {
        side,
        y: h - 236.0,
        height: 188.0,
    }
}

fn build_central_hub() -> RoomSpec {
    let w = 1800.0;
    let h = 1000.0;
    let mut blocks = Vec::new();
    shell_with_openings(
        &mut blocks,
        w,
        h,
        &[
            low_side_opening(WallSide::Left, h),
            low_side_opening(WallSide::Right, h),
        ],
    );

    blocks.push(ae::Block::one_way("hub center shelf", ae::Vec2::new(650.0, 720.0), ae::Vec2::new(500.0, 18.0)));
    blocks.push(ae::Block::one_way("hub upper shelf", ae::Vec2::new(745.0, 430.0), ae::Vec2::new(310.0, 18.0)));
    blocks.push(ae::Block::blink_wall("hub soft column", ae::Vec2::new(565.0, 615.0), ae::Vec2::new(32.0, 230.0), ae::BlinkWallTier::Soft));
    blocks.push(ae::Block::blink_wall("hub right soft column", ae::Vec2::new(1210.0, 610.0), ae::Vec2::new(32.0, 235.0), ae::BlinkWallTier::Soft));
    blocks.push(ae::Block::pogo_orb("hub routing note", ae::Vec2::new(900.0, 600.0), 18.0));
    blocks.push(ae::Block::rebound(
        "hub launcher",
        ae::Vec2::new(1060.0, 905.0),
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
        LoadingZone::edge_exit(
            "to scroll lab",
            ae::Vec2::new(w - 30.0, h - 224.0),
            ae::Vec2::new(30.0, 176.0),
            1,
            ae::Vec2::new(210.0, 805.0),
        ),
        LoadingZone::door(
            "to vertical shaft",
            ae::Vec2::new(840.0, 300.0),
            ae::Vec2::new(120.0, 122.0),
            2,
            ae::Vec2::new(500.0, 2305.0),
        ),
        LoadingZone::edge_exit(
            "to square arena",
            ae::Vec2::new(0.0, h - 224.0),
            ae::Vec2::new(30.0, 176.0),
            3,
            ae::Vec2::new(1570.0, 1695.0),
        ),
        LoadingZone::door(
            "to tiny chamber",
            ae::Vec2::new(840.0, h - 190.0),
            ae::Vec2::new(120.0, 142.0),
            4,
            ae::Vec2::new(150.0, 425.0),
        ),
    ];
    RoomSpec { world, loading_zones }
}

fn build_scroll_lab() -> RoomSpec {
    let world = ae::build_endgame_sandbox();
    let loading_zones = vec![LoadingZone::edge_exit(
        "to central hub",
        ae::Vec2::new(0.0, world.size.y - 224.0),
        ae::Vec2::new(30.0, 176.0),
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
    blocks.push(ae::Block::rebound("shaft bottom launcher", ae::Vec2::new(240.0, 2308.0), ae::Vec2::new(130.0, 24.0), ae::Vec2::new(0.0, -920.0)));
    blocks.push(ae::Block::hazard("shaft low hazard", ae::Vec2::new(620.0, 2328.0), ae::Vec2::new(260.0, 24.0)));

    let world = ae::World {
        name: "Ambition: Vertical Shaft",
        size: ae::Vec2::new(w, h),
        spawn: ae::Vec2::new(500.0, h - 95.0),
        blocks,
    };
    let loading_zones = vec![LoadingZone::door(
        "to central hub",
        ae::Vec2::new(438.0, h - 190.0),
        ae::Vec2::new(124.0, 142.0),
        0,
        ae::Vec2::new(900.0, 405.0),
    )];
    RoomSpec { world, loading_zones }
}

fn build_square_arena() -> RoomSpec {
    let w = 1800.0;
    let h = 1800.0;
    let mut blocks = Vec::new();
    shell_with_openings(&mut blocks, w, h, &[low_side_opening(WallSide::Right, h)]);

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
        ae::Vec2::new(w - 30.0, h - 224.0),
        ae::Vec2::new(30.0, 176.0),
        0,
        ae::Vec2::new(260.0, 905.0),
    )];
    RoomSpec { world, loading_zones }
}

fn build_tiny_chamber() -> RoomSpec {
    let w = 900.0;
    let h = 520.0;
    let mut blocks = Vec::new();
    shell_with_openings(&mut blocks, w, h, &[low_side_opening(WallSide::Right, h)]);

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
        ae::Vec2::new(w - 30.0, h - 224.0),
        ae::Vec2::new(30.0, 176.0),
        0,
        ae::Vec2::new(1040.0, 905.0),
    )];
    RoomSpec { world, loading_zones }
}
