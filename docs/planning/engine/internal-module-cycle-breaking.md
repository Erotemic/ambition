# Internal module cycle breaking

**Scope:** source-level cycles inside Ambition workspace crates. This plan is
about domain boundaries. It is not a target to remove every cycle.

**State (2026-10-28):** C1–C7 are complete. On `c8b77b877`, the assembled app
check, 2,551 owner and policy tests, and 72 app integration tests passed. The
integration tests cover preparation, refusal, reload, restoration, possession
and real rollback. Keep the intentional groups and required components below.

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

## Reviewed boundary packets

The table records the reviewed boundaries. Each packet below states its owner
and the invariants of the groups that remain.

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

`body::PrimaryLiveRoom` owns primary-body room selection. The lifecycle query
returns room identities; presentation applies `camera_layers::live_room_band`
to the same component-bearing roots. Lifecycle no longer imports body or
camera policy. The room-selection test covers missing and duplicate primary
bodies, sole-room fallback and multiple-room refusal.

Runtime owns the developer request reducer and the primary-frame display
mirror. Gravity no longer imports body selection or frame publication. The only
remaining group is **GROUP_PACKAGE**: `construction`, `feature_overlay` and
`lifecycle` govern room-root admission, candidate visibility and retirement.
Keep their candidate visibility rules and all body component requirements.

### C2 — character definition versus preparation and control

`prepared::CharacterCatalogAppExt` owns fragment registration and installs
its preparation barrier. The catalog registry only assembles definition data.
Registration still builds a complete candidate before it publishes resources;
a failed registration leaves the previous assembly intact. Tests stay with
their owners, including the fragment-only preparation test.

Catalog declarations no longer construct controllers. `control::brain_binding`
owns autonomous sources, context, and initial preset selection; selection and
validation use one namespace rule. The existing actor-spawn builders construct
fresh brains, and `prepared::action_presets` lowers action sets. Live actor
policy stays in the actor kernel. Rollback fields, keys and codecs stay the same.

The remaining character groups are **GROUP_PACKAGE**. `prepared` and
`session_cast` enforce the frozen-generation reload boundary. The declaration,
control, equipment and action-scheme group holds typed character facets and the
body's current repertoire. Keep `ActionSet`'s required resolved scheme components;
do not replace typed character facets with a generic dispatch service.

**Acceptance:** a fragment alone gets the same preparation guarantee; a
prepared character has the same defaults and brain profile; an absent
optional controller does not need dummy resources; public external use does
not need private module paths. Validate session re-entry and character
preparation, not only compilation.

### C3 — generic actor effects versus NPC policy

Generic hit application publishes the accepted `ActorStimulus::DamagedBy`.
The existing aggression reducer owns strike counting, retaliation, speech and
the transition that publishes the durable provocation fact. It consumes the
fact in the same tick. A character with no provoked policy keeps its peaceful
standing and does not announce a hostile turn.

The remaining `errand` and `navigation` pair is **GROUP_PACKAGE**: one goal's
stable item identity drives this tick's route advice, and settlement consumes
that advice to finish or refuse the same goal. Keep snapshot state separate
from the derived graph cache.

**Acceptance:** accepted hits count once; rejected hits produce no response;
per-body thresholds govern the actual transition and its speech. Player-caused
provocation and release keep their durable effects on replay and restore.

### C4 — physical input versus glyph names

The `glyphs` owner implements control and binding labels, including keyboard
labels and seat prompts. It reads canonical binding data and uses one gamepad
label table. `bindings` no longer imports glyph policy. The public label methods
keep their device selection, rebinding and fallback behavior.

The remaining binding, layout, preset, settings and active-device group is
**GROUP_PACKAGE**: it builds and projects one seat's live `InputMap`. A device's
vendor is an input fact; its printed button names belong to `glyphs`.

**Acceptance:** keyboard, Xbox-like, DualShock, Switch and unknown controls keep
their defined labels; a rebind changes the prompt and glyph together.

### C5 — provider source versus lifecycle

`world_source` holds source results and refusal data. Lifecycle owns request
gates, transaction fan-out and refusal publication. `catalogs` owns authored
catalog declarations and registration conflicts. `presentation` projects route
declarations into camera, defense and HUD state. The authoring frontend installs
these owners without an internal compatibility alias. The scan finds no cycle.

**Acceptance:** a bad world still refuses preparation without a partial live
session; valid sources, reloads and candidate publication keep exact prepared
session identity. Route changes select the declared presentation.

### C6 — LDtk data versus conversion

The project model owns raw field decoding and structural entity-layer access.
The converter owns room metadata and named entity-layer interpretation. IntGrid
and terrain owners select their own named layers. Validation asks the caller's
converter registry directly; field decoding imports no conversion vocabulary.
The scan finds no cycle.

**Acceptance:** built-in and game-added entities pass with their own vocabulary;
unknown entities fail for that caller. Parsed fields, level conversion and
entity order keep their defined results.

### C7 — combat and movement: preserve the invariant first

Move playback owns `end_moves_for_bodies_out_of_play`. It reads the lower death
fact and calls the same canonical cancellation path. Runtime keeps it first in
the combat trigger chain. The behavior test checks volume removal and playback
that returns while `OutOfPlay` remains. No observer or paused-clock substitute
replaces the per-tick invariant. The combat group falls from 14 to 10 modules;
its remaining contacts, capture, held repertoire and strike dispatch share one
combat authority and are **GROUP_PACKAGE**.

The core movement group is **GROUP_PACKAGE**. `AbilityBase` still requires
`AbilityContributions`; projection and movement consume one body's authored and
effective capabilities. Keep that component requirement and its rollback state.

**Acceptance:** leaving play cancels playback and its strike volumes on every
simulation path, including replay. Body construction still supplies required
ability contributions. Run movement, combat and rollback behavior tests.

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
