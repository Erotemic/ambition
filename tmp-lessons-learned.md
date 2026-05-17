# Candidate lessons learned (draft, do not apply yet)

Drafted on 2026-05-09 while another agent overlays a zipfile patch.
None of these have been added to `docs/lessons_learned.md` yet. Each
one references a real fix in the git history; the bar set by the
existing entries is "diagnosis took >1 hour and the lesson saves the
next person time".

---

## ControlFrame edge fields cannot be derived from a held axis

**Date:** 2026-05-07. **Fixed in:** `ebe3686` (AgentAction path),
`42f3545` (mobile_input touch path), `a63c258` (touch fold stomping
keyboard).

### Symptom

Holding Down on the keyboard (or holding the touch joystick down)
caused the player sprite and camera to "shake" or "blink" at ~30 Hz —
oscillating between Standing and Crouching body modes. Two consecutive
held-down frames also incorrectly fired MorphBall via
`register_down_tap`'s double-tap-down detector. Symptom only appeared
on multi-frame held inputs; single-frame tests stayed green.

### Root cause

`ControlFrame.down_pressed` is documented as edge-triggered (true only
on the frame the input was just pressed). The desktop input pipeline
correctly populates it from Leafwing's `actions.just_pressed(MoveDown)`.
But three other `ControlFrame` writers each independently re-derived
the field as `down_pressed = move_y > 0.5` from the held axis,
producing `true` on every frame the user held Down. `register_down_tap`
counted each frame as a fresh tap, the double-tap window fired on
frame 2, MorphBall transitioned, then the next frame's body-mode
driver (which reads the held axis directly) flipped back. Per-frame
flip = ~30 Hz oscillation.

The third occurrence (touch fold) was a related shape: even with no
touch input, the touch fold ran every frame and unconditionally
overwrote `ControlFrame`, zeroing out the keyboard-derived
`down_pressed` between frames where Leafwing had set it true.

### Fix

Three coordinated changes:

1. **Don't auto-derive edge fields from a held axis.** Source structs
   that feed `ControlFrame` (`AgentAction`, `TouchInputState`) gain
   explicit `up_pressed` / `down_pressed` edge fields with
   `#[derive(Default)]` set to `false`. The source must opt in by
   setting the field once on the desired edge frame.
2. **Compute touch edges from a one-frame history.**
   `read_joystick_messages` keeps a `Local<f32>` of the previous
   frame's `move_y` and emits explicit `move_y_just_crossed_up` /
   `move_y_just_crossed_down` flags only on threshold crossings.
3. **Gate the touch fold on activity.** The fold checks
   `touch_state_is_active(...)` before writing; with no deflection /
   no held button / no edge flag, the existing `ControlFrame` is left
   intact for the keyboard-derived state to survive the frame.

Regression tests: `crouch_stability.rs` (held Down for 30 frames must
stay Crouching, per-frame `pos.y` delta < 5 px),
`fold_held_down_without_edge_flag_does_not_fire_down_pressed` (pins
the touch path).

### Takeaway

**Edge fields are a contract, not a derivation.** Any `ControlFrame`-
shaped resource with both axis fields and edge fields needs an
unambiguous answer to "who computes the edge, and from what?" — and
the answer can never be "from the held axis, in this writer". The
right shape is either:

- (a) a one-frame history kept by the source so the writer can detect
  threshold crossings, or
- (b) explicit edge fields on the source so the caller declares their
  intent.

When more than one source writes the same frame-rebuilt resource,
every writer must additionally gate on its own activity. A stateless
writer that runs every frame and unconditionally writes will overwrite
state another writer just computed.

This bug class is hard to spot in unit tests — held inputs need
multi-frame coverage. Add a "held axis for N frames" test for any new
input source the moment you wire it up.

---

## Local-copy ControlFrame doesn't propagate to other Bevy systems

**Date:** 2026-05 (mid-month). **Fixed in:** `da1151a`.

### Symptom

A double-tap-down gesture correctly entered fast-fall (visible in the
in-frame physics path), but the body-mode driver — a separate Bevy
system scheduled later in the frame — never saw the
`fast_fall_pressed = true` write that `input_timer_phase` performed
inside `sandbox_update`. Crouch worked because it reads the held
`controls.axis_y` populated upstream, but MorphBall never fired.

### Root cause

`sandbox_update` consumes `ControlFrame` and mutates a value the
inside-frame helpers share. But `ControlFrame` is *also* rebuilt every
frame from the input pipeline by
`populate_control_frame_from_actions.before(sandbox_update)`, and
later systems that read `Res<ControlFrame>` get the snapshot the
input pipeline put there — not the in-frame mutation. Worse, the
"fast_fall_pressed" field was being repurposed as a derived "double-
tap detected" signal, conflating the input boundary with a derived
state.

### Fix

Add a separate one-frame "pending edge" field on the long-lived
`SandboxRuntime` resource:

```rust
pub struct SandboxRuntime {
    pub double_tap_down_pending: bool,
    // ...
}
```

`input_timer_phase` sets it whenever `register_down_tap` returns true.
The body-mode driver consumes it via `mem::take` (so a stale signal
can't latch across frames) and `SandboxRuntime::reset` clears it
defensively. Add a regression test that sets `fast_fall_pressed = true`
on the resource directly and asserts the driver does **not** enter
MorphBall — that pins the routing and prevents future refactors from
silently re-coupling a derived signal back onto the input boundary.

### Takeaway

**Don't overlay derived gameplay signals onto an input-boundary
resource.** `ControlFrame` is what the *input pipeline* says happened
this frame; its values are rebuilt every frame. Anything derived from
input (gesture detections, multi-frame edges) belongs on a separate
state resource that lives across frames. The discipline pays off
twice: it's the right architectural seam (driver reads what driver
needs), and it makes the routing testable independently of the input
pipeline.

---

## Trace dump records state AFTER each step — replay must align accordingly

**Date:** 2026-05. **Fixed in:** `31d45ca`.

### Symptom

A determinism guard binary (`trace_replay`) re-ran a deterministic sim
against a `--dump-trace`-recorded fixture and reported sub-pixel `dx`
/ `dy` divergence by frame 1, accumulating across the trace. The sim
was deterministic and the dump was stable; the divergence was in the
replay loop's frame alignment, not in the sim.

### Root cause

The dump convention is "record state AFTER each step". So
`frames[i]` holds `(controls applied in step i+1, player_pos after
step i+1)`. The replay loop did `skip(1)` and applied
`frames[i].controls` on step `i` — i.e. it paired the controls of
frame `i` with the post-state of frame `i-1` in the comparison. Any
non-zero velocity introduces a one-frame offset that drifts as the
sim integrates.

### Fix

Apply `frames[i].controls` on step `i` (no skip), compare post-step
position to `frames[i].player_pos`. With the alignment correct, a
30-tick round trip reports `max_dx == 0.0`, `max_dy == 0.0`. That
makes `trace_replay` a real determinism guard usable as a CI fixture.

### Takeaway

**Write the alignment convention down at the dump site, in plain
words, next to the format definition.** Off-by-one on a record/replay
seam is invisible in a single-frame test and cumulative across long
traces — exactly the failure mode that's easy to look at, shrug, and
say "must be float drift". It isn't. Verify with `max_dx == 0.0`
end-to-end before claiming the determinism guard works; sub-pixel is
not "close enough" on a deterministic sim.

---

## Bevy 0.18 buffered events are now `Message`, not `Event`

**Date:** 2026-05 (early). **Fixed in:** `c49c1e5` and the rest of
ADR 0012's slices.

### Symptom

Migration to Bevy 0.18 broke `#[derive(Event)]` and
`EventReader<T>` / `EventWriter<T>` system params. The trait `Event`
still existed but had different requirements; `EventReader` was
unresolved.

### Root cause

Bevy 0.18 split the historical `Event` trait into two distinct APIs:

- The **buffered (queue-style) API** was renamed to `Message`. Use
  this for "system A produces N per frame, system B drains them later
  in the schedule". This is what the historical `Event` trait was
  used for in 99% of cases.
- The **observer-style API** (single dispatch, no buffering, fired
  via `world.observe(...)` or `commands.trigger(...)`) kept the
  `Event` name.

So in 0.18, `#[derive(Event)]` is now reserved for observer-style
one-shots and is incompatible with `MessageReader`/`MessageWriter`
system parameters, even though the physical role most code wanted
hadn't changed.

### Fix

Mechanical rename across the crate:

```text
Event           ->  Message            (#[derive(Message)])
EventReader     ->  MessageReader
EventWriter     ->  MessageWriter
Events<T>       ->  Messages<T>
add_event::<T>  ->  add_message::<T>
```

For each historical `Event`, decide: was it queue-style (rename to
`Message`) or genuinely one-shot (keep as `Event`, switch consumers
to observers)? In Ambition, all sandbox events were queue-style and
became `Message`s.

### Takeaway

**On a Bevy minor-version migration, read the changelog for renames
before searching for runtime errors.** This one is a clean rename;
`cargo check` reports the old names as unresolved but doesn't tell
you the new names. The compiler error is necessary but not sufficient
guidance; the changelog explains *why* the trait split.

---

## How to add to this file (unchanged from existing template)

Use the existing template at the bottom of `docs/lessons_learned.md`.
Skip the pretty narrative; aim for grep-ability — somebody hunting
for "down_pressed every frame" or "ControlFrame stomping" or
"frames[i].controls" should land on the right entry.

---

## NOTES — what NOT to add

The following candidates I considered and rejected as not meeting the
"diagnosis took >1 hour" bar:

- **`bevy_ecs_ldtk` IntGrid renders by default** — already in the
  existing lessons file (top of doc).
- **LDtk `__cWid` ceil vs floor** — already in the existing lessons
  file.
- **Greedy row-major rect-merge produces vertical bars on diagonals**
  — already in the existing lessons file.
- **Wall-cling y-sweep teleport** — already in the existing lessons
  file (top entry, with extensive followup).

These are all great prior lessons — just no need to re-add.

The following bug-fix commits I considered for new lesson entries but
the diagnosis was straightforward enough that they belong in commit
messages, not in the lessons doc:

- `b8c0f84` (music: fix goblin encounter intro→loop volume drop). The
  diagnosis was "intro plays the mastered full mix, loop plays
  per-stem outputs that bypass the mastering chain → wave stems
  measure -50 LUFS where intro is -15.5". Worth a note in the music
  renderer docs, but not a >1hr lesson — once you know the renderer's
  per-stem-vs-full-mix path, the fix is direct.
- `cbdea83` (joystick floating vs fixed). UX choice, not a debugging
  lesson.
- `45 5c436` (joystick `axis()` vs `value()`). Library API misuse —
  one-line fix.

Things to add to the lessons doc IF the in-progress audio/music/input/
trace refactor surfaces a hard-to-diagnose failure:

- A "things that go wrong when splitting a `pub mod foo` file with a
  bunch of `pub use` re-exports at the crate root" entry would be
  high-value — the existing benchmark question covers the
  `COYOTE_TIME` case but if more failure modes surface (e.g. doc
  comments orphaned from items, `#[derive(...)]` left attached to a
  function, helper-fn visibility downgrade), each one is a different
  invariant worth recording.
