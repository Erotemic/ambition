# Sandbag spritesheet generator

This drop-in generator creates a procedural pale cloth sandbag character that intentionally rhymes with the uploaded reference without copying it. The character keeps the recognizable vocabulary: a soft vertical stuffed bag, stitched top/bottom seams, a small top strap, and simple black oval eyes. The silhouette, strap proportions, seam layout, shading, and animation poses are original procedural drawings.

Animations emitted by default:

- `idle`: 6 frames, subtle breathing/bob
- `hit`: 4 frames, squash + recoil + impact burst
- `death`: 7 frames, topple/collapse + dust

The default output is now a **sparse** sheet: it emits only `idle`, `hit`, and `death`. It no longer pads `walk`, `run`, `jump`, `fall`, `slash`, `blink_out`, `blink_in`, or `dash` with alias frames.

## Unpack

From anywhere:

```bash
unzip ~/Downloads/ambition_sandbag_spritesheet_generator_v2.zip -d /home/joncrall/code/ambition
```

## Patch runtime for sparse rows

```bash
cd /home/joncrall/code/ambition/tools/generators/gen2d
./apply_sandbag_runtime_patch.sh
```

The patch updates `CharacterSheetSpec` so each character can define only the rows it owns. Missing animation requests resolve to `Idle` at runtime, so the sandbag does not need alias rows. It also wires `FeatureVisualKind::Sandbag` to `sandbag_spritesheet.png`, with the existing goblin sheet as a fallback if the sandbag asset is absent.

Patched files:

```text
crates/ambition_sandbox/src/character_sprites.rs
crates/ambition_sandbox/src/rendering.rs
```

## Generate assets

```bash
cd /home/joncrall/code/ambition/tools/generators/gen2d
./generate_sandbag_assets.sh
```

That writes:

```text
tools/generators/gen2d/assets/sandbag_spritesheet.png
tools/generators/gen2d/assets/sandbag_spritesheet.yaml
crates/ambition_sandbox/assets/sprites/sandbag_spritesheet.png
crates/ambition_sandbox/assets/sprites/sandbag_spritesheet.yaml
```

## Optional old-runtime compatibility

For an unpatched fixed-11-row runtime only:

```bash
python draw_sandbag_spritesheet.py --legacy-aliases
```

This writes `sandbag_legacy_11row_spritesheet.*` next to the sparse sheet, but the sparse runtime patch is the preferred path.
