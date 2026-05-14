use super::*;

impl FeatureRuntime {
    pub fn from_world(world: &ae::World) -> Self {
        let paths = room_paths(world);
        Self::from_world_with_paths(world, &paths)
    }

    pub fn from_room_spec(room: &crate::rooms::RoomSpec) -> Self {
        let paths = room_spec_paths(room);
        Self::from_world_with_paths(&room.world, &paths)
    }

    fn from_world_with_paths(world: &ae::World, paths: &[(String, ae::KinematicPath)]) -> Self {
        let mut runtime = Self {
            hazards: Vec::new(),
            enemies: Vec::new(),
            bosses: Vec::new(),
            breakables: Vec::new(),
            pickups: Vec::new(),
            chests: Vec::new(),
            npcs: Vec::new(),
            switches: Vec::new(),
            banner: String::new(),
            banner_timer: 0.0,
        };

        for object in &world.objects {
            match &object.kind {
                ae::RoomObjectKind::DamageVolume(volume) => {
                    runtime.hazards.push(HazardRuntime::new_with_paths(
                        object,
                        volume.clone(),
                        paths,
                    ));
                }
                // Phase 3/4/5 strangler: static pickups, chests, and
                // breakables are now spawned as ECS feature entities by
                // `spawn_room_feature_entities`. Keep these runtime vectors for
                // legacy tests and dynamic compatibility only.
                ae::RoomObjectKind::Pickup(_)
                | ae::RoomObjectKind::Chest(_)
                | ae::RoomObjectKind::Breakable(_) => {}
                ae::RoomObjectKind::Interactable(interactable) => {
                    if matches!(interactable.kind, ae::InteractionKind::Npc { .. }) {
                        // Phase 6/7 strangler: authored NPCs are ECS actors now.
                    } else if let ae::InteractionKind::Custom(payload) = &interactable.kind {
                        if payload.starts_with("switch:") {
                            // Switches are ECS entities now; encounter arming reads
                            // EncounterSwitchIndex rebuilt from SwitchFeature/SwitchOn.
                        }
                    }
                }
                ae::RoomObjectKind::EnemySpawn(_) => {
                    // Authored enemies are ECS actors now. Dynamic encounter mobs
                    // still enter through `FeatureRuntime::spawn_enemy`.
                }
                ae::RoomObjectKind::BossSpawn(brain) => {
                    runtime.bosses.push(BossRuntime::new(object, brain.clone()));
                }
                ae::RoomObjectKind::Actor(_)
                | ae::RoomObjectKind::KinematicPath(_)
                | ae::RoomObjectKind::DebugLabel(_)
                | ae::RoomObjectKind::DestinationLabel(_) => {}
            }
        }
        runtime
    }
}
