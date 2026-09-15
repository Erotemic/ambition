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

**Current state — the ROOM scope is closed for the WORLD, with two named gaps
that are not about the world (2026-09-14).** ⚠ A review rejected an earlier,
flatter "completely closed" here and it was right to: what is closed is that no
road can change the authoritative world without a verdict. The transition state
machine still advances to `playing` on a refusal, and the custody-deferred
supersession is Model B rather than Model A. Both are named below. Every room lifecycle
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

**A10.1 landed (2026-09-14): the control plane is EXACT.** The candidate entities
carried exact `TransactionId`s while the baseline, the staged world and the
verdict were all *"the pending one"* / *"the last one"* — three App resources a
second publication would have overwritten, and the opposite direction from the
rest of A10. One ENTITY is the publication now: `begin_publication` returns a
`PublicationHandle`, and the baseline, effects, staged world, owning lane
transactions, target room and verdict all hang off it, so `baseline(P)`,
`staged_world(P)` and `verdict(P)` are the same P by construction rather than by a
check. `replace_live_world` and `spawn_contents` return the handle.

⇒ **Production authorizes from its OWN publication.** The reset and the dev
reload asked `LastConstructionVerification` — last-writer-wins, keyed by room
NAME, so two operations on one room were indistinguishable — before wiping the
save or advancing the session's content generation. They ask
`publication_succeeded(world, handle)` now. `LastConstructionVerification` stays,
as diagnostics.

⇒ **And `StagedWorldIsNotThisRoom` is DELETED.** It was defence against a stale
replacement being consumed by the wrong transaction; with exact identity that
state is not expressible, and a guard for an impossible state is a check that
cannot fail. The staged world also no longer needs `spawn_candidate_state` — it
lives on the publication — though that primitive stays for domains that need
candidate-owned state of their own.

**Finding 4 closed: the projection no longer promises an effect publication
cannot perform.** Every supersession used to project the predecessor away and the
verifier then proved ONE occupant remained — but `retire_superseded` declines to
despawn a body in another entity's custody, so publication produced TWO holders of
one identity while the projection had certified one, and the code's answer was
that a later baseline capture would notice. `DepartureAuthority` is DECLARED by
the transaction now (`Publication` or `Custodian`, read from the baseline at
`open`), `departing()` excludes a deferred one, `retire_superseded` acts by the
declaration instead of sniffing `InCustodyOf`, and the verifier admits exactly the
declared pair — a THIRD holder still refuses. Witnessed by
`a_deferred_departure_stays_in_the_projection_and_a_third_holder_still_refuses`.

⚠ **This is the review's Model B with explicit vocabulary, not Model A.** The
window where two holders exist is real and now described rather than hidden. Model
A — the checkpoint restore owning the room candidate AND the custody projection as
one outer publication, so the postcondition is not established until custody
resolves — remains the better end state and is not built.

Measurements live in the
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

**A10.3 landed (2026-09-14): every effect of the dev reload crosses the same
receipt.** The body transit, the dialog close, the combat-timer and
room-transition-cooldown resets, the preset flash and the parallax/room-visual
spawns all ran unconditionally right after `replace_live_world`, so a REFUSED
candidate left the OLD room live with the player re-seated into the new room's
spawn, dialogue closed, timers zeroed and the candidate's backdrop already spawned
over it. The room's own last-good-world property was intact and the OPERATION's
was not. They are one closure queued behind `publication_succeeded(world, P)` now,
re-acquiring the body, the resources and a `CommandQueue` from the world.
Presentation is a projection of the PUBLISHED room: it still reads the plan — the
plan IS the published room once the verdict says so — but it no longer dresses a
room that does not exist. `reload_ldtk_world_from_disk` lost eight parameters and
`handle_ldtk_hot_reload` lost four system params to the change, which is the
visible shape of it.

⚠ **UNWITNESSED, and that is a REAL gap, not a formality.** The LDtk hot-reload
road has no end-to-end coverage in either direction, so this is compile-verified
and reasoned, not measured. A harness for it is recorded in the owner document and
is not part of this packet.

⇒ **And `close` no longer takes the room name.** `verify_and_publish` read the
room id from a parameter and re-derived the owning lane transactions from the plan
— two more spellings of facts `RoomPublication` already holds. Both are read off P
now, so the two ends of the bracket cannot disagree about which room, or which
lanes, a verdict is for.

**A10.3b landed (2026-09-14): the ORDINARY room transition authorizes its commit
effects from its exact publication too.** A review found A10.3's defect still open
in the road every player uses: `RoomTransitionApplication::apply` called
`replace_live_world`, DROPPED the returned handle, and then cleared the projectile
carryover, played the door cue, reset the sim clock and the transition cooldown,
flashed the developer overlay, reset combat and the blink camera, closed the
dialogue AND the conversation, recorded the Class-B transit, asked for the
destination's visuals and emitted the arrival VFX/SFX and the landing diagnostic —
all before the deferred transaction had taken a verdict. Worse, BOTH hosts read
`apply() == Ok` as *"the crossing committed"*: the eager one consumed the exact
lifecycle intent, owed the checkpoint restore and advanced the barrier and game
mode; the confirmed one returned `CommitOutcome::Committed` and applied the
checkpoint restore.

⇒ `apply` is split into `stage` and a shared `finalize_room_transition(world, P)`.
Staging may read, gather and build the hidden candidate; it may not say the
crossing happened — the effect channels are GONE from `RoomTransitionApplication`,
so that is no longer expressible. Finalization asks the exact publication, applies
every crossing effect if it published, consumes the receipt either way, and on a
refusal cancels the transaction under the existing terminal policy (the intent is
spent, no restore is owed, the player keeps room N). The eager host reaches it
through `finalize_committed_room_transition`, an exclusive system chained after
the commit so the deferred verifier has run; the confirmed host calls the same
function immediately after `state.apply(world)`. One definition of a successful
transition, two scheduling mechanics.

⇒ **The shipped refusal witness now measures the effects, and has a control.**
`a_room_the_transaction_refuses_leaves_the_room_the_player_is_in_intact` arms the
developer flash, the body's arrival flash and the room-visual request to values a
committed crossing overwrites, and asserts none moved, plus that no checkpoint
restore is owed and the transaction is not sitting in `Committed`.
`a_crossing_that_publishes_does_every_transition_effect` walks a crossing that
PUBLISHES and asserts all three move — without it, a decayed value or a drained
message would read exactly like *"the effect did not run"* and the refusal
assertions could not fail.

**A10.3c landed (2026-09-14): a publication's RECEIPT has an explicit lifetime.**
`begin_publication` reaped every finished publication in the world, so the
sequence *A finishes → caller still holds A's handle → B begins* silently turned
`publication_succeeded(A)` from `true` into `false`. That is ownership by
coincidence. Retention is DECLARED at `begin_publication` now
(`UntilTheVerdictIsRecorded` for a caller that drops the handle — session
activation's first room — or `UntilOwnerRetires`), nothing reaps anything, and
`retire_publication` is the only thing that ends a publication. Every owner
retires its own, unconditionally and in its own statement, because a reader that
returns early on a refusal leaks on that branch. Witnessed by
`a_later_publication_does_not_invalidate_an_earlier_owners_receipt`, which drives
two REAL publications through `replace_live_world`. This is what lets A10.4's
candidate session hold its first room's receipt for as long as the activation
decision takes.

**Still not behind a verdict, named rather than implied:** the transition state
machine, which advances to `playing` on a REFUSAL as well — the crossing is
cancelled and the mode returns to `playing`, so a persistently refused door is a
livelock rather than a corrupted world.

⚠ **The custody Model B exception is UNCHANGED and is not to be expanded.**
`DepartureAuthority::Custodian` describes a real window in which the predecessor
and the candidate both stand and custody removes the predecessor immediately
afterwards. Model A — the outer checkpoint restore owning the room publication AND
the custody projection, establishing no postcondition until both complete —
remains the preferable end state, particularly for replication, and is not built.

**Current blocker — the SESSION scope, a different transaction, not started.**
`SessionScopeSet::Activate` is still retire-then-overwrite one level up:
`ActiveGameplaySession`, `ActiveSessionScope`, `SessionMechanics` and
`ActiveContentBinding` are process-global mirrors replaced in place. The handoff is
the case where a world N really exists to lose, and it is measured at ONE FRAME
(though not one command flush — `Cleanup`, `Activate` and the provider build are
different sets), so a candidate session root is hidden for the same order of time
a room's candidates are.

**A10.4 started (2026-09-14): the first ordering fact.** Session activation queued
its first room's build BEFORE it spawned the session root, so the activating room
was the one room publication in the project that took its verdict in a world where
its own session did not exist — and everything that publishes THROUGH the root
answers `None` there and says nothing. `PlatformerSessionBuilder::build` spawns the
root first now, and `verify_and_publish` REFUSES any room publication in a
shell-routed composition that has no root to publish into (the discriminator is
composition, as for the content binding — a direct-entry fixture legitimately has
no session). That is the prerequisite for a first room that can be built INTO a
candidate session.

**A10.4 second step: the content generation is SESSION-OWNED, not a process
global.** `ActiveContentBinding` was a `Resource` — one mirror of a per-session
fact — and the commit boundary compared every room plan against it. A candidate
session cannot use that: its first room must verify against the CANDIDATE's
generation while the outgoing session is still live under its own, and a global
could only be overwritten before the candidate's room verified, which destroys
the live session's answer. It is a component on the session root now. A room
transaction reads it off the root it is publishing INTO (`session_root_for_scope`
by the plan's scope; an `UNSCOPED` plan falls back to the composition's single
root, which is what a direct-entry host or headless fixture has). MEASURED:
asking by scope unconditionally made four refusal fixtures PUBLISH, because a
`None` scope matches no root and the comparison then had nothing to compare
against. It is waived in the component rollback census with its writers named —
activation and the hot reload, both in `Update`, never the simulation.

**A10.4 third step: THE SESSION ITSELF IS A CANDIDATE UNTIL ITS FIRST ROOM
PUBLISHES.** `PlatformerSessionBuilder::build` spawns the session root hidden
(`construction::hide_candidate_session_root`, the `SessionRoot +
InactiveCandidate` shape), builds the first room into it, and queues ONE closure
on that room's exact publication: published → `publish_candidate_session_root`
in the same command flush, so no system observes a half-published session;
refused → the candidate session is discarded whole (root, world bundle, content
binding) and the shell is told `ShellCommand::ExperienceFailed`, instead of being
left with a live session holding an empty world. `simulation_world` returns a
`SimulationWorld { player, publication }` and its caller declares the receipt's
retention, so the activation decision owns the receipt.

**A10.4 fourth step: the candidate session is ONE AGGREGATE and its WHOLE
POPULATION is hidden.** Two holes in the third step, both found by review and
both confirmed in source before fixing:

- **Hiding the root is not hiding the session.** Bevy's disabling components do
  not inherit through ownership, so the initial player — spawned through
  `spawn_session_scoped`, like every other session-owned entity — was fully
  visible while its root was hidden, and the gameplay queries that find a body
  find it by its own markers. A candidate prepared beside a live session would
  have produced two visible players. The fix is a visibility POLICY on the
  ownership context every session-owned spawn already captures:
  `SessionSpawnScope::candidate(scope)` vs `scoped(scope)`, applied in
  `apply_to`, which is the single point all six `spawn_*`/`insert_*` helpers pass
  through. `PartialEq` is hand-written on `id` alone — visibility is a spawn
  policy, not part of the ownership identity, and `TransactionId` reads
  `session.id()` and nothing else, so no construction identity changed.
  `publish_candidate_session` / `discard_candidate_session` act on the whole
  population.
- **A candidate may not write the live session's process state.**
  `simulation_world` installed `MovingPlatformSet` the instant the first room was
  STAGED, and activation installed `SessionMechanics` before a single root was
  built. Both now ride in the aggregate and are installed by the publication.

⇒ `CandidateSessionPublication` holds scope, root, activation, the first room's
receipt, the frozen mechanics and the platform state, and settles them in one
operation. **It is a VALUE captured by the publication closure, not a resource** —
a `PendingSessionMechanics`/`PendingMovingPlatformSet` pair would be the
retire-then-overwrite shape one level up, and a second candidate would overwrite
the first's.

Witnessed by
`a_hidden_candidate_session_is_invisible_to_the_live_world_and_visible_to_its_transaction`
(three facts: invisible to `session_world_entity`, visible to
`session_root_for_scope`, and the OWNED BODY hidden too — with both premises
asserted on an ordinary session first, so an unregistered filter cannot satisfy
it while hiding nothing) and
`discarding_a_candidate_session_takes_its_whole_population_and_nothing_else`
(a live session standing beside the discarded candidate survives it).

⚠ **THE REFUSAL HALF HAS NO PRODUCTION WITNESS.** The shipped app cannot be made
to refuse its first room the way the transition arm is: the first room's
`ActiveContentBinding` is written by setup from that room's own plan, so it
always matches. The PUBLISH half is exercised by every `app_it` test that boots.
The vocabulary the design rests on is witnessed at unit level by
`a_hidden_candidate_session_root_is_invisible_to_the_live_lookup_and_visible_to_its_transaction`,
which asserts the premise (an ordinary root IS visible) first so an unregistered
filter cannot fake it. ⇒ **A10 IS NOT CLOSED**: the acceptance criterion is a
PRODUCTION composition demonstrating that a failed candidate leaves the last-good
world playable, and at session scope that demonstration does not exist yet.

**ACTUAL BLOCKER — WORLD N IS DESTROYED BEFORE CANDIDATE N+1 IS BEGUN, AND IT IS
AN ORDERING FACT, NOT A MISSING TEST (MEASURED 2026-09-14 from source).**
`translate_shell_session_lifecycle` emits `RouteDeactivated` and `RouteActivated`
from ONE run, so a handoff retires and activates in the same frame; and
`SessionScopePlugin` chains `RetireAuthority -> Cleanup -> Activate ->
Presentation`, with `despawn_retired_session_entities` in `Cleanup`. ⇒ By the
time the provider builds the incoming session's candidate, the outgoing session's
entities are ALREADY DESPAWNED, unconditionally.

⚠ **THAT ORDER IS NOT AN ACCIDENT AND MUST NOT SIMPLY BE REVERSED.** It was
changed to retire-first on 2026-09-13 for a measured reason recorded in the
plugin: with `Cleanup` last, the incoming session's provider built its room while
the outgoing scope's placements were still live and the whole room was refused —
`room-refused central_hub_complex :: 18x Duplicated` — and
`reset_session_scoped_resources_on_retire` removed `SessionMechanics` after the
activation that installed it.

⇒ **THE SESSION-SCOPE ACCEPTANCE CRITERION IS THEREFORE UNREACHABLE TODAY**: a
refused candidate session leaves NO session at all, because N was already gone.
This is why A10 is not closed, and it is a larger statement than "the refusal
half has no witness".

⚠ **AND THE FIX IS NOT TO REORDER THE RETIREMENT — CORRECTED 2026-09-14, THE
SAME DAY THE ROW ABOVE WAS WRITTEN.** My first reading said the outgoing
session's retirement had to become a declared effect of the incoming
publication, the room packet's shape lifted one level. It does not: the
retirement is fine where it is, because **a candidate verified BEFORE the route
activates is already known-good by the time `RouteDeactivated(A)` is written**.
A refused candidate never activates, so A is never retired. That also leaves the
measured 2026-09-13 reason for the current order untouched.

⛔⛤ **A10.5 WAS ATTEMPTED AND REVERTED — 2026-09-14, AND THE MEASUREMENT IS THE
ROW.** The full change compiled (shell reservation ledger + `adopt_world`,
`build_candidate` spawning its own hidden root, a pending-phase preparer holding
the route, a registered `ShellActivationGates` evaluator, adoption on
`Activated`) and then failed 17+ `app_it` arms with *"reached no session world"*.
The working tree was reverted to the green HEAD; the patch is kept out of tree.

⇒ **THE CAUSE IS SHAPE, NOT DETAIL: I REPLACED AN UNCONDITIONAL CONSTRUCTOR WITH
A CONDITIONAL ONE.** The activation system built a world for
EVERY activation of an authored-catalog experience. The candidate road only fires
when a pending route has already published its prepared session, so every
activation that does not pass through that exact state — and there are several
roads that do not, plus an ordering question about whether preparation has
published by the time the pending-phase system looks within the same frame — got
no world at all.

⇒ **THE NEXT ATTEMPT MUST BE ADDITIVE.** Activation keeps a constructor for the
case where no candidate was prepared for it; the candidate road is an
OPTIMISATION of the ordinary road, not a replacement for it, and only the routes
that actually prepared a candidate get the strong guarantee. Ship it behind that
fallback, measure which activations take which road, and only then consider
removing the fallback. (⚠ REASONED, not measured: the two causes above were not
separated before the revert — the next attempt should instrument which one fires.)

**A10.5 LANDED (2026-09-14) — the candidate session is prepared and VERIFIED
before the route activates.** `prepare_candidate_platformer_session` builds the
whole candidate while the shell route is still pending and HOLDS that route with
a transaction-specific hold id; `candidate_session_gate` — one registered
evaluator, Q118's own barrier — answers `Hold` until the first room takes a
verdict, `Admit` when it published, and `Refuse` when it did not, discarding the
candidate whole. `adopt_candidate_platformer_session` then adopts the already-
verified world: `ActiveGameplaySession::adopt_world` takes the root the candidate
spawned, the shell facts it could not know go on, the projections are installed
and the population is promoted.

⇒ **THAT IS THE SESSION-SCOPE LAST-GOOD-WORLD GUARANTEE.** A refused candidate
never activates, so `RouteDeactivated(A)` is never written and the session that is
playing is never retired — with no new retirement machinery and the measured
2026-09-13 reason for the current retire-then-activate order untouched.

**MEASURED, in the shipped app:** `road=prepared-before-activation` on both
activations of the handoff arm, with matching activation ids and scopes, and
`room-loaded central_hub_complex` now printed BEFORE `session-start`. Three
defects were found by that instrumentation rather than by reasoning, and each is
recorded at its fix: `activate` minted a fresh id because it read the reservation
off `self.pending` AFTER the caller cleared it (candidate prepared for
`ShellActivationId(3)`, activated as `4`, so every session silently took the
fallback road); the hidden candidate root's `SimId` read as an unowned candidate
and as a stray to the room's own verifiers, which the old code avoided only by
accident of command order; and the save's occurrence ledger was adopted on
`SessionScopeActivated`, which is now too late for the first room.

⚠ **THE FALLBACK STAYS, deliberately.** An activation nobody prepared for still
builds and adopts inside the activation — the pre-A10.5 behaviour with the
pre-A10.5 guarantee. The world log names which road each session took, so the
fallback is removed on evidence rather than on hope.

⛔⛤ **THE ACCEPTANCE WITNESS IS STILL NOT WRITABLE, AND THE REASON IS NOW EXACT
(MEASURED 2026-09-14 by writing it twice and watching it fail).** Two attempts:

1. A test loop poisoning the candidate's `ActiveContentBinding` between frames —
   the candidate activated anyway. The preparer, the command flush that verifies
   the first room, and the router's gate evaluation all happen inside ONE
   `app.update()` whenever the route's content was already prepared.
2. The same poison as a SYSTEM ordered `.after` the preparer and before the
   router's advance — also activated anyway. The candidate's ROOT is a queued
   spawn: it does not exist until the same flush that verifies its first room, so
   even an in-frame observer has nothing to write onto.

⇒ **AND THE BINDING POISON IS STRUCTURALLY UNREACHABLE FOR A FIRST ROOM
ANYWAY.** The plan's generation and the binding written on the candidate's root
both come from `prepared_content.identity().epoch`, one frozen value — they cannot
disagree. The room transition arm can use this poison only because the room it
refuses is prepared against a generation the SESSION already holds.

⇒ **THE WITNESS NEEDS AUTHORED CONTENT THAT FAILS VERIFICATION, NOT A POISON.**
A registered test experience whose start room authors two placements with the same
id refuses through `verify_committed_roster` on the production road, with nothing
test-only inside construction. That is the next concrete step toward closing A10,
and it is bounded: the experience-authoring surface
(`PlatformerExperienceAuthoring`) already exists and `demo_shell_smoke` already
composes a tiny content plugin.

⇒ **UNTIL THEN A10 IS NOT CLOSED.** The session-scope guarantee is implemented and
wired — the gate refuses, the candidate is discarded, the route does not activate —
and it is REASONED, not measured, on shipped traffic.

**Next implementation — the remaining A10.5 work.** Prepare and
verify the candidate session while the shell route is still PENDING, hold the
route with a `ShellActivationGates` evaluator keyed to that candidate, and let
the gate's `Admit` / `Refuse` be the activation decision — the barrier Q118
already built for exactly this. Two identity prerequisites, both about deciding
an EXISTING host-local id earlier rather than inventing a new one (⛔ A10 must
not mint a second identity workaround; see ID-PEER):

- `ShellRouter::activate` mints the `ShellActivationId` at activation, and the
  candidate root's `SimId::singleton("session", activation_id)` needs it at
  preparation. The id must be RESERVED when the route goes pending. Re-keying
  that `SimId` to something else is not A10's call.
- `ActiveSessionScope::begin` mints a scope AND makes it current in one
  statement. A candidate needs the identity without the selection, so the
  allocator and the active-scope selector must separate — without creating a
  second current-scope authority.

`ActiveGameplaySession::spawn_world_for` is also unusable for a candidate by
construction (it starts `let instance = self.0.as_mut()?` and validates against
the already-published session), which is correct for its contract: the candidate
lives in `CandidateSessionPublication` until activation ADOPTS it. They do not need to become components on the
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
