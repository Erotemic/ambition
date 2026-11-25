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
use ambition_conversation::dialog::authored_conditions::decided;
use ambition_platformer2d_shared_tangle::authored_logic::{AuthoredArg, ConditionCatalog, ConditionId, ConditionOutcome};

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
    conversation: ambition_conversation::RunnerConversation,
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
    // The talking seat's history (Q134).
    let seat = conversation.history_seat();
    snap.visit_counts.clear();
    for visit in data.dialog_visits().iter().filter(|visit| visit.seat == seat) {
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
// (`ambition_platformer2d_runtime::world_facts`), and dialogue uses
// the generic `<<command "world.set_flag" "<id>" true>>` verb, mirroring
// `condition("world.flag_set", "<id>")`. See
// `ambition_conversation::dialog::authored_commands`.

/// `<<challenge>>` — the body that started the conversation challenges the
/// one it talks to. The generic dialogue-gated combat trigger: it emits an
/// [`ActorStimulus::Challenged`] for the conversation's speaker entity, which
/// `apply_actor_stimuli` turns into the same in-place peaceful→hostile flip a
/// strike would cause — but unconditionally, since picking "challenge" IS the
/// consent to fight. Any content (the Perfect Cell-ular Automaton and beyond)
/// arms a boss/duel by authoring this one command on a choice; no Rust per-NPC
/// branch. Logs and no-ops if there's no in-world speaker (scripted dialogue).
///
/// ⛔ THE CHALLENGER IS THE CONVERSATION'S INITIATOR, as for `<<pet>>`: the
/// body the player talks through, which is a possessed body while the player
/// possesses one. Some player body in the world is not that body. That the
/// challenge is the player's is the command itself, not the challenger's
/// faction (see `apply_actor_stimuli`).
pub fn cmd_challenge(
    // Read the authority, not `DialogState`: this is a simulation effect, and
    // the UI read-model is not rewound by rollback.
    conversation: ambition_conversation::RunnerConversation,
    // The speaker's live identity, not its `SimId` alone: two instances of
    // one room hold the same authored ids, and the command means THIS body.
    bodies: ambition_platformer2d_shared_tangle::lifecycle::LiveBodies,
    mut narrative: NarrativeInputWriter<
        ambition_platformer2d_actor::body::ChallengeRequested,
    >,
) {
    let Some(actor) = conversation.talker() else {
        warn!("<<challenge>>: no speaker entity in dialogue context; ignoring");
        return;
    };
    let Some(target) = bodies.id_of(actor) else {
        warn!("<<challenge>>: speaker has no SimId; ignoring");
        return;
    };
    narrative.write(
        ambition_platformer2d_actor::body::ChallengeRequested {
            target,
            challenger: conversation.initiator().and_then(|initiator| bodies.id_of(initiator)),
        },
    );
}

/// `<<pet>>` — the body that started the conversation pets the one it talks
/// to. The petted character's catalog row must author `petting`; the
/// simulation refuses the pet with a log otherwise. Content offers it as a
/// choice, so the conversation stays reachable and the pet is one answer in it.
pub fn cmd_pet(
    conversation: ambition_conversation::RunnerConversation,
    bodies: ambition_platformer2d_shared_tangle::lifecycle::LiveBodies,
    mut narrative: NarrativeInputWriter<
        ambition_platformer2d_actor::body::PetRequested,
    >,
) {
    let (Some(initiator), Some(talker)) = (conversation.initiator(), conversation.talker()) else {
        warn!("<<pet>>: the conversation has no two in-world bodies; ignoring");
        return;
    };
    let (Some(petter), Some(petted)) = (bodies.id_of(initiator), bodies.id_of(talker)) else {
        warn!("<<pet>>: a participant has no SimId; ignoring");
        return;
    };
    narrative.write(ambition_platformer2d_actor::body::PetRequested { petter, petted });
}

/// `<<transform "character_id">>` — the character the player talks to wears
/// another catalog row: another look of the same character (its armor, its
/// grown form). The simulation refuses, with a log, an id the catalog does
/// not have. The conversation stays the one authored on the placement, so one
/// node offers every look and the way back.
pub fn cmd_transform(
    In(character): In<String>,
    conversation: ambition_conversation::RunnerConversation,
    bodies: ambition_platformer2d_shared_tangle::lifecycle::LiveBodies,
    mut narrative: NarrativeInputWriter<
        ambition_platformer2d_actor::body::WearRequested,
    >,
) {
    let Some(actor) = conversation.talker() else {
        warn!("<<transform>>: no speaker entity in dialogue context; ignoring");
        return;
    };
    let Some(body) = bodies.id_of(actor) else {
        warn!("<<transform>>: speaker has no SimId; ignoring");
        return;
    };
    narrative.write(ambition_platformer2d_actor::body::WearRequested {
        body,
        character: ambition_entity_catalog::CharacterId::new(character),
    });
}

/// `<<use_brain "preset">>` — switch the NPC the player is talking to onto an
/// explicit brain preset at runtime, changing its AUTONOMOUS behaviour (a
/// dialogue outcome like "fight me" pairs this with the `<<challenge>>` command
/// for the disposition change). Emits a
/// [`BrainCommand`](ambition_platformer2d_actor::body::BrainCommand) routed by the speaker's stable
/// id, so the runtime switch is deterministic and snapshot-safe; it never edits
/// the `Brain` component directly. No-ops (with a log) if the speaker has no
/// stable id (scripted/anonymous dialogue).
pub fn cmd_use_brain(
    In(preset): In<String>,
    conversation: ambition_conversation::RunnerConversation,
    bodies: ambition_platformer2d_shared_tangle::lifecycle::LiveBodies,
    mut narrative: NarrativeInputWriter<
        ambition_platformer2d_actor::body::BrainCommand,
    >,
) {
    let Some(actor) = conversation.talker() else {
        warn!("<<use_brain>>: no speaker entity in dialogue context; ignoring");
        return;
    };
    let Some(target) = bodies.id_of(actor) else {
        warn!("<<use_brain>>: speaker has no SimId; ignoring");
        return;
    };
    narrative.write(
        ambition_platformer2d_actor::body::BrainCommand::use_preset(
            target,
            ambition_characters::actor::character_catalog::BrainPresetId::new(preset),
        ),
    );
}

/// `<<restore_brain>>` — free the NPC the player is talking to ("you are free"):
/// the inverse of `<<challenge>>`. Emits a
/// [`ReleaseProvocation`](ambition_platformer2d_actor::body::ReleaseProvocation) by the speaker's
/// stable id, which restores BOTH the peaceful disposition and the catalog-default
/// autonomous source + complete config. (A bare `BrainCommand::RestoreDefault`
/// would restore only the brain/source, leaving a provoked NPC still hostile.)
pub fn cmd_restore_brain(
    conversation: ambition_conversation::RunnerConversation,
    bodies: ambition_platformer2d_shared_tangle::lifecycle::LiveBodies,
    mut narrative: NarrativeInputWriter<
        ambition_platformer2d_actor::body::ReleaseProvocation,
    >,
) {
    let Some(actor) = conversation.talker() else {
        warn!("<<restore_brain>>: no speaker entity in dialogue context; ignoring");
        return;
    };
    let Some(target) = bodies.id_of(actor) else {
        warn!("<<restore_brain>>: speaker has no SimId; ignoring");
        return;
    };
    narrative.write(ambition_platformer2d_actor::body::ReleaseProvocation::new(target));
}

/// `<<give_item "kind" count>>` — grant the player an item by adding
/// to the live `OwnedItems` catalog resource. The kind string is
/// resolved through [`ambition_items::ItemCatalog::item_by_dialog_id`]
/// (loose spelling); an unknown kind or non-positive count is logged
/// and ignored.
pub fn cmd_give_item(
    In((kind, count)): In<(String, f32)>,
    mut narrative: NarrativeInputWriter<ambition_items::ItemGrantRequested>,
    items: ambition_items::ItemCatalogRead,
    conversation: ambition_conversation::RunnerConversation,
) {
    let Some(mut request) = item_grant(items.get(), &kind, count) else {
        warn!(
            target: "ambition_conversation::dialog::yarn",
            "give_item: ignored kind={kind:?} count={count} (unknown item or non-positive count)",
        );
        return;
    };
    request.participant = talking_seat(&conversation);
    narrative.write(request);
}

/// `<<checkpoint>>` — bank what this conversation has done so far: a
/// checkpoint where the talking seat's body stands, through the same road as a
/// shrine's (Q164: an authored dialogue, its ending or a hand-over may
/// establish one, as one transaction with the facts and objects it moved). The
/// commit lands after the flags and hand-overs asked before it in the script.
pub fn cmd_checkpoint(
    mut narrative: NarrativeInputWriter<ambition_platformer2d_shared_tangle::lifecycle::CheckpointEstablishRequested>,
    conversation: ambition_conversation::RunnerConversation,
) {
    let seat = talking_seat(&conversation).map(|seat| seat.0);
    narrative.write(ambition_platformer2d_shared_tangle::lifecycle::CheckpointEstablishRequested { seat });
}

/// The seat whose conversation runs this command: its input owner. A
/// conversation every seat drives, or none, names no seat.
fn talking_seat(
    conversation: &ambition_conversation::RunnerConversation,
) -> Option<ambition_characters::control::PlayerSlot> {
    seat_of(conversation.input_owner()?)
}

/// The seat an input owner names: a conversation every seat drives names none.
fn seat_of(owner: ambition_input::InputOwner) -> Option<ambition_characters::control::PlayerSlot> {
    match owner {
        ambition_input::InputOwner::Participant(id) => Some(ambition_characters::control::PlayerSlot(id.slot())),
        ambition_input::InputOwner::Primary => Some(ambition_characters::control::PlayerSlot::PRIMARY),
        ambition_input::InputOwner::AllParticipants => None,
    }
}

/// `<<buy_item "id" price>>` — spend `price` from the player's wallet and grant
/// one of the catalog item if affordable. A merchant dialogue node calls this on
/// a purchase choice; the deal is the item owner's exchange
/// ([`ambition_items::shop::ShopTransactionRequested::exchange`]).
pub fn cmd_buy_item(
    In((id, price)): In<(String, f32)>,
    mut narrative: NarrativeInputWriter<ambition_items::shop::ShopTransactionRequested>,
    items: ambition_items::ItemCatalogRead,
    conversation: ambition_conversation::RunnerConversation,
) {
    let Some(item) = items.get().item_by_dialog_id(&id) else {
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
        participant: talking_seat(&conversation),
        item,
        price: coins,
        side: ambition_items::shop::ShopSide::Buy,
    });
}

/// `<<sell_item "id" price>>` — remove one of the catalog item and credit the
/// wallet if the player owns it. See
/// [`ambition_items::shop::ShopTransactionRequested::exchange`].
pub fn cmd_sell_item(
    In((id, price)): In<(String, f32)>,
    mut narrative: NarrativeInputWriter<ambition_items::shop::ShopTransactionRequested>,
    items: ambition_items::ItemCatalogRead,
    conversation: ambition_conversation::RunnerConversation,
) {
    let Some(item) = items.get().item_by_dialog_id(&id) else {
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
        participant: talking_seat(&conversation),
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
fn item_grant(
    items: &ambition_items::ItemCatalog,
    kind: &str,
    count: f32,
) -> Option<ambition_items::ItemGrantRequested> {
    if count <= 0.0 {
        return None;
    }
    let item = items.item_by_dialog_id(kind)?;
    Some(ambition_items::ItemGrantRequested {
        participant: None,
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
        emitter: ambition_sfx::SfxEmitter::Global,
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
        (Entity, &ambition_platformer2d_core::BodyKinematics),
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    // The fireworks are drawn in the live room of the player they are near.
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    let (room, origin) = player_q
        .single()
        .map(|(player, kin)| (rooms.of(player), kin.pos + ae::Vec2::new(0.0, -40.0)))
        .unwrap_or((rooms.sole(), ae::Vec2::new(480.0, 260.0)));
    fireworks.write(ambition_vfx::vfx::FireworksRequest::around(room, origin));
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
        // The runner asked, so it is the presented conversation's node.
        world
            .get_resource::<ambition_conversation::ActiveConversation>()
            .zip(ambition_conversation::speaking_conversation_instance(world))
            .and_then(|(active, instance)| active.of_instance(&instance))
            .map_or("<no live conversation>", |live| live.instance.node()),
    )
}

/// Ask the catalog's `id` question with one name argument, from a named alias.
/// A missing catalog is unanswerable, not a no.
fn ask_named(world: &mut World, alias: &str, id: &str, name: String) -> ConditionOutcome {
    let Some(condition) = ConditionId::parse(id) else {
        return ConditionOutcome::unanswerable(format!("{id:?} is not a condition id"));
    };
    if !world.contains_resource::<ConditionCatalog>() {
        bevy::log::warn!(
            target: "ambition_content::yarn_vocabulary",
            "{alias}({name:?}): no condition catalog in this composition",
        );
        return ConditionOutcome::unanswerable("no condition catalog in this composition");
    }
    let asked_by = asking_node(world);
    let outcome = world.resource_scope::<ConditionCatalog, _>(|world, catalog| {
        catalog.evaluate(world, &condition, &[AuthoredArg::Name(name.clone())], &asked_by)
    });
    if let ConditionOutcome::Unanswerable(reason) = &outcome {
        bevy::log::warn!(
            target: "ambition_content::yarn_vocabulary",
            "{alias}({name:?}) in {asked_by:?} is unanswerable: {reason}",
        );
    }
    outcome
}

/// `boss_cleared(id)` — ask the boss domain's published condition.
///
/// The Boolean alias is for positive gates: *unanswerable is not satisfied*,
/// so a branch nobody can answer stays closed. A gate with a negative arm asks
/// `boss_cleared_verdict(id)`.
fn ask_boss_cleared(In(id): In<String>, world: &mut World) -> bool {
    ask_named(world, "boss_cleared", "boss.cleared", id).is_satisfied()
}

/// `boss_cleared_verdict(id)` — the same question, answered with its verdict
/// word once per decision.
fn ask_boss_cleared_verdict(In(id): In<String>, world: &mut World) -> String {
    decided(world, format!("boss.cleared {id:?}"), |world| ask_named(world, "boss_cleared_verdict", "boss.cleared", id))
        .to_string()
}

/// `quest_active(id)` — ask the quest domain's published condition, for
/// positive gates (as [`ask_boss_cleared`]).
fn ask_quest_active(In(id): In<String>, world: &mut World) -> bool {
    ask_named(world, "quest_active", "quest.active", id).is_satisfied()
}

/// `quest_active_verdict(id)` — the same question as a verdict word.
fn ask_quest_active_verdict(In(id): In<String>, world: &mut World) -> String {
    decided(world, format!("quest.active {id:?}"), |world| ask_named(world, "quest_active_verdict", "quest.active", id))
        .to_string()
}

/// `can_afford(price)` — ask the wallet domain's published condition about the
/// conversation's payer, for positive gates.
///
/// - a fractional price is compared in `f64`, so 25 coins cannot buy a 25.7
///   item;
/// - a negative price is an authoring slip, reported as unanswerable (so the
///   branch stays closed).
fn ask_can_afford(In(price): In<f32>, world: &mut World) -> bool {
    can_afford_outcome(world, price).is_satisfied()
}

/// `can_afford_verdict(price)` — the same question as a verdict word.
fn ask_can_afford_verdict(In(price): In<f32>, world: &mut World) -> String {
    decided(world, format!("wallet.can_afford {price:?}"), |world| can_afford_outcome(world, price)).to_string()
}

fn can_afford_outcome(world: &mut World, price: f32) -> ConditionOutcome {
    let Some(condition) = ConditionId::parse("wallet.can_afford") else {
        return ConditionOutcome::unanswerable("\"wallet.can_afford\" is not a condition id");
    };
    if !world.contains_resource::<ConditionCatalog>() {
        bevy::log::warn!(
            target: "ambition_content::yarn_vocabulary",
            "can_afford({price}): no condition catalog in this composition",
        );
        return ConditionOutcome::unanswerable("no condition catalog in this composition");
    }
    let asked_by = asking_node(world);
    // The purse `<<buy_item>>` would charge: this conversation's seat's body
    // (`shop_payer`, the shop's own rule), never the primary body for a
    // second seat's conversation.
    let Some(payer) = conversation_payer(world) else {
        bevy::log::warn!(
            target: "ambition_content::yarn_vocabulary",
            "can_afford({price}) in {asked_by:?}: no single body pays for this conversation",
        );
        return ConditionOutcome::unanswerable("no single body pays for this conversation");
    };
    world.resource_scope::<ConditionCatalog, _>(|world, catalog| {
        catalog.evaluate_of(
            world,
            &condition,
            payer,
            &[AuthoredArg::Number(f64::from(price))],
            &asked_by,
        )
    })
}

/// The body a purchase in the speaking conversation charges: its seat's body
/// by the shop's rule (`shop_payer`). Read at each question, never kept
/// between the offer and the purchase.
fn conversation_payer(world: &World) -> Option<bevy::prelude::Entity> {
    let owner = world
        .get_resource::<ambition_conversation::ActiveConversation>()
        .zip(ambition_conversation::speaking_conversation_instance(world))
        .and_then(|(active, instance)| active.of_instance(&instance))
        .map(|live| live.input_owner);
    ambition_platformer2d_items::narrative::shop_payer(world, owner.and_then(seat_of))
}

/// `can_buy("id", price)` / `can_sell("id", price)` — would this
/// conversation's payer succeed at the deal `<<buy_item>>` / `<<sell_item>>`
/// would run (A7)? One question for the item and the price together, asked
/// of the item owner's exchange: a full stack, an item the bag does not hold
/// or a payout the purse cannot take greys the option out, as the deal would
/// refuse it. An unknown item, an authored price that is not a price, or no
/// single payer answers false with a warning.
fn shop_offer_answer(world: &mut World, side: ambition_items::shop::ShopSide, id: &str, price: f32) -> bool {
    let verb = match side {
        ambition_items::shop::ShopSide::Buy => "can_buy",
        ambition_items::shop::ShopSide::Sell => "can_sell",
    };
    let Some(item) = ambition_items::item_catalog(world).item_by_dialog_id(id) else {
        bevy::log::warn!(target: "ambition_content::yarn_vocabulary", "{verb}({id:?}, {price}): unknown item");
        return false;
    };
    let coins = match ambition_items::shop::authored_price(f64::from(price)) {
        Ok(coins) => coins,
        Err(problem) => {
            bevy::log::warn!(target: "ambition_content::yarn_vocabulary", "{verb}({id:?}, {price}): {}", problem.observed());
            return false;
        }
    };
    let Some(payer) = conversation_payer(world) else {
        bevy::log::warn!(target: "ambition_content::yarn_vocabulary", "{verb}({id:?}, {price}): no single body pays for this conversation");
        return false;
    };
    ambition_platformer2d_items::narrative::shop_offer(world, payer, side, item, coins).is_ok()
}

fn ask_can_buy(In((id, price)): In<(String, f32)>, world: &mut World) -> bool {
    shop_offer_answer(world, ambition_items::shop::ShopSide::Buy, &id, price)
}

fn ask_can_sell(In((id, price)): In<(String, f32)>, world: &mut World) -> bool {
    shop_offer_answer(world, ambition_items::shop::ShopSide::Sell, &id, price)
}

/// `wallet_balance()` — the player's coins, read from the wallet itself.
///
/// A registered system, not a mirror snapshot: the catalog cannot return a
/// number, but a registered system can. The live `BodyWallet` is the one
/// authority.
///
/// The purse is the conversation's payer, the one `can_afford` asks and
/// `<<buy_item>>` charges. With no single payer the line shows 0: a display
/// value, not a balance any gate reads.
fn ask_wallet_balance(In(()): In<()>, world: &mut World) -> f32 {
    conversation_payer(world)
        .and_then(|payer| world.get::<ambition_characters::actor::BodyWallet>(payer))
        .map_or(0.0, |wallet| wallet.balance as f32)
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
    // Each Boolean alias has a verdict form for gates with a negative arm
    // (O6b): `boss_cleared_verdict(id) == "not_satisfied"`.
    let boss_cleared_verdict = commands.register_system(ask_boss_cleared_verdict);
    runner.library_mut().add_function("boss_cleared_verdict", boss_cleared_verdict);
    let quest_active_verdict = commands.register_system(ask_quest_active_verdict);
    runner.library_mut().add_function("quest_active_verdict", quest_active_verdict);
    let can_afford_verdict = commands.register_system(ask_can_afford_verdict);
    runner.library_mut().add_function("can_afford_verdict", can_afford_verdict);
    let wallet_balance = commands.register_system(ask_wallet_balance);
    runner.library_mut().add_function("wallet_balance", wallet_balance);
    // A merchant's offer: the item and the price together, asked of the deal's
    // own exchange (A7).
    let can_buy = commands.register_system(ask_can_buy);
    runner.library_mut().add_function("can_buy", can_buy);
    let can_sell = commands.register_system(ask_can_sell);
    runner.library_mut().add_function("can_sell", can_sell);

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
    // the live `OwnedItems`; `ItemCatalog::item_by_dialog_id` owns loose spelling. See
    // `ambition_platformer2d_items::conditions`.
    }

// Loose item spelling has one implementation: `ItemCatalog::item_by_dialog_id`.

/// Register the generic custom dialogue commands on the runner. Called
/// from `spawn_dialogue_runner`; content commands are installed right
/// after via [`ambition_dialog::YarnContentBindings`]. Each command name maps to a
/// Bevy system registered against the `World`.
pub fn register_commands(commands: &mut Commands, runner: &mut DialogueRunner) {
    let challenge_id = commands.register_system(cmd_challenge);
    let use_brain_id = commands.register_system(cmd_use_brain);
    let restore_brain_id = commands.register_system(cmd_restore_brain);
    let pet_id = commands.register_system(cmd_pet);
    let transform_id = commands.register_system(cmd_transform);
    let give_item_id = commands.register_system(cmd_give_item);
    let buy_item_id = commands.register_system(cmd_buy_item);
    let sell_item_id = commands.register_system(cmd_sell_item);
    let spawn_chest_id = commands.register_system(cmd_spawn_chest);
    let play_sfx_id = commands.register_system(cmd_play_sfx);
    let music_id = commands.register_system(cmd_music);
    let spawn_fireworks_id = commands.register_system(cmd_spawn_fireworks);
    let camera_zoom_id = commands.register_system(cmd_camera_zoom);
    let checkpoint_id = commands.register_system(cmd_checkpoint);
    let cmds = runner.commands_mut();
    cmds.add_command("challenge", challenge_id);
    cmds.add_command("use_brain", use_brain_id);
    cmds.add_command("restore_brain", restore_brain_id);
    cmds.add_command("pet", pet_id);
    cmds.add_command("transform", transform_id);
    cmds.add_command("give_item", give_item_id);
    cmds.add_command("buy_item", buy_item_id);
    cmds.add_command("sell_item", sell_item_id);
    cmds.add_command("spawn_chest", spawn_chest_id);
    cmds.add_command("play_sfx", play_sfx_id);
    cmds.add_command("music", music_id);
    cmds.add_command("spawn_fireworks", spawn_fireworks_id);
    cmds.add_command("camera_zoom", camera_zoom_id);
    cmds.add_command("checkpoint", checkpoint_id);
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
            item_grant(ambition_items::test_item_catalog(), "health_potion", 2.0),
            Some(ambition_items::ItemGrantRequested {
                participant: None,
                item: Item::HealthCell,
                count: 2
            })
        );
        // Loose spelling resolves, and the count is floored: Yarn arithmetic is
        // f32, so "1.9 potions" is valid input.
        assert_eq!(
            item_grant(ambition_items::test_item_catalog(), "HealthPotion", 1.9),
            Some(ambition_items::ItemGrantRequested {
                participant: None,
                item: Item::HealthCell,
                count: 1
            })
        );

        // Unknown kind asks for nothing.
        assert_eq!(item_grant(ambition_items::test_item_catalog(), "definitely_not_an_item", 5.0), None);
        // Non-positive count asks for nothing.
        assert_eq!(item_grant(ambition_items::test_item_catalog(), "DataChip", 0.0), None);
        assert_eq!(item_grant(ambition_items::test_item_catalog(), "DataChip", -3.0), None);
    }

    // Inventory does not depend on a save: `inventory.holds` reads `OwnedItems`
    // directly. `a_composition_with_no_inventory_cannot_answer` in
    // `ambition_platformer2d_items::conditions` covers the
    // other half.
}
