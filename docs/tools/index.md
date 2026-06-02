
# Tools index

Author-time tools support LDtk authoring, generated assets, reports, and experiments. Read this before adding a tool or asking an agent to use one.

## Active tool groups

| Group | Doc | Runtime impact |
|---|---|---|
| LDtk/world tools | [`ldtk-tools.md`](ldtk-tools.md) | Mutates or validates authored world data. |
| Generated audio tools | [`generated-audio-tools.md`](generated-audio-tools.md) | Renders music/SFX/banks that may be published to runtime assets. |
| Generated visual tools | [`generated-visual-tools.md`](generated-visual-tools.md) | Renders sprites/backgrounds/parallax/promo images. |
| Optimization/reporting | [`optimization-and-reporting.md`](optimization-and-reporting.md) | Produces diagnostics for humans/agents. |
| ECS inventory | [`ecs-inventory-tool.md`](ecs-inventory-tool.md) | Static inventory of the sandbox Bevy ECS surface for refactor planning / review. |
| Tool authoring policy | [`tool-authoring-policy.md`](tool-authoring-policy.md) | Rules for adding or promoting tools. |

## Quick rules

- Do not hand-edit `sandbox.ldtk`; use `python -m ambition_ldtk_tools`.
- Generated outputs are local until explicitly installed/published into runtime assets.
- `tools/experimental/` is reference/in-progress work; do not install runtime assets from it without promotion.
- Keep the tool README and this index aligned when a tool becomes agent-relevant.
