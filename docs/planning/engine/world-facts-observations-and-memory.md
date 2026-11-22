# World facts, observations and memory — Engine 1.0 program

**State:** OPEN. World truth and actor belief have different authorities.
The durable fact layer and tactical memory exist. The 2026-11-22 source
audit at `00bf92c22142` selects bounded corrections OBS1–OBS3 through the
existing perception and controller owners. General durable social knowledge
remains customer-gated. This audit ran no gameplay or performance witness.

## Goal

Give systemic characters and agent tooling structured access to **what is true,
what happened, and what a particular actor could know**, without making an LLM
or dialogue generator authoritative over the simulation.

> The simulation determines what is true. AI decides what characters think,
> want, say and try to do about it.

## Three layers

### Authoritative world facts

Examples: door open, machine powered, item custody, actor alive/location,
encounter outcome, persistent world mutation.

**Current shape.** The durable fact layer is `AmbitionGameSaveData`: typed fact
families (flags, switches, items, wallet, occurrences, custody, minted items,
encounters, bosses, quests, dialogue visits, checkpoint, inventory-saved).
It also has bounded named world values for procedural mechanics. Its fields
are `pub(crate)` behind readers and named setters, so
no other crate writes a durable fact by assignment. Paired setters keep one
fact whole: `set_inventory` also sets `inventory_saved`, and
`set_durable_horizon` writes occurrences, custody and minted descriptions.
`scripts/measure/durable_fact_writers.py` lists who writes each family.

The native [world-value owner](../../../crates/ambition_platformer2d_runtime/src/world_values.rs)
already supplies named values, revisions, admitted writes and change delivery
at its stated read cut. The Bridge Warden is a real consumer. Keep that
supported mechanism; do not migrate typed custody, actor health or knowledge
into it merely because a named value can encode them. The
[causal-commit owner](causal-world-commit.md) governs independent-room writes.

Authored rules read facts through the condition catalog. Each domain publishes
its own conditions beside the systems that write the fact:

| Condition | Reads | Published by |
| --- | --- | --- |
| `world.flag_set`, `world.switch_on` | the save | `actor_monolith/world_facts.rs` |
| `boss.cleared` | the save (keyed by authored encounter id) | `ambition_boss_encounter` |
| `encounter.cleared` | the save | `ambition_encounter_features` |
| `quest.active` | the save | `game/ambition_content` (the game publishes it) |
| `inventory.holds` | live ECS | `ambition_platformer2d_items/conditions.rs` |
| `custody.is_held` | live ECS | `ambition_held_items` |
| `wallet.can_afford` | live `BodyWallet` | `ambition_platformer2d_items/wallet_conditions.rs` |
| `body.can`, `body.fits` | live ECS | `ambition_abilities/body_conditions.rs` |

The save is the durable mirror of the rule-readable surface, not the surface
itself. A rule asking "does the player hold X" wants the live hand. The facts
read from the save are the ones with no live form between sessions (a cleared
boss, a set flag). `scripts/measure/authored_route_gates.py` counts which families
conditions read.

`wallet.can_afford` reads the price through `ambition_items::shop::authored_price`,
the same reading `<<buy_item>>` uses, so a price the guard refuses is a price the
transaction refuses. A no-wallet composition answers `Unanswerable`, not `false`.

**Open correction at `f41218979625`:** sharing the price parser does not bind
the same purse. The condition and `wallet_balance()` still read the primary
body; a live merchant command charges its conversation's input-owner body.
[O6a](authored-gameplay-logic-and-orchestration.md#o6a--bind-the-merchants-question-and-action-to-the-same-subject)
uses the existing subject evaluator and retains source attribution. World
questions remain global; a missing required subject must not select a
different body. A shop offer also needs the
[item owner's exchange decision](item-custody-and-accounting.md#a7-exchange--one-rule-for-an-offer-and-its-accepted-exchange),
which includes stack capacity and exact payout.

The native third verdict must survive its consumer.
[O6b](authored-gameplay-logic-and-orchestration.md#o6b--preserve-an-unanswered-question-through-the-dialogue-decision)
corrects the Yarn adapter's Boolean collapse. `Unanswerable` cannot authorize
a negative gameplay arm through `not` or a catch-all `else`. This is missing
domain/subject evidence, not character belief or a new world-fact family.

`dialog_visits`, `checkpoint`, `minted_items` and `inventory_saved` publish no
condition. Do not publish one per field: `inventory_saved` and `minted_items`
are restore mechanics no rule should ask about.

**The Yarn mirror.** `ambition_dialog::YarnStateMirrorData` holds only
`visit_counts` (dialogue's own bookkeeping) and content `extras`. Every other
Yarn function asks the catalog live or reads the live component
(`wallet_balance()` reads `BodyWallet`). A new mirror field is a claim that the
catalog cannot answer the question, and the burden is on the field. Enumerate
the authored verbs bound over a field, not only the field.

**Switch writers.** The save's switch family has two writing roads, disjoint
by action kind: `drain_switch_activations` (`ambition_encounter/src/switches.rs`)
and `apply_wave_encounter_effects` (`ambition_encounter_features/src/systems.rs`,
which greens every switch of a completed encounter). The activation queue is a
cross-tick channel: the drain is ordered before `EncounterSimulation` by its
consumers, and `apply_switch_effects` pushes in `GameplayEffects`, so an
activation resolves on the next tick
(`a_switch_activation_is_drained_on_the_tick_after_it_was_pushed`). Ordering the
drain after the push would be a schedule cycle.

**Open: which source a condition reads.** When a fact has both a live form and a
durable row, the choice is implicit in each evaluator (`boss.cleared` argues
its choice in its docstring). A `reads: FactSource` field on
`ConditionDescriptor` would force the choice, but nothing would read it yet. Add
it with its first consumer: the condition inspector showing the source, a guard
that durable gates do not use live-reading conditions, or agent tooling that
filters by source.

### Observations/events

Structured facts that a character or system could have perceived: saw body X,
heard event Y, received item Z, witnessed gate opening. No general durable
witness representation exists. Saved provocation and per-seat dialogue
visits are narrower policies/history, described below; neither records
which character witnessed an event.

Time of evaluation is part of this distinction. The
[dormant NPC refinement](radiant-npc-planning.md#life2--admit-placement-and-historical-inputs-before-their-effects)
finds that past-hour `Present` currently reads who is in the room now. Name
that reconstruction-time approximation, supply supported historical facts,
or decline the evaluation. Unknown history is not an answered absence. The
planner's predicted postconditions are likewise not settled world truth.
This bounded customer does not require a general observation journal.

### Memory/belief

Actor-specific retained interpretation of observations. It may be incomplete,
stale or wrong without changing world truth.

**Tactical belief is built.** `WorldMemory` (`ambition_characters::perception`)
is the per-controller belief that outlives the viewport: keyed by actor id,
refreshed for what is seen, decayed for what left view, forgotten below a
confidence floor. Its `update` is pure. It is rollback state, and it records the
live room it was formed in, so a body that changes live room forgets it. It is
in no durable save family.

Durable social knowledge ("this NPC knows you stole the thing") is a different
mechanism. `WorldMemory` decays by construction, which is right for sight and
wrong for a grudge. Do not extend `WorldMemory` to carry durable knowledge.

## Evidence, estimates and policy have different meanings

The reusable boundary is the evidence an observer actually acquired. A
shared live snapshot makes acquisition cheap; it does not authorize every
reader to know every row. An exact observation needs a subject in a live
room, an admitted source/read cut and the facts that source establishes.
The decision may then use a prediction or an interpretation of those facts.

| Value | Example | What it does not establish |
| --- | --- | --- |
| Authoritative outcome | Native reception reports a protected strike or actual damage | Who saw it or recognized its source |
| Admitted sight | This observer attended to this body's position and living/dead state at this cut | Its current position after the cut, or another observer's knowledge |
| Estimate | Pursuit point extrapolated from the last admitted position and velocity | A new sighting that may be reported as firsthand |
| Report | A crew member reports a named subject from a particular sighting | That the listener saw it, or that the caller's foe is automatically the listener's foe |
| Response policy | Join a crew member's fight; retaliate against a striker | A witnessed injury or a durable reputation fact |
| Retained social fact | A selected character remembers a recognized incident across a specified reset boundary | A permanent copy of every tactical memory row |

These are distinctions in existing domain values, not a request for one
generic evidence schema. `PerceptionPeer`, `WorldMemory`, crew calls,
native hit reception and saved provocation already have their own owners.
Let each carry the minimum fact its actual reader needs. Borrow shared peer
rows for acquisition; retain only the bounded evidence needed after that
cut. Do not record an all-event transcript to reconstruct a small belief.

## OBS2 — preserve evidence meaning through memory and communication

**Prerequisite and neighbor:** [OBS1](bounded-perception-and-attention.md#obs1--make-attention-the-native-evidence-admission-boundary)
corrects which exact rows are admitted. This packet corrects what an
admitted row or report means. The pure values stay in
[`ambition_characters::perception`](../../../crates/ambition_characters/src/perception.rs);
native collection, combat settlement and crew delivery stay in
[`body::perception`](../../../crates/ambition_platformer2d_actor_monolith/src/body/perception.rs).

### Concrete source witnesses

1. **A seen death is discarded.** `PerceptionPeer` and `PerceivedActor`
   carry `alive`; `SeenActor` and `RememberedActor` do not. Cheap acquisition
   tests hostility but not liveness, whereas `WorldView::hostiles` tests both.
   A visible dead hostile can refresh memory to confidence 1 and make the
   native snapshot say `target_alive = true`. Filtering dead rows out before
   memory sees them would still leave a formerly living target remembered.
2. **Position confidence can suppress an attributed hit.** `struck_by`
   sets hostility at confidence 0.5, then `hear` compares whole records.
   A previously seen non-hostile record above that confidence rejects the
   hit's hostility too. A different-faction actor can physically hit without
   an existing faction hostility; this is a supported source sequence.
3. **Old belief can masquerade as a fresh report.** `hostiles_in_view`
   means confidence >= 1. A sharing body keeps its policy when possessed.
   The native loop still gathers its crew calls, then skips its autonomous
   memory fold because it has a driver. Its pre-possession sight can remain
   at confidence 1 and be called out from its new position without another
   observation. The existing memory-room check correctly prevents carrying
   this call into a different room.
4. **A report can name its listener as the foe.** A same-faction caller
   with `Grudge::Body(listener)` reports that listener. `hear_crew` excludes
   the caller, not the subject, and copies `hostile_to_self` from the
   caller. The listener's direct sight excludes itself, so that fold cannot
   correct the resulting self-target belief. This does not prove self damage;
   damage has its own admission.

These are source-derived failure sequences, not newly executed reproductions.
The earlier [hit-memory settlement receipt](../../../dev/benchmark-candidates/hit-memory-must-settle-before-snapshot-2026-11-01.md)
remains valid: attributed hit memory settles in `CombatSet::Settle` before
snapshot. Do not move it back to next tick or add a second hit-message history.

### Small native representation and merge rules

Keep a direct sample distinguishable from the current pursuit estimate.
Today `update_from_seen` dead-reckons `RememberedActor.pos` in place even
though the field's contract calls it the last directly perceived position.
Either retain the sample and derive its estimate, or name both explicitly
if the selected consumer needs both. A report must not stamp an extrapolated
position as a fresh direct sight. Preserve current motion/decay behavior
unless the change is deliberately accepted with its replay baseline.

Use the existing deterministic tick/read cut for ordering and provenance,
and the existing elapsed-time owner for confidence decay/prediction.
They answer different questions. Do not use confidence, wall time or a new
global serial as a substitute for the observation cut.
The cut must distinguish simulation steps even when gameplay time is held.
Current extrapolation spends the observer's room-scaled `dt`; deriving
from global elapsed time minus `last_seen` would change pursuit during
room holds or different rates. Retain an accumulated prediction age with
that same meaning, or reuse a clock that actually supplies it.

| Input | Native reconciliation rule |
| --- | --- |
| Admitted direct sight | Refresh that subject's observed pose and living/dead evidence at this cut. Only an eligible live subject becomes a live target. |
| No admitted sight | Age the prior estimate under the existing policy. An out-of-view or unattended subject has not thereby been observed absent or dead. |
| Accepted attributed hit | Preserve the independently established attacker/hostile-act evidence even if an older position estimate is more confident. Do not discard it through a whole-record positional comparison. |
| Crew sighting | Retain subject, caller and original sight cut; apply a stated position freshness/source rule. A newer observed death cannot be undone by an older report of a living target. |
| Crew assistance | Reject self subjects and wrong-room reports; apply the existing assistance policy to a valid report. A caller-relative foe conclusion is not an objective faction fact. |

An observed death disconfirms live pursuit; an offscreen death does not.
Keep a tactical negative observation only as long as needed by that policy,
including rejection of delayed reports. A later admitted live observation
may establish a live target again under the existing occurrence rules.
Do not query authoritative global health to correct every stale belief.

Define source/freshness precedence per field that needs it. A single
confidence comparator or global latest-write-wins rule cannot substitute
for these rules. Equal admissible reports must reduce deterministically;
the old assertion that every caller saw the same fields at the same moment
does not cover frozen memory or caller-relative hostility. Use scoped
subject/caller identity and the native cut as tie-break inputs where needed,
never ECS allocation order.

Retain `shares_sightings` and the pirate crew's useful assistance behavior.
Separate the small report payload from the listener's adoption decision
without requiring an authored diplomacy language. Direct-only reporting
must require a direct sample from the preceding simulation step's observation
cut, not merely the caller's most recent decision and not confidence 1.
A skipped decision does not extend the sample's eligibility. In the first
slice, preserve the cheap
driven-body path: no autonomous refresh means no new AI sighting to report
after the last eligible cut expires. If content later wants driven-body
scouting, declare that narrow observer consumer through OBS3. Do not rebuild
a full tactical view merely to invalidate an old report. Hearsay must not
acquire a new firsthand stamp when retold.

Reuse the listener's existing peer identity for self exclusion. Do not add
another room scan, a new global actor ID, or a durable grudge for every crew
call. Live hostility policy stays with targeting/aggression; tactical
evidence only supplies what the current memory/pursuit consumer needs.

### First proof and removal

- In the native decision path, compare a visible dead foe with a living
  foe that leaves view and dies unobserved. Only the first observation
  disconfirms pursuit. Include a nearer corpse and farther living target.
- At the pure memory owner, start with a recent non-hostile sight, then
  apply an attributed hit outside view. Keep the hit's hostility while
  preserving any better position evidence. Also reject an older living
  report after an admitted death.
- Through native crew collection, possess a sharing caller after it sees
  a foe and move it within the same room. Old memory cannot create new
  firsthand reports. Autonomous loss of sight, hear-only memory and the
  existing room-change guard are controls.
- Deliver a valid crew report to a third ally and to the report's subject.
  The ally retains the authored assistance; the subject cannot target itself.
  Permuting equivalent report delivery cannot change the selected belief.

Delete confidence-as-provenance, whole-record `hear` replacement as the
merge authority for unrelated facts, and unlabeled copying of caller-relative
hostility. Keep borrowed IDs, steady-state no-new-string allocation, room
clearing, deterministic ordering and the existing Settle timing.
Changing retained fields or admission changes canonical rollback state;
update its codec/checksum and replay compatibility explicitly. The
temporary admitted set and report preparation remain derived.

## Existing social state and its horizons

The code already supports useful narrow social policies. Preserve them.

| Current mechanism | Actual meaning | Retention boundary |
| --- | --- | --- |
| `Grudge::Body` and prepared provoked mind | Live retaliation against an actual source | Live targeting/aggression; may settle when an endpoint is down/gone |
| `npc_<id>_hostile` | Saved NPC hostility that reconstruction can restore as `Grudge::Faction(Player)` | Save-backed flag, included in checkpoint fact restore and room-write retraction |
| Crew sighting and rally | Tactical assistance; rally infers an active fight from a grudge or swing | Live/decaying state, not a witness history |
| `PersistedDialogVisit` | A seat has visited a dialogue node | Saved seat history; deliberately survives death/checkpoint fact restore |
| `ConversationsInRooms` | Room-local continuity of live conversation authority | Rewind horizon; not a durable transcript |

Sources: [aggression](../../../crates/ambition_platformer2d_actor_monolith/src/body/aggression.rs)
and its [Player-only durability guard](../../../crates/ambition_platformer2d_actor_monolith/src/body/aggression/tests.rs),
[rally](../../../crates/ambition_platformer2d_actor_monolith/src/body/rally.rs),
[checkpoint facts](../../../crates/ambition_platformer2d_session/src/fact_horizon.rs),
[saved dialogue visits](../../../crates/ambition_persistence/src/save_data.rs),
and [conversation continuity](../../../crates/ambition_conversation/src/rooms.rs).
The saved hostility restriction is intentional and tested. It is not a newly
found failure to remember the original attacker.

### A later incident customer must choose what survives

No native theft-witness, recognized-person reputation or general incident
store was found in the inspected owners. A later authored response such as
“I saw that body strike this person” must first specify:

- the native incident/result it uses, and who could observe it at that cut;
- whether the observer recognized a lasting social subject or only a live
  body occurrence; a character kind or dialogue name is not recognition;
- whether the reader needs firsthand knowledge, a report or either;
- what survives room retirement, body death, checkpoint restore, independent
  room replay and process restart.

Start from [native reception](../../../crates/ambition_damage/src/lib.rs)
and its [reporter](../../../crates/ambition_damage/src/reception.rs) when the
claim concerns a hit. `BodyHitResolved` distinguishes protected, blocked
and damaged results and retains source/target context before retirement.
`ActorStimulus::DamagedBy` is already a retaliation interpretation:
protected strikes can produce it without HP loss. A grudge or a swing can
support rally policy without proving that a blow landed.

Those native outcomes still do not say which bystander saw or recognized
them. Admit only the selected observer's evidence, then promote the specific
recognized incident to an appropriate retained owner if later dialogue
needs it. Existing declared durable module records are one possible owner,
not proof that all memory should become script state. Reuse CR1 for final
reception meaning and causal world commit for cross-room commitment.
Do not persist `WorldMemory`, every contact, or a model-generated summary as
simulation truth. This is a future feature gate, not OBS2's completion bar.

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

That makes it a demanding early customer of whatever fact/observation
representation this program picks: a fact that cannot be named in an authored
condition, or whose change cannot be observed, is not usable by a rule.

The governing rule above is unchanged by this. Authored rules alter
deterministic world state through semantic operations; **LLM character
intelligence never becomes the authoritative rule engine.** Simulation determines
reality; AI determines what characters think, infer, want, say, remember and
attempt.

## Candidate crate / Bevy shape

Do not begin with a universal key-value fact database. Prefer typed domain facts
and a narrow observation/projection seam. A common journal/memory crate should
emerge only if several domains need the same retention/query semantics.

An LLM adapter must sit above deterministic world state, not below it.

## Open design questions

Answered for the tactical-belief slice (see
[`bounded-perception-and-attention.md`](bounded-perception-and-attention.md)),
open for the general program:

- Observation permission for a body perceiving other bodies is viewport
  containment within its own live room, not line-of-sight
  (`peer_is_visible_to_body`). The omniscience escape is a policy.
- The remembered-actor set is rollback state (`WorldMemory` has a
  `from_snapshot` road), so attention ordering carries an id tiebreak.
- The rich view's kept set is bounded at `TACTICAL_ATTENTION` (16).
  [OBS1](bounded-perception-and-attention.md#obs1--make-attention-the-native-evidence-admission-boundary)
  closes the native memory/admission bypass; that end-to-end property is not
  yet established. Retention over time is separate open work.

Open:

- Typed facts/components versus an extensible fact registry?
- Which events deserve durable history and which are ephemeral messages?
- How is observation permission determined for the general program?
- How long should memories persist, and what is saved?
- Which additional contradictions need representation beyond OBS2's observed
  liveness, source/freshness and attributed-hit rules?
- What facts are private to a participant in multiplayer?
- How are summaries generated for LLM context without losing critical detail?

## Knowledge reduction at the observation boundary

A consumer of a published fact needs less knowledge of its producer. An observation view may expose a
stable, scoped consequence without handing its reader the producer's mutable
resources, broad context or callback. Keep fact identity, observer memory and
live simulation authority distinct.

An agent response based on an observation needs subject/session and revision or
expiry checks before normal action admission. Replaying accepted intentions must
not rerun external observation/model queries. Multi-instance scope comes from the live room
(`InRoomInstance`), not from a universal world-facts bus.

## Procedural callers receive the appropriate knowledge contract

A portable module is not automatically omniscient. An actor controller consumes
its granted observation/knowledge projection; an explicitly authorized world-rule
module may consume wider authoritative facts. Diagnostic inspection is a separate
read-only role. [Domain contracts](extension-domain-contracts.md) records that
scope with the port, including instance, coordinate frame, phase and missing data.

This is a direction, not a claim that all current controller ports implement
it. `ActorThink.target` currently supplies the target's live position.
[OBS3](agentic-character-runtime.md#obs3--resolve-observation-demand-across-controller-providers)
adds an explicit limited-knowledge choice and resolves native/module demand
before one perception projection is prepared. The SDK delivery label
`Observation` does not establish that an actor perceived the value:
`WorldValuesPort`, body queries and whereabouts already expose intentional
authoritative reads.

World truth, an observation and a belief remain different values. Do not create
one generic WorldSnapshot that silently gives limited-knowledge brains every
fact or makes gameplay rules read stale beliefs. Caches pin their source revision
and read cut; restoring or clearing a derived cache cannot change the outcome.
