# Tools index

Author-time tools support LDtk authoring, generated assets, reports, and experiments. Read this before adding a new tool or asking an agent to use an existing one.

## Active tools

| Tool | Location | Use |
|---|---|---|
| LDtk tools | `tools/ambition_ldtk_tools/` | Validate, repair, roundtrip, compact, list metadata, initialize worlds, and author areas/entities. Prefer `python -m ambition_ldtk_tools`. |
| Music renderer | `tools/ambition_music_renderer/` | Render/publish generated music cues and transition labs. |
| SFX renderer | `tools/ambition_sfx_renderer/` | Render/audit generated SFX and banks. Note: this is a nested tool checkout in some working trees; do not delete accidentally. |
| SFX packer | `tools/ambition_sfx_pack/` | Pack sound effects into runtime banks. |
| 2D sprite renderer | `tools/ambition_sprite2d_renderer/` | Generate/publish 2D character/entity spritesheets and rig assets. |
| Background renderer | `tools/ambition_background_renderer/` | Generate background images. |
| Parallax renderer | `tools/ambition_parallax_renderer/` | Generate parallax layers. |
| Optimization report | `tools/optimization_report/` | Collect optimization/performance reports. |
| Test coverage report | `tools/test_coverage_report.sh` | Coverage-oriented helper script. |
| Vanity card prep | `tools/vanity_card_prep/` | Prepare vanity/promo card material. |

## Experimental tools

`tools/experimental/` contains reference or in-progress work. Do not install runtime assets from these without first promoting the tool and documenting the workflow.

## Rules

- Do not hand-edit `sandbox.ldtk`; use LDtk tools.
- Generated outputs usually stay gitignored until explicitly published into runtime assets.
- If a tool should be used by agents, document its command shape here and in the tool README.
- If a tool is obsolete, archive or delete the docs rather than leaving it as a current workflow.

## LDtk quick commands

```bash
python -m ambition_ldtk_tools validate crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
python -m ambition_ldtk_tools repair crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk --in-place
python -m ambition_ldtk_tools area create tools/ambition_ldtk_tools/specs/mob_lab_area.yaml --backup
```
