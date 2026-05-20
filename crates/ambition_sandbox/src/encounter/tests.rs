use super::*;
use crate::ae;
use crate::ae::{AabbExt, PersistedEncounterState};
use crate::encounter::switches::{EncounterSwitchIndex, EncounterSwitchLink};
use crate::ldtk_world::LdtkProject;

/// Drive an EncounterState past `Starting` into the first wave's
/// `Active` phase. The lab_spec uses `intro_seconds: 0.0` so a
/// single tick is enough.
fn advance_past_intro(state: &mut EncounterState) {
    let _ = state.tick_intro_or_wave(0.001, |_| true);
}

fn lab_spec() -> EncounterSpec {
    EncounterSpec {
        id: "mob_lab".into(),
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
    }
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
    advance_past_intro(&mut state);
    // Wave 1 has 1 mob; spawn it then mark defeated.
    let _ = state.tick_intro_or_wave(0.001, |_| true);
    let _ = state.on_mob_defeated();
    // Wave 2 has 2 mobs; tick past the 0.70s inter-wave delay so
    // both pending entries spawn (otherwise their delays are still
    // counting down and `on_mob_defeated`'s legacy alive_ids.pop
    // path no-ops, leaving the wave stuck).
    let _ = state.tick_intro_or_wave(ENCOUNTER_INTER_WAVE_DELAY_SECONDS + 0.01, |_| true);
    let _ = state.on_mob_defeated();
    let events = state.on_mob_defeated();
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
    let act = SwitchActivation::parse_custom("switch:reset:ResetEncounter:mob_lab").unwrap();
    assert_eq!(act.id, "reset");
    assert_eq!(act.action, "ResetEncounter");
    assert_eq!(act.target_encounter, "mob_lab");
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
    let state = reg.ensure("mob_lab");
    assert_eq!(state.phase, EncounterPhase::Inactive);
}

#[test]
fn registry_active_camera_zoom_picks_active_encounter() {
    let mut reg = EncounterRegistry::default();
    let mut spec = lab_spec();
    spec.camera_zoom = 1.6;
    let state = reg.ensure("mob_lab");
    state.spec = Some(spec);
    state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
    assert_eq!(reg.active_camera_zoom(), 1.6);
}

#[test]
fn registry_camera_zoom_falls_back_to_one_when_inactive() {
    let mut reg = EncounterRegistry::default();
    reg.ensure("mob_lab").spec = Some({
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
fn load_encounter_specs_picks_up_mob_lab() {
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let save = ae::SandboxSaveData::default();
    let entries = load_encounter_specs_from_ldtk(&project, &save);
    let mob_lab = entries
        .iter()
        .find(|(id, _, _)| id == "mob_lab")
        .expect("mob_lab encounter should be loadable");
    assert!(!mob_lab.1.waves.is_empty());
    assert!(mob_lab.1.camera_zoom > 1.0);
    assert_eq!(mob_lab.2, PersistedEncounterState::Untouched);
}

#[test]
fn load_encounter_specs_respects_persisted_cleared() {
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let mut save = ae::SandboxSaveData::default();
    save.set_encounter("mob_lab", PersistedEncounterState::Cleared);
    let entries = load_encounter_specs_from_ldtk(&project, &save);
    let (_, _, state) = entries
        .iter()
        .find(|(id, _, _)| id == "mob_lab")
        .expect("mob_lab encounter should be loadable");
    assert_eq!(*state, PersistedEncounterState::Cleared);
}

#[test]
fn ldtk_switch_runtime_id_matches_activation_payload() {
    // Regression for the bug where the Switch RoomObject id was
    // entity.iid (e.g. "Switch-4072") but the
    // SwitchActivation payload's id was the LDtk `id` field
    // ("mob_lab_reset_switch"). That mismatch made switch state
    // updates a no-op and the switch sprite stayed stuck red.
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let room_set = project.to_room_set().expect("mob_lab world composes");
    let mob_lab = room_set
        .rooms
        .iter()
        .find(|r| r.id == "mob_lab")
        .expect("mob_lab room");
    let switch_object = mob_lab
        .interactables
        .iter()
        .find(|authored| matches!(&authored.payload.kind, ae::InteractionKind::Custom(s) if s.starts_with("switch:")))
        .expect("mob_lab has a switch interactable");
    let payload = match &switch_object.payload.kind {
        ae::InteractionKind::Custom(s) => s.clone(),
        _ => panic!("switch kind"),
    };
    let activation = SwitchActivation::parse_custom(&payload).expect("parse");
    assert_eq!(
        switch_object.id, activation.id,
        "Authored switch id must equal the SwitchActivation.id so set_switch_on works"
    );
}

#[test]
fn mob_lab_loaded_spec_has_three_waves_lockwall_and_intro() {
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let save = ae::SandboxSaveData::default();
    let entries = load_encounter_specs_from_ldtk(&project, &save);
    let (_, spec, _) = entries
        .iter()
        .find(|(id, _, _)| id == "mob_lab")
        .expect("mob_lab encounter should be loadable");
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
        "mob_lab spec should pick up the LockWall marker"
    );
    assert!(spec.intro_seconds > 0.0);
    // mob_lab is driven by generated_music.rs (intro → adaptive
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
    assert!(switch_index(&[]).encounter_armed("mob_lab"));
}

#[test]
fn encounter_armed_when_linked_switch_off() {
    let index = switch_index(&[("mob_lab_reset_switch", "mob_lab", false)]);
    assert!(index.encounter_armed("mob_lab"));
}

#[test]
fn encounter_disarmed_when_linked_switch_on() {
    let index = switch_index(&[("mob_lab_reset_switch", "mob_lab", true)]);
    assert!(!index.encounter_armed("mob_lab"));
}

#[test]
fn unrelated_switches_dont_arm_other_encounters() {
    let index = switch_index(&[("boss_reset_switch", "boss_room", true)]);
    assert!(index.encounter_armed("mob_lab"));
    assert!(!index.encounter_armed("boss_room"));
}

#[test]
fn switch_id_for_encounter_finds_linked_switch() {
    let index = switch_index(&[
        ("other_switch", "other_room", false),
        ("mob_lab_reset_switch", "mob_lab", false),
    ]);
    assert_eq!(
        index.switch_id_for_encounter("mob_lab"),
        Some("mob_lab_reset_switch".into())
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
fn sync_lock_walls_inserts_and_removes_block() {
    use ambition_engine::Block;
    let mut world = ae::World::new(
        "test",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::ZERO,
        vec![Block::solid(
            "floor",
            ae::Vec2::ZERO,
            ae::Vec2::new(2000.0, 16.0),
        )],
    );
    let mut reg = EncounterRegistry::default();
    let mut spec = lab_spec();
    spec.lock_wall = Some(LockWallSpec {
        min: [100.0, 100.0],
        size: [32.0, 200.0],
    });
    let state = reg.ensure("mob_lab");
    state.spec = Some(spec);
    state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
    sync_lock_walls(&mut world, &reg);
    assert!(world.blocks.iter().any(|b| b.name == "lockwall:mob_lab"));
    // Force back to Inactive — wall should be removed.
    let state = reg.ensure("mob_lab");
    state.phase = EncounterPhase::Inactive;
    sync_lock_walls(&mut world, &reg);
    assert!(!world.blocks.iter().any(|b| b.name == "lockwall:mob_lab"));
}
