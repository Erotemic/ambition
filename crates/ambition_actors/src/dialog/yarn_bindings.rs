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
//! and writes [`ambition_dialog::YarnPresentationCue`] resource entries that the
//! camera and audio layers consume. Authored dialogue uses these to
//! *spice* the presentation.
//!
//! ## Why a single module
//!
//! Per the migration design, the "what verbs / functions /
//! markup can authored dialogue invoke" surface lives here as a
//! single source of truth. Couples to `SandboxSave`, `SfxMessage`,
//! `GameplayEffect`, etc. — that's the bridge's whole job.

//! The generic binding machinery (the [`YarnStateMirror`] shape, the
//! [`ambition_dialog::YarnPresentationCue`], the [`ambition_dialog::YarnContentBindings`] installer seam, and
//! [`ambition_dialog::YarnBindingsPlugin`]) lives in the reusable `ambition_dialog` crate (E1c).
//! This module keeps only Ambition's game-specific vocabulary — the commands
//! and functions that touch actor/save state — and the per-frame refresh that
//! fills the mirror from `SandboxSave`. It registers on the runtime through the
//! installer seam via [`install_game_bindings`].

use std::sync::Arc;

use ambition_engine_core as ae;
use bevy::prelude::*;
use bevy_yarnspinner::prelude::DialogueRunner;

use crate::features::SetFlagRequested;
use ambition_persistence::save::SandboxSave;

use ambition_dialog::{YarnStateMirror, YarnStateMirrorData};

/// The host installer: registers Ambition's generic Yarn vocabulary
/// (commands + functions) on the runner. Pushed into
/// [`ambition_dialog::YarnContentBindings`] by [`crate::dialog::YarnBindingsPlugin`] so the
/// reusable bridge names no concrete game command.
pub fn install_game_bindings(
    commands: &mut Commands,
    runner: &mut DialogueRunner,
    mirror: &YarnStateMirror,
) {
    register_commands(commands, runner);
    register_functions(runner, mirror);
}

/// Per-frame refresh: copy the relevant slices of [`SandboxSave`]
/// into the mirror so Yarn functions read consistent values for the
/// duration of a single tick. Runs unconditionally — cheap because
/// the data is small (flags/bosses/quests are short Vecs).
pub fn refresh_yarn_state_mirror(
    save: Option<Res<SandboxSave>>,
    owned: Option<Res<crate::items::OwnedItems>>,
    wallet: Query<&ambition_characters::actor::BodyWallet, With<crate::actor::PrimaryPlayer>>,
    mirror: Res<YarnStateMirror>,
) {
    let mut snap = mirror.0.write().expect("YarnStateMirror poisoned");
    snap.wallet_balance = wallet.iter().next().map(|w| w.balance).unwrap_or(0);
    // Inventory is a live ECS resource, not part of `SandboxSave`;
    // refresh it independently (and before the save early-return) so
    // `inventory_has(...)` reflects pickups even in a save-less sandbox.
    snap.inventory_counts.clear();
    // Primary source: the 24-item catalog (superset). Insert each item under its
    // catalog dialog id, plus a legacy alias (e.g. "healthpotion" → HealthCell)
    // so older scripts keep resolving.
    if let Some(owned) = owned.as_deref() {
        for item in crate::items::Item::ALL {
            let count = owned.count(item);
            snap.inventory_counts
                .insert(item.dialog_id().to_string(), count);
            // Mirror under the legacy alias too (only "healthpotion" diverges)
            // so older scripts using the old id keep resolving.
            if let Some(alias) = item.legacy_dialog_alias() {
                snap.inventory_counts.insert(alias.to_string(), count);
            }
        }
    }
    let Some(save) = save else {
        return;
    };
    let data = save.data();
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

// ===== Commands ================================================
//
// Bevy systems with `In<T>` parameters. The Yarn syntax
// `<<cmd_name arg1 arg2>>` invokes these via `world.register_system`
// at runner-build time. Each takes ownership of its args and writes
// to a typed message channel.

/// `<<set_flag "id">>` — flip a save flag to `true`. Routes through
/// `GameplayEffect::SetFlag` so existing consumers (quest advance
/// listeners, save mirror) see the change.
pub fn cmd_set_flag(In(name): In<String>, mut effects: MessageWriter<SetFlagRequested>) {
    effects.write(SetFlagRequested { id: name, on: true });
}

/// `<<clear_flag "id">>` — flip a save flag to `false`.
pub fn cmd_clear_flag(In(name): In<String>, mut effects: MessageWriter<SetFlagRequested>) {
    effects.write(SetFlagRequested {
        id: name,
        on: false,
    });
}

/// `<<challenge>>` — provoke the NPC the player is currently talking to into
/// a fight. The generic dialogue-gated combat trigger: it emits an
/// [`ActorStimulus::Challenged`] for the conversation's speaker entity, which
/// `apply_actor_stimuli` turns into the same in-place peaceful→hostile flip a
/// strike would cause — but unconditionally, since picking "challenge" IS the
/// consent to fight. Any content (the Perfect Cell-ular Automaton and beyond)
/// arms a boss/duel by authoring this one command on a choice; no Rust per-NPC
/// branch. Logs and no-ops if there's no in-world speaker (scripted dialogue).
pub fn cmd_challenge(
    dialogue: Res<ambition_dialog::DialogState>,
    player: Query<Entity, With<crate::actor::PlayerEntity>>,
    mut commands: Commands,
) {
    let Some(actor) = dialogue.speaker_entity() else {
        warn!("<<challenge>>: no speaker entity in dialogue context; ignoring");
        return;
    };
    // ARM a deferred challenge rather than flipping hostile this instant: the
    // player is still in the dialog box (and likely overlapping the NPC) when
    // `<<challenge>>` fires. `tick_pending_challenges` emits the actual
    // `Challenged` stimulus only after the box closes + a grace period, so the
    // fight doesn't start point-blank-in-the-body. See [`PendingChallenge`].
    commands
        .entity(actor)
        .insert(crate::features::PendingChallenge {
            challenger: player.iter().next(),
            grace: crate::features::CHALLENGE_GRACE_S,
        });
}

/// `<<use_brain "preset">>` — switch the NPC the player is talking to onto an
/// explicit brain preset at runtime, changing its AUTONOMOUS behaviour (a
/// dialogue outcome like "fight me" pairs this with a disposition change). Routes
/// through the central [`ActorDirectiveRequest`] seam →
/// [`BrainCommand`](crate::features::BrainCommand), so the runtime switch is
/// deterministic and snapshot-safe; it never edits the `Brain` component directly.
/// No-ops (with a log) if the speaker has no stable id (scripted/anonymous
/// dialogue).
///
/// [`ActorDirectiveRequest`]: crate::features::ActorDirectiveRequest
pub fn cmd_use_brain(
    In(preset): In<String>,
    dialogue: Res<ambition_dialog::DialogState>,
    sim_ids: Query<&ambition_platformer_primitives::sim_id::SimId>,
    mut directives: MessageWriter<crate::features::ActorDirectiveRequest>,
) {
    let Some(actor) = dialogue.speaker_entity() else {
        warn!("<<use_brain>>: no speaker entity in dialogue context; ignoring");
        return;
    };
    let Ok(sim_id) = sim_ids.get(actor) else {
        warn!("<<use_brain>>: speaker has no SimId; ignoring");
        return;
    };
    directives.write(crate::features::ActorDirectiveRequest {
        target: sim_id.clone(),
        directive: crate::features::ActorDirective::UseBrainPreset(
            ambition_characters::actor::character_catalog::BrainPresetId::new(preset),
        ),
    });
}

/// `<<restore_brain>>` — restore the NPC the player is talking to back to its
/// character-default brain (e.g. "you are free"). The runtime counterpart of the
/// spawn-time `CharacterDefault`. Routes through the same
/// [`ActorDirectiveRequest`](crate::features::ActorDirectiveRequest) seam.
pub fn cmd_restore_brain(
    dialogue: Res<ambition_dialog::DialogState>,
    sim_ids: Query<&ambition_platformer_primitives::sim_id::SimId>,
    mut directives: MessageWriter<crate::features::ActorDirectiveRequest>,
) {
    let Some(actor) = dialogue.speaker_entity() else {
        warn!("<<restore_brain>>: no speaker entity in dialogue context; ignoring");
        return;
    };
    let Ok(sim_id) = sim_ids.get(actor) else {
        warn!("<<restore_brain>>: speaker has no SimId; ignoring");
        return;
    };
    directives.write(crate::features::ActorDirectiveRequest {
        target: sim_id.clone(),
        directive: crate::features::ActorDirective::RestoreDefaultBrain,
    });
}

/// `<<give_item "kind" count>>` — grant the player an item by adding
/// to the live `OwnedItems` catalog resource. The kind string is
/// resolved through [`crate::items::Item::from_dialog_id`]
/// (loose spelling); an unknown kind or non-positive count is logged
/// and ignored.
pub fn cmd_give_item(
    In((kind, count)): In<(String, f32)>,
    mut owned: ResMut<crate::items::OwnedItems>,
) {
    let granted = apply_give_item(&mut owned, &kind, count);
    if granted == 0 {
        warn!(
            target: "ambition_actors::dialog::yarn",
            "give_item: ignored kind={kind:?} count={count} (unknown item or non-positive count)",
        );
    } else {
        info!(
            target: "ambition_actors::dialog::yarn",
            "give_item: granted {granted}x {kind:?}",
        );
    }
}

/// `<<buy_item "id" price>>` — spend `price` from the player's wallet and grant
/// one of the catalog item if affordable. A merchant dialogue node calls this on
/// a purchase choice; the affordability check lives in [`crate::shop::buy`].
pub fn cmd_buy_item(
    In((id, price)): In<(String, f32)>,
    mut owned: ResMut<crate::items::OwnedItems>,
    mut wallets: Query<
        &mut ambition_characters::actor::BodyWallet,
        With<crate::actor::PrimaryPlayer>,
    >,
) {
    let Some(item) = crate::items::Item::from_dialog_id(&id) else {
        warn!(target: "ambition_actors::dialog::yarn", "buy_item: unknown item {id:?}");
        return;
    };
    let Ok(mut wallet) = wallets.single_mut() else {
        return;
    };
    let outcome = crate::shop::buy(&mut wallet, &mut owned, item, price.max(0.0) as i32);
    info!(
        target: "ambition_actors::dialog::yarn",
        "buy_item: {id:?} @ {price} -> {outcome:?} (balance now {})", wallet.balance,
    );
}

/// `<<sell_item "id" price>>` — remove one of the catalog item and credit the
/// wallet if the player owns it. See [`crate::shop::sell`].
pub fn cmd_sell_item(
    In((id, price)): In<(String, f32)>,
    mut owned: ResMut<crate::items::OwnedItems>,
    mut wallets: Query<
        &mut ambition_characters::actor::BodyWallet,
        With<crate::actor::PrimaryPlayer>,
    >,
) {
    let Some(item) = crate::items::Item::from_dialog_id(&id) else {
        warn!(target: "ambition_actors::dialog::yarn", "sell_item: unknown item {id:?}");
        return;
    };
    let Ok(mut wallet) = wallets.single_mut() else {
        return;
    };
    let outcome = crate::shop::sell(&mut wallet, &mut owned, item, price.max(0.0) as i32);
    info!(
        target: "ambition_actors::dialog::yarn",
        "sell_item: {id:?} @ {price} -> {outcome:?} (balance now {})", wallet.balance,
    );
}

/// Pure core of [`cmd_give_item`]: add `count` (floored to a
/// non-negative integer) of the named item to the bag. Returns the
/// number actually granted (0 when the kind is unknown or the count
/// is non-positive) so the command can log and tests can assert
/// without a live `World`.
fn apply_give_item(owned: &mut crate::items::OwnedItems, kind: &str, count: f32) -> u32 {
    if count <= 0.0 {
        return 0;
    }
    let Some(item) = crate::items::Item::from_dialog_id(kind) else {
        return 0;
    };
    let n = count as u32;
    owned.grant(item, n);
    n
}

/// `<<spawn_chest "id">>` — spawn a reward chest by id. Logged-stub;
/// the chest spawn path is currently driven by room+encounter spec
/// data, not by dialogue. Wire when needed.
pub fn cmd_spawn_chest(In(id): In<String>) {
    info!(
        target: "ambition_actors::dialog::yarn",
        "spawn_chest: id={id} (stub; chest spawn consumer pending)",
    );
}

/// `<<play_sfx "id">>` — emit an `SfxMessage::Play`. The id is a
/// string that `SfxId::new` hashes at the call site (matches every
/// other dynamic-id audio path in the codebase).
pub fn cmd_play_sfx(In(id_str): In<String>, mut sfx: ambition_sfx::SfxWriter) {
    sfx.write(ambition_sfx::SfxMessage::Play {
        id: ambition_sfx::SfxId::new(&id_str),
        pos: ae::Vec2::ZERO,
    });
}

/// `<<spawn_fireworks>>` — spawn a short test sequence of reusable explosion
/// VFX/SFX near the player. Authored from the Kernel Guide dialog so designers
/// can verify the explosion pipeline without entering a boss room.
pub fn cmd_spawn_fireworks(
    mut fireworks: MessageWriter<ambition_vfx::vfx::FireworksRequest>,
    // SLOT-0 BY DESIGN: Yarn's `$player_x`/`$player_y` are authored against the
    // local player's position — dialogue is told to a human, not to a body.
    player_q: Query<&crate::actor::BodyKinematics, crate::actor::PrimaryPlayerOnly>,
) {
    let origin = player_q
        .single()
        .map(|kin| kin.pos + ae::Vec2::new(0.0, -40.0))
        .unwrap_or(ae::Vec2::new(480.0, 260.0));
    fireworks.write(ambition_vfx::vfx::FireworksRequest::around(origin));
}

/// `<<camera_zoom factor>>` — adjust camera zoom. Logged-stub; the
/// camera-zoom system currently reads its zoom from the active
/// encounter spec. Wire when a dialogue-driven zoom override
/// resource lands.
pub fn cmd_camera_zoom(In(factor): In<f32>) {
    info!(
        target: "ambition_actors::dialog::yarn",
        "camera_zoom: factor={factor:.2} (stub; cinematic zoom consumer pending)",
    );
}

// The cut-rope boss commands (`watch_cut_rope_video`,
// `reset_cut_rope_room`) and the `cut_rope_heavy_object_is` function
// moved to `ambition_content::bosses::yarn` (the content crate) — installed via
// [`ambition_dialog::YarnContentBindings`] so this generic module names no content.

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
    // inventory_has(item) -> bool: does the player currently hold at
    // least one of the named item? Reads the inventory slice the
    // refresh system mirrors from `OwnedItems`. The item argument
    // is normalized (lowercased, non-alphanumerics dropped) so
    // `"HealthPotion"`, `"health_potion"`, and `"health potion"` all
    // match the item's `dialog_id()`.
    let m = Arc::clone(&mirror.0);
    lib.add_function("inventory_has", move |item: String| -> bool {
        m.read()
            .map(|snap| mirror_inventory_has(&snap, &item))
            .unwrap_or(false)
    });
    // wallet_balance() -> number: the player's current money, so a merchant node
    // can show it ("You have {wallet_balance()}g").
    let m = Arc::clone(&mirror.0);
    lib.add_function("wallet_balance", move || -> f32 {
        m.read()
            .map(|snap| snap.wallet_balance as f32)
            .unwrap_or(0.0)
    });
    // can_afford(price) -> bool: gate a purchase choice on affordability.
    let m = Arc::clone(&mirror.0);
    lib.add_function("can_afford", move |price: f32| -> bool {
        m.read()
            .map(|snap| snap.wallet_balance >= price.max(0.0) as i32)
            .unwrap_or(false)
    });
}

/// Pure `inventory_has` lookup over a mirror snapshot: true iff the
/// player holds at least one of the named item. Split out from the
/// registered closure so it is unit-testable without a live
/// `DialogueRunner`.
fn mirror_inventory_has(data: &YarnStateMirrorData, item: &str) -> bool {
    let key = normalize_item_id(item);
    data.inventory_counts.get(&key).copied().unwrap_or(0) > 0
}

/// Normalize an authored item id for `inventory_has` lookups:
/// lowercase and strip every non-alphanumeric character. Mirrors how
/// [`crate::items::Item::dialog_id`] is keyed, so authored
/// dialogue can spell the item loosely.
fn normalize_item_id(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Register the generic custom dialogue commands on the runner. Called
/// from `spawn_dialogue_runner`; content commands are installed right
/// after via [`ambition_dialog::YarnContentBindings`]. Each command name maps to a
/// Bevy system registered against the `World`.
pub fn register_commands(commands: &mut Commands, runner: &mut DialogueRunner) {
    let set_flag_id = commands.register_system(cmd_set_flag);
    let clear_flag_id = commands.register_system(cmd_clear_flag);
    let challenge_id = commands.register_system(cmd_challenge);
    let use_brain_id = commands.register_system(cmd_use_brain);
    let restore_brain_id = commands.register_system(cmd_restore_brain);
    let give_item_id = commands.register_system(cmd_give_item);
    let buy_item_id = commands.register_system(cmd_buy_item);
    let sell_item_id = commands.register_system(cmd_sell_item);
    let spawn_chest_id = commands.register_system(cmd_spawn_chest);
    let play_sfx_id = commands.register_system(cmd_play_sfx);
    let spawn_fireworks_id = commands.register_system(cmd_spawn_fireworks);
    let camera_zoom_id = commands.register_system(cmd_camera_zoom);
    let cmds = runner.commands_mut();
    cmds.add_command("set_flag", set_flag_id);
    cmds.add_command("clear_flag", clear_flag_id);
    cmds.add_command("challenge", challenge_id);
    cmds.add_command("use_brain", use_brain_id);
    cmds.add_command("restore_brain", restore_brain_id);
    cmds.add_command("give_item", give_item_id);
    cmds.add_command("buy_item", buy_item_id);
    cmds.add_command("sell_item", sell_item_id);
    cmds.add_command("spawn_chest", spawn_chest_id);
    cmds.add_command("play_sfx", play_sfx_id);
    cmds.add_command("spawn_fireworks", spawn_fireworks_id);
    cmds.add_command("camera_zoom", camera_zoom_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::{Item, OwnedItems};

    #[test]
    fn normalize_item_id_collapses_spelling_variants() {
        assert_eq!(normalize_item_id("HealthPotion"), "healthpotion");
        assert_eq!(normalize_item_id("health_potion"), "healthpotion");
        assert_eq!(normalize_item_id("Health Potion"), "healthpotion");
        assert_eq!(normalize_item_id("SPARE-BATTERY"), "sparebattery");
        // The mirror is keyed by `dialog_id()`, which is already in
        // normal form, so normalization is idempotent on the keys.
        for item in Item::ALL {
            assert_eq!(normalize_item_id(item.dialog_id()), item.dialog_id());
        }
    }

    #[test]
    fn mirror_inventory_has_reads_counts_with_loose_spelling() {
        let mut data = YarnStateMirrorData::default();
        data.inventory_counts.insert("healthpotion".into(), 2);
        data.inventory_counts.insert("datachip".into(), 0);

        // Present item, however the author spells it.
        assert!(mirror_inventory_has(&data, "HealthPotion"));
        assert!(mirror_inventory_has(&data, "health_potion"));
        // Zero count reads as not held.
        assert!(!mirror_inventory_has(&data, "DataChip"));
        // Unknown item is not held.
        assert!(!mirror_inventory_has(&data, "Grapple"));
    }

    #[test]
    fn apply_give_item_adds_known_kinds_and_ignores_bad_input() {
        let mut bag = OwnedItems::default();
        // The legacy "health_potion" / "healthpotion" alias resolves to HealthCell.
        assert_eq!(bag.count(Item::HealthCell), 0);

        // Loose spelling resolves and grants.
        assert_eq!(apply_give_item(&mut bag, "health_potion", 2.0), 2);
        assert_eq!(bag.count(Item::HealthCell), 2);
        // Granting stacks (floored count) — consumables stack.
        assert_eq!(apply_give_item(&mut bag, "HealthPotion", 1.9), 1);
        assert_eq!(bag.count(Item::HealthCell), 3);

        // Unknown kind grants nothing.
        assert_eq!(apply_give_item(&mut bag, "definitely_not_an_item", 5.0), 0);
        // Non-positive count grants nothing.
        assert_eq!(apply_give_item(&mut bag, "DataChip", 0.0), 0);
        assert_eq!(apply_give_item(&mut bag, "DataChip", -3.0), 0);
        assert_eq!(bag.count(Item::DataChip), 0);
    }

    #[test]
    fn refresh_mirrors_player_inventory_into_the_snapshot() {
        // Minimal app: no save / cut-rope resources (both Option<Res>
        // resolve to None), only a live OwnedItems catalog + the mirror.
        let mut app = App::new();
        app.init_resource::<YarnStateMirror>();
        app.insert_resource(OwnedItems::starter());
        app.add_systems(Update, refresh_yarn_state_mirror);
        app.update();

        let mirror = app.world().resource::<YarnStateMirror>();
        let snap = mirror.0.read().expect("mirror readable");
        assert_eq!(
            snap.inventory_counts.get("healthpotion").copied(),
            Some(3),
            "starter inventory carries three health cells, mirrored under the legacy alias"
        );
        // The catalog dialog id resolves too.
        assert_eq!(snap.inventory_counts.get("healthcell").copied(), Some(3));
        assert!(mirror_inventory_has(&snap, "HealthPotion"));
        assert!(mirror_inventory_has(&snap, "SpareBattery"));
        // Inventory must populate even though there is no SandboxSave
        // (the save early-return runs only after the inventory slice).
        assert!(snap.flags.is_empty());
    }
}
