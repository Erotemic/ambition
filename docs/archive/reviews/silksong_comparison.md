---
title: "Silksong-style movement/combat comparison"
review_date: "2026-06-12"
status: "historical comparison refreshed as high-level gap index"
---

# Silksong-style comparison review

This document is design inspiration and gap-tracking, not an authoritative claim about Silksong internals and not a source-code audit. Older path-level tables in this review were tied to a 2026-05-26 source snapshot and have been removed because they drifted from the current crate layout.

## Current Ambition baseline

Ambition already has a broad platformer vocabulary:

- coyote time, jump buffering, dash buffering, variable jump, terminal fall caps;
- double jump / air jump, wall cling/jump/climb, ledge grab/getup/release/roll/attack/jump;
- directional dash, fast fall, glide, debug/ability flight, blink/precision blink;
- dodge roll, shield/parry, directional melee, pogo/down-attack bounce;
- charged and motion-input projectiles;
- explicit hostile hitbox entities, boss damageable volumes, and a canonical `HitEvent` transport for combat damage;
- movement op traces, headless simulations, and integration tests for regression coverage.

Use the current system docs for implementation ownership:

- `docs/systems/input-and-control-frame.md`
- `docs/systems/brain-driver.md`
- `docs/systems/gameplay-effects.md`
- `docs/mechanics/expressibility-checklist.md`
- `docs/mechanics/projectiles-and-motion-inputs.md`
- `docs/systems/transition-spawn-validation.md`

## Remaining feel gaps

These are still good design targets:

1. A general-purpose action buffer with explicit expiry and consume semantics beyond jump/dash.
2. A formal cancel-window matrix for attack, dash, blink, ledge, hitstun, recovery, and tool transitions.
3. Attack, pogo, projectile/tool, blink, and ledge-action buffers where the move set needs forgiveness.
4. Apex hang or held-jump sustain if the jump arc needs more polish.
5. Sprint/long-jump or momentum-preserving jump rules if the traversal kit needs more expressive speed tech.
6. Richer per-hit result metadata on top of `HitEvent`: stagger/poise, armor, elemental/status effects, hitstop, presentation hints, resource rewards, and rejection reasons.
7. A hurtbox/hitbox/collider audit so collision forgiveness is deliberate rather than incidental.

## Current combat direction

The old split damage-message model is no longer the current target. New combat work should build on the canonical `HitEvent` path and add richer result semantics only when a concrete mechanic needs them. Do not reintroduce parallel split damage transports.

## Maintenance rule

If this comparison becomes a code-grounded review again, refresh it against the current tree and keep path-level claims short. Prefer capability summaries and links to current system docs over long file-by-file inventories.
