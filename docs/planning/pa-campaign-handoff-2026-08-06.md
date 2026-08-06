# PA campaign handoff — 2026-08-06

Jon, verbatim: **"Yes do the architecture task PA1-PA4."** The spec is
[`engine/participant-action-system.md`](engine/participant-action-system.md)
(§9 carries dated landed/owed annotations); binding/remap detail is owned by
[`engine/character-actions.md`](engine/character-actions.md) P1/P5. This file
is the operational handoff: exactly what landed today, exactly how to finish,
and which forks are decisions rather than work.

## What landed today (verify with `git log`, don't re-derive)

| commit | what |
|---|---|
| `746a95e25` | `BindingRecipe` + `rebuild_maps_from_recipes` + `sync_primary_recipe_from_settings`: every seat's map is built from a declared recipe, engine-owned rebuild. Killed the app-only `single_mut()` preset resync. |
| `cf868687e` | `SeatActiveDevices` (per-seat device fact, `machine()` projection) replaces `ActiveInputKind` + `ActiveInputMethod`; real vendor detection (`gamepad_style_of`); glyph stack moved to `ambition_input::glyphs`. |
| `8839e9fc7` | ONE gamepad label table (`glyphs::button_label(button, style)`, `PhysicalControl::label_for`); `GAMEPAD_MAP`/`action_label`/`movement_label` deleted as dead. |
| `acc50d48c` | Touch overlay is the PRIMARY seat's in both directions (pressed-state read; virtual-binding insertion). |
| `b242c7e51` | Pre-existing versus red fixed: fixture now composes `declare_versus_experience_scope` + the shell's now-public `release_departed_experience_state`. |
| `7a957d8b2` | Pre-existing monolith fixture break (`BrainSnapshot.movement_tuning`) repaired. |

## PA1 remainder — the override model, specced to the keystroke

**Goal:** a persisted per-action binding override, folded into the recipe, so
"change one binding" (not just "change preset") reaches behavior + glyphs +
touch in the same frame. This is `character-actions.md` P1's last clause; the
rebind-capture UI is P5 and comes later (see decision D3).

The seams, all existing:

- `BindingRecipe` (`crates/ambition_input/src/bindings.rs`) gains an
  `overrides` field. ⚠ it derives `Copy` today — overrides carry a `Vec`, so
  it drops to `Clone` (its two consumers, the spawn sites in
  `crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs`
  and the sync system beside them, take one `.clone()` each).
- `BindingRecipe::build()` applies overrides AFTER the base map. Selective
  replacement, not `clear_action` (which nukes both device classes): leafwing
  0.20 has `InputMap::remove(&action, input)` (input_map.rs:991). To remove
  "the existing keyboard bindings for this action", enumerate them via the
  projection that already exists — `ActionBindings::from_map(&map)
  .controls(&action)`, filter `PhysicalControl::Key(_)` — then `remove` each
  and `insert` the override key. Same shape for gamepad with `Button(_)`.
- Persistence: `ControlSettings` (`crates/ambition_input/src/settings.rs`)
  gains a `#[serde(default)]` overrides field — serde-default keeps every
  existing settings file loading. Encoding is decision **D1** below.
- The sync path already exists: `sync_primary_recipe_from_settings` compares
  a wanted recipe against the live one and writes only on change;
  `rebuild_maps_from_recipes` (registered in the host's `InputSet::Collect`
  chain, `crates/ambition_platformer2d_host/src/lib.rs`) does the rest —
  carrying the seat's pad association and resetting `ActionState` on a real
  change. Extending the wanted-recipe computation with overrides-from-settings
  is the whole wiring.
- Exit test (the honest one): TWO participants live, an override lands in
  settings → the primary's map binds the new key (behavior), `SeatBindings`
  reports the new label the same frame (glyphs), and the couch seat is
  untouched. Put it beside `a_preset_change_reaches_the_primary_beside_a_
  second_seat` in `input_systems.rs`'s test module.
- Also owed under PA1: per-seat presets for secondary seats (couch seats are
  hardcoded `GamepadOnly` today). The recipe is the seam — a seat's recipe
  just needs a source other than "the one settings preset". Product half is
  decision **D2**.

## PA2 — contexts, surface by surface

The pattern is established: an owning surface declares/retracts a
`ContextClaim` (`crates/ambition_input/src/participant.rs`), resolution is
per-seat (`SeatInputContexts`), and `declare_in_session_input_contexts`
(`input_systems.rs`) is the worked example (dialogue + cutscene claims).

- **Pause** — ⚠ read the deliberate exception first: the comment right under
  `declare_in_session_input_contexts` explains pause is NOT a claim today
  because `GameMode::Paused` stops the world and the paused path writes a
  MENU frame into `ControlFrame`. Migrating it needs decision **D5** (who
  pauses, who navigates) OR preserve today's semantics behind a claim
  (any-seat pause, world stop, primary navigates) and leave D5 as data.
- **Inventory / specialized menus** — the kaleidoscope
  (`game/ambition_app/src/menu/kaleidoscope_app.rs`) and grid backend
  (`.../grid_backend.rs`) already publish cues under a minted context id;
  what's missing is the claim-declared ownership replacing `GameMode` checks.
- **Dev overlays** — only where they should capture product input.
- **Dialogue world-stop** — ⛔ BLOCKED on decision **D4**
  (`awaiting-maintainer-decision.md`). Work around it.
- **Loading/retry** — ⛔ deliberately NOT migrated: moving its raw read into
  `Consume` creates a schedule cycle through load-presentation + shell
  sequence sets. Leave it; the spec's §8 lists it.

## PA3 — the standard UI module

Seed: `ambition_ui_nav` (it already owns `RowPress` — press/move/release with
drag-cancel — and `DialogChoiceSlot`). The convergence list:

- The kaleidoscope keeps a PRIVATE copy of the RowPress idea (queue G11
  residue: "the kaleidoscope keeps its own copy; other menus still activate
  on press"). Converge them; the row identity is the INDEX, not the entity
  (windowed lists rebuild rows between press and release — the trap is
  documented at the kaleidoscope's press store).
- One semantic activation event for controller submit, virtual-touch submit,
  mouse click, direct touch.
- Directional repeat/focus/wrap behind the module; `MenuControlFrame`
  consumers keep their nav-axis semantics (locked decision 2 in
  character-actions.md — consumption unchanged).
- ▢ noted in `input_systems.rs`: `MenuFrameCutsceneSkip` vs `MenuNavConsume`
  fragmentation — one `MenuFrameConsume` set is "a rename plus a widening
  with a behavioural consequence at the menu-backend switch, so it is a
  decision, not this edit." Take it deliberately or leave it.

## PA4 — the cue/glyph/touch contract

- `UiCue` (`crates/ambition_input/src/cues.rs`) is deliberately label-only;
  its own doc says fields (glyph, enabled, interaction, touch hint) "grow
  HERE as fields when a consumer exists." The first real consumer: give
  `ControlPrompt.binding` the SEAT'S vendor spelling —
  `PhysicalControl::label_for(style)` exists, `SeatActiveDevices::for_seat`
  has the style; the prompt writers are in
  `crates/ambition_sim_view/src/control_prompt.rs`.
- Touch layout still knows fixed `ControlSlot` positions
  (`crates/ambition_touch_input/src/layout.rs`) — the contract makes
  visibility/labels/hit-testing read only cues.
- The sanic legend (`game/ambition_demo_sanic/src/lib.rs`, "START … JUMP")
  is pinned to preset 0 because it renders at room GENERATION with no
  settings access — the fix is a presentation-time label fed by
  `SeatBindings`, i.e. this contract. The ⚠ comment at the site says so.

## Decisions (see the section Jon answered, if he has)

- **D1 (technical, recommendation ready): override encoding.** `KeyCode` is
  not `Serialize` under ambition_input's thin bevy features. (a) depend on
  `bevy_input` directly with its `serde` feature — the crate already uses
  exactly this direct-dep pattern for `bevy_window`, and `bevy_input` has a
  `serde` feature flag; typed, total, no drift table. (b) store key-NAME
  strings and write a reverse lookup — a second table of the class PA1 just
  spent three commits deleting, and `key_name` is lossy ("?" on miss).
  (c) an owned mirror enum — the worst drift surface. **Recommend (a)**;
  proceed with it unless Jon objects.
- **D2 (product): per-seat settings.** Do couch seats get their own persisted
  preset/overrides/deadzones (profiles picked at the select screen?), or do
  machine-wide settings apply to every seat? Blocks the "per-seat presets"
  half of PA1 only; the override model lands on the primary regardless.
- **D3 (sequencing): P5 rebind UX timing.** Model now (PA1), capture-UI after
  PA3 so the rebind rows are built ON the standard menu module rather than
  rebuilt twice. Proceeding on that unless redirected.
- **D4 (product, already filed): dialogue world-stop default** —
  `awaiting-maintainer-decision.md`. Per-seat contexts exist; on a couch one
  player talking freezes everyone. Keep world-stop / go per-seat / policy per
  experience.
- **D5 (product): couch pause semantics.** Who may pause, and who navigates
  the pause menu. Default preserved by the migration if undecided: any seat
  pauses, world stops, primary navigates.

## Traps this session hit (so the next one doesn't)

- ⛔ **The planning docs LAG the tree.** Five "open" candidates this morning
  were already landed. Grep the tree before believing any ▢ — including in
  this file.
- **Feature-gated test suites lie when run bare:**
  `cargo test -p ambition_input` needs `--features input` (93 vs 55);
  `cargo test -p ambition_touch_input` needs `--all-features` (44 vs 4).
- The gate is `cargo check -p ambition_app` (~25s), never per-crate; the 11s
  contracts job needs `--check` to enforce
  (`python3 scripts/check_absence_contracts.py --check`, 25/25 at handoff).
- Jon, verbatim, 2026-08-06: **"Don't bother with full verification if you
  ran targeted ones."** Targeted green = done.
- `MODULES.md` is GENERATED from `//!` headers — after editing one, run
  `python3 scripts/modules_md.py --write` (a doc-verification agent caught
  a stale row this session).
- A worktree subagent may fork from a STALE base — this session's forked from
  three days back. Check `git -C <worktree> log --oneline -1` before
  integrating; `git apply -3` merges its diff, then re-verify targeted.
- rust-analyzer diagnostics were noisy all session (feature resolution);
  trust cargo, not the panel.
- This tree has CONCURRENT sessions. Re-read `git log` before assuming HEAD,
  and expect `.llm_resource_tally/` churn (never `git add -A`).
