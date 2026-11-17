# Script-first Ambition gameplay: SDK capability program

**Direction:** Maintainer, 2026-11-17. **Scope:** new gameplay authoring, not migration of existing games. **Execution selection:** [`queue.md`](../queue.md), scripting priority 0b.

## Product goal

Rhai should be an effective way to **invent, implement, and iterate on new gameplay in Ambition**, using a prebuilt engine and authored content. Mary-O and Sanic may reveal missing general engine capabilities; neither is a porting target. The same domain contracts must be usable by portable Rust modules built as WASM or linked natively, so successful prototypes may be promoted selectively without changing their content bindings or state semantics.

The completed [first-class procedural authoring acceptance](first-class-procedural-game-authoring.md) proves the three-backend promotion, parity, and desktop packaging workflow for Pulse Forge. It does **not** establish that the SDK is expressive enough to author an arbitrary new Ambition mechanic. Treat A1-A10 as a retained capability and closed receipt, not work to repeat.

The measure of progress is *new playable Ambition gameplay* and fewer engine changes required for subsequent ideas. Port count, Rhai lines of code, and the number of existing systems converted from Rust are not success metrics.

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

## How SDK work is selected

1. Select a **genuinely desirable new Ambition gameplay feature** from the actual flagship plans or a maintainer-requested idea. Secondary games can provide additional counterexamples, but do not replace the flagship as the primary customer. Do not fabricate a feature only to justify a port.
2. Attempt the feature with the installed Rhai aliases and authored data first. Note the first blocking missing capability, the relevant native domain owner, and the game-visible effect that is blocked.
3. Decide whether existing data, composition, or a described typed port already solves it. If not, add the *smallest reusable semantic interface* through the domain owner; describe authority, observation scope, read/consume phase, result/refusal, rollback, lifetime, bounds, cost, and availability.
4. Expose that same contract through Rhai and the portable Rust SDK. Do not create script-only domain authority or game-specific extension-host branches. Prefer a normal domain request over a series of private component setters.
5. Complete the selected gameplay feature *in the real Ambition game*, including authored content, in-game interaction and focused validation. Preserve the native implementations elsewhere.
6. Show that the new interface is general, with a second independent use case when it is not obvious from the semantic contract. A focused fixture is enough; do not require a second large game feature or ship a placeholder mechanic.

When an idea genuinely requires a new physics, rendering, or engine subsystem, implement that service natively and provide an appropriate optional scripting API. A script is not required to replace core engine implementation.

## First implementation slice: finite handoff

The initial agent deliverable has three parts:

1. **Coverage census:** identify existing Rhai aliases, existing typed native/WASM ports not yet available to Rhai, and missing cross-game capabilities. Link each proposed high-value gap to its real domain owner and one or more concrete gameplay situations. Mark a category as data-only when scripting is not needed. Do not start with a target number of ports.
2. **One new flagship mechanic:** choose a useful, not-yet-implemented Ambition gameplay mechanic from current product work and implement it primarily in Rhai + authored data. The host's existing Rust gameplay remains intact. If the SDK blocks it, add only the reusable domain capability needed and include a small reuse witness.
3. **Playback and cost proof:** run the mechanic through the supported session path, test its relevant rollback/state and admission/refusal behavior, demonstrate no-Cargo script iteration, and record the script's runtime cost in the scenario that actually uses it. Note whether staying in Rhai or promoting it to portable Rust makes sense; no promotion is required merely for acceptance.

If an intended feature requires a fundamental new service, report it as such rather than disguising a subsystem rewrite as a scripting adapter. Use existing domain tests and a small number of high-value behavior witnesses; no full-suite or test-count quota.

## Acceptance and exit

**The program's first slice is complete when** a new, useful mechanic has actually been added to Ambition from Rhai and authored data; any new SDK interfaces are general, domain-owned, and available to portable Rust; tests/replay establish real behavior; and the author can iterate on the script using a prebuilt host. The coverage census identifies the next most useful capability gap. Retain a concise report of the edit loop and measured gameplay costs.

**The longer-term product goal is achieved when** typical new Ambition gameplay ideas can be implemented without first modifying game-specific Rust code or seeking a new special-purpose engine adapter. This is an ongoing usability outcome, not an instruction to port existing games or expose every engine function.

No fixed count of Rhai modules, ports, or demonstration mechanics is required. The flagship feature and its evidence are the deliverables, not the size of the scripting surface.

## Existing owners and references

- [Extension model](extension-model.md) defines the three tiers, authority and backend independence.
- [Domain contracts](extension-domain-contracts.md) specifies port semantics, refusals, time and lifetime.
- [First-class procedural authoring](first-class-procedural-game-authoring.md) is the completed desktop/backend promotion receipt.
- [Fast iteration packet catalog](fast-iteration-implementation.md) holds prior port migrations as history, not a current migration mandate.
- [Public SDK](public-sdk-1.0.md) covers portable game consumers and capability boundaries.
- [Runtime authoring audit](runtime-authoring-audit-2026-11-14/execution-plan.md) covers P1-P6 implementation and lifecycle work.
- [Rhai port reference](../../sdk/script-ports.md) is generated from installed port descriptions.
