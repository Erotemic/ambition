# Actor residual-kernel decomposition

**State:** OPEN — incremental ownership/dependency work.

Durable decomposition doctrine:
[`../../architecture/package-and-capability-boundaries.md`](../../architecture/package-and-capability-boundaries.md).
Capability closure:
[`capability-and-runtime-composition.md`](capability-and-runtime-composition.md).

## Goal

Reduce `ambition_platformer2d_actor_monolith` to an honest reusable actor/body
simulation kernel. The objective is **not** a target line count and is **not** a
promised frame-time win.

The current reasons to continue are:

- architectural ownership;
- dependency direction and capability closure;
- compile/change isolation;
- test isolation;
- safer feature work;
- reusable engine packages;
- a public SDK that does not expose historical implementation topology.

Recent runtime measurement found a broad set of individually small systems and
did not establish monolith decomposition as a frame-time optimization. Treat
runtime savings as evidence to measure, not as the campaign rationale.

## Residual kernel target

The final actor package should own the closely coupled simulation that truly
belongs to an actor/body:

- authoritative actor/body state;
- control/intent acceptance and actor-local decision integration;
- movement/contact/body-mode integration;
- actor-local lifecycle semantics;
- narrow action/body integration interfaces;
- construction needed specifically to establish the actor-local simulation
  state above.

It should not retain a concern merely because actors happen to use it.

Strong candidates to live outside the residual kernel include:

- named provider/game content and catalog lookup;
- dialogue/conversation;
- encounters and boss orchestration;
- persistence/session policy;
- menus/UI;
- audio/VFX/presentation;
- developer facilities;
- optional item/projectile/portal/mount capabilities with independent semantics;
- world-authoring/backend integration;
- host/platform composition.

Some of these already have dedicated owners. Prefer completing those ownership
moves over inventing new crates.

## Current measured dependency shape

The last reconciled census on 2026-08-29 distinguished three different numbers:

| measure | value |
|---|---:|
| `ambition_*` lines in the monolith `[dependencies]` table | 28 |
| direct edges in the default resolved graph | 27 |
| full default resolved closure | 34 |

Do not compare these as though they were the same metric. Before a footprint
carve, run `cargo tree -i <dependency>`; another path may already keep the crate
in the product closure.

At that census, only four monolith dependencies reached it through a single path
and therefore had the possibility of shrinking closure by removing that edge:

```text
ambition_dev_tools
ambition_mount
ambition_items
ambition_damage
```

This is a prioritization hint, not an instruction to move them blindly. A
multi-path dependency can still be worth removing for ownership or compile
isolation.

## What recent carves taught

Several completed slices established rules that should guide future ones:

- A forwarding/re-export edge can be worth deleting even when total closure does
  not move; it makes ownership honest and prevents future callers from learning
  the wrong path.
- Gating one obvious dependency does not help footprint when another carved
  domain re-supplies it. Count all suppliers first.
- A domain should publish a semantic event/fact and let an optional presentation
  or orchestration consumer translate it rather than importing the optional
  domain downward.
- Domain-owned rollback registration is no longer a reason for generic runtime
  or actor packages to know each concrete gameplay component.
- Moving files without moving the authority, plugin registration and dependency
  edge is not a carve.

The detailed sequence of LDtk, conversation, boss, mount and other historical
carves is available in git history and should not be reconstructed here.

## Slice selection

Before starting a carve, answer:

```text
What authority is moving?
Who owns it after the move?
What systems/resources/messages move with it?
What dependency edge or change-fanout path should disappear?
Does another path keep the dependency in product closure?
What small App or consumer can test the new owner?
What old facade/import/registration can be deleted?
```

A good slice normally satisfies more than one of:

1. removes a direct monolith dependency;
2. shrinks a minimal consumer's resolved capability footprint;
3. removes duplicate/shared authority;
4. isolates a meaningful compile/test unit;
5. deletes a compatibility/re-export path;
6. creates a coherent domain plugin with an independent test surface.

## Current frontier

Prefer outside-in ownership work before splitting the central actor kernel.
Useful frontiers are:

### Developer and presentation dependencies

Developer tools and presentation facilities are poor reasons for the simulation
kernel to depend upward. Remove them when the consumer can observe semantic
facts through an existing engine seam.

### Items, mounts and other optional gameplay domains

Move vertically: the domain owns its state/messages/plugin, the actor kernel
exposes the body/action hooks it needs, and the old actor-side implementation is
deleted. Do not replace one central switch with another.

### Encounter/conversation/world orchestration

The actor kernel should emit/consume small semantic facts. Room, encounter,
conversation and provider orchestration belongs to their owning runtime/domain
packages.

### Character preparation versus actor simulation

Prepared character/content ownership should continue moving toward character
and provider packages. The residual actor kernel consumes prepared body/action
facts rather than becoming the content compiler/catalog.

### Central kernel split

Do this last. Once outer domains are gone, the remaining dependency graph will
show whether body state, movement, decision integration and construction still
need one crate or have another stable seam. Do not pre-split this core because a
source file is large.

## Explicit non-goals

Do not:

- carve by LOC targets;
- add wrapper crates that import the whole monolith;
- scatter feature gates through the kernel merely to move a `cargo tree` number;
- duplicate runtime authority during migration;
- keep historical re-exports for compatibility in this pre-release engine;
- claim runtime/startup improvement without an A/B measurement;
- turn every internal domain into an independently published Bevy crate.

## Exit

This program is complete when:

1. the residual actor package can be described as actor/body simulation without
   listing unrelated product capabilities;
2. optional domains install through semantic capability/plugin seams rather than
   actor-kernel imports;
3. major game/provider orchestration no longer lives in the actor package;
4. minimal consumers do not inherit unrelated domains through the residual
   kernel;
5. the public facade exposes actor semantics without exposing the historical
   monolith name/topology;
6. remaining dependencies are justified by genuine actor-kernel ownership, not
   migration residue.
