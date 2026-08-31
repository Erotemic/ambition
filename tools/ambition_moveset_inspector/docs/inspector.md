# Moveset balance inspector

Frame data, hitbox geometry and cross-roster comparison for Ambition's smash
cast, without loading the game.

## Launch

```bash
tools/ambition_moveset_inspector/serve_inspector.sh --open
```

Serves the UI at <http://127.0.0.1:8777>. Pass `--no-export` to look at the
bundle already on disk.

⛔⛔ NOTHING HERE INVOKES CARGO. The wrapper used to `cargo run` the exporter on
every start, which takes the cargo build LOCK — so opening the inspector could
block, or be blocked by, an agent building on another branch. It runs binaries
that already exist, and prints the build command and each binary's provenance
every time, whether or not anything is missing:

```text
[inspector] this tool never builds; refresh a binary yourself with:
[inspector]   cargo build -p ambition_app_tools --bin moveset_export --bin moveset_takes --bin moveset_render
[inspector] moveset_export  <target>/debug/moveset_export  (built 2026-08-27 17:04, 2h old)
[inspector] moveset_takes   NOT BUILT — there will be no recorded takes to look at
[inspector]                   cargo build -p ambition_app_tools --bin moveset_takes
```

⛔ AND THE COMMAND IS COPY-PASTEABLE, which it was not: it printed a brace
expansion, and `--bin {a,b,c}` expands to `--bin a b c` while cargo takes ONE
value per `--bin` — *"error: unexpected argument 'moveset_takes' found"*. A
suggested command that does not run costs the reader a round trip to discover.

⭐ THE BUILD COMMAND IS NOT ONLY A FAILURE MESSAGE. Somebody refreshing a binary
that already exists needs the same line as somebody who has none, and the age is
what tells them whether they should.

The renderer's own path and build time also ride back with its frames, so the
Engine Takes label reads `sprites: rendered by the engine (moveset_render built
2026-08-27 18:35)` and its tooltip is the binary's full path.

Missing binaries are never fatal. No exporter means the bundle already on disk is
served; no renderer means the CPU fallback; no `moveset_takes` means there are no
takes to look at yet.

⚠ THE COST OF NEVER BUILDING is that a binary can be older than the source it was
built from. The exporter's path and build time are printed when it runs, so a
stale answer is at least a visible one.

Binaries are looked for in `${CARGO_TARGET_DIR:-<repo>/target}/{release,debug}`,
the convention `scripts/profile_desktop.sh` already uses.

## What it reads

`moveset_export` boots `build_visible_app` and reports what the **composed
host** resolves, not what an authoring file writes. Those differ whenever a
repertoire slot overwrites a move (`UpSpecial::into_spec` sets `gates.recovery`)
or a provider composes one fighter out of another's table — and the composed
answer is the only one a balance decision can be made against.

The bundle is generated, so it is gitignored. `check_bundle_contract.mjs`
asserts every field the UI reads is present; run it after changing either side.

## The five views

- **Roster** — every fighter with a real moveset, filterable, grid-only by default.
- **Fighter** — the moveset as a sortable frame-data table, plus per-move
  timeline, hitbox diagram and the body's own numbers.
- **Compare** — one slot across the whole roster. Cells more than two median
  absolute deviations from the roster median for that slot are flagged high or
  low. This is the view that answers *"is this move out of line"*.
- **Status** — what this server has, where it looked for it, how old each piece
  is, whether the recorded takes carry the fields this build DRAWS, and the build
  command for every binary whether or not it is present. Open this FIRST when
  something is missing: it answers "is it trying", "does it know
  where it is" and "what do I run" without reading a terminal.
- **Engine takes** — recorded playback of the real simulation **in the real art**:
  the fighter's own sprite, animated by the move that is playing, with its live
  hitboxes over the top and anything its move spawned drawn beside it. Recorded
  by `moveset_takes`; absent until it has been run. The `Art` button turns the
  sprites off when a volume is easier to read on its own.

## How the art gets there

`moveset_export` emits an atlas table (`sheets`) beside the fighters: frame size,
page images, the body rectangle, the feet pixel, and every row's per-frame
sub-rects with trim offsets. The server exposes the engine's own sprite directory
read-only at `/art/`, so nothing is copied and the page always shows what the
build would draw.

A take records `[sheet_key, row_index, holds_last_frame]` per body per tick. The
FRAME within the row is derived in the viewer by counting consecutive ticks on
that row — one clock (the sim tick the take was recorded at) instead of two that
can drift.

## What is REAL here and what is RECONSTRUCTED

Worth stating plainly, because "recorded from the engine" is easy to over-claim.

REAL — observed from a running composed host:

- the simulation itself: real control frames, physics, move resolution, body
  positions, live hitboxes, summons;
- **all combat geometry**, from `CombatGeometryView`: live strike volumes in
  world space, effective hurtboxes, and which of the runtime's three damageable
  states produced them;
- **the move clock**: the authored window the move is inside, elapsed of
  duration, the facing the move committed to, whether it has landed;
- **who is who**: the scenario role of every body, strike and shot;
- WHICH MOVE is playing on each tick (`MovePlayback`);
- the art: the engine's own sheets, served from its own asset directory;
- WHICH ROW that move draws from — resolved through the move's authored
  `ClipBinding` and `clip_slot`, the same function and the same fallback chain
  the renderer walks.

RECONSTRUCTED — computed by the viewer, matching the engine's rules but not read
from it:

- **the frame cursor.** `CharacterAnimator` does not exist headless, so the
  viewer re-derives the frame from ticks-on-row. It follows the animator's rules
  (`duration_secs` is PER FRAME; a clip holds its last frame, a pose loops) but
  it is a reimplementation and can drift from the real one. See
  "Deleting the reimplementation" below for exactly what stands in the way.
- **⚠ every non-move pose.** The game picks from 56 semantic body states — walk,
  run, fall, land, crouch, shield, hitstun, tumble. This picks `jump` when
  airborne and `idle` otherwise. A fighter walking or in hitstun therefore shows
  IDLE, which is the largest fidelity gap in this view.
- **⚠ mirroring** uses `facing` alone. `authored_faces_left` is exported and not
  yet applied, so a left-drawn sheet may face the wrong way.

⛔ Two traps are already paid for here, and both cost a rebuild:

- **`CharacterAnimator` is the wrong source.** It is what the renderer uses, and
  the render layer only inserts it once a sprite ASSET has loaded — which never
  happens under `NoWindow`. Asking for it recorded 14446 bodies with art on
  exactly zero of them.
- **A summon wears no catalog character.** Joining the sheet on `WornCharacter`
  alone drew the pirate in full art and his shark as an empty box, which is the
  one pairing this view exists to show. `ActorConfig::sprite_character_id` is the
  fallback the renderer itself uses.

The viewer places a frame on the body's own centre (`feet_pixel.x`), not the
frame's. A sheet cell is sized by the widest pose and the art sits wherever the
crop left it — `projectile_polygon` is 17% of a 377px frame left of centre — so
centring the cell would reproduce, in the viewer, the exact defect the engine's
own sprite anchor had until 2026-08-27.

```bash
cargo run -p ambition_app_tools --bin moveset_takes -- --characters npc_pirate_admiral
```

It drives the nineteen repertoire presses through the real control frame into a
real seated match — a **fresh match per take**, because `afford_recovery` refuses
a recovery whose airtime already spent one and an instrument whose answer depends
on what ran before it is not measuring the thing it names.

Read the report line. `MISMATCH: drove <id> but the engine played {...}` means
the press did not reach the move it is bound to, and that is a finding rather
than noise: it is how D252 (the back air is unreachable for the whole cast) and
D253 (`player_robot_v3` cannot throw any of its five specials) were both found.
The per-frame `gesture` field says what the engine understood the input to be,
which is usually the reason.

## Feedback

Every fighter and every move takes a 1–10 score, issue tags and free notes.
Reviews are durable authoring data under `reviews/<character>/<move>.yaml`, keyed
by the stable id rather than by the numbers — the point is that *"the pirate's
forward smash is too strong"* survives the next tuning pass. The numbers as they
stood when the note was written are recorded beside it as `snapshot`.

For an agent asked to *address the feedback on the pointed polygon*:

```bash
python3 -m ambition_moveset_inspector.server --report
```

prints every standing note that asks for a change (scored below 6, or tagged).

## Deleting the reimplementation

The frame cursor is the one part of this tool that duplicates engine logic, and a
duplicate drifts. `moveset_takes` prints a `[presentation]` census on every run so
the gap stays visible instead of being rediscovered; measured 2026-08-27 it reads
`PlayerVisual=0 CharacterAnimator=0 BodyPoseView=0`.

TWO blockers, and neither is "headless cannot animate":

1. **`BodyPoseView` is unavailable to a smash fighter in ANY mode.**
   `rebuild_body_pose_views` is filtered `With<PlayerVisual>`, and `PlayerVisual`
   is granted in exactly one production place — `session/setup.rs`, to the
   exploration player's avatar. A seated `MatchSeat` fighter never carries it,
   windowed or not. The pose read-model is simply not built for match fighters,
   so this was never a tooling limitation.
2. **`CharacterAnimator` needs a render app.** It is built by the render layer
   from a loaded `CharacterSpriteAsset`, and `NoWindow` sets `backends: None`,
   which omits the render app by design.

The route out is `OffscreenGpu` — it HAS a render app, and `capture_scene` already
runs it headlessly on this machine. Switching this tool's mode alone is NOT
enough: measured, it panics inside `bevy_pbr`'s skin batching, because
`capture_scene` boots through `build_visible_app_with` plus its own camera and
render-target setup. Giving `moveset_takes` that same boot is the bounded piece of
work that would let it read `CharacterAnimator::frame` directly and delete the
derivation here.

## The engine render (GPU, on demand)

The derived frame cursor is the FALLBACK now, not the only answer.

ONE binary: **`moveset_render`**. Nothing else in the tool needs a GPU.

⛔⛔ **THIS SECTION DESCRIBED `capture_scene` UNTIL 2026-08-29, AND THAT TOOL
PHOTOGRAPHS A FIGHTER STANDING.** The `/api/render` route took a character alone
and cached one picture of somebody doing nothing, so every move of a fighter
shared it. `moveset_render` performs the requested move and captures exact ticks;
the section below this one owns the details, and the two must not describe two
different architectures again. ⚠ a doc that names a superseded binary sends the
next reader to build the wrong thing.

```bash
# you build it; the tool never will
cargo build -p ambition_app_tools --bin moveset_render

# and it is useful by hand
target/debug/moveset_render --character projectile_polygon --verb attack \
    --out /tmp/a --frames 24 --stride 2
```

The server looks for it in `${CARGO_TARGET_DIR:-<repo>/target}/{release,debug}/`
— the same convention `scripts/profile_desktop.sh` uses — and when it cannot find
it, the `503` names every path it tried.

⚠ For completeness, the two binaries the REST of the inspector needs, neither of
which wants a GPU: `moveset_export` (the bundle; `serve_inspector.sh` runs it for
you) and `moveset_takes` (the recorded takes; run it yourself for the fighters
you care about).

`--frames N` is how many pictures to take; `--stride K` advances K simulation
ticks between them, and every PNG is named for the exact `SimTick` it was
captured on.

The inspector asks `/api/render?character=<id>&verb=<verb>` once per
character-and-verb per session and caches under `data/renders/<id>/`. Engine
Takes then draws the engine's own picture with the hitboxes over it, and the
label beside the scrubber reads `sprites: rendered by the engine`.

## Numbers, without reading 150 ticks of geometry

```bash
python3 scripts/moveset_report.py --takes tools/ambition_moveset_inspector/data/takes/takes.json \
    --character npc_pirate_admiral --verb attack --out /tmp/inspection
```

Writes `report.json` (the machine-readable authority) and `summary.md` (a short
causal read) from what the runtime published: authored window ticks, first live
volume, max reach from the body origin, attack extents, travel before and during
the active window, spawns, launch speed, and both kinds of contact claim.

⛔⛔ **`overlap_ticks` AND `contacts` ARE TWO DIFFERENT CLAIMS AND NEVER ONE.**
The first is this script measuring rectangles; the second is the runtime's own
hit-once memory. They disagree whenever the victim was intangible, on the same
team, shielded, or already struck by that strike — and a report that merged them
would be confidently wrong in exactly the cases somebody opens it to
investigate. Measured on the admiral's jab at 38px: **5 ticks of overlap, one
resolved contact**, because a jab hits once.

With `--out` it writes the whole bundle — `report.json` (the authority, carrying
provenance: the source recording, its timestamp, and all three schema versions),
`summary.md`, `trace.jsonl` (one line per tick, for the question the report did
not anticipate) and `filmstrip.svg`.

### Why a hit resolved the way it did

The chain above says WHAT changed. WHY — `ignored` / `blocked` / `armored` /
`wallet_shielded` / `damaged` — is the engine's own vocabulary and it travels on
`BodyHitResolved`, behind the default-off `causal` feature. Record with it and
the take carries the inspector's facts:

```bash
cargo run -p ambition_app_tools --bin moveset_takes --features causal -- \
    --characters npc_pirate_admiral --verbs attack --spacing 40
```

⛔⛔ **A DAMAGE DELTA CANNOT TELL `Blocked` FROM `Ignored`.** Both leave HP
unchanged, and so does a windbox that authored no damage. The report reads the
resolution or reports nothing; it never infers one — and a recording made
without the feature says so where the answer would have been, so an absence of
evidence cannot be read as a decision.

⚠ A seated fighter's causal subject is its SEAT (`seat:1`), not its `SimId`:
`body_subject` prefers `SubjectKey::Seat` for any body a participant drives.
Joining on the id alone finds nothing for exactly the two bodies an inspection
scenario is about.

Before and after, for a tuning change:

```bash
python3 scripts/moveset_report.py --takes after.json --against before.json \
    --character npc_pirate_admiral --verb attack
```

```text
startup        13 → 11  (-2)
max reach px   52 → 61  (+9)
first contact   6 → 4   (-2)
```

⛔ A diff whose two scenarios differ — a different target, a different behaviour —
says so at the top instead of presenting the mixture as a change in the move.

## A picture with no rasterizer at all

```bash
python3 scripts/render_take_diagnostic.py \
    --takes tools/ambition_moveset_inspector/data/takes/takes.json \
    --out /tmp/sheets --character npc_pirate_admiral
```

One SVG contact sheet per take: subject and target labelled in words, cyan
hurtboxes, strike volumes in their real shapes, projectiles, and the move/pose/
clip and authored window of each tick. **No WGPU, no sprite decode, no browser.**

⭐⭐ **THE CELLS ARE THE TICKS THAT MEAN SOMETHING**, not a stride: opening pose,
last startup, first live volume, first contact, max reach, spawns, last active,
end of recovery — each labelled with why it was chosen. ⛔ An even strip samples
the CLOCK rather than the move, and a jab is live for five of a hundred and fifty
ticks, so twelve evenly spaced frames usually miss every one of them and show a
fighter standing still. `--select even` restores the old stride for an old take.

⛔⛔ **IT SAYS ON ITS FACE THAT IT IS DERIVED.** An ENGINE RENDER is what the
production Bevy graph drew (`moveset_render`); a DIAGNOSTIC RENDER is derived
from a recorded take. This page is careful never to pass one off as the other,
and an exported file leaves the context that made that obvious — so the
distinction lives on the picture.

⭐ **SVG RATHER THAN PNG**, because geometry is what a take records. Rasterizing
would need the sheets decoded and a compositor, which is the work this avoids.

## The two panels show ONE fight

⛔⛔ **THE SCENARIO TRAVELS WITH THE RENDER REQUEST.** The engine render sits
beside the diagnostic canvas, and a render staged from across the stage next to a
take recorded at 40px is two different fights presented as one. `/api/render`
takes `target` and `spacing`, the browser sends the ones the TAKE was recorded
with, and the render cache is keyed by them — so a scenario change cannot be
served from another scenario's pictures.

They are still two executions with no shared origin, which is why they are
synchronised by `action_tick` (how far into the exercise a frame is) rather than
by absolute `sim_tick`.

## Why the engine render is or is not available, BEFORE anybody asks for one

`/api/status` carries `render_capability`, and the Status page shows it:

```text
vulkan loader     libvulkan.so.1
vulkan ICDs       lvp_icd.json, radeon_icd.json, …
software adapter  lvp_icd.json
offscreen capture likely
```

```bash
python3 scripts/render_capability_doctor.py          # or --json
```

⛔⛔ **IT EXISTS BECAUSE THE QUESTION WAS ANSWERED WRONG.** `/api/render` returns
its `503` only after composing the whole game, which is minutes late and blames
whatever the driver reported. A 2026-08-29 review read one such message,
concluded this machine had no Vulkan adapter, and recommended installing a
package that was **already installed** — on a machine where `moveset_render` then
produced real engine PNGs.

⭐ **LAVAPIPE ALONE IS ENOUGH, AND NO XVFB.** `VisibleRenderMode::OffscreenGpu`
creates no window and disables winit; it needs an adapter that can render to a
texture, not a physical GPU and not an X server.

⛔ **IT REPORTS, IT DOES NOT PROVE.** An ICD on disk is necessary and not
sufficient — a driver can still refuse — so the verdict is `likely`, never
`available`, and the tool says it created no adapter. An engine render
succeeding is the authoritative answer.

⛔ EVERY FAILURE IS A JSON ANSWER. No GPU, no built binary, a driver that will not
start — the route returns `503 {available: false, reason}` and the viewer falls
back to the derived sprites, saying `sprites: derived — engine render unavailable
(…)`. A view that silently swapped between the two would be one whose fidelity
nobody could trust.

⚠ The render is currently a whole-scene capture of a fighter standing in
`hall_of_characters`; it is not yet driven per MOVE. Driving `--press` from the
move's verb is the next step and is why the route already takes a frame count.

## Combat geometry: whose, where, and can it be hit

Every volume in this view comes from one place: `CombatGeometryView`, the read
model the production developer overlay draws. The recorder does not resolve a
volume, the browser does not, and the SVG exporter does not —
`ambition_sim_harness::combat_observation` serializes the view and every consumer
reads that.

**Both halves of the interaction are drawn.**

| drawn | colour | what it is |
|---|---|---|
| strike volume | red (subdued when it is not the subject's) | a live hitbox, in world space |
| hurtbox | cyan | where this body can actually be struck this frame |
| SUBJECT / TARGET | blue / amber, and the word | the scenario role, recorded not inferred |

An attack volume shown alone cannot answer the question people open this view
with. A strike passing through a fighter may be passing through a frame in which
that fighter is INTANGIBLE, or through a silhouette much narrower than the
sprite, and the picture is identical either way. So the recording carries the
runtime's three-way damageable rule and the view says which one it was:

- `published` — a publisher named volumes for this body;
- `body_fallback` — nothing published, so the coarse body box stands in;
- `intangible` — published and EMPTY, on purpose. Drawn as the word
  `INTANGIBLE`, because an empty list and a missing recording look the same.

⚠ `published` does not distinguish an authored silhouette from the default
publisher's coarse envelope: `refresh_body_damageable_volumes` publishes the body
box as a single volume when a character has no `ResolvedHurtboxes`, and the view
cannot see which of the two it received. Telling them apart needs that component,
which lives above the read model's crate.

### Roles, not seats

A take seats two fighters and may seat the SAME character twice, so neither the
character id nor a seat index names the thing being inspected. Every body,
strike and shot carries one of `subject`, `target`, `subject_owned`,
`target_owned`, `other`, and the take names its subject and target at the top.
`subject_owned: false` used to be the only answer available and covered the
target, the target's summon and a stage hazard alike.

### The target stands still by default

`--target-behavior passive` seats the opponent on the stand-still brain preset: a
real, seated, damageable fighter that makes no decisions. It is not a frozen
body and nothing mutates its components — a CPU seat naming no brain profile is
refused at preparation on purpose. `--target-behavior cpu` restores the live
duelist, which is a different measurement, and the take records which one it was
beside `opponent_output` so a zero there reads as the scenario rather than as
evidence.

```bash
cargo run -p ambition_app_tools --bin moveset_takes -- \
    --characters npc_pirate_admiral --target projectile_polygon
```

### Shapes

Strikes are drawn as the shape they ACTUALLY are — convex polygon, disc, rotated
box — not as the axis-aligned box around them.

⛔⛔ THE BOX AROUND AN ARC IS NOT THE ARC. `moveset_takes` originally recorded
`Hitbox::world_aabb`, which sits directly beside `world_volume` in the same impl.
Measured on this cast: 405 of 481 recorded hitboxes are convex, and their real
area is a median **55%** of the bounding box they were drawn as — the box
overstated the strike by a median 1.8x and up to **5.3x**. A reach diagram that
wrong is worse than none, because it is confidently wrong.

The AABB is still recorded beside the shape: it is the broad phase the engine
itself uses, a viewer can draw it knowing no geometry, and a take made before the
shape existed still renders. `check_bundle_contract.mjs` prints the shape census
and warns if a recording carries none.

## "I pressed Art and nothing happened"

⛔⛔ REBUILDING THE BINARIES DOES NOT RE-RECORD THE TAKES. A `takes.json` made
before a field existed keeps not having it forever, and the Art button then
toggles between "sprites" and "boxes" with no sprites to toggle to — which reads
as a dead control rather than as missing data.

```bash
cargo run -p ambition_app_tools --bin moveset_takes -- --characters <id>,<id>
```

The Status tab reports it directly — `14390/14445 bodies with art, 481/481
strikes with geometry` — and flags a recording that carries neither. The Engine
Takes label says it too, where the button is.

## Only one fighter in Engine Takes?

Not any more, and the reason it used to happen is worth keeping: the picker
listed the fighters found in `takes.json`, so a fighter existed in this view only
once somebody had recorded a bulk take for it.

**The roster now comes from the prepared bundle and the takes are a cache.**
Every prepared fighter is selectable immediately, every move it binds is
selectable, and what the cache does or does not hold is shown as status:

```text
PIRATE ADMIRAL · 19 takes      <- recorded
PROJECTILE POLYGON · not recorded
```

A move with no recording still opens: the diagnostic canvas says so plainly and
the engine-render panel beside it still photographs the move on demand, because
`/api/render` drives one character and one verb and needs no take at all.

Record one when you want the per-tick geometry:

```bash
cargo run -p ambition_app_tools --bin moveset_takes -- \
    --characters npc_pirate_admiral,projectile_polygon
```

The fighter picker follows the fighter you were reading in the Fighter view when
you switch tabs. Status names coverage too.

Or record the whole grid:

```bash
cargo run -p ambition_app_tools --bin moveset_takes -- --characters grid
```

⚠ MEASURED 2026-08-27: **~1m17 per character**, so the 21-fighter grid is about
**27 minutes**. (It was 7m08 until `settle` stopped serialising a whole frame to
read three booleans.) Every take settles a real match between presses and there
are 19 verbs. This is a background job, not a click — which is why the default
records one fighter and the take view shows coverage (`21 prepared · 2
recorded`) rather than pretending every fighter has evidence behind it.

## Checks

```bash
node tools/ambition_moveset_inspector/check_bundle_contract.mjs   # the data
node tools/ambition_moveset_inspector/check_draw_path.mjs         # the drawing
node tools/ambition_moveset_inspector/check_takes_discovery.mjs   # who is listed
```

⛔⛔ `node --check` SAYS A FILE PARSES, NOT THAT ITS IDENTIFIERS RESOLVE. A call
to `drawHitboxShape` once shipped with no such function in the file and
`node --check` passed; in the browser `drawTake` threw the instant a strike
appeared, which killed the playback timer. The Jab's first hitbox is on frame 3,
so Play ran for exactly three frames and stopped — with the error only in a
console nobody had open.

`check_draw_path.mjs` draws every frame of every recorded take against a stubbed
canvas and fails on the first exception. Run it after touching `app.js`.

`check_takes_discovery.mjs` drives the real discovery functions against
synthesised bundles and asserts the thing that was wrong for a long time: a
bundle of five fighters offers five with zero takes recorded, and two recordings
do not shrink the picker to two.

## The move renderer (`moveset_render`)

It draws the engine's own combat overlay over the real art by default
(`--combat-overlay off` turns it off), so one PNG carries the actual rendered
character, the actual target, the actual VFX **and** the actual runtime volumes —
from ONE execution, with no browser-side transform between two coordinate
systems. Nothing in the tool draws a box; the production
`draw_combat_geometry_view` does.

Beside every PNG the manifest carries that shot's `observation`: the same tick's
bodies, roles, hurtboxes, strike volumes and move clock, in the same schema the
recorder writes. It is sampled BEFORE the shutter, for the same reason `move` and
`grounded` are — the zero-duration pump loop keeps running `Update`, so anything
read after it describes a different moment than the picture.


The GPU path renders a fighter **performing a selected move**, one PNG per exact
simulation tick.

```bash
cargo build -p ambition_app_tools --bin moveset_render
target/debug/moveset_render --character npc_pirate_admiral --verb special_up \
    --out /tmp/upb --frames 24 --stride 2
```

`/api/render?character=<id>&verb=<verb>` drives it and caches on **character and
verb** — it used to take a character alone and photograph a fighter standing, so
every move of a fighter shared one cache entry of somebody doing nothing.

⛔⛔ IT DOES NOT REUSE `capture_scene`, and the reason is concrete: that tool calls
`App::run()`, so the RUNNER owns the loop and a driver cannot decide what a frame
costs. Its `--frames` cadence shows the cost — `request_capture` returns early
while a readback is pending but the app keeps updating, so shots are spaced by
`stride + however long the GPU took`.

⭐ SIMULATION TIME AND GPU TIME ARE SEPARATE HERE. The sim advances only at the
canonical manual period; a readback is serviced with `ManualDuration(ZERO)`,
which runs the schedules and moves no clock (measured: 3 pumps per shot). So the
manifest can say the frames are at ticks 31, 33, 35 … and mean it.

⛔⛔ A PRESS IS A REQUEST, AND SUCCESS IS THE INTENDED MOVE APPEARING. The
manifest carries `intended_move` (the composed host's own verb binding),
`observed_moves`, and `reached_intended_move`. A mismatch is reported and the
browser REFUSES the sequence, showing the reason and the diagnostic canvas —
it briefly declared success whenever ANY move played, which would have filed a
forward air under `attack_air_back`.

⭐ TWO PANELS, NOT ONE PICTURE. The engine render is a whole-scene shot in the
CAMERA's space; the diagnostic canvas is in the TAKE's world space. Compositing
them put a strike nowhere near its fighter. They sit side by side and synchronise
on `action_tick` — two separate sessions share no absolute `sim_tick`, but they
share how far into the exercise each frame is.

⛔ AND `--frames` / `--stride` CHOOSE WHAT IS OBSERVED, NEVER WHAT IS PERFORMED.
The hold was `shot < frames / 4`, so asking for more pictures charged a smash
differently: 24 frames at stride 2 held ~12 ticks, a 12-frame run ~6, the
recorder ~37. `HOLD_TICKS` is now a shared constant and the exercise is a tick
schedule both binaries execute.

⚠ Capture-state verbs (pummels, throws) are deliberately absent from the shared
exercise: they need a grabbed opponent, which it cannot set up, and listing them
would promise a capture that would only ever report a mismatch.
