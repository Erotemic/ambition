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

use crate::engine_core as ae;
use bevy::prelude::*;
use bevy_yarnspinner::prelude::DialogueRunner;

use crate::features::SetFlagRequested;
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
    /// Yarn-facing id for the heavy object currently hanging in the cut-rope room.
    pub cut_rope_heavy_object: String,
    /// Item `dialog_id()` → held count, mirrored from the live
    /// `PlayerInventory` resource so `inventory_has(...)` can read it.
    pub inventory_counts: std::collections::HashMap<String, u32>,
    /// Player money, mirrored from the primary player's `PlayerWallet` so a
    /// merchant dialogue can show the balance / gate purchases (`wallet_balance`,
    /// `can_afford`).
    pub wallet_balance: i32,
}

#[derive(Resource, Default, Clone)]
pub struct YarnStateMirror(pub Arc<RwLock<YarnStateMirrorData>>);

/// Per-frame refresh: copy the relevant slices of [`SandboxSave`]
/// into the mirror so Yarn functions read consistent values for the
/// duration of a single tick. Runs unconditionally — cheap because
/// the data is small (flags/bosses/quests are short Vecs).
pub fn refresh_yarn_state_mirror(
    save: Option<Res<SandboxSave>>,
    cut_rope_heavy_object: Option<Res<crate::ambition_content::bosses::CutRopeHeavyObjectCycle>>,
    inventory: Option<Res<crate::inventory::PlayerInventory>>,
    owned: Option<Res<crate::items::OwnedItems>>,
    wallet: Query<&crate::player::PlayerWallet, With<crate::player::PrimaryPlayer>>,
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
            if let Some(legacy) = item.legacy_kind() {
                snap.inventory_counts
                    .insert(legacy.dialog_id().to_string(), count);
            }
        }
    } else if let Some(inventory) = inventory {
        // Fallback for builds without the catalog resource (shouldn't happen in
        // practice, but keeps dialogue working with just the legacy bag).
        for kind in crate::inventory::ItemKind::ALL {
            snap.inventory_counts
                .insert(kind.dialog_id().to_string(), inventory.count(kind));
        }
    }
    let Some(save) = save else {
        return;
    };
    let data = save.data();
    snap.cut_rope_heavy_object.clear();
    snap.cut_rope_heavy_object.push_str(
        cut_rope_heavy_object
            .map(|cycle| cycle.current_dialogue_id())
            .unwrap_or("anvil"),
    );
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

/// `<<give_item "kind" count>>` — grant the player an item by adding
/// to the live `PlayerInventory` resource. The kind string is
/// resolved through [`crate::inventory::ItemKind::from_dialog_id`]
/// (loose spelling); an unknown kind or non-positive count is logged
/// and ignored.
pub fn cmd_give_item(
    In((kind, count)): In<(String, f32)>,
    mut inventory: ResMut<crate::inventory::PlayerInventory>,
    owned: Option<ResMut<crate::items::OwnedItems>>,
) {
    let legacy_granted = apply_give_item(&mut inventory, &kind, count);
    // Also grant into the 24-item catalog (the superset that covers items with
    // no legacy `ItemKind` — weapons, abilities, the Alice/Bob key items, …).
    let catalog_granted = if count > 0.0 {
        match (owned, crate::items::Item::from_dialog_id(&kind)) {
            (Some(mut owned), Some(item)) => {
                let n = count as u32;
                owned.grant(item, n);
                n
            }
            _ => 0,
        }
    } else {
        0
    };
    if legacy_granted == 0 && catalog_granted == 0 {
        warn!(
            target: "ambition_sandbox::dialog::yarn",
            "give_item: ignored kind={kind:?} count={count} (unknown item or non-positive count)",
        );
    } else {
        info!(
            target: "ambition_sandbox::dialog::yarn",
            "give_item: granted {}x {kind:?}",
            legacy_granted.max(catalog_granted),
        );
    }
}

/// `<<buy_item "id" price>>` — spend `price` from the player's wallet and grant
/// one of the catalog item if affordable. A merchant dialogue node calls this on
/// a purchase choice; the affordability check lives in [`crate::shop::buy`].
pub fn cmd_buy_item(
    In((id, price)): In<(String, f32)>,
    mut owned: ResMut<crate::items::OwnedItems>,
    mut wallets: Query<&mut crate::player::PlayerWallet, With<crate::player::PrimaryPlayer>>,
) {
    let Some(item) = crate::items::Item::from_dialog_id(&id) else {
        warn!(target: "ambition_sandbox::dialog::yarn", "buy_item: unknown item {id:?}");
        return;
    };
    let Ok(mut wallet) = wallets.single_mut() else {
        return;
    };
    let outcome = crate::shop::buy(&mut wallet, &mut owned, item, price.max(0.0) as i32);
    info!(
        target: "ambition_sandbox::dialog::yarn",
        "buy_item: {id:?} @ {price} -> {outcome:?} (balance now {})", wallet.balance,
    );
}

/// `<<sell_item "id" price>>` — remove one of the catalog item and credit the
/// wallet if the player owns it. See [`crate::shop::sell`].
pub fn cmd_sell_item(
    In((id, price)): In<(String, f32)>,
    mut owned: ResMut<crate::items::OwnedItems>,
    mut wallets: Query<&mut crate::player::PlayerWallet, With<crate::player::PrimaryPlayer>>,
) {
    let Some(item) = crate::items::Item::from_dialog_id(&id) else {
        warn!(target: "ambition_sandbox::dialog::yarn", "sell_item: unknown item {id:?}");
        return;
    };
    let Ok(mut wallet) = wallets.single_mut() else {
        return;
    };
    let outcome = crate::shop::sell(&mut wallet, &mut owned, item, price.max(0.0) as i32);
    info!(
        target: "ambition_sandbox::dialog::yarn",
        "sell_item: {id:?} @ {price} -> {outcome:?} (balance now {})", wallet.balance,
    );
}

/// Pure core of [`cmd_give_item`]: add `count` (floored to a
/// non-negative integer) of the named item to the bag. Returns the
/// number actually granted (0 when the kind is unknown or the count
/// is non-positive) so the command can log and tests can assert
/// without a live `World`.
fn apply_give_item(
    inventory: &mut crate::inventory::PlayerInventory,
    kind: &str,
    count: f32,
) -> u32 {
    if count <= 0.0 {
        return 0;
    }
    let Some(item) = crate::inventory::ItemKind::from_dialog_id(kind) else {
        return 0;
    };
    let n = count as u32;
    inventory.add(item, n);
    n
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
pub fn cmd_play_sfx(In(id_str): In<String>, mut sfx: MessageWriter<crate::audio::SfxMessage>) {
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::SfxId::new(&id_str),
        pos: ae::Vec2::ZERO,
    });
}

/// `<<spawn_fireworks>>` — spawn a short test sequence of reusable explosion
/// VFX/SFX near the player. Authored from the Kernel Guide dialog so designers
/// can verify the explosion pipeline without entering a boss room.
pub fn cmd_spawn_fireworks(
    mut fireworks: MessageWriter<crate::presentation::fx::FireworksRequest>,
    player_q: Query<&crate::player::BodyKinematics, crate::player::PrimaryPlayerOnly>,
) {
    let origin = player_q
        .single()
        .map(|kin| kin.pos + ae::Vec2::new(0.0, -40.0))
        .unwrap_or(ae::Vec2::new(480.0, 260.0));
    fireworks.write(crate::presentation::fx::FireworksRequest::around(origin));
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

/// `<<watch_cut_rope_video>>` — authored post-boss branch placeholder.
///
/// TODO(web-open): desktop builds could optionally open the URL in the user's
/// browser after a settings/privacy opt-in. For now the Yarn line presents the
/// link and this command records the choice as a save flag for traceability.
pub fn cmd_watch_cut_rope_video(mut effects: MessageWriter<SetFlagRequested>) {
    info!(
        target: "ambition_sandbox::dialog::yarn",
        "watch_cut_rope_video: TODO optional browser launch for https://www.youtube.com/watch?v=ucLGm27DDL0",
    );
    effects.write(SetFlagRequested {
        id: "smirking_behemoth_video_suggested".into(),
        on: true,
    });
}

/// `<<reset_cut_rope_room>>` — replay the Smirking Behemoth room from the start.
///
/// The command is reached by Yarn immediately after the NPC's final line is
/// presented, before the player has dismissed that line. Latch a pending replay
/// resource instead of resetting immediately; the simulation emits the real
/// replay request once `DialogState` is inactive.
pub fn cmd_reset_cut_rope_room(
    mut pending: ResMut<crate::ambition_content::bosses::PendingCutRopeRoomReplay>,
) {
    pending.requested = true;
    info!(
        target: "ambition_sandbox::dialog::yarn",
        "reset_cut_rope_room: latched Smirking Behemoth room replay until dialogue closes",
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
    // cut_rope_heavy_object_is(id) -> bool: lets authored Yarn branch on the
    // runtime-selected heavy object without reaching into Bevy resources.
    let m = Arc::clone(&mirror.0);
    lib.add_function("cut_rope_heavy_object_is", move |id: String| -> bool {
        m.read()
            .map(|snap| snap.cut_rope_heavy_object == id)
            .unwrap_or(false)
    });
    // inventory_has(item) -> bool: does the player currently hold at
    // least one of the named item? Reads the inventory slice the
    // refresh system mirrors from `PlayerInventory`. The item argument
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
/// [`crate::inventory::ItemKind::dialog_id`] is keyed, so authored
/// dialogue can spell the item loosely.
fn normalize_item_id(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Register all eleven custom commands on the runner. Called from
/// `spawn_dialogue_runner`. Each command name maps to a Bevy
/// system registered against the `World`.
pub fn register_commands(commands: &mut Commands, runner: &mut DialogueRunner) {
    let set_flag_id = commands.register_system(cmd_set_flag);
    let clear_flag_id = commands.register_system(cmd_clear_flag);
    let give_item_id = commands.register_system(cmd_give_item);
    let buy_item_id = commands.register_system(cmd_buy_item);
    let sell_item_id = commands.register_system(cmd_sell_item);
    let spawn_chest_id = commands.register_system(cmd_spawn_chest);
    let play_sfx_id = commands.register_system(cmd_play_sfx);
    let spawn_fireworks_id = commands.register_system(cmd_spawn_fireworks);
    let camera_zoom_id = commands.register_system(cmd_camera_zoom);
    let watch_cut_rope_video_id = commands.register_system(cmd_watch_cut_rope_video);
    let reset_cut_rope_room_id = commands.register_system(cmd_reset_cut_rope_room);
    let cmds = runner.commands_mut();
    cmds.add_command("set_flag", set_flag_id);
    cmds.add_command("clear_flag", clear_flag_id);
    cmds.add_command("give_item", give_item_id);
    cmds.add_command("buy_item", buy_item_id);
    cmds.add_command("sell_item", sell_item_id);
    cmds.add_command("spawn_chest", spawn_chest_id);
    cmds.add_command("play_sfx", play_sfx_id);
    cmds.add_command("spawn_fireworks", spawn_fireworks_id);
    cmds.add_command("camera_zoom", camera_zoom_id);
    cmds.add_command("watch_cut_rope_video", watch_cut_rope_video_id);
    cmds.add_command("reset_cut_rope_room", reset_cut_rope_room_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::{ItemKind, PlayerInventory};

    #[test]
    fn normalize_item_id_collapses_spelling_variants() {
        assert_eq!(normalize_item_id("HealthPotion"), "healthpotion");
        assert_eq!(normalize_item_id("health_potion"), "healthpotion");
        assert_eq!(normalize_item_id("Health Potion"), "healthpotion");
        assert_eq!(normalize_item_id("SPARE-BATTERY"), "sparebattery");
        // The mirror is keyed by `dialog_id()`, which is already in
        // normal form, so normalization is idempotent on the keys.
        for kind in ItemKind::ALL {
            assert_eq!(normalize_item_id(kind.dialog_id()), kind.dialog_id());
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
        let mut bag = PlayerInventory::default();
        assert_eq!(bag.count(ItemKind::HealthPotion), 0);

        // Loose spelling resolves and grants.
        assert_eq!(apply_give_item(&mut bag, "health_potion", 2.0), 2);
        assert_eq!(bag.count(ItemKind::HealthPotion), 2);
        // Granting stacks (floored count).
        assert_eq!(apply_give_item(&mut bag, "HealthPotion", 1.9), 1);
        assert_eq!(bag.count(ItemKind::HealthPotion), 3);

        // Unknown kind grants nothing.
        assert_eq!(apply_give_item(&mut bag, "Grapple", 5.0), 0);
        // Non-positive count grants nothing.
        assert_eq!(apply_give_item(&mut bag, "DataChip", 0.0), 0);
        assert_eq!(apply_give_item(&mut bag, "DataChip", -3.0), 0);
        assert_eq!(bag.count(ItemKind::DataChip), 0);
    }

    #[test]
    fn refresh_mirrors_player_inventory_into_the_snapshot() {
        // Minimal app: no save / cut-rope resources (both Option<Res>
        // resolve to None), only a live PlayerInventory + the mirror.
        let mut app = App::new();
        app.init_resource::<YarnStateMirror>();
        app.insert_resource(PlayerInventory::starter()); // 2 health, 1 battery, 1 chip
        app.add_systems(Update, refresh_yarn_state_mirror);
        app.update();

        let mirror = app.world().resource::<YarnStateMirror>();
        let snap = mirror.0.read().expect("mirror readable");
        assert_eq!(
            snap.inventory_counts.get("healthpotion").copied(),
            Some(2),
            "starter inventory carries two health cells"
        );
        assert!(mirror_inventory_has(&snap, "HealthPotion"));
        assert!(mirror_inventory_has(&snap, "SpareBattery"));
        // Inventory must populate even though there is no SandboxSave
        // (the save early-return runs only after the inventory slice).
        assert!(snap.flags.is_empty());
    }
}
