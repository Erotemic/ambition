# Character actions — the slot→action control seam + context-sensitive control prompts

> **State:** PLANNED (2026-07-17). Drafted by Opus 4.8, reviewed and revised by
> Fable 5 in session with Jon; all shape decisions below are locked with Jon.
> Nothing has landed yet. Supersedes the touch overlay's affordance relabel
> path as the design of record for "context-sensitive on-screen buttons."
>
> **Executing-model sign-off:** Fable 5 (revision + this document).

## The law (target)

A **character owns its actions**, and the **action is the gate**: input never
pokes the movement kernel or combat directly — it fires one of the controlled
subject's declared actions, and the action's gate is what reaches the kernel
(`MovementAction`), a content technique, the moveset runtime, or world
interaction. The same declared action carries its presentation (display text,
optional visual), so the on-screen buttons and the simulation **cannot
disagree by construction**: the prompt asks the same resolver the brain calls
on press.

The touch UI (and any future prompt surface) is a dumb presenter: it asks
"what am I controlling?", receives an ordered list of labeled actions with
live binding glyphs, and only **arranges and presents**.

## Why

Today the on-screen buttons are a shallow touch-only relabel of a fixed
smash-vocabulary table (`PlayerAffordances`,
`crates/ambition_actors/src/affordances/`), computed from physics state only.
They don't know the controlled character's real moveset, never relabel in
menus, and the button set is hardcoded. Underneath, jump/dash/blink are
hardcoded bools copied through three frame structs
(`ControlFrame → ActorControlFrame → InputState`) and AND-ed with an
`AbilitySet` bool inside the kernel; Sanic's ball dash — the one real
per-character movement technique — is an ad-hoc content system that intercepts
`ActorControl` fields in a fragile schedule window
(`game/ambition_demo_sanic/src/ball_dash.rs`,
`.after(tick_player_brains).before(gate_worn_player_control)`).

Wanted: hook a controller to a character and the buttons name **that
character's** moves; open a menu and the buttons say **Equip / Use / Back**,
not Jump; remap a button and every surface reflects it.

## The model

```text
device (kbd / pad / touch)
  │
  ▼  ActiveBindings  — ONE remappable source: physical input → SLOT
  │                    (gameplay slots AND menu slots; feeds InputMap + glyphs)
  ▼  SandboxAction   — the SLOT vocabulary (already exists; finite, like thumbs)
  ▼  ControlFrame    — POD per-tick slot edges; the netcode wire format (latch/stream)
  │
  │            ═══ sim-side, deterministic ═══
  ▼  ActionScheme (component on the controlled body; follows ControlledSubject)
  │     ordered slot → ActionSpec { id, display, visual, gate }
  │     resolve(slot, body ctx) → the action that WOULD fire now
  │        gate: Movement(MovementAction) ──► kernel (InputState keyed by action)
  │              Technique(id)            ──► content system via sanctioned edges
  │              Move(verb)               ──► moveset runtime (directional chain)
  │              Interact                 ──► world interaction (proximity resolver)
  │
  ▼  ControlContext — ONE owner of input: Character(entity) | Menu(ctx) | Dialogue
  ▼  ControlPrompt (ambition_sim_view read-model, ordered):
  │     per shown slot: { glyph(live binding), label(resolved action), visual,
  │                       available, pressed }
  ▼  Touch UI arranges & presents. Zero mode branches, zero sim reach.
```

Two remap layers fall out naturally:

- **physical → slot** — global, device-level, persisted (`ActiveBindings`;
  presets become default binding tables).
- **slot → action** — per character, *data* (the scheme). Sanic's attack slot
  maps to `spin_rev`, the protagonist's to `swipe`. No field-stealing, no
  post-hoc gating.

## Invariants (binding)

1. **The wire format is device-shaped.** `ControlFrame`
   (`crates/ambition_engine_core/src/control_frame.rs`) stays a POD,
   fixed-size slot-edge frame — it is what `ControlFrameLatch`/`InputStream`
   latch and stream. Scheme resolution happens **sim-side and
   deterministically**, so rollback re-resolves identically even when the
   scheme changed inside the rollback window (powerup, possession, form
   toggle). Streaming *resolved actions* is forbidden. (Worth an ADR when P3
   lands.)
2. **One resolver, shared.** The function the brain uses to resolve a slot
   press is the same function the prompt producer calls to label the slot.
   The old affordances module's "Migration target" note wished for exactly
   this; this design is that migration.
3. **One bindings source.** The live `InputMap` build and every glyph render
   read the same `ActiveBindings` (kills the real
   `SandboxDevState.preset_index` vs
   `UserSettings.controls.keyboard_preset_index` split — today the settings
   toggle changes glyphs but not bindings, and nothing re-applies to a live
   player).
4. **Menu nav is an axis, never an action.** Directional navigation
   (stick/dpad/drag-scroll) keeps its `MenuControlFrame` nav-axis semantics.
   Menu *commands* (confirm/back/page) are slots — `MenuSelect`, `MenuBack`,
   `MenuPage*` already are `SandboxAction`s — and join `ActiveBindings` +
   the presentation contract, but their **consumption path is unchanged**
   (`MenuControlFrame` consumers, `MenuNavConsume` ordering web, touch fold
   pins all stay as-is).
5. **Determinism:** schemes are ordered `Vec`s; resolution is pure; no
   `HashMap` iteration in sim (existing poison-grep contract applies).
6. **UI arranges only.** The touch overlay renders `ControlPrompt` entries at
   layout positions keyed by SLOT (stable positions = muscle memory; labels
   change, buttons don't move). Slots with no action in the current scheme are
   hidden — Sanic shows no Shot button. Raw-touch hit testing
   (`layout.rs::touch_action_at_position`) must follow visibility.

## What this retires (full replacement, no bridges)

- `crates/ambition_actors/src/affordances/` — `PlayerAffordances`, the fixed
  `*Variant` enums, `compute_player_affordances`, and the touch overlay's
  `update_button_verb_from_affordances`. **Honesty note:** the *contextual
  resolve* is inherited, not deleted — directional attack labels come from the
  moveset's `move_for_directional_verb` chain, interact labels from the
  proximity classifier (`NearestInteractable` survives as the interact
  action's resolver), technique labels from technique state. `glyph_for` and
  the device-detection live on, re-homed with `ActiveBindings`.
- `gate_worn_player_control`
  (`crates/ambition_actors/src/avatar/starting_character.rs:388`) — a body
  without an action simply has no scheme entry; nothing to strip after the
  fact.
- The ball-dash `ActorControl` interception window
  (`game/ambition_demo_sanic/src/lib.rs:850-855`) — replaced by a scheme-
  declared technique consuming sanctioned edges. **Honesty note:** the chord
  logic (crouch-held + attack edge + crouch-release) *stays content code*; the
  scheme gives it identity, its slot claim, labels, and a sanctioned edge
  feed — it does not absorb the state machine.
- The hardcoded `special_pressed = blink_pressed` player-brain aliasing
  (`crates/ambition_characters/src/brain/player.rs:156`) and the per-verb
  bool copies around it — replaced by slot→scheme resolution.

## Phases

Each phase compiles, is tested, and lands as its own commit(s) on main.
`engine_core` struct changes are batched inside P3 (one full-rebuild window,
~10min build discipline).

### P0 — Action vocabulary + scheme derivation *(no behavior change)*

- `ActionSchemeContract` in `crates/ambition_entity_catalog` **next to
  `MovesetContract`** (`src/lib.rs:707`): ordered
  `Vec<ActionSpec { id, slot, display_name, visual: Option<VisualId>, gate }>`.
  String `ActionId` (matches `MovesetContract.verbs` keys) + well-known
  constants; `ActionGate::{Movement, Technique, Move, Interact}`.
  `MovementAction` enum + `ActionEdges` go in `ambition_engine_core` (the
  kernel will consume them in P3).
- `display_name: Option<String>` added to `MoveSpec`
  (`entity_catalog/src/lib.rs:375`) and to technique/action specs, authored in
  catalog RON / prefabs; fallback = title-cased id (`sandbag_swat` → "Sandbag
  Swat"). (Rust `ron` parse test per parser-drift rule if RON schema grows.)
- Runtime wrapper `ActorActionScheme(ActionSchemeContract)` component in
  `ambition_characters`, exactly the `ActorMoveset(MovesetContract)` pattern
  (`crates/ambition_combat/src/moveset/mod.rs:198`). Attached wherever
  movesets are attached (`combat/moveset/prefabs.rs:635` seam + boss/NPC
  spawn paths) so **possessed bodies carry schemes too**.
- Pure builder `derive_action_scheme(abilities, moveset, techniques)` — from
  the SAME authorities that gate behavior today: `AbilitySet`
  (`engine_core/src/abilities.rs:20`) for movement actions,
  `ActorMoveset`/`ActionSet` for combat, content-registered techniques.
- Tests: derivation per capability profile (full-kit protagonist vs
  movement-only Sanic vs boss); deterministic ordering; poison test (empty
  scheme renders empty, never smash defaults); **scheme⇔behavior parity**
  (scheme has attack ⇔ old path would fire melee) — this is the guard for the
  P2 window where prompts are scheme-driven but behavior is still old-path.

### P1 — `ActiveBindings`: one source of truth *(fixes a live bug)*

- New resource: per-slot physical bindings (keyboard + gamepad + touch),
  defaults from the 4 presets (`ambition_input/src/presets.rs:216
  input_map()`); **menu slots included** (`MenuSelect`/`MenuBack`/
  `MenuPage*` bindings move into the same table).
- Collapse the split: `SandboxDevState.preset_index`
  (`ambition_dev_tools/src/lib.rs:111` — written by nothing) vs
  `settings.controls.keyboard_preset_index` — both replaced by
  `ActiveBindings` seeded from settings.
- Re-apply system: bindings change ⇒ rebuild `InputMap` on live players
  (today `attach_player_input_components`,
  `actors/src/schedule/input_systems.rs:74`, only inserts
  `Without<ActionState>` — a preset change never reaches a live player).
- `glyph_for` re-pointed at `ActiveBindings` (keyboard branch already
  preset-derived; gamepad branch stays table-driven until P5 derives it from
  the live map).
- Persistence: bindings serialize in `ControlSettings`
  (`ambition_input/src/settings.rs:270`).

### P2 — `ControlContext` + `ControlPrompt` + touch UI renders it *(first visible win)*

- `ControlContext` resource — ONE owner of input:
  `Character(Entity) | Menu(MenuContextId) | Dialogue` — derived from
  `GameMode` + `ControlledSubject`. Single publisher; no producer race.
- `ControlPrompt` read-model in `ambition_sim_view` (the
  `PlayerHudFacts`/`BossFrameIndex` pattern): ordered entries
  `{ slot, glyph(live), label(resolved), visual, available, pressed }`.
  Character producer resolves each slot through the subject's scheme
  (shared resolver, invariant 2); labels for directional moves delegate to
  `move_for_directional_verb`, interact to the proximity classifier.
- Touch overlay rewritten as a `ControlPrompt` presenter: per-slot buttons at
  fixed layout positions, hidden when the scheme has no action for the slot;
  label + glyph + pressed straight from the prompt. Deletes
  `update_button_verb_from_affordances` and the static label table
  (`touch_input/src/bevy_plugin.rs:818`, `layout.rs`). The crate's
  `ambition_actors::affordances` reach goes away (long-flagged decoupling
  smell).
- **Payoff already here:** possess a body → buttons change, because the
  scheme is real — even though input still flows the old path (guarded by the
  P0 parity test).

### P3 — The seam refactor: kernel consumes actions *(parity-gated)*

- **Parity harness FIRST** (bold-refactor discipline): headless movement
  traces for jump / dash / blink / fast-fall / fly-toggle / wall-jump /
  drop-through + Sanic spin-dash/ball across the refactor; gate = compiles +
  parity holds.
- `ControlFrame` **unchanged in shape** (invariant 1). The brain
  (`characters/src/brain/player.rs:58`) stops per-verb bool copying: it
  resolves slot edges through `ActorActionScheme` and writes action-keyed
  edges into `ActorControlFrame` (`characters/src/actor/control.rs:152`).
- `InputState` (`engine_core/src/movement/input.rs:18`) re-keyed by
  `MovementAction` edges; both bridges follow
  (`actor/control.rs:297 to_input_state`,
  `actors/src/features/ecs/attack.rs:50`). Kernel limbs read
  `input.action(Jump).pressed` at the existing gate sites
  (`movement/abilities.rs:40/43`, `movement/control.rs:16` blink,
  `movement/simulation.rs` jump branches, `abilities.rs:187 apply_dash`).
  `AbilitySet` remains the tuning/capability source (air-jump/dash counts);
  availability = scheme presence + ability permit.
- Techniques: resolve writes sanctioned per-technique `ActionEdges`; content
  systems consume them in `GameplayEffects`. Ball dash + Sanic form toggle
  migrate; the interception window and `gate_worn_player_control` are
  deleted. Fast-fall **stays an axis-derived gesture edge** (double-tap-down
  is not a bindable slot), resolved where it is today.
- AI/boss brains: `to_input_state` path gets the same keying; brain-authored
  `ActorControlFrame`s write action edges directly (they already know their
  intent — no scheme lookup needed on the AI side, the scheme is authoring
  data for what a body CAN do).
- Write the ADR: input wire format is device-shaped slots; action resolution
  is deterministic sim-side.

### P4 — Menu presentation providers

- Menu contexts implement the same slot→label provider the character side
  uses: inventory grid/cube publish the selected item's verb
  (`MenuPageAction` + `action_label` "Equip"/"Use",
  `game/ambition_app/src/menu/model.rs:314`) on the `MenuSelect` slot, Back /
  Page on theirs; dialogue publishes Advance/Choose/Close from
  `confirm_or_advance` state (`ambition_dialog/src/runtime.rs:377`). Touch UI
  relabels for free — it just renders `ControlPrompt`.
- Shell pause menu: migrate off direct key reads (`shell_action_edges`,
  `game_shell/src/pause_menu.rs:136`) onto the shared bindings source, and
  give it a provider — otherwise it's the one surface that can still lie.
- Consumption unchanged everywhere (invariant 4).

### P5 — Full remap UX

- Settings-menu rebind-capture rows (listen-for-next-press) writing per-slot
  overrides into `ControlSettings`; persistence in `UserSettings`; conflict
  handling (steal-with-warning is fine pre-release).
- Gamepad glyphs derived **from the live `InputMap`** (replacing the parallel
  hardcoded table, `affordances/devices.rs:201` — doc there already warns it
  drifts); finish gamepad `ActiveInputMethod` detection + `GamepadKind`
  vendor inference (`devices.rs:110` TODO stub) so glyph style actually
  switches on a pad.
- Touch UI reflects rebinds live (already true via P1/P2 plumbing).

### P6 — Author the cast

- **Sanic:** jump, spin-dash (Technique, attack slot), ball/form toggle
  (Technique, utility slot), boost/rev labels; no attack/shot slots shown.
- **Mary-O:** jump, run, fireball (the existing powerup `EquipmentGrant`
  path — scheme entry appears when the grant lands), ground-pound
  (Technique).
- **Protagonist:** jump, dash, blink, Swipe (attack), Bubble (special), Bolt
  (ranged) — display names authored in
  `game/ambition_content/assets/data/character_catalog.ron` + presets.
- Possession of NPCs/enemies/bosses shows their schemes (falls out of P0).
- Ship the visual hook end-to-end for at least one action (one authored icon)
  so `visual: Option<…>` is proven, or explicitly defer icons to a follow-up
  — decide at execution time based on the icon-pipeline cost.

## Testing & verification

- Unit: scheme derivation, ordering determinism, display-name fallback,
  bindings override → glyph agreement, shared-resolver label = fired action.
- Parity: P0 scheme⇔behavior guard; P3 headless movement-trace harness (the
  refactor's only behavioral gate).
- Read-model: `ControlPrompt` follows possession; menu-open swaps provider;
  pre-poisoned prompt test (stale/empty prompt trips, per pre-poison
  pattern).
- Touch UI (minimal-plugin App + manual update): per-slot button set matches
  the controlled scheme; hidden slots don't hit-test; relabel on possession
  and on menu-open.
- End-to-end (draw blind): run with `mobile_touch`, drive protagonist vs
  Sanic, open inventory, rebind a key — buttons rename/hide/re-glyph
  correctly; screenshot the three states.
- Anchors: `cargo test -p ambition_entity_catalog`, `cargo test -p
  ambition_characters --lib`, `cargo test -p ambition_engine_core --lib
  movement`, `cargo test -p ambition_touch_input`, `./run_tests.sh -k action`.

## Locked decisions (2026-07-17, with Jon)

1. **Slot-layer revision accepted** — `ControlFrame` stays the POD device/slot
   wire format; kernel consumes `MovementAction` edges; scheme resolves
   sim-side. (Revised from the Opus draft's "remove bools from all three
   structs", which would have streamed scheme-resolved content — rollback
   desync + non-POD frame.)
2. **Menus: bindings + presentation unified; consumption unchanged.** Menu
   slots join `ActiveBindings`; menus publish labels via the provider
   contract; `MenuControlFrame` consumers and the nav axis untouched. (Full
   behavioral unification considered and rejected: rewires the hard-won
   menu/touch ordering web for no player-visible gain; menus are not sim
   entities.)
3. **Scheme home:** contract in `ambition_entity_catalog` beside
   `MovesetContract`; runtime component in `ambition_characters` (the
   `ActorMoveset` pattern). No new crate.
4. **Move display names:** authored `display_name` in catalog RON/prefabs,
   title-cased-id fallback.
5. Surface = touch overlay (desktop/gamepad prompt surface is a later
   consumer of the same `ControlPrompt`); cast authoring in scope (P6);
   remapping planned holistically (P1 model + P5 UX).

## Open questions (small, decide at execution)

- Slot display order / layout capacity: the diamond fits ~8 gameplay slots;
  schemes exceeding layout capacity drop trailing entries with a `log` (no
  silent truncation) — revisit if any real scheme overflows.
- Whether `ControlPrompt.available` should surface cooldown/charge state
  (dash charges) in v1 or just presence — lean presence-only first.
- Icon pipeline scope for `ActionVisual` (see P6 bullet).
