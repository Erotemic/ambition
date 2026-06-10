# Code smell backlog

Running log of smells noticed *opportunistically* while doing other work (Jon's
standing instruction, 2026-06-10). The rule: while focused on a big task, don't chase
smells — append them here so they aren't forgotten, and revisit later. Only fix inline
when the fix is very clear AND carries no risk of slowing the main task.

Append-only during runs; triage/prune during cleanup passes (move fixed items to the
Resolved section with the fixing commit).

Entry format:

```
## YYYY-MM-DD <short title>
- **Where:** file:line (or module)
- **Smell:** what's wrong, one or two sentences
- **Noticed while:** the task being worked
- **Suggested fix / size:** sketch + rough effort (S/M/L)
```

---

## Open

## 2026-06-10 check_doc_links.py was already red before Stage 20
- **Where:** docs/planning/player-ecs-bandaid-phase0.md, docs/planning/universal-brain-interface.md (engine_core/movement paths), dev/journals/lessons_learned.md (body_mode.rs -> body_mode/), docs/adr/0019 missing "## Current implications for agents" section
- **Smell:** 8 broken local links + 1 missing section predate tonight's run — they reference files deleted in earlier refactor waves (ae::Player deletion, engine_core crate extraction). The checker isn't in CI so drift accumulates.
- **Noticed while:** Stage 20 A3 (fixing the ~37 link breaks the bisection itself caused)
- **Suggested fix / size:** S — update the stale links (or mark those plan docs archived in stale_docs_index.md) and add check_doc_links to CI

## 2026-06-10 ItemKind/Item enums are content-flavored machinery
- **Where:** crates/ambition_sandbox/src/inventory (ItemKind), items (Item) — named variants (HealthPotion, DataChip, PortalGun...) baked into machinery enums
- **Smell:** the item ROSTER is named content but lives as machinery enums; true content/machinery split of items needs a data-driven item registry
- **Noticed while:** A1 menu equip inversion
- **Suggested fix / size:** L — item registry keyed by id, roster authored content-side

## 2026-06-10 EnemyConfig.archetype is the per-archetype tuning hub
- **Where:** crates/ambition_sandbox/src/features/ecs/enemy_clusters.rs:49 + ~25 method-projection reads across actors/damage/mount/save_sync
- **Smell:** the named EnemyArchetype enum is woven into the generic actor layer as its tuning provider; blocks moving the actor combat core into mechanics::combat
- **Noticed while:** A2 kit extraction
- **Suggested fix / size:** L — dissolve into capability components + tuning struct written at spawn (markers: CompositeSpawn, ChargeAttacker, respawn policy already exists)

## 2026-06-10 FeatureVisualKind::Sandbag variant in the generic kit
- **Where:** crates/ambition_sandbox/src/mechanics/combat/events.rs (FeatureVisualKind)
- **Smell:** a named-ish variant in kit vocabulary (excluded from the combat-kit guard word list)
- **Noticed while:** A2 guard authoring
- **Suggested fix / size:** S — rename to TrainingDummy (touches LDtk/content mapping)

## Resolved

(none yet)
