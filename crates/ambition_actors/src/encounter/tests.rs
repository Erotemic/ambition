//! Integration tests for the encounter module: the wave adapter flow over the
//! generic lifecycle (start/wave/complete/fail/reset), multi-wave + delayed
//! sub-spawn timing, switch arming, LDtk loading of the `goblin_encounter`
//! fixture, reward-chest placement, and lock-wall sync.

use super::*;
use crate::encounter::switches::{EncounterSwitchIndex, EncounterSwitchLink};
use crate::ldtk_world::LdtkProject;
use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;
use ambition_entity_catalog::placements::PlacementSchema;
use ambition_persistence::save_data::PersistedEncounterState;
use ambition_world::rooms::InteractionKindSpec;
use bevy::math::bounding::IntersectsVolume;

fn install_test_world_manifest() {
    use crate::ldtk_world::{install_world_manifest, WorldManifest, WorldSource};
    use ambition_asset_manager::AssetId;
    let worlds_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../game/ambition_content/assets/worlds");
    install_world_manifest(WorldManifest {
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
    });
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
        let lifecycle = EncounterLifecycle::with_intro(spec.intro_seconds);
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
        if matches!(self.lifecycle.phase, EncounterPhase::Active)
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
        reward: super::spec::default_encounter_reward(),
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
        super::spec::default_encounter_reward(),
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
    assert!(enc.lifecycle.phase.locks_exits());
    assert!(events.contains(&EncounterEvent::Started));
    assert!(events.contains(&EncounterEvent::LockChanged { locked: true }));
    // First Active tick arms wave 0 and spawns its single mob.
    enc.tick(0.001);
    assert_eq!(enc.lifecycle.phase, EncounterPhase::Active);
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
        enc.lifecycle.phase,
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
    assert_eq!(enc.lifecycle.phase, EncounterPhase::Completed);
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
    assert_eq!(enc.lifecycle.phase, EncounterPhase::Inactive);
    assert!(!enc.lifecycle.phase.locks_exits());
}

#[test]
fn lock_active_truthy_during_active_phase() {
    let mut enc = WaveEncounter::new(lab_spec());
    enc.start();
    assert!(enc.lifecycle.phase.locks_exits());
    enc.tick(0.001);
    assert!(enc.lifecycle.phase.locks_exits());
}

#[test]
fn hud_summary_shows_wave_progress() {
    let mut enc = WaveEncounter::new(lab_spec());
    enc.start();
    enc.tick(0.001);
    let summary = enc.waves.hud_summary(enc.lifecycle.phase, &enc.parts);
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
    reg.insert("goblin_encounter", e);
    assert_eq!(reg.entity("goblin_encounter"), Some(e));
    assert_eq!(reg.remove("goblin_encounter"), Some(e));
    assert_eq!(reg.entity("goblin_encounter"), None);
}

#[test]
fn active_camera_zoom_picks_active_encounter() {
    let mut spec = lab_spec();
    spec.camera_zoom = 1.6;
    let mut enc = WaveEncounter::new(spec);
    enc.start();
    assert_eq!(
        active_encounter_camera_zoom([(enc.lifecycle.phase, enc.waves.spec.camera_zoom)]),
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
        active_encounter_camera_zoom([(enc.lifecycle.phase, enc.waves.spec.camera_zoom)]),
        1.0
    );
}

#[test]
fn apply_persisted_cleared_keeps_lock_off() {
    let mut enc = WaveEncounter::new(lab_spec());
    enc.lifecycle
        .apply_persisted(PersistedEncounterState::Cleared);
    assert_eq!(enc.lifecycle.phase, EncounterPhase::Completed);
    assert!(!enc.lifecycle.phase.locks_exits());
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

// ── LDtk loader ────────────────────────────────────────────────

#[test]
fn load_encounter_specs_picks_up_goblin_encounter() {
    install_test_world_manifest();
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let save = ambition_persistence::save_data::SandboxSaveData::default();
    let entries = load_encounter_specs_from_ldtk(&project, &save);
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
    install_test_world_manifest();
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let mut save = ambition_persistence::save_data::SandboxSaveData::default();
    save.set_encounter("goblin_encounter", PersistedEncounterState::Cleared);
    let entries = load_encounter_specs_from_ldtk(&project, &save);
    let (_, _, state) = entries
        .iter()
        .find(|(id, _, _)| id == "goblin_encounter")
        .expect("goblin_encounter encounter should be loadable");
    assert_eq!(*state, PersistedEncounterState::Cleared);
}

#[test]
fn ldtk_switch_runtime_id_matches_activation_payload() {
    // Regression for the bug where the Switch RoomObject id was
    // entity.iid (e.g. "Switch-4072") but the
    // SwitchActivation payload's id was the LDtk `id` field
    // ("goblin_encounter_reset_switch"). That mismatch made switch state
    // updates a no-op and the switch sprite stayed stuck red.
    install_test_world_manifest();
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let room_set = project
        .to_room_set()
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
    install_test_world_manifest();
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let save = ambition_persistence::save_data::SandboxSaveData::default();
    let entries = load_encounter_specs_from_ldtk(&project, &save);
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
        enc.lifecycle.phase,
        EncounterPhase::Starting { .. }
    ));
    assert!(!evs
        .iter()
        .any(|e| matches!(e, EncounterEvent::SpawnCommand { .. })));
    // After the rest of the intro: Active; the NEXT tick spawns (the adapter
    // reads the reducer's phase, one frame behind at most).
    enc.tick(1.2);
    assert_eq!(enc.lifecycle.phase, EncounterPhase::Active);
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
    // Regression heritage: the "encounter ends after 2 seconds" bug — a
    // freshly-spawned mob must not be counted dead by the SAME tick's stale
    // liveness. The adapter refreshes `alive` BEFORE the director tick and
    // fresh spawns append `alive = true` after, so the wave (and the generic
    // objective) hold for at least one frame.
    let mut spec = lab_spec();
    spec.intro_seconds = 0.0;
    spec.waves = vec![EncounterWaveSpec {
        label: "wave 1".into(),
        mobs: vec![EncounterMobSpec::new("medium_striker", [100.0, 100.0])],
    }];
    let mut enc = WaveEncounter::new(spec);
    enc.start();
    // The refresh reports all-dead (the runtime hasn't seen the new mob yet —
    // the bug condition), but the fresh spawn is added AFTER the refresh.
    kill_all(&mut enc.parts);
    enc.tick(0.001);
    assert_eq!(
        enc.lifecycle.phase,
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
fn switch_id_for_encounter_finds_linked_switch() {
    let index = switch_index(&[
        ("other_switch", "other_room", false),
        ("goblin_encounter_reset_switch", "goblin_encounter", false),
    ]);
    assert_eq!(
        index.switch_id_for_encounter("goblin_encounter"),
        Some("goblin_encounter_reset_switch".into())
    );
    assert_eq!(index.switch_id_for_encounter("nonexistent"), None);
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
    use super::lock_walls::desired_lock_wall_blocks;
    let wall = LockWallSpec {
        min: [100.0, 100.0],
        size: [32.0, 200.0],
    };
    let mut enc = WaveEncounter::new(lab_spec());
    enc.start();
    // In-flight phase → the gate solid is derived this frame. Generic (E12):
    // the derivation reads the LIFECYCLE + the authored wall, never the kind.
    let blocks = desired_lock_wall_blocks([("goblin_encounter", enc.lifecycle.phase, &wall)]);
    assert!(blocks.iter().any(|b| b.name == "lockwall:goblin_encounter"));
    // Reset back to Inactive — the overlay clears each frame, so "removal" is
    // simply the wall no longer being derived (no reconcile against a base).
    enc.lifecycle
        .reduce(0.0, [&EncounterCommandKind::Reset], &enc.parts, None);
    let blocks = desired_lock_wall_blocks([("goblin_encounter", enc.lifecycle.phase, &wall)]);
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
    use super::lock_walls::desired_lock_wall_blocks;
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
    let blocks = desired_lock_wall_blocks([("signal_puzzle", lifecycle.phase, &wall)]);
    assert!(blocks.iter().any(|b| b.name == "lockwall:signal_puzzle"));
    assert_eq!(
        active_encounter_camera_zoom([(lifecycle.phase, 1.4)]),
        1.4,
        "zoom derives from the staging policy, not the wave component"
    );
}

// ── Ownership-driven cleanup (E10) ─────────────────────────────

mod cleanup {
    use super::*;
    use crate::encounter::apply_encounter_cleanup;
    use ambition_encounter::{
        reduce_encounter_lifecycles, EncounterCleanupPolicy, EncounterCommand,
        EncounterCommandKind, EncounterEventMsg, EncounterLifecycle, Ownership, SpawnedCleanup,
    };
    use bevy::prelude::*;

    /// Minimal-plugin App running the REAL reducer + cleanup adapter, chained
    /// exactly as the sim registers them.
    fn cleanup_app() -> App {
        let mut app = App::new();
        app.init_resource::<ambition_platformer_primitives::time::SimDt>();
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
    /// as ordinary unowned actors (GPT-5.6 review, 2026-07-16).
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
    /// (GPT-5.6 review, 2026-07-16).
    #[test]
    fn cleanup_resolves_a_nulled_participant_cache_through_sim_identity() {
        let mut app = cleanup_app();
        let (spawned, adopted) = spawn_mixed_encounter(&mut app, None);
        // The body carries its canonical identity; the relation's cache is
        // nulled, as a restored world's would be.
        app.world_mut().entity_mut(spawned).insert(
            ambition_platformer_primitives::sim_id::SimId::placement("mob_1"),
        );
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
