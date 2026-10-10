# Asset preparation, device materialization and residency

**State:** OPEN — the lifecycle model and quality authority are established.
Current work is stage-specific observability, demand timing, pacing of expensive
completion, explicit residency ownership, and robust live-quality/readiness
semantics.

This page owns the **current asset lifecycle contract and open packets**.

Performance conclusions that span the whole frame belong in
[`performance-and-iteration.md`](performance-and-iteration.md).

## Goal

A visible asset should be requested early enough, prepared/materialized under one
quality authority, and resident when needed without turning asset readiness into
simulation authority.

The engine must distinguish:

```text
DECLARED / DEMANDED
    semantic content says an asset will be needed

CPU PREPARED
    source bytes/metadata are decoded and usable for materialization

DEVICE MATERIALIZED
    GPU/provider resources exist

RESIDENT USE
    the prepared/materialized representation is actually used by a visible
    presentation entity / first draw
```

Do not collapse these stages into one boolean called “loaded.”

## Authority contract

### Content declares demand

Content/domain systems identify semantic assets. Presentation/asset services
resolve those identities to concrete representations.

A room or character should not hand-write filesystem paths in every consumer.

The demand ledger (`ambition_asset_manager::image_stages`) stamps a road name on
every content-art demand. The vocabulary, every string literal reaching
`note_demand` / `load_sheet_image` / `load_sprite_pages`:

```text
asset-manifest   boss-sheet   character-parts   character-sheet   entity-sprite
fx-sheet         held-item    parallax          portrait          projectile-art
room-dressing    shrine-sheet vanity-card
```

**THIRTEEN live roads.** `room-dressing` (the terrain skin, motes and decor of
a room's theme, `RoomDressingSet`) is the thirteenth, 2026-10-10. `character-parts` (a part flipbook's pages,
`RIGGED_SPRITE_ROAD`) was stamped all along through a constant the scan does not
read; it surfaced when the pages moved onto `load_sheet_image` (2026-10-05). Derived from the call sites by
`scripts/tests/test_demand_road_vocabulary_is_derived.py`; a road added in code
without a row here is red there. Menu icons, shell images and prop pngs are
deliberately not stamped.

### Quality has one authority

Every quality-aware materialization road consumes the active shared quality
budget/tier selection.

FX-sheet loading resolves its quality variant through the same tier authority.

A deliberately full-resolution narrow loader may opt out only with that policy
stated at the call site.

### Quality transitions are swaps

Changing quality requests a replacement representation and retires the old one
when the replacement is ready. It is not a “demote the existing asset in place”
operation.

Avoid transient states where a body loses its only usable representation while a
new tier is still materializing.

### Demand, readiness and simulation are separate

Simulation must not become dependent on whether a texture/material finished
preparing on this machine.

Readiness may gate visible presentation or a transition whose contract explicitly
requires presentation readiness. It must not change deterministic gameplay
outcome.

### Residency has an owner

A cache/resident asset remains because some declared owner/budget keeps it. “It
happened to be loaded earlier” is not a residency policy.

When eviction becomes necessary, the policy must name:

- owner/scope;
- budget/pressure signal;
- re-demand behavior;
- minimum representation/readability requirements.

### Products are laid out by what they are (Q82, Q83)

Rulings 2026-10-04 ([`../maintainer-decisions.md`](../maintainer-decisions.md)):
the directory and product layout says what an asset is (source/editor
artifact, runtime product, quality tier, generated intermediate), so a
packager works over meaningful roots instead of a long exclusion list. An
editor-only product that the runtime never consumes is not packaged or
resident, and it does not live where it can be taken for a shipping product.
Requesting one runtime product does not admit hundreds of MB of unrelated
products unless they are one runtime unit; this is a dependency/residency
rule, not a "one consumer" rule. A large shared SOURCE pack may stay; the
runtime boundary is what splits.

**Measured 2026-10-04 (queue row ASSET-PRODUCT-LAYOUT).**

- Tier layout is not symmetric. Under
  `crates/ambition_platformer2d_actor_monolith/assets/`, full quality is the
  bare root `sprites/` (300 MB) and the lower tiers are suffixed sibling roots
  `sprites_0_5x/`, `sprites_0_25x/`, `sprites_potato/`. Parallax does the same
  (`backgrounds/parallax_layers{,_0_5x,_0_25x,_potato}`). Only the ultrapack
  names every tier: `sprite_packs/{full,half,quarter,potato}/`. The code
  vocabulary is `TextureResolutionScale {Full, Half, Quarter, Potato}`
  (`ambition_persistence/src/settings/video/quality.rs`, `folder_suffix` is
  `""` for `Full`) and the pack tier names `full/half/quarter/potato`
  (`ambition_sprite_sheet/src/sprite_packs.rs`). Target: one runtime root with
  `full/`, `half/`, `quarter/`, `potato/` children for each product family.
  Path builders to move together: `scaled_logical_asset_path`
  (`ambition_asset_manager/src/platformer_assets/mod.rs`),
  `RUNTIME_SPRITE_ROOTS` (`asset_publish/mod.rs`), `builders/visuals.rs`,
  `ambition_sprite_sheet/build.rs` and `src/boss.rs`, `entity_sprite.rs`,
  `character_sprites/assets.rs` (`resolve_variant_pair`,
  `character_sprite_tier`), `scripts/generate_visual_quality_variants.py`,
  `scripts/regen/sprites.sh`, `scripts/package_asset_guard.py`. Audit each
  consumer before you move a file.
- No source/editor root exists inside the asset tree;
  `package_asset_guard.py` ships every regular file except dotfiles, `*.ipfs`
  and `fonts/local/**`. The LDtk preview case is narrower than its question
  said: the player tileset `sprite_player_robot_v3` (used only by the
  `PlayerStart` entity's editor tile in seven worlds) is the player's runtime
  sheet, so the file ships anyway; the waste is the extra full-resolution
  decode `bevy_ecs_ldtk` does at boot (`[image-unrouted]`,
  `ambition_platformer2d_host/src/portal.rs`).
  `dev/patches/ldtk-player-tileset-retarget-20260902.patch` retargets that
  editor tile to `sprites_0_25x/`; it puts an editor reference into a runtime
  tier, so prefer an editor product under an editor root. Mary-O's
  `ldtk_editor_art_mary_o.png` feeds auto-layers and may draw at runtime
  (unverified); classify it before you move it.
- The ultrapack (`sprite_packs/`, 449 MB over four tiers, 183 targets) has one
  reader (`intro_cart`, `game/ambition_content/src/intro/sprites.rs`), and
  `scripts/measure_pack_reachability.py` finds 444 MB (98.8%) unreachable.
  The runtime already loads only the pages a target's frames use
  (`load_sprite_pages`, `used_pages`); it is the PACKAGE that carries all of
  it, because the packager ships every file. Split the runtime product by
  runtime unit (pages per target or target group), not the source pack.

### Quality is a presentation policy (Q84)

Ruling Q84 (2026-10-04): portraits take part in quality scaling like other
presentation assets. Each quality level provides the cheapest product that
still does the semantic UI job acceptably. That is a different bar from Q69,
which lets a potato sprite be humorously small: a portrait's job is to be read
in a dialogue box, so its reduced tiers are sized from its draw size
(`DialogLayoutProfile`, `game/ambition_content/src/presentation/dialog.rs`:
56×62, 82×94 or 104×120 px), not from the 1/16 sprite rule.

Today the reduced portrait tiers are generated and shipped but never loaded:
`bake_portrait_manifests` (`ambition_sprite_sheet/build.rs`) reads only
`sprites/`. `dev/patches/portrait-tiers-are-never-baked-20260902.patch`
implements the opposite answer (full resolution only) and is superseded by
Q84. Work: queue row PORTRAIT-TIERS.

The broader rule: a quality mode may in time choose a cheaper implementation
of any presentation-only system (textures, particles, animation detail,
decorative populations, lighting, post-processing), not only a smaller
texture. Simulation and deterministic gameplay must not depend on the
highest-fidelity presentation; a test that passes at `Full` and fails at
`Potato` on a gameplay fact is a defect in the gameplay side. This is a design
constraint, not a plan to build each of those now.

## Current measured conclusions

These are the conclusions still used for architecture decisions.

### Global quality tiers are the current policy

There is no room-level sprite tier cap. Do not reintroduce one through a local
loader or materialization shortcut.

### Oversampling/weak-tier cost is real

Authored high-resolution art can be significantly larger than the presentation
requires on weak targets. The correct response is quality-tier selection and
materialization policy, not arbitrary runtime rescaling disconnected from the
asset catalog.

### The visible hitch is not equivalent to “texture bytes are not ready”

Measurements separated asset readiness from visible-entity/materialization
latency. A body can be known to simulation while its presentation entity has not
yet appeared.

Instrumentation therefore needs to report all lifecycle stages rather than
blaming every placeholder/hitch on I/O.

### One-body/one-path claiming and asset preparation are different problems

A placeholder that persists because the body has no claimed visual entity is not
fixed by changing texture tier or preloading more bytes. Preserve ownership
instrumentation alongside asset timing.

### Live quality changes must preserve a usable representation

The established quality-transition contract is replacement/swap. Keep current
art visible until the requested representation is ready unless a product policy
explicitly chooses a lower fallback.

## Current work

### A1 — stage-specific observability

For a named asset/body, make it possible to answer when each stage occurred:

```text
demanded
CPU prepared
materialization requested
materialized
presentation entity claimed/created
first resident use / first draw
```

Requirements:

- stable semantic asset/body identity in the trace;
- current requested/resolved tier;
- no inference from “asset server says loaded” to “drawable exists”;
- capture/report tooling should print missing stages rather than substituting a
  later timestamp.

Acceptance: a visible hitch can be assigned to one stage without reading source
and guessing.

### A2 — demand before first visible use

Identify assets whose first semantic demand occurs too close to first visible
use.

Move demand earlier only at a domain boundary that already knows the asset will
be needed. Do not preload the whole game to hide one transition.

Acceptance:

- demand lead time is measurable;
- no new game/session loads unrelated rooms/characters;
- headless simulation can omit visual materialization.

### A3 — pace expensive completion, not declarations

If a burst is caused by materialization/device work, pace or budget that stage.
Do not delay cheap semantic declarations simply to spread a later expensive
stage.

Any pacing policy must preserve deterministic gameplay and define what happens
when first visible use arrives before the preferred budget slot.

### A4 — define residency scopes and budgets

Before adding eviction, inventory the real owners:

- boot/core shared assets;
- current session/room;
- character/roster assets;
- short-lived VFX;
- optional provider assets.

For each scope, decide what may remain resident across room/session transitions
and under what pressure it is evicted.

Do not implement a generic LRU before these scopes are known.

### A5 — eliminate accidental re-preparation/reload

A second request for the same semantic asset/tier should join/reuse the existing
preparation whenever its lifetime allows.

Instrument and poison:

- duplicate demand from two consumers;
- room leave/re-enter;
- character reappearance;
- quality swap away and back;
- session reset.

The acceptance question is whether repeated expensive preparation was required
by lifetime/policy, not whether the same path string appeared twice.

### A6 — live quality switching

A quality change must:

1. update the requested quality authority;
2. demand replacement variants where applicable;
3. keep the current usable representation until replacement is ready;
4. atomically switch presentation to the new representation;
5. retire old residency according to policy;
6. avoid simulation changes.

Include FX sheets and character sheets in the same acceptance surface.

### A7 — explicit readiness semantics

Every transition that waits on assets should state **which stage** it needs.
Examples:

- semantic simulation may need only catalog declaration;
- visible spawn may require a drawable-ready representation;
- screenshot/capture acceptance may require first draw;
- gameplay must not wait for GPU materialization unless the product contract
  explicitly makes visible readiness part of the transition.

Replace generic “assets ready” predicates when their callers require different
stages.

## Instrumentation expectations

Asset tooling should prefer generated/current facts over copied tables.

A useful report includes:

- semantic asset identity;
- source/variant selected;
- requested and resolved quality tier;
- stage timestamps/durations;
- bytes/pixels when relevant;
- owner/scope;
- current resident/use state.

Historical hall/tier tables are evidence, not live policy. Re-run the instrument
when a current number is needed.

## Explicit non-goals

- no room-level quality cap;
- no “preload everything” solution;
- no simulation dependency on GPU readiness;
- no generic eviction cache before residency ownership is defined;
- no second quality selector inside an individual loader;
- no treating every missing visual as an asset-I/O problem.

## Acceptance

The current architecture slice is complete when:

1. all important presentation asset roads use one quality authority;
2. demand/preparation/materialization/first-use are separately observable;
3. first-visible-use hitches have a named stage and owner;
4. expensive bursts are paced at the expensive stage;
5. residency ownership/budgets are explicit enough to support eviction if
   needed;
6. duplicate demand does not cause accidental duplicate expensive preparation;
7. live quality switching is replacement/swap and covers character + FX roads;
8. transitions wait on the readiness stage they actually require.

Potato stays at 1/16 linear scale for every sprite family (Q69 in
[`maintainer-decisions.md`](../maintainer-decisions.md)). A
proposed renderer aid is still not applied and not validated:
`dev/patches/swing-fighter-render-honours-quality-scale-20260902.patch` makes
the four swing-fighter targets in `tools/ambition_sprite2d_renderer` refuse a
quality scale they cannot honour. Without it, they write a full-resolution
sheet into a reduced tier. It is a validation aid, not a quality policy, and the
patch states how to validate it with one render.

## Prepared simulation content versus presentation residency

[A6/A8/A9](actor-monolith-work-frontier.md) rely on a hard separation: validated
mechanical content, immutable prepared definitions and device-ready presentation
resources have different readiness and lifetime rules. A visual tier swap must
not change body size, hurtboxes, authored occurrence identity or the active
technique program. Geometry validation must work in a render-absent profile.

Publish a complete accepted prepared revision to simulation. Do not let one actor
combine a new flow/geometry definition with a prior revision's references merely
because one asset completed loading first. Stronger hot-reload guarantees need
revision-pinning and failure fixtures, not a broad mutable content service.

Residency budgets remain a product/hardware decision under Q94. Preparation may
produce an accurate byte estimate and eviction policy without inventing the
budget. Distinguish source bytes, decoded CPU data, device residency and active
simulation definitions in measurements; dependency count is none of these.
