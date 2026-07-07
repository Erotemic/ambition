# Compositional benchmark candidates

Single-issue questions like the ones in `rust-questions.md` test whether
an agent can preserve **one** invariant during a refactor. Real
maintenance work usually puts several invariants in flight at once. This
file collects benchmark prompts that *compose* multiple invariants into
one task, with notes on what additional capability the composition
tests beyond the sum of its parts.

The point of composition is not to make the questions harder by
piling on. It is to test capabilities the single-issue questions
cannot reach:

- **Enumeration.** Can the agent list the categories of mistakes it
  needs to guard against, *before* being told what to look for?
- **Synthesis.** Can the agent hold several invariants in working
  memory while drafting one patch?
- **Interference detection.** Can the agent recognise when one
  invariant fights another (e.g. the cleanest re-export shape requires
  exposing a helper the sibling-visibility lesson says to keep
  `pub(super)`)?
- **Anticipation.** Given the agent has fixed one class of mistake,
  can it predict the *next* class of mistake the same patch is likely
  to introduce?

A composed question is worth keeping when its score on a model is
*not* fully predicted by that model's score on the component
questions. If a model that aces every single-issue question still
fails the composition, the composition is measuring something real.
If models tend to score the composition exactly equal to the
worst-component score, the composition is mostly a stand-in for that
component and adds little.

---

## C-001 (composition of Q-001…Q-005): the entire 4-module split task

### Source

The 18a56c2 commit that split `audio.rs`, `music.rs`, `input.rs`, and
`trace.rs` simultaneously into facade-backed children. All five
single-issue questions in `rust-questions.md` (re-exports, attribute
adjacency, sibling visibility, `include_str!` paths, test-only
derives) were triggered by this *one* commit. The component questions
were extracted post-hoc; the composition is the operation as it
actually happened.

### Setup

You are refactoring a Bevy game crate. The crate has four large
single-file modules:

```text
crates/ambition_actors/src/audio.rs   (~1226 lines)
crates/ambition_actors/src/music.rs   (~1673 lines)
crates/ambition_actors/src/input.rs   (~1061 lines)
crates/ambition_actors/src/trace.rs   (~1799 lines)
```

The crate root re-exports specific items from each:

```rust
// lib.rs (excerpt)
pub mod audio;
pub mod input;
pub mod music;
pub mod trace;

pub use audio::{SfxMessage, AudioLibrary, MusicChannel /* ... */};
pub use input::{ControlFrame, MenuControlFrame, SandboxAction /* ... */};
// (etc.)
```

Each large file contains a mix of:

- Public types with `#[derive(Resource)]`, `#[derive(Component)]`,
  `#[derive(Serialize)]`, etc.
- Public functions with rustdoc comments and `#[cfg(feature = "...")]`
  attributes.
- Private helper functions called by other items in the same file.
- Inline tests that load checked-in game asset fixtures with
  `include_str!("../assets/...")`.
- Small marker enums and structs with no derive on `PartialEq`/`Eq`
  yet.

You want to split each into a facade plus 3-5 private children
organised by gameplay domain (e.g. `audio/render.rs`, `audio/runtime.rs`,
`audio/tests.rs`).

### Question

Plan the refactor as one PR. **Before** writing any code, list the
categories of mistakes you will explicitly guard against, and for
each category name a static check or test that should catch it
pre-handoff. Then perform the split and run the checks.

You may not look at compiler errors before producing the
enumeration. The goal is the enumeration, not the patch.

### Expected answer

The enumeration should cover at least:

1. **Facade re-exports.** Every name the crate root re-exports from
   `pub use audio::{...}` (etc.) must remain visible at
   `crate::audio::Foo`. Compare the parent re-export list against the
   facade exports name-by-name. Static check: a small surface-audit
   script, plus `cargo check -p ambition_actors`.
2. **Attribute and rustdoc adjacency.** `#[derive(...)]` and `///`
   doc comments are item-adjacent — they bind to the *next* item in
   the source file. When extracting an item, the decoration must move
   with it, or `cargo fmt` reports `expected item after doc comment`.
   Static check: `cargo fmt --all`. Audit: every file ends with an
   item, not a doc comment; every `#[derive(...)]` is followed by a
   struct/enum/union.
3. **Sibling-module visibility.** A private `fn foo()` in
   `audio/render.rs` is visible only inside `render.rs`; siblings
   like `audio/runtime.rs` cannot call it. Demote helpers used by
   siblings to `pub(super)` and add explicit
   `use super::render::foo;` in callers — do not expose them publicly
   unless the helper truly needs to be part of the crate's public
   API. Static check: `cargo check -p ambition_actors`.
4. **`include_str!` / `include_bytes!` / `#[path]` resolution.**
   These macros resolve relative to the source file containing the
   macro, not the crate root or the old module path. Tests moved one
   directory deeper need `../../assets/...` instead of `../assets/...`,
   or should switch to `concat!(env!("CARGO_MANIFEST_DIR"), ...)`.
   Static check: `cargo test -p ambition_actors --lib` (the bug may
   live under `#[cfg(test)]` and pass `cargo check`).
5. **Test-only trait derives.** New regression tests using
   `assert_eq!` on small marker enums (e.g.
   `Option<TouchActionButton>`) require `PartialEq` — adding the
   test forces a derive on the production enum, or the test must be
   rewritten as `.is_none()`/`matches!`. Static check:
   `cargo test --lib`.

The pre-handoff check pipeline:

```bash
cargo fmt --all
cargo check -p ambition_actors
cargo test  -p ambition_actors --lib
```

(plus the project's surface-audit script if one exists.)

### What this composition tests beyond the sum of parts

The single-issue questions tell the agent *what* invariant exists and
ask whether the agent can apply it. This composed question demands
the *enumeration* itself — the agent must recall the categories of
mistakes from the shape of the task, not from the prompt. A model
that has memorised individual answers but not internalised the
"things that go wrong when splitting a Rust facade" shape will fail
to enumerate, even if it can answer each component when the category
is named.

A weaker variant (Level B for this composition): give the agent the
list of categories and ask it to perform the split. That tests
synthesis without enumeration.

---

## C-002 (composition of input-edge + multi-source merge + activity gate): adding a third ControlFrame source

### Source

Three independent commits in mid-2026-05 together produced this
composition:

- `ebe3686` — explicit edge fields on `AgentAction`.
- `42f3545` — explicit edge fields on `TouchInputState`.
- `a991d7c` — keyboard/touch ControlFrame merge: axes exclusive,
  buttons OR-merge.
- `a63c258` — touch fold gates on `touch_state_is_active` so an
  inactive source doesn't stomp the active one.

These four commits collectively define the contract for "adding a
third writer to `ControlFrame`."

### Setup

A Bevy game has a per-frame `ControlFrame` resource consumed by
gameplay simulation. Two writers exist today:

- the desktop input pipeline (Leafwing `actions.just_pressed(...)` →
  `ControlFrame`), and
- the on-screen touch joystick + buttons (mobile builds), folded into
  `ControlFrame` via a `fold_to_control_frame` system that runs
  `.after(populate_control_frame_from_actions)`.

You're now adding a *third* writer: an `AgentAction` source for an RL
agent that wants to drive the same simulation through Bevy without
ripping out the human-input plumbing. The agent emits one
`AgentAction { move_x, move_y, jump, attack, ... }` per simulation
step.

### Question

Specify the contract the new writer must obey to coexist with the
existing two. Cover at least: (a) how edge fields are populated;
(b) how the new writer's axis interacts with the keyboard/touch axes
that may also be active in the same frame; (c) how button presses
combine across all three sources; (d) what the writer does when the
agent has nothing to say this frame; (e) what test pattern catches
the most likely classes of failure.

### Expected answer

(a) **Edge fields cannot be derived from the held axis.**
`AgentAction` must gain explicit `up_pressed`, `down_pressed`,
`jump_pressed`, etc. (default `false`, set `true` only on the frame
the agent wants the edge), or the writer must keep a `Local<f32>`
history of the agent's `move_y` and emit edges from threshold
crossings. Otherwise `down_pressed = move_y > 0.5` on every held-down
frame, `register_down_tap` fires repeatedly, and the body-mode
driver oscillates ~30 Hz. (See Q "Don't auto-derive a per-frame edge
flag from a held axis.")

(b) **Axes are mutually exclusive across sources.** Three writers
each producing a non-zero axis cannot all "win." Pick one rule and
apply it consistently — the existing convention is "whichever
source has post-deadzone magnitude > some threshold wins; if multiple
qualify, later writers in the schedule win." Document the rule next
to the merge code.

(c) **Edge and held button flags OR-merge across sources.** Holding
Jump on the keyboard while the agent independently emits
`AgentAction { attack: true }` should walk-from-keyboard,
attack-from-agent in the same frame:

```rust
frame.jump_pressed   = a || b || c;
frame.attack_pressed = a || b || c;
// (etc., for every edge and held flag)
```

(d) **The writer must gate on its own activity.** With no agent
input this frame, the new writer must leave `ControlFrame` alone —
not zero out keyboard- or touch-derived state. The activity gate is
"any axis above deadzone OR any held button OR any edge flag set."
Without the gate, an idle RL agent stomps the human input every
frame and produces the same ~30 Hz oscillation as the recurring
edge-derivation bug.

(e) **Test pattern.** A "held axis for 30 frames" test on each
writer (per the input-edge Q's regression test). A "two writers
active simultaneously" test that asserts both contributions reach
`ControlFrame`. A "this writer idle, another writer active" test
that asserts the active writer's state survives the frame.

### What this composition tests beyond the sum of parts

The single-issue input questions test each invariant in isolation.
Composing them at the "add a new writer" task forces the agent to
notice that all four invariants are properties of *every* writer,
not of one specific writer — i.e. that there is a generic contract
("ControlFrame writer discipline") which any new code must respect,
not a per-writer cleanup that's already done because the existing
writers happen to follow it.

The interference-detection part: rule (b) (axes exclusive) and rule
(c) (buttons OR-merge) look contradictory at first read. Enumerating
*both* rules without conflating them is the test.

---

## C-003 (composition of split + Bevy version migration): module refactor during an API rename

### Source

The `c49c1e5`…`81900dd` slice of the events refactor migrated audio
and a chunk of features from the pre-Bevy-0.18 `Event`/`EventReader`
API to the post-0.18 `Message`/`MessageReader` API while *also*
extracting an audio-events seam. Two independent invariants —
"preserve crate facade" and "rename per Bevy 0.18" — moved through
the same files at the same time.

### Setup

You are migrating a Bevy game crate from Bevy 0.17 to Bevy 0.18 and,
in the same PR, splitting a 1500-line `main.rs` into a thin shim plus
a new `app.rs` module that owns the gameplay schedule.

The crate uses several historical `#[derive(Event)]` types
(`SfxEvent`, `VfxEvent`, `DebrisBurstEvent`, ...) read by
`EventReader<...>` system parameters and queued via `EventWriter`. In
Bevy 0.18:

- The buffered (queue-style) API was renamed:
  `Event` → `Message`, `EventReader` → `MessageReader`,
  `EventWriter` → `MessageWriter`, `Events<T>` → `Messages<T>`,
  `add_event::<T>` → `add_message::<T>`.
- The `Event` trait still exists but is now reserved for the
  *observer-style* one-shot API (`commands.trigger(...)`).

The `cargo check` after the migration reports unresolved imports
("`EventReader` not found"), missing items ("no `add_event` on `App`"),
and "is not a Bundle" errors in unrelated UI code that turns out to
be `BorderRadius` having moved into `Node::border_radius` in 0.18 —
not all of the compiler's complaints are about your migration.

### Question

How do you order the work so the two concerns don't tangle in the
diff, and how do you tell which compile error is "from the rename"
versus "from the split" versus "from an unrelated 0.18 change you
hadn't anticipated"? What does the PR look like — one commit, two
commits, more?

### Expected answer

Order:

1. **Do the rename first, in its own commit.** Bevy 0.18 buffered
   events: every `#[derive(Event)]` on a queued type becomes
   `#[derive(Message)]`; every `EventReader<T>` becomes
   `MessageReader<T>`; every `EventWriter<T>` becomes
   `MessageWriter<T>`; every `add_event::<T>()` becomes
   `add_message::<T>()`. Decide for each historical `Event` whether
   it was queue-style (rename to `Message`) or genuinely one-shot
   (keep as `Event`, switch consumers to observers). After this
   commit, `cargo check` reports a much smaller error set focused on
   actual semantic changes (e.g. `BorderRadius` moved into `Node`,
   reflection trait bounds tightened, etc.).
2. **Catch the unrelated 0.18 changes in a second commit.** Read the
   Bevy 0.18 changelog before searching for runtime errors. Examples
   you may hit beyond the buffered-events rename: `BorderRadius`
   moved into `Node::border_radius`; `Visibility` access counts as a
   mutable component read for `B0001`; some reflection trait bounds
   tightened.
3. **Split the module last.** Now that the crate compiles cleanly on
   0.18, the split is a pure structural refactor. Apply the
   single-issue "facade re-exports" / "attribute adjacency" /
   "sibling visibility" / "include_str!" / "test-only derives"
   discipline.

Reasoning:

- A combined diff makes it impossible to bisect a regression — you
  can't tell whether a runtime panic came from the rename, the
  split, or a third-party crate that needed updating.
- The renames are mechanical and large; doing them mixed with a
  module split makes the rename's blast radius look like a refactor.
- Compiler errors in step 1 confirm or refute your understanding of
  the 0.18 rename (you should see "no `EventReader` in scope"
  resolving cleanly to "no `MessageReader` in scope" once renamed —
  if some other error remains, it's a real semantic change worth
  reading the changelog for).

### What this composition tests beyond the sum of parts

This composition tests **error attribution** — when `cargo check`
spits out 80 errors after a multi-axis change, can the agent
correctly diagnose which axis owns each error? An agent that has
internalised both the API rename and the module-split discipline can
still flounder if it tries to fix everything in one commit and
mis-attributes errors across the two concerns.

The Level B variant — give the agent the compiler output of a
combined attempt and ask "which errors come from which change?" —
isolates the attribution capability without asking for the full
plan.

---

## Patterns for inventing new compositions

When a real Ambition refactor touches multiple invariants, ask:

- **Which single-issue questions does this commit's failure mode
  cover?** (Tag the commit with each Q-id.)
- **Did the agent need to enumerate the invariants without being
  told?** If yes, the task is composition-worthy; capture the prompt
  shape that *doesn't* mention the categories.
- **Is there a natural ordering that decouples the concerns?** If
  yes, the composition can test that ordering ability ("plan the PR
  shape") rather than the synthesis ability ("write all of it at
  once").
- **Could the composition produce a Level A prompt? A Level B?** Many
  compositions naturally have both: Level A asks for the plan; Level
  B asks the agent to read a real combined-error output and
  attribute it.

When a composition lands, add it here and cross-link the component
single-issue Qs by name. The cross-references make it possible to
score a model on a single-issue Q, then independently on the
composition that contains it, and look for compositional gaps.

---

## Related: the composition of *journal* lessons

Some lessons in `dev/journals/lessons_learned.md` repeat across
commits — the same class of mistake shipped multiple times because
the lesson didn't propagate well (e.g. "touch buttons should name
actions, not keyboard keys" appears in three separate commits;
"`down_pressed` cannot be derived from the held axis" appears in
three separate writers). A high-value composition for those is:

> "Here are three commits from the project history where the same
> class of bug shipped under different surface conditions. Read them
> and produce a single distilled rule that, applied at code-review
> time, would have rejected all three. What test or lint would
> mechanise the rule?"

This tests whether the agent can *cluster failures* — a capability
that's easy for humans to take for granted but that LLMs can fail
on when each occurrence has different superficial details.
