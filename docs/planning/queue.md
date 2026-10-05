# The queue — live execution order

This file is the current executable engineering queue. It is not a work log, a
review transcript or an archive. Git history keeps completed investigations.
Durable design and measurements belong in the linked owner document. Product
decisions belong in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

Each row states the current failure, the owner, the next action and the
acceptance. A row stays here only while an engineer can act on it. When a row
closes, delete it in the same change. Keep a receipt of 1 to 5 lines (what was
wrong, what fixed it, the guard, any standing prohibition) only when an open
row, a script or an inbound link depends on it. Receipts are at the end.

Re-measure a row before you implement it. Check a row's owner against who is
actually running before you wait on them. When a row converges, rewrite it
around its result. Do not append to it.

Scripts read this file:

- `scripts/check_blocking_set_names_every_gate.py` reads each `**Blocked by:**`
  field to the next blank line, and each `**Acceptance:**` clause.
- A heading that contains `✅`, `DONE` or `CLOSED` marks its row closed for
  that script, for `scripts/check_discharged_holds_are_rewritten.py` and for the
  two consolidation-ledger checks. The ledger holds work on `ID-PEER` by its
  row id, and a ledger test needs `A10` to stay a closed row.
- Other documents and source comments link to row headings. Do not rename a
  heading without fixing its inbound links (`rg -n "queue.md#" .`).

Measure the per-row mass with:

```sh
awk '/^### /{if(n)printf "%s %s\n", c, n; n=$2; c=0} {c++} END{printf "%s %s\n", c, n}' \
  docs/planning/queue.md | sort -rn | head
```

## P0 — architecture and correctness

### ID-PEER — remove host-local lineage from peer-stable mechanical identity

**Status:** ✅ DONE 2026-10-03. Every road is closed; the standing prohibitions
below still hold.

**Owner:** deterministic identity / rollback architecture. The identity map is
in [`consolidation/architecture-census.md`](consolidation/architecture-census.md).
The input-payload contract is in
[`engine/netcode.md`](engine/netcode.md#the-input-payload-two-peers-exchange),
and schema negotiation is netcode's
[`N3`](engine/netcode.md#n3--contentschema-negotiation).

**Current state:** every road that touches `SessionRoot`, activation or
provenance is closed, and each one has an arm in
`game/ambition_app/tests/id_peer_audit.rs` or `shell_host_lifecycle.rs`. The
closed roads are: the smash roster seed, `SessionScopedEntity` in the checksum,
the match ordinal, the `MatchInstance`-stamped resources, `SessionMatchOrdinal`,
`random_context`, the checkpoint operation keys, the session root's `SimId`
(the constant `SimId::singleton("session", "root")`), `TransactionId`
provenance, `GameplayElapsed`, the startup-resume checksum, perception's
`Entity`-index fallback, and the GGRS carrier order
(`rebase_rollback_carrier_order`). The `ControlFrame` shape is pinned by a
ratchet. The peer-identity checkpoint that C03 and C05 waited on is discharged.
`two_local_histories_compute_the_same_ggrs_component_checksums` and
`the_peer_visible_surface_does_not_record_which_route_the_host_visited_first`
show no row that differs between a fresh host and a veteran one: `SimTick`
agreed from 2026-10-03 (road 2), and `AmbitionGameSave` from the same day
(road 4). Road 3 closed the same day with two peers in one process. Every
road is closed.

**Open roads:**

1. ✅ **CLOSED 2026-10-03: the snapshot schema fingerprint hashes mechanical
   facts, not prose** (`Q122`, schema v303). `compute_schema_fingerprint`
   hashes `RollbackRegistry::mechanical_dump()`: each peer-schema row's name,
   kind, wire type and mechanism token. A token names the road within the kind
   (entity, set or map remapping; the canonical checksum style; probed or
   unhashed). It is declared with the road's sentence in
   `rollback_kind::spelling` and recorded in
   `the_mechanism_tokens_are_recorded`. The prose stays in `schema_dump()` and
   both baselines. A custom-checksum description, a derived row's reason and the
   dynamic anchor's note (87 rows) map to the token `described`. Those 87 rows
   describe the projection code, and the schema version answers for that code,
   not the words. Witness: `rewording_a_row_leaves_the_fingerprint_alone`.
   Control: `a_row_on_another_mechanism_moves_the_fingerprint`.
2. ✅ **CLOSED 2026-10-03: the canonical timeline is session-relative**
   (`Q128` decided by engineering as option (a) and confirmed by the
   maintainer 2026-10-03, with the session-scope
   activation as the agreed start). `SimTick` is a `SessionScopedResources`
   member, so the activation sets it to `0` on every host, whatever the App ran
   before. `advance_sim_tick` lost the `Local` that skipped the first increment:
   a reset could not clear it, and a rewind to the first frame did not restore
   it. Now the first step is tick `1`, and `0` names the moment before it. Two
   process-wide stores held an absolute tick across a session and are scoped
   with it: `ImpactHitstop` (session-scoped), and `NarrativeInputLedger`
   (forgets its records on `SessionScopeActivated`, because a conversation's
   instance id contains the tick it opened on). Host intents already filter by
   scope. Witness: `two_local_histories_compute_the_same_ggrs_component_checksums`
   and `the_peer_visible_surface_does_not_record_which_route_the_host_visited_first`
   no longer excuse `SimTick`. Poison (the tick reset skipped): both name it.
   Why not a rebase at frame zero: room crossings also declare frame zero, and
   stored ticks outlive a crossing. Why not a projection: it would drop the
   timeline. A P2P session (`N2`) must activate its session scope at the agreed
   start, which is the same edge.
3. ✅ **CLOSED 2026-10-03: the unchecksummed float rows** (S7 in
   [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md);
   23 rows by the census script on 2026-10-03).
   ⛔ The state half was a carrier count until 2026-10-03: all eleven sharp
   rows are presence-probed, and a presence census returns `xor: 0`, so
   `two_local_histories_agree_about_the_sharp_unchecksummed_rows` compared how
   many carriers each had. It now strengthens them with a value probe
   (`strengthen_the_sharp_rows`), and the values agree across the two hosts in
   five walks. The timeline half: `game/ambition_app/tests/two_peers.rs` runs
   two GGRS P2P peers in one process over an in-memory link three updates late
   (`start_peer_session`, `loopback_pair`), with value probes on 22 float rows
   (`lifecycle.room_visual` is a unit marker). Both peers agree on every probed
   row at every confirmed frame to 240, with rollbacks, and GGRS reports no
   desync. Poisons: a position changed on one peer is a desync in
   `session_health`; a value outside the checksum is no desync, and only the
   census names it. `two_peers_agree_in_the_rooms_that_carry_the_float_rows`
   walks seven more rooms, and together the walks carry 21 of the 22 rows; the
   22nd (`MountedSize`) has no production writer. A peer-session room crossing
   is netcode's open question, not this road's (the lifecycle commit runs only
   under a local sync test).
4. ✅ **CLOSED 2026-10-03: the save belongs to the experience that plays it**
   (`Q129`, decided the same day: shared durable state is peer state). Measured: the save
   differed in one field, `flags`. The veteran's Sanic and Mary-O sessions had
   written their room-visit flags (`room_visited_<room>`) into the one
   process-wide save, and the autosave wrote them into Ambition's
   file. `SaveOwner` (`ambition_persistence::save`) records whose save is
   live, and the shell gives it to the activating experience
   (`hand_the_save_to_the_activating_experience`, `SessionScopeSet::Activate`,
   before the providers build): the save it had earlier in the process, else
   its own `<experience>/sandbox_save.ron`, else a new save. Ambition's file
   path is unchanged. Witness: both two-host arms above, with the save's
   waiver deleted. Poison (the handover skipped): both name
   `AmbitionGameSave`. Unit arms: `each_experience_gets_its_own_save_back`,
   `the_autosave_writes_the_owners_file`.

A negotiated input version does not exist. It is not owed while netcode is at
`N2`.

**Standing prohibitions:**

- Keep the session term in the rendered `TransactionId`. Only its peer
  projection drops it, because the construction scope's gather filter and A10's
  candidate-vs-live separation read the local stamp.
- Do not project `TransactionId` to `{room}` alone. The peer content term
  (`PeerContentIdentity`) must stay in the projection.
- Do not replace a raw `SessionScopeId` with the nearest canonical-looking value.
  `SimTick` looks canonical and is host-local.
- Do not mint a local `PeerSessionIdentity`. A peer session identity can only
  come from a session handshake.
- An id that crosses to a peer is a function of content or of a canonically
  sorted set. It is never a function of insertion or allocation order
  (`RollbackOrdered`, an `Entity` index).
- `id_peer_audit` censuses type names, so it cannot see a provenance defect.
  Hold provenance with value-level arms in the crate that mints the identity.
  `scripts/one_owner_per_canonical_identity.py` keeps each constant identity to
  one production mint.

**Blocked by:** nothing.

**Acceptance:** two Apps that have burned different numbers of local session
activations can enter the same deterministic match and produce the same
canonical mechanical identity/checksum. The witness must first assert that their
local counters differ.

### ROLLBACK-MUTATOR-POPULATION — the mutator guard sees a quarter of rollback state — ✅ DONE 2026-10-03

**Owner:** rollback scheduling (`scripts/check_rollback_mutators_run_in_sim.py`).

**Current state (measured 2026-10-03):** the guard reads every rollback
registration in every supported parameter spelling, including writes in the
body of an exclusive-world system. It reports 490 systems that mutate rollback
state and 0 acknowledged offenders.

✅ `adopt_occurrence_checkpoint_from_save`, `complete_durable_restore` and
`restore_inventory_from_save` left the scan on 2026-10-03: they run in the
simulation schedule (BODY-BORN-ON-THE-TIMELINE). For one day before that they
were `WAIVERS`, held by a runtime check that no save was applied over a live
timeline. That check was red on main for one commit (0ef79200b): the Sanic and
Mary-O rollback fixtures build their body on the timeline. The check and its
waivers are gone with the `Update` window. The old waiver's "the latch has no
`true -> false` transition" was false: teardown is one.

✅ `reconcile_roster_with_frozen_topology` left on 2026-10-03. Its one rollback
write was `ActiveMatch::adopt_seat_topology`, a copy of the roster's record that <!-- cite-ok: records a deleted method -->
nothing read. The copy is deleted (schema 302), and the reconciler reads
`ActiveMatch` only.

The exit code means "no new offender", not "clean". `ACKNOWLEDGED` names drift
that is real and the row that owes it. `WAIVERS` carry an argument. A banked
name that the scan stops reporting is fatal.

**Next action:** none. No offender is acknowledged.

**Known limits of the guard:**

- `Transform` writes are excluded by name. Ruled 2026-09-19 (`Q139`): do not
  grow architecture to satisfy this census. A green says nothing about
  `Transform`.
- A write inside a helper is attributed to the registered system that calls it,
  for ONE hop only (`inherited_mutations`, 2026-10-03). A call is attributed only
  when it is a free-function call (not a method) to a name defined once in
  production sources, or once in the caller's file. A helper name defined in
  several places (`tick`, `install`) is not attributed, and neither is a write
  two calls deep. The hop added 4 registered systems (490 to 494) and no new
  offender. `handle_ldtk_hot_reload` is seen again through
  `reload_ldtk_world_from_disk`, and its waiver now has a subject.
- A run condition is not a reachability proof.

⛔ Do not demote `derived`-documented clone registrations on a keyword match. A
component whose presence a query filter reads is authoritative even when its
value is derived. A waiver is a claim about the tree: re-read the function it
cites when you touch it.

**Blocked by:** nothing.

**Acceptance:** ✅ met 2026-10-03. Each item and its test in
`scripts/tests/test_rollback_mutators_run_in_sim.py`:
- The population is every rollback registration, not one registration
  spelling: `test_the_scan_covers_the_whole_registration_surface_not_one_file`.
- `handle_ldtk_hot_reload` is visible without its waiver being deleted:
  `test_the_hot_reload_is_seen_through_its_helper`. Poisoned three ways (method
  calls counted, uniqueness dropped, inheritance dropped); each fails its arm.
- A poison that respells a write in a supported param form still reddens the
  guard: the bundle-field, exclusive-world, session-world-helper and
  qualified-spelling arms.
- The population floor fails when a spelling stops matching:
  `test_a_shrinking_population_is_a_failure_not_a_clean_report`.
The first, third and fourth were met by earlier work and mapped here, not
re-poisoned. The known limits above stay open as limits, not as this row.

### BODY-BORN-ON-THE-TIMELINE — the simulation applies the save — ✅ DONE 2026-10-03

**Owner:** durable restore (`session/durable_horizon.rs`).

**Result:** the restore chain (`adopt_occurrence_checkpoint_from_save`,
`restore_inventory_from_save`, `complete_durable_restore`) runs in the
simulation schedule, at the head of the gameplay root, after the clock and
before the core step. The save, the latch and every value the chain writes are
rollback state (`AmbitionGameSave` is `resource-clone-custom-checksum`;
this row said otherwise for a day, from a search of one crate's registration
file). So applying the file is an ordinary deterministic step: a rewind past it
applies it again on the same tick. It does not matter whether the body was built
before the timeline started or by it.

Removed with the `Update` window: the `Q135` session-start gate in
`maintain_local_session` and `durable_hydration_is_pending`, the live-timeline
check `refuse_a_restore_over_a_live_timeline`, the fixture declaration <!-- cite-ok: records a deleted check -->
`TheBodyIsBornOnTheTimeline`, and three mutator-guard waivers. The gate would
deadlock now: under the rollback host the sim does not run until a session
starts. `Q135` is reopened in the awaiting file with the evidence. Its
guarantee holds: no tick is simulated over an unapplied save
(`a_conversation_on_the_first_tick_of_a_session_is_counted_exactly_once` passes
without the gate).

Witnesses:
- `the_save_is_applied_by_the_simulation` (Sanic fixture, body born on the
  timeline): `(in the GGRS step, in Update) == (1, 0)`, healthy. Poison (the
  chain back in `Update`): `(0, 1)`.
- `a_startup_load_is_applied_on_the_timeline_and_resimulates_identically`
  (shipped composition, seeded occurrence and custody rows): the session is
  live before the save is applied, and every one of 240 frames is healthy. This
  is the acceptance's "a non-empty save resimulates checksum-identically across
  its rise".
- `a_mid_session_load_does_not_reach_back_across_the_rewind`: a load's ledger
  write used to be lost (the `Update` write was rewound away). It lands now,
  and every replay of a tick agrees.

### DEATH-IS-ROOM-LOCAL — a participant's death rewinds its own horizon, not the session's

**Owner:** the death/checkpoint road (`session/checkpoint.rs`,
`runtime/sandbox_reset.rs`) and
[open-world runtime and residency](engine/open-world-runtime-and-residency.md).

**Ruling:** Q151 (2026-10-03, [`maintainer-decisions.md`](maintainer-decisions.md)).
An ordinary death is local to the dying participant and the affected room.
Another participant's live room and its consequences (a boss defeat, its
reward) stay. The live world and durable state share one rewind horizon. An
explicit whole-session reload may rewind the whole session. No global durable
rewind followed by reconciliation of surviving rooms.

**Current state:** the boss half is done. The death's admission
(`resume_at_checkpoint_on_reset`) names the live rooms that other participants
hold (`RoomReplayAdmitted::spared`), and `retract_boss_defeats_on_replay`
keeps the defeats in those rooms. A New Game spares nothing. The subject's own
room is never spared, also when another participant shares it. The option-A
machinery (`RoomsOwedTheRestore`, the replay of every other live room) is
deleted (rollback schema 305). Witness:
`a_death_in_one_room_leaves_the_boss_defeat_in_the_other_players_room`.

**Breakable respawns are served by the live room:** the restore forgets
every respawn record, and `mirror_breakable_respawns` records Bob's again
on the next tick from his platform's running timer, with the same due time
(probe, 2026-10-03). Witness:
`a_death_keeps_the_respawn_of_a_platform_in_another_players_room`.
A pickup's regrowth (Q152) is served the same way, from its running
`RespawnTimer` through `regrow_pickups`
(`a_death_keeps_the_regrowth_of_a_heart_in_another_players_room`).

**Alice's custody across Bob's room is served:** an item Alice banked in
hand and then put down in Bob's live room is back in her hand after her
death, held once, with the ledger saying `InCustody`
(`a_death_takes_back_what_was_put_down_in_another_players_room`). Every
other ledger row of a live room is republished from live state while the
room is loaded (`continuity.rs`), as the respawn record is.

**A reward taken from Bob's boss stays:** Alice opens the chest of Bob's
boss in his room after the checkpoint and dies in the hub. She keeps the
coins, and the chest stays looted with the boss cleared
(`a_death_keeps_the_reward_taken_from_the_other_players_boss`).

**The wallet goes back with the bag (2026-10-04).** A death restored the
bag (`OwnedItemsBaseline`) and not the wallet, so a purchase after the
checkpoint lost its goods and kept its price, and the save mirrored the loss.
The baseline now also holds the primary body's balance at the checkpoint
(captured at commit, adopted from the save on load, pinned in the restore
inputs), and the restore writes it back, plus the coins of each reward
grant still on record (a defeat the death keeps, such as Bob's boss, keeps
what it paid: `a_death_keeps_the_reward_taken_from_the_other_players_boss`).
Another participant's wallet is not rewound. Witness:
`a_death_undoes_a_purchase_since_the_checkpoint_whole` (control: a purchase
before the checkpoint survives); poisons on capture, load adoption and
restore each fail it. Schema 308 -> 309. `OwnedItems` is one session-wide
bag (the demo inventory is a demonstration, 2026-10-01: no per-seat bags).

**A grant names its owners (2026-10-04).** A coin Alice took in Bob's live
room, while he was there, was lost when she died elsewhere: the coin stayed
gone in his room and her wallet went back to the checkpoint (measured: 0 of
25 kept). What a placed pickup or an ordinary chest gives is now recorded
(`GrantSource::Authored`) with the participants in its live room when it was
taken, and the restore's acceptance keeps it in the bag and the purse while
one of them is spared. Alone, the room is built again with the source, so
the grant goes back with it. Witnesses:
`a_death_keeps_the_coin_taken_in_another_players_live_room` (control: the
coin taken alone) and `an_ordinary_chests_grant_is_owned_by_the_seats_in_its_room`
(control: a boss reward chest keeps its placement). Poisons: never keep, and
always keep, an authored grant; record a pickup's or a chest's grant with no
owners. Schema 311 -> 312. Open: what a spared participant takes OUT of the
shared bag since the checkpoint (an item used, a purchase) is not recorded,
so Alice's death gives it back. No production road seats a second player in
Ambition yet (Q153).

- Review 2026-10-05, P3: the shape for that residual. The restore rebuilds
  "the checkpoint bag plus the surviving bag mutations since it", so each
  mutation needs a sign and a provenance: (owners, item, signed delta,
  cause), recorded where the mutation happens (a grant is one kind; a use, a
  sale, a purchase and a transfer are others). A rewind takes the dying
  participant out of the owners and folds what is left over the checkpoint
  bag. Not another list of positive exceptions, and not ownership inferred
  later from room residency. A purchase is the sharp case: Bob's purse is
  outside Alice's rewind, so reverting the bag alone loses his purchase with
  no refund. Solve the shared-bag customers first; this is not a general
  inventory-event framework.
  - **Built 2026-10-05 for the one customer production has.** Read first:
    every road that takes from the shared bag acts for the primary body
    (a shop buy or sale, an item use from the menu, and the throw of an item
    the menu equipped from the bag), and the restore's subject is always the
    primary body. So rewinding those spends is correct, with one exception.
    Alice throws a menu-equipped quantity into Bob's live room. The throw
    spends the quantity and mints an object in his room. On her death, the
    checkpoint's bag gives the quantity back while the object stays.
    Measured: 2 javelins after the death, where there was 1.
  - The spend is recorded where it happens:
    `ambition_held_items::BagSpendsSinceCheckpoint` (item, and the object the
    quantity became). Its owner is the OBJECT, not the participants in the
    room of the throw. The acceptance keeps the spend of each object the
    restore keeps (lying in a spared room, or held by a body there), and the
    record is forgotten at a commit, a restore and a teardown. Schema 313 ->
    314.
  - Witness:
    `a_death_does_not_put_back_in_the_bag_what_was_thrown_into_another_players_room`
    (red at 2 before). Control 1: the same throw in Alice's own room is
    undone (1, 1). Control 2: thrown into Bob's room, then carried out in
    Alice's hand, is also undone (1, 1); a room-owned spend gives 0 there.
  - Poisons: no record (red at 2); keep every spend (control 1 red, 0);
    keep every held object (control 2 red, 0).
  - Not built: the purchase case has no production subject (the shop pays
    from the primary purse only, and the primary is the one who dies). The
    grants record (`RewardGrantsSinceCheckpoint`) and the spends record stay
    two records. Join them if a third kind of mutation of the shared bag
    gets a road that can survive the dying participant's rewind.

**The items go with the coins (2026-10-04).** An item a kept reward gave was
lost: the restore put the checkpoint's bag back whole while the reward stayed
taken (Bob's chest stayed looted; a banked defeat's mint was not built
again). The restore's acceptance now pins the bag and purse it promises: the
checkpoint's, plus what each grant it keeps gave. A grant is kept unless its
boss defeat is one the restore retracts. The boss crate states that rule once
(`retracted_by_restore`, shared with `take_for_restore`), and
`kept_by_restore` applies it to the grants. The verification reads the same
bag. Witnesses: `a_death_keeps_the_item_taken_from_the_other_players_boss`
(bag 1 then 0 before) and
`a_death_keeps_an_ability_taken_after_the_checkpoint_from_a_banked_defeat`.
Poisons: no grant pinned (both fail, and so does the coin witness); every
grant kept (the defeat retracted by a death keeps its ability and bounty).
This keeps a grant even if its source could come back, on two measured facts
stated at `kept_by_restore`: a bag-pickup mint has no ledger row, and an
opened chest's looted flag is not rewound.

**The whole-session restart is served (2026-10-04).** Measured 2026-10-03:
a New Game beside Bob's live room took back his boss defeat in the save and
its chest, while his room stayed the same instance with the dead boss in
it. Now the commit of a fresh checkpoint operation is a restart: it retires
every other live room in the same publication (`retires_beside`: residents
in the outgoing roster, roots despawned at application), and no other
player's body keeps a room live or is joined. The transaction's world stays
the replaced room alone, because two live room roots wear one identity
(open-world "Root identity"). Witness:
`a_new_game_leaves_no_live_room_holding_what_it_took_back` (no longer
ignored; it also counts live rooms and entities stamped with a room that is
not live). Where a seated participant's body goes on a restart is part of
the join road (Q153): here Bob is a placement of his room and goes with it.

**A defeat Bob won in a room he has since left stays (2026-10-04).** A
defeat record carries the participants whose bodies were in its room when the
boss fell (`BossDefeatSinceCheckpoint::present`), and the death's admission
names every participant but the dying one (`RoomReplayAdmitted::spared_participants`).
The restore keeps a defeat one of them won, outside the dying participant's
own room. A defeat with nobody else present still goes back. Witness:
`a_death_keeps_the_defeat_another_player_won_in_a_room_he_left` (poisons: the
restore ignores `present`, or the record leaves it empty; both read the boss
uncleared). Decision recorded here: a defeat is credited to everyone in its
room when it falls, since the edge has no attacker; a shared win stays when
one of its winners dies elsewhere.

**Ownership, not exceptions (review 2026-10-04).** A review found that the
restore is still a global rewind with exceptions (`spared`,
`spared_participants`), and that the exceptions fail where nothing live
republishes a consequence. Target: each consequence since the checkpoint
names the participants whose horizons own it; a participant's rewind takes
them out of each, and a consequence with no owner left goes back. Do not add
new uses of `spared` / `spared_participants` as the model.

- ✅ 2026-10-04, dormant world time: a `WorldTimeSchedule` record holds its
  owners (the seats in its live room when it was made). The restore's
  admission takes the dying participant out of each record
  (`disown_scheduled_returns_on_restore`), and the commit keeps a record of a
  room that is not live while it has an owner. It used to forget every
  record. Witness:
  `a_death_keeps_the_respawn_of_a_platform_another_player_broke_in_a_room_he_left`
  (control: Alice's own break goes back). Poisons: the commit forgets all, a
  record with no owners, and an admission that keeps every owner each fail
  it. Schema 310.
- ✅ 2026-10-04, consumed one-time pickups: `ConsumedSinceCheckpoint`
  holds each row consumed since the checkpoint with its room and owners. The
  restore's acceptance (`resume_at_checkpoint_on_reset`) pins the rows a
  spared participant owns into the ledger it restores, so the room the
  restore rebuilds, a later rebuild and the restore's verification all read
  one ledger. The admission takes the dying participant out of each record
  (`disown_consumed_pickups_on_restore`). It used to put the checkpoint's
  ledger back whole, so the room authored the pickup again on a later visit.
  Witness:
  `a_death_keeps_gone_a_one_time_heart_another_player_took_in_a_room_he_left`
  (control: Alice's own heart comes back; the witness also asserts that the
  restore committed). Poisons: the acceptance pins nothing, the record has no
  owners (both fail the subject), and the acceptance ignores owners (fails
  the control and the Q154 death test). Disabling the disown changes nothing
  in play, because only the primary participant's death restores; its
  arithmetic is held by
  `a_restore_takes_the_dying_participant_out_of_each_consumed_record`.
  Schema 311. A row cannot be written back by a `CheckpointDomainApply`
  reducer: `verify_restored_domains` compares the ledger with the pinned
  one and fails closed into `Paused`. The first version of this slice did
  that, and the witness found it.
- ✅ 2026-10-04, boss defeats: a restore keeps only the spared participants
  in each kept defeat's `present` (`take_for_restore`), so a later restore
  of another participant does not keep a defeat for a winner whose own
  restore already took it back. Not reachable in play today (only the
  primary body's death restores), so a unit test holds the arithmetic:
  `a_restore_takes_the_dying_participant_out_of_a_kept_defeats_winners`
  (poison: no shrink; Alice's later restore keeps the shared defeat).

**Acceptance:** Alice dies while Bob's room holds a boss he defeated after the
checkpoint: Bob's room, the boss row and its reward stay; Alice's room agrees
with the durable records it reads; a durable record written in Alice's room
after the checkpoint goes back. The shared-room and whole-session-reload
cases have their own arms.

### WEAR-REFUSES-UNPREPARED — a character outside the prepared generation is never worn — ✅ DONE 2026-10-03 (two remainders)

**Owner:** `ambition_combat::worn_kit::WornKit::of` and
`avatar/starting_character.rs::wear_character`.

**Ruling:** Q103 (2026-10-03): refuse. A character admitted into simulation was
prepared for that generation; no engine-default kit, no App-global catalog read.

**Done:** the kit compiler takes a prepared definition (`WornKit::of`), not an
id, so no road can ask for the kit of an id outside the cast. `WornKit::resolve`
and `resolve_playable_action_set` (the peaceful kit) are deleted. This also
closes the match-kit arm, which wore an unprepared id with a stage's borrowed
set. `wear_character` answers `None` for such an id and writes nothing: no name
made from the id, no identity. On a refusal `apply_worn_character_gameplay`
consumes the request, reports it, and puts `WornCharacter` back to the character
last applied (`PersonaBaseline::id`), so the body's id and its kit stay one
answer.

**Witnesses** (`avatar::starting_character::tests`):
`a_rewear_to_an_unprepared_id_keeps_the_previous_character` (the id, the name,
the pistol, the identity and the baseline are kept, and the request is consumed)
and `an_unprepared_id_writes_nothing_on_the_body` (no cast, an empty cast, and a
borrowed match kit). Poisons: the old fallback fails both; the refusal without
the `WornCharacter` revert fails the first.

**Remainders, not decided here:**

- **A generation that drops a worn id.** When the cast changes and no longer
  holds the id a live body wears, the body keeps its kit from the old
  generation and the refusal is reported once. That mixes generation N and
  N+1, which Q103 forbids. The repair is probably at admission (a generation
  that drops a live body's character is not admitted), not at wear time.
- **The home body of an unprepared starting id.** `session::setup` still builds
  the home body (an empty kit, named after the id, reported). Refusing it is a
  session-admission question: the session then has no body to drive.

### SETTINGS-ROLLBACK — finish the settings/mechanics admission boundary

**Owner:** rollback/mechanical-policy owners.

**Current state:** no simulation system reads `UserSettings`
(`scripts/measure_user_settings_in_simulation.py`). The frame-mode half is
closed: `ControlFrame` carries `control_frame_modes`, stamped at capture in
`populate_seat_control_frames`, and GGRS replays it per frame. The damage half is
open: `project_player_damage_policy` writes `PlayerDamagePolicy` from
`UserSettings.gameplay` in `Update`, the policy has no rollback registration, and
three simulation systems read it (`apply_player_hit_events`,
`apply_feature_hit_events`, `charge_projectile_input`). The in-game System
overlay can change `Difficulty`, `Assist` and `PlayerDamage` during a live
timeline, so a resimulation of frame N reads the policy that holds now.

**Ruling (`Q127`, 2026-09-19): deprioritised.** There is no generic
one-dimensional difficulty architecture. Difficulty is game policy expressed as
presets. Participant handicaps and CPU brain levels are separate concepts from
match policy. Spend no substantial effort here until the default game plays
exceptionally well, and do not box the design in.

**Ruling (`Q68`, 2026-10-03):** difficulty, gameplay modifiers and combat
behaviour are game-owned settings; the shell owns audio, display, bindings,
reusable accessibility and localization. `UserSettings.gameplay` is in the shell
crate today.

**Next action (when picked up):** use the `PortalTuning` precedent (the `Q120`
admission protocol). The settings road writes a mirror and proposes in
`MechanicalEditSet::Propose`, and the publisher is the only writer of the
authority. A match-wide policy is admitted at match activation; a
per-participant policy travels with deterministic per-seat input. Do not
reintroduce simulation reads of mutable `UserSettings`. Peers must not have to
share accessibility settings, so input interpretation travels with the input.

**Blocked by:** nothing.

**Acceptance:** rewinding/resimulating frame N observes the policy admitted for
that timeline, not whatever the settings UI contains now; the settings-to-policy
projection remains witnessed end to end; and no `sim`-schedule system takes
either policy resource as a parameter.

### THROW-MODIFIERS — route throws through rage and staleness policy — ✅ DONE 2026-10-03

**Owner:** Smash combat/knockback policy.

**Ruling (`Q133`, 2026-09-19):** for the Smash-like game, follow Smash. Ordinary
scaling throws obey rage, and set knockback keeps its set-knockback semantics.
Rage is game-level combat policy that the engine must be able to express.

**Done:** rage, as before: `ambition_entity_catalog::launch::launch_speed`
owns the rule that a set launch declines rage, and the throw road calls it
(`a_hurt_captor_throws_farther_and_a_set_throw_is_immune`).

Staleness, following Smash: a throw stales the THROW, which is its own move.
`CaptureThrowRequested` carries the emitting use's `move_instance`, and
`apply_capture_throws` claims the captor's playback only when it is that use.
The throw reads the move's stale count and records the use on the playback's
landed edge, as `mark_move_playback_landed_hits` does for a landing.
Damage stales through `hitbox::staled_damage`, the one law both roads use.
The percent term stales through `knockback_stale_scale`. A set throw
stales its damage and not its launch. A throw that no use claims is not
staled and not recorded. Shipped throw moves author no hit volumes, so a
throw is recorded once.

**Witnesses:** `ambition_demo_smash` `capture::a_repeated_throw_stales_and_a_neutral_ruleset_leaves_it_whole`
(three grab-and-throw sequences on George's table through the production
chain: 11/10/9 against the neutral 11/11/11, three uses recorded in each arm,
and the third stale throw launches slower). `ambition_combat`
`a_set_throw_stales_its_damage_and_not_its_launch` and
`a_throw_that_no_playing_use_claims_is_not_staled_or_recorded`. Poisoned:
dropping the record, blinding the read, and dropping the claim check each
fail the predicted assertion.

### DUEL-GUARD-RUNG — the CPU duel guard fails at rung 5 on main today

**Owner:** [BRAIN](#brain--finish-truthful-fighter-attack-selection). The
diagnostic half is done: `[dealt]` and the passenger assertion count only damage
dealt to a seat.

**Current state:** `two_cpus_in_the_shipped_composition_damage_each_other` passes
at its default rung 9 and fails at rung 5 (`AMBITION_DUEL_RUNG=5`, about 0.2
against the pair floor). Rung 5's fighters spend most of their damage on
summoned bodies, because the brain answers 71–84% of its decisions with
`grapeshot` and `call_the_shark`. That is a brain selection defect, owned by
BRAIN.

⛔ Do not fix this by lowering the floor or by normalising the metric.

**Acceptance:** the guard passes at all five published rungs at HEAD, because
rung 5's CPUs fight each other.

### A4 — separate control authority from body execution on the real schedule

**Owner:** accepted control writer map and actor-monolith frontier.

**Current state:** the prerequisite writer census is complete and did not find a
competing control authority. The old `PlatformerRuntimeSet` vocabulary is gone; <!-- cite-ok: the DELETED `PlatformerRuntimeSet` vocabulary, named on purpose: a resolvable citation here would mean the deletion did not happen -->
the real realization places body integration inside
`WorldPrepSet::Integrate`. Prior prose that mapped old and new set names by name
is not an implementation guide.

**Next implementation:** size the packet against the actual schedule seam:
control production, accepted control authority, then body execution/integration.
Keep the measured invariant that a body is advanced once per tick.

**Acceptance:** one accepted control fact feeds one body execution road; no
second body tick or hidden writer is introduced; schedule witnesses are placed
between actual neighboring phases rather than only `.after(...)` an abstract set.

### AUTHORITY-POLISH — one owner per mechanical fact, and no mirror in the rollback kernel

**Owner:** the architecture-completion campaign (C11 in
[`consolidation-plan.md`](consolidation/consolidation-plan.md)). Evidence for
`W0xx` rows stays in [`architecture-warts/README.md`](architecture-warts/README.md)
until the row closes. This row is the execution order. It does not displace
ID-PEER, A4 or the rollback rows.

**The question every item answers:** what owns this semantic fact? Target shape:
authoring/ruleset → preparation → one construction or transition → one canonical
live authority → disposable read models → presentation. Prefer deleting the
field, the writer and the reconciler over a stronger sync.

**Current state (2026-09-27):** steps 1–5 of the review order are done. Step 6
was reassessed: `measure_kernel_module_graph.py --scc` gives the 6-module kernel
(`abilities features items projectile session world`) plus
`assets ↔ character_sprites`, and each kernel leg has a recorded verdict in
AP14. The discovery passes then pointed at game expression. Closed items
(receipts in Git history): AP3, AP8, AP10–AP13, AP15, AP17–AP19, AP28–AP34,
AP36, AP37, AP45, AP48, AP54, AP56–AP58, AP62, AP66–AP71, AP76–AP85, AP93,
AP101, AP102, AP104, AP108, AP109, AP128, AP133, AP137, AP140, AP146–AP148.

**Open items:**

| # | Item | Next step |
|---|---|---|
| AP9 | Stale architecture docs (continuous) | Remove closed wart rows as items land. Keep a one-line receipt only where another row depends on it. |
| AP14 | Semantic actor-monolith SCC decomposition (continuous, Jon 2026-09-24) | The residual kernel is the 6-module SCC. Its legs were walked and judged genuine; the one misplaced leg, `abilities→features` (the puppy-slug gun asks the actor domain to spawn a minion), is not inverted only for the graph. Open question: is the runtime-mint description in `session→items` (`MintedItemBaseline`, `OwnedItemsBaseline`, `ItemCheckpointRestoreInputs`) item knowledge or occurrence-lifecycle knowledge? When an owner is clear, move state, behavior and installation together and delete the old edge (no callbacks, no compatibility re-exports). After each migration run `python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80`. See [`actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md) and [`actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md). |

`WorldTime` is recomputed from `Time.delta` × `ClockState.time_scale` at the
head of each step. That is the canonical clock, not a defect.

**Acceptance for the lane:** no mechanical fact has two mutable canonical
owners; rollback rows are authorities, not projections; construction publishes
no plausible-but-incomplete object; required mechanical policy does not fail
open. Close with a fresh census rather than a checked list.

### CHECKPOINT-ADMISSION-IS-NOT-COMMIT — an accepted restore changes nothing until it commits — ✅ BUILT 2026-10-05

**Source:** GPT review 2026-10-05, P1 (its first priority). **Owner:**
`session/checkpoint.rs` (`resume_at_checkpoint_on_reset`,
`cancel_accepted_checkpoint_restore`, `apply_committed_checkpoint_restore`)
and the room-transition terminal roads (`room_transition/{loading,commit}.rs`).

**The defect.** The admission of a checkpoint restore writes
`RoomReplayAdmitted` (`.to_the_checkpoint()`) on the frame it accepts the
operation, before the room is prepared. Fourteen production systems read that
message on that frame (census 2026-10-05). They return the subject to spawn,
retract boss defeats in the save (rows, looted flags, quests, chest,
`BossDefeatRetracted` and the rewards it takes back), forget timers, disown
`WorldTimeSchedule` and `ConsumedSinceCheckpoint` records, and reset gravity,
portals, cut-rope arenas and pending hits. None of them runs at the commit.
So `CheckpointRestoreOutcome::Cancelled` ("the live world is unchanged") is
false on the composed path. `a_failed_preparation_ends_the_operation_once_and_does_not_retry_it`
does not see it, because it composes none of those readers.

**A second hole.** Only a preparation failure publishes `Cancelled`. A
refused publication (`finalize_committed_room_transition`, `published ==
false`) and the terminal `SubjectGone` / `SubjectCannotTransit` roads take the
intent, and `retire_accepted_checkpoint_restore` then retires the operation
with no outcome. That breaks "one operation, one terminal outcome".

**The ruled shape (review):**
- Accept: pin the prospective restore inputs, and the prospective persisted
  fates (the save with the retracted boss rows, and the scheduled returns).
  Change no live gameplay state.
- Prepare: build the candidate from those values
  (`CommitFactsSource::Stated`), not from the live save. Moving the boss
  retraction to the commit alone is not enough, because the candidate reads
  boss fate from the live save.
- Publication accepted: apply the checkpoint domain exactly once (in
  `CheckpointDomainApply`, inside the exclusive commit, before a rebase: a
  message written there would be read in frame 0 of the new timeline, which a
  rewind can clear), and publish `Committed`.
- Every terminal road that does not commit: discard the prospective state,
  publish `Cancelled` exactly once, and leave live state bit for bit as it
  was. One terminalization answers a preparation failure, a refused
  publication and an invalid subject.
- Do not repair a cancellation by reversing mutations afterwards: that is a
  second reconstruction authority.

**Measured before the fix (2026-10-05):** a checkpoint at 7 coins, then a
boss defeated and its bounty paid, then the body moved away, then a restore
whose preparation fails. The outcome was `Cancelled`, and every fact had
changed: the body was back at spawn ((1303, 952) to (950, 904)), the boss
uncleared, the purse 57 to 7, the defeats since the checkpoint 1 to 0.

**Built:**
- The admission pins its replay in the operation (`AcceptedRestore::replay`)
  and writes no `RoomReplayAdmitted`.
- The room of a restore is built from `CommitFactsSource::AfterTheRestore`:
  the facts at the commit, with the consequences that construction reads
  applied to COPIES by the same functions (`prospective_commit_fates`: the
  boss retraction's record edits, `retract_defeat_records`; the two timer
  consequences, `forget_room` and `keep_only_owners`).
- The consequences run in a new schedule, `RestoreConsequences`, from
  `verify_and_publish` after its verdict accepts the room, while the
  candidates are hidden and before anything of the old room is retired. That
  is the world the replay was admitted against: the subject goes back to the
  old spawn, then arrives. All fourteen readers read through one parameter,
  `AdmittedReplays` (the messages in the simulation, the pinned replay in that
  schedule), in sets that keep the simulation's order.
- One terminal rule: an operation whose intent leaves the slot with no outcome
  gets `Cancelled { NotCommitted }` from `retire_accepted_checkpoint_restore`,
  the one place a refused publication, a subject that is gone or cannot
  transit, and a retraction all reach.
- Schema 312 -> 313 (the operation's checksum folds the replay).

**Witnesses:** `a_cancelled_restore_changes_nothing` (app_it). The property:
the four facts are as before the request. Its control: a committed restore
changes each of them. Poison: the admission writes the replay again; the
property goes red with exactly the measured facts. Unit:
`the_accepted_restore_outlives_its_frame_matches_its_intent_and_retires_with_the_slot`
(no replay message at the admission, and `Cancelled { NotCommitted }` when
the intent leaves uncommitted; poison: no publish, red). Control:
`an_operation_answered_before_its_intent_leaves_keeps_its_one_answer`.

**The prospect, measured by poison (construction reads the live world
instead):** the timer half is load-bearing, and three tests go red
(`breakable_respawn_across_rooms` ×2,
`pickup_regrowth_across_rooms::a_death_keeps_the_regrowth_of_a_heart_in_another_players_room`).
The boss half is load-bearing for the boss's first PHASE, not its life
(measured 2026-10-05). Its life is the encounter driver's:
`update_boss_encounters` gives it full health on its first tick unless the
save records the placement cleared. A boss built with the fate `Dead` starts
`Defeated`, and `update_ecs_bosses` makes it `Active` one tick later. With
the boss half alone poisoned (no retraction edit in the prospect), the
restored boss is alive (28) and `Defeated` on frame 2 of the death.
`a_restored_room_builds_the_boss_the_restore_takes_back_alive` now checks
each frame of the death and goes red there; the timer tests stay green under
that poison.

**Not built:** a composed witness for a refused publication or a subject
that is gone. Those roads reach the unit-witnessed retirement and run no
consequence, but no composed test forces one.

## P1 — ownership, composition and iteration

### GATE-PER-ACTOR — a body/capability gate is solid or open for each actor — ✅ DONE 2026-10-03

**Owner:** [`engine/capability-progression-and-world-gating.md`](engine/capability-progression-and-world-gating.md)
jointly with the gated-wall road (`gated_lock_walls.rs`, the per-room collision
overlay).

**Ruling (`Q54`, 2026-10-01, [`maintainer-decisions.md`](maintainer-decisions.md)):**
the gate is evaluated per actor. Alice in Phase Boots passes a phase wall; Bob
without them collides with it.

**Done:** a condition can publish a subject form (`SubjectConditionEvaluator`,
`ConditionCatalog::ask_for`); `body.can` and `body.fits` publish one. A gated
wall with a subject form stands in its room's `gate_solids`, and the publisher
writes a `GatePass` for each body that satisfies it. One rule,
`RoomCollision::gates_open_for`, names the gates open for a body. Body steps
read `ComposedRooms::solids_for`. The decide pass in `update.rs` gives the
brain the same walls: `ground_ends_ahead` reads them (its ridden
`SurfaceRef::Block(i)` is an index into them), and the floor queries
(`floor_below`, `supporting_floor`, `ground_below`) pass through a gate open
for the body (`PerceivedSolid::open_for_self`). A projectile, a dropped item,
the brain's line of fire and any reader that names no body meet the wall as
solid. An undriven body is asked as itself. The crouch/morph clearance check
still meets the wall as solid. A wall gated on a population fact
(`world.flag_set`, `inventory.holds`) has one answer for every body.

**Witnesses:** `a_body_gate_is_open_only_for_the_bodies_that_satisfy_it`,
`a_gate_open_for_one_body_is_missing_only_from_that_body_s_walls`,
`a_gate_open_for_self_is_no_floor_and_still_blocks_the_line_of_fire`,
`the_view_marks_only_the_gates_open_for_this_body`, and two composed ones, each
poisoned at its line in `update.rs`:
`a_gate_open_for_a_badnik_does_not_change_the_ground_it_plans_on` (sanic; the
paths diverge at frame 17 when `ground_ends_ahead` reads the shared walls) and
`a_fighter_over_a_floor_open_for_it_plays_as_over_the_void` (smash; a CPU over
a gate floor open for it plays exactly as over the bare void, and diverges at
frame 25 when its view gets no open gates).

**Not ruled:** awareness through an open gate (does the brain see a target
behind it?). It stays on the shared walls, which is the conservative reading.

### BOSS-REPLAY-RETRACTION — a replay that un-defeats a boss un-defeats it for every family — ✅ DONE 2026-10-03

**Owner:** the generic boss-progress road (`crates/ambition_boss_encounter`)
jointly with the save's replay policy
(`crates/ambition_persistence/src/save_data.rs`,
`every_durable_family_says_whether_a_replay_retracts_it`).

**Ruling (`Q51` and `Q56`, 2026-10-01, [`maintainer-decisions.md`](maintainer-decisions.md)):**
if a replay or rewind makes a boss undefeated again, the consequences its defeat
created after that point are undone too, for every boss family. A rollback
boundary that does not yet enforce this is a known issue, not an open question.

**Current state (landed 2026-10-01):** `BossDefeatsSinceCheckpoint`
(`ambition_boss_encounter::retraction`, rollback state
`boss.defeats_since_checkpoint`) records each placement cleared since the last
committed checkpoint. `retract_boss_defeats_on_replay` takes the entries of the
replay's live room, puts each placement back to `Untouched`, despawns its
reward chest, clears its looted flag and announces `BossDefeatRetracted`. The item domain
(`retract_mints_of_retracted_boss_defeats`) despawns the mints whose parent is
that boss and retracts their ledger rows, and takes back what a collected
mint or the opened reward chest gave (2026-10-02): `RewardGrantsSinceCheckpoint`
records each grant by its source (a mint's parent, or a chest's placement), and
the retraction takes the coins out of the collector's wallet (down to zero if
spent) and the granted item out of the bag. `reset_cut_rope_attempt_on_replay`
is now only the "try again" re-fight road, keyed by the replay's live room. The
re-fight travels with the request (`RoomReplayRequested::refight`) to
`RoomReplayAdmitted::refight`, so a refused "try again" leaves nothing latched
(2026-10-02). The quest step a defeat advanced goes back (2026-10-03): the boss
road queues `BossDefeated` caused by its placement
(`QuestRegistry::push_event_caused_by`), the quest drain records each step that
event moved, and the retraction calls `QuestRegistry::retract_caused_by`, which
puts the step back in the registry and the save.
A death retracts every defeat since the checkpoint, in every room (2026-10-03).
The death road is the checkpoint restore, which puts the bag back everywhere,
and its admitted replay was keyed by the death room only. So a death in another
room took the boss's ability out of the bag but left the boss dead and its quest
complete. `RoomReplayAdmitted::to_checkpoint` marks the checkpoint road, and the
retraction then takes every defeat since the checkpoint, except in the live
rooms another participant holds (`BossDefeatsSinceCheckpoint::take_for_restore`,
Q151; row DEATH-IS-ROOM-LOCAL). The retraction runs at the
restore's admission, before the restore forgets the reward grants, so it takes
the bounty out of the wallet too, and no defeat from before a restore is left
for a later replay to retract.
A quest that moved on after the defeat goes back with it (2026-10-03): its
steps are ordered, so `retract_caused_by` puts back a quest that stands at or
past where the defeat left it. The pirate-treasure payout follows the quest
(`grant_quest_completion_rewards` takes it back when the quest is no longer
complete). A flag that a later conversation set stays: it records the
conversation, not the defeat. The admiral's `npc_pirate_admiral_talked` is
one such flag. The boss road writes no other consequence: it writes the boss
row, the defeat record and the quest event, and the cut-rope dialogue sets no
flag.
Witnesses are in `game/ambition_app/tests/boss_replay_retraction.rs`.

**Known issues:** none open.

**Acceptance:** ✅ one generic retraction on `RoomReplayAdmitted`, keyed by the
replay's live room, for every boss family, with the cut-rope special case
deleted into it, the minted reward retracted with it, a witness per family shape
and a control. ✅ Every consequence in the known-issues list is retracted: the
mints, the bounty, the reward chest, the quest steps and their payout, and a
defeat in another room on the death road.

### MENU-OVER-DIALOGUE — an overlay opened during a conversation must not end it — ✅ DONE 2026-10-02

**Owner:** `crates/ambition_dialog/src/systems.rs` (dialogue input) jointly
with the menu input owner (`crates/ambition_input/src/menu.rs`,
`MenuControlFrame`).

**Ruling (`Q75`, 2026-10-01, [`maintainer-decisions.md`](maintainer-decisions.md)):**
pause, map and inventory may open during dialogue. The dialogue stays live
underneath without navigation input. Map and inventory are mutually exclusive
primary overlays.

**Done:**

- Start no longer ends a conversation: `apply_dialog_menu_input` closes on
  `back` alone. The pause, map and inventory keys may open the primary overlay
  in `Dialogue` mode (`menu::model::primary_overlay_may_open`). The overlay
  records that it opened from a conversation
  (`InventoryUiState::opened_from_dialogue`), and its close puts `GameMode`
  back to `Dialogue`, not `Playing` (`mode_on_overlay_open` and
  `mode_on_overlay_close`, for both backends).
- The dialogue reads no input while the overlay is open. The open overlay
  declares `INVENTORY_CONTEXT`, now at priority 160, above `DIALOGUE` (150).
  `dialog_input` and `dialog_pointer_input` return when a claim above
  `DIALOGUE` captures the seat (`dialogue_input_is_captured`), and both
  run after `InputSet::ResolveContext`.
- ONE RULE for map and inventory: they are two faces of the one primary
  overlay, so they cannot both be open. While the overlay is open, the map key
  or the inventory key turns it to that face. If the overlay already shows that
  face, the key closes it (`menu::model::open_overlay_key`).

**Witnesses:** in `ambition_dialog`,
`start_never_ends_a_conversation_and_back_alone_does` and
`an_overlay_above_the_conversation_captures_its_input`. For each backend,
`start_during_a_conversation_opens_the_{menu,cube}_and_closes_back_to_it` and
`a_face_key_turns_the_open_{menu,cube}_to_its_{tab,face}_and_closes_it_from_there`.
In the composed app, `update_schedule_census::the_conversation_reads_its_input_after_the_inventory_claims_it`.
Two edges order that pair today, and either one is sufficient: the direct
`.after(ResolveContext)`, and an indirect edge through a `Route` member that
runs `.before(CoreSimulation)`. Each edge was removed to show this.

**Residual (read from source, not measured):** under the Grid backend,
`menu.map` also toggles the standalone map panel
(`ambition_menu::map::input::handle_map_menu_hotkeys`). That panel declares no
input context, so if it opens over a conversation, it does not capture the
conversation's input. The web build always uses the Grid backend
(`KALEIDOSCOPE_MENU_BACKEND_ENABLED` is false on wasm). Under the Cube backend,
the native default, the panel does not open on `menu.map`.

### CANDIDATE-GENERATION-ORDER — a candidate session is prepared from the generation before its own activation

**Owner:** [`engine/extension-model.md`](engine/extension-model.md) (content
reload) jointly with the session-lifecycle owner,
`crates/ambition_platformer2d_provider/src/lifecycle.rs`.

**Current shape:** live session A stays on published generation N. Candidate B
is built and verified from transaction-local N+1, which
`PendingGenerationInputs` (`ambition_platformer2d_runtime/src/content_identity.rs`)
hands to the transaction's own preparation, keyed by its `load_id` claim.
Adoption makes B/N+1 authoritative atomically. Construction runs
`.before(AmbitionGameShellSet::Pending)` and the content commit runs after it, so
the channel, not an ordering edge, is how a candidate sees N+1. The channel
carries the admitted cast (`characters`), the boss catalog (`bosses`) and the
character catalog (`catalog`). The fighter ladder and the encounter waves are
standing projections that converge after the commit, so they are not frozen.
Guard: `the_candidate_is_built_before_the_router_advances_and_providers_only_adopts`.

**Next action:** a boss's numbers already reach the candidate. Construction
seeds each boss from the frozen N+1 catalog (`SessionMechanics::bosses`), and
since 2026-10-03 `BossConfig::seed` is required, so no boss reads the App
catalog at construction or on its first tick. Witness:
`a_boss_tuning_saved_while_the_game_runs_is_played`. Read from source
2026-10-03, not measured by a test: the other inputs preparation reads from the
App cannot differ between N and N+1.
- `ReloadRequest` refuses a change to `sheets` (`AuthoredSheets`), and its
  only writer is `register_character_sheet_ron` at plugin build.
- `forced_brains`, `population_cap` and `perception_extent` are immutable
  developer knobs, read once at build.
- Audio is the exception. Its domains take part in a reload, and the
  transaction holds the N+1 `AudioCatalogRegistry` until the commit, while
  preparation reads the App's (N). Preparation reads it only for provider
  presence: `validate`'s `has_provider`, and `music_ready` /
  `procedural_sfx_ready`, which ask whether a fragment exists. A reload
  replaces a provider's fragment, so N and N+1 agree, except for a reload that
  drops a provider's whole music or SFX fragment. That case is unmeasured, and
  it is the next thing to measure: a witness that drops the music fragment, and
  then either the channel carries `audio`, or preparation reads the pending
  audio by its `load_id`.

⛔ Not by an ordering edge and not by re-fingerprinting. Do not reopen A10.5's
guarantee that a candidate that cannot be built never retires the live session.
This is engineering, not a maintainer question.

**Blocked by:** nothing.

**Acceptance:** every generation input a candidate must see at N+1 reaches it
through the transaction-local channel rather than through global publication,
and a value that is a standing projection is stated as one rather than frozen.

### I2/I3 — finish independent content authoring and safe reload

**Owner:** [`engine/extension-model.md`](engine/extension-model.md) and content
reload/preparation owners. Implementation state:
[I3](engine/fast-iteration-implementation.md#i3---coordinated-generation-publication-and-local-reload)
and [I4](engine/fast-iteration-implementation.md#i4---small-procedural-sdk-and-one-native-semantic-reference).

**Current state:** a prebuilt host loads edited content without Cargo. Reload
has explicit outcomes, a no-op revision does not advance the generation, and a
stale attempt carries the generation it was prepared against. A build that reads
content off disk watches the pack's sources (`ambition_content::content_watch`)
and reloads on save. Participating families include moves, boss profiles and
encounters, the character catalog, fighter facets, the boss seed library,
validator bands, items and audio registries. The procedural tier (I4) runs
technique, boss-special and wielded-item modules on the linked and WASM roads,
with hot reload through the mechanical-edit protocol.

**Rulings (2026-09-19):** `Q110`: mechanical registry changes use explicit
lifecycle/replacement semantics; do not invent a universal silent overwrite.
`Q104`: content-authored movesets are the long-term authority, and duplicate
Rust move tables are migration scaffolding.

**Open work:**

- Converge the remaining reloadable registries on one prepare/admit/publish contract.
- I4: save eligibility; ports for body motion so the remaining wielded items (dive, blink, grapple, mark/recall) can become modules; GNU-ton's conductor as a module.

**Blocked by:** nothing.

**Acceptance:** changed content publishes exactly once under a new admitted
generation; identical content is a no-op; stale work refuses rather than folding
against a generation it did not read; no runtime road silently falls back to a
second authoring source.

### CONTENT-AUTHORITY-DUPLICATES — one authoritative reader per authored fact

**Owner:** [`engine/extension-model.md`](engine/extension-model.md) and
[`engine/participant-action-system.md`](engine/participant-action-system.md).

**Current state:** some authored facts have two readers:

- ✅ 2026-10-03: `boss_ron_target` no longer strips `_body`/`_hands` to map
  two files to one record; the key is the file stem. A probe on the strip
  fired 0 times over app_it (949 tests) and the content and boss lanes, with a
  positive control that fired. The `tools` generator still writes
  `gnu_ton_boss_{body,hands}` and `giant_gnu_{body,hands}` sheets into the
  published (gitignored) `gnu_ton_boss/` folder, and nothing loads them.
  The rows, the animation row and the hurtbox sample row of each boss attack
  are authored in `boss_art_keys.ron`; no other boss animation or sprite map
  was searched for after those moved.
- Yarn dialogue has its reader, and
  `game/ambition_content/src/content_validation.rs` checks dialogue references
  again. ✅ 2026-10-03, the `__` root fold: `known_dialogue_ids` also accepted
  the root of every `root__x` title. Four roots exist only as `__N` jump
  targets, so the validator accepted NpcSpawn ids the runtime cannot start
  (`a_spawn_naming_a_root_that_exists_only_as_variants_is_refused`). The ids
  are exact titles now. Open: the ids still come from `yarn_title_ids`, a
  `title:` line scan beside the Yarn compiler. The compiler is an optional
  dependency (`ui`), so replacing the scan is a dependency decision.
- LDtk/world cross-reference rules in `content_validation.rs` repeat rules that
  a world owner already checks. ✅ 2026-10-03, the LoadingZone target rule:
  `validate_ldtk_room_links` refused every zone without both targets, which
  refused a landing pad that `LdtkProject::validate` allows and reported half a
  target twice. It now checks only that a complete target names a room and a
  zone that exist (`a_landing_pad_is_allowed_and_half_a_target_is_refused_once`).
  ✅ 2026-10-03, the rest of the LoadingZone rules, compared with their owners:
  - Measured first: the validator had its own scan of the `LoadingZone`
    fields, and it trimmed a target that the converter did not trim. With
    `target_room = "scroll_lab "` the validator reported nothing and the room
    set dropped the link: a dead door that validation accepted.
  - The LDtk owner has the one text rule now (`field_text`: trimmed, blank is
    absent). `LdtkProject::validate` and `collect_room_links` both use it.
  - A blank zone id and a second zone with one id in an area are the LDtk
    owner's errors now. `validate` gates each room set, so each LDtk game gets
    them, not only this validator's game.
  - `rooms::unresolved_links` is the one judge of "this link names a room and
    a zone that exist". `RoomSet::try_from_parts` warns of an unknown room and
    `layout_warnings` of an unknown zone, both from it. A room set cannot
    refuse such a link: measured, a partial set keeps the exits of its rooms
    (one room alone, and the engine's 59-room world, whose 4 links name rooms
    that only the full game adds).
  - The validator holds the complete game, so there each unresolved link is
    an error. It reads the rooms and links that the runtime builds
    (`to_room_parts`), and its own scan is deleted.
  ✅ 2026-10-03, the NpcSpawn ids, same class. The validator judged
  `character_id`, `brain_override` and `dialogue_id` trimmed, and
  `convert_npc_spawn` gave the runtime the raw values (measured:
  `"npc_ai_slop "`, `" guard "`). An unknown character is a body with no
  identity, and an unknown preset is a panic when the room loads. The
  converter reads the three with `field_text` now
  (`an_npc_spawn_carries_its_trimmed_ids`).
  ✅ 2026-10-04, the quest checks' boss and encounter ids. Measured on the
  shipped world: the validator slugged the display name of each `BossSpawn`,
  and for 9 of the 11 bosses that is not the id that the boss reports when it
  is defeated (`system_boss` against `clockwork_warden`, `t_rex` against
  `trex_boss`). A quest that named the reported id was refused, and a quest
  that named the slug was accepted and could not complete. The two shipped
  boss quests name the only two bosses for which the ids agree.
  - `ambition_boss_encounter::behavior::authored_boss_behavior` is the one
    resolution from a placement to its behaviour. The boss constructor, the
    room boss-art keys and the validator use it.
  - The boss looted flag is keyed by the placement id, as
    `ambition_boss_encounter::rewards` keys the chest. The validator derived
    it from the slug.
  - The encounter ids and their looted flags are those of the loader
    (`load_encounter_specs_from_rooms`), which builds only the first trigger
    of a room. The validator's scan named each trigger.
  - Witnesses: `a_quest_names_a_boss_by_the_id_its_defeat_reports`,
    `a_boss_looted_flag_is_keyed_by_its_placement`. `check_quest_steps` takes
    the quests as an argument, so a test can plant a step.
  ✅ 2026-10-04, the rest of the quest targets. The flags of NPC talk,
  switches and `flag:` pickups, and the ids of NPCs, pickups and rooms, are
  read from the composed rooms (`QuestTargets::of`), each made by the function
  the runtime makes it with (`npc_talk_dialogue_id`, `SwitchActivation::
  parse_custom`, `PickupKind::StoryFlag`). Measured on the shipped world before
  the change: the sets were equal (198 flags, 172 NPCs, 38 pickups, 72 rooms)
  but for one flag that the scan did not know, `npc_generic_npc_talked`, which
  a talk to an NPC with no dialogue sets. The validator has no scan of the
  LDtk entities for a quest target now; `authored_flag_ids`,
  `authored_npc_ids`, `authored_pickup_ids` and `authored_entity_iids` are
  deleted. <!-- cite-ok: records deleted functions -->
  Open: `QuestStepCondition::ItemCollected` has no producer of its event in
  the tree (a search for `ItemCollected` over `.rs`, `.ron` and `.yarn`), and
  no shipped quest uses it. The cutscene bindings still read `active_area_ids`
  from the project; that is the level-by-level read the two-per-room rule
  needs.
  `scripts/check_world_graph_is_navigable.py` is a third reader of the zone
  targets (it does not trim, so it is stricter than the engine).

**Open work:**

- Remove each duplicate. Do not wrap Yarn in a schema unless that removes an
  authority. Do not move worlds into a content pack only for uniformity.
- External-capability witness: one capability outside the actor monolith uses a
  provider schema, a provider semantic action with a real device binding
  (`ProviderBindings`), and a causal fact, through public APIs only. It needs no
  new central enum variant and no private reader.

**Blocked by:** nothing.

**Acceptance:** each fact has one authoritative read; diagnostics name the
authored source; the old reader is deleted.

### A9 — establish truthful minimal engine profiles

**Owner:** public SDK/composition architecture.

**Current state:** capability-footprint and absence-contract tooling measure
what a profile links and installs. The three composition probes in
`composes_through_the_sdk` pin the fixed step and assert that `FixedUpdate` ran.
A profile witness must assert that it stepped, not only that it built.

**Ruling (2026-09-19, `Q100`, `Q106`, `Q108`, `Q97`, with `Q146` and `Q144`):**
capabilities are optional and composable. A capability that authored production
content requires and the composition lacks must refuse that content or its
admission. Reduced tools and tests may omit capabilities explicitly. The ruling
says: implement this architecture rather than continuing to census hypothetical
composition variants.

**Next implementation:** encode supported profiles as named capability
contracts, then make construction/step witnesses and absence guards test those
contracts.

**Blocked by:** nothing.

**Acceptance:** each supported profile constructs and steps a real subject; its
promised absent capabilities are absent from installation and resolved dependency
closure; the full Ambition composition remains intact.

### A7 — make item occurrence authority explicit where it carries a real invariant

**Owner:** [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md).

**Current state:** `GroundItem` construction is sealed, but that seal does not own
occurrence creation. Death drops, match spawns and other roads still mint
identity/provenance/custody facts at their occurrence sites. Two mint sites
degrade on purpose and belong to the `ItemCustody` inventory leg:
`ambition_held_items`'s thrown-item mint and `puppy_slug_gun`'s minion mint.

**Next implementation:** centralize only the occurrence decisions that share an
actual invariant (identity, custody, provenance, rollback ownership). Do not add a
generic request bus merely to reduce writer count.

**Acceptance:** reward policy consumes accepted occurrence outcomes and cannot
become an alternative minting authority; every remaining occurrence creator has
an explicit ownership reason.

### BRAIN — finish truthful fighter attack selection

**Owner:** [`engine/fighter-brain.md`](engine/fighter-brain.md), which holds the
modeling rules and the evaluation-rig contract.

**Current state:** the option layer prices what a move can do from one combat
model:

- A move's hittable region (`MoveFrameData::coverage`) and its push region (`push_coverage`) are separate. A pure shove is priced by `Features::displacement_value` (push coverage × the foe's proximity to a blast line × whether the push sends them that way).
- Admission is absolute: the opponent must lie inside the move's own region, with `ADMISSION_SLACK_PX` on each side. Admission leads the foe over `Perceived::staleness_s()` plus startup. Scoring stays on the observed position, so `reaction_ms` stays the difficulty axis.
- The brain and the hit resolver spend one launch law, `ambition_entity_catalog::launch::launch_speed`. A hazard is a value (`MoveHazard`, `ThreatTravel` in `ambition_entity_catalog/src/hazard.rs`), and an unresolvable ranged request is no offer.
- The shipped ladder's weights still order the rungs at least as well as any single-weight change tried (2026-09-21 refit sweep), so do not refit them.

Instrument: the `#[ignore]`d sweep `every_fighter_on_the_grid_can_fight_its_mirror`
(21 mirror matches of 3600 ticks; `AMBITION_GRID_TRACE` for one fighter). Run it
before and after a change, on one binary, and report both columns.

**Open roads, in order:**

1. **A brain cannot decline to attack.** The decision takes `options.attacks.first()` whenever the body is free, so the move that survives at range is thrown until the world changes. The brain does not remember its own last move. A decline gate on decision ticks was built and was a no-op, because presses are bounded by move duration (24 ticks), not the decision cadence (5). Next: a refusal that lasts as long as the move would have (`frames.total_s`), which needs one more piece of brain state and its snapshot projection.
2. **A counter and a buff cannot be chosen between.** Pricing them needs a defensive feature ("is the opponent committed to a swing"), not a wider admission rule.
3. **Rung 5's CPUs do not fight each other** (DUEL-GUARD-RUNG).
4. **`sanic` @5 leaves the stage early** (about 640–780 ticks, both seats airborne). That is a stage defect, not the one-move lock. The guard for it is a bout that ends early, not move variety.
5. **A ranged move scores `launch: 0`.** A projectile hit writes a dimensionless `HitKnockbackMagnitude::FeelScale(0.85)`, not `LaunchSpeed`. Settle whether the feel reference belongs on `LaunchConditions`, whether `0.85` leaves the projectile stepper, and what `max_knockback` means for a launcher. A launcher also needs a hazard-coverage feature (its `reach_fit` is zero at every range).
6. **A placed trap has a position, and `reach` is a radius.** Carry the dangerous region relative to the body at the resolved-offer seam, not another reach scalar.
7. **A teleport's destination does not reach the brain** (`TeleportParams`: `behind_nearest_foe`, `behind_gap`, aim). It belongs in the same resolved offer.
8. **The ladder's step per rung.** All shipped rungs author `rollout_depth: 0`, so `read_weight` is inert (Q90 ruling: remove it, row CPU-LADDER) and the L3 step the engine ladder takes at level 6 is missing. Measure `--rungs 3,4,6,7,8` to separate step size from one pair.

**Standing prohibitions:**

- Do not fix the lock with a second, softer admission rule or a `reach_fit` floor.
- Do not feed push coverage back into `reach_fit`.
- Do not patch the evaluator with a fighter-specific exception. Keep press generation separate from move utility.
- A two-move fixture cannot price a move against a kit. Pin a feature against the kit `attack_kit_of` builds.
- A fixture that shares an app with a live CPU measures the CPU too.

**Acceptance:** representative CPUs select movement-compatible and
movement-transition attacks from their authored menu across the intended
difficulty ladder, with no regression to the press/move identity contract.

### D72 — continue Smash parity from the inventory

**Owner:** [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md).

**Current state:** the inventory is the source of feature/parity truth. Do not
turn this queue row back into a chronological parity diary.

**Next implementation:** take the next inventory row whose policy is settled,
implement it on the production path, update that inventory row and add the
production acceptance witness.

**Blocked where applicable by:** product rows named by the inventory. (Q62
and Q89 were ruled on 2026-10-04: Q62 is the LDTK-SEMANTIC-DIFF task, and a
stand-in may keep an incomplete kit.)

### D166 — make character authoring boundaries load-bearing

**Owner:** [`engine/character-authoring-package.md`](engine/character-authoring-package.md).

**Next implementation:** for each remaining duplicated authored/runtime value,
choose one authoring owner and make every runtime representation a projection or
admitted prepared value. Prefer deleting the second truth to synchronizing it.

**Acceptance:** the owner document can name one authoritative authored value for
each migrated fact, and production consumers cannot bypass its preparation or
projection boundary.

### ROLLBACK-DEAD-SESSION — an invalidated GGRS session stops the clock in silence — ✅ DONE 2026-10-02

**Ruling (`Q138`, 2026-09-19):** an invalidated harness must refuse or fail
rather than silently produce frozen observations.

**Done:** `Platformer2dSimHarness::step` and `step_frame` refuse an unhealthy
session: they panic with the session's error and do not step. `try_step` and
`try_step_frame` return the refusal. An arm whose subject is behaviour under an
unhealthy session says so with `step_over_an_unhealthy_session`. Witness:
`rollback_room_transition::the_harness_refuses_to_step_an_unhealthy_session`
(a mismatch and an invalidation, each refused with the session's error, and
nothing advances).

The refusal found five arms that stepped a diverged session. Two step over it
by design (`a_confirmed_commit_refuses_to_rebase_over_a_diverged_session`, and
the `Update`-writer fixture of `how_much_of_the_peer_checksum_actually_varies`).
One reads the desync and now stops at the refusal
(`a_flag_requested_after_its_consumer_desyncs_the_timeline`). Two were
measuring over a session that diverged at frame 2; see SAVE-DIVERGES-AFTER-RELEASE.

### SAVE-DIVERGES-AFTER-RELEASE — a pickup and release in `blink_run` desyncs the sync test on the save — ✅ DONE 2026-10-02

**Owner:** rollback determinism, with the item custody road.

**Failure (measured 2026-10-02):** in
`does_a_presence_probed_row_move_when_its_value_does`, after the authored ground
item is picked up and released, the sync test diverged on `AmbitionGameSave` at
frame 2. A value probe per save field named the field: `custody`, and no other.

**Cause:** the save's custody rows are read from `InCustodyOf`, which is derived
rollback state. A load does not restore it; `project_custody_onto_residency`
inserts it again through `Commands` each tick. `DurableHorizonSet` had no edge to
the item residency chain, so the mirror could run before that projection and
read the value the latest forward frame left, not the value of the frame being
resimulated.

**Done:** `DurableHorizonSet` runs after `ResidencyStep::Project`, which also
flushes the projection's commands. The two arms run un-ignored over a healthy
session. Poison: without the edge, both are refused at tick 7 with the original
mismatch at frame 2.

**Guard:** `every_reader_of_in_custody_of_runs_after_both_derivers` asks the
shipped sim schedule. It finds the value readers by a probe that writes
`InCustodyOf` (today: the save mirror and `capture_custody_baseline`), and
asserts each is ordered after both derivers. Without the edge above it names the
save mirror. A query that only filters on `InCustodyOf` (`RoomResident`,
`Without<InCustodyOf>`) is not in its population; those readers are in
`ResidencyStep::Record` or run at a commit or a restore.

### DURABLE-HORIZON-CHECKSUM — the save mirrors write hashed state from `Update`

**Owner:** `ambition_platformer2d_actor_monolith/src/session/durable_horizon.rs`.

**Current state:** every writer of hashed save state runs in the simulation
schedule: the three `persist_*_to_save` mirrors and
`count_the_dialogue_visit_when_a_conversation_opens`.
`resources_crossing_the_rewind_boundary.py` reports that `AmbitionGameSave` does
not cross the rewind boundary. `AuthoredOccurrences` is rollback state with a
peer-stable checksum. The restore chain runs in the simulation schedule
(BODY-BORN-ON-THE-TIMELINE, 2026-10-03) and writes only while `SaveRestored` is
false, for exactly one primary body with a wallet, held by
`a_population_the_restore_cannot_complete_on_is_written_to_by_nobody`.

**Decided 2026-10-03:** Q129: shared durable world state is peer state, and the
eventual peer protocol compares its canonical semantic form, not the save
file's bytes; the netplay representation waits for active peer networking.
Q134: dialogue visit counts are per participant, not shared world state.

**What is left:** `dialog_visits` lives in the shared save and its checksum
projection, which Q134 rules out. It moves to the participant with the
reactive-character memory model (P6), not before. `OwnedItems` is
`resource-clone` while `OwnedItemsBaseline` is hashed; under Q129 the bag is
compared, so the two agree (engineering, `ROLLBACK-BAG-DESYNC`'s deferred
split).

**Acceptance:** ✅ Q129 is answered and the three mirrors follow the ruling (they
run in the simulation, and the save stays in the checksum); ✅ the
dialog increment has its own answer, which was not the mirrors'; ✅ the one-shot
pair's ordering against GGRS start is characterised rather than assumed; and ✅
the restore chain has its road: the simulation schedule (2026-10-03,
BODY-BORN-ON-THE-TIMELINE), which replaced the `Q135` session-start gate.

### MUSIC-CANDIDATES — music is chosen from scoped, prioritized candidates

**Review 2026-10-05 (carried forward):** same-room music is still last writer
wins (one `priority_track`, one `priority_owner`, and `claim_priority` lets a
later writer win). Keep "star power ends when victory starts". The service
holds `(room or scope, stable source) -> (cue, priority)` candidates with a
deterministic order and tie-break, and a source releases only its own claim.
Do not add a tier, a slot or an ordering edge for the next simultaneous
customer.

**Owner:** `ambition_encounter::music` (`EncounterMusicRequest`) and the music
intent in `ambition_platformer2d_actor_monolith/src/music/intent.rs`.

**Ruling:** Q72 and Q150 (2026-10-03). Each source contributes a candidate
(scope, track/cue, priority) with ambient < encounter < boss. The highest
authored priority wins; the primary participant breaks ties; the choice is
deterministic. `EncounterEffect::SetMusic` may be removed; an encounter's
ability to influence music stays.

**Current state:** the arbitration across participants is built. An
encounter claims one of two tiers (`priority_track`, `base_track`) of its
live room, `EncounterMusicRequest::priority_of` ranks a room 2/1/0
(boss/encounter/ambient), and `the_room_the_music_plays_for` picks the
participants' room with the highest rank, the primary seat's on a tie, then
the lowest room. Witnesses:
`a_participants_boss_outranks_the_primary_seats_room_music` and
`the_heard_room_is_the_highest_priority_then_the_primary_then_the_lowest`.
No shipped encounter authors `SetMusic`.

**What is left:**

- Each room has one priority slot, and the last writer wins it
  (`EncounterMusicRequest::claim_priority`, `ambition_encounter/src/music.rs`).
  Its owner is a `&'static str` that names a kind of source, not an instance.
  When two sources claim one room and the later one releases, the earlier
  claim is gone, unless its source claims again on each tick (review
  2026-10-04). Target: each source owns its candidate (source instance,
  scope, track, priority). A release removes only that source's candidate,
  and the choice is made again from the candidates that remain. The
  cross-room arbitration above is done; this is the arbitration in one room.
- The priority is fixed by the tier, not authored per candidate. Make it an
  authored property of the candidate when content needs a value between the
  tiers.
- Remove `EncounterEffect::SetMusic`, or give it a customer.

**Acceptance:** Bob's boss candidate in his room outranks Alice's ambient room
music; two candidates of equal priority resolve to the primary participant's;
in one room, two sources claim and the later one releases, and the earlier
one's track plays with no new claim; the result is the same on every peer and
after a rewind.

### MOUNT-RIDER-CUSTOMER — a shipped rider controls a shipped mount

**Owner:** `features/brain_command.rs` (`MountedBrainCache`, the source-only
arm) and the dismounted-rider road (`features/ecs/dismounted_rider.rs`).

**Ruling:** Q76 (2026-10-03): keep mount/rider support and give it an early
customer (a character riding the dog, or the robot commandeering a shark).

**Current state:** `MountedBrainCache` has no production constructor, so
`ControlClaimant::Mount` has no production writer. Nothing resumes the mount's
recorded brain when a ride ends; a mount death rebuilds a solo brain from
config.

**Next action:** author the customer, then specify and repair, each with a
witness: control transfer, the mount's previous brain, dismount and its
restoration, body and room lifetime, damage and death, participant ownership,
animation composition.

**Acceptance:** the customer mounts, the mount obeys the rider, and on
dismount the mount runs the brain it had before; the same after a rewind.

### CAST-FRAMING-TARGET — framing asks for a composition, not only a floor

**Owner:** `ambition_sim_view::camera_snapshot` (`CastFraming`).

**Ruling:** Q86 (2026-10-03): framing expresses a bidirectional desired target.
Desired framing/subject scale, hard zoom bounds, room constraints and smoothing
stay separate; downstream stages clamp the target.

**Current state:** the cast bounds set a minimum view size, so framing can
only zoom out.

**Acceptance:** two subjects that close in zoom the view in toward the desired
subject scale, and a hard bound or room constraint still clamps that target.

### AUTHORED-INTERACTABLE-STATE — facing gates, per-chest and per-pickup persistence

**Owner:** `ambition_entity_catalog::placements` (`InteractableSpec`,
`ChestSpec`, `PickupSpec`) and their consumers.

**Ruling:** Q63 (2026-10-03). Wanted: a facing gate with authored semantics
(not sprite geometry); per-chest persistence; persistence of a physical pickup
(a different fact from a Q45 entitlement). The per-breakable debris cue is
wanted, but deferred. Each capability adds its field and its consumer in one
change. Q105 (2026-10-03): an authored world may start non-pristine (a chest
already open, a pickup already absent, a door open); the authored state lowers
into the same canonical state gameplay produces, such as the `Opened` marker.

**Next action:** measure what a save → quit → load keeps today of an opened
chest and a collected pickup, then add the missing durable fact.

**Acceptance:** an opened chest and a taken pickup stay so across a reload; an
interactable that requires facing refuses a body that faces away; a chest
authored open is built with the same state as a chest the player opened.

### REACH-VIEW — AI reads reach from the move's geometry

**Owner:** the moveset; `ambition_entity_catalog::MoveFrameData::reach` for
fighters and `ambition_characters::brain::action_set` (`reach_px`) for action
sets.

**Ruling:** Q35 (2026-10-03): the moveset/attack definition owns prospective
mechanical reach, also during startup. AI reads a derived view of the move
geometry, never sprite bounds. This extends Q107.

**Next action:** find each AI read of reach, and make sure that each one comes
from the move geometry by phase. `MoveFrameData::reach` is already derived from
the Active volumes.

**Acceptance:** a test changes a move's hitbox and sees the AI's reach change,
with no sprite change.

**Measured first (2026-10-03, the shipped prepared cast, 29 rows with an
autonomous profile):** 15 Smash rows have an attack move, and for 14 of them the
authored `smash_hit_band` was more than 1 px from the reach of the move. Examples:
goblin 32 against 38, the automata 36 against 44, Carl Stargan 36 against 22,
the goblin brute and the pirate heavies 36 against 48.4. Also measured: the
moveset derived from an action set reaches `1.1 × reach_px` (28 gives 30.8), so
`reach_px` is not the hit extent. The moveset geometry is the one answer.

**Slice 1, DONE 2026-10-03: the Smash brain.**

- `BrainSnapshot::melee_reach` is the reach of the move that the body's forward
  attack press starts, in the real posture of the body. The snapshot builder
  fills it with `melee_reach_of`, which asks `move_for_attack`, the resolver of
  the press road. A running body thus reads its dash attack.
- `tick_smash` sets its three distance bands from that reach each tick
  (`SmashCfg::with_hit_band`). The bands in a `SmashCfg` are only those of a
  body with no attack move (`NO_ATTACK_MOVE_HIT_BAND`, 36 px).
- `BrainProfile::smash_hit_band` is deleted, with its four authored rows. <!-- cite-ok: records a deleted field -->
- Witnesses: `the_hit_band_is_the_reach_the_snapshot_states` (combat),
  `a_smash_brain_swings_where_the_hitbox_of_its_move_reaches` (the acceptance
  test: two bodies that differ only in one hitbox) and
  `the_reach_is_that_of_the_move_the_press_starts` (monolith), and
  `a_smash_enemy_swings_from_the_reach_of_its_own_move` (app_it: the goblin
  brute decides its first press at 48.3 px; with the snapshot line removed it
  closes to 35.4 px).

This changes how the Smash enemies space themselves: each now stops and swings
where its own move reaches.

**Slice 2, DONE 2026-10-03: MeleeBrute, the hostile Aerial bird, the aggressive
Patrol.** Each read `cfg.attack_range`, an authored distance. Each now reads
`BrainSnapshot::melee_reach`, and its cfg distance is only for a body with no
attack move. The snapshot builder derives the reach for every body with an
attack move (review 2026-10-04: the reach is a fact of the moveset, so no
brain gate decides if it is derived). Each brain decides if it reads it: a
peaceful patroller does not, because its `attack_range` is where it stops to
talk (`a_peaceful_patroller_keeps_its_own_distance_when_told_a_reach`; poison:
it reads the reach, and it stops turning to the foe at 40 px). The old gate,
`StateMachineCfg::closes_to_its_melee_reach`, is deleted. <!-- cite-ok: a deleted name --> The shipped MeleeBrute users are
the provoked pirate heavies (reach 48.4, authored 53 to 59 with the 56 px
floor). The parrot read 60 and fsm_noodling 50; both pecks reach 52.8.
Witnesses: `melee_reach_tests` in `brain/state_machine/tests.rs`. Each read
was poisoned alone, and only its own test failed.

**Remaining readers (not changed):**

- `ChargeCrashCfg::bite_range` (the shark 200 against 46.2). It is not a
  reach. The shark presses its bite and then charges, and the charge carries
  the hitbox to the foe. A view from geometry must add the travel of the
  charge to the reach of the bite. This needs a measurement of how far the
  charge moves the hitbox while the Active window is open.
- The fighter's `assumed_foe_reach`: a number for the reach of the FOE.
- A body with no attack move keeps the authored distance of its brain. One
  case looks incorrect and is not measured in play: a dismounted rider with no
  ranged item gets a MeleeBrute brain whose distance is the profile's
  `attack_range` (1100 px for the pirate raider), so it can stop and press
  nothing from far away.
- `MoveFrameData::reach` is the reach of the volumes in the body frame. It
  does not include the motion of the move (a dash attack moves the body), so a
  running Smash enemy reads 40 px for a dash attack that travels farther.

### BREAKABLE-SOLIDITY — a solid breakable is a barrier, not a blink wall — ✅ DONE 2026-10-03

**Ruling:** Q102 (2026-10-03): a solid breakable is not semantically a
`BlinkWall { Hard }`. If both need the same collision property, extract or reuse
a generic solidity/barrier semantic that both compose.

**Measured first:** a body with `blink_through_hard_walls` blinked through an
unbroken solid crate (it landed at x = 340 past a crate at 220..242).

**Done:** `ambition_platformer2d_core::BlockKind::Barrier`: full collision on
both axes, and no blink upgrade passes it. `BreakableCollision::Solid` publishes
it. The one solidity mechanism is
`collision_semantics::is_full_collision_surface` (with `is_support_surface` for
the kinds a body rests on): `Solid`, `BlinkWall` and `Barrier` are its members,
and the 21 readers that listed `Solid` and `BlinkWall` by hand now call it.
A barrier is an object, not terrain, so a reader that asks for `Solid` alone
still does not see a crate, as before. A brain perceives it as a solid. The
block has no tile and no fill: the crate draws itself.

**Witness:** `the_hard_blink_upgrade_does_not_pass_an_unbroken_solid_breakable`
(monolith `features::ecs::world_overlay`): the real overlay publisher, then the
core blink. The control is a hard blink wall of the same rectangle, which the
same body passes. The arm failed before the change.

**Follow-up, the moving platform (2026-10-03, decided on Q102's rule; no
maintainer question).** A moving platform is composed as `BlinkWall { Soft }`
(`ambition_platformer2d_world::platforms`). Q102's question is: is it a solid
that only shares the blink-wall shape, or a blink wall?

- Measured, in the world that collision composes: a body with no
  through-upgrade stops at the platform (it lands at x = 214 before a platform
  at 229..251). A body with the soft upgrade passes it (x = 340). A body with
  only the hard upgrade stops (x = 214).
- `as_collision_block` states this as its purpose: a moving platform is
  "deliberately not" a hard blink blocker, and the soft upgrade passes it "just
  like a soft blink membrane". The solid breakable was different: it was a
  hard blink wall only to get full collision, and the pass was a defect.
- No other reader gives a moving platform a different result from a solid.
  Perception, the fighter's recovery and the projectile response each put
  `BlinkWall` in the same arm as `Solid`. The tile sprite, the fill colour and
  the debug colour do draw a blink wall differently, but they read the room's
  authored blocks and not the composed collision world.

So the platform is a blink wall in the one sense that matters, and it stays
one. `the_soft_blink_upgrade_passes_a_moving_platform` pins the behaviour, and
it fails if the platform becomes a `Barrier`.

### CPU-LADDER — the brain owns the knobs, Smash owns the ladder

**Owner:** `ambition_combat::brain::fighter` (mechanism) and the Smash
rules/content (ladder tuning).

**Ruling:** Q88 (2026-10-03): the generic fighter brain exposes reusable
controls (reaction, tactics/aggression, prediction/evaluation, execution/error);
Smash owns the mapping from "CPU level N" to them. Q90 (2026-10-03): remove the
inert `read_weight`; a correctly named parameter returns with an implementation
that gives it meaning.

**Current state:** `fighter_brain_ladder.ron` is in
`game/ambition_content/assets/data/`, and its schema is registered by
`ambition_combat`. Every shipped rung sets the rollout fields to zero, so
`read_weight` is read only behind `uses_rollouts()` and changes nothing.

**Acceptance:** the ladder data and its level vocabulary live with the Smash
rules/content; another game can build the brain with no ladder; no authored
ladder field is inert.

### BARK-CARDINALITY — a singular bark role has one owner — ✅ DONE 2026-10-03

**Ruling:** Q52 (2026-10-03): cardinality is explicit. Two providers that
contribute the same singular bark role conflict; a plural, composable bark
collection is modelled explicitly when content wants one.

**Measured (2026-10-03):** the question was asked of `CombatBanterRegistry`, a
name-keyed registry that three installers filled and where a second set
replaced the first. That registry was deleted on 2026-10-02 (`957961618`). A
character's barks are now only the `barks` field of its catalog row
(`CharacterBarks`, one pool for each `BarkSituation`), and a boss names a row as
its `voice`. One provider authors a row: catalog assembly refuses a second
provider's row for the same character id with `DuplicateCharacter`, whose
report names both providers, and the earlier assembly stays as it was. So today
a second contribution conflicts; it does not replace and does not concatenate.

**Witness:** `a_second_provider_cannot_author_the_barks_of_one_character`
(`ambition_characters`, catalog registry). Poison: with the duplicate check off,
the second provider is accepted and the arm fails.

**Not checked:** one provider that writes the same character id twice in one
catalog file. No plural bark collection exists, because no content asks for one.

### SESSION-EDGE-STATE — a session starts from nothing the last one left

**State:** a room-scoped spawn takes its session as an argument, so the
session retirement ends it; seven resources, the declared message channels,
the presentation channels (not the sound), the camera and the per-attempt
ledgers are reset at the session edge (2026-10-04). A session is built
from the save of its own experience, not from the live save, and it begins
with the bag that the process began with. Three host constants are recorded,
not reset. Open: a replacement by the SAME experience whose save changes
before the adoption, and the New Game restore's own spelling of the starting
bag.

**An entity that a session spawned as it ran outlived the session
(2026-10-04).** This is the largest finding of the row, and the first census
did not see it, because the hub walk spawned nothing at run time. In
`portal_bridge`, a session that placed one portal was replaced. The placed
portal was in the world of the next session on each of its first 31 ticks
(a fresh host with the same save: none), with three shots in flight at tick 0.
The peer rows `RoomScopedEntity`, `InRoomInstance`, `SimId` and `SimIdCounter`
differed from the fresh host's on each tick. Both successions (replaced in
place, through the title) gave the same result.

- **The cause was a second spawn road.** `SpawnScopedExt::spawn_room_scoped`,
  on plain `Commands`, stamped the room scope and NO session owner. The session
  retirement (`despawn_retired_session_entities`) despawns by the
  `SessionScopedEntity` stamp only. Seven production sites used that road: the
  portal shot, the placed portal, the dropped portal gun, the thrown item, the
  match item, and the two world-item spawns (a Mary-O block's reward).
- **The repair deletes the road.** The seven sites call
  `spawn_room_in_session` with the scope of `SessionCommands::spawn_scope()`;
  the two world-item functions take the scope as an argument. The trait
  `SpawnScopedExt` is deleted (its other method, `spawn_mode_scoped`, had no
  production caller: a mode owner is spawned with `spawn_mode_owner`). A system
  that reads intents reads them with no session too and drops them, so an old
  intent does not fire in the next session.
- ⚠ A bundle can still name `RoomScopedEntity` by hand in a plain `spawn`. The
  witness below asks a running world for each room-scoped entity with no
  session owner; no source guard forbids the shape.
- **The audit of that shape found one more site (2026-10-04).** I read each
  production site under `crates/` and `game/` that names `RoomScopedEntity`,
  `RoomVisual` (which requires it), or a bundle that holds it
  (`FeatureLifecycleBundle` and the bundles built on it). Each takes a session
  scope, with one exception: the person who comes out of the cut-rope boss
  (`cut_rope/victory.rs`) was a plain `spawn` with the room stamp put on by
  hand. Measured: that person was in the world at the title after its session
  ended, and the next session had the same entity (both successions). Two
  hosts agree on it, so a census does not see it. The system now takes
  `SessionCommands` and spawns with the scope. Witness:
  `shell_host_lifecycle::the_victory_npc_of_a_cleared_boss_ends_with_its_session`
  (premise: one person; red before the repair on five readings). The sites in
  `items/pickup/minted_horizon.rs`, `world/rooms/stage.rs` and
  `lifecycle/custody_horizon.rs` are in test modules.

**A presentation effect crossed the edge (2026-10-04, review finding).** The
first repair of the message channels kept each channel that presentation
reads, on the ground that no simulation reads one. That used the rollback
class of a channel (`for_each_presentation_effect`) as its lifetime across a
session. Measured on the shell host: a session that asks for a camera shake, a
finishing zoom and an effect in room 0 on each update was replaced, and at the
activation frame of the next session two of its effects were on the bus
(`VfxInRoom` names a `LiveRoomInstance`, and the first room of each session
has the same one) and the camera of the next session was at 6 px of shake and
a full zoom.

- Each presentation channel now ends at the activation
  (`end_presentation_effects_with_their_session`). The default for a new
  family is that it ends. One channel crosses, with its reason stated there:
  `OwnedSfxMessage`, whose reader plays a sound only for the live audio owner.
- The camera rests at the activation (`rest_the_camera_on_activation`). The
  appliers run before the activation in the update that replaces a session
  (measured by a poison), and a motion decays over time, so emptying the two
  request channels is not sufficient.
- Witness:
  `shell_host_lifecycle::an_effect_that_a_session_asked_for_is_not_presented_by_the_next`
  (two successions; premises: the old session's camera moves, and its effect
  and a sound are on the bus), and
  `external_effects::tests::only_the_sound_channel_crosses_a_session_activation`.
  Poisons: each family kept (2 effects on the bus at frame 0, and the unit arm
  names seven families); the camera rest not registered (the camera is at
  (6.0, 1.0) at frame 0); the sound not kept (the bus control is red).
- ⚠ My first fixture did not reach the edge, and its bus prediction missed: a
  message that a test writes between two updates is read in the old session's
  own frame, because a replacement takes effect on the second update. The
  fixture writes in `PreUpdate`, where a rollback host releases the confirmed
  effects of the last tick.
- ⚠ The reason I gave for keeping the sound did not reproduce. With the sound
  channel emptied too, the whole of `app_it` fails only the bus control of the
  new arm. The channel stays kept because its reader refuses a sound of a
  session that ended; no test shows that a sound must cross.

**A session was built from the save of the session that played (2026-10-04,
review finding B).** A session is built hidden, before its route is activated.
The activation is what gives the live save to the experience of the session
(`hand_the_save_to`). So the builder read a save that was not its own when the
session that played was of another experience. Two readers did this:

- the durable horizon of the candidate (`CandidateDurableHorizon::from_save`:
  which occurrences are gone, the custody rows, the minted items), and
- the commit of its first room (`PersistedFates::of_world`: which bodies are
  dead, provoked or cleared), which also read the world-time schedule of the
  session that played. The second reader was found while I read the first.

Measured on the shell host. The save of Ambition says that the hub's gun sword
is `Consumed` and that one person of the hub was provoked. A veteran plays
Sanic and replaces that session with Ambition. For the first 3 frames of the
Ambition session the gun sword was in the world and the person was peaceful; a
fresh host with the same save has neither on any frame. From frame 3 the two
hosts agreed (I did not measure which road corrects it).

- Persistence prepares the save: `prepare_the_save_of(owner, ..)` gives the
  live save to its owner, the save that was put aside to an experience that
  played before, and the file to an experience that did not (read one time and
  put aside). It takes `&AmbitionGameSave` and changes no owner, so a refused
  candidate leaves the session that plays as it was. The hand-over then gives
  the value that the session was built from.
- The builder reads it through `CandidateSave` (`session/durable_horizon.rs`).
  The horizon also carries the fates of the first room, and the commit of
  that room takes them as a stated value (`CommitFactsSource::Stated`). A room
  of the session that plays, and a direct-entry demo, read the world at the
  commit as before.
- Witness:
  `shell_host_lifecycle::a_session_prepared_while_another_experience_plays_is_built_from_its_own_save`
  (premises: with a new save the hub has the item and the person is peaceful;
  a fresh host with the save has no item and a hostile person; Sanic has the
  live save and no row), with an adoption arm and a refusal arm (two holders
  of one identity: Sanic stays live with its owner and its save, and the
  session that is admitted later equals the fresh host's). Unit:
  `save::tests::a_prepared_save_is_the_save_its_owner_is_then_given`.
  Poisons, one at a time: the horizon from the live save (item 1 and peaceful,
  frames 0 to 2); the first room's facts from the world (peaceful, frames 0 to
  2); the prepared save taken out and not put aside (the session is given a
  new save and the row is lost on 31 frames); the file read each time (the
  unit arm).
- ⚠ A prediction missed. I expected the wrong ledger to be written over the
  row in Ambition's save (a durable loss). It was not: the save kept the row on
  each frame. The measured cost of the defect is 3 frames that a fresh host
  does not have. The durable loss is what the third poison gives.
- ⛔ Nothing guarded the row in those 3 frames. I did not measure why it
  survived. One candidate (not measured): nothing writes the ledger into the
  save before the correction on frame 3. That would be an order of systems,
  not a rule, so the witness reads the row on each frame.
- ⛔ THE RESIDUAL, not repaired: when the candidate is of the experience that
  has the live save (a restart, a world reload), the prepared value is the live
  save at the preparation. The session that plays can change it before the
  adoption, and the candidate is then built from the older value. A witness
  would be `a_session_that_replaces_its_own_experience_is_built_from_the_save_at_its_adoption`:
  change the save in the frames between the preparation and the adoption, and
  compare with a fresh host that has the later save.
- ⚠ For two peers the same question is open at a door: a plan lowers from this
  host's save (`minted_baseline_from_save`), so it must lower from a horizon
  that the peers agree on. See [netcode](engine/netcode.md).

**Two resources of a room crossed the edge (same day).** With the entities
gone, `PortalFrameHistory` held one frame of the old session at tick 0 (a fresh
host: none). The first room of each session has the same live key, so the
eviction read that frame as a portal of the new room that closed, and it would
push a body that stands across the old plane. The portal crate now forgets the
history at the activation (`forget_portal_frames_on_activation`).
`CutRopeHeavyObjectCycle` is in the peer checksum and a room replay advances
it: after one replay the next session held index 1 on each of 31 ticks and a
fresh host held index 0, so one host hung the piano and the other the anvil.
The content crate now starts the cycle again at the activation
(`restart_heavy_object_cycle_on_activation`). Each owner resets its own
resource in `SessionScopeSet::Activate`, as the per-attempt ledgers do; they
are not members of `SessionScopedResources`.

- ⚠ The probe was the reason the first census could not read these. The row of
  `PortalFrameHistory` is a presence probe (its map iterates in no fixed
  order), so it read `(1, 0)` on each host. `RollbackChecksumProbes` now has
  `strengthen_resource_with`, which gives one resource a value projection for
  a test, and `PortalFrameHistory::len` is the value. `CutRopeHeavyObjectCycle`
  has a checksum, and the first walk did not replay the room.

**Witness of both:**
`shell_host_lifecycle::what_a_session_spawned_and_cycled_does_not_reach_the_next_session`
(two rooms, two successions; the peer census and three whole rows on each frame
of ticks 0..=30; premises: the old session ends with a placed portal, a shot in
flight and a frame in the history, or with the cycle off its default; the saves
are equal; each host starts in the room). It first asks the old session for
each room-scoped entity with no session owner. Poisons, one at a time: the
shot and the portal spawned with no scope (red at the owner check, which names
three `Portal shot` and `Portal: blue`); the history reset not registered (1
frame at tick 0); the cycle reset not registered (31 frames, ticks 0 to 30).
Unit witnesses, each with a no-activation control:
`eviction::tests::a_session_activation_forgets_the_last_sessions_portal_frames`
(the body is not pushed) and
`cut_rope::tests::a_session_activation_starts_the_heavy_object_cycle_again`.

**Found by measurement (2026-10-04).** A session is retired inside a schedule
run, and a route replacement (`ShellCommand::ReplaceWith`: a world reload, a
restart) retires one session and activates the next in one frame.

- **The clock pace crossed the edge.** `RequestedClockScale` and `ClockState`
  are peer-compared and were not in the session reset. On the shipped host, a
  session replaced during a hitstop left an asked pace of 0.0 and a live pace of
  0.42, and the next session ran its first two ticks at 0.65 and 0.88. A peer
  whose last session ended at the neutral pace does not. Both are now in
  `SessionScopedResources`, reset at the activation. The reset also changed a
  first session: in the sim harness, tick 1 ran at a pace of 0.96 (the frames
  before the session left the clock in a ramp), and it now runs at 1.0. One
  test had its premise satisfied by that tick only
  (`the_trace_records_one_clock_per_tick`); it now asks for the pace it sets,
  and its poison (the actor trace reads `ClockState`) fails it on tick 1.
- **Simulation messages were alive across the edge.** The population is the
  channels a domain declares to the rollback census (`clear_message_on_rollback`).
  A probe in both registrars, over `app_it` by module: 59 first ticks of a later
  session; at 10 of them (4 modules) one `ClockScaleRequest` and one
  `ActorActionMessage` of the old session were still on the bus. Both are
  written every tick. No other declared channel was alive there. ⚠ No reader was
  shown to consume one: with the clear off, the reader of `ClockScaleRequest`
  did not read the old requests again (it had read them in the old session),
  and no suite fails. So this repair is a structural guarantee and not the fix
  of an observed misread. The activation now empties every declared channel
  (`lifecycle::session_messages`), from the same declaration, so there is one
  list. ⚠ A presentation channel was kept until the same day's review; see
  "A presentation effect crossed the edge" below.

- **Three more peer-compared resources crossed the edge** (a census at each tick
  from 0, same day). Two hosts with EQUAL saves on the shell host under
  rollback: a session that followed another one (replaced in place, or through
  the title), and the first session of a fresh host that was given that save.
  `GatePortalPhases` was a mechanic: with the gate switch on, the portal of the
  session that followed was `On` from tick 0 and the fresh host's was `Opening`
  until tick 40, so for 39 ticks one peer could take the gate and the other
  could not. `WorldTime` held the old session's last step at tick 0.
  `OwnedItemsBaseline` held the old session's bag at tick 0 against zeros, with
  equal bags; it had been left out of the reset on purpose (2026-09-18), on the
  premise that only different save files could make it differ. The three are
  now in `SessionScopedResources`. The older two-host arm did not see them: its
  first reading is after `settle`, past tick 0.

**Witnesses:**
`id_peer_audit::a_new_session_starts_at_the_neutral_pace_with_an_empty_clock_bus`
(the shipped host; poison: no clock reset, the pace is `(0.0, 0.42)`; poison: no
clear, 2 requests on the bus), and three arms in `lifecycle::session_messages`
(a declared channel; a kept channel; a message the new session writes after its
activation is not taken). Also
`shell_host_lifecycle::a_session_that_follows_another_starts_as_a_fresh_hosts_does`
(the peer census on each frame of ticks 0..=48, two successions; premises: the
old session's portal is `On` and its last step is a hitstop step, the saves are
equal, the fresh host's portal opens inside the window). Poisons, one reset
removed at a time: `WorldTime` 1 frame at tick 0; `GatePortalPhases` 40 frames,
ticks 0 to 39; `OwnedItemsBaseline` 1 frame at tick 0.

**Host constants that cross the edge, recorded:** `FriendlyFire` and
`RegimePolicy` are rollback-registered, and a value written by hand before the
replace was still there at every tick of the next session (`RegimePolicy` in
the peer census). No production code writes either one (grep of `crates/` and
`game/`, non-test). `FactionRelations` is the third (2026-10-04): its only
value in production is the default, and with a value probe it was equal on
each frame of both rooms above. It has a row in the same guard. They are NOT in the reset, because they are the host's and
the world's configuration: `resolved_combat_tuning` sets `FriendlyFire` as "the
world's authored friendly-fire rule" on a composed host, and `RegimePolicy` is
the regime of the process. A reset to the default at each activation would
replace a configured value. The first system that writes one of them inside
a session makes it session state, and it must then join the reset;
`scripts/check_host_configuration_has_no_session_writer.py` fails when such a
writer arrives (no `&mut` to either type in production code, and each install
site is named).

**Not measured:** the other whole-state rows were scope ordinals
(`TransactionId`, `SessionScopedEntity`), a declared-derived cache
(`GatedLockWallCache`, tick 0) and a count of launcher entities (`Name`, tick 0,
replace flow only). The walk did not change `OwnedItems`; the changed bag is
measured below. Message
channels that no domain declares to the rollback census are not counted. A long
visit to the title carries no message: the bus was empty after 80 frames.

**A changed bag crossed the edge, and reached another experience's save
(2026-10-04).** The bag (`OwnedItems`) is one process resource and was not
reset. `restore_inventory_from_save` replaces it only when the save holds an
inventory; with none it keeps the live bag, which was the starting bag in the
first session of a process only. Measured on the shell host, a first session
that is granted one bomb through `ItemGrantRequested`:

- The same experience (replaced in place, through the title), against a fresh
  host with the veteran's save: the veteran's bag had the bomb at tick 0 and
  the fresh host's had not. From tick 1 they agreed, and the peer census
  agreed on each frame.
- Another experience (the veteran then enters Sanic), against a fresh host
  that enters Sanic first: the veteran's Sanic session had Ambition's bomb on
  each of 31 ticks, its mirror wrote the bomb into Sanic's save (11 items
  against 10), and the peer rows `AmbitionGameSave` and `OwnedItemsBaseline`
  differed on ticks 1 to 30.

The repair: each session begins with the bag that the process began with.
`StartingBag` (`items/starting_bag.rs`) records `OwnedItems` at `Startup`, so
the composition's own bag is the one authority (the Ambition content plugin
inserts the starter set; a composition that inserts none has an empty bag),
and `start_the_bag_again_on_activation` gives it back in
`SessionScopeSet::Activate`. The save of the session is then the only thing
that changes the bag. Witness:
`shell_host_lifecycle::a_bag_that_a_session_changed_reaches_a_later_session_through_its_save_only`
(premises: the first session has the bomb in its bag and in its save; the
Sanic save has no inventory at its activation), and
`items::starting_bag::tests::a_session_activation_gives_the_bag_that_the_process_began_with`.
Poisons: the reset not registered (the three arms above); the record runs in
`Update` (the unit arm: the starting bag follows the live bag).

- ⚠ A fixture prediction missed: a bomb is a unique item, so a grant of three
  gives one.
- ✅ 2026-10-04: the New Game restore (`session/checkpoint.rs`) stated the
  starting bag a second time, as `OwnedItems::starter(catalog)`. It now pins
  `StartingBag`. Witness: `a_new_game_gives_the_bag_the_composition_began_with`
  (an empty `StartingBag`, set with `StartingBag::of`, stands in for a
  composition that begins with an empty bag; control: the App's own bag).
  Poison: the starter set again; the empty-bag arm gets the starter items.
- ⚠ Recorded, not a defect of the edge: on a fresh host the first Sanic
  session has the Ambition starter set (10 items in its save), because the
  shell host is one composition with one bag.
- Review 2026-10-05, P4: `StartingBag` fixed the leak at the wrong lifetime.
  The starting bag is an authored initial condition of an EXPERIENCE, so it
  belongs in the prepared experience/session description beside the rest of
  its starting state (e.g. `PlatformerExperienceAuthoring::initial_inventory`):
  the candidate carries it, activation installs it, and a persisted inventory
  overlays it when the save has one. A one-experience App lowers its
  composition-level `OwnedItems` into its one experience. Do not grow more
  semantics around the process-wide value, and do not add Sanic or demo
  exceptions.
  **Done 2026-10-05 (NamekAmbition).** An experience declares its bag
  (`PlatformerExperienceAuthoring::with_initial_inventory`, recorded in
  `InitialInventories` at registration), the candidate session carries it, and
  the adoption installs it as the bag and as `StartingBag`
  (`StartingBag::begin_the_session`). The save overlay is as it was. An
  experience that declares none begins with an empty bag. Ambition's
  experience declares the starter set, and `AmbitionContentPlugin` no longer
  puts it into the App. `start_the_bag_again_on_activation` is deleted: one
  road. A New Game reads `StartingBag` as before, which is now the bag of the
  experience of the live session.
  - Measured before
    (`shell_host_lifecycle::each_experience_begins_with_its_own_bag`): the last
    session of each walk [Ambition], [Sanic], [Ambition, Sanic], [Sanic,
    Ambition] read (bag at its activation, bag 30 frames later, items in its
    save) = (10, 10, 10). After: (0, 0, 0) when the last session is Sanic's and
    (10, 10, 10) when it is Ambition's. Poisons: the builder reads Ambition's
    bag for each experience (each walk 10); the adoption installs no bag
    (each walk 0).
  - The direct road (`install_direct_session_root`, one session and no
    activation) keeps the record at `Startup`: the bag that such a composition
    was built with is the bag of its one session.
  - Not built: a shell App that lowers a composition-level bag into its one
    experience. No composition in the tree builds a bag now. A shell App that
    builds one, with no declaration on an experience, gets an error at the
    preparation of a session that names the declaration, and no bag.
  - Named limit: the bag is built from the item catalog that the App holds
    when the candidate is prepared, as the starter set was at App build. A
    content reload that changes item uniqueness between that moment and the
    adoption is not measured.
  - This removes the fact recorded above ("on a fresh host the first Sanic
    session has the Ambition starter set").
- Review 2026-10-05, P2 (assigned to NamekAmbition): a same-experience
  candidate is still prepared from the LIVE save (`prepare_the_save_of`
  returns `&live.0` when the owner already holds it), so the session that
  plays can change the save between preparation and adoption, and
  `hand_the_save_to` is then a no-op. Ruling: optimistic validation. The
  candidate remembers the save value it was prepared from
  (`AmbitionGameSaveData: Eq`); at adoption an equal save publishes it, and a
  different save makes it stale (discard and prepare it again from the new
  save). Do not restore the old save at adoption: that throws away progress
  made while the outgoing session was playable. Witness: change a durable
  fact that construction reads between preparation and adoption, and compare
  with a fresh candidate prepared from the later save.
  **Done 2026-10-05 (NamekAmbition).** The candidate keeps the save value it
  was built from (`PreparedFromSave`, read back through
  `ambition_persistence::save::the_save_of`). `candidate_session_gate` compares
  it first, before the verdict of the first room: a stale candidate is
  discarded (`session-candidate-stale` in the world log, counted by
  `CandidateSessionSlot::stale_discards`), the answer is `Hold`, and the next
  frame prepares a new candidate from the save as it is.
  - **The `Q118` race in a second shape, found by the witness.** In a host
    with no other hold, the candidate is prepared and adopted in one frame,
    so nothing can change between them. A hold with no gate on the same route
    (the loading screen's) opens the interval, and there the gate said `Admit`
    once, its hold was released at once, and the route activated some frames
    later with no second question. `advance_pending_route` now asks each gate
    on each frame that the route waits, and releases the holds of the gates
    only on the frame where each gate says `Admit` and no other hold is on the
    route. `answer_the_publication_gate` (content reload) gets the same rule
    through the router.
  - **The lease breaker beside that gate, measured 2026-10-05**
    (`an_edit_reaches_the_shipped_game::a_reload_whose_boundary_closes_while_it_waits_is_cancelled_whole`,
    the shipped app, each owner removed in turn). Its witnesses in
    `reload_tests.rs` write `RouteActivated` by hand, with no router and no
    gate, so they say nothing about the composed host. There: a boundary that
    is closed on a frame that the route is ready (a foreign timeline or a
    recorded divergence, for one frame or to the end) is cancelled whole on
    the next frame by the breaker, and by the gate alone with the breaker
    removed: the same state (not activated, the old generation, nothing
    staged, the two holds of the transaction released). With the two removed
    each arm publishes, a recorded divergence too. The breaker is the one
    owner of the frames before the route is ready (a slow preparation): closed
    after the adoption and open again before the ready frame, the gate does
    not see it and the reload publishes; closed to the end, the gate cancels
    at the ready frame, 6 frames later in the fixture. So
    `break_the_publication_lease_when_the_boundary_closes` stays: it is the
    transaction-lifetime lease of the `Q118` ruling, and the gate is the
    question at the activation. A boundary that closes and opens before the
    adoption is seen by neither, and nothing is owed then. One prediction
    missed: that a boundary closed for one frame at the adoption publishes with
    the breaker removed. The route is ready on the next frame and the gate
    sees it. Not measured: whether a recorded divergence can become healthy
    again inside a preparation (a lifecycle commit starts a new timeline), and
    the frame interval after the breaker that
    `a_boundary_that_closes_after_the_breaker_still_publishes` records in a
    fixture with no router.
  - Witnesses: `an_admit_is_consumed_only_by_the_activation` (shell lib; four
    of its arms were red before the router change, and its control is a gate
    that stays `Admit`), arm 5 of
    `the_candidate_gate_refuses_rather_than_retiring_a_playing_session`, and
    `app_it`
    `a_session_that_replaces_its_own_experience_is_built_from_the_save_at_its_adoption`
    (control; the save changes on the frame that releases the route; the save
    changes 3 frames before). The adopted session is equal to a fresh host of
    the later save on each of 31 frames, with 1 stale discard and the scope
    after the discarded one. Each poison alone (no equality check; the earlier
    router) gives the world of the earlier save, 0 discards, on both arms.
  - Two predictions missed and are recorded here. (1) "A frame is between the
    preparation and the adoption in the headless host": there is none. (2) The
    first fixture wrote a `Consumed` occurrence row into the live save. The
    session that plays writes those rows from its own ledger, so the row was
    gone one frame later, and the adopted session was correct for the save at
    its adoption while the test compared it with another save. The fixture
    now changes a flag.
  - Residual, named and not fixed: a save that changes on each frame holds
    the route for as long as it changes. No save field does that today; one
    that did would be its own defect.

**The per-attempt ledgers crossed the edge (2026-10-04).** Mary-O's
`BrokenBricks` and `SpentPowerBlocks` and Sanic's `SpentMonitors` are keyed by
the live room (`PerLiveRoom`). The live-room counter is the session's
(`RoomSet::next_live_room`), so the first room of each session is
`LiveRoomInstance(0)` again. The re-arm keeps the state of a live key, so a
brick broken in one Mary-O session was broken in the next, through the
launcher and back, and a Sanic monitor too. `install_attempt_scoped` now also
registers `forget_attempts_on_activation` in `SessionScopeSet::Activate`, with
no condition, so the one statement that makes a ledger per-attempt also makes
it per-session. Witnesses:
`shell_host_lifecycle::a_block_broken_in_one_session_is_whole_in_the_next`
(the shipped rollback host; it reads the live room's collision overlay; the
premise asserts that the second session's first room has the first one's key),
`bricks::tests::a_session_activation_forgets_the_broken_bricks` (control: a
frame with no activation keeps the brick), and the two composition tests of
`attempt_scoped_retraction` (each ledger, both plugin roads). Poison (the reset
writes nothing): the Mary-O arm, then the Sanic arm with the Mary-O assert made
non-fatal, and the unit test are red. Poison (the system out of the set): both
composition tests are red, at their `SetNotFound` expect.

**The frame on which the timeline starts is a decision (2026-10-04).**
`LocalSessionSet::Maintain` had no edge to `GameplaySessionSet::Providers`, the
set that builds the session world, so the sort chose the order. Before the
merge of this row's system (in `SessionScopeSet::Activate`) with the HUD of
each view (Q150), the shipped host ran the providers first, and the rollback
session came up in the `Update` that built the world. After it, the maintainer
ran first and the session came up one frame later; the floor of
`versus_stage::the_roster_arm_writes_the_scoreboard_before_the_timeline_starts`
went red because no firing frame installed a session. A frame on which the
world exists and the session does not is a frame outside the timeline, so a
composer that installs the two sets now states the order:
`rollback::start_the_timeline_with_the_session_world` (in the facade, the
lowest crate that sees the two sets) puts the maintainer after the providers.
Each composer of the Ambition shell host calls it, and `PlatformerApp` calls it
for a rollback composition.

- The versus test asks that every firing frame installs the session. Poison
  (the edge removed): its floor is red.
- `reload_publication_is_installed` recorded that nothing ordered the session
  start against the generation commit (Q118). The edge orders them on one
  frame (the commit is before the providers), and the arm now asserts that
  order. A timeline that starts on an earlier frame of the wait is still the
  case of `break_the_publication_lease_when_the_boundary_closes`.
- `shell_host_lifecycle`'s misordered-retirement arm puts the maintainer before
  the session bridge. With the host's edge that is a cycle, so that arm alone
  uses `compose_ambition_shell_host_with_the_timeline_start_unordered`.

- The SDK composition has the edge:
  `versus_through_the_sdk::an_sdk_rollback_host_starts_the_timeline_with_the_session_world`
  reads the path in the schedule graph and, with the maintainer allowed to
  start, the session at the end of the frame that built the world. Poison (the
  call removed from `PlatformerApp`): the graph half is red.

**Not covered:** the pocket demo composes the shell with no rollback backend,
so it has no maintainer to order.

### LEDGE-OCCUPANCY — two fighters can hold one ledge

**Owner:** `ambition_combat::ledge_trump`, `ambition_platformer2d_core`
ledge grab. Plan:
[`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md#ledge-occupancy).

**Ruling:** Q43 follow-up (2026-10-04): the target is Super Smash Bros.
Ultimate-like occupancy and trump, deterministic and rollback-compatible.

**Current failure:** "one edge" is two hang anchors within 1 px, and an anchor
depends on the body's size, so two fighters of different sizes both hang on one
corner. No ledge occupant exists.

**Next action:** write down Ultimate's ledge rules (researched, not recalled)
in the plan, then key occupancy by the ledge, not by a body's anchor.

**Acceptance:** with two fighters of different sizes on one corner, one holds
it; trump, release, death and knockoff each free or transfer the hold as the
plan says; a rewind across a trump gives the same holder.

### HAZARD-BEATS-LEDGE — a hanging body is immune to hazards

**Owner:** `ambition_platformer2d_core::movement` (hazard gate) and
`ambition_combat::hazards`. Plan:
[`engine/combat-model.md`](engine/combat-model.md#hazards-beat-a-ledge-hang-q43).

**Ruling:** Q43 (2026-10-04): a hazard wins over a ledge hang.

**Current failure:** the kernel hazard gate skips a frame the ledge grab
consumed, and hazard volumes respect the ledge-grab intangibility window.

**Acceptance:** a body hanging over a lethal hazard dies on both roads; the
same hang without a hazard holds; an attack in the grab window still misses.

### HALL-STILL — every Hall actor stays where it is placed

**Owner:** `ambition_platformer2d_core::movement::adhesive_crawler`. Plan:
[`../concepts/hall-of-characters-is-not-special.md`](../concepts/hall-of-characters-is-not-special.md#what-the-hall-is-for-q85-2026-10-04).

**Ruling:** Q85 (2026-10-04): the Hall's stationary policy holds for every
showcase actor through one population policy.

**Current failure:** a Puppy Slug (`npc_puppy_slug`, the one
`surface_walker`) crawls under a `stand_still` brain, because the crawler's
pace comes from its policy and it never reads the commanded axis.

**Acceptance:** a test steps the generated Hall and finds every spawned actor
where it started; a crawler with a patrolling brain still moves (control).

### MIRROR-SYMMETRY — mirrored CPUs stay mirrored per tick

**Owner:** `ambition_combat::brain` and the systems it reads. Plan:
[`engine/fighter-brain.md`](engine/fighter-brain.md#mirror-symmetry-is-a-correctness-property-q49).

**Ruling:** Q49 (2026-10-04): symmetry is a correctness property; variation
comes only from modelled asymmetric facts.

**Current failure:** the Emmy test compares positions only, accepts a break at
the first grab, and asks only 1.5x the ordinary rate; known asymmetry sources
(`signum(0)`, `SimId` tie-breaks, a left-first recovery search, a 69% seat-0
term) are untriaged.

**Acceptance:** a per-tick reflection test of position, velocity, facing,
move, decision and stream position; a reflected-observation unit test of the
decision layer; each poison listed in the plan turns one of them red.

### WEAPON-READINESS — a refused trigger is visible as "not ready"

**Owner:** `ambition_sim_view::control_prompt` and the fire roads. Plan:
[`engine/participant-action-system.md`](engine/participant-action-system.md#p5--weapon-readiness-is-a-semantic-state-q33).

**Ruling:** Q33 (2026-10-04).

**Current failure:** a blocked shot is dropped with no fact; readiness is one
boolean for one prompt slot.

**Acceptance:** fire during a cooldown makes no shot and no success
presentation, and publishes `recharging` with progress; fire after it shoots.

### RIG-LANDMARKS — interactions read authored landmarks

**Owner:** character package and `ambition_characters` body rig. Plan:
[`engine/runtime-rigged-sprite-animation.md`](engine/runtime-rigged-sprite-animation.md#semantic-landmarks-q41).

**Ruling:** Q41 (2026-10-04).

**Current failure:** the pet gesture, the player fireball and the rider's hand
are placed from boxes and constants.

**Acceptance:** one landmark query, answered by the rig or the package; the pet
hand meets the petted body's authored contact point.

### RIG-IMPOSTOR-CONTAINMENT — a part-drawn body is drawn whole or refused

**Review 2026-10-05 (carried forward, not fixed):** cells are still
`frame_size + 2 * IMPOSTOR_MARGIN`, the parity oracle still clips to the cell,
and a page is drawn through one camera with no per-cell scissor. The
invariant: every pixel an admitted rig can rasterize lies in its cell.
Compute a conservative asymmetric draw envelope at preparation (frames,
mirroring, transforms, tween and rotation), choose the cell class from it,
and fall back to the baked sprite when no class holds it. Then remove the
oracle's clip and add a two-body shared-page containment test.

**Owner:** `ambition_render::rendering::actors::rigged` (`impostor_cell_class`)
and `scripts/measure_rigged_parity.py`. Plan:
[`engine/mary-o-part-realization.md`](engine/mary-o-part-realization.md).

**Current failure (review 2026-10-04):** the cell class is chosen from the
flipbook's `frame_size` plus a margin. Parts can draw outside the frame: a
banner past the cell of the oni leader that faces left. Such parts are cut
at the cell edge. The parity script clips its oracle to the same cell, so
the gate cannot see the cut. `every_published_flipbook_fits_an_impostor_cell`
checks the frame size, not where the parts draw.

**Next action:** compute a conservative draw envelope per flipbook (every
draw of every frame and mirror row, with tween in-betweens) at publication.
Choose the cell from the envelope, or refuse admission to the part road when
no class holds it. Then remove the oracle clip, so that a cut part reads as
a parity failure.

**Acceptance:** the oni leader's banner is drawn whole while it faces left; with
the envelope poisoned back to `frame_size`, the unclipped parity run fails.

### CHARGE-SPEC-NAME — `SmashChargeSpec` is a generic mechanism

**Owner:** `ambition_entity_catalog` (`SmashChargeSpec`, field `smash_charge`).

**Ruling:** Q44 (2026-10-04): no leaf-game name on a generic API.

**Current state:** 21 Rust references; 7 RON sites, including the Performer
and Projectile Polygon, not only Smash fighters. The sibling names
(`MoveCharge`, `ChargeSustain`, `ChargeGesture`) suggest `MoveChargeSpec` /
`move_charge`. `ChargeGesture::Smash` names the gesture and stays.

**Acceptance:** no `SmashChargeSpec` in source; the content fingerprint change
is the only content change.

### MENU-HOVER — hover is a third menu state

**Owner:** `ambition_menu::render::bevy_ui`. Plan:
[`engine/ui-localization-and-accessibility.md`](engine/ui-localization-and-accessibility.md#hover-is-a-third-state-q70).

**Ruling:** Q70 (2026-10-04).

**Current state:** the flat renderer draws hover as its own state. One rule,
`hover_lift`, lightens the tab or control fill. `sync_bevy_ui_menu_hover` is
the only writer of `MenuVisualState::hovered`, and the in-place restyles
follow it. The launcher ignores `MenuActionPreviewed`, so a hover does not move
`ShellLauncherState::selected`. Tests: `ambition_menu` `bevy_ui::tests`
(`a_hovered_tab_draws_the_hover_style` and the hover tests after it) and
`ambition_game_shell` `pointer_hover_tests`.

**Next action:** the in-game Grid menu still moves its cursor on hover
(`grid_menu_pointer_hover` in `game/ambition_app/src/menu/grid_backend.rs`
calls `cursor.mark_keyboard`). Make that hover presentation-only too; keep
pointer activation.

**Acceptance:** a hover over a Grid menu row leaves `KaleidoscopeCursor`
unchanged, and a click on the row still activates it.

### DENSE-MELEE-ROOM — author the dense-melee development room

**Owner:** content; first measured customer
[`engine/bounded-perception-and-attention.md`](engine/bounded-perception-and-attention.md#acceptance-for-the-open-increment).

**Ruling:** Q93 (2026-10-04): keep or create a deliberate dense-melee
development/stress room. It is a real scenario, not polished content.

**Current state:** no such room; the density script fakes density by widening
the viewport over the Hall's `stand_still` cast.

**Next action:** after the P1 correctness rows above, author one room (LDtk or
a generated spec) with many tactical-brain fighters in close melee, reachable
by an ordinary route and by `measure_perception_density.sh`.

**Acceptance:** in the room `kept` tracks population until the budget caps it,
and the census, frame time and camera framing are readable from one run.

### LDTK-SEMANTIC-DIFF — review content diffs by meaning, and find the rewrite

**Owner:** `tools/ambition_ldtk_tools` (`edit/semantic_diff.py`). Plan:
[`engine/authoring-and-tools.md`](engine/authoring-and-tools.md#content-diffs-need-domain-aware-comparison-q62-q78).

**Ruling:** Q62 and Q78 (2026-10-04) are engineering and evidence tasks, not
maintainer choices.

**Current state:** the comparison is done: `diff semantic` (file or
`REV:PATH`), `diff range` and `diff normalize`, with verdict, per-level
summary, ambiguities and a noise section (tests:
`tests/test_semantic_diff.py`). It prints the per-level summary for the
post-fix rewrite commits `576a8fd`, `c6df2b7` and `056079f`. `48f8e26` is not
in this clone, so `48f8e26 → cb7062a` is not re-measured. LDtk files still
rewrite far beyond their edits after `54d99e7fb` and `2e69e81b9`; the writer
is not identified.

**Next action:** run each LDtk writer twice on an unchanged input and compare
the two outputs with `diff semantic`; the writer whose second run is not
`identical` is a rewriter.

**Acceptance:** a second run of each writer on its own output changes no
byte.

### ASSET-PRODUCT-LAYOUT — runtime roots by tier; editor products apart

**Owner:** `ambition_asset_manager` path builders, `ambition_sprite_sheet`,
the publish scripts. Plan:
[`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md#products-are-laid-out-by-what-they-are-q82-q83).

**Ruling:** Q82, Q83 (2026-10-04).

**Current state:** full quality is the bare `sprites/` root and the reduced
tiers are suffixed siblings; the packager ships every file, including the
98.8% of the 449 MB ultrapack that no runtime road reaches.

**Next action:** audit the listed path builders, then move to
`<root>/{full,half,quarter,potato}/` in one change; then cut the ultrapack's
runtime product by runtime unit.

**Acceptance:** every tier is a named directory; the packager selects roots
and has no new exclusion; the packaged ultrapack holds only reachable pages;
the game draws the same at each tier.

### PORTRAIT-TIERS — portraits scale with quality and stay usable

**Owner:** `ambition_sprite_sheet` (`bake_portrait_manifests`) and the dialog
portrait consumer. Plan:
[`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md#quality-is-a-presentation-policy-q84).

**Ruling:** Q84 (2026-10-04).

**Current state:** reduced portrait tiers are generated and shipped, and
nothing loads them.

**Acceptance:** each quality tier loads its own portrait product, sized from
the dialog draw size; potato portraits are smaller than full and still fill
their box.

### TEST-LANES — keep required test lanes executable

**Owner:** test runner / app integration lane. Operational rules and what a
green lane does not clear:
[`running-the-heavy-app-it-lane.md`](../recipes/running-the-heavy-app-it-lane.md).
How a check can fail to run: [`checks-that-did-not-run.md`](../recipes/checks-that-did-not-run.md).

**Current state:** the `app_it` lane runs (`713 passed / 0 failed / 45 ignored`
on 2026-09-17). Cargo diagnostics are read through
`scripts/lib/cargo_output.py`, which disables colour and strips ANSI codes, and
`scripts/tests/test_cargo_diagnostics_are_read_plain.py` holds every script that
reads cargo output to it.

**Open items:**

1. **The compile-cost ratchet fails the full gate** (`scripts/compile_ratchet.py`, measured 2026-09-18). Its baseline records commit `b3bd00a4a` (2026-09-05), which no ref reaches, and it disagrees with itself in three places. Over budget: `ambition_platformer2d_actor_monolith`'s largest unit (100,742 → 115,105 lines) and edit cost, and `ambition_geometry`'s worst edit cost (94.9% of the workspace). ⛔ Do not re-freeze to go green. Next: find which part of the monolith's largest unit belongs in its own crate, and repair the baseline's self-disagreement before any deliberate re-freeze. <!-- cite-ok: `b3bd00a4a` is quoted BECAUSE it resolves nowhere; it is `dev/compile_ratchet_baseline.json`'s own recorded `commit` field -->
2. **An arm fails only in company** (see [the triage page](triage/a-composition-acceptance-that-only-fails-in-company.md)): `composes_through_the_sdk::a_host_that_omits_boss_encounters_still_builds_and_steps` failed once on 2026-09-10, and its assertion was never captured. Three other instances of the signature were per-arm measurements reading process-global state (`app_it` runs arms as threads of one process); they are fixed, and `scripts/a_test_static_is_a_channel_between_arms.py` guards the class. Next: capture this arm's assertion. The next candidate of the class is `hall_redecode_census.rs`, which asserts over a delta of a process-wide counter. ⛔ Do not add a retry.
3. **One older session-root handoff failure** did not reproduce in four full runs, and its assertion was never captured. Both candidate arms (`the_shipped_app_never_holds_two_session_roots_across_a_handoff`, `a_candidate_session_replaced_while_pending_is_discarded`) assert their own premises, so a new failure carries its cause. The next step is not more runs.

4. **`NOT RUN` is a first-class receipt state** (Q59 ruling, 2026-10-03). A
   ledger or receipt must tell PASS, FAIL, "not run, not required now" and "not
   run, required at this boundary" apart; a gate blocks only where its policy
   requires it at the current boundary. Next: find the receipts that collapse a
   lane that did not run into PASS or FAIL, and give each lane its cadence in
   [testing and validation](../concepts/testing-and-validation.md#validation-states-and-cadence).

The published-sheet floor in `ambition_sprite_sheet` (780 below a floor of 800
on one checkout) is machine state. ⛔ Do not lower the floor.

**Acceptance:** the failing population is reproducible or explicitly classified,
and the production cause is fixed or the harness proves why the failure is not a
production invariant.

## Receipts

Closed rows that an open row, a script or an inbound link still names.

### NPC-UNREGISTERED-CHARACTER — a person who names an unregistered character stopped the game — ✅ DONE 2026-10-05

**Owner:** `construction::preflight_planned_bodies`
(`ambition_platformer2d_actor_monolith`).

**Found** 2026-10-04 on the world reload road: an `NpcSpawn` whose
`character_id` is in no cast and no catalog passed LDtk validation and plan
preparation, and the recipe panicked when the room was built
(`report_unprepared_character`, `actor_spawn/character_spawn_plan.rs`). A
person is a placement row, and the preflight read no placement row. An
`EnemySpawn` with the same fault was already refused when its room was planned.

**Done:** the preflight takes the character catalog and refuses a person in
the one case the NPC road cannot build: the cast does not have the character,
a cast is published, and no catalog row can give it a body. It is the enemy's
refusal (`BodyCharacterNotRegistered`), and the room error names the room and
the character. The three cases the NPC road can build stay plans: a character
that only the catalog has (a borrowed kit), a person who names nobody, and a
composition that published no cast (a warning).

**Witnesses:**
`construction::tests::a_person_who_names_an_unregistered_character_is_refused_when_the_room_is_planned`
(the refusal and the four controls), and on the shipped session
`a_world_reload_that_cannot_rebuild_one_live_room_rebuilds_none`
(`an_edit_reaches_the_shipped_game.rs`): an enemy in the other live room (the
control, refused before and after), a person in the other live room, and a
person in the one live room. Each reload is refused with the room and the
character named, and no live room and no generation changes. Poison: the
refusal removed fails the unit arm, and the two person arms panic as before.

**Not done:** the assertion in `report_unprepared_character` stays. A road
that builds a person with no plan (none is known) would still reach it.

### NEW-GAME-RESYNC — a New Game after durable hydration fails the sync test — ✅ DONE 2026-09-29

A New Game is a checkpoint resume to the fresh baseline at the start room, on
the confirmed-frame road the death restore takes (`resume_at_checkpoint_on_reset`
reads `NewGameRequested`; the fresh-run reducers run inside
`CheckpointDomainApply`). ⛔ Do not register the four `derived` body types for
rollback to hide a rebuild on a speculative frame. Witness:
`a_new_game_asked_for_by_the_host_commits_once_under_a_rewind`.

### A10 — candidate world / last-good-world publication — ✅ DONE, DEMOLITION CLOSED 2026-09-16

A failed candidate world leaves world N playable and unchanged, and N+1 is
verified before N is retired. Every road that changes the authoritative world
crosses its own publication verdict. Owner and witnesses:
[construction and reconstitution](engine/construction-and-reconstitution.md).
⛔ Not part of A10: peer-stable identity (ID-PEER), the defensive
`DepartureAuthority::Custodian` fallback, and the refused-door player signal.

### CUTSCENE-ROLLBACK-DECISION — two session-scoped cutscene values cross into simulation with no rollback decision — ✅ DONE 2026-09-28

The cutscene dismiss and skip are seat intents on the `ControlFrame`, which
`tick_active_cutscene` reads from `SlotControls`, and `CutsceneSkipHold` is
rollback state. The host-side `Update` request is deleted. Guards:
`a_cutscene_dismiss_on_the_seats_input_survives_the_rewind_once` and
`a_cutscene_skip_is_a_hold_of_the_seats_cancel`.

### ROLLBACK-KIND-SPELLING — one registration, one kind, spelled once — ✅ DONE 2026-09-16

`ambition_platformer2d_core::rollback_kind::spelling` holds every (kind,
sentence) pair once, and both registration roads reference it. Guard:
`scripts/check_rollback_kind_spelled_once.py`.

### A12 — finish move-contact attribution and reflection identity

✅ DONE 2026-09-19. A late projectile or melee outcome, a reflected shot and an
ability contact cannot credit the wrong move occurrence. An outcome that names
another occurrence, or none, credits no move (ruled: an ability contact is
independent by default). Witnesses: `an_outcome_naming_another_occurrence_credits_no_move`,
`an_unclaimed_outcome_credits_no_move`,
`a_move_occurrence_reaches_the_same_number_with_and_without_a_rewind`. ⛔ The
frontier's `A12` (flow validation) is a different subject.

### ROLLBACK-BAG-DESYNC — `AmbitionGameSave` disagrees with its own rollback replay — ✅ REPAIRED 2026-09-16, acceptance MET; the authority/representation split is DEFERRED

The three live→save mirrors ran in `Update` and wrote a hashed resource that a
replay did not re-derive. They register through `app.sim_schedule()` now. Guards:
`no_hashed_entry_disagrees_with_its_replay_when_the_bag_moves` and
`the_saves_hashed_snapshot_tracks_the_frames_it_is_compared_at`. ⛔ Do not remove
`AmbitionGameSave` from the checksum: most of its writers run inside rewinding
schedules. Deferred: the authority/representation split. Q129 (decided
2026-10-03) keeps shared durable state in what peers compare, by its canonical
semantic form rather than the save's serialization.

### MENU-RESET-MIDSESSION — the menu writes rollback state from `Update` — CLOSED 2026-09-19

Menu presses reach the simulation through `HostIntentLedger<M>`
(`crates/ambition_platformer2d_actor_monolith/src/session/host_intents.rs`),
released at the head of the stamped tick in every pass. Ruled (`Q140`): one frame of stale UI is acceptable. ⛔ Do not add
duplicate authoritative inventory state or optimistic reconciliation to hide it.
Witness: `a_health_cell_used_from_the_menu_heals_once_and_spends_one_cell`.
