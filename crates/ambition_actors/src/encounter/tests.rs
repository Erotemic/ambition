//! Integration tests for the encounter module: state-machine flow
//! (start/wave/clear/fail/retry), multi-wave + delayed sub-spawn timing,
//! switch arming, LDtk loading of the `goblin_encounter` fixture, reward-chest
//! placement, and lock-wall sync.

use super::*;
use crate::encounter::switches::{EncounterSwitchIndex, EncounterSwitchLink};
use crate::ldtk_world::LdtkProject;
use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;
use ambition_persistence::save_data::PersistedEncounterState;

fn install_test_world_manifest() {
    use crate::ldtk_world::{install_world_manifest, WorldManifest, WorldSource};
    use ambition_asset_manager::AssetId;
    let worlds_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ambition_content/assets/worlds");
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

/// Drive an EncounterState past `Starting` into the first wave's
/// `Active` phase. The lab_spec uses `intro_seconds: 0.0` so a
/// single tick is enough.
fn advance_past_intro(state: &mut EncounterState) {
    let _ = state.tick_intro_or_wave(0.001, |_| true);
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
        // Tests want immediate spawn on entry — skip the intro
        // delay so `entering_trigger_starts_first_wave` etc. can
        // check the Active state right after `maybe_start`.
        intro_seconds: 0.0,
        music_track: String::new(),
        reward: super::spec::default_encounter_reward(),
    }
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
    let mut state = EncounterState::default();
    state.spec = Some(lab_spec());
    let events = state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
    assert!(state.lock_active);
    assert!(matches!(state.phase, EncounterPhase::Starting { .. }));
    assert!(events
        .iter()
        .any(|e| matches!(e, EncounterEvent::Started { .. })));
    assert!(events
        .iter()
        .any(|e| matches!(e, EncounterEvent::LockChanged { locked: true })));
    // After the intro tick, we land in Active{wave 0}.
    advance_past_intro(&mut state);
    assert_eq!(
        state.phase,
        EncounterPhase::Active {
            wave_index: 0,
            remaining_mobs: 1,
        }
    );
}

#[test]
fn standing_outside_trigger_does_not_start() {
    let mut state = EncounterState::default();
    state.spec = Some(lab_spec());
    let events = state.maybe_start(ae::Vec2::new(2000.0, 50.0), ae::Vec2::new(20.0, 30.0));
    assert!(events.is_empty());
    assert_eq!(state.phase, EncounterPhase::Inactive);
    assert!(!state.lock_active);
}

#[test]
fn defeating_all_mobs_clears_each_wave_and_then_encounter() {
    let mut state = EncounterState::default();
    state.spec = Some(lab_spec());
    state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
    // `advance_past_intro` also spawns wave 1's single mob (delay 0).
    advance_past_intro(&mut state);
    // Wave 1's mob is reported dead → wave advances to wave 2.
    let _ = state.tick_intro_or_wave(0.001, |_| false);
    // Wave 2 has 2 mobs; tick past the 0.70s inter-wave delay so both
    // pending entries spawn before they can be reported dead.
    let _ = state.tick_intro_or_wave(ENCOUNTER_INTER_WAVE_DELAY_SECONDS + 0.01, |_| true);
    // Both wave-2 mobs reported dead → the encounter clears.
    let events = state.tick_intro_or_wave(0.001, |_| false);
    assert_eq!(state.phase, EncounterPhase::Cleared);
    assert!(!state.lock_active);
    assert!(events
        .iter()
        .any(|e| matches!(e, EncounterEvent::Cleared { .. })));
    assert!(events
        .iter()
        .any(|e| matches!(e, EncounterEvent::LockChanged { locked: false })));
}

#[test]
fn player_death_during_active_encounter_unlocks_and_marks_failed() {
    let mut state = EncounterState::default();
    state.spec = Some(lab_spec());
    state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
    advance_past_intro(&mut state);
    let events = state.on_player_death();
    assert_eq!(state.phase, EncounterPhase::Failed);
    assert!(!state.lock_active);
    assert!(events
        .iter()
        .any(|e| matches!(e, EncounterEvent::Failed { .. })));
}

#[test]
fn reset_for_retry_returns_to_inactive_after_failure() {
    let mut state = EncounterState::default();
    state.spec = Some(lab_spec());
    state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
    advance_past_intro(&mut state);
    state.on_player_death();
    state.reset_for_retry();
    assert_eq!(state.phase, EncounterPhase::Inactive);
}

#[test]
fn lock_active_truthy_during_active_phase() {
    let mut state = EncounterState::default();
    state.spec = Some(lab_spec());
    state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
    assert!(state.phase.locks_exits());
    assert!(state.lock_active);
    advance_past_intro(&mut state);
    assert!(state.phase.locks_exits());
}

#[test]
fn hud_summary_shows_wave_progress() {
    let mut state = EncounterState::default();
    state.spec = Some(lab_spec());
    state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
    advance_past_intro(&mut state);
    let summary = state.hud_summary();
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
fn registry_ensure_creates_default_state() {
    let mut reg = EncounterRegistry::default();
    let state = reg.ensure("goblin_encounter");
    assert_eq!(state.phase, EncounterPhase::Inactive);
}

#[test]
fn registry_active_camera_zoom_picks_active_encounter() {
    let mut reg = EncounterRegistry::default();
    let mut spec = lab_spec();
    spec.camera_zoom = 1.6;
    let state = reg.ensure("goblin_encounter");
    state.spec = Some(spec);
    state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
    assert_eq!(reg.active_camera_zoom(), 1.6);
}

#[test]
fn registry_camera_zoom_falls_back_to_one_when_inactive() {
    let mut reg = EncounterRegistry::default();
    reg.ensure("goblin_encounter").spec = Some({
        let mut s = lab_spec();
        s.camera_zoom = 1.6;
        s
    });
    // Phase still Inactive — no zoom applied.
    assert_eq!(reg.active_camera_zoom(), 1.0);
}

#[test]
fn apply_persisted_cleared_keeps_lock_off() {
    let mut state = EncounterState::default();
    state.spec = Some(lab_spec());
    state.apply_persisted(PersistedEncounterState::Cleared);
    assert_eq!(state.phase, EncounterPhase::Cleared);
    assert!(!state.lock_active);
}

#[test]
fn to_persisted_collapses_active_to_untouched() {
    let mut state = EncounterState::default();
    state.spec = Some(lab_spec());
    state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
    assert_eq!(state.to_persisted(), PersistedEncounterState::Untouched);
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
    let switch_object = goblin_encounter
        .interactables
        .iter()
        .find(|authored| matches!(&authored.payload.kind, ambition_interaction::InteractionKind::Custom(s) if s.starts_with("switch:")))
        .expect("goblin_encounter has a switch interactable");
    let payload = match &switch_object.payload.kind {
        ambition_interaction::InteractionKind::Custom(s) => s.clone(),
        _ => panic!("switch kind"),
    };
    let activation = SwitchActivation::parse_custom(&payload).expect("parse");
    assert_eq!(
        switch_object.id, activation.id,
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
    let mut state = EncounterState::default();
    let mut spec = lab_spec();
    spec.intro_seconds = 1.5;
    state.spec = Some(spec);
    state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
    // Halfway through the intro: still Starting, no spawns yet.
    let evs = state.tick_intro_or_wave(0.5, |_| true);
    assert!(matches!(state.phase, EncounterPhase::Starting { .. }));
    assert!(!evs
        .iter()
        .any(|e| matches!(e, EncounterEvent::SpawnCommand { .. })));
    // After the rest of the intro: Active + a spawn command.
    let evs = state.tick_intro_or_wave(1.2, |_| true);
    assert!(matches!(state.phase, EncounterPhase::Active { .. }));
    assert!(evs
        .iter()
        .any(|e| matches!(e, EncounterEvent::SpawnCommand { .. })));
}

#[test]
fn delayed_sub_spawn_holds_then_fires() {
    let mut state = EncounterState::default();
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
    state.spec = Some(spec);
    state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
    // Tick once: intro elapses, wave 1 starts, immediate mob spawns.
    let evs = state.tick_intro_or_wave(0.5, |_| true);
    let immediate_spawns = evs
        .iter()
        .filter(|e| matches!(e, EncounterEvent::SpawnCommand { .. }))
        .count();
    assert_eq!(immediate_spawns, 1);
    // Tick to 1.0s wave-elapsed: still nothing new.
    let evs = state.tick_intro_or_wave(0.5, |_| true);
    assert_eq!(
        evs.iter()
            .filter(|e| matches!(e, EncounterEvent::SpawnCommand { .. }))
            .count(),
        0
    );
    // Tick past 2.0s: delayed mob fires.
    let evs = state.tick_intro_or_wave(1.5, |_| true);
    assert_eq!(
        evs.iter()
            .filter(|e| matches!(e, EncounterEvent::SpawnCommand { .. }))
            .count(),
        1
    );
}

#[test]
fn wave_clears_only_when_all_pending_and_alive_are_resolved() {
    let mut state = EncounterState::default();
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
    state.spec = Some(spec);
    state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
    // Intro tick: Starting → Active{wave 0}; immediate mob spawned.
    // Closure says "alive" so the just-spawned id sticks.
    let _ = state.tick_intro_or_wave(0.001, |_| true);
    // 0.5s elapsed: alive mob marked dead, but the delayed mob
    // hasn't fired yet → wave still pending.
    let _ = state.tick_intro_or_wave(0.5, |_| false);
    assert!(matches!(
        state.phase,
        EncounterPhase::Active { wave_index: 0, .. }
    ));
    // 1.001s wave-elapsed: delayed mob spawns. Retain runs first
    // (no alive ids to drop; closure won't see new id this tick).
    let _ = state.tick_intro_or_wave(0.5, |_| false);
    // Still wave 1: the just-spawned mob is alive in the encounter
    // bookkeeping (not yet been retained against a stale lookup).
    assert!(
        matches!(state.phase, EncounterPhase::Active { wave_index: 0, .. }),
        "wave 1 should hold while the just-spawned mob is alive"
    );
    // Next tick: retain drops the just-spawned mob (closure
    // returns false), wave clears, wave 2 starts.
    let _ = state.tick_intro_or_wave(0.001, |_| false);
    assert!(
        matches!(state.phase, EncounterPhase::Active { wave_index: 1, .. }),
        "expected wave 2 active, got {:?}",
        state.phase
    );
}

#[test]
fn just_spawned_mob_survives_one_tick_before_retain() {
    // Regression for the "encounter ends after 2 seconds" bug:
    // newly-spawned mobs were immediately reaped because retain
    // ran AFTER spawn with a stale alive_lookup. The fix is to
    // run retain BEFORE spawn so the new id has a frame to live.
    let mut state = EncounterState::default();
    let mut spec = lab_spec();
    spec.intro_seconds = 0.0;
    spec.waves = vec![EncounterWaveSpec {
        label: "wave 1".into(),
        mobs: vec![EncounterMobSpec::new("medium_striker", [100.0, 100.0])],
    }];
    state.spec = Some(spec);
    state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
    // Intro elapses + spawn happens. Closure returns false (the
    // runtime hasn't seen the new id yet — the bug condition).
    let _ = state.tick_intro_or_wave(0.001, |_| false);
    // The mob must still be tracked: the wave shouldn't be cleared.
    assert!(
        matches!(
            state.phase,
            EncounterPhase::Active {
                wave_index: 0,
                remaining_mobs: 1
            }
        ),
        "just-spawned mob must survive the first tick; got {:?}",
        state.phase
    );
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
    let mut reg = EncounterRegistry::default();
    let mut spec = lab_spec();
    spec.lock_wall = Some(LockWallSpec {
        min: [100.0, 100.0],
        size: [32.0, 200.0],
    });
    let state = reg.ensure("goblin_encounter");
    state.spec = Some(spec);
    state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
    // Starting/Active phase → the gate solid is derived this frame.
    let blocks = desired_lock_wall_blocks(&reg);
    assert!(blocks.iter().any(|b| b.name == "lockwall:goblin_encounter"));
    // Force back to Inactive — the overlay clears each frame, so "removal" is
    // simply the wall no longer being derived (no reconcile against a base).
    let state = reg.ensure("goblin_encounter");
    state.phase = EncounterPhase::Inactive;
    let blocks = desired_lock_wall_blocks(&reg);
    assert!(!blocks.iter().any(|b| b.name == "lockwall:goblin_encounter"));
}
