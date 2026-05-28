//! Yarn command + function + markup registrations — the "vocabulary"
//! that authored `.yarn` content can invoke at runtime.
//!
//! The bindings split into three concerns:
//!
//! **Commands** (`<<set_flag X>>` syntax). Bevy systems with
//! `In<T>` parameters. Registered on the runner's `commands_mut()`
//! via `world.register_system(...)`. Each one writes to a typed
//! game-state channel (`GameplayEffect::SetFlag`, `SfxMessage::Play`,
//! …). Authored dialogue uses them to *drive* gameplay.
//!
//! **Functions** (`<<if boss_cleared("X")>>` syntax). Pure functions
//! registered on the runner's `library_mut()`. Functions can't be
//! Bevy systems — they're called synchronously from the runtime
//! interpreter — so they read save state through a shared
//! [`YarnStateMirror`] refreshed each frame by
//! [`refresh_yarn_state_mirror`]. Authored dialogue uses them to
//! *read* gameplay.
//!
//! **Markup cues** (`Speaker: [shout]LINE[/shout]` inline). The
//! bridge's `on_present_line` observer scans `LocalizedLine.attributes`
//! and writes [`YarnPresentationCue`] resource entries that the
//! camera and audio layers consume. Authored dialogue uses these to
//! *spice* the presentation.
//!
//! ## Why a single module
//!
//! Per the migration design, the "what verbs / functions /
//! markup can authored dialogue invoke" surface lives here as a
//! single source of truth. Couples to `SandboxSave`, `SfxMessage`,
//! `GameplayEffect`, etc. — that's the bridge's whole job.

use std::sync::{Arc, RwLock};

use ambition_engine as ae;
use bevy::prelude::*;
use bevy_yarnspinner::prelude::DialogueRunner;

use crate::features::GameplayEffect;
use crate::persistence::save::SandboxSave;

// ===== Shared state mirror =====================================

/// Snapshot of save data the Yarn `library` functions read from.
/// Refreshed each frame from `SandboxSave` by
/// [`refresh_yarn_state_mirror`]. Wrapped in `Arc<RwLock<...>>` so
/// the closures registered on the runner's `Library` (which capture
/// by move) can read it without taking a Bevy resource.
///
/// Yarn `library` functions are synchronous pure functions — they
/// can't take a `Res<SandboxSave>` like a Bevy system can. The
/// mirror shape solves that: a refresh system updates the snapshot
/// inside the lock once per frame, and function closures lock-and-
/// read on every Yarn `<<if>>` evaluation.
#[derive(Default, Clone, Debug)]
pub struct YarnStateMirrorData {
    /// flag id → on/off.
    pub flags: std::collections::HashMap<String, bool>,
    /// canonical boss encounter ids in `Cleared` state.
    pub bosses_cleared: std::collections::HashSet<String>,
    /// canonical quest ids whose state is `InProgress`.
    pub quests_active: std::collections::HashSet<String>,
    /// dialogue id → visit count.
    pub visit_counts: std::collections::HashMap<String, u32>,
}

#[derive(Resource, Default, Clone)]
pub struct YarnStateMirror(pub Arc<RwLock<YarnStateMirrorData>>);

/// Per-frame refresh: copy the relevant slices of [`SandboxSave`]
/// into the mirror so Yarn functions read consistent values for the
/// duration of a single tick. Runs unconditionally — cheap because
/// the data is small (flags/bosses/quests are short Vecs).
pub fn refresh_yarn_state_mirror(
    save: Option<Res<SandboxSave>>,
    mirror: Res<YarnStateMirror>,
) {
    let Some(save) = save else {
        return;
    };
    let data = save.data();
    let mut snap = mirror.0.write().expect("YarnStateMirror poisoned");
    snap.flags.clear();
    for flag in &data.flags {
        snap.flags.insert(flag.id.clone(), flag.on);
    }
    snap.bosses_cleared.clear();
    for boss in &data.bosses {
        if matches!(boss.state, crate::save::PersistedEncounterState::Cleared) {
            snap.bosses_cleared.insert(boss.id.clone());
        }
    }
    snap.quests_active.clear();
    for quest in &data.quests {
        if matches!(quest.state, crate::save::PersistedQuestState::InProgress) {
            snap.quests_active.insert(quest.id.clone());
        }
    }
    snap.visit_counts.clear();
    for visit in &data.dialog_visits {
        snap.visit_counts.insert(visit.id.clone(), visit.count);
    }
}

// ===== Markup cue ==============================================

/// Per-frame presentation cue surface populated by the bridge's
/// `on_present_line` observer whenever a Yarn line carries `[shout]`
/// or `[whisper]` markup. Camera shake / audio pitch consumers read
/// this in their normal Update systems; the cue clears each frame
/// via [`clear_yarn_presentation_cue`] before the bridge writes the
/// next one.
///
/// Phase 4 today: the cue resource exists + the bridge writes to
/// it. Wiring the camera shake and audio pitch CONSUMERS is left
/// as scaffolding — these are presentation hooks that need their
/// own consumer systems (small follow-up; the Yarn side is done).
#[derive(Resource, Default, Debug, Clone)]
pub struct YarnPresentationCue {
    /// True iff the most recent line carried `[shout]` markup.
    /// Consumers (camera shake, voice volume) trigger off the rising
    /// edge.
    pub shout: bool,
    /// True iff the most recent line carried `[whisper]` markup.
    /// Consumers (audio pitch / volume drop) trigger off the rising
    /// edge.
    pub whisper: bool,
}

/// Reset the markup cue once per frame. Runs before the bridge
/// observer fires (which writes the cue for THIS frame's line).
pub fn clear_yarn_presentation_cue(mut cue: ResMut<YarnPresentationCue>) {
    cue.shout = false;
    cue.whisper = false;
}

// ===== Plugin ===================================================

pub struct YarnBindingsPlugin;

impl Plugin for YarnBindingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<YarnStateMirror>();
        app.init_resource::<YarnPresentationCue>();
        app.add_systems(
            Update,
            (clear_yarn_presentation_cue, refresh_yarn_state_mirror).chain(),
        );
    }
}

// ===== Commands ================================================
//
// Bevy systems with `In<T>` parameters. The Yarn syntax
// `<<cmd_name arg1 arg2>>` invokes these via `world.register_system`
// at runner-build time. Each takes ownership of its args and writes
// to a typed message channel.

/// `<<set_flag "id">>` — flip a save flag to `true`. Routes through
/// `GameplayEffect::SetFlag` so existing consumers (quest advance
/// listeners, save mirror) see the change.
pub fn cmd_set_flag(In(name): In<String>, mut effects: MessageWriter<GameplayEffect>) {
    effects.write(GameplayEffect::SetFlag { id: name, on: true });
}

/// `<<clear_flag "id">>` — flip a save flag to `false`.
pub fn cmd_clear_flag(In(name): In<String>, mut effects: MessageWriter<GameplayEffect>) {
    effects.write(GameplayEffect::SetFlag { id: name, on: false });
}

/// `<<give_item "kind" count>>` — grant the player an item.
/// Logged-stub today; the inventory system doesn't have a "give
/// item" Bevy channel yet. The command exists so authored dialogue
/// can use it; when the inventory consumer lands, wire this to
/// write the matching message.
pub fn cmd_give_item(In((kind, count)): In<(String, f32)>) {
    info!(
        target: "ambition_sandbox::dialog::yarn",
        "give_item: kind={} count={} (stub; inventory consumer pending)",
        kind,
        count as i32,
    );
}

/// `<<spawn_chest "id">>` — spawn a reward chest by id. Logged-stub;
/// the chest spawn path is currently driven by room+encounter spec
/// data, not by dialogue. Wire when needed.
pub fn cmd_spawn_chest(In(id): In<String>) {
    info!(
        target: "ambition_sandbox::dialog::yarn",
        "spawn_chest: id={id} (stub; chest spawn consumer pending)",
    );
}

/// `<<play_sfx "id">>` — emit an `SfxMessage::Play`. The id is a
/// string that `SfxId::new` hashes at the call site (matches every
/// other dynamic-id audio path in the codebase).
pub fn cmd_play_sfx(
    In(id_str): In<String>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::SfxId::new(&id_str),
        pos: ae::Vec2::ZERO,
    });
}

/// `<<camera_zoom factor>>` — adjust camera zoom. Logged-stub; the
/// camera-zoom system currently reads its zoom from the active
/// encounter spec. Wire when a dialogue-driven zoom override
/// resource lands.
pub fn cmd_camera_zoom(In(factor): In<f32>) {
    info!(
        target: "ambition_sandbox::dialog::yarn",
        "camera_zoom: factor={factor:.2} (stub; cinematic zoom consumer pending)",
    );
}

// ===== Functions ================================================
//
// Pure functions registered on the runner's `library_mut()`. Each
// captures `Arc<RwLock<YarnStateMirrorData>>` by clone so it can
// read save state on every `<<if>>` evaluation without touching
// Bevy resources.

/// Build closures around the shared mirror and register all five
/// custom functions on the runner's library. Called from
/// `spawn_dialogue_runner` after the runner is built but before it
/// is spawned, so the functions are baked in.
pub fn register_functions(runner: &mut DialogueRunner, mirror: &YarnStateMirror) {
    let lib = runner.library_mut();
    // boss_cleared(id) -> bool: is the named boss encounter in
    // Cleared state?
    let m = Arc::clone(&mirror.0);
    lib.add_function("boss_cleared", move |id: String| -> bool {
        m.read()
            .map(|snap| snap.bosses_cleared.contains(&id))
            .unwrap_or(false)
    });
    // flag(id) -> bool: read a save flag.
    let m = Arc::clone(&mirror.0);
    lib.add_function("flag", move |id: String| -> bool {
        m.read()
            .map(|snap| snap.flags.get(&id).copied().unwrap_or(false))
            .unwrap_or(false)
    });
    // visit_count(id) -> f32: how many times the named dialogue
    // node has been entered. Returns f32 because Yarn arithmetic
    // is f32-typed (`<<if visit_count("oiler") == 1>>` etc.).
    let m = Arc::clone(&mirror.0);
    lib.add_function("visit_count", move |id: String| -> f32 {
        m.read()
            .map(|snap| snap.visit_counts.get(&id).copied().unwrap_or(0) as f32)
            .unwrap_or(0.0)
    });
    // quest_active(id) -> bool: is the named quest InProgress?
    let m = Arc::clone(&mirror.0);
    lib.add_function("quest_active", move |id: String| -> bool {
        m.read()
            .map(|snap| snap.quests_active.contains(&id))
            .unwrap_or(false)
    });
    // inventory_has(item) -> bool: stub; the inventory system
    // doesn't have a query surface yet. Always returns false so
    // authored dialogue can reference it without parse errors.
    lib.add_function("inventory_has", |_item: String| -> bool { false });
}

/// Register all six custom commands on the runner. Called from
/// `spawn_dialogue_runner`. Each command name maps to a Bevy
/// system registered against the `World`.
pub fn register_commands(commands: &mut Commands, runner: &mut DialogueRunner) {
    let set_flag_id = commands.register_system(cmd_set_flag);
    let clear_flag_id = commands.register_system(cmd_clear_flag);
    let give_item_id = commands.register_system(cmd_give_item);
    let spawn_chest_id = commands.register_system(cmd_spawn_chest);
    let play_sfx_id = commands.register_system(cmd_play_sfx);
    let camera_zoom_id = commands.register_system(cmd_camera_zoom);
    let cmds = runner.commands_mut();
    cmds.add_command("set_flag", set_flag_id);
    cmds.add_command("clear_flag", clear_flag_id);
    cmds.add_command("give_item", give_item_id);
    cmds.add_command("spawn_chest", spawn_chest_id);
    cmds.add_command("play_sfx", play_sfx_id);
    cmds.add_command("camera_zoom", camera_zoom_id);
}
