//! Yarn command + function + markup registrations — the "vocabulary"
//! that authored `.yarn` content can invoke at runtime.
//!
//! ⛔ **this lived in the ENGINE crate**
//! (`ambition_platformer2d_actor_monolith::dialog::yarn_bindings`) and named
//! this game's items, shop, brains and save flags from inside it. That is an
//! ownership error, not a decomposition one: `ambition_dialog` already exposes
//! [`YarnContentBindings`](ambition_dialog::YarnContentBindings) precisely so a
//! HOST pushes its own vocabulary in from outside, and this crate already
//! pushed two installers through that seam (the duel, the cut-rope commands).
//! It is the third.
//!
//! ⚠ **and it is not a decomposition win** — the monolith still names
//! `ambition_dialog` through `conversation`, so no Cargo edge moved. Recorded as
//! what it is in `docs/planning/engine/actor-monolith-decomposition.md`.
//!
//! The bindings split into three concerns:
//!
//! **Commands** (`<<set_flag X>>` syntax). Bevy systems with
//! `In<T>` parameters. Registered on the runner's `commands_mut()`
//! via `world.register_system(...)`. Each one writes to a typed
//! game-state channel (`GameplayEffect::SetFlag`, `SfxMessage::Play`,
//! …). Authored dialogue uses them to *drive* gameplay.
//!
//! **Functions** (`<<if boss_cleared("X")>>` syntax). Pure closures
//! registered on the runner's `library_mut()`, reading save state
//! through a shared [`YarnStateMirror`] refreshed each frame by
//! [`refresh_yarn_state_mirror`]. Authored dialogue uses them to
//! *read* gameplay.
//!
//! ⛔ **this module's header used to say functions "can't be Bevy systems", and
//! that was FALSE of the crate in the lockfile.** `SystemId<In<P>, O>` implements
//! `YarnFn` and `bevy_yarnspinner` hands it the interpreter's live `&mut World`.
//! ⇒ the mirror is a convenience for what has no published condition, not a
//! necessity — and `flag(id)` is gone because `world.flag_set` is published. A
//! new read here should first ask whether its domain can publish a condition
//! instead; see
//! [`ambition_platformer2d_actor_monolith::dialog::authored_conditions`].
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
//! single source of truth. Couples to `AmbitionGameSave`, `SfxMessage`,
//! `GameplayEffect`, etc. — that's the bridge's whole job.
//!
//! ## ⛔ Which side of the rollback boundary each command is on
//!
//! **This table is the classification, and it is the point of the module.**
//! Every command here runs in `Update`, driven by a Yarn runner that is content
//! executing in real time — outside the simulation and outside rollback,
//! deliberately, because rewinding a typewriter would stutter the box. A command
//! that reaches from there into the simulation has no replay story: the channels
//! it writes are cleared on rollback, the resources it mutates are restored, and
//! the runner does not execute between resimulated ticks to produce either
//! again.
//!
//! So a **gameplay-bearing** command records a request in the conversation's
//! [`NarrativeInputLedger`](ambition_platformer2d_actor_monolith::conversation::NarrativeInputLedger) and a
//! simulation system applies it on the tick it was stamped for. A
//! **presentation-facing** command writes its own channel exactly as before,
//! because its consumer is already downstream of the effect quarantine's release
//! and nothing about it is authoritative.
//!
//! | command | crosses into the sim? | how |
//! |---|---|---|
//! | `set_flag` / `clear_flag` | yes — save flags drive quests and content | ledger → `SetFlagRequested` |
//! | `challenge` | yes — it starts a fight | ledger → `ChallengeRequested` |
//! | `use_brain` / `restore_brain` | yes — it changes autonomous behaviour | ledger → `BrainCommand` / `ReleaseProvocation` |
//! | `give_item` | yes — `OwnedItems` is rollback state | ledger → `ItemGrantRequested` |
//! | `buy_item` / `sell_item` | yes — so is `BodyWallet` | ledger → `ShopTransactionRequested` |
//! | `play_sfx` | **no** — reaches the speakers, never read back | its own channel |
//! | `spawn_fireworks` | **no** — a visual sequence | its own channel |
//! | `spawn_chest`, `camera_zoom` | **no** — logged stubs with no consumer | nothing |
//!
//! ⚠ **persistent metadata is a third category and stays where it is.** Dialogue
//! visit counts and quest advancement are SAVE state, not simulation state: they
//! are not rewound, nothing in the sim branches on them within a tick, and
//! routing them through a rollback-shaped seam would be machinery for a
//! guarantee they do not need. Classified deliberately rather than by whichever
//! side happens to call them.

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
use ambition_platformer2d_actor_monolith::features::SetFlagRequested;

use ambition_dialog::YarnStateMirror;

use ambition_platformer2d_actor_monolith::conversation::NarrativeInputWriter;
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
    register_functions(runner, mirror);
}

/// Per-frame refresh: copy the relevant slices of [`AmbitionGameSave`]
/// into the mirror so Yarn functions read consistent values for the
/// duration of a single tick. Runs unconditionally — cheap because
/// the data is small (flags/bosses/quests are short Vecs).
pub fn refresh_yarn_state_mirror(
    save: Option<Res<AmbitionGameSave>>,
    wallet: Query<
        &ambition_characters::actor::BodyWallet,
        With<ambition_platformer2d_actor_monolith::actor::PrimaryPlayer>,
    >,
    mirror: Res<YarnStateMirror>,
) {
    let mut snap = mirror.0.write().expect("YarnStateMirror poisoned");
    snap.wallet_balance = wallet.iter().next().map(|w| w.balance).unwrap_or(0);
    // ⛔ **the inventory slice is GONE** — a whole second copy of `OwnedItems`,
    // rebuilt every frame under both a catalog id and a legacy alias, so that a
    // synchronous `<<if>>` could read it. `inventory.holds` is published, so the
    // `<<if>>` asks the bag.
    let Some(save) = save else {
        return;
    };
    let data = save.data();
    // ⛔ **the flag slice is GONE.** It existed so `flag(id)` could read a save
    // flag synchronously; that question is the condition catalog's
    // `world.flag_set`, asked live. ⚠ what is left in this function is the
    // remainder the catalog cannot answer yet — see this module's header on why
    // the mirror is now a projection rather than a peer.
    snap.bosses_cleared.clear();
    for boss in &data.bosses {
        if matches!(
            boss.state,
            ambition_platformer2d_actor_monolith::save::PersistedEncounterState::Cleared
        ) {
            snap.bosses_cleared.insert(boss.id.clone());
        }
    }
    snap.quests_active.clear();
    for quest in &data.quests {
        if matches!(
            quest.state,
            ambition_platformer2d_actor_monolith::save::PersistedQuestState::InProgress
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

/// `<<set_flag "id">>` — flip a save flag to `true`. Routes through
/// `SetFlagRequested` so existing consumers (quest advance
/// listeners, save mirror) see the change.
pub fn cmd_set_flag(In(name): In<String>, mut narrative: NarrativeInputWriter<SetFlagRequested>) {
    narrative.write(SetFlagRequested { id: name, on: true });
}

/// `<<clear_flag "id">>` — flip a save flag to `false`.
pub fn cmd_clear_flag(In(name): In<String>, mut narrative: NarrativeInputWriter<SetFlagRequested>) {
    narrative.write(SetFlagRequested {
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
    // ⛔ **the AUTHORITY, not `DialogState`.** This command provokes a fight, so
    // it is a simulation effect; keying it off the UI read-model meant a
    // gameplay consequence read a resource that rollback does not rewind.
    conversation: Res<ambition_platformer2d_actor_monolith::conversation::ActiveConversation>,
    player: Query<Entity, With<ambition_platformer2d_actor_monolith::actor::PlayerEntity>>,
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
    // ⛔ **this used to INSERT `PendingChallenge` from here**, which is a
    // structural write into the simulation from a system that is not part of it,
    // carrying an `Entity` across a boundary that remaps entity handles. The
    // simulation arms it now (`arm_requested_challenges`), which is also what
    // makes the armed state rollback state — see `PendingChallenge`.
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
    conversation: Res<ambition_platformer2d_actor_monolith::conversation::ActiveConversation>,
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
    conversation: Res<ambition_platformer2d_actor_monolith::conversation::ActiveConversation>,
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
/// resolved through [`ambition_platformer2d_actor_monolith::items::Item::from_dialog_id`]
/// (loose spelling); an unknown kind or non-positive count is logged
/// and ignored.
pub fn cmd_give_item(
    In((kind, count)): In<(String, f32)>,
    mut narrative: NarrativeInputWriter<
        ambition_platformer2d_actor_monolith::items::ItemGrantRequested,
    >,
) {
    let Some(request) = item_grant(&kind, count) else {
        warn!(
            target: "ambition_platformer2d_actor_monolith::dialog::yarn",
            "give_item: ignored kind={kind:?} count={count} (unknown item or non-positive count)",
        );
        return;
    };
    narrative.write(request);
}

/// `<<buy_item "id" price>>` — spend `price` from the player's wallet and grant
/// one of the catalog item if affordable. A merchant dialogue node calls this on
/// a purchase choice; the affordability check lives in [`ambition_platformer2d_actor_monolith::shop::buy`].
pub fn cmd_buy_item(
    In((id, price)): In<(String, f32)>,
    mut narrative: NarrativeInputWriter<
        ambition_platformer2d_actor_monolith::shop::ShopTransactionRequested,
    >,
) {
    let Some(item) = ambition_platformer2d_actor_monolith::items::Item::from_dialog_id(&id) else {
        warn!(target: "ambition_platformer2d_actor_monolith::dialog::yarn", "buy_item: unknown item {id:?}");
        return;
    };
    narrative.write(
        ambition_platformer2d_actor_monolith::shop::ShopTransactionRequested {
            item,
            price: price.max(0.0) as i32,
            side: ambition_platformer2d_actor_monolith::shop::ShopSide::Buy,
        },
    );
}

/// `<<sell_item "id" price>>` — remove one of the catalog item and credit the
/// wallet if the player owns it. See [`ambition_platformer2d_actor_monolith::shop::sell`].
pub fn cmd_sell_item(
    In((id, price)): In<(String, f32)>,
    mut narrative: NarrativeInputWriter<
        ambition_platformer2d_actor_monolith::shop::ShopTransactionRequested,
    >,
) {
    let Some(item) = ambition_platformer2d_actor_monolith::items::Item::from_dialog_id(&id) else {
        warn!(target: "ambition_platformer2d_actor_monolith::dialog::yarn", "sell_item: unknown item {id:?}");
        return;
    };
    narrative.write(
        ambition_platformer2d_actor_monolith::shop::ShopTransactionRequested {
            item,
            price: price.max(0.0) as i32,
            side: ambition_platformer2d_actor_monolith::shop::ShopSide::Sell,
        },
    );
}

/// Pure core of [`cmd_give_item`]: resolve a loosely-spelled kind and a Yarn
/// `f32` count into the grant the simulation should apply, or `None` when the
/// kind is unknown or the count is non-positive.
///
/// ⚠ **the flooring lives here, not at the applier.** Yarn arithmetic is
/// `f32`-typed, so "1.9 potions" is a parsing question and belongs on the side
/// that speaks Yarn. An applier that re-decided it would be a second place for
/// the rule to live and drift.
fn item_grant(
    kind: &str,
    count: f32,
) -> Option<ambition_platformer2d_actor_monolith::items::ItemGrantRequested> {
    if count <= 0.0 {
        return None;
    }
    let item = ambition_platformer2d_actor_monolith::items::Item::from_dialog_id(kind)?;
    Some(
        ambition_platformer2d_actor_monolith::items::ItemGrantRequested {
            item,
            count: count as u32,
        },
    )
}

/// `<<spawn_chest "id">>` — spawn a reward chest by id. Logged-stub;
/// the chest spawn path is currently driven by room+encounter spec
/// data, not by dialogue. Wire when needed.
pub fn cmd_spawn_chest(In(id): In<String>) {
    info!(
        target: "ambition_platformer2d_actor_monolith::dialog::yarn",
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
    player_q: Query<
        &ambition_platformer2d_actor_monolith::actor::BodyKinematics,
        ambition_platformer2d_actor_monolith::actor::PrimaryPlayerOnly,
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
        target: "ambition_platformer2d_actor_monolith::dialog::yarn",
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
    // ⛔ **`flag(id)` USED TO BE HERE, and its deletion is what this file's
    // mirror-shaped design cost.** A save flag is a world fact, the world-fact
    // domain publishes `world.flag_set` into the condition catalog, and authored
    // dialogue now asks it through the engine's generic
    // `condition("world.flag_set", "<id>")` verb — the same road a lock wall
    // takes. Two mechanisms answering one question is exactly the second
    // authority this project refuses elsewhere. See
    // `ambition_platformer2d_actor_monolith::dialog::authored_conditions`.
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
    // ⛔ **`inventory_has(item)` USED TO BE HERE**, over a mirrored copy of the
    // bag with its own spelling-normalisation and its own legacy-alias table.
    // The inventory domain publishes `inventory.holds` into the condition
    // catalog, so authored dialogue asks
    // `condition("inventory.holds", "<item>")` and reads the live `OwnedItems`
    // — with `Item::from_dialog_id` as the single owner of loose spelling. See
    // `ambition_platformer2d_actor_monolith::items::conditions`.
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

// ⛔ **`mirror_inventory_has` and `normalize_item_id` lived here** and are gone
// with the function they served. `normalize_item_id` was a second copy of the
// normalisation inside `Item::from_dialog_id` — the two agreed, which is the
// only reason nobody noticed there were two.

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
    use ambition_platformer2d_actor_monolith::items::Item;

    // ⛔ **two tests died with the functions they pinned**
    // (`normalize_item_id_collapses_spelling_variants`,
    // `mirror_inventory_has_reads_counts_with_loose_spelling`). Their subject —
    // loose item spelling — is now pinned once, in the item domain's own
    // condition, where the single implementation of it lives.

    #[test]
    fn item_grant_resolves_known_kinds_and_ignores_bad_input() {
        // The legacy "health_potion" / "healthpotion" alias resolves to HealthCell.
        assert_eq!(
            item_grant("health_potion", 2.0),
            Some(
                ambition_platformer2d_actor_monolith::items::ItemGrantRequested {
                    item: Item::HealthCell,
                    count: 2
                }
            )
        );
        // Loose spelling resolves, and the count is FLOORED — Yarn arithmetic is
        // f32-typed, so "1.9 potions" is a real thing an author can write.
        assert_eq!(
            item_grant("HealthPotion", 1.9),
            Some(
                ambition_platformer2d_actor_monolith::items::ItemGrantRequested {
                    item: Item::HealthCell,
                    count: 1
                }
            )
        );

        // Unknown kind asks for nothing.
        assert_eq!(item_grant("definitely_not_an_item", 5.0), None);
        // Non-positive count asks for nothing.
        assert_eq!(item_grant("DataChip", 0.0), None);
        assert_eq!(item_grant("DataChip", -3.0), None);
    }

    // ⛔ **`refresh_mirrors_player_inventory_into_the_snapshot` died too**, and
    // its most interesting assertion — that inventory survives a save-less
    // sandbox, because the slice was filled before the save early-return — is
    // now structural rather than tested: there is no slice, and
    // `inventory.holds` reads `OwnedItems` directly whether a save exists or not.
    // `a_composition_with_no_inventory_cannot_answer` in
    // `ambition_platformer2d_actor_monolith::items::conditions` pins the other
    // half.
}
