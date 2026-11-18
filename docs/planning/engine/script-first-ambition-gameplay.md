# Script-first Ambition gameplay: SDK capability program

**Direction:** Maintainer, 2026-11-17. **Scope:** new gameplay authoring, not migration of existing games. **Execution selection:** [`queue.md`](../queue.md), scripting priority 0b.

**Current status:** The first slice below is complete. **The full script-first SDK program is NOT complete.** A request to "complete this plan" means completing the follow-on authoring and engine work below. Do not close priority 0b on the strength of the 2026-11-17 receipt, a port list, or another single scripted attack.

## Product goal

Rhai should be an effective way to **invent, implement, and iterate on new gameplay in Ambition**, using a prebuilt engine and authored content. Mary-O and Sanic may reveal missing general engine capabilities; neither is a porting target. The same domain contracts must be usable by portable Rust modules built as WASM or linked natively, so successful prototypes may be promoted selectively without changing their content bindings or state semantics.

The completed [first-class procedural authoring acceptance](first-class-procedural-game-authoring.md) proves the three-backend promotion, parity, and desktop packaging workflow for Pulse Forge. It does **not** establish that the SDK is expressive enough to author an arbitrary new Ambition mechanic. Treat A1-A10 as a retained capability and closed receipt, not work to repeat.

Measure progress by the gameplay that authors can express and the engine changes that subsequent ideas no longer require. Playable examples can show this progress. Port counts, script size, and native migrations do not establish capability completeness.

## Gameplay expressiveness requirements

Maintainer direction, 2026-11-18: use GDScript gameplay expressiveness as a
reference. Keep Ambition's domain model, authoritative simulation, rollback,
movement rules, and native performance. API count does not measure success.

1. **Address meaningful objects.** Scripts can identify important bodies,
   props, items, and room objects, inspect relevant state, and use their domain
   operations. References have explicit scope, validity, and retirement rules.
2. **React to important events.** Damage, death, pickup, interaction, entry,
   exit, animation completion, and state changes can start authored behavior.
   Native owners publish confirmed facts. Delivery has defined ordering,
   bounds, lifetime, and replay behavior.
3. **Create and compose objects.** Authors can create enemies, interactive
   props, and systems of several objects from reusable building blocks.
   Creation reports actual outcomes and identities. A predefined weapon
   request alone does not establish this capability.
4. **Define reusable script abstractions.** Authors can define, parameterize,
   and combine gameplay components, object behaviors, and systems in the
   scripting environment. A new variation must not require a Rust adapter.
   Shared behavior and persistent instance state must remain distinct.

The fourth requirement guides the other three. A set of unrelated operations
is insufficient if authors cannot build and reuse higher-level behavior with
it. Check shared functions or libraries, behavior parameters, independent
instances, object references, event routing, and state ownership together.
Use a prebuilt host to change an abstraction and a separate use of it, with
clear load errors and replay-safe state. Record missing mechanisms in the
[census](script-first-capability-census.md); do not infer support from the
scripting language alone.

This direction does not require raw ECS access, a Node tree, inheritance, or
a universal prefab format. The separate
[world composition plan](reusable-authored-world-composition.md) records open
structural choices. Those choices must not block reusable script behavior.

## Non-goals and performance rule

- Do **not** migrate working Mary-O, Sanic, or Ambition gameplay to Rhai to demonstrate API coverage. A migration needs its own concrete product/iteration benefit, a separately selected customer, and acceptable measured cost.
- Do **not** make Rhai the default implementation of a performance-critical mechanic merely because the SDK supports it. Keep native code where it is a good fit. Measure actual workload costs; promote specific scripts to WASM/linked Rust when warranted.
- Do not expose arbitrary Bevy ECS mutation, directly write private health/geometry/custody state, or duplicate the engine's physics, inventory, combat, rollback, or world authority.
- Do not invent game-named SDK operations such as `break_mary_o_brick` or `swallow_take_coconut`. Ports describe reusable semantics, even when prompted by a particular feature.
- Do not build a generic visual scripting system, compiler from Rhai to Rust, or a broad compatibility framework before a real gameplay author needs it.

## Requirements discovery: cross-game capability matrix

Use a **read-only audit** of current Mary-O, Sanic, and Ambition systems to find required *capability families* without changing those systems. For each representative mechanic record:

| Question | Evidence to capture |
| --- | --- |
| What authored data already expresses it? | Concrete catalog/schema, LDtk entity, command or prepared value |
| What starts the logic? | Event, contact, input, timer, room/lifecycle transition, or public invocation phase |
| What does the algorithm need to know? | Bounded observation, scope, identity, ordering, coordinate frame, lifetime |
| What does it need to change? | Reusable domain request, admission/refusal and eventual outcome |
| What state survives? | Body, room, occurrence, session or durable record and rollback semantics |
| What is available now? | Rhai alias; typed portable port with no Rhai descriptor; native/data-only owner; genuinely absent service |
| What is the generalizable gap? | A capability usable independently of the named mechanic, or a reason no new API is needed |

Keep the inventory short, evidence-linked, and prioritized; do not exhaustively enumerate every system or demand a port per operation.

**Candidate families to audit (not an implementation checklist):** scoped spatial/world observations; body and geometry contact facts/results; actor/control intent; interactive authored world-object state and collision changes; object and item creation/lifetime; equipment/form/grant transitions; accepted travel and room transitions; character/room/session state and gameplay events; progression and reward transactions; audio, camera, cutscene and HUD presentation requests; game-defined content/LDtk schema and bindings. Distinguish already data-authorable cases from missing scripting capabilities.

## Capability work

The maintainer clarified the scope on 2026-11-17: a current game consumer is
not required. Implement capabilities that have clear semantics and plausible
expressive value. A mechanic can supply an example. It does not limit the
capability program to the smallest interface needed for that example.

1. Use the census to select a coherent capability family. Describe what an
   author can express with it and which native domain owns its authority.
2. Check existing data, composition, and typed ports. Complete the shared
   contract instead of adding a special adapter for each game idea.
3. Define scope, identity, read and consume phases, admission and results,
   state lifetime, rollback, bounds, cost, and optional availability.
4. Supply the same contract to Rhai and portable Rust. Keep physics, combat,
   inventory, travel, and world changes in their domain services.
5. Validate composition through a supported host. A focused fixture is valid
   when no game uses the capability yet. Use real gameplay when a suitable
   feature exists. Do not migrate working native mechanics to create demand.
6. Document the authoring interface and measured cost. Check that an author
   can combine it with other capabilities using a prebuilt host.

A new native service is in scope when a portable adapter cannot supply the
semantics. Implement that service once and expose its supported operations.
Do not offer private component setters as a substitute.

## Acceptance and open work

The first slice below is a closed receipt. The capability program remains
open. Success means that an author can implement substantial new gameplay
without first adding game-specific Rust or a special engine adapter.

Use these capability groups to assess coverage:

- Scoped, bounded world and target observations with stable identities.
- Controller-neutral contacts and accepted combat outcomes.
- Authored world-object state and consume/reward transactions.
- Creation, custody, equipment, form, travel, and lifetime operations.
- Body, room, and session records plus accepted gameplay events.
- Composable combat, motion intent, status, camera, audio, and HUD services.
- Game content schemas, bindings, diagnostics, and a complete edit loop.

For each group, distinguish existing data, described native services,
services that need an adapter, and missing native authority. An available
request does not prove that its trigger or result is available. Preserve
explicit refusals and unsupported cases in the census.

There is no required port count or demonstration count. A feature slice
proves its own composition. It does not prove that these groups are complete.

## First-slice receipt (2026-11-17)

- The new flagship feature is the Boss of the Game's encore stage jets
  (`31443e23b`). The script owns the warning, alternating pairs, and pulse
  limit. Existing native boss moves remain in place. Lightning is a second
  new inventory skill in the same change.
- The missing Rhai surfaces were descriptions of the existing boss-special
  trigger and authored effect request (`ec483e1ac`). Native and portable Rust
  use the same contracts. No new game-specific engine verb was added.
- The [coverage census](script-first-capability-census.md) records the
  read-only cross-game audit and the next useful capability gap.
- The assembled `the_boss_stage_has_scripted_pyro` and
  `lightning_is_authored_in_rhai` witnesses cover real collision/damage,
  rollback state, cancellation/refusal, and live source edits. The
  [recipe](../../recipes/scripted-stage-attacks.md) describes the prebuilt
  host loop. The [measurement](../../../dev/benchmark-candidates/scripted-stage-entry-cost.md)
  records the scenario and cost: keep this bounded stage sequence in Rhai.
- The longer-term usability goal remains open. Do not infer a migration
  mandate or complete engine API coverage from this first slice.

## Active follow-on: finish the gameplay authoring capability (2026-11-17)

**Maintainer clarification:** This is a gameplay and engine capability task, not a documentation-only audit or a Rhai-alias-count task. **Native engine work, including structural refactoring, is in scope and is preferred when it creates a cleaner general capability.** The engine is pre-release. Do not preserve an awkward boundary or create game-specific glue to avoid changing it. Keep Bevy, physics, collision arbitration, inventory, travel and rollback authority in their domain owners. Rhai, WASM and linked Rust must use the same semantic contracts where procedural access is appropriate.

The finite first slice and Pulse Forge receipts are historical milestones. They are **not** the exit criteria for this active follow-on. The next agent should implement the missing capability families, exercise them through loaded authored fixtures or new Ambition gameplay, and record limitations from source and runtime evidence. A shipped consumer is not required. Do not stop after identifying a blocker, adding a descriptor, or writing a proposal for future engine work when the blocker is within this task's engineering scope.

### Required authoring experiences

These are the user-facing capabilities the completion review must cover. A capability can be supplied by existing authored data instead of a Rhai call **if an actual authored example demonstrates that path**; otherwise it needs a general domain-owned procedural seam. Never claim a capability from a plausible API name alone.

| Authoring need | Minimum observable behavior | What does not count |
| --- | --- | --- |
| React to gameplay, not just poll every tick | A new scripted rule reacts to a domain-confirmed event or contact, with scoped subject and stable provenance; it works for the relevant body/controller kinds | Polling every entity every frame to guess an event; presentation effects as event evidence |
| Know whether a requested action happened | The script can correlate a submission with a typed accepted/refused/applied outcome at the domain's defined read cut, including eventual completion when needed | Logging a refusal only in Rust; treating submission or queueing as success |
| Interact with authored world objects | New content can define an interactive or breakable object, detect legitimate interaction, consume/alter it through the world owner, update collision/art as appropriate, and grant a reward once | Game-named engine callbacks, raw collider setters, a second inventory or room-state store |
| Select relevant targets and respond to combat | Bounded, room-scoped observations with stable identities; accepted hit/contact results, not guesses from overlaps; no cross-controller special cases | Unbounded ECS scans or spatial coordinates without identities where a later action needs a target |
| Change gameplay inventory, entities and progression | Reusable authoring can request spawn/despawn or item/equipment/progression changes through existing domain owners and observe their outcomes, with correct lifetime | A script mutating health, ECS entities, save flags and inventory as an uncoordinated sequence |
| Follow travel and room lifecycles | A script can react to confirmed arrival/exit, failed travel or owner/room retirement when a selected mechanic needs them; gate-specific mechanics stay with the gate owner | Treating a requested portal as actual arrival or retaining references to an unloaded room |
| Iterate without rebuilding the host | An author can add or modify the new game's script and its declared content bindings in a running/prebuilt Ambition host and see valid changes or actionable rejection | A test-only script that needs a new hand-edited Rust game adapter for every use |

The first four rows are **immediate high-priority gaps** in the census; the next two must receive an end-to-end coverage decision and working proof rather than being silently declared out of scope. Presentation is a supporting capability: use current sound/effect/cutscene/data paths, and add a typed camera/HUD contract only for a real new gameplay customer. Do not turn this into an instruction to expose every Bevy or rendering API.

### Engine work is allowed and sometimes required

Use the **domain owner** as the place to improve mechanics, then give it script and portable-Rust bindings as needed. Examples of legitimate native work:

- Add a small typed gameplay event/result service, or improve existing domain event publication, so occurrences and outcomes are replay-safe and available at declared phases. A universal unbounded event bus is **not** a requirement. Prefer a few coherent typed streams over duplicative per-game bridges.
- Refactor contact, authored-object consumption and room collision overlays to share identity, first-winner admission and rollback semantics. If a new common domain service is simpler than another adapter around Mary-O bricks or Sanic monitors, build that service. Keep their existing gameplay working; do not migrate those implementations for a port-count target.
- Improve target observation, actor creation, item grant or travel services when their current shapes prevent a reusable, safe script contract. Use authored schemas/selectors for named content, not embedded game-specific Rust branches.
- Remove provisional duplicate paths or incompatible APIs when a unified pre-release design is better. Do not add compatibility scaffolding for consumers that do not exist.

An agent must not report "requires engine changes" as the final answer for a necessary capability. Identify the owner, make the architectural change, wire the common port, and prove the feature. Escalate only an actual product decision (for example, whether gate authorization is a shipped rule), not ordinary engineering ownership decisions. Work in the assigned checkout; do not create or switch worktrees.

### Build the missing seams in a useful order

These are dependency stages, **not** permission to declare the whole program complete after any one stage.

1. **Trace the full action path.** Recheck the source head for `script_ports()`, phase installs, the native contact/hit ports, `story_script.rs`, authored room commands, item collection, actor spawn and gate travel. Mark each need as `Rhai`, `portable-only`, `native/data-only`, or `absent`. Confirm trigger *and* request availability in the same authorized phase; a descriptor in the registry is not enough. Update the [census](script-first-capability-census.md) as work lands, not instead of doing it.
2. **Unify outcomes and important event triggers.** Supply reusable scoped facts and request correlation. Keep `submitted`, `admitted`, `applied`, `rejected`, `canceled` and terminal outcomes distinct where the owner distinguishes them; do not fabricate a same-tick result if arbitration happens later. Specify stable occurrence keys, identity, scope, ordering, read cut, bounded delivery, state/retirement, rollback replay, and conflict resolution. A shielded shot must not appear as a successful damaging hit. Ensure the feature is usable from Rhai and the portable typed contract, rather than only a Bevy event.
3. **Complete world-object interaction.** Select a real, desirable new Ambition interaction and author it in data plus Rhai. It must use authoritative contact/activation, an atomic first-successful-consumer or equivalent domain rule, appropriate art/collision changes, and an actual pickup/grant/other result. Try competing interactions, rejection, room reload, save/rollback and replay where applicable. Prefer improving world-item/room-object services to introducing an unrelated script-owned collision model.
4. **Complete identified targeting and combat reaction.** Use a real new combat/interaction request to motivate bounded queries and an accepted contact/damage outcome. Preserve provenance after reflection, shielding and delay. Expose controller-neutral contact/hit trigger behavior where appropriate. Native physics/combat continues to decide hits; scripts decide new policy and timing. A small independent reuse witness is sufficient if a second complete game feature would be artificial.
5. **Close creation, progression and travel seams.** For each row in the active [gap register](script-first-capability-census.md), either demonstrate a real data-authoring path or implement the reusable domain contract needed by a plausible new Ambition mechanic. In particular, verify that scripts can observe success/failure for item grants, actor creation and relevant travel, even if the principal action stays entirely native. The illicit traveler's one-use gate key is a **candidate** cross-domain acceptance feature, not a fixed lore or gate-authorization requirement: only implement it if the gameplay rule is selected. Do not invent gate restriction policy as part of SDK work.
6. **Prove a practical authoring loop and review the shape of the engine.** Run the assembled Ambition gameplay rather than relying only on port unit tests. Edit the new scripts and content in a prebuilt host; validate load/reload, error reporting, supported platform builds and relevant runtime cost. Demonstrate at least one independent reuse of newly added semantic interfaces. Remove feature-specific glue and record remaining performance and platform limitations accurately.

### Failure cases the witnesses must distinguish

Choose a small set of **high-value** focused tests/recorded replays; do not manufacture a broad testing program. Reuse existing domain and game tests where possible. Cover these cases when the corresponding domain is part of the new gameplay:

- Two simultaneous consumers compete for the same world object: exactly one domain grant succeeds; the object, collision and rendering agree.
- A script requests an invalid command, nonexistent entity or inactive room: it receives the correct refusal or terminal outcome, not a silent log-only failure; its own state does not advance as if the action applied.
- A projectile is blocked, reflected or misses: contact and actual damage outcomes differ correctly; source credit follows the resolved hit.
- A spawn request is accepted for processing but construction fails or its owner expires: the script does not receive a phantom body ID and cleans its pending state.
- A room unload, interrupted cutscene or canceled crossing removes or finalizes pending operations; no orphan callback changes later gameplay.
- Rollback/replay does not duplicate an item, payout, actor, cue or result consumption; phase and identity ordering are deterministic.
- A script/content edit can be applied or rejected without rebuilding the game executable or corrupting the last valid generation.

### Completion gate and handoff

**Do not mark script-first gameplay complete merely because** Pulse Forge parity passed, the encore/Lightning examples run, a census was written, all easy port descriptions were added, or one cross-domain feature was demonstrated. These are valid receipts but not general gameplay authoring readiness.

The active program can be marked complete only when:

1. All four gameplay expressiveness requirements and the required authoring experiences above have **real implementation evidence**, or a justified, demonstrated data/native-authoring equivalent. A high-priority family cannot remain a known blocker with no working path.
2. Loaded authored fixtures or new Ambition gameplay exercise **multiple independent domain families**, including an interaction with an authoritative outcome. The contracts are reusable outside that mechanic. Check a materially different use of the abstraction; a change of attack art is insufficient. No shipped consumer is required.
3. Engine changes, if needed for those semantics, are implemented cleanly in owning domains and used by Rhai and portable Rust; no gameplay-specific SDK hacks, raw ECS exposure, parallel simulators or unbounded scans are left as the normal authoring path.
4. A prebuilt-host editing cycle, relevant replay/state/cancellation evidence, and measured cost are documented with commands and observed results. Verification claims distinguish executed checks from static reasoning; do not require a full four-hour suite for a local scripting change.
5. The census marks each high-priority gap as `implemented`, `data-authorable with witness`, or `still blocked`, with links to code and proof. **Any remaining high-priority `still blocked` entry means the program remains open.** A deliberate reduction of this scope requires an explicit maintainer decision, not unilateral agent deferral.

When time or dependencies prevent full completion, commit the finished coherent improvements and report **partial progress**, the remaining blocking capability, its owner, and the next executable step. Never call the full plan finished while a gate above remains unsatisfied. A request to "complete these plans" authorizes the necessary engine work; it is not an instruction to ask for permission at each ordinary engine refactor.

## Existing owners and references

- [Extension model](extension-model.md) defines the three tiers, authority and backend independence.
- [Domain contracts](extension-domain-contracts.md) specifies port semantics, refusals, time and lifetime.
- [First-class procedural authoring](first-class-procedural-game-authoring.md) is the completed desktop/backend promotion receipt.
- [Fast iteration packet catalog](fast-iteration-implementation.md) holds prior port migrations as history, not a current migration mandate.
- [Public SDK](public-sdk-1.0.md) covers portable game consumers and capability boundaries.
- [Runtime authoring audit](runtime-authoring-audit-2026-11-14/execution-plan.md) covers P1-P6 implementation and lifecycle work.
- [Rhai port reference](../../sdk/script-ports.md) is generated from installed port descriptions.
