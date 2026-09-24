//! Yarn command, function, and markup registrations available to authored `.yarn` content.
//!
//! This game owns its content vocabulary and installs it through
//! [`YarnContentBindings`](ambition_dialog::YarnContentBindings).
//!
//! Gameplay-bearing commands must enter the simulation through
//! [`NarrativeInputLedger`](ambition_conversation::NarrativeInputLedger). Presentation
//! commands use presentation channels directly. Persistent dialogue/quest metadata is
//! save state and is not rewound with the simulation.
//!
//! New generic gameplay verbs should use
//! [`ambition_conversation::dialog::authored_commands`]; new reads should prefer
//! published conditions through [`ambition_conversation::dialog::authored_conditions`].
//!
//! Conversation-specific verbs, presentation commands, Yarn functions, and markup cues
//! remain here because they are host content rather than engine vocabulary.

//! The generic binding machinery (the [`YarnStateMirror`] shape, the
//! [`ambition_dialog::YarnPresentationCue`], the [`ambition_dialog::YarnContentBindings`] installer seam, and
//! [`ambition_dialog::YarnBindingsPlugin`]) lives in the reusable `ambition_dialog` crate (E1c).
//! This module keeps only Ambition's game-specific vocabulary — the commands
//! and functions that touch actor/save state — and the per-frame refresh that
//! fills the mirror from `AmbitionGameSave`. It registers on the runtime through the
//! installer seam via [`install_game_bindings`].

use std::sync::Arc;

use ambition_platformer2d_core as ae;
use bevy::prelude::*;
use bevy_yarnspinner::prelude::DialogueRunner;

use ambition_persistence::save::AmbitionGameSave;

use ambition_dialog::YarnStateMirror;

use ambition_conversation::NarrativeInputWriter;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// The host installer: registers Ambition's generic Yarn vocabulary
/// (commands + functions) on the runner. Pushed into
/// [`ambition_dialog::YarnContentBindings`] by [`crate::plugin::AmbitionContentPlugin`] so the
/// reusable bridge names no game-specific command. It owns only generic
/// presentation commands such as `present_speaker` and `portrait_clip`.
pub fn install_game_bindings(
    commands: &mut Commands,
    runner: &mut DialogueRunner,
    mirror: &YarnStateMirror,
) {
    register_commands(commands, runner);
    register_functions(commands, runner, mirror);
}

/// Run condition: a conversation is live, so the Yarn mirror has a reader.
///
/// Defined once and shared by both systems that feed the mirror.
///
/// It checks conversation liveness, not dialog-box presence. The mirror must
/// be fresh on the frame a Yarn `<<if>>` evaluates; a presentation-shaped gate
/// is one frame late.
pub fn a_conversation_is_live(
    conversation: Option<bevy::prelude::Res<ambition_conversation::ActiveConversation>>,
) -> bool {
    conversation.is_some_and(|conversation| conversation.is_live())
}

/// Refresh the mirror so Yarn functions read consistent values for the duration
/// of a single tick.
///
/// Gated on a live conversation, the only time a Yarn `<<if>>` reads it. The
/// refresh takes a write lock and rebuilds collections with a `String` clone
/// per element, and `dialog_visits` grows with playtime, so running it every
/// frame is unbounded work for a reader that is usually absent.
///
/// The gate is conversation liveness, not "a dialog box is drawn", so the
/// mirror is fresh on the frame the `<<if>>` evaluates.
pub fn refresh_yarn_state_mirror(
    save: Option<Res<AmbitionGameSave>>,
    mirror: Res<YarnStateMirror>,
) {
    let mut snap = mirror.0.write().expect("YarnStateMirror poisoned");
    // No inventory slice: `inventory.holds` is a published condition, so the
    // `<<if>>` asks the bag.
    let Some(save) = save else {
        return;
    };
    let data = save.data();
    // No flag, boss or quest slices: `world.flag_set`, `boss.cleared` and
    // `quest.active` answer live from the condition catalog. A projection here
    // would be a second authority with a one-frame lag. What remains is what
    // the catalog cannot answer yet.
    snap.visit_counts.clear();
    for visit in data.dialog_visits() {
        snap.visit_counts.insert(visit.id.clone(), visit.count);
    }
}

// ===== Commands ================================================
//
// Bevy systems with `In<T>` parameters. The Yarn syntax
// `<<cmd_name arg1 arg2>>` invokes these via `world.register_system`
// at runner-build time. Each takes ownership of its args and writes
// to a typed message channel.

// Setting and clearing flags is not a command here. The world-fact domain
// publishes `world.set_flag(flag, on)` into the command catalog
// (`ambition_platformer2d_actor_monolith::world_facts`), and dialogue uses
// the generic `<<command "world.set_flag" "<id>" true>>` verb, mirroring
// `condition("world.flag_set", "<id>")`. See
// `ambition_conversation::dialog::authored_commands`.

/// `<<challenge>>` — provoke the NPC the player is currently talking to into
/// a fight. The generic dialogue-gated combat trigger: it emits an
/// [`ActorStimulus::Challenged`] for the conversation's speaker entity, which
/// `apply_actor_stimuli` turns into the same in-place peaceful→hostile flip a
/// strike would cause — but unconditionally, since picking "challenge" IS the
/// consent to fight. Any content (the Perfect Cell-ular Automaton and beyond)
/// arms a boss/duel by authoring this one command on a choice; no Rust per-NPC
/// branch. Logs and no-ops if there's no in-world speaker (scripted dialogue).
pub fn cmd_challenge(
    // Read the authority, not `DialogState`: this is a simulation effect, and
    // the UI read-model is not rewound by rollback.
    conversation: Res<ambition_conversation::ActiveConversation>,
    player: Query<Entity, With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>>,
    sim_ids: Query<&SimId>,
    mut narrative: NarrativeInputWriter<
        ambition_platformer2d_actor_monolith::features::ChallengeRequested,
    >,
) {
    let Some(actor) = conversation.talker() else {
        warn!("<<challenge>>: no speaker entity in dialogue context; ignoring");
        return;
    };
    let Ok(target) = sim_ids.get(actor) else {
        warn!("<<challenge>>: speaker has no SimId; ignoring");
        return;
    };
    narrative.write(
        ambition_platformer2d_actor_monolith::features::ChallengeRequested {
            target: target.clone(),
            challenger: player
                .iter()
                .next()
                .and_then(|player| sim_ids.get(player).ok())
                .cloned(),
        },
    );
}

/// `<<use_brain "preset">>` — switch the NPC the player is talking to onto an
/// explicit brain preset at runtime, changing its AUTONOMOUS behaviour (a
/// dialogue outcome like "fight me" pairs this with the `<<challenge>>` command
/// for the disposition change). Emits a
/// [`BrainCommand`](ambition_platformer2d_actor_monolith::features::BrainCommand) routed by the speaker's stable
/// id, so the runtime switch is deterministic and snapshot-safe; it never edits
/// the `Brain` component directly. No-ops (with a log) if the speaker has no
/// stable id (scripted/anonymous dialogue).
pub fn cmd_use_brain(
    In(preset): In<String>,
    conversation: Res<ambition_conversation::ActiveConversation>,
    sim_ids: Query<&SimId>,
    mut narrative: NarrativeInputWriter<
        ambition_platformer2d_actor_monolith::features::BrainCommand,
    >,
) {
    let Some(actor) = conversation.talker() else {
        warn!("<<use_brain>>: no speaker entity in dialogue context; ignoring");
        return;
    };
    let Ok(sim_id) = sim_ids.get(actor) else {
        warn!("<<use_brain>>: speaker has no SimId; ignoring");
        return;
    };
    narrative.write(
        ambition_platformer2d_actor_monolith::features::BrainCommand::use_preset(
            sim_id.clone(),
            ambition_characters::actor::character_catalog::BrainPresetId::new(preset),
        ),
    );
}

/// `<<restore_brain>>` — free the NPC the player is talking to ("you are free"):
/// the inverse of `<<challenge>>`. Emits a
/// [`ReleaseProvocation`](ambition_platformer2d_actor_monolith::features::ReleaseProvocation) by the speaker's
/// stable id, which restores BOTH the peaceful disposition and the catalog-default
/// autonomous source + complete config. (A bare `BrainCommand::RestoreDefault`
/// would restore only the brain/source, leaving a provoked NPC still hostile.)
pub fn cmd_restore_brain(
    conversation: Res<ambition_conversation::ActiveConversation>,
    sim_ids: Query<&SimId>,
    mut narrative: NarrativeInputWriter<
        ambition_platformer2d_actor_monolith::features::ReleaseProvocation,
    >,
) {
    let Some(actor) = conversation.talker() else {
        warn!("<<restore_brain>>: no speaker entity in dialogue context; ignoring");
        return;
    };
    let Ok(sim_id) = sim_ids.get(actor) else {
        warn!("<<restore_brain>>: speaker has no SimId; ignoring");
        return;
    };
    narrative.write(
        ambition_platformer2d_actor_monolith::features::ReleaseProvocation::new(sim_id.clone()),
    );
}

/// `<<give_item "kind" count>>` — grant the player an item by adding
/// to the live `OwnedItems` catalog resource. The kind string is
/// resolved through [`ambition_items::Item::from_dialog_id`]
/// (loose spelling); an unknown kind or non-positive count is logged
/// and ignored.
pub fn cmd_give_item(
    In((kind, count)): In<(String, f32)>,
    mut narrative: NarrativeInputWriter<ambition_items::ItemGrantRequested>,
) {
    let Some(request) = item_grant(&kind, count) else {
        warn!(
            target: "ambition_conversation::dialog::yarn",
            "give_item: ignored kind={kind:?} count={count} (unknown item or non-positive count)",
        );
        return;
    };
    narrative.write(request);
}

/// `<<buy_item "id" price>>` — spend `price` from the player's wallet and grant
/// one of the catalog item if affordable. A merchant dialogue node calls this on
/// a purchase choice; the affordability check lives in [`ambition_items::shop::buy`].
pub fn cmd_buy_item(
    In((id, price)): In<(String, f32)>,
    mut narrative: NarrativeInputWriter<ambition_items::shop::ShopTransactionRequested>,
) {
    let Some(item) = ambition_items::Item::from_dialog_id(&id) else {
        warn!(target: "ambition_conversation::dialog::yarn", "buy_item: unknown item {id:?}");
        return;
    };
    // One reading of the price, shared with `wallet.can_afford`. A plain cast
    // would make `-5` free and charge 25 for `25.7`.
    let coins = match ambition_items::shop::authored_price(f64::from(price)) {
        Ok(coins) => coins,
        Err(problem) => {
            warn!(
                target: "ambition_conversation::dialog::yarn",
                "buy_item {id:?} {price}: {} — refusing the transaction rather than \
                 rounding it into one the affordability check did not agree to",
                problem.observed()
            );
            return;
        }
    };
    narrative.write(ambition_items::shop::ShopTransactionRequested {
        item,
        price: coins,
        side: ambition_items::shop::ShopSide::Buy,
    });
}

/// `<<sell_item "id" price>>` — remove one of the catalog item and credit the
/// wallet if the player owns it. See [`ambition_items::shop::sell`].
pub fn cmd_sell_item(
    In((id, price)): In<(String, f32)>,
    mut narrative: NarrativeInputWriter<ambition_items::shop::ShopTransactionRequested>,
) {
    let Some(item) = ambition_items::Item::from_dialog_id(&id) else {
        warn!(target: "ambition_conversation::dialog::yarn", "sell_item: unknown item {id:?}");
        return;
    };
    // One reading of the price, shared with `wallet.can_afford`. A plain cast
    // would make `-5` free and charge 25 for `25.7`.
    let coins = match ambition_items::shop::authored_price(f64::from(price)) {
        Ok(coins) => coins,
        Err(problem) => {
            warn!(
                target: "ambition_conversation::dialog::yarn",
                "sell_item {id:?} {price}: {} — refusing the transaction rather than \
                 rounding it into one the affordability check did not agree to",
                problem.observed()
            );
            return;
        }
    };
    narrative.write(ambition_items::shop::ShopTransactionRequested {
        item,
        price: coins,
        side: ambition_items::shop::ShopSide::Sell,
    });
}

/// Pure core of [`cmd_give_item`]: resolve a loosely-spelled kind and a Yarn
/// `f32` count into the grant the simulation should apply, or `None` when the
/// kind is unknown or the count is non-positive.
///
/// Flooring lives here, not in the applier: Yarn arithmetic is `f32`, so
/// "1.9 potions" is a parsing question for the Yarn side. A second rule in the
/// applier could drift.
fn item_grant(kind: &str, count: f32) -> Option<ambition_items::ItemGrantRequested> {
    if count <= 0.0 {
        return None;
    }
    let item = ambition_items::Item::from_dialog_id(kind)?;
    Some(ambition_items::ItemGrantRequested {
        item,
        count: count as u32,
    })
}

/// `<<spawn_chest "id">>` — spawn a reward chest by id. Logged-stub;
/// the chest spawn path is currently driven by room+encounter spec
/// data, not by dialogue. Wire when needed.
pub fn cmd_spawn_chest(In(id): In<String>) {
    info!(
        target: "ambition_conversation::dialog::yarn",
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

/// `<<music "track_id">>` requests room-scoped presentation music. An empty id
/// clears the request. Fight scoring stays authored separately from `<<challenge>>`.
pub fn cmd_music(
    In(track): In<String>,
    mut music: ResMut<ambition_conversation::NarrativeMusicRequest>,
) {
    music.request(&track);
}

/// `<<spawn_fireworks>>` — spawn a short test sequence of reusable explosion
/// VFX/SFX near the player. Authored from the Kernel Guide dialog so designers
/// can verify the explosion pipeline without entering a boss room.
pub fn cmd_spawn_fireworks(
    mut fireworks: MessageWriter<ambition_vfx::vfx::FireworksRequest>,
    // Slot 0 by design: Yarn's `$player_x`/`$player_y` refer to the local
    // player; dialogue is told to a human, not to a body.
    player_q: Query<
        &ambition_platformer2d_core::BodyKinematics,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
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
        target: "ambition_conversation::dialog::yarn",
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

/// Which authored thing is asking, for the three Yarn functions below.
///
/// The node, read from the live conversation. The verdict ring records who
/// asked, and "a Yarn function" is not a source an author can find
/// (`kernel.yarn`'s shop menu alone calls `can_afford` ten times). With no
/// live conversation (a fixture), the subject says so.
fn asking_node(
    world: &bevy::prelude::World,
) -> ambition_platformer2d_shared_tangle::authored_logic::AuthoredAsk {
    ambition_platformer2d_shared_tangle::authored_logic::AuthoredAsk::new(
        "dialogue",
        world
            .get_resource::<ambition_conversation::ActiveConversation>()
            .and_then(|active| active.live())
            .map_or("<no live conversation>", |live| live.instance.node()),
    )
}

/// `boss_cleared(id)` — ask the boss domain's published condition.
///
/// The third answer collapses as the catalog specifies: Yarn's `<<if>>` needs
/// a bool, and "unanswerable" is not satisfied, so the branch stays closed.
/// Same rule as `condition(id, arg)`.
fn ask_boss_cleared(In(id): In<String>, world: &mut World) -> bool {
    use ambition_platformer2d_shared_tangle::authored_logic::{
        AuthoredArg, ConditionCatalog, ConditionId,
    };
    let Some(condition) = ConditionId::parse("boss.cleared") else {
        return false;
    };
    if !world.contains_resource::<ConditionCatalog>() {
        bevy::log::warn!(
            target: "ambition_content::yarn_vocabulary",
            "boss_cleared({id:?}): no condition catalog in this composition",
        );
        return false;
    }
    let asked_by = asking_node(world);
    let outcome = world.resource_scope::<ConditionCatalog, _>(|world, catalog| {
        catalog.evaluate(world, &condition, &[AuthoredArg::Name(id.clone())], &asked_by)
    });
    outcome.is_satisfied()
}

/// `quest_active(id)` — ask the quest domain's published condition.
///
/// Same collapse rule as [`ask_boss_cleared`]: Yarn's `<<if>>` needs a bool and
/// *unanswerable is not satisfied*, so a branch nobody can answer stays closed.
fn ask_quest_active(In(id): In<String>, world: &mut World) -> bool {
    use ambition_platformer2d_shared_tangle::authored_logic::{
        AuthoredArg, ConditionCatalog, ConditionId,
    };
    let Some(condition) = ConditionId::parse("quest.active") else {
        return false;
    };
    if !world.contains_resource::<ConditionCatalog>() {
        bevy::log::warn!(
            target: "ambition_content::yarn_vocabulary",
            "quest_active({id:?}): no condition catalog in this composition",
        );
        return false;
    }
    let asked_by = asking_node(world);
    let outcome = world.resource_scope::<ConditionCatalog, _>(|world, catalog| {
        catalog.evaluate(world, &condition, &[AuthoredArg::Name(id.clone())], &asked_by)
    });
    outcome.is_satisfied()
}

/// `can_afford(price)` — ask the wallet domain's published condition.
///
/// `can_afford` returns a boolean, so it belongs in the condition catalog
/// (unlike `wallet_balance`, which returns a number).
///
/// Behaviour follows the simulation:
/// - a fractional price is compared in `f64`, so 25 coins cannot buy a 25.7
///   item;
/// - a negative price is an authoring slip, reported as unanswerable (so the
///   branch stays closed).
fn ask_can_afford(In(price): In<f32>, world: &mut World) -> bool {
    use ambition_platformer2d_shared_tangle::authored_logic::{
        AuthoredArg, ConditionCatalog, ConditionId,
    };
    let Some(condition) = ConditionId::parse("wallet.can_afford") else {
        return false;
    };
    if !world.contains_resource::<ConditionCatalog>() {
        bevy::log::warn!(
            target: "ambition_content::yarn_vocabulary",
            "can_afford({price}): no condition catalog in this composition",
        );
        return false;
    }
    let asked_by = asking_node(world);
    let outcome = world.resource_scope::<ConditionCatalog, _>(|world, catalog| {
        catalog.evaluate(
            world,
            &condition,
            &[AuthoredArg::Number(f64::from(price))],
            &asked_by,
        )
    });
    outcome.is_satisfied()
}

/// `wallet_balance()` — the player's coins, read from the wallet itself.
///
/// A registered system, not a mirror snapshot: the catalog cannot return a
/// number, but a registered system can. The live `BodyWallet` is the one
/// authority.
///
/// Exactly one wallet, as in `wallet.can_afford` and
/// `apply_shop_transactions`. A world with two primary purses has no single
/// balance to report.
fn ask_wallet_balance(In(()): In<()>, world: &mut World) -> f32 {
    let mut wallets = world.query_filtered::<
        &ambition_characters::actor::BodyWallet,
        bevy::prelude::With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
    >();
    let mut found = wallets.iter(world);
    match (found.next().map(|w| w.balance), found.next()) {
        (Some(balance), None) => balance as f32,
        _ => 0.0,
    }
}

/// Build closures around the shared mirror and register the remaining
/// mirror-backed functions on the runner's library. Called from
/// `spawn_dialogue_runner` after the runner is built but before it
/// is spawned, so the functions are baked in.
pub fn register_functions(
    commands: &mut Commands,
    runner: &mut DialogueRunner,
    mirror: &YarnStateMirror,
) {
    // `boss_cleared` asks the condition catalog live (`boss.cleared`), also
    // reachable from a `gated_by` line and from `condition("boss.cleared", id)`.
    // The name is kept so existing `.yarn` content works.
    //
    // A registered system, not a closure, because the catalog needs `&World`
    // (as in `install_condition_binding`). It runs inside `continue_runtime`,
    // already exclusive, so no sync point is added.
    let boss_cleared = commands.register_system(ask_boss_cleared);
    runner.library_mut().add_function("boss_cleared", boss_cleared);
    // `quest_active` the same way, over `quest.active`, published by the game's
    // quest plugin (`crate::quests::conditions`); the engine has no quest
    // domain.
    let quest_active = commands.register_system(ask_quest_active);
    runner.library_mut().add_function("quest_active", quest_active);
    // `can_afford` asks the wallet domain's published condition.
    let can_afford = commands.register_system(ask_can_afford);
    runner.library_mut().add_function("can_afford", can_afford);
    // `wallet_balance` reads the live `BodyWallet` through a registered system.
    let wallet_balance = commands.register_system(ask_wallet_balance);
    runner.library_mut().add_function("wallet_balance", wallet_balance);

    let lib = runner.library_mut();
    // visit_count(id) -> f32:
    // How many times the named dialogue node has been entered. Returns f32
    // because Yarn arithmetic is f32 (`<<if visit_count("oiler") == 1>>`).
    let m = Arc::clone(&mirror.0);
    lib.add_function("visit_count", move |id: String| -> f32 {
        m.read()
            .map(|snap| snap.visit_counts.get(&id).copied().unwrap_or(0) as f32)
            .unwrap_or(0.0)
    });

    // Inventory checks use `condition("inventory.holds", "<item>")`, which reads
    // the live `OwnedItems`; `Item::from_dialog_id` owns loose spelling. See
    // `ambition_platformer2d_actor_monolith::items::conditions`.
    }

// Loose item spelling has one implementation: `Item::from_dialog_id`.

/// Register the generic custom dialogue commands on the runner. Called
/// from `spawn_dialogue_runner`; content commands are installed right
/// after via [`ambition_dialog::YarnContentBindings`]. Each command name maps to a
/// Bevy system registered against the `World`.
pub fn register_commands(commands: &mut Commands, runner: &mut DialogueRunner) {
    let challenge_id = commands.register_system(cmd_challenge);
    let use_brain_id = commands.register_system(cmd_use_brain);
    let restore_brain_id = commands.register_system(cmd_restore_brain);
    let give_item_id = commands.register_system(cmd_give_item);
    let buy_item_id = commands.register_system(cmd_buy_item);
    let sell_item_id = commands.register_system(cmd_sell_item);
    let spawn_chest_id = commands.register_system(cmd_spawn_chest);
    let play_sfx_id = commands.register_system(cmd_play_sfx);
    let music_id = commands.register_system(cmd_music);
    let spawn_fireworks_id = commands.register_system(cmd_spawn_fireworks);
    let camera_zoom_id = commands.register_system(cmd_camera_zoom);
    let cmds = runner.commands_mut();
    cmds.add_command("challenge", challenge_id);
    cmds.add_command("use_brain", use_brain_id);
    cmds.add_command("restore_brain", restore_brain_id);
    cmds.add_command("give_item", give_item_id);
    cmds.add_command("buy_item", buy_item_id);
    cmds.add_command("sell_item", sell_item_id);
    cmds.add_command("spawn_chest", spawn_chest_id);
    cmds.add_command("play_sfx", play_sfx_id);
    cmds.add_command("music", music_id);
    cmds.add_command("spawn_fireworks", spawn_fireworks_id);
    cmds.add_command("camera_zoom", camera_zoom_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_items::Item;

    // Loose item spelling is tested in the item domain's own condition.

    #[test]
    fn item_grant_resolves_known_kinds_and_ignores_bad_input() {
        // The legacy "health_potion" / "healthpotion" alias resolves to HealthCell.
        assert_eq!(
            item_grant("health_potion", 2.0),
            Some(ambition_items::ItemGrantRequested {
                item: Item::HealthCell,
                count: 2
            })
        );
        // Loose spelling resolves, and the count is floored: Yarn arithmetic is
        // f32, so "1.9 potions" is valid input.
        assert_eq!(
            item_grant("HealthPotion", 1.9),
            Some(ambition_items::ItemGrantRequested {
                item: Item::HealthCell,
                count: 1
            })
        );

        // Unknown kind asks for nothing.
        assert_eq!(item_grant("definitely_not_an_item", 5.0), None);
        // Non-positive count asks for nothing.
        assert_eq!(item_grant("DataChip", 0.0), None);
        assert_eq!(item_grant("DataChip", -3.0), None);
    }

    // Inventory does not depend on a save: `inventory.holds` reads `OwnedItems`
    // directly. `a_composition_with_no_inventory_cannot_answer` in
    // `ambition_platformer2d_actor_monolith::items::conditions` covers the
    // other half.
}
