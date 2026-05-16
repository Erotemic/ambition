# Ambition 2D Sprite Renderer

Procedural 2D sprite renderer for Ambition. Two surfaces share the package:

1. **Adapter targets** — a `BaseAdapter` per target (robot / goblin / boss /
   robot25d), driven by YAML jobs in `configs/`. Powered by the
   `adapters.TARGETS` registry; jobs are loaded by `config.CharacterJob`
   and packed into sheets by `sheet.py`. This is the "character lab"
   surface formerly published as `proc2d_character_lab`.
2. **Tack-on targets** — a per-target module under `targets/<name>.py`
   that exposes `TARGET_NAME`, `SHEET_FILES`, and a
   `render(out_dir, **opts) -> list[Path]` function. Used for one-off
   sheets that do not yet plug into the adapter system. Currently:
   `sandbag`, `creator_lab_props`, `town_tileset`. See the `TODO(integrate-sandbag-into-adapters)` note in
   `targets/sandbag.py` for the path to fold it in.

## Modal CLI

Adapter (character lab) commands:

```
python -m ambition_sprite2d_renderer list-targets        # adapter + tack-on targets
python -m ambition_sprite2d_renderer draw-all            # render every job in configs/
python -m ambition_sprite2d_renderer draw-canonicals     # canonical poses + contact sheet
python -m ambition_sprite2d_renderer draw-review         # curated general-character + NPC review pass
python -m ambition_sprite2d_renderer draw-character <cfg> # one config: canonical + spritesheet + YAML
python -m ambition_sprite2d_renderer draw-entities       # non-character entity sprites
python -m ambition_sprite2d_renderer spritesheet <cfg> <out>
python -m ambition_sprite2d_renderer single <cfg> <out> --animation idle --frame-index 0
```

Tack-on commands:

```
python -m ambition_sprite2d_renderer render <target>           # write to generated/<target>/
python -m ambition_sprite2d_renderer preview <target>          # render and report paths
python -m ambition_sprite2d_renderer install <target>          # copy into sandbox assets
python -m ambition_sprite2d_renderer render-publish <target>   # render then install
```

`render` writes the sheet into
`tools/ambition_sprite2d_renderer/generated/<target>/`. `install` copies the
canonical sheet files into `crates/ambition_sandbox/assets/sprites/`.
`render-publish` does both.

## Targets

### Adapter targets

| Target | Animations | Job |
|---|---|---|
| `robot` | idle, walk, run, jump, fall, slash, hit, death, blink_out, blink_in, dash | `configs/robot.yaml` |
| `ninja` | idle, walk, run, jump, fall, slash, hit, death, blink_out, blink_in, dash | `configs/ninja.yaml`, `configs/ninja_leader.yaml` |
| `toon` | idle, walk, run, jump, fall, talk, interact, slash, dash, celebrate, hit, death | `configs/review/*.yaml`, `configs/fascist_enforcer.yaml` |
| `goblin` | idle, walk, run, jump, fall, slash, hit, death, blink_out, blink_in, dash | `configs/goblin.yaml` |
| `boss` | rest, floor_slam, side_sweep, spike_halo, dash_echo, hit, death | `configs/boss.yaml` |
| `robot25d` | (legacy 2.5D experiment) | — |

Run `python -m ambition_sprite2d_renderer list-targets` to see the live
animation map for each adapter.

### Ninja first-pass target

`ninja` is a bespoke masked shadow-duelist renderer that still participates in
the adapter spritesheet/YAML pipeline.  The base `shadow_duelist` archetype is
slim and blade-forward: slate cloth, red eye slits, angular armor plates,
scarf/sash tails, and a long katana.  The `shadow_oni_leader` archetype uses
the same renderer but deliberately changes the silhouette: visible oni horns,
a ragged command banner, broader shoulder plates, a skirted waist profile,
lamellar chest marks, and a less awkward command-idle arm pose.

Use `draw-character` for art iteration when you want the canonical still and
the runtime sheet/manifest from the same spec in one command:

```bash
python -m ambition_sprite2d_renderer draw-character ambition_sprite2d_renderer/configs/ninja.yaml --out-dir generated
python -m ambition_sprite2d_renderer draw-character ambition_sprite2d_renderer/configs/ninja_leader.yaml --out-dir generated
```

The older split commands still work:

```bash
python -m ambition_sprite2d_renderer single ambition_sprite2d_renderer/configs/ninja.yaml generated/ninja_shadow_duelist_canonical.png --animation idle --frame-index 1
python -m ambition_sprite2d_renderer spritesheet ambition_sprite2d_renderer/configs/ninja.yaml generated/ninja_shadow_duelist_spritesheet.png
```

### Tack-on targets

#### sandbag

Procedural pale cloth sandbag character. Sparse output (only `idle`, `hit`,
`death`). Runtime support for missing animations is provided by
`character_sprites.rs` resolving them to `idle` at load time. Pass
`--legacy-aliases` to also emit the 11-row alias sheet
(`sandbag_legacy_11row_spritesheet.*`) for old-runtime compatibility.

```bash
python -m ambition_sprite2d_renderer render sandbag
python -m ambition_sprite2d_renderer render-publish sandbag
```

#### creator_lab_props

Procedural environment-prop library for a sci-fi creator / fabrication lab.
The sheet contains subtle 4-frame idle loops for containment and machinery
objects such as a genesis vat, specimen jar, neural console, resonance coil,
power core, repair cradle, drone cradle, and portal calibrator. The target
also emits a contact sheet for quick review.

```bash
python -m ambition_sprite2d_renderer render creator_lab_props
python -m ambition_sprite2d_renderer render-publish creator_lab_props
```

#### town_tileset

Procedural **side-scroller** town-environment tileset for building facades,
houses, market streets, and compact settlement scenes. The atlas now contains
96 variants arranged for side-view use rather than top-down pathing: terrain
caps and slopes, cobble and foundation pieces, multiple wall materials
(plaster, timber, brick), dedicated windows and doors, two roof palettes,
platform / balcony / stair parts, and street or civic props such as lamp
posts, benches, wells, market stalls, trees, hedges, and banners. The target
emits a YAML manifest with tile atlas coordinates plus a labeled contact sheet
for quick review.

```bash
python -m ambition_sprite2d_renderer render town_tileset
python -m ambition_sprite2d_renderer render-publish town_tileset
```


## Character specs and review casts

`CharacterJob` now accepts optional `name`, `output_name`, and `spec` fields.
The `toon` target uses those `spec` overrides to author silhouette-first
characters without inventing a brand new renderer per NPC. Review presets can
be intentionally trope-heavy: `absurd_general` is the shouting-general pass with
a giant star cap, epaulets, medals, awards, baton, and irate yell face. Example:

```yaml
target: toon
name: Merchant Prototype
output_name: merchant_prototype
archetype: merchant_prototype
spec:
  torso_w: 31.5
  leg_upper: 10.5
```

The curated review pass lives in `ambition_sprite2d_renderer/configs/review/`
and is meant to answer the question, “do these feel like different characters?”
Use `draw-review` to regenerate the current cast (`general_hero`,
`absurd_general`, `fascist_enforcer`, `kernel_guide`, `merchant_prototype`,
`vault_keeper`, `architect`) along with a canonical contact sheet. The
`fascist_enforcer` preset is a fictional fascist enemy pass: severe cap,
charcoal tunic, red armband with an invented black sigil, collar skull tabs,
and a long rifle so it reads as a distinct villain rather than a variant of
`absurd_general`.

## Adding a new target

### As an adapter target (preferred for character-shaped sprites)

1. Drop a module under `targets/<name>.py` that subclasses
   `adapters.BaseAdapter` and implements `animations()`, `sample_spec()`,
   and `render_frame()`.
2. Register the adapter class in `adapters.TARGETS`.
3. Add a `configs/<name>.yaml` job describing the render parameters.

### As a tack-on target (when adapter shape doesn't fit)

1. Drop a module under `targets/<name>.py` exposing `TARGET_NAME`,
   `SHEET_FILES`, and `render(out_dir, **opts) -> list[Path]`.
2. Register the target id in `_TACKON_TARGETS` in `cli.py`.
3. File a follow-up to fold it into the adapter system once it stabilizes
   (see the `TODO` block in `targets/sandbag.py` for the rough plan).

## Conventions

- Generated outputs live under `generated/` and are gitignored.
- Targets must be deterministic for a given input (same code → same bytes).
- Runtime assets are written only by explicit `install` / `render-publish`
  (or `draw-all` / `draw-canonicals` / `draw-entities` for adapter targets).
- Do not commit `.png`, `.yaml`, etc., from `generated/`.

See `docs/design.md` for the architecture rationale and `docs/ENTITY_TODOS.md`
for outstanding entity-sprite work.
