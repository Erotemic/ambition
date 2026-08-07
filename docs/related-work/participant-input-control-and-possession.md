# Participant input, control authority, and possession

**Checked 2026-08-07.** This page compares the ownership problem, not just input
APIs: where device state lives, how contexts are selected, what controls a body,
and whether camera/view ownership is forced to follow that body.

## The Ambition question

Ambition is deliberately splitting facts that are commonly bundled together:

```text
physical devices
    -> persistent participant
    -> bindings / processors / interactions
    -> logical actions
    -> ordered input contexts
    -> deterministic participant packet
    -> participant -> slot -> control authority -> controlled actor
                                      |
                                      +-> view subject is a separate policy
```

The active design is in
[`participant-action-system.md`](../planning/engine/participant-action-system.md).
Its strongest invariant is that possession changes the controlled actor without
recreating participant input state, and that input stays in screen axes until the
controlled body's motion frame is known.

The unfinished parts are consequential rather than cosmetic:

- PA2–PA4 still owe complete context and cue ownership for ordinary surfaces;
- PA5 must retire the seat-0 routing special case and make local-N participant
  routing ordinary;
- non-capturing contexts still lack action-specific ownership and prompt rules;
- PA6 must choose how movement/aim frame policy becomes deterministic across a
  network session;
- PA7 must prove an external provider can add an action without editing the
  closed Ambition adapter enum.

---

## Unreal Engine — the closest ownership split, but view still leans toward the controller/pawn pair

Unreal separates **Controller** from **Pawn**. A Controller is non-physical and
can possess/unpossess a Pawn; the standard human and AI paths are
`PlayerController` and `AIController`. The default relation is one controller to
one pawn at a time.

Source: [Controllers in Unreal Engine](https://dev.epicgames.com/documentation/unreal-engine/controllers-in-unreal-engine?lang=en-US)
(Epic, official). The page calls Controllers "non-physical Actors" and documents
`Possess` / `Unpossess`.

Enhanced Input puts **Input Mapping Contexts** on a local player. Contexts may be
added, removed and prioritized, and priority resolves collisions when multiple
contexts map the same action.

Source: [Enhanced Input in Unreal Engine](https://dev.epicgames.com/documentation/en-us/unreal-engine/enhanced-input-in-unreal-engine)
(Epic, official). The key sentence says contexts can be "added, removed, or
prioritized for each user".

Camera is related but not identical. Unreal's `PlayerCameraManager` owns the
final view for a player, while a possessed Pawn may automatically become the
view target. That gives Unreal a separable camera mechanism, but possession has a
built-in path that couples the common case.

Source: [Cameras in Unreal Engine](https://dev.epicgames.com/documentation/unreal-engine/cameras-in-unreal-engine?lang=en-US)
(Epic, official).

### Lesson for Ambition

Unreal validates three Ambition choices:

1. **control authority should be non-physical;**
2. **contexts belong near the participant/local-user layer, not on the body;**
3. **view has its own object/policy even when the default follows possession.**

The place to be more explicit than Unreal is the spatial boundary. Ambition's
participant owns input semantics but the controlled body owns the frame that
interprets movement and aim. That is important for unusual gravity, possession
between differently oriented bodies, replay, and deterministic networking.

---

## Unity Input System — strong per-user device/action ownership, weaker control-body semantics

Unity's `PlayerInput` associates one player with a set of Input Actions. Multiple
`PlayerInput` components receive private action copies, can have devices paired
to them, and can enable/switch action maps independently. Its camera association
is explicitly only required for split-screen.

Source: [PlayerInput component](https://docs.unity3d.com/Packages/com.unity.inputsystem%401.5/manual/PlayerInput.html)
(Unity, official).

Unity's action editor supports multiple bindings per action and device-generic
control paths. Interactions add phases such as waiting, started, performed and
canceled; hold/tap logic can therefore live at the input-action layer rather
than in gameplay code.

Sources:

- [Input Actions Editor](https://docs.unity3d.com/Packages/com.unity.inputsystem%401.8/manual/ActionsEditor.html)
  (Unity, official).
- [Interactions](https://docs.unity3d.com/Packages/com.unity.inputsystem%401.13/manual/Interactions.html)
  (Unity, official). An Interaction is described as a "specific input pattern".

### Lesson for Ambition

Unity is the strongest prior art here for **participant/device pairing and
binding UX**. It is not a model for the lower deterministic boundary: the cited
input facilities do not define a persistent possession relation, body-local
reference-frame resolution, or a simulation packet shared by human, replay, AI,
and network sources.

Ambition should therefore make the seam between `ParticipantActionState` and
`ControlFrame` clearer rather than growing device abstractions below it. The
competitive target is: Unity-grade rebinding/local-user ergonomics above a much
stricter deterministic packet and control-authority seam.

---

## Godot — simple global action map, intentionally little ownership machinery

Godot's `InputMap` is a singleton that stores named actions and the input events
that trigger them. It supports runtime modification and action deadzones.

Source: [InputMap](https://docs.godotengine.org/en/stable/classes/class_inputmap.html)
(Godot, official). It is documented as a "singleton that manages" action events.

`Camera2D` is associated with a viewport, with one active camera per viewport;
this is a presentation relation rather than a participant/control-authority
model.

Source: [Camera2D](https://docs.godotengine.org/en/stable/classes/class_camera2d.html)
(Godot, official).

### Lesson for Ambition

Godot shows the value of a small zero-ceremony action surface, but it is the
wrong ownership baseline for Ambition. A global action map is simple for one
local participant; PA5 specifically exists because Ambition must make multiple
participants, device loss/reassignment, UI ownership, possession, headless input,
and rollback coexist without global resources fighting over authority.

---

## Bevy + Leafwing Input Manager — useful action machinery, not the engine contract

Ambition currently uses Leafwing Input Manager. Its standard model attaches an
`InputMap<A>` to an entity and exposes an `ActionState<A>` for gameplay queries;
it also has processors, clash handling, timing and serialization-friendly action
diffs.

Source: [leafwing-input-manager crate documentation](https://docs.rs/leafwing-input-manager/latest/leafwing_input_manager/)
(third-party upstream crate documentation).

This is valuable plumbing, but Ambition must keep it below the public contract.
`InputMap` is not the participant, `ActionState` is not the rollback packet, and
the entity carrying them must not accidentally become the controlled body just
because the library's smallest example puts input on a `Player` entity.

---

## Competitive design frontier

| Problem | Strong prior art | Ambition bar |
|---|---|---|
| Per-user devices and rebinding | Unity `PlayerInput`, Unreal Enhanced Input | participant-owned bindings/device facts with one cue/touch projection |
| Context priority | Unreal Input Mapping Contexts | ordered contexts **plus action-specific consumption and cue ownership** |
| Control-body separation | Unreal Controller/Pawn | persistent participant -> slot -> authority -> body, with possession changing only the last relation |
| Spatial interpretation | mostly game-specific in the cited systems | body-resolved movement/aim frame after participant action resolution |
| Local multiplayer | Unity per-player actions/devices | same participant model for local-N, UI ownership, loss/reassignment, and deterministic packets |
| View ownership | Unreal PlayerCameraManager / view target | explicit `ViewSubject`, defaulting to controlled body but never synonymous with control authority |
| Replay/AI/network | not unified by the input APIs above | all produce the same lower semantic packet as human input |

## What still needs to be designed

### 1. Action-specific context arbitration

The eventual participant result should not merely say which context is highest
priority. It should be able to answer, for each action:

- which contexts are active;
- which context observes it;
- which context consumes it;
- which context owns the cue/prompt;
- whether transition into/out of that context preserves or suppresses held-edge
  state.

That is the missing model behind the current non-capturing-context caveat. Copy
Unreal's priority idea, not its single winner as the whole abstraction.

### 2. A synchronized frame-policy contract

Before production network play, PA6 must choose one explicit policy for
`ScreenRelative` / body-relative movement and aim interpretation: encode policy
in each control packet, freeze it into the session contract, or change it only
through synchronized commands. Local settings cannot be an implicit simulation
input.

### 3. Participant-keyed UI ownership

PA5 should prove that one participant can own a UI surface while another keeps
playing when host policy allows it. That is a better forcing case than merely
"two gamepads move two actors" because it exercises contexts, devices, focus,
prompts, and simulation routing at once.

### 4. Provider-extensible actions without a second action system

PA7 should be acceptance-driven: one external provider registers one custom
action and carries it through binding, context, consumer, cue and touch without
editing a core closed enum. The proof should delete any adapter it temporarily
needed.

## What this changed

This comparison strengthens the current participant-centered plan rather than
replacing it. The important competitive opportunity is **not** to invent another
input library. It is to make control authority, context ownership, spatial frame,
view subject, and deterministic packet five explicit relations that can vary
independently.

That is both easier to reason about under possession/unusual gravity and more
reusable than the common "input component on the controlled character" shape.
