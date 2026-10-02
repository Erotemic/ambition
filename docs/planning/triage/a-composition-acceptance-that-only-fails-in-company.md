# A composition acceptance that only fails in company

**Status:** open. Filed 2026-09-10. Owner row: `TEST-LANES` in
[`../queue.md`](../queue.md).

## Finding

`composes_through_the_sdk::a_host_that_omits_boss_encounters_still_builds_and_steps`
(`game/ambition_app/tests/composes_through_the_sdk.rs`) failed once inside a
full `cargo test --workspace` run and passed when run alone. Its assertion was
never captured. A minimal-host composition test is valuable only if nothing
else loaded can change its verdict, so an in-company failure means that its
isolation claim is not proven.

`app_it` runs its arms as threads of one process, so process-global state is a
channel between arms. Three other instances of this signature were per-arm
measurements that read process-global state, and they are fixed. The cause of
the one that had a measured cause was a `static AtomicUsize` input cadence in
the test file itself (`how_much_of_the_peer_checksum_actually_varies.rs`), not
an engine global. `scripts/a_test_static_is_a_channel_between_arms.py` guards
that class in `tests/`.

Eliminated for the open instance, by reading the code:

- `composes_through_the_sdk.rs` holds no `static`.
- `ambition_platformer2d_runtime` does not depend on `ambition_render`.
- The two globals in `ambition_boss_encounter` (a warn-once `BTreeSet` and a
  read-only `LazyLock` catalog) cannot zero a fixed-step count or panic a
  plugin build.

The next candidate of the class is `hall_redecode_census.rs`, which asserts
over a delta of a process-wide counter.

## Rules for this class

- ⛔ Do not add a retry.
- Check the instrument's own globals before the subject's.
- Read the failure text before the failure pattern. Two arms with the same
  panic at the same source line are a deterministic check, not this class.
- A solo pass is a different population from a run in company. It is not a
  negative result.
- An intermittent fault needs a repeat count sized from its rate. At the last
  measured rate (5 of 20, 2026-09-16), about 25 clean runs are needed before a
  variation can be called clean.

## Evidence command

```bash
cargo test -p ambition_app --test app_it -- --test-threads=1
cargo test -p ambition_app --test app_it
```

Repeat each configuration until the rate is known, and capture the full output
to a file.

## Trigger to promote

Promote to a queue row when the failing arm's assertion is captured, or when a
second arm in `app_it` fails in company with a different panic text.
