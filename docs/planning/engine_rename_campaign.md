# Engine restructuring candidates and couch multiplayer — remaining work

> **Verified against `cecd01ca` (2026-08-13).** The original stale
> `Sandbox*`/Ambition naming campaign is complete: `scripts/check_retired_crate_names.py`
> reports no retired production names. Its full record is archived at
> [`../archive/planning-superseded/2026-08-13/engine_rename_campaign.md`](../archive/planning-superseded/2026-08-13/engine_rename_campaign.md).
>
> This live file retains only the architecture/product work that was bundled
> behind that rename campaign. These are candidates/triggers, not one mandatory
> flag-day refactor. The focused actor carve remains
> [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md).

> ⭐ **RE-VERIFIED against `8bb0dd5a7` (2026-09-03), three weeks on, and the page is still true — with
> ONE candidate half-built that it does not know about.** A page that is
> accurate and looks stale costs a reader the same hour as one that is wrong, so
> the numbers are here rather than the date alone. Each was measured with the
> method the claim it checks already uses.
>
> - **The rename campaign is still closed.** `scripts/check_retired_crate_names.py`:
>   *"No retired crate name is live (14 tracked)."*
> - **Split persistence — not started.** `ambition_persistence` exists; neither
>   `ambition_user_settings` nor `ambition_game_save` does.
> - **Feel tuning — not started, and the page never said where it lives.**
>   `Platformer2dFeelTuningMonolith` is in `crates/ambition_combat/src/feel.rs`
>   and is named 135 times.
> - **Snapshot vocabulary — not started.** `ambition_snapshot` does not exist.
> - **The actor carve is live and its tracked metric has not moved.** The
>   monolith's `[dependencies]` table holds **28** `ambition_*` lines, counted
>   the way the decomposition page counts them (that section only — a whole-file
>   grep says 34 and includes six `[dev-dependencies]`).
>
> ⛔⛔ **AND THE INPUT SPLIT IS HALF BUILT, WHICH CHANGES WHAT THE SECTION BELOW
> IS ASKING FOR.** `ambition_input` exists — but it is the WHOLE of the proposed
> split, not the generic half, and `ambition_platformer2d_input` does not exist.
> Two consequences the page predates:
>
> 1. **`ControlFrame` already moved, the other way.** The section below assigns
>    it to the platformer crate; it lives in `ambition_platformer2d_core` today,
>    and `ambition_input` keeps only a re-export carrying its own
>    `TODO(compat-remove)`. So that line item is not "to do" — it is "finish
>    removing the compat re-export".
> 2. **The everything-enum is still closed, and its seam is now measurable.**
>    `Platformer2dInputActionMonolith` has **36 variants — 9 `Menu*` and 27
>    gameplay** — and 448 references. That 9/27 split is exactly the shell-vs-
>    platformer boundary the section proposes, so the split is a named partition
>    rather than a judgement call. ⚠ It sits in the crate the section wants to be
>    the GENERIC one, which is the reverse of where it should end up.

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

`Platformer2dFeelTuningMonolith` still mixes unrelated concerns. Split fields into domain-owned resources when those systems are next modified:

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

`Platformer2dInputActionMonolith` should not remain a closed everything-enum. Separate shell actions, platformer actions, and AmbitionGame-specific actions.

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

Keep the monolith name while the residue still mixes multiple ownership
domains. The active incremental carve is specified in
[`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md):
compile isolation and consumer dependency leakage now make decomposition explicit
work, while each individual boundary is still chosen by semantic ownership rather
than arbitrary file splitting.

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

