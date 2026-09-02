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
budgeting a residency are both cheaper when the demand is smaller.** Fewer pages,
a lower quality tier for gallery previews, or eviction all attack the 43% before
any scheduling machinery has to.

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
when the remainder is dropped. **Host tell, still owed:** zero "nothing demanded
it" warnings at the hall reveal, `asset_wait_ms` in the seconds (129 frames of
ration at least), no >33 ms frames after the cover lifts. The hitches now happen
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
and prop pngs. The ledger's road names cover CONTENT ART decoded at runtime —
`character-sheet`, `portrait`, `projectile-art`, `parallax`, `fx-sheet`,
`boss-sheet`, `vanity-card` — because that is the population the hall-entry hitch
is about. UI chrome is small, loaded once and not what a room's reveal waits on;
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
(Bevy's loader does both on the IO pool and `Added` fires after); "resident
use" (first draw) is not stamped; an image demanded by a road that calls
neither funnel prints `demand=unknown` — in the hall that is exactly one,
`game://sprites/player_robot_v3_spritesheet.png`: found 2026-09-02, it is
`bevy_ecs_ldtk` loading the four worlds' editor-preview tileset (`relPath:
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

⛔⛔ **AND `PROCEDURAL` IS NEVER A FINDING ON THAT ROW.** The stage is stamped
from `ExtractedSprites`, and a render target, a shader input or a material
texture is never a sprite — it is written to or sampled, not extracted. Those
rows are permanently "never drawn" BY CONSTRUCTION, so the 4-6 MP the bucket
reports in a hall capture is the instrument describing its own blind spot, not an
asset problem. Only the file-backed roads answer a residency question here.

⛔⛔ **AND THE ROW PRINTS `-`, NOT `0`, WITHOUT A RENDER WORLD.** With nothing
extracted, EVERY resident image is "never drawn" — which on a headless road means
nobody could have drawn anything, not that the pixels were wasted.
`render_world_present()` is the fact that separates the two readings and the row
consults it; a readout that skipped that check would accuse a `NoWindow` run of
waste it cannot commit. Both rules are guarded on the pure ledger and
poison-verified (drop first-write-wins and the guard names the instant that
moved).

⭐⭐ **MEASURED 2026-09-02, `capture_scene hall_of_characters player --warmup 400`
(llvmpipe, OffscreenGpu), byte-identical across two runs:**

```text
total 239 images, 30.7MP, 122.9MB resident | never drawn 195 (25.4MP)
resident by road: UNROUTED(no demand) 9×7.9MP, PROCEDURAL(no file) 26×4.6MP,
                  character-sheet 138×4.2MP, fx-sheet 41×9.6MP,
                  parallax 4×2.4MP, boss-sheet 1×2.0MP, held-item 20×0.1MP
```

⇒ **44 of 239 images are drawn, and they are 5.3 of the 30.7 MP.** Eighty-three
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
[image-unrouted] 9 file(s) decoded with no demand stamp:
  7.6MP game://sprites/player_robot_v3_spritesheet.png   <- the editor-preview tileset
  0.3MP sprites/shrine_spritesheet.png                    <- NEW, not previously known
  0.0MP game://sprites/intro_lab_tileset.png
  0.0MP sprites/props/portal_gun_blue.png                 <- NEW
  0.0MP sprites/props/portal_gun_orange.png               <- NEW
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
the FX set is 41 sheets / 9.6 MP, loaded at boot, in every room.**
`load_game_assets` calls `load_fx_sheets` unconditionally, so the engine's whole
effect vocabulary is resident whether or not anything ever plays one — 31% of the
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

⛔ **AND IT IS 13 SHEETS ACROSS 41 PAGES, not 41 sheets** — the census counts
IMAGES. Corrected here because the first write-up of this row said 41 sheets.

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
residency can say is the size: 9.6 MP resident in every room, of which 76 of 196
rows are art nothing can currently ask for.
⚠ A string search cannot see a name built at runtime (`format!`), so a row it
calls unnamed could in principle be constructed. The five fully-named sheets are
the evidence the method works; treat a single unnamed row as a lead, not a
verdict.

### 3. Pace expensive completion, not declarations

Staging/demand and expensive materialization are different operations. Declare
all required work promptly, then pace only the stage whose burst cost is
measured.

Choose a budget from rendered measurements. "One character per frame" is a
current useful bound, not a universal theorem.

### 3a. ✔ LANDED 2026-09-02 (`dc3cd0d91`): the ROOM-LEVEL sprite tier cap

Built as planned below with two departures worth knowing. (1) No new authored
field yet: the cap derives from the existing authored `gallery` flag
(`room_sprite_tier_cap`), which is every pedestal room today; the derivation
is one function and an authored field can replace it. (2) Staleness became a
RANGE `(floor, ceiling)`, not a single tier: a Full sheet standing in a
gallery is kept (oversampled), a Quarter sheet carried into a Full room is
retired before that room's reveal — so entering the hall loads only NEW
characters, at Quarter, and hub-shared ones are not churned. The demand now
carries a per-token tier and the convergence knows the room being loaded
(`PendingRoomTierFloor`); without those the in-room drain realized the
forwarded cast at the hub's Full and the convergence retired the Quarter ones
the frame after they arrived (measured headless: 103 Full behind the cover).
Headless: 124 new sheets at Quarter, 5 hub-shared kept at Full. The
materialization ration became AREAL in the same day (`d19acd516`): one Full
character's worth of pixels per frame, so the Quarter cast starts sixteen per
frame and the hall's cover holds ~9 frames for its 129 sheets rather than 129
(item 3 below, "choose a budget from rendered measurements", now has a unit —
the Full-sheet bound that was measured — instead of a head count). **Host tell:**
`image_arrivals` megapixels in the hall window (434 before), `resident_mb`
(2153 before), and the cover's hold time.

#### The plan as written (kept for the record)

Now that the frame is answered (250-310 fps everywhere; the hall's entry hitch
is the one user-visible cost left), this is the first thing to build. The
pedestal measurement above says WHY (132 px drawn, 496 px loaded); the honest
run says HOW MUCH (434 MP in two seconds, nine frames of 89-355 ms).

Shape: the drawn size of a character is a property of the ROOM's camera
framing, so the cap is authored on the room, not derived per sprite.

1. `RoomMetadata` gains `sprite_tier_cap: Option<TextureResolutionScale>`
   (schema registers in both compositions — see the content-schema rule).
2. `hall_of_characters` authors `Quarter`. Nothing else changes; a room with
   no cap keeps today's behaviour exactly.
3. The effective tier is `min(settings tier, room cap)` at the ONE seam:
   `character_sprite_tier(budget)` gains the room cap as an input, and both
   callers (`converge_character_residency_to_active_quality`,
   `materialize_demanded_character_sheets`) pass the active room's. Entering
   or leaving the hall is then the existing "quality transition" path — demote
   stale realizations, re-demand — so no new lifecycle is invented.
4. Missing variants: `performer`, `actor`, `medic` have no Quarter; fall to
   the nearest tier that exists and say so once per character (the freshness
   checker already reports absence).
5. Measure with the same capture (`--no-tracy`, V-Sync Off, walk into the
   hall): `image_arrivals` megapixels in the hall window (434 today), the
   spike list (nine frames over 89 ms today), `resident_mb` (2153 today).
   Expected: ~11x fewer megapixels, hitches under the 33 ms line or gone.

What it does NOT fix: the boot's 504 ms frame and portal_lab's 123 ms entry,
which are different rooms' art; those go through item 3 above (pace the
upload) or the same cap once measured.

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
(`leaving_the_gallery_re_tiers_the_shared_cast_up_to_the_setting`, hall →
hub, 300 frames after the commit): **0 orphan pages**; the retired gallery
cast leaves memory. Poison: holding five gallery page handles across the exit
names exactly those five. What stays resident in the hub and why: the hub's
placed cast and the worn character (their realizations), the one-hop
neighbours' casts the prefetch realized at THEIR tier (`basement_enemies`
spawns an "Ai Slop", which is why `npc_ai_slop` comes back at Full — for the
basement, not for nobody), the fx set, and 24 images on no demand road (4.5 MP).
⚠ THAT LAST FIGURE WAS TWO POPULATIONS IN ONE BUCKET and is superseded: split on
2026-09-02 (`a20b5b1a2`), the hall reads `UNROUTED(no demand) 9×7.9MP` — real
findings, now named on an `[image-unrouted]` line — and `PROCEDURAL(no file)
26×4.6MP`, which can never carry a road because there is no load to stamp. ✔ **Working-set GROWTH measured the same evening**
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

### 5. Eliminate accidental re-preparation/reload

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
`leaving_the_gallery_re_tiers_the_shared_cast_up_to_the_setting`, and its first
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
