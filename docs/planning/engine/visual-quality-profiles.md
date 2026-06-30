# Visual quality profiles

A single global **quality profile** that resolves to a structured **runtime/device
budget** every visual subsystem reads. The immediate driver is Android performance —
40 FPS baseline, diving to 10–20 in portal rooms — but the profile is not portal-only.

> Status: **implemented** (folded onto `main` 2026-06-30; texture pipeline completed +
> `Potato` tier added same day). The profile spine (`VisualQualityProfile` /
> `VisualQualityBudget` / `VisualQualitySettings`, `ResolvedVisualQuality`), portal
> static caps + scheduling, the parallax budget, the menu/settings/parity rows, and the
> F3 inspector readout are all in. The **variant texture pipeline is now complete end to
> end**: the generator rescales every pixel-space RON field (frame rects, trim, body /
> hit / hurt boxes, feet pixel) to match the resized PNG — not just the PNG — and the
> runtime **pairs the variant `_spritesheet.ron` with the variant PNG** (character +
> boss load paths) so a low profile actually loads smaller, self-consistent atlases.
> `Potato` is the new bare-minimum tier. This doc remains the design of record; numbers
> are still a starting point, to be re-grounded against on-device measurement.

---

## The principle

```
Authoring assets/config describe the ideal presentation.
Quality profiles describe the runtime/device budget.
Asset loaders and render systems consume the resolved budget.
Effective config = authored config, clamped by the quality budget.
```

The corollary is the elegance constraint: **no subsystem independently interprets
Low/Medium/High.** One type resolves a profile to a budget; subsystems read the budget.
A subsystem that branches on the profile enum is a smell.

Language follows the relativity stance — *controlled body*, *focus*, *viewpoint*,
*local presentation focus*; never "the player's camera." F3/inspector rows use the
**exact Rust field names**, not humanized labels.

---

## The problem is two problems (measured)

The external draft bundles both under one "resolution scale" knob. They are different
costs with different mechanisms, and conflating them is the draft's main flaw.

### 1. Portal dive — a *real-time, per-frame* cost

Confirmed in `crates/ambition_portal_presentation/src/view_cones.rs`: every visible
portal **re-renders the whole scene** (world layer + parallax + recursion) into its own
render target, **every frame**, with **no throttle and no cap on simultaneous
captures**. `sync_portal_view_cones` updates every rig each frame; `is_active` only
gates whether a capture camera renders, never how *often* or how *many*.

So the dominant lever is **the number of extra full-scene passes per frame**, not
texture memory. ROI order of the levers:

1. **Throttle** — update at most `max_updates_per_frame` portals per frame, honor a
   `min_refresh_interval_s`, cap `max_active_captures`; skipped/offscreen/blocked
   portals **reuse their last texture**. *This is the win for the dive.*
2. `recursion_depth → 0` and `include_parallax → false` — removes whole render layers
   (`PORTAL_WINDOW_RENDER_LAYER`, the parallax layer) from each capture pass.
3. `max_resolution` / `texels_per_world_px` caps — cheaper fill per pass. Default
   `max_resolution = 4096`, `texels_per_world_px = 1.0` is the authored ceiling; the
   budget clamps it down.

This is genuinely a live, per-device budget — a profile **resource** read each frame is
the right home.

### 2. Baseline VRAM — a *load-time / packaging* cost

Measured footprint of installed sprites is **~838 MB VRAM** (RGBA, uncompressed) against
a typical mobile budget of **256–512 MB**. Biggest offenders:

| Asset | Sheet px | ~VRAM |
| --- | --- | --- |
| Perfect Cell-ular Automaton (2 paged sheets) | ~4096² ×2 | ~126 MB |
| `player_robot` + `player_extended` | ~2640² ×2 | ~53 MB |
| `sandbag_full_review` | 2412² | ~22 MB |

Two facts shape the fix:

- The sprite generator already renders at **`render_scale = 2`** (2× native pixels, for
  crispness). The native art is *already* half-size — so a half-res variant is close to
  "publish at the resolution we supersample down from," not a quality cliff.
- Packing is **already solved and not the lever here.** The generator alpha-trims +
  MaxRects-packs every frame (`authoring/packer.py`), policy data-driven per target in
  `registry/pack_groups.py` (default `trim=True, page_size=4096, max_dim=16384`); PCA
  takes the default, so it ships as trimmed frames across 4096-capped pages and the old
  16384-px dimension crash is already guarded by `page_size`. The point for *this* work:
  PCA's ~126 MB is **post-trim** — its frames are genuinely large, so **resolution
  variants (render at a lower scale), not packing, are the remaining VRAM lever.**

Because textures load once, this is fundamentally a *which set do we load* decision.
**Per Jon's call, we build the runtime-switchable variant pipeline** (folders +
per-asset fallback + manifest variants), with the asset-reload cost made explicit (see
[Variant pipeline](#variant-pipeline)) rather than pretending it's free.

---

## The shape

```rust
enum VisualQualityProfile { Potato, Low, Medium, High /*default desktop*/, Ultra, Custom }

struct VisualQualityBudget {
    portal:      PortalCaptureBudget,
    sprites:     SpriteTextureBudget,
    backgrounds: BackgroundTextureBudget,
    parallax:    ParallaxBudget,
    shaders:     ShaderBudget,
    particles:   ParticleBudget,
}
```

Flow:

```
UserSettings.video.quality : VisualQualitySettings   (persisted; profile + Custom budget)
        │  resolved_budget()  →  for_profile() table, or the Custom budget verbatim
        ▼
ResolvedVisualQuality { profile, budget }            (Bevy Resource in ambition_render)
        │  read by …
        ├─ portal view-cone sync   → EffectivePortalCaptureBudget = min(authored, budget)
        ├─ parallax spawn          → enabled / max_layers
        ├─ asset loader            → variant folder selection + fallback
        ├─ screen-effects          → screen_shader_scale  (cap)
        └─ vfx spawn               → max_particles / spawn_rate_scale  (cap)
```

`ResolvedVisualQuality` is the single seam. A sync system recomputes it when
`UserSettings.video.quality` changes; nothing downstream re-reads `UserSettings`.

**All six categories are defined now** (Jon's call). Portal, parallax, sprite, and
background budgets get **real consumers** in this work. `ShaderBudget` and
`ParticleBudget` are **defined and resolved**, but their consumers are wired only where
a central, obvious spawn/strength path already exists; otherwise the consumer is a
documented `// TODO(quality):` at the spawn site, not a fabricated system. This keeps
the *type* complete (so profiles and Custom are total) without inventing VFX plumbing
that doesn't exist yet.

---

## The budget, by profile

`TextureResolutionScale { Potato, Quarter, Half, Full }` with `folder_suffix()` /
`asset_subdir()` / `parallax_subdir()` / `asset_id_suffix()` / `scale_factor()`
helpers, plus `MANIFEST_VARIANTS` (the below-`Full` tiers — single source of truth
for the manifest-registration loops; see [Variant pipeline](#variant-pipeline)).

| field | Potato | Low | Medium | High | Ultra |
| --- | --- | --- | --- | --- | --- |
| `portal.max_resolution` | 128 | 384 | 512 | 1024 | 2048 |
| `portal.texels_per_world_px` | 0.05 | 0.25 | 0.50 | 1.00 | 1.00 |
| `portal.recursion_depth` | 0 | 0 | 0 | 1 | 1 |
| `portal.max_active_captures` | 1 | 1 | 1 | 2 | 4 |
| `portal.max_updates_per_frame` | 1 | 1 | 1 | 2 | 4 |
| `portal.min_refresh_interval_s` | 0.250 | 0.100 | 0.050 | 0.000 | 0.000 |
| `portal.include_parallax` | false | false | false | true | true |
| `sprites.resolution_scale` | Potato | Half | Half | Full | Full |
| `sprites.prefer_scaled_variants` | true | true | true | false | false |
| `backgrounds.resolution_scale` | Potato | Half | Half | Full | Full |
| `backgrounds.max_texture_resolution` | 256 | 1024 | 1536 | 2048 | 4096 |
| `backgrounds.prefer_scaled_variants` | true | true | true | false | false |
| `parallax.enabled` | **false** | true | true | true | true |
| `parallax.max_layers` | Some(0) | Some(2) | Some(3) | None | None |
| `parallax.resolution_scale` | Potato | Half | Half | Full | Full |
| `shaders.screen_shader_scale` | 0.0 | 0.5 | 0.75 | 1.0 | 1.0 |
| `shaders.allow_expensive_materials` | false | false | true | true | true |
| `particles.max_particles` | 16 | 128 | 256 | 512 | 1024 |
| `particles.spawn_rate_scale` | 0.1 | 0.5 | 0.75 | 1.0 | 1.0 |

**Potato** is the deliberate floor — "runs on a literal potato," and a bit of a
joke. Everything is stripped, and its textures are shrunk toward ~1% of the
authored size with a per-sheet **8px frame floor** (so `player_robot`'s sheet
goes 2638² ≈ 28 MB VRAM → 82² ≈ 0.03 MB). It is *meant* to look bad.

`Custom` resolves to its stored budget verbatim. Numbers are a starting point — the
portal row and the sprite `resolution_scale` are the two that actually move Android
frames; re-baseline against on-device capture before treating any of them as final.

### Platform defaults

```
default_visual_quality_profile():
    #[cfg(target_os = "android")] → Medium
    #[cfg(not)]                   → High
```

`Default for VisualQualitySettings` uses the platform default profile **and** seeds
`custom` with that profile's budget (so flipping to Custom starts from a sane place).
Ultra is opt-in everywhere.

---

## Variant pipeline {#variant-pipeline}

Runtime-switchable, per Jon's choice. Two halves: **generate** the variants, then
**select** them at load with silent fallback.

### Installed layout

```
crates/ambition_gameplay_core/assets/
  sprites/                          # Full  (current path, unchanged)
  sprites_0_5x/                     # Half
  sprites_0_25x/                    # Quarter (optional first pass; Half is the priority)
  backgrounds/parallax_layers/      # Full
  backgrounds/parallax_layers_0_5x/ # Half
  backgrounds/parallax_layers_0_25x/# Quarter
```

`TextureResolutionScale::asset_subdir()` / `parallax_subdir()` own these strings.

### Landmine: `build.rs` embed scope

`crates/ambition_gameplay_core/build.rs` embeds every `*_spritesheet.ron` from
`assets/sprites/` **plus one level of subdirectories**. Sibling variant folders
(`sprites_0_5x/…`) are **not embedded** unless `build.rs` is taught about them. This is
the first thing to get right or Android (which trusts the embedded/packaged manifest set
rather than the filesystem) silently has no variant RONs to load. Decide explicitly:
embed all variant folders, or embed only the platform's chosen tier at build time.

### Selection + fallback

Resolution is a **load-time** decision driven by `ResolvedVisualQuality.budget`. For a
base folder `base` and scale `s`:

```
quality_sprite_folder(base, s):  Full → base,  Half → "{base}_0_5x",  Quarter → "{base}_0_25x"
```

Fallback chain (silent; preserves the existing colored-rectangle placeholder as the
floor):

1. variant path (`sprites_0_5x/…`, `backgrounds/parallax_layers_0_5x/…`)
2. full path (`sprites/…`, `backgrounds/parallax_layers/…`)
3. existing placeholder

`--sprite-folder custom_sprites` must still work: Half tries `custom_sprites_0_5x` then
`custom_sprites`. `SandboxAssetCatalog::try_path_for_load` pre-checks the filesystem on
desktop and trusts packaging on Android — the resolver must respect both (so on Android,
fallback can't rely on a runtime `exists()`; it relies on what `build.rs` embedded).

Two viable resolver designs; pick during implementation:

- **Path helper + fallback** (smaller): a `try_quality_path_for_load(...)` that walks the
  chain. Acceptable for the first pass if unit-tested.
- **Manifest variant IDs** (more robust, larger): variant asset IDs
  (`sprite.character.player@0.5x`, `background.parallax.lab.sky@0.5x`) alongside the
  originals. Better for the Android packaging story; more churn.

### Live reload (the chosen behavior)

**Decision: resolution changes apply live**, in-menu, without a room reload — the thing
good games do. The runtime budgets (portal, parallax count, shader/particle caps) are
read each frame and were always live; the work is making **sprite/background resolution**
live too, since those are loaded once into `GameAssets`.

The mechanism: a system watches `ResolvedVisualQuality` for a change in
`sprites.resolution_scale` / `backgrounds.resolution_scale`, **re-resolves the variant
paths, and re-issues the asset loads into `GameAssets`** (the same resolver as cold
load, just re-run). Because visual entities reference assets through `GameAssets`'
handles, the cleanest shape is:

- `GameAssets` is the single owner of the image/atlas handles;
- the reload system rebuilds those handles from the new scale and swaps them in place;
- a light refresh pass re-points any entity that cached its own `Handle<Image>` /
  `TextureAtlas` (or, better, entities read from `GameAssets` indirectly so the swap is
  automatic). Settling which entities cache handles vs. look them up is the main design
  task here.

Cost to be honest about: a brief load hitch while the new-scale atlases stream in (one
tier resident at a time — we do **not** pre-load both, that doubles VRAM and defeats the
point on mobile). A short pop-in on switch is acceptable and expected; document it.

`load_game_assets(...)` gains an `Option<&VisualQualityBudget>` (or the two scales on
`GameAssetConfig`, set after settings load, before the call) and is callable again for
the live reload. Character/boss sprites (`character_sprites/assets.rs`), entity sprites
and parallax paths (`assets/game_assets/entity_sprite.rs`) all route through the
resolver, so the cold-load and live-reload paths share one resolution function.

---

## Consumers

### Portal captures — the real-time fix

Add a **pure, testable** clamp in `view_cones.rs`:

```rust
struct EffectivePortalCaptureBudget { /* the seven portal fields */ }

fn effective_portal_capture_budget(
    config: &PortalViewConeConfig, q: &PortalCaptureBudget,
) -> EffectivePortalCaptureBudget {
    // min() the authored caps; take the scheduling fields straight from q
}
```

Do **not** mutate `PortalViewConeConfig` (it stays the authored ideal). Route
`capture_dims`, the `RebuildKey`, the recursion render-layer decision, and parallax
inclusion through the *effective* values:

```rust
fn capture_render_layers(recursion_depth: u32, include_parallax: bool, parallax_layer: usize) -> RenderLayers {
    let mut l = RenderLayers::layer(WORLD_RENDER_LAYER);
    if include_parallax { l = l.with(parallax_layer); }
    if recursion_depth > 0 { l = l.with(PORTAL_WINDOW_RENDER_LAYER); }
    l
}
```

`RebuildKey` must include effective resolution / recursion / parallax-inclusion so a
profile change rebuilds the targets.

**Static caps first** (`max_resolution`, `texels_per_world_px`, `recursion_depth`,
`include_parallax`) — small, high-confidence, already most of the fill-rate win.
**Then scheduling** (`max_updates_per_frame`, `min_refresh_interval_s`,
`max_active_captures`) — the dominant win, but more involved:

- skipped captures keep their last texture; hidden/offscreen/blocked windows don't
  consume an update slot;
- at most `max_updates_per_frame` captures refresh per frame, deterministic order first
  (fancy priority later);
- add a last-update frame/time to `PortalViewRig` as needed.

If scheduling balloons, land static caps, leave a clear `// TODO(quality): scheduling`,
and say so — static caps alone are shippable.

### Parallax

`crates/ambition_render/src/rendering/parallax.rs`: honor `parallax.enabled` (spawn
none, keep the clear color/world) and `parallax.max_layers` (cap `RUNTIME_PARALLAX_LAYERS`
in priority order Sky → FarBackplate → NearBackground → ForegroundAtmosphere).
`resolution_scale` is handled by the asset resolver; runtime only needs it for the
effective-budget readout. `spawn_parallax_layers(...)` gains the quality argument, or its
callsites (`scene_setup.rs`, `dev_runtime.rs`, `world_flow/room_flow.rs`) read the
resource — whichever is the smaller diff.

### Shaders / particles

`ShaderBudget` caps screen-effect strength (`screen_shader_scale` as an upper multiplier
in `crates/ambition_render/src/screen_effects/`, never editing user settings).
`ParticleBudget` caps spawn rate/count **if** a central VFX spawn path exists under
`crates/ambition_render/src/fx`; otherwise the budget is resolved but its consumer is a
`// TODO(quality):` at the spawn site. No fabricated VFX systems.

---

## Settings, menu, inspector

The enum + menu path is the well-worn ~7-touchpoint pattern (cf. the existing
`FramePaceCap` row); the killer touchpoint is `curated_options` — settings live in the IR
**and** a per-page allow-list both renderers filter through.

- `persistence/settings/video/quality.rs` (new) — the types; re-exported from
  `video/mod.rs`. `VideoSettings` gains `#[serde(default) ] pub quality:
  VisualQualitySettings`; `Default` and `clamp_all()` updated. `VisualQualityProfile`
  gets `ALL` / `label()` / `next()` / `prev()` matching the existing enum convention.
- Menu: a `SettingsOptionId::VisualQuality` row under Video (near `FramePacing`), built
  with `enum_index` + `cycle(...)`, applied with the `cyc!` macro. Regular menu exposes
  **only the profile**, not every budget field.
- Parity tests (`ambition_app/src/menu/parity_tests.rs`): add the new id to
  `ALL_SETTINGS_OPTION_IDS`, the IR-surface test, and the curated/system-model test.
- F3 portal inspector (`ambition_app/src/dev/portal_inspector.rs`): a read-only
  **Effective Quality Budget** group with **exact field-name** rows
  (`quality.profile`, `portal.max_resolution`, … `effective_max_resolution`,
  `effective_recursion_depth`, `effective_include_parallax`). Hover doc spells out
  `effective_max_resolution = min(config.max_resolution, quality.budget.portal.max_resolution)`.

---

## Generators — `scripts/generate_visual_quality_variants.py` (landed)

The runtime loads variants; the generator **produces** them. Implemented as a
deterministic **post-publish** helper (chosen over re-rendering at a lower
`render_scale`, which would shift auto-crop boxes per sheet): full-resolution
publish stays byte-for-byte unchanged, then this mirrors `sprites/` →
`sprites_0_5x/` / `sprites_0_25x/` / `sprites_potato/` and
`backgrounds/parallax_layers/` → its `_0_5x` / `_0_25x` / `_potato` siblings.

The piece the first scaffold got wrong (and this fixes): a packed
`*_spritesheet.ron` carries **pixel coordinates** that index the PNG. So the
generator rescales the PNG **and** every pixel-space RON field by one consistent
per-sheet factor — `frame_width/height`, `label_width`, `y_offset`, each frame
rect's `x/y/w/h` + trim `off`, and the `body_metrics` body / hit / hurt boxes
(`bbox`, `parts`, `poly`, per-frame `frames`) + `feet_pixel`. Normalized data
(`feet_anchor_norm`, per-frame `anchors`, durations, `collision_scale`) is left
untouched. Scaled rects are clamped into the resized page bounds so per-field
rounding can't push an atlas cell off the texture. The RON parse/serialize is a
small purpose-built reader (Python RON libs mangle `None`/unit variants — see
[[feedback_pyron_unit_variants]]); the **drift guard** is the Rust-side
`every_spritesheet_ron_parses_into_sheet_record` test, which now also walks the
variant folders when present and deserializes them via the real `ron` crate.

`Potato` aims for ~1% but **floors each sheet at an 8px frame** so atlases stay
loadable; the effective factor is therefore per-sheet and is baked into the
variant RON, so the runtime never needs to know it.

Both `regen_sprites.sh` / `regen_backgrounds.sh` keep working on a fresh clone;
variant PNGs/RONs are **generated, never committed** (gitignored) — re-run
`python3 scripts/generate_visual_quality_variants.py` to reproduce them. The
sprite2d / parallax renderers still own full-res output; this script is the
variant tack-on.

---

## Sequence (elegance order — each stage is shippable)

1. **Portal static caps + the profile spine.** `VisualQualityProfile` /
   `VisualQualityBudget` / `VisualQualitySettings`, `ResolvedVisualQuality` resource +
   sync, the `effective_portal_capture_budget` clamp wired through `capture_dims` /
   `RebuildKey` / `capture_render_layers`. Menu row + persistence + parity tests. Ships
   the first portal-fill win and the whole settings surface.
2. **Portal scheduling.** `max_updates_per_frame` / `min_refresh_interval_s` /
   `max_active_captures` with last-texture reuse. *The dive fix.*
3. **Parallax budget.** `enabled` / `max_layers`.
4. **Variant generation.** Sprite + parallax `_0_5x` output; `build.rs` embed scope
   decided.
5. **Variant selection + fallback** at load — *the baseline-VRAM win.*
   Then **live reload**: a system re-issues the asset loads when the resolution scale
   changes, so the menu switches resolution in-place.
6. **Shader/particle caps** where a central path exists; TODOs where not.
7. **Inspector readout** + Custom-profile polish.

Stages 1–2 are the Android FPS fix and are independent of the asset pipeline; 4–5 are
the VRAM/baseline fix. Land in this order so the portal dive is fixed before the larger
asset work.

---

## Tests

- **Budget**: `for_profile` returns the table above; `default()` is platform-correct;
  `next/prev` cycle Low→…→Custom; old settings missing `quality` deserialize.
- **Menu**: the Video row appears, applies, and parity tests pass.
- **Paths**: `quality_sprite_folder` (Full/Half/Quarter, and `custom_sprites`→
  `custom_sprites_0_5x`); parallax folder helper; fallback-to-full when a variant is
  absent (unit-testable helper).
- **Portal**: `effective_portal_capture_budget` clamps authored config by budget;
  `RebuildKey` changes when effective resolution changes; `capture_render_layers` drops
  parallax when `include_parallax == false` and drops recursion when effective
  `recursion_depth == 0`.
- **Generators**: variant publish writes self-consistent PNG/YAML/RON; Half frames ≈
  half the full install; variant RON parses via `ambition_sprite_sheet::SheetRecord`;
  full publish unchanged; parallax variant dirs written.

Verify against the real sim/headless where possible, per the engine stance — the portal
clamp and the path resolver are both pure and unit-testable without a GPU.

---

## Validation (when implementing)

```
cargo fmt --check
cargo check -p ambition_gameplay_core
cargo check -p ambition_render
cargo check -p ambition_portal_presentation --features effect_view_cones
cargo check -p ambition_app --features desktop_dev
cargo test  -p ambition_gameplay_core -p ambition_render -p ambition_portal_presentation
cd tools/ambition_sprite2d_renderer && uv run pytest tests/test_render_scale.py tests/test_core_pipeline.py tests/test_packer.py
```

Manual: desktop default resolves High, Android cfg resolves Medium; the Video menu shows
**Quality Profile**; Low/Medium prefer variant assets and fall back silently when
absent; F3 shows the portal effective cap below the authored 4096 under Low/Medium;
parallax layer count caps under Low/Medium.

---

## Open questions (settle during implementation)

- **`build.rs` embed scope** — embed all variant folders, or only the platform tier?
  (Drives the Android packaging size.)
- **Live-reload handle ownership** — decided that resolution switches live (no room
  reload); the open part is *which* visual entities cache their own handles vs. look
  them up through `GameAssets`, since the swap is automatic only for the latter.
- **Resolver design** — path-helper-with-fallback (smaller) vs manifest variant IDs
  (more robust on Android).
- **Quarter (`_0_25x`)** — ship now or after Half proves out.

## Explicitly out of scope

- Sheet packing — already done (alpha-trim + MaxRects, `registry/pack_groups.py`); the
  16384-px dimension crash is already guarded by `page_size=4096`. This work changes
  *render resolution* (variants), not the packing layout.
- Any change to authored `PortalViewConeConfig` defaults — the budget clamps; it does
  not rewrite the ideal.
