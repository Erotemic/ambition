# GPT 5.6 review through `c32e690` — verification and plan

Handed over by Jon on 2026-08-06, with a second round of reviewer feedback on the
plan. Worked on branch `review-fixes-2026-08-06` in the write-ahead worktree
(`/home/agent/code/ambition-workahead`) because another agent holds `main`.

⛔ **every finding below was checked against the tree before planning.** A review
is a claim, not a measurement — the 08-02 lesson (`ASK the tool, don't MODEL it`:
two guard drafts, five confident FALSE findings) applies to reviews of my own
work as much as to mine of others'. Six of eight confirmed, one blocked outside
my reach, one process. The `file:line` anchors are the evidence; re-grep before
trusting them, they were taken at `b24ccb6b2`.

---

## The reviewer's adjustments to my first plan

Recorded before acting, and they change three of the four items:

1. **Single-authority endpoint, not a second authoritative component.** *"prefer
   going directly to the single-authority endpoint rather than making
   `HeldByConversation` another independently authoritative rollback component if
   you are doing the larger refactor now. `ActiveConversation` should be
   rollback-owned; `HeldByConversation` and the conversation `ScriptedControl`
   should be projections that can be deterministically repaired from it."* The
   staged 1a repair (registering the marker) is **contingency only**, if the
   refactor has to be deferred.
2. **Mapped `Entity` inside the rollback resource is fine.** *"There is no need to
   invent a parallel stable string identity for this."*
3. **No implicit "claim every seat".** *"do not make missing attribution
   automatically mean 'claim every seat.' Make the conversation input owner
   explicit."* Actor-initiated derives from the controlled actor's participant;
   scripted/system conversations choose an explicit policy.
4. **Prompts**: the `ActiveDevice`-aware `ActionBindings::control_for(..)`
   approach, labels and glyphs sharing it.
5. **Menus**: calibration from the actual pad; repeat timing stays a
   user/machine preference.
6. **No `ParticipantId`/`PlayerSlot` rename now**, but *"avoid adding new direct
   assumptions that their numeric values are inherently identical. Put the current
   correspondence behind a small mapping seam/helper so the future split is
   localized."*
7. Patent Clerk stays untouched while the renderer submodule is hands-off; the
   Hall syntax scanner is frozen.
8. *"Use narrow checks. Do not burn the session on the full workspace suite."*

Sequence the reviewer set: **conversation authority → participant-specific
dialogue ownership → per-seat menu calibration → device-aware binding selection
(parallel if convenient).**

---

## R1 — the conversation authority ✔

**Confirmed, and it is the serious one.** Two mechanisms, both real:

- `break_dialogue_on_hit_or_separation` and `release_conversation_hold` are
  registered in `app.sim_schedule()`
  (`features/mod.rs:717-740`) — under a rollback host that IS the GGRS schedule,
  so they resimulate.
- `ScriptedControl` is rollback-registered (`rollback/domains/characters.rs:76`).
  `HeldByConversation` (`features/ecs/interact.rs:420`) is registered nowhere.
  `DialogState` (`ambition_dialog/src/runtime.rs:40`) appears in no rollback
  domain.
- The gate at `interact.rs:406` — `if held.get(brained).is_err()` — is exactly
  the [derive-MEMO](../../memory) class: an "already applied" flag that gates
  behaviour rather than caching a value. Rewind removes `ScriptedControl`, leaves
  the marker, and the gate then declines to restore the hold. A "held" NPC with
  no control override.
- `DialogState` is mutated from sim code — `dialogue.close()` at
  `interact.rs:323` and `:372`. A `close()` on a resimulated tick is an
  unrewindable write to non-rollback state.

### Shape

`ActiveConversation`, rollback-owned, is the only conversation state the
simulation reads:

```
initiator:   Option<Entity>     // remapped
talker:      Option<Entity>     // remapped
dialogue_id                      // which Yarn node is live
input_owner: ConversationInputOwner
```

Registered through `rollback_resource_map_entities` (`rollback/registry.rs:561`)
— the entity-remapping facility already exists, which is why mapped `Entity`
beats inventing a parallel string identity.

`DialogState` keeps everything at `runtime.rs:98-133` (reveal state, pointer arm,
Yarn option ids) and becomes a **projection** driven by a bridge outside the sim
schedule.

**The hold stops being independently authoritative.** One reconcile system, a
total function of `ActiveConversation` → world: the body the authority names as
talker gets `(ScriptedControl, HeldByConversation)`; anything else wearing
`HeldByConversation` loses both. Idempotent, so a rewind needs no memory —
whatever GGRS restored reconciles from the rewound authority on the next tick.
`HeldByConversation` gets NO rollback registration; it is the projection's own
record of what it placed, which is what `interact.rs:399-405` always said it was.

`ScriptedControl` stays rollback-registered: five other claimants use it (death,
flagpole, act clear, versus, seating) and this refactor does not own them.

⚠ **the one hazard, and it needs a probe rather than a claim.** A stale marker
plus a different claimant on the same body would let the reconcile strip a death
beat's `ScriptedControl`. That is impossible today by construction —
`interact.rs:402-405` records that all five other owners mark the PLAYER's driven
body while this marks the NPC — and the projection preserves that separation
rather than relying on it. Assert the invariant, and poison it with a body
wearing both a stale conversation marker and a death hold.

### What landed, and what it turned up

⭐ **the continuity code left `features/ecs/interact.rs` entirely.** Jon,
2026-08-07: *"if we add things to the monolith, try to do it so it's obvious what
the decomposition should be. we will need to address that bloat in the coming
days."* So this is not an addition to the pile — `crate::conversation` is four
small files (`authority`, `hold`, `rules`, `ui_bridge`) whose module header
carries the carve accounting: every outward edge is already a crate BELOW the
monolith, and exactly ONE inward edge remains
(`features::npc_ambient_bark_line`, named as a `pub(crate) use` rather than
opening the whole `npcs` module, so the size of the remaining coupling is
visible). What stays in `interact.rs` is the moment somebody presses Interact —
keeping a conversation alive was never an interaction.

⛔ **the first draft of the projection reintroduced the bug, and the probe is why
that is a footnote rather than a shipped defect.** The insert was gated on
`held.get(talker).is_err()` — the same memo shape, so a rewind that left the
marker would still skip restoring the override. The gate has to ask whether the
body is FULLY held (both components), which makes every half-state self-repairing
without knowing which half went missing.

⭐ **a rule fell out of this that is worth keeping: whether the authority may be
`Option` depends on whether the system OWNS it or observes it.** A system that
OPENS a conversation takes a hard `Res` — absence is a mis-composition and must
fail loudly, which is what four fixtures found out. A system that merely reads
whose conversation it is takes `Option` and treats absence as "no conversation",
because that is a true answer rather than a waiver. The Bevy param panic
recommends `Option<T>` for both, and it is right about only one.

**Follow-ups this turned up** (rows, not asides):

- ▢ **the narrative→sim edge is still a non-rewound read.**
  `close_conversation_when_the_narrative_ends` reads `DialogState::active()` to
  learn the Yarn runner finished. That is an EXTERNAL INPUT rather than a rule —
  it only ever closes, never opens or chooses a participant — but `DialogState`
  is not rewound, so a resimulated tick reads the live runner. ⚠ **not a
  regression**: every continuity rule read this resource before. The fix is a
  `ConversationEnded` message with `clear_message_on_rollback`, and it needs the
  runner's own lifecycle to have an opinion, so it is its own row.
- ▢ **`DialogState`'s entity API is now dead weight.** `set_speaker_entity`,
  `set_initiator_entity` and `participants()` have no production caller left.
  Deleting them is the "one authority" payoff; it is held back only so the
  deletion is not tangled with the move.
- ▢ **`dialog/yarn_bindings.rs` still asks `DialogState::speaker_entity()`** in
  three commands (`<<challenge>>`, `<<use_brain>>`, `<<restore_brain>>`), and
  `<<challenge>>` starts a FIGHT — a simulation effect keyed off view state.
  Repoint at `ActiveConversation::talker()`.
- ✔ **`stable_schema_name_count` was stale before this change** (336 recorded,
  337 actual). Display-only — the ratchet compares the name SET — but corrected
  while adding the two new names rather than left to rot.

### Placement

⛔ **checked, and it constrains the answer.** `ambition_input` and
`ambition_characters` are SIBLINGS — neither depends on the other; both sit on
`ambition_platformer2d_core` + `ambition_entity_catalog`. The type goes beside
the break rule in the actor monolith, registered from `rollback/domains/` — the
same split `ScriptedControl` already uses (type in `ambition_characters`,
registration in `ambition_platformer2d_runtime`). Evidence says runtime depends
on the monolith (`runtime/src/lib.rs:117` re-exports the monolith's input
systems); confirm that direction before placing, because backwards means a new
dep edge, and a new dep edge fails the contracts job with an opaque `cargo tree`
traceback until `fixtures/minimal_game/Cargo.lock` is regenerated and committed
WITH the manifest.

---

## R2 — participant-specific dialogue ownership ✔

**Confirmed, and the code says so itself.** `declare_in_session_input_contexts`
(`schedule/input_systems.rs:326-349`) claims `DIALOGUE_CONTEXT` on every
participant. Its own doc at `:313-318` states this was staged deliberately: *"this
is behaviour-identical today, on purpose… the per-seat version becomes a change
at THIS function."* The world-running half already landed (`:567` uses
`stops_the_world`, not `allows_gameplay`).

`ConversationInputOwner` is explicit — no implicit fallback:

- `Participant(..)` — derived from the initiator body's `Brain::Player(slot)`
  (`ambition_characters/src/brain/mod.rs:193`). ⭐ the control seam answers "who
  drives this body"; an entity index standing in for a seat is the thing the
  reviewer's finding 5 warns about.
- `Primary` / `AllParticipants` — chosen explicitly by scripted and system
  conversations, where that behaviour is actually intended.

⭐ **where it attaches makes the audit bounded.** `DialogState::start(dialogue_id,
npc_name, context)` (`runtime.rs:212`) is the single entry, and `DialogueContext`
already has exactly two constructors splitting the cases: `between(speaker,
listener)` (the actor-initiated path, `interact.rs:128`) and `scripted()`.
Threading the policy through that context makes the COMPILER enumerate every
start site instead of a grep.

Probe drives the **production** declarer with two participants — the existing
coexistence test hand-builds contexts and structurally cannot catch this.

### What landed

⛔ **the existing test was a green test sitting beside the bug it looked like it
covered, and it said so.**
`a_seat_browsing_a_menu_stops_driving_its_slot_and_the_others_keep_playing`
asserted that a surface CAN own one seat's input — by declaring seat 0's claim
itself and running only the resolver and the router, under a comment reading
*"re-running the declarer would retract what we just claimed, which is the
point."* Every word of that is true. It proved the SEAM supported per-seat
ownership while the only thing that declares in production claimed on every
participant.

⭐ **the generalizable tell: a test that skips a production system BY NAME to
protect its own setup is telling you the production system disagrees with it.**
Either the system is wrong or the setup is unreachable; "run less of the chain"
is never the third option, and a comment explaining why reads as diligence while
doing the opposite. The whole chain runs now, declarer included.

⚠ **one behavioural change worth knowing**: capture begins on the frame the
conversation OPENS rather than the frame the mode transition lands, because
`next_mode` applies a frame later. That is the more correct edge, but it is a
change and not a pure refactor.

---

## R3 — per-seat menu calibration ✔

**Confirmed.** `decode_menu_frame` (`input_systems.rs:733-737`) reads the global
`user_settings.controls.left_stick_deadzone` for every seat;
`populate_seat_menu_frames` (`:779-808`) has no `SeatActiveDevices` parameter at
all — while the gameplay path fifteen lines earlier resolves per pad
(`:596-602`).

Jon's rule is already written at `:589-595`: *filtering per pad, bindings
shared*, and the PREFERENCES (dash mode, inverted aim) stay machine-wide because
those are about the person. So the repeat delays at `:750-751` must NOT become
per-pad — they are habits, not hardware.

`decode_menu_frame` takes resolved `ControlFilters` alongside `user_settings`;
`populate_seat_menu_frames` gains the devices resource and resolves per row; the
global path resolves for primary; absent devices fall back to `from_settings`,
matching gameplay's `:601`.

⛔ **probe trap.** `ControlFilters::for_pad` returns the machine values UNCHANGED
when an explicit controller profile is set (`ambition_input/src/settings.rs:328-330`).
A probe that does not hold the profile at `Default` passes without testing
anything — green by construction, the failure mode this repo keeps hitting.

---

## R4 — device-aware prompts ✔

**Confirmed, plus a cause the review did not name.** `ActionBindings::label_for`
(`ambition_input/src/bindings.rs:343-351`) takes `.first()` and re-spells it, so a
keyboard `Z` bound before the pad binding stays `Z` under a DualSense.

⭐ the deeper fault is the TYPE: `label_for` takes a `GamepadStyle`, which cannot
express "this seat is on a keyboard". `SeatActiveDevices::gamepad_style_for`
collapses every non-pad device to the Xbox default and says so in its own doc
(`active_input.rs:180-188`). So this is a signature error, not a missing branch.

The selection primitive already exists on the glyph side —
`glyphs::bound_control(bindings, action, want_key)` (`glyphs.rs:55-66`),
dispatched from `glyph_for` on `ActiveDevice` (`:37-48`). Promote it to
`ActionBindings::control_for(action, device: ActiveDevice)` and have both glyph
functions and both label functions call the one primitive, so they cannot drift
apart again. `label_for_slot` already takes `Option<&SeatActiveDevices>`
(`bindings.rs:430`), so its callers do not move — it asks `for_seat` instead of
`gamepad_style_for`.

Probe on a MIXED keyboard+controller map — the normal primary map, which the
current test avoids by constructing a controller-only one — asserting both
directions.

✔ **the premise held**: `input_map` inserts the keyboard half at
`presets.rs:257` and the gamepad half at `:258`. ⚠ and it is the PRIMARY seat
specifically — secondary seats get `gamepad_only_map()`, where `.first()` is
already a button — so the seat that showed the wrong prompt is the one most
likely to be at a docked machine holding a pad.

**The probe was red with the exact predicted symptom** (`Some("Z")` where
`Some("Cross")` belongs), which is the difference between reading the code
correctly and being sure.

⭐ **and the fix turned up a SECOND instance nobody had reported.**
`ambition_demo_sanic`'s speedway legend resolved a `GamepadStyle` the same way
and passed it to the same method, so the sign in the level read `Z: JUMP` to
somebody holding a DualSense. It is fixed in the same commit — the signature
change is what found it, because a type that cannot express "this seat is on a
keyboard" produces the same bug everywhere it is used.

---

## R5 — the mapping seam ✔

No rename (the reviewer deferred it explicitly). What lands is a small helper
holding the `ParticipantId` ↔ `PlayerSlot` correspondence in ONE place, used by
everything R2 and R3 write plus the sites they already touch. No broader sweep:
the point is that the future split has one place to change, not that today's call
sites all move now.

Placement is forced by the same sibling-crate fact as R1 — it goes in the
monolith, where the conversion already happens inline at `input_systems.rs:575`
(`PlayerSlot(participant.id.slot())`).

---

## Not work this pass

- **Patent Clerk regeneration** (review finding 6) — CONFIRMED as a
  source-of-truth defect and BLOCKED. The SVG and rig live at
  `tools/ambition_sprite2d_renderer/assets/patent-clerk.svg` and
  `.../targets/characters/rigged/patent_clerk/patent_clerk_side.rig.json`, inside
  the submodule that is standing-order hands-off and that Jon has uncommitted
  work in. Diagnosis is exact and recorded: in the `Patent Clerk - Side Left`
  view, `path1115` is owned by both `torso` and `neck`; it needs one owner.
  Already a row in `JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`.
- **Generic feature-volume presentation** (finding 7) — deferred by the review's
  own priority. Principle to carry when that work resumes: gameplay extent ≠
  stretched sprite bounds; tile, repeat at native scale, or author a real
  visualization of the volume.
- **Hall census scanner** (finding 8) — FROZEN. No new parsing cases. The review
  is right for the reason actually observed: the scanner reported success while
  blind to single-quoted ids, and deleting a waiver went green for the wrong
  reason. The durable replacement is a machine-readable target manifest emitted
  by the renderer, which is another change inside the hands-off submodule — so it
  is a question for Jon, not a task.
- **`ParticipantId` rename** (finding 5) — deferred; R5 is the localized part.

---

## Verification posture

Narrow probes at each seam. `cargo check -p ambition_app` is the gate, never
per-crate. ⚠ a package filter under-reports here: `-p ambition_input` needs
`--features input` (99 tests vs 48 bare). The 11-second repo-tooling contracts
job runs before any commit — it is the only thing that catches a dependency edge
landing in the wrong place, and R1's placement question is exactly that risk.

⚠ **this worktree builds against its own target dir**
(`CARGO_TARGET_DIR=/home/joncrall/ambition-target-wa`). `AGENTS.md:128` says a
second target dir is not an escape because the volume runs at 92% — that is STALE
as of 2026-08-07: Jon ran a `cargo clean` and the volume is at 14% with 418G free.
