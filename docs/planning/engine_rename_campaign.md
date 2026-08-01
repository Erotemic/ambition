The main focus remains building an exceptional 2D platformer engine.
That vertical push should stay central: movement, combat, worlds, portals,
  actors, runtime composition, and presentation should become increasingly
  polished and coherent as a dedicated `platformer2d` stack.

At the same time, Ambition should leave clean seams for external contributors to
  extend the engine and for other styles of games to become possible in the
  future.
The goal is not to prematurely generalize every platformer subsystem.
It is to avoid claiming generic names for crates that are currently
  platformer-specific, while preserving genuinely general services such as
  content compilation, input, causal inspection, loading, assets, audio, and
  time.

This supports the longer-term ambition of becoming a real Unity or Godot
  competitor without weakening the immediate product focus.
Ambition can grow outward from a deep, high-quality 2D platformer engine rather
  than attempting to become shallowly universal from the beginning.

---


Recommended debt vocabulary

Use each term for one specific condition:

Monolith
    one type or crate owns multiple concerns that should eventually have
    different authorities

Tangle
    dependency direction and ownership are mutually entangled

Legacy
    an old path retained temporarily while replacement code is already active

Bridge
    intentional temporary integration between two architectures

Compatibility
    intentionally retained translation for an older external contract - WE SHOULD NEVER BE USING THIS UNTIL WE HAVE A REAL RELEASE, WHICH WILL NOT HAPPEN ANYTIME SOON.

Do not use Legacy for merely unattractive code, and do not use Monolith for every aggregate.

---

# Retire Stale Sandbox and Ambition Naming

## Extended abstract

The engine has outgrown the historical `Sandbox*` vocabulary. Most remaining uses no longer describe experimental systems: they name the production simulation schedule, game save, asset catalog, presentation stack, and runtime composition. These names now obscure whether a concept belongs to the reusable platformer engine or the shipped Ambition game.

This should be handled as a bounded mechanical rename campaign. It should not introduce compatibility aliases or become a disguised architecture rewrite.

Use three naming classes:

```text
Platformer2d*
    reusable platformer-engine concepts

AmbitionGame*
    shipped Ambition game concepts

descriptive unqualified names
    generic engine concepts
```

Keep `Sandbox*` only for actual sandbox worlds, development modes, experimental loadouts, or similarly literal concepts.

## Recommended direct renames

### Platformer-engine concepts

```text
SandboxSet → Platformer2dSimulationPhase

SandboxSetsPlugin → Platformer2dSimulationSchedulePlugin

SandboxSim → Platformer2dSimHarness

SandboxSimOptions → Platformer2dSimHarnessOptions

SandboxSolidContributor → PlatformerWorldSolidContributor
```

### Shipped-game concepts

```text
SandboxSave → AmbitionGameSave

SandboxSaveData → AmbitionGameSaveData

SandboxDevState → AmbitionGameDeveloperState

SandboxAssetCatalog → AmbitionGameAssetCatalog

SandboxCatalogInputs → AmbitionGameAssetCatalogInputs

SandboxDataSpec → AmbitionGamePlatformerDefaults

SandboxDataAsset → AmbitionGamePlatformerDefaultsAsset

SandboxSimState → AmbitionGameSessionState

SandboxSimulationPlugin → AmbitionGameSimulationPlugin

SandboxSimulationResourcesPlugin → AmbitionGameSimulationResourcesPlugin

SandboxPresentationPlugin → AmbitionGamePresentationPlugin

SandboxAudioPlugin → AmbitionGameAudioPlugin

SandboxLdtkPlugin → AmbitionGameLdtkPlugin

SandboxLdtkProject → AmbitionGameLdtkProject

SandboxEventWriters → AmbitionGameEventWriters

SandboxFeelTuning → AmbitionGameFeelTuningMonolith

SandboxAction → AmbitionGameInputActionMonolith

ambition/platformer_defaults.ron → ambition/platformer_defaults.ron
```


Is SandboxQueues dead code? Can it be removed?

The same pass should correct unambiguous bare `Ambition*` names:

* shipped-game concepts become `AmbitionGame*`;
* platformer concepts become `Platformer2d*`;
* generic engine types lose redundant nominative prefixes.

The package namespace may remain `ambition_*`. Runtime identifiers such as content IDs, asset namespaces, experience IDs, and save-directory names are product identities and must not be changed mechanically.

## Small semantic subpass

`SandboxReset*` should be renamed according to what each operation actually does:

```text
RoomReplayRequested
SessionRestartRequested
NewGameResetRequested
EraseProgressRequested
```

This requires inspection, but not a larger redesign.

## Explicit exclusions

Do not mechanically rename:

```text
SandboxAction
SandboxFeelTuning
```

`SandboxAction` belongs to the planned input separation. `SandboxFeelTuning` combines several ownership domains and should be decomposed rather than hidden behind a new broad name.

The campaign is complete when production systems no longer use `Sandbox*` as a historical namespace, while literal sandbox content and development modes retain the name.


The eventual decomposition is roughly:

```
AmbitionGameInputActionMonolith
    ├── ShellAction
    │     menu navigation, confirm, cancel, pause
    ├── Platformer2dAction
    │     movement, jump, dash, attacks, traversal
    └── AmbitionGameAction
          inventory, map, and game-specific commands
```


----


# Candidate Engine Restructures by Difficulty

## Extended abstract

The crate rename exposed several useful decomposition opportunities. They should not be executed as one architecture campaign. Difficulty here reflects dependency risk and semantic uncertainty, not merely the amount of file movement.

## Lower difficulty

### Split persistence

Separate user preferences from shipped-game progression:

```text
ambition_persistence
    ↓

ambition_user_settings
ambition_game_save
```

`ambition_user_settings` should own display, audio, accessibility, controller preferences, serialization, and settings-path discovery.

`ambition_game_save` should own `AmbitionGameSave`, progression, persistent world state, autosave, and save-version handling.

Current quest data should remain with the game save until a proper quest architecture exists.

### Decompose gameplay tuning as touched

`SandboxFeelTuning` mixes unrelated concerns. Split fields into domain-owned resources when those systems are next modified:

```text
MovementFeelTuning
CombatFeelTuning
TransitionTuning
TimeFeelTuning
```

This need not be a standalone campaign.

## Moderate difficulty

### Extract generic snapshot vocabulary

Move genuinely generic deterministic snapshot machinery out of the platformer core, possibly into:

```text
ambition_snapshot
```

Candidate contents include:

* snapshot traits;
* deterministic readers and writers;
* canonical encoding helpers;
* deterministic hashing support.

Do not turn this into a miscellaneous simulation kernel.

These remain platformer concepts:

```text
ControlFrame
InputFrameMode
```

After extraction, rerun the dependency census before creating any broader low-level crate.

### Split generic input from platformer input

First establish the identity model:

```text
InputSourceId
ParticipantId
SessionSeatId
ControlChannelId
```

Then split responsibilities:

```text
ambition_input
    source discovery
    participant and source assignment
    joining and leaving
    semantic actions
    bindings and contexts
    action state
    menu and shell routing

ambition_platformer2d_input
    ControlFrame
    InputFrameMode
    movement and aim
    jump, dash, attacks, and traversal
    platformer presets
    semantic-action-to-control-frame translation
```

`SandboxAction` should not survive as another closed everything-enum. Separate shell actions, platformer actions, and AmbitionGame-specific actions.

This is moderate rather than easy because it changes ownership at the boundary between devices, participants, session preparation, rollback input, and controlled actors.

## Continuing decomposition campaign

### Shrink the actor monolith by destination

Move behavior to existing owners:

```text
body and movement integration
    → platformer simulation

character projection and replacement
    → characters

combat adapters
    → combat

world and session orchestration
    → runtime or world

presentation adapters
    → presentation or rendering

controlled-body input translation
    → platformer input
```

Keep the monolith name until it stops accepting reusable behavior by default.

This work should accompany feature and repair work rather than becoming an arbitrary file-splitting exercise.

## Higher difficulty

### Untangle the shared tangle

The clearest large strands are:

```text
construction/
gameplay_presentation/
```

Possible destinations include:

```text
ambition_construction
ambition_presentation_model
ambition_platformer2d_sim
ambition_content_binding
```

Extract a strand only when its authority boundary and dependency direction are understood. The goal is not smaller crates by itself; it is removing dependency knots.

### Separate sprite format from sprite runtime

A likely eventual structure is:

```text
ambition_sprite_sheet
    metadata schemas and pack format

ambition_sprite2d_runtime
    animation playback and asset resolution

domain adapters
    character, boss, item, and world presentation
```

This should wait until substantive sprite-runtime work makes the seam valuable.

### Replace the current quest model

The current quest system is too underdeveloped to deserve preservation as an independent abstraction.

A future quest redesign should establish:

* authored quest definitions;
* objectives and conditions;
* progression state;
* event observation;
* rewards and consequences;
* content validation;
* save integration;
* multiplayer authority.

Do not extract the current quest code merely to improve crate symmetry.

## Guiding rule

Prefer decompositions that establish a clear owner and improve dependency direction. Do not create generic layers merely because several unrelated concepts are low-level, and do not delay product work to prove every future engine configuration.


---

# Smash Siblings Couch Multiplayer

## Extended abstract

Smash Siblings should support multiple local participants using distinct input sources. The first required case is one participant on keyboard and another on a gamepad.

The current input model largely treats keyboard and gamepad as interchangeable ways to drive one primary participant. That behavior should remain available for normal single-participant Ambition play, but Smash Siblings needs explicit source ownership.

## Identity model

Represent these as separate concepts:

```text
InputSourceId
    → ParticipantId
    → SessionSeatId
    → ControlChannelId
    → controlled actor
```

`InputSourceId` identifies a keyboard-and-mouse bundle, gamepad, touch controller, synthetic source, replay, or agent.

`ParticipantId` identifies the local or remote input authority.

`SessionSeatId` identifies the participant’s place in the current lobby or match.

`ControlChannelId` identifies the deterministic input stream consumed by simulation or rollback.

A connected device is not automatically a participant, and a participant is not the same thing as a temporary match seat.

## Assignment policies

Support:

```text
UnifiedPrimary
    keyboard, gamepad, and other local sources drive one participant

JoinToClaim
    an unassigned source joins as a distinct participant

ExplicitAssignment
    the host supplies the source-to-participant mapping
```

`UnifiedPrimary` preserves the existing single-participant workflow.

`JoinToClaim` provides the normal Smash Siblings couch flow.

## Lobby and character selection

The lobby or character-select flow should own:

* joining and leaving;
* source assignment;
* seat assignment;
* character selection;
* ready or lock-in state.

Before the match starts, freeze:

```text
participant
session seat
control channel
input sources
controlled actor
```

Simulation should receive independent deterministic control frames without knowing which physical device produced them.

## Disconnect behavior

A disconnect must not reorder participants or transfer ownership.

The disconnected participant should:

* retain its seat and actor;
* produce neutral input;
* reconnect to the same assignment when possible;
* change ownership only through an explicit host action.

## Input ownership

The couch-multiplayer work should drive the input split:

```text
ambition_input
    sources, participants, assignments, joins, bindings,
    contexts, semantic actions, and menu routing

ambition_platformer2d_input
    ControlFrame, InputFrameMode, platformer actions,
    presets, and control-frame translation
```

## First playable milestone

The first milestone is complete when:

1. Keyboard joins one participant.
2. A gamepad joins a second participant.
3. Both receive stable session seats.
4. Both select distinct controlled actors.
5. Both produce independent control frames during the match.
6. Disconnecting the gamepad does not transfer ownership.
7. Reconnecting restores the same participant.
8. Single-participant Ambition still supports unified keyboard and gamepad control.

This milestone does not require online multiplayer, complete rebinding UI, every device backend, or cross-build protocol compatibility.

