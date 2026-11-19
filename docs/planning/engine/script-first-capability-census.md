# Script capability census

Date: 2026-11-17. This audit reads Ambition, Mary-O, and Sanic. It does not change their existing native mechanics. The selected headline customer is the Boss of the Game's encore. Lightning is a second new inventory ability. See the [program](script-first-ambition-gameplay.md).

**Status:** First-slice census retained as source evidence; **active coverage gaps below remain open**. This is not proof of full SDK usability. Recheck the current source before implementing any row, because this list describes the 2026-11-17 snapshot.

## Available contracts

The [runtime registry](../../../crates/ambition_platformer2d_runtime/src/extension_composition.rs) installs these Rhai aliases:

- Boss pattern and presentation: `boss_special`, `boss_conduct`, `mount`, `effect`, `held_ray`, `lightning`, `lightning_aura`.
- Techniques and held items: `technique_use`, `wielded_use`, `wielded_alternate`, `spend_mana`.
- Creation and lifetime: `create_actors`, `spawn_module_entity`, `module_entity_tick`, `projectile`, `end_module_entity`.
- Actor rules: `actor_think`, `fire`, `velocity`, `group_think`, `steer`.
- Custody and references: `take`, `release`, `whereabouts`.
- Room story: `story_tick`, `story_command`, `play_cutscene`.
- Action outcomes: `action_result`, `action_history`, `cancel_action`.
- Native damage decisions: `hit_receptions`.
- Status and hit response: `hold_status`, `status_tick`, `time_scale`, `status_clock`, `in_play`, `struck`, `parried`, `answer_parry`, `parry_window`.
- Combat credit and areas: `credit`, `blast`.
- Bounded body facts: `body_query`, `body_query_result`.
- Physical contacts: `body_contact`, `contact_bounce`, `defeat_body`.
- Object and inventory transactions: `consume_contact`, `inventory_exchange`, `inventory`.
- Effects: `sound`, `pull_bodies`, `launch_toucher`.

The [generated reference](../../sdk/script-ports.md) gives the fields. A descriptor does not grant a phase. The native domain must also install the port in the entry's phase. For example, a request alias can exist while a compatible trigger has no Rhai alias. The portable Rust modules use the same port values, codecs, admission, and state store.

## Representative families

All positions below use world units with +Y down unless stated otherwise. A native entity handle is not a portable identity. Room observations expire at the phase read cut. A script must keep durable references in declared record fields.

| Family and source | Data and start | Read scope, identity, order, bounds | Requests, result, state | Present support and useful gap |
| --- | --- | --- | --- | --- |
| Spatial targeting: [Ambition foes](../../../crates/ambition_combat/src/extension/targets.rs), [swallows](../../../game/ambition_content/rhai/swallows.rhai) | A technique selector or catalog group tag starts a phase call. | `FoesPort` reads living targets in the body's live room, in SimId order. It returns positions with no identities or count/radius parameter. `group_think` returns room members and loose items in identity order; its ids support custody. Both scale with room population. | Steering and item custody go through domain requests. Group records and reference resolution survive rollback. An observation is not a reserved target. | `body_query` supplies bounded, identified native body facts through a correlated reply. It uses the shared relationship owner, distance/identity order, authored room budgets, and exact-ID revalidation. `FoesPort` remains a simpler typed observation. See [body queries](../../sdk/body-queries.md). |
| Boss pattern and stage sequence: [encore jets](../../../game/ambition_content/rhai/boss_stage_pyro.rhai), [boss trigger](../../../crates/ambition_boss_special_port/src/lib.rs) | Authored `Telegraph`, `Strike`, and `Rest` steps select a special key and its windows. | One call per bound key per boss. The read cut gives settled body/combat boxes, facing, optional tracked point, and optional live room size. It has no target identity list or world scan. | A script requests effects, sound, and ordinary projectiles. It keeps pulse state in body records. Idle resets the beat; replay restores it. Pattern admission and native projectile results remain with their owners. | `boss_special` and `effect` now describe existing typed contracts. The native boss modules are independent customers. No new boss-specific engine verb is needed. |
| Body contact and damage: [Mary-O stomp](../../../game/ambition_demo_mary_o/src/stomp.rs), [contact port](../../../crates/ambition_combat_port/src/contact.rs), [hit reaction](../../../crates/ambition_combat_port/src/reaction.rs) | A held status or custom brain selector binds a contact rule; a move's on-hit data selects a reaction. | Body contact samples native physical material after movement and before contact damage. Touchers have authored identities in their live room and phase-local response indices in SimId order. Typed references now serve creation and body queries; contact participant IDs remain authored names. Recipient and toucher can use any controller; room and session bound the sample. `StruckPort` runs on the victim after a strike and carries durable striker credit. Neither is a projectile-owned accepted-hit stream. | Native contact responses and hit reactions own defeat, launch, and status effects. Transient trigger facts are derived again on replay. Module records are canonical state. | `body_contact` version 3 has Rhai and portable records, native surface points and normals, held-status binding, game-authored bounds, and empty samples. It uses physical aperture partitions. `contact_bounce` and `defeat_body` share the native responses. Rhai can also read `struck` and `parried`. See [body contacts](../../sdk/body-contacts.md). A projectile outcome subscription needs shot identity, room lifetime, and replay-safe delivery. |
| Actor intent: [Sanic spin dash](../../../game/ambition_demo_sanic/src/ball_dash.rs), [actor trigger](../../../crates/ambition_combat_port/src/think.rs) | Sanic data claims the Attack slot. Its native body state implements charge, rolling shape, and launch. Catalog custom brains select actor callbacks. | `technique_use` supplies slot edges, body-local aim, gravity basis, resource level, and spawn authority for each bound body. It does not supply arbitrary physics internals. | `steer`, `fire`, and `velocity` enter their domain adapters. Sanic's shape and rolling authority stay native and rollback state stays on the body. | Common intent is scriptable. Sanic's rolling shape service has no portable script contract. Do not emulate a motion solver with script position writes. No migration is selected. |
| Authored world contacts and collision changes: [Mary-O bricks](../../../game/ambition_demo_mary_o/src/bricks.rs), [Sanic monitors](../../../game/ambition_demo_sanic/src/monitors.rs) | LDtk block metadata describes a brick, grant, or monitor. A head contact or rolling/falling overlap starts the rule. | Mary-O uses `ContactSource::Block` and `GeoId`, then an authored name. Sanic selects breakers in SimId order in their own live rooms. Collision overlays read per-room broken names. | A first break spends the room object, grants its effect, and removes collision and art. Per-live-room state rewinds. Authored base geometry stays immutable. | Native/data owners already provide the behavior. Rhai receives native world contacts with geometry identity. The native destructible owner now exposes atomic contact consumption with shared inventory terms. General activation and prop composition remain open. Do not offer a raw geometry setter. |
| Creation and lifetime: [projectile request](../../../crates/ambition_projectile_spec/src/lib.rs), [module entity](../../../crates/ambition_combat_port/src/module_entity.rs) | Technique, held use, or a module entity tick submits authored spawn data. | Scope supplies owner and occurrence credit. The body must have a SimId and a mint stream. Projectile movement uses the native collision and portal path. Module ticks expose their own position, parameters, and scoped targets. | `projectile` and `spawn_module_entity` request creation. `end_module_entity` ends its own scope. Lifetime, room ownership, simulation ids, and flight state rewind. Submitted does not mean a hit occurred. | Available in Rhai and portable Rust. Lightning requires no new spawn service. Accepted-hit batches identify shots and current credit. Creation replies identify actual births. Interception and retirement outcomes remain open. |
| Equipment and form: [Mary-O powerups](../../../game/ambition_demo_mary_o/src/powerups.rs), [Ambition items](../../../game/ambition_content/assets/data/items.ron) | Authored equipment rows specify grants, modifiers, exclusive slots, and hit armor. Pickups or menu host intents request use/equip. | The equipment owner resolves the body and current rows. A slot grant changes the action scheme for any controller. | Shared equipment and world-item collection own equip and consume. Mary-O derives its grown form from worn equipment. Worn rows, resources, and body state rewind. | Data already expresses Lightning's binding. Rhai does not need to set form or collider fields. General equipment transaction results are not a described Rhai service. |
| Travel and room lifecycle: [Mary-O pipes](../../../game/ambition_demo_mary_o/src/pipe.rs), [portal port](../../../crates/ambition_portal_port/src/lib.rs) | LDtk mouths and exits define travel. Input and contact admit entry. | A transit owns a control/pose hold on its body. Live room instances distinguish repeated room definitions. Held portal pairs use owner, module, and slot identity. | Travel owns admission, arrival, release, and room changes. Pipe latches and transit state rewind. A requested portal pair has no acknowledgement port. | Authored travel remains data/native. Held portal pairs are typed Rust/WASM only. A portable accepted-arrival event would support new arrival rules without private room mutation. |
| Room and gameplay state: [intro lesson](../../../game/ambition_content/rhai/intro_training.rhai), [story contract](../../../crates/ambition_platformer2d_runtime/src/story_script.rs) | A room definition selects `story_tick`; authored cutscenes and commands supply actions. | One call per live room root. Frozen room clocks do not call it. Bodies have durable ids; marks have authored names. Flags and body facts are phase observations, not a general event log. Lists are room-sized. | `story_command` prepares a command against the catalog and refuses bad lines or missing rooms. `play_cutscene` refuses unknown ids. Room records rewind. Cutscene playback owns its cursor. | Existing Rhai path supports the Creator lesson. Do not replace it for coverage. General scoped event subscriptions remain open. Native confirmed facts must define their authority. |
| Progression and rewards: [world items](../../../crates/ambition_world_items/src/world_item.rs), [authored commands](../../../crates/ambition_platformer2d_world/src/authored_room_commands.rs) | Catalog pickup effects and prepared room commands express grants. | Collection resolves the touching body and item in the live room. Durable save ownership differs from body-local technique state. | Native owners admit collection and publish the grant. A script can request a registered story command, but cannot infer a durable grant from submission alone. | Prefer existing pickup data. `consume_contact` now supplies an atomic destructible claim with inventory terms. General pickup and custody outcomes remain open. Do not add independent inventory, wallet, and flag setters to simulate a transaction. |
| Presentation: [cutscenes](../../../game/ambition_content/assets/data/cutscenes/intro.ron), [Lightning looks](../../../game/ambition_content/assets/data/projectile_visuals.ron), [ray](../../../crates/ambition_combat_port/src/ray.rs) | Content selects dialog policy, pose, cue, projectile look, or effect. | `sound` resolves its place from the invocation body. Projectile looks do not affect flight geometry. Cutscenes own actors and timing. | Sound and cutscene requests use existing owners. Held rays are native combat requests with presentation derived from their actual casts. | Rhai has `sound`, `effect`, `play_cutscene`, `held_ray`, `lightning`, and `lightning_aura`. Drawn rows remain typed Rust/WASM only. Camera and HUD are authored/native surfaces, not arbitrary script UI mutation. Describe a typed presentation port when a new customer needs it. |
| Game schemas and bindings: [Mary-O vocabulary](../../../game/ambition_demo_mary_o/src/ldtk_vocabulary.rs), [project entry](../../../game/ambition_app/src/app/project.rs) | Providers register pack schemas and LDtk vocabulary. A project supplies pack, worlds, modules, and starting items. | Pack preparation validates named content before publication. Rhai declarations register module schemas and selectors, not arbitrary new LDtk entity compilers. | Reload publishes prepared candidates. Invalid source keeps the last program. State schema admission prevents a module from writing another owner's records. | Existing content families and Rhai records are authorable without Rust. A new domain-specific content schema still needs its native schema owner. This is distinct from adding a named row to an installed schema. |

## Existing examples and open capabilities

The Boss of the Game's [plan](../game/final-boss.md) calls for encore pyrotechnics. The native fight had no stage-jet attack. The [new script](../../../game/ambition_content/rhai/boss_stage_pyro.rhai) adds warning, alternating jets, and a finite pulse limit. Authored pattern data supplies the strike and recovery windows. The engine owns collisions and damage. The first blocker was access to the existing boss-special trigger and authored effect request. Both had native/portable Rust contracts but no Rhai descriptions. The new `boss_special` and `effect` aliases describe those same values. They add no game-named engine operation. Native boss modules and equipment effects are independent existing customers of the contracts.

Lightning now uses `technique_use`, `spend_mana`, `held_ray`, `drawn_row`, and `sound`. Its [script](../../../game/ambition_content/rhai/lightning.rhai) holds a short cone from the body's hand and owns pulse cadence and spread. The engine owns resources, ray casts, guard response, damage, and the leased lightning look. The earlier projectile fan used the same native spawn contract as Pulse Forge and the sentry.

A chain-lightning variant can now request bounded, identified body facts. Native hit receptions and projectile outcomes now supply confirmed combat facts; the current cone does not. Contact is not accepted damage: a shield, invulnerability, or a wall can change the result. Do not fire follow-up damage from a guessed overlap or a presentation event.

The next shared family is scoped interactive-object behavior and event composition, after the substrate repairs below. Contact consumption and native factories supply existing domain actions. Stable object references, activation and lifecycle subscriptions must compose them. A current game consumer is not required. Keep each native game mechanic unless a separately selected product benefit justifies migration.

## Active gap register and acceptance evidence

**Use this section when continuing or completing the [program](script-first-ambition-gameplay.md).** The successful encore and Lightning implementations establish that some scripting paths work; they do not close the gaps below. Classify a completed row as `implemented` or `data-authorable with witness`, with an actual feature and source location. `Still blocked` is not a completion state. Change the domain owner and its contract when necessary: **engine refactoring is permitted and preferred over a bespoke script bridge if it produces a more elegant reusable engine service**.

This register is ordered by likely leverage, not by the number of names in `script_ports()`. Some rows share a root cause and should be solved together.

| Priority and capability | Observed limitation in this snapshot | Owner / place to inspect | Required evidence to close |
| --- | --- | --- | --- |
| **P0: authoritative event triggers — contact samples implemented** | `body_contact` now supplies bounded native body and world-contact samples for held-status and brain bindings, for every controller. World samples retain geometry identity and solver role; old movement output is unavailable. Empty samples, identities, native surfaces, room holds, and replay are covered. Native hit receptions and projectile outcomes supply the separate combat facts below. | `crates/ambition_platformer2d_runtime/src/story_script.rs`; `crates/ambition_combat_port/src/contact.rs` and `reaction.rs`; `extension_composition.rs` | [Loaded contact fixture](../../../game/ambition_app/tests/scripted_body_contacts.rs) activates a contact rule with `hold_status`, reads bounded native facts, keeps a body reference, and requests native bounce for a controlled body and an NPC through replay. A world-contact fixture starts native actor creation from real head contacts on both controller paths. Owning tests cover controller pairs, status/brain deduplication, room/session isolation, holds, and response bounds. Contact samples are implemented (`b4e9e7fe0`); a game-wide event log is not implied. Geometry refusals remain explicit incomplete reads. |
| **P0: domain action outcome** | `story_command` has correlated `action_result` replies, `action_history`, and `cancel_action`. Flag batches and encounter commands publish from their native consumers. Projectile requests now report actual creation or refusal through the same action contract. Inventory exchanges also publish actual application or refusal. `create_actors` and `create_breakables` report completed native construction or refusal. Equipment/custody, cutscene, portal, and older summon requests still need downstream results. | `story_script.rs`; `extension-domain-contracts.md`; the actual domain reducers and extension outbox | Correlate an invocation/request occurrence with typed domain verdicts at a defined read cut, including later application or cancellation. A rejected action cannot advance script-owned success state; rollback does not double-consume results. |
| **P0: reusable script abstractions and composition — partial** | Captured libraries, shared functions, parameterized behaviors, and typed state factories now have a working path. Library environments cannot retain hidden state across invocations. Host records retain nested instance state, including bounded text, options and sequences. Rhai factories and portable `record_value!` types compose the same tagged values. Parameterized factories can create native character compositions and initialize independent typed state. Shared factories also create native destructible props with collision, art, and timed respawn. General activation, object state observations, and cross-object event routing remain open. Room story records now retire with their native root. Each new instance has a fresh root and creation namespace; session records span room visits. General object activation and event routing remain open. | [Compiler](../../../crates/ambition_extension_rhai/src/source.rs); [file preparation and reload](../../../crates/ambition_platformer2d_runtime/src/extension_composition/module_files.rs); [author guide](../../sdk/script-libraries.md) | The loaded `scripted_libraries` fixture composes crafting and firing through shared code. Two modules have distinct state on controlled and autonomous bodies. It checks native outcomes, GGRS, shared edits, refusal, and repair without a host rebuild. Nested schema edits preserve live values by tag; durable save adoption uses the same recursive migration. See [structured state](../../sdk/script-state.md). This does not establish the remaining object and event model. |
| **P0: authored object interaction / collision — partial** | `consume_contact` commits native destructible state and inventory terms from current movement contact. Existing collision, art, respawn, and persistent occurrence owners publish the result. `create_breakables` adds checked room-owned prop factories with native collision, art, and timed respawn. General activation and object state observations remain open. | [Native owner](../../../crates/ambition_breakables/src/consumption.rs); [portable contract](../../../crates/ambition_world_port/src/consumption.rs); [guide](../../sdk/contact-consumption.md) | The loaded `scripted_contact_consumption` fixture checks competing claims, an NPC purchase, actual rewards, refusal, retries, collision/art, GGRS, save/load, and a price edit. The `scripted_breakable_creation` fixture composes room and body factories with actual native contacts, respawn, rules, room retirement, and library edits. See [prop creation](../../sdk/breakable-creation.md). |
| **P0: bounded, identified spatial facts — implemented** | `body_query` returns native facts for a bounded same-room and same-session selection. Exact-ID follow-up can require every target to remain valid. | [Native owner](../../../crates/ambition_combat/src/extension/body_query.rs); [portable contract](../../../crates/ambition_combat_port/src/body_query.rs); [guide](../../sdk/body-queries.md) | [Loaded Rhai fixture](../../../game/ambition_app/tests/scripted_body_queries.rs) checks controlled and autonomous callers, retained body references, bounded selection, stale relationship refusal, and replay. Owning tests cover room/session isolation, identity order, truncation, grudges, and target removal. Commit: `83ae5b219`. |
| **P0: accepted combat contact / damage — implemented** | Projectile spawning and `module_entity_tick` are scriptable. `hit_receptions` supplies bounded native reception decisions to held-status bodies and authored room observers, including actual damage and defenses. It has Rhai and portable values, game budgets, identity capture, and replay delivery. Native emission captures source facts and reserves IDs. Late requests survive rollback, including hit-reaction counter shots. Accepted batches now identify direct and splash shots after source retirement. The common native hit writer retains credit for every hit kind. Creation replies identify actual births. Native `projectile_outcomes` supplies bounded creation, interception, clash spending and retirement facts through portable/Rhai bindings. The loaded reflection fixture observes credit and retirement across deflector loss and GGRS replay. Contact is not synonymous with damage. See [hit receptions](../../sdk/hit-receptions.md). | [Native receiver adapter](../../../crates/ambition_damage/src/extension.rs); [portable contract](../../../crates/ambition_combat_port/src/reception.rs); `HitEvent`; projectile materialization and interception | A new script reacts to confirmed hit/defense/damage with correct striker credit after shield, reflection, invulnerability, lifetime change and replay. Do not calculate damage from animation or overlap. |
| **P1: entity and actor creation/lifetime — partial** | `create_actors` builds prepared character compositions through the shared native planner, including giant clusters and native relations. It initializes body records, statuses, and optional wallets, reports actual identities or refusal, and preserves room/session ownership. Fresh destructible props also use the common planner and source counter. General retirement outcomes, durable dynamic objects, and prop activation remain open. | [Native constructor](../../../crates/ambition_platformer2d_construction/src/runtime.rs); [adapter](../../../crates/ambition_platformer2d_runtime/src/actor_creation.rs); [author guide](../../sdk/actor-creation.md) | The loaded `scripted_actor_creation` fixture creates independently parameterized behavior on ordinary and giant bodies for controlled and autonomous callers. It checks state, actual births, native projectiles, refusal without partial births or counter spending, and GGRS replay. Native planner regressions and creation-rule reload also pass. The lifetime and object families keep this row open. |
| **P1: inventory, equipment, durable progression** | `inventory_exchange` now has atomic costs/grants, real refusal/application replies, retry identity, and an inventory observation. Shared bag history records source room and checkpoint retention. The native bag still has a fixed 24-slot catalog. Authored destructible claims now commit consumption with the same exchange core. Wallet capability configuration is available at character and actor birth. Equipment/custody results and general durable object instances remain open. | `crates/ambition_world_items`; equipment/item custody; authored commands; persistence | A scriptable new interaction uses the existing grant/consume owner, observes success/refusal, and survives room reset, saving and rollback without duplicate rewards. Existing declarative grants may be enough if shown working. |
| **P1: travel, portal and gate lifecycle** | Held portal-pair ports are typed-only; confirmed arrival/exit and request outcomes are not a general Rhai surface. Current gates have no faction authorization rule. | `crates/ambition_portal_port`; room transition lifecycle; gate network domain | A selected new rule can react to confirmed crossing, failed admission and retirement using the existing travel owner. If the illicit one-use gate key is selected, consume it on **successful** authorized connection, not request submission. Do not implement speculative gate lore as SDK policy. |
| **P2: presentation and authoring selectors** | `sound`, `effect`, `play_cutscene`, and `held_ray` work; drawn row, camera/HUD and some other native presentation paths are not Rhai authoring contracts. New schemas can require a native owner. | Presentation/cutscene owners; content pack schemas and LDtk bindings; generated `docs/sdk/script-ports.md` | For actual new content, make presentation and bindings editable through existing authored data or a reusable typed interface; show in-game playback and edit/reload. Do not make arbitrary scripting UI mutation a completion gate. |

### Dependency and phase checks

For each implementation, record **all four parts**: (1) the authored selector/content binding, (2) the trigger or observation at a known read cut, (3) the request and the owning reducer's actual admission/result, and (4) persistent state, rollback and end-of-life rules. The existence of a typed port, Rhai alias or a successful test of its codec establishes only one part. Cross-check the named phase installs at the current source head; a registered descriptor with no installed invocation is not usable.

Favor shared domain repairs over many narrow adapters. The Bevy engine may need a clearer world-object owner, event/result publication seam, or consistent spawn/travel completion contract. Make those changes if they are the clean solution. Keep normal Rust gameplay, Mary-O and Sanic as regression customers; **do not port them to Rhai as a coverage demonstration**. Do not add a game-specific entry to `extension_composition.rs` just because a requested mechanic has a proper noun.

### Candidate proof routes (selection is not canon)

- **Interactive world object (first choice for coverage):** Choose a real Ambition object from active gameplay plans. Use contact/activation, one-time consume, collision and artwork state when appropriate, and a legitimate reward or world-state change. Show two consumers and rollback. A small second authored object should reuse the same semantic path.
- **Combat consequence (separate capability):** A genuinely desired reactive projectile or technique can select identified targets and react to confirmed combat outcomes. Chain lightning is a possible customer, **not** a requirement to ship that move. A shield, reflected shot or miss must not trigger false follow-on damage.
- **Cross-domain journey (only if selected):** The illicit traveler's one-use key could test world pickup -> custody -> travel request -> successful authorization -> consume/return. Its gate policy is still a worldbuilding decision. Use it only after the mechanic is chosen, and avoid embedding a story-specific key into the engine.

The successful Boss encore and Lightning scripts are retained regression witnesses. They should keep working, but adding a third attack with the same `technique_use` + `projectile` pattern **does not** close world-object, event, result or travel capability gaps.

### How to update the register before closing the task

For each P0 and P1 row, record one of these states: `implemented` (with source, installed Rhai/portable binding and observed game behavior); `data-authorable with witness` (with exact authored file and behavior); `still blocked` (with missing owner/contract and next engineering action). Record `not needed for selected game feature` only for P2 items or after explicit maintainer scope reduction; it does not silently close a P0/P1 gap. Prefer existing focused tests and deterministic playback evidence; avoid a large test-count quota.

**Completion rule:** the script-first program remains active while any high-priority row above is `still blocked`. Report partial completion and continue the missing engine/general SDK work. The first-slice receipt is not the program exit. Link final results from this register to the [program's completion gate](script-first-ambition-gameplay.md#completion-gate-and-handoff).

## Program scope after the first slice

The program remains open. The maintainer permits capability work without a
current consumer. Complete a useful semantic family through its domain owner;
use a gameplay example or a focused host fixture as evidence.

### Object and event substrate review

The maintainer's 2026-11-18 review used `2c0438c4252d`. It was a static review.
Fix this substrate before the general interaction and event model. Use focused
behavioral witnesses. Do not migrate working demo gameplay for coverage.

| Priority | Required contract |
| --- | --- |
| P1 | Recipient resolution must name one object occurrence through room travel, equal authored IDs, retirement, and rollback reconstruction. Both native publishing APIs must use the same rule. Implemented: [occurrence receipt](#object-occurrence-routing-receipt). |
| P1 | A callback acknowledges delivery only after its state and requests commit. Faults need bounded deterministic retry or an inspectable terminal diagnostic. Include unavailable subscribers and paused rooms. Implemented: [delivery receipt](#action-callback-acknowledgement-receipt). |
| P1 | Each domain declares cancellation support and its suppression mechanism. Unsupported or late cancellation must be refused. A canceled operation cannot later mutate the world. Implemented: [cancellation receipt](#native-cancellation-contract-receipt). |
| P2 | Invalid callback wiring must be caught at preparation or reported through a guaranteed observation of the submitting scope. Reload must not leave silent orphan work. Implemented: [callback receipt](#callback-preparation-receipt). |
| P2 | Completed object publication follows all native and script initialization. A creation listener must see wallet, statuses, records, relations, and the stable occurrence together. Refused preflight must create nothing. Native boundary implemented: [publication receipt](#completed-native-publication-receipt). Scoped script birth delivery remains open. |
| P2 | Measure native contact collection and frame cost with few and many subscribers in crowded rooms, including two live rooms. Use the native spatial broadphase or confirmed contact stream when the measurements justify a change. Preserve ordering and replay. Local benchmark and native spatial candidates implemented: [contact receipt](#contact-observer-scaling-receipt). Release and device budgets remain unverified. |

Then implement two distinct reusable object behaviors whose events cause each
other's native domain actions. Include accepted and refused outcomes, competing
actions, hot reload, room duplication, travel, and replay. Keep native collision
and damage authority. Do not build an unbounded global event bus or a generic
scene-node clone. The model remains open until these contracts have evidence.

### Cost refusal follow-up

Static review of `ExtensionOutbox::refuse_invocation` found that it removes
unlowered requests after host state has committed. A correlated request in that
outbox can receive no terminal result. Reproduce this with a loaded factory and
an unaffordable native cost, then publish native rejection before removal.
Preserve the distinction between handler commit and native action application.

### Unified object model in progress

Keep the full capability program open while these contracts lack evidence:

- Typed occurrence references, bounded discovery and domain-owned inspection
  must work for actors, props, room roots and session lifetimes. Authored names
  remain locators. Runtime references must not survive the durable save road.
- Reusable behavior bindings must attach owned records and declared handlers
  through the same initialization boundary for actors and props. Factories
  must compose independent instances and relations without game-specific Rust.
- Directed object calls must distinguish native admission, handler delivery and
  logical completion. A handler can await another domain's real result before
  it completes the original call. Reach and scope rules belong to native owners.
- Typed native facts and authored signals need bounded scoped subscriptions.
  Birth, retirement and important state changes must retain provenance. Delivery
  must reuse host commit receipts, with inspectable retry or terminal failure.
  Component removal during rollback reconstruction is not gameplay retirement.
- Two distinct authored behaviors must cause each other's native actions and
  handle competition and refusal. Prove room duplication, travel, retirement,
  replay and live edits through the real host, with portable parity and costs.

Typed references, subject payloads and private behavior bindings are implemented.
Directed calls have a loaded replay witness. Creation-time composition, general
discovery, native event subscriptions and cross-room message
causality during independent room replay remain open. See
[object behaviors](../../sdk/object-behaviors.md).

The remaining contracts need more than aliases: general world-object
activation and composition, actor lifetime, equipment/custody outcomes,
and accepted travel events. Their native authority,
identity, lifetime, and replay rules must be explicit. Existing commands and
visual events do not supply these results.

The conduct observation now supplies the live pattern, encounter state,
tracked body, room geometry summary, and mount. `mount` includes its actual
walkable slope as a polyline. The scripted guitar example combines these
observations with ordinary rays and leased lightning. It does not mutate
world geometry or run a private hit solver.

The status fixture has no shipped gameplay consumer. It composes time scale,
status clocks, body credit, and area strikes for a controlled body and an NPC
through the assembled host. Credit tokens and `u64` record fields use exact
decimal strings in scripts; native wire values remain unsigned 64-bit values.
This also permits records to retain seeds with the high bit set.

## Action-result foundation

The [action contract](../../sdk/action-results.md) now supplies scoped tickets,
next-read-cut replies, bounded history, cancellation, and canonical rollback
state. The portable story values live in `ambition_world_port`. The command
runner distinguishes a native queue write from an applied change. Flag batches
and encounter commands publish from their actual native consumers.

The assembled headless fixture is
[`scripted_action_results.rs`](../../../game/ambition_app/tests/scripted_action_results.rs).
It checks a real flag change, preparation refusal, callback ownership, directed
delivery, and rewind. Owning tests check expired retries, pending bounds, source
removal, and a recreated body with the same identity.

The P0 domain-action row is still blocked for the full program: other action
families must publish their own results. Mailboxes are live-session state. They
do not replace durable consume/reward transactions. World objects, actor creation, inventory, and travel remain open. Bounded identified body queries now have a portable contract, a
Rhai binding, game-authored budgets, and a loaded-host replay fixture.

## Native hit reception

Native damage receivers now publish a required decision with captured identity
and scope. `hit_receptions` exposes bounded batches through portable and Rhai
values. Held statuses bind body rules; authored room rules bind room observers.
The same rules also set the batch budget. Commit: `70d3d62f5`.

The guard is the loaded
[`scripted_hit_receptions.rs`](../../../game/ambition_app/tests/scripted_hit_receptions.rs)
fixture, with native retirement and isolation tests in the damage owner.
The fixture checks both controller paths, defenses, refusal, room observation,
a status response, bounds, and replay. Native damage and demo tests preserve
existing gameplay.

Per-shot identity, retained credit, interception, and retirement now use the
common native services. See [projectile outcomes](../../sdk/projectile-outcomes.md).
Physical contact, damage reception, move confirmation, and interception remain
separate native facts.

## Projectile credit and portable session context

Current projectile credit is canonical state, separate from creation provenance
and the mapped live link. Reflection changes credit and allegiance together.
Scoped repair cannot assign a retired deflector's shot back to its creator.
The runtime publishes identified shots before first contact. Commit: `101f6dd87`.

The guard is
[`scripted_projectile_credit.rs`](../../../game/ambition_app/tests/scripted_projectile_credit.rs):
a loaded module fires, arms the native parry window, and retires an NPC. GGRS
replays the ownership change and retirement. Native tests check reconstruction,
travel, and ambiguous or foreign scope matches. See
[projectile ownership](../../sdk/projectile-ownership.md).

Reception wire version 2 keeps session counters inside native routing. Scripts
receive identities only in their invocation session. Equal gameplay after
different host session histories produces equal wire facts. Commit: `2f88bfb94`.

The P0 combat row remains open for per-shot callbacks and accepted damage
correlation. Captured emission context is covered below.


## Captured projectile emissions

Queued shots could read source facts too late, and late requests could be lost
on rollback. The combat writer now captures source facts and reserves identity.
A cloned and remapped pending queue retains work across ticks. Commit:
`1d0389200`. The projectile model keeps physical construction and admission.

Guards: native
[`projectile_spawn/materialize.rs`](../../../crates/ambition_combat/src/projectile_spawn/materialize.rs)
and the loaded
[`scripted_hit_receptions.rs`](../../../game/ambition_app/tests/scripted_hit_receptions.rs)
fixture. A Rhai hit reaction emits a counter shot after the early materializer;
GGRS replays the pending interval. Session edges clear the queue. Observation
messages do not own spawning. See [projectile ownership](../../sdk/projectile-ownership.md).

The P0 combat row stays open for per-shot notifications and accepted damage
correlation. This receipt does not complete the SDK capability program.


## Retained hit origins

Deferred damage could lose a retired source's credit and the shot identity.
The native `HitWriter` now captures identity for all hit kinds. Detached
volumes and pending hits retain it. Reception version 3 supplies the shot ID
and current credit for direct and splash decisions. Commit: `e7979d090`.

Guards: the loaded
[`scripted_projectile_credit.rs`](../../../game/ambition_app/tests/scripted_projectile_credit.rs)
fixture checks accepted damage after reflection and deflector retirement under
GGRS replay. Native tests check retired melee sources, detached splash origins,
semantic checksums, and portable session masking. See
[hit receptions](../../sdk/hit-receptions.md).

Do not infer the absence of later damage from shot retirement. Spawn replies,
interception results, and retirement outcomes remain open. This receipt does
not complete the SDK program.

## Projectile creation reply receipt

Commit: `0c066fa6b`. The native emission queue carries an optional action ticket.
The materializer reports the actual shot identity after construction. Retries
do not reserve another ID. Native cancellation removes queued work; it cannot
undo a constructed shot. Entry removal does not invent a canceled outcome.

Loaded Rhai fixtures cover both controllers, native refusal, retry identity,
late hit-reaction emission, and GGRS replay. Native tests cover cancellation
after room travel and construction after observer retirement. The app warning
check, causal compile, owner libraries, Smash, schema, resource and focused
content checks pass. Workspace policy has the same 11 baseline violations.
Global asset freshness and broad union/tooling verification remain incomplete.

Interception and retirement outcomes remain open. This receipt does not close
the projectile family or the script-first program.

## Native projectile outcome receipt

Commits: `43545d2c5`, `9d19fd15f`. Native owners publish creation,
reflection, absorption, clash spending and retirement. The installed
`projectile_outcomes` trigger has portable and Rhai values. Pending script
facts are bounded components on their observers. They rewind, follow a live
body through travel, and end with that observer. Hidden work cannot write
into live script observers.

The loaded `scripted_projectile_credit` fixtures check retained credit,
retirement, separate damage results, and pending creation through GGRS replay
for both controllers. Native tests check absorption, bounds, replacement
refusal, visibility, and room/session isolation. Owner libraries, app checks,
Smash, warning checks, absence contracts, documentation links and module maps
pass. Workspace policy retains the same 11 baseline violations. Global asset
freshness and broad union/tooling verification remain incomplete.

Do not infer damage or the absence of pending damage from projectile removal.
The P0 combat row is implemented. The other high-priority rows keep the
script-first program open.

## Inventory exchange receipt

Commits: `e36f11750`, `7acb4c95d`. Portable and Rhai requests apply exact
costs, grants, and wallet changes through the native inventory owner.
Application and refusal produce correlated results. Checkpoint history keeps
actual quantities, source rooms, and retention authority.

The loaded `scripted_inventory_exchanges` fixture checks both controllers,
retries, refusal, observations, save/load, and GGRS replay. A real falling death
checks checkpoint restoration. Item, integration, and session owner tests,
app checks, Smash, warning checks, and absence contracts pass before the
upstream merge. The merged app compiles; nine focused tests pass, including
inventory, checkpoint, schema, resource coverage, and Contextual Blink replay.
Focused union checks also cover inventory, schema, real causal recording,
cutscene input replay, and the four Mary-O spark checks. Workspace policy
retains its 11 baseline violations. The broad union and tooling sweeps remain
incomplete. The partial tooling run had 684 passes and 15 skips, with missing
regeneration targets for three existing character sheets.

The fixed 24-slot catalog and equipment/custody results remain open.
[Contact consumption](../../sdk/contact-consumption.md) now supplies atomic
authored destructible claims. Session action tickets cannot replace a durable
claim. Do not imitate consume-and-reward with separate state setters.

## Script library receipt

Commits: `c12a4e89c`, `c8a52ba02`, `014127af5`. Source files could not share
reusable code.
Prepared library graphs now support functions, behavior parameters, and state
factories. Runtime imports use fresh environments and the caller's work budget.
The file watcher captures dependencies and stages affected entries together.
Script-visible source names also stay independent of installation paths.

Guards: the Rhai owner tests and loaded `scripted_libraries` fixture. They
check closure isolation, budget ownership, real native actions, independent
instances, replay, and edit recovery. See [script libraries](../../sdk/script-libraries.md)
for bounds and measured cost. The default app warning check, optional-backend
profiles, WebAssembly owner check, Android app check, and absence contracts
pass. Workspace policy retains its 11 baseline violations. Broad union and
tooling verification remain incomplete. Do not retain mutable library
environments as a substitute for host-owned records. The remaining object and event capabilities
keep the program open.

## Structured state receipt

Commit: `a5a7fde08`. Reusable behaviors had only flat scalar, option and sequence
fields. SDK API 0.2 adds nested tagged records and bounded UTF-8 text. Rhai
factories and portable `record_value!` types use the same schema. Reload and
durable save adoption migrate retained children by tag. Smaller bounds and
incompatible retained kinds refuse the candidate and keep current state.

Guards: 74 SDK, host and Rhai owner tests, the nested reference observation,
and seven loaded-library and rollback-schema tests pass. The default app warning
check, WebAssembly owner check, Android app check, documentation links and 53
absence contracts pass. Platform checks prove compilation, not device playback.
Broad union and tooling sweeps were not run for this change. Do not use opaque
VM values for persistent state. General object and event composition remain open.

## Actor composition receipt

Commits: `f4a5ec7cf`, `4691f085b`. Scripts could not create general native character
compositions with typed state and confirmed identities. `create_actors` now uses
the shared checked planner, body kits, limb clusters and relations. Initial
records and status selectors make parameterized behavior factories usable from
birth. Per-game creation bounds are authored and reloadable. Giant constructors
reserve their limb identity range and keep the host's faction on unpiloted limbs.

Guards: the loaded actor-factory fixture checks controlled and autonomous
creators, native relations, confirmed projectile births, refusal without partial
births or counter spending, GGRS, and edits on existing bodies. The native facade,
18 SDK tests, 56 planner tests, rule reload, four rollback-schema checks, the Smash
lane, default app warning check, WebAssembly and Android compilation, and 53
absence contracts pass. Policy retains the same 11 baseline violations.
Broad union and tooling sweeps were not run. Do not use the developer scene-setup
message for gameplay creation. Interactive props, general retirement outcomes,
durable dynamic actors and cross-object event routing keep the program open.

## Native world contact receipt

Commit: `ab563c917`. Script contact rules could read bodies but not the world
contacts already known to the movement solver. `body_contact` version 3 now
publishes bounded world contacts with geometry identity, solver role, impact,
and explicit step freshness. The shared core owns the dated movement output.

The guard is the loaded `scripted_body_contacts` fixture: real head contacts
start native actor creation for controlled and autonomous bodies, with actual
replies and GGRS replay. Native tests check order, bounds, and stale output.
See [body and world contacts](../../sdk/body-contacts.md) and the
[local cost measurement](../../../dev/benchmark-candidates/scripted-world-contact-reaction-cost.md).

These facts alone do not authorize geometry writes or reward payment.
[Contact consumption](../../sdk/contact-consumption.md) now supplies native
destructible claims and their collision/art changes. General prop state and
composition remain open. Do not synthesize movement contacts from a second
envelope query. A display name
does not identify geometry.

## Contact consumption receipt

Commit: `6a98fe604`. Scripts could read world contact but could not commit a
native destructible claim and inventory terms together. `consume_contact` now
uses the breakable, inventory, collision, view, and occurrence owners. The
movement stamp also rejects a foreign room or session at the same tick.

Guard: four loaded `scripted_contact_consumption` tests cover competing claims,
an NPC head purchase, native refusal, retry identity, collision/art state,
GGRS, persistent save/load, and a live price edit. Existing inventory, library,
creation, contact, breakable, and Smash tests pass. Default app warnings,
portable WebAssembly compilation, links, and 53 absence contracts pass.
Workspace policy retains the exact 11 prior violations. Broad union and
full tooling verification remain open. Do not replace atomic claims with
separate state setters. General prop creation, activation, state observation,
actor lifetime, and travel remain open.

## Wallet birth receipt

Commit: `47dd9d1ad`. Script-created actors needed native fixture edits to carry
a wallet. Character data and `create_actors@2` now author the starting balance.
The common body applier distinguishes construction from replacement; encounter
waves also complete character initialization at spawn.

Guard: the loaded `scripted_actor_creation` factory funds ordinary and giant
actors that perform native exchanges under GGRS. A live library edit changes
future balances while existing actors keep spent money and identity. Native
catalog, replacement, and full-range arithmetic tests also pass. Do not refill
a wallet on character replacement. Durable NPC wallets and dynamic actors
remain open with the wider object, lifetime, event, inventory, and travel work.

## Native room prop factory receipt

Commit: `23b46d8bd`. Room and body scripts can create checked destructible
batches through the common native planner. Replies name actual objects.
Creation bounds reload from game data. Room lifetime, timed respawn, collision,
stand collapse, visual discovery, and live sheet demand use their native owners.
Feature views and pose history distinguish equal IDs in different live rooms.

Guard: five loaded `scripted_breakable_creation` tests cover both controllers,
retries, refusal, bounds, contacts, rewards, respawn, room retirement, independent
room creators, GGRS, and library edits without a host rebuild. Native owner,
view, render, actor, contact, return, rule reload, Smash, and portable WebAssembly
checks pass. After the upstream sprite merge, the app warning check and seven
loaded prop/actor factory tests pass. A production GPU capture resolves both
requested prop sheets. The policy run before that merge retained the exact 11
baseline violations. Broad union and tooling verification remain incomplete.

Do not save a runtime prop's return timer without its reconstruction recipe.
General activation, object state observations, durable dynamic objects, custody,
and travel outcomes keep the program open.

## Room observer lifetime receipt

Commit: `1cf303415`. Room replacement retained script records and other observer
state on a reused root. Each new room instance now gets a fresh root and
creation namespace. Native retirement removes the old attached state. Joining
an existing live room retains its owner. Traveling bodies retain their own
records, identities, and counters. Session records span room visits.

Guard: 181 publication tests and 348 shared-domain tests pass. The loaded
`scripted_room_state_restarts_on_reentry_and_replays_with_its_root` test checks
initialization and native creation replies once per visit, three visits of
session state, a GGRS crossing, and native door re-entry. The shell identity
test and all 53 absence contracts also pass. See [state lifetime](../../sdk/script-state.md#state-lifetime).

Do not clear only script records when a native owner retires. All attached
observer state must follow the same lifetime. General object activation,
event routing, durable dynamic objects, custody, and travel outcomes remain open.

## Object occurrence routing receipt

Commit: `9bbb0e72e`. Authored IDs could collide across live rooms, and the two
reply APIs examined different recipient populations. Both APIs now use the
same session and native `ObjectOccurrence` stamp. The stamp retains its
allocation namespace through travel. Retirement removes it, and rollback
reconstruction restores it. A replacement receives a different stamp.
Mailbox state no longer owns object identity. Body-query results still report
the current room. Snapshot schema 496 includes the stamp and its room counter.

Guard: the native routing test checks both APIs, equal IDs, a non-mailbox
duplicate, travel, retirement, and snapshot reconstruction. The loaded
`equal_authored_object_ids_receive_independent_replies_after_travel_and_rollback`
fixture passes through GGRS. The existing action fixture, schema checks, 349
shared-domain tests, 15 query/projectile owner tests, assembled app check, and
53 absence contracts pass.

Do not use an authored ID or current room as an object lifetime. The other
substrate repairs are recorded below. The general object/event model remains
open.

## Action callback acknowledgement receipt

Commit: `324f9604b`. Queuing a callback marked it delivered before its handler
ran. The host now acknowledges delivery after state and requests commit.
Faults retry under room rules, then retain a quarantined row. An explicit retry
can recover its terminal callback without repeating native work. History port
version 2 exposes delivery state and attempts. Snapshot schema 497 retains
this state. Full VM error text stays outside canonical state.

Guard: the loaded Rhai fixture faults, reaches quarantine, publishes a repaired
script, and applies its callback once without another native creation. It runs
with GGRS under local session ownership. The held-room and occurrence fixtures
also pass. Executed checks: 7 loaded action/schema tests, 350 shared tests,
47 host tests, 18 SDK tests, assembled app and wasm32 SDK checks, 53 absence
contracts, and doc links. Workspace policy has the same 11 baseline violations
in two failing tests. No full workspace or demo sweep was run for this change.

Do not acknowledge a callback before the host commit. Do not put local VM
error text in rollback state. The object and event model remains open.

## Native cancellation contract receipt

Commit: `7e81de66f`. An absent native handler could report cancellation without
suppressing work. Each installed action domain now declares unsupported,
pending-ticket, or native-handler cancellation. Unknown domains refuse it.
The cancellation request has its own correlated result through port version 2.
A refusal leaves the target unchanged. Success follows native suppression.

Guard: the loaded Rhai fixture checks queued native flag work, supported
cancellation, unsupported and unknown domains, a committed action, an unknown
ticket, and module ownership under GGRS. Each cancellation callback commits
once. Executed checks: 8 loaded action/schema tests, the native projectile
cancellation test, 350 shared tests, 18 SDK tests, 2 rollback resource-accounting
tests, assembled app and wasm32 SDK checks, 53 absence contracts, and doc links.
No full workspace or demo sweep was run for this change.

Do not infer native cancellation support from a missing handler. A queued
consumer must check its pending ticket before mutation or use a suppression
handler. The later receipts record the remaining substrate repairs. The
general object and event model remains open.

## Callback preparation receipt

Commit: `36cc43983`. Callback checks ran after action reservation, and invalid
wiring could commit a waiting script state. Action ports now supply admission
metadata to the shared host. It checks callbacks before state and requests
commit. History port 3 exposes bounded submission refusals without reserving
request numbers. Required reply paths participate in admission, reload, and
portable declarations. SDK API 0.3 uses `ambition-ext-2` and Rhai ABI 3.
Snapshot schema 498 retains refusal rows.

Guard: the loaded Rhai fixture reads its refusal rows, repairs missing and
foreign-module paths with request 0 and occurrence 0, then refuses removal of
a required callback under GGRS. Native and portable host tests check whole
invocation discard. Executed checks include 9 loaded action/schema tests,
7 loaded domain fixtures, 49 host tests, 18 SDK tests, 10 Rhai tests, 2 WASM
tests, 350 shared tests, focused combat and inventory tests, affected test-target
checks, actual assembled WASM replacement, app warning and portable checks,
53 absence contracts, and doc links. All five local portable artifacts were
rebuilt. These behavior checks preceded the upstream linker change; the
assembled app check passed again after its merge. Full workspace and demo
sweeps were omitted.

Do not reserve an action or commit awaiting state before callback preparation.
Do not deliver a wiring refusal through that broken callback. The later
receipts record native publication and contact measurement. The general
object and event model remains open.

## Completed native publication receipt

Commit: `bb8cb1dcd`. Runtime actors became visible before wallet, status and
script-record initialization. Actors and breakables now use one hidden native
construction boundary. It checks the complete batch, assigns all occurrence
references, publishes all roots, then emits native `ObjectPublished` notices.
Owner-prepared components use a root-scoped initializer. A refused batch does
not publish new roots or spend allocation counters.

Guard: the loaded factory listener reads wallet, status, seeded script records,
rig and relation references at publication. Its birth record survives GGRS
without duplicate effects. Executed: 20 assembled creation, contact, action and
schema tests; 351 shared tests; 14 constructor and breakable tests; app check
and default all-target warning check; 53 absence checks. Policy reports the
same 11 baseline violations in two failed groups; 33 groups pass. Full workspace
and optional-feature union checks were omitted. These checks preceded the next
upstream linker merge.

Do not use component-add hooks as completed creation events. Hidden construction
does not undo arbitrary native hook effects or panics. Scoped script birth
delivery and the general object/event model remain open. The contact receipt
below records the later benchmark.

## Contact observer scaling receipt

Commit: `70df6093c`. The sampler compared each observer with all room bodies.
The focused headless sample measured 10.8 ms for 256 observers and 22.1 ms for
512 across two rooms. It now prepares each material piece once and uses the
native layered geometry grid per session and room. Exact checks retain portal
pieces, resting faces, refusal reporting and identity order. The same sample
measures 3.3 ms and 7.0 ms. Few-observer index overhead is below 0.2 ms.

Guard: 12 native contact tests pass, including a distant portal contact and
refused geometry without a usable envelope. The loaded contact, actor, prop,
consume, action and schema population passes: 22 tests, with the separate cost
probe run explicitly. These include GGRS checks after the upstream linker
merge. Smash passes 74 tests, with 6 ignored. The default app all-target
warning check, absence contracts and doc links pass. Full workspace and
optional-feature union checks were omitted. See the
[measurement and remaining limits](../../../dev/benchmark-candidates/scripted-contact-observer-scaling.md).

Do not treat output bounds as a fixed collection budget. Every-body Rhai
polling still takes about 85 ms per complete debug step with 512 observers.
Release and device budgets, the separate room-story polling cost, scoped
script birth subscriptions, and the general object/event model remain open.

## Live occurrence references

Authored names could not retain one live occurrence in script state. Typed
`ObjectRef` values now share one native resolver across object facets and
room or session allocation. Creation and body queries return these references.
State validation rejects checkpoint and durable capture. Commit: `97a3a16f8`.

The guard is the Rhai and actual WASM reference fixture in
[`scripted_action_results.rs`](../../../game/ambition_app/tests/scripted_action_results.rs).
It checks equal authored IDs, travel, retirement, replacement, session state
and GGRS replay. The focused assembled run passed 43 tests with one ignored.
The default-feature app compiled without warnings. Five portable module
artifacts rebuilt. Workspace policy retained its 11 baseline violations;
this change added none. Browser, mobile and the full workspace were not run.
See [object references](../../sdk/object-references.md).

## Private behavior bindings and directed calls

One subject could not host independent instances of the same schema. The host
now separates the native subject and private state owner (`2bf2b9854`). Bindings
attach reusable methods, parameters and records. Directed calls distinguish
admission, committed delivery and explicit completion. Cancellation suppresses
queued methods and is refused after commit. Commit: `6dfe02d1b`.

The guard is
[`scripted_object_behaviors.rs`](../../../game/ambition_app/tests/scripted_object_behaviors.rs).
It checks both controllers, private records, native wallet outcomes, competition,
fault exhaustion without a staged reward, cancellation and GGRS. The broader
focused population passed 45 tests with one ignored. Final index and receipt
changes passed 12 focused tests and 353 shared-owner tests. The default app
warning check, wasm32 world-port check and 53 absence contracts passed. Workspace
policy retained its 11 baseline violations. Actual WASM method coverage and private room membership are recorded below.
Do not infer independent room replay correctness from GGRS alone. See
[object behaviors](../../sdk/object-behaviors.md) for the other remaining work.

## Inherited scope and portable behavior methods

Private instances were absent from room snapshots. `ObjectScopeOwner` now
supplies one owner link for room membership and visibility cuts. Allocation
uses the owning scope's counter. Commit: `c42c48b65`.

The shared scope witness checks travel and owner reconstruction. The loaded
[`scripted_object_behaviors.rs`](../../../game/ambition_app/tests/scripted_object_behaviors.rs)
fixture restores private callbacks through room snapshots without duplicate
rewards. Shared tests passed 354 cases; assembled scope, result, schema and
room rewind tests passed 30 with 14 ignored. The default app warning check and
53 absence contracts passed. Cross-room message causality during independent
replay remains open.

## Portable method results and payload lifetime

Portable methods now return bounded structured data, including occurrence
references, through a ticket-matched observation. A nonterminal callback cannot
read a later completion value. Native admission normalizes named-field order
before data can affect gameplay. Commit: `990939c23`.

The Rhai and actual WASM guards are in
[`scripted_object_behaviors.rs`](../../../game/ambition_app/tests/scripted_object_behaviors.rs).
They use one callback per tick and check returned references, native outcomes,
room snapshots and GGRS. The shared mailbox guard pins a quarantined payload
while later callbacks rotate history. Body queries use the same retention rule.
Final checks passed 15 assembled tests, SDK 19, host 50, Rhai 10 and shared 355.
The default app warning check and 53 absence contracts passed. Workspace policy
retained its 11 baseline violations. Native event subscriptions, creation-time
composition and cross-room message causality remain open.
The merged-tree check also passed the Smash app tests (74 passed, six ignored)
and the assembled app all-targets compile.
