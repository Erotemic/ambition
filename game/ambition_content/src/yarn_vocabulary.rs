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
/// ⭐⭐ DEFINED ONCE ON PURPOSE. Two systems feed this mirror and both used to
/// run every frame of every room; the shape this campaign keeps finding is a
/// predicate hand-copied into three files and consulted by nobody, so this one
/// gets written down once and imported.
///
/// ⛔ IT IS CONVERSATION LIVENESS, NOT DIALOG-BOX PRESENCE. The mirror must be
/// fresh on the frame a Yarn `<<if>>` evaluates; a presentation-shaped gate is
/// one frame late and would feed the script a stale snapshot.
pub fn a_conversation_is_live(
    conversation: Option<bevy::prelude::Res<ambition_conversation::ActiveConversation>>,
) -> bool {
    conversation.is_some_and(|conversation| conversation.is_live())
}

/// Refresh the mirror so Yarn functions read consistent values for the duration
/// of a single tick.
///
/// ⛔⛔ IT NO LONGER RUNS UNCONDITIONALLY, and the claim it used to carry —
/// *"cheap because the data is small"* — had gone stale. Every frame of every
/// room, in a Smash match as much as in a conversation, this took a WRITE guard
/// on the mirror's `RwLock` and rebuilt three collections with a `String` clone
/// per element. Two of them are bounded by content (`bosses`, `quests`); the
/// third is not: `dialog_visits` GROWS MONOTONICALLY WITH PLAYTIME, so the
/// per-frame cost of a save's dialogue history rises for as long as somebody
/// keeps playing.
///
/// ⭐ The only reader is a live Yarn `<<if>>`, so the gate is exactly
/// "a conversation is live". ⚠ AND IT MUST BE THAT, not "a dialog box is
/// drawn": the mirror has to be fresh on the frame the `<<if>>` evaluates, and a
/// presentation-shaped gate would hand it a one-frame-stale snapshot.
///
/// ⚠ NOT MEASURED. The cost tracks save-data size, not frame count, so on a
/// fresh match it is small and on a long save it is not — and this machine's
/// noise floor could not resolve either. It is fixed because unbounded per-frame
/// work for a reader that is usually absent is wrong at any size.
pub fn refresh_yarn_state_mirror(
    save: Option<Res<AmbitionGameSave>>,
    wallet: Query<
        &ambition_characters::actor::BodyWallet,
        With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
    >,
    mirror: Res<YarnStateMirror>,
) {
    let mut snap = mirror.0.write().expect("YarnStateMirror poisoned");
    snap.wallet_balance = wallet.iter().next().map(|w| w.balance).unwrap_or(0);
    //  the inventory slice is GONE — a whole second copy of `OwnedItems`,
    // rebuilt every frame under both a catalog id and a legacy alias, so that a
    // synchronous `<<if>>` could read it. `inventory.holds` is published, so the
    // `<<if>>` asks the bag.
    let Some(save) = save else {
        return;
    };
    let data = save.data();
    //  the flag slice is GONE. It existed so `flag(id)` could read a save
    // flag synchronously; that question is the condition catalog's
    // `world.flag_set`, asked live.  what is left in this function is the
    // remainder the catalog cannot answer yet — see this module's header on why
    // the mirror is now a projection rather than a peer.
    // ⭐ THE BOSS SLICE IS GONE TOO, for the same reason the flag slice above
    // it went: `boss.cleared` answers it live from the catalog, so a projection
    // here would be a second authority with a one-frame lag.
    snap.quests_active.clear();
    for quest in &data.quests {
        if matches!(
            quest.state,
            ambition_persistence::save_data::PersistedQuestState::InProgress
        ) {
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

//  `cmd_set_flag` AND `cmd_clear_flag` USED TO BE HERE, and their deletion
// is what the COMMAND half of the authored-logic contract cost. Two
// hand-written Bevy systems differing by one bool, each registered by name below,
// each with its own conversion from Yarn's untyped text — for a verb the
// world-fact domain is perfectly able to describe itself.
//
// The world-fact domain publishes `world.set_flag(flag, on)` into the command
// catalog (`ambition_platformer2d_actor_monolith::world_facts`), and authored
// dialogue asks for it through the engine's generic
// `<<command "world.set_flag" "<id>" true>>` verb — the same road, pointed the
// other way, that `condition("world.flag_set", "<id>")` already takes. Two
// mechanisms for one verb is the second authority this project refuses
// elsewhere. See `ambition_conversation::dialog::authored_commands`.

/// `<<challenge>>` — provoke the NPC the player is currently talking to into
/// a fight. The generic dialogue-gated combat trigger: it emits an
/// [`ActorStimulus::Challenged`] for the conversation's speaker entity, which
/// `apply_actor_stimuli` turns into the same in-place peaceful→hostile flip a
/// strike would cause — but unconditionally, since picking "challenge" IS the
/// consent to fight. Any content (the Perfect Cell-ular Automaton and beyond)
/// arms a boss/duel by authoring this one command on a choice; no Rust per-NPC
/// branch. Logs and no-ops if there's no in-world speaker (scripted dialogue).
pub fn cmd_challenge(
    //  the AUTHORITY, not `DialogState`. This command provokes a fight, so
    // it is a simulation effect; keying it off the UI read-model meant a
    // gameplay consequence read a resource that rollback does not rewind.
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
    narrative.write(ambition_items::shop::ShopTransactionRequested {
        item,
        price: price.max(0.0) as i32,
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
    narrative.write(ambition_items::shop::ShopTransactionRequested {
        item,
        price: price.max(0.0) as i32,
        side: ambition_items::shop::ShopSide::Sell,
    });
}

/// Pure core of [`cmd_give_item`]: resolve a loosely-spelled kind and a Yarn
/// `f32` count into the grant the simulation should apply, or `None` when the
/// kind is unknown or the count is non-positive.
///
///  the flooring lives here, not at the applier. Yarn arithmetic is
/// `f32`-typed, so "1.9 potions" is a parsing question and belongs on the side
/// that speaks Yarn. An applier that re-decided it would be a second place for
/// the rule to live and drift.
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
    // SLOT-0 BY DESIGN: Yarn's `$player_x`/`$player_y` are authored against the
    // local player's position — dialogue is told to a human, not to a body.
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

/// `boss_cleared(id)` — ask the boss domain's published condition.
///
/// ⛔ THE THIRD ANSWER COLLAPSES THE WAY THE CATALOG SPECIFIES. Yarn's `<<if>>`
/// needs a bool, and `unanswerable is not satisfied` leaves a branch CLOSED —
/// the other direction would open a door in exactly the world where the
/// question is least understood. Same rule as `condition(id, arg)`.
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
    let outcome = world.resource_scope::<ConditionCatalog, _>(|world, catalog| {
        catalog.evaluate(world, &condition, &[AuthoredArg::Name(id.clone())])
    });
    outcome.is_satisfied()
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
    // ⭐⭐ `boss_cleared` NO LONGER READS THE MIRROR — it asks the condition
    // catalog, live, and the mirror's `bosses_cleared` slice is GONE.
    //
    // The comment that used to sit here said *"Two mechanisms answering one
    // question is exactly the second authority this project refuses
    // elsewhere"*, and it was describing this function. It is now one authority
    // with two spellings: `boss.cleared` in the catalog, reachable from an
    // authored `gated_by` line and from `condition("boss.cleared", id)`, and
    // this name kept so existing `.yarn` content is not rewritten.
    //
    // ⭐ SAME MOVE THE FLAG SLICE ALREADY MADE. `flag(id)` went when
    // `world.flag_set` landed; this is the next fact, and the precedent is
    // three lines up in `refresh_yarn_state_mirror`.
    //
    // ⚠ A REGISTERED SYSTEM, not a closure, because the catalog needs `&World`
    // — the same reason `install_condition_binding` registers one. It runs
    // inside `continue_runtime`, already exclusive, so no sync point is added.
    let boss_cleared = commands.register_system(ask_boss_cleared);
    runner.library_mut().add_function("boss_cleared", boss_cleared);

    let lib = runner.library_mut();
    // visit_count(id) -> f32:
    // how many times the named dialogue node has been entered. Returns f32 because Yarn arithmetic
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
    // The inventory domain publishes `inventory.holds` into the condition catalog, so authored
    // dialogue asks `condition("inventory.holds", "<item>")` and reads the live `OwnedItems` — with
    // `Item::from_dialog_id` as the single owner of loose spelling. See
    // `ambition_platformer2d_actor_monolith::items::conditions`. wallet_balance() -> number: the
    // player's current money, so a merchant node can show it ("You have {wallet_balance()}g").
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

//  `mirror_inventory_has` and `normalize_item_id` lived here and are gone
// with the function they served. `normalize_item_id` was a second copy of the
// normalisation inside `Item::from_dialog_id` — the two agreed, which is the
// only reason nobody noticed there were two.

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

    //  two tests died with the functions they pinned
    // (`normalize_item_id_collapses_spelling_variants`,
    // `mirror_inventory_has_reads_counts_with_loose_spelling`). Their subject —
    // loose item spelling — is now pinned once, in the item domain's own
    // condition, where the single implementation of it lives.

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
        // Loose spelling resolves, and the count is FLOORED — Yarn arithmetic is
        // f32-typed, so "1.9 potions" is a real thing an author can write.
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

    //  `refresh_mirrors_player_inventory_into_the_snapshot` died too, and
    // its most interesting assertion — that inventory survives a save-less
    // sandbox, because the slice was filled before the save early-return — is
    // now structural rather than tested: there is no slice, and
    // `inventory.holds` reads `OwnedItems` directly whether a save exists or not.
    // `a_composition_with_no_inventory_cannot_answer` in
    // `ambition_platformer2d_actor_monolith::items::conditions` pins the other
    // half.
}
