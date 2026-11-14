# Gameplay composition audit

Snapshot: `34f03976b639`, packaged 2026-11-14. This is a static source review. No build, simulation run, or performance measurement was done in this pass.

## Conclusion

The source has a useful simulation foundation for a runtime authoring layer. Preserve its common movement kernel, frame types, composed collision world, source-owned capability contributions, domain-owned rollback declarations, and staged construction. The next step is to make these capabilities available through stable, runtime-loaded behavior contracts. Replacing them with unrestricted script access to the ECS would make ordinary authoring less predictable.

The most useful composition work is concrete:

1. Make ordinary body forms and numeric effects authored data, with runtime instances and explicit ownership.
2. Give runtime behaviors access to common body requests and their outcomes. Reuse the existing transition and collision functions.
3. Separate an actor's controller from behaviors attached to its body. Possession should change the controller without removing a physical behavior such as a contact reaction or timed status.
4. Keep native collision, portal geometry, and motion admission below the script boundary. Let script policy propose actions, velocities, forces, or trajectories that these owners admit.
5. Extend the external-consumer proof to a prebuilt host with runtime-only game files. Keep the present Rust consumer as the native SDK proof.

This is not a case for a generic split of `ambition_platformer2d_actor_monolith`. Its current module map has 11 root modules, and the source has a coherent control/body/action center. Some broad integration and lifetime responsibilities remain, but a package move is useful only when it removes a dependency or gives a semantic owner its whole lifecycle.

## Method and sample map

I followed the repository's cold-start route, used `scripts/agent_query.py` and the regenerated ECS packet to find owners, and checked the relevant source. The ECS inventory is a locator, not proof that a queried population executes or that a feature is optional in a particular build.

The review samples are:

| Sample | End-to-end source path inspected | Question answered |
| --- | --- | --- |
| Body control and integration | accepted `ActorControl` → participant/autonomous publications → control gate → home/actor adapters → `step_body` → `step_motion` | Is shared physics real, and what must runtime policy preserve? |
| Crouch, morph, and trapdoor | body gesture adapter and `ActorActionMessage` → body-mode transition → shape/collision/presentation readers | Which aspects of a new form can be authored today? |
| Speed Shoes and gravity modifiers | authored Sanic rule → live modifier slot → movement resolver; compared with equipment and ability contributions | Can independent effects compose without adding fields? |
| Portal integration | portal domain plugin → runtime set placement → body geometry/collision admission → movement and combat consumers | What belongs in native code, and what is a reusable extension seam? |
| External SDK | Outlander character/room construction → own authoritative component/codec → simulation system | What has the SDK already proved, and what authoring proof is still missing? |

The source inspection supports architectural claims. It does not establish current player-visible defects or the cost of the inspected functions.

## 1. Body control is substantially unified; preserve the common kernel

### Implemented strengths

`ActorControlFrame` is a shared controller contract. It distinguishes body-local locomotion from a world-space velocity target using `LocalAxes` and `WorldVec2`. Fire requests carry a frame policy. This prevents several classes of gravity and possession errors from becoming conventions that every author has to remember. See `crates/ambition_characters/src/actor/control.rs:160–215`, `:70–157`.

Autonomous decisions are built separately from publication. `publish_actor_decision_frames` writes only bodies without `DrivingParticipant`; participant authority redirects through `project_driving_participant` without replacing the body's `Brain`. See `crates/ambition_platformer2d_actor_monolith/src/body/actors/update.rs:979–990` and `src/control/authority.rs:15–49`.

The common control gate derives a body's action scheme from its live actions, abilities, moveset, techniques, and equipment. It applies room grants only where the body has participant authority. The same derivation is used for prompts. See `crates/ambition_platformer2d_actor_monolith/src/control/acceptance.rs:67–162`. This is a useful model for the script API: the author asks for an action, and the body owns whether it can occur.

There is one registered body integration system. It has disjoint actor and home-body queries, and both routes call `ambition_characters::actor::step_body`, which calls `ae::step_motion`. The lower layer owns collision and actual movement. See:

- `crates/ambition_platformer2d_actor_monolith/src/body/actors/update.rs:1380–1669`;
- `crates/ambition_platformer2d_actor_monolith/src/body/integration.rs:36–122`;
- `crates/ambition_platformer2d_actor_monolith/src/body/movement.rs:267–375`;
- `crates/ambition_characters/src/actor/body_step.rs:11–101`.

Do not describe the two query adapters as two independent physics engines. They share the physical implementation. Their differences include legitimate lifecycle, controller, and outcome policy.

### Remaining design pressure

The integration adapters each assemble a long list of common motion inputs: current frame, tuning, combat state, pose authority, move motion scale, recovery state, room contact, and portal admission. New cross-body semantics can require edits to both adapters. The source comments record earlier mismatches, but those comments are not evidence that the current code still has those defects. Current code forwards `out_of_play`, pose ownership, move playback, and portal admission on both routes.

This is a useful consolidation target after the runtime behavior contract is clear. Introduce a cohesive prepared body-step input or common preparation operation if it removes duplicate derivation. Keep actor-specific death/contact policy outside that operation. Do not merge systems merely to reduce line count, and do not hide unrelated state in a large `SystemParam`.

The current `MotionModel` is a closed native enum: `AxisSwept`, `SurfaceMomentum`, and `AdhesiveCrawler`. Runtime switching already preserves private state for same-model tuning changes and initializes destination state on a model change. See `crates/ambition_platformer2d_core/src/movement/model.rs:35–77`, `:735–746`, `:997–1055`.

A new collision algorithm reasonably needs native code. A new game-specific force law, steering rule, or trajectory should not automatically need a new enum variant. The script boundary should let author policy compute intent or a motion proposal and let the existing sweep/admission owner execute it. This is a design recommendation, not a claim that the current runtime port already supports every such proposal.

## 2. Body forms expose a concrete native-authoring boundary

### What exists

The common collision-safe body-mode transition exists. `try_change_body_mode_clusters` constructs a candidate, keeps the feet fixed in the body's acceleration frame, checks expansion against the supplied collision world, and publishes the result only if it fits. A rejected request leaves the input state unchanged. See `crates/ambition_platformer2d_core/src/player_state.rs:374–420`.

`update_body_mode` uses the composed collision world, so moving platforms, ECS solids, and an admitted portal hole can affect clearance. Its automatic gesture path requires `DrivingParticipant` and the slot's double-tap state. See `crates/ambition_platformer2d_actor_monolith/src/body_mode/mechanics/mod.rs:16–70`, `:142–146`.

That participant filter is an input-adapter fact, not proof that only participant bodies can change shape. Authored trapdoors consume `ActorActionMessage` and query body clusters without a participant filter. They enter `Submerged`, clear retained velocity, later find an admitted arrival, and return to `Standing`. See `crates/ambition_platformer2d_actor_monolith/src/abilities/traversal/trapdoor.rs:26–83`, `:116–174`. The shared transition helper also has no participant dependency.

### Concrete limits

`BodyMode` is a closed enum that combines posture (`Crouching`), movement state (`Climbing`), a compact form (`MorphBall`), and absence from the stage (`Submerged`). `BodyMode::shape` has native dimensions and ratios:

- crouch height is `base_size.y * 0.5`;
- crawl is `(0.85 * width, 0.35 * height)`;
- slide is `(1.05 * width, 0.40 * height)`;
- morph is always `14 × 14`.

See `crates/ambition_platformer2d_core/src/player_state.rs:136–175`, `:217–295`. The morph value is explicitly tied in its comments to the current 16-pixel tunnel. It is also used by posed-body geometry: `crates/ambition_character_sprites/src/posed_body.rs:340–344` calls `BodyMode::shape`.

Thus an author cannot simply define a differently sized compact form through this body-mode surface. A new mode also requires edits to native exhaustive matches, snapshot representation, and relevant readers. This is a direct example of content meaning that can still cause a rebuild.

The gesture system discards the boolean transition result. See its `let _ = ae::try_change_body_mode_clusters(...)` calls at `body_mode/mechanics/mod.rs:158–166`, `:173–181`, `:208–216`, `:229–237`, and `:245–253`. Collision remains safe, but the caller does not publish a semantic answer such as “cannot stand: blocked by this surface.” The existing `IntentOutcome`/`BlockReason` vocabulary shows the intended direction for action feedback, but the body-mode helper returns only a boolean. See `crates/ambition_characters/src/actor/control.rs:21–68`.

### Recommended work package: authored forms plus admitted transitions

Use the present body geometry and transition functions as the implementation base.

1. Move form shape, animation/pose selection, and permitted entry/exit conditions into prepared definitions. Use stable authored IDs resolved to compact handles at preparation.
2. Keep collision-safe resize, feet anchoring, and movement integration native.
3. Expose a body request with an explicit source, target form, and a meaningful result. The controller, move timeline, and script should call the same operation.
4. State which source owns a temporary form and when that ownership ends. A move-owned under-stage state must not be overwritten by the ordinary crouch adapter on the next tick.
5. Do not replace `Submerged` with several freely writable booleans for collision, gravity, damage, and visibility. Keep an admitted semantic transition that applies its dependent facts together.
6. Provide a cheap result for a refused transition and make it visible to an author or headless scenario on demand. Do not add unbounded per-frame logging.

The first acceptance case is small: change the morph form size in runtime content, reload it, and enter/leave the form through the same helper under normal and sideways gravity. A second witness uses a move-owned form whose lifetime survives possession and ends correctly on interruption. No new native form enum arm should be needed for those edits.

## 3. Effects have good precedents but still use special slots

### Good existing composition

`AbilityContributions` is source-owned. The result is `(base union all lends) intersect all ceilings`; a source removes only its own entry. This avoids save/overwrite/restore bugs. See `crates/ambition_platformer2d_core/src/ability_projection.rs:20–70`. Projection also settles invalidated flight, blink, dash-charge, and jump state when capabilities change, at `:74–129`.

Equipment modifiers are resolved at read time. Adds precede Muls and base values are not rewritten. See `crates/ambition_characters/src/equipment.rs:256–290`. Invulnerability reasons and existing source-owned capability contributions are useful separate typed domains. Do not replace all of them with a universal property dictionary.

### Current special slots

Speed Shoes read their duration and scales from authored Sanic rules, but the application code writes `SurfaceMomentumMotion::boost = Some(MomentumBoost { ... })`. It branches on the native motion-model variant and suppresses the effect for the named super form. See `game/ambition_demo_sanic/src/monitors.rs:325–338`.

The movement model owns a single `Option<MomentumBoost>` with two fixed parameters and one timer. See `crates/ambition_platformer2d_core/src/movement/model.rs:631–647`. The current representation expresses one refreshable boost. It does not express several independent sources with separately ending lifetimes.

Timed gravity has another special pair of fields. `MotionModel::set_gravity_modifier` applies only to `AxisSwept` and returns `false` for the other models. See `crates/ambition_platformer2d_core/src/movement/model.rs:248–260`, `:494–513`, `:845–880`. A script binding that hides this return value would make a valid-looking effect silently fail on some bodies.

These are already recognized by `docs/planning/engine/actor-effects-and-modifier-composition.md`. This audit recommends bringing that work into the runtime-authoring critical path. It is not a new competing effects design.

### Recommended work package: typed, source-owned live effects

Preserve the distinctions among immutable tuning, numeric contributions, capability grants, invulnerability, resource quantities, and periodic behavior.

An effect instance should carry its stable definition, source/instance identity, target, explicit stacking rule, and simulation-clock lifetime. A consumer reads prepared or resolved values; it does not learn the name “Speed Shoes.” Use content IDs or prepared handles for dynamic instances. The current `AbilityContributions` key is `&'static str` (`ability_projection.rs:34–52`), which is suitable for a few native sources but is not the identity model to copy for arbitrary script-created instances.

Canonical evaluation order is required when numeric contributors can arrive from independent authors. The existing equipment fold iterates row order. Its algebra is order-independent over exact arithmetic, but floating-point addition and multiplication are not bitwise order-independent. Sort by stable instance/source keys, or otherwise define a deterministic evaluation order. Do not rely on an unordered script table or on the word “commutative.”

Use two effects on the same body as the acceptance case: equipment and a timed pickup both alter top speed; ending either leaves the other contribution. Rewind across expiry and reload the effect definition through the chosen revision policy. This proves the needed behavior without a large policy-test framework.

## 4. Portals show the native boundary worth keeping

The portal implementation is no longer just a host-specific weapon path. Its domain plugin owns portal messages, resources, systems, internal ordering, and rollback registration. The runtime composes its sets with common phases and host input/lifecycle adapters. See `crates/ambition_portal2d/src/plugin.rs:71–193`, `crates/ambition_platformer2d_runtime/src/portal_schedule.rs:24–161`, and `crates/ambition_portal2d/src/lib.rs:89–103`.

The schedule has real semantic ordering:

- portal poses follow moving-platform advancement;
- carves follow poses and gravity-zone collection;
- frame/eviction and crossing operate on the settled body path;
- passage geometry follows transit before later consumers.

See `portal_schedule.rs:45–57`, `:113–141`, and `ambition_portal2d/src/plugin.rs:173–189`. These edges should be part of the runtime behavior contract. A script must not choose an arbitrary Bevy system label or write a position between two geometry phases.

`BodyGeometry` combines material maps and optional spatial restrictions for live rooms. Integrated bodies obtain motion admission from that geometry and the collision world for their own room. See `crates/ambition_platformer2d_shared_tangle/src/body/geometry.rs:53–101`, `:137–180`, and `crates/ambition_platformer2d_actor_monolith/src/body/actors/update.rs:1546–1584`, `:1627–1661`.

This is the right foundation for rich scripted mechanics. A script should be able to place or move a portal through the portal owner, request a body impulse in a named frame, or propose a constrained motion. It should not reproduce body carving, transit reconciliation, or collision sweeps in script.

The source already keeps the distinction between generic portal capability and a held portal-gun adapter (`ambition_portal2d/src/lib.rs:1–16`). Preserve that distinction. A new authored portal emitter should be a customer of the capability, not another engine path.

No runtime speed claim follows from this inspection. The presence of `Vec`s, maps, geometric partitions, or many phase sets does not establish a runtime bottleneck.

## 5. The native SDK proof is real; add a runtime-only proof

Outlander is outside the workspace and imports the public facade. It authors a room, a playable character, an autonomous sentry, staged construction, and a gate with its own authoritative charge state. Character body facts and autonomous policy are separate. See `fixtures/external_consumer/src/lib.rs:116–139`, `:205–259`.

The consumer defines `BeaconCharge`, its snapshot codec, and its simulation-clock update. It can declare authoritative state unknown to the engine. See `fixtures/external_consumer/src/lib.rs:279–335` and the following registration code. This is a substantive native extension proof, not only an import smoke test.

However, the room, character definitions, component, and behavior in this fixture are Rust source. It proves the native SDK. It does not prove a no-rebuild authoring loop. Keep it, and add a second fixture that runs on an already-built host and changes only content plus a runtime behavior module.

That fixture should author a new room, a custom mechanic with explicit state, and a new character from outside the engine repository. It should validate, run headlessly, reload a change, and return structured observations. The “no native rebuild” result follows from the workflow and immutable host artifact, not a permanent test that scans command text for `cargo`.

The facade already exposes useful semantic operations and observations, including `transit_body`, `TransitVelocity`, body facts, move playback, pose views, participant/seat identities, and construction provenance. See `crates/ambition_platformer2d/src/lib.rs:986–1089`. A scripting SDK should use the same underlying owners, while presenting serializable requests and observations rather than Rust query tuples or arbitrary mutable `BodyKinematics`.

## 6. Shared lifecycle and observation are part of the authoring API

The actor owner registers authoritative errands, visits, life state, and the life ledger for rollback; it declares decision snapshots, output frames, and perception as derived. See `crates/ambition_platformer2d_actor_monolith/src/rollback_registration.rs:134–160`, `:202–242`.

The runtime authoring layer should preserve this distinction automatically for module instances:

- authoritative state is serializable and owned by a body, room, or session;
- transient outputs are rebuilt each tick;
- prepared definitions belong to a content revision;
- presentation effects are emitted through existing owners;
- a controller change is distinct from body retirement;
- reload either migrates the instance at a defined boundary or starts a new instance under a documented policy.

The LLM-facing SDK needs answers about behavior, not only lists of ECS types. For each runtime request, expose its target, coordinate frame, phase, required capability, outcome, and relevant source/definition. A failed shape change should report its clearance refusal; an unsupported gravity modifier should name the unsupported body policy. Return this information through scenario/query tooling, using existing read-model and causal machinery where possible.

## 7. Suggested sequence

This table orders local composition work. The integrated [execution plan](execution-plan.md) sets the overall campaign: establish existing-runtime launch and the first stateful script before expanding the composition surface. Forms and numeric effects are bounded parallel customers and must not block that first source-script result.

| Priority | Work | Why it comes here | Completion evidence |
| --- | --- | --- | --- |
| First | Choose one body-attached mechanic with explicit state as the runtime acceptance customer | Forces the contract to cover real behavior and lifetime rather than a one-shot effect | Same mechanic works for a controlled body, an autonomous body, and possession handoff |
| First | Bind common native body operations and structured refusals to the runtime surface | Avoids raw position/shape writes and duplicate physics | Shape/impulse/interaction requests use the same native owners and report rejection |
| First | Complete source-owned numeric effects for current customers | Removes the next likely special field before scripts depend on it | Two sources, independent expiry, deterministic rewind |
| Next | Author body-form dimensions and transitions | Removes a direct content-to-core rebuild cause | A new compact form and dimensions are loaded without a native enum edit |
| Next | Prove a runtime-only external game loop | Establishes the actual product bar | Existing host validates, runs, reloads, and explains a new external game's mechanic |
| Then | Consolidate repeated body-step input preparation where it has become stable | Reduces edit fanout without moving policy into physics | One common derivation feeds both adapters; lifecycle policy remains explicit |

Do not block runtime authoring on a complete actor-kernel rename or a universal effects language. Conversely, do not call the effort complete after a scripting parser can call a handful of commands. The acceptance mechanic must use state, composition, failure feedback, and the engine's real lifecycle.

## Findings to avoid overstating

- The participant body-mode filter is an adapter restriction. Shared transition code and generic authored trapdoors exist.
- Separate home and actor queries do not imply separate movement kernels. Both call `step_body` and `step_motion`.
- The closed motion-model enum is not by itself an architectural defect. Native solvers can remain closed while runtime policy becomes expressive.
- `ParamSchemaRegistry` admits unknown keys, but `TechniqueSupport` replaces that admission path with unknown-key, duplicate-owner, delivery-site, and parameter checks. See `crates/ambition_entity_catalog/src/lib.rs:689–800`. Do not present the old registry as proof that current prepared content accepts missing techniques.
- The decomposition plans have already moved session, construction, and other ownership out of the actor kernel. Do not repeat older findings that assumed those dependencies remain.
- This review does not measure compile time, runtime performance, determinism, or current player-visible failures. It identifies source-supported limits and concrete work that can remove them.
