---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/systems/camera-and-visual-profiles.md
  - docs/concepts/generated-assets-audio.md
---

# Parallax backgrounds

Parallax is provider-owned visual content rendered from camera/room read models.
It must not leak into simulation, collision, or room-loading authority.

## Flow

```text
provider room/visual profile + asset IDs
    -> prepared provider content
    -> active room/camera read model
    -> ambition_render parallax presentation
    -> layered sprites/materials
```

Provider data owns named layers, assets, depth factors, repeats, tint, and room
bindings. Reusable render code owns layer realization, camera-relative motion,
asset fallback, and cleanup.

## Invariants

- Headless simulation does not create or require parallax entities/assets.
- Room/session scope cleanup removes old layers on transition/reset.
- Camera-relative offsets are derived; they are not persisted authority.
- Gravity/camera orientation policy is explicit where it affects the visual.
- Asset transport differences (desktop/web/Android) do not create gameplay forks.
- Generated visual assets have a reproducible source and explicit publish step.

## Validation

```bash
python scripts/agent_query.py "parallax background room profile"
python scripts/agent_query.py tests "parallax cleanup"
./run_tests.sh -p ambition_render -k parallax
./run_tests.sh -p ambition_content -k parallax
```

Visual feel still requires a visible smoke pass, but lifecycle and data-resolution
invariants should be headless tests.
