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
And **13 of 229 sheets have no Quarter twin** (216 vs 229), so a per-use tier
needs a defined answer for a missing variant rather than a silent fall back to
Full.

⚠ This is a DEMAND measurement — megapixels asked for — not a hitch measurement.
The frame cost of materializing them is a GPU-upload question and this machine
rasterises in software; that half still needs real hardware.

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
