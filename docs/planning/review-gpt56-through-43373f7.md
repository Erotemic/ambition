# GPT 5.6 review through `43373f7` — what was done

Handed over by Jon on 2026-08-08, on `main`, worked linearly. This is the
REVISED review: it retires two of the previous round's findings (the
sparse-source→dense-channel behavioural defect, the conversation diagnostic
fingerprint) and carries six forward, four of which it asks to be designed as
**one boundary** rather than four patches.

⛔ **every finding was checked against the tree before acting**, and two of the
four immediate ones were checked by writing a PROBE that failed first. Three
findings turned out to be worse in the shipped build than the review said, and
those are the part a summary loses.

---

## Three things the review did not know

### Ordinary play is a GGRS host

`game/ambition_app/src/app/cli.rs:649` and `:1011` set
`SimulationHost::Ggrs` for the normal desktop build, not just the dev
observatory. So the story game's NPCs, Yarn commands, inventory and wallet all
run under a rollback host. Prediction distance is zero at rest — F9 raises it for
a bounded proof pulse — which is why none of this was visible. **Latent, not
absent**, and every finding below is about the shipped path rather than a
hypothetical netplay future.

### `PendingChallenge` was not rollback state at all

`<<challenge>>` inserted it from `Update`; `tick_pending_challenges` consumes and
REMOVES it in the sim schedule; nothing in `rollback/domains/` registered it. So
a rewind restored the simulation to before the removal and left the removal
standing — the fight the narrative armed was quietly disarmed by a rollback. That
is word-for-word the `InventoryRestored` latch failure already written up in
`rollback/domains/items.rs`, in another domain.

### Combat participation had TWO proxies, not one

The review names `RulesetOwnsDeath` (in `CombatStanding::of`). The stand-down
guard at `features/ecs/actors/update.rs` used a **different** one,
`Has<MatchSeat>`. A seat is not participation either: an eliminated fighter keeps
its seat, and its body keeps standing until a ruleset removes it. So one missing
fact was being approximated by two components that mean other things.

---

## 1. Gameplay-bearing Yarn commands — FIXED, and it was more than Yarn

**Confirmed, and the crossing was wider than the command list.**

The review's mechanism is exactly right: the Yarn runner is content executing in
real time, outside rollback, and the channels its commands write are cleared on
rollback by a host that will not re-run the presentation system that filled them.
What the review lists as "some directly mutate rollback-owned components" is
three of the twelve commands, and they are the worst three: `<<give_item>>`,
`<<buy_item>>` and `<<sell_item>>` mutated `OwnedItems` and `BodyWallet` — both
registered rollback state — from `Update`.

Two crossings the review did not name:

- **`<<challenge>>` inserted a component**, which is a structural write into the
  simulation from outside it, carrying an `Entity` across a boundary that remaps
  entity handles.
- **the cut-rope replay** was worse in the other direction: the Yarn command
  latched a resource from `Update`, and the sim system that drained it waited on
  `DialogState::active()` — a **sim system branching on presentation state that
  is deliberately not rewound**. For a ROOM RESET that means despawning the world
  on a tick the other timeline did not. It waits on the conversation authority
  now, which is the thing the box is a projection of.

### The shape that landed

`NarrativeInputLedger<M>` in `conversation/ledger.rs`, and it is deliberately the
**mirror image of `ExternalEffectJournal`**:

```text
ExternalEffectJournal    sim ──▶ outside   held until the frame is CONFIRMED
NarrativeInputLedger     outside ──▶ sim   held until the tick can never be
                                           SIMULATED AGAIN
```

Four rules, each of which is a defect that would otherwise be reachable:

1. **stamped on the way in** with the conversation instance and the first tick
   the simulation may act on it (the next one — presentation observes in
   `Update`, after this frame's simulation has run);
2. **released on an EDGE** (`from_tick == now`), not a level. The slot this
   replaced used `<=`, which is safe only because closing a conversation twice is
   closing it once. A grant is not idempotent;
3. **instance-gated** — a record releases only while its own conversation is
   live, which is Jon's call (2026-08-08) and the same judgement
   `ExternalEffectJournal::discard_after` makes outbound. The visible cost is
   stated rather than hidden: in netplay a remote player who breaks your
   conversation can take back an item you watched being granted;
4. **pruned to the replay horizon, never by consumption.** Nothing marks a record
   used — a replayed tick has to find it again.

The classification is the deliverable, and it is a table in
`dialog/yarn_bindings.rs`. `play_sfx` and `spawn_fireworks` stay on their own
channels because their consumers are already downstream of the outbound
quarantine's release; deferring a sound to a simulation tick would delay it for
nothing. Dialogue visit counts and quest advancement are **save state, not
simulation state**, and are named as such rather than inheriting whichever side
happens to call them.

---

## 2. `ObservedNarrativeEnd` depth one — FIXED, and the probe failed first

**Confirmed exactly as stated.** The type's own doc argued depth one was safe
because "a player has to read the first one", and the review is right that this
is not an engine invariant. `<<reset_cut_rope_room>>` is reached *before the
player has dismissed the line it is on* — the counterexample was already in the
tree.

The probe (`two_narrative_ends_in_one_window_both_replay`) was written and
watched FAIL against the tree before the fix: two conversations complete at ticks
103 and 106, a rewind to 101 restores the first, and tick 103 left it live because
the second record had overwritten the first.

`ObservedNarrativeEnd` is deleted. The end is now one payload type in the ledger
above — **not a special case**, which is the structural half of the repair.

---

## 3. `(dialogue_id, opened_at)` — FIXED, with the sharpening Jon added

**Confirmed, and Jon's sharpening is the load-bearing part.** He wrote:

> a simple rollback-rewound counter is not automatically sufficient, because a
> corrected branch could reuse the same counter value for a different
> conversation.

That is right, and it generalises into the contract the type now states:

> **The id must be deterministically re-mintable by resimulation, from the
> conversation's own opening facts.**

Which rules out both obvious answers. A **nonce** cannot be re-minted at all, so a
resimulated tick mints a different one and every record stops matching. A
**rollback-rewound counter** re-mints perfectly and is still wrong — it encodes
HISTORY, so a corrected branch that opened a *different* conversation mints the
same number for it. The counter would rewind exactly as designed and hand the
record to the wrong conversation anyway.

So `ConversationInstanceId` is a function of CONTENT: `opened_at`, the Yarn node,
and both bodies' `SimId`s. Not `Entity` (remapped on load) and not
`DialogueContext`'s character ids (two identical NPCs share theirs — the review's
own counterexample is exactly what a character id cannot separate).

⚠ **the weak no-timeline case is answered by the ledger, not by the id.** A
composition whose clock never advances cannot tell two visits apart, and that is a
degenerate clock rather than a hole in the identity. What made it a BUG was the
level-triggered release; the ledger delivers once and drops the record when there
is no `SimTick`, because with no timeline there is no replay to reproduce
anything for. That fork is stated in one place instead of as
`tick.map_or(u64::MAX, ..)` at each reader — a sentinel meaning "act immediately"
that read as an ordinary comparison.

---

## 4. Presentation attachment — FIXED, probe failed first

**Confirmed and reachable.** The memo meant "I projected this once" and was never
cleared, so:

```text
predicted remote hit breaks the conversation
  → presentation closes the box at end of frame
  → the real input arrives, the correction restores the SAME conversation
  → the memo refuses to rebuild it
```

Simulation holds the talker and captures a seat while the player looks at
nothing. `a_conversation_restored_after_its_authority_vanished_is_projected_again`
was written and watched fail.

⭐ **the two systems became one, and that is the actual repair.** Opening and
closing were separate systems where only ONE wrote bookkeeping — which is how
presentation's record of what it had done outlived the thing it recorded. A
projection is current derived state or it is not a projection.

⚠ `replaying_the_opening_tick_does_not_reopen_the_box` covers the other case (the
box closes while the authority stays live) and stayed green.

---

## 5. `ParticipantId` as `ControlChannelId` — RULE STATED, no refactor

**Agreed with, including the instruction not to act on it further.** The
behavioural defect is fixed and `LocalChannelPlan` must not be undone. What
remains is that one number carries three concepts with three different lifetimes:

```text
LocalInputSource   what somebody picked up          — sparse, separated ✔
ParticipantId      the PERSON                       — outlives the session
SessionSeatId      a seat in this session's topology — MISSING
ControlChannelId   a deterministic input channel    — MISSING
PlayerSlot         what the simulation reads
```

The deliverable is a **rule plus a place to find it**, not a type: stated in
`ambition_input/src/channels.rs` and `participant_seat.rs` (where somebody about
to break it is already reading), plus one `tracks.md` row under parallel
maintenance. New code must not add arithmetic equality between a participant and
a `PlayerSlot`/GGRS handle; route through `LocalChannelPlan`, so a future
`ControlChannelId` replaces the spelling in one place rather than in every caller
that did the arithmetic itself.

---

## 6. Combat participation — FIXED, and it replaced two proxies

**Confirmed, and the review understates it** — see the third item at the top.
`RulesetOwnsDeath` answers *whose business this body's death is*; `MatchSeat`
answers *which seat it was entered into*. Neither means *is this body in a fight*,
and the correlation both ride on breaks at exactly one reachable point:
**elimination**.

`ActiveCombatant` is that fact, attached by the ruleset that enters a body into a
fight (`prepared_match`, beside `RulesetOwnsDeath`) and removed by the one that
takes it out (`spend_fighter_stocks`, with `FighterEliminated`). Jon's call
(2026-08-08): an explicit component, not a match-phase derivation (every reader
would have to name the ruleset, and a training dummy has no answer) and not a
third `ActorDisposition` arm (that is the conflation this ends).

It now answers all four questions that were being answered separately:

| question | asked before | asks now |
|---|---|---|
| does a landed hit take health? | `RulesetOwnsDeath` | `ActiveCombatant` |
| does the body stand down for want of a target? | `Has<MatchSeat>` | `ActiveCombatant` |
| does it keep attack windup through the read-model rebuild? | `ActorDisposition` alone | `CombatStanding` |
| does it compete for a combat slot? | `ActorDisposition` alone | `CombatStanding` |

⚠ **the last two were still broken after the damage fix landed.** A socially
non-hostile combatant got `BodyCombat::peaceful` every frame, which drops the
windup and swing timers — so it could be hit and could not SWING. It was masked
because `MatchSeat` kept match fighters artificially `Hostile`.

The existing damage test named the wrong authority in its fixture (it spawned
`RulesetOwnsDeath`); its intent was right and it keeps it, plus the poison it was
missing — **death ownership WITHOUT participation**, which is the eliminated
fighter, and which the old code reported as a live combatant.

`ActorDisposition` is an AI/social fact again, and the state that was
inexpressible is now a test:

```text
active combatant · human controlled · socially non-hostile · damageable · able to attack
```

---

## 7–8. The opportunistic pair — DONE

`scripts/regen_music_registry.py` carried a second copy of the consuming crate's
name as a fallback default, directly beneath a comment reading *"two readers of
one declaration, never two declarations"*. It reads `scripts/lib/asset_roots.sh`
now and RAISES if it cannot — the posture the renderer's `publish_root()` already
takes, and for the same reason: guessing that name is how 69 cues were once
published into a directory nothing reads while both halves reported success.

Provenance trimmed from comments in files this campaign already edited. Invariant
sentences stay; "GPT X found Y on date Z, finding N" goes. Not a sweep.

---

## What this cost the schema, and what caught it

Two version bumps, and the second one is the interesting one:

- **v16** — the narrative boundary. Six stable names joined the wire format.
- **v17** — `actor.active_combatant`. Unlike the markers beside it, this one is
  REMOVED during a match, so a v16 peer rewinding past an elimination puts back a
  fighter that is out and this one does not.

⛔ **`rollback-wire-format-is-frozen` went red on the first of those, which is the
contract working.** The ratchet has a baseline nobody regenerates automatically,
so a registration added without recording it leaves the contracts job red for
whoever runs it next. Both baselines (`tests/rollback_schema_baseline.txt` and
`slice-evidence/rollback-schema-baseline.json`) are updated with the reason each
name joined.

⚠ **nothing catches you forgetting the version bump itself** — the baselines
catch the registration, and the version is a separate hand edit. That is worth
knowing before the next schema change.

---

## Gates

- `cargo check -p ambition_app` — the gate, never per-crate.
- `cargo test -p ambition_app --test app_it` — 318 tests, ~150s, green.
- `python3 scripts/check_absence_contracts.py --check` — 25/25.
- `cargo test -p ambition_platformer2d_actor_monolith --lib` — 1216 green.

The next review baseline is `2388f4631`.
