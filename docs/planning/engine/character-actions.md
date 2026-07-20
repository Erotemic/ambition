# Character actions — the slot→action control seam + context-sensitive control prompts

> **State (2026-07-17):** LARGELY LANDED. P0–P4 + the input-ownership seam are
> implemented: the action vocabulary, the `ControlPrompt`/touch relabel, the
> movement-kernel re-key, the shared resolver, the technique-edge migration, the
> real player Special, the touch Special route, and the menu Equip/Use provider
> (see the Status section for the per-gate breakdown + validation). Remaining:
> **P1** (`ActiveBindings` source-of-truth), **P5** (remap UX), **P6** (cast
> authoring) — and a runtime playtest. Drafted by Opus 4.8, revised by Fable 5,
> executed by Opus 4.8 (incl. the GPT-5.6 follow-up review's seven gates).
> Supersedes the touch overlay's affordance relabel path as the design of record.
>
> **Executing-model sign-off:** Opus 4.8 (GPT-5.6 follow-up review completion).
>
> **2026-07-20 ownership update:** the persistent participant, explicit host
> contexts, virtual-touch device, and startup/launcher routing supersede this
> document's older P2 mechanism that derived `ControlContext` from `GameMode`.
> Use [`participant-input.md`](participant-input.md) for the landed slice and
> [`participant-action-system.md`](participant-action-system.md) for open host/
> device/UI migration. This document remains authoritative for sim-side
> slot→action resolution and P1/P5 binding/remap requirements.

## The law (target)

A **character owns its actions**, and the **action is the gate**: input never
pokes the movement kernel or combat directly — it fires one of the controlled
subject's declared actions, and the action's gate is what reaches the kernel
(`MovementAction`), a content technique, the moveset runtime, or world
interaction. The same declared action carries its presentation (display text,
optional visual), so the on-screen buttons and the simulation **cannot
disagree by construction**: the prompt asks the same resolver the brain calls
on press. (This property is structural from P3 on; through P0–P2 it is
enforced by the scheme⇔behavior parity guard instead — see P0/P2.)

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
   toggle). Streaming *resolved actions* is forbidden. This is mechanical,
   not aspirational, because the scheme itself is **derived state** — a pure
   function of already-snapshotted authorities (`AbilitySet`, `ActorMoveset`,
   technique registrations) — so a rollback reconstructs the tick-correct
   scheme for free from the restored authorities; nothing scheme-shaped
   enters the stream or the snapshot ledger (the `ResolvedMotionFrame`
   precedent). (Worth an ADR when P3 lands.)
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
  (`crates/ambition_actors/src/avatar/starting_character.rs`) — **NOT retired
  yet.** The P3 end-state is a body without an action simply lacking a scheme
  entry (nothing to strip after the fact), reached once the kernel consumes
  actions directly. Until then the gate REMAINS — now as the consumer of the
  shared per-slot dispatcher `resolve_control_slots`
  (`crates/ambition_characters/src/action_scheme.rs`): it derives the scheme and
  routes/strips EVERY combat slot's verb through that ONE pure function (Attack,
  Special, Projectile, QuickAction), no longer an Attack-only special-case. Its
  deletion is a P3 step, sequenced after the movement-kernel re-key.
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

**Status (2026-07-17):** **P0 ✅ + P2 ✅ landed — the shippable milestone.**
- P0a: `entity_catalog::action_scheme` vocabulary + combat derivation (5 tests).
- P0b: `characters::action_scheme` — `derive_action_scheme` + `ActorActionScheme`
  + scheme⇔behavior parity guard (4 tests).
- P2a: `actors::action_scheme::reconcile_action_schemes` attaches the scheme to
  every body from its live `AbilitySet` + moveset, change-detected (3 tests).
- P2b: `sim_view::ControlPrompt` read-model — the controlled subject's labels,
  follows possession (2 tests).
- P2c: the touch overlay's buttons relabel from `ControlPrompt` and hide slots
  the scheme lacks (Sanic shows no Attack/Shot); dropped the `PlayerAffordances`
  label reach. Full app graph compiles; 35 touch tests green.

- P4-minimal: in a menu/dialogue the touch overlay's select-functional
  buttons (Jump/Interact) wear the menu confirm verb ("Select"/"Advance"), and
  gameplay-only buttons hide. `ControlPrompt.menu_confirm` set per `GameMode`.
  **Both halves of the original ask now land: character moves AND menu verbs.**

**GPT-5.6 review execution (2026-07-17):** the review flagged two correctness
bugs to fix before the broad P3 migration, plus the settled `InputState` design
(split at the ownership boundary; enum-indexed storage + typed accessors).
Landed:
- **Review #1 — hidden actions were still tappable.** One
  `touch_action_available(action, prompt)` predicate now drives BOTH visibility
  AND the raw hit test (`update_buttons_from_interactions` masks unavailable
  actions out of the held/edge derivation); `Empty` context hides gameplay
  buttons. Tap-hidden→no-edge test.
- **Review #2 — the canonical player's scheme was wrong.** `derive_action_scheme`
  now unions combat from the moveset **and** the `ActionSet` (the player's
  ranged/special come from `ActionSet` + the legacy pipeline; its moveset is
  melee-only), with a parity test built from the real default-player bundle.
  Plus: `ActionSchemeContract::new` normalizes one-action-per-slot; corrected the
  reconcile-ordering comment (`PlayerInput` is chained before `WorldPrep`).
- **P3 groundwork:** dedicated `SandboxAction::Special` + per-preset key + glyph
  (blink is no longer Special's source); `MovementAction` enum + enum-indexed
  `ActionEdges` + typed accessors in `engine_core`.
- **`InputState` re-key (P3 step 6) — LANDED with full parity** (`8c6c802f9`).
  The movement kernel now dispatches locomotion on `MovementAction` through
  enum-indexed `ActionEdges` + typed accessors: the 5 locomotion verbs collapse
  into `movement: ActionEdges<MovementAction>`, `jump_pressed()` etc. are
  accessor methods, and both bridges build the edges. `attack/interact/reset/
  shield` stay explicit named fields — the kernel genuinely reads them
  (slash-recoil, ledge get-up/climb-confirm, reset flag, shield/roll) and they
  are not locomotion verbs, so they're honestly kept rather than dishonestly
  broadened into `MovementAction`. The post-hit stagger gates preserve edge
  granularity (hitstun eats only the jump *press*, keeping an in-progress jump's
  held/released). **Parity net green:** engine_core 329 movement tests +
  characters 387 + actors 810 + the demo_sanic speedway momentum oracles (63) +
  repro_walls (9). (A first regex attempt was reverted; the redone
  qualified-path transform — skip `-> [path::]InputState {`, comment-strip,
  hand-fix one shorthand init — landed cleanly. Steps 7-8 folded in here.)
- **Step 10 — LANDED** (`57e529123`): the `special_pressed = blink_pressed` alias
  is retired. `ControlFrame` gained a `special_pressed` field (serde-default-safe
  for the stream; per-field OR merge), sourced from `SandboxAction::Special`; the
  brain reads it instead of blink. Surgical + positive test: Special fires
  special not blink, Blink fires blink not special. Validated netcode-clean —
  the `desync_canary` suite (incl. the snapshot-coverage ledger) is 19/19 green.
- **Step 11 — LANDED** (`cf24c5558`): explicit AI-body movement parity test
  (AI bodies route jump/dash/blink + the post-hit gates through the re-keyed
  bridge).
- **Step 12 — LANDED**: [ADR 0025](../../adr/0025-character-actions-input-ownership.md).
- **Snapshot ledger:** `ControlPrompt` (resource) and `ActorActionScheme`
  (component) recorded as reviewed derived debt — both are intentionally
  unregistered (rebuilt/re-reconciled each tick from snapshotted authorities).
- **Step 5 (first pass) — a drift GUARD, not the resolver** (`4d84a0230`): a
  unit test locking scheme-slot ⇔ gate authority. Useful, but a `Option::is_some`
  comparison is not a shared resolver — superseded below.
- **Step 9 (first pass) — identity/label only** (`aa14405b2`): `ActorTechniques`
  gave Sanic's spin its slot + "Spin Dash" label, but the BEHAVIOR still
  intercepted raw `melee_pressed` — superseded below.

### Follow-up review (GPT-5.6, 2026-07-17) — the completion the first pass claimed but hadn't finished

A second review found the "all 12 landed" claim exceeded the implementation
(phantom player Special, a drift *guard* mislabelled as the resolver, a
technique that was identity-only, no touch Special, hardcoded menu verb). Its
seven gates are now genuinely landed:

- **Gate 1 — the canonical player's Special is a real move.** `build_actor_moveset`
  folds `ActionSet.special` into a `"special"`-verb `MoveSpec` — the Special is now
  in the player's MOVESET, not a phantom capability. Pressing Special starts the
  bubble_shield move, and `sustain_bubble_shield` raises the ONE shield on the
  PRESS tick (it reads the `special_pressed` edge in `PlayerInput`, before the
  `WorldPrep` kernel bridge, so the move being triggered later in `Combat` does not
  make the guard one tick late) AND for the move's duration. A real
  production-schedule app_it test (`player_bubble_shield`) drives
  `AgentAction{special}` through the whole sim and asserts `BodyShieldState.active`
  on the press tick. (Enemy/boss archetype specials stay authored — not re-folded;
  possessed bosses are `Without<PlayerEntity>`, so the gate never touches them.)
- **Gate 2 — one shared resolver, two consumers.** Both the persona gate
  (`gate_worn_player_control`) and `ControlPrompt` call the SAME
  `derive_action_scheme` on the body's IMMEDIATE authorities each tick to build the
  scheme; the gate then APPLIES it through the pure per-slot dispatcher
  `resolve_control_slots`, which handles EVERY slot — routes techniques, strips the
  verbs the scheme doesn't own (Attack/Special/Projectile), keeps `Move`s, and
  REJECTS (returns for a debug-assert, never silently drops) a technique declared
  on a movement or Interact slot, since those await the P3 kernel re-key. Not a
  guard — the actual resolution, unit-tested as a slot matrix.
- **Gate 3 — sanctioned technique edges.** `ResolvedTechniqueEdges` is a
  **required component** of `ActorTechniques`, so declaring a technique always
  attaches the edge sink (no silent input loss). `resolve_control_slots` routes a
  `Technique`-gated slot's device edge there and clears the raw verb; Sanic's
  ball-dash reads the `spin_dash` edge and the fragile
  `.before(gate_worn_player_control)` interception is DELETED. A plain melee edge
  is no longer the content API (tested both directions).
- **Gate 4 — no same-tick drift.** The prompt derives from immediate authorities
  (no lagged cache on the path); `ActorActionScheme`/`reconcile_action_schemes`
  are demoted to a documented observation cache. A same-tick kit-swap test runs
  the REAL gate and REAL prompt together — the visible slot and the executable
  verb flip together on the swap tick.
- **Gate 5 — touch Special + gamepad policy.** A 9th touch button routes
  `ControlSlot::Special` into `ControlFrame.special_pressed`; the gamepad is
  full, so gamepad-Special is a documented+tested dynamic-slot deferral to remap.
- **Gate 6 — the real menu verb.** An app provider (`publish_menu_confirm_prompt`)
  resolves the focused inventory item's verb from the shared `KaleidoscopeCursor`
  + `OwnedItems` and publishes `MenuConfirmPrompt`; `rebuild_control_prompt` folds
  it into `ControlPrompt.menu_confirm`, so runtime inventory controls say
  **Equip / Use** (tested through the real provider path, not an injected string).
- **Gate 7 — this reconciliation** + [ADR 0025](../../adr/0025-character-actions-input-ownership.md) updated to match.

**Now genuinely landed:** the action vocabulary, the prompt + touch relabel, the
`InputState` re-key (`MovementAction`/`ActionEdges`), the `blink→special` alias
retirement, the shared resolver (`derive_action_scheme` + the per-slot dispatcher
`resolve_control_slots`), the technique-edge migration, the real player Special,
the touch Special route, and the menu Equip/Use provider.

**Precisely what is and isn't done on the resolver/technique axis** (so this doc
does not over-claim P3): technique routing is generic across the four COMBAT slots
(Attack/Special/Projectile/QuickAction) — the dispatcher routes or strips each.
Movement/Interact-slot techniques are *rejected*, not wired: firing a technique
from Jump/Dash/Blink needs the kernel to consume actions (P3), which is NOT done.
Consequently `gate_worn_player_control` is **NOT retired** — it is the dispatcher's
consumer and remains until the P3 re-key deletes it.

Validated — engine_core 329, characters 388, actors 816, combat 104, sim_view 16,
demo_sanic 63, touch 41, plus the app menu provider path and the `player_bubble_shield`
app_it integration test.

Pending (genuinely NOT done): **P1** (`ActiveBindings` source-of-truth + the live
preset-split bug + glyph re-home), **P5** (rebind-capture UX, gamepad glyphs from
the live map, gamepad Special), **P6** (author the full cast + `MoveSpec.display_name`
+ icons). **Not yet runtime-playtested** — seam-unit + end-to-end integration
verified; the on-screen relabel/hide wants a visual check (headless env has no
DISPLAY). P1 is invasive (live input) and wants Jon's playtest before landing.

### P0 — Action vocabulary + scheme derivation *(no behavior change)*

- `ActionSchemeContract` in `crates/ambition_entity_catalog` **next to
  `MovesetContract`** (`src/lib.rs:707`): ordered
  `Vec<ActionSpec { id, slot, display_name, visual: Option<VisualId>, gate }>`.
  String `ActionId` (matches `MovesetContract.verbs` keys) + well-known
  constants; `ActionGate::{Movement, Technique, Move, Interact}`.
  `MovementAction` enum + `ActionEdges` go in `ambition_engine_core` (the
  kernel will consume them in P3).
- Move labels: `MoveSpec::display()` title-cases the id today
  (`sandbag_swat` → "Sandbag Swat"). The authored `display_name:
  Option<String>` field on `MoveSpec` lands in **P6** — added in the same
  commit that fills it in at the move construction sites, so P0 doesn't
  scatter `display_name: None` through ~14 literals for no consumer.
  `ActionSpec` already carries `display_name` for scheme-level overrides.
  (Rust `ron` parse test per parser-drift rule when the field lands.)
- **Combat-slot invariant, encoded:** `combat_from_moveset` binds the
  Projectile slot to the `"ranged"` verb, so the three combat slots
  (Attack/Special/Projectile) line up 1:1 with the three moveset verbs.
  Tested in `entity_catalog::action_scheme::tests`.
- Runtime wrapper `ActorActionScheme(ActionSchemeContract)` component in
  `ambition_characters`, exactly the `ActorMoveset(MovesetContract)` pattern
  (`crates/ambition_combat/src/moveset/mod.rs:198`). Attached wherever
  movesets are attached (`combat/moveset/prefabs.rs:635` seam + boss/NPC
  spawn paths) so **possessed bodies carry schemes too**.
- Pure builder `derive_action_scheme(abilities, moveset, techniques)` — from
  the SAME authorities that gate behavior today: `AbilitySet`
  (`engine_core/src/abilities.rs:20`) for movement actions,
  `ActorMoveset`/`ActionSet` for combat, content-registered techniques.
- The scheme component is **derived, snapshot-DERIVED, reconciled on
  change**: a reconcile system re-derives it whenever a source authority
  changes (equipment grant, ability grant, form change — the
  `reconcile_autonomous_actors` pattern), and the snapshot ledger registers
  it DERIVED so restore re-derives instead of persisting (the
  `ResolvedMotionFrame` precedent). This is what makes invariant 1's
  rollback claim mechanical: the scheme never needs its own snapshot or
  stream entry.
- **Every derived action gets a slot, and Special gets a DEDICATED slot**:
  new `SandboxAction::Special` variant (default bindings authored in P1).
  Today the protagonist's blink button double-fires blink AND the bubble
  special (`special_pressed = blink_pressed`), while the touch Shot button
  wears the special's labels but fires ranged — two existing HUD lies.
  One-slot-one-action splits them: blink slot → Blink, Special slot →
  Bubble. Without this, retiring the alias in P3 strands every special
  between P3 and P6.
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
- P2 is a **shippable player-facing milestone** on its own; P3 (the seam
  refactor) remains committed per Jon's full-seam decision, but it is a
  separable risk window, not a prerequisite for the visible feature.

### P3 — The seam refactor: kernel consumes actions *(parity-gated)*

- **Parity harness FIRST** (bold-refactor discipline): headless movement
  traces for jump / dash / blink / fast-fall / fly-toggle / wall-jump /
  drop-through + Sanic spin-dash/ball across the refactor; gate = compiles +
  parity holds. **Two coverage requirements (Opus review):**
  (a) the harness MUST include an **AI-driven body** (a boss that
  jumps/dashes), since re-keying `ActorControlFrame` touches every brain
  that writes movement verbs (`brain/smash/arena.rs`,
  `brain/fighter/options.rs`, `boss_pattern/seeds.rs`) — a player-only
  harness would let a broken AI movement path pass green;
  (b) the blink/special split's authored exception must be **surgical and
  positive** — assert the special STILL fires on the new `Special` slot and
  that blink no longer fires it, not merely suppress the old assertion, or
  the exception hides a genuinely-broken special.
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
- Retiring the `special_pressed = blink_pressed` alias is safe ONLY because
  Special owns its slot by P0/P1 — sequencing requirement, not polish. The
  blink/special double-fire split is a DELIBERATE behavior change recorded
  as an authored exception in the parity harness (pre-release, behavior not
  sacred), and it fixes both existing HUD lies named in P0.
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

## Review addendum (2026-07-17, Opus 4.8 review → addressed by Fable 5)

1. **Rollback claim made mechanical** — the scheme is snapshot-DERIVED,
   a pure function of already-snapshotted authorities, reconciled on
   change; nothing scheme-shaped enters the stream or the ledger
   (invariant 1 + P0).
2. **Special-slot sequencing hole closed** — dedicated
   `SandboxAction::Special` slot in P0/P1 so the P3 alias retirement
   cannot strand specials before P6. The blink/special double-fire split
   is an authored behavior change in the parity harness, and it fixes two
   pre-existing HUD lies (Shot button wearing special labels; blink button
   silently firing the special).
3. **Honesty bound on the headline** — "cannot disagree by construction"
   is a post-P3 structural property; through P0–P2 it is parity-guarded
   (the law + P2).
4. **P2 marked shippable** — the visible feature lands at P2; P3 stays
   committed (Jon's full-seam decision stands) as a separable risk window.

## Open questions (small, decide at execution)

- Slot display order / layout capacity: the diamond fits ~8 gameplay slots;
  schemes exceeding layout capacity drop trailing entries with a `log` (no
  silent truncation) — revisit if any real scheme overflows.
- Whether `ControlPrompt.available` should surface cooldown/charge state
  (dash charges) in v1 or just presence — lean presence-only first.
- Icon pipeline scope for `ActionVisual` (see P6 bullet).
