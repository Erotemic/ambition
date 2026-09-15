# The queue — live execution order

This file is the **current executable engineering queue**. It is not a work log,
review transcript or archive. Git history owns completed investigations. Durable
design and measurements belong in the linked owner document. Product decisions
belong in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

A row remains here only while an engineer can act on it. When it closes, keep a
short receipt only where another open row depends on that fact; otherwise remove
it.

## P0 — architecture and correctness

### A10 — candidate world / last-good-world publication

**Owner:** [construction and reconstitution](engine/construction-and-reconstitution.md).

**CURRENT INVARIANT.** A failed candidate world leaves the currently playable
world N intact, at BOTH scopes and UNCHANGED — not merely playable. A candidate
N+1 is prepared and verified off to the side; only a validated candidate becomes
authoritative; N is retired only after that publication succeeds.

**CURRENT HEAD BEHAVIOUR (2026-09-15).** **CLOSED.** Every road that can change
the authoritative world crosses its own publication's verdict — the room
transition, the reset, the death reconstruction, the shell handoff and the dev
LDtk reload, each with a refusal arm and an admission control in `app_it`. There
is ONE road into a live session and ONE publication authority per level, with no
mode flag: the candidate-bracket selector is deleted rather than frozen on. A
verified publication also FREEZES what it owes the world outside its own
population, so a room published inside a pending candidate session announces
nothing to the live one until that session is admitted.

The five ownership contracts named by the 2026-09-15 holistic audit:

| # | Contract | State |
| --- | --- | --- |
| 1 | Nested non-entity/effect ownership | CLOSED — `FrozenPublicationEffects`, consumed by `finalize_room_publication` |
| 2 | Exact per-publication custody ownership | CLOSED — `CustodyHandoffs` on the exact `RoomPublication` |
| 3 | Candidate-owned minted reconstruction input | CLOSED structurally and behaviourally |
| 4 | True pre-construction refusal | CLOSED — `construct_room_candidate`, witnessed by an insertion hook |
| 5 | One exact verification-and-application target | CLOSED — one entity through both ends, every sink preflighted |

**NEXT IMPLEMENTATION STEP.** None. Post-A10 demolition is the active lane:
delete the mechanisms A10's replacement made dead, and keep the consolidation
control plane matching source. ⛔ Not part of A10: peer-stable identity
(ID-PEER's row below), the defensive `DepartureAuthority::Custodian` fallback,
and the refused-door player signal — the last is presentation policy awaiting a
product ruling, not last-good-world correctness.

**ACCEPTANCE CRITERIA.** A production composition demonstrating (a) failed
candidate construction/publication leaves the last-good world playable AND
unchanged, and (b) successful replacement validates N+1 before retiring N. ⇒
**MET at both scopes**, in `game/ambition_app/tests/an_edit_reaches_the_shipped_game.rs`
and `death_restores_the_checkpoint.rs`.

**ACTUAL BLOCKER.** None.

**RECEIPTS.** The measurements, the witnesses, the three instrument defects that
made two of them pass before they worked, and the investigation that closed the
session scope are in the owner document — see its *"2026-09-15 holistic audit"*,
*"decisive acceptance statement"* and *"How the session scope got closed"*
sections. ⛔ They are NOT repeated here: this row had grown to 694 of the queue's
1048 lines, which made the live executable queue two thirds one closed campaign's
diary.

### ID-PEER — remove host-local lineage from peer-stable mechanical identity

**Owner:** deterministic identity / rollback architecture; see the identity map in
[`consolidation/architecture-census.md`](consolidation/architecture-census.md).

**Current state (2026-09-15):** the roads by which host-local identity reached
peer-compared state are being closed one at a time. ⛔ **A GPT architecture
review on 2026-09-15 found that the first attempt replaced one host-local term
with another**, and the table below is written to be re-checkable rather than
reassuring.

| road | state |
|---|---|
| smash random-roster seed | **CLOSED** — `agreed_match_seed` hashes the agreed lobby. Its first version still hashed the local input device INDEX; that is fixed and asserted |
| `SessionScopedEntity` in the peer checksum | **CLOSED** (schema 184) — probed clone; still snapshotted, because the construction scope gather reads it |
| the four `MatchInstance`-stamped resources | **CLOSED** (schema 186) — `ActiveMatch`, `StocksMatchSettled`, `SuddenDeathEntered`, `LiveMatchTicks` compare MECHANICAL FACTS ONLY (agreed seat count, verdict, latched-or-not, micros elapsed from the match's own start). All four still snapshot whole |
| `MatchInstance::random_context` | **OPEN, RECORDED** — still mixes the activation tick; see below |
| checkpoint operation keys | **OPEN, newly found** — `CheckpointOperationKey::write_into` writes the raw `SessionScopeId`, and four carriers put it in a peer checksum |
| `TransactionId` provenance | **OPEN** — `ContentEpoch` + `SessionScopeId` |
| the canonical timeline itself | **OPEN, and the largest** — see below |

⛔⛔ **`SimTick` IS AN ABSOLUTE PER-APP COUNTER AND IT IS ALREADY A WHOLE-VALUE
PEER CHECKSUM INPUT.** Measured 2026-09-15: one writer
(`ambition_time::advance_sim_tick`, `+1` per step), `init_resource`'d once at
App build, never rebased anywhere in the workspace, and registered
`resource-canonical`. It sits UNCONDITIONALLY at the head of the sim schedule,
so it counts menu frames. ⇒ **Two Apps that have been running for different
lengths of time disagree about `sim_tick` from the first compared frame**, before
anything else in this campaign matters. Everything keyed on it inherits that:
the first fix here moved four resources onto the activation tick and called it
peer-stable, which was the same error one layer down.

⚠ **`random_context` KEEPS the tick deliberately**, recorded rather than fixed.
Removing it with no replacement makes every match in a run replay the first
match's item drops — a visible regression pinned by
`two_activations_are_two_draw_contexts`. A checksum is compared every frame, so a
false desync there is fatal; a repeated item table is not. ⇒ **The replacement is
the match's ORDINAL WITHIN THE AGREED SESSION**: zero for everyone who joins
together, insensitive to menu time and prior sessions, and it still separates
consecutive matches.

⚠ **AND NOTHING IN THE REPOSITORY CAN CURRENTLY OBSERVE ANY OF THIS.** The only
sessions in use are `SyncTestSession` — one machine rewinding itself, zero
distance. A desync canary that compares a machine against its own past is
structurally incapable of catching a two-peer disagreement, which is why every
leak in the table above had to be found by reading.

**The standing guard, and what it could not see.** `id_peer_audit.rs` reads the
LIVE registry. ⛔ Until 2026-09-15 its population was a list of variant names
kept in the test, which omitted every `*CustomChecksum` kind — **25 of the 29
registrations that feed a peer checksum were never examined**, the checkpoint
family among them. The question now lives on `RollbackEntryKind` itself as
`feeds_peer_checksum()`, where a new variant cannot be added without answering
it. Poison-verified in both directions: claiming a custom-checksum kind does not
feed the checksum reddens the guard and names what it stopped covering.

⚠ The review asked for this to be settled *before* A10 made transaction provenance
more central. That did not happen: A10's room scope landed first, and a publication
now declares the lane `TransactionId`s it owns (`PublicationEffects::owned_by`),
with `CandidateNotOwned` refusing anything stamped outside them. A10 deliberately
used the existing interfaces rather than hardening host-local lineage into a new
provenance contract, so the dependency stayed narrow — but it is wider than it was.
`a_transaction_identity_still_depends_on_host_local_lineage_counters`
(`shared_tangle/src/construction/tests.rs`) records the divergence and flips the
day the identities are split.

**What the acceptance has, and what it lacks.** A two-App witness exists:
`two_differently_aged_hosts_publish_the_same_roster_through_the_shipped_road`
(`game/ambition_app/tests/id_peer_audit.rs`) builds two real Apps with
`build_visible_app`, ages one by extra select-route visits, asserts their
activation counters DIFFER, and asserts the shipped match-start road publishes
the same roster from both. ⚠ It ages the host along the SHELL ACTIVATION axis
only. The construction poison the review asked for — A burns a candidate content
epoch, B does not, both construct identical content, canonical snapshots must
agree — is a different axis and is still unwritten.

**Next implementation, in order.** (1) The peer-agreed match ordinal, which
closes `random_context` without the gameplay regression. (2) The peer/local split
on `CheckpointOperationKey` — the scope's stale-operation job is local and real
and must NOT simply be deleted; `SessionCheckpointOperations` already advances
its sequence only on ADMISSION for exactly this reason. (3) `TransactionId`:
drop the session term and replace `ContentEpoch` with a peer-stable content
fingerprint. (4) The timeline itself — a session-relative tick, which is netcode
work and wants a maintainer decision before anyone starts.

⛔ Do NOT continue by mechanically replacing each raw `SessionScopeId` with the
nearest canonical-looking value. `SimTick` is why: it looks canonical, it
rewinds, it is already checksummed, and it is host-local.

**Acceptance:** two Apps that have burned different numbers of local session
activations can enter the same deterministic match and produce the same canonical
mechanical identity/checksum. The witness must first assert that their local
counters differ.

### ROLLBACK-KIND-SPELLING — one registration, one kind, spelled once

**Owner:** rollback registration (`platformer2d_runtime/src/rollback/registrar.rs`
and `platformer2d_rollback_ggrs/src/registration.rs`).

**Current state:** every registrar method spells its `RollbackEntryKind` TWICE —
once on the RECORDING road (`runtime`'s registrar, which writes the descriptor
the schema baseline and every census read) and once on the INSTALLING road
(`rollback_ggrs`, which adds the snapshot plugin and checksum system). Nothing
derives one from the other.

⛔ **MEASURED 2026-09-15, BY MAKING THE MISTAKE.** Splitting
`resource-canonical-custom-checksum` out of `resource-canonical`, I changed the
recording road only. The result was one registration arriving under two
different kinds, caught at app build by `RollbackRegistry`'s
conflicting-registration check — which is accidental cross-evidence, not a
designed guard, and **covers only names BOTH roads reach**. A kind spelled
wrongly on a registration that only one road installs has nothing checking it.

**Next implementation:** give each registrar method ONE kind, named where the
method is declared rather than at each call of `descriptor::<T>` /
`record::<T>`. Do not add a third table mapping method names to kinds; that is
the same duplication with an extra hop.

**Acceptance:** changing a method's kind in one place changes both roads, and a
poison that changes only one side fails to compile rather than relying on a
runtime conflict check. The existing conflict check stays — it covers a
different failure (two different registrations claiming one name).

### ROLLBACK-MUTATOR-POPULATION — the mutator guard sees a quarter of rollback state

**Owner:** rollback scheduling (`scripts/check_rollback_mutators_run_in_sim.py`).

**Current state:** the guard that keeps rollback state from being mutated outside
the rewinding schedule defines that state as `rollback_*_canonical::<T>` only.
Every clone and custom-checksum registration is outside its population —
`RoomSet` and `LdtkRuntimeIndex` among them — although those values are
snapshotted and restored on every rewind exactly like the canonical ones, so an
outside mutation drifts identically.

⛔ **MEASURED 2026-09-15.** Widening the population to all rollback registrations
takes the systems it can see from 555 to 564 and surfaces **65 unwaived
offenders**. ⚠ That number is not a defect count: many are `Transform` writes
from camera, sprite and inspection systems, which are PRESENTATION reading a
component that happens to be rollback-registered. The widening is not landable
until sim writes and presentation writes can be told apart.

⚠ A second, independent hole in the same guard was closed on 2026-09-15: its
param pattern matched only `&mut T` and `ResMut<T>`, so `SessionWorldMut<T>` was
invisible and a refactor that respelled one write made the guard report a live
mutation as gone. All six types reached through that param are rollback-registered.

**Next implementation:** give the guard a way to distinguish a simulation write
from a presentation write — most likely by schedule rather than by type, since
`Transform` is legitimately written in both. Then widen the population and triage
what remains.

**Acceptance:** the population is every rollback registration, not one
registration spelling; `handle_ldtk_hot_reload` is visible without its waiver
being deleted; and a poison that respells a write in any supported param form
still reddens the guard.

### SETTINGS-ROLLBACK — finish the settings/mechanics admission boundary

**Owner:** rollback/mechanical-policy owners.

**Current state:** direct `UserSettings` reads have been removed from the
simulation schedule. Control-frame modes are projected per seat, and damage uses
`PlayerDamagePolicy`. The remaining damage policy is still forward-only across a
rollback timeline.

**Blocked by:** [Q127](awaiting-maintainer-decision.md#q127--are-difficulty-assist-and-player-damage-modifiers-match-wide-or-participant-specific).

**Next implementation:** after Q127, make the admitted damage policy follow the
chosen lifetime: match activation if match-wide, or deterministic per-seat input
if participant-specific. Do not reintroduce simulation reads of mutable
`UserSettings`.

**Acceptance:** rewinding/resimulating frame N observes the policy admitted for
that timeline, not whatever the settings UI contains now; the settings-to-policy
projection remains witnessed end to end.

### THROW-MODIFIERS — route throws through rage and staleness policy

**Owner:** Smash combat/knockback policy.

**Current state:** authored throw base/growth values reach launch, but the throw
road bypasses the rage and staleness modifiers used by ordinary strikes. This is
a mechanical consistency defect, not a request to retune all throws.

**Next implementation:** route throw launch through the same named modifier
policy where Smash semantics require it, then remeasure representative throws.
Keep authored throw formulas and move-specific values intact.

**Acceptance:** a controlled throw witness shows the intended rage/staleness
change, and a neutral arm proves base authored throw behavior is unchanged when
both modifiers are neutral.

### A2 — close the remaining projectile construction-identity hole

**Owner:** [`engine/projectile-contact-protocol.md`](engine/projectile-contact-protocol.md).

**Current state:** the swept-contact resolver, finite obstruction, exact ordering,
targeted delivery and compound solid-contact policy are established. Build-site
census coverage also exists. The remaining hole is independent: `ensure_sim_id`
intentionally skips a `BodyKinematics` entity that has no `SimId`, `FeatureId` or
`PrimaryPlayer`, and that silent skip has no production diagnostic/guard.

**Next implementation:** make that skipped population observable or impossible at
the construction boundary. Do not add a fallback ID that invents canonical
identity from query order.

**Acceptance:** an intentionally unidentified damageable body cannot silently
survive the identity sweep; the witness identifies the construction fault rather
than relying only on a total population count.

### A12 — finish move-contact attribution and reflection identity

**Owner:** [`engine/authored-technique-admission.md`](engine/authored-technique-admission.md)
and combat/projectile occurrence identity.

**Current state:** ranged feedback carries `MoveOccurrence` end to end, so a
projectile launched by move A cannot be credited to whatever move happens to be
playing when it lands. Melee stamps use the same occurrence authority.

**Open engineering:** add the guard that a body which has started a move cannot
lose `MoveOccurrence` during ordinary body lifetime; finish reflection/contact
attribution after the product rule is settled.

**Blocked by:** [Q101](awaiting-maintainer-decision.md#q101--may-an-abilitys-own-contact-satisfy-the-launching-moves-connected-condition).

**Acceptance:** late projectile/melee feedback, reflection and independent
ability contacts cannot credit the wrong move occurrence, including across an
idle gap and rollback.

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

## P1 — ownership, composition and iteration

### I2/I3 — finish independent content authoring and safe reload

**Owner:** [`engine/extension-model.md`](engine/extension-model.md) and content
reload/preparation owners.

**Current state:** a prebuilt host can load edited move content without Cargo;
reload has explicit outcomes, no-op revisions do not advance generations, and
stale attempts carry the generation they were prepared against. The runtime
already consumes loadable content rather than requiring the legacy Rust move
tables.

**Open work:** converge the remaining reloadable registries on one explicit
prepare/admit/publish contract and settle the permanent authoring source.

**Blocked by:** [Q110](awaiting-maintainer-decision.md#q110--may-a-provider-keyed-fragment-registry-gain-a-named-hot-reload-replacement-operation)
and [Q104](awaiting-maintainer-decision.md#q104--is-the-rust-move-table-or-the-content-file-the-source-of-a-moveset).

**Acceptance:** changed content publishes exactly once under a new admitted
generation; identical content is a no-op; stale work refuses rather than folding
against a generation it did not read; no runtime road silently falls back to a
second authoring source.

### A9 — establish truthful minimal engine profiles

**Owner:** public SDK/composition architecture.

**Current state:** capability-footprint and absence-contract tooling can measure
what a profile actually links/installs. The remaining work is semantic: define
what each supported profile promises instead of optimizing for a crate-count
number.

**Blocked by:** [Q100](awaiting-maintainer-decision.md#q100--should-the-facade-pull-bevydebug-because-it-always-links-ambition_dev_tools),
[Q106](awaiting-maintainer-decision.md#q106--are-ambition_items-and-ambition_encounter-optional-facade-capabilities),
[Q108](awaiting-maintainer-decision.md#q108--which-capabilities-may-a-featureless-ambition_platformer2d-link),
and the admission policy in [Q97](awaiting-maintainer-decision.md#q97--may-authored-content-name-a-technique-this-composition-did-not-install).

**Next implementation:** encode supported profiles as named capability contracts,
then make construction/step witnesses and absence guards test those contracts.

**Acceptance:** each supported profile constructs and steps a real subject; its
promised absent capabilities are absent from installation and resolved dependency
closure; the full Ambition composition remains intact.

### A7 — make item occurrence authority explicit where it carries a real invariant

**Owner:** [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md)
and [`engine/item-writer-inventory.md`](engine/item-writer-inventory.md).

**Current state:** `GroundItem` construction is sealed, but that seal does not own
occurrence creation. Death drops, match spawns and other roads still mint
identity/provenance/custody facts at their occurrence sites. The earlier claim
that one component constructor created one occurrence authority was false.

**Next implementation:** centralize only the occurrence decisions that share an
actual invariant (identity, custody, provenance, rollback ownership). Do not add a
generic request bus merely to reduce writer count.

**Acceptance:** reward policy consumes accepted occurrence outcomes and cannot
become an alternative minting authority; every remaining occurrence creator has
an explicit ownership reason.

### BRAIN — finish truthful fighter attack selection

**Owner:** [`engine/fighter-brain.md`](engine/fighter-brain.md).

**Current state:** the truthful attack kit evaluates the action a press actually
produces, and the previous rung-9 quantization defect is closed. Current failures
are no longer evidence that the old attack-kit mapping is wrong. The remaining
work is the owner's F6 decision/menu problem: a brain must be able to stop or
change movement so a movement-incompatible authored move can become selectable.

**Next implementation:** complete the F6 menu/utility term on the owner plan.
Keep press generation separate from move utility; do not patch the evaluator with
a fighter-specific exception.

**Acceptance:** representative CPUs select movement-compatible and
movement-transition attacks from their authored menu across the intended
difficulty ladder, with no regression to the press/move identity contract.

### D-POTATO-ASPECT — finish low-tier sprite aspect/trim policy

**Owner:** [`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md).

**Current state:** systematic downscale/trim generation defects were repaired.
The remaining product choice is whether character sprites at `potato` may fall
back to the `0_25x` tier.

**Blocked by:** [Q69](awaiting-maintainer-decision.md#q69--at-potato-should-character-sprites-fall-back-to-the-0_25x-tier).

**Acceptance:** the same authored frame preserves the intended world-space trim
and aspect at each supported tier; missing tiers follow the explicit policy
rather than an incidental fallback.

### D72 — continue Smash parity from the inventory

**Owner:** [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md).

**Current state:** the inventory is the source of feature/parity truth. Do not
turn this queue row back into a chronological parity diary.

**Next implementation:** take the next inventory row whose policy is settled,
implement it on the production path, update that inventory row and add the
production acceptance witness.

**Blocked where applicable by:** [Q62](awaiting-maintainer-decision.md#q62--keep-or-discard-the-epoch-captured-4741-line-mary_oldtk-delta),
[Q89](awaiting-maintainer-decision.md#q89--what-special-should-each-robot-stand-in-have),
[Q115](awaiting-maintainer-decision.md#q115--which-per-move-hitboxinflate-values-should-the-untuned-bone-derived-specs-carry),
and other product rows named by the inventory.

### D166 — make character authoring boundaries load-bearing

**Owner:** [`engine/character-authoring-package.md`](engine/character-authoring-package.md).

**Next implementation:** for each remaining duplicated authored/runtime value,
choose one authoring owner and make every runtime representation a projection or
admitted prepared value. Prefer deleting the second truth to synchronizing it.

**Acceptance:** the owner document can name one authoritative authored value for
each migrated fact, and production consumers cannot bypass its preparation or
projection boundary.

### D-SCENARIO-IDENTITY — confirm and then finish scenario cache identity

**Owner:** performance/scenario tooling.

**Current state:** current source inspection does not locate the named cache
subject in the tree. Treat that as an investigation requirement, not as
permission to implement an inferred replacement.

**Next implementation:** locate the current scenario cache/key owner and prove the
identity collision still exists. If the subject was removed or renamed and the
collision no longer exists, close this row. Otherwise include geometry identity
in the cache key at the owner boundary.

**Acceptance:** two scenario geometries with equal benchmark knobs cannot share a
cached result accidentally.

### TEST-LANES — keep required test lanes executable and diagnose `app_it` flake

**Owner:** test runner / app integration lane.

**Current state:** missing prerequisites are reported as incomplete rather than
pass. An order-dependent `app_it` failure remains unresolved, and the A10 work
also observed one non-reproducing session-root handoff failure whose assertion
message was not captured.

⛔⛤ **THE ORDER-DEPENDENT FAILURE HAS A NAME AND A MECHANISM NOW, AND MY FIRST
CLASSIFICATION OF IT WAS WRONG.** The arm is
`composes_through_the_sdk::a_host_that_omits_boss_encounters_still_builds_and_steps`.
I first recorded it as machine CONTENTION, because it appeared while two or three
full suites were running concurrently — then it reproduced **serially, on a quiet
machine, with one rust process and a load average of 2**. A coherent measured
story that fits the first observation is still the wrong one if it was never
tested against a second.

MEASURED at `770ac4bff`:
- Intermittent across runs of the FULL suite: 674/1 and 675/0 on the same tree.
- **12 of 12 green running that test file ALONE**, so it needs the whole process.
- The failure is a PANIC inside `FixedMain`, not a timeout and not a kill —
  `Encountered a panic in system bevy_app::main_schedule::FixedMain::run_fixed_main`.
- ⚠ The panic's own message is SWALLOWED: the arm has no assertion (it builds the
  engine with and without one plugin and steps 8 frames), and libtest's capture
  shows only bevy's three "Encountered a panic in system" lines. That is why this
  has read as a silent flake for so long.

⇒ **It needs other tests in the same PROCESS, which makes the suspect population
process-global state** — a static, an env var, or a shared registry — not test
ordering within one app.

**Next implementation:** on the next reproduction, capture the full failing
assertion and isolate the production ordering/state source before changing test
ordering or adding retries. Keep compile-cost and prerequisite failures distinct
from behavioral flakes, and from CONTENTION.

**Acceptance:** the failing population is reproducible or explicitly classified,
and the production cause is fixed or the harness proves why the failure is not a
production invariant.

## P2 — product/authoring work with an executable owner

- **Character feel / Smash tuning:** use the real roster and the measurement tools
  named by [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md).
  Do not infer the roster from `game/ambition_content/src/*_moveset.rs` or from one
  demo registration table. Product values waiting on a ruling stay in the
  decision ledger.
- **Q80 art/hitbox tolerance:** once the pixel tolerance is chosen, encode it in
  authoring/tool validation rather than subjective screenshots.
- **Q94 residency target:** once the memory target is chosen, use the asset owner
  plan to trade tiers/residency against a measured budget.

## P3 — human-gated or local-machine measurements

Do these only on a machine/environment that can answer the question:

- **D-RASTER-3:** measure weak-GPU framebuffer scale versus source-tier behavior.
- **Switch Pro outer range:** run the controller diagnostic on both target
  machines and compare the raw range.
- **Web reveal branch:** validate the existing reveal-barrier branch in the real
  browser/runtime.
- **Kaleidoscope Bevy-0.19 flash:** reproduce interactively before filing a fix.
- **LDtk preview tilesets:** measure whether editor-preview assets are still
  required by the authoring workflow before Q82 is resolved.
- **Capture after window close:** reproduce against the current capture path.
- **External consumer/platform checks:** follow the SDK/external-consumer owner
  documents; do not infer support from workspace-only builds.

## Replenishment rule

Before adding or promoting a row:

1. inspect current HEAD and confirm the problem still exists;
2. link the focused owner document;
3. state current behavior, next implementation, blockers and acceptance;
4. create/name a `Q` for every maintainer decision that blocks the row;
5. keep measurements in the owner document or a durable receipt, not as queue
   chronology;
6. remove closed rows instead of preserving their investigation history here.
