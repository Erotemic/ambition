//! Durable persistence for occurrence, custody, and minted-item checkpoint state.
//!
//! The on-disk representation stores domain descriptions, not ECS component snapshots:
//! authored occurrence whereabouts, custody links, and dynamic-mint identity/provenance/spec ids.
//! Loading installs those baselines and triggers the ordinary checkpoint-reset path, so the
//! checkpoint restore remains the single reconstruction authority.
//!
//! Placed runtime mints can be recreated from their saved description. In-flight mints that were
//! never admitted to the occurrence ledger are outside this horizon. `OwnedItems` remains a
//! quantity table; consumed occurrences round-trip but currently have no live producer.

use std::collections::BTreeMap;

use bevy::prelude::*;

pub use ambition_persistence::save::AmbitionGameSave;
use ambition_persistence::save_data::{
    PersistedCustody, PersistedOccurrence, PersistedWhereabouts,
};
use ambition_platformer2d_shared_tangle::lifecycle::{
    live_custody_rows, AuthoredOccurrences, CustodyBaseline, CustodyDurability, InCustodyOf,
    OccurrenceBaseline,
    OccurrenceWhereabouts, ResetToCheckpoint, RoomScopedEntity,
};
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// Whether the loaded save has been applied to this world.
///
/// This is the single durable-restore latch across inventory and occurrence
/// domains. It gates behavior and is therefore rollback state, not a cache.
#[derive(Resource, Default, Clone)]
pub struct SaveRestored(pub bool);

/// Adopt the lifecycle/occurrence domain from a loaded file.
///
/// This is deliberately one domain adapter rather than one parameter per
/// checkpoint baseline in a global census. Item-domain baselines are adopted by
/// `items::persist::restore_inventory_from_save`, beside the item state the file
/// actually restores.
pub fn adopt_occurrence_checkpoint_from_save(
    restored: Res<SaveRestored>,
    save: Res<AmbitionGameSave>,
    bodies: Query<(), ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly>,
    occurrences: Option<ResMut<AuthoredOccurrences>>,
    occurrence_baseline: Option<ResMut<OccurrenceBaseline>>,
    custody_baseline: Option<ResMut<CustodyBaseline>>,
) {
    if restored.0 || bodies.is_empty() {
        return;
    }
    let Some(occurrences) = occurrences else {
        return;
    };
    adopt_the_ledger(
        save.data(),
        occurrences,
        occurrence_baseline,
        custody_baseline,
    );
}

/// ⛔⛤ **THE LEDGER HAS ONE ADOPTION ROAD FOR A SESSION BEING BUILT, AND IT IS
/// THE CANDIDATE'S HORIZON.** There used to be a second:
/// `adopt_the_occurrence_ledger_at_activation`, a system on
/// `SessionScopeActivated` that re-read `AmbitionGameSave` into
/// `AuthoredOccurrences`, `OccurrenceBaseline` and `CustodyBaseline` process-wide.
/// It was the A10.5-era answer to a load that constructed its first room knowing
/// nothing; preparing the candidate before the route activates moved that moment
/// EARLIER still, and left the system writing the same three resources from the
/// same file one system before [`CandidateDurableHorizon::install`] wrote them
/// again.
///
/// ⚠ MEASURED 2026-09-15 over the shipped `app_it` composition: **639 gameplay
/// activations, 639 of them ran both writers, and the values agreed in every
/// one** (`save_rows` equalled the installed row count, 637 x 0 and 2 x 1). They
/// agreed because both read the same resource in the same frame — which is how a
/// second authority hides. The order was `reset_session_scoped_resources_on_activation`
/// -> the system -> `install`, uniformly, so the candidate's value won by
/// SCHEDULE ACCIDENT rather than by any edge. A save swapped between preparing
/// the candidate and publishing it, or a schedule that ever put those two the
/// other way round, would have silently made the live file authoritative over
/// the horizon the candidate was validated against.
///
/// ⭐ THE SAME MEASUREMENT FOUND ZERO UNPREPARED ACTIVATIONS — the shell's
/// `None => active_scope.begin()` branch, the road for an activation nobody
/// prepared a candidate for, was taken 0 times in 639 while the other two probes
/// fired 639 times each, so the zero is the instrument working rather than a
/// build that never ran. A mid-session load still has
/// [`adopt_occurrence_checkpoint_from_save`] above, which is a different trigger
/// (the `SaveRestored` latch) rather than a second answer to this question.
///
/// A candidate session's durable horizon, held as a VALUE.
///
/// ⛔⛤ **REVIEW FINDING 1, 2026-09-15: PREPARING A CANDIDATE MUST NOT WRITE THE
/// LIVE SESSION'S CHECKPOINT STATE.** `adopt_the_ledger_for_a_pending_candidate`
/// installed `AuthoredOccurrences`, `OccurrenceBaseline` and `CustodyBaseline`
/// process-wide while the OUTGOING session was still the live one — and the two
/// baselines are rollback-authoritative checkpoint state, deliberately NOT equal
/// to the current save/live projection. A candidate that then refused left A
/// playable with different death semantics than it had a moment before, which is
/// precisely what the last-good-world invariant forbids.
///
/// ⇒ **THE SAME SHAPE `SessionMechanics` AND `MovingPlatformSet` ALREADY USE:**
/// built from the save as a value, carried on the candidate, and installed by
/// ADOPTION. A refused candidate drops it and A's resources were never touched.
/// ⛔ Deliberately NOT another process-global `PendingFoo` mirror.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CandidateDurableHorizon {
    occurrences: AuthoredOccurrences,
    custody: BTreeMap<SimId, SimId>,
    /// ⛔⛤ **THE SECOND DESCRIPTOR CANDIDATE CONSTRUCTION NEEDS — REVIEW FINDING
    /// 2.** A ledger row says WHERE a runtime-minted occurrence is; only this
    /// says WHAT it is. Carrying the first without the second made the candidate
    /// plan from B's whereabouts and A's descriptions.
    ///
    /// ⚠ **THE RULE IS NARROW, deliberately**: every durable value consumed in
    /// deciding whether candidate B's world can be CONSTRUCTED comes from B's
    /// horizon. `OwnedItemsBaseline`, the wallet and the rest do not participate
    /// in candidate room construction and are not pulled in here.
    minted: crate::items::pickup::minted_horizon::MintedItemBaseline,
}

impl CandidateDurableHorizon {
    /// Read the save into a value. Touches no resource.
    pub fn from_save(save: &AmbitionGameSave) -> Self {
        let (rows, custody) = ledger_from_save(save.data());
        let mut occurrences = AuthoredOccurrences::default();
        occurrences.adopt_rows(rows);
        Self {
            occurrences,
            custody,
            minted: crate::items::pickup::minted_horizon::minted_baseline_from_save(save.data()),
        }
    }

    /// What the candidate's construction reads to rebuild a runtime mint.
    pub fn minted(&self) -> &crate::items::pickup::minted_horizon::MintedItemBaseline {
        &self.minted
    }

    /// What the candidate's construction reads for `OccurrenceContinuity` —
    /// the candidate's own ledger, never the live session's.
    pub fn occurrences(&self) -> &AuthoredOccurrences {
        &self.occurrences
    }

    /// Make this horizon authoritative. Called by ADOPTION and by nothing else.
    pub fn install(self, world: &mut bevy::prelude::World) {
        let Self {
            occurrences,
            custody,
            minted,
        } = self;
        if let Some(mut live) = world.get_resource_mut::<AuthoredOccurrences>() {
            *live = occurrences.clone();
        }
        if let Some(mut baseline) = world.get_resource_mut::<OccurrenceBaseline>() {
            baseline.adopt(occurrences);
        }
        if let Some(mut baseline) = world.get_resource_mut::<CustodyBaseline>() {
            baseline.adopt(custody);
        }
        if let Some(mut baseline) = world
            .get_resource_mut::<crate::items::pickup::minted_horizon::MintedItemBaseline>()
        {
            *baseline = minted;
        }
    }
}

/// The save's two ledgers, as plain maps. Pure: the half of `adopt_the_ledger`
/// that reads, split out so a candidate can have the value without the write.
fn ledger_from_save(
    data: &ambition_persistence::save_data::AmbitionGameSaveData,
) -> (
    BTreeMap<SimId, OccurrenceWhereabouts>,
    BTreeMap<SimId, SimId>,
) {
    let ledger_rows: BTreeMap<SimId, OccurrenceWhereabouts> = data
        .occurrences()
        .iter()
        .map(|row| {
            (
                SimId::from_snapshot(row.id.clone()),
                match &row.whereabouts {
                    PersistedWhereabouts::InCustody => OccurrenceWhereabouts::InCustody,
                    PersistedWhereabouts::Placed { room, x, y } => OccurrenceWhereabouts::Placed {
                        room: room.clone(),
                        at: Vec2::new(*x as f32, *y as f32),
                    },
                    PersistedWhereabouts::Consumed => OccurrenceWhereabouts::Consumed,
                },
            )
        })
        .collect();
    let held: BTreeMap<SimId, SimId> = data
        .custody()
        .iter()
        .map(|row| {
            (
                SimId::from_snapshot(row.occurrence.clone()),
                SimId::from_snapshot(row.custodian.clone()),
            )
        })
        .collect();

    (ledger_rows, held)
}

fn adopt_the_ledger(
    data: &ambition_persistence::save_data::AmbitionGameSaveData,
    mut occurrences: ResMut<AuthoredOccurrences>,
    occurrence_baseline: Option<ResMut<OccurrenceBaseline>>,
    custody_baseline: Option<ResMut<CustodyBaseline>>,
) {
    let (ledger_rows, held) = ledger_from_save(data);
    occurrences.adopt_rows(ledger_rows);
    if let Some(mut baseline) = occurrence_baseline {
        baseline.adopt(occurrences.clone());
    }
    if let Some(mut baseline) = custody_baseline {
        baseline.adopt(held);
    }
}

/// Mark durable adoption complete and request the ordinary checkpoint resume.
///
/// This is the only global completion point. Domain adopters run before it and
/// touch only their own state; this system states that every adopter in the
/// chain has had its turn. Keeping the request here prevents an item or lifecycle
/// domain from becoming the coordinator for its siblings.
pub fn complete_durable_restore(
    mut restored: ResMut<SaveRestored>,
    save: Res<AmbitionGameSave>,
    ready_body: Query<
        &ambition_characters::actor::BodyWallet,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    mut resets: MessageWriter<ResetToCheckpoint>,
) {
    if restored.0 || ready_body.single().is_err() {
        return;
    }
    restored.0 = true;
    let data = save.data();
    if !data.occurrences().is_empty()
        || !data.custody().is_empty()
        || !data.minted_items().is_empty()
    {
        resets.write(ResetToCheckpoint);
    }
}

/// Install the complete durable-save application/mirroring chain owned by the
/// actor integration layer.
///
/// The generic runtime calls this one domain offer.
pub fn install_durable_save_horizon(app: &mut App) {
    // ⛔⛔ **THE CHANNEL BESIDE THE SYSTEM THAT READS IT.** A `MessageReader` for
    // an unregistered message fails PARAMETER VALIDATION at runtime, not at
    // compile time: adding `reset_inventory_on_new_game` to the chain below
    // panicked six durable-horizon fixtures on their first frame. `add_message`
    // is guarded against a second registration, so declaring it here costs a
    // composition that also installs `session::reset` nothing and saves one that
    // does not.
    app.add_message::<crate::session::reset::NewGameResetCommitted>();
    // ⛔⛤ **THE NEW-GAME RESET BELONGS IN THE REWIND WINDOW, AND IT USED TO SIT
    // IN `Update` AT THE HEAD OF THE CHAIN BELOW.** It writes `BodyWallet` and
    // lowers `SaveRestored`, both rollback-registered, so from `Update` it
    // mutates state that every rewind restores and that this mutation is not
    // replayed with — silent drift from the peer, and only under GGRS, because a
    // fixed-tick host runs the same schedule either way.
    //
    // ⚠ The producer was already here: `process_new_game_reset_request` runs in
    // the SIM schedule's `ResetProcessing`, and `NewGameResetCommitted` is
    // `clear_message_on_rollback`. So a message produced INSIDE the rewind window
    // was being consumed OUTSIDE it.
    //
    // ⇒ `.after(clear_transient_on_sandbox_reset)` puts it on the road its
    // sibling consumer of this same message already takes, one step further
    // along the chain that flushes the producer's deferred write.
    //
    // ⛔ THE ORDERING AGAINST THE MIRRORS IS NOW STATED, NOT INHERITED FROM THE
    // FRAME. While the mirrors sat in `Update` this was carried by
    // `RunFixedMainLoop` running first; they are in this schedule too now, so
    // the edge that stops the OLD run's bag reaching the freshly wiped save has
    // to be an explicit `.after`.
    let sim = app.sim_schedule();
    app.add_systems(
        sim,
        crate::items::persist::reset_inventory_on_new_game
            .after(crate::session::reset::clear_transient_on_sandbox_reset),
    );
    // ⛔⛤ **THE LIVE→SAVE MIRRORS CROSS THE SAME ROLLBACK BOUNDARY AS THE STATE
    // THEY MIRROR.** They ran in top-level `Update` — once per FRAME — while
    // `AmbitionGameSave` is `rollback_resource_clone_checksum` and is compared
    // once per TICK. A rewind re-simulates ticks and does not replay `Update`, so
    // the same historical frame saw a different save: measured, 1 of 364 probed
    // checksum entries diverged and it was this one, with the replay xor CONSTANT
    // across frames 2, 3 and 4 while the first-pass xor moved every frame.
    //
    // ⚠ NO DISK I/O MOVES HERE. These three derive the save RESOURCE from live
    // simulation state and nothing else; autosave and the file write stay outside
    // the simulation. The layering is rollback-owned durable representation →
    // persistence projection → disk, and only the first hop is in this schedule.
    //
    // ⚠ `.chain()` is load-bearing rather than tidy: all three take
    // `ResMut<AmbitionGameSave>`, so an unordered set would leave their relative
    // order ambiguous, and an ambiguous order inside a rewinding schedule is
    // nondeterminism the checksum would then report as a desync.
    //
    // ⭐ AND THE VISIT COUNTER JOINS THEM, for the `ResMut<AmbitionGameSave>`
    // reason the `.chain()` note above gives. Its `.after` edge is the one thing
    // it needs beyond theirs: it fires on the tick a conversation OPENED, so
    // running before the opening would mean the edge is gone by the next tick and
    // the visit is never counted at all.
    app.add_systems(
        sim,
        (
            crate::items::persist::persist_inventory_to_save,
            persist_occurrence_horizon_to_save,
            crate::items::pickup::minted_horizon::persist_minted_item_horizon_to_save,
            count_the_dialogue_visit_when_a_conversation_opens
                .after(crate::features::interact_ecs_actors_and_switches),
        )
            .chain()
            .after(crate::items::persist::reset_inventory_on_new_game),
    );
    app.init_resource::<SaveRestored>()
        .add_systems(
            Update,
            (
                // Lifecycle state first: the room/custody baseline must be present
                // before the load asks the ordinary checkpoint-resume road to act.
                adopt_occurrence_checkpoint_from_save,
                // Item state second. This applies the saved bag and adopts BOTH
                // item checkpoint baselines from the post-load values.
                crate::items::persist::restore_inventory_from_save,
                // The host-level completion point comes last: only now is the file
                // fully applied, and only now may a checkpoint resume be requested.
                complete_durable_restore,
                // ⚠ THE MIRRORS USED TO CHAIN HERE AND ARE NOW IN THE SIM SCHEDULE
                // ABOVE. Their "only after the latch is true" guard is unchanged and
                // still theirs: each returns early on `!restored.0`. What changed is
                // that the latch is now read from a schedule that runs BEFORE this
                // one in the frame, so on the single frame the latch flips they
                // mirror one frame later. They are value-compared and idempotent, so
                // that costs a frame of freshness and nothing else.
            )
                .chain(),
        );
}

/// Count a dialogue visit on the tick its conversation opened. (sim)
///
/// ⛔⛤ THIS REPLACES AN INCREMENT IN `dispatch_pending_dialog_requests`, WHICH
/// RAN IN TOP-LEVEL `Update` AND LOST THE VISIT ON EVERY REWIND. Measured: an
/// `Update` write to this save is taken back by the restore, while the same
/// increment inside this schedule lands exactly once per tick across every
/// replay of that tick — `a_dialogue_visit_counted_from_update_is_taken_back_by_the_rewind`
/// and `an_increment_inside_the_tick_is_made_idempotent_by_the_restore` in
/// `game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`.
///
/// ⭐ THE RESTORE IS WHAT MAKES AN INCREMENT SAFE HERE, and that is the
/// non-obvious part. A resimulated tick does not add to the value the previous
/// run left: the snapshot puts the save back to its state before the tick, so
/// every replay adds one to the same base. The sibling mirrors above converge
/// because they DERIVE the save from sim state; this one converges for a
/// different reason, and needs no derivation.
///
/// ⚠ THE EDGE IS `opened_at == now`, NOT CHANGE DETECTION. A restore marks a
/// rollback-registered resource changed, so `Res::is_changed` fires every frame
/// under GGRS. The instance's opening tick is a pure function of rollback state
/// and names exactly one tick, so the same tick replayed re-fires it and no
/// other tick does.
///
/// ⚠ NO TIMELINE, NO VISIT. Without `SimTick` every frame would match
/// `opened_at == 0` and the count would climb forever. That is the degenerate
/// clock `ConversationInstanceId::opened_at` documents rather than a case to
/// support: a composition that cannot tell two visits apart has no visit edge.
/// Every shipped composition has the clock.
pub fn count_the_dialogue_visit_when_a_conversation_opens(
    // `Option` because a composition may install the save without the
    // conversation domain, or the clock without either.
    conversation: Option<Res<ambition_conversation::ActiveConversation>>,
    tick: Option<Res<ambition_time::SimTick>>,
    restored: Res<SaveRestored>,
    save: Option<ResMut<AmbitionGameSave>>,
) {
    // The siblings' guard, for the siblings' reason: before the latch the save is
    // still being applied from the file, and a visit written into it now is
    // written into a value the load is about to replace.
    if !restored.0 {
        return;
    }
    let (Some(conversation), Some(tick), Some(mut save)) = (conversation, tick, save) else {
        return;
    };
    let Some(live) = conversation.live() else {
        return;
    };
    if live.opened_at() != tick.0 {
        return;
    }
    save.data_mut().increment_dialog_visit(live.dialogue_id());
}

/// Mirror the current occurrence horizon into the save after restore completes. Writes are
/// value-compared so an unchanged horizon does not retrigger autosave.
///
/// Persist a custody relationship only when its owning domain can reconstruct that custody after
/// process restart. `ItemCustody` qualifies; transient body possession does not. Other occurrence
/// states cross the durable horizon directly.
#[allow(clippy::too_many_arguments)]
pub fn persist_occurrence_horizon_to_save(
    restored: Res<SaveRestored>,
    occurrences: Option<Res<AuthoredOccurrences>>,
    carried: Query<(&SimId, &InCustodyOf), With<RoomScopedEntity>>,
    custodians: Query<&SimId>,
    // The occurrences whose custody survives a process boundary, because the item
    // domain saves `ItemCustody` and applies it again on load.
    //
    // ⚠ THIS MATCHES THE COMPONENT, NOT THE `Held` VARIANT, and the two readings differ:
    // `ItemCustody` is an enum with `InWorld` as well as `Held { holder }`, so this set
    // also contains items lying on the ground. That is WIDER than the filter comment
    // below claims ("a hand it can reconstruct"), and wider is the safe direction — it
    // drops FEWER occurrence rows, so it cannot strand a `custody` or `minted_items` row
    // whose occurrence went missing. ⇒ Recorded rather than tightened: narrowing this to
    // `Held` would change what the save omits, which is a durability decision and not a
    // tidy-up. See `docs/planning/engine/item-custody-and-accounting.md`.
    durably_held: Query<&SimId, With<ambition_held_items::ItemCustody>>,
    mut save: ResMut<AmbitionGameSave>,
) {
    if !restored.0 {
        return;
    }
    let Some(occurrences) = occurrences else {
        return;
    };
    let restorable: std::collections::BTreeSet<&str> =
        durably_held.iter().map(SimId::as_str).collect();
    let rows: Vec<PersistedOccurrence> = occurrences
        .rows()
        // An `InCustody` row is a claim that something is holding this, and the
        // file may only make that claim about a hand it can reconstruct. Every
        // other whereabouts — `Placed`, `Consumed` — is a fact about the world
        // itself and crosses unconditionally.
        .filter(|(sim_id, whereabouts)| {
            !matches!(whereabouts, OccurrenceWhereabouts::InCustody)
                || restorable.contains(sim_id.as_str())
        })
        .map(|(sim_id, whereabouts)| {
            PersistedOccurrence::new(
                sim_id.as_str(),
                match whereabouts {
                    OccurrenceWhereabouts::InCustody => PersistedWhereabouts::InCustody,
                    OccurrenceWhereabouts::Placed { room, at } => PersistedWhereabouts::Placed {
                        room: room.clone(),
                        // INTEGER pixels — see `PersistedWhereabouts::Placed`.
                        // A float would cost the save's `Eq` and make a NaN
                        // rewrite the file every frame forever.
                        x: at.x.round() as i32,
                        y: at.y.round() as i32,
                    },
                    OccurrenceWhereabouts::Consumed => PersistedWhereabouts::Consumed,
                },
            )
        })
        .collect();
    // ⭐⭐ **THE LIVE CUSTODY ROWS ASK THE RELATION, NOT THE SUBJECT'S DOMAIN.**
    // `InCustodyOf::durability` is stated by whichever producer wrote the row —
    // `Restored` for an item in a hand, `SessionOnly` for a rider, a limb or a
    // possession. This was `restorable`, i.e. *"does the subject carry
    // `ambition_held_items::ItemCustody`"*, which made a THIRD producer
    // non-durable by default and silently: the accepted-control writer map named
    // that risk in writing, and the field is the answer to it.
    //
    // ⚠ THE OCCURRENCE FILTER ABOVE STILL ASKS `restorable`, AND THE TWO
    // POPULATIONS ARE NOT THE SAME ONE. That filter runs over
    // `AuthoredOccurrences` ROWS, not over live entities, so an occurrence
    // recorded `InCustody` whose entity carries no `InCustodyOf` — an
    // inconsistent state, but one this function must not make worse — is kept by
    // the wider marker and would be DROPPED by the field. Dropping a save row is
    // the dangerous direction; see this parameter's own note.
    let durable: std::collections::BTreeSet<&str> = carried
        .iter()
        .filter(|(_, custody)| custody.durability == CustodyDurability::Restored)
        .map(|(sim_id, _)| sim_id.as_str())
        .collect();
    let custody: Vec<PersistedCustody> = live_custody_rows(&carried, &custodians)
        .into_iter()
        .filter(|(occurrence, _)| durable.contains(occurrence.as_str()))
        .map(|(occurrence, custodian)| {
            PersistedCustody::new(occurrence.as_str(), custodian.as_str())
        })
        .collect();
    let data = save.data();
    if data.occurrences() == rows && data.custody() == custody {
        return;
    }
    // ⛔ `data_mut()` ONLY PAST THE GUARD ABOVE. Reaching it derefs the
    // `ResMut`, which marks the resource changed whether or not the value
    // differs -- that is what the early return protects.
    save.data_mut().set_durable_horizon(rows, custody);
}

#[cfg(test)]
mod tests;
