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

**Owner:** [`engine/actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md),
[construction and reconstitution](engine/construction-and-reconstitution.md), and
the construction/session owners.

**Current state — the ROOM scope is closed (2026-09-14).** Every room lifecycle
path (transition, reset, dev reload) runs through `replace_live_world`, which
stages the whole replacement in `PendingWorldReplacement`, builds every root
hidden under `ROOM_CANDIDATE_BRACKET = true`, declares what publication would do
(`superseding` / `retiring` / `owned_by`), and only then verifies. `transaction::open`
makes that split against the baseline it captures, never against the
post-construction world. Refusal drops the candidate roots and the staged world
and leaves N byte-identical; admission runs `publish_candidate`,
`apply_world_replacement`, `retire_superseded` as one authority in one order.
`commit_deferred` and `retire_outgoing` are deleted <!-- cite-ok: named BECAUSE the A10 room packet deleted them; a resolvable citation here would mean the deletion did not happen -->,
so no road can commit a room without a verdict.

The staged world is CANDIDATE-OWNED STATE, not a process global:
`construction::spawn_candidate_state` puts it on a hidden entity stamped with the
room's transaction, so a refusal retires it with everything else the candidate
made, a leak carries a dead stamp no later room can find, and publication adopts
it and drops the carrier. Measurements live in the
[owner document](engine/construction-and-reconstitution.md#a10---bounded-safe-candidate-materialization).

Three verifiers guard it: `verify_committed_roster` (is the built world coherent),
`verify_projected_roster` (would the roster be valid if this published), and
`verify_staged_world` (is the non-entity world coherent and is it THIS room's).
They are complements, not alternatives. Production refusals are currently zero, so
the refusal apparatus is proven by its arms rather than by shipped traffic.

**A10.2 landed (2026-09-14): the generic layer no longer assumes one candidate
publication per world.** `project_post_publication_roster` added EVERY hidden
entity and `verify_projected_roster` refused any whose stamp was not this
publication's, so two regions prepared offside would each have refused the other.
The projection now includes only candidates this publication OWNS — `ScopeMember`
carries its `owner`, because a `ScopeClassification` is relative to the one
transaction the scope was gathered against and reads a publication's own
capability lane as foreign. Published identities are still counted GLOBALLY, so a
candidate taking a live identity must still declare its supersession. What was
`CandidateNotOwned` ("not one of mine") is now `CandidateUnowned` ("nobody's, so
no publication can admit it and no refusal can retire it"). Witnessed by
`two_independent_candidate_publications_do_not_invalidate_each_other`:
verify west while east is hidden, publish west, east stays hidden and intact, then
east verifies against the world west left behind.

**Not yet behind the verdict**, named rather than implied: the dev reload's
`transit_body` and its dialog/combat/cooldown resets (that road has no end-to-end
coverage in either direction); the reload's presentation spawns, which read the
plan and so cannot dress the wrong room; and the transition state machine, which
advances to `playing` either way — a persistent refusal is a livelock rather than
a corrupted world.

**Current blocker — the SESSION scope, a different transaction, not started.**
`SessionScopeSet::Activate` is still retire-then-overwrite one level up:
`ActiveGameplaySession`, `ActiveSessionScope`, `SessionMechanics` and
`ActiveContentBinding` are process-global mirrors replaced in place. The handoff is
the case where a world N really exists to lose, and it is measured at ONE FRAME
(though not one command flush — `Cleanup`, `Activate` and the provider build are
different sets), so a candidate session root is hidden for the same order of time
a room's candidates are.

**Next implementation:** stage the session authorities as VALUES published at a
verdict, as the room packet does. They do not need to become components on the
candidate root — that framing would have charged `MovingPlatformSet` a
rollback-wire-format change it does not have to pay. First concrete step is an
ordering fact, not a type change: activation queues its room build before it
spawns the session root, so `spawn_world_for` must precede `simulation_world`.
Reordering alone buys little — a refused first room leaves the session broken
either way — so the shell also owes a policy for what an activation does when its
first room refuses. Do not replace this with "save the live resources and restore
them on failure"; that creates duplicate authority plus recovery logic.

**Acceptance:** the shipped production composition demonstrates the last-good-world
guarantee at session activation and handoff, both arms poison-verified, as the
room scope now does for transition, death/checkpoint reconstruction and hot reload.

### ID-PEER — remove host-local lineage from peer-stable mechanical identity

**Owner:** deterministic identity / rollback architecture; see the identity map in
[`consolidation/architecture-census.md`](consolidation/architecture-census.md).

**Current state:** `SessionScopeId` is an App-local lifetime/correlation identity.
The remaining campaign is to ensure local activation counts cannot influence a
canonical checksum, peer-stable seed or rollback-visible identity. Keep the local
session term where it is useful for ownership; do not make local lifetime and
peer identity the same type by accident.

⚠ The review asked for this to be settled *before* A10 made transaction provenance
more central. That did not happen: A10's room scope landed first, and a publication
now declares the lane `TransactionId`s it owns (`PublicationEffects::owned_by`),
with `CandidateNotOwned` refusing anything stamped outside them. A10 deliberately
used the existing interfaces rather than hardening host-local lineage into a new
provenance contract, so the dependency stayed narrow — but it is wider than it was.
`a_transaction_identity_still_depends_on_host_local_lineage_counters` records the
divergence and flips the day the identities are split. The two-App poison (A burns
a candidate epoch, B does not, both construct identical content, canonical
snapshots must agree) is still unwritten.

**Next implementation:** define the peer-agreed session/match term explicitly and
migrate canonical provenance to it. Keep `SessionScopeId` for local lifetime only
where that is its real job.

**Acceptance:** two Apps that have burned different numbers of local session
activations can enter the same deterministic match and produce the same canonical
mechanical identity/checksum. The witness must first assert that their local
counters differ.

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

**Next implementation:** on the next reproduction, capture the full failing
assertion and isolate the production ordering/state source before changing test
ordering or adding retries. Keep compile-cost and prerequisite failures distinct
from behavioral flakes.

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
