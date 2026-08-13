# GPT 5.6 review through `ffa57c5` — what was done

Handed over by Jon on 2026-08-07, on `main`, worked linearly (no parallel
landing, so the branch ritual in `AGENTS.md` does not apply).

⛔ **every finding was checked against the tree before acting.** A review is a
claim, not a measurement — the standing lesson (`ASK the tool, don't MODEL it`)
applies to reviews of my own work as much as to mine of others'. All four
substantive findings confirmed by reading the code they name; two of them turned
out to be **worse in the shipped build than the review said**, and those are
recorded below because that is the part a summary loses.

---

## 1. Sparse local source → dense GGRS channel — FIXED

**Confirmed, and reachable three ways rather than the two the review names.**
`ControllerBinding::Human { device_slot }` carried a LOBBY SOURCE number, which
is deliberately sparse, and everything downstream read it as a dense channel:
`ControlAuthority::LocalInput { channel }` → `Brain::Player(PlayerSlot(channel))`
→ a GGRS handle. The session was sized by COUNTING the humans.

The third case is the one that matters, because it is the SHIPPED path: the
Smash demo claims `InputAssignmentPolicy::JoinToClaim` on its own routes, so its
select screen offers the KEYBOARD as source 0 and each pad after it. Two people
on two pads therefore publish `Pad(1)`/`Pad(2)` as source numbers,
`seat_input_participants_for_roster` spawned participants 1 and 2 beside the
boot-time primary — three participants for a two-handle session — and the
fighter reading `PlayerSlot(2)` received nothing for the whole match.

**The fix is the review's stated endpoint, not its fallback.** Rejecting sparse
channels before session construction was offered as a minimum; the explicit
mapping is what landed:

```text
LocalInputSource  Keyboard | Pad(n)   what somebody picked up — sparse
LocalChannelPlan  channel → source    position IS the channel — dense
ParticipantId     the channel         == the GGRS handle, == PlayerSlot
```

`ambition_input::channels` owns both types. `MatchParticipantRoster::
local_channel_plan()` is the ONE definition; `prepare_match` reads each seat's
channel out of it (and refuses a plan that disagrees with the seat it
describes); `LocalSeatTopology` and `SessionSeatingSource` store the plan rather
than a count. `local_input_channels() -> usize` is gone — finding 5 is right
that "how many?" stopped being a sufficient representation, and every caller
wanted the map.

Three things fell out of making the map explicit:

- **the keyboard is a variant, not a hole.** `slot.saturating_sub(1)` at two
  sites was the old way to say "the keyboard is not a device row". It could not
  express a keyboard player in any seat but the first, and it failed by
  returning `None` — a person who is simply inert.
- **a declared plan outranks `keyboard_owner_for`.** That function answers
  `Some(PRIMARY)` for EVERY `JoinToClaim` session ("nobody has claimed the
  keyboard, so it stays with the seat that has been playing"), so the shipped
  Smash couch bound player one to `Entity::PLACEHOLDER` — deaf to the controller
  in their hands — whenever nobody was actually on keys.
- **one controller cannot drive two fighters.** Preparation refuses it by name
  rather than opening a second channel nothing writes.

⚠ **not done, deliberately**: the `ParticipantId`/`PlayerSlot` rename. The
review says not to launch it, and `participant_seat` remains the one seam. The
new code routes through it (`realize_seat`, the autonomous reconciler) instead
of spelling `PlayerSlot(raw)`.

## 2. Conversation lifetime crossing rollback — FIXED, both directions

**Confirmed exactly as described**, and the diagram is worth keeping:

```text
before                              after
──────────────────────────────      ──────────────────────────────
sim  → runner.start_node   (!)      sim  → ActiveConversation
runner-ended → message → sim (!)      ↳ presentation projects the box
                                    runner-ended → STAMPED record → sim
```

- **opening**: `DialogState::start` ran from the sim schedule. It resets the
  line, the options and the typewriter and enqueues `runner.start_node`, so a
  rollback across the opening tick replayed all of that onto a box the player is
  reading. `DialogState` is left out of rollback so a rewind does not stutter it;
  replaying the write stuttered it anyway.
- **ending**: a `Message`, cleared on rollback — and the system that would
  re-deliver it (presentation, watching the live runner) does not execute
  BETWEEN resimulated ticks. So the rewind simply lost the end.

The endpoint chosen is the review's second option: **a stamped narrative input**.
`ObservedNarrativeEnd` records which conversation instance finished and the first
`SimTick` the simulation may act on it, and is deliberately NOT rollback state —
a rewind restores what the simulation DECIDED, never what it was TOLD. Opening is
now a projection (`open_dialog_ui_when_the_conversation_starts`, in `Update`,
memoized on the conversation instance so a restored authority is recognised).

`LiveConversation` gained `opened_at`, which does two jobs: it distinguishes one
visit to an NPC from the next (a node id alone cannot), and it is what lets the
projection tell a rewind-restored conversation from a new one.

⚠ **what is still NOT deterministic**, and the module says so: the Yarn runner
itself. WHICH tick it finishes on is presentation's answer. What is now true is
that every replay of that tick agrees with the original run.

## 3. `ActiveConversation`'s diagnostic projection — FIXED

`rollback_resource_clone_entity_set` localizes through the two bodies' stable sim
identities and is silent about everything else — so two conversations differing
in `input_owner` (whose controls the box captures), `dialogue_id` or `opened_at`
reported as identical. `rollback_resource_clone_entity_set_probed` adds the
value's non-entity fields to the same probe. No raw entity numbers in the
fingerprint: those differ across a load by design, which is why the entity half
goes through identities in the first place.

## 4. Combat participation vs social disposition — SEPARATED

`apply_actor_hit` asked `disposition.is_peaceful()` to decide whether a hit takes
health, so `ActorDisposition` answered two questions: *how does this actor regard
combat* and *may this body be hurt*. `CombatStanding::of(disposition,
ruleset_owns_death)` is the seam; `RulesetOwnsDeath` is the marker that already
means "a ruleset owns this body's death", which is the same decision as "this
body is in a fight". A town NPC keeps provoke-before-damage; a match fighter is
damageable whatever its brain is doing.

⚠ **the `MatchSeat` stand-down exemption STAYS, and its reason changed.** The
review calls it a special case around the conflation, and half of that is now
gone — a stood-down fighter is still damageable. What the exemption defends now
is the read-model: `BodyCombat::peaceful` drops the attack windup and swing
timers every frame, so a stood-down fighter could be hit and could not SWING, and
the anti-clump slot board stops seeing it. Removing it is a separate behavioural
question, not a tidy-up.

## 5. `local_input_channels()` as a warning sign — FIXED with finding 1

## 6. Avoid another planning/guard detour — HEEDED

This document is the only planning prose written for this round. No new guards,
no absence contracts, no coverage machinery.

## 7. Trailing whitespace — FIXED

One line in `docs/archive/queue-72h-2026-08-06.md`.

---

## Verification

Probes first at the level that owns each invariant, then the suites that would
notice a regression:

| what | result |
|---|---|
| `-p ambition_input --features input` | 113 pass (⚠ the bare package filter runs 55 of these) |
| `-p ambition_platformer2d_actor_monolith --lib` | 1200 pass before the conversation work, re-run after |
| `-p ambition_demo_smash` | 52 pass |
| `-p ambition_app --test app_it` | 318 pass, 10 ignored |
| `cargo check --workspace --all-targets` | clean |

New probes, each written to fail against the code it replaced:

- a human seated BEHIND a CPU lands on channel zero while the plan still
  remembers pad 1;
- pads 0/1/3 become channels 0/1/2;
- two seats on one pad are refused by name;
- a sparse couch seats one participant per person;
- a declared couch with nobody on keys gives BOTH seats their pads;
- a rewind past a narrative end replays it at the same tick;
- replaying the opening tick does not reopen a box the player watched close;
- a peaceful body a ruleset owns takes damage instead of barking.
