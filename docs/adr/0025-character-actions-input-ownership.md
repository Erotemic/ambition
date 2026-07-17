# ADR 0025: Character actions — device-shaped input slots, deterministic sim-side action resolution

## Status

**Accepted; implemented** (the input-ownership seam — decisions 1–7 — is in;
only the P1 bindings source, P5 remap UX, and P6 cast authoring remain, and a
runtime playtest). Tracks `docs/planning/engine/character-actions.md`. A first
implementation pass over-claimed completion; a GPT-5.6 follow-up review
(2026-07-17) drove the remaining seams to done. This ADR records the design of
record.

Landed:

- The **`ActionScheme`** vocabulary (`ambition_entity_catalog::action_scheme`)
  and its per-character derivation (`ambition_characters::action_scheme`) from
  the body's live authorities.
- The **`ControlPrompt`** read-model (`ambition_sim_view`) the on-screen buttons
  render, following `ControlledSubject`.
- **`MovementAction`** + enum-indexed **`ActionEdges`** in `ambition_engine_core`,
  and the movement kernel consuming them through typed accessors
  (`InputState` re-keyed).
- Dedicated **`SandboxAction::Special`** slot; the `special_pressed =
  blink_pressed` alias is retired.
- The **shared resolver** (§Decision 6): the persona gate and `ControlPrompt`
  both call `derive_action_scheme` on the body's immediate authorities — the
  same resolution feeds gating and labelling, so they cannot drift (proven by a
  same-tick kit-swap test running both).
- **Sanctioned technique edges** (`ResolvedTechniqueEdges`): the gate routes a
  `Technique`-gated slot's device edge and clears the raw verb; Sanic's ball-dash
  consumes the `spin_dash` edge and the before-gate interception is deleted.
- The **canonical player's Special is a real move** (bubble_shield folded into
  the moveset), and the **touch overlay has a dedicated Special button**.
- The **menu Equip/Use provider** publishes the focused item's verb into
  `MenuConfirmPrompt`, which `ControlPrompt.menu_confirm` folds in.

Remaining (tracked in the plan doc): **P1** (`ActiveBindings` source-of-truth +
the live preset-split bug), **P5** (remap UX + gamepad glyphs + gamepad Special),
**P6** (cast authoring + `MoveSpec.display_name` + icons).

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

6. **One shared resolver, consumed by both gameplay and the prompt** *(landed)*.
   Two pure functions: `derive_action_scheme` builds the ordered
   `control slot → concrete action gate` scheme from the body's IMMEDIATE
   authorities, and is called at BOTH the persona gate
   (`gate_worn_player_control`) and the `ControlPrompt` producer — same function,
   same authorities, same tick, so the on-screen buttons and gameplay **cannot
   drift** (a same-tick kit-swap test runs both). The gate then APPLIES the scheme
   through `resolve_control_slots`, the per-slot dispatch: for EVERY combat slot
   (Attack/Special/Projectile/QuickAction) it routes a `Technique`-gated slot's
   device edge into `ResolvedTechniqueEdges` (the sanctioned content-technique
   seam, a **required component** of `ActorTechniques` so the sink is never
   missing) and clears the raw verb, keeps `Move`s, and strips the verbs the scheme
   doesn't own. A technique on a movement/Interact slot is **rejected** (surfaced
   for a debug-assert, never silently dropped) — those cannot fire until the kernel
   consumes actions (a per-mechanic follow-up, Decision 5). **`gate_worn_player_control`
   is therefore NOT retired**: it is the dispatcher's consumer and remains until
   that kernel re-key. `special_pressed = blink_pressed` is retired: the `Special`
   slot drives `special_pressed`; blink drives blink.

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
- **No UI/gameplay drift:** one resolution (`derive_action_scheme` on immediate
  authorities) feeds both the gate and the prompt every tick.
- **Honest kernel boundary:** `MovementAction` is exactly the locomotion verbs;
  the other kernel signals are named, not hidden in a grab-bag or a dishonestly
  wide enum.
- **Per-character controls:** possessing a body swaps the prompt to its scheme;
  a movement-only character (Sanic) shows only its real controls, never a
  phantom attack.

## Guardrails (landed)

- The derived types (`ActorActionScheme`, `ResolvedTechniqueEdges`,
  `ControlPrompt`, `MenuConfirmPrompt`) are recorded in the snapshot-coverage
  ledger as reviewed derived debt — a NEW unregistered type is a review event,
  not a silent stream of resolved actions (the `ControlFrame`-is-POD invariant).
- The scheme⇔behavior derivation guard (a slot is in the scheme iff the authority
  that gates its behavior provides it) — `ambition_actors::action_scheme`.
- The shared-resolver same-tick no-drift test: the real gate and the real prompt,
  run together across a kit swap, keep the visible slot and the executable verb in
  lockstep — `ambition_sim_view::control_prompt`.
- The per-slot dispatch matrix: `resolve_control_slots` unit tests assert absent /
  `Move` / `Technique` for Attack, Projectile, and Special, plus rejection of a
  technique on a non-combat slot — `ambition_characters::action_scheme`.
- The Bubble Shield production-schedule test: `AgentAction{special}` through the
  full sim raises `BodyShieldState.active` on the press tick — `player_bubble_shield`
  (app_it). And the menu Equip/Use provider is verified through its real
  sim-schedule registration (`install_menu_confirm_provider`), not a hand-chained
  pair — `kaleidoscope_app` tests.
- A plain melee edge is no longer the spin-dash content API (both directions) —
  `ambition_demo_sanic::ball_dash`.
