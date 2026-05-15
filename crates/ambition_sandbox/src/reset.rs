//! Sandbox-wide reset: wipe the save and rebuild the runtime so the
//! player ends up in the world's start room with every NPC alive,
//! every encounter armed, and every persisted flag cleared.
//!
//! Triggered by setting [`SandboxResetRequested::request`] from
//! anywhere — today the pause-menu "Reset Sandbox" item is the only
//! caller, but the seam is generic so future debug hotkeys / dev
//! tools can reuse it.
//!
//! ## What gets reset
//!
//! - **Save**: replaced with `SandboxSaveData::default()` (encounters,
//!   switches, bosses, quests, flags all cleared). Bevy's
//!   change-detection picks up the mutation and `autosave_sandbox_save`
//!   writes the empty save to disk on the same tick, so the reset
//!   survives a fresh game launch.
//! - **Encounter / boss / quest registries**: replaced with their
//!   `Default` values, which sets `specs_loaded` / `initialized` to
//!   false. The populate Update systems
//!   (`populate_encounter_registry`, etc.) re-run on the next frame
//!   and rebuild the registries from the LDtk project + the now-empty
//!   save.
//! - **`EncounterController` entities** (one per encounter): despawned.
//!   The next populate spawns fresh ones.
//! - **`RoomVisual` entities**: despawned. The room-visual respawn
//!   path runs after the active room flips back to the start.
//! - **`SandboxRuntime`**: warped to the start room's spawn via
//!   `runtime.reset(world, tuning)` — this rebuilds `features` from
//!   the world (so all NPCs are alive and peaceful again, all
//!   breakables intact, all chests closed). Immediately afterward,
//!   the moving-platform runtime state is re-seeded from the start
//!   room's LDtk-authored `MovingPlatform`, falling back to the legacy
//!   test/reference platform only for rooms that have no authored
//!   platform yet.
//! - **Active room**: `room_set.active` resets to `room_set.start`
//!   (captured at `RoomSet::from_parts` time) so the player ends up
//!   wherever a fresh game would start, not wherever they happened
//!   to be when they hit reset.
//!
//! ## What does NOT get reset
//!
//! - User settings (audio mix, controls, video, gameplay tuning) —
//!   those live in `crate::settings::UserSettings` and are not part
//!   of the sandbox state. Reset is about gameplay progress only.
//! - Keyboard preset selection.
//! - Dev-tool toggles (the F3 stats editor's invincible flag etc.)
//!   live in `SandboxRuntime` and ARE reset, because `runtime.reset`
//!   refills them — that's actually a feature: a player who
//!   accidentally enabled invincibility and wants to play "for real"
//!   gets a clean slate.

use bevy::prelude::*;

use ambition_engine as ae;

use crate::boss_encounter::BossEncounterRegistry;
use crate::encounter::{EncounterController, EncounterMusicRequest, EncounterRegistry};
use crate::game_assets::GameAssets;
use crate::physics;
use crate::platforms;
use crate::quest::QuestRegistry;
use crate::rendering::{spawn_room_visuals, RoomVisual};
use crate::rooms::RoomSet;
use crate::save::SandboxSave;
use crate::SandboxRuntime;

/// Cross-system trigger for "wipe the save and rebuild the runtime."
/// Set `request = true` from anywhere; the next
/// `process_sandbox_reset_request` tick consumes it.
#[derive(Resource, Default, Debug)]
pub struct SandboxResetRequested {
    pub request: bool,
}

impl SandboxResetRequested {
    pub fn request(&mut self) {
        self.request = true;
    }
}

/// Bevy system: drains a pending reset request and rebuilds the
/// sandbox state. Idempotent on `request = false` (early returns).
///
/// Schedule: runs in `Update` AFTER `sandbox_update` so a reset
/// triggered mid-frame doesn't race with in-flight gameplay
/// mutations, and BEFORE the populate systems so when they run on
/// the next frame the cleared registries see fresh state.
pub fn process_sandbox_reset_request(
    mut request: ResMut<SandboxResetRequested>,
    mut save: ResMut<SandboxSave>,
    mut encounter_registry: ResMut<EncounterRegistry>,
    mut boss_registry: ResMut<BossEncounterRegistry>,
    mut quest_registry: ResMut<QuestRegistry>,
    mut music_request: ResMut<EncounterMusicRequest>,
    mut runtime: ResMut<SandboxRuntime>,
    mut room_set: ResMut<RoomSet>,
    mut world: ResMut<crate::GameWorld>,
    tuning: Res<crate::dev_tools::EditableMovementTuning>,
    assets: Option<Res<GameAssets>>,
    mut commands: Commands,
    mut banner: ResMut<crate::features::GameplayBanner>,
    encounter_controllers: Query<Entity, With<EncounterController>>,
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
    mut player_q: Query<
        (
            &mut crate::player::PlayerMovementAuthority,
            &mut crate::player::PlayerAnimState,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    if !request.request {
        return;
    }
    request.request = false;

    info!(
        target: "ambition::reset",
        "sandbox reset requested — wiping save, registries, and runtime"
    );

    // 1. Wipe the persisted save. Change-detection will trigger the
    //    autosave system to write the empty save to disk this tick.
    *save.data_mut() = ae::SandboxSaveData::default();

    // 2. Clear registries. Setting them to Default flips
    //    `specs_loaded` / `initialized` back to false so the populate
    //    Update systems re-run on the next frame.
    *encounter_registry = EncounterRegistry::default();
    *boss_registry = BossEncounterRegistry::default();
    *quest_registry = QuestRegistry::default();
    music_request.desired_track = None;

    // 3. Despawn the EncounterController entities; populate will spawn
    //    a fresh set keyed off the post-reset save state.
    for entity in &encounter_controllers {
        commands.entity(entity).despawn();
    }

    // 4. Despawn all room visuals (and their physics colliders if
    //    Avian2D installed any). The room-visual respawn path that
    //    sandbox_update / room-load already use will rebuild them
    //    once the active room flip below kicks in.
    for (entity, physics_entity) in &room_visuals {
        if physics_entity.is_some() {
            physics::retire_physics_entity(&mut commands, entity);
        } else {
            commands.entity(entity).despawn();
        }
    }

    // 5. Warp the active room back to the start room (where the
    //    player begins on a fresh game). `RoomSet::start` was
    //    captured at construction.
    let start_index = room_set.start;
    let start_spec = room_set.set_active(start_index).clone();
    world.0 = start_spec.world.clone();

    // 6. Reset the runtime: player back to spawn, features rebuilt
    //    from the world (NPCs alive again, breakables intact, chests
    //    closed). `runtime.reset` already does the right thing for
    //    a same-room reset; we're calling it after flipping the
    //    active room so it uses the start room's spawn point.
    runtime.reset(&world.0, tuning.as_engine());
    // Mirror the reset into the ECS authority so the next sandbox_update
    // frame starts from the spawn position rather than the pre-reset position.
    // Also zero the animation state so post-reset frames don't continue a
    // mid-air slash or dash-startup pose from before the reset.
    if let Ok((mut authority, mut anim)) = player_q.single_mut() {
        authority.player = runtime.player.clone();
        anim.reset();
    }
    crate::features::spawn_room_feature_entities(&mut commands, &start_spec);
    runtime.moving_platforms = platforms::moving_platforms_for_room(&start_spec);

    // 7. Respawn the static world visuals + moving platform for the
    //    start room. Without this, the despawn in step 4 leaves the
    //    scene empty until something else (LDtk reload, room transition)
    //    triggers a fresh `spawn_room_visuals`. Mirrors the pattern in
    //    `app::world_flow::load_room` and the LDtk hot-reload path.
    crate::rendering::spawn_parallax_layers(
        &mut commands,
        &world.0,
        &start_spec.metadata,
        assets.as_deref(),
    );
    spawn_room_visuals(
        &mut commands,
        &world.0,
        &start_spec.loading_zones,
        runtime.physics_settings,
        assets.as_deref(),
    );
    platforms::spawn_moving_platforms(&mut commands, &world.0, &runtime.moving_platforms);

    // 8. User feedback: surface a banner so the reset is visibly
    //    confirmed. The HUD's banner channel is the same one used
    //    for "ARENA CLEAR" etc.
    banner.show("SANDBOX RESET", 3.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev_tools::EditableMovementTuning;
    use crate::GameWorld;

    /// Pin the request resource's defaults: a fresh app starts with
    /// no reset queued. Important because the reset processor must
    /// be a no-op when nothing has been requested.
    #[test]
    fn request_default_is_idle() {
        let req = SandboxResetRequested::default();
        assert!(!req.request);
    }

    /// `request()` sets the flag; the processor consumes it.
    #[test]
    fn request_helper_sets_the_flag() {
        let mut req = SandboxResetRequested::default();
        req.request();
        assert!(req.request);
    }

    fn dummy_world() -> ae::World {
        ae::World::new(
            "test",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 1000.0),
            vec![ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 1500.0),
                ae::Vec2::new(2000.0, 32.0),
            )],
        )
    }

    /// Build a minimal Bevy app wired with the reset processor and
    /// just enough resources for it to run: the request resource,
    /// the save, the three registries it clears, the music request,
    /// runtime + world + room set + tuning, and the relevant entity
    /// queries (empty here — no controllers / no room visuals to
    /// despawn in this synthetic harness).
    fn min_app() -> App {
        let mut app = App::new();
        let world = dummy_world();
        app.insert_resource(SandboxResetRequested::default());
        app.insert_resource(SandboxSave::default());
        app.insert_resource(EncounterRegistry::default());
        app.insert_resource(BossEncounterRegistry::default());
        app.insert_resource(QuestRegistry::default());
        app.insert_resource(EncounterMusicRequest::default());
        app.insert_resource(crate::features::GameplayBanner::default());
        let runtime = SandboxRuntime::new(
            &world,
            ae::AbilitySet::sandbox_all(),
            ae::DEFAULT_TUNING,
            crate::physics::PhysicsSandboxSettings::default(),
        );
        app.insert_resource(runtime);
        app.insert_resource(GameWorld(world.clone()));
        // Construct a minimal RoomSet with one room so `start` and
        // `active` are both valid indices.
        let room_spec = crate::rooms::RoomSpec {
            id: "test".into(),
            world: world.clone(),
            loading_zones: Vec::new(),
            metadata: crate::rooms::RoomMetadata::default(),
            camera_zones: Vec::new(),
            kinematic_paths: Vec::new(),
            moving_platforms: Vec::new(),
        };
        app.insert_resource(crate::rooms::RoomSet::from_parts(
            "test",
            vec![room_spec],
            Vec::new(),
        ));
        app.insert_resource(EditableMovementTuning::default());
        app.add_systems(Update, process_sandbox_reset_request);
        app
    }

    /// Sanity: with no request, the processor leaves state alone.
    /// Set a save flag, run a tick, confirm it's still set.
    #[test]
    fn processor_is_a_noop_without_request() {
        let mut app = min_app();
        {
            let mut save = app.world_mut().resource_mut::<SandboxSave>();
            save.data_mut().set_flag("npc_test_hostile", true);
        }
        app.update();
        let save = app.world().resource::<SandboxSave>();
        assert!(save.data().flag("npc_test_hostile"));
    }

    /// The headline behavior: a queued request wipes the save flags
    /// (the thing the user noticed — NPCs persisting as dead) and
    /// flips registries back to "specs not loaded" so the populate
    /// systems repopulate on the next frame.
    #[test]
    fn processor_wipes_save_flags_and_clears_registries() {
        let mut app = min_app();
        // Pre-populate the state the user is trying to reset:
        // - a save flag remembering an NPC turned hostile
        // - a save flag remembering an encounter chest was looted
        // - "specs already loaded" on the registries
        {
            let mut save = app.world_mut().resource_mut::<SandboxSave>();
            save.data_mut().set_flag("npc_kira_hostile", true);
            save.data_mut()
                .set_flag("encounter_mob_lab_reward_dropped", true);
            save.data_mut()
                .set_encounter("mob_lab", ae::PersistedEncounterState::Cleared);
        }
        {
            let mut reg = app.world_mut().resource_mut::<EncounterRegistry>();
            reg.specs_loaded = true;
        }
        {
            let mut reg = app.world_mut().resource_mut::<BossEncounterRegistry>();
            reg.specs_loaded = true;
        }
        {
            let mut reg = app.world_mut().resource_mut::<QuestRegistry>();
            reg.initialized = true;
        }
        // Queue the reset.
        {
            let mut req = app.world_mut().resource_mut::<SandboxResetRequested>();
            req.request();
        }
        app.update();

        // Save is wiped.
        let save = app.world().resource::<SandboxSave>();
        assert!(!save.data().flag("npc_kira_hostile"));
        assert!(!save.data().flag("encounter_mob_lab_reward_dropped"));
        assert_eq!(
            save.data().encounter("mob_lab"),
            ae::PersistedEncounterState::Untouched
        );
        // Registries flag-flipped back so populate Update systems
        // will re-run on the next frame.
        let enc = app.world().resource::<EncounterRegistry>();
        assert!(!enc.specs_loaded);
        let boss = app.world().resource::<BossEncounterRegistry>();
        assert!(!boss.specs_loaded);
        let quest = app.world().resource::<QuestRegistry>();
        assert!(!quest.initialized);
        // Banner surfaces the action so the player can see it.
        assert_eq!(app.world().resource::<crate::features::GameplayBanner>().text, "SANDBOX RESET");
        // Request consumed.
        let req = app.world().resource::<SandboxResetRequested>();
        assert!(!req.request);
    }

    /// After reset, the player is warped to the start room's spawn
    /// regardless of where they were before the reset. This is the
    /// "back to a fresh game" guarantee.
    #[test]
    fn processor_warps_player_to_start_spawn() {
        let mut app = min_app();
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            runtime.player.pos = ae::Vec2::new(1234.0, 1234.0);
        }
        {
            let mut req = app.world_mut().resource_mut::<SandboxResetRequested>();
            req.request();
        }
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        let world = app.world().resource::<GameWorld>();
        assert_eq!(runtime.player.pos, world.0.spawn);
    }

    /// Reset must restore the moving platform from the start room's
    /// authored LDtk platform, not from the old procedural fallback.
    #[test]
    fn processor_restores_authored_start_room_platform() {
        let mut app = min_app();
        let authored = crate::platforms::MovingPlatformState::from_authored(
            ae::Vec2::new(512.0, 900.0),
            ae::Vec2::new(128.0, 16.0),
            192.0,
            75.0,
        );
        {
            let mut room_set = app.world_mut().resource_mut::<RoomSet>();
            room_set.rooms[0].moving_platforms = vec![authored.clone()];
        }
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            runtime.moving_platforms = vec![crate::platforms::MovingPlatformState::from_authored(
                ae::Vec2::new(10.0, 20.0),
                ae::Vec2::new(32.0, 8.0),
                64.0,
                10.0,
            )];
        }
        {
            let mut req = app.world_mut().resource_mut::<SandboxResetRequested>();
            req.request();
        }
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.moving_platforms[0].pos, authored.pos);
        assert_eq!(runtime.moving_platforms[0].size, authored.size);
    }
}
