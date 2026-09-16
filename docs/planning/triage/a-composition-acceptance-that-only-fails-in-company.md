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

## A third instance, 2026-09-16 — and this one is DETERMINISTIC

The named next step above was run, on a different subject, and it answers the
first open question for that subject: **parallelism, not ordering.**

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

⇒ **Deterministic in both directions**, unlike the 2026-09-10 observation. So the
class has at least three instances and this is the first with a reproduction
anyone can run in one command.

⚠ **AND ONE SUSPECT IS ELIMINATED.** `ambition_items::install_item_catalog` is a
documented process-global `OnceLock` — the obvious candidate — but it ALLOWS
identical reinstallation and both fixtures install the same catalog, so it is not
this. `probes.rs` holds no statics and `RollbackRestoreAudit` is per-App. The
shared state is elsewhere and is not yet named.

⇒ **WHAT THIS BUYS THE ROW:** a cheap, deterministic harness for the whole class.
Two concurrently-built sim Apps is a two-line fixture, so whoever takes this can
bisect the shared state by composing Apps with successively fewer plugins instead
of chasing an intermittent failure in a 733-arm binary.

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

A₂ is identical to A₁ on all three numbers. ⇒ **Order is innocent.** The failure
needs the two Apps running AT THE SAME TIME, which moves the search from shared
DATA to shared RUNTIME. First candidate: Bevy's process-global task pools —
`TaskPoolPlugin` initialises them once per process, so two Apps schedule their
systems onto one set of worker threads.

⛔ **AND NOT THE WALL-CLOCK TIMESTEP EITHER**, which was the standing candidate
for this whole class. `013b70c89` measured that `add_headless_foundation` leaves
`TimeUpdateStrategy::Automatic`, so fixed steps come from ELAPSED WALL TIME and a
contended box runs a different number of them — a machine-load-dependent world,
offered there as a mechanism to test for exactly this page. It does not apply to
this instance: `Platformer2dSimHarness::set_timestep` calls
`enable_manual_stepping` whenever rollback is enabled, both fixtures build with
`with_sync_test_rollback_settings`, and all three readings above show exactly 240
comparisons for 240 steps. A load-dependent world would also be INTERMITTENT, and
this failure is deterministic in both directions.

⇒ **WHAT REMAINS**, in the order that costs least: compose the second App with
successively fewer plugins until the production arm stops failing under default
parallelism, and see whether `TaskPoolPlugin`'s presence is the boundary. That is
a two-line fixture change per step, against a deterministic failure — the first
time this row has had either.
