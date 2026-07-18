# In-game GGRS rollback proof

Developer-visible Ambition builds run authoritative gameplay through
`GgrsSchedule`. Ordinary local play uses a zero-distance local SyncTest
session: GGRS drives each simulation tick but skips snapshot saves, historical
loads, and checksum comparisons.

Press **F9** during gameplay to request one bounded proof pulse. F9 is mapped
through the canonical `DeveloperAction` registry rather than read by the
observatory directly. The app rebases
its owned local session over the current world, runs a six-frame rollback and
the following SyncTest checksum comparison, then immediately rebases back to
the zero-distance baseline. The expensive determinism check is bounded to that
proof pulse rather than left running continuously. There is no observatory CLI
flag or environment variable.

The feature is included by `dev_tools`, so desktop-dev and `android_dev` share
the same control resource. A future Android developer menu, authored switch,
or developer item can request the same pulse.

## What you should see

The top-left HUD reports:

- current and confirmed GGRS frames;
- the historical frame loaded by the proof;
- rollback depth;
- `LoadWorld` and `AdvanceWorld` executions;
- frames executed during resimulation;
- rollback-authoritative and allocator-recreated entity counts;
- checksum verification or mismatch status.

Cyan outlines are body poses captured immediately after the real GGRS
`LoadWorld` schedule restores a historical frame. Lines connect each loaded
pose to the final pose after GGRS resimulates to the present. Magenta outlines
identify entities whose Bevy `Entity` allocation changed during rollback while
their stable `SimId` remained the same.

Move, jump, attack, spawn projectiles, or interact with enemies, then press F9.
After the checksum comparison the HUD reports `VERIFIED; BASELINE RESTORED` and the
historical outlines fade out. Press F9 again to repeat the proof.

## Platform-neutral activation seam

Keyboard input is only one adapter. Other developer surfaces request a pulse
through the public app resource:

```rust
fn request_from_developer_menu(
    mut control: ResMut<RollbackObservatoryControl>,
) {
    control.request_proof();
}
```

The adapter does not know how GGRS sessions are installed, rebased, or returned
to baseline.

## Input ownership

The observatory does not create a second keyboard, gamepad, or touch path.
Device adapters continue to populate the canonical `ControlFrame`, and the
existing `ControlFrameLatch` accumulates render-frame samples. The latch is
consumed inside GGRS's `ReadInputs` schedule, exactly when GGRS requests a
simulation input. This preserves short press edges even when several rendered
frames occur between simulation ticks.

## Validation

```bash
cargo fmt --all
cargo check -p ambition_app
cargo test -p ambition_runtime device_edges_are_consumed_when_read_inputs_runs_not_each_render_frame
cargo test -p ambition_app rollback_observatory::tests
cargo test -p ambition_app --features rl_sim --test app_it desync_canary::
```

The observatory never stores or restores simulation state itself. It observes
and presents the work performed by the single `bevy_ggrs` rollback authority.
Gameplay-trace recording skips historical replay passes, and pending trace files
are flushed only from `PostUpdate`; rollback replay cannot synthesize or duplicate
irreversible disk writes.

A successful local LDtk hot reload cancels any active proof pulse and restarts
the ordinary zero-distance baseline against the newly committed prepared-content
identity.
