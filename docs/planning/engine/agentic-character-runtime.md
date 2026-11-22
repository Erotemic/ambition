# Agentic character runtime — Engine 1.0 program

**State:** OPEN / LATER for general agentic characters. OBS3 below is a
bounded current controller-authoring correction, reviewed at runtime
`00bf92c22142` on 2026-11-22. It does not select an LLM provider or a general
goal framework.

## Current state

| layer | state |
|---|---|
| authoritative world and actor facts | exists: `world_facts`, `WorldFactConditionsPlugin` |
| observations and memory | exist: `WorldMemory`, `PerceptionMemory`, `AgentObservation`, `CombatObservation` |
| goals | Radiant authors scheduled work; broader persistent character goals remain unresolved |
| navigation and reachability over world geometry | in one room, for one body: exists since 2026-10-09 (a surface graph checked in the kernel, and a brain that follows it). Between rooms: absent. Owner: [`platformer-navigation-and-reachability.md`](platformer-navigation-and-reachability.md) |
| planner or policy | native catalogs plus a live module-backed custom-brain route; shared observation demand is missing (below) |
| typed engine action intent | exists and is live: `ActionRequest` in `ActorActionMessage`, consumed by traversal abilities, `brain_effects.rs` and `ambition_held_items` |

Two different boundaries remain:

1. **World navigation and execution.** In-room navigation exists; the
   [navigation owner](platformer-navigation-and-reachability.md) owns broader
   routes. Authored flow reachability, system dependency graphs and local
   fighter recovery are different services. A planned route still needs
   native action admission/completion.
2. **Knowledge across policy providers.** Native templates remain catalog
   types, but `CharacterBrain::Custom` can already select a module through
   [`queue_actor_thinks`](../../../crates/ambition_combat/src/extension/think.rs).
   Mary-O's Laden Swallow and Void Lizard use it. The trigger runs after native
   control publication and before the shared control gate. The missing
   abstraction is an effective observation demand and knowledge policy,
   not the absence of all module-backed policy.

The 2026-10-09 `Roam` receipt—ten edits in six files of two crates plus its
own file—describes adding a native catalog policy. Preserve that historical
cost, but do not use it to justify another provider registry after the
module route has landed. General remote/model decisions still need their
own deterministic acceptance and replay contract.

## Goal

Let persistent characters pursue goals, move through the world, choose engine
actions and participate in dialogue without giving an AI model authority to
rewrite simulation truth.

This is broader than combat `Brain` behavior and narrower than "LLM controls the
game".

## Layering

The domain projects authoritative world/actor facts into the policy's
permitted observations and retained memory. A planner reads those plus its
goals and emits typed engine intent. Ordinary actor/control/interact owners
admit and execute that intent. A world-rule module can receive an explicit
authoritative read contract without making every character omniscient.

LLM-backed reasoning, scripted planners, utility AI and deterministic fallback
brains should be interchangeable policy providers above the same action seam.

## OBS3 — resolve observation demand across controller providers

### Current source and the missing choice

Native [decision wiring](../../../crates/ambition_platformer2d_actor_monolith/src/body/actors/update.rs)
asks only `brain_ref.perception_requirement()`.
`enforce_empty_belief_for_none` clears memory for a native brain with no
perception demand. A sighted belief overrides that call's local
`BrainSnapshot`; it does not change global `ActorTarget`.

Later, `queue_actor_thinks` resolves `ActorTarget.entity` to current
`BodyKinematics` and publishes the position in
[`ActorThink.target`](../../../crates/ambition_combat_port/src/think.rs).
That is the port's documented authoritative contract. The Laden Swallow
has native `Aerial` steering plus module fire; its
[`over` predicate](../../../game/ambition_demo_mary_o_modules/src/laden_swallow.rs)
uses the live trigger target. One body can therefore steer from sighted
belief while firing from current world position. This is a real
compositional difference, not proof that the current long-range firing
policy is unwanted.

A limited-knowledge module cannot repair this by merely reading
`PerceptionMemory`: its native fallback could be `StandStill`/`None`,
and the memory would not be maintained. Conversely, making every custom
brain tactical would force cheap modules to pay for rich views and would
silently change world-rule behavior.

### Bounded implementation

Resolve the body's effective observation demand from its native policy and
admitted module consumers before acquisition. Keep three answers distinct:

| Contract | Selected by | Consequence |
| --- | --- | --- |
| Knowledge scope | The controller/domain binding | Sighted actor knowledge or an explicit authoritative/world-rule read |
| Representation need | The actual consumer | None, narrow target evidence or the existing tactical view |
| Decision timing | The consuming policy | Which current, remembered or delayed cut this decision may read |

A module's representation request cannot grant itself broader knowledge.
Use the existing compiled/admitted binding to carry the choice; do not
rescan every module descriptor inside every actor's decision or place
visibility policy in the generic executor. Recompute effective demand when
the controller/binding changes, including replacement and reload. Removing
the last consumer restores the zero-perception path.

The perception owner prepares one domain projection for the selected cut.
Native brains and a sighted module use that projection or its narrow
target subset. The extension host routes/encodes it. Keep the borrowed
`TargetBelief` path; a module needing one target does not require a
`WorldView`. [OBS1](bounded-perception-and-attention.md#obs1--make-attention-the-native-evidence-admission-boundary)
and [OBS2](world-facts-observations-and-memory.md#obs2--preserve-evidence-meaning-through-memory-and-communication)
own admission and evidence meaning.

Preserve today's `ActorThink` contract for existing consumers, or migrate
one selected content policy with an explicit version/authoring change.
Supply the sighted route through a domain offer/binding whose contract names
its scope, cut and absence behavior. A missing required projection is
unavailable; an admitted projection with no target means no target evidence,
not that the room has no foe. Do not fall back to `ActorTarget.entity`
when the sighted projection is empty.

The world-value, body-query and whereabouts ports remain authoritative
mechanism APIs. `queue_group_thinks` and the shipped
[`swallows.rhai`](../../../game/ambition_content/rhai/swallows.rhai)
already operate as a group rule. They do not automatically become
independent per-bird perception queries. Trusted native modules remain
trusted code; a delivery label is not an access-control sandbox.

Keep existing decision-specific timing. Fighter engagement/pursuit
deliberately uses current admitted target knowledge, while tactical
choice uses `DelayedPerception::perceive`
([`tick_fighter_owned`](../../../crates/ambition_combat/src/brain/fighter/decision.rs)).
The `a_fighter_with_an_engagement_notices_its_foe_before_it_fights` and
`borrowed_and_owned_fighter_world_views_remain_equivalent` guards document
that distinction. Do not impose one latency on all decisions.

### Native customer, proof and deletion

Extend the existing [loaded custom-brain fixture](../../../game/ambition_app/tests/a_custom_brain_module_fires_through_the_gate.rs)
with one explicitly sighted policy. With a visible foe, native and module
decisions share the admitted target. Move the foe outside the viewport
but keep it alive in the same room: the module may use the declared pursuit
estimate, not its new authoritative position. Repeat with a native
`None` fallback to prove the module's demand owns maintenance.

Keep an explicitly authoritative module as a control; it still receives
the live target. Remove the sighted module demand to prove the empty
fast path returns. Use the existing possession, control-gate and rollback
roads for the selected policy. The
[Laden Swallow Rust/Rhai fixtures](../../../game/ambition_demo_mary_o_app/tests/the_laden_swallow_is_a_module.rs)
remain controls for their current authored rule, not permission to change
every shipped module's knowledge.

Delete the direct live-target reread from the sighted route and duplicate
per-body acquisition/maintenance. Keep explicit authoritative routes where
their existing content needs them. The projection is derived; retained
belief and module state follow their current rollback owners. No new
crate, generic `WorldSnapshot`, remote-model adapter or per-frame dynamic
registry is required by this packet. Measure performance only if a later
optimization claim needs it.

## Requirements

- typed action vocabulary rather than free-form mutation;
- deterministic simulation remains authoritative;
- planning failure/timeouts degrade to safe behavior;
- NPC movement uses ordinary navigation/body mechanics;
- dialogue context comes from structured facts/memory;
- headless inspection can explain goal, plan, action and rejection reason;
- no dependency from low-level actor/world crates on an LLM service.

## Candidate crate / Bevy ecosystem value

A small generic "agent controller" plugin could become ecosystem-worthy if it
only defines observation/action/planner contracts and does not assume Ambition's
story or remote model provider. Any actual LLM service adapter should be a
separate optional crate/tool boundary.

## Open design questions — deliberately unresolved

- Which decisions must be deterministic for rollback/network play?
- Can remote/LLM decisions be authoritative, advisory, or only outside rollback
  windows?
- What is the offline/no-model fallback?
- How much world context can an actor inspect directly?
- How are long-running goals represented and interrupted?
- How should characters coordinate or negotiate shared plans?
- What is the latency/cost budget for model-backed characters?
- What safety/content constraints belong in the game rather than the engine?

## Runtime agent boundary versus authoring agent boundary

The [authoring control plane](authoring-and-tools.md) edits, validates and publishes
content outside the simulation tick. A runtime agent instead receives bounded
observations and proposes semantic intentions under the normal actor-control
acceptance rules. These are different trust, latency and lifetime contracts.

A delayed model response needs subject/session identity, observation revision,
expiry and cancellation policy before acceptance. It cannot mutate a Bevy World,
install a provider, alter a moveset graph in place or bypass action eligibility.
Playback/rollback must consume the accepted intent, not re-query a remote model.

Trusted Rust extensions remain trusted code, not a sandbox. Data-authored flows
need A11/A12 validation and execution bounds. Do not create a universal runtime
service bus to make an agent capable of everything the engine can express.
