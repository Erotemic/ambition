# Participant and semantic action system — current residual work

**State:** OPEN, narrowed 2026-08-30.

The original participant/input migration is largely landed. This plan owns only
remaining participant-context and provider-action architecture.

## Landed model

The current architecture already provides:

- explicit participant/seat identity and device assignment;
- one per-seat control delivery road rather than a privileged seat-zero channel;
- participant-owned input contexts;
- per-seat menu frames and deterministic pause-menu ownership;
- `ambition_ui_nav::ListCursor`/focus/navigation primitives used by current
  menus;
- an action registry and semantic action IDs;
- provider-defined physical keyboard bindings through `ProviderBindings`;
- provider action state installed on participant entities;
- deterministic `SemanticActionPressed { id, participant }` publication;
- context filtering for provider-action presses;
- a global `ControlPrompt` for the current one-screen gameplay presentation.

These pieces should be extended rather than replaced by another input manager.

## P1 — decide the per-seat dialogue/gameplay model — ✔ DONE (verified `f51619ae2`, 2026-09-02)

⭐ **THIS IS BUILT AND ITS THREE ACCEPTANCE CONDITIONS ARE ALL PINNED BY ONE
TEST.** `dialogue_claims_the_talker_while_a_pause_still_stops_everybody`
(`actor_monolith/src/schedule/input_systems.rs`) sets `GameMode::Dialogue`, gives
seat 0 a `ConversationInputOwner`, and asserts seat 0's context owner is
`DIALOGUE_CONTEXT` while `seats.gameplay_owned(1)` holds and seat 1's
`jump_held` still arrives — *"ONE PLAYER READS A DIALOGUE BOX WHILE THE OTHER
KEEPS RUNNING — the thing the GameMode gate could not express, and the reason
this moved"*. Its second half then sets `Paused` and pins that pausing still
stops everybody.

**The answer to the design question, as the code states it:** *the context claim
carries ownership; `stops_the_world` carries the clock; and dialogue now says
only the first.* Ownership lives in `SeatInputContexts` / `ParticipantContexts`
(`ambition_input::participant`, with `DIALOGUE_CONTEXT`, `owner()` and
`gameplay_owned(slot)`), not in the mode.

⚠ **AND THE PARAGRAPH BELOW WAS TRUE BUT MISLEADING, WHICH IS WHY IT IS KEPT
HERE RATHER THAN DELETED.** `GameMode::allows_gameplay()` is still literally
`matches!(self, Self::Playing)` — unchanged. A reader checking that one function
would conclude nothing had happened. What changed is that it is no longer the
thing deciding per-seat gameplay routing. The repair was explicitly NOT "delete
the mode gate": pause, room transition and cutscene must keep stopping
everybody, and they still do.

Original text, for the record:

> `DialogueStopsTheWorld` already makes simulation-clock suspension explicit, but
> `GameMode::allows_gameplay()` still treats `Dialogue` as globally unable to
> route gameplay input. That is coherent for one active local participant. It
> cannot express one seat conversing while another seat continues gameplay.
> Do not solve this by threading the world-stop flag into another global gate.
> The open design question is what owns **per-seat permission to route gameplay
> while another seat owns a dialogue surface**.
>
> Acceptance for a promoted slice:
>
> - one seat can own/advance dialogue without stealing another seat's unrelated
>   gameplay controls when the experience permits it;
> - an experience may still explicitly choose world-stopping/global dialogue;
> - simulation-clock policy and input-ownership policy remain separate concepts.

## P2 — finish provider actions at composition boundaries

A provider can now declare an action, bind it to a key, and receive a semantic
per-seat press without adding a variant to the engine's closed built-in action
enum.

The remaining responsibilities are deliberately outside `ambition_input`:

- composition maps a participant press to the body/domain request that should
  consume it;
- controller/touch presentation chooses which finite physical/on-screen slot a
  provider action occupies;
- authoring schemas for bindings are added only when a tooling customer needs
  them.

Do not make `ambition_input` learn actor/body concepts to close the final hop.

## P3 — decide prompt multiplicity with the multiview customer

`ControlPrompt` is one global read model describing the primary local gameplay
surface. That is reasonable for one screen, especially for one shared touch
overlay.

With several independent local views/seats, one participant may need a different
prompt from another. Do not make `ControlPrompt` plural solely for naming
symmetry. Resolve the desired UI/product shape together with
[`multiplayer-and-multiview.md`](multiplayer-and-multiview.md):

- one shared display may intentionally have one prompt;
- split views may require view- or participant-indexed prompts;
- touch overlays are device/screen policy, not automatically one per participant.

## P4 — keep context vocabulary semantic

Do not add a `VEHICLE` context merely because mounted bodies have different
verbs. Menu/dialogue contexts exist because another surface owns the
participant's input. A rider still drives a gameplay body; the body's action
scheme should change the prompt/repertoire.

Add another context only when ownership/routing semantics actually differ.

Loading/retry and specialized UI contexts should migrate when their schedule and
ownership seams are clear. Avoid introducing a dependency cycle merely to make
all surfaces use the same enum immediately.

## Menu activation policy

Pointer activation already shares `MenuTapMode` policy across current menu
call-shapes, including destructive-row guarding. Gamepad submit does not have
the same stray-touch failure mode.

Treat extending destructive double-confirm behavior to gamepads as a product
feel decision, not unfinished plumbing.

## Test requirement

Input/menu tests are feature-sensitive. A plain per-crate `cargo test` may omit
substantial participant or presentation test modules. Verification for a changed
slice must name the feature composition that actually includes the code being
changed, plus the relevant real host when ownership depends on shell composition.

## Exit

The architecture is complete enough to leave active planning when:

1. participant identity, context and body assignment remain separate concepts;
2. per-seat dialogue/gameplay ownership has an explicit model;
3. a provider action can travel from authored/composed binding through a
   participant semantic edge to a domain request without a core action-enum edit;
4. controller/touch presentation has a deliberate finite-slot policy;
5. multi-view prompt ownership is explicit when a product actually uses several
   independent local views.
