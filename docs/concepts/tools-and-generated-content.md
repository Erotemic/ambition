---
id: tools-and-generated-content
status: current
aliases:
  - tools
  - generated assets
  - renderers
  - validators
  - author-time generators
related_docs:
  - docs/tools/index.md
  - docs/concepts/generated-assets-audio.md
  - docs/systems/asset-manager.md
last_verified: 2026-05-17
---

# Tools and generated content

## Definition

`tools/` contains author-time generators, validators, renderers, and experiments. Generated outputs should be reproducible or explicitly published/installed into runtime assets.

## Core invariant

Do not hand-edit generated or structured authoring files when a tool owns the workflow. Use the tool, validate the output, and document the tool if agents are expected to use it.

## Edit protocol

1. Read `tools/README.md` and `docs/tools/index.md`.
2. Use the package README for the specific tool.
3. Generate into tool-local output first when possible.
4. Publish/install into runtime assets explicitly.
5. Do not commit generated scratch output unless the runtime intentionally owns it.

## Common failure modes

- Documenting only one tool while the repo has many active generators.
- Installing assets from `tools/experimental/` without promoting the tool first.
- Hand-editing LDtk JSON instead of using `ambition_ldtk_tools`.
- Treating generated audio/sprites as untraceable binary blobs.
