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

**CURRENT INVARIANT.** A failed candidate world leaves the currently playable
world N intact, at BOTH scopes. A candidate N+1 is prepared and verified off to
the side; only a validated candidate becomes authoritative; N is retired only
after that publication succeeds.

**CURRENT HEAD BEHAVIOUR (2026-09-15).** The acceptance criterion is MET in the
PRODUCTION composition at both scopes, and every road that can change the
authoritative world now crosses its own publication's verdict — the room
transition, the reset, the death reconstruction, the shell handoff and the dev
LDtk reload, each with a refusal arm and an admission control in `app_it`. There
is ONE road into a live session (A10.5's fallback is deleted) and ONE publication
authority per level.

**NEXT IMPLEMENTATION STEP.** Custody supersession Model B → Model A, which needs
`restore_custody_to_checkpoint` split so publication can despawn the predecessor
while the custodian still finds its key. ⚠ Model A was ATTEMPTED and MEASURED to
fail as a straight inversion (`death_restores_the_checkpoint` 1/11,
`two_persistence_authorities_for_one_item` with `still_owned=1`); it is a
custody-domain restructuring, not an A10 ordering change. ⇒ **Its first step is
the ledger DRAIN, not the record** — see the owner document: an undrained ledger
leaves a permanently stale hand, which is worse than the window it replaces.

⭐ **AND ITS PRIORITY IS LOWER THAN IT LOOKED — MEASURED 2026-09-15.** The window
is real (5 of 838 `app_it` publications declare one) but it is NOT OBSERVABLE:
`a_custody_deferred_supersession_is_never_visible_as_two_holders` samples every
frame of the reset and finds peak 1, and POISONING the custodian takes it to 2,
so the arm is about a state that occurs and the custodian is what closes it inside
the frame. ⇒ Model A buys a structural guarantee, not the removal of a duplicate
anything can see. `LastConstructionVerification::left_to_custodian` was added so
the PREMISE is assertable — the first version of that arm used an abbreviated
fixture, opened NO window, and passed while measuring nothing; the premise
assertion is what caught it.

**ACCEPTANCE CRITERIA.** A production composition demonstrating (a) failed
candidate construction/publication leaves the last-good world playable, and (b)
successful replacement validates N+1 before retiring N. ⇒ **MET at room scope and
at session scope.**

**ACTUAL BLOCKER.** None for the criterion. What remains is listed under
*Remaining work* in the owner document and is narrower in kind: custody Model B (a
declared two-holder window on the SUCCESS path, admitted by the verifier as
exactly two), and a refused door telling the player nothing.

**RECEIPTS — WHAT LANDED, AND THE MEASUREMENT BEHIND EACH.** Everything below
this line is a receipt rather than a field; the five statements above are the
row's current state.

⚠ **AND A CITATION IN THIS CAMPAIGN'S OWN DOCUMENT POINTED AT A TEST THAT NEVER
EXISTED.** The owner document named
a&#95;hidden&#95;candidate&#95;session&#95;**root**&#95;…&#95;live&#95;**lookup**&#95;…
(spelled without backticks here on purpose — quoting a broken citation verbatim
makes the quote a citation, and reddens the very checker that found it)
as the unit-level witness for the whole candidate-visibility design; the only
occurrence of that name in the repository was the citation itself (the real arm is
`a_hidden_candidate_session_is_invisible_to_the_live_world_and_visible_to_its_transaction`).
⇒ **The citation gate checks PATHS and SYMBOLS, not test names in prose**, so a
renamed or imagined arm sits indefinitely. `scripts/check_planning_test_citations.py`
asks `git grep` for a matching `fn`; it found this one and **seven more in six
documents owned by other campaigns**, which is why it is not wired into
`--maintenance` yet — deciding rename-vs-never-written is the owning campaign's
call, and a name pointing at nothing may mean the EVIDENCE is missing rather than
just the label.

⛔⛤ **A `debug_assert` IS NO GUARD WHERE RELEASE PICKS AN ANSWER (2026-09-15).**
Found twice in an hour, once in my own fresh code. The candidate slot's backstop
was a `debug_assert!(slot.0.is_none())` guarding the bare `slot.0 = Some(..)` that
had leaked a whole prepared session that morning — so in the build players run,
the assert is gone and the bare assignment is back. And `session_root_for_scope`
asserted *"a scope owns at most one `SessionRoot`"* and then returned the first
one regardless: two roots on one scope is the duplicate-authority condition A10
forbids, in the ONE lookup every session-owned authority is read through, decided
arbitrarily and in silence. ⇒ **The distinction is not how likely, but what
happens INSTEAD**: unreachable → `debug_assert`; control continues into an
arbitrary choice → log it as well; continues into data loss → handle it. Both
repairs left behaviour unchanged and only added a voice.

**A CANDIDATE REPLACED WHILE PENDING IS DISCARDED (2026-09-15).**
`CandidateSessionSlot` is one deep and `slot.0 = Some(candidate)` overwrote it, so
a second pending route dropped a whole prepared session — hidden root, hidden
first room, publication receipt and reserved scope, all alive and none of them
reachable again. ⇒ **A candidate that is neither PUBLISHED nor DISCARDED is the
state A10's lifecycle exists to make impossible, and it was one `=` away.** The
superseded candidate is now discarded through `discard_candidate_session`, its
receipt retired and its reservation RELEASED — `ReservedGameplayScopes`' own doc
said a reservation whose activation never happens "is dropped, and the scope id is
simply never used", which describes a leak in the voice of a policy.

MEASURED: the path fires **0 times in the whole `app_it` suite**, so it was
unwitnessed as well as broken.
`a_candidate_session_replaced_while_pending_is_discarded` reaches it in the
shipped composition by issuing the second `ReplaceWith` three frames after the
first, and it discards **20 entities**. POISON-VERIFIED: skipping the discard
leaves `session_root_for_scope(SessionScopeId(0))` returning `Some(1489v0)` — that
lookup deliberately sees THROUGH the disabling marker, so a merely-hidden root
cannot pass it. Its premise is the sequential allocator: a live session at scope 1
means scope 0 was reserved by an activation that never became live.

⛔⛤ **AND NOTHING A CANDIDATE REGISTERS OUTLIVES IT ANY MORE (2026-09-15).**
`ShellActivationGates::register` had NO matching `forget` anywhere on A10's road,
so every candidate ever prepared — adopted, refused or superseded — left an
evaluator entry behind for the life of the process. MEASURED: after one superseded
handoff, entries for activation 2 (superseded) AND 3 (adopted) were both still
registered. All three exits forget now.

⚠ **AND THE ORDER IS LOAD-BEARING, MEASURED THE HARD WAY.** Forgetting the
evaluator while the hold is still registered leaves the router a hold it cannot
evaluate: `held=[ShellHoldId("session-publication:2")]`, the route stays held
forever, its reservation is never adopted, **and the superseding session never
starts either** — a worse failure than the leak. The hold is released FIRST, using
a route id recorded ON the candidate, because a candidate superseded by a route of
a different name cannot be cleaned up from the superseding route's id.

⛔⛤ **AND THE FOURTH EXIT IS CLOSED TOO: `ShellCommand::CancelPending`.** Found by
enumerating the writes to `CandidateSessionSlot` — exactly three — and asking what
else can end a pending route. Cancellation clears the router's pending transaction
and told the provider nothing, so the candidate sat in the slot forever: **a
cancelled route leaked exactly what a superseded one did.**

⭐ **THE THIRD AND FOURTH EXITS ARE NOW ONE SITE**, because they are one question —
*is the router still pursuing this candidate's activation?* —
`discard_abandoned_candidate`, at the head of the preparer. A fifth way for a
pending route to end needs no fifth copy of the four releases. The discriminator
also asks the RESERVATION (the ledger adoption consumes), which is what keeps it
out of the activation window: MEASURED, it fires exactly once in 663 arms and that
once is the supersession case. `a_candidate_session_whose_route_is_cancelled_is_discarded`
witnesses it, with the supersession arm as its control; poisoning the one site
reddens both.

⛔⛤ **AND THE TWO PROPERTIES THAT WERE HELD BY READING CALL SITES ARE COUNTED NOW
(2026-09-15).** `rooms::outstanding_publications` and
`construction::outstanding_candidates` are both asserted **0** after a committed
dev reload and after a cancelled candidate — *no publication receipt outlives its
operation* and *no candidate outlives its transaction*, which is A10's invariant
stated as a number. Both poison-verified (dropping the reload's own
`retire_publication` leaves 1; disabling the abandoned-cleanup leaves a candidate
root). ⭐ The candidate census carries a POSITIVE CONTROL — the cancel arm asserts
it counts **> 0** three frames in, while a candidate is supposed to be standing —
because a census reporting 0 from a wrong query reads exactly like a clean world.
⚠ Both return a COUNT, never a list: `RoomPublication` and `InactiveCandidate` stay
`pub(crate)`, and that containment is worth more than a richer accessor.

⛔⛤ **AND A10'S SUPERSESSION CASE HAS A PRODUCTION WITNESS.** The brief asks for
one explicitly — *old A remains live while hidden B is verified and B declares it
supersedes A* — and it is provable from two facts rather than by observing the
world mid-verification: `ProjectionViolation::SupersededNotLive` refuses a declared
supersession whose live half is already gone, so **a declared supersession that
PUBLISHED is one whose predecessor was still standing when the projected world was
verified**. `LastConstructionVerification::supersessions` records the count, and
`a_custody_deferred_supersession_is_never_visible_as_two_holders` asserts it is
`> 0` on a reset that re-authors an identity a hand is holding.

⭐ **AND EVERY ONE OF THESE CENSUSES CARRIES A NON-ZERO ASSERTION SOMEWHERE IN ITS
OWN ARM.** A census that reports 0 because its query is wrong reads exactly like a
clean world, and a fixture whose scenario already finished reads the same way —
MEASURED: an attempt to strengthen the cancel arm by starting it from a live
session had the route fully activated by frame 3, and every settled-state
assertion would have passed on an arm that no longer reached its subject. The
`> 0` assertion is what failed instead. Same reason
`LastConstructionVerification` carries `left_to_custodian` and `supersessions`:
without a COUNT of the thing an arm is about, *"the property held"* and *"the
situation never occurred"* are the same green.

⛔⛤ **AND "NOTHING REMAINS OUTSIDE CANDIDATE OWNERSHIP" IS ASSERTED NOW, NOT
ARGUED.** `SessionMechanics` and `MovingPlatformSet` are the two process-global
RESOURCES a candidate builder could most easily write during preparation — the
review that flagged this said so by name. They belong to the candidate aggregate
and are installed by ADOPTION, so the cancel arm (a candidate fully prepared and
then abandoned, with no session ever live) asserts BOTH are absent afterwards — no
`SessionMechanics` at all and an EMPTY `MovingPlatformSet` — with the handoff arm
as its control. MEASURED: that handoff installs **1** moving platform against the
cancel's **0**, so the pair discriminates rather than being two readings of an
always-empty resource.

**THE ROOM SCOPE, IN ONE PARAGRAPH.** Every room lifecycle
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

**A10.3d landed (2026-09-15): A10.3 IS MEASURED NOW, IN BOTH DIRECTIONS — and
witnessing it found the one effect still running ahead of the verdict.** Two arms
in `game/ambition_app/tests/an_edit_reaches_the_shipped_game.rs` drive the shipped
`build_visible_app`, activate gameplay and press `ApplyLdtkReload`. No file is
written: re-reading the same project is an equivalent reload and still runs the
whole candidate bracket.
- `a_committed_world_reload_applies_its_effects` — `applied_count` rises, the
  preset flash reads `1.0`, the status says applied with no errors.
- `a_refused_world_reload_leaves_the_running_game_untouched` — two
  process-resident holders of one `SimId::placement` make a world
  `TransactionBaseline::capture` cannot describe, so the reload's candidate room is
  refused. The flash stays `0.0`, `applied_count` is unchanged, and the player is
  in the same room. Its control is the arm above; its PREMISE is asserted, because
  "the flash did not move" is equally true of a reload that never ran.

⛔ **THE DEFECT IT FOUND: the developer-facing status was still unconditional.**
MEASURED — a refused reload reported `applied_count 0 -> 1` and
`"world reload applied to 'X' (#1)"`. `mark_applied` was called on the `Ok` of
`reload_ldtk_world_from_disk`, and that `Ok` means STAGED: the verdict is decided
later in the same command flush. It is a verdict-gated queued effect now, like
every other effect on this road, and a refusal records the verification's own
violations.

⛔ **AND THE WITNESS IMMEDIATELY FOUND A SECOND ONE: the local rollback restart.**
MEASURED — across a REFUSED reload, `session_is_active` went `true -> false`.
`stop_session_deferred` and the `RestartLocalGgrsAfterLdtkReload` marker were both
issued at the top of `handle_ldtk_hot_reload`, before a single root was built, so a
refusal left the running game with the world it was playing intact and its rollback
timeline torn down for a room that does not exist. The marker is inserted behind
the publication's verdict now (`true -> true` across a refusal), and the deferred
stop is GONE rather than moved: the `PostUpdate` owner already stops a live session
before releasing ownership, and nothing simulates between the two — `FixedUpdate`
runs before `Update`, not between `Update` and `PostUpdate`. The committed arm
asserts the baseline IS released, which is what gives the refusal assertion its
discriminating power.

⇒ **TWO UNCONDITIONAL EFFECTS SURVIVED A CHANGE WHOSE WHOLE SUBJECT WAS
UNCONDITIONAL EFFECTS, and both were outside the closure A10.3 moved.** One was
the status a human READS; the other was issued BEFORE the function that owns the
bracket was even called. A road's effects are not only the writes inside its
transaction.

**A10.5's fallback is DELETED (2026-09-15): there is ONE activation road.** An
activation nobody prepared a candidate for built its world inside the activation,
with the pre-A10.5 guarantee. MEASURED **655 activations, 0 fallback** (634
`app_it`, 21 `--workspace --lib`); it is a `panic!` naming the schedule edges that
make it unreachable, and 661 `app_it` arms pass without firing it. Its shell-side
twin went too: `candidate_session_gate` admitted when nothing was prepared, which
would have turned the deleted road into a crash; it refuses now, and the playing
session survives. The `road=` label in the world log is retired with it.

⛔ **THE EVIDENCE THE PLAN ASKED FOR WAS AN UNRELIABLE BUCKET.** The owner document
said to remove the fallback on the world log's `road=` label. That log is capped at
`WORLD_LOG_CAP` = 4000 lines PER PROCESS and one `app_it` run hits the cap: it
reported **458** of the 634 activations it saw. ⇒ **A diagnostic log with a line
cap is not a census instrument.** The count above is from an uncapped probe with
the strong road as its positive control.

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

⭐ **THE ROAD TO A10.5, AND WHY THE ROW ABOVE ONCE SAID A10 COULD NOT BE CLOSED,
IS IN THE OWNER DOCUMENT** — the session-scope blocker as it was measured, the
reverted first attempt, and the ordering fact that made the acceptance criterion
unreachable. All of it is resolved; none of it is current state.

**A10.5 LANDED (2026-09-15, on the second attempt): the candidate session is
prepared and VERIFIED before the route activates.**
`prepare_candidate_platformer_session` builds the whole candidate — hidden root,
hidden initial player, hidden first room, session-owned content binding — while
the shell route is still PENDING, and HOLDS that route with a
transaction-specific hold id. `candidate_session_gate`, one registered evaluator
answering through Q118's own barrier, says `Hold` until the first room takes a
verdict, `Admit` when it published, and `Refuse` when it did not, discarding the
candidate whole. `adopt_candidate_platformer_session` adopts the already-verified
world through `ActiveGameplaySession::adopt_world`, puts on the shell facts a
candidate cannot know, installs the projections and promotes the population.

⇒ **A refused candidate never activates, so `RouteDeactivated(A)` is never
written and the session that is playing is never retired.** No new retirement
machinery, and the measured 2026-09-13 reason for the current retire-then-activate
order untouched.

⚠ **THE FIRST ATTEMPT SHIPPED THE DEFECT A10 EXISTS TO REMOVE, AND THE WORLD LOG
CAUGHT IT.** With the verifiers still process-wide, the handoff candidate's first
room reported *"18 declared departures retired"* one frame BEFORE `session-start`:
a whole-world baseline found the PLAYING session's bodies wearing ids this room
plans, declared them superseded, and publication despawned them. It was reverted,
the session-scoping prerequisite landed first, and A10.5 was re-applied on top —
the same handoff now reports *"0 declared departures retired"*.

**MEASURED, in the shipped app:** `road=prepared-before-activation` on both
activations of the handoff arm; `room-loaded` printed BEFORE `session-start`; and
`a_candidate_session_does_not_retire_the_playing_sessions_world` samples EVERY
FRAME of the handoff and asserts N's population is untouched while N is still the
live scope. POISON-VERIFIED: making the two scoped primitives ignore their session
reproduces the defect and the arm fires with `163` against `181` — the same 18
bodies the world log named. Restores checked by md5.

⭐⭐ **AND THE REFUSAL HALF IS WITNESSED TOO (2026-09-15), WHICH MEETS A10'S
ACCEPTANCE CRITERION AT SESSION SCOPE.**
`a_candidate_session_the_transaction_refuses_leaves_the_live_session_playable`
drives a real handoff in the shipped app with TWO process-resident holders of one
identity standing in the world. An entity carrying a canonical `SimId` and NO
session owner is in every session's world by definition — including the
candidate's — and `TransactionBaseline::capture` refuses a duplicated identity
outright, so the candidate's first room cannot be verified at all. Nothing
test-only is wired into construction.

⇒ It asserts the PREMISE first (a room transaction ran and was REFUSED, so the
arm is not satisfied by an app that did nothing), and then that the live session
keeps its activation id, its scope, its population and its room. Its control is
`a_shell_handoff_publishes_the_incoming_sessions_room`, which drives the same two
commands with no corruption standing and lands the incoming session.

⚠ **TWO HOLDERS, NOT ONE, AND THE DIFFERENCE IS THE MECHANISM.** A single extra
holder of an id the room PLANS is a predecessor: the candidate declares it
superseded and publication retires it, which is ordinary A10 and was measured
doing exactly that. A pair is a world that cannot be described.

⚠ **A known order-dependent `app_it` failure remains**
(`composes_through_the_sdk::a_host_that_omits_boss_encounters_still_builds_and_steps`),
unrelated to this work: it failed in a full run with A10.5 ABSENT earlier the same
day and passes both in isolation and on a repeat full run.



 Prepare and
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
