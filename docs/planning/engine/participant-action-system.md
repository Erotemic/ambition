# Participant action system — input architecture for external 2D platformers

> **State (2026-07-20): OPEN continuation after the landed startup/launcher
> slice.** The source-backed implementation record is
> [`participant-input.md`](participant-input.md). The sim-side slot-to-action
> law and the remaining binding/remap details are
> [`character-actions.md`](character-actions.md). This document owns the
> **forward architecture and executable migration** that neither older plan
> fully captures: a participant-centered, action-based input system that a new
> game can use without writing keyboard, gamepad, mouse, or touch glue.
>
> **Executor:** Opus-level implementation. Land one vertical slice at a time;
> stop only when source contradicts a factual assumption. Do not replace the
> working deterministic lower half while migrating the host/device half.

## 1. Product promise

A conventional external 2D platformer should be able to:

```rust
app.add_plugins(PlatformerInputPlugin::standard());
commands.assign_participant(participant, actor);
commands.spawn(StandardMenu::new(items));
```

and receive, without game-specific device code:

- keyboard, gamepad, mouse, and touch input;
- menu navigation and direct pointer/touch activation;
- actor control and possession-safe control transfer;
- contextual labels, glyphs, icons, and virtual touch controls;
- runtime binding overrides;
- replay/headless/AI/network input through the same simulation packet;
- C4/gravity-relative movement and aim behavior already provided by the
  controlled-body reference-frame seam.

A game declares **actions, controlled actors, and control surfaces**. The
engine owns devices, routing, focus, presentation cues, and deterministic
sampling.

This is narrower than Unity/Unreal/Godot general-purpose input, but should be
substantially easier for the supported class of games.

## 2. Binding architecture

The durable structure is:

```text
physical and virtual devices
→ persistent participant
→ bindings + pure processors/interactions
→ logical actions
→ ordered active contexts
→ consumer
    ├─ local UI/host command
    └─ deterministic participant control packet

same resolved action + binding + consumer context
→ label + glyph + icon + touch affordance + accessibility cue
```

### 2.1 Participant is the authority

`InputParticipant` is the person in front of one or more devices. It exists
before, during, and after a gameplay session. It owns or indexes:

- device assignments;
- binding profile and runtime overrides;
- resolved action state;
- active input contexts;
- most recently active device class for presentation;
- connection/device-loss state.

The participant does **not** own an acceleration frame. Linking a participant
to an actor changes control authority, not device state:

```text
participant → participant slot → Brain::Player(slot) → controlled actor
```

Possession changes the last relationship only.

### 2.2 Logical actions, not physical controls

Actions express intent such as:

```text
standard.ui.navigate
standard.ui.submit
standard.ui.cancel
standard.platformer.move
standard.platformer.aim
standard.platformer.jump
standard.platformer.attack_primary
standard.platformer.interact
standard.startup.acknowledge
```

Each action has a stable identity and a value kind:

- button;
- scalar axis;
- 2D axis;
- pointer position/delta;
- scroll.

The engine should provide standard UI/platformer/startup/dialogue sets. A
provider should be able to contribute genuinely new actions without editing a
central Ambition enum, but **do not replace `SandboxAction` speculatively**.
First inspect Leafwing 0.20's practical requirements for a dynamic/stable
`InputActionId`. If a stable ID can implement the required action trait cleanly,
migrate incrementally. If not, keep Leafwing as the standard-device adapter
and introduce one engine-owned resolved-action table rather than parallel
per-game `ActionState` stacks.

**Decision gate:** this extension work begins when a second external game needs
an action not representable by the standard platformer/control-slot vocabulary.
Until then, `SandboxAction` remains the finite adapter vocabulary.

### 2.3 Bindings are one authority

The one binding authority must feed all of:

- the live device/action map;
- keyboard and gamepad glyphs;
- touch virtual controls;
- menu commands;
- runtime remapping and persistence.

The detailed `ActiveBindings` and remap work remains owned by
[`character-actions.md`](character-actions.md) P1/P5. The participant migration
changes its owner: bindings belong to a participant, never to an actor or
`PlayerVisual`.

A binding includes:

- action id;
- physical or virtual control path;
- device scheme;
- pure processors (deadzone, scale, invert, normalization, response curve);
- interaction (press, release, tap, hold, repeat, chord where required).

Avoid arbitrary modifiers that read unrelated game state. Spatial/body-frame
conversion belongs at the controlled-body consumer seam, not in bindings.

### 2.4 Contexts are explicit and ordered

A context states which actions are meaningful and who consumes them:

```text
startup acknowledgement
launcher / standard UI
gameplay
dialogue
pause
inventory
loading / retry
cutscene
vehicle / special control mode
developer overlay
```

The owning surface declares and retracts its own context. Never infer local
input ownership from `GameMode`, actor existence, or presentation visibility.

Contexts require explicit priority and capture semantics. The current
`ContextClaim`/`ActiveInputContext` implementation is sufficient for one
capturing owner plus observers, but its non-capturing semantics are not yet a
complete action-layer model: a high-priority observer currently becomes
`owner()` for prompt publication. Do not use non-capturing claims for product
features until routing and cue ownership become action/context-specific.

The eventual per-participant result should answer both:

- which contexts are active/open;
- which context consumes each action.

## 3. Preserve the strong lower half

The migration must retain:

- `ControlFrame` as the compact, fixed-size, device-shaped simulation packet;
- `ControlFrameLatch` frame-to-tick edge preservation;
- `SlotControls`;
- `Brain::Player(slot)` and possession/control-authority transfer;
- `ActionSchemeContract` and the one shared slot-to-action resolver;
- headless, replay, RL, AI, and network code constructing simulation frames
  without a visible device stack;
- existing menu navigation/repeat behavior until the standard UI module owns
  it completely.

Do not stream content-resolved action identities through rollback. The
simulation packet carries participant intent; restored body authorities resolve
that intent through the tick-correct action scheme.

## 4. Reference-frame invariants

The participant/action layers resolve bindings and contexts. They **never**
rotate movement or aim into body/world coordinates.

Required pipeline:

```text
keyboard / pad / virtual touch / replay / network
→ raw screen-frame movement and aim axes
→ participant action state
→ ControlFrame / SlotControls
→ controlled body's ResolvedMotionFrame
→ separate movement and aim InputFrameMode policies
→ body-local ActorControl
```

Binding invariants:

1. Movement and aim remain raw `ScreenAxes` until the controlled body is known.
2. `ScreenRelative`, `BodyRelativeStrict`, and `BodyRelativeAssist` preserve
   their existing mathematics.
3. Movement and precision aim retain separate `ControlFrameModes`.
4. The body-local `ResolvedMotionFrame`, not participant/global gravity, is the
   spatial authority.
5. Possession automatically changes interpretation through the new body
   without recreating participant input state.
6. Zero-acceleration bodies retain environment-authored orientation where the
   existing frame resolver does so.
7. Blink, ranged aim, movement, drop-through gestures, and interaction
   directions retain their existing source-specific frame behavior.
8. “Resolved action” means binding/context resolution, not spatial transform.

The current source of frame-mode policy is still local settings. For future
network determinism, choose one explicit model before multiplayer:

- carry participant movement/aim policy in each `ControlFrame`; or
- negotiate immutable participant policy in the session contract; or
- send synchronized policy-change commands.

Do not merely latch a local setting per participant: every peer must receive
the same policy that interprets a participant's axes.

## 5. Standard UI input module

Menus should not consume devices. A standard UI module consumes actions:

```text
ui.navigate
ui.submit
ui.cancel
ui.page_left / ui.page_right
ui.point
ui.pointer_activate
ui.scroll
```

It owns:

- selected/focused item;
- directional repeat;
- wrapping/clamping/navigation graph;
- pointer hover and active-device arbitration;
- pointer/touch row activation;
- drag/scroll;
- disabled items;
- one semantic activation message.

Controller submit, virtual-touch submit, mouse click, and direct touch must
emit the same final activation event. Specialized surfaces (inventory cube,
kaleidoscope) may keep custom presentation but consume the same UI action
stream.

`MenuControlFrame` may remain an internal compatibility frame during migration,
but it stops being the place devices are merged and eventually becomes either:

- a participant-keyed UI action frame; or
- transient commands emitted by the standard UI module.

No consumer should clear global edges to coordinate with another consumer in
the final model.

## 6. Contextual cues and touch projection

Behavior and presentation are projections of the same resolved contract.
Extend the current label-only `UiCue`/`ControlPrompt` model toward:

```rust
ResolvedActionCue {
    participant,
    context,
    action,
    label,
    glyph,
    visual,
    enabled,
    interaction,
    touch_presentation,
    accessibility_description,
}
```

Examples:

```text
ui.submit → Play / Equip / Continue
platformer.interact → Talk / Pick Up / Enter
platformer.attack_primary → Fire Portal / Swipe
```

The one binding authority supplies glyphs. The active consumer supplies the
contextual verb. The actor's `ActionSchemeContract` supplies gameplay labels
and availability.

Touch is a virtual device and a presenter:

- buttons/sticks publish virtual controls through the participant bindings;
- direct UI taps use the standard pointer activation path;
- the touch HUD renders the current cue set;
- stable action slots preserve muscle memory;
- unavailable controls hide and stop hit-testing;
- conventional platformers receive a competent default layout automatically;
- providers may override layout policy without writing device routing.

The current fixed layout and `ControlPrompt` adapter are retained until the cue
model can fully describe glyphs, interactions, and touch placement.

## 7. Host versus deterministic simulation

Classify actions at the context/consumer boundary.

### Local host actions

Examples: launcher, settings, graphics, ordinary local pause UI, pointer hover.
They remain local and contribute a neutral gameplay packet while captured.

### Simulation actions

Examples: movement, attack, jump, synchronized dialogue choice, a
save-mutating cutscene advance. They enter the deterministic participant packet
or are resolved from synchronized simulation state.

The simulation must never read raw devices, local wall-clock holds, local focus,
or presentation resources.

Cutscene authority remains a separate design-first card in `tracks.md`:
semantic playback state belongs to deterministic simulation; local hold-to-skip
progress is presentation; only the completed edge crosses through participant
input. The current overlay fixes duplicate local request production but does
not claim this deterministic migration is complete.

## 8. Current source state and open gaps

### Landed

- persistent primary `InputParticipant` at boot;
- device state removed from `PlayerVisual`;
- explicit startup, launcher, and gameplay context claims;
- same-frame input schedule pipeline;
- shell consumes semantic input only;
- startup cards support semantic confirm and tap-anywhere;
- touch buttons/stick are Leafwing virtual controls;
- startup/launcher cues reach `ControlPrompt` and touch labels;
- assembled no-actor startup/launcher acceptance tests;
- raw screen-axis equivalence across keyboard/gamepad/touch.

### Still global/single-participant

- `ActiveInputContext` is one global primary-participant resource;
- `ActiveInputKind` is global;
- `ActiveInputMethod`/`GamepadKind` in actor affordances is a second device
  presentation model that can drift from `ActiveInputKind`;
- `ControlFrame`, `ControlFrameLatch`, `MenuControlFrame`, and menu repeat state
  are global primary-seat resources;
- gameplay context claims are applied to every participant even though routing
  resolves only the primary seat.

### Contexts not migrated

- loading/retry (retry still reads raw R / gamepad West);
- pause;
- dialogue;
- inventory and specialized menus beyond cue publication;
- deterministic cutscene input;
- developer overlays and special control modes.

### Raw-device reads that need classification

Migrate product control surfaces to participant actions:

- loading retry;
- any remaining game/menu hotkey that a provider should be able to rebind.

Keep explicitly host/platform-level adapters where appropriate, but document
them as such:

- browser audio unlock gesture;
- device discovery and active-device detection;
- developer-only emergency/debug hotkeys, unless intentionally exposed as
  remappable product actions.

Run a fresh `rg` audit before each phase; do not ban raw input from the actual
device adapters.

### Binding/cue gaps

- presets are the authority, but there is no per-participant override model;
- gamepad glyphs still use a parallel table and incomplete device detection;
- `UiCue` is label-only;
- touch layout still knows fixed `ControlSlot` positions;
- `SandboxAction` remains a closed Ambition adapter enum.

### Context-transition gaps

- menu analog repeat state is global and continues updating outside an
  explicitly migrated UI context;
- held-axis transition behavior is tested less strongly than held confirm;
- non-capturing contexts do not yet have action-specific ownership/cue rules;
- focused-surface despawn recovery is not generalized beyond current shell
  lifecycle behavior.

## 9. Executable migration

### PA1 — one participant-local binding/device presentation authority

Build on `character-actions.md` P1/P5 rather than duplicating it.

- Introduce participant-owned `ActiveBindings`/overrides.
- Rebuild the participant `InputMap` from that source.
- Move glyph lookup to the same binding data.
- Collapse `ActiveInputKind` and actor-side `ActiveInputMethod` into one
  participant-local presentation fact, retaining gamepad vendor/style detail.
- Keep device class presentation-only; it never changes simulation output.

**Delete:** parallel preset/glyph tables and global active-device state.

**Exit:** changing one binding changes behavior, keyboard/gamepad glyphs, and
virtual touch mapping in the same frame; no actor is recreated.

### PA2 — migrate all ordinary local control surfaces to explicit contexts

Migrate in small commits:

1. loading/retry;
2. pause;
3. dialogue presentation/ordinary choices;
4. inventory and specialized menus;
5. developer overlays only where they should capture product input.

For each surface:

- the owner declares/retracts its context;
- raw device reads are deleted;
- input arrives through standard UI actions;
- cue publication is context-keyed;
- closing/despawning restores the previous context without false edges.

**Exit:** `GameMode` and controlled-body presence no longer decide host input
ownership; raw product UI reads are gone.

### PA3 — standard UI module convergence

- Put directional repeat, focus, submit/cancel, pointer activation, and scroll
  behind one shared module.
- Make pointer/touch activation and controller submit emit one event.
- Move standard shell, pause, settings, and list/grid menus onto it.
- Retain specialized rendering backends as presentation consumers.
- Replace edge-clearing coordination between global menu consumers.

**Exit:** a provider spawns a standard menu without device code; all devices
activate identical semantic commands.

### PA4 — cue/glyph/touch contract

- Extend cues with action id, glyph, visual, enabled state, interaction, touch
  hint, and accessibility description as real consumers land.
- Fold actor `ActionSchemeContract` and UI surface commands into one resolved
  cue vocabulary.
- Make touch visibility, labels, hit testing, and layout read only cues.
- Preserve stable platformer slots and offer a standard auto-layout.

**Exit:** behavior, glyph, contextual verb, and touch affordance cannot drift
because all derive from one action/binding/context resolution.

### PA5 — participant-keyed routing and local multiplayer foundation

- Move active contexts and device/presentation state onto each participant.
- Pair devices to participants and handle connection/loss.
- Replace global UI/gameplay frames with participant/slot-keyed routing.
- Ensure one participant can own UI while another remains in gameplay where
  the host policy permits it.
- Spawn the primary participant even if a secondary participant already exists
  (the immediate boot invariant is fixed in the closeout overlay).

**Exit:** two local participants can use disjoint devices and produce distinct
slot frames without competing for a global resource.

### PA6 — deterministic policy and cutscene boundary

- Select and implement the synchronized frame-policy model.
- Route simulation-relevant dialogue/cutscene commands through participant
  packets or synchronized commands.
- Keep local hold/tap recognition and visual progress outside simulation.
- Add replay/rollback tests for policy changes and semantic cutscene edges.

**Exit:** peers receiving identical participant packets resolve identical
movement/aim/cutscene behavior regardless of their local settings/devices.

### PA7 — provider-extensible action schemas

Begin only after a real external provider requires a non-standard action.

- Introduce stable provider action IDs and action-set registration.
- Preserve the standard platformer/UI sets as the zero-config path.
- Prove one external action through binding, context, consumer, cue, and touch
  projection without editing `SandboxAction`.
- Delete any temporary adapter used for the proof; do not maintain two action
  systems.

**Exit:** an external game adds one custom action without editing core and
without implementing a device-specific path.

## 10. Tests and forcing oracles

### Source ownership

- no action state/input map on actor or visual entities;
- participant survives session teardown, actor death, and possession;
- secondary participants do not suppress primary boot creation.

### Context transitions

- startup → launcher → gameplay with no false press/release edges;
- held confirm does not retrigger across contexts;
- held direction does not inherit stale repeat timing into a new menu;
- focused-surface despawn restores the prior owner;
- non-capturing observers cannot steal prompt or action ownership.

### Device parity

- keyboard, gamepad, virtual touch, replay, and headless inputs produce the
  same raw screen-frame packet for equivalent intent;
- mouse click/direct touch/controller submit emit the same UI activation;
- binding override changes behavior and cues together.

### Reference frames

- screen-relative input stays screen-relative under all C4 gravity directions;
- strict/assisted body-relative modes retain current behavior;
- movement and precision aim can differ;
- possession between differently oriented bodies changes interpretation
  without changing participant state.

### Multi-participant

- distinct devices feed distinct participants/slots;
- one participant's context does not clear another's actions;
- device loss neutralizes only the affected participant.

### Determinism

- simulation consumers read only participant packets and restored sim state;
- frame-policy and semantic cutscene commands reproduce under replay/rollback;
- local focus, device kind, glyphs, and touch layout do not affect checksums.

## 11. Author-facing endpoint

The intended API is small and opinionated:

```rust
app.add_plugins(PlatformerInputPlugin::standard());
commands.assign_participant(primary, actor);
commands.spawn(StandardMenu::new(items));
commands.spawn(AcknowledgeSurface::new("Continue").tap_anywhere());
```

A provider may additionally register:

- default binding profile;
- action schemas/action sets when genuinely needed;
- custom control contexts;
- touch layout overrides;
- localized cue metadata.

Game code must not implement keyboard/gamepad/touch branching.

## 12. Non-goals

- no Unity-scale arbitrary device/editor asset graph before customers demand it;
- no dynamically typed universal input event bus;
- no arbitrary stateful modifiers that can read the world;
- no requirement that local UI commands become rollback state;
- no spatial frame conversion in the participant/binding layer;
- no compatibility facade around migrated raw-device paths;
- no complete split-screen/couch-multiplayer UI in PA1–PA4;
- no replacement of `ControlFrame` simply to look more generic;
- no new framework without deleting a concrete old path in the same slice.
