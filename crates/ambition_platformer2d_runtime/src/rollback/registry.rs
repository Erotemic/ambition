//! Thin Ambition registration layer over `bevy_ggrs`.
//!
//! This is deliberately not a snapshot registry. `bevy_ggrs` owns storage,
//! history, entity reconciliation, save/load ordering, and checksum aggregation.
//! The registry here records the exact typed contract installed into GGRS so
//! prepared content and peers can reject incompatible binaries before play.

use std::collections::BTreeMap;
use std::fmt;

use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use bevy_ggrs::{
    ComponentSnapshotPlugin, LoadWorld, LoadWorldSystems, ResourceSnapshotPlugin, RollbackApp,
};

use crate::content_identity::SnapshotSchemaFingerprint;
use crate::SimulationHost;

use super::{
    cursor_checksum, resolved_checksum, state_checksum, CanonicalCodecStrategy, SnapshotCursor,
    SnapshotResolve, SnapshotState,
};

/// Managed same-build schema version for Ambition's GGRS registration contract.
///
/// ⚠ **v29 (2026-08-15): `ActorControl` encodes one bool fewer.** D126.2 deleted
/// `ActorControlFrame::drop_through`, a field no brain ever set and
/// `to_input_state` never mapped — drop-through is a DERIVED gesture
/// (`descend + jump`, owned by `movement::integration::wants_drop_through`), so
/// there was no `InputState` slot for the boolean to reach and the declaration
/// was a refusal written as a capability. The registration SET is unchanged, so
/// this moves the same two things v27 did and nothing else: this constant, and
/// `scripts/tests/rollback_codec_shape.txt` (238 → 236 primitives for
/// `ambition_characters/src/snapshot_impls.rs`, one `put_bool` and one
/// `r.bool()`). The frozen-name JSON and the descriptor baseline record no field
/// detail and correctly do not move.
///
/// ⚠ **v28 (2026-08-14): `resource.combat_slot_board` is no longer registered.**
/// Its subject was a crowd-arbitration board that no production reader
/// consumed, so every peer was agreeing each tick about a value nothing asked
/// for. Removing a registration shrinks the SET, so unlike v27 this moves the
/// frozen-name count and the schema baseline as well as the codec-shape hash.
///
/// ⚠ **v27 (2026-08-14): `BodyCombat` encodes seven fields, not nine.** AC3
/// deleted `alive` and `attacking` as duplicate authorities — liveness is
/// `BodyHealth`'s and the melee answer is `BodyMelee`'s, and a mirror that
/// rewinds is a second thing a peer can disagree about. The registration SET is
/// unchanged, so both guards that watch this area stayed green: the frozen-name
/// contract counts names and the schema baseline records name/kind/type/
/// description, and a field inside a codec moves none of them. What noticed was
/// `scripts/rollback_codec_shape.py`, which hashes the ordered sequence of codec
/// primitives — exactly the question a peer depends on.
///
/// ⛔ **and v25/v26 have no entry here, which is the same omission one step
/// earlier.** v26 is AC1's deletion of two write-only actor mirrors: the
/// registrations `actor.intent` and `actor.cooldowns` left the schema; the
/// version was bumped and the log was not written. ⚠ **named by REGISTRATION
/// and not by type on purpose** — the goal guard greps this crate for those two
/// type names to prove they are gone from production, and writing them here to
/// say they are gone turns that check red. Documenting a removal must not break
/// the guard that verified it, which is a rule this repo has now learned four
/// times and the fourth was this sentence. Recorded now rather than
/// left as a gap, because a version log with holes cannot answer *what changed
/// between these two peers*, which is the one question it exists for.
///
/// ⚠ **v20 (2026-08-09): a crate name stops being part of the wire format.**
/// [`RollbackRegistry::schema_dump`] wrote `std::any::type_name` whole, so moving
/// a registered type between crates — or between modules of one crate — moved
/// `SnapshotSchemaFingerprint` while nothing a peer can observe had changed. That
/// made every carve in the decomposition campaign a netplay compatibility break.
/// The dump now writes [`wire_type_identity`], the type's final segment alone,
/// which is the only part of the name a carve leaves alone. This is v5's decision
/// applied one level out: an organisational label is not a wire-format fact, and
/// unlike the owner nobody chose to hash this one. Note the descriptor list is
/// otherwise unchanged and every stable name is identical — but a v19 peer
/// computes a different number over the same schema, and they must not believe
/// they agree. [`RollbackRegistry::try_register`] now REJECTS two different types
/// that reduce to one identity, which is what keeps the narrower form sound.
/// ⚠ **v19 (2026-08-08): a conversation's identity includes what YARN is entered
/// with.** `ConversationInstanceId` named the tick, the node and the two bodies'
/// `SimId`s; the `DialogueContext` — `$speaker_id`, `$listener_id`,
/// `$speaker_is_self`, which content branches on — sat beside it, and the speaker
/// resolves from the initiator's `WornCharacter`, which is rollback-owned and
/// runtime-mutable. So two corrected timelines could agree on all four body facts
/// while entering the node as different characters, and a v18 peer calls those
/// ONE conversation: it applies an abandoned branch's ledger records to the
/// corrected one and leaves the text box attached, running with the old
/// `$speaker_id` in Yarn's variable storage. That is a different history, not a
/// different encoding of one — and the checksum moves with it, because the
/// context now arrives inside the hashed instance id instead of as three fields
/// hashed after it (GPT 5.6 review, D29). Note the descriptor list is
/// byte-identical: this is the wire-change class only the version constant can
/// see. `speaker_name` stays out of both — it is a display string.
/// ⚠ **v18 (2026-08-08): TwinTrack's Relativity Plaza/Festival encodings.**
/// Two overlay changes land together, so they are ONE bump rather than the two
/// the overlay carried: `AbilitySet` separates a body's flight CAPABILITY from
/// permission to expose a runtime flight toggle (permanent free flight is a
/// spacecraft, not a body with a button it must not press); axis-swept flight
/// tuning optionally carries an invariant speed; light emitters, signals and
/// arrival history preserve stable emitter identity, opaque payloads and
/// optional destination channels; and the experiment component carries the
/// guided-introduction step, multi-round light-tag attempts and the
/// spacetime-replay cursor. A v17 peer decodes every one of those shifted.
/// ⚠ **v17 (2026-08-08): being IN a fight becomes a component of its own.**
/// `CombatStanding` read `RulesetOwnsDeath` — whose question is whose business a
/// body's death is — and the stand-down rule read `MatchSeat`, which an
/// eliminated fighter keeps. `ActiveCombatant` is the one authority, and unlike
/// the marker it replaces it is REMOVED during a match, so a v16 peer rewinding
/// past an elimination puts back a fighter that is out and this one does not.
/// The two would disagree about who is still playing.
/// ⚠ **v16 (2026-08-07): every narrative fact crosses through a LEDGER, and the
/// ledger holds more than one.** `ObservedNarrativeEnd` held exactly one record
/// and justified it by arguing a player has to read the first conversation
/// before a second can finish — not an engine invariant. A v15 peer reaching
/// back past two completed conversations replays only the later one, so the
/// earlier one comes back live and stays live, holding a body and capturing a
/// seat the other peer has released. `message.conversation_ended` returns as the
/// ledger's released payload, cleared on load so the resimulated tick is handed
/// it again rather than remembering it.
/// ⚠ **v15 (2026-08-07): the narrative end stops being a cleared MESSAGE.**
/// `message.conversation_ended` is gone and `ObservedNarrativeEnd` — a stamped
/// external input that is deliberately NOT rollback state — replaced it. A v14
/// peer clears the message on load and has nothing that survives the rewind, so
/// it resimulates every tick after a conversation ended with that conversation
/// still live, holding a body and capturing a seat. That is a different
/// simulation from this one, and the two must not believe they agree.
/// ⚠ **v11 (2026-08-06): cutscene PLAYBACK becomes canonical state.**
/// `ActiveCutscene::is_playing()` drives a capturing input-context claim, so a
/// v10 peer cannot reconstruct whether the participant was allowed to act — it
/// would resimulate a cutscene frame with gameplay input live.
/// ⚠ **v10 (2026-08-05): the optional SR causal-pursuit capability adds a
/// canonical target declaration, while TwinTrack extends its canonical experiment
/// state with observer-local aim, view mode, pursuit timing, and hit results. A v9
/// peer cannot reconstruct which worldline is an intercept target or the same game
/// phase after rewind.
///
/// ⚠ **v9 (2026-08-05): the optional SR observer capability adds canonical
/// optical-source and controlled-observer declarations. The observer optical
/// view remains derived, but a v8 peer cannot restore which entities authored
/// those declarations.
///
/// ⚠ **v8 (2026-08-05): the optional SR signal capability adds authoritative
/// Minkowski coordinate time, proper-time transmitter cooldown, emitter,
/// receiver/pool configuration, analytic null-signal state, bounded arrival
/// history, and cleared signal-message buffers. TwinTrack rules depend on these
/// values surviving rewind; a v7 peer does not encode them.
///
/// ⚠ **v7 (2026-08-04): the optional relativity capability adds authoritative
/// f64 proper-time clocks, spacetime identity, and TwinTrack experiment state.**
/// A host that installs it has a different snapshot contract from v6.
///
/// ⚠ **v6 (2026-07-31): `BodyHealth` carries its damage METER and DEATH POLICY
/// on the wire.** It had gained both when the stocks loop landed and encoded
/// neither, so every rewind restored a fighter at 0% under `HpDepleted` — a
/// value change the checksum could not see, because it hashed the same
/// incomplete encoding (GPT 5.6 review, finding 1). A peer on v5 stores three
/// fields where this stores five; they must not believe they agree.
///
/// ⚠ **v5 (2026-07-31): the fingerprint stopped hashing the registration
/// OWNER.** The owner is an organisational label nothing reads, and hashing it
/// made "which module registered this" a wire-format fact — so moving a
/// registration between modules declared two otherwise-identical peers
/// incompatible. Bumped rather than changed silently: peers on v4 computed a
/// different number over the same schema, and they must not believe they agree.
/// ⚠ **v24 (2026-08-10): a Mary-O coin block remembers how much it still owes.**
/// `SpentPowerBlocks` was a set — a block was spent or it was not — and a
/// multi-coin block needs a COUNT, so it now carries a per-block tally beside
/// the set and folds that tally into its checksum projection. A v23 peer hashes
/// the same set and cannot see how many coins a partly-paid block has left, so
/// two peers could agree on the hash while disagreeing about which one runs out
/// first.
///
/// ⚠ **no `put_*` codec changed, which is why the shape guard is silent here.**
/// This is a `rollback_resource_clone_checksum` registration: the snapshot is a
/// Clone and the wire-visible half is the checksum. `scripts/rollback_codec_shape.py`
/// watches primitive sequences and cannot see a projection — a second blind spot
/// worth knowing, and the reason this entry exists rather than a red check.
///
/// ⚠ **v23 (2026-08-10): the platform-fighter feel layer joins the wire
/// format, and one deferred transition learns what sound it owes.** Four value
/// encodings changed, none of them adding or removing a registration — which is
/// exactly why this needed a deliberate bump rather than being caught by a
/// checker. `rollback-wire-format-is-frozen` counts stable NAMES and every name
/// here is unchanged, so a peer on v22 encodes the same 348 names over
/// different bytes and would believe it agreed.
///
/// * `BodyCombat` gains `landing_lag_timer` — an authored aerial's landing
///   commitment, a hard input lock a peer must reconstruct or it hands control
///   back early.
/// * `AxisManeuverState` gains `jump_squat_timer` and `AxisLocomotion` gains
///   `jump_squat_time` + `max_air_speed`. The squat timer is a COMMITTED leap
///   mid-flight: a peer that cannot restore it either drops the jump or fires
///   it on the wrong tick.
/// * `PendingLifecycleCommit`'s `Transition` intent gains `zone_sfx`, the cue
///   the crossing owes. Rollback state because the intent is, and it changes
///   the byte layout of a variant a v22 peer already decodes.
///
/// ⚠ **v22 (2026-08-09): resolved body hits become the authoritative
/// on-hit seam.** `HitboxOnHit` no longer snapshots/maps a per-victim entity
/// set: body-hit deduplication lives in `HitboxHits`, while a same-frame
/// `LandedBodyHit` carries the already-resolved attacker/victim/contact fact to
/// move confirms and on-hit effects. `feature.pogo_target` and
/// `map.hitbox_on_hit` therefore leave the schema, and
/// `message.landed_body_hit` joins the load-clear contract. The value encoding
/// of `HitboxOnHit` also changes from `EffectRef + BTreeSet<Entity>` to
/// `EffectRef + world_fired`, and that mutable world-contact latch now has its
/// own checksum projection instead of hiding behind a presence-only clone. A v21
/// peer therefore decodes different bytes even aside from the registration-set
/// change.
///
/// ⚠ **v21 (2026-08-09): the DEATH INTERLUDE joins the wire format** (ADR
/// 0033). `actor.out_of_play` and `actor.death_interlude` carry whether a
/// participant's attempt has ended, how long its window still has to run, and
/// whether it still owes the game its consequence. All of that changes mid-run
/// and all of it is gameplay truth: a peer that cannot reconstruct it
/// resimulates with a body the world has stopped touching for a death that has
/// not happened in its branch, or replays a level reset that already did.
///
/// ⭐ they REPLACE a game-side registration. `content.mary_o_death_sequence`
/// carried the same facts for one game and was clone-probed rather than
/// checksummed, so this is the wire format gaining truth it was already relying
/// on, not gaining a feature.
/// ⚠ **v30 (2026-08-15): ITEM CUSTODY joins the wire format.** A physical item
/// used to be despawned when a body picked it up and a fresh one spawned when it
/// was thrown, so "who has this axe" was carried by the existence of an entity
/// and GGRS reproduced it through the entity anchor. `item.item_custody` is the
/// state that took that job over: it decides, on every later frame, whether the
/// item is drawn, stepped by the item physics, and grabbable by anyone else. A
/// peer that cannot reconstruct it resimulates with the same axe in a hand and
/// on the floor.
///
/// ⭐ it REPLACES a despawn/spawn pair rather than adding a fact. What is
/// genuinely new is that the item now KEEPS its identity across the transfer —
/// an authored ground item's `SimId::placement(...)` used to die at the pickup.
///
/// ⚠ **v31 (2026-08-15): the GATE PORTAL PHASE joins the wire format.**
/// `resource.gate_portal_phases` is the `Opening`/`Closing` timer that decides
/// when a gate becomes traversable, integrated every simulated tick from a
/// switch that was ALREADY in the wire format (`resource.sandbox_save`). Only the
/// input rewound; the integral did not. A peer resimulating after a rollback
/// carried the speculative timeline's elapsed forward, so the frame on which the
/// gate opened depended on that peer's rollback history — and
/// `detect_room_transition_system` gates a room crossing on it.
///
/// ⭐ it was previously covered by a WAIVER that said "authored gate portals",
/// which was a true sentence about the switch ids and sprite names sharing the
/// resource and a false one about the timer. The phase now has its own resource
/// so the waiver's claim about the remaining one is true.
/// ⚠ **v32 (2026-08-15): the RESET BASELINE joins the wire format.**
/// `resource.occurrence_baseline` and `resource.custody_baseline` are what a
/// death restores — the occurrence ledger and the hands, as they stood at the
/// last committed checkpoint.
///
/// ⭐ **they are the first values here that NOTHING republishes**, and that is
/// why they had to join rather than be declared derived like the live ledger
/// they copy. A checkpoint commits from a shrine touched mid-frame, so the write
/// is squarely inside the rollback window; a peer that rewound past it and did
/// not restore these would hold a baseline recorded in a timeline that no longer
/// happened, and would put the world back to it on the next death.
///
/// ⚠ **their two channels join too** (`message.checkpoint_committed`,
/// `message.reset_to_checkpoint`). A reader's cursor is `Local` state GGRS never
/// rewinds, and for an ordinary channel a stale cursor costs one duplicated or
/// skipped read; for these it decides whether a baseline is recorded at all, and
/// nothing later re-derives the answer.
/// ⚠ **v33 (2026-08-16): the MINTED-INSTANCE DESCRIPTION joins the wire
/// format.** `resource.minted_item_baseline` is the third leg of the same
/// checkpoint, and the first one that is not an identity: it says how to REBUILD
/// an instance the simulation minted, because such an instance is room-scoped
/// and carryable — it can enter `resource.custody_baseline` — and no authored
/// record anywhere describes it.
///
/// ⭐ **the encoding is provenance plus a spec ID, and deliberately not a
/// snapshot of the occurrence's components.** Component state is what rollback
/// already carries; a checkpoint outlives the entity, so it stores the
/// DEFINITION (a reference into the item catalog) and the
/// `SpawnOrigin::Dynamic` that says which spawner it descends from. A held
/// object needs no position — the hand supplies one, and nothing steps an item
/// that is not `InWorld`.
///
/// ⚠ **v34 (2026-08-16) is a RENAME, not a new value.**
/// `resource.inventory_restored` became `resource.save_restored`: the latch stopped
/// meaning "the catalog has been applied" and started meaning "the loaded save has
/// been applied", because the durable occurrence horizon
/// (`session::durable_horizon`) reads the same flag. Deliberately ONE latch — a
/// second would be a second answer to one question, free to disagree the day one
/// leg's precondition was met and the other's was not. The row's projection is
/// unchanged (a bare clone, no checksum), but a row KEY is part of the wire
/// identity, so two peers whose schemas differ here cannot agree about a snapshot
/// and the version has to say so.
pub const GGRS_ROLLBACK_SCHEMA_VERSION: u32 = 34;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RollbackEntryKind {
    ComponentCanonical,
    ComponentCloneCursor,
    ComponentCloneResolved,
    ComponentClone,
    ComponentCloneCanonicalChecksum,
    ComponentCloneCustomChecksum,
    ResourceCanonical,
    ResourceCloneCursor,
    ResourceClone,
    ResourceCloneCustomChecksum,
    MessageClear,
    EntityMapping,
    ResourceEntityMapping,
    RequiredRollback,
    Derived,
    DynamicAnchor,
}

impl RollbackEntryKind {
    /// True when this registration means "the rollback carries this type's VALUE
    /// across a save/load", and therefore that a localizer must be able to see it.
    ///
    /// The distinction is what makes probe coverage testable rather than asserted:
    /// the other kinds accompany a state registration for the same type (entity
    /// remapping, `Rollback` requirement) or describe something with no restored
    /// value at all (a message channel that is cleared, a dynamic anchor). Only
    /// these carry state, and every one of them must own a probe.
    ///
    /// `Derived` is included deliberately. Derived state is not snapshotted, but its
    /// contract — "the named system rebuilds it before anything reads it" — is
    /// exactly what the resimulation-boundary comparison tests, and
    /// `ProjectileOwner` was a `declare_rollback_derived` naming a system whose query
    /// could not see enemy projectiles. A derived declaration with no probe is a
    /// promise with no auditor.
    pub fn carries_state(self) -> bool {
        match self {
            Self::ComponentCanonical
            | Self::ComponentCloneCursor
            | Self::ComponentCloneResolved
            | Self::ComponentClone
            | Self::ComponentCloneCanonicalChecksum
            | Self::ComponentCloneCustomChecksum
            | Self::ResourceCanonical
            | Self::ResourceCloneCursor
            | Self::ResourceClone
            | Self::ResourceCloneCustomChecksum
            | Self::Derived => true,
            Self::MessageClear
            | Self::EntityMapping
            | Self::ResourceEntityMapping
            | Self::RequiredRollback
            | Self::DynamicAnchor => false,
        }
    }

    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::ComponentCanonical => "component-canonical",
            Self::ComponentCloneCursor => "component-clone-cursor",
            Self::ComponentCloneResolved => "component-clone-resolved",
            Self::ComponentClone => "component-clone",
            Self::ComponentCloneCanonicalChecksum => "component-clone-canonical-checksum",
            Self::ComponentCloneCustomChecksum => "component-clone-custom-checksum",
            Self::ResourceCanonical => "resource-canonical",
            Self::ResourceCloneCursor => "resource-clone-cursor",
            Self::ResourceClone => "resource-clone",
            Self::ResourceCloneCustomChecksum => "resource-clone-custom-checksum",
            Self::MessageClear => "message-clear",
            Self::EntityMapping => "entity-mapping",
            Self::ResourceEntityMapping => "resource-entity-mapping",
            Self::RequiredRollback => "required-rollback",
            Self::Derived => "derived",
            Self::DynamicAnchor => "dynamic-anchor",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RollbackRegistrationDescriptor {
    pub name: String,
    pub owner: String,
    pub kind: RollbackEntryKind,
    pub type_name: String,
    pub detail: String,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct RollbackRegistry {
    entries: BTreeMap<String, RollbackRegistrationDescriptor>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RollbackRegistrationError {
    EmptyName,
    EmptyOwner,
    Conflict {
        name: String,
        existing: RollbackRegistrationDescriptor,
        incoming: RollbackRegistrationDescriptor,
    },
    /// Two DIFFERENT Rust types reduce to the same [`wire_type_identity`].
    ///
    /// This is what keeps v20's narrower identity sound. The fingerprint hashes
    /// the type's final segment so that relocating a type is not a wire-format
    /// change — and that is only truthful while final segments are unique. Two
    /// crates each registering a `Cooldown` would hash equal, and a peer that
    /// had them the other way round would be declared compatible.
    ///
    /// ⚠ registering ONE type under several stable names is not this. The whole
    /// point of a stable name is that it identifies the registration; 39 of the
    /// live rows do exactly that, and they carry identical type names.
    TypeIdentityCollision {
        identity: String,
        existing: RollbackRegistrationDescriptor,
        incoming: RollbackRegistrationDescriptor,
    },
}

impl fmt::Display for RollbackRegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "rollback registration name must not be empty"),
            Self::EmptyOwner => write!(f, "rollback registration owner must not be empty"),
            Self::Conflict {
                name,
                existing,
                incoming,
            } => write!(
                f,
                "conflicting rollback registration '{name}': existing {existing:?}, incoming {incoming:?}"
            ),
            Self::TypeIdentityCollision {
                identity,
                existing,
                incoming,
            } => write!(
                f,
                "two different types share the rollback wire identity '{identity}', which the \
                 schema fingerprint cannot tell apart: existing {existing:?}, incoming {incoming:?}. \
                 Since v20 the fingerprint hashes a type's FINAL SEGMENT so that moving a type \
                 between crates or modules is not a wire-format change, and that stays sound only \
                 while final segments are unique. Rename one of the two types."
            ),
        }
    }
}

impl std::error::Error for RollbackRegistrationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RollbackRegistrationOutcome {
    /// The descriptor was inserted and the active GGRS host should install its
    /// runtime snapshot/checksum machinery.
    Inserted,
    /// The exact descriptor was already present.
    Idempotent,
    /// The descriptor was inserted for schema/content identity, but this host
    /// does not run GGRS and therefore must not install rollback machinery.
    RecordedOnly,
}

/// Record a localization probe beside the checksum projection.
///
/// Called from the SAME arm that installs the GGRS checksum, so a component
/// cannot be rollback-registered and stay invisible to
/// [`crate::rollback::RollbackRestoreAudit`]. Both holes in the previous
/// instrument were "the sweep did not know to look here", and coupling the two
/// registrations is what stops that from recurring.
fn record_probe(app: &mut App, probe: crate::rollback::ChecksumProbe) {
    app.world_mut()
        .get_resource_or_insert_with(crate::rollback::RollbackChecksumProbes::default)
        .register(probe);
}

/// **The part of a type's name that a CARVE leaves alone.**
///
/// `std::any::type_name` spells the crate and the module path, and until v20 the
/// whole string went into [`RollbackRegistry::schema_dump`] and therefore into
/// the fingerprint. Moving a type then declared two peers running byte-identical
/// snapshot logic incompatible — the same category of mistake v5 removed when it
/// stopped hashing `owner`, except that nobody chose this one: it arrived inside
/// a string that was being used for identity.
///
/// ⭐ **the final segment, and not the module path below the crate**, which is
/// what the answer was until the diff it cited was read. D33 step 2
/// (`24b43f93a`) moved two registered components, and the crate changed AND the
/// path below it changed — `features::ecs::actor_clusters` → `character::anim`,
/// `avatar::components` → `camera_ease` — because a carve puts a type where it
/// belongs rather than merely somewhere else. Only the final segment survived
/// either move.
///
/// Every path INSIDE the name is shortened, not only the outermost one, so a
/// generic keeps its constructor: `Vec<foo::Bar>` is `Vec<Bar>` and not `Bar>`.
/// No registration is generic today; taking a single `rsplit` would quietly give
/// `Vec<X>` and `VecDeque<X>` one identity, and this is a hash whose entire job
/// is telling wire formats apart.
fn wire_type_identity(type_name: &str) -> String {
    let mut out = String::with_capacity(type_name.len());
    let mut path = String::new();
    for ch in type_name.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == ':' {
            path.push(ch);
        } else {
            out.push_str(final_segment(&path));
            path.clear();
            out.push(ch);
        }
    }
    out.push_str(final_segment(&path));
    out
}

fn final_segment(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}

impl RollbackRegistry {
    pub fn try_register(
        &mut self,
        descriptor: RollbackRegistrationDescriptor,
    ) -> Result<RollbackRegistrationOutcome, RollbackRegistrationError> {
        if descriptor.name.trim().is_empty() {
            return Err(RollbackRegistrationError::EmptyName);
        }
        if descriptor.owner.trim().is_empty() {
            return Err(RollbackRegistrationError::EmptyOwner);
        }
        match self.entries.get(&descriptor.name) {
            Some(existing) if existing == &descriptor => {
                return Ok(RollbackRegistrationOutcome::Idempotent);
            }
            Some(existing) => {
                return Err(RollbackRegistrationError::Conflict {
                    name: descriptor.name.clone(),
                    existing: existing.clone(),
                    incoming: descriptor,
                });
            }
            None => {}
        }
        // **What keeps v20's narrower identity sound.** The fingerprint hashes
        // [`wire_type_identity`] so that relocating a type is not a wire-format
        // change; two crates each registering a `Cooldown` would then hash equal,
        // and a peer that had the two the other way round would be declared
        // compatible with this one. The duplicate-NAME refusal above does not
        // reach it — these arrive under different stable names, which is exactly
        // the case that looks legitimate.
        let identity = wire_type_identity(&descriptor.type_name);
        let collision = self
            .entries
            .values()
            .find(|existing| {
                existing.type_name != descriptor.type_name
                    && wire_type_identity(&existing.type_name) == identity
            })
            .cloned();
        if let Some(existing) = collision {
            return Err(RollbackRegistrationError::TypeIdentityCollision {
                identity,
                existing,
                incoming: descriptor,
            });
        }
        self.entries.insert(descriptor.name.clone(), descriptor);
        Ok(RollbackRegistrationOutcome::Inserted)
    }

    pub fn descriptors(&self) -> impl Iterator<Item = &RollbackRegistrationDescriptor> {
        self.entries.values()
    }

    /// Stable human-readable representation; byte-identical under equivalent
    /// plugin/registration insertion orders.
    pub fn deterministic_dump(&self) -> String {
        let mut out = format!("ggrs-rollback-schema-v{}\n", GGRS_ROLLBACK_SCHEMA_VERSION);
        for entry in self.entries.values() {
            use std::fmt::Write as _;
            let _ = writeln!(
                out,
                "{}\t{}\t{}\t{}\t{}",
                entry.name,
                entry.owner,
                entry.kind.canonical_name(),
                entry.type_name,
                entry.detail
            );
        }
        out
    }

    /// **What the schema actually IS**, with every organisational label removed.
    ///
    /// [`Self::deterministic_dump`] carries `owner` and the type's full path
    /// because a human reading a conflict wants to know which module registered
    /// a thing and where the type lives. Nothing else reads either — and both
    /// were once hashed into the fingerprint, which made purely organisational
    /// facts part of the WIRE FORMAT. Two peers running identical snapshot logic
    /// would have been declared incompatible because one of them had moved a
    /// registration to a different module, or moved the TYPE to a different one.
    ///
    /// That is not hypothetical: Campaign 2 exists to move every registration
    /// out of the central runtime into domain adapters, and R3 asks each move to
    /// "verify the resulting schema fingerprint is unchanged" — which was
    /// impossible while the fingerprint hashed who did the registering. The
    /// decomposition campaign then hit the same wall one level out, because a
    /// carve moves the TYPE and not only its registration. `owner` left in v5;
    /// [`wire_type_identity`] is the second half of that decision, in v20.
    pub fn schema_dump(&self) -> String {
        let mut out = format!("ggrs-rollback-schema-v{GGRS_ROLLBACK_SCHEMA_VERSION}\n");
        for entry in self.entries.values() {
            use std::fmt::Write as _;
            let _ = writeln!(
                out,
                "{}\t{}\t{}\t{}",
                entry.name,
                entry.kind.canonical_name(),
                wire_type_identity(&entry.type_name),
                entry.detail
            );
        }
        out
    }

    /// **Which of these requirements is NOT installed.**
    ///
    /// A capability offers its rollback state and the composition installs it,
    /// which keeps the capability's dependency closure to foundations. The hole
    /// that leaves is that nothing forces the composition to accept the offer —
    /// and a skipped registration is a DESYNC, not a missing feature.
    ///
    /// This closes it the way the content compiler closes the same shape: the
    /// obligation is declared next to the thing that has it
    /// ([`ambition_platformer2d_core::snapshot::RequiredRollbackState`]) and the
    /// assembler can refuse when it is unmet.
    ///
    /// ⚠ it checks the OWNER too. A name registered by somebody else is not
    /// this capability's state — two capabilities may reasonably both want a
    /// `cooldown`, and only the owner distinguishes them.
    pub fn missing_required_state<'a>(
        &self,
        required: &'a [ambition_platformer2d_core::snapshot::RequiredRollbackState],
    ) -> Vec<&'a ambition_platformer2d_core::snapshot::RequiredRollbackState> {
        required
            .iter()
            .filter(|req| {
                !self
                    .entries
                    .values()
                    .any(|entry| entry.name == req.name && entry.owner == req.owner)
            })
            .collect()
    }

    pub fn schema_fingerprint(&self) -> SnapshotSchemaFingerprint {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"ambition.ggrs-rollback-schema\0");
        hasher.update(&GGRS_ROLLBACK_SCHEMA_VERSION.to_le_bytes());
        let dump = self.schema_dump();
        hasher.update(&(dump.len() as u64).to_le_bytes());
        hasher.update(dump.as_bytes());
        SnapshotSchemaFingerprint::from_bytes(*hasher.finalize().as_bytes())
    }
}

fn descriptor<T: 'static>(
    owner: &'static str,
    name: &'static str,
    kind: RollbackEntryKind,
    detail: &'static str,
) -> RollbackRegistrationDescriptor {
    descriptor_owned::<T>(owner, name, kind, detail.to_string())
}

/// [`descriptor`] for a detail this crate COMPOSES rather than quotes.
///
/// The recorded `detail` is two halves: how the value is stored (the backend's
/// half — "bevy_ggrs clone snapshot") and what the checksum sees (the domain's
/// half). A domain that registers its own state through
/// [`ambition_platformer2d_core::snapshot::RollbackRegistrar`] supplies only the
/// second half, precisely so a crate with no `bevy_ggrs` dependency never has to
/// write the word; this joins them back into the exact string the schema baseline
/// records.
fn descriptor_owned<T: 'static>(
    owner: &'static str,
    name: &'static str,
    kind: RollbackEntryKind,
    detail: String,
) -> RollbackRegistrationDescriptor {
    RollbackRegistrationDescriptor {
        name: name.to_string(),
        owner: owner.to_string(),
        kind,
        type_name: std::any::type_name::<T>().to_string(),
        detail,
    }
}

/// **The one install body behind `rollback_resource_clone_checksum`**, shared by
/// the `App` façade ([`AmbitionRollbackApp`]) and by the domain-facing registrar
/// in [`super::registrar`].
///
/// ⚠ **factored rather than copied on purpose.** The two entry points differ
/// only in who composes the recorded `detail` string; the snapshot registration,
/// the GGRS checksum and the localization probe are one behaviour, and a second
/// copy of them is a place for the probe half to silently go missing — which is
/// the exact class of defect (`presence accounted, value unrestored`) this
/// registration exists to close.
pub(in crate::rollback) fn install_resource_clone_checksum<T>(
    app: &mut App,
    owner: &'static str,
    name: &'static str,
    detail: String,
    checksum: for<'a> fn(&'a T) -> u64,
) where
    T: Resource + Clone,
{
    if register_app_descriptor(
        app,
        descriptor_owned::<T>(
            owner,
            name,
            RollbackEntryKind::ResourceCloneCustomChecksum,
            detail,
        ),
    ) == RollbackRegistrationOutcome::Inserted
    {
        RollbackApp::rollback_resource_with_clone::<T>(app);
        RollbackApp::checksum_resource(app, checksum);
        record_probe(
            app,
            crate::rollback::ChecksumProbe::new(std::any::type_name::<T>(), move |world| {
                crate::rollback::census_resource_with::<T>(world, checksum)
            }),
        );
    }
}

fn register_app_descriptor(
    app: &mut App,
    descriptor: RollbackRegistrationDescriptor,
) -> RollbackRegistrationOutcome {
    // Every composition records the same typed vocabulary because prepared
    // content identity includes the snapshot-schema fingerprint. Only a GGRS
    // host turns a newly inserted descriptor into bevy_ggrs runtime machinery.
    // Fixed/render-frame games therefore keep exact compatibility metadata
    // without paying for schedules, snapshots, checksums, or session handling.
    let ggrs_host =
        app.world().get_resource::<SimulationHost>().copied() == Some(SimulationHost::Ggrs);
    app.init_resource::<RollbackRegistry>();
    let outcome = app
        .world_mut()
        .resource_mut::<RollbackRegistry>()
        .try_register(descriptor)
        .unwrap_or_else(|error| panic!("{error}"));
    match (ggrs_host, outcome) {
        (true, outcome) => outcome,
        (false, RollbackRegistrationOutcome::Inserted) => RollbackRegistrationOutcome::RecordedOnly,
        (false, RollbackRegistrationOutcome::Idempotent) => RollbackRegistrationOutcome::Idempotent,
        (false, RollbackRegistrationOutcome::RecordedOnly) => unreachable!(),
    }
}

/// App-level typed registration vocabulary. Every method records the exact
/// managed schema identity once; a GGRS host additionally installs the real
/// `bevy_ggrs` runtime plugin/system for newly inserted descriptors.
pub trait AmbitionRollbackApp {
    fn rollback_component_canonical<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + SnapshotState;

    fn rollback_component_cursor<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotCursor;

    fn rollback_component_resolved<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotResolve;

    fn rollback_component_clone<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone;

    /// Clone-snapshot a component that holds an ENTITY REFERENCE, and probe it
    /// through the target's stable sim identity.
    ///
    /// The same snapshot contract as [`Self::rollback_component_clone`] — no GGRS
    /// checksum, because a raw entity id legitimately differs after a load and
    /// putting it in the aggregate would report a desync on every rewind. What it
    /// adds is a VALUE-sensitive localization probe: a restore that puts back the
    /// right number of references and points one of them at a different body changes
    /// this census, and does not change a presence count.
    ///
    /// `referenced` extracts the handle. Pair this with
    /// [`Self::rollback_map_entities`], which is what actually remaps it.
    fn rollback_component_clone_entity_ref<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> bevy::prelude::Entity,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone;

    /// Clone-snapshot a component holding a SET of entity references, probed
    /// through their stable sim identities. The multi-handle twin of
    /// [`Self::rollback_component_clone_entity_ref`].
    fn rollback_component_clone_entity_set<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<bevy::prelude::Entity>,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone;

    /// Clone-snapshot a component holding a KEYED MAP of entity references,
    /// probed with the key folded in.
    ///
    /// The map twin of [`Self::rollback_component_clone_entity_set`], and NOT
    /// interchangeable with it: a set fold is commutative, so a map measured as
    /// a set cannot see two keys exchange their targets. Use this whenever the
    /// association between key and entity is itself the state.
    fn rollback_component_clone_entity_map<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<(u64, bevy::prelude::Entity)>,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone;

    /// Clone-snapshot with a projection the LOCALIZER measures and the GGRS
    /// aggregate does not.
    ///
    /// The distinction matters and is easy to lose: `rollback_component_clone_checksum`
    /// hands the same projection to both, which makes any nondeterminism in it a
    /// session-wide desync report. This arm strengthens only the diagnostic. Use it
    /// for per-tick mutable state whose restore you want localizable without changing
    /// what the sync test calls a divergence — sharpening an instrument should not
    /// move the guard it is used to investigate.
    fn rollback_component_clone_probed<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        projection: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone;

    /// Clone the exact component for load/mapping, but checksum a canonical
    /// projection. Use this for state containing `Entity` handles or authored
    /// references that GGRS must preserve and remap rather than decode itself.
    fn rollback_component_clone_state<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotState;

    /// Clone the exact component and include a domain-owned deterministic
    /// checksum projection. The detail string is part of the exact schema.
    fn rollback_component_clone_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
        checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone;

    fn rollback_resource_canonical<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Resource + SnapshotState;

    /// Canonical snapshot for a resource that legitimately COMES AND GOES.
    ///
    /// [`Self::rollback_resource_canonical`] cannot serve one: it installs
    /// `bevy_ggrs`'s `ResourceChecksumPlugin`, whose system takes `Res<T>` and
    /// therefore panics on any frame the resource is absent — *"Parameter
    /// `Res<'_, ActiveMatch>` failed validation: Resource does not exist"*. The
    /// SNAPSHOT half already handles absence correctly (`ResourceSnapshotPlugin`
    /// maps `(Some(_), None)` to `remove_resource`), so the gap was only ever in
    /// the checksum.
    ///
    /// This supplies a checksum over `Option<T>`: absence hashes to a distinct
    /// constant, so "the match had not activated yet" and "the match activated
    /// with these seats" are different checksums rather than one of them being
    /// unrepresentable. That distinction IS the state for a latch whose whole
    /// job is to exist (AA2 / AC2).
    fn rollback_resource_optional_canonical<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Resource + SnapshotState;

    fn rollback_resource_cursor<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Resource + Clone + SnapshotCursor;

    fn rollback_resource_clone<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Resource + Clone;

    /// Clone-snapshot a RESOURCE holding entity references, probed through their
    /// stable sim identities. The resource twin of
    /// [`Self::rollback_component_clone_entity_set`].
    fn rollback_resource_clone_entity_set<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<bevy::prelude::Entity>,
    ) -> &mut Self
    where
        T: Resource + Clone;

    /// The same, **plus the fields the entity set cannot see**.
    ///
    /// ⛔ **an entity-set probe is silent about everything that is not an
    /// entity**, and for a resource that holds both it reports two divergent
    /// values as identical. `ActiveConversation` is the case that found it: the
    /// probe localized the two bodies faithfully while `input_owner` — which
    /// decides whose controls the conversation captures — could differ between
    /// peers with no signal at all (GPT 5.6, 2026-08-07).
    ///
    /// ⚠ `facts` must NOT hash raw entity handles. Those differ across a load by
    /// design, which is the whole reason the entity half goes through stable sim
    /// identities; mixing them in would make every load look like a desync.
    fn rollback_resource_clone_entity_set_probed<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<bevy::prelude::Entity>,
        facts: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: Resource + Clone;

    fn rollback_resource_clone_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
        checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: Resource + Clone;

    fn rollback_map_entities<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + bevy::ecs::entity::MapEntities;

    fn rollback_resource_map_entities<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Resource + bevy::ecs::entity::MapEntities;

    fn require_rollback<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component;

    fn clear_message_on_rollback<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Message;

    fn declare_rollback_derived<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: 'static;

    /// Declare derived state that is a COMPONENT, and register a presence probe
    /// for it.
    ///
    /// A `declare_rollback_derived` is an assertion about behaviour: "the system
    /// named in `reason` rebuilds this every tick". Nothing checked that assertion,
    /// and one of them was false — `ProjectileOwner` named a healing system whose
    /// query could not see enemy projectiles at all, which cost a day of
    /// bisection and was the equipment oracle's whole divergence. A derived
    /// declaration that lies is worse than no declaration, because it satisfies
    /// the coverage sweep.
    ///
    /// The probe makes it PARTLY falsifiable: `RollbackRestoreAudit` compares each
    /// frame's census against the first pass, so a derived component that FAILS to be
    /// rebuilt on a replayed frame shows up by name. That is the failure that shipped,
    /// and the one this arm was added for.
    ///
    /// It is not the whole contract, and the earlier version of this comment claimed
    /// it was (GPT 5.6, 2026-07-26). A presence census sees a MISSING derived
    /// component; it cannot see one rebuilt with entirely wrong values on the right
    /// number of carriers, and for a singleton derived resource "present" is nearly a
    /// constant. `declare_rollback_derived_component_state` is the value-sensitive
    /// twin, and gameplay-significant derived state should use it. Which of these are
    /// still presence-only is enumerated by
    /// `rollback_exit_oracle::every_presence_only_probe_is_named_with_its_reason`.
    fn declare_rollback_derived_component<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Component;

    fn declare_rollback_derived_component_state<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Component + SnapshotState;

    /// Declare derived state that is a RESOURCE, and register a presence probe.
    ///
    /// Same contract and same reason as
    /// [`Self::declare_rollback_derived_component`]. Split only because
    /// `declare_rollback_derived` bounds `T: 'static` and a probe needs to know
    /// whether to look in the component store or the resource store.
    fn declare_rollback_derived_resource<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Resource;

    fn declare_rollback_derived_resource_state<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Resource + SnapshotState;

    fn declare_dynamic_anchor<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
    ) -> &mut Self
    where
        T: 'static;
}

impl AmbitionRollbackApp for App {
    fn rollback_component_canonical<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + SnapshotState,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentCanonical,
                "bevy_ggrs canonical codec snapshot + identical canonical checksum projection",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            self.add_plugins(ComponentSnapshotPlugin::<CanonicalCodecStrategy<T>>::default());
            RollbackApp::checksum_component(self, state_checksum::<T>);
            record_probe(
                self,
                crate::rollback::ChecksumProbe::new(
                    std::any::type_name::<T>(),
                    crate::rollback::census_state::<T>,
                ),
            );
        }
        self
    }

    fn rollback_component_cursor<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotCursor,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentCloneCursor,
                "bevy_ggrs clone snapshot + canonical mutable-cursor checksum projection",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            RollbackApp::checksum_component(self, cursor_checksum::<T>);
            record_probe(
                self,
                crate::rollback::ChecksumProbe::new(
                    std::any::type_name::<T>(),
                    crate::rollback::census_cursor::<T>,
                ),
            );
        }
        self
    }

    fn rollback_component_resolved<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotResolve,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentCloneResolved,
                "bevy_ggrs clone snapshot + canonical authored-reference checksum projection",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            RollbackApp::checksum_component(self, resolved_checksum::<T>);
            record_probe(
                self,
                crate::rollback::ChecksumProbe::new(
                    std::any::type_name::<T>(),
                    crate::rollback::census_resolved::<T>,
                ),
            );
        }
        self
    }

    fn rollback_component_clone<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentClone,
                "bevy_ggrs clone snapshot; state checksum supplied by another authoritative projection",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            // PRESENCE only, because this arm's contract is "snapshotted here,
            // value checksummed by some other authoritative projection" — there is
            // no projection to measure. A count still catches a carrier that
            // bevy_ggrs did not put back, which is `PlayerVisual`'s exact failure.
            //
            // It is genuinely weaker, and G2 made that weakness enumerable rather
            // than implied: a presence probe satisfies the F3 coverage test, which
            // compares type NAMES, while saying nothing about the value. If the type
            // has any stable projection at all — including an entity reference's
            // target identity — reach for `rollback_component_clone_entity_ref` instead.
            record_probe(
                self,
                crate::rollback::ChecksumProbe::presence_for::<T>(
                    std::any::type_name::<T>(),
                    crate::rollback::census_presence::<T>,
                ),
            );
        }
        self
    }

    fn rollback_component_clone_entity_ref<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> bevy::prelude::Entity,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentClone,
                "bevy_ggrs clone snapshot; entity handle remapped, probed through the target's stable sim identity",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            // No GGRS checksum, for the same reason as the plain clone arm: the raw
            // handle differs across a load by design. But the TARGET's identity does
            // not, so localization is not stuck at presence.
            record_probe(
                self,
                crate::rollback::ChecksumProbe::new(std::any::type_name::<T>(), move |world| {
                    crate::rollback::census_entity_reference::<T>(world, referenced)
                }),
            );
        }
        self
    }

    fn rollback_component_clone_entity_set<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<bevy::prelude::Entity>,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentClone,
                "bevy_ggrs clone snapshot; entity SET remapped, probed through the targets' stable sim identities",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            record_probe(
                self,
                crate::rollback::ChecksumProbe::new(std::any::type_name::<T>(), move |world| {
                    crate::rollback::census_entity_set::<T>(world, referenced)
                }),
            );
        }
        self
    }

    fn rollback_component_clone_entity_map<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<(u64, bevy::prelude::Entity)>,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentClone,
                "bevy_ggrs clone snapshot; keyed entity MAP remapped, probed with each key folded against its target's stable sim identity",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            record_probe(
                self,
                crate::rollback::ChecksumProbe::new(std::any::type_name::<T>(), move |world| {
                    crate::rollback::census_entity_map::<T>(world, referenced)
                }),
            );
        }
        self
    }

    fn rollback_component_clone_probed<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        projection: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentClone,
                "bevy_ggrs clone snapshot; value-probed for localization, not in the session checksum",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            record_probe(
                self,
                crate::rollback::ChecksumProbe::new(std::any::type_name::<T>(), move |world| {
                    crate::rollback::census_with::<T>(world, projection)
                }),
            );
        }
        self
    }

    fn rollback_component_clone_state<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotState,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentCloneCanonicalChecksum,
                "bevy_ggrs clone snapshot + canonical checksum; exact Entity/reference values are remapped after load",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            RollbackApp::checksum_component(self, state_checksum::<T>);
            record_probe(
                self,
                crate::rollback::ChecksumProbe::new(
                    std::any::type_name::<T>(),
                    crate::rollback::census_state::<T>,
                ),
            );
        }
        self
    }

    fn rollback_component_clone_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
        checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentCloneCustomChecksum,
                detail,
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            RollbackApp::checksum_component(self, checksum);
            // The SAME projection GGRS was just handed, so the probe measures
            // exactly what the session's aggregate measures.
            record_probe(
                self,
                crate::rollback::ChecksumProbe::new(std::any::type_name::<T>(), move |world| {
                    crate::rollback::census_with::<T>(world, checksum)
                }),
            );
        }
        self
    }

    fn rollback_resource_canonical<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Resource + SnapshotState,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceCanonical,
                "bevy_ggrs canonical codec snapshot + identical canonical checksum projection",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            self.add_plugins(ResourceSnapshotPlugin::<CanonicalCodecStrategy<T>>::default());
            RollbackApp::checksum_resource(self, state_checksum::<T>);
            record_probe(
                self,
                crate::rollback::ChecksumProbe::new(
                    std::any::type_name::<T>(),
                    crate::rollback::census_resource_state::<T>,
                ),
            );
        }
        self
    }

    fn rollback_resource_optional_canonical<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Resource + SnapshotState,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceCanonical,
                "bevy_ggrs canonical codec snapshot + presence-aware canonical checksum projection",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            self.add_plugins(ResourceSnapshotPlugin::<CanonicalCodecStrategy<T>>::default());
            // Ambition's own checksum system rather than
            // `RollbackApp::checksum_resource`, which installs the `Res<T>` one.
            // Absent hashes to a fixed sentinel; present hashes the canonical
            // encoding — so the ABSENT→PRESENT edge, which is the whole content
            // of an activation latch, moves the aggregate checksum.
            let update = move |mut commands: Commands,
                               resource: Option<Res<T>>,
                               mut checksum: Query<
                &mut bevy_ggrs::ChecksumPart,
                (
                    Without<bevy_ggrs::RollbackId>,
                    With<bevy_ggrs::ChecksumFlag<T>>,
                ),
            >| {
                const ABSENT: u128 = 0x4142_5345_4E54_u128;
                let part = bevy_ggrs::ChecksumPart(
                    resource.map_or(ABSENT, |value| state_checksum(value.as_ref()) as u128),
                );
                if let Ok(mut existing) = checksum.single_mut() {
                    *existing = part;
                } else {
                    commands.spawn((part, bevy_ggrs::ChecksumFlag::<T>::default()));
                }
            };
            self.add_systems(
                bevy_ggrs::SaveWorld,
                update.in_set(bevy_ggrs::SaveWorldSystems::Checksum),
            );
            record_probe(
                self,
                crate::rollback::ChecksumProbe::new(
                    std::any::type_name::<T>(),
                    crate::rollback::census_resource_state::<T>,
                ),
            );
        }
        self
    }

    fn rollback_resource_cursor<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Resource + Clone + SnapshotCursor,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceCloneCursor,
                "bevy_ggrs clone snapshot + canonical mutable-cursor checksum projection",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_resource_with_clone::<T>(self);
            RollbackApp::checksum_resource(self, cursor_checksum::<T>);
            record_probe(
                self,
                crate::rollback::ChecksumProbe::new(
                    std::any::type_name::<T>(),
                    crate::rollback::census_resource_cursor::<T>,
                ),
            );
        }
        self
    }

    fn rollback_resource_clone<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Resource + Clone,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceClone,
                "bevy_ggrs clone snapshot; state checksum supplied by another authoritative projection",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_resource_with_clone::<T>(self);
            // Presence only, for the same reason as the component arm: no
            // projection was supplied. 0-or-1 distinguishes "absent after a load"
            // from "present", and nothing else — for a singleton resource that is
            // almost always "present", which is the narrowest a probe gets.
            record_probe(
                self,
                crate::rollback::ChecksumProbe::presence_for::<T>(
                    std::any::type_name::<T>(),
                    crate::rollback::census_resource_presence::<T>,
                ),
            );
        }
        self
    }

    fn rollback_resource_clone_entity_set<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<bevy::prelude::Entity>,
    ) -> &mut Self
    where
        T: Resource + Clone,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceClone,
                "bevy_ggrs clone snapshot; entity SET remapped, probed through the targets' stable sim identities",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_resource_with_clone::<T>(self);
            // No GGRS checksum: the raw handles differ across a load by design.
            // The TARGETS' identities do not, so localization is not stuck at
            // presence — which for a singleton resource is very nearly nothing.
            record_probe(
                self,
                crate::rollback::ChecksumProbe::new(std::any::type_name::<T>(), move |world| {
                    crate::rollback::census_resource_entity_set::<T>(world, referenced)
                }),
            );
        }
        self
    }

    fn rollback_resource_clone_entity_set_probed<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<bevy::prelude::Entity>,
        facts: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: Resource + Clone,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceClone,
                "bevy_ggrs clone snapshot; entity SET remapped and probed through the targets' stable sim identities, mixed with a projection of the value's non-entity fields",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_resource_with_clone::<T>(self);
            // No GGRS checksum, for the same reason as the plain entity-set arm:
            // the raw handles differ across a load by design. The localization
            // probe carries both halves.
            record_probe(
                self,
                crate::rollback::ChecksumProbe::new(std::any::type_name::<T>(), move |world| {
                    let mut census =
                        crate::rollback::census_resource_entity_set::<T>(world, referenced);
                    if let Some(value) = world.get_resource::<T>() {
                        census.xor = census.xor.wrapping_add(facts(value));
                    }
                    census
                }),
            );
        }
        self
    }

    fn rollback_resource_clone_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
        checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: Resource + Clone,
    {
        install_resource_clone_checksum::<T>(self, owner, name, detail.to_string(), checksum);
        self
    }

    fn rollback_map_entities<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + bevy::ecs::entity::MapEntities,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::EntityMapping,
                "bevy_ggrs LoadWorld entity-reference remapping",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::update_component_with_map_entities::<T>(self);
        }
        self
    }

    fn rollback_resource_map_entities<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Resource + bevy::ecs::entity::MapEntities,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceEntityMapping,
                "bevy_ggrs LoadWorld resource entity-reference remapping",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::update_resource_with_map_entities::<T>(self);
        }
        self
    }

    fn require_rollback<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::RequiredRollback,
                "component presence automatically installs bevy_ggrs::Rollback",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::require_rollback::<T>(self);
        }
        self
    }

    fn clear_message_on_rollback<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Message,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::MessageClear,
                "clear abandoned-future message buffer in LoadWorld::Mapping",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            self.add_systems(
                LoadWorld,
                clear_message_channel::<T>.in_set(LoadWorldSystems::Mapping),
            );
        }
        self
    }

    fn declare_rollback_derived_resource<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Resource,
    {
        self.declare_rollback_derived::<T>(owner, name, reason);
        // PRESENCE, and for a singleton resource that means 0-or-1: it can catch a
        // derived resource that was never rebuilt, and cannot catch one rebuilt
        // WRONG. `declare_rollback_derived_resource_state` is the value-sensitive
        // twin, and gameplay-significant derived state should use it (G2).
        record_probe(
            self,
            crate::rollback::ChecksumProbe::derived_for::<T>(
                std::any::type_name::<T>(),
                crate::rollback::census_resource_presence::<T>,
            ),
        );
        self
    }

    /// Declare derived state that HAS a canonical projection, so the resimulation
    /// comparison sees a wrongly-rebuilt value and not merely a missing one.
    fn declare_rollback_derived_resource_state<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Resource + SnapshotState,
    {
        self.declare_rollback_derived::<T>(owner, name, reason);
        record_probe(
            self,
            crate::rollback::ChecksumProbe::derived_value(
                std::any::type_name::<T>(),
                crate::rollback::census_resource_state::<T>,
            ),
        );
        self
    }

    fn declare_rollback_derived_component<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Component,
    {
        self.declare_rollback_derived::<T>(owner, name, reason);
        // PRESENCE: catches a derived component nobody rebuilt (which is what
        // `ProjectileOwner`'s broken derived promise actually was), and cannot catch
        // a motion sample rebuilt with entirely wrong values on the right number of
        // entities. `declare_rollback_derived_component_state` is the strong twin.
        record_probe(
            self,
            crate::rollback::ChecksumProbe::derived_for::<T>(
                std::any::type_name::<T>(),
                crate::rollback::census_presence::<T>,
            ),
        );
        self
    }

    /// Declare a derived COMPONENT that has a canonical projection.
    fn declare_rollback_derived_component_state<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Component + SnapshotState,
    {
        self.declare_rollback_derived::<T>(owner, name, reason);
        record_probe(
            self,
            crate::rollback::ChecksumProbe::derived_value(
                std::any::type_name::<T>(),
                crate::rollback::census_state::<T>,
            ),
        );
        self
    }

    fn declare_rollback_derived<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: 'static,
    {
        register_app_descriptor(
            self,
            descriptor::<T>(owner, name, RollbackEntryKind::Derived, reason),
        );
        self
    }

    fn declare_dynamic_anchor<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
    ) -> &mut Self
    where
        T: 'static,
    {
        register_app_descriptor(
            self,
            descriptor::<T>(owner, name, RollbackEntryKind::DynamicAnchor, detail),
        );
        self
    }
}

fn clear_message_channel<T: Message>(messages: Option<ResMut<Messages<T>>>) {
    if let Some(mut messages) = messages {
        messages.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, owner: &str, detail: &str) -> RollbackRegistrationDescriptor {
        RollbackRegistrationDescriptor {
            name: name.to_owned(),
            owner: owner.to_owned(),
            kind: RollbackEntryKind::Derived,
            type_name: "test::Type".to_owned(),
            detail: detail.to_owned(),
        }
    }

    #[test]
    fn schema_is_insertion_order_independent() {
        let mut a = RollbackRegistry::default();
        a.try_register(entry("z", "provider-b", "second")).unwrap();
        a.try_register(entry("a", "provider-a", "first")).unwrap();

        let mut b = RollbackRegistry::default();
        b.try_register(entry("a", "provider-a", "first")).unwrap();
        b.try_register(entry("z", "provider-b", "second")).unwrap();

        assert_eq!(a.deterministic_dump(), b.deterministic_dump());
        assert_eq!(a.schema_fingerprint(), b.schema_fingerprint());
    }

    #[test]
    fn identical_registration_is_idempotent() {
        let descriptor = entry("same", "provider", "same");
        let mut registry = RollbackRegistry::default();
        assert_eq!(
            registry.try_register(descriptor.clone()).unwrap(),
            RollbackRegistrationOutcome::Inserted
        );
        assert_eq!(
            registry.try_register(descriptor).unwrap(),
            RollbackRegistrationOutcome::Idempotent
        );
        assert_eq!(registry.descriptors().count(), 1);
    }

    #[test]
    fn non_ggrs_hosts_record_schema_without_installing_runtime_machinery() {
        let mut app = App::new();
        app.insert_resource(SimulationHost::Fixed60Hz);
        app.add_plugins(crate::rollback::AmbitionRollbackSchemaPlugin);

        let registry = app.world().resource::<RollbackRegistry>();
        assert!(
            registry.descriptors().count() > 1,
            "the host-independent schema plugin records the engine contract"
        );
        assert!(
            !app.world()
                .contains_resource::<bevy_ggrs::RollbackFrameRate>(),
            "a fixed-tick game must not install GGRS runtime resources"
        );
    }

    #[test]
    fn ggrs_host_records_the_same_registration_vocabulary() {
        let mut app = App::new();
        app.insert_resource(SimulationHost::Ggrs);
        app.declare_rollback_derived::<u32>("test", "derived", "test-only descriptor");

        let registry = app.world().resource::<RollbackRegistry>();
        assert_eq!(registry.descriptors().count(), 1);
    }

    fn typed_entry(name: &str, type_name: &str) -> RollbackRegistrationDescriptor {
        RollbackRegistrationDescriptor {
            name: name.to_owned(),
            owner: "test-owner".to_owned(),
            kind: RollbackEntryKind::Derived,
            type_name: type_name.to_owned(),
            detail: "test-only descriptor".to_owned(),
        }
    }

    fn registry_of(rows: &[(&str, &str)]) -> RollbackRegistry {
        let mut registry = RollbackRegistry::default();
        for (name, type_name) in rows {
            registry.try_register(typed_entry(name, type_name)).unwrap();
        }
        registry
    }

    /// **Where a type LIVES is not part of the wire format** (v20).
    ///
    /// The two rows are the ones D33 step 2 actually moved (`24b43f93a`), and
    /// they are here rather than an invented pair because they refute the
    /// narrower answer: the crate changed AND the module path below it changed,
    /// because a carve puts a type where it belongs rather than merely
    /// somewhere else. Only the final segment survived either move.
    #[test]
    fn relocating_a_type_leaves_the_fingerprint_alone() {
        let before = registry_of(&[
            (
                "actor.anim_override",
                "ambition_platformer2d_actor_monolith::features::ecs::actor_clusters::ActorAnimOverride",
            ),
            (
                "player.blink_camera_state",
                "ambition_platformer2d_actor_monolith::avatar::components::PlayerBlinkCameraState",
            ),
        ]);
        let after = registry_of(&[
            (
                "actor.anim_override",
                "ambition_sprite_sheet::character::anim::ActorAnimOverride",
            ),
            (
                "player.blink_camera_state",
                "ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState",
            ),
        ]);
        assert_eq!(
            before.schema_fingerprint(),
            after.schema_fingerprint(),
            "moving a rollback-registered type to another crate and another \
             module moved the schema fingerprint. Nothing a peer can observe \
             changed, so two peers running byte-identical snapshot logic would \
             refuse to agree — which makes every carve in the decomposition \
             campaign a netplay compatibility break."
        );

        // POISON. Without it this test is equally green for a fingerprint that
        // hashes nothing about the type at all, and dropping `type_name` from
        // the dump entirely was a real alternative — it costs the last signal
        // that a DIFFERENT Rust type got registered under an existing name.
        let renamed = registry_of(&[
            (
                "actor.anim_override",
                "ambition_sprite_sheet::character::anim::ActorAnimOverride",
            ),
            (
                "player.blink_camera_state",
                "ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkEaseState",
            ),
        ]);
        assert_ne!(
            after.schema_fingerprint(),
            renamed.schema_fingerprint(),
            "a stable name that changed which TYPE it registers left the \
             fingerprint alone, so the dump is no longer hashing the type in \
             any form."
        );
    }

    /// **What makes the narrower identity sound.**
    ///
    /// Two `Cooldown`s in two crates hash equal once the final segment is the
    /// identity, so a peer holding them the other way round would be declared
    /// compatible. The second half asserts the guard is not merely strict: one
    /// type registered under two stable names is the ordinary case, and 39 of
    /// the live rows are it.
    #[test]
    fn two_types_sharing_a_final_segment_are_rejected_and_one_type_twice_is_not() {
        let mut registry = RollbackRegistry::default();
        registry
            .try_register(typed_entry(
                "ability.cooldown",
                "ambition_combat::ability::Cooldown",
            ))
            .unwrap();

        let error = registry
            .try_register(typed_entry(
                "weapon.cooldown",
                "ambition_projectiles::weapon::Cooldown",
            ))
            .unwrap_err();
        assert!(
            matches!(
                error,
                RollbackRegistrationError::TypeIdentityCollision { .. }
            ),
            "two different types whose names end in `Cooldown` were accepted, \
             and the fingerprint cannot tell them apart: {error}"
        );

        registry
            .try_register(typed_entry(
                "ability.cooldown_mirror",
                "ambition_combat::ability::Cooldown",
            ))
            .expect(
                "registering ONE type under a second stable name is not a \
                 collision — the stable name is what identifies a registration, \
                 and refusing this would reject 39 of the live rows",
            );
    }

    #[test]
    fn conflicting_registration_is_transactional() {
        let mut registry = RollbackRegistry::default();
        registry
            .try_register(entry("same", "provider-a", "old"))
            .unwrap();
        let before = registry.deterministic_dump();
        let error = registry
            .try_register(entry("same", "provider-b", "new"))
            .unwrap_err();
        assert!(matches!(error, RollbackRegistrationError::Conflict { .. }));
        assert_eq!(registry.deterministic_dump(), before);
    }
}
