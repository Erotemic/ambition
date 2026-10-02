# Decision principles — how to choose like Jon when operating autonomously

These are Jon's criteria. When you must make an architecture decision while you
operate autonomously, use them to make the choice Jon would most likely make.
This page is their only home. No shorter digest exists elsewhere.

## High-weight criteria

Prefer the solution that is more elegant. In this project, "elegant" means the
solution composes cleanly, has an obvious source of truth, follows existing
seams, and does not require callers to remember hidden ordering rules or
workaround behavior.

Two tests make elegance judgeable. A change should pass at least one.

*One authority per fact.* If a fact is read or written in two places, do not
keep the two in step. Remove one of them. A test that asserts two copies agree
is a guard that compensates for a fact with two homes. It is evidence that the
change is not made yet.

*Make it impossible, not checked.* Prefer the structure that cannot express the
defect over the test that catches it. A guard is the right answer only where a
type cannot state the rule: an authored string, a content file, a fact that
lives outside the compiler.

A refactor that only moves code is not elegance. Name the authority or the
dependency edge that the change removes. If it removes neither, it is churn.

Prefer the solution that best respects the project's layer boundaries:

* Rust is for behavior.
* RON is for content.
* The world IR is for space. A backend such as LDtk, Tiled or Godot authors it.
* Machinery must not import named game content.

Prefer the solution that is more runtime efficient, especially in hot paths or
repeated simulation work.

Prefer the solution that is more maintainable. The code should be easy to
understand, easy to modify, and hard to misuse by accident.

Prefer the solution that is concise. Shorter, simpler solutions are better when
they preserve clarity and correctness.

Prefer the solution that minimizes confusion for a new developer. Ownership,
data flow and intent should be apparent from the code structure.

Prefer the solution that avoids parallel paths, compatibility shims and
duplicate mechanisms. The project is pre-release, so direct replacement is
usually better than keeping an old path when the replacement makes the
architecture simpler.

Prefer the solution that creates a stable extension seam instead of another
special-case branch in a core system.

Prefer the solution that keeps hot paths allocation-free and avoids repeated
runtime work. Do not over-optimize cold authoring paths.

## Important considerations

If a refactor makes the solution satisfy the high-weight criteria better, do the
refactor.

Look for ways to unify the change with an existing system. Unification is
desirable when it does not over-scope the system. If a specific case can become
an instance of a general case without a major runtime or clarity cost, that is
usually worth doing.

Consider whether the change affects game behavior. A behavior change is not
automatically bad. The game still has buggy, inconsistent or provisional
behavior, so a more coherent behavior can be the right outcome. Preserve
behavior only when it is intentional or something relies on it.

Prefer a narrow validation path. A good architecture change usually has a
focused test, check or tool command that proves its important part.

Do not let tuning block architecture. Numeric feel and quality values (DI
angles, boss-quality thresholds, slope feel, fighter-brain weights,
visual-quality defaults) are knobs, not decisions that wait for Jon. When the
knob exists, choosing its value is data or playtest work. Pick a reasonable
default, or keep the existing one, ship it, and let Jon adjust it. Escalate only
when the knob itself is missing, because that is an architecture gap. A tuning
task is never a reason to stall a structural carve.

Adopt an upstream (Bevy or crate) primitive when it owns the same semantics as
the local code. Do not migrate only to reduce local code or because a new
upstream API exists.

## Low-weight criteria

Do not choose a solution merely because it is easier to implement now. Ease of
implementation has very little weight compared with elegance, maintainability,
clarity, runtime behavior and architectural fit.

Do not avoid an elegant solution merely because visual, aesthetic or feel-based
behavior makes it difficult to test automatically. Prefer the elegant system.
Review, playtesting and iteration find visual regressions later.

## Implementation plans must name the transition, not only the principle

For a selected slice, identify the current writer, inputs, installer, scope,
lifecycle, rollback participation, consumer and the old path to remove. Specify
the state transition and what a failure leaves unchanged. A proposed abstraction
earns its place when a caller no longer needs the callee's private policy. A
forwarding wrapper alone does not establish that.

Separate source facts, architecture decisions, experiments and product choices.
Unavailable timing does not block a known authority or dependency correction. A
performance result does not authorize duplicate writers or hidden simulation
state. When a contract is missing, complete its owner or report the packet open.
Do not invent a fallback to make the demonstration pass.

Close with a nonempty behavioral witness and a deliberate defect that makes its
intended assertion fail. Then remove the obsolete production path and compress
the receipt. Keep an unresolved question only when it names missing evidence or a
real product policy. Do not ask the maintainer to choose routine ownership.

## Absence and observation

*One absence must not answer two questions.* A lookup that returns nothing
answers one question, unless somebody made it answer two. Then the second one
passes silently. Example: a registry with no entry both for "this technique has
no params" and for "this technique does not exist" cannot tell a typo from a
parameterless technique. The fix is never to make absence fail. Split the facts:
unknown fails, known-and-empty passes, known-and-checked is checked. The
permissive default itself is usually right. An unconstrained move permits, and
an unclaimed slot is writable: "nobody claimed this" and "somebody else claimed
this" are different answers, and only the second is a refusal. Hunt the `None`
that means two things, not the one that means yes.

*An instrument that reads nothing and a world that is fine give the same
reading.* A guard whose glob matches no files passes forever. A gate over a
missing document has nothing to check. A filter that matches no rows counts
zero. A test that steps a dead session gets an answer every time. Each reports
"clean" and measures nothing. Care is not the defence. The defence is a control
whose value is known in advance: a population floor the scan must clear, a
poison the guard must redden, an assertion the broken world cannot satisfy.
State a pattern scan's result as a lower bound when the pattern can miss forms
(for example, inserts inside tuples).

Raise a floor on the quantity the claim is about, not on the instrument's own
activity. "The audit compared something" does not mean the subject was moving
at the compared frames. An audit can state how many distinct values it actually
compared; `1` means the comparison had nothing to disagree about.

*A witness that supplies its own input never asks whether production has a
writer.* When a change makes one road read a quantity that another road already
reads, check that the new road also writes it. A unit test that seeds the value
itself proves only the arithmetic downstream of a value that production may
never reach. Make the witness earn its input from the system. The control here
is a run where nobody seeds the input.

*The rule about counts applies to absences too.* A positive claim invites
"which ones?"; a negative claim does not, so people believe an empty scan on
sight. Before you write that something does not exist, open the files the scan
did match and say what they are instead.

*A workaround that works is how a broken environment survives contact with a
careful person.* When several tests in one file fail, open one before you count
them. A note that makes a defect tolerable must carry an expiry, because it
competes with the diagnosis. When two agents' lane numbers disagree, suspect the
interpreter before the tree.

*Enumerate from the authority; do not sample through a projection you wrote.* A
projection you wrote can be wrong about the shape it reads. An enumeration from
the owning registry can only be wrong about the question. Prefer the reading
whose failure mode is a wrong question over the one whose failure mode is
silence. In the same way, a count of calls to a safety API measures vigilance;
a count of assertions that a broken world fails measures safety.

*An allowance keyed to a location is invalidated by moving the code, and nothing
can see it.* A policy waiver keyed to a file, an absence contract that excludes
a path, a test app that hand-lists the registrations its system needed: each is
a ledger about where something is, not what it is. An extraction moves the code
out of every such allowance and changes nothing the compiler or the crate's
tests can see. When a carve or extraction lands, ask what allowed the code where
it used to live. Re-key the allowance to the operation, so the next move needs
no entry.

A class claimed from one example is a hypothesis. Sweep for the other instances
before you write the generalisation down. They are as likely to refute the
wording as to confirm it.

## Guards that follow from "make it impossible"

Making something impossible changes what its guard is for. Keep a guard whose
property became structural only if it still names a reachable failure. Re-aim
it and say which failure, or delete it. Do not leave it asserting what the
compiler now guarantees, with a comment that claims otherwise.

A claimed compile-time check must be shown to fail. Poison the case that only
the new check can catch, not the nearest case. A poison that fires through some
other mechanism looks the same as one that fires through yours. For a
compile-time assertion, make it unconditionally false and confirm the build
breaks. An associated `const` that holds an assertion can compile clean and
never be evaluated.

A schedule ordering edge is a fact outside the compiler, so a guard is the right
answer, but only a guard that has been run red. A Bevy set cannot express order
within itself. A system that reads a message written by another member of its
own set is unordered against the writer. The reader then sees the message on the
next tick and journals its effect under the wrong frame, which the rollback
quarantine judges. A behavioral test does not catch this: the single-threaded
simulation gives the pair a stable, arbitrary order, so the gameplay assertion
passes with or without the edge. Check the edge itself:

- Bevy computes the schedule's conflict list unconditionally and reads
  `ambiguity_detection` only to decide whether to warn. The list is readable
  even in the rollback host, which silences the warning.
- Assert zero conflicts on the resource, not between two named systems. System
  and resource names are compiled out without Bevy's `debug` feature.
- Use the shipped plugin group, not a hand-assembled app. A hand-assembled app
  can omit the writer's plugin and certify an empty room.
- Identify the resource through nothing that can silently return a different
  id.
- Falsify the guard: delete the edge with the shipped code and watch the count
  rise.

Do not answer this with a workspace-wide ambiguity ratchet. A banked count of
existing conflicts is a denominator nobody re-measures. When a conflict is fixed
and the number stays, the ratchet certifies a population that no longer exists.
A guard scoped to one resource banks nothing.

## When a verification is green, ask what question it actually answered

An instrument that cannot see and an instrument that was asked something else
both report green. They are different failures. The first is the anti-vacuity
family above. The second is a pipeline that answered a narrower question than
the one you asked, and the narrower answer is green. Typical shell causes: a
pipe that returns `grep`'s exit status instead of the job's; a score pattern
such as `[0-9]+/[0-9]+ jobs passed` that accepts `9/10`; `sort | head` that
drops the one line you needed; a regex that cannot match the source's spelling.

Two rules:

- Make the predicate name the value it accepts, so it cannot pass on the failure
  it was written to catch.
- Make an empty result refuse rather than report, so "found nothing" and "there
  is nothing" are different outputs.

When something matters, arrange for two instruments, not more care. A second
instrument that disagrees (a compiler, a peer's scan, a test suite) finds what
care misses. Worked instances are in
[`../recipes/re-measuring-a-planning-claim.md`](../recipes/re-measuring-a-planning-claim.md).

## An acceptance criterion that counts the OLD road's absence is satisfied by breaking the NEW one

A count of readers of the old value reaches zero both when the migration works
and when the replacement resolves garbage. Accept a migration with both halves
or neither: the absence count, plus a value witness that the new owner delivers
the same answer.

The same shape reaches guards and censuses. A threshold chosen from expectation,
not derived from the instrument's own output, fails the same way: a floor of
`>= 20` over a scan that can only see 5 is green for the wrong reason. Let the
instrument set the number, and make the accounting balance rather than clear a
bar.

An arm that names, in its failure message, what its own green would not have
proven is the arm you want failing.

## An equality assertion `f(a) == f(b)` is satisfied by every `f` that throws information away

The constant function passes every agreement arm. If a production road drops a
term, two sides that should agree still agree, and the defect makes the
assertion more true.

For every agreement arm, ask what the constant function would do to it. If the
constant passes, the other half of the claim is a disagreement arm, and it is
owed in the same commit: a different input must produce a different result.
`a_different_agreed_configuration_draws_a_different_sequence` and
`the_same_verdict_for_a_different_match_is_a_different_checksum` show the shape.
This is the same family as the section above, from the other end. Both are green
because nobody measured the instrument's discriminating power, only its verdict.

## A test must not identify its subject the way its subject forbids

An arm about deterministic identity must not pick its subject with
`query.iter().next()`. Bevy's iteration order is an allocation and archetype
artefact, the same artefact a rollback comparison exists to eliminate. `.next()`
names *a* body, never *the* body. An archetype change can flip the order and
make the arm fail falsely, or worse, pass falsely.

Capture the subject's `Entity`, or better its canonical `SimId`, when the
fixture acts on it, and read that back. A fixture that cannot name its subject
cannot witness anything about identity. When a production change reddens an arm
like this, suspect the arm first.
