# A composition acceptance that only fails in company

**Status:** open, unattributed. Filed 2026-09-10.

## What was seen

`composes_through_the_sdk::a_host_that_omits_boss_encounters_still_builds_and_steps`
**FAILED** inside a full `cargo test --workspace` run, and **PASSED** when run
alone against the same tree, twice:

    cargo test --workspace                     -> FAILED (622 passed, 3 failed)
    cargo test -p ambition_app --test app_it \
        composes_through_the_sdk::a_host_...   -> ok
    cargo test -p ambition_app --test app_it   -> did NOT reproduce
                                                  (623 passed, 2 failed --
                                                  the 2 were an unrelated
                                                  rollback-coverage red)

⇒ **Intermittent, not deterministic-in-company.** A single re-run of the same
binary did not reproduce it, so this is not simply "test B leaves state that
test A reads" — or if it is, the interleaving that does it is not the default
one.

## Why it is worth a row rather than a retry

⛔⛤ **A MINIMAL-HOST COMPOSITION TEST IS EXACTLY THE TEST WHOSE VALUE DEPENDS ON
NOTHING ELSE BEING LOADED.** Its subject is that a host which omits
`BossEncounterSimulationPlugin` still builds and steps. If a sibling test can
change its verdict, then **the isolation it claims to prove is not the isolation
it has** — and the failing direction is the dangerous one, because a green run
is then also uninformative.

⚠ Its own doc says *"the only thing that would make it fail is a boss system
that some other capability turns out to require."* That sentence is a claim
about the SUBJECT. This failure is a claim about the HARNESS, and the two are
indistinguishable from the exit code.

## What has NOT been established

- **Whether it is ordering-dependent or timing-dependent.** Not measured.
- **Whether it predates 2026-09-10.** Not measured. It was observed on a tree
  carrying uncommitted occurrence work; nothing in that work touches plugin
  composition, but that is reasoning, not a bisect.
- **Which sibling, if any, is involved.** Not measured.

⇒ The next step is a repeat run with `--test-threads=1` and with a fixed seed
order, comparing against the failing composition — **not** a fix.

## A third instance, 2026-09-16 — INTERMITTENT, like the first

⛔⛤ **THIS SECTION FIRST SAID "AND THIS ONE IS DETERMINISTIC". IT IS NOT, AND THE
CLAIM CAME FROM ONE OBSERVATION PER DIRECTION.** Repeating the closest
configuration three times gave **pass, pass, FAIL**. ⇒ The class is therefore
UNIFIED rather than split: all three instances are intermittent, which is itself
the most useful thing this instance contributes, because it means a single
mechanism can still explain all of them.

⚠ **AND IT INVALIDATES THE ELIMINATIONS I DREW FROM ONE OR TWO CLEAN RUNS.**
⇒ **THE RATE IS NOW MEASURED: 20 runs of the same configuration gave 15 pass,
5 FAIL — 25%.** At one in four, a single clean run happens three times in four
even when the fault is fully present, so "this variation did not reproduce it" is
not evidence of absence. Recorded so nobody builds on them: a second sim App built-and-stepped
without the audit read clean twice, and one with an `Update` writer and no audit
read clean once. **Those are not negatives.** The only elimination below that
does not depend on run counts is the wall-clock one, because it rests on reading
the code rather than on counting failures.

The named next step above was run, and it answers the first open question for
this subject: **parallelism, not ordering.**

Building a SECOND sim App inside the `app_it` process makes a neighbouring
audit-based arm report a corrupted baseline. Subject:
`how_much_of_the_peer_checksum_actually_varies::no_registered_type_is_written_outside_the_rewinding_schedule`,
which asserts that no rollback-registered type is written outside the rewinding
schedule. Adding a second fixture in the same file —
`run_with_a_writer_outside_the_schedule`, a second full sim App — made it report
**99** types instead of none, which is the signature of a stale comparison
baseline rather than a finding.

    the new arm alone                         1 passed
    the audit arm alone                       1 passed
    both, `--test-threads=1`                  2 passed
    both, default parallelism                 1 passed, 1 FAILED

⇒ Those were one run each. **Repeated three times, the same configuration gave
pass, pass, FAIL** — so the class has at least three instances and all of them
are intermittent. What this one does contribute is a CHEAP reproduction: two
fixtures in one file, ~16 s a run, versus a 733-arm binary.

⚠ **AND ONE SUSPECT IS ELIMINATED.** `ambition_items::install_item_catalog` is a
documented process-global `OnceLock` — the obvious candidate — but it ALLOWS
identical reinstallation and both fixtures install the same catalog, so it is not
this. `probes.rs` holds no statics and `RollbackRestoreAudit` is per-App. The
shared state is elsewhere and is not yet named.

⇒ **WHAT THIS BUYS THE ROW:** a cheap 16-second reproduction for the whole class,
instead of a 733-arm binary. ⚠ **NOT a deterministic one** — that word was
published off one observation per direction and RETRACTED the same day when the
configuration was repeated three times and gave *pass, pass, FAIL*. Two
concurrently-built sim Apps is a two-line fixture, so whoever takes this can
bisect by composing Apps with successively fewer plugins — but each configuration
needs a REPEAT COUNT sized from the rate, not one run.

⚠ The new arm is `#[ignore]`d rather than deleted, with the measurement in its
doc, so the lane stays green and the reproduction is not lost. Run it with
`--ignored` or with `--test-threads=1`.

### Two mechanisms eliminated, and the search narrowed to shared RUNTIME

⛔ **NOT LEAKED STATE — MEASURED, not reasoned.**
`probe_whether_a_second_sim_app_leaves_state_behind` runs three sim Apps
sequentially in one thread: the production fixture, then the second App, then the
production fixture AGAIN. If a second App left process-global state behind, the
third reading would differ from the first.

    A1  (production fixture, first)    live_comparisons=240 outside=0 moved=32
    B   (writer outside the schedule)  live_comparisons=240 outside=0 moved=32
    A2  (production fixture, AFTER B)  live_comparisons=240 outside=0 moved=32

A₂ is identical to A₁ on all three numbers. ⚠ **ONE RUN, so this argues against a
DETERMINISTIC leak — which is what a leak would be — without excluding an
intermittent one.** Taken with the intermittency, the search still points at
shared RUNTIME rather than shared DATA. First candidate: Bevy's process-global
task pools, since `TaskPoolPlugin` initialises them once per process and two Apps
then schedule their systems onto one set of worker threads.

⚠ **AND A BARE `MinimalPlugins` APP RUNNING 240 UPDATES CONCURRENTLY READ CLEAN**
— one run, so not a negative either, but it is the cheapest variation to repeat
enough times to become one, and if it stays clean over ~20 runs the shared thing
is not merely "two Apps on one task pool".

⛔ **AND NOT THE WALL-CLOCK TIMESTEP EITHER**, which was the standing candidate
for this whole class. `013b70c89` measured that `add_headless_foundation` leaves
`TimeUpdateStrategy::Automatic`, so fixed steps come from ELAPSED WALL TIME and a
contended box runs a different number of them — a machine-load-dependent world,
offered there as a mechanism to test for exactly this page. It does not apply to
this instance: `Platformer2dSimHarness::set_timestep` calls
`enable_manual_stepping` whenever rollback is enabled, both fixtures build with
`with_sync_test_rollback_settings`, and all three readings above show exactly 240
comparisons for 240 steps. ⛔⛤ **AND THE FIRST VERSION OF THIS
PARAGRAPH CLOSED WITH A SECOND ARGUMENT THAT HAS SINCE BEEN RETRACTED** — *"a
load-dependent world would also be INTERMITTENT, and this failure is
deterministic in both directions."* The failure IS intermittent, so that leg is
gone. ⇒ **The elimination survives anyway, and only because the surviving leg is
STRUCTURAL: it reads the code rather than counting failures.** A structural
elimination does not care how many times anything ran; a statistical one is
worthless without a rate. ⚠ A retraction has to be walked into every sentence
built on it — this one sat two paragraphs below the correction for part of a day.

⇒ **WHAT REMAINS, AND THE METHOD MATTERS MORE THAN THE CANDIDATE.** An
intermittent fault cannot be bisected one run at a time. Each configuration needs
a REPEAT COUNT chosen from the observed rate before its result means anything —
at roughly one in three, ~20 runs to call a variation clean with any confidence.
⇒ So the next step is a loop, not a fixture. **The baseline rate is 25% (5 of 20),
measured 2026-09-16**, which is what every later run count must be sized against:
ten cleans put a 95% upper bound near 26% and therefore establish nothing here,
and roughly twenty-five cleans are needed before a variation can be called clean.
⇒ Run the cheapest variation (bare `MinimalPlugins` alongside the production arm)
twenty-five times, then a built-but-unstepped sim App. A rate is the measurement;
a single pass is not.

⚠ Nobody should take this row expecting a quick answer. What it now has that it
did not have on 2026-09-10 is a 16-second reproduction and a known rate to size
the runs against.
