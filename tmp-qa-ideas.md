# Candidate benchmark questions (draft, do not apply yet)

Drafted on 2026-05-09 while another agent overlays a zipfile patch. None of
these have been added to `dev/benchmark-candidates/rust-questions.md` yet.
They are reconstructed from real fixes already in `git log`. Each entry
flags whether it is **Level A** (pre-error operation), **Level B** (error
repair), or **Level C** (minimal language/API rule), per the workflow in
`dev/benchmark-candidates/README.md`.

If/when the in-progress audio/music/input/trace split surfaces compile or
test failures, prefer adding a fresh Q distilled from THAT failure first
— the pre-error context is freshest and the failure is brand-new. The
items below are candidates I dredged up from older commits that the
current `rust-questions.md` doesn't cover yet.

---

## Q1 — Don't auto-derive an edge flag from a continuous axis (Level A, recurring)

**Source commits:** `ebe3686` (AgentAction crouch flicker),
`42f3545` (mobile_input touch path repeat of the same bug),
`a63c258` (mobile fold stomping keyboard ControlFrame).

**Why this is a strong candidate:** the *exact same bug class* shipped
**three times** in different writers — this is high-signal evidence that
a default-thinking agent will reach for the same wrong shape. The fix
pattern (explicit edge fields, gate writes on activity) is the
benchmark.

### Setup

`ControlFrame` is a per-frame Bevy resource that downstream simulation
code reads. Two of its fields are documented like this:

```rust
pub struct ControlFrame {
    /// Continuous axis from -1.0 (held up) to 1.0 (held down).
    pub move_y: f32,
    /// True only on the frame the player JUST pressed Down. False
    /// while held. Used by `register_down_tap` to count distinct taps;
    /// two distinct taps within the double-tap window fire the
    /// MorphBall transition.
    pub down_pressed: bool,
    // ...
}
```

The desktop input pipeline populates `down_pressed` from
`actions.just_pressed(MoveDown)` (Leafwing's edge query) and `move_y`
from the held axis.

You're now writing a second source — an `AgentAction` → `ControlFrame`
converter for an RL agent — that emits one `AgentAction { move_y: f32, ... }`
per frame.

### Question

Sketch the converter. What field-by-field rule do you use for axis vs
edge fields, and what would the symptoms be if you got it wrong on
`down_pressed`?

### Expected answer

- **Continuous axes (`move_y`, `move_x`, aim sticks)** copy across
  every frame.
- **Edge flags (`down_pressed`, `up_pressed`, `jump_pressed`, etc.)
  must NOT be derived from the held axis.** Either give the source a
  matching explicit edge field (`AgentAction { down_pressed: bool, ... }`,
  default `false`, set true only on the frame the agent wants the
  edge), or compute the edge from a one-frame history of the source's
  own axis. The held axis being above a threshold every frame is
  exactly the wrong signal — `register_down_tap` will count it as a
  fresh tap each frame, double-tap-down fires on frame 2, MorphBall
  triggers on held Down, and the player's body mode oscillates
  ~30 Hz.

When more than one source can write `ControlFrame` in the same frame
(e.g. keyboard + touch), each writer must also gate on its own
activity rather than unconditionally writing — otherwise the inactive
source zeroes out the active one each frame, producing the same
~30 Hz oscillation symptom on the held-state axis fields.

### Why this was easy to miss

`down_pressed = move_y > 0.5` is the *natural* shape for a translator
that wants to feel responsive and has no per-source frame history.
The bug is invisible in single-frame tests; only a multi-frame held
input reproduces it.

---

## Q2 — Local-copy ControlFrame won't propagate to other Bevy systems (Level A)

**Source commit:** `da1151a` (route double-tap-down edge through SandboxRuntime).

### Setup

You have a Bevy schedule like:

```rust
fn sandbox_update(mut controls: ResMut<ControlFrame>, /* ... */) {
    input_timer_phase(&mut controls, &mut runtime);
    player_simulation_phase(&controls, &mut player, &mut runtime);
    // ...
}

fn body_mode_driver(controls: Res<ControlFrame>, runtime: Res<SandboxRuntime>, /* ... */) {
    if controls.fast_fall_pressed {
        // enter MorphBall
    }
}
```

`input_timer_phase` detects a double-tap-down and wants to signal the
body-mode driver to enter MorphBall this frame. You write:

```rust
fn input_timer_phase(controls: &mut ControlFrame, runtime: &mut SandboxRuntime) {
    if runtime.register_down_tap() {
        controls.fast_fall_pressed = true;
    }
}
```

`body_mode_driver` is registered after `sandbox_update` in the
schedule. You verify that the in-frame fast-fall path inside
`sandbox_update` works (it shares the same `&mut ControlFrame`), but
the body-mode driver never sees the flag.

### Question

Why doesn't the body-mode driver see the flag, and what's the right
shape for a one-frame "edge pending" signal that crosses Bevy systems?

### Expected answer

`sandbox_update` takes `controls: ResMut<ControlFrame>` and mutates it
in place, so any system that runs *later* in the same schedule and
reads `Res<ControlFrame>` will see the change — that's not the bug
here. The bug is:

- If `controls` was passed by value (a local copy) or rebuilt fresh
  each frame from the input pipeline before `body_mode_driver` runs
  (which is what `populate_control_frame_from_actions.before(sandbox_update)`
  does), the in-frame mutation gets clobbered or never reaches the
  driver.
- More importantly, `ControlFrame` is the *input* boundary — overlaying
  a derived "double-tap-down detected" edge onto an input field
  conflates two layers and is fragile to exactly this kind of
  rebuild.

The right shape is a separate "pending edge" field on a long-lived
state resource (e.g. `SandboxRuntime { double_tap_down_pending: bool, ... }`),
written by `input_timer_phase`, consumed by the body-mode driver via
`mem::take` (so a stale signal can't latch), and cleared in
`SandboxRuntime::reset` defensively. Add a regression test that sets
`fast_fall_pressed = true` on the resource directly and asserts the
driver does **not** enter MorphBall — that pins the routing and
prevents future refactors from silently re-coupling.

### Tag

`bevy-resource`, `game-input`, `cross-system-signal`.

---

## Q3 — Trace record/replay frame alignment (Level A)

**Source commit:** `31d45ca` (trace_replay: fix off-by-one alignment).

### Setup

Your sim is deterministic (fixed timestep, deterministic RNG). You add
a `--dump-trace` flag to the headless binary. The convention is:

> "Record state AFTER each step."

So the dump file is a JSON array `frames: [Frame; N]` where:

```rust
struct Frame {
    controls: ControlFrame,   // input applied during step i+1
    player_pos: Vec2,         // player position AFTER step i+1 ran
}
```

You now want to write a `trace_replay` binary that takes a dump file,
re-runs the same sim, and asserts that `live.player_pos == frame.player_pos`
for every step — a determinism guard suitable for a CI fixture test.

### Question

Write the replay loop. Be specific about which `frames[i].controls` is
applied on which step, and which `frames[i].player_pos` is compared
to which post-step state. What sub-pixel symptom appears if you get
this wrong?

### Expected answer

The dump records `(controls applied in step i+1, pos after step i+1)`
into `frames[i]`. So in the replay:

```rust
for i in 0..frames.len() {
    sim.set_controls(frames[i].controls);
    sim.step();
    assert_eq!(sim.player_pos(), frames[i].player_pos);
}
```

i.e. `frames[i].controls` drives the i-th step (0-indexed), and the
post-step position must equal `frames[i].player_pos`. **Do not** skip
the first frame and apply `frames[i].controls` on step `i+1` — that
pairs the controls of one frame with the post-state of a *different*
frame, and any non-zero velocity introduces sub-pixel drift that
accumulates across the trace. Verify with a 30-tick round trip and
require `max_dx == 0.0 && max_dy == 0.0` end-to-end before claiming
the determinism guard is real.

### Tag

`record-replay`, `deterministic-sim`, `off-by-one`.

---

## Q4 — Multi-source ControlFrame: axis exclusive, buttons OR-merge (Level A)

**Source commit:** `a991d7c` (mobile_input: merge touch + keyboard ControlFrame).

### Setup

The sandbox runs on desktop with keyboard and on Android with on-screen
touch joystick + buttons. Both populate the same `ControlFrame`
resource each frame. The author intent is:

- Keyboard and touch axes are **mutually exclusive** — whichever has
  active deflection wins; the inactive source's axis is ignored. (You
  don't want a stuck-at-zero touch stick fighting against held WASD.)
- Action buttons (Jump/Attack/Dash/Blink/Interact/Projectile/Reset/
  Start) are **independent** — any source firing the button counts.
  Holding Jump on the keyboard while dragging the touch stick should
  attack-from-keyboard and walk-from-touch in the same frame.

The first naive shape the touch fold takes is:
`*control_frame = touch_to_control_frame(&touch);` — a full replace.

### Question

Describe the merge rule axis-by-axis and field-by-field. What
deadzone behavior is needed to decide "active"? What test pins the
intent?

### Expected answer

```text
axis (move_x, move_y):
    if touch deflection magnitude > 0.05 (post-deadzone) -> use touch axis
    else                                                 -> keep keyboard axis
aim (aim_x, aim_y):
    same shape, with a slightly larger threshold (0.10) to ignore stick noise.
edge flags (down_pressed, up_pressed, jump_pressed, ...):
    OR-merge: result = keyboard_edge || touch_edge
held flags:    OR-merge.
released flags: OR-merge.
```

Activity gate at the writer level: the touch fold must also check
`touch_state_is_active(...)` (any deflection or any held button or
any edge flag) before writing **anything** — with no touch input,
leave `ControlFrame` alone so the keyboard pipeline's writes survive
the frame.

Pin the intent with: a "held A on keyboard while dragging touch
stick" test that asserts both `move_x` non-zero AND
`attack_pressed == true` after the merge. And a "no touch input"
test that asserts the touch fold doesn't zero out keyboard-set
fields.

### Why this is hard

The "replace whole frame" shape is the natural first draft because
each source builds a complete `ControlFrame`. The mutual-exclusion
intent on axes pulls toward "winner takes the frame", and only the
buttons-are-independent intent forces the per-field merge.

### Tag

`game-input`, `multi-source-input`, `mobile-touch`.

---

## Q5 — `bevy_ecs_ldtk` IntGrid renders by default (Level B, repurpose existing lesson)

**Source commit:** `ded1dc2` (already in `docs/lessons_learned.md`).

This is already a great lesson — promote to a Level B benchmark.

### Setup (post-symptom)

You're using `bevy_ecs_ldtk` and an LDtk IntGrid layer named
"Collision" as a *data* layer (your own composer reads cells and
spawns gameplay blocks at centered Bevy coordinates). You spawn a
single `LdtkWorldBundle` at the default transform.

In-game, gameplay blocks render at the right positions, but you also
see ~1000 small extra colored sprites repeating horizontally — the
IntGrid value-def colors (gray for Solid, light blue for OneWay,
purple for BlinkSoft). They appear offset by ~half-room-width on x,
in raw LDtk world-pixel space (top-left origin, +y down) instead of
your `world_to_bevy` centered frame. Entities (NPCs, doors, loading
zones) are NOT duplicated.

### Question

Why are duplicates rendering, and what's the minimal `LdtkSettings`
change that suppresses the extras while still letting your IntGrid
indexer read cell data?

### Expected answer

When an IntGrid layer has no tileset configured, the default
`IntGridRendering::Colorful` mode emits a colored tile sprite per
non-zero cell directly from the plugin, parented to the
`LdtkWorldBundle` root which is at world-pixel space (top-left, +y
down). That conflicts with the project's centered Bevy frame.

Fix:

```rust
.insert_resource(LdtkSettings {
    level_background: LevelBackground::Nonexistent,
    int_grid_rendering: IntGridRendering::Invisible,
    ..default()
})
```

`Invisible` still spawns the `IntGridCell` components your composer
indexes from, so the data path keeps working. Entities are unaffected
because your project's `AmbitionLdtkMarkerBundle` deliberately omits
`Sprite`.

### Why this is hard

The diagnostic isn't visible from your code's logs — the extra
sprites aren't in *your* `world.blocks`. You have to read the
plugin's source (`bevy_ecs_ldtk-0.14.0/src/level.rs:557-595`) to
realize a default render path exists. "Data is correct, but
something is rendering it wrong" is the meta-symptom.

### Tag

`bevy-plugin-default`, `ldtk`, `coordinate-frame`.

---

## Q6 — LDtk's `__cWid` is `ceil(pxWid / gridSize)`, not `floor` (Level B, repurpose existing lesson)

**Source commit:** `56acf3b` (already in `docs/lessons_learned.md`).

### Setup

You're writing a Python migration that paints rectangular IntGrid
patches into an existing `.ldtk` file (so authors can keep editing
in LDtk afterward). The level is 1900×1024 px with `gridSize=16`. You
write:

```python
def cells_for_size(px: int) -> int:
    return px // GRID  # ❌
```

…and emit `__cWid` and a flat-row-major `intGridCsv` of length
`cWid * cHei`. The official JSON schema accepts your file. After the
user opens it in LDtk and saves (Ctrl+S), every column of cells is
"smeared" into a 1-cell-per-row staircase going left-down. The smear
is now baked in.

### Question

What did your migration get wrong about `__cWid`, and how can you
add a *one-line cross-check* that catches the same family of bugs
even when the JSON schema is happy?

### Expected answer

`__cWid` must be `ceil(pxWid / gridSize)`, not floor:
`(1900 + 15) // 16 == 119`, not `118`. LDtk reads `intGridCsv` with
*its* expected stride 119 even when your file says 118. Off-by-one
stride per row is exactly the diagonal staircase symptom.

```python
def cells_for_size(px: int) -> int:
    return (px + GRID - 1) // GRID  # ceil
```

The cross-check that pays for itself:

```python
assert __cWid * __cHei == len(intGridCsv)
```

…computed using LDtk's ceil rule, not your own. Better still, when
interoperating with an editor that owns the canonical file format,
load the file once with the editor (or a verified third-party
parser) and diff the on-disk values against your expectations
before handing off — the schema accepts whatever; the editor's
behavior is the ground truth.

### Tag

`ldtk`, `editor-interop`, `off-by-one`, `migration`.

---

## Q7 — Greedy row-major rect-merge produces vertical bars on diagonals (Level B, repurpose existing lesson)

**Source commit:** `8332349` (already in `docs/lessons_learned.md`).

### Setup

You wrote a function `emit_collision_blocks_from_intgrid(grid)` that
merges contiguous IntGrid cells into Bevy collision blocks. The
greedy shape is:

1. Scan row-major, find first unconsumed non-zero cell.
2. Extend right while the value matches.
3. Extend the resulting `[cx, x_end)` row-rect *down* while every
   column in that span matches in the next row.

On a horizontal floor (one wide row), it emits one wide block. On a
vertical wall (one column, many rows), it emits one tall block. On a
staircase pattern:

```
......#
.....##
....###
...####
..#####
```

…it emits a column of 1×N vertical bars instead of stair-stepped
1×1 blocks. Visually, the staircase inverts.

### Question

Without rewriting from scratch, what two-pass shape produces faithful
output on staircases AND keeps the rectangular-floor/wall coalescing?
What's the worst case on truly irregular shapes, and is that the
right outcome?

### Expected answer

Two-pass:

1. **Per-row horizontal coalesce.** Collapse adjacent same-value
   cells in each row into runs `(cx, x_end, value)`.
2. **Per-column vertical span-stack.** Adjacent rows whose runs match
   *exactly* (same `[cx, x_end)` and same value) stack into one
   block. Mismatched widths break the stack.

Wide horizontal floors stay one row. Vertical N-wide walls stack.
Staircases stay as the per-row 1×1 mosaic the editor shows — author
intent preserved. Worst case is per-cell on irregular shapes;
that's correct, since "merging across a stair-step" would lie about
the geometry.

### Why this was hard

Greedy rect-merge "extend-right then extend-down" is the textbook
shape and is *correct on rectangles*. The staircase failure mode is
specifically because staircases happen to match the down-extension
predicate column-by-column without matching it row-shape-by-row-shape.
The fix isn't a smarter greedy choice — it's a strict per-row run
shape on the second pass.

### Tag

`procgen`, `geometry`, `merge-pass`.

---

## Q8 — Bevy 0.18 buffered events are now `Message` (Level C, micro)

**Source commit:** `c49c1e5` (slice 1 of events refactor).

This is in the user's `feedback_bevy_0_18_message_api` memory but isn't a
benchmark question yet. It's a clean Level C.

### Setup

You're migrating a Bevy 0.17 plugin to 0.18 and you reach for
`#[derive(Event)]` on a struct, plus an `EventReader<Foo>` system
parameter. Both compile-fail.

### Question

Map every name change for the *buffered* (queue-style) events API,
and explain what the legacy `Event` derive was repurposed for.

### Expected answer

```text
Event           ->  Message            (#[derive(Message)])
EventReader     ->  MessageReader
EventWriter     ->  MessageWriter
Events<T>       ->  Messages<T>
add_event::<T>  ->  add_message::<T>
```

In 0.18, the `Event` trait is now reserved for the *observer-style*
one-shot event API (single dispatch, no buffering). Buffered queues
are `Message`s. Keep them straight: a system that reads `Foo` events
across systems uses `MessageReader<Foo>` against a `Messages<Foo>`
resource registered with `add_message::<Foo>()`. A trigger-style
`Event` is observed via `world.observe(...)` and isn't queued.

### Tag

`bevy-0.18`, `api-rename`.

---

## Q9 — Generic SfxMessage::Play vs typed enum variants (Level A, design)

**Source commits:** `32f641d`, `6f07ad4`, `8ca3f1d`, `53c2e45`.

### Setup

You're building an SFX system. There's a `SfxMessage` enum used as the
sim/presentation seam. Today it has:

```rust
#[derive(Message, Clone, Copy, Debug)]
pub enum SfxMessage {
    Jump { pos: Vec2 },
    DoubleJump { pos: Vec2 },
    Dash { pos: Vec2 },
    Blink { pos: Vec2, precision: bool },
    Pogo { pos: Vec2 },
    // ... etc, ~10 variants
}
```

A new feature wants 50+ wired SFX (chest open, coin pickup, switch
toggle, hazard contacts, UI accept/back/confirm, footsteps per
surface, save-point idle loop, ...). Each variant is just
`{ id_marker, pos: Vec2 }`. The team is debating whether to keep
adding typed variants or introduce a generic
`Play { id: SfxId, pos: Vec2 }` variant alongside.

### Question

What's the recommended split between "typed variant" and "generic
`Play` variant", and what cache shape supports the generic variant
without exploding decode cost on first play?

### Expected answer

- **Keep typed variants** for cues with bespoke per-cue logic on the
  consumer side (e.g. precision-vs-default Blink, Dash with feel-tuned
  panning, anything where presentation reads variant-specific
  payload). These are rare.
- **New everyday SFX** (chest open, coin, hazard contact, UI nav,
  footsteps, ...) prefer the generic `Play { id: SfxId, pos: Vec2 }`
  variant. Adds a bank entry + an emitter call site; no enum churn,
  no presentation code change beyond a single dispatch table.
- Cache shape: `SfxBankHandleCache` resource keyed on `SfxId`, lazy
  on first play (`StaticSoundData::from_cursor` on the bank entry's
  bytes). Misses log once and fall through to silence — never panic
  on a missing id. Subsequent plays are O(1) HashMap lookups, no
  decode.

A common mistake is to add one `FeatureEvents.<thing>_played: Vec<...>`
typed-vec-per-class on the sim side. That works for events with
multiple consumers (VFX + persistence + quests + audio), but is
wasteful for SFX-only sites. Add `FeatureEvents.sfx_plays:
Vec<(SfxId, Vec2)>` instead, with a `play_sfx(id, pos)` helper —
sites that only want audio enqueue directly; consumers that need the
typed event keep their dedicated vec; `handle_feature_events` drains
both channels.

### Why this is hard

The textbook ECS pattern is "one typed message per gameplay concept,
strongly typed". For 50 SFX, that pattern produces 50 enum variants
and 50 emitter call sites the presentation must individually
dispatch — pure boilerplate, with no benefit because every variant's
payload is identical. The generic variant trades a compile-time
exhaustiveness check (which the typed enum gave you) for an
order-of-magnitude reduction in the per-feature wiring cost. The
right answer is a hybrid: keep the typed variants exactly where they
earn their keep, generic-`Play` everything else.

### Tag

`bevy-resource`, `event-design`, `audio`, `code-shape-tradeoff`.

---

## Lower-priority Qs to consider later

- **Activity-gated writer pattern** (general distillation of Q1+Q4):
  any frame-rebuilt resource with multiple writers must gate each
  writer on its own activity, never unconditionally write.
- **`process_attack` -> `Vec` collector vs inline play_sound** (slice 1
  of events refactor): when does a function take a `&mut Vec<Foo>`
  collector parameter, and when does it take a `MessageWriter<Foo>`
  directly? Answer: helper-internal sim systems take collectors so
  they remain headless-runnable; presentation-side subscribers take
  the `MessageWriter`/`MessageReader` directly.
- **Atomic dump-filename sequence (`AtomicU64` Relaxed)** — why
  embedding unix-seconds + nanoseconds is insufficient when two
  dumps can fire in the same nanosecond, and why `Ordering::Relaxed`
  is the correct atomic ordering for "this is just for filename
  uniqueness" (no happens-before required). This is a Level C Rust
  concurrency micro-Q.
- **`mem::take` for one-frame edge resources** — when consuming a
  pending edge from a `SandboxRuntime`-style resource, why
  `let pending = mem::take(&mut runtime.double_tap_down_pending)` is
  better than `if runtime.double_tap_down_pending { ... }` followed
  by a manual reset (avoids the latch-stale-edge class of bug).

These can be lifted into formal questions if/when the in-progress
refactor produces a similarly-shaped failure that motivates the
specific Q being asked.

---

## Strategy doc thoughts

`dev/benchmark-candidates/README.md` already exists and is solid. It
covers workflow, quality bar, prompt levels (A/B/C), and an example
distillation. What it's *missing*, given the user's stated goal of
potentially submitting to a NeurIPS dataset/benchmark track:

1. **Discoverability.** It's referenced from `docs/code_structure.md`
   line 41 (one line), and the auto-memory has nothing pointing at
   it. A future agent reading the user's onboarding text won't know
   the dataset workflow exists. Two cheap fixes: add a memory entry
   pointing at `dev/benchmark-candidates/README.md`, and add a
   one-line reference to it from `docs/lessons_learned.md` so the
   two systems cross-reference each other (lesson = >1hr-bug-journal,
   benchmark candidate = distilled hard question).

2. **Q-ID convention.** Right now the existing entry is dated
   `2026-05-09:` as the heading. As the corpus grows, a stable ID
   (e.g. `Q001-rust-module-facade-reexports`) makes citation across
   docs easier and survives renaming. Worth adding a "naming
   convention" subsection.

3. **Per-question metadata block.** For dataset-track use, each Q
   wants a small frontmatter:
   ```yaml
   id: Q001
   level: A           # A | B | C
   tags: [rust-module-refactor, rust-visibility]
   source_commit: f355e3d
   verification_command: cargo check -p ambition_engine
   ```
   …so an evaluation harness can filter by tag, target the right
   crate for `cargo check`, and audit each Q against the source
   commit.

4. **Failure-mode tag.** Add a tag for the *bug class* (not just the
   surface technology). Examples observed in this repo:
   `edge-vs-held-state`, `record-replay-alignment`,
   `local-copy-vs-resource`, `multi-source-merge`,
   `editor-interop-stride-mismatch`, `plugin-default-path`. These
   cluster better than tech tags when filtering "what kinds of bugs
   does the model get wrong".

5. **"Pre-error context" tightening.** The README mentions
   reconstructing pre-error context. In practice, the recurring
   trap is that the *commit message* of the fix often gives away
   the answer. The Q-author has to deliberately strip the post-hoc
   knowledge — easy to forget. Add a checklist line: "the prompt
   contains nothing the agent couldn't have known at the moment of
   the mistake."

6. **NeurIPS framing (optional).** If the user wants the corpus to
   be submission-ready, a top-level
   `dev/benchmark-candidates/dataset_card.md` describing collection
   methodology, license, expected use, and "who is the population of
   bug-makers we're sampling from" (Claude agents during real
   refactor work in this repo) would be the missing piece. Probably
   not urgent today but worth deferring as a Proposed item.

---

## Apply order when the other agent finishes

Suggested order, no opinion on whether all of these get applied:

1. Read the other agent's resulting compile errors / test failures.
   *Those* are the freshest benchmark candidates — pre-error context
   is intact and the failure shape is real. Capture them first.
2. Cross off any of Q1–Q9 above whose lesson the other agent already
   landed (e.g. if they hit COYOTE_TIME class on a new module,
   that's a duplicate of the existing entry).
3. For lessons (`tmp-lessons-learned.md`), apply only the entries
   whose source commit's diagnosis genuinely took >1 hour. The
   ControlFrame edge-fields entry is the strongest candidate; the
   `Bevy 0.18 Message` rename is a coin flip (it's a well-known
   migration; the value is the consolidated cross-reference, which
   the auto-memory `feedback_bevy_0_18_message_api.md` already does).
4. For Qs in `tmp-qa-ideas.md`, apply the Level A pre-error-operation
   ones first (Q1, Q2, Q3, Q4) — those are the highest-value
   benchmark candidates because they test planning, not error
   repair.
5. Defer Q5/Q6/Q7 — they're repackagings of existing lessons. Maybe
   worth doing as a batch later when promoting lessons → benchmark
   candidates becomes a deliberate pass.
