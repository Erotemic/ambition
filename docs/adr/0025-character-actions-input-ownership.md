# ADR 0025: Character actions — device-shaped input slots, deterministic sim-side action resolution

## Status

Proposed; **partially implemented** (the vocabulary + the movement-kernel
re-key have landed; the shared resolver and the alias retirement are still
pending). Tracks `docs/planning/engine/character-actions.md`. Reviewed by
GPT-5.6 (2026-07-17); this ADR records the design of record so the remaining
seams don't drift from it.

Landed today (commits `8ab75893d` → `cf24c5558`):

- The **`ActionScheme`** vocabulary (`ambition_entity_catalog::action_scheme`)
  and its per-character derivation (`ambition_characters::action_scheme`) from
  the body's live authorities.
- The **`ControlPrompt`** read-model (`ambition_sim_view`) the on-screen buttons
  render, following `ControlledSubject`.
- **`MovementAction`** + enum-indexed **`ActionEdges`** in `ambition_engine_core`,
  and the movement kernel consuming them through typed accessors
  (`InputState` re-keyed). Parity: engine_core 329 movement tests + characters
  387 + actors 810 + the demo_sanic momentum oracles + repro_walls all green.
- Dedicated **`SandboxAction::Special`** input slot (blink is no longer its
  source; the alias itself is retired in a later step).

Pending: the **shared resolver** (below, §Decision 6) and the
`special_pressed = blink_pressed` alias retirement.

## Context

Input reached behavior through a **grab-bag `InputState`** of named booleans
(`jump_pressed`, `blink_pressed`, …) copied field-by-field through
`ControlFrame → ActorControlFrame → InputState`, then AND-ed with an
`AbilitySet` bool inside the kernel. The on-screen buttons were a **touch-only**
relabel of a fixed smash-vocabulary affordance table that never read the
controlled character's real moveset and never relabelled in menus. The player
brain hardcoded the slot→verb mapping (including `special_pressed =
blink_pressed`, because there was no special input). Adding a movement verb
meant editing three structs plus a kernel limb; a character's on-screen buttons
could disagree with what actually fired.

## Decision

1. **`ControlFrame` is the device-shaped wire format.** It stays a POD,
   fixed-size frame of device/slot edges — the thing `ControlFrameLatch` /
   `InputStream` latch and stream for netcode. Scheme resolution happens
   **sim-side and deterministically**, so a rollback re-resolves identically
   even when the scheme changed inside the rollback window (powerup, possession,
   form toggle). **Streaming resolved actions is forbidden.**

2. **`SandboxAction` is the slot vocabulary** (device-free semantic buttons).
   The physical-input → slot binding (`ActiveBindings`, planned) is the input
   layer's concern; one binding source must feed both the live `InputMap` and
   the on-screen glyphs.

3. **A per-character `ActionScheme` maps slot → action.** Each action carries a
   stable id, presentation (display text + optional visual), and an
   `ActionGate` (`Movement` / `Technique` / `Move` / `Interact`). The scheme is
   **derived state** — a pure function of already-snapshotted authorities
   (`AbilitySet` + moveset + `ActionSet` + content techniques) — so it is
   reconstructed by re-derivation on restore and never itself streamed or
   persisted. Combat presence is the **union** of the moveset and `ActionSet`
   (the canonical player's ranged/special come from `ActionSet` + the legacy
   pipeline, not its melee-only moveset).

4. **The movement kernel dispatches locomotion on `MovementAction`** through
   enum-indexed `ActionEdges` + typed accessors — not raw named booleans.
   Adding a locomotion verb is one enum variant plus its handling.

5. **Non-locomotion kernel signals stay explicit named fields.**
   `attack`/`interact`/`reset`/`shield` are genuinely read by the kernel
   (slash-recoil, ledge get-up / climb-confirm, the reset flag, shield deploy +
   dodge roll) but are **not** locomotion verbs. They are kept as named fields
   rather than dishonestly broadened into `MovementAction`; relocating each
   mechanic's ownership fully out of the kernel is a per-mechanic follow-up.

6. **One shared resolver, consumed by both the brain and the prompt** *(planned)*.
   A single resolution `physical binding → control slot → character scheme →
   concrete action gate`. The player brain uses it to drive behavior and
   `ControlPrompt` uses it to label — so the on-screen buttons and gameplay
   **cannot drift**. Retiring `special_pressed = blink_pressed` falls out of
   this: the `Special` slot's gate drives `special_pressed`; blink drives blink.

7. **Presentation reads a read-model, not the sim.** `ControlPrompt`
   (`ambition_sim_view`) is rebuilt each tick from the controlled subject's
   scheme; the touch overlay (and any future prompt surface) renders it and
   never queries the sim's live components. A slot the scheme lacks is hidden
   **and untappable** (one availability predicate feeds both visibility and the
   raw hit test).

## Consequences

- **Netcode-safe by construction:** device intent is streamed; the deterministic
  scheme resolution replays identically. No scheme-shaped data in the stream or
  snapshot ledger.
- **No UI/gameplay drift** once the shared resolver lands: one resolution feeds
  both. Until then, a scheme⇔behavior parity guard stands in.
- **Honest kernel boundary:** `MovementAction` is exactly the locomotion verbs;
  the other kernel signals are named, not hidden in a grab-bag or a dishonestly
  wide enum.
- **Per-character controls:** possessing a body swaps the prompt to its scheme;
  a movement-only character (Sanic) shows only its real controls, never a
  phantom attack.

## Guardrails (to add as the seams land)

- Poison-grep that no resolved-action type is streamed / snapshot-registered
  (the `ControlFrame`-is-POD invariant).
- The scheme⇔behavior parity test (a slot is in the scheme iff the authority
  that gates its behavior says the body has it).
- The shared-resolver test: the value the brain resolves for a slot equals the
  action `ControlPrompt` labels it with.
