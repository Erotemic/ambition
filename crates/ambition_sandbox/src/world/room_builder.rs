//! Code-first room authoring helper.
//!
//! LDtk is the production authoring surface for shipped rooms, but
//! prototyping a quick test room (water test, cutscene test, AI
//! sandbox) through the LDtk JSON is more friction than it's worth.
//! This builder lets a Rust caller assemble a `RoomSpec` directly
//! using a fluent API:
//!
//! ```no_run
//! use ambition_sandbox::room_builder::RoomBuilder;
//!
//! let room = RoomBuilder::new("water_test", 1024.0, 768.0)
//!     .floor()
//!     .walls()
//!     .ceiling()
//!     .water_volume([400.0, 400.0], [600.0, 700.0], Default::default())
//!     .npc_dialogue("guide", "Press Up + Jump while clinging.", [200.0, 660.0])
//!     .door_zone("exit", "exit_door", [80.0, 600.0], [80.0, 120.0])
//!     .build();
//! ```
//!
//! `build()` returns a `RoomSpec` ready to drop into a `RoomSet`.
//! See `register_code_rooms` in `code_rooms.rs` for the actual
//! consumer wiring.

use ambition_engine as ae;

use crate::rooms::{LoadingZone, LoadingZoneActivation, RoomSpec};

const WALL_THICK: f32 = 36.0;
const FLOOR_THICK: f32 = 36.0;

#[derive(Clone, Debug)]
pub struct RoomBuilder {
    id: String,
    size: ae::Vec2,
    spawn: ae::Vec2,
    blocks: Vec<ae::Block>,
    objects: Vec<ae::RoomObject>,
    water_regions: Vec<ae::WaterRegion>,
    loading_zones: Vec<LoadingZone>,
    next_block_id: u32,
}

impl RoomBuilder {
    pub fn new(id: impl Into<String>, width: f32, height: f32) -> Self {
        let size = ae::Vec2::new(width.max(160.0), height.max(120.0));
        Self {
            id: id.into(),
            size,
            spawn: ae::Vec2::new(width * 0.5, height - FLOOR_THICK - 30.0),
            blocks: Vec::new(),
            objects: Vec::new(),
            water_regions: Vec::new(),
            loading_zones: Vec::new(),
            next_block_id: 0,
        }
    }

    pub fn with_spawn(mut self, x: f32, y: f32) -> Self {
        self.spawn = ae::Vec2::new(x, y);
        self
    }

    /// Add the standard playfield floor.
    pub fn floor(mut self) -> Self {
        let name = self.next_block_name("floor");
        self.blocks.push(ae::Block::solid(
            name,
            ae::Vec2::new(0.0, self.size.y - FLOOR_THICK),
            ae::Vec2::new(self.size.x, FLOOR_THICK),
        ));
        self
    }

    /// Add left + right walls.
    pub fn walls(mut self) -> Self {
        let left = self.next_block_name("wall_left");
        let right = self.next_block_name("wall_right");
        self.blocks.push(ae::Block::solid(
            left,
            ae::Vec2::ZERO,
            ae::Vec2::new(WALL_THICK, self.size.y),
        ));
        self.blocks.push(ae::Block::solid(
            right,
            ae::Vec2::new(self.size.x - WALL_THICK, 0.0),
            ae::Vec2::new(WALL_THICK, self.size.y),
        ));
        self
    }

    pub fn ceiling(mut self) -> Self {
        let name = self.next_block_name("ceiling");
        self.blocks.push(ae::Block::solid(
            name,
            ae::Vec2::ZERO,
            ae::Vec2::new(self.size.x, WALL_THICK),
        ));
        self
    }

    /// Add a Solid platform with the given top-left corner and size.
    pub fn platform(mut self, min: [f32; 2], size: [f32; 2]) -> Self {
        let name = self.next_block_name("platform");
        self.blocks.push(ae::Block::solid(
            name,
            ae::Vec2::new(min[0], min[1]),
            ae::Vec2::new(size[0], size[1]),
        ));
        self
    }

    /// Add a one-way platform (jump-up-through, land-from-above).
    pub fn one_way_platform(mut self, min: [f32; 2], size: [f32; 2]) -> Self {
        let name = self.next_block_name("one_way");
        self.blocks.push(ae::Block::one_way(
            name,
            ae::Vec2::new(min[0], min[1]),
            ae::Vec2::new(size[0], size[1]),
        ));
        self
    }

    pub fn hazard(mut self, min: [f32; 2], size: [f32; 2]) -> Self {
        let name = self.next_block_name("hazard");
        self.blocks.push(ae::Block::hazard(
            name,
            ae::Vec2::new(min[0], min[1]),
            ae::Vec2::new(size[0], size[1]),
        ));
        self
    }

    /// Add a water region that triggers swim mechanics. Defaults to
    /// `WaterKind::Clear` — programmatic builders that need murky
    /// water can call `water_region` directly instead.
    pub fn water_volume(self, min: [f32; 2], size: [f32; 2], spec: ae::WaterVolumeSpec) -> Self {
        self.water_region(min, size, ae::WaterKind::Clear, spec)
    }

    pub fn water_region(
        mut self,
        min: [f32; 2],
        size: [f32; 2],
        kind: ae::WaterKind,
        spec: ae::WaterVolumeSpec,
    ) -> Self {
        let aabb = ae::aabb_from_min_size(
            ae::Vec2::new(min[0], min[1]),
            ae::Vec2::new(size[0], size[1]),
        );
        self.water_regions
            .push(ae::WaterRegion::new(aabb, kind, spec));
        self
    }

    /// Add a peaceful NPC anchored at `pos`.
    pub fn npc_dialogue(
        mut self,
        npc_id: impl Into<String>,
        prompt: impl Into<String>,
        pos: [f32; 2],
    ) -> Self {
        let npc_id = npc_id.into();
        let aabb = ae::Aabb::new(ae::Vec2::new(pos[0], pos[1]), ae::Vec2::new(14.0, 22.0));
        let interactable = ae::Interactable::new(
            npc_id.clone(),
            prompt.into(),
            aabb,
            ae::InteractionKind::Npc {
                dialogue_id: Some(npc_id.clone()),
                patrol_radius: 0.0,
                patrol_path_id: None,
            },
        );
        self.objects.push(ae::RoomObject::new(
            npc_id.clone(),
            npc_id,
            aabb,
            ae::RoomObjectKind::Interactable(interactable),
        ));
        self
    }

    /// Add an enemy at `pos` driven by an `EnemyBrain::Custom(brain)`
    /// archetype id (e.g. "medium_striker", "small_skitter").
    pub fn enemy(mut self, brain: impl Into<String>, pos: [f32; 2]) -> Self {
        let id = self.next_object_id("enemy");
        let aabb = ae::Aabb::new(ae::Vec2::new(pos[0], pos[1]), ae::Vec2::new(11.0, 19.0));
        self.objects.push(ae::RoomObject::new(
            id.clone(),
            id,
            aabb,
            ae::RoomObjectKind::EnemySpawn(ae::EnemyBrain::Custom(brain.into())),
        ));
        self
    }

    /// Add a door-style loading zone. Caller wires the
    /// destination through `RoomLink` separately.
    pub fn door_zone(
        mut self,
        zone_id: impl Into<String>,
        zone_name: impl Into<String>,
        min: [f32; 2],
        size: [f32; 2],
    ) -> Self {
        let aabb = ae::aabb_from_min_size(
            ae::Vec2::new(min[0], min[1]),
            ae::Vec2::new(size[0], size[1]),
        );
        self.loading_zones.push(LoadingZone {
            id: zone_id.into(),
            name: zone_name.into(),
            activation: LoadingZoneActivation::Door,
            aabb,
        });
        self
    }

    pub fn edge_zone(
        mut self,
        zone_id: impl Into<String>,
        zone_name: impl Into<String>,
        min: [f32; 2],
        size: [f32; 2],
    ) -> Self {
        let aabb = ae::aabb_from_min_size(
            ae::Vec2::new(min[0], min[1]),
            ae::Vec2::new(size[0], size[1]),
        );
        self.loading_zones.push(LoadingZone {
            id: zone_id.into(),
            name: zone_name.into(),
            activation: LoadingZoneActivation::EdgeExit,
            aabb,
        });
        self
    }

    pub fn build(self) -> RoomSpec {
        let world = ae::World::new(self.id.clone(), self.size, self.spawn, self.blocks)
            .with_objects(self.objects)
            .with_water_regions(self.water_regions);
        RoomSpec {
            id: self.id,
            world,
            loading_zones: self.loading_zones,
            metadata: crate::rooms::RoomMetadata::default(),
            camera_zones: Vec::new(),
            kinematic_paths: Vec::new(),
            moving_platforms: Vec::new(),
            props: Vec::new(),
        }
    }

    fn next_block_name(&mut self, prefix: &str) -> String {
        self.next_block_id = self.next_block_id.saturating_add(1);
        format!("{prefix}_{}", self.next_block_id)
    }

    fn next_object_id(&mut self, prefix: &str) -> String {
        self.next_block_id = self.next_block_id.saturating_add(1);
        format!("{}__{prefix}_{}", self.id, self.next_block_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_box_room_has_floor_walls_and_ceiling() {
        let room = RoomBuilder::new("box", 800.0, 600.0)
            .floor()
            .walls()
            .ceiling()
            .build();
        assert_eq!(room.world.blocks.len(), 4);
        assert!(room.world.size.x > 0.0);
    }

    #[test]
    fn water_volume_lands_as_water_region() {
        let room = RoomBuilder::new("water", 800.0, 600.0)
            .floor()
            .walls()
            .water_volume([200.0, 300.0], [400.0, 200.0], Default::default())
            .build();
        assert_eq!(room.world.water_regions.len(), 1);
        assert_eq!(room.world.water_regions[0].kind, ae::WaterKind::Clear);
    }

    #[test]
    fn door_zone_appears_in_loading_zones() {
        let room = RoomBuilder::new("doors", 800.0, 600.0)
            .floor()
            .walls()
            .door_zone("exit", "Exit", [40.0, 480.0], [80.0, 110.0])
            .build();
        assert_eq!(room.loading_zones.len(), 1);
        assert!(matches!(
            room.loading_zones[0].activation,
            LoadingZoneActivation::Door
        ));
    }

    #[test]
    fn enemy_spawn_lands_as_room_object() {
        let room = RoomBuilder::new("test", 800.0, 600.0)
            .floor()
            .walls()
            .enemy("medium_striker", [400.0, 540.0])
            .build();
        assert!(room
            .world
            .objects
            .iter()
            .any(|o| matches!(o.kind, ae::RoomObjectKind::EnemySpawn(_))));
    }

    #[test]
    fn npc_dialogue_lands_as_interactable() {
        let room = RoomBuilder::new("npc", 800.0, 600.0)
            .floor()
            .walls()
            .npc_dialogue("guide", "Hi.", [200.0, 540.0])
            .build();
        let kind_count = room
            .world
            .objects
            .iter()
            .filter(|o| matches!(&o.kind, ae::RoomObjectKind::Interactable(it) if matches!(it.kind, ae::InteractionKind::Npc { .. })))
            .count();
        assert_eq!(kind_count, 1);
    }
}
