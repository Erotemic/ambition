# Collision-invariant oracle — first sweep findings (2026-06-04)

Autonomous long-run deliverable for TODO §D **"Headless collision-invariant fuzz
oracle"** + groundwork for Jon's deferred non-autonomous **OOB-fixing** session
(Jon's List: *"significant numbers of out of bound errors… defer for a
non-autonomous run"*). The oracle is the detection legwork so that session starts
with a precise repro list instead of flying around by hand.

## What it is

`crates/ambition_sandbox/tests/collision_invariant_oracle.rs` — a fuzz-driven
per-tick invariant checker over the deterministic `SandboxSim`. Each tick it
reads the player's live AABB + the room's Solid collision world and flags:

- **EMBEDDED-IN-SOLID** — player center sits inside a Solid block (the
  "teleported into a wall" / clipped-through signature).
- **OOB-ABOVE-CEILING** — center went above `y=0` (the bug Jon hit flying up).
- **OOB-BELOW-FLOOR** / **OOB-SIDE** — center left the world the other ways
  (below is usually a legit gap fall; the catalog labels the side so a human can
  tell bug from gap).
- **TELEPORT** — a single-tick jump > 250px that is **not** a door load or a
  death→respawn (both filtered via `active_room` / `resets` deltas), so it only
  fires on a genuine same-room in-place warp. (Blink carries 150px, under the bar.)

It is a **diagnostic, not a CI gate** (deliberate): OOB-via-authored-gap is
expected in some rooms, and the embed/teleport classes are the *deferred* bugs —
a hard assert would false-positive on gap rooms or red-light CI on a known-
deferred bug. So the in-CI `collision_oracle_smoke` only proves the harness runs;
the catalog comes from the `#[ignore]`d full sweep:

```bash
cargo test -p ambition_sandbox --test collision_invariant_oracle \
    -- --ignored --nocapture
```

## First-sweep result — the serious OOB classes do NOT reproduce

**58 rooms × 3 seeds = 174 episodes, 52,200 steps, 1349 violations.**
Every single violation is **OOB-SIDE** or **OOB-BELOW-FLOOR**. There were:

- **0 EMBEDDED-IN-SOLID** — no player ever had its center deep inside a wall.
- **0 OOB-ABOVE-CEILING** — nobody popped out the top flying up.
- **0 TELEPORT** — no in-place warps.

This is the headline: the **"teleported into a wall"** and **"flew up and popped
OOB above the ceiling"** bugs Jon reported this session **did not reproduce in
52k fuzzed steps across every room** — consistent with the two OOB fixes shipped
earlier in this run (`resolve_x_penetration` far-edge/never-eject de-pen, and the
ceiling-graze swept-de-pen defer-regardless-of-immediate-contact;
`crates/ambition_sandbox/src/engine_core/movement/collision.rs`). The *stuck-
inside-a-wall* (embedded) class looks closed.

### UPDATE — the embed check was not the whole story (clip-through-a-wall)

A later through-wall classifier (db02cc2b) revealed the "0 OOB bugs" headline was
**incomplete**. The embed check only catches a center *inside* a Solid; it misses
the player clipping THROUGH a boundary wall and ending up *outside* the room
(center past the wall, not in it) — which is exactly "popped OOB into a wall". The
classifier probes just inside each crossed edge: of the 1335 OOB, most are open-
edge walk-offs (design), but **175 ended up PAST a Solid at the crossed edge**
— concentrated in **intro_escape_shaft** (OOB-SIDE ×37, e.g. seed=1 tick=99 →
(-113,1250)) and **under_town_pipes** (OOB-SIDE ×44 + OOB-BELOW ×94, e.g. →
(-153,738) / (3,793)). Those are *suspect clip-throughs* worth investigating, not
the clean "no bug" the embed-only count implied.

Honest caveat on the 175: it's a **heuristic** (`[past-solid?]`). A player can also
leave through an open gap and then *drift while OOB* (gravity) to a coordinate
that happens to be walled — a false positive.

**RESOLVED — traced, and it IS drift, not a clip** (`trace_oob_under_town_pipes`).
Replaying the worst repro (under_town_pipes seed=1) and logging the trajectory:

```
tick 90: pos=(-193,704) dx=+3 left_edge_solid@y=false   <- already OOB, edge OPEN here
tick 95: pos=(-171,723) dx=+4 left_edge_solid@y=false   <- drifting RIGHT, falling
tick 99: pos=(-153,738) dx=+4 left_edge_solid@y=true    <- now at a y the edge IS walled
```

The player was **already outside** (x=-193) by tick 90, at a y where the left edge
is **open**, then eased back right (+4/tick) while falling. It only reaches a
walled y at tick 99 — but it never *crossed* that wall; it was outside above it
and drifted down past it. So the `[past-solid?]` flag here is a **drift false
positive**, and the 175 are (at least for the worst repro) the same story. The
original conclusion stands: **the OOB are open-boundary level-authoring, not
clip-throughs.** The classifier is still worth keeping as a regression tripwire —
if a genuinely *enclosed* room ever shows `[past-solid?]`, that one IS a real clip
— but the current 175 are not bugs. (The over-flagging comes from counting every
OOB tick; a per-event dedup + a "was inside last tick" gate would tighten it.)

Caveat (be honest): the fuzz is biased toward vertical/fly/pogo/jump input to
stress ceilings, but it is *random*, not Jon's exact input sequence — "did not
reproduce" reduces, not eliminates, the possibility. A targeted replay of Jon's
trace is the complement. Also the embed check is center-based (catches deep
embedding, not a partial clip that resolves the same tick), and a same-tick
OOB→respawn is invisible to a post-tick observation.

## The catalog — OOB at *un-exit* boundaries, by room

The oracle cross-references each OOB against the room's authored LoadingZone /
EdgeExit AABBs (loaded from the LDtk project) and **suppresses OOB that lands at
an opening** — legit traversal, not a clip. The refined sweep (52,200 steps)
suppressed only **14** OOB at authored exits; the remaining **1335 are at
boundaries with *no* loading zone**, so they are NOT the player leaving through a
door. Each is the player **center crossing a world boundary by ~18–50px** (with a
~15px half-width, that's the whole body past the edge), held just past it — which
means it's either an **open/void boundary** (authored, e.g. a sky arena's open
sides or a shaft's open top) or a **boundary the player clips through** (a real
collision bug). Rooms, by frequency (labelled by the room the player was actually
in, so transitions attribute correctly):

| Room | Kind | Count | First repro (seed/tick/pos) | world |
|------|------|------:|------|------|
| under_town_pipes | OOB-SIDE | 509 | 2026 / 153 / (1042,428) | 1024×768 |
| intro_escape_shaft | OOB-SIDE | 266 | 1 / 31 / (-50,1120) | 1280×1280 |
| under_town_pipes | OOB-BELOW-FLOOR | 188 | 1 / 112 / (-136,787) | 1024×768 |
| square_arena | OOB-SIDE | 117 | 2026 / 143 / (1822,1519) | 1800×1800 |
| alice_relay | OOB-SIDE | 89 | 2026 / 156 / (1048,392) | 1024×768 |
| pirate_sky_lookout | OOB-SIDE | 78 | 42 / 79 / (-22,547) | 1280×768 |
| intro_escape_shaft | OOB-BELOW-FLOOR | 48 | 1 / 112 / (-96,1299) | 1280×1280 |
| tiny_chamber | OOB-SIDE | 40 | 2026 / 189 / (926,156) | 900×520 |

Reproduce one: `cargo run -p ambition_sandbox --bin rl_random_walker -- <STEPS>
<SEED>` after launching that room as `--start-room`.

### Interpretation — characterized: these are OPEN boundaries, not clips

Two of the eight (room, boundary) pairs were rendered with `render_room_geometry`
to settle clip-vs-void, and **both are open boundaries, not solid-wall clip-
throughs**:

- **`tiny_chamber`** — solid left wall + ceiling + floor, but the **right side is
  open**: just a LoadingZone entity partway down (y≈310–490) with open dark space
  above it. The OOB at (926, **156**) is *above* the door, where there is no wall
  at all — the player walks off the open right edge. Not a clip.
- **`under_town_pipes`** — **no solid wall on either the left or right boundary**
  (only the nominal world-bounds line); the interior is one-way platforms. The
  OOB-SIDE at (1042,428)/(-19,613) is the player walking off the open sides; the
  ceiling even has an authored gap. Not a clip.

So the catalog is **level-authoring, not a physics bug**: these rooms have edges
the player can leave because nothing is there to stop them. The question for the
interactive session is design — *should* those edges be walled, or are they
intended open (sky arena, shaft, pipe maze)? — not "why does collision fail."
Combined with the zero embed/ceiling/teleport, **no movement/collision OOB bug
reproduced in 52,200 steps**, which corroborates that this run's two de-pen fixes
closed the "teleported into a wall" physics class.

(The other six pairs — `intro_escape_shaft`, `square_arena`, `alice_relay`,
`pirate_sky_lookout` — read the same way from their names/sizes; render any to
confirm. The oracle narrowed 1349 events to 8 boundaries, two of which are now
characterized.)

## Follow-up enhancements (deferred)

1. ✅ *Done in this commit:* suppress OOB at authored EdgeExit/LoadingZone spans
   (the exit cross-reference above). Next: also test whether a **Solid block sits
   at the crossed boundary** to auto-split clip-through (bug) from open-void
   (design) without the manual render step.
2. A scripted-replay mode that feeds Jon's captured OOB trace inputs (targeted
   repro alongside the random sweep).
3. Optionally promote EMBEDDED/ABOVE/TELEPORT to a CI gate once they're confirmed
   zero on a stable build (they are zero today) — a regression tripwire for the
   stuck-in-wall class that was just fixed.
