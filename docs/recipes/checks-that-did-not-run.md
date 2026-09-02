# The check that was correct and did not run

A test that fails is information. A test that never executed and reports green
is worse than no test at all, because it also spends the attention that would
have found the defect by hand.

This page is the dual of
[`cheapest-sufficient-check.md`](cheapest-sufficient-check.md). That page asks
*what is the least I can run to settle this change*. This one asks *did what I
ran actually run* — and it exists because on 2026-09-02 a single day's work
turned up **seven** members of the same family in one gate script.

⭐ **THE FINDING IS NOT THE SEVEN. It is that five of the seven were found by
accident** — by running a suite for an unrelated reason, by an external reviewer
reading source, by running `cargo check` by hand before the gate. Not one was
found by the checking system noticing its own hole. A gate cannot audit its own
coverage, because the same assumption that makes a job skip also makes the
report say it ran.

## The four questions

Ask these of any check you are about to trust. They are cheap, and each one
found at least one member below.

1. **What does this flag skip?** A scope flag is a promise about what you are
   NOT running, and that promise is usually only in the help text.
2. **Which plan is this job in?** A job that exists in the exhaustive plan and
   not the default one does not run on the command anybody actually types.
3. **Does it re-derive, or does it reuse a cache?** A scan over incremental
   build output sees only what recompiled.
4. **What would this look like if the feature were silently off?** If the answer
   is "exactly the same, and green", the check is measuring the harness.

⛔ **ASK THEM TOGETHER, NOT SEPARATELY.** Member #6 below is blind in two
independent ways at once, and each question in isolation returned a locally
reassuring answer. "Which plan is this in" and "which branch does this compile"
were both asked of that job, months apart, and it stayed broken.

## The seven, and what each one teaches

| # | the check | how it lied | status at `f9ffc3fbc` |
|---|---|---|---|
| 1 | `./run_tests.sh --rust` | skipped the whole Python lane, so the rollback stable-name ratchet, the codec-shape baseline and a stale `MODULES.md` sat red **for a day** behind a gate reporting 4/4 GREEN | fixed `2945f3381` |
| 2 | the wasm build job | gated on `if not only and everything` — the exhaustive plan only, so a default run never checked `wasm32` at all | fixed `b85b4db20` |
| 3 | `check_no_warnings.py` | parses diagnostics instead of setting `-D warnings`, so it reuses the build fingerprint — and **cached crates do not re-emit warnings**. In a warm tree it reports clean while real warnings exist | ⛔ **STILL LIVE.** Its own docstring offers `--fresh`; `run_tests.py:368` invokes it without |
| 4 | the repo-tooling lane | simply not invoked by the flag anybody was using | fixed `2945f3381` |
| 5 | the wasm CHECK | is **TYPE-ONLY**. `cargo check` cannot see a `#[cfg]` that removes BEHAVIOUR rather than breaking a build | ⛔ **STRUCTURALLY LIVE** |
| 6 | "web persona BOOTS" | runs the web composition **NATIVELY** (`--features visible_web_base`, native target), so it compiles the `not(wasm32)` branch — and `run_tests.py:588` gates it on the exhaustive plan | ⛔ **STRUCTURALLY LIVE, TWICE** |
| 7 | the coverage footer | said `- the wasm/web build LINK (the wasm CHECK ran)` **unconditionally**, while the job is appended only `if wasm_target_installed()`. No target → no web job, all green, exit 0, and a report that it was checked | fixed `159e76ba8` |

Between #5 and #6 the web path had **zero behavioural coverage**: one job could
not execute code, and the other executed the wrong branch of it. The hole that
exposed them was real — the web reveal barrier never waited for the GPU, because
the entire render-world half was `#[cfg(not(wasm32))]`. The commit that fixed it
names the mechanism exactly: *"The web reveal never waited for the GPU, because
the FACT was gated with the CLOCK"* (`2d623308f`, branch `web-gpu-wait`). A
`#[cfg]` that removes a timing concern took the fact it was timing with it, and
no type check can see that shape.

## The three remedies, and which one you are actually reaching for

Reading the fixes together is more useful than reading any one of them, because
they are not the same kind of fix.

- **Make the job run.** #2 moved the wasm CHECK into the default plan. This is
  the only remedy that adds coverage, and it is the rarest, because it costs
  time on every run — 505 s cold and 26 s warm, in that case, and the team took
  it for the CHECK and refused it for the LINK.
- **Make the silence audible.** The dominant remedy. #7's footer now derives
  from the *planned* jobs rather than asserting; #2's `elif not only:` branch
  exists for no purpose but to print that the web build is UNCHECKED; and #1's
  fix kept a `--rust-alone` that still skips everything — but its help text now
  reads *"those went red unnoticed for a day the last time that happened."*
  ⭐ Note what that means: the blindness was not removed, it was **named and made
  loud**. That is a legitimate outcome, and it is the one to aim for when
  coverage is genuinely too expensive.
- **Accept that it cannot be fixed, and compensate elsewhere.** #5 and #6. A
  type check will never execute code and a native run will never be a wasm run.
  The only defence is a human knowing the gap exists, which is why it is written
  down here instead of filed as a bug.

## Before you believe an error list, diff it against a clean checkout

`check_agent_kb.py` could not pass in an agent worktree. It used
`Path.resolve()`, which follows symlinks, and worktree seeding makes
`.agent/README.md` a symlink into the primary tree — so two phantom "links
outside repo" errors appeared **in every worktree and in no clean checkout**.

An external reviewer saw three errors. The session in the worktree saw five. Two
of them were its environment.

⛔ **THE GENERAL RULE: your error list is a property of your environment until
you have compared it with one you did not build.** This cuts both ways — the
extras may be phantom, and a clean checkout may also show you something your
warm tree has been hiding since member #3.

## What this page cannot do

It cannot make a gate honest. Every member above was found by a person asking
one of the four questions about a specific job, and the gate is still the thing
that will tell you it is green. The honest claim is narrow: these are the shapes
that have actually lied in this repository, so that the next one is recognised
rather than rediscovered.

Related: [`cheapest-sufficient-check.md`](cheapest-sufficient-check.md),
[`../reviewer-guide.md`](../reviewer-guide.md) (§Testing).
