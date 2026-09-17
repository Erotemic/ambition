# Decision principles — how to choose like Jon when operating autonomously

*(Jon's own criteria, verbatim — relocated from `docs/vision/driving_decision_principles.md` <!-- cite-ok: the path is named because it is GONE --> into the planning stack, 2026-07-05. Read this before any architectural choice.)*

> ⛔ **THE POINTER THAT USED TO SIT HERE IS DEAD, AND IT POINTED AT A REAL
> THING.** This line read *"[`vision.md`](vision.md) §8 is the digest"* until
> 2026-09-03. It was true when written: at `docs/planning/vision.md` in July the
> sections were numbered and **§8 was literally "Principles digest"**. That page
> has since been rewritten with unnumbered sections, the digest section was
> removed, and it was not relocated — a search of all of `docs/` finds the phrase
> nowhere, and vision.md's §8 is now "Execution", three paragraphs about the
> queue. ⇒ **So this page is the only home these criteria have**, and a reader
> sent looking for a shorter version should stop looking. ⚠ A dead
> cross-reference survives a link checker perfectly: `vision.md` still exists, so
> nothing was ever going to report this.

If you need to make an architecture decision while operating autonomously, use these criteria to make the choice Jon would most likely make.

## High-weight criteria

Prefer the solution that is more elegant. In this project, “elegant” means the solution composes cleanly, has an obvious source of truth, follows existing seams, and does not require callers to remember hidden ordering rules or workaround behavior.

Two tests make that judgeable rather than a matter of taste, and a change should pass at least one.

*One authority per fact.* If a fact is read or written in two places, the elegant change is not to keep them in step — it is to remove one of them. A test asserting that two copies agree is a guard compensating for a fact having two homes, and it is evidence the change has not been made yet.

*Make it impossible, not checked.* Prefer the structure that cannot express the defect over the test that catches it. A guard is the right answer only where a type cannot state the rule — an authored string, a content file, a fact that lives outside the compiler.

A refactor that only moves code is not elegance. Name the authority or the dependency edge the change removes. If it removes neither, it is churn.

*One absence must not answer two questions.* A lookup that returns nothing is answering exactly one question unless somebody made it answer two, and the second one then passes silently. `ParamSchemaRegistry` has no entry for a technique with no params and no entry for a technique that does not exist, so that lookup alone cannot distinguish an authored typo from a parameterless technique; the current production validation gap is specified in [A11](engine/actor-monolith-work-frontier.md); `boss.cleared` read a missing save key as `Untouched`, so a wrong id was a shut door rather than an error. The fix is never to make absence fail — it is to split the facts, so that unknown fails, known-and-empty passes, and known-and-checked is checked. ⚠ The permissive default itself is usually right: an unconstrained move permits, an unclaimed slot is writable, and `ExperienceStaging::is_writable_by` states the distinction outright — *"nobody claimed this" and "somebody else claimed this" are different answers and only the second is a refusal*. Hunt the `None` that means two things, not the one that means yes.

*An instrument that reads nothing and a world that is fine give the same reading.* This is the run-time twin of the absence rule above, and it is about the OBSERVER rather than the data: a guard whose glob matches no files passes forever, a gate over a missing document has nothing to check, a filter that matches no rows counts zero, and a test that steps a dead session gets an answer every time. Each reports "clean" and each is measuring nothing. ⛔ The defence is not care, because every one of these was found by somebody being careful — it is a CONTROL WHOSE VALUE IS KNOWN IN ADVANCE: a population floor the scan must clear, a poison the guard must redden, an assertion the broken world cannot satisfy. Measured 2026-09-16, six instances at six altitudes in one day, found by three agents working separately: a `--maintenance` run reporting 7/7 over an emptied `status.md` (`ee6aff974`, restored; the gate that could not see it is `b5dcfac0f`); `check_headless_arms_can_fail` reporting eight offenders that were all in other agents' checkouts (`256fe65e9`), and the same glob in a script that TOUCHES what it finds rather than reading it (`6a8c120cf`); a GGRS session that invalidated at tick 6 and kept accepting 240 more steps, so every later assertion agreed with a frozen world (`c4e471d2a`); a poison that edited an arm body while the arm stepped through a helper, removing zero calls and coming back green; and a save projection filtered for `"HealthCell"` against rows storing `id: "healthcell"`, which read 0 for a resource that was tracking the bag 1:1 (`91f56a8d0`). ⛔ **AND THE SIXTH IS THE ONE THAT SHOULD WORRY A READER MOST, because it was announced as a PLAN by somebody who had just agreed with this principle.** Asked to check whether components with no mutable borrow are nonetheless replaced by re-insertion, an `insert(T…)` pattern over bevy source returned 0 or 1 for all thirteen candidates — including components that must be inserted somewhere to exist at all, because bevy inserts them inside tuples. ⇒ The honest headline is a LOWER BOUND on writers, not a count of constants: those thirteen are "no mutable borrow", not "never written", and the difference is invisible unless somebody says it out loud. The same session then proposed pointing `RollbackRestoreAudit` at 25 rows to see whether their floats drift — an audit that compares `census_all`, where a `rollback_component_clone` row gets a presence probe whose census hard-codes `xor: 0`. It counts carriers. Every float in every carrier can drift by any amount and the census is byte-identical. (YardratAmbition's measurements and their catch, both times.)

⭐ **AND THE FLOOR THAT IS SUPPOSED TO PREVENT THIS HAS ITS OWN VERSION OF IT.** `resimulations > 0` says the audit compared something; it does not say the SUBJECT was moving at the frames the audit compared. A ground item falls for two frames and settles, so a 24-step window whose three compared frames all hold it at rest satisfies the floor and reports "this value reproduces across a rewind" about a stationary object — with a control passing. ⇒ A floor must be raised on the quantity the claim is about, not on the instrument's own activity. The repair is a number the audit states about itself: how many DISTINCT censuses the probe took at the frames it actually compared, where `1` means the comparison had nothing to disagree about.

*A witness that supplies its own input never asks whether production has a writer.* This is the seventh instance and it is a distinct shape: the earlier six are an OBSERVER that sees nothing, this one is a QUANTITY that cannot differ. Measured 2026-09-16 on [THROW-MODIFIERS](queue.md#throw-modifiers--route-throws-through-rage-and-staleness-policy): routing a throw's launch through the staleness modifier reads `BodyStaleMoves::occurrences` for the throw's move, and move wear is recorded at exactly one site, gated on `hitbox::LandedBodyHit`, which the capture road never emits — so a throw-only move id is structurally pinned at `0` forever. ⛔ The acceptance the row asks for does not catch this and cannot: *"a controlled throw witness shows the intended rage/staleness change"* is satisfied by a unit test that calls `queue.record(..)` itself, which proves the arithmetic downstream of a value production can never reach. It was green while the shipped composition got exactly nothing, confirmed by a four-rung sweep coming back bit-identical to HEAD. ⇒ When a change makes one road READ a quantity another road already reads, check that the new road also WRITES it — and make the witness earn its input from the system rather than hand it over. A control whose value is known in advance is the defence; here the control is a run where nobody seeds the queue.

⚠ The save-filter instance had TWO independent reasons to read empty — the wrong string, and a `{ id, count }` shape where granting the same item moves a count and leaves the row count alone — so finding one cause and stopping would still have been wrong.

*The rule about counts applies to ABSENCES too, and absences are where it is hardest to notice.* A positive claim invites "which ones?"; a negative one does not, so a scan that finds nothing is believed on sight. Both of the worst errors of 2026-09-16 were absence claims made from a grep without opening a file: `grep supersed` concluded that no integration arm launches the same route twice, when `a_candidate_session_replaced_while_pending_is_discarded` had been doing exactly that and passing all along, in a file the grep MATCHED (it uses `ReplaceWith`, not `GoTo`); and a grep for `rollback_health` concluded six rollback arms were exposed, when all six refuse a frozen world by other means. ⇒ Before writing down that something does not exist, open the files the scan DID match and say what they are instead. (ToothbrushAmbition's rule, from their instance and mine.)

*A workaround that works is how a broken environment survives contact with a careful person.* `pytest scripts/tests` reported 25 failures on a frozen tree, twenty of them because every test that spawns `run_tests.py` got `SystemExit: 2` and a refusal naming its own cause — an interpreter that cannot run the Python lane, with a stale in-repo `.venv` shadowing the per-machine store. That `.venv/bin` held `activate`, `py.test` and `pydoc.bat` and NO python binary at all. ⛔ The failure text said all of this from the first run; the `FAILED` lines were grouped by file three times and never once opened. ⚠ And the standing note being carried — *"`pytest scripts/tests` cannot collect here because `audio_levels.py` imports absent numpy; pass `--ignore`, that is not a failure and not mine"* — was the same symptom wearing a different hat, comfortable enough to stop the question. ⇒ Two rules, and the first is nearly free: **when several tests in one file fail, open ONE before counting them.** And a note that makes a defect tolerable should carry an expiry, because it is competing with the diagnosis.

*Enumerate from the authority; do not sample through a projection you wrote.* The corollary, and it is what actually resolved the case above. Four hypotheses about which hashed value diverged were each cheap and each wrong, including one "eliminated" by the broken filter. The registry already knew every entry feeding the peer checksum, and asking it — `RollbackChecksumProbes::census_all`, keyed by type name — returned one differing entry out of 364 on the first try. ⇒ A projection you wrote can be wrong about the shape it reads; an enumeration from the owning registry can only be wrong about the question. Prefer the reading whose failure mode is a wrong question over the one whose failure mode is silence. The same distinction separates counting calls to a safety API, which measures vigilance, from counting assertions a broken world fails, which measures safety — a census of the first kind over-reported six exposed arms where the measured number was zero.

*An allowance keyed to a LOCATION is invalidated by moving the code, and nothing can see it.* A policy waiver keyed to a file, an absence contract that pins "this belongs to one place" by excluding that path, a test app that hand-lists the registrations its system needed at the time — each is a ledger about WHERE something is rather than WHAT it is. An extraction moves the code out of every allowance keyed to its old home while changing nothing about the code, so neither the compiler nor the crate's own tests can notice: `reflect_parried_shot`'s `kin.vel = -kin.vel * scale` was sanctioned for years as *"a projectile reflecting off a surface, mirroring its own velocity"*, and carving it into its own operation moved it out of the skip list that said so. ⇒ When a carve or an extraction lands, ask what allowed the code where it USED to live — and re-key the allowance to the operation, so the next move needs no entry. Three distinct ledgers in this repo are keyed this way; the count is the argument.

A class claimed from one example is a hypothesis. Sweep for the other instances before writing the generalisation down, because they are as likely to refute the wording as to confirm it — the sentence above was first written as "an absence that reads as a pass", and every other instance in the tree turned out to be correct.

Three things follow from the second test, and all three were learned by getting them wrong.

Making something impossible changes what its guard is for. A guard whose property has become structural is worth keeping only if it still names a reachable failure; re-aim it and say which, or delete it. Leaving it asserting what the compiler now guarantees, with a comment claiming otherwise, is worse than either.

A claimed compile-time check must be shown to fail. Poison the case that only the new check can catch, not the nearest case to hand — a poison that fires through some other mechanism is indistinguishable from one that fires through yours. For a compile-time assertion specifically, make it unconditionally false and confirm the build breaks. An associated `const` holding an assertion can compile clean while never being evaluated, which is a guard that cannot fail wearing the strongest possible disguise.

A schedule ordering edge is a fact that lives outside the compiler, so a guard is the right answer — but only a guard that has been run red. Bevy expresses ordering with sets, and the one thing a set cannot express is order *within itself*: a system that reads a message written by another member of its own set is unordered against the writer, and no phase chain will say so. The cost is not lateness but frame misattribution — the reader picks the message up on the following tick and journals its effect under a frame that did not produce it, which is what the rollback quarantine then judges. A behavioural test does not catch this. The simulation runs single-threaded, so an unordered pair still gets an order, and that order is stable: the gameplay assertion passes whether or not the edge exists. Deterministic, reproducible, and arbitrary.

Check the edge itself. Bevy computes the schedule's conflict list unconditionally and consults `ambiguity_detection` only to decide whether to warn — so the list is readable even in the rollback host, which silences it. Assert zero conflicts on the *resource*, not between two named systems: system and resource names are compiled out without Bevy's `debug` feature, and the invariant is about the resource anyway. Two ways this guard fails green, both met in practice: composing a hand-assembled app rather than the shipped plugin group, which certifies an empty room because the writer's plugin was never added; and identifying the resource through anything that can silently return a different id. Falsify it by deleting the edge and watching the count rise, with the code that ships rather than an earlier draft.

Do not answer this with a workspace-wide ambiguity ratchet. A banked count of existing conflicts is a denominator nobody re-measures, and it rots in the direction that looks like progress: when a conflict is genuinely fixed and the number stays put, the ratchet goes on certifying a population that no longer exists. A guard scoped to one resource banks nothing.

Prefer the solution that best respects the project’s layer boundaries:

* Rust is for behavior.
* RON is for content.
* The world IR  is for space and it is authored by a backend like LDtk, tiled, or godot.
* Machinery must not import named game content.

Prefer the solution that is more runtime efficient, especially in hot paths or repeated simulation work.

Prefer the solution that is more maintainable. The code should be easy to understand, easy to modify, and hard to accidentally misuse.

Prefer the solution that is concise. Shorter, simpler solutions are better when they preserve clarity and correctness.

Prefer the solution that minimizes confusion for a new developer. Ownership, data flow, and intent should be apparent from the code structure.

Prefer the solution that avoids parallel paths, compatibility shims, and duplicate mechanisms. This project is still pre-release, so direct replacement is usually better than preserving an old path when the replacement makes the architecture simpler.

Prefer the solution that creates a stable extension seam instead of adding another special-case branch to a core system.

Prefer the solution that keeps hot paths allocation-free and avoids repeated runtime work, while not over-optimizing cold authoring paths.

## Important considerations

If the code can be refactored so the solution better satisfies the high-weight criteria, do the refactor.

Look for ways to unify the change with an existing system. Unification is desirable when it does not over-scope the system. If a specific case can become an instance of a general case without a major runtime or clarity cost, that is usually worth doing.

Consider whether the change affects game behavior. Behavior changes are not automatically bad. The game still contains buggy, inconsistent, or provisional behavior, so making behavior more coherent may be the right outcome. Preserve behavior only when the existing behavior is intentional or relied upon.

Prefer a narrow validation path. A good architecture change should usually have a focused test, check, or tool command that proves the important part of the change.

Do not let TUNING block architecture (Jon, 2026-07-06). Numeric feel/quality values — DI angles, boss-quality thresholds, slope feel, fighter-brain weights, visual-quality defaults, and the like — are KNOBS, not Jon-blocking decisions. When the right variable already exists as a knob, treat choosing its value as data/playtest work and pick a reasonable default (or leave the existing one); ship it BLIND and let Jon adjust. Only escalate when the KNOB ITSELF is missing (an architecture gap), not when only its value is unset. A tuning task is never a reason to stall a structural carve.

## When a verification is green, ask what question it actually answered

⛔ **An instrument that CANNOT SEE and an instrument that WAS ASKED SOMETHING ELSE
both report green, and they are different failures.** The first is the
anti-vacuity family — an empty corpus, a pattern that cannot match, a control
that applies to nothing. The second is a pipeline that answered a NARROWER
question than the one you asked, and the narrower question has an answer, and the
answer is green.

Four instances hit one agent in one night (2026-09-16), all in shell plumbing
rather than in the repository: `| grep` exiting with GREP's status so a green lane
read as a failed job; a score-shaped grep gating a commit on
`[0-9]+/[0-9]+ jobs passed`, which accepts **`9/10`**; `sort | head -20` on a
caller list dropping the one `game/…` caller after every `crates/…` path;
and a regex that could not match the source's spelling returning a zero
indistinguishable from a clean tree. A fifth, from a second agent the same night:
a checkout's `.venv/bin/python3` was a BROKEN SYMLINK, so `python3 -m pytest`
fell through to an interpreter without the dependencies and reported 25 failures
where another machine measured 3. ⇒ **When two agents' lane numbers disagree,
suspect the INTERPRETER before the tree.**

⇒ **Two rules, and they are cheap.** Make the predicate NAME THE VALUE IT
ACCEPTS, so it cannot pass on a failure it was written to catch. And make an
empty result REFUSE rather than report, so "found nothing" and "there is nothing"
stop being the same string.

⚠ What found three of the four was not care, it was a SECOND INSTRUMENT
disagreeing — a compiler naming the caller a grep had dropped, a peer's scan
disagreeing by an order of magnitude, a test suite red where a lane was green.
**When something matters, arrange for two instruments rather than more care.**

The worked instances, with what each cost, are in
[`../recipes/re-measuring-a-planning-claim.md`](../recipes/re-measuring-a-planning-claim.md);
they are not repeated here.

## An acceptance criterion that counts the OLD road's absence is satisfied by breaking the NEW one

MEASURED 2026-09-16, across seven red arms in two lanes. A migration moved
per-seat frame policy off `SeatControlFrameModes` and onto the control frame, and
accepted itself with `measure_user_settings_in_simulation.py` reporting **zero**
simulation readers of the old value. It truthfully did. ⇒ Every fixture that
still wrote the old table now configured a policy nothing read, and a counter of
READERS cannot tell that from success.

⛔ **The absence of the old road is achievable by making the replacement resolve
garbage.** So: **both halves or neither** — the absence count, PLUS a value
witness that the new owner delivers the same answer. The value witness existed
here (`gravity_symmetry_room`, which asserts velocities rather than wiring) and
was not run.

⚠ The same shape reaches guards and censuses, not just migrations: a threshold
chosen from EXPECTATION rather than derived from the instrument's own output is
the same failure. A floor of `>= 20` over a scan that could only ever see 5 is
green for the same reason. ⇒ **Let the instrument set the number, and make the
accounting balance rather than clear a bar.**

⭐ And the corollary for the arms themselves: the arm that made this diagnosable
refused to let a partial pass stand in for the whole table, and said so in its own
failure message — *"the per-seat table is not reaching the derivation at all, so
the seat-zero claim below proves nothing."* **An arm that names what its own green
would NOT have proven is the arm you want failing.**

## An equality assertion `f(a) == f(b)` is satisfied by every `f` that throws information away

⛔⛤ **THE CONSTANT FUNCTION PASSES EVERY AGREEMENT ARM.** MEASURED 2026-09-16 in
ID-PEER: `two_hosts_at_different_content_epochs_share_one_construction_provenance`
asserts two hosts holding the same content agree about a construction stamp, and
for a day it reported closed while three ordinary production roads were dropping
the content term altogether. `content-unstated` agrees with `content-unstated`
perfectly — **the defect made the assertion MORE true.**

⇒ The price of the missing term was then measured rather than argued: poisoning
`ContentBinding::canonical_summary` to render a STATED BUT CONSTANT content term
leaves **all six** arms of `id_peer_audit` green, the one whose whole subject is
that term included. The only arm in the workspace that reddens is one that takes
a single process through two prepared fingerprints and asserts the provenance
MOVED.

⇒ **THE RULE: for every agreement arm, ask what the constant function would do to
it. If the constant passes, the other half of the claim is a DISAGREEMENT arm**,
and it is owed in the same commit —
`a_different_agreed_configuration_draws_a_different_sequence` and
`the_same_verdict_for_a_different_match_is_a_different_checksum` are the shape.
⚠ This is the same family as the section above, from the other end: that one is a
counter of the old road's ABSENCE, this one is a comparison whose two sides can
both collapse. Both are green because the instrument's discriminating power was
never measured, only its verdict.

## Low-weight criteria

Do not choose a solution merely because it is easier to implement right now. Ease of implementation has very little weight compared with elegance, maintainability, clarity, runtime behavior, and architectural fit.

Do not avoid an elegant solution merely because it is difficult to test automatically due to visual, aesthetic, or feel-based behavior. Prefer the elegant system. Visual regressions can be found and fixed later through review, playtesting, and iteration.

## Implementation plans must name the transition, not only the principle

For a selected slice, identify current writer, inputs, installer, scope, lifecycle,
rollback participation, consumer and old path to remove. Specify the state
transition and what a failure leaves unchanged. A proposed abstraction earns its
place when a caller no longer needs the callee's private policy; a forwarding
wrapper alone does not establish that.

Separate source facts, architecture decisions, experiments and product choices.
Unavailable timing does not block a known authority/dependency correction. A
performance result does not authorize duplicate writers or hidden simulation
state. When a contract is missing, complete its owner or report the packet open;
do not invent a fallback to make the demonstration pass.

Close with a nonempty behavioral witness and a deliberate defect that makes its
intended assertion fail. Then remove the obsolete production path and compress
the receipt. Keep unresolved questions only when they name missing evidence or a
real product policy, rather than asking the maintainer to choose routine ownership.
