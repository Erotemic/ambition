# Internal module cycle breaking

**Scope:** source-level cycles inside Ambition workspace crates. This plan is
about domain boundaries. It is not a target to remove every cycle.

**Authority:** [engine architecture](architecture.md),
[actor decomposition](actor-monolith-decomposition.md), and
[capability composition](capability-and-runtime-composition.md).
**Execution order:** [the queue](../queue.md). This page holds candidate work
and design rules. It is not a second execution queue.
**Baseline:** commit `5b47f840ac9f37c7a0a39d54fd296dceb804fc72`, scanned on
2026-10-25 local time. See the
[full graph snapshot](../../architecture/module-cycle-audit-2026-10-25.md).

## Goal

Remove dependencies that cross an ownership boundary in the wrong direction.
Keep a direct dependency when the owner needs a lower domain's fact. Keep a
cycle inside one authority when it protects a single invariant.

A module cycle is a reason to inspect a boundary. It is not proof of a defect.
A smaller strongly connected component (SCC) is not proof of a better design.
Do not add a bus, trait, callback, facade or new crate only to make a graph
smaller. A carve is complete only when state, writers, rules, installation,
lifetime and supported users have a clear owner.

## Baseline and measure

The source-path auditor found the following in the snapshot:

| Scope | Result |
| --- | ---: |
| Workspace members scanned | 107 |
| Members with top-level cycles | 29 |
| Top-level cyclic SCCs | 39 |
| Nested sibling SCCs | 10 in 5 members |
| Members with a cycle at either level | 30 |

`ambition_platformer2d_actor_monolith` has no top-level SCC in this scan.
It still has two nested sibling SCCs: `features::{ecs,npcs}` and
`features::ecs::{errand,navigation}`. The top-level DAG is a completed
structural improvement; the nested results are separate work candidates.

The auditor reads reachable production Rust files and finds qualified module
references, root re-exports, and some grouped imports. It excludes ordinary
test-only code. It does not resolve all Rust names, generated macro paths,
feature selections, inline module trees, or custom source paths. It does not
measure runtime order, Cargo package cycles, or ownership of ECS state.
Confirm any proposed cut in source. Use the existing Cargo, ECS and policy
checks for the different questions they answer.

Run from the repository root:

```bash
python3 scripts/audit_internal_module_cycles.py --top 30
python3 scripts/audit_internal_module_cycles.py --crate ambition_platformer2d_shared_tangle --details
python3 scripts/audit_internal_module_cycles.py --crate ambition_platformer2d_actor_monolith --details
python3 scripts/audit_internal_module_cycles.py --json /tmp/ambition-module-cycles.json
python3 -m unittest scripts.tests.test_audit_internal_module_cycles -v
```

`--crate` accepts a workspace path or crate name. Use `--details` to see
candidate cuts and source sites, including cycles with no top-level SCC.
The JSON output records all detected edges and both SCC levels. The tool
reports evidence; it does not block a build or rank architectural value.

## Boundary decisions

For each candidate edge, choose one of the dispositions already defined in
[actor decomposition](actor-monolith-decomposition.md#edge-dispositions):

- **KEEP_DIRECT:** the caller needs the lower owner's contract.
- **MOVE_AUTHORITY:** one owner must take the state, writers and behavior.
- **MOVE_ADAPTER:** move work that joins two domains to their composition owner.
- **GROUP_PACKAGE:** both modules implement one authority and remain together.
- **HOLD:** a named semantic or test condition must be proved first.
- **REMOVE_AFTER_REPLACEMENT:** replace a former path in full before deletion.

Before any cut, identify the source and target facts, all writers, the owner
of system installation, and the lifetime of each resource. Check which
references are only public aliases. Confirm whether a call is same-tick or
deferred. Check how it runs on rollback, room retirement and session reload.
Do not replace a same-tick rule with delayed delivery without a separate
semantic decision. Do not remove a Bevy component requirement to remove an
edge. Do not move a type to `shared_tangle` merely because several modules use
it. Do not leave an internal compatibility mirror after a completed move.

## Candidate boundary packets

The table is a semantic review order, not a claim that every listed edge must
be removed. Choose work from here in `queue.md` when it is ready to execute.

| Packet | Boundary to inspect | Evidence | Intended result |
| --- | --- | --- | --- |
| C1 | `shared_tangle`: room lifecycle vs body and display | 8-module SCC | Core room identity stays independent of seat and rendering policy |
| C2 | `characters`: actor catalog vs preparation and brain | 10-module SCC | Definition data, control and installation have clear owners |
| C3 | Residual actor kernel: generic ECS vs NPC behavior | 2-module nested SCC | Generic contact and damage do not implement NPC policy |
| C4 | `input`: bindings vs glyph labels | 6-module SCC | Control facts are below device display rules |
| C5 | Provider: world source vs preparation lifecycle | 3-module SCC | World-source contracts do not import an orchestration owner |
| C6 | LDtk: raw project vs conversion policy | 6-module SCC | Data models do not own game-vocabulary decisions |
| C7 | Combat and movement: authority review | 14- and 6-module SCCs | Preserve simulation invariants; extract only a real boundary |

### C1 — live-room contract versus body and display

In `ambition_platformer2d_shared_tangle`, the `lifecycle` module reads
`body::PrimaryBody` from `lifecycle/room_instance.rs:212`. The same file calls
`camera_layers::live_room_band` near line 160. Room identity, primary-seat
selection and camera band policy are different questions.

Keep `LiveRoomInstance`, `InRoomInstance` and the live-room query as the
shared lifecycle contract. Review whether the primary-seat query belongs in
the body or runtime reader, and whether camera band selection belongs in a
presentation reader. A direct call is valid if there is a proven single owner;
do not create a generic room lookup service.

**Acceptance:** a missing primary body still selects the sole live room; more
than one live room has no false sole-room answer; the developer gravity
control and music tie-break keep their defined room choice (Q150). Keep
room-stamp, split-view and session-retirement tests applicable. Re-run the
SCC scan and name any remaining shared invariant.

### C2 — character definition versus preparation and control

`ambition_characters::actor::character_catalog::registry.rs:713-720` installs
`CharacterPreparationPlugin` while it registers a catalog fragment. That
ensures the fragment gets preparation. It also makes the catalog definition
path know an installation rule. The larger SCC includes `actor`, `brain`,
`control`, `prepared` and other actor features.

Review the registry callers first. Keep catalog and brain declarations as
data. Put preparation behavior in its owner and plugin assembly with the
capability or composition owner that has the needed information. Do not make
registration succeed without the barrier it used to install. For the actor
and brain back-edges, determine which facts must be shared and which decision
belongs to the controller.

**Acceptance:** a fragment alone gets the same preparation guarantee; a
prepared character has the same defaults and brain profile; an absent
optional controller does not need dummy resources; public external use does
not need private module paths. Validate session re-entry and character
preparation, not only compilation.

### C3 — generic actor effects versus NPC policy

The residual actor crate's `features::ecs` and `features::npcs` are a nested
SCC. In `features/ecs/damage/actor_hit.rs:258`, generic damage handling reads
NPC-specific behavior. In `features/npcs.rs:93`, the NPC path writes a generic
ECS effect flag. The `errand` and `navigation` modules also form a nested SCC;
that pair may be one navigation authority and must be reviewed separately.

Keep hit facts and accepted damage in the common actor path. Put NPC speech,
provocation policy and their durable effects with the NPC/conversation owner
or an explicit integration system. Keep same-tick and persistence meaning.
Do not add a parallel NPC damage path or a catch-all message bus.

**Acceptance:** the same hit still produces the correct NPC response and
world flag; generic damage no longer names a specific NPC response owner;
replayed and restored states keep the same accepted damage and effects. Do
not split errand and navigation without a reason beyond the SCC count.

### C4 — physical input versus glyph names

`ambition_input::bindings.rs:351` calls `glyphs::button_label` from
`PhysicalControl::label_for`. The glyph module imports binding types. Both
must give one control the same device-specific name, but canonical input data
must not depend on an upper display policy.

Review the users of `PhysicalControl::label` and `label_for`. Keep one table
for gamepad names. Move only the formatting contract or its call sites so
presentation can read binding data without a reverse edge. Do not duplicate
the label table or break public prompts.

**Acceptance:** keyboard, Xbox-like, DualShock and Switch names still agree
between a binding prompt and glyph; changing a binding updates the prompt;
unknown controls keep their fallback label; no input value depends on a
display-only module.

### C5 — provider source versus lifecycle

`ambition_platformer2d_provider::world_source.rs:14` imports
`lifecycle::PlatformerPreparation`. The lifecycle module imports the authored
catalog registry. Inspect whether the source only needs a narrow preparation
contract or whether its code owns an orchestration operation.

Keep world-source interpretation below session preparation. Put a shared
source result or refusal type with its semantic owner. Put sequencing and
installation with the lifecycle or host. Do not introduce a service locator.

**Acceptance:** a bad world file still produces a visible refusal without a
partial live session; a valid source still admits an exact prepared session;
reload and candidate-publication semantics do not change.

### C6 — LDtk data versus conversion

`ambition_platformer2d_ldtk::project.rs:11` imports helpers from `fields`
and `intgrid`, although its header names a pure data-model role.
`fields.rs:25` imports `conversion::LdtkVocabulary` to test a caller's
entity vocabulary. These references tie raw LDtk representation to the
conversion stage.

Keep deserialized LDtk data independent of game-specific conversion rules.
Place vocabulary decisions with the converter that receives the caller's
vocabulary. Keep field decoding with a clear lower-level owner if the data
model needs it. Do not remove the ability to accept game-registered entity
names.

**Acceptance:** built-in and game-added entities still pass validation when
the caller supplies their vocabulary; wrong entities still fail for that
caller; level and field conversion output does not change.

### C7 — combat and movement: preserve the invariant first

`ambition_combat` has a 14-module SCC. A graph-only cut suggests removing
`death_rules -> moveset`. That edge is not a defect by itself:
`death_rules.rs:271` cancels move playback for an out-of-play body. The code
states why a rollback `Added<OutOfPlay>` observer or a paused playback is
not an equal replacement. Move the system only if the new owner preserves
strike-volume teardown on every simulation path.

`ambition_platformer2d_core` has a 6-module SCC and a nested movement SCC.
`body_clusters.rs:121` uses Bevy `#[require(AbilityContributions)]`. It
ensures a spawned `AbilityBase` gets the related component. Do not remove
that requirement to save an import. A narrow shared type may make sense,
but only if it keeps the same spawn and rollback state.

**Acceptance:** combat playback is cancelled and its strike volumes are
removed for out-of-play bodies, including rollback replay. Body spawning
still supplies the required ability contribution. Movement and combat
behavior tests cover the new boundary. If no clear independent owner or
consumer exists, mark the cycle GROUP_PACKAGE and leave it in place.

## Cycles that are not current carve targets

The audit also found cycles that can be correct within one owner:

- `ambition_platformer2d_publication`: transaction and custody handoffs.
  They coordinate one publication receipt and its lifetime.
- `ambition_platformer2d_rollback_ggrs`: session and first-run witness.
  Their calls support rollback checking.
- `ambition_sprite_sheet`: baked-asset generation modules share one output
  generation. Do not split the generation guarantee for graph metrics.
- `ambition_extension_sdk`, `ambition_match`, `ambition_load` and
  `ambition_world_items`: inspect only when a specific boundary blocks a
  supported user.
- Game demo modules and `ambition_app`: composition and local content may
  refer both ways. A game is not a foundation crate.
- Renderer, portal, world, touch input, view and boss modules: review with
  their owner work. Small one-reference cuts can be useful cleanup, but
  they are not the main engine decomposition work.

A cycle can change category when source evidence changes. Record the
reason for KEEP_DIRECT or GROUP_PACKAGE. Do not add a blanket no-cycle gate.

## Working method and acceptance

For each selected packet:

1. Re-run the crate scan at the current commit. Read the source sites and
   callers. Confirm the reference points to the owner the scanner reports.
2. State the bad dependency direction and the intended owner. Identify one
   consumer, test or profile that benefits from the change.
3. Record the current state writers, installation rules, phase order, deferred
   command boundaries, rollback state, session lifetime and cleanup.
4. Move the whole responsibility or its narrow integration adapter. Delete
   internal mirrors. Do not use a trait or event merely to hide a dependency.
5. Check the graph again. Review each remaining back-edge by disposition.
   Treat a changed SCC as evidence, not acceptance.
6. Run focused behavior tests. For authority or dependency changes, include
   `cargo test -p ambition_workspace_policy`. For changes that affect the
   assembled game, use `cargo check -p ambition_app` and the relevant game
   and integration tests. Follow `AGENTS.md` target-bind instructions before
   any Cargo build.
7. Keep the implementation and its behavior witness in one reviewable change.
   Put its execution row in `queue.md` while active, and remove the row when
   done. Keep only a short receipt when another open item needs it.

**End state:** each foundation has one clear responsibility. An upper domain
may call a lower one, and composition may join them. Intentional internal
cycles have a written invariant and remain inside the same owner. The public
facade and independent game profiles do not need private engine modules.
