# ADR 0009: LDtk is the world-composition authoring source

## Status

Accepted and updated 2026-05-17.

## Context

Ambition started with code/RON-shaped room data because it was quick to test. That direction is superseded. The project now needs visual world authoring, editor roundtrip safety, entity metadata, platform packaging, and runtime ECS projection. LDtk is the current authoring source for world composition.

## Decision

Use LDtk as the canonical authoring source for world/level composition.

Rules:

- Do not hand-edit `sandbox.ldtk` JSON for semantic changes.
- Use `python -m ambition_ldtk_tools` for authoring/repair/validation/roundtrip workflows.
- Treat LDtk IntGrid/entity semantics as runtime data that feeds Bevy ECS and reusable engine types.
- Keep the runtime projection testable: validators and tests should catch missing graph links, bad loading zones, collision/category mismatches, and spawn repairs.
- Preserve static/embedded map paths needed by web and Android.

RON room/world manifests are historical. RON may still be used for tuning, save/settings, audio specs, and other non-world data.

## Consequences

Docs that mention RON-based world authoring must be archived or rewritten. Current world docs should point to `docs/systems/ldtk-world-composition.md`, `docs/recipes/ldtk-authoring.md`, `docs/tools/index.md`, and LDtk-related tests/tools.

## Current implications for agents

- Treat LDtk as world-composition source of truth.
- Use LDtk tooling rather than hand-editing JSON.
- Keep authored world data, runtime ECS projection, and presentation separate.
