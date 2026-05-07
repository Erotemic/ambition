# tools/

Standalone tool projects that support Ambition development. Each project
under `tools/` is a self-contained Python package with its own
`pyproject.toml`, README, and (where useful) `.venv`. They are not
checked-in workspace members of the Cargo project; they are author-time
helpers.

## Layout

```
tools/
  ambition_music_renderer/      # MusicIR YAML → adaptive OGG stems / preview mixes
  ambition_sprite2d_renderer/   # procedural 2D sprite/spritesheet renderer
  ambition_ldtk_tools/          # modal CLI for editing/validating sandbox.ldtk
  experimental/
    ambition_sprite3d_renderer/ # reference / failed Blender-first 3D experiment
  audio/                        # legacy install scripts + shims (being phased out)
  generators/                   # legacy 2D character lab + sandbag shim
```

## Conventions

### Renderers (music, sprite2d)

- Generate local output under `tools/<project>/generated/<target>/`.
- Publish/install explicitly into `crates/ambition_sandbox/assets/...`.
- Generated outputs are gitignored — do not commit `.png`, `.yaml`,
  `.ogg`, `.wav`, `.mid`, etc., produced by these tools.
- Every renderer should expose at least:
  - `render <target>`
  - `install <target>` (or `publish`)
  - `render-publish <target>` (run both)

### LDtk tools

- Use `python -m ambition_ldtk_tools …` for any semantic edit.
- Agents should not hand-edit `crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk`
  JSON.
- Mutations always run repair + validate before writing the file.

### Experimental

- `tools/experimental/` contains reference-only or failed experiments.
  Don't install runtime assets from these tools; revive them out of
  `experimental/` first if needed.

## Top-level CLIs

| Tool | Module | What it does |
|---|---|---|
| Music renderer | `python -m ambition_music_renderer` | Render + publish MusicIR cues |
| 2D sprite renderer | `python -m ambition_sprite2d_renderer` | Render + install sprite targets (e.g., sandbag) |
| LDtk tools | `python -m ambition_ldtk_tools` | Validate, repair, round-trip, area/entity/def edits |

## Compatibility shims

The pre-flatten paths still work but print a deprecation note. They
forward to the new CLIs:

- `tools/audio/render_sandbox_music.py` → `ambition_music_renderer sandbox render-publish`
- `tools/generators/gen2d/draw_sandbag_spritesheet.py` → `ambition_sprite2d_renderer render sandbag`
- `tools/generators/gen2d/generate_sandbag_assets.sh` → `ambition_sprite2d_renderer render-publish sandbag`
- `tools/validate_ambition_ldtk.py` → `ambition_ldtk_tools validate`
- `tools/repair_ambition_ldtk.py` → `ambition_ldtk_tools repair`
- `tools/check_ldtk_editor_roundtrip.py` → `ambition_ldtk_tools roundtrip`
- `tools/fetch_ldtk_schema.py` → `ambition_ldtk_tools schema fetch`
- `tools/author_ldtk_area.py` → `ambition_ldtk_tools area create`
- `tools/add_ldtk_entity_to_level.py` → `ambition_ldtk_tools entity add`
- `tools/register_ldtk_entity_def.py` → `ambition_ldtk_tools def register-entity`
