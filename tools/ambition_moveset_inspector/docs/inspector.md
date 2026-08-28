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
[inspector]   cargo build -p ambition_app_tools --bin {moveset_export,moveset_takes,capture_scene}
[inspector] moveset_export  <target>/debug/moveset_export  (built 2026-08-27 17:04, 2h old)
[inspector] moveset_takes   NOT BUILT — there will be no recorded takes to look at
[inspector]                   cargo build -p ambition_app_tools --bin moveset_takes
```

⭐ THE BUILD COMMAND IS NOT ONLY A FAILURE MESSAGE. Somebody refreshing a binary
that already exists needs the same line as somebody who has none, and the age is
what tells them whether they should.

The renderer's own path and build time also ride back with its frames, so the
Engine Takes label reads `sprites: rendered by the engine (capture_scene built
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

ONE binary: **`capture_scene`**. Nothing else in the tool needs a GPU.

```bash
# you build it; the tool never will
cargo build -p ambition_app_tools --bin capture_scene

# and it is useful by hand
target/debug/capture_scene hall_of_characters player /tmp/a/anim.png 480x360 \
    --warmup 60 --character projectile_polygon --frames 24 --stride 2
```

The server looks for it in `${CARGO_TARGET_DIR:-<repo>/target}/{release,debug}/`
— the same convention `scripts/profile_desktop.sh` uses — and when it cannot find
it, the `503` names every path it tried.

⚠ For completeness, the two binaries the REST of the inspector needs, neither of
which wants a GPU: `moveset_export` (the bundle; `serve_inspector.sh` runs it for
you) and `moveset_takes` (the recorded takes; run it yourself for the fighters
you care about).

`--frames N` re-arms the capture after each readback and numbers the files
`<stem>.NNNN.png`; `--stride K` advances K sim frames between shots. A single
shot keeps the exact path it was given, so every existing room recipe is
unchanged.

The inspector asks `/api/render?character=<id>` once per fighter per session and
caches under `data/renders/<id>/`. Engine Takes then draws the engine's own
picture with the hitboxes over it, and the label beside the scrubber reads
`sprites: rendered by the engine`.

⛔ EVERY FAILURE IS A JSON ANSWER. No GPU, no built binary, a driver that will not
start — the route returns `503 {available: false, reason}` and the viewer falls
back to the derived sprites, saying `sprites: derived — engine render unavailable
(…)`. A view that silently swapped between the two would be one whose fidelity
nobody could trust.

⚠ The render is currently a whole-scene capture of a fighter standing in
`hall_of_characters`; it is not yet driven per MOVE. Driving `--press` from the
move's verb is the next step and is why the route already takes a frame count.

## Hitbox geometry

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

The take list shows the fighters that have been RECORDED, which is only ever the
ones you asked for:

```bash
cargo run -p ambition_app_tools --bin moveset_takes -- \
    --characters npc_pirate_admiral,projectile_polygon
```

The fighter picker in front of the take list is the set of recorded characters,
and it follows the fighter you were reading in the Fighter view when you switch
tabs. Status names them too.

Or record the whole grid:

```bash
cargo run -p ambition_app_tools --bin moveset_takes -- --characters grid
```

⚠ MEASURED 2026-08-27: **~1m17 per character**, so the 21-fighter grid is about
**27 minutes**. (It was 7m08 until `settle` stopped serialising a whole frame to
read three booleans.) Every take settles a real match between presses and there
are 19 verbs. This is a background job, not a click — which is why the default
records one fighter and the take view shows coverage (`2 of 21 grid fighters
recorded`) rather than pretending the roster is there.

## Checks

```bash
node tools/ambition_moveset_inspector/check_bundle_contract.mjs   # the data
node tools/ambition_moveset_inspector/check_draw_path.mjs         # the drawing
```

⛔⛔ `node --check` SAYS A FILE PARSES, NOT THAT ITS IDENTIFIERS RESOLVE. A call
to `drawHitboxShape` once shipped with no such function in the file and
`node --check` passed; in the browser `drawTake` threw the instant a strike
appeared, which killed the playback timer. The Jab's first hitbox is on frame 3,
so Play ran for exactly three frames and stopped — with the error only in a
console nobody had open.

`check_draw_path.mjs` draws every frame of every recorded take against a stubbed
canvas and fails on the first exception. Run it after touching `app.js`.
