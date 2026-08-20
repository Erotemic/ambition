//! Backend-neutral rollback schema metadata.
//!
//! Gameplay domains declare typed rollback obligations through
//! `ambition_platformer2d_core::snapshot::RollbackRegistrar`. This module records
//! the exact managed schema those declarations describe; concrete rollback hosts
//! install storage/checksum machinery separately. Keeping the catalog here makes
//! prepared-content identity available to fixed/render-frame hosts without linking
//! a netcode backend.

use std::collections::BTreeMap;
use std::fmt;

use bevy::prelude::*;

use crate::content_identity::SnapshotSchemaFingerprint;

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
/// ⚠ **v35 (2026-08-17): `resource.stocks_match_settled` CHANGED VALUE UNDER AN
/// UNCHANGED KEY, which is the case this counter exists for.** D147 moved the
/// stocks verdict out of `ambition_combat::stocks` and into
/// `features::stocks_match`, and on the way it stopped being a timeless boolean
/// and became *the outcome for match X* — it now carries the `MatchInstance`
/// (session plus the tick the cast was built on) that the receipt beside it
/// publishes.
///
/// ⭐ **the row KEY did not move, and that is exactly what makes the bump
/// necessary rather than optional.** v34 above records the mirror-image case: a
/// rename where the projection was unchanged. Here the spelling two peers
/// negotiate is identical and the bytes behind it are not, so nothing else in
/// the handshake could tell an old peer from a new one — they would agree to
/// disagree about the same name.
///
/// ⚠ caught by `scripts/rollback_codec_shape.py`, not by the schema baseline:
/// the baseline watches the registered SET and the key was still there. **A
/// payload change under a stable key is invisible to it by construction**, which
/// is why the codec-shape tracker is a separate instrument.
/// ⚠ **v37 (2026-08-18): TWO codecs each gained one encoded `bool`** as the
/// capture verb entered the vocabulary — `AbilitySet::grab` and
/// `ActorControlFrame::grab_pressed`. Both are ordinary field additions, so an
/// old peer and a new one disagree about every snapshot from the first frame.
///
/// ⛔⛔ **and only ONE of the two was caught, which is the part worth recording.**
/// `AbilitySet` encodes with a `put_bool` per field, so its primitive sequence
/// moved and `rollback_codec_shape.py` said so. `ActorControlFrame` encodes its
/// flags through `for b in [f.a, f.b, …] { put_bool(out, b); }` — ONE primitive
/// call however long the array is — so it went from 19 flags to 20 with the
/// file's hash unmoved at `b54dbef425527998` on both sides. The wire changed and
/// the instrument that exists to notice could not.
///
/// ⭐ the checker now folds the array's ELEMENT COUNT into its hash, the same
/// patch `snapshot_pod!` had already been given in that file for the identical
/// reason — one construct had been noticed and the other had not. Re-recording
/// it also moved `ambition_cutscene` and `ambition_demo_twintrack`, which is the
/// instrument CHANGING and NOT a wire change in either: neither file was edited.
///
/// ⚠ **v54 (2026-08-20) is the GAIT: `AxisManeuverState::running` plus
/// `AxisLocomotion::run_commit_frac`** — one bool in the maneuver run and one
/// float in the params run, so both halves of the motion codec move.
/// ⛔ **it exists because the dash attack was keyed to the wrong state.** The
/// selector asked `BodyMotionFacts::dashing`, which is the TRAVERSAL dash's
/// timer, and `SMASH_FIGHTER_KIT` switches `AbilitySet::dash` off on purpose —
/// so the move was unreachable in the only game that authors it, and every test
/// passed by telling the selector it was dashing.
/// ⚠ **`running` is derived every tick and is still serialized**, on the same
/// terms as `gliding` and `fast_falling` beside it: a restore that lands
/// mid-run must not present a standing body for the tick before the next
/// integration rewrites the fact.
/// ⚠ **v53 IS DELIBERATELY SKIPPED, and it is a HOLE rather than a reservation.**
/// It was offered to the lane working `ControlAuthority` on main so two branches
/// could not both take one number; that lane then found it needed no bump at all
/// (`DrivingParticipant` is a per-tick DERIVE — no registry entry, no snapshot,
/// no wire) and handed it back. Renumbering down would have churned two recorded
/// baselines to save a number nobody needs. ⭐ a hole in a monotonic log costs
/// nothing; a collision costs a wire format.
/// ⚠ **v52 (2026-08-20) is `MovementTuning::parry_timing`**, one DISCRIMINANT in
/// the motion codec, put and read as a hand-written `put_u8`.
/// ⭐⭐ **the first knob shipped under Jon's smash-LIKE ruling** (2026-08-20):
/// *"It would be nice if there was a set of knobs we could tune to reproduce
/// ultimate"* and *"if ultimate does it I do want a setting for get ultimate, so
/// release style shielding is in scope as an option."* Smash 4 opens the
/// perfect-shield window on the PRESS and Ultimate on the RELEASE; both are now
/// settings a stage declares, and `OnRaise` is the default so no shipped body's
/// feel moved.
/// ⛔ `BodyShieldState::parrying()` lost its `active &&` term to make the second
/// setting reachable at all — a release-timed window is live while the guard is
/// DOWN. The term is not lost, only moved: `resolve_shield` is the one place the
/// timer is armed, and only a guard that was UP can be released.
/// ⚠ **a hand-written `put_u8` of a discriminant is NOT what
/// `snapshot_unit_enum!` folds**, so `rollback_codec_shape.py` sees this as one
/// more primitive and nothing about the variant set. Adding a THIRD timing later
/// would move no token — the claim lives here.
/// ⚠ **v51 (2026-08-20) is the SPOT DODGE**, and it moves THREE things:
///
/// ```text
/// MovementOp::SpotDodge = 35                       a new wire CODE
/// MovementTuning::spot_dodge_time                  one float, motion codec
/// AxisManeuverState::spot_dodging                  one bool, motion codec
/// ```
///
/// ⭐ **one timer, two verbs.** The window rides `dodge_roll_timer` because the
/// i-frames are the same term the damage rule reads either way; splitting the
/// TIMER would have made `evading()` a two-place question for no gain. What
/// differs is only what it is DRAWN as, and `spot_dodging` is that fact.
/// ⚠ and the row it asks for — `spot_dodge` — was already in the shipped sheets.
/// Fourth time this branch has found art nothing was asking for.
/// ⚠ **v50 (2026-08-20) is `MovementTuning::sdi_step`**, one float in the
/// motion codec, put and read. SMASH DIRECTIONAL INFLUENCE: how far a body may
/// shift ITSELF per tick of hitlag.
/// ⭐ **the defensive half of a mechanic whose offensive half already shipped.**
/// DI bends the launch a fighter is about to take; SDI moves it out of the NEXT
/// hit's way while the current one is still frozen, which is what makes hitlag a
/// WINDOW rather than merely a pause. It needed no new state — the shift is
/// written straight to `pos` on the tick `step_body` is about to zero `dt`.
/// ⚠ ours rewards the HOLD where the genre counts fresh stick inputs; an
/// edge-counting version would need per-window state inside the rollback window,
/// and the total is bounded either way by the hitlag's own length.
/// ⚠ **v49 (2026-08-20) is `AxisManeuverState::time_off_ledge`**, one f32 in
/// the motion codec, put and read. It is what a ledge grab's intangibility is
/// now BOUGHT with: the window used to be a flat 0.50s on every grab, so the
/// edge was a free reset a fighter could hold forever.
/// ⭐ **airtime, not a regrab counter** — the genre buys ledge intangibility
/// with time spent off the edge, and a counter would punish a fighter who was
/// knocked away and recovered exactly as hard as one stalling on the ledge.
/// ⚠ it starts FULL rather than zero: a body that has never touched a ledge has
/// been off one forever, and a zero would hand every first grab the floor.
/// ⚠ **v48 (2026-08-20) is `SmashHoldState::escape_seconds`**, one f32 on a
/// `snapshot_pod!` component, so the pod body folds it by name and the row
/// widens. ⚠ `escape_progress` is RENAMED to `mash_credit` in the same commit;
/// the rename moves no bytes, and the codec's one extra primitive is the new
/// field alone.
/// ⭐ **the field exists because the hold's length stopped being a constant.** A
/// grab now holds a body for `90 + 1.7p` frames of ITS OWN damage, read once
/// when the hold begins — Ultimate's rule, and the reason it is stored rather
/// than recomputed is that a hold which re-read the percent every tick would
/// grow every time its captor pummelled.
/// ⚠ **the codec-shape guard reddened on the RENAME too, and that is a known
/// false positive of its `snapshot_pod!` fold**: it hashes the macro body
/// verbatim, field names included, where the array fold deliberately takes *"the
/// COUNT, never the names"*. Harmless here — the new field is a real wire change
/// and the bump is owed anyway — but a commit that ONLY renamed a pod field
/// would redden it for nothing.
/// ⚠ **v47 (2026-08-20) is `FootstoolTuning::air_tumble_time` and
/// `stomper_invuln`**, two floats in the motion codec, put and read
/// (338 -> 342 primitives). The first splits the
/// victim's reaction in two: a grounded victim flinches, an airborne one
/// tumbles, and a tumble's length is AUTHORED rather than derived from the
/// shove — a footstool produces no real knockback, so feeding its 220 px/s to
/// the launch threshold would tumble nobody. The second is the stomper's four
/// frames of intangibility, which is what makes a footstool an escape from
/// disadvantage. ⚠ `victim_stun` was RENAMED to `flinch_time` in the same
/// commit; a rename moves no bytes, and the codec's four extra primitives are
/// the two new fields alone.
/// ⚠ **v46 (2026-08-20) is `BodyStaleMoves`**, a NEW rollback component
/// (`body.stale_moves`): nine `u32` slots and a cursor, remembering what this
/// body last LANDED so a repeated move is worth less. Both a new stable key and
/// a new payload behind it.
/// ⭐ **hashes rather than move ids, and that is what makes it snapshot state at
/// all.** A `Vec<String>` of names would allocate per body per save; a `[u32; 9]`
/// is copied like any other pod. Nothing reads the hash back as a name, so a
/// collision costs one move a staleness it did not earn and nothing else.
/// ⚠ it is hand-written rather than `snapshot_pod!` because that macro maps a
/// field to a READER METHOD and there is none for an array — and the explicit
/// `[0u32; 9]` on the decode side is what `rollback_codec_shape.py` reads as the
/// width. ⛔ that regex did not accept `0u32` until this commit widened it, which
/// would have made the ring's WIDTH invisible to the checker: the same
/// no-primitive-call hole as `snapshot_pod!`, the array loop, and
/// `snapshot_unit_enum!` before it. Fourth instance.
/// ⚠ **v45 (2026-08-20) is `ShieldTuning::min_coverage`**, the last of the
/// guard's numbers: how much of the body a SPENT shield still covers. One float
/// in the motion codec beside the other seven.
/// ⚠ **v44 (2026-08-20) is `ShieldTuning::pushback_per_damage`**, one float in
/// the motion codec, put and read. The guard's third cost — a block moves the
/// body behind it — and the tuning rides the axis-swept projection like the
/// other six.
/// ⚠ **v43 (2026-08-20) is `BodyJumpState::footstool_claimed`**, one bool on a
/// `snapshot_pod!` component, so the pod body folds it by name and the row moves.
/// ⭐ **it exists because v42's footstool was arbitrated in the wrong place.** The
/// bounce was applied AFTER the kernel by overwriting velocity, on the argument
/// that a later write wins; the kernel had already spent an air jump and emitted
/// `MovementOp::DoubleJump` by then, so the same footstool cost a charge when
/// you had one and nothing when you did not. The claim is read by the jump chain
/// AHEAD of the air jump, which makes one input edge mean one thing.
/// ⚠ **v42 and v43 arrive TOGETHER or not at all.** Main was at v41 when the
/// footstool was held back from a merge, and v42's whole content is the
/// footstool's floats and its op code — so a reader on main who sees v41 next to
/// a v43 entry is not looking at a lost version, they are looking at one feature
/// that bumped twice on its way in.
/// ⚠ **v42 (2026-08-20) is the FOOTSTOOL's wire, and ONLY that.** Two things
/// move, and they are measured differently:
///
/// ```text
/// core/motion_codec.rs   326 -> 334   +8   four FootstoolTuning floats, put AND read
/// MovementOp::Footstool = 34               a new wire CODE
/// ```
///
/// ⛔ **an earlier draft of this entry also claimed `stun_per_damage` and
/// `BodyShieldState`'s three fields. Both are v41's** — they were already on
/// main when v41 was recorded, and v41's corrected entry below now names them.
/// Two hand-written entries, two corrections in one hour, both understating the
/// same way: ⇒ **the recorded baseline is MEASURED and this log is PROSE, so the
/// prose is the side that drifts.** When they disagree, trust the baseline.
///
/// ⭐ **and `Footstool = 34` is the first op code any instrument has ever
/// caught.** `snapshot_unit_enum!` was invisible to
/// `scripts/rollback_codec_shape.py`: a variant list is bare idents with no
/// primitive call, so `core/snapshot_impls.rs` hashed IDENTICALLY with the
/// variant added — the third time that file's own docs describe this hole, after
/// `snapshot_pod!` and the array-driven codec. The fold now takes the
/// discriminants SORTED, so a rename and a reorder are not wire changes and an
/// added or renumbered code is.
/// ⛔⛔ **v41's ENTRY BELOW WAS INCOMPLETE, AND THE DECISION WAS STILL RIGHT.**
/// Corrected 2026-08-20 the same day, after a peer lane found the rest. The bump
/// was correct and `--record` captured the whole tree, so the BASELINE never
/// lied; what was short is the prose, which is the human-readable contract for
/// what a version MEANS. THREE codec files moved at v41, not one — measured by
/// diffing the recorded baseline across the commit rather than by re-reading the
/// diff that prompted it:
///
/// ```text
/// ambition_characters/snapshot_impls.rs        240 prims (unchanged)  hash moved   the taunt bool
/// ambition_platformer2d_core/motion_codec.rs   314 -> 326 prims       +12          ShieldTuning's SIX fields, put and read
/// ambition_platformer2d_core/snapshot_impls.rs 102 prims (unchanged)  hash moved   BodyShieldState + depleted/break_timer/stun_timer
/// plus `MovementOp::ShieldBreak = 33`, a new wire CODE
/// ```
///
/// ⭐ **the lesson is about the instrument, not the miss.** `pytest
/// scripts/tests/` truncates its diff — it printed *"At index 1 diff"* and one
/// row, and I described the tree from that one row. ⇒ **when a recorded baseline
/// reddens, diff the RECORDED FILE across the commit and enumerate every moved
/// row**; the test's message names an example, not the change. Two of the three
/// rows above have an UNCHANGED primitive count and a moved hash, which is
/// exactly the array/struct-folded case the checker was taught to catch and the
/// case a reader skimming counts would call unmoved.
///
/// ⚠ **`MovementOp` codes are the least wire-looking wire there is.** A
/// `BodyComboTrace` carrying op 33 is undecodable to a v40 peer, and nothing
/// about the name says "wire".
///
/// ⚠ **v41 (2026-08-20) is ONE BIT: `ActorControlFrame::taunt_pressed`.** The
/// taunt verb travels the road a grab travels, and its codec edge exists for the
/// same reason the grab edge does — a resimulated tick that lost the press did
/// not taunt, and the two histories diverge. `ActorControl`'s bool run widens by
/// one, so the bytes a peer encodes change and this bumps.
/// ⛔ **it was missed by the lane that added it and caught by
/// `scripts/rollback_codec_shape.py`, not by any Rust test.** The app-level
/// `rollback_schema_baseline` stayed GREEN through it: that baseline records
/// stable KEYS and this change altered a PAYLOAD behind an unchanged key, which
/// is exactly the split the three baselines exist for. ⇒ a wire change is not
/// established by the app suite; run the repo tooling tests before believing a
/// codec is unmoved.
/// ⚠ **v40 (2026-08-19) puts the player's ENTITLEMENTS inside the checkpoint
/// horizon.** `resource.owned_items_baseline` joins the three baselines a commit
/// already writes, with an entity-free stored-quantity checksum. ⭐ it is the
/// half that made D132's gate openable: a mint may now SPEND the quantity it
/// came from — one granted javelin manifested two objects until it did — and
/// spending is only safe because a death can put the row back. Either half alone
/// is a bug in one direction (a phantom) or the other (annihilation).
/// ⚠ **v39 (2026-08-19) SPLITS a capture into its RELATION and this ruleset's
/// POLICY.** `pummels_landed`, `held_for` and `escape_progress` left
/// `ambition_combat::capture::CapturedBy` for
/// `ambition_characters::smash_capture::SmashHoldState`, which registers as
/// `smash.hold_state` under the characters domain. This IS a payload change and
/// so it bumps: a snapshot now carries a second component for a held body, and
/// the relation's own encoding shrank by the three fields it gave up.
/// ⭐ the reason is ownership, not tidiness (2026-08-19 GPT review): *"A
/// radically different game may quite reasonably want `actor A constrains actor
/// B` without any concept of pummels, mash escape, a four-second grab timeout,
/// or a standing-grab grounded rule."* A capture in such a game now pays to
/// rewind the relation and nothing else.
/// ⚠ **2026-08-19 FINISHES the slot-owned control migration, with NO version
/// bump.** It removes only `derived.player_input_frame`: the entity-local frame
/// was a declared-derived copy of `derived.slot_controls`, not snapshot payload.
/// `Brain::Player(slot)` now reads the slot snapshot directly and body mechanics
/// consume `ActorControl`. Removing a stable descriptor changes the schema
/// fingerprint by itself; v38 remains correct because no payload changed behind
/// an unchanged stable key.
/// ⚠ **v38 (2026-08-18) FINISHES the projectile spawn-road migration.** It
/// prunes `resource.enemy_projectile_state`, `projectile.player_marker`,
/// `projectile.enemy_marker`, and `projectile.owner_id`: the resource was a
/// field-less compatibility handle
/// from the pre-ECS pool, while the two family markers merely remembered which
/// historical spawn road selected the shot's presentation. The string owner id
/// duplicated `ProjectileOwner(Entity)` and had no production reader. The
/// permanent facts are `LiveProjectile` occurrence identity, `ProjectileOwner`,
/// optional `ProjectileKind`, `ProjectileVisualId`, and the actor-domain
/// `ProjectileAllegiance`.
/// `message.spawn_projectile` keeps its stable key while its concrete message
/// becomes `ProjectileSpawnRequest`, so abandoned-future spawn requests remain
/// cleared on load through the same wire identity.
pub const GGRS_ROLLBACK_SCHEMA_VERSION: u32 = 54;

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
    /// That is not hypothetical: Campaign 2 first moved every gameplay
    /// registration into runtime-side domain adapters, and the completed
    /// domain-owned migration then moved those declarations into their owning
    /// crates. Both moves require the schema fingerprint to stay unchanged —
    /// which was impossible while the fingerprint hashed who registered a row. The
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

pub fn descriptor<T: 'static>(
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
pub fn descriptor_owned<T: 'static>(
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

/// Record one schema descriptor on an app, independent of the active rollback backend.
///
/// Backend installation deliberately has a separate idempotence authority: a row may
/// already have been recorded by a capability plugin before a concrete rollback host
/// installs its typed snapshot machinery. Therefore callers must not interpret an
/// `Idempotent` schema row as evidence that a backend registration already exists.
pub fn record_descriptor(
    app: &mut App,
    descriptor: RollbackRegistrationDescriptor,
) -> RollbackRegistrationOutcome {
    app.init_resource::<RollbackRegistry>();
    app.world_mut()
        .resource_mut::<RollbackRegistry>()
        .try_register(descriptor)
        .unwrap_or_else(|error| panic!("{error}"))
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
