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
