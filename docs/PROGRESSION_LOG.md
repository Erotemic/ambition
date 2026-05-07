# Progression log

This log is a compact map of major architecture and feature waves. It is not a full changelog.

## Early direction

- Ambition began as a code-first platformer/metroidvania movement sandbox.
- The first design law became: the game should be fun as raw collision boxes.
- The story premise centered on an AI-like player discovering agency, embodiment, mathematics, collaboration, and ethical compromise.

## Bevy and engine split

- The project moved from earlier prototypes into a Bevy 0.18 sandbox.
- The engine was initially treated as backend-neutral, but that constraint was later relaxed.
- Current decision: `ambition_engine` may depend on Bevy and Bevy-adjacent crates when they provide robust primitives.

## Data-driven rooms and generated audio

- Room layout, ability flags, movement tuning, audio specs, and room transitions moved toward RON.
- Generated audio shifted toward FunDSP-backed startup WAV rendering.
- The canonical sandbox asset path became `crates/ambition_sandbox/assets/ambition/sandbox.ron`.

## Input, pause, and game modes

- Leafwing Input Manager replaced bespoke keyboard polling.
- Gameplay input is converted into `ControlFrame` only when the relevant game mode allows it.
- Bevy `GameMode` states now distinguish playing, paused, dialogue, room transition, and cutscene intent.
- Presentation/debug previews were also gated so paused input does not visually leak into gameplay previews.

## Interaction/hazard/actor skeleton

- Engine-side vocabulary was added for actors, health, damage volumes, hitboxes, hurtboxes, interactables, pickups, chests, breakables, respawn policies, kinematic paths, enemy/boss placeholders, and debug/destination labels.
- Rooms now distinguish collision blocks from authored gameplay objects.

## Feature basement wave

- The central hub gained a basement feature wing.
- Focused rooms test hazards, enemies, boss patterns, breakables, treasure/pickups, and NPC dialogue hooks.
- Loading zones render destination labels for debugging.
- This is a proving ground, not final content.

## Input-feel pass

- Existing jump buffer and coyote time were preserved.
- Dash buffering and sandbox-side interaction buffering were added for more forgiving feel.
- HUD/inspector visibility improved for buffer tuning.

## Crate foundation pass

- `seldom_state` was added as the intended per-entity state-machine foundation.
- `bevy_asset_loader` was added as a conservative asset-collection/loading foundation while preserving embedded fallback startup.
- `insta` and `proptest` were added for lightweight snapshots and property tests.

## Documentation source-of-truth pass

- README was rewritten as a stable project portal.
- `CURRENT_STATE.md`, `GOAL_STATE.md`, `AGENT_HANDOFF.md`, ADRs, and story/mode notes were added so future agents can distinguish current truth from historical notes.

## Sandbox-first audit + mechanic completion (2026-05)

- Reaffirmed the sandbox-first priority: every gameplay component lands in
  test-arranged form before being assembled into a story-arranged slice.
- Engine mechanics rounded out: Glide / slow-fall ability with `glide_fall_speed`
  + `glide_air_accel`; per-player promotions of `damage_multiplier` /
  `invincible` / `mana: ResourceMeter` from the sandbox runtime to the engine
  `Player`; `Player::was_riding_platform` scratch flag for diagnostic logging.
- Sim → presentation seam hardened: `PlayerDiedMessage` replaced
  `runtime.player_died_pending` bool; multi-frame
  `tests/scripted_gameplay.rs` integration test pins the seam under
  MinimalPlugins. ADR 0012 events refactor slices 1-5 audit confirmed
  zero direct presentation calls in `app.rs`.
- Authoring ergonomics: LDtk × audio cross-validation warns on unknown
  `music_track` ids; map menu gained zoom controls (`+` / `−` / `0`)
  and full room-name labels; quest log moved into its own
  `QuestPanelText` UI surface; verbose debug HUD trimmed of
  inspector-redundant fields.
- Coverage: BodyShape::fits_at + ResourceMeter + wall-jump start-position
  proptests added; mob_lab lock-wall teleport repro test pins the
  geometry that the `body_is_side_contact` predicate covers.
- CI: GitHub Actions workflow with engine + sandbox + headless-binary
  smoke; PR template wires the FEATURES.md update into a checklist.
- Music director: adaptive resolver iterates all encounter bindings
  (instead of hardcoding mob_lab) so future encounter cues drop in
  via a new `EncounterMusicBinding` entry.
- Settings: per-controller-profile filter defaults (Xbox 360 widens
  deadzones + trigger band; PlayStation tightens deadzones).
