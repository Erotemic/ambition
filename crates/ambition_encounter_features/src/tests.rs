//! Integration tests for the encounter module: the wave adapter flow over the
//! generic lifecycle (start/wave/complete/fail/reset), multi-wave + delayed
//! sub-spawn timing, switch arming, LDtk loading of the `goblin_encounter`
//! fixture, reward-chest placement, and lock-wall sync.

use crate::*;
use ambition_encounter::switches::{EncounterSwitchIndex, EncounterSwitchLink};
use ambition_encounter::{
    active_encounter_camera_zoom, encounter_reward_chest_pos, Encounter, EncounterCommandKind,
    EncounterEvent, EncounterLifecycle, EncounterMobSpec, EncounterParticipant,
    EncounterParticipants, EncounterPhase, EncounterRegistry, EncounterRole, EncounterSpec,
    EncounterWaveSpec, EncounterWaves, LockWallSpec, SwitchActivation,
    ENCOUNTER_INTER_WAVE_DELAY_SECONDS, WAVES_EXHAUSTED_SIGNAL,
};
use ambition_entity_catalog::placements::PlacementSchema;
use ambition_persistence::save_data::PersistedEncounterState;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_ldtk::LdtkProject;
use ambition_platformer2d_world::rooms::InteractionKindSpec;
use bevy::math::bounding::IntersectsVolume;

/// The sandbox world these tests read, as a plain value. No install, no
/// process global: each test names the manifest it loads through.
fn test_world_manifest() -> ambition_platformer2d_world::world_manifest::WorldManifest {
    use ambition_asset_manager::AssetId;
    use ambition_platformer2d_world::world_manifest::{WorldManifest, WorldSource};
    let worlds_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../game/ambition_content/assets/worlds");
    WorldManifest {
        entry_room: "central_hub_complex".to_string(),
        ron_rooms: Vec::new(),
        worlds: vec![WorldSource {
            id: AssetId::new("world.sandbox_ldtk"),
            asset_path: "game://worlds/sandbox.ldtk".to_string(),
            loose_path: Some(worlds_dir.join("sandbox.ldtk")),
            embedded_text: None,
            embedded_bevy_path: Some("ambition_content/worlds/sandbox.ldtk"),
            required: true,
        }],
    }
}

/// A wave encounter's live authority set, as `populate_encounter_registry`
/// spawns it (lifecycle + wave policy + objective + participants).
struct WaveEncounter {
    lifecycle: EncounterLifecycle,
    waves: EncounterWaves,
    parts: EncounterParticipants,
}

impl WaveEncounter {
    fn new(spec: EncounterSpec) -> Self {
        let lifecycle =
            EncounterLifecycle::from_persisted(spec.intro_seconds, PersistedEncounterState::Untouched);
        Self {
            lifecycle,
            waves: EncounterWaves::new(spec),
            parts: EncounterParticipants::default(),
        }
    }

    /// Emit the Start command the trigger adapter writes on player entry.
    fn start(&mut self) -> Vec<EncounterEvent> {
        self.parts.members.clear();
        let objective = self.waves.objective();
        self.lifecycle.reduce(
            0.0,
            [&EncounterCommandKind::Start],
            &self.parts,
            Some(&objective),
        )
    }

    /// One adapter+reducer tick: director cadence while Active (publishing the
    /// exhaustion signal through the command ingress), then the generic
    /// reducer with the wave objective — exactly the shape
    /// `drive_wave_encounters` + `reduce_encounter_lifecycles` run per frame.
    fn tick(&mut self, dt: f32) -> Vec<EncounterEvent> {
        let mut events = Vec::new();
        let mut commands = Vec::new();
        if matches!(self.lifecycle.phase(), EncounterPhase::Active)
            && self.waves.tick_active(dt, &mut self.parts, &mut events)
        {
            commands.push(EncounterCommandKind::Signal(
                WAVES_EXHAUSTED_SIGNAL.to_string(),
            ));
        }
        let objective = self.waves.objective();
        events.extend(
            self.lifecycle
                .reduce(dt, commands.iter(), &self.parts, Some(&objective)),
        );
        events
    }
}

/// Mimic the host's liveness refresh reporting every live minion dead (the
/// director reads `participant.alive`, which the host sets from the runtime).
fn kill_all(parts: &mut EncounterParticipants) {
    for m in &mut parts.members {
        m.alive = false;
    }
}

fn lab_spec() -> EncounterSpec {
    EncounterSpec {
        id: "goblin_encounter".into(),
        waves: vec![
            EncounterWaveSpec {
                label: "wave 1".into(),
                mobs: vec![EncounterMobSpec::new("dummy", [100.0, 100.0])],
            },
            EncounterWaveSpec {
                label: "wave 2".into(),
                mobs: vec![
                    EncounterMobSpec::new("dummy", [120.0, 100.0]),
                    EncounterMobSpec::new("dummy", [180.0, 100.0]),
                ],
            },
        ],
        trigger_min: [0.0, 0.0],
        trigger_size: [400.0, 200.0],
        camera_zoom: 1.5,
        lock_wall: None,
        // Tests want immediate spawn on entry — skip the intro delay so the
        // first tick after Start can check the Active state.
        intro_seconds: 0.0,
        music_track: String::new(),
        reward: ambition_encounter::spec::default_encounter_reward(),
    }
}

/// The trigger-entry AABB test the adapter runs against the player body.
fn player_hits_trigger(spec: &EncounterSpec, pos: ae::Vec2, size: ae::Vec2) -> bool {
    let player_aabb = ae::aabb_from_min_size(
        ae::Vec2::new(pos.x - size.x * 0.5, pos.y - size.y * 0.5),
        size,
    );
    spec.trigger_aabb().intersects(&player_aabb)
}

#[test]
fn encounter_reward_defaults_to_small_heal_and_is_authorable() {
    use ambition_interaction::PickupKind;
    // Back-compat: the default reward stays the legacy small heal, so
    // specs that don't set `reward` behave exactly as before.
    assert_eq!(
        ambition_encounter::spec::default_encounter_reward(),
        PickupKind::Health { amount: 2 }
    );
    // Per-encounter authoring: a fight can now grant something else, and
    // it survives a serde roundtrip (data-authorable, not hardcoded at
    // the chest spawn site).
    let mut spec = lab_spec();
    spec.reward = PickupKind::Currency { amount: 25 };
    let ron = ron::to_string(&spec).expect("EncounterSpec should serialize");
    let back: EncounterSpec = ron::from_str(&ron).expect("EncounterSpec should deserialize");
    assert_eq!(back.reward, PickupKind::Currency { amount: 25 });
}

#[test]
fn entering_trigger_starts_first_wave() {
    let spec = lab_spec();
    // The adapter's trigger test: inside fires, and the Start command drives
    // the generic lifecycle into the first wave.
    assert!(player_hits_trigger(
        &spec,
        ae::Vec2::new(50.0, 50.0),
        ae::Vec2::new(20.0, 30.0)
    ));
    let mut enc = WaveEncounter::new(spec);
    let events = enc.start();
    assert!(enc.lifecycle.phase().locks_exits());
    assert!(events.contains(&EncounterEvent::Started));
    assert!(events.contains(&EncounterEvent::LockChanged { locked: true }));
    // First Active tick arms wave 0 and spawns its single mob.
    enc.tick(0.001);
    assert_eq!(enc.lifecycle.phase(), EncounterPhase::Active);
    assert_eq!(enc.waves.run.wave_index, Some(0));
    assert_eq!(enc.waves.remaining_mobs(&enc.parts), 1);
}

#[test]
fn standing_outside_trigger_does_not_start() {
    let spec = lab_spec();
    assert!(
        !player_hits_trigger(
            &spec,
            ae::Vec2::new(2000.0, 50.0),
            ae::Vec2::new(20.0, 30.0)
        ),
        "the adapter only writes Start when the player AABB hits the trigger"
    );
}

#[test]
fn defeating_all_mobs_clears_each_wave_and_then_encounter() {
    let mut enc = WaveEncounter::new(lab_spec());
    enc.start();
    // First tick spawns wave 1's single mob (delay 0).
    enc.tick(0.001);
    // Wave 1's mob is reported dead → wave advances to wave 2.
    kill_all(&mut enc.parts);
    enc.tick(0.001);
    assert_eq!(enc.waves.run.wave_index, Some(1), "wave 2 armed");
    assert_eq!(
        enc.lifecycle.phase(),
        EncounterPhase::Active,
        "no completion between waves (exhaustion signal not yet fired)"
    );
    // Wave 2 has 2 mobs; tick past the 0.70s inter-wave delay so both
    // pending entries spawn.
    enc.tick(ENCOUNTER_INTER_WAVE_DELAY_SECONDS + 0.01);
    // Both wave-2 mobs reported dead → the encounter completes through the
    // generic objective (exhaustion signal + all minions defeated).
    kill_all(&mut enc.parts);
    let events = enc.tick(0.001);
    assert_eq!(enc.lifecycle.phase(), EncounterPhase::Completed);
    assert!(events.contains(&EncounterEvent::Completed));
    assert!(events.contains(&EncounterEvent::LockChanged { locked: false }));
}

#[test]
fn player_death_fails_then_resets_for_a_fresh_attempt() {
    // The death adapter writes Fail + Reset in one command batch; the reducer
    // applies them in order — the trace sees the loss, and the next trigger
    // entry starts fresh.
    let mut enc = WaveEncounter::new(lab_spec());
    enc.start();
    enc.tick(0.001);
    let objective = enc.waves.objective();
    let events = enc.lifecycle.reduce(
        0.0,
        [&EncounterCommandKind::Fail, &EncounterCommandKind::Reset],
        &enc.parts,
        Some(&objective),
    );
    assert!(events.contains(&EncounterEvent::Failed));
    assert_eq!(enc.lifecycle.phase(), EncounterPhase::Inactive);
    assert!(!enc.lifecycle.phase().locks_exits());
}

#[test]
fn lock_active_truthy_during_active_phase() {
    let mut enc = WaveEncounter::new(lab_spec());
    enc.start();
    assert!(enc.lifecycle.phase().locks_exits());
    enc.tick(0.001);
    assert!(enc.lifecycle.phase().locks_exits());
}

#[test]
fn hud_summary_shows_wave_progress() {
    let mut enc = WaveEncounter::new(lab_spec());
    enc.start();
    enc.tick(0.001);
    let summary = enc.waves.hud_summary(enc.lifecycle.phase(), &enc.parts);
    assert!(summary.contains("WAVE 1/2"), "got: {summary}");
    assert!(summary.contains("1 left"), "got: {summary}");
}

// ── SwitchActivation parsing ──────────────────────────────────

#[test]
fn switch_activation_parses_full_payload() {
    let act =
        SwitchActivation::parse_custom("switch:reset:ResetEncounter:goblin_encounter").unwrap();
    assert_eq!(act.id, "reset");
    assert_eq!(act.action, "ResetEncounter");
    assert_eq!(act.target_encounter, "goblin_encounter");
}

#[test]
fn switch_activation_tolerates_empty_target() {
    let act = SwitchActivation::parse_custom("switch:reset:ResetEncounter:").unwrap();
    assert_eq!(act.target_encounter, "");
}

#[test]
fn switch_activation_rejects_non_switch_payload() {
    assert!(SwitchActivation::parse_custom("door:foo:bar").is_none());
    assert!(SwitchActivation::parse_custom("switch").is_none());
}

// ── EncounterRegistry ──────────────────────────────────────────

#[test]
fn registry_indexes_encounter_ids_to_entities() {
    // E1: the registry is a pure `id -> Entity` index; the live state lives on
    // the entity's lifecycle/wave components.
    let mut reg = EncounterRegistry::default();
    assert_eq!(reg.entity("goblin_encounter"), None);
    let e = bevy::prelude::Entity::PLACEHOLDER;
    reg.point_at_live_entity("goblin_encounter", e);
    assert_eq!(reg.entity("goblin_encounter"), Some(e));
    assert_eq!(reg.remove("goblin_encounter"), Some(e));
    assert_eq!(reg.entity("goblin_encounter"), None);

    // ⭐ REPLACEMENT IS THE POLICY, so it gets an arm rather than a comment.
    // This index points an id at whatever entity is LIVE; an encounter that
    // despawns and respawns gets a new one, and refusing the second write would
    // pin the index to a DEAD entity — the opposite of the defect the 2026-09-02
    // registry inventory is about. Stated in
    // `EncounterRegistry::point_at_live_entity` and asserted here so the
    // decision cannot be quietly reversed into refusal by somebody applying that
    // inventory's ruling mechanically.
    let respawned = bevy::prelude::Entity::from_raw_u32(4242).expect("a valid raw entity");
    reg.point_at_live_entity("goblin_encounter", e);
    reg.point_at_live_entity("goblin_encounter", respawned);
    assert_eq!(
        reg.entity("goblin_encounter"),
        Some(respawned),
        "a respawned encounter must take the index from its dead predecessor"
    );
}

#[test]
fn active_camera_zoom_picks_active_encounter() {
    let mut spec = lab_spec();
    spec.camera_zoom = 1.6;
    let mut enc = WaveEncounter::new(spec);
    enc.start();
    assert_eq!(
        active_encounter_camera_zoom([(enc.lifecycle.phase(), enc.waves.spec.camera_zoom)]),
        1.6
    );
}

#[test]
fn active_camera_zoom_falls_back_to_one_when_inactive() {
    let mut spec = lab_spec();
    spec.camera_zoom = 1.6;
    let enc = WaveEncounter::new(spec);
    // Phase still Inactive — no zoom applied.
    assert_eq!(
        active_encounter_camera_zoom([(enc.lifecycle.phase(), enc.waves.spec.camera_zoom)]),
        1.0
    );
}

#[test]
fn a_lifecycle_built_from_a_cleared_save_keeps_the_lock_off() {
    let mut enc = WaveEncounter::new(lab_spec());
    enc.lifecycle =
        EncounterLifecycle::from_persisted(0.0, PersistedEncounterState::Cleared);
    assert_eq!(enc.lifecycle.phase(), EncounterPhase::Completed);
    assert!(!enc.lifecycle.phase().locks_exits());
}

#[test]
fn to_persisted_collapses_active_to_untouched() {
    let mut enc = WaveEncounter::new(lab_spec());
    enc.start();
    assert_eq!(
        enc.lifecycle.to_persisted(),
        PersistedEncounterState::Untouched
    );
}

// ── Encounter loader ───────────────────────────────────────────
//
// So they still load the shipped sandbox world and CONVERT it, which means a converter that
// stopped emitting a trigger fails here rather than at runtime.

/// The shipped sandbox world, converted through the real room pipeline.
fn sandbox_rooms() -> Vec<ambition_platformer2d_world::rooms::RoomSpec> {
    let manifest = test_world_manifest();
    let project = LdtkProject::load_default_for_dev(&manifest).expect("sandbox LDtk should load");
    project
        .to_room_set(
            &manifest,
            &ambition_platformer2d_ldtk::LdtkVocabulary::engine(),
        )
        .unwrap_or_else(|errors| panic!("sandbox converts to rooms: {errors:?}"))
        .rooms
}

#[test]
fn load_encounter_specs_picks_up_goblin_encounter() {
    let rooms = sandbox_rooms();
    let save = ambition_persistence::save_data::AmbitionGameSaveData::default();
    let entries = load_encounter_specs_from_rooms(&rooms, &save, None);
    let goblin_encounter = entries
        .iter()
        .find(|(id, _, _)| id == "goblin_encounter")
        .expect("goblin_encounter encounter should be loadable");
    assert!(!goblin_encounter.1.waves.is_empty());
    assert!(goblin_encounter.1.camera_zoom > 1.0);
    assert_eq!(goblin_encounter.2, PersistedEncounterState::Untouched);
}

#[test]
fn load_encounter_specs_respects_persisted_cleared() {
    let rooms = sandbox_rooms();
    let mut save = ambition_persistence::save_data::AmbitionGameSaveData::default();
    save.set_encounter("goblin_encounter", PersistedEncounterState::Cleared);
    let entries = load_encounter_specs_from_rooms(&rooms, &save, None);
    let (_, _, state) = entries
        .iter()
        .find(|(id, _, _)| id == "goblin_encounter")
        .expect("goblin_encounter encounter should be loadable");
    assert_eq!(*state, PersistedEncounterState::Cleared);
}

#[test]
fn ldtk_switch_runtime_id_matches_activation_payload() {
    let manifest = test_world_manifest();
    let project = LdtkProject::load_default_for_dev(&manifest).expect("sandbox LDtk should load");
    let room_set = project
        .to_room_set(
            &manifest,
            &ambition_platformer2d_ldtk::LdtkVocabulary::engine(),
        )
        .expect("goblin_encounter world composes");
    let goblin_encounter = room_set
        .rooms
        .iter()
        .find(|r| r.id == "goblin_encounter")
        .expect("goblin_encounter room");
    // Interactables lower through the single `placements` channel (fable audit
    // F9.2); the switch is authored as an `Interactable` placement record.
    let switch_object = goblin_encounter
        .placements
        .iter()
        .find(|record| matches!(
            &record.schema,
            PlacementSchema::Interactable(spec)
                if matches!(&spec.kind, InteractionKindSpec::Custom(s) if s.starts_with("switch:"))
        ))
        .expect("goblin_encounter has a switch interactable placement");
    let payload = match &switch_object.schema {
        PlacementSchema::Interactable(spec) => match &spec.kind {
            InteractionKindSpec::Custom(s) => s.clone(),
            _ => panic!("switch kind"),
        },
        _ => panic!("switch placement schema"),
    };
    let activation = SwitchActivation::parse_custom(&payload).expect("parse");
    assert_eq!(
        switch_object.id.as_str(),
        activation.id,
        "Authored switch id must equal the SwitchActivation.id so set_switch_on works"
    );
}

#[test]
fn goblin_encounter_loaded_spec_has_three_waves_lockwall_and_intro() {
    let rooms = sandbox_rooms();
    let save = ambition_persistence::save_data::AmbitionGameSaveData::default();
    let entries = load_encounter_specs_from_rooms(&rooms, &save, None);
    let (_, spec, _) = entries
        .iter()
        .find(|(id, _, _)| id == "goblin_encounter")
        .expect("goblin_encounter encounter should be loadable");
    assert_eq!(
        spec.waves.len(),
        3,
        "expected 3 waves; got {}",
        spec.waves.len()
    );
    assert_eq!(spec.waves[0].mobs.len(), 2);
    assert_eq!(spec.waves[1].mobs.len(), 3, "wave 2 = 2 goblins + 1 big");
    assert_eq!(spec.waves[2].mobs.len(), 2, "wave 3 = 2 big goblins");
    // Wave 2's third mob should have a delay > 0 (the timer-based
    // big-goblin reinforcement).
    assert!(
        spec.waves[1].mobs.iter().any(|m| m.delay > 0.0),
        "wave 2 should have at least one delayed sub-spawn"
    );
    assert!(
        spec.lock_wall.is_some(),
        "goblin_encounter spec should pick up the LockWall marker"
    );
    assert!(spec.intro_seconds > 0.0);
    // goblin_encounter is driven by generated_music.rs (intro → adaptive
    // stem loops → outro), so its EncounterSpec deliberately has
    // an empty `music_track` — the encounter system must NOT push
    // a `RoomMusicRequest` swap on entry. See the conditional in
    // `load_encounter_specs_from_ldtk`.
    assert_eq!(spec.music_track, "");
}

// ── Multi-wave spawning behavior ───────────────────────────────

#[test]
fn intro_delays_first_wave_spawn_until_elapsed() {
    let mut spec = lab_spec();
    spec.intro_seconds = 1.5;
    let mut enc = WaveEncounter::new(spec);
    enc.start();
    // Halfway through the intro: still Starting, no spawns yet (the director
    // only runs while Active).
    let evs = enc.tick(0.5);
    assert!(matches!(
        enc.lifecycle.phase(),
        EncounterPhase::Starting { .. }
    ));
    assert!(!evs
        .iter()
        .any(|e| matches!(e, EncounterEvent::SpawnCommand { .. })));
    // After the rest of the intro: Active; the NEXT tick spawns (the adapter
    // reads the reducer's phase, one frame behind at most).
    enc.tick(1.2);
    assert_eq!(enc.lifecycle.phase(), EncounterPhase::Active);
    let evs = enc.tick(0.001);
    assert!(evs
        .iter()
        .any(|e| matches!(e, EncounterEvent::SpawnCommand { .. })));
}

#[test]
fn delayed_sub_spawn_holds_then_fires() {
    let mut spec = lab_spec();
    spec.intro_seconds = 0.0;
    // One immediate, one delayed-by-2s.
    spec.waves = vec![EncounterWaveSpec {
        label: "wave 1".into(),
        mobs: vec![
            EncounterMobSpec::new("medium_striker", [100.0, 100.0]),
            EncounterMobSpec::new("large_brute", [200.0, 100.0]).with_delay(2.0),
        ],
    }];
    let mut enc = WaveEncounter::new(spec);
    enc.start();
    // First Active tick: wave 1 starts, immediate mob spawns.
    let evs = enc.tick(0.5);
    let immediate_spawns = evs
        .iter()
        .filter(|e| matches!(e, EncounterEvent::SpawnCommand { .. }))
        .count();
    assert_eq!(immediate_spawns, 1);
    // Tick to 1.0s wave-elapsed: still nothing new.
    let evs = enc.tick(0.5);
    assert_eq!(
        evs.iter()
            .filter(|e| matches!(e, EncounterEvent::SpawnCommand { .. }))
            .count(),
        0
    );
    // Tick past 2.0s: delayed mob fires.
    let evs = enc.tick(1.5);
    assert_eq!(
        evs.iter()
            .filter(|e| matches!(e, EncounterEvent::SpawnCommand { .. }))
            .count(),
        1
    );
}

#[test]
fn wave_clears_only_when_all_pending_and_alive_are_resolved() {
    let mut spec = lab_spec();
    spec.intro_seconds = 0.0;
    spec.waves = vec![
        EncounterWaveSpec {
            label: "wave 1".into(),
            mobs: vec![
                EncounterMobSpec::new("medium_striker", [100.0, 100.0]),
                EncounterMobSpec::new("medium_striker", [200.0, 100.0]).with_delay(1.0),
            ],
        },
        EncounterWaveSpec {
            label: "wave 2".into(),
            mobs: vec![EncounterMobSpec::new("large_brute", [150.0, 100.0])],
        },
    ];
    let mut enc = WaveEncounter::new(spec);
    enc.start();
    // First Active tick: wave 0 armed; immediate mob spawned (alive).
    enc.tick(0.001);
    // 0.5s elapsed: alive mob marked dead, but the delayed mob hasn't fired
    // yet → wave still pending, no advance.
    kill_all(&mut enc.parts);
    enc.tick(0.5);
    assert_eq!(enc.waves.run.wave_index, Some(0));
    // 1.001s wave-elapsed: delayed mob spawns (appended alive AFTER the
    // refresh, so it survives this tick).
    kill_all(&mut enc.parts);
    enc.tick(0.5);
    assert_eq!(
        enc.waves.run.wave_index,
        Some(0),
        "wave 1 should hold while the just-spawned mob is alive"
    );
    // Next tick: refresh reports the just-spawned mob dead → wave clears,
    // wave 2 starts.
    kill_all(&mut enc.parts);
    enc.tick(0.001);
    assert_eq!(
        enc.waves.run.wave_index,
        Some(1),
        "expected wave 2 active, got {:?}",
        enc.waves.run
    );
}

#[test]
fn just_spawned_mob_survives_one_tick_before_liveness_refresh() {
    // The adapter refreshes `alive` BEFORE the director tick and fresh spawns append `alive = true`
    // after, so the wave (and the generic objective) hold for at least one frame.
    let mut spec = lab_spec();
    spec.intro_seconds = 0.0;
    spec.waves = vec![EncounterWaveSpec {
        label: "wave 1".into(),
        mobs: vec![EncounterMobSpec::new("medium_striker", [100.0, 100.0])],
    }];
    let mut enc = WaveEncounter::new(spec);
    enc.start();
    kill_all(&mut enc.parts);
    enc.tick(0.001);
    assert_eq!(
        enc.lifecycle.phase(),
        EncounterPhase::Active,
        "just-spawned mob must survive the first tick"
    );
    assert_eq!(enc.waves.remaining_mobs(&enc.parts), 1);
}

// ── Switch arming gate ─────────────────────────────────────────

fn switch_index(links: &[(&str, &str, bool)]) -> EncounterSwitchIndex {
    EncounterSwitchIndex {
        links: links
            .iter()
            .map(|(switch_id, target, on)| EncounterSwitchLink {
                switch_id: switch_id.to_string(),
                target_encounter: target.to_string(),
                on: *on,
            })
            .collect(),
    }
}

#[test]
fn encounter_armed_when_no_linked_switch() {
    assert!(switch_index(&[]).encounter_armed("goblin_encounter"));
}

#[test]
fn encounter_armed_when_linked_switch_off() {
    let index = switch_index(&[("goblin_encounter_reset_switch", "goblin_encounter", false)]);
    assert!(index.encounter_armed("goblin_encounter"));
}

#[test]
fn encounter_disarmed_when_linked_switch_on() {
    let index = switch_index(&[("goblin_encounter_reset_switch", "goblin_encounter", true)]);
    assert!(!index.encounter_armed("goblin_encounter"));
}

#[test]
fn unrelated_switches_dont_arm_other_encounters() {
    let index = switch_index(&[("boss_reset_switch", "boss_room", true)]);
    assert!(index.encounter_armed("goblin_encounter"));
    assert!(!index.encounter_armed("boss_room"));
}

#[test]
fn switch_ids_for_encounter_finds_linked_switches() {
    let index = switch_index(&[
        ("other_switch", "other_room", false),
        ("goblin_encounter_reset_switch", "goblin_encounter", false),
    ]);
    assert_eq!(
        index.switch_ids_for_encounter("goblin_encounter"),
        vec!["goblin_encounter_reset_switch".to_string()]
    );
    assert!(index.switch_ids_for_encounter("nonexistent").is_empty());
}

// ── Chest spawn position ───────────────────────────────────────

#[test]
fn encounter_reward_chest_pos_sits_on_trigger_floor() {
    let spec = lab_spec(); // trigger_min [0,0], trigger_size [400,200]
    let trigger = spec.trigger_aabb();
    let chest_size = ae::Vec2::new(28.0, 28.0);
    let chest_pos = encounter_reward_chest_pos(&spec, chest_size);
    let chest_bottom = chest_pos.y + chest_size.y * 0.5;
    assert!(
        (chest_bottom - trigger.max.y).abs() < 1e-3,
        "chest bottom ({chest_bottom}) must rest on trigger floor ({})",
        trigger.max.y
    );
    assert!((chest_pos.x - trigger.center().x).abs() < 1e-3);
}

// ── Lock wall sync ─────────────────────────────────────────────

#[test]
fn lock_wall_is_derived_while_active_and_dropped_when_inactive() {
    use crate::lock_walls::desired_lock_wall_blocks;
    let wall = LockWallSpec {
        min: [100.0, 100.0],
        size: [32.0, 200.0],
    };
    let mut enc = WaveEncounter::new(lab_spec());
    enc.start();
    // In-flight phase → the gate solid is derived this frame. Generic (E12):
    // the derivation reads the LIFECYCLE + the authored wall, never the kind.
    let blocks = desired_lock_wall_blocks([("goblin_encounter", enc.lifecycle.phase(), &wall)]);
    assert!(blocks.iter().any(|b| b.name == "lockwall:goblin_encounter"));
    // Reset back to Inactive — the overlay clears each frame, so "removal" is
    // simply the wall no longer being derived (no reconcile against a base).
    enc.lifecycle
        .reduce(0.0, [&EncounterCommandKind::Reset], &enc.parts, None);
    let blocks = desired_lock_wall_blocks([("goblin_encounter", enc.lifecycle.phase(), &wall)]);
    assert!(!blocks.iter().any(|b| b.name == "lockwall:goblin_encounter"));
}

// ── Staging is generic over the lifecycle (E12) ────────────────

/// E12 exit pin: a NON-wave encounter (no `EncounterWaves` anywhere) stages
/// exactly like an arena — the lock/camera consumers read the generic
/// lifecycle + authored staging policy, never the encounter kind. (The ECS
/// queries enforce the same at compile time: neither consumer names
/// `EncounterWaves` anymore.)
#[test]
fn a_non_wave_encounter_stages_the_same_lock_and_zoom() {
    use crate::lock_walls::desired_lock_wall_blocks;
    let mut lifecycle = EncounterLifecycle::default();
    lifecycle.reduce(
        0.0,
        [&EncounterCommandKind::Start],
        &EncounterParticipants::default(),
        None,
    );
    let wall = LockWallSpec {
        min: [0.0, 0.0],
        size: [16.0, 64.0],
    };
    let blocks = desired_lock_wall_blocks([("signal_puzzle", lifecycle.phase(), &wall)]);
    assert!(blocks.iter().any(|b| b.name == "lockwall:signal_puzzle"));
    assert_eq!(
        active_encounter_camera_zoom([(lifecycle.phase(), 1.4)]),
        1.4,
        "zoom derives from the staging policy, not the wave component"
    );
}

// ── Ownership-driven cleanup (E10) ─────────────────────────────

mod cleanup {
    use crate::apply_encounter_cleanup;
    use super::*;
    use ambition_encounter::{
        reduce_encounter_lifecycles, EncounterCleanupPolicy, EncounterCommand,
        EncounterCommandKind, EncounterEventMsg, EncounterLifecycle, Ownership, SpawnedCleanup,
    };
    use bevy::prelude::*;

    /// Minimal-plugin App running the REAL reducer + cleanup adapter, chained
    /// exactly as the sim registers them.
    fn cleanup_app() -> App {
        let mut app = App::new();
        app.init_resource::<ambition_platformer2d_shared_tangle::time::SimDt>();
        app.add_message::<EncounterCommand>();
        app.add_message::<EncounterEventMsg>();
        app.add_systems(
            Update,
            (reduce_encounter_lifecycles, apply_encounter_cleanup).chain(),
        );
        app
    }

    /// An encounter with one SPAWNED and one ADOPTED participant, both
    /// resolved to live entities. Returns (spawned_entity, adopted_entity).
    fn spawn_mixed_encounter(
        app: &mut App,
        policy: Option<EncounterCleanupPolicy>,
    ) -> (Entity, Entity) {
        let spawned = app.world_mut().spawn_empty().id();
        let adopted = app.world_mut().spawn_empty().id();
        let mut spawned_member =
            EncounterParticipant::spawned("mob_1", Some(spawned), EncounterRole::Minion);
        spawned_member.alive = true;
        let adopted_member =
            EncounterParticipant::adopted("npc_1", adopted, EncounterRole::Protected);
        let mut entity = app.world_mut().spawn((
            Encounter::new("arena"),
            EncounterLifecycle::default(),
            EncounterParticipants::new(vec![spawned_member, adopted_member]),
        ));
        if let Some(policy) = policy {
            entity.insert(policy);
        }
        app.world_mut()
            .write_message(EncounterCommand::new("arena", EncounterCommandKind::Start));
        app.update();
        (spawned, adopted)
    }

    fn members_of(app: &mut App) -> Vec<(String, Ownership)> {
        let mut q = app.world_mut().query::<&EncounterParticipants>();
        q.iter(app.world())
            .next()
            .expect("encounter exists")
            .members
            .iter()
            .map(|m| (m.id.clone(), m.ownership))
            .collect()
    }

    /// E10 exit: a spawned-owned actor must NOT leak when the encounter ends
    /// under the (default) DespawnOnEnd policy — and the adopted actor must
    /// NOT be despawned by the same cleanup.
    #[test]
    fn end_despawns_spawned_participants_and_never_adopted_ones() {
        let mut app = cleanup_app();
        let (spawned, adopted) = spawn_mixed_encounter(&mut app, None);
        assert!(app.world().get_entity(spawned).is_ok());

        app.world_mut()
            .write_message(EncounterCommand::new("arena", EncounterCommandKind::Fail));
        app.update();

        assert!(
            app.world().get_entity(spawned).is_err(),
            "a spawned-owned participant leaked past its encounter's end"
        );
        assert!(
            app.world().get_entity(adopted).is_ok(),
            "an ADOPTED participant was despawned by encounter cleanup"
        );
        // The relation records follow the entities: spawned rows leave the
        // list, adopted rows survive.
        assert_eq!(
            members_of(&mut app),
            vec![("npc_1".into(), Ownership::Adopted)]
        );
    }

    /// Reset (re-arm / area exit) is an end too: spawned participants follow
    /// the cleanup rule, adopted survive.
    #[test]
    fn reset_applies_the_same_ownership_rule() {
        let mut app = cleanup_app();
        let (spawned, adopted) = spawn_mixed_encounter(&mut app, None);
        app.world_mut()
            .write_message(EncounterCommand::new("arena", EncounterCommandKind::Reset));
        app.update();
        assert!(app.world().get_entity(spawned).is_err());
        assert!(app.world().get_entity(adopted).is_ok());
    }

    /// An authored `Keep` policy hands spawned participants to the room —
    /// cleanup consults the POLICY, not just the ownership enum. `Keep` is an
    /// explicit ownership RELEASE, not a silently still-owned leftover: the
    /// ended encounter drops its spawned relations while the bodies live on
    /// as ordinary unowned actors.
    #[test]
    fn keep_policy_releases_spawned_participants_but_leaves_them_alive() {
        let mut app = cleanup_app();
        let (spawned, adopted) = spawn_mixed_encounter(
            &mut app,
            Some(EncounterCleanupPolicy {
                spawned: SpawnedCleanup::Keep,
            }),
        );
        app.world_mut().write_message(EncounterCommand::new(
            "arena",
            EncounterCommandKind::Complete,
        ));
        app.update();
        assert!(
            app.world().get_entity(spawned).is_ok(),
            "Keep policy must leave spawned participants alive in the world"
        );
        assert!(app.world().get_entity(adopted).is_ok());
        assert_eq!(
            members_of(&mut app),
            vec![("npc_1".into(), Ownership::Adopted)],
            "an ended encounter owns nothing it spawned — Keep releases the relation"
        );
    }

    /// The generic durable-id → live-entity resolution: cleanup despawns a
    /// spawned participant whose entity CACHE is nulled (exactly a snapshot
    /// restore's participants) by resolving `SimId::placement(member.id)` —
    /// canonical simulation identity, not a type-specific marker query
    /// .
    #[test]
    fn cleanup_resolves_a_nulled_participant_cache_through_sim_identity() {
        let mut app = cleanup_app();
        let (spawned, adopted) = spawn_mixed_encounter(&mut app, None);
        // The body carries its canonical identity; the relation's cache is
        // nulled, as a restored world's would be.
        app.world_mut()
            .entity_mut(spawned)
            .insert(ambition_platformer2d_shared_tangle::sim_id::SimId::placement("mob_1"));
        {
            let mut q = app.world_mut().query::<&mut EncounterParticipants>();
            let mut parts = q.iter_mut(app.world_mut()).next().expect("encounter");
            parts.members[0].entity = None;
        }
        app.world_mut()
            .write_message(EncounterCommand::new("arena", EncounterCommandKind::Fail));
        app.update();
        assert!(
            app.world().get_entity(spawned).is_err(),
            "a spawned participant with a nulled cache must still clean up, \
             resolved by its canonical SimId"
        );
        assert!(app.world().get_entity(adopted).is_ok());
    }
}

/// A death-reset must NOT retire the reward. Only a deliberate switch re-arm does.
///
/// ⛔ THE RULE THIS PINS, and it is a game rule rather than a plumbing one:
/// *dying and re-running an encounter does not re-pay its reward; deliberately
/// re-arming it does.* [`apply_encounter_cleanup`] reacts to
/// `Completed | Failed | Reset` and releases or despawns the encounter's spawned
/// PARTICIPANTS. It must never touch the reward chest, because the death road
/// resets an encounter without any switch toggle — and the switch-off path is
/// the only thing that clears the chest and the `reward_looted` flag.
///
/// ⚠ WRITTEN BECAUSE THE OBVIOUS REFACTOR IS WRONG. Retiring the reward looks
/// like it belongs beside this cleanup: both react to the encounter ending, and
/// moving it here would free the last kernel seam in the encounter adapter. It
/// would also clear the flag on every death, so a player who dies after looting
/// could re-clear the encounter and be paid again. This test fails the moment
/// someone makes that move.
#[test]
fn a_reset_does_not_retire_the_reward_chest() {
    use ambition_combat::components::{ChestFeature, EncounterRewardChest, FeatureId};
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_message::<ambition_encounter::EncounterEventMsg>();

    app.world_mut().spawn((
        Encounter {
            id: "goblin_encounter".into(),
        },
        EncounterParticipants::default(),
    ));
    let chest = app
        .world_mut()
        .spawn((
            ChestFeature::new(ambition_interaction::Chest::new(
                "encounter_chest_goblin_encounter",
                None,
            )),
            EncounterRewardChest {
                encounter_id: "goblin_encounter".into(),
            },
            FeatureId("encounter_chest_goblin_encounter".into()),
        ))
        .id();

    app.world_mut()
        .resource_mut::<Messages<ambition_encounter::EncounterEventMsg>>()
        .write(ambition_encounter::EncounterEventMsg::new(
            "goblin_encounter",
            EncounterEvent::Reset,
        ));

    app.add_systems(Update, crate::apply_encounter_cleanup);
    app.update();

    assert!(
        app.world().get_entity(chest).is_ok(),
        "a Reset must leave the reward chest standing — the death road resets an \
         encounter with no switch toggle, and only a deliberate re-arm retires \
         the reward. Despawning it here would pay the encounter out twice."
    );
}

/// ⛔⛔ THE PRODUCTION SEAM THE MULTI-SWITCH REPAIR WAS ABOUT, end to end.
///
/// The two halves of the switch gate disagreed: `encounter_armed` arms on ANY
/// red link, while completion asked `switch_id_for_encounter` for the FIRST
/// link only and greened that one. On a two-switch arena the encounter
/// therefore completed, greened one switch, stayed armed on the other — and the
/// driver re-started the fight the player had just finished, under them.
///
/// ⚠ The unit tests either side of this one establish the ARMING rule and the
/// INDEX lookup separately, and both passed throughout the defect. What was
/// missing is the sequence that actually failed: complete a real wave encounter
/// through `apply_wave_encounter_effects`, persist, rebuild the index from the
/// save, and ask whether it re-arms. A cross-system defect needs the
/// cross-system test; two green halves are what let this ship.
#[test]
fn completing_a_two_switch_encounter_greens_both_and_leaves_it_disarmed() {
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_message::<ambition_encounter::EncounterEventMsg>();
    app.add_message::<ambition_combat::events::GameplayBannerRequested>();
    app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
    app.init_resource::<ambition_gameplay_trace::GameplayTraceBuffer>();
    app.init_resource::<ambition_encounter::EncounterView>();
    app.init_resource::<ambition_persistence::quest::QuestRegistry>();

    // TWO links, both red: the shape the first-link-only completion mishandled.
    app.insert_resource(switch_index(&[
        ("arena_switch_north", "twin_switch_arena", false),
        ("arena_switch_south", "twin_switch_arena", false),
    ]));

    // A live session, or the system's own guard returns before any effect.
    let mut active =
        ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope::default();
    let scope = active.begin();
    app.insert_resource(active);
    // ⛔⛔ THE SESSION ROOT MUST CARRY `EncounterMusicRequest`. The system takes
    // it as `SessionWorldMut<EncounterMusicRequest>` — a `Single` — and Bevy
    // SKIPS a system whose `Single` does not match exactly one entity. Silently:
    // no panic, no warning, the system simply never runs. A harness that omits
    // it gets a green "nothing happened" and would have made this acceptance
    // test pass for the wrong reason if it asserted absence instead of presence.
    app.world_mut().spawn((
        ambition_platformer2d_shared_tangle::lifecycle::SessionRoot(scope),
        ambition_encounter::EncounterMusicRequest::default(),
    ));

    // A wave encounter — completion effects apply only to encounters carrying
    // the wave policy.
    app.world_mut().spawn((
        Encounter {
            id: "twin_switch_arena".into(),
        },
        EncounterLifecycle::default(),
        // A real wave policy: completion effects apply only to encounters
        // that carry one, which is the branch this test must reach.
        EncounterWaves::new(lab_spec()),
        EncounterParticipants::default(),
    ));

    // A player body, or the system returns before the completion effects.
    app.world_mut().spawn((
        ambition_platformer2d_core::BodyKinematics::default(),
        ambition_platformer2d_shared_tangle::markers::PlayerEntity,
    ));

    app.world_mut()
        .resource_mut::<Messages<ambition_encounter::EncounterEventMsg>>()
        .write(ambition_encounter::EncounterEventMsg::new(
            "twin_switch_arena",
            EncounterEvent::Completed,
        ));

    app.add_systems(Update, crate::apply_wave_encounter_effects);
    // One frame to settle message buffers, then the completion event.
    app.update();
    app.world_mut()
        .resource_mut::<Messages<ambition_encounter::EncounterEventMsg>>()
        .write(ambition_encounter::EncounterEventMsg::new(
            "twin_switch_arena",
            EncounterEvent::Completed,
        ));
    app.update();

    // 1. BOTH switches persisted green, not just the first.
    let save = app
        .world()
        .resource::<ambition_persistence::save::AmbitionGameSave>();
    for switch_id in ["arena_switch_north", "arena_switch_south"] {
        assert!(
            save.data().switch(switch_id),
            "completion must green EVERY linked switch; `{switch_id}` is still red, \
             which is what left the arena armed after the player cleared it"
        );
    }

    // 2. Rebuild the index from what was persisted — the production road back
    //    into the arming gate — and confirm the encounter does not re-arm.
    let rebuilt = switch_index(&[
        (
            "arena_switch_north",
            "twin_switch_arena",
            save.data().switch("arena_switch_north"),
        ),
        (
            "arena_switch_south",
            "twin_switch_arena",
            save.data().switch("arena_switch_south"),
        ),
    ]);
    assert!(
        !rebuilt.encounter_armed("twin_switch_arena"),
        "a cleared arena must stay cleared: one red link re-arms it and the wave \
         driver restarts the fight under the player"
    );
}
