
# tools/

Standalone author-time tool projects for Ambition. They are not Cargo workspace members. Most are Python packages with their own README, `pyproject.toml`, and tests.

For agent routing, start with [`../docs/tools/index.md`](../docs/tools/index.md).

## Active groups

| Group | Location | Primary purpose |
|---|---|---|
| LDtk tools | `ambition_ldtk_tools/` | Validate, repair, roundtrip, compact, initialize, and author LDtk worlds. |
| Music renderer | `ambition_music_renderer/` | Render/audit generated music and transition material. |
| SFX renderer / packer | `ambition_sfx_renderer/`, `ambition_sfx_pack/` | Render/audit generated SFX and pack runtime banks. |
| Sprite renderer | `ambition_sprite2d_renderer/` | Generate/publish gameplay and dialogue-portrait sprite sheets plus runtime-facing metadata through plural authoring families. |
| Background/parallax renderers | `ambition_background_renderer/`, `ambition_parallax_renderer/` | Generate static and parallax visual assets. |
| Optimization reports | `optimization_report/` | Collect LLM-readable performance/build diagnostics. |

## Experimental

`experimental/` contains reference or in-progress work. Do not install runtime assets from experimental tools until the tool is promoted and documented.

## Conventions

- Every active Python tool owns `tools/<tool>/.venv`; bootstrap them with
  `../run_developer_setup.sh`. Normal renderer runs reuse those environments;
  rerun setup only when dependencies, the requested Python version, or
  submodules change.
- Prefer `python -m <package>` for package CLIs, using that tool-local interpreter.
- Generated outputs stay local until an explicit install/publish step.
- Do not hand-edit `game/ambition_content/assets/worlds/sandbox.ldtk`; use `ambition_ldtk_tools`.
- Keep tool READMEs concise and update `docs/tools/` when a workflow becomes agent-relevant.
