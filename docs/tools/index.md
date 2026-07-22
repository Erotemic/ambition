---
status: current
last_verified: 2026-07-18
---

# Author-time tools

Tools create, validate, inspect, or publish provider/engine inputs. They are not
runtime architecture and must not become an alternate source of live authority.

| Group | Guide | Main contract |
|---|---|---|
| LDtk/world | [`ldtk-tools.md`](ldtk-tools.md) | Typed, reviewable world edits; never hand-edit `.ldtk` JSON. |
| Generated audio | [`generated-audio-tools.md`](generated-audio-tools.md) | Source spec -> render/audit -> explicit publish/runtime artifact. |
| Generated visuals | [`generated-visual-tools.md`](generated-visual-tools.md) | Generator target -> review -> explicit install/publish. |
| ECS inventory | [`ecs-inventory-tool.md`](ecs-inventory-tool.md) | Commit-matched static Bevy localization evidence. |
| Optimization/reporting | [`optimization-and-reporting.md`](optimization-and-reporting.md) | Reproducible diagnostic artifacts, not gameplay authority. |
| Packaged assets | [`packaged-asset-guard.md`](packaged-asset-guard.md) | One composed tree plus byte contract for Android and installed desktop builds. |
| Tool policy | [`tool-authoring-policy.md`](tool-authoring-policy.md) | Requirements for adding/promoting tools. |

## Discovery

Prefer supported modal CLIs and root orchestration scripts:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools --help
(cd tools/ambition_sprite2d_renderer && python -m ambition_sprite2d_renderer --help)
uv run --project tools/ambition_music_renderer python -m ambition_music_renderer --help
uv run --script scripts/ecs_inventory.py --help
```

CLI help and tool-local README files are command authority. Documentation should
explain purpose, mutation/publish policy, and validation—not duplicate every
flag.

## Tool contract

An agent-facing tool should have:

- one obvious launcher and `--help`;
- explicit input/output/mutation behavior;
- deterministic output where practical;
- dry-run/output/backup modes for destructive authoring;
- actionable diagnostics and nonzero failure exits;
- tests for parsers/transforms and at least one end-to-end smoke path;
- an explicit publish/install step before generated output becomes runtime input;
- provider-qualified stable IDs rather than hard-coded engine content names.

`tools/experimental/` and nested tool checkouts are not automatically supported
runtime dependencies. Promote a workflow deliberately before documenting it as
the canonical front door.
