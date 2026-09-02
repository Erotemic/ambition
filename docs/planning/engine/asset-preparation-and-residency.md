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
now realize one per frame behind the cover (stalled at 6 before); the guard
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
  the MAIN thread in the earlier capture) is a CLONE of the decoded bytes,
  because every sheet is loaded with the default
  `RenderAssetUsages::MAIN_WORLD | RENDER_WORLD` — which also keeps the CPU copy
  alive, which is where the 2.2 GB comes from. `RENDER_WORLD` alone makes the
  extract a move and frees the CPU copy — but then `Assets<Image>::get` returns
  `None` for that sheet, and whoever reads a sheet's pixels or size from the
  main world (sprite sizing, portraits, the image census) must be found first.
  Grep `Assets<Image>` readers before flipping it.

**Both are now one environment variable away, so Jon can measure them on the
3090 without a code change** (each is read once and recorded on the
`[census] visual_quality` row and in the ledger label, so a capture says which
way it was taken):

```sh
AMBITION_RENDER_ASSET_MB_PER_FRAME=64 scripts/profile_desktop.sh --no-tracy    # +upload:64MB
AMBITION_IMAGES_RENDER_WORLD_ONLY=1   scripts/profile_desktop.sh --no-tracy    # +render-world-only
```

The second routes every sheet/parallax/boss-page load through
`ambition_sprite_sheet::game_assets::load_sheet_image`; the main-world readers
of `Assets<Image>` were checked first — `texture_is_ready` and the room
transition's dependency check use the asset server's load state and fall back
to residency only for handles without one, and the remaining readers are the
census (an instrument, which will count fewer resident images) and procedural
images (portal cones, quasar shader, touch overlay) that are not loaded this way.
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

Not covered, honestly reported: source IO and decode are one stage here
(Bevy's loader does both on the IO pool and `Added` fires after); "resident
use" (first draw) is not stamped; an image demanded by a road that calls
neither funnel prints `demand=unknown` — in the hall that is exactly one,
the player's own `game://sprites/player_robot_v3_spritesheet.png`, whose load
site is still to be found and routed. `[image-gpu]` lines only appear with a
render world; headless runs show `awaiting gpu` growing instead, which is the
readout saying nobody uploaded.

### 2. Demand before first visible use

Where semantic composition already knows the roster/room/UI assets, raise demand
there rather than from `Added<ActorConfig>` or another first-use event.

Do not prefetch every asset in the product. Demand should follow the prepared
composition and the expected near-term experience/room.

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

### 5. Eliminate accidental re-preparation/reload

Audit repeated runtime-generated images, portrait/sheet re-loads and per-frame
asset mutation where measurements show repeated work. Retain the semantic handle
when an asset is intentionally resident; compare before writing materials/assets
so unchanged values do not trigger uploads.

### 6. Live quality switching

Quality changes should re-tier the same logical asset and converge predictably in
both directions. Keep the currently reported live quality-switch issue attached
to this program until a real rendered session demonstrates the round trip.

### 7. Load/readiness semantics

Required readiness is a semantic contract, not a percentage bar. A session may
commit when its required prepared work is ready; degradable presentation work can
remain explicit and continue resolving afterward.

✔ **"Ready" now includes the GPU copy (2026-09-02).** `inspect_room_asset_manifest`
— the one readiness function both the room transition and the direct startup
cover consult — holds a decoded page as `pending` under the label
`<page> (gpu upload)` while a render world exists that has not yet prepared
it (`ImageStageLedger::is_awaiting_gpu`). That converts the upload of a room's
cast from the first frame AFTER the cover lifts (measured: every sheet of the
hall's reveal in one render frame) into cover time, and it is what makes a
byte-per-frame upload budget safe to adopt: paced uploads extend the cover by
a few frames instead of popping sprites in after it. Headless and `NoWindow`
compositions have no render world and the term is always false — a reveal
never waits on a GPU it does not have (unit-tested both ways). The startup
cover prints `[startup-cover] revealed after N updates (M of them waiting only
on GPU uploads)`; the room transition's `asset_wait_ms` now includes the wait.
⚠ Still a decode-plus-upload metric, not "resident use": nothing stamps the
first draw, and a manifest never names a `MAIN_WORLD`-only image (which no
render world would ever prepare) — if one ever does, the label says which.

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
   use;
2. stage-specific telemetry identifies whether a hitch is IO, decode, asset
   insertion or render/device preparation;
3. one representative uncovered gameplay case demonstrates bounded
   materialization without a large completion burst;
4. residency ownership/scopes are explicit enough to explain why a retained
   image remains live;
5. quality switching preserves logical identity and round-trips in a rendered
   session;
6. no new global cache duplicates domain/catalog ownership.
