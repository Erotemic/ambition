# Asset preparation, device materialization and residency

**State:** OPEN — measured user-visible hitch/residency architecture.

Durable asset semantics:
[`../../concepts/asset-management.md`](../../concepts/asset-management.md).
Performance measurements and budgets:
[`performance-and-iteration.md`](performance-and-iteration.md).

## Goal

Make asset demand, preparation, render/device materialization and residency
explicit enough that:

- gameplay does not discover large render assets at the instant they must draw;
- completed background work does not arrive as an unbounded main/render-frame
  burst;
- critical assets have explainable readiness;
- quality changes preserve logical identity;
- long sessions do not retain every asset merely because some historical handle
  survived;
- diagnostics distinguish the stage that is actually late.

This is not a request for a new universal asset manager. Ambition already has
catalog, load, demand and character-residency machinery. Extend the existing
owners when the missing concept is real.

## Measured current model

A rendered desktop capture on 2026-08-29 had healthy steady state but severe
hitches:

- p50 about **7.54 ms**;
- p99 about **12.50 ms**;
- worst frame about **516 ms**;
- `extract_render_asset<GpuImage>` reached about **454.9 ms** against a tiny mean;
- large spikes correlated with bursts of image megapixels arriving together;
- resident images increased during the run and did not fall.

Decode was already asynchronous. The frame-visible cost occurred downstream in
render extraction/device preparation. Current guidance must therefore avoid
calling the problem "synchronous sprite decode."

A follow-up run after prewarming, earlier semantic demand, bounded character
materialization, retained HUD handles, avoiding unconditional hit-flash material
mutation, and other fixes saw a worst in-play frame of about **78.4 ms** instead
of 516.3 ms. That run was not an identical-scene controlled A/B, so treat it as
evidence that the burst architecture was improvable, not as a precise percentage
claim.

The gallery materialization sweep showed that bounding work reduced simultaneous
completion/burst magnitude, but the benefit on an uncovered gameplay frame still
needs a rendered A/B.

### 2026-08-31 — the hall's demand, with the population attached

A windowed capture (`desktop-timeline-run-20260831T210231Z`) walked into
`hall_of_characters` and put a number on what one room asks for at once. From
its own `runtime_census.csv`:

```text
t=65.32   bodies=2     archetypes=1813
t=66.35   bodies=130   archetypes=1975
```

and across the following 3.4 seconds, **71 spritesheet decodes**. **22 of the
run's 30 over-threshold frames are inside that window**, peaking at 199 ms.

⭐ **The demand is concentrated in very few characters.** Two of roughly forty
own 43% of the whole session's decode work, at seven sheets of about 4096² each:

```text
115.6 MP   7 sheets   noether_spritesheet
107.7 MP   7 sheets   perfect_cellular_automaton_spritesheet
-------------------
223.3 MP of the session's 519.5 MP
```

⇒ This is upstream of §3 (pacing) and §4 (budgets): **pacing a demand and
budgeting a residency are both cheaper when the demand is smaller.** Fewer pages
or eviction attack the 43% before any scheduling machinery has to.
⛔⛔ **THE THIRD LEVER THIS SENTENCE USED TO OFFER — "a lower quality tier for
gallery previews" — IS RULED OUT, 2026-09-02.** Jon, after an Ultra host capture
drew the hall from `sprites_0_25x`: *"I DO NOT WANT A LOWER QUALITY TIER FOR
GALLERY PREVIEWS."* The standing rule is wider than this paragraph: nothing may
draw fewer pixels than the user's quality setting asks for, for any room, view or
distance reason, without his explicit yes. ⚠ §3a below and the measurements that
followed it were built on that lever; they are kept as measurements and marked,
not deleted. See `maintainer-decisions.md`, 2026-09-02.

⚠ The worst in-play frame moved the right way against 2026-08-29 — 516 ms → 199
ms — but on a HEAVIER hall, and still not a controlled A/B. The rendered A/B the
paragraph above asks for is still owed.

### ⭐ MEASURED 2026-09-01: the tier is global, and a hall pedestal is 4x oversampled

The doc above says *"a lower quality tier for gallery previews"* attacks the 43%
before any scheduling machinery has to. Here is the number.

**How large a hall character is actually drawn.** Captured the hall twice at
1920x1080 through the real render stack, once with `AMBITION_ACTOR_POPULATION_CAP=0`
and once with `=1`, and differenced the images to isolate exactly one character:

```text
bbox   298 x 131 px
```

Controlled for tier, because a sprite drawn at its texture's native size would
make this measurement circular: the same difference at `ultra` is 133 px and at
`potato` 131 px, so **drawn size is tier-invariant** and 132 px is geometry.

**What is loaded for it.** `noether`'s ladder, from the sheet manifests:

```text
Full     496 x 528 per frame     7 pages    115.6 MP
Half     248 x 264               2 pages     29.2 MP
Quarter  124 x 132               1 page       7.5 MP     <- matches 132 px 1:1
Potato    31 x  33               1 page       0.5 MP
```

⇒ **A pedestal preview drawn 132 px tall loads frames of 496 x 528** — 4x
linear, 16x areal. Quarter matches the drawn size almost exactly.

**Catalog-wide**, over the actors asset root that the desktop dev build actually
reads (`actors_desktop_asset_root()` -> the monolith's `assets/`, NOT the content
root, which ships no variants at all):

```text
tier       pages   megapixels   vs Full
Full         229       1329.9      1.0x
Half         218        352.9      3.8x
Quarter      216        116.0     11.5x
Potato       214          6.3    212.7x
```

⛔⛔ **AND THE TIER IS ONE GLOBAL SETTING.**
`converge_character_residency_to_active_quality` derives a single `active` tier
from `UserSettings.video.quality`, defaulting to `Full`. Nothing considers how
large a thing is drawn, or at what display resolution — so a gallery of 129
pedestal previews and a full-screen fighter ask for the same pages.

⚠ Two caveats before anyone builds on this. The correct tier is
**resolution-dependent**: at 4K the same pedestal is ~264 px and Half becomes
right, which is itself the argument that a global setting is the wrong shape.
⚠ **A correction to my own first reading of the variant counts.** 229 Full pages
against 216 Quarter is NOT 13 missing twins — `noether_spritesheet.2..6` have no
Quarter counterpart because Quarter fits in ONE page. Page counts falling is the
tier working. Counted by CHARACTER instead, coverage is nearly complete:

```text
                characters   no variant at this tier
sprites (Full)         206   —
sprites_0_5x           205   performer_spritesheet
sprites_0_25x          205   performer_spritesheet
sprites_potato         203   performer, actor, medic
```

So a per-use tier still needs a defined answer for a missing variant rather than
a silent fall back to Full — but the population that needs it is three
characters, not thirteen sheets.

⛔⛔ **AND THE FRESHNESS CHECKER COULD NOT SEE THEM.**
`scripts/check_quality_variants_are_fresh.py` walks the TIER directory asking
"is this file older than its source", so a variant that was never published is
never visited. It reported **166 stale files and said nothing about the three
characters that have no variant at all.** Absence now reported alongside
staleness — gameplay sheets only, because `_portraits.ron` (160 vs 9) and
`_actor.ron` (192 vs 47) are published selectively and flagging them buried the
real finding under 979 false ones.

⚠ **166 published tier files are also STALE**, up to 8.4 days behind their
source art, which the checker's own message explains: the game draws old art at
Low/Medium/Potato and current art at High, *"which looks like the character
changing when the quality setting does."* The fix is one incremental command,
`./scripts/regen/quality_variants.sh` — **not run here**, because regenerating
~166 binary art files is Jon's call, not a side effect of a performance run.

⚠ This is a DEMAND measurement — megapixels asked for — not a hitch measurement.
The frame cost of materializing them is a GPU-upload question and this machine
rasterises in software; that half still needs real hardware.

### ⭐ MEASURED 2026-09-01, REAL HARDWARE: the hall-entry hitch, in frames

Jon's `desktop-timeline-run-20260901T231435Z` (RTX 3090, Ultra, Tracy attached
— so absolute frame times carry the instrument's tax, but a 500 ms frame is not
an instrument artefact). The hall loads at wall 24.45 s; the cover lifts at
24.6 s. What follows:

```text
wall s   frame ms        images arriving (1 s census windows)
24.6       132
25.3       118           27.3 s window:  28 images, 150.7 MP, resident 1119 MB
25.5       129           32.3 s window:  93 images, 280.2 MP, resident 2240 MB
25.7       124
26.3       201
26.5       191
26.8       287
27.3       542   <- the frame the 150 MP window lands in
27.5       154
28.2       138
```

Ten frames over 100 ms in the four seconds AFTER the cover lifts, and the art
keeps arriving for eight seconds: **430 MP decoded and uploaded while the hall is
already playable, 2.2 GB of images resident at the end.** The Potato run of the
same room decoded 92 MP in total. This is the "nasty frame drop entering the
hall"; it is not the room load (70 ms under the cover), it is the cast's Full
tier arriving late and all at once.

### ⭐⭐ ROOT CAUSE, found and fixed 2026-09-02 (`2c8f27b32`)

The art arrived late because the reveal barrier never asked for it. Loads
are rationed to one character per frame (`MAX_CHARACTERS_MATERIALIZED_PER_FRAME
= 1`); `demand_room_character_sheets` built its demand in a LOCAL that went out
of scope, so the transition realized ONE character and dropped the other 128;
and the barrier waited only on the pages of realized sheets — one character's,
already cached, 3 ms. The game said so at the reveal: 111 × *"declared as
'npc_x' but not materialized — nothing demanded it"*. The rest were demanded by
their own actors after the reveal and loaded one per frame in the open: the
nine 89-355 ms frames. The comment on the ration claimed `character_reveal_ready`
held the curtain; only `capture_scene` ever called it.

Fix: the remainder is forwarded to the global `CharacterLoadDemand` (transition
and startup; the neighbour prefetch deliberately not), the poll rebuilds the
manifest as sheets realize, and `inspect_demanded_characters` holds the reveal
while any demanded character is only declared. Headless proof: the hall's 129
now realize behind the cover (stalled at 6 before) — one Full character's
worth of pixels per frame, which at the gallery's Quarter tier is sixteen
sheets a frame and the whole cast in ~8 frames (`take_within_budget`, the
areal ration that replaced the head count the same day); the guard
`the_reveal_waits_for_every_placed_character_not_just_the_realized_ones` is red
when the remainder is dropped. ⭐⭐ **HOST TELL PAID, 2026-09-02 — Jon's Ultra run
`desktop-timeline-run-20260902T215256Z`, and all three named tells come back
clean.** The bundle's own **Room reveal** section:

```text
  never materialized      0        (111 on 2026-09-01)
  retired                 0
  undeclared              0
  total                   0
  seq  wait_ms  covered  move
    1      292     True  central_hub_complex -> hall_of_characters
  frames over 33.4 ms AFTER the last transition (t=8.321s):  0   (nine on 2026-09-01)
```

⇒ **The reveal barrier holds on the host**: nothing draws a placeholder, the
cover is up for the wait, and the nine 89-355 ms frames are gone.

⭐⭐ **AND THE SAME RUN SAYS WHERE THE REMAINING HOST HITCH IS: STARTUP, NOT THE
HALL — and the two are the same shape with and without a cover.** The run has
exactly three frames over 33.4 ms, all BEFORE the transition, and
`asset_activity.csv` puts them inside a decode burst:

```text
  wall_s  decoded_images  decoded_MP  decoded_MB     frame spikes in the window
   2.262        0            0.0           0
   2.989       98           20.7          82.8        125.3 ms @2.386, 203.3 ms @2.589
   ...
   6.997      122           42.1         168.5
   8.002      250           71.2         284.7        none — the hub → hall entry
```

⇒ **The hall entry decodes MORE than the startup burst — 128 images and 116 MB
against 98 and 83 — and costs ZERO spikes, because the cover holds for it.** The
startup burst has no cover and costs a 203 ms frame and a 125 ms frame.
⚠ AND THE COMPARISON IS CONSERVATIVE: the hall was under the room tier cap in
this run (quarter-tier art), while startup art was at the setting's own tier, so
the hall's real Full-tier load is larger still and the gap widens.
⇒ **The reveal barrier is not a hall fix, it is a mechanism the first room does
not yet use.** That is the same conclusion the headless work reached from the
other end, now with host frames attached.
  ⛔⛔ **AND THAT READING WAS WRONG — CORRECTED THE SAME NIGHT, by df's question
  rather than by my measurement.** *"A frame spike under a cover is cover time,
  not a hitch"* is this campaign's own rule, applied to the hall six paragraphs
  up, and I did not apply it to my own finding. The run's route lines settle it:

  ```text
  [2.246s] [game-mode]  0.930s f    0  initial playing
           spikes at wall 2.386 (125.3 ms) and 2.589 (203.3 ms)
  [5.179s] [world-event] 3.863s f 1131  room-loaded central_hub_complex
  ```

  ⇒ **Both spikes land between `initial playing` and `room-loaded`** — during the
  first room's load, which is exactly when the load screen with its "Load the
  first room's art" row is up. ⚠ `[game-mode] initial playing` at FRAME 0 is not
  "the player is in the world"; it is the same trap that made an offscreen
  capture report eighteen pops earlier the same day.
  ⇒ **So the honest statement is: three spikes, all before the first room
  finished loading, and the bundle cannot say whether a curtain covered them.**
  ✔ **AND THE ORDERING SURVIVES THE CLOCK TRAP, checked after `d1c63cd5a`
  fixed one just like it.** That commit's lesson is that before/after
  `room-loaded` is a FRAME comparison and its author had used the clock — the
  census prints in `Last`, so a log line's wall stamp reads later than the
  event it describes. ⇒ The comparison above is wall-time too, and it holds
  anyway because the margin is not one frame: the spikes are at 2.386 and 2.589
  against a `room-loaded` stamped 5.179, **2.6 seconds apart**. ⚠ A frame-stamp
  comparison would be strictly better and the bundle carries the frames
  (`f 1131`); anyone re-deriving this should use them.
  ⚠ Independence, since the same merge touched this area: these figures come
  from the bundle's own `## Room reveal` / `## Frame time` sections and from
  `asset_activity.csv` read directly, not from
  `scripts/measure_first_room_manifest.py`, which is what `d1c63cd5a` changed.
  The claim that this is "the last user-visible hitch" is withdrawn — it needs
  the route's presentation state at 2.4-2.6 s, which nobody has yet read.
  ⭐ What survives untouched is the COMPARISON, because both halves are measured
  the same way: the hall decodes more and spikes zero WITH a cover, and the
  startup burst spikes twice at whatever cover it has. That still says the cover
  mechanism works; it no longer says a player sees the startup one.

⛔⛔ **BUT ONE OF THE THREE IS CONFOUNDED AND MUST BE RE-TAKEN.** This is the run
in which Jon saw blur — the ROOM TIER CAP was live and the hall drew from
`sprites_0_25x/` (§3a). The two COUNT tells are tier-independent and stand: a
placeholder means an actor resolved no sprite at all, and the cover either held
or it did not. **The TIMING tell is not**: "0 frames over 33.4 ms" was measured
while the hall was decoding quarter-tier art, which is a fraction of the pixels
Full asks for. ⇒ **Re-take the >33.4 ms count after the cap is removed**, on the
same route, before calling the hitch closed. ⚠ The prediction to check it
against: if the barrier is what fixed it, the count stays 0 at Full; if the cap
was doing the work, it comes back. 
⛔⛔ **AND VALIDATING THE READER AGAINST THAT CAPTURE FOUND A DEFECT IN IT — A
`[frame-spike]` LINE CARRIES TWO CLOCKS.** `[   2.386s] [frame-spike]
1.071s  125.3ms` holds the stamper's wall time AND the game's own elapsed
time, 1.3 s apart here. The transition line is a `tracing` record with no game
clock, so the stamp is the only quantity both share — and the parser was
ordering a spike's GAME time against a transition's STAMP. ⇒ On this bundle
both clocks agree (every spike precedes the reveal), so the verdict above was
right BY LUCK; a run with a spike inside the offset would have been
misreported. Fixed, and pinned by an arm where the two clocks disagree —
the first version of that test could not fail, because the real fixture cannot
discriminate.

⚠ Also corrected: the transition's time is the COMPLETION line's stamp
(8.321 s), not the `room-transition begin` marker three lines earlier
(7.767 s). My hand-check misread that; the parser had it right.

The profiling bundle summary grows a
**Room reveal** section parsing all three out of the game's own stamped log, so
a host capture states its own verdict instead of needing someone to read stderr
and judge — which is how the 2026-09-01 capture's 111 warnings became evidence
for a cause nobody checked.
⛔ **THE WARNING'S DIAGNOSIS SPLIT IN THREE and the phrasing above predates it.**
A RETIRED sheet was demanded AND decoded, so counting it beside "never
materialized" is the conflation `retired_tier` was added to end; the section
reports the split. ⛔ The spike log caps at 60 lines, so a count past the cap is
a FLOOR and says so. Guarded by
`test_room_reveal_tells_are_read_from_the_log.py`, whose BEFORE fixture is the
2026-09-01 capture's shape — 111 warnings, `asset_wait_ms=3`, nine spikes of
89-355 ms after the transition line. The hitches now happen
UNDER the cover, which is what a cover is for; the tier cap (3a) makes them
small; the upload pacer / render-world-only knobs below make them smaller.

**Two mechanisms, and the lever for each — to MEASURE, not yet to change:**

- The **upload** half runs in the render world's `prepare_assets<GpuImage>`.
  Bevy 0.19 ships a pacer for exactly this: insert
  `RenderAssetBytesPerFrame::new(bytes)` and `GpuImage::byte_len` participates;
  assets over the budget wait for the next frame (whole assets only — a 4096²
  RGBA page is 67 MB and always uploads in one frame). The game inserts nothing,
  so the budget is unlimited.
- The **extract** half (`extract_render_asset<GpuImage>`, the ~455 ms zone on
  the MAIN thread in the earlier capture) was a CLONE of the decoded bytes,
  because every sheet loaded with the default
  `RenderAssetUsages::MAIN_WORLD | RENDER_WORLD` — which also kept the CPU copy
  alive, which is where the 2.2 GB came from. **Landed 2026-09-02: sheet,
  parallax, fx, boss and character-page images load `RENDER_WORLD` only by
  default** (`load_sheet_image`). Bevy 0.19 then `take_gpu_data`s the pixels at
  extract (a move, not a clone) and leaves the `Image` in `Assets<Image>` with
  `data == None` and its size intact. The main-world readers were checked
  first: `texture_is_ready` and the room manifest read the asset server's load
  state (residency only for handles without one), the image census derives
  bytes from the descriptor when the data is gone, and the remaining
  `Assets<Image>` readers are procedural images (portal cones, quasar shader,
  touch overlay, render targets) that never go through the funnel.
  Measurement (`capture_scene hall_of_characters player … --warmup 400`,
  Quarter, llvmpipe): the two captures are **byte-identical** (md5
  `c0312413be50`) and peak RSS is **1533 MB → 1392 MB**; the 141 MB is the
  hall's 131 MB of decoded pages plus their extract clone. On the 3090 the
  same CPU bytes leave the process at every tier (2.2 GB at Full was mostly
  this copy), and the extract zone becomes a move.
  `AMBITION_IMAGES_RENDER_WORLD_ONLY=0` restores the CPU copy for an A/B; the
  visual-quality census row and the profile-history label
  (`+render-world-only`) record which way a capture was taken, so rows from
  before 2026-09-02 stay a separate experiment.

**The upload pacer is still one environment variable away, so Jon can measure it
on the 3090 without a code change** (read once and recorded on the
`[census] visual_quality` row and in the ledger label):

```sh
AMBITION_RENDER_ASSET_MB_PER_FRAME=64 scripts/profile_desktop.sh --no-tracy    # +upload:64MB
AMBITION_IMAGES_RENDER_WORLD_ONLY=0   scripts/profile_desktop.sh --no-tracy    # the old CPU-copy loading, for an A/B
```

What to read after each: the hall-entry spike list, `resident_mb`, and whether
any sprite draws blank.

**Software rasterizer (calculex VM, 6 vCPU, llvmpipe) — NOT A GPU RUN, 2026-09-02.**
Adapter `llvmpipe (LLVM 20.1.2, 256 bits)`, a **Cpu** adapter; visual quality
therefore seeded itself to **`potato`**, which is a consequence of the adapter
and not a choice. `capture_scene hall_of_characters player 640x360 --warmup 400`
at `234bcc686`, three reps per arm, **arms interleaved** (rep1 a,b,c; rep2
a,b,c) via `scripts/asset_pacer_ab.sh`; medians, `scripts/asset_pacer_ab_report.py`.
⛔ These numbers cannot be compared with a 3090 row and must not be averaged
into one: this machine has no `/dev/dri` at all, and frame cost here is raster
and decode.

| arm | resident MB | images (MP) | never drawn | insert→gpu max | awaiting gpu | 3 worst spikes (ms) | max RSS MB |
|---|---|---|---|---|---|---|---|
| default (no lever) | 118.1 | 235 (29.5) | 215 (28.6MP) | 161 ms | 0 | 181, 156, 147 | 1236 |
| `AMBITION_RENDER_ASSET_MB_PER_FRAME=64` | 118.1 | 235 (29.5) | 215 (28.6MP) | **235 ms** | 0 | **261, 246, 149** | 1269 |
| `AMBITION_IMAGES_RENDER_WORLD_ONLY=0` | 118.1 | 235 (29.5) | 215 (28.6MP) | 168 ms | 0 | 178, 177, 167 | **1341** |

⭐ **THE PACER IS A COST HERE AND BUYS NOTHING.** `insert→gpu max` rises 161 → 235 ms
and the worst spikes rise 181 → 261 ms, with the three pacer reps (227/235/251 ms)
entirely clear of the three default reps (144/161/188 ms) — a separation, not
noise. Residency is byte-identical across all three arms, because the pacer
throttles when pixels reach the GPU and changes nothing about what was decoded.
⛔ `awaiting gpu` read 0 at every census sample in every arm, INCLUDING the paced
one; the delay is real and the always-on counter did not show it, because
`awaiting` is sampled at the five-second window boundary and the backlog drains
between samples. Read `insert→gpu max`, not `awaiting`, when asking whether a
pacer engaged.

⭐ **AND THE CPU-COPY ARM AGREES WITH THE EARLIER llvmpipe READING, ON A SECOND
SOFTWARE RASTERIZER.** `AMBITION_IMAGES_RENDER_WORLD_ONLY=0` costs **+105 MB**
max RSS (1236 → 1341) here, against the **+141 MB** (1392 → 1533) recorded above
on the aivm box. Same direction, same order of magnitude, different absolute
values — expected, since the tier and room state differ. Two independent
software rasterizers now say the CPU copy is worth roughly the hall's decoded
pages, which is what the RENDER_WORLD-only default was landed to reclaim.

⚠ What this row does NOT settle: whether the pacer helps on real GPU hardware,
where the upload is a PCIe transfer competing with a frame rather than a memcpy.
The 3090 A/B above is still the one that answers that, and this row is evidence
that its default must not simply be flipped on for everyone — on a machine with
no GPU the lever is a regression.

Neither replaces the per-use tier above: at Quarter the same hall asks for
11.5x fewer megapixels and neither half has 430 MP to move.

## Existing architecture to build on

The character path already has much of a residency service in domain-specific
form:

- a demand token/set identifies required character products;
- materialization fulfills demanded sheets;
- live-quality convergence changes the selected product tier.

Keep this semantic ownership. Do not replace it with a disconnected global
cache merely to centralize bookkeeping.

## Open work

### 1. Stage-specific observability

Keep separate evidence for:

```text
source IO
→ decode
→ Bevy asset insertion
→ render extraction / GPU preparation
→ resident/ready use
```

A late-asset report should name the requested logical asset, provider/source,
stage, demand time, completion time and whether gameplay was already live.

✔ **Three of the five stages are on one ledger since 2026-09-02**
(`ambition_asset_manager::image_stages`, process-global because three worlds
write it): the DEMAND (stamped in `load_sheet_image` — character pages,
parallax, fx, boss sheets — and in the manifest catalog's `load_optional`),
the INSERTION into `Assets<Image>` (the census, main world) and the GPU
PREPARATION (`stamp_gpu_prepared_images`, render world, after
`prepare_assets::<GpuImage>`). Readout, on the same clock as `[image]`:

⛔ **WHAT IS DELIBERATELY NOT STAMPED, so a `demand=unknown` on one of these is
EXPECTED rather than a defect to chase:** menu icons, shell presentation images
and prop pngs. The ledger's road names cover CONTENT ART decoded at runtime, because that is
the population the hall-entry hitch is about.

⭐ **MEASURED 2026-09-02, because three descriptions of this vocabulary
disagreed** — this list, a code comment, and a reviewer's reading. Every string
literal reaching `note_demand` / `load_sheet_image` / `load_sprite_pages`:

```text
asset-manifest   boss-sheet   character-sheet   fx-sheet   held-item
parallax         portrait     projectile-art    shrine-sheet   vanity-card
```

**TEN live roads.** This list named seven and omitted `asset-manifest`,
`held-item` and `shrine-sheet`; the review that flagged the drift said nine and
missed `shrine-sheet` too. ⇒ Nobody's hand-kept copy was right, which is the
argument for deriving it rather than restating it — a road is added by passing a
new string literal at a call site, and nothing makes the prose follow. UI chrome is small, loaded once and not what a room's reveal waits on;
labelling it would make the ledger's rows less comparable, not more.

⛔⛔ **A `demand=unknown` MAY BE A RE-DECODE RATHER THAN AN UNROUTED LOAD, and
today you cannot tell them apart.** `ImageStageLedger::removed()` does
`self.rows.remove(&id)` — it deletes the whole row, demand stamp included — while
`insertions_by_path` deliberately survives (its own doc says "which is the point").
`demand()` only ever runs at a LOAD call site, and a second `load` of a path
already resident is a handle lookup rather than a decode, so an image that is
DROPPED and later RE-DECODED comes back with no demand, permanently.

⇒ The population that loses its attribution is exactly the one worth attributing:
the wasted half of the decode budget, which `[image-dropped]` is there to count.
⚠ Anyone reading `demand=unknown` as "some road forgot to stamp" is sent hunting
through call sites that are already correct — measured 2026-09-02, an exhaustive
sweep of every `.load(` in the tree (plus `load_untyped`/`load_acquire`/
`load_folder`/`get_handle`/`load_with_settings`, which have ZERO uses) found no
unstamped route for the sheet that reported it.

✔ **FIXED 2026-09-02 (`438662619`), and NOT quite as this row prescribed.**
`ImageStageLedger` keeps `demand_by_path`, so a re-decode inherits the road that
first demanded the path and the readout says `first demanded via <road>` — a
third phrase beside a stamped demand and a genuinely unstamped one. ⛔ The row
asked for `demanded_at` to be carried across too, and it deliberately is NOT:
that instant belongs to the FIRST demand, so `wait()` computed from it would
measure from the wrong moment and print a duration this decode never took. Only
the SOURCE is adopted, and the phrase is worded so it quotes no duration it
cannot honestly compute. ⇒ `demand=unknown` now means unstamped, full stop; the
"unstamped OR re-decoded" ambiguity this row existed for is gone.

⚠ And the `game://` in a row is NOT an authored prefix. `load_sheet_image` labels
each row with `AssetPath::to_string()`, and an `AssetPath` PRINTS ITS SOURCE — so
every load through the `game` source reads that way whether or not any code wrote
the scheme. Grepping the tree for `game://` finds one const and a handful of doc
comments; the question is always which ROAD inserted the image without stamping,
never which string was written.

```text
[image]      0.911s 3006x2462 7.4MP live=1 sprites/bob_spritesheet.png demand→insert 219ms via character-sheet — DECODED DURING GAMEPLAY …
[image-gpu]  1.404s 7.4MP live=1 sprites/bob_spritesheet.png insert→gpu 493ms demand→insert 219ms via character-sheet
[image-census] … | gpu +17 (+64.3MP) insert→gpu p50 493ms max 493ms | awaiting gpu 0
```

What the first run (hall, llvmpipe, `capture_scene`) showed: every sheet the
reveal demanded was inserted 219–385 ms after its demand and then prepared in
ONE render frame — seventeen sheets, 64 MP, `insert→gpu 493ms` for all of
them, because `RenderAssetBytesPerFrame` is unlimited by default. That is the
upload half of the hitch made visible; `AMBITION_RENDER_ASSET_MB_PER_FRAME`
spreads it and the census line's `awaiting gpu` shows the backlog it creates.
Software-rasteriser numbers, so the magnitude is not Jon's; the SHAPE (one
frame takes the whole batch) is the finding.

The same ledger counts the two kinds of WASTE open work 5 asks about, on the
census line: `re-decodes N` (a path inserted a second time, under any id) and
`dropped before gpu N (MP)` (decoded, removed before any GPU saw it), with an
`[image-dropped]` line per notable file. First reading, `capture_scene`
starting in the hall: re-decodes 0; **8 sheets, 25.6 MP decoded for nobody**
(architect, bob, erdish, goblin, alice, oiler + two small), all
`via character-sheet`, dropped at 1.2 s. ✔ CLASSIFIED the same hour, by
reading the names: that is the INTRO CAST, and
`load_intro_npc_sprites_system` (`game/ambition_content/src/intro/plugin.rs`)
loads every intro NPC's sheet ONCE AT STARTUP, in whatever room the game
boots into, at the SETTING tier, and publishes them under display names
(`publish_under`) so an LDtk `NpcSpawn.name` can find them. Global eager
loading — this document's own non-goal — from the content side: on every
boot ~26 MP is decoded before any of it can be seen, at Full about 100 MB
resident, and in a capped room the tier convergence then retires it unseen.
The fix shape is open work 2's: register those sheets as AUTHORED sheets for
their characters (`AuthoredSheets`, the U1 seam the goblin/architect rows
already use through `sheet_for_character_id_in`) so the room's cast demand
raises them on entry at the room's tier, and delete the startup preload. The
intro-only targets (`creator`, `oiler`, `erdish`, `news_board`) are the ones
with no catalog character today, which is why the preload existed. Not the
hitch — it runs before the cover — but it is the first named row of open
work 2 with a measured size.

✔✔ **AND IT IS GONE — the table was deleted, and the row above is the reading
that got it deleted.** `crate::intro::sprites` now carries the tombstone: eleven
`(display name, filename, spec)` rows published under DISPLAY NAMES that the
intro world never authors — every `NpcSpawn` in `intro.ldtk` carries a
`character_id` with `name: None`, and the peaceful-NPC road sets
`sprite_override_npc_name: None`, so **no lookup could reach a single one of
them**. Two rows were doubly dead (`EnemySpawn`s with their own ids since
2026-08-12) and one had no placement at all. The rows also fed
`extend_with_intro_sprite_entries` into `PreloadGroup::SandboxCore`, a second
preload road off the same table; deleting the table closed both. Props keep
their loader — a `Prop` is keyed by `Prop.kind`, which the world does author.
⇒ The fix shape this paragraph proposed (re-register as `AuthoredSheets`) was
not needed: the sheets had no reachable consumer to re-register FOR.

✔ **The one `demand=unknown` in the hall is the LDtk PREVIEW TILESET (found
2026-09-02 by probing the ledger, not by grepping).** Every world file —
`hall_of_characters`, `intro`, `sandbox`, `you_have_to_cut_the_rope` — declares
a tileset `sprite_player_robot_v3 = ../sprites/player_robot_v3_spritesheet.png`
(3072×2484, 7.6 MP) so the editor can draw entity previews, and `bevy_ecs_ldtk`
decodes every tileset of a project when the project loads: asset index 4 of
the process, path printed with the `game://` source, no demand stamp because
no code of ours asked for it. It is the FULL-tier player sheet, decoded beside
the tier the game actually realizes (Potato in that run), on every boot and
every world load, and it is never drawn by the runtime. Fix is one line per
world in the map submodule — point the preview tileset at the `sprites_0_25x`
copy (0.5 MP; the editor preview survives at a quarter of the resolution) —
and that is Jon's file, so it sits in `queue.md`'s host section awaiting his
say-so rather than being edited from here. Everything else that decodes an
image in the shipped composition now stamps its road.

Not covered, honestly reported: source IO and decode are one stage here
(Bevy's loader does both on the IO pool and `Added` fires after).

⛔ **TWO CLAIMS THAT WERE IN THIS PARAGRAPH ARE STALE, AND THE SECOND IS
CONTRADICTED FURTHER DOWN THIS PAGE.** It said *"'resident use' (first draw) is
not stamped"* — the FOURTH STAGE IS BUILT, `first_drawn_at` and
`resident_never_drawn()` are on the ledger and a render system feeds them; see
the ✔ section below. And it said the unrouted set *"in the hall is exactly one"*
— the census that split `UNROUTED` from `PROCEDURAL` reports **nine** file-backed
rows, listed below, of which three were new. Corrected 2026-09-02; the tileset
below is still the LARGEST of the nine and still the one that matters.

`game://sprites/player_robot_v3_spritesheet.png`, found 2026-09-02, is
`bevy_ecs_ldtk` loading the FIVE worlds' editor-preview tileset (`relPath:
../sprites/player_robot_v3_spritesheet.png`) through the `game` source — the
same file the player's realization decodes again as `sprites/…` through the
default source (host captures `015511Z`/`015909Z`: 7.6 MP at 0.8 s game time
and again 0.15 s after `room-loaded`). ⚠ Two sources make two asset ids, so
the ledger's re-decode census, keyed by path string, cannot see this pair;
the fix is the tileset retarget in `queue.md`'s host section (Jon's
submodule), not a route. `[image-gpu]` lines only appear with a
render world; headless runs show `awaiting gpu` growing instead, which is the
readout saying nobody uploaded.

#### ✔ The fourth stage: RESIDENT USE (first draw) — BUILT 2026-09-02

⭐⭐ **BOTH HALVES LANDED.** The ledger half (`b360f4a3a`): `first_drawn_at`,
`first_drawn(id, at)` and `resident_never_drawn()`. The render half: a
`stamp_first_drawn_images` sibling of `stamp_gpu_prepared_images` in
`ImageStagePlugin`, ordered after `RenderSystems::ExtractCommands`, reading
`ExtractedSprites` and stamping first-write-wins. A `[image-drawn]` line prints
`demand→draw NNNms via <road>` at the same NOTABLE threshold the other stages
use, and the census row gains `never drawn N (M MP)`.

⭐⭐ **AND IT NAMES A POP.** `live_at_first_draw` sits beside `live_at_insert` and
answers a different question: the insert flag says whether the DECODE cost a live
frame, this says whether the ARRIVAL was seen. They disagree in both directions —
art decoded under a cover and first drawn minutes later is fine, art decoded live
but never drawn is waste rather than a pop. `[image-drawn]` says `POP (drawn
during gameplay, after the cover)` rather than `live=1`, because a hitch log's
reader should not have to know which way a flag points.

⛔⛔ **AND IT ONLY SAYS THAT WHERE A COVER EXISTED.** `capture_scene` reaches
`[game-mode] initial playing` at 0.633 s, before the room loads — so on a road
with no cover every first draw is trivially "during gameplay", and the first hall
run after this landed reported EIGHTEEN pops that were the harness.
`saw_covered_frame` (set on the ledger's first not-live frame, never cleared) is
what separates *"the cover did not cover this"* from *"nothing here has a
cover"*.
⚠ With that guard in, the hall capture still reports eighteen — the transition
DOES suspend gameplay, so the road has a cover and the "no cover" diagnosis was
wrong. ⛔ That is not a claim that the hall pops: the eighteen are 0.0 MP entity
tiles first drawn ~0.5 s after `room-loaded`, and whether a player sees that is
not something an offscreen capture's warmup ordering can settle. **The mechanism
is what exists; the verdict is a host reading.**

⛔⛔ **AND `PROCEDURAL` IS NEVER A FINDING ON THAT ROW.** The stage is stamped
from `ExtractedSprites`, and a render target, a shader input or a material
texture is never a sprite — it is written to or sampled, not extracted. Those
rows are permanently "never drawn" BY CONSTRUCTION, so the 4-6 MP the bucket
reports in a hall capture is the instrument describing its own blind spot, not an
asset problem. Only the file-backed roads answer a residency question here.

⛔⛔ **AND THE ROW PRINTS `-`, NOT `0`, WITHOUT A RENDER WORLD.** With nothing
extracted, EVERY resident image is "never drawn" — which on a headless road means
nobody could have drawn anything, not that the pixels were wasted.
The ASKING App's `RenderWorldPresent` resource is the fact that separates the two
readings and the row consults it; a readout that skipped that check would accuse
a `NoWindow` run of waste it cannot commit. ⭐ It was a `bool` ON THE PROCESS
LEDGER until 2026-09-02, which answered "did any App in this process render"
rather than "does the one asking" — see `queue.md`'s ledger row for the fix and
why it was latent rather than live. Both rules are guarded on the pure ledger and
poison-verified (drop first-write-wins and the guard names the instant that
moved).

⭐⭐ **MEASURED 2026-09-02, `capture_scene hall_of_characters player --warmup 400`
(llvmpipe, OffscreenGpu), byte-identical across two runs.**
⛔⛔ **AT POTATO TIER, AND THE NUMBER IS MEANINGLESS WITHOUT THAT.** llvmpipe
classifies as `DetectedGpuClass::Cpu`, whose seed profile is `Potato` — by
design, and `seed_profile_for_gpu` says why ("a software rasteriser is not a weak
GPU, it is NO GPU"). So these megapixels are NOT comparable with this document's
434 MP (Full) or 38 MP (Quarter) hall figures; the paths in the run confirm it
(`sprites_potato/…`). What IS comparable across tiers is the SHAPE — the ratio of
drawn to resident, and the split by owner — because those are counts.

```text
total 239 images, 30.6MP, 122.4MB resident | never drawn 219 (29.6MP)
resident by road: UNROUTED(no demand) 8×7.6MP, PROCEDURAL(no file) 26×4.5MP,
                  character-sheet 138×4.2MP, fx-sheet 13×9.4MP,
                  entity-sprite 28×0.1MP, parallax 4×2.4MP, boss-sheet 1×2.0MP,
                  held-item 20×0.1MP, shrine-sheet 1×0.3MP
```

⚠ **RE-TAKEN AFTER TWO OF THE DAY'S OWN FIXES, and both moved it.** Routing the
shrine sheet took it out of `UNROUTED` (9 files → 8) and gave it a road of its
own; giving the 28 entity icons `entity-sprite` took them out of `fx-sheet`
(41 → 13). The earlier block in this section quoted the pre-fix numbers, which is
how a document acquires two readings of one run.
⛔ AND THE `never drawn` FIGURE MOVED FOR A DIFFERENT REASON: 195 became 219
because the DRAW GATE changed, not because less was drawn — the first version of
`[image-drawn]` could not fire in a capped room at all. Do not read the two as a
regression.

⇒ **20 of 239 images are drawn, and they are 1.0 of the 30.6 MP.** Ninety-seven
per cent of the resident megapixels are not on screen at that framing.
⛔ THAT IS NOT A WASTE NUMBER AND MUST NOT BE QUOTED AS ONE. The hall places 129
characters and the camera sees a slice of them; most of "never drawn" is
off-camera, which is correct. What the number IS, and what no earlier stage could
say, is the SIZE OF THE HEADROOM: the resident set is 5.8× the pixels the camera
draws, so a residency policy that evicted on first-draw evidence has that much to
work with. Open work 4 asks for the owner of retained assets before any eviction
policy; this is the other half of that question.

⭐ **AND THE UNROUTED SPLIT PAID FOR ITSELF ON THE FIRST HOST-SHAPED RUN.** Before
`a20b5b1a2` that row read `? 35×12.5MP`; the nine real findings were invisible
inside twenty-six procedural inserts. The census now names them:

```text
[image-unrouted] 9 file(s) decoded with no demand stamp:   <- 8 now; the shrine was routed
  7.6MP game://sprites/player_robot_v3_spritesheet.png   <- the editor-preview tileset
  0.3MP sprites/shrine_spritesheet.png                    <- NEW; ROUTED the same day
  0.0MP game://sprites/intro_lab_tileset.png
  0.0MP sprites/props/portal_gun_blue.png                 <- NEW; left unrouted, see below
  0.0MP sprites/props/portal_gun_orange.png               <- NEW; left unrouted, see below
  0.0MP game://worlds/{hall_of_characters,intro,sandbox}.ldtk#int_grid_image (+1)
```

⇒ The 7.6 MP tileset is the one this document already identified by a bespoke
ledger probe; it is on the census line now, every run, for free. The shrine sheet
and the two portal-gun props are NEW — small, but they are art reaching
`Assets<Image>` by a road that stamps nothing, which is the class this bucket
exists to catch. The `#int_grid_image` rows are `bevy_ecs_ldtk`'s own, the same
family as the tileset.

⚠ Taken on a tree carrying another session's uncommitted population-cap work.
That knob is inert with `AMBITION_ACTOR_POPULATION_CAP` unset, so it cannot move
an image count — but the run is not a pure `main` reading and is recorded as
such.

<details><summary>The scoping note, kept because it is what the build was measured against</summary>

The ledger measures demand -> insert -> GPU. All three are about the asset
ARRIVING; none says it was ever USED. That gap is why the re-decode census and
the reveal barrier both have to talk about "prepared" rather than "drawn", and
why `[image-dropped]` can only report pixels decoded for nobody AFTER the fact.

⭐ **THE SEAM EXISTS AND IS ONE RESOURCE.** `bevy_sprite_render::ExtractedSprites`
is a render-world `Resource` holding `Vec<ExtractedSprite>`, and
`ExtractedSprite.image_handle_id` is an `AssetId<Image>` — the same key the
ledger already uses. It is reachable as `bevy::sprite_render::…` behind bevy's
`bevy_sprite_render` feature. Extraction happens AFTER visibility culling, so an
id appearing there means "this frame would draw it", which is the honest meaning
of resident use — closer than anything the three current stages can say.
⚠ `SpriteBatch.image_handle_id` is one step later and strictly stronger (it
survived batching), at the cost of running after `RenderSystems::Queue`. Prefer
`ExtractedSprites` unless a measurement shows extraction over-reports.

**What it would take**, stated so nobody re-derives it: `ImageStagePlugin`
already owns a render-world system (`stamp_gpu_prepared_images`, after
`RenderSystems::PrepareAssets`) and already inserts the shared `ImageStageClock`,
so this is a SIBLING of an existing hook rather than new machinery — and the
ledger is a process-global `static`, so a render-world system can reach it
without a channel. Three pieces: a `first_drawn_at` field plus a `first_drawn(id,
at)` method on `ImageStageLedger` (first write wins; later frames must not
overwrite), one `stamp_first_drawn_images` system reading `ExtractedSprites`, and
one `.add_systems` line. ⛔ NOT BUILT IN THE SCOPING PASS, because that is three
pieces rather than the one-line hook that would justify landing code inside one.

⛔ **AND ITS TEST STILL GOES ON THE PURE LEDGER, though the reason changed under
it the same day.** When this was scoped, a `NoWindow` composition decoded NO
file-backed art at all (`ImagePlugin` registers the image loader in
`Plugin::finish`, which never ran under that composition's `app.update()` loop),
so no `app_it` test could wait for anything. ✔ `124684f56` fixed that and images
now decode headlessly — but a DRAW is still not available there: `NoWindow` has
no render world, so `ExtractedSprites` never exists and nothing is ever
extracted. ⇒ Unit-test `first_drawn`'s FIRST-WRITE-WINS rule directly on
`ImageStageLedger`; the end-to-end confirmation belongs to `capture_scene`
(OffscreenGpu) or the windowed host. A test that waits for a draw headlessly
would wait forever, and such a test tends to become one that waits a fixed
number of frames and asserts nothing.

⚠ **Two things to get right when it is built.** (1) The stamp must be
FIRST-WRITE-WINS, or it becomes a per-frame write on every visible sprite and the
ledger's own cost shows up in what it measures. (2) A headless or `NoWindow`
composition has no render world at all, so `first_drawn` is permanently absent
there — the same asymmetry `is_awaiting_gpu` already documents, and the readout
must say "no render world" rather than "never drawn", which are different facts.

</details>

### 2. Demand before first visible use

✔ **Two rows closed 2026-09-02, both found by the ledger's "resident by
road" census** (`[image-census] … resident by road: …`, the owner measurement
open work 4 asks for first): the intro cast's startup preload (deleted — it
published under display names nothing looked up) and **every dedicated boss
sheet decoding at boot** — seven sheets, 30 MP, the hall's single largest
owner with no boss in the room. `load_game_assets` decodes none now;
`ensure_boss_sheets_loaded` is the one seam, called by
`build_room_asset_manifest` / the direct startup manifest when a room authors
a `BossSpawn`, so the reveal waits on them there. Hall of characters, same
capture, same tier: **86.6 MP / 346 MB resident → 32.7 MP / 131 MB**, and the
largest upload frame 299 ms → 101 ms (llvmpipe). What remains under `?` is
the LDtk preview tileset (7.6 MP, Jon's submodule) and ~4.7 MP of unstamped
UI/prop art, by design. ✔ And the narrowing landed the same day: a boss room
demands only ITS bosses' sheets — `boss_sheet_keys_for_room` runs the
renderer's own key derivation (`canonical_boss_id_from` → profile → lowercased
id) over the authored placements before anything spawns, so a one-boss arena
decodes one sheet, not seven; `basement_boss` (deliberately generic) decodes
none, correctly.

Where semantic composition already knows the roster/room/UI assets, raise demand
there rather than from `Added<ActorConfig>` or another first-use event.

Do not prefetch every asset in the product. Demand should follow the prepared
composition and the expected near-term experience/room.

▢ **A THIRD ROW OF THE SAME SHAPE, SIZED 2026-09-02 AND DELIBERATELY NOT CLOSED:
the FX set is 13 sheets / 9.4 MP, loaded at boot, in every room.**
`load_game_assets` calls `load_fx_sheets` unconditionally, so the engine's whole
effect vocabulary is resident whether or not anything ever plays one — 32% of the
hall's resident megapixels on the capture above, and the second-largest owner on
the `resident by road` line after the cast.

⛔ **AND IT IS NOT OBVIOUSLY THE SAME MISTAKE**, which is why this is a row and
not a fix. The two rows this section closed had no defence: the intro cast
published under display names nothing looked up, and the boss sheets belonged to
a boss that was not in the room. The FX set has one, stated at its load site: *"an
engine that draws an asset has to be able to ship it"* — before that registration
Smash, Sanic and Mary-O drew the particle fallback for every effect, forever. And
an effect is not a cast: a room's characters are known at entry, while any effect
may fire on any frame, so there is no "demand at room entry" seam to move it to.

⛔⛔ **AND `fx-sheet 41×9.6MP` WAS TWO POPULATIONS — corrected twice before it
was right, which is the lesson of this row.** First write-up: "41 sheets".
Second: "13 sheets across 41 pages". Both wrong. `load_entity_sprites` stamped
its 28 entity icons — door zones, solid tiles, one-way tiles, NPC terminals —
with the road `"fx-sheet"`, so the bucket held the effect vocabulary AND the
world's entity icons and a measurement of "how big is the effect set" counted
both. Given its own road (`entity-sprite`, 2026-09-02) the hall reads:

```text
fx-sheet 13×9.4MP        entity-sprite 28×0.1MP
```

⇒ **The FX set is 13 images and 9.4 MP; the 28 that made it look like 41 are
0.1 MP of entity icons.** Same class as the thirteen vfx sheets that were stamped
`character-sheet` until the ownership rule landed — and found the same way, by an
instrument printing a road beside a path: `[image-drawn]
sprites/entities/door_zone.png … via fx-sheet` is the line that said it out loud.
✔ **AND THE REST OF THE STAMPS WERE SWEPT THE SAME DAY — no third instance.**
Every `load_sheet_image` / `note_demand` call site in the workspace, label read
against what it loads: `portrait` (HUD portraits, select-screen portraits),
`held-item` (wielded + ground item visuals), `projectile-art`, `parallax`,
`boss-sheet`, `vanity-card`, `shrine-sheet`, `asset-manifest`, and the
character/fx dispatcher that takes its road as a parameter. All correct. Two
mis-stamps have now been found in this system (vfx as `character-sheet`, entity
icons as `fx-sheet`) and both were found by a readout printing a road beside a
path, not by reading the call sites — which is the argument for the readouts.

⭐ THE INSTRUMENT FOUND ITS OWN INPUT'S BUG. A residency census is only as good as
its stamps, and nothing else in the pipeline would have noticed: the images
loaded, drew and were freed correctly the whole time. Only the ATTRIBUTION was
wrong, and attribution is the entire product of open work 4.

⚠ **THE DRAW COUNT IS TOO NOISY TO QUOTE, and I nearly quoted it.** One scripted
smash capture (admiral, four moves) ended with `fx-sheet 38×8.8MP` never drawn —
3 pages drawn — and was about to be recorded as "3 of 41". Three more runs with a
LONGER press sequence, including George and Noether, all ended `fx-sheet
41×9.6MP`: **zero**. `never_drawn` only shrinks, so those are real disagreements,
not sampling of one clock — what differs is whether a swing happened to CONNECT
and fire a hit effect. ⇒ A scripted capture draws somewhere between none and a
handful, and the number depends on combat outcomes it does not control. Do not
build a policy on it; the union over a REAL play session is still the reading
that would settle one.

⭐⭐ **SO THE ANSWER CAME FROM A COUNT INSTEAD, and it is sharper.**
`scripts/measure_fx_row_reachability.py` asks which fx ROWS any content can name
at all — an effect is drawn by name (`FxId::new(row)`), so a row nothing names is
art the running game has no way to request. Corpus: every tracked
`.rs`/`.ron`/`.yarn`/`.json` except the baked sheet manifests themselves.

```text
13 sheets, 196 rows, 120 named, 76 named by nothing
sheets with NO row named by anything: 1 ['pirate_admiral_vfx']
  george_booul_vfx        21 rows,  1 named, 20 unnamed
  pirate_admiral_vfx      14 rows,  0 named, 14 unnamed
  generic_exotic_fx       24 rows,  9 named, 15 unnamed
  pca / patent_clerk / carl_stargan / noether / generic_explosions: fully named
```

⛔⛔ **AND THE CHEAP FIX IS THE WRONG ONE.** "Drop from the preload what nothing
can request" would delete `pirate_admiral_vfx` — whose fourteen rows are
`grapeshot_cloud`, `heave_to_anchor`, `heave_to_brake`, `cutlass_wake`,
`boarding_wake`, `captains_mark`… beside a moveset whose moves are named
`grapeshot`, `heave_to`, `gun_sword`. The art was drawn FOR his kit; the moveset
names `muzzle_flash` and `air_slice` from the generic sheets instead. George is
the same: 20 of his 21 rows (`bivalence_weak/strong`, `excluded_middle_*`,
`modus_ponens_*`, `reductio_*`) sit beside moves called `bivalence`,
`excluded_middle`, `commitment`.
⇒ **This is a MISSING-WIRING finding, not a dead-art one**, and it belongs to
whoever owns those two fighters' presentation rather than to residency. What
residency can say is the size: 9.4 MP resident in every room, of which 76 of 196
rows are art nothing can currently ask for.
⚠ A string search cannot see a name built at runtime (`format!`), so a row it
calls unnamed could in principle be constructed. The five fully-named sheets are
the evidence the method works; treat a single unnamed row as a lead, not a
verdict.

⛔⛔ **MEASURED 2026-09-02: THE SEAM BELOW ASSUMES A NAMING CORRESPONDENCE THAT
DOES NOT EXIST, and the sheet-to-owner map is not derivable at all.** "Demand
`<character>_vfx` beside `<character>`'s pages" reads as if the sheet name comes
off the character. Three conventions are in play:

```text
noether_vfx        -> npc_emmy_noether       (id shares no token with the sheet)
carl_stargan_vfx   -> npc_carl_stargan       but its SHEET TARGET is carl_runga
pca_vfx / patent_clerk_vfx / george_booul_vfx  bare ids, no npc_ prefix
```

⇒ String surgery would build `npc_noether`, match nothing, and the sheet would
silently stop loading — the effects fall back to particles with no error, which
is the failure mode this subsystem specialises in. Ownership has to be DECLARED,
and a hand-declared table across three conventions is where a silent mis-mapping
gets written.

⭐⭐ **SO IT WAS MEASURED INSTEAD. `scripts/measure_fx_row_reachability.py
--owners` reports which FILES name each sheet's rows** — a sheet whose rows are
named only by one moveset belongs to that moveset, on the evidence rather than
on the name. Byte-identical across three runs:

```text
SHARED, stay resident (4)   generic_action_fx, generic_world_fx,
                            generic_exotic_fx, generic_explosions
                            — each named by four or more unrelated movesets,
                              and generic_explosions also by the engine itself
OWNED by one moveset (7)    oiler_vfx 18 rows -> oiler_moveset.rs
                            pca_vfx 14 -> cellular_automaton_moveset.rs
                            patent_clerk_vfx 14 -> patent_clerk_moveset.rs
                            carl_stargan_vfx 12 -> carl_stargan_moveset.rs (+1 from performer)
                            noether_vfx 12 -> emmy_noether_moveset.rs
                            ninja_shadow_oni_leader_vfx 10 -> its own moveset
                            projectile_polygon_vfx 3 -> projectile_polygon_moveset.rs
NAMED BY NO CONTENT (2)     pirate_admiral_vfx (nothing at all),
                            george_booul_vfx (a TEST and the engine's fx.rs only)
```

⛔ **THREE CORRECTIONS TO THE PROSE BELOW, which was written from the names.**
(1) `projectile_polygon_vfx` was listed among the five "no character owns" — it
has a single owner and is not generic. (2) `pca_vfx` is named by
`cellular_automaton_moveset.rs`: "pca" is not a fighter. (3) `george_booul_vfx`
is not merely under-wired — **no content names any of its rows**; its only
askers are a test and the engine. So the split is 4 shared / 7 owned / 2 dead,
not "five generic, eight per-character".

⭐⭐ **LANDED 2026-09-02: the seam is CHARACTER REALIZATION, and the ownership
column the paragraph above said was owed was never needed.** A realized
character carries its own prepared moveset, and the moveset's `Vfx` events
name the rows — so `character_sprites::demand_character_fx_sheets` asks
`PreparedCharacterDefinition.kit.projectable_moveset()` the frame the sprite
realizes (`materialize_character_demand`, every road: room transition, first
room, in-room drain) and decodes whichever character-owned sheets those rows
live on (`fx::owned_fx_sheets_named_by`, over `MoveSpec::vfx_effects()` — the
same enumeration `presentation_problems` validates at install). Ownership is
read off the content that fires the effect, not off a name; there is no table
to mis-map. Each `FxSheet` declares `FxResidency::{Core, OwnedByCharacter}`:
four core sheets (`generic_action_fx`, `generic_world_fx`, `generic_exotic_fx`,
`generic_explosions`) decode at boot as before; the nine others (the eight
character-named sheets and `projectile_polygon_vfx`, which one moveset owns)
follow their character. The room manifest lists every fx sheet demanded so far
(`fx:<target>`), so the reveal barrier waits on a fighter's own effects like a
page. `pirate_admiral_vfx` and `george_booul_vfx` — named by no moveset — are
now never decoded, which is the right residency for art nothing can request;
the wiring question stays in `awaiting-maintainer-decision.md`. Guard:
`the_engine_ships_its_own_effects.rs` (boot decodes the core set and only it;
`cell_birth` is a particle burst at boot and sheet art once the PCA realizes)
+ `fx::tests::owned_sheets_are_owed_by_the_rows_that_name_them`. The size,
from the sheets' own headers (`scripts/fx_residency_census.sh` reads the
live `resident by road: fx-sheet` term for any room): boot went from 13
sheets / 9.4 MP to **4 / 2.8 MP**; the 9 owned sheets are 6.7 MP and each
arrives only with its fighter (the host run of 2026-09-02 evening, taken
before this landed, still shows `fx-sheet 10×7.7MP` never drawn in the hub).

⭐ **THE BOOT POPULATION, from Jon's host run (2026-09-03,
`scripts/measure_first_room_manifest.py` over
`desktop-timeline-run-20260902T215256Z`).** The run's only two spikes off the
hall (125 ms and 203 ms at 2.4–2.6 s) sit between `initial playing` and the
first `room-loaded`, i.e. under the shell's load screen. What decoded before
that `room-loaded`, ordered **by frame** (the census runs in `Last`, so its
clock can read after a `room-loaded` the same frame's `PreUpdate` preceded):
**7 images / 23.7 MP**, of **252 images / 78.3 MP decoded in total** — ⚠ the
`[image]` ledger prints only decodes ≥ 1.0 MP (`NOTABLE_MEGAPIXELS`), so these
7 are 3% of the boot by count and every figure here is a floor.

By road: `character-sheet` 8.9 MP, `unknown` 8.6, `vanity-card` 3.0,
`boss-sheet` 2.0, `fx-sheet` 1.2. Five of the seven (15.1 MP) arrived on a road
a manifest speaks; the two `unknown` (8.6 MP) carry no demand stamp and are
counted neither way.

⭐ **The largest item is the player sheet, and it decodes TWICE.**
`game://sprites/player_robot_v3_spritesheet.png` at frame 0 with no demand
stamp, and `sprites/player_robot_v3_spritesheet.png` at frame 1129 via
`character-sheet` — 3072×2468, 7.6 MP, both times. That one sheet is 15.2 MP of
the 23.7 across the two roads, and the second decode is 69 ms.

**Traced 2026-09-02, and it is ONE file decoded twice, not two files.** Five
LDtk worlds (`hall_of_characters`, `intro`, `sandbox`,
`you_have_to_cut_the_rope`, and sanic's `sanic_speedway`) carry the tileset
`relPath: "../sprites/player_robot_v3_spritesheet.png"`. Loaded as
`game://worlds/<file>`, that resolves to `game://sprites/player_robot_v3_spritesheet.png`
— and `game/ambition_content/assets/sprites` is a **committed symlink** at
`crates/ambition_platformer2d_actor_monolith/assets/sprites`, so it is the same
file the `character-sheet` road reaches as `sprites/…`. Two asset paths, one
file, two `AssetId`s, two decodes. ⇒ The cost is 7.6 MP and 69 ms of redundant
decode for the protagonist's sheet at every boot, and Jon's relPath retarget is
the named fix.

⛔ **AND IT COSTS NOTHING ON DISK OR IN THE PACKAGE — do not go looking for the
megabytes.** I first read the identical sha256 in both roots as "the art ships
twice"; it does not. The two roots hold one file, and
`package_asset_guard.py::iter_regular_files` SKIPS symlinked directories when
it walks (`if path.is_symlink(): continue`, the `dirnames` loop), so the sprite
tree is enumerated exactly once, from the monolith root. The defect is purely a
runtime double-decode. ⚠ `Path.rglob` does not descend a symlinked directory
either, which is how a census over the two roots can report "0 files in common"
about a tree where every sprite is shared.

⇒ The `prepare-first-room-art` cover already waits for the 38 assets the first
room names. ⚠ 38 assets is not comparable to 7 images — a manifest asset may be
several pages, and the cover counts what it NAMED rather than what decoded. A
cover that also waited for the shell's own art would be a longer cover, which
is a product choice, not a defect.

⛔ COVERABLE IS AN UPPER BOUND: it says the image arrived on a road the manifest
speaks, not that a manifest would resolve it.

The tell after Jon's relPath retarget is the same two spikes gone or halved.

### 3. Pace expensive completion, not declarations

Staging/demand and expensive materialization are different operations. Declare
all required work promptly, then pace only the stage whose burst cost is
measured.

Choose a budget from rendered measurements. "One character per frame" is a
current useful bound, not a universal theorem.

⭐ **THE HALL AT FULL, HEADLESS, WITH THE CAP GONE (2026-09-03, this VM, 12
vCPU / 3 IO threads):** `hall_transition_cover::the_halls_transition_bills_its_whole_cast_and_covers_the_wait`
— **barrier released after 135 frames with 129/129 realized.** That is the
RATION floor (one Full sheet started per frame, 129 sheets) plus six frames;
decode kept pace with it here. ⇒ On the host the floor is different: at 300
fps 135 frames is 0.45 s, and Jon's Quarter run decoded 130 images / 36 MP
inside `wait_ms 292` (~123 MP/s aggregate on the 3090's four IO threads), so
**434 MP at Full predicts a cover hold of ~3.5 s, DECODE-bound, not
ration-bound.** The two levers that move a decode-bound hold, neither of which
touches pixels: Bevy's IO pool is capped at 4 threads by default
(`bevy_app` `task_pool_plugin.rs`: 25%, max 4 — 16 cores get 4), so a
`TaskPoolPlugin` setting on the host could double decode throughput during a
covered load at the price of compute threads the cover does not need; and the
PNG format itself (a QOI or zstd-raw sheet decodes several times faster — a
pipeline change, Jon's). A bigger COVERED ration would not help the host (it is
not ration-bound there) and would only shorten this VM's number. The host walk
at Full measures which bound applies; do not add the knob before it.

### 3a. ⛔⛔ REMOVED 2026-09-02 BY JON'S RULING: there is NO room-level sprite tier cap

**Jon, on the 3090 at Ultra, profile run `desktop-timeline-run-20260902T215256Z`:
"I DO NOT WANT A LOWER QUALITY TIER FOR GALLERY PREVIEWS. WHOEVER WROTE THAT IS
WRONG."** The run shows why he saw it: every hall sheet decoded from
`sprites_0_25x/` (`[image] … sprites_0_25x/noether_spritesheet.png … via
character-sheet`) at `profile=Ultra`, and the gallery was blurry at 1600×900.

The cap (`dc3cd0d91`: `room_sprite_tier_cap: gallery → Quarter`, a `(floor,
ceiling)` staleness range, a per-token tier on `CharacterLoadDemand`,
`PendingRoomTierFloor`, `budget_for_room`) is deleted, mechanism and all, not
parked behind a flag — a capability that can lower the user's tier is the
defect, whichever room uses it. **Standing rule: nothing may draw fewer pixels
than `UserSettings.video.quality` asks for, for any room, view, distance or
population reason, without Jon's explicit yes.** The "4x oversampled pedestal"
measurement (132 px drawn at 1080p vs 496 px loaded) stands as a number; the
decision it was used to justify was a feel ruling that was not mine to make.
The "view-scoped tier" follow-up below is dead for the same reason.

What the hall entry costs again, and how it is paid: the cast decodes at the
user's tier (434 MP at Full, measured 2026-09-01) UNDER THE COVER — the reveal
barrier (`2c8f27b32`) holds the loading foreground until every manifest page is
decoded and uploaded, so the cost is cover time, never a hitch after the
reveal and never pixels. The levers that remain are the ones this document
already names: demand seams (§2), the areal ration and upload pacing (§3), and
residency/eviction (§4). ⚠ The 2026-09-02 host run's `wait_ms 292` and the
"expect ~40 MP" tells in `queue.md` were taken WITH the cap and are void; the
next host walk re-measures the hall's cover hold at Full.

<details>
<summary>What was built and removed (kept so nobody rebuilds it by accident)</summary>

Built 2026-09-02 as planned: the cap derived from the authored `gallery`
flag; staleness became a range `(floor, ceiling)` so a Full sheet in a gallery
was kept and a Quarter sheet carried into a Full room retired before the
reveal; the demand carried a per-token tier and the convergence knew the room
being loaded (`PendingRoomTierFloor`) so the in-room drain did not realize the
forwarded cast at the hub's Full (103 Full sheets behind the cover, headless).
Headless the hall went 434 MP → ~40 MP and the cover held ~9 frames; the host
run at Ultra read `wait_ms 292`, 0 frames over 33 ms after the reveal, and a
blurry gallery. The original plan text (authored `sprite_tier_cap` field,
`min(settings, cap)` at one seam, nearest-tier fallback for `performer`,
`actor`, `medic`) is in the history of this section at `dc3cd0d91`.

The VIEW-scoped tier idea (only draw the tier a live camera needs; 14 of 138
cast pages drawn in a hall capture) is recorded in the same history. It was
scoped as needing three feel rulings — pop, hysteresis, whose view — and the
ruling above answers it: no.

</details>

### 4. Define residency ownership and budgets

Name the owner for retained assets, for example:

- process/global shell;
- current experience;
- current/nearby room;
- active roster/participant;
- transient presentation effect.

Then measure working-set growth and choose eviction/release policy. Do not pick
LRU before the ownership/budget model exists.

✔ **The owner of a character page is its realization, and that is now a rule
with a guard (2026-09-02, `124684f56`).** Every image on the `character-sheet`
road belongs to a row of `CharacterSpriteAssets` (a character sheet or a prop);
the fx set demands on its own `fx-sheet` road and belongs to `FxSheetAssets`
(13 vfx sheets were stamped `character-sheet` until this landed, which is how
the rule was found). Retiring a realization drops the page's last handle, so
after a room exit the ledger's resident `character-sheet` rows must all be
owned by a live realization — a page resident with no realization is held by
something else, and that something is the leak. Measured on the reverse leg
(`leaving_the_gallery_keeps_the_shared_cast_and_retires_the_rest`, hall →
hub, 300 frames after the commit): **0 orphan pages**; the retired gallery
cast leaves memory. Poison: holding five gallery page handles across the exit
names exactly those five. What stays resident in the hub and why: the hub's
placed cast and the worn character (their realizations), the one-hop
neighbours' casts the prefetch realized at THEIR tier (`basement_enemies`
spawns an "Ai Slop", which is why `npc_ai_slop` comes back at Full — for the
basement, not for nobody), the fx set, and 24 images on no demand road (4.5 MP).
⚠ THAT LAST FIGURE WAS TWO POPULATIONS IN ONE BUCKET and is superseded: split on
2026-09-02 (`a20b5b1a2`), the hall reads `UNROUTED(no demand) 8×7.6MP` — real
findings, now named on an `[image-unrouted]` line — and `PROCEDURAL(no file)
26×4.5MP`, which can never carry a road because there is no load to stamp. ✔ **Working-set GROWTH measured the same evening**
(`two_round_trips_through_the_gallery_return_the_same_working_set`, headless,
real decode): hub = 6 realizations / 16 character pages / 13.4 MP; hall =
139 pages / 44.3 MP at Quarter; back in the hub after each of two laps:
**6 / 16 / 13.4 MP, identical** — the set returns to baseline, nothing
accumulates. The price of that boundedness is visible in the same run: the
second hall entry RE-DECODES the gallery (`RE-DECODE #2` on every hall
sheet, 44 MP at Quarter, 7 frames under the cover on a warm page cache vs 32
cold). That is the eviction question stated with numbers: today's policy is
"retire everything the destination does not place", and a budget policy
would instead keep the last room's cast resident while the total stays under
a limit. ⚠ Still open: the limit itself (a host number — `resident_mb` at
Full on the 3090 after a hub→hall→hub walk is the input), and the neighbour
prefetch remains the only road that decodes in the open on purpose.
⛔ **AND THE 2026-09-02 HOST RUN DOES NOT SUPPLY IT, though it looks like it
does.** `desktop-timeline-run-20260902T215256Z` reports *"decoded images 0 → 252
(78.3 MP, 313.1 MB of decode work); images resident at end 251"* — ⚠ that is
DECODE WORK over the run and a count at the end, not a steady-state residency,
and the hall leg of it ran under the room tier cap (quarter-tier art), so it is
not the FULL figure this limit needs. It is also a one-way walk: hub → hall, with
no return leg. ⇒ The input is still owed, and whoever takes the capture should
make it hub → hall → hub at Full with the cap gone.

⚠ **Parallax WAS the one road that ACCUMULATED — superseded the same evening
by the retire below; kept as the sizing.** (Read from the code 2026-09-02.)
`ensure_parallax_layers_for_room` lazy-loads a theme's four layers on first
visit and nothing releases them: `ParallaxLayerSet` has `ensure_theme_loaded`
and no retire. Nine themes × 4 layers = 37 files, **21.2 MP at Full** (every
theme 2.4 MP), so a session that visits every zone holds 21 MP / ~85 MB of
backgrounds for the rest of the process. Owner by the list above: "current/
nearby room" — the same rule character pages follow — so a retire-on-commit
that keeps the active theme and the one-hop neighbours' would bound it at
≤3 themes. Bounded and small next to a hall cast at Full, which is why it is
recorded rather than built; the `resident by road: parallax` term after a
multi-zone walk is the measurement.

⭐ **BUILT AND MEASURED 2026-09-02 (`f1445c142`). And the accumulation was not a
caller's omission — it was a GUARANTEE OF THE TYPE**, which is why the fix had to
start in `ambition_sprite_sheet` and not at a call site:

- `ParallaxLayerSet`'s whole API was `get`, `ensure_theme_loaded`, `len`,
  `is_empty` over a PRIVATE map; no `remove`, `clear` or `retain` existed and
  nothing outside its module named the type;
- `GameAssets` is assigned once, by `bind_game_assets` on `Startup`, so the
  accumulating set lives for the whole process;
- the one path that looks like a release,
  `refresh_parallax_layers_on_quality_change`, despawns `ParallaxLayerVisual`
  ENTITIES and respawns them without touching the handle store.

`ParallaxLayerSet::retain_themes` is the eviction API and owns no policy;
`world_flow::parallax_residency` owns the rule (active + one-hop), ordered after
the prefetch. Verified end to end by `scripts/measure_parallax_retire.sh`:
`[Hub, Basement, Boss]` → `[Hub, Basement]`, and the retired theme's images leave
`Assets<Image>` — asserted separately, because dropping a handle is necessary and
not sufficient when a spawned visual may hold a clone.

⚠ **Two corrections to the estimate above, both downward.** The ceiling is
reached less often than "visits every zone" suggests, and the rule's own bound is
looser than what is ever loaded:

- ⛔ **`NEIGHBOR_PREFETCH_ROOM_BUDGET` is 4** — the prefetch prepares at most four
  neighbours however many a room has. `central_hub_main` has 21 exits into six
  biomes, so the rule PERMITS six themes there, but only **three** are ever
  resident (`[Hub, Basement, Boss]`, measured). Sizing this leak from adjacency
  overestimates it by double.
- The route that would test it is not the obvious one: `central_hub_main` and
  `hall_of_characters` BOTH resolve to `ParallaxTheme::Hub` (`biome: hall` is not
  a `from_key` key, so it falls through to `visual_theme: default`), so the hall
  door crosses no theme boundary and a retire assertion on it passes while doing
  nothing. The fixture walks `tech_bros_door` into Basement instead.

#### ⭐ The hall by TIER after the cap removal (calculex VM, software rasterizer, ff1ce535b)

`capture_scene hall_of_characters player 640x360 --warmup 400`, tier forced with
`AMBITION_QUALITY_PROFILE` and **verified from `[census] visual_quality
profile=`** rather than assumed. ⛔ Counts, not clocks: this box has no GPU, so
the megapixel and image figures transfer and the frame times do not.

| tier | images | resident MP | MB | character-sheet | fx-sheet | parallax | UNROUTED |
|---|---|---|---|---|---|---|---|
| Potato | 234 | 24.1 | 96.5 | 137 × 4.2MP | 10 × 7.7MP | 4 × 0.0MP | 8 × **7.6MP** |
| High | 235 | 29.9 | 119.4 | 138 × 5.4MP | 10 × 7.7MP | 4 × 2.4MP | 8 × **7.6MP** |
| Ultra | 235 | 29.9 | 119.4 | 138 × 5.4MP | 10 × 7.7MP | 4 × 2.4MP | 8 × **7.6MP** |

⭐ **THE UNROUTED POPULATION IGNORES THE TIER, AND IT IS THE BIGGEST SINGLE
THING IN THE ROOM.** `game://sprites/player_robot_v3_spritesheet.png` is 7.6 MP
at Potato, at High and at Ultra — byte-identical across all three. At Potato that
one file is **32% of the hall's entire resident megapixels**, and it is the road
the census names `UNROUTED(no demand)`: decoded with nobody claiming to have
asked for it. An image that arrives without a demand stamp also arrives without a
quality path, so the tier cannot reach it.

⛔⛔ **AND IT IS A SECOND COPY OF A SHEET THE ROOM ALREADY HAS AT THE RIGHT
TIER. The census names the cause in its own line:**

```text
[image]  0.923s f 0 3072x2468 7.6MP live=0 game://sprites/player_robot_v3_spritesheet.png
         demand=unknown (not through load_sheet_image)
[image-drawn] 1.412s 0.1MP sprites_potato/player_robot_v3_spritesheet.png
         demand→draw 128ms via character-sheet
```

Frame **0**, before gameplay, 3072×2468, and `demand=unknown (not through
load_sheet_image)` — the instrument says exactly which road was skipped. Half a
second later the SAME character's sheet arrives again through the demand road at
the resolved tier, 0.1 MP, and that is the copy the game actually draws. The
7.6 MP one appears in `never drawn` for the rest of the run.

⛔ **AND THE LOADER IS NOT OURS — CORRECTED 2026-09-02 after this was first
written up.** It is `bevy_ecs_ldtk` loading every project tileset: **FIVE**
`.ldtk` worlds declare `sprite_player_robot_v3` with
`relPath=../sprites/player_robot_v3_spritesheet.png` at 3072×2484, for EDITOR
entity previews. So `demand=unknown` is accurate and the road is genuinely
absent — but the fix is retargeting five declarations at `../sprites_0_25x/…`
in the map submodule, which is Jon's, and the row that owns it is in
[`../queue.md`](../queue.md).

⚠ **FIVE, NOT FOUR — recounted 2026-09-02 in the submodule itself.** The four
`ambition_content` worlds (`hall_of_characters`, `intro`, `sandbox`,
`you_have_to_cut_the_rope`) plus **`ambition_demo_sanic/worlds/sanic_speedway.ldtk`**,
which the earlier "verified in all four world files" missed because it counted
the worlds `ambition_content` exposes rather than the worlds that carry the
declaration. All five still point at full resolution; none is retargeted.
`ambition_demo_mary_o/worlds/mary_o.ldtk` does NOT reference the sheet.

ⓘ And the count is the ONLY thing wrong here: the files under
`game/ambition_content/assets/worlds/` are git symlinks (mode 120000) into
`game/ambition_map_assets`, so `git ls-files` listing them is not evidence they
are ours. They are Jon's submodule, exactly as this paragraph says. ⚠ Do not go looking for
this in our character-sprite loader; an earlier version of this paragraph would
have sent a reader there.

⇒ **So this is not "one image ignored the tier". It is a DUPLICATE: 7.6 MP
loaded at boot by the LDtk spine, never drawn, alongside the 0.1 MP copy the
realization loads and the game actually draws.** At Potato that duplicate is 32% of the hall's resident megapixels and
76× the drawn copy.

⛔ **AND THE VARIANT IT SHOULD HAVE USED EXISTS.** All four are on disk for that
sheet — `sprites/` **4.3 MB**, `sprites_0_5x/` 2.2 MB, `sprites_0_25x/` 844 KB,
`sprites_potato/` **56 KB**. The Potato run decoded the 4.3 MB one. That is
**77× the bytes** of the variant its own tier authored, for the single largest
image in the room, and the seven other unrouted files are ~0.0 MP each — so this
one file IS the unrouted population in any sense that costs memory.

⇒ **So "the tier decides how many pixels the hall holds" is true of 5.8 MP of it
and false of 7.6 MP of it.** Potato → Ultra moves 24.1 → 29.9 MP; the unrouted
7.6 MP is constant underneath. ⛔ This is a ROUTING observation, not a proposal to
draw fewer pixels — the fix is a demand stamp, after which that sheet honours
whatever tier the user chose, at whatever size that tier says.

⛔⛔ **AND A SECOND ROAD IGNORES THE TIER, LARGER THAN THE FIRST: `fx-sheet` is
7.7 MP at Potato, High and Ultra alike.** This one is not a routing gap — the FX
sheets DO carry a demand stamp (`via fx-sheet`). The cause is one missing
parameter, visible in the signature:

```rust
let characters = character_sprites::load_character_sprites_in(…, quality);
let entities   = load_entity_sprites(catalog, asset_server, quality);
let fx         = character_sprites::load_fx_sheets(asset_server, layouts, &config.sprite_folder);
```

`load_fx_sheets` is the only one of the three that never receives the quality
budget. It builds its set `with_sprite_folder(sprite_folder)` — a fixed folder —
so it cannot select a variant even though the variants exist: 12 fx PNGs in
`sprites/` (1.3 MB), 12 in `sprites_0_25x/` (964 KB) and 12 in `sprites_potato/`
(**68 KB**). The Potato run decoded `sprites/generic_exotic_fx_spritesheet.png`
at 1216×958.

⭐ **AND THIS ONE IS INVISIBLE ON THE MACHINE THAT WOULD NOTICE IT LEAST.** At
Ultra, full-resolution FX is exactly right, so the 3090 sees no defect at all. It
costs only the configurations that ASKED for less — Steam Deck, mobile, web, and
a weak-GPU desktop — which is the target class `../../planning/vision.md` names
and the one no 3090 measurement can reach. Together with the undrawn duplicate
above, **15.3 of the hall's 24.1 resident MP at Potato — 63% — is art that does
not respond to the tier at all.**

⭐ **AUDITED ACROSS EVERY LOADER, so this is a complete list rather than one
example.** Of the eleven `load_*`/`ensure_*` entry points in the asset path,
**seven take the quality budget and three do not**:

| takes `quality` | does NOT |
|---|---|
| `load_character_sprites_in`, `load_entity_sprites`, `load_game_assets`, `ensure_boss_sheets_loaded`, `ensure_theme_loaded`, `load_parallax_layers_for_theme`, `ensure_parallax_layers_for_room` | ⛔ `load_fx_sheets`, ⛔ `ensure_fx_sheet_loaded`, ✔ `load_prop_sheet_for_target` (deliberate — see below) |

`load_sheet_image` is the eleventh and is correctly absent from both columns: it
is the primitive that takes an already-resolved path, so the caller owns the
choice.

⛔ **BOTH FX LOADERS MISS IT — the boot core AND the per-character owned road** —
which is why all ten resident fx sheets are full-resolution rather than just the
four core ones. ⚠ **`load_prop_sheet_for_target` IS NOT ONE OF THEM, corrected after checking
rather than leaving it "unmeasured".** It hard-codes `TextureResolutionScale::Full`
and states the reason in place — *"this path never consults a quality budget, so
nothing was asked for beyond `Full` and nothing but the authored PNG was
loaded"* — and its docstring scopes it to a demo registering one animated prop
outside the asset catalog. Prop variants do exist (30 PNGs each in `sprites/`
304 KB, `sprites_potato/` 120 KB, `sprites_0_25x/` 136 KB), so it COULD tier;
it deliberately does not, on a narrow road, and says so.

⛔ **So the gap is TWO loaders, not three.** `ensure_fx_sheet_loaded` also
hard-codes `Full` — but unlike the prop loader it gives no reason, which is the
difference between a decision and an omission. That distinction is the whole
value of the audit: a count that lumps them together would have reported 50%
more than exists and pointed a fix at code that is already correct.

⛔ Routing again, not pixels: the fix gives a Potato user the 68 KB sheets they
asked for and changes nothing at Ultra.

⚠ **NEVER-DRAWN HEADROOM, same runs.** What the room holds against what it puts
on screen:

| tier | resident | never drawn | DRAWN | headroom |
|---|---|---|---|---|
| Potato | 24.1 MP / 234 img | 23.2 MP / 214 img | **0.9 MP** | **26.8×** |
| High | 29.9 MP / 235 img | 26.5 MP / 211 img | **3.4 MP** | **8.8×** |
| Ultra | 29.9 MP / 235 img | 26.5 MP / 211 img | **3.4 MP** | **8.8×** |

⭐ **The headroom GROWS as the tier drops — 8.8× to 26.8× — which is the
duplicate above showing through.** Lowering the tier shrinks what is drawn
(3.4 → 0.9 MP) but the 7.6 MP undrawn copy does not shrink with it, so it becomes
a larger share of a smaller total: nearly a third of everything resident at
Potato. A residency ratio measured at a low tier is therefore dominated by
whatever ignores the tier.

⛔ **NOT comparable to the "5.8×" in `../tracks.md`**, and this row must not be
read as correcting it. That figure is a host walk-in; this is a 640×360 staged
capture, and a smaller viewport draws fewer sprites, which inflates headroom by
construction. Same instrument, different question.

⚠ **High and Ultra are identical here** (29.9 MP, 119.4 MB, same road split), so
for this room's residency the ceiling is reached at High. And the reveal barrier
held **10 updates** at both Potato and Ultra — the tier did not change what the
room waited for.

⛔ Not comparable to the 434 MP host figure above: that is a walked-in hall on
real hardware, this is a staged capture at 640×360 on a software rasterizer. The
two answer different questions and no delta between them means anything.

### 5. Eliminate accidental re-preparation/reload

⛔⛔ **FOUR SHEETS' REDUCED TIERS ARE NOT REDUCED — MEASURED 2026-09-02, and it
is a COST defect, not a quality one.** `sprites_0_5x` and `sprites_0_25x` exist
so a room or a setting can ask for a cheaper character. Nothing checked that the
variant IS cheaper. `scripts/measure_tier_variant_scaling.py`:

```text
sheet        Full MP   0_5x   0_25x     verdict
actor           9.03   9.03    9.03     identical at every tier
author          8.36   8.36    8.36     identical at every tier
medic           7.54   7.54    7.54     identical at every tier
officer         8.64   8.65    8.65     one pixel taller, not smaller
```

⇒ **A room asking for those tiers decodes 67.2 MP where the tier promises
~10.5 MP** — 6.4x, per sheet, at every stage: decode, upload, residency. For
scale, the whole hall's resident set measures 30.7 MP.

⭐ **WHY IT SURVIVED: THE FAILURE IS INVISIBLE WHERE ANYONE LOOKS.** The art is
correct — it is simply larger than asked for — so a Quarter room renders
perfectly and costs Full. Nothing on screen is wrong, and the tier system's own
accounting believes it saved 16x.

⛔ **IT IS THE OPPOSITE OF THE GALLERY-PREVIEW RULING AND MUST NOT BE READ AS
CHALLENGING IT.** Jon's rule is that nothing may draw FEWER pixels than the
setting asks for; these draw MORE. Regenerating the variants removes no pixels
from any tier that requested them.

⚠⚠ **CORRECTED WITHIN THE HOUR: THIS IS A GENERATOR DEFECT, NOT COMMITTED ART.**
The first write-up called fixing it "a content change" that wanted Jon's say-so.
Every sprite PNG is GITIGNORED and generated — `assets/sprites/.gitignore` is
`*.png`, and `sprites_0_5x` / `sprites_0_25x` / `sprites_potato` are ignored
whole. Nothing here is committed art; the tree holds seven tracked files under
`assets/sprites`, all of them JSON or markdown. ⇒ The fix belongs in
`scripts/generate_visual_quality_variants.py`, is Python, and needs no art
decision at all.
⛔⛔ **AND MY CORROBORATION WAS FALSE — RETRACTED THE SAME HOUR.** I checked the
primary worktree, saw the identical four sheets undownscaled, and wrote that it
"reproduces on a second, independently-generated tree". **It is the same tree.**
`scripts/mirror_assets_for_worktree.py` symlinks a worktree's generated assets
at the main checkout's, file by file — `sprites_0_25x/author_spritesheet.png`
here IS `/home/joncrall/code/ambition/...` there. I measured one set of bytes
twice and called the second read independent evidence.

⭐⭐ **AND THE "WAIT FOR A CLEAN REGEN" DEFERRAL IS NO LONGER THE ONLY ROAD —
MEASURED 2026-09-02, `--ages`.** The leading alternative to a generator defect
was that these are leftovers an earlier render abandoned. They are not: at BOTH
reduced tiers the four are the NEWEST files present (ranks 201–204 of 205),
written 2026-08-27, 4.5 days after the tier's median of 08-22.

```text
sheet      0_5x    0_25x   its own full sheet   reading
actor      09:48   09:49   09:47                same run as its full sheet
author     09:52   09:53   09:52                same run as its full sheet
medic      09:46   09:46   09:46                same run as its full sheet
officer    09:50   09:51   19:53                variants predate it by 10 h
```

⇒ **Three of the four were written within MINUTES of their own full-resolution
sheet**, so a live run produced a full sheet and an unshrunk "reduced" one
together — a generator defect on that code path, not stale output. ⛔ **`officer`
is a DIFFERENT mechanism**: its variants predate its own full sheet by ten
hours, i.e. its full sheet was re-rendered later and its variants were never
regenerated. A fix aimed only at the first mechanism leaves `officer` broken.

⚠ mtimes are per-machine and a copy rewrites them; the evidence is the contrast
inside one tree — the four being the newest files at their tier while the tier's
median is four days older is the part that carries the argument.

⭐⭐ **AND `actor` IS THE PRE-RENAME DUPLICATE OF `performer` — TRACED
2026-09-02 BY ASKING THE GENERATOR INSTEAD OF READING IT.**
`discover_all_targets()` reports 51 `config` and 158 `module` targets, and
**`actor` IS NOT IN THE REGISTRY AT ALL.** `Actor` was renamed to `Performer`
(recorded in `awaiting-maintainer-decision.md` §44), and
`sprites/actor_spritesheet.png` is **byte-identical** to
`sprites/performer_spritesheet.png` — same sha256, different inodes in both the
worktree and the main checkout, so this is real duplication and not a symlink.
**Nothing in the workspace names `actor` as a sprite target.**

⇒ That explains two of the five names at once, and neither is a live generator
defect: `actor`'s reduced variants are **pre-rename leftovers the generator can
never regenerate**, because it no longer knows the name; and `performer` has
**no** variants because the new name never had any generated. ⇒ **18 `actor_*`
files, 14.5 MB, ship as a manifested, bakeable target nothing requests.**

⛔ **THREE HYPOTHESES DIED ON THE WAY, recorded so nobody retraces them.** (1)
"Sheets lacking `source_frame_width` fail to downscale" — 54 of 54 that declare
it are fine, but so are 147 of 151 that do not, so it is necessary-at-best and
explains nothing. (2) "The 08-27 run produced them" — `generic_world_fx` was
written that same day and is fine. (3) "They are the YAML-authored `config`
targets that re-render instead of downscaling" — the exact opposite:
`alice`, which downscales correctly, IS `config`; `author`/`medic`/`officer` are
`module`. ⭐ I built all three by reading the generator and comparing configs;
the answer came from RUNNING `discover_all_targets()` and asking it.

ⓘ **A SWEEP FOR OTHER DEAD TARGETS FOUND ONE, AND THE SWEEP'S BOUNDS ARE THE
INTERESTING PART.** Of 212 sheet targets, exactly one is BOTH absent from
`discover_all_targets()` AND unnamed by engine or content: `pirate_heavy_v2`
(10 files, 2.58 MB, unique bytes — not a duplicate of the four named
`pirate_heavy_*` sheets). ⛔ **The other 27 targets nothing names ARE NOT A
FINDING**: they are authored characters not yet wired, and content breadth is
not gated here — a guard over them would pressure exactly the thing the project
wants unpressured. No ratchet was written for this.

⚠ **The sweep is bounded in both directions and neither bound is small.** A
quoted-string grep over `.rs`/`.ron`/`.json` alone reports **106** of 212 dead,
because the character catalogs are YAML and `git grep` does not enter
submodules. A bare-name grep over every text type reports **ZERO**, because the
renderer defines a module per target and `assets/sprites/<name>_spritesheet.yaml`
names itself — a generated file naming itself is not a reference. The usable
predicate excludes the asset trees and the renderer, and it still MISSES
`actor`, whose bare name collides with `ACTOR_CONSTRUCTION_DOMAIN = "actor"`.
⇒ Short target names cannot be settled by grep; `actor` was found by asking the
registry, not by searching.

⭐⭐ **✔ MECHANISM FOUND 2026-09-02 — `author`, `medic` and `officer` SHARE A
RENDERER FAMILY WHOSE SHEET PATH TAKES NO QUALITY SCALE.** All four of
`performer`, `author`, `medic`, `officer` — and no other target — import
`targets/characters/_authored_swing_fighter.py`. That module's two entry points
do not agree:

```python
def render(self, out_dir, actor_metadata: dict):            # the SHEET — no scale
def render_portraits(self, out_dir, *, clips=None, quality_scale=None):
```

and each target's `build_spritesheet` calls `_FIGHTER.render(out_dir, ACTOR_METADATA)`,
dropping `opts` entirely — so the sheet renders at full resolution whatever tier
was asked for, while the portrait path threads the scale through.

⛔ **AND SOMEONE ALREADY FIXED THIS ONCE, FOR THE OTHER HALF.** `render_portraits`
carries the comment *"⛔ A QUALITY TIER SCALES THE PORTRAIT TOO. Ignoring
`quality_scale`…"* — the same defect was found and fixed for portraits, and the
sheet path beside it was never given the same treatment. ⇒ The cluster that kept
reappearing all day — four names sharing three symptoms — is **one shared module**,
and `actor` is only in it because `actor` is `performer`'s old name.

⛔⛔ **AND THAT IS NOT THE ROAD THE TIER FILES COME FROM — CORRECTED THE SAME
DAY, after yardrat's fresh-clone generation produced CORRECTLY SCALED variants
for all four.** Two boxes disagreeing is what forced the reconciliation; the
mechanism above is real and it is not what bit here.

**WHICH ROAD PRODUCES A TIER FILE TODAY.**
`generate_visual_quality_variants.py` chooses per sheet:
`source_publishable_targets()` keeps only `kind == "config"` targets, and a unit
gets `target` set only if it is in that dict AND its `.ron` sits directly in
`assets/sprites`. Those go to `publish_source_quality_target` — a **re-render**
at `quality_scale`. **Everything else, including every `module`-kind target,
goes to `build_sheet_variant`, which RESIZES the full sheet's isolated frames.**
`author`, `medic`, `officer` and `performer` are `module`-kind, so through this
script they take the resize road and scale correctly.

⇒ **That is why a fresh box is clean and this one is not.** A fresh clone's
tiers are produced entirely by that script. This box's four files were written
2026-08-27 09:46–09:53, minutes after their own full sheets, by a DIFFERENT
invocation: the renderer's own `publish` CLI, which takes `--quality-scale` and
`--dest-root`, calls `render_sheet(out_dir, **opts)` and installs straight into
the tier directory. For a `module` target that lands in `medic.render`, whose
body is:

```python
def render(out_dir: str | Path, **opts):
    del opts                      # ⛔ quality_scale, explicitly discarded
    return _FIGHTER.render(out_dir, ACTOR_METADATA)
```

⇒ **CAN THE RENDER DEFECT STILL BITE? Yes, but only one way in.** A direct
renderer `publish`/`render` aimed at a tier `--dest-root` renders full size and
installs it, silently, because `del opts` throws away the scale the caller
passed. It cannot bite through `generate_visual_quality_variants.py` at all. ⇒
So the tier files on any box that ran only the variant script are correct, and
the ratchet must NOT encode this box's stale per-tier render as the expected
state — `KNOWN_UNSCALED` is emptied, and a box with stale renders now fails,
which is the truthful outcome because those files really are wrong there.

⇒ **The renderer fix is still worth making** — `del opts` discards a scale a
caller deliberately passed — and is drafted UNVALIDATED at
`dev/patches/swing-fighter-render-honours-quality-scale-20260902.patch`. It is
in a submodule and needs one render on a box that can run the renderer.

⇒ **WHAT IS AND IS NOT ESTABLISHED.** Established: on this machine's generated
assets, four sheets' 0_5x/0_25x pages and manifests are full-size, and a room
asking for those tiers pays for them. NOT established: that any other machine
generates the same, or that the generator is at fault. ⚠ The arithmetic points
AWAY from a scaling bug — `effective_scale` returns 0.25 for these sheets, the
same as for `carl_runga`, which scales correctly — so the likeliest cause is a
STALE OUTPUT that the freshness check (`_published_and_current`) never rebuilds,
which would be per-machine rather than a repository defect. ▢ The cheap decider
is `scripts/regen/quality_variants.sh --target author --force`: if the variant
comes back smaller, it was staleness; if it comes back full-size, the generator
is wrong for these sheets. That has not been run here, and running it needs the
renderer.

⇒ **A finding about GITIGNORED, GENERATED files is a finding about the generator
only if it reproduces where the generator ran SEPARATELY** — and a symlinked
worktree is not a separate run. That test is the whole reason this paragraph
exists; I applied it and then accepted an answer that could not have failed.

⚠ **AND THE QUANTITY DECIDED THE ANSWER.** Measured by one page's dimensions the
list was SIX, including `noether` and `perfect_cellular_automaton` whose 0_5x
page is taller than their Full page. Those are multi-page packed atlases that
repack per tier; measured by total page megapixels across all pages — what
residency actually pays — they are genuinely smaller and drop out. Six was the
wrong number for a defensible reason.

⛔⛔ **AND THERE IS A SECOND MECHANISM THAT LOOKS NOTHING LIKE THE FIRST.**
`performer` (9.03 MP) publishes **no 0_5x or 0_25x variant at all** — so a room
asking for a reduced tier gets the full page, at the same cost, and comparing
megapixels can never see it because there is nothing to compare against. ⭐ It
is byte-identical to `actor`, which DOES publish both variants and shrinks
neither: **the same artwork fails to get cheaper by two different routes.**
⇒ Five sheets, ~42 MP, that no reduced tier ever makes cheaper. A census
reporting only the first mechanism would have called the tree 4/213 clean when
it is 5/213.

⇒ Ratcheted by `test_tier_variants_are_actually_smaller.py`, both mechanisms:
the named sheets carry their date, a NEW one of either kind fails, and a name
that gets fixed must LEAVE its list or the guard silently permits its
regression. Poison-verified in every direction, with a positive control
asserting the tree really does contain correctly-scaled variants — otherwise
two absence assertions would pass forever on an empty measurement.

⭐⭐ **ONE PROP READS THE SHARED SPRITE PACK. 197 TARGETS ARE IN IT, AND 98.8%
OF ITS BYTES ARE UNREACHABLE — MEASURED 2026-09-02**
(`scripts/measure_pack_reachability.py`). `build_prop_sprite_asset_packed` is
the pack's ONLY production consumer; it has ONE call site, the intro prop loop
in `game/ambition_content/src/intro/plugin.rs`; and it runs only for
`intro_prop_sprite_rows()` entries whose 4th tuple element is `Some(target)`.
**Exactly one row is: `intro_cart`.** Characters have no pack road at all —
`load_character_sprites_in` takes the per-target `*_spritesheet.ron` every time.
On this machine that is **442.6 MB of pack pages of which 5.2 MB sits on a page
any consumer can reach.**

⛔ **THAT INVERTS WHAT THIS ROW USED TO SAY.** It read *"the pack is the
preferred road and the per-target PNG is the fallback"*, inferred from
`extend_with_sprite_pack_entries`' note that a checkout with no packs *"falls
back to its per-target sheet"*. That comment is accurate about the ONE prop and
says nothing about characters, and I read a design intent as a measurement of
adoption. ⇒ Dropping the per-target PNGs would not save 197 sheets' worth of
bytes; it would remove every character's only road to its art.

⚠ **NOT A CLAIM THAT THE PACK IS WRONG.** Packing every target is what a packer
should do. Adoption never followed. Narrowing the generator, adopting the pack
for characters, or dropping the tiers nobody reads are DECISIONS, not
measurements. ⭐ Reachability is a SOURCE fact and reads the same on any
checkout; the megabytes are this machine's generated output.

**SIZES, for context.** The two shipped asset roots hold **1378.7 MB**, of which
the per-target sheets are 649.6 MB across four tiers and `sprite_packs` (the
"ultrapacks") a further **460.6 MB — the single largest category at 33%**.

⛔ **THE PACK COVERS 197 OF 212 PUBLISHED SHEETS.** The 15 absent are
systematically the BIG ones — median **7.54 MP against 0.97 MP** for packed
sheets, and a median largest-frame of 331 px against 176:

```text
gnu_ton_boss / giant_gnu / giant_gnu_hands   14.94 MP each, 768 px frames
actor / performer  9.03    officer 8.64    author 8.36    medic 7.54
mockingbird_boss 2.50   + 6 small ones (sandbag, gnu_ton_apple, …)
```

⭐⭐ **AND ALL FIVE SHEETS WHOSE REDUCED TIERS ARE NOT REDUCED ARE IN THAT 15** —
`actor`, `author`, `medic`, `officer`, `performer`, with none of them packed.
Five of five. That is a strong hint of ONE cause behind two symptoms: whatever
excludes a sheet from the pack pipeline is at least correlated with whatever
leaves its variants unshrunk. ⇒ It is a LEAD, not a diagnosis — 15 are absent
from the pack and only 5 have broken variants, so pack-absence does not imply
the tier defect.

⚠ **AND THE SAME PER-MACHINE CAVEAT APPLIES TO ALL OF IT**: packs and sheets
alike are gitignored generated output, and the clean generation that would
separate staleness from a pipeline defect has not run.

⛔ **THE OPEN QUESTION THIS ROW USED TO CARRY — "does a shipped build need the
per-target PNGs once packs cover a sheet?" — IS ANSWERED ABOVE, AND THE ANSWER
IS THE OPPOSITE OF WHAT THE QUESTION ASSUMED.**

ⓘ It also explains a capture I had misread: `sprites_0_25x/noether_spritesheet.png`
decoding for a character the pack fully covers is not a fallback and not a
defect — it is that character's only road. And the quarter pack's 154 pages have
a median of 0.26 MP, so NOT ONE clears `NOTABLE_MEGAPIXELS` (1.0): the `[image]`
ledger can never show pack usage at that tier, and "both roads loaded in one
run" was never something that capture could tell me.

⚠⚠ **AND THE OCCUPANCY CENSUS BELOW COVERS HALF THE TREE, NOT THE TREE.** It
asks how much of a CLAIMED page is sampled, and says nothing about pages no
manifest claims at all. Re-measured 2026-09-02 across all four tiers: of 2172
PNGs, **1043 are claimed — 49% by megapixels, but 81% by BYTES** (the "44%"
this line used to carry predated the four-tier sweep).

⛔ **THE TWO DENOMINATORS DISAGREE BY FOUR TIMES, AND THE BYTE ONE IS THE ONE
THAT SHIPS.** The unclaimed population is 51% of the tree's megapixels and only
**19% of its bytes** — 120 MB of 630 MB — because stranded pages are
large-dimension and mostly empty, so they compress to almost nothing. A reader
who takes "half the megapixels are unreachable" as "half the package is
recoverable" will be wrong by a factor of four. Megapixels are the right unit
for decode and residency; bytes are the right unit for install size, and this
finding is an install-size finding. `scripts/measure_orphan_shipped_pages.py` measures
those across all four tiers, in **four buckets that do not deserve the same
confidence** — three that carry a reason and one that is an upper bound:

* ⭐ **STRANDED PAGES — 44 files, 92.0 MB.** `<base>_spritesheet.<n>.png` beside
  a manifest that does not name it. A sheet's pages resolve ONLY through its
  manifest, so these are unreachable by construction. Four sheets:
  `pointed_polygon` (20 pages, 43.8 MB), `pugnacious_polygon` (12, 35.8),
  `projectile_polygon` (8, 9.5), `carl_stargan` (4, 2.9). ⓘ All four manifests
  are SINGLE-PAGE (`image: "x.png"`) with no `images:` list at all — I first
  described this as "a list that shrank", which the tree does not show; the
  siblings are left from a time the sheet was multi-page, which makes the
  reachability conclusion stronger, not weaker.
* ⭐ **SHEETS WITH NO MANIFEST — 16 files, 13.6 MB.** `<base>_spritesheet.png`
  with no `<base>_spritesheet.ron` beside it.
  `ambition_sprite_sheet/build.rs::collect_spritesheet_rons` bakes the spec
  index by scanning these four tier dirs for `*_spritesheet.ron`, and every
  loader goes through a spec (`try_load_spec_for_target(target)?`), so a sheet
  with no manifest has no spec and no road. All 16 are four `gnu_ton_boss`
  renders × four tiers: `gnu_ton_boss_full`, `gnu_ton_boss_body`,
  `gnu_ton_boss_hands`, `giant_gnu_body` — `_full`/`_body`/`_hands` layer
  outputs left beside the sheets the boss really uses. ⚠ **THE BOSS IS FINE:**
  `gnu_ton_boss_spritesheet.ron` and `giant_gnu_spritesheet.ron` both exist and
  are claimed. These are extra renders, not missing art — I checked, because
  `attack_geometry/mod.rs` derives its metrics from
  `gnu_ton_boss_spritesheet.ron` and a genuinely absent manifest there would
  have been a content defect rather than waste.
* ⭐ **REDUCED-TIER PORTRAITS — 487 files, 14.2 MB.**
  `bake_portrait_manifests` collects portrait manifests from `assets/sprites`
  ONLY; the reduced tier dirs are never scanned, and the function says why:
  *"Portraits are presentation products and currently have no quality-tier
  variants"*. The generator emits them at all four tiers anyway. Full res is
  164 PNG / 164 RON; the reduced tiers are ~163 PNG against 9, 9 and 0 RON.
  ⛔ **Counting only UNCLAIMED files understates this by 34.** A handful of
  reduced tiers do carry a `_portraits.ron`, which marks the PNG claimed — but
  that `.ron` is never baked either, so claimedness is simply the wrong question
  here. ⇒ This is generator over-production against a stated engine intent, not
  an engine gap.

  ⓘ **The missing manifests are POLICY; the present images are the anomaly.**
  `check_quality_variants_are_fresh.py::absent_variants` already records that
  portraits are *"published SELECTIVELY, so their absence is policy"* — 160
  `_portraits.ron` at full against 9 per reduced tier. The open half is why the
  PNG is generated at a tier whose manifest is deliberately withheld — and why
  the 9 that ARE published per reduced tier cannot be read either:
  `PortraitSheetRegistry` is built `from_baked_table(BAKED_PORTRAIT_RONS)`, and
  `build.rs` bakes from `assets/sprites` only. A deliberate selective
  publication produces files no build can load.

  ⭐⭐ **AND THE NINE NAME FOUR FAMILIAR SHEETS.** They are `actor`, `author`,
  `medic`, `officer` plus five pirates — and `actor`, `author`, `medic`,
  `officer` are FOUR OF THE FIVE sheets whose reduced tiers are not reduced, and
  are among the 15 the pack does not cover. ⇒ A **third** symptom on the same
  four names. ⚠ Still a LEAD, not a diagnosis, and the disconfirming halves
  belong with it: the five pirates carry the portrait symptom without the other
  two, and `performer` carries the tier symptom without this one. If it were one
  cause the sets would coincide, and they do not.
* ⚠ **UNMENTIONED — 300 files, 1.6 MB, UPPER BOUND ONLY.** Named in no manifest
  and in no committed `.rs`/`.ron`/`.ldtk`/`.toml`/`.json`/`.py`. A path
  assembled at runtime (`format!("sprites/{name}.png")`) is named nowhere either
  and would land here while being perfectly live. A research prompt, not a
  delete list — and now small enough that it is no longer where the megabytes
  are.

⭐⭐ **AND AN AGE SIGNAL SPLITS THE THREE INTO TWO DIFFERENT PROBLEMS —
measured 2026-09-02, and it changes what a clean regen can settle.** Each file
is compared against the reference the same run should have written (a stranded
page against its own manifest, an unmanifested sheet against a manifested
sibling, a reduced-tier portrait against its full-resolution twin):

```text
stranded pages          44 older /   0 same-run of  44   median -1.97 d  STALE
sheets without manifest  4 older /  12 same-run of  16   median +0.00 d  still produced
reduced-tier portraits  36 older / 439 same-run of 475   median +3.07 d  still produced
```

⇒ **The 92 MB of stranded pages were left by an earlier render — a clean regen
removes them.** The other 27.8 MB is written by the CURRENT pipeline, in the
same run as the art beside it, so **yardrat's clean generation will reproduce
both buckets and cannot settle them.** Those two need a pipeline change or a
decision, not a regen. ⚠ mtimes are per-machine and a copy rewrites them; the
evidence is the CONTRAST between buckets in one tree with one history, not the
absolute dates.

⇒ **119.8 MB across the three explained buckets, against 1.6 MB still
speculative.** The first pass had 26.7 MB sitting in "named nowhere, but so is
a constructed path"; asking how each KIND of art is actually reached — a sheet
through its baked spec, a portrait through the full-resolution portrait index —
moved 96% of that weight into buckets that carry a reason.

Ratcheted by `scripts/tests/test_shipped_sheet_pages_are_claimed.py` on the
four stranded sheets, so the count cannot grow; poisoned both directions (drop
a name → new-orphan test red; add a fixed name → rot test red). It deletes
nothing and asks for nothing to be deleted.

⛔ **THE COST IS PACKAGE SIZE, NOT RESIDENCY.** No manifest names them, so
nothing decodes them; but `package_asset_guard.py` records *"every regular
file"* from the asset roots, so they ship.

⚠⚠ **AND THIS IS PER-MACHINE UNTIL SOMEONE SEES IT ON A SEPARATELY-GENERATED
TREE.** These are gitignored generated outputs and the likely cause is an
earlier render leaving pages behind — the same staleness class as the tier
variants above. ⛔ Checking the primary worktree does NOT count: its assets are
symlinks at this checkout's, so it is the same bytes. The decider is a clean
generation on another box.

⭐⭐ **PACK OCCUPANCY, MEASURED 2026-09-02 — AND THE ANSWER IS "STOP".** A page is
decoded, uploaded and held resident whole; only the frame rects its baked
manifest names are ever sampled. `scripts/measure_sheet_occupancy.py` reads
every published manifest and its PNG headers and reports what fraction that is,
per page, ranked by waste. Byte-identical across runs:

```text
tier    pages   page MP   sampled MP   occupancy   waste MP
full      225     662.0        595.4         90%       66.6
0_5x      214     182.1        163.4         90%       18.7
0_25x     212      73.4         64.3         88%        9.1
potato    210       3.6          2.4         66%        1.2
```

⇒ **THE PACKING IS ALREADY GOOD AND THIS IS NOT A LEAD.** 90% at Full, and the
10% that is left is DIFFUSE: the top 25 pages of 225 hold only 45% of it, so
there is no handful of sheets to fix and no pipeline change that pays for
itself. The worst single page is `jeff_hinter_armored` at 61% (2.24 MP), and
the largest sheets — `noether`, `giant_gnu`, `gnu_ton_boss` — are already at
89%. ⛔ Recorded so the question is closed with a number rather than re-asked;
if a repack is ever proposed, it is worth ~10% of decode, not a multiple.

⚠ **AND IT IS NOT A PIXEL-QUALITY QUESTION AT ALL.** Repacking would decode
fewer pixels that nothing draws; it would not draw fewer pixels than the
setting asks for. Those are different claims and only the first is supported
here — Jon's ruling on gallery previews governs the second.

⛔⛔ **THE FIRST RUN OF THIS SCRIPT SAID 5% OCCUPANCY AND 447.9 MP OF WASTE**, a
7× error that would have launched a repacking campaign. Two rect shapes exist in
the baked manifests — plain grid `(x, y, w, h)` in 384 of them, and packed
`(x, y, w, h, page: N, off: (dx, dy))` from a trimmed multi-page atlas in 174 —
and a pattern anchored on `h: NNN)` reads NONE of the second. The most tightly
packed sheets in the tree reported **0% occupancy**, which reads as a finding
rather than as a parser failure. ⇒ Caught because zero is not a plausible
measurement, not because the regex looked wrong. An earlier version failed the
OPPOSITE way, counting `body_metrics`' hurtbox/hitbox rects — identical in
shape, in image space — as sampled area, which argues there is no waste at all.
**Neither a low nor a high number is self-evidently right here**, which is why
the script now checks its own premise every run (parsed rects must equal each
sheet's declared `frame_count`; 211 of 211 single-page sheets agree) and
`test_sheet_occupancy_reads_both_rect_shapes.py` pins both shapes, poison-verified
in both directions.


Audit repeated runtime-generated images, portrait/sheet re-loads and per-frame
asset mutation where measurements show repeated work. Retain the semantic handle
when an asset is intentionally resident; compare before writing materials/assets
so unchanged values do not trigger uploads.

⭐ **THE INSTRUMENT EXISTS AND HAD NEVER BEEN RUN (2026-09-02).**
`ImageStageLedger::inserted` has been counting `re_decodes` all along — it bumps
whenever a path is inserted a second time — and `asset_census` already prints the
total. Nothing had ever run that counter over the Hall entry, the one transition
big enough for a repeat to cost anything.
`scripts/measure_hall_redecodes.sh` does, driving
`hall_redecode_census::the_halls_entry_is_counted_for_art_it_decodes_twice`.
⛔ It is `#[ignore]`d ON PURPOSE and must stay that way. `ambition_app` has ONE
`[[test]]` target, so every file under `tests/` is a module of `app_it` sharing a
process, and cargo runs those as parallel threads; the ledger is a process-global
`static`. The test's before/after delta already excludes everything that ran
EARLIER in that process — only concurrency is left, and the only fix for that is
to run the test alone, which is what the script does. Un-ignoring it would make
it flaky rather than red.
⭐⭐ **RE-MEASURED 2026-09-02 ON A REAL POPULATION, AND THE ANSWER IS STILL
ZERO.** `124684f56` made the no-window builder finish its plugins, so the road
below finally decodes file-backed art. `scripts/measure_hall_redecodes.sh` now
reads: **237 images resident, 67.6 MP, 270.4 MB, of which 213 arrived through a
demand road; re-decodes 0.** That is the first time this census has had art in
it — the correction below explains why every earlier reading did not.
⇒ **The Hall entry decodes nothing twice on the headless road.** The host number
is still owed (`AMBITION_PROFILE_CENSUS=1`, read `re-decodes N`), and only the
host has the 434 MP; but the shape of the answer is no longer unknown.
⛔ `dropped before gpu 0` on this road is STRUCTURAL, not a finding: headless has
no render world, so `awaiting gpu` holds all 237 and nothing can be dropped
*before* a GPU that never looks. Do not quote it as evidence of no waste.
⛔⛔ **AND ON THIS BOX, READ THE COUNTS AND NOT THE CLOCK.** Five runs of that
script across the 2026-09-02 merges gave byte-identical counts every time —
`0 re-decodes / 237 resident / 67.6 MP / 213 routed / 126 newly staged` — and
frame-spike totals of **61, 4, 9, 6, 52** for the same five runs, at load average
7–12 with two other agents compiling. ⇒ A timing reading taken here is a reading
of who else was building. I nearly recorded the 61→4 pair as a 15× improvement
from the first-room-art prefetch; re-running three times is what stopped it, and
a single before/after would have published it. The counts are deterministic and
are what this environment can honestly measure; the wall-clock rows in this
document that came from llvmpipe runs should be read with the same caution, and
the host is the only place a timing claim settles.
⛔⛔ **AND THE `UNROUTED` BUCKET WAS 24 NON-FINDINGS. Split 2026-09-02
(`a20b5b1a2`).** The same census read `UNROUTED(no demand) 24×4.5MP`, which reads
as *"something loaded art and no road said so"* — 24 times. Every one of the 24
has **no path**: they came from no file at all, inserted directly (render
targets, procedural sprites, shader inputs), and an image with no load can never
acquire a demand stamp. `resident_by_road` keyed both kinds `"?"`. Two keys now
(`ROAD_UNROUTED`, `ROAD_PROCEDURAL`), and the census NAMES the first on an
`[image-unrouted]` line with paths — because on the host the one that matters
(the 7.6 MP editor-preview tileset below) would otherwise have been the 25th
entry in a bucket of noise. Unit-tested and poison-verified in
`ambition_asset_manager`; ⛔ that crate's `image_stages` is behind the `bevy`
feature, so `cargo test -p ambition_asset_manager --lib` runs NONE of those
tests and reports 56 green.

<details><summary>The earlier reading, and why it was empty — kept because the correction is the lesson</summary>

⛔⛔ **MEASURED: 0 re-decodes — AND THE NUMBER IS EMPTY, NOT SMALL. CORRECTED
2026-09-02, same day, twice.** The first write-up of this said the headless run
decoded "22 images / 4.5 MP, ~5% of the host's 434 MP", and treated that as a
small-but-real population. It is not a small population, it is NO population:
all 22 are keyed `"?"` in `resident_by_road`, meaning `source == None` — images
that reached `Assets<Image>` without passing a stamped demand road, i.e.
inserted directly rather than decoded from a file. **Routed images: 0.**
⭐ THE CAUSE, found by `df`: `ImagePlugin` registered the image loader in
`Plugin::finish`, and `finish()` did not run under the `app.update()` loop a
`NoWindow` composition uses. ⇒ **No file-backed art decoded on that road at
all** — not 5% of it.
✔ **FIXED THE SAME DAY at `124684f56`** (the NoWindow builder finishes its
plugins). Re-measured through the same script: **225 images resident, 60.5 MP,
201 of them through a demand road**, 0 re-decodes, over a 126-character Hall
entry. The premise guard is green on a real population and the census finally
measures the thing it is named for.
⚠ The population is NOT fixed run to run — 225/201 here, 237/213 on another run
the same day — because how much lands inside the frame budget varies. The guard
is a THRESHOLD for that reason and must stay one.
⇒ The test's premise guard now asks the question that matters — did ANY resident
image arrive through a demand road — and **it fails today**, naming the reason.
That is the correct outcome: a re-decode census over a population with no art in
it cannot answer anything, and a guard that passed on 22 unrouted images was
reporting a number it never had.

</details>

⛔ **THE FIRST DRAFT OF THE TEST WOULD HAVE REPORTED THAT ZERO CONFIDENTLY.** Its
only premise guard was the staged-cast size — but staging is a DEMAND and
`re_decodes` counts INSERTIONS, so the guard and the measurement were watching
different things and a run that decoded nothing at all would have passed. A
second premise now pins the decoded population, and it is poison-verified (raise
it above the reading and the test fails naming the count).
⭐ **THE HOST TELL, since only a host run can answer this**: boot with
`AMBITION_PROFILE_CENSUS=1` and read `re-decodes N` from the `[image-census]`
line. That number over a real Hall entry is the measurement this section wants.

⛔⛔ **AND THE "NOT MATERIALIZED" WARNING ASSERTS A CAUSE THE TYPE SAYS IT CANNOT
KNOW.** `CharacterSheetState::Declared`'s own doc names TWO causes — *"either it
never has, or its realization was retired by a quality change"* — and the warning
in `ambition_render/src/rendering/actors/mod.rs:721` reports only the first:
*"nothing demanded it, so the engine never decoded its sheet"*. That is the
warning the host run saw 111 times on the Hall reveal, so its diagnosis is
evidence for a cause it never checked.
The two are not distinguishable today: `demote_stale_realizations_outside`
removes the token from `sheets` and deliberately leaves `declared` intact (the
entry is the recipe for re-making it), so a retired realization and one that was
never made are the SAME state, and nothing records that a retirement happened.
⇒ A retirement followed by a re-demand IS accidental re-preparation, which puts
this squarely in this section rather than in observability.
✔ **FIXED THE SAME DAY.** `CharacterSpriteAssets` now keeps a `retired` trace
(token → the tier whose pixels it actually held), written by
`demote_stale_realizations_outside` and cleared by both publish paths the moment
the token is resident again — so a character that comes back stops being
described by a retirement it recovered from. `retired_tier(token)` exposes it and
the warning now says either *"declared as 'X' and RETIRED from Full — it was
decoded and then dropped by a quality transition, so this is a re-realization
that has not happened yet"* or *"declared as 'X' but never materialized"*.
⚠ It is a TRACE, not state anything decides on: nothing reads it to choose what
to load, and `None` is also the answer for an undeclared token, since only
declared tokens are ever retired. Guarded by two arms in
`quality_convergence_tests` — one proving the two causes are distinguishable
(with a never-realized character alongside, so it cannot pass by reporting a
retirement for everything), one proving re-realizing clears the trace under BOTH
keys the double-keyed table holds. Poison-verified separately: dropping the
record kills both arms, dropping the clear kills only the second.
⚠ **Still unmeasured on the host**: this makes the 111-warning run *answerable*,
not answered. The next host Hall entry will say how many of those 111 were
retirements rather than first loads, and that is the number this section wants.

⭐ **AND THE THIRD CANDIDATE IS ELIMINATED: PREFETCH SCOPE IS NOT THE CAUSE.**
The 111 warnings had three candidate explanations — prefetch scope, retired
realizations, re-decode. Measured 2026-09-02 by
`hall_transition_cover::every_character_the_hall_places_is_reached_by_its_demand`:
**every one of the hall's placed characters is reached by a demand.** The test
asks `CharacterLoadStates::outcome(id).is_none()`, which is the scope question
exactly — any outcome at all (Ready, pending, even failed) means the demand
reached it, while `None` means nothing ever asked and no amount of waiting helps.
Poison-verified with a fabricated id, which it names.
⚠ The room authors **129 `NpcSpawn` placements with 129 DISTINCT `character_id`s
and no duplicates** (counted from `hall_of_characters.ldtk`), so the shortfall a
count could show would be a missing character rather than a deduplicated one.
⇒ This also settles the "126 newly staged of 129 authored" reading from the
re-decode census above: with all 129 reached, the three are characters the HUB
had already staged before the transition, not three the hall failed to ask for.
⇒ Of the three candidates, **scope is out** (measured), **re-decode is out on
the headless road** (0 repeats over a real population since `124684f56`: 237
resident / 213 routed on one run, 225 / 201 on another — the population varies
with what lands inside the frame budget, so the guard is a threshold; the
earlier "~5% of the art" reading was wrong and the "no art decodes at all"
correction to it is now also superseded), and **retired realizations are the
one candidate left standing** — distinguishable since the same day, but only a
host run can count how many of the 111 they were. That is the single
measurement this section is still waiting on, and it is one boot with
`AMBITION_PROFILE_CENSUS=1`.

### 6. Live quality switching

✔ **The reverse leg is measured and half-repaired (2026-09-02).** Leaving the
gallery (Quarter) for the hub (Full) is now a test in the shipped composition,
`leaving_the_gallery_keeps_the_shared_cast_and_retires_the_rest`, and its first
run found three defects on the way out: (1) the per-token tier a transition
forwards was CLAMPED to the active room's cap in `materialize_character_demand`
(`floor.min(budget)`), so the hub's cast was re-demanded at Full and decoded
at Quarter again — fixed, the token's floor is used as given; (2) the
transition re-demanded EVERY retired sheet, i.e. the whole gallery cast at
Full for a room that places five of them — fixed, only the destination's
tokens and the player population's worn characters are re-demanded, the rest
stay retired; (3) three mary_o demo systems (`register_snakes_on_a_plane_
sheets`, `register_solid_snake_sheet`, `register_ai_slop_sheet`) re-published
their sheets under display names at full resolution whenever the engine
retired them — a permanent fight with tier convergence — now gated on the
engine not having declared the character. ✔ And the COMMIT-time half, the
same day: `converge_character_residency_to_active_quality` still retires every
below-floor sheet once the hub is active, but re-demands only the characters
STILL IN USE — worn by a body, or named by a live actor's config — and leaves
the rest retired (declared, so the next room that places one demands it then).
Before, `demand.request_all(stale)` decoded the gallery's ~125 Quarter sheets
at Full into a room that placed five, after the reveal, in the open. Guard:
`apply_re_decodes_only_the_characters_still_in_use` (two Half sheets, one worn;
Apply High re-realizes the worn one and leaves the other retired; red with the
filter removed). The quality-Apply fixtures now put a BODY on the character
they converge, which was their claim all along. Host tell after a hall exit:
no `[image]` line for a hall-only character with `live=1`.

Quality changes should re-tier the same logical asset and converge predictably in
both directions. ✔ The round trip is demonstrated headless with real decode
(`a_quality_round_trip_converges_back_with_every_page_loaded_and_nothing_orphaned`,
2026-09-02): Full → Potato → Full in the direct host; each leg is judged only
once the worn sheet's PAGES are loaded, every token resolves to the sheet it
started with, and no character page is left resident without a realization
(poison: three Full pages held across the drop are named). Keep the reported
live quality-switch issue attached until a RENDERED session shows the same —
the GPU half (old tier's `GpuImage` released, new tier's prepared before the
swap draws) is not observable here.

### 7. Load/readiness semantics

Required readiness is a semantic contract, not a percentage bar. A session may
commit when its required prepared work is ready; degradable presentation work can
remain explicit and continue resolving afterward.

✔ **"Ready" now includes the GPU copy (2026-09-02).** `inspect_room_asset_manifest`
— the one readiness function both the room transition and the direct startup
cover consult — holds a decoded page as `pending` under the label
`<page> (gpu upload)` while a render world exists that has not yet prepared
it (`ImageStageLedger::is_awaiting_gpu` — POSITIVE proof since `8bd19f890`:
the GPU stamp has landed, not "the id is not in the awaiting list", because
the insertion stamp runs in `Last` while readiness polls in `Update`, and the
old reading called every page ready on the frame it landed — the one frame a
paced upload could slip past the cover). That converts the upload of a room's
cast from the first frame AFTER the cover lifts (measured: every sheet of the
hall's reveal in one render frame) into cover time, and it is what makes a
byte-per-frame upload budget safe to adopt: paced uploads extend the cover by
a few frames instead of popping sprites in after it. Headless and `NoWindow`
compositions have no render world and the term is always false — a reveal
never waits on a GPU it does not have (unit-tested both ways). The startup
cover prints `[startup-cover] revealed after N updates (M of them waiting only
on GPU uploads)`; the room transition's `asset_wait_ms` now includes the wait.
⚠ Still a decode-plus-upload metric, and a manifest never names a
`MAIN_WORLD`-only image (which no render world would ever prepare) — if one
ever does, the label says which.
⭐ THE FIRST DRAW IS STAMPED NOW (the fourth stage, same day) — and readiness
must still NOT consult it. A cover exists so nothing is drawn until the room
is ready; a barrier that waited for a draw would wait for the thing it is
holding back. The stage answers *"was this decode ever used"* after the fact,
which is a residency question, not a readiness one.

⛔⛔ **Until 2026-09-02 no image ever decoded in a headless composition, and
every readiness claim measured there was a claim about the table.** Bevy
registers the `ImageLoader` in `ImagePlugin::finish`, and `App::update()`
never calls `finish` — only a runner does — so in every `NoWindow` app (the
whole `app_it` suite and `--headless`) every `Handle<Image>` sat in
`LoadState::Loading` forever: no room barrier ever opened, the hall → hub test
was still in the hall with the cover up after 300 frames, and `headless_room_
frame.sh` timed a hall with no art in it. `build_visible_app_with` now
finishes and cleans up the plugins for `NoWindow` (`124684f56`); the headless
host decodes 232 images / 57 MP entering the hall, the hall barrier settles
in ~8 frames of game time with 129/129 realized, and the re-decode census
counts 213 of 237 resident images through demand roads. Two tests had been
built on the break — the cover test waited 600 frames for a stall report that
a settling barrier never files — and were rewritten to the contract they
meant. ⚠ Still not observable headless: GPU upload and first draw (no render
world); those remain `capture_scene` / host measurements.

## Explicit non-goals

Do not yet build:

- a universal LRU cache;
- a second asset catalog/registry;
- a custom renderer to solve an asset-demand problem;
- global eager loading of the entire product;
- a decode-only metric and call it readiness;
- a fixed pacing number without a rendered validation case.

## Exit for the current architecture slice

This program has reached a stable first plateau when:

1. critical asset demand is raised from semantic composition before first visible
   use — ✔ for a room transition (the placed cast, `2c8f27b32`), for the direct
   start (`startup_loading`, worn included) and for the shell route's first room
   (`prepare-first-room-art`, `aca57e636`); ⚠ the neighbour prefetch is demand
   AFTER a reveal by design, one ration per neighbour;
2. stage-specific telemetry identifies whether a hitch is IO, decode, asset
   insertion or render/device preparation — ✔ four stages on one ledger
   (demand → insert → GPU → first draw, `image_stages`), with IO and decode
   still one stage (Bevy's loader does both on the IO pool);
3. one representative uncovered gameplay case demonstrates bounded
   materialization without a large completion burst — ◐ the ration is areal
   and the prefetch takes one per neighbour, but the UNCOVERED burst was only
   ever measured on the host before the fixes (the 434 MP hall arrival); the
   post-fix uncovered case needs the host walk;
4. residency ownership/scopes are explicit enough to explain why a retained
   image remains live — ✔ for character and fx pages (a resident page is owned
   by a realization, guarded on the hall exit and on a quality round trip; two
   laps return the same working set); open for UI/prop art and for the BUDGET
   (the 5.8× never-drawn headroom is the number a policy would spend);
5. quality switching preserves logical identity and round-trips in a rendered
   session — ✔ headless with real decode (`a1c03c179`); the rendered half (old
   tier's `GpuImage` released, new one prepared before the swap draws) is a
   host observation;
6. no new global cache duplicates domain/catalog ownership — ✔ no cache was
   added; ⚠ the image-stage LEDGER is a process-global instrument, which is
   the right shape for one shipped App and the wrong one for a test process
   with many (it is why the residency guards read `Assets<Image>` per App and
   use the ledger only to classify a path).
