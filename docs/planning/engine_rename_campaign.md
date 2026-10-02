# Engine restructuring candidates

**Status:** candidates with triggers, not a flag-day refactor. The retired
`Sandbox*` crate naming is guarded by `python3 scripts/check_retired_crate_names.py`.
The active actor carve is
[`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md).
Couch input assignment (`InputAssignmentPolicy` in
`crates/ambition_input/src/sources.rs`) and participant/view architecture
([`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md))
are built and owned elsewhere.

Take a candidate only when its trigger fires. None is queued.

## Guiding rules

- Prefer a decomposition that establishes a clear owner and improves dependency
  direction. Do not create generic layers because several unrelated concepts are
  low-level.
- Naming follows ownership. `actor_monolith` and `shared_tangle` are warning
  labels while their mixed contents remain. `combat`, `core`, `characters` and
  `runtime` are existing names, not proof of coherent capabilities. Do not
  mass-rename to finished-sounding names (`actors`, `platformer`). First move the
  state, behavior, lifetime and installation that share an authority, then name
  the API.
- A source-only rename preserves serialized, schema and wire identity unless a
  separately reviewed migration changes it.
- Measure a carve by the domain that left, not by line counts or the monolith's
  `[dependencies]` count (a carve raises that count).
- Do not delay product work to prove every future engine configuration.

## Candidates

| candidate | current state | trigger / shape |
|---|---|---|
| Split persistence | `ambition_persistence` only | When settings or save work is next touched: `ambition_user_settings` (display, audio, accessibility, controller preferences, settings path) and `ambition_game_save` (`AmbitionGameSave`, progression, persistent world state, autosave, save versions). Quest data stays with the save until a real quest architecture exists. |
| Decompose feel tuning | `Platformer2dFeelTuningMonolith` in `crates/ambition_combat/src/feel.rs` mixes concerns | Split fields into domain resources (`MovementFeelTuning`, `CombatFeelTuning`, `TransitionTuning`, `TimeFeelTuning`) as those systems are modified. Not a standalone campaign. |
| Generic snapshot vocabulary | no `ambition_snapshot` crate | Move only generic deterministic snapshot machinery (traits, deterministic readers/writers, canonical encoding, hashing). `ControlFrame` and `InputFrameMode` stay platformer concepts. Rerun the dependency census before any broader low-level crate. Not a miscellaneous kernel. |
| Split generic from platformer input | `ambition_input` holds the whole input model; `ControlFrame` is in `ambition_platformer2d_core` with a `TODO(compat-remove)` re-export in `ambition_input`; `Platformer2dInputActionMonolith` is one closed enum of shell and gameplay actions | First fix the identity model (`InputSourceId`, `ParticipantId`, `SessionSeatId`, `ControlChannelId`). Then `ambition_input` keeps sources, assignment, join/leave, semantic actions, bindings, contexts, action state and menu routing; a platformer input crate takes `ControlFrame`, `InputFrameMode`, movement/aim, verbs, presets and action-to-frame translation. Separate shell, platformer and game actions. Moderate risk: it crosses devices, participants, session preparation, rollback input and controlled actors. |
| Shrink the actor monolith by destination | live; see the decomposition doc | body/movement integration → platformer simulation; character projection → characters; combat adapters → combat; world/session orchestration → runtime or world; presentation adapters → presentation; controlled-body input translation → platformer input. Choose each boundary by semantic ownership. |
| Untangle `shared_tangle` | `construction/` and `gameplay_presentation/` are the largest strands | Extract a strand only when its authority boundary and dependency direction are understood. The goal is removing dependency knots, not smaller crates. |
| Separate sprite format from runtime | `ambition_sprite_sheet` holds schemas and runtime pieces | When substantive sprite-runtime work makes the seam valuable: sheet format crate, a sprite runtime crate (playback, asset resolution), and domain adapters. |
| Replace the quest model | underdeveloped | A redesign establishes authored definitions, objectives, progression state, event observation, rewards, validation, save integration and multiplayer authority. Do not extract the current quest code for crate symmetry. |
