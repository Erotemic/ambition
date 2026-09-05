# World facts, observations and memory — Engine 1.0 program

**State:** OPEN — authoritative-world/AI-belief separation is settled; fact and memory representation is not.

## Goal

Give systemic characters and agent tooling structured access to **what is true,
what happened, and what a particular actor could know**, without making an LLM
or dialogue generator authoritative over the simulation.

The governing rule is:

> The simulation determines what is true. AI decides what characters think,
> want, say and try to do about it.

## Three layers

### Authoritative world facts

Examples: door open, machine powered, item custody, actor alive/location,
encounter outcome, persistent world mutation.

⭐⭐ **MEASURED 2026-09-04: THIS LAYER IS NOT MISSING — IT IS `AmbitionGameSaveData`,
and the open question is which of its rows a rule can READ.** The page's
"Candidate crate" section says *"do not begin with a universal key-value fact
database; prefer typed domain facts"*, and that is already what shipped: the save
holds **thirteen** typed fact families, not a string map. ⚠ `AmbitionGameSaveData`
has FOURTEEN `pub` fields; `version` is schema metadata rather than a fact, which
is the one exclusion — said here so a recount reads as agreement instead of a
correction. **TEN published conditions read SEVEN of them** (⚠ this sentence has been
re-measured three times in one day — six reading four, then nine reading six,
now ten reading seven as `wallet.can_afford` landed. ⇒ Re-run
`scripts/authored_route_gates.py` rather than quoting it; the counts here are
dated, not live):

| durable fact family | route-readable? |
|---|---|
| `flags` | ✔ `world.flag_set` |
| `switches` | ✔ `world.switch_on` |
| `items` | ✔ `inventory.holds` |
| `occurrences` / `custody` | ✔ `custody.is_held` |
| `encounters` | ✔ `encounter.cleared` (published 2026-09-04) |
| `bosses` | ✔ `boss.cleared` published 2026-09-04, retiring a mirror slice; **REACHABLE since 2026-09-05** — its three executable authored callers spell `cove.mockingbird`, the AUTHORED encounter id the save is now keyed by ([question 57](../awaiting-maintainer-decision.md), ruled and implemented). ⛔ They could never be true before that: they passed the BEHAVIOUR id against a save keyed by the LDtk placement, and a missing key reads `Untouched`, so the gate stayed shut and looked like content nobody wrote. Two guards hold it now — a wrong id is a RED, and an end-to-end arm takes the id from the booted room and the question from the shipped dialogue |
| `quests` | ✔ `quest.active` (published 2026-09-04, retiring a mirror slice — and the first condition published by the GAME) |
| `wallet` | ✔ `wallet.can_afford` (published 2026-09-04, retiring the mirror's LARGEST customer — ten authored shop lines) |
| `dialog_visits`, `checkpoint`, `minted_items`, `inventory_saved` | ⛔ nothing publishes a condition |

⇒ **So the first slice of THIS program is not a representation decision, it is a
publication gap**, and it is the same shape the capability-progression program
turned out to have: the fact exists, the reader does not. ⚠ That does NOT mean
publishing all eight — `inventory_saved` and `minted_items` are restore
mechanics no rule should ask about, and a condition per field would be the
key-value database this page refuses, wearing typed clothes. The ones with an
obvious authored customer are `encounters` (*"has this arena been cleared"* —
distinct from its switch, which is the mechanism's state rather than the
outcome), `bosses` and `quests`.

✔ **`encounter.cleared` is the first of those, and it ships from
`ambition_encounter_features` rather than the actor monolith** — the fifth
condition provider and the first to live beside the systems that WRITE the fact,
which is what "a domain owns its own publication" has to mean once the domains
stop sharing a crate. It is also the first condition over a NON-BOOLEAN durable
fact, and it publishes one named state rather than a
`state_is(encounter, state)` accessor: a generic reader would be exactly the
key-value fact database this page refuses, arriving one enum at a time. A second
state becomes a second named question when something wants it.

⭐⭐ **AND THE PUBLICATION GAP IS NOT A GAP — IT IS A FORK WITH A PRECEDENT AND
LIVE CUSTOMERS (measured 2026-09-04).** The remaining eight families are not
merely unpublished; several are already ANSWERED, by a second mechanism, to
authored content that ships.

⚠ **RE-TENSED 2026-09-04 LATE, by this file's own carve.** As written this
paragraph said the mirror *carries* the boss and quest slices; the two migrations
below deleted both, so the present tense went stale within hours of the sentence
being true. Kept as history rather than removed — the argument is what makes the
migrations legible, and *"re-tense, do not delete"* is the queue's own routine
for exactly this.

`ambition_dialog::YarnStateMirrorData` (`crates/ambition_dialog/src/bindings.rs:16`)
**carried** `bosses_cleared` and `quests_active` — both now gone — alongside
`visit_counts`, `wallet_balance` and `extras`, which remain. And
`game/ambition_content/src/yarn_vocabulary.rs` **bound** `boss_cleared(id)`,
`quest_active(id)`, `visit_count(id)`, `wallet_balance()` and `can_afford(price)`
as bespoke Yarn functions over it; the first two are registered systems asking
the catalog now, the last three still read the mirror. **Authored content called
them then and calls them still, unchanged** — `cove.yarn:3`, `cove.yarn:220`,
`kernel.yarn:271`, `kernel.yarn:293`.

⇒ **So this is the "second authority" shape, and BOTH modules already say so at
the site.** `authored_conditions.rs`: *"facts already exposed through the
authored-condition catalog must be queried there rather than duplicated here.
The mirror remains only for facts the catalog cannot answer."*
`yarn_vocabulary.rs:415`: *"Two mechanisms answering one question is exactly the
second authority this project refuses elsewhere."*

✔ **AND THE MIGRATION HAS ALREADY HAPPENED ONCE, which is what makes this a
carve rather than a proposal.** The mirror's flag slice is GONE
(`yarn_vocabulary.rs:107`): *"It existed so `flag(id)` could read a save flag
synchronously; that question is the condition catalog's `world.flag_set`, asked
live."* ⇒ Publishing a condition here **retires a mirror slice**; it does not add
an unused verb.

⛔⛔ **WHICH REVERSES THE OBVIOUS CAUTION, and the reversal is the point.**
`capability-progression-and-world-gating.md` measures five of nine published
conditions as authored NOWHERE, so "publish more conditions" reads as
dormant-cluster growth. **It is the opposite here**: `boss.cleared` and
`quest.active` have authored callers on day one — the callers exist, through the
other door. The dormant-cluster risk applies to conditions with no customer, not
to conditions whose customers are currently served by the fork.

⚠ **AND `encounter.cleared` DOES NOT ALREADY COVER IT.** `encounters`
(`PersistedEncounter`) and `bosses` (`PersistedBossDefeat`) are separate save
fields (`save_data.rs:313`, `:317`), and the mirror reads `data.bosses`. Checked,
because "the boss is an encounter" is the plausible assumption that would have
made this look already-done.

✔✔ **AND `boss.cleared` IS LANDED (`39a48d4fa`), which makes the argument above
a receipt rather than a proposal.** `ambition_boss_encounter` publishes it — the
SIXTH condition provider — `boss_cleared(id)` is now a registered system asking
the catalog live rather than a closure over the mirror, and
`YarnStateMirrorData`'s boss slice **and its refresh loop are deleted** <!-- cite-ok: the field this row records RETIRING; naming a deleted symbol is the receipt -->.
Authored `.yarn` content keeps its spelling and gains the live answer, so no
content migration was needed. Three tests, poison-verified; the whole workspace
checks clean and the four affected suites are green.
✔✔ **AND `quest.active` LANDED WITH IT (`03f31eee3`) — the sibling, and the
FIRST CONDITION PUBLISHED BY THE GAME.** The engine has no quest domain: the
roster is `ambition_content::quest::default_quest_specs` and the pump is
registered by `AmbitionQuestContentPlugin`, both in `game/`. So *"a domain owns
its own publication"* puts it there, and it shows the catalog is extensible by a
GAME and not only by the engine — a composition without Ambition's quests never
sees the question. `quests_active` and its refresh loop are deleted too: the
mirror's THIRD slice to go, after `flag` and the boss slice.
⇒ **The publication table above records both.**

⛔ **THE MIRROR MIGRATION IS NOT FINISHED, AND THIS PARAGRAPH SAID IT WAS.**
It read: *"the remaining fields are deliberately staying … `visit_counts` is
dialogue's own bookkeeping rather than a world fact, and `wallet_balance` is a
NUMBER, which the catalog's boolean-outcome shape cannot express."* Both halves
of that are true and the conclusion does not follow, because the ruling
enumerated the mirror's FIELDS and the fork is in its FUNCTIONS.

⭐⭐ **MEASURED 2026-09-04. `can_afford(price)` IS A BOOLEAN OVER A DURABLE
FACT, AND IT HAS MORE AUTHORED CALLERS THAN ANY PUBLISHED CONDITION.**

```text
can_afford(price)   10 authored calls   kernel.yarn (the whole shop menu)
visit_count(id)      2 authored calls   (4 raw — 2 are spoken prose)
wallet_balance()     0 authored calls
```

- `wallet` is a durable save field (`save_data.rs:331`) whose live authority is
  `BodyWallet` on the `PrimaryPlayer`;
- `can_afford` is registered as a closure over the mirror's per-frame snapshot,
  so the boolean question *"can the player pay 25g"* has TWO authorities — the
  snapshot and the component — which is the exact shape both call sites already
  refuse in writing;
- ⚠ and the catalog CAN express it: `ParamKind::Number` / `AuthoredArg::Number`
  exist and `body.fits 32` already uses them. The "it is a number" exemption is
  a claim about `wallet_balance()`, the value, and it does not reach
  `can_afford(price)`, the predicate.

⇒ **So the exemption stands for `wallet_balance` and `visit_count` and falls for
`can_afford`.** Those two really are values the boolean catalog cannot return;
this one is a question it answers natively. ⛔ **The lesson is the shape of the
error, not the missing condition:** the ruling was written by listing the
mirror's struct fields and asking which were expressible. The forks live in the
FUNCTIONS bound over those fields, and one field can carry both a value verb and
a predicate verb — `wallet_balance` is exempt while `can_afford`, reading the
same `i32`, is not. **Enumerate the authored surface, not the storage.**

▢ **NEXT SLICE, and it is the same carve executed twice already:** publish
`wallet.can_afford(amount)` beside `inventory.holds` (`items/conditions.rs` in
the actor kernel — the sibling question, already importing both
`authored_logic` and the item types, and adding no crate edge; `ambition_items`
does NOT depend on `shared_tangle`, so publishing from there would create one).
Repoint `can_afford` at the catalog as a registered system, the way
`boss_cleared` and `quest_active` now are. ⭐ The mirror's `wallet_balance`
field STAYS, because the numeric verb still needs it — an empty mirror is not
the goal, one authority per question is.

⚠ **`wallet_balance()` has zero authored callers and is NOT being deleted here.**
A verb nothing calls yet is content breadth, not dead code, and its own comment
names the use (*"a merchant node can show it"*). Recorded so the zero is not
rediscovered as a finding.

⇒ **An empty mirror is not the goal; one authority per question is.** A future
field here is a claim that the catalog CANNOT answer the question, and the
burden is on the field — which `YarnStateMirrorData`'s own doc now says at the
type. ⚠ Read that burden as covering every VERB bound over the field, which is
what this correction cost.

⛔ **AND IT SAYS NOTHING ABOUT THE OTHER TWO LAYERS.** Observations and memory
have no durable representation at all outside the tactical-belief slice below;
the save is a snapshot of what is TRUE, with no record of what happened or who
could have seen it. A reader should not take the table above as progress on
those — it is progress on exactly one of three layers, which is the confusion
this page's own three-layer split exists to prevent.

### Observations/events

Structured facts that a character or system could have perceived: saw body X,
heard event Y, received item Z, witnessed gate opening.

### Memory/belief

Actor-specific retained interpretation of observations. This may be incomplete,
stale or wrong without changing world truth.

⭐⭐ **THE TACTICAL SLICE OF THIS IS BUILT AND IT IS AN ENGINE FACT — worth naming
because the open question above reads as if none of it existed.** `WorldMemory`
(`crates/ambition_characters/src/perception.rs:785`) is *"the per-controller
belief that outlives the viewport (invariant I6)"*: keyed by actor id, refreshed
for what is seen, decayed for what has left view, forgotten below a confidence
floor. Its `update` is pure, so it is replay-deterministic and assertable
headless. ⇒ *"Should knowledge ever be an engine fact"* is answered YES for
perception, by shipped code, in `ambition_characters` rather than in content.

⛔⛔ **AND IT CAN NEVER BECOME THE DURABLE ONE — that is structural, not a
backlog item.** `WorldMemory` DECAYS BY CONSTRUCTION and forgets below a
confidence floor. That is exactly right for sight and exactly wrong for a grudge:
*"this NPC knows you stole the thing"* must not fade because the NPC looked away.
⇒ So durable social knowledge is not "the same system, persisted" — extending
`WorldMemory` to carry it would mean removing the decay that makes it correct for
its own job. **They are two mechanisms that share a word**, and the word is why
this looks half-solved whenever anyone checks.
⚠ It is also in none of the fourteen durable save families, so nothing persists
it today by accident either.

## Why this matters

- reactive dialogue without giant quest-stage switches;
- agentic character planning constrained by reality;
- explainable LLM context instead of dumping raw ECS state;
- social/knowledge gating that remains separate from physical capability gates;
- debugging of "why does this character believe that?".

## Deterministic authored orchestration is both a consumer and a producer

[`authored-gameplay-logic-and-orchestration.md`](authored-gameplay-logic-and-orchestration.md)
will read world facts and observations as rule **conditions**, and will set or
clear facts and publish observations as rule **effects** — through explicit
semantic domain operations.

⭐ that makes it a demanding early customer of whatever fact/observation
representation this program picks: a fact that cannot be named in an authored
condition, or whose change cannot be observed, is not usable by a rule.

⛔ the governing rule above is unchanged by this. Authored rules alter
deterministic world state through semantic operations; **LLM character
intelligence never becomes the authoritative rule engine.** Simulation determines
reality; AI determines what characters think, infer, want, say, remember and
attempt.

## Candidate crate / Bevy shape

Do not begin with a universal key-value fact database. Prefer typed domain facts
and a narrow observation/projection seam. A common journal/memory crate should
emerge only if several domains need the same retention/query semantics.

An LLM adapter must sit above deterministic world state, not below it.

## Open design questions — deliberately unresolved

⭐ **THREE OF THESE HAVE SHIPPED ANSWERS FOR THE TACTICAL-BELIEF SLICE, and this
page did not know (checked 2026-09-03).** They are NOT answers for the general
fact/memory program — that is still open, and the layer below is one customer,
not the design. But a reader re-deriving them from scratch would be redoing work
that is already in the tree and already rollback-registered. See
[`bounded-perception-and-attention.md`](bounded-perception-and-attention.md).

- *"How is observation permission determined: proximity, line-of-sight, room,
  explicit communication, something else?"* — for a body perceiving other
  BODIES it is **viewport containment**, and deliberately not line-of-sight:
  `peer_is_visible_to_body` is
  `perception.knows_bodies_anywhere() || viewport.contains(peer.pos)`. No
  raycast, no room test. The omniscience escape is a policy, not a fallback.
- *"What parts, if any, participate in deterministic rollback?"* — the actor's
  remembered-actor set does. `WorldMemory` is rollback state with a
  `from_snapshot` road in `snapshot_impls.rs`, which is why the attention
  budget's ordering carries an id tiebreak: two peers at equal distance must be
  kept in the same order on every host or the snapshot diverges.
- *"How long should memories persist, and what is saved?"* — partially, and only
  the second half: what is CARRIED each tick is bounded at
  `TACTICAL_ATTENTION` (16), hostiles first and nearer first, with the remainder
  kept as counts and one distance rather than dropped silently. ⚠ That bounds
  the per-tick kept set, NOT retention over time, which is still open.

⚠ The remaining questions below are untouched by that work.


- Typed facts/components versus an extensible fact registry?
- Which events deserve durable history and which are ephemeral messages?
- How is observation permission determined: proximity, line-of-sight, room,
  explicit communication, something else?
- How long should memories persist, and what is saved?
- Should beliefs support contradiction/uncertainty explicitly?
- What facts are private to a participant in multiplayer?
- How are summaries generated for LLM context without losing critical detail?
- What parts, if any, participate in deterministic rollback?
