# Script-first Ambition gameplay: SDK capability program

**Direction:** Maintainer, 2026-11-17. **Scope:** new gameplay authoring, not migration of existing games. **Execution selection:** [`queue.md`](../queue.md), scripting priority 0b.

**Current status (2026-11-20):** the program's [completion gate](#completion-gate-and-handoff) is met: slices C1–C7 of the [completion plan](#completion-plan-2026-11-20) are done, and the [handoff](#completion-handoff-2026-11-20) gives each gate item its evidence, separating what was run from what is reasoned. The capstone is a playable project ([`examples/projects/bridge_warden`](../../../examples/projects/bridge_warden/README.md)). Later work extends the census rows; it does not reopen the gate.

## Product goal

Rhai should be an effective way to **invent, implement, and iterate on new gameplay in Ambition**, using a prebuilt engine and authored content. Mary-O and Sanic may reveal missing general engine capabilities; neither is a porting target. The same domain contracts must be usable by portable Rust modules built as WASM or linked natively, so successful prototypes may be promoted selectively without changing their content bindings or state semantics.

The completed [first-class procedural authoring acceptance](first-class-procedural-game-authoring.md) proves the three-backend promotion, parity, and desktop packaging workflow for Pulse Forge. It does **not** establish that the SDK is expressive enough to author an arbitrary new Ambition mechanic. Treat A1-A10 as a retained capability and closed receipt, not work to repeat.

Measure progress by the gameplay that authors can express and the engine changes that subsequent ideas no longer require. Playable examples can show this progress. Port counts, script size, and native migrations do not establish capability completeness.

## Reactive capability direction

Maintainer direction, 2026-11-19: complete an island-local reactive authoring
model over native capabilities. Keep stable occurrences, private behavior
state, named methods, the addressed inbox and authoritative domain outcomes.
An object can observe native facts, react to typed notifications and request
native actions. This is an interface over ECS domains, not a new object tree.

A capability supplies read-only facts, typed transitions and typed actions with
actual outcomes. Organize coverage by these contracts and by what an author can
build. The census locates missing native owners; it is not a port checklist.

Direct behavior calls stay within one rollback authority. Cross-island effects
belong to committed world-domain transactions. Do not implement arbitrary remote
methods, cancellation futures or foreign callback forwarding as a completion
requirement. Keep the deferred inbox in `actor_think`; use existing synchronous
domain hooks for decisions that must affect the current physics/combat phase.

The next playable proof is a switch, a door and an actor in an Ambition room.
Use native interaction selection and `InteractReach`. Resolve authored names
within one explicit live room instance through native object identity. Do not
add a script proximity solver or discover placements by guessing coordinates.

Native state and transaction outcomes must remain recoverable. Notifications
can wake a behavior under a declared bounded policy, including coalescing or
loss when reconciliation remains possible. A transient occurrence that cannot
be reconstructed needs retained native evidence or reliable delivery. Capture
occurrence and simulation-scope provenance before source retirement.

The authoring layer must reduce repeated callback and request-number wiring.
Start with Rhai libraries and declaration preparation. Keep the typed lower
level available. Do not add a language, coroutine runtime, universal sequencer,
generic ECS setter or unbounded event log.

## Gameplay expressiveness requirements

Maintainer direction, 2026-11-18: use GDScript gameplay expressiveness as a
reference. Keep Ambition's domain model, authoritative simulation, rollback,
movement rules, and native performance. API count does not measure success.

1. **Address meaningful objects.** Scripts can identify important bodies,
   props, items, and room objects, inspect relevant state, and use their domain
   operations. References have explicit scope, validity, and retirement rules.
2. **React to important events.** Damage, death, pickup, interaction, entry,
   exit, animation completion, and state changes can start authored behavior.
   Native owners report authoritative domain transitions. Global confirmation
   remains a separate world boundary. Delivery has defined ordering,
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

The [addressed behavior inbox evaluation](addressed-behavior-inbox.md) defines
the shared delivery seam for calls, outcomes and future native subscriptions.
It also records the open confirmed transaction and local construction work.

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

1. Secure local calls and outcome delivery. Refuse unsupported foreign work.
   Define local lifetimes, recoverable state, notification policy and source
   retirement. Keep the existing deferred read cut.
2. Complete the playable room interaction. Reuse native activation arbitration,
   add scoped authored-object resolution and useful domain inspections. Author
   switch/door/actor behavior in content and Rhai. Check acceptance, refusal,
   state changes, duplicate live rooms, hot reload and rollback.
3. Add authored signals and essential lifecycle transitions through the same
   inbox. Keep signals distinct from native facts. Start with activation,
   accepted defeat, retirement and relevant authored-object state changes.
4. Demonstrate one confirmed world-domain bridge. A speculative source cannot
   mutate B. A confirmed, identified transaction enters B's deterministic input
   model, survives its rewind and applies once. Include source retirement and
   recipient travel. This is not a general remote-call service. Done
   2026-11-19 for a transient post (`post_world_signal`,
   `a_world_post_crosses_rooms_once_and_only_when_confirmed`); a persistent
   world-domain store waits for a customer.
5. Finish authoring ergonomics and domain coverage through useful variations.
   Check equipment, custody, spawning and travel outcomes. Extract mixed
   actor/prop construction only when a real atomic composition requires it.
   Show edits in a prebuilt host and the intended portable path.

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
2. Normal content and scripts implement a mechanism with several stateful objects, a character that reacts to native combat/world events and handles actual action outcomes, and a persistent consequence between rooms through confirmed world authority. A variation combines existing capabilities without a new Rust adapter. Include the playable room, the normal edit/reload loop, rollback and the intended portable path.
3. Engine changes, if needed for those semantics, are implemented cleanly in owning domains and used by Rhai and portable Rust; no gameplay-specific SDK hacks, raw ECS exposure, parallel simulators or unbounded scans are left as the normal authoring path.
4. A prebuilt-host editing cycle, relevant replay/state/cancellation evidence, and measured cost are documented with commands and observed results. Verification claims distinguish executed checks from static reasoning; do not require a full four-hour suite for a local scripting change.
5. The census records the capability contracts and evidence that support those authoring experiences. A required capability without a working path keeps the program open. Do not substitute port counts for these acceptance conditions or require general remote RPC for theoretical completeness.

When time or dependencies prevent full completion, commit the finished coherent improvements and report **partial progress**, the remaining blocking capability, its owner, and the next executable step. Never call the full plan finished while a gate above remains unsatisfied. A request to "complete these plans" authorizes the necessary engine work; it is not an instruction to ask for permission at each ordinary engine refactor.

## Completion plan (2026-11-20)

Maintainer, 2026-11-20: finishing this program completely is the top
priority. Measured against the completion gate above at `8d74f01d7`, these
are what remain. Each slice lands with a loaded Rhai witness through the real
host, rollback health and its guide section, and updates the
[census](script-first-capability-census.md) row it closes.

| Slice | Gap (source checked 2026-11-20) | Owner |
| --- | --- | --- |
| C1 | Custody requests (`ambition.item.take`, release) are refused by a warning and dropped; a script learns the result only by polling the next observation. Give them action outcomes (applied, refused with a reason) through the action contract. **Done 2026-11-20** (`scripted_custody_outcomes`). | `ambition_held_items`, `ambition_abilities::group` |
| C1b | Equipment: no script request equips a bag item into a body's hand or worn set, or stows it. The native road is the stamped `ItemEquipRequested`/`ItemStowRequested` of a participant, applied by the hand and by the worn set; give it a script request with outcomes. **Done 2026-11-20** (`scripted_equipment_outcomes`). | `ambition_held_items::apply_hand_requests`, `ambition_platformer2d_items::narrative` |
| C2 | Travel and room lifecycles: no confirmed arrival, departure or refused crossing reaches a script, and a room's retirement tells no subscriber outside it that its sources are gone (the retirement fact the 2026-11-19 row left waiting). **Crossings done 2026-11-20**: `object_crossed` and `object_crossing_refused` (`scripted_crossing_facts`, both hosts); a void crossing on a sync test now rebases as a peer's does, because the ring restored the cleared intent and began it again. The room-retirement half moved to C6: a room's teardown is one owner of the same object retirement fact. | room transition commit; room lifecycle |
| C3 | Persistent consequence between rooms: `post_world_signal` is transient. Gate 2 needs persistent world state written through confirmed transactions, saved, readable in every room and observable on change. **Done 2026-11-20**: [world values](../../sdk/world-values.md) (`set_world_value` with an expected revision, `add_world_count`, the `world_values` observation, `world_value_changed` at the session), kept in the save and applied on a tick no room can replay (`scripted_world_values`, including a write from a room that rewinds alone). A script's own session record written from a room is still not retracted by an island rewind; recorded in the island census. | the committed world road (`world_posts`) and persistence |
| C4 | Authored object state beyond destruction: a door or barrier a script locks and unlocks through its native owner, with outcomes, and inspections of that state. **Done 2026-11-20**: world values are authored conditions (`world.value_is`, `world.count_at_least`), so a `LockWall`'s `gated_by` (and any route or dialogue condition) follows a value a script writes with outcomes; `lock_walls` reads the room's gated walls and whether each stands ([guide](../../sdk/world-values.md#open-and-shut-an-authored-gate), `scripted_lock_walls`). | interaction and loading-zone owners |
| C5 | The remaining action outcomes: cutscene play, portal requests and summons. **Cutscenes done 2026-11-20**: `play_cutscene` with a reply hears `cutscene_queued`, `cutscene_started` and terminal `cutscene_ended` from playback, or a refusal (`scripted_cutscene_outcomes`); the pending asks are rollback state on the asking scope (GGRS schema 518). **Summons and portals done 2026-11-20**: `summon_ride` is a Rhai request answered with the mount the executor built (`ride_summoned`, its subject the mount) or `ride_not_made` (`scripted_summon_outcomes`, GGRS schema 519); `held_portal_pair` is a Rhai request and `held_portal_pairs` reads back each held slot, open or spent (`scripted_held_portal_pair`). **C5 done.** | cutscene, portal and summon owners |
| C6 | Actor lifetime: a body's retirement as a fact (other than defeat), and retiring a script's own creation through its owner. A room's retirement publishes the same fact for each subscribed source it takes with it (from C2). **Done 2026-11-20**: `object_retired` is published at the tail of the first tick a subscribed source no longer resolves, once per subscriber (what each subscriber heard is rollback state on it, GGRS schema 520), so a creator's retirement, an owner's despawn and a room taken down behind a body are one fact; `retire_creation` retires an object whose provenance names the caller (`scripted_retirement`). | actor lifecycle; construction; room lifecycle |
| C7 | The gate's mechanism: several stateful objects, a character that reacts to combat and world events and handles outcomes, and a persistent consequence between rooms. Add a variation with no Rust adapter, portable parity, a measured cost, and the census rewritten to current source. **Done 2026-11-20**: the bridge warden ([project](../../../examples/projects/bridge_warden/README.md), [witnesses](../../../game/ambition_app/tests/scripted_bridge_warden.rs)): a room's script makes a warden and two posts, the warden hears native defeats, counts them in the world, lowers the bridge with an expected revision and hears every outcome; the caller retires the fallen posts and the warden hears them leave; the next room's authored gate opens on the confirmed value and that room's script reads it. Variations: a gate on the count alone (no warden write), and the warden's methods as a portable WASM module (`fixture::bridge_warden`) with the same outcomes in the same order. Played by a runner's own strikes on a sync-test host; edited live (published 155 ms after the save); [cost](../../../dev/benchmark-candidates/bridge-warden-cost.md); [census](script-first-capability-census.md) rewritten to current source. | content and Rhai |

## Completion handoff (2026-11-20)

Each item of the [completion gate](#completion-gate-and-handoff), with its evidence. **Run** means a check executed on 2026-11-20 at the commits named; **reasoned** means my reading of the source, not a check.

1. **Expressiveness and authoring experiences.** Run: every P0 and P1 row of the [census register](script-first-capability-census.md#active-gap-register-and-acceptance-evidence) is `implemented`, each with a loaded Rhai witness through the real host (C1 `scripted_custody_outcomes`, C1b `scripted_equipment_outcomes`, C2 `scripted_crossing_facts`, C3 `scripted_world_values`, C4 `scripted_lock_walls`, C5 `scripted_cutscene_outcomes`, `scripted_summon_outcomes`, `scripted_held_portal_pair`, C6 `scripted_retirement`, C7 `scripted_bridge_warden`, beside the earlier object, contact, hit and creation fixtures). Of the seven required experiences, *react* is the warden hearing native defeats; *know whether it happened* is every C-slice's outcome; *world objects* is the toll mechanism and the gated wall; *targets and combat* is `body_query`, `hit_receptions` and `projectile_outcomes`; *inventory, entities and progression* is custody, equipment, creation, retirement and world values; *travel and room lifecycle* is the crossing and retirement facts; *iterate without a build* is item 4.
2. **The mechanism.** Run: `scripted_bridge_warden` (five arms, all pass): the capstone in `blink_run`/`portal_bridge`; the count-gate variation with no warden write; WASM parity; the project played by the runner's own strikes (sync-test host, rollback healthy); the live edit. The project is the playable room. Before the runner could strike, the same walk stopped at the standing gate: the gate is the value's, not the test's.
3. **Clean engine changes.** Reasoned: each C-slice added its port to its owning domain (the item owner, the hand and worn set, the room transition commit, the world domain, the interaction owner's gated walls, cutscene playback, the summon executor, the portal owner, the construction owner), installed through the shared action contract and used by Rhai and portable Rust alike. No raw ECS access, no parallel simulator, no game-named engine entry. The capstone needed no engine change.
4. **Editing cycle, evidence and cost.** Run: the project's script edited in the running host is published 155 ms after the save and lowers the bridge on the next fallen post ([README](../../../examples/projects/bridge_warden/README.md#edit-it-while-it-runs)); every C-slice witness runs on a sync-test host with rewinds; refusals and cancellation are the [cancellation receipt](script-first-capability-census.md#native-cancellation-contract-receipt) and the C-slice refusal codes; cost is [bridge-warden-cost.md](../../../dev/benchmark-candidates/bridge-warden-cost.md) (a story entry run every tick is the dominant cost; the reactive methods are 0.3 ms a call in Rhai, 0.76 ms as WASM, debug build). A sync-test host refuses a reload, by design.
5. **The census.** Run: rewritten to the source of 2026-11-20 (the alias list read from `script_ports()`; each row with its state, owner and witness). What stays outside a row's contract is named there: durable dynamic actors, faction gate rules (a product decision), camera/HUD contracts (P2).

Follow-ups the capstone found, none of them a gate item: a once-per-visit room trigger, so a room's setup does not run a story entry every tick (queue.md, SCRIPT-ROOM-ONCE).

## Existing owners and references

- [Extension model](extension-model.md) defines the three tiers, authority and backend independence.
- [Domain contracts](extension-domain-contracts.md) specifies port semantics, refusals, time and lifetime.
- [First-class procedural authoring](first-class-procedural-game-authoring.md) is the completed desktop/backend promotion receipt.
- [Fast iteration packet catalog](fast-iteration-implementation.md) holds prior port migrations as history, not a current migration mandate.
- [Public SDK](public-sdk-1.0.md) covers portable game consumers and capability boundaries.
- [Runtime authoring audit](runtime-authoring-audit-2026-11-14/execution-plan.md) covers P1-P6 implementation and lifecycle work.
- [Rhai port reference](../../sdk/script-ports.md) is generated from installed port descriptions.
