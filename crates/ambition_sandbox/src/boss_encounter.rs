//! Sandbox-side boss encounter coordinator.
//!
//! Bridges `ae::BossEncounterState` (the phase machine) with the
//! existing `BossRuntime` (the in-arena physical actor) and the
//! adaptive music + cutscene + save-state systems.
//!
//! Each `BossSpawn` LDtk entity in the active room maps to one
//! encounter id (defaulting to the boss `name`). When the player
//! enters the room the encounter goes Dormant → Intro and the
//! cutscene queue is asked to play `boss_intro_<id>`. From that point
//! the engine state machine drives transitions; this module mirrors
//! them onto the seldom_state `BossPhase` component, the audio
//! request, and the save resource.

use std::collections::BTreeMap;

use ambition_engine as ae;
use bevy::prelude::*;

use crate::cutscene::CutsceneTriggerQueue;
use crate::quest::QuestRegistry;

/// Encounter id of the pirate-cove boss. The chest dropped on its
/// defeat reuses the standard `encounter_chest_<id>` naming so the
/// existing open / looted-flag plumbing
/// (`crate::encounter::encounter_reward_looted_flag`) handles
/// persistence with no special case.
///
/// GENERALIZATION PLAN: this id + the chest constants below are the
/// only mockingbird-specific knobs in `sync_mockingbird_treasure_chest`.
/// When a second boss needs an on-defeat drop, lift these into a small
/// data-driven table — e.g. a `BossDeathReward { encounter_id,
/// chest_size, drop_offset, reward: PickupKind }` map registered next
/// to `default_boss_specs()`. The sync function then iterates the
/// table instead of hard-coding `MOCKINGBIRD_ENCOUNTER_ID`. We're
/// intentionally NOT building the abstraction yet (one example isn't
/// a pattern), but the named-after-the-thing function is the smell
/// that points to the future refactor.
pub const MOCKINGBIRD_ENCOUNTER_ID: &str = "mockingbird";

/// World-space y-offset applied to the chest's *spawn point*,
/// relative to the mockingbird's spawn anchor. The bird is airborne,
/// so the chest is born just below the bird and then lets gravity
/// (`features::CHEST_FALL_GRAVITY`) carry it the rest of the way to
/// whatever solid block the arena puts under it. A small offset keeps
/// the spawn from clipping the bird's own collision while still
/// reading as "the chest tumbled out of the bird's grip".
const MOCKINGBIRD_CHEST_DROP_OFFSET_Y: f32 = 24.0;
/// Pirate hoard footprint. Deliberately oversized — narratively a
/// pirate galleon's hoard is a *big* chest, and visually we want a
/// satisfying target the player can't miss in the arena.
const MOCKINGBIRD_CHEST_SIZE: ae::Vec2 = ae::Vec2::new(56.0, 56.0);

#[derive(Resource, Default)]
pub struct BossEncounterRegistry {
    pub encounters: BTreeMap<String, ae::BossEncounterState>,
    /// id -> the boss runtime id we wired to. Used to route damage.
    pub runtime_ids: BTreeMap<String, String>,
    /// True once we've registered the default boss specs.
    pub specs_loaded: bool,
}

impl BossEncounterRegistry {
    pub fn ensure(&mut self, spec: ae::BossEncounterSpec) {
        let id = spec.id.clone();
        self.encounters
            .entry(id)
            .or_insert_with(|| ae::BossEncounterState::new(spec));
    }

    pub fn get(&self, id: &str) -> Option<&ae::BossEncounterState> {
        self.encounters.get(id)
    }

    pub fn link_runtime(&mut self, encounter_id: &str, runtime_id: &str) {
        self.runtime_ids
            .insert(encounter_id.to_string(), runtime_id.to_string());
    }

    pub fn active_phase(&self) -> Option<(&str, ae::BossEncounterPhase)> {
        for (id, state) in &self.encounters {
            if !matches!(state.phase, ae::BossEncounterPhase::Dormant) {
                return Some((id.as_str(), state.phase));
            }
        }
        None
    }
}

/// Default boss specs shipped with the sandbox. Populated lazily so
/// hot reloads of LDtk content don't double-register.
pub fn default_boss_specs() -> Vec<ae::BossEncounterSpec> {
    vec![
        ae::BossEncounterSpec::gradient_sentinel(),
        ae::BossEncounterSpec::mockingbird(),
    ]
}

/// Sanitize an authored boss `name` into a stable encounter id. Lowercases,
/// strips non-alphanumeric characters, replaces spaces with underscores.
/// `"Clockwork Warden"` → `"clockwork_warden"`.
pub fn encounter_id_from_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_was_underscore = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_was_underscore = false;
        } else if !prev_was_underscore && !out.is_empty() {
            out.push('_');
            prev_was_underscore = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "boss".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encounter_id_from_name_normalizes_capitalization_and_spaces() {
        assert_eq!(
            encounter_id_from_name("Clockwork Warden"),
            "clockwork_warden"
        );
        assert_eq!(
            encounter_id_from_name("Gradient Sentinel"),
            "gradient_sentinel"
        );
        assert_eq!(
            encounter_id_from_name("BOSS-of-the-Year!"),
            "boss_of_the_year"
        );
        assert_eq!(encounter_id_from_name("   "), "boss");
    }

    #[test]
    fn encounter_id_from_name_handles_empty_input() {
        assert_eq!(encounter_id_from_name(""), "boss");
    }

    #[test]
    fn encounter_id_from_name_collapses_consecutive_separators() {
        // Multiple spaces, multiple punctuation runs collapse to single
        // underscore, matching the per-char sanitizer's invariant.
        assert_eq!(encounter_id_from_name("a   b"), "a_b");
        assert_eq!(encounter_id_from_name("a---b"), "a_b");
        assert_eq!(encounter_id_from_name("a -+= b"), "a_b");
    }

    #[test]
    fn encounter_id_from_name_strips_trailing_underscores() {
        assert_eq!(encounter_id_from_name("Boss!"), "boss");
        assert_eq!(encounter_id_from_name("Boss   "), "boss");
        assert_eq!(encounter_id_from_name("Boss--"), "boss");
        assert_eq!(encounter_id_from_name("Boss_"), "boss");
    }

    #[test]
    fn encounter_id_from_name_preserves_alphanumeric_runs() {
        // Numbers stay as-is; lowercase preserved; mid-word digits OK.
        assert_eq!(encounter_id_from_name("R2D2"), "r2d2");
        assert_eq!(encounter_id_from_name("phase4-monster"), "phase4_monster");
    }

    /// World with a single solid floor sitting well below the
    /// mockingbird's spawn anchor (which the tests place at the
    /// world origin). The floor spans the entire world's x-extent so
    /// chests dropped at any x land on it — the fast-settle path in
    /// `sync_mockingbird_treasure_chest` needs an actual collision
    /// surface to terminate against.
    fn empty_world() -> ae::World {
        ae::World::new(
            "test_arena",
            ae::Vec2::new(4000.0, 4000.0),
            ae::Vec2::new(20.0, 20.0),
            vec![ae::Block::solid(
                "floor",
                // `Block::solid(name, min, size)` — top-left corner +
                // size, NOT center + half-size. Span covers x in
                // [-2000, 2000] so any test spawn anchor lands on it.
                ae::Vec2::new(-2000.0, 1000.0),
                ae::Vec2::new(4000.0, 40.0),
            )],
        )
    }

    fn empty_features() -> crate::features::FeatureRuntime {
        crate::features::FeatureRuntime {
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
        }
    }

    /// Build a minimal `FeatureRuntime` carrying one mockingbird
    /// `BossRuntime` at `spawn`. We construct the boss with a struct
    /// literal because `BossRuntime::new` is `pub(super)` to the
    /// features module — fine here because the type's fields are
    /// public, and the sync function only reads `id` and `spawn`.
    fn mockingbird_features_with_boss(spawn: ae::Vec2) -> crate::features::FeatureRuntime {
        let mut features = empty_features();
        features.bosses.push(crate::features::BossRuntime {
            id: "MockingbirdSpawn-0".to_string(),
            name: "mockingbird".to_string(),
            pos: spawn,
            spawn,
            size: ae::Vec2::new(48.0, 48.0),
            health: ae::Health::new(30),
            brain: ae::BossBrain::Custom("mockingbird".into()),
            alive: false,
            pattern_timer: 0.0,
            movement_timer: 0.0,
            attack_windup_timer: 0.0,
            attack_timer: 0.0,
            attack_cooldown: 0.0,
            hit_flash: 0.0,
        });
        features
    }

    #[test]
    fn sync_mockingbird_treasure_chest_drops_chest_when_save_marks_cleared() {
        let spawn = ae::Vec2::new(120.0, 80.0);
        let mut features = mockingbird_features_with_boss(spawn);
        let mut registry = BossEncounterRegistry::default();
        registry.ensure(ae::BossEncounterSpec::mockingbird());
        registry.link_runtime(MOCKINGBIRD_ENCOUNTER_ID, "MockingbirdSpawn-0");
        let mut save = ae::SandboxSaveData::default();
        save.set_boss(
            MOCKINGBIRD_ENCOUNTER_ID,
            ae::PersistedEncounterState::Cleared,
        );

        sync_mockingbird_treasure_chest(&mut features, &save, &registry, &empty_world());

        let chest = features
            .chests
            .iter()
            .find(|c| c.id == "encounter_chest_mockingbird")
            .expect("treasure chest must spawn once the bird is dead");
        assert!(
            !chest.opened,
            "fresh-kill chest starts closed (looted flag isn't set yet)",
        );
        assert_eq!(
            chest.pos.x, spawn.x,
            "chest aligns horizontally with the boss anchor",
        );
        assert!(
            chest.pos.y > spawn.y,
            "chest sits below the (airborne) boss spawn anchor",
        );
    }

    #[test]
    fn sync_mockingbird_treasure_chest_is_a_no_op_before_kill() {
        let mut features = mockingbird_features_with_boss(ae::Vec2::new(0.0, 0.0));
        let mut registry = BossEncounterRegistry::default();
        registry.ensure(ae::BossEncounterSpec::mockingbird());
        registry.link_runtime(MOCKINGBIRD_ENCOUNTER_ID, "MockingbirdSpawn-0");
        // No `set_boss` call → save default is `NotStarted`.
        let save = ae::SandboxSaveData::default();

        sync_mockingbird_treasure_chest(&mut features, &save, &registry, &empty_world());

        assert!(
            features.chests.is_empty(),
            "no chest before the boss is cleared",
        );
    }

    /// On a fresh kill (no looted flag), the chest spawns mid-air
    /// near the boss anchor with `falling = true` so the live update
    /// loop can play out a visible drop animation. Pinning this
    /// behavior protects the "kill the bird → watch the chest fall"
    /// UX beat from a future refactor that accidentally fast-settles
    /// the first drop.
    #[test]
    fn sync_mockingbird_treasure_chest_starts_falling_on_first_kill() {
        let mut features = mockingbird_features_with_boss(ae::Vec2::new(0.0, 0.0));
        let mut registry = BossEncounterRegistry::default();
        registry.ensure(ae::BossEncounterSpec::mockingbird());
        registry.link_runtime(MOCKINGBIRD_ENCOUNTER_ID, "MockingbirdSpawn-0");
        let mut save = ae::SandboxSaveData::default();
        save.set_boss(
            MOCKINGBIRD_ENCOUNTER_ID,
            ae::PersistedEncounterState::Cleared,
        );
        // Looted flag is NOT set — this is the fresh-kill path.

        sync_mockingbird_treasure_chest(&mut features, &save, &registry, &empty_world());

        let chest = features
            .chests
            .iter()
            .find(|c| c.id == "encounter_chest_mockingbird")
            .expect("chest must spawn on fresh kill");
        assert!(
            chest.falling,
            "first-kill chest must start falling so the player sees the drop animation"
        );
    }

    /// On reload, when the chest was looted in a prior session, the
    /// player expects the chest to be sitting right where they left
    /// it — *not* dropping in again from above. The sync function
    /// fast-settles the chest in-line so by the time the renderer
    /// reads the position, the chest is already on the floor with
    /// `falling = false`.
    #[test]
    fn sync_mockingbird_treasure_chest_fast_settles_on_reload_when_looted() {
        let mut features = mockingbird_features_with_boss(ae::Vec2::new(0.0, 0.0));
        let mut registry = BossEncounterRegistry::default();
        registry.ensure(ae::BossEncounterSpec::mockingbird());
        registry.link_runtime(MOCKINGBIRD_ENCOUNTER_ID, "MockingbirdSpawn-0");
        let mut save = ae::SandboxSaveData::default();
        save.set_boss(
            MOCKINGBIRD_ENCOUNTER_ID,
            ae::PersistedEncounterState::Cleared,
        );
        save.set_flag(
            crate::encounter::encounter_reward_looted_flag(MOCKINGBIRD_ENCOUNTER_ID),
            true,
        );

        sync_mockingbird_treasure_chest(&mut features, &save, &registry, &empty_world());

        let chest = features
            .chests
            .iter()
            .find(|c| c.id == "encounter_chest_mockingbird")
            .expect("chest must spawn on reload even after looting");
        assert!(
            !chest.falling,
            "looted chest must be settled on reload, not falling again"
        );
        assert!(
            chest.opened,
            "looted flag still mirrors onto the freshly-settled chest"
        );
    }

    #[test]
    fn sync_mockingbird_treasure_chest_mirrors_looted_flag_on_reload() {
        let mut features = mockingbird_features_with_boss(ae::Vec2::new(0.0, 0.0));
        let mut registry = BossEncounterRegistry::default();
        registry.ensure(ae::BossEncounterSpec::mockingbird());
        registry.link_runtime(MOCKINGBIRD_ENCOUNTER_ID, "MockingbirdSpawn-0");
        let mut save = ae::SandboxSaveData::default();
        save.set_boss(
            MOCKINGBIRD_ENCOUNTER_ID,
            ae::PersistedEncounterState::Cleared,
        );
        save.set_flag(
            crate::encounter::encounter_reward_looted_flag(MOCKINGBIRD_ENCOUNTER_ID),
            true,
        );

        sync_mockingbird_treasure_chest(&mut features, &save, &registry, &empty_world());

        let chest = features
            .chests
            .iter()
            .find(|c| c.id == "encounter_chest_mockingbird")
            .expect("re-spawned chest must surface even after looting");
        assert!(
            chest.opened,
            "looted flag must mark the re-spawned chest as already opened",
        );
    }

    #[test]
    fn encounter_id_from_name_drops_non_ascii() {
        // Non-alphanumeric Unicode is treated as a separator (matches
        // the `is_ascii_alphanumeric` predicate). Future i18n work can
        // relax this if needed.
        assert_eq!(encounter_id_from_name("日本語 Boss"), "boss");
        assert_eq!(encounter_id_from_name("Ω-omega"), "omega");
    }
}

pub fn populate_boss_encounter_registry(
    mut registry: ResMut<BossEncounterRegistry>,
    save: Res<crate::save::SandboxSave>,
) {
    if registry.specs_loaded {
        return;
    }
    for spec in default_boss_specs() {
        registry.ensure(spec);
    }
    let save_data = save.data();
    for (id, state) in registry.encounters.iter_mut() {
        let persisted = save_data.boss(id);
        if matches!(persisted, ae::PersistedEncounterState::Cleared) {
            // Already-defeated bosses skip straight to Death so the
            // arena renders empty next time the player walks in.
            // `phase = Dormant`, `hp = 0` is the cleanest carry-over
            // shape — the boss runtime won't spawn into the arena
            // and the encounter machinery stays silent.
            state.hp = 0;
        }
    }
    registry.specs_loaded = true;
}

/// Tick all live boss encounters. The single resource read keeps the
/// system param count low so this can be called as a regular Bevy
/// system without splitting.
pub fn update_boss_encounters(
    time: Res<Time>,
    mut registry: ResMut<BossEncounterRegistry>,
    mut runtime: ResMut<crate::SandboxRuntime>,
    mut save: ResMut<crate::save::SandboxSave>,
    mut music_request: ResMut<crate::encounter::EncounterMusicRequest>,
    mut quests: ResMut<QuestRegistry>,
    mut cutscene_queue: ResMut<CutsceneTriggerQueue>,
    room_set: Res<crate::rooms::RoomSet>,
    world: Res<crate::GameWorld>,
) {
    let dt = time.delta_secs();
    let _active_room = room_set.active_spec().id.clone();

    // Build a list of boss runtime ids alive in the current room so we
    // can wake up encounters when the player walks in.
    let bosses_in_room: Vec<(String, String, ae::Vec2, i32, i32)> = runtime
        .features
        .bosses
        .iter()
        .map(|b| {
            (
                b.id.clone(),
                b.name.clone(),
                b.pos,
                b.health.current,
                b.health.max,
            )
        })
        .collect();

    // Lazy registration: derive a *semantic* encounter id from the
    // boss's authored `name` (e.g. "clockwork warden" →
    // "clockwork_warden"). The LDtk iid (`BossSpawn-0158`) lives on
    // as the runtime_id link so combat damage still reaches the
    // right `BossRuntime`. Authored specs (registered before this
    // system runs) take precedence; only bosses without a spec fall
    // through to the auto-registered defaults.
    for (boss_runtime_id, boss_name, _pos, _hp, max_hp) in &bosses_in_room {
        let encounter_id = encounter_id_from_name(boss_name);
        registry.link_runtime(&encounter_id, boss_runtime_id);
        if registry.encounters.contains_key(&encounter_id) {
            continue;
        }
        let mut spec = ae::BossEncounterSpec::gradient_sentinel();
        spec.id = encounter_id.clone();
        spec.name = boss_name.to_string();
        // Pick up the runtime's authored max_hp so the encounter
        // doesn't replace it on first link.
        spec.max_hp = (*max_hp).max(1);
        registry.ensure(spec);
    }

    // Wake up an encounter whose boss is now visible in the room.
    for (_runtime_id, boss_name, _pos, _hp, _max) in &bosses_in_room {
        let encounter_id = encounter_id_from_name(boss_name);
        if let Some(state) = registry.encounters.get_mut(&encounter_id) {
            if matches!(state.phase, ae::BossEncounterPhase::Dormant) && state.hp > 0 {
                let evs = state.enter_intro();
                publish_events(
                    &encounter_id,
                    &evs,
                    &mut music_request,
                    &mut cutscene_queue,
                    &mut runtime.features,
                );
            }
        }
    }

    // Tick all in-flight encounters. Unrolled because we need to
    // mutate the runtime with the boss reference based on each
    // encounter's HP, and the borrow checker prefers a copy-out then
    // route style.
    let mut deferred_events: Vec<(String, Vec<ae::BossEncounterEvent>)> = Vec::new();
    for (id, state) in registry.encounters.iter_mut() {
        if matches!(state.phase, ae::BossEncounterPhase::Dormant) {
            continue;
        }
        let evs = state.tick(dt);
        if !evs.is_empty() {
            deferred_events.push((id.clone(), evs));
        }
    }
    for (id, evs) in deferred_events {
        publish_events(
            &id,
            &evs,
            &mut music_request,
            &mut cutscene_queue,
            &mut runtime.features,
        );
    }

    // Damage routing: when the sandbox `BossRuntime.health` decreases,
    // mirror the delta into the engine state and feed it back. The
    // BossRuntime is still the source of truth for HP because
    // existing combat/feature systems already mutate it; the engine
    // state is the *progression machine* fed by the damage delta.
    let runtime_id_lookup: BTreeMap<String, String> = registry.runtime_ids.clone();
    for (id, state) in registry.encounters.iter_mut() {
        let runtime_id = runtime_id_lookup
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.clone());
        let Some(boss) = runtime
            .features
            .bosses
            .iter_mut()
            .find(|b| b.id == runtime_id)
        else {
            continue;
        };
        // Sync max_hp on first link (the BossRuntime defaults to 18,
        // the engine spec might say more). The engine spec wins
        // because it carries the design intent.
        if boss.health.max != state.spec.max_hp.max(1) {
            boss.health = ae::Health::new(state.spec.max_hp.max(1));
        }
        // Mirror engine HP into the runtime so combat reads a
        // single number.
        if boss.health.current != state.hp && state.hp > 0 {
            boss.health.current = state.hp;
        }
        // Suppress runtime-side death animation while boss is in an
        // invulnerable phase (Intro/Transition/Stagger). We use a
        // hack: writing damage 0 just feeds the tick. Real damage
        // routing happens via `on_boss_damaged` below from the
        // `apply_player_attack` site.
        if state.phase.boss_invulnerable() && boss.alive {
            // Reset hit flash so the arena reads "neutral" during
            // the locked beats — small but readable presentation
            // smoothing.
            boss.hit_flash = 0.0;
        }
        // Death resolution: when engine state reports Death and the
        // outro is over, mark the runtime dead and update the save.
        if matches!(state.phase, ae::BossEncounterPhase::Death) && state.death_complete() {
            if boss.alive {
                boss.alive = false;
            }
            let prior = save.data().boss(id);
            if !matches!(prior, ae::PersistedEncounterState::Cleared) {
                save.data_mut()
                    .set_boss(id, ae::PersistedEncounterState::Cleared);
                // Push a quest advance event so any quest watching
                // this boss can progress.
                quests.push_event(ae::QuestAdvanceEvent::BossDefeated(id.clone()));
            }
        }
    }

    // While any encounter is in flight, the encounter music request
    // takes precedence over the legacy mob-encounter request. We
    // write the boss's per-phase track as `desired_track`; if both a
    // mob encounter AND a boss are active (shouldn't happen at the
    // same time, but guard) the boss wins because the boss path runs
    // after `update_encounters_from_world`.
    if let Some((_id, phase)) = registry.active_phase() {
        let _ = phase; // Already published as MusicRequested events.
        let _ = music_request; // Already mutated in `publish_events`.
    }

    sync_mockingbird_treasure_chest(&mut runtime.features, save.data(), &registry, &world.0);
}

/// Idempotent sync: when the mockingbird encounter is `Cleared`, make
/// sure the pirate-hoard chest exists in the live arena. Runs each
/// frame from `update_boss_encounters`.
///
/// GENERALIZATION PLAN (see `MOCKINGBIRD_ENCOUNTER_ID` for the broader
/// picture): the body here is not actually mockingbird-specific —
/// "find a boss by encounter id, and if its save state is `Cleared`,
/// drop a chest at its spawn anchor + offset" works for any boss
/// fight. Generalize by:
///   1. Introducing a `BossDeathReward` table keyed by encounter id.
///   2. Replacing the single early-return on `MOCKINGBIRD_ENCOUNTER_ID`
///      with a loop over the table.
///   3. Letting the per-entry `PickupKind` and offset feed in from
///      that table instead of constants.
/// We're keeping the special-case while there's exactly one entry —
/// adding the table for a single user is premature abstraction.
///
/// In its current form the sync ensures the chest:
///   - drops the moment the boss dies (encounter flips to `Cleared`),
///   - re-appears on room re-entry (FeatureRuntime is rebuilt empty by
///     `from_world`, so without this sync the chest would vanish on
///     reload),
///   - mirrors the persisted "looted" flag onto `chest.opened` so a
///     re-spawned chest reads as already-opened.
///
/// Cheap when the encounter isn't cleared, when there is no live
/// mockingbird in the room, or when the chest already exists
/// (`spawn_chest` short-circuits on duplicate id).
pub fn sync_mockingbird_treasure_chest(
    features: &mut crate::features::FeatureRuntime,
    save: &ae::SandboxSaveData,
    registry: &BossEncounterRegistry,
    world: &ae::World,
) {
    // Source of truth is the save: the engine's phase stays at
    // `Death` post-defeat, while the save bumps the boss to `Cleared`
    // once the death animation completes. The save reading also
    // survives room reloads where the encounter state machine is
    // freshly built each time.
    if !matches!(
        save.boss(MOCKINGBIRD_ENCOUNTER_ID),
        ae::PersistedEncounterState::Cleared
    ) {
        return;
    }
    // Pull the on-floor chest position from the boss's authored spawn
    // anchor. We use `spawn` (not `pos`) so the chest lands at the
    // same place on first kill and on every subsequent room reload —
    // the BossRuntime is rebuilt at its LDtk-authored spawn each time.
    let runtime_id = registry
        .runtime_ids
        .get(MOCKINGBIRD_ENCOUNTER_ID)
        .cloned()
        .unwrap_or_else(|| MOCKINGBIRD_ENCOUNTER_ID.to_string());
    let Some(boss) = features.bosses.iter().find(|b| b.id == runtime_id) else {
        return;
    };
    let chest_pos = ae::Vec2::new(boss.spawn.x, boss.spawn.y + MOCKINGBIRD_CHEST_DROP_OFFSET_Y);
    let chest_id = format!("encounter_chest_{MOCKINGBIRD_ENCOUNTER_ID}");
    // Detect first-spawn so we can kick off the falling-chest physics
    // *only* when the chest is genuinely new. `spawn_chest` is
    // idempotent on id, so once the chest is in the runtime, we leave
    // its falling/settled state alone — otherwise the per-frame sync
    // would teleport it back up every tick.
    let just_spawned = features.chests.iter().all(|c| c.id != chest_id);
    features.spawn_chest(
        chest_id.clone(),
        // The "real" payout is the admiral's quest reward; this
        // chest is mostly diegetic, so a small heal is fine. The
        // narrative reward (data chips, batteries, potions) lands
        // in the inventory when the player returns to the admiral
        // — see `quest::grant_pirate_treasure_reward`.
        Some(ae::PickupKind::Custom("pirate_hoard".to_string())),
        chest_pos,
        MOCKINGBIRD_CHEST_SIZE,
    );
    // Mirror the persisted "looted" flag onto the live chest so a
    // save+reload after looting doesn't show the chest as closed.
    let looted = save.flag(&crate::encounter::encounter_reward_looted_flag(
        MOCKINGBIRD_ENCOUNTER_ID,
    ));
    if let Some(chest) = features.chests.iter_mut().find(|c| c.id == chest_id) {
        chest.opened = looted;
        if just_spawned {
            // Born above the floor — let gravity do the rest. After
            // it lands, `chest.falling` clears and the chest behaves
            // like any other static interactable.
            chest.falling = true;
            chest.vel_y = 0.0;
            // If the player has already looted this chest in a prior
            // session, the dramatic drop animation is no longer
            // appropriate — they expect the chest to be sitting open
            // where they left it. Run the same physics tick the live
            // update loop runs, but in a tight in-line loop, so the
            // chest reaches its resting position *this frame* before
            // the renderer ever sees it. Capped iteration count is a
            // safety net against pathological geometry (e.g. a chest
            // spawn over an open pit) — at ~16 ms per virtual tick,
            // 240 iterations covers 3.84 s of fall before bailing.
            if looted {
                let virtual_dt = 1.0 / 60.0;
                for _ in 0..240 {
                    if !chest.falling {
                        break;
                    }
                    crate::features::tick_chest_fall(chest, world, virtual_dt);
                }
            }
        }
    }
}

fn publish_events(
    encounter_id: &str,
    events: &[ae::BossEncounterEvent],
    music_request: &mut crate::encounter::EncounterMusicRequest,
    cutscene_queue: &mut CutsceneTriggerQueue,
    features: &mut crate::features::FeatureRuntime,
) {
    for event in events {
        match event {
            ae::BossEncounterEvent::PhaseChanged { to, .. } => {
                if matches!(to, ae::BossEncounterPhase::Intro) {
                    cutscene_queue.request(format!("boss_intro_{encounter_id}"));
                }
                features.banner = match to {
                    ae::BossEncounterPhase::Intro => format!("BOSS APPROACHES — {encounter_id}"),
                    ae::BossEncounterPhase::Phase1 => "PHASE 1".to_string(),
                    ae::BossEncounterPhase::Transition => "...".to_string(),
                    ae::BossEncounterPhase::Phase2 => "PHASE 2".to_string(),
                    ae::BossEncounterPhase::Stagger => "STAGGERED — punish".to_string(),
                    ae::BossEncounterPhase::Enrage => "ENRAGED".to_string(),
                    ae::BossEncounterPhase::Death => "DEFEATED".to_string(),
                    ae::BossEncounterPhase::Dormant => String::new(),
                };
                features.banner_timer = 1.4;
            }
            ae::BossEncounterEvent::MusicRequested { track } => {
                if !track.is_empty() {
                    music_request.desired_track = Some(track.clone());
                }
            }
            ae::BossEncounterEvent::DamageApplied { .. } => {}
            ae::BossEncounterEvent::Defeated => {
                // Death cutscene swap could go here in a richer build.
                features.banner = format!("VICTORY: {encounter_id}");
                features.banner_timer = 2.5;
            }
        }
    }
}

/// Helper: feed a damage delta into the encounter machine. Called by
/// `apply_player_attack` after damage hits the BossRuntime.
pub fn record_boss_damage(
    registry: &mut BossEncounterRegistry,
    music_request: &mut crate::encounter::EncounterMusicRequest,
    cutscene_queue: &mut CutsceneTriggerQueue,
    features: &mut crate::features::FeatureRuntime,
    boss_runtime_id: &str,
    damage: i32,
) {
    let Some((id, _)) = registry
        .runtime_ids
        .iter()
        .find(|(_id, runtime_id)| runtime_id.as_str() == boss_runtime_id)
        .map(|(id, runtime_id)| (id.clone(), runtime_id.clone()))
    else {
        return;
    };
    let Some(state) = registry.encounters.get_mut(&id) else {
        return;
    };
    let evs = state.apply_player_damage(damage);
    publish_events(&id, &evs, music_request, cutscene_queue, features);
}
