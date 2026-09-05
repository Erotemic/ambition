# Every link true and the chain false: a referent that drifts across a valid argument

**Tags:** `identifier-identity`, `planning-truth`, `submodule-pin`, `silent-degradation`, `agent-verification`

## The shape

A conclusion rests on two or three statements that are each independently
verifiable and each TRUE. The conclusion is false anyway, because two of the
steps use one name for two different objects. Nothing in any single statement
reveals it; the drift lives in the join.

```text
step 1   "the script defers to the renderer refusing"        TRUE
step 2   "the renderer refuses, and has tests for it"        TRUE
                                        └─ of the BRANCH
conclusion "the ask is guarded"                              FALSE
                                        └─ the PIN ships a different renderer
```

⇒ **This is not sloppy reasoning and it does not look like any.** Checking the
argument step by step confirms it. The only thing that finds it is asking, of
each shared noun, *which object is this, exactly* — and that question is not
prompted by anything in the text.

⛔ **The dangerous form is a CLOSED item.** An open row invites checking; a row
marked ✔ with a sound argument attached is protected by its own correctness. A
reader who spot-checks the reasoning finds it holds and moves on.

## Three 2026-09-04 instances, in unrelated subsystems

**1. A dialogue gate that can never open.** `kernel.yarn` asks
`boss_cleared("mockingbird")` — a BEHAVIOUR id. The writer stores
`set_boss(&runtime_id, Cleared)` where `runtime_id` is the PLACEMENT id
(`BossSpawn-4308`). The evaluator does an exact string lookup. Every statement
about each layer is correct; the chain is broken because "the boss" names two id
spaces. Five authored calls have never been able to return true.

**2. A standing user requirement, recorded as guarded, unguarded in every
clone.** A repo script warns-and-continues and defers explicitly to "the
renderer" refusing to ship General-MIDI stand-ins. The renderer does refuse — on
an unmerged branch. The superproject pins a commit whose `cli.py` contains ONE
match for the concept and it is a comment. The planning entry that concluded
"guarded" reasoned correctly about a renderer that is not the one a fresh clone
gets.

**3. Right module, wrong defect.** Three tests failed on a correctly-provisioned
machine; the diagnosis named `matplotlib`, which is indeed the module involved
and is indeed undeclared. But the production docstring says *"Matplotlib is
intentionally optional"* and the code writes a skip note. The proposed fix —
declare it — would have made an intentionally-optional dependency mandatory:
fixing a guard by changing the contract it guards. The tests were asserting an
optional path unconditionally.

## The invariant

**When a conclusion joins two statements through a shared noun, the noun is a
variable, not a constant. Bind it on both sides before believing the join.**

Concretely, the three questions that would have caught all three:

* *which ID SPACE?* — a behaviour id and a placement id are both "the boss".
* *which COMMIT?* — a branch and a pin are both "the renderer".
* *which is the CONTRACT and which is the TEST?* — a missing dependency and an
  optional dependency are both "matplotlib is not installed".

## A hard question

> A planning document contains this closed item:
>
> > ✔ **Is the "no General-MIDI fallbacks" requirement guarded?** Yes.
> > `scripts/regen/music.sh` warns and continues when the instrument libraries
> > are missing — so the repo script is *not* the protection; it defers entirely
> > to the renderer refusing, which its own comment says outright. And the
> > renderer does refuse, at two levels, both with tests in
> > `tools/ambition_music_renderer/tests/`.
>
> Every factual claim in that item is true. The requirement is nonetheless
> unprotected in a fresh clone. What is wrong, and what single command would
> have shown it?

**Expected answer.** `tools/ambition_music_renderer` is a SUBMODULE. The tests
and the refusal exist on a branch; the superproject pins a different commit, so
"the renderer" in step two is not the renderer step one defers to. The
discriminator is to look at the PINNED tree rather than the working tree —
e.g. `git ls-tree --name-only origin/main tests/ | grep fallback`, or
`git show origin/main:ambition_music_renderer/cli.py | grep -c REFUSE` against
the same on the branch (1 comment vs 10 real matches).

⚠ A model that answers *"run the test suite"* has not solved it: the suite passes
in the working tree, which is checked out at the branch.

## Validation

The repair is not a code change: it is a fast-forward of the submodule's `main`
plus a pointer bump. ⛔ Note the trap in the obvious alternative — reverting the
pointer to the guarded commit trades an unguarded-but-durable pointer for a
guarded-but-DANGLING one, because that commit is on a deletable agent branch.

A durable guard for this class would compare the pinned submodule tree against a
declared requirement, rather than the checked-out one. None exists yet.
