# ADR 0015: LDtk tileset rendering is presentation-only

## Status

Accepted direction; implementation remains incremental.

## Decision

LDtk Tiles layers are presentation. LDtk IntGrid layers remain gameplay grid truth. LDtk Entities represent distinct authored identities.

```text
Tiles    -> visuals only
IntGrid  -> grid gameplay such as collision/hazard/water/ladder
Entities -> authored gameplay identities
```

Rendering tiles should not change collision truth. If tiles and gameplay layers disagree, gameplay wins and authoring tools should flag the mismatch.

## Context

Ambition historically drew colored collision rectangles while LDtk authoring matured. That was useful for debugging but is not the intended visual identity. The durable rule is layer separation, not a particular patch sequence.

## Consequences

- Visual tiles can be rendered by Bevy/LDtk integration or Ambition presentation code.
- Collision/debug overlays should be toggleable and not treated as shipping visuals.
- Coordinate-frame reconciliation between LDtk world coordinates and Ambition active-area coordinates is spatial-review-sensitive.

## Current implications for agents

- Do not make Tiles authoritative for collision.
- Do not hide IntGrid validation just because a visual tile looks correct.
- Use `docs/tools/ldtk-tools.md` and `docs/recipes/ldtk-authoring.md` for LDtk edits.
- Mark coordinate-frame seams with `AMBITION_REVIEW(spatial): ...` when they need later visual verification.
