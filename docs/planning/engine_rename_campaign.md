# Engine restructuring candidates - remaining work

**Status:** candidates and triggers, not one flag-day refactor. The retired
`Sandbox*` naming campaign is complete; `python3
scripts/check_retired_crate_names.py` guards it. The focused actor carve is
[`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md).

State on 2026-09-17:

- **Split persistence:** not started. `ambition_persistence` exists;
  `ambition_user_settings` and `ambition_game_save` do not.
- **Feel tuning:** not started. `Platformer2dFeelTuningMonolith` is in
  `crates/ambition_combat/src/feel.rs`, and its use is increasing.
- **Snapshot vocabulary:** not started. `ambition_snapshot` does not exist.
- **Input split:** half done. `ambition_input` exists and holds the whole input
  model; `ambition_platformer2d_input` does not exist. `ControlFrame` is in
  `ambition_platformer2d_core`, and `ambition_input` keeps only a
  `TODO(compat-remove)` re-export. `Platformer2dInputActionMonolith` has 36
  variants (9 `Menu*`, 27 gameplay); that split is the shell/platformer
  boundary.
- **Actor carve:** live. Do not use the monolith's `[dependencies]` count as a
  progress metric: a carve makes it go up, because the kernel then depends on
  the new crate. Measure a carve by the domain that left, not by line counts.
- **Couch multiplayer:** implemented. `InputAssignmentPolicy`
  (`UnifiedPrimary`, `JoinToClaim`, `ExplicitAssignment`) is in
  `crates/ambition_input/src/sources.rs`, and a frozen session keeps its device
  mapping when a pad disconnects. Participant and view architecture is owned by
  [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md).

## Candidates by difficulty

The crate rename exposed several useful decomposition opportunities. They should not be executed as one architecture campaign. Difficulty here reflects dependency risk and semantic uncertainty, not merely the amount of file movement.

### Lower difficulty

#### Split persistence

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

#### Decompose gameplay tuning as touched

`Platformer2dFeelTuningMonolith` still mixes unrelated concerns. Split fields into domain-owned resources when those systems are next modified:

```text
MovementFeelTuning
CombatFeelTuning
TransitionTuning
TimeFeelTuning
```

This need not be a standalone campaign.

### Moderate difficulty

#### Extract generic snapshot vocabulary

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

#### Split generic input from platformer input

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

`Platformer2dInputActionMonolith` should not remain a closed everything-enum. Separate shell actions, platformer actions, and AmbitionGame-specific actions.

This is moderate rather than easy because it changes ownership at the boundary between devices, participants, session preparation, rollback input, and controlled actors.

### Continuing decomposition campaign

#### Shrink the actor monolith by destination

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

Keep the monolith name while the residue still mixes multiple ownership
domains. The active incremental carve is specified in
[`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md):
compile isolation and consumer dependency leakage now make decomposition explicit
work, while each individual boundary is still chosen by semantic ownership rather
than arbitrary file splitting.

### Higher difficulty

#### Untangle the shared tangle

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

#### Separate sprite format from sprite runtime

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

#### Replace the current quest model

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

### Guiding rule

Prefer decompositions that establish a clear owner and improve dependency direction. Do not create generic layers merely because several unrelated concepts are low-level, and do not delay product work to prove every future engine configuration.

## Naming is subordinate to ownership

The [current responsibility map](engine/architecture-responsibility-map.md)
supersedes cosmetic interpretations of this campaign. `actor_monolith` and
`shared_tangle` are useful warning labels while their work remains. `combat`,
`core`, `characters` and `runtime` are existing names, not proof that their mixed
contents already form coherent capabilities.

Do not mass-rename these to `actors`, `platformer` or other finished-sounding names.
First move the state, behavior, lifetime and installation that share an authority,
then name the resulting API. Preserve serialized/schema/wire identity during a
source-only rename unless a separately reviewed migration changes it.
