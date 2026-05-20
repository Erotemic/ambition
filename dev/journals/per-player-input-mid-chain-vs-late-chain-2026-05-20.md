# 2026-05-20: A per-player input migration looked mechanical but stale-read the previous frame mid-chain

Today's #17.5 follow-up batch tried to incrementally migrate the
remaining `Res<ControlFrame>` readers in the sandbox onto the
per-player `PlayerInputFrame` component. The migration pattern looked
straightforward — the projectile / sandbox_update / attack_advance /
record_frame readers had already moved cleanly in the first slice, and
each migration was a 3-line diff: drop `Res<ControlFrame>` from the
system signature, add `&PlayerInputFrame` to the existing player
Query, read `input.frame.<field>` instead of `controls.<field>`.

`update_body_mode` migrated cleanly. The unit tests caught the missing
`LocalPlayer` + `PlayerInputFrame` components on the test fixture
(`body_mode/tests.rs::body_app`), and adding them plus scheduling
`sync_local_player_input_frame` before `update_body_mode` made the 22
body-mode tests pass. Existing tests that mutated `Res<ControlFrame>`
directly kept working because the sync still mirrored the resource
onto the component on each tick.

Then I migrated `interaction_input_system` in `app/sim_systems.rs` to
the same pattern. The change compiled, the tests passed, the diff was
3 lines.

The bug: `interaction_input_system` runs in the middle of the
`SandboxSet::PlayerInput` chain — *before*
`sync_local_player_input_frame` (which is the last system in the
chain). Reading `PlayerInputFrame.frame.interact_pressed` mid-chain
returns the **previous frame's snapshot**, because the sync hasn't
fired yet for the current frame.

```rust
// Schedule order inside SandboxSet::PlayerInput (.chain()):
(
    // … time-control bookkeeping …
    input_timer_system,           // writes ControlFrame.fast_fall_pressed
    interaction_input_system,     // reads ControlFrame.interact_pressed
    sync_local_player_input_frame, // ControlFrame → PlayerInputFrame
)
```

After migration, `interaction_input_system` reads
`PlayerInputFrame.frame.interact_pressed`, but the per-player
component was last written by sync **at the end of the previous
frame's chain**. So the interact buffer would arm one frame later
than expected. At 60Hz the lag is invisible in unit tests (the
buffered-interact window is multi-frame) and would only surface as a
~17ms feel issue in-game.

The unit tests didn't catch it because they were structured the wrong
way for this class of bug. The test schedules
`sync_local_player_input_frame` *before* the system under test, which
matches a *late-chain* reader but not the *mid-chain* shape that
`interaction_input_system` is in production. So both versions
(stale-reading and fresh-reading) pass the same tests.

I reverted the migration and added a paragraph-long comment to
`interaction_input_system` explaining the boundary rule, then logged
it in `OVERNIGHT-TODO.md`'s #17.5 retired-items bullet:

> Systems running in `SandboxSet::Progression` or later sets can read
> `PlayerInputFrame` safely; systems running mid-`SandboxSet::PlayerInput`
> MUST continue to read `Res<ControlFrame>` because
> `sync_local_player_input_frame` runs at the end of the input chain
> and mid-chain reads would deliver the previous frame's snapshot.

## Lessons

1. A per-entity mirror system creates a **schedule-order tax** on
   downstream readers. The migration looks signature-level mechanical
   but actually requires walking the system graph to confirm the read
   happens after the mirror's write.
2. Tests that exercise the system in isolation (with the sync
   scheduled before the SUT) hide the bug because the test's schedule
   doesn't match the live chain order. If you migrate a mid-chain
   reader, you also need a test that schedules the sync AFTER the SUT
   to prove the bug is absent.
3. The pre-release no-compat rule says to migrate aggressively, but
   "migrate aggressively" assumes the migration is semantically a
   no-op. When the migration introduces a 1-frame data-staleness gap,
   it's not a no-op, even if the immediate tests pass. The right move
   is to revert + document the boundary rule so the next agent
   doesn't re-attempt it.

The distilled invariant lives in
`dev/benchmark-candidates/per-player-component-mirror-schedule-boundary-2026-05-20.md`
as a benchmark candidate for future agents.
