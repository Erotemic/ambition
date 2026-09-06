# Per-player-component mirror: schedule boundary determines which readers can migrate

**Trap shape**: A Bevy app introduces a per-player component
(`PlayerInputFrame`) that mirrors a global resource (`Res<ControlFrame>`)
so a future multiplayer build can give each player their own input
frame. A "sync" system (`sync_local_player_input_frame`) snapshots
the resource onto the local primary player's component each tick.
Then individual readers are migrated incrementally from
`Res<ControlFrame>` to `&PlayerInputFrame` — and it looks like every
reader is a candidate, because both expose the same struct shape.

The mistake: assume any reader can be migrated. The actual rule is
**only readers that run *after* the sync system in the same frame
see this-frame data**. Mid-chain readers will silently observe the
previous frame's snapshot.

**Concrete shape** (Ambition's `SandboxSet::PlayerInput` chain):

```rust
(
    // ... time-control bookkeeping ...
    input_timer_system,           // writes ControlFrame.fast_fall_pressed
    interaction_input_system,     // reads ControlFrame.interact_pressed
    sync_local_player_input_frame, // ControlFrame -> PlayerInputFrame
)
    .chain()
    .in_set(SandboxSet::PlayerInput);
```

Naive migration: switch `interaction_input_system` from
`Res<ControlFrame>` to `&PlayerInputFrame`. The signature change
compiles, and Ambition's existing tests pass — because the test
buffer window is multiple frames at 60 Hz, so a 1-frame lag is
invisible to the buffered-interact unit tests. The bug only
manifests as a 1-frame input lag in-game, well below the threshold
most agents would think to verify.

**Decision rule**: a sandbox/sim system can migrate from the global
resource to the per-player component if and only if it runs at or
after the sync system in the same frame. In Ambition's schedule,
that means:

- Systems in `SandboxSet::Progression` or later sets — safe.
- Systems in `SandboxSet::PlayerInput` chain BEFORE the sync — must
  keep reading the resource.
- Systems in `SandboxSet::PlayerInput` chain AFTER the sync (if you
  add any) — safe.

**Why a future agent will get this wrong**: the migration pattern
looks mechanical ("replace `Res<ControlFrame>` with
`Query<&PlayerInputFrame>` on systems that already query a player
entity"). The schedule-order subtlety is invisible from the
signature alone, and tests that exercise the system in isolation
(spawning a player + scheduling the sync directly before the system
under test) don't reproduce the live chain order. A test that
schedules the system AFTER the sync will pass even though the live
build runs it BEFORE the sync.

**Pre-flight checks** before migrating:

1. Identify where the sync system is scheduled. If your reader runs
   in the same set with `.chain()` ordering, check whether your
   reader comes before or after the sync.
2. If the reader is mid-chain, do NOT migrate. Document the boundary
   inline (`// reads ControlFrame because we run before sync`).
3. If you do migrate, add a test that schedules the sync at the END
   of the same chain (matching the live order) and asserts the
   reader sees this-frame data.

**Ambition session example** (2026-05-20): one C-bucket migration
succeeded (`update_body_mode`, lives in `SandboxSet::Progression`
which runs after PlayerInput); a second attempted migration
(`interaction_input_system`) was reverted after recognizing it ran
before the sync in the same chain.

**Bench question for a future agent**: "Here is a Bevy app where a
per-player component is mirrored from a global resource by a sync
system. Migrate all sandbox/sim readers from the resource to the
component. Explain your decision for each reader, especially any
you choose to leave on the resource."

The expected answer: walk the system graph, identify which readers
run after the sync system, and migrate only those. Document the
schedule-order rule on any reader left on the resource.

**Tags**: `bevy-schedule-order`, `per-entity-mirror`,
`game-input`, `cross-system-signal`, `architecture-seam`.
