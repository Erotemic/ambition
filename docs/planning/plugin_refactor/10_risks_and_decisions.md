# Risks, Decisions, and Unintended Consequences

> Historical execution note: this file records the completed plugin-refactor run. It is not current planning guidance; use `docs/planning/plugin_refactor/README.md`, `22_monolith_breaker_survey.md`, and `runtime_extraction_backlog.md` for active follow-up.


This document records tradeoffs that should be revisited during implementation.

## Decision: proto-crate module before real crate

### Option A: isolate `platformer_runtime/` inside `ambition_sandbox`

Benefits:

- Fastest way to make boundaries visible.
- Lets architecture tests enforce dependency direction before Cargo extraction.
- Avoids premature API freezing.

Risks:

- Because it is still one crate, bad imports are technically possible.
- The proto-module can become another dumping ground unless forbidden-import tests exist.

Recommendation: start here.

### Option B: create real crates immediately

Benefits:

- Cargo enforces dependency direction.
- Can improve compile times once stable.

Risks:

- More time spent on packaging before boundaries are proven.
- May freeze bad APIs too early.
- Bevy feature/dependency plumbing can distract from architecture.

Recommendation: do later.

## Decision: plugin registration before file split

For portal, moving registration ownership first is more valuable than file splitting first.

Benefits:

- Shrinks `app/plugins.rs`.
- Reveals schedule/resource coupling.
- Creates a real ownership seam before code is rearranged.

Risk:

- Ordering bugs may appear because registration order was implicit.

Mitigation:

- Introduce stable subsystem sets.
- Use messages for cross-plugin communication.

## Decision: portal as optional plugin

Benefits:

- Forces generic helpers out of portal.
- Makes a clear reusable mechanic boundary.
- Improves build personas.

Risks:

- First feature-gate pass will be noisy.
- Inventory, LDtk conversion, debug overlay, and tests currently assume portal exists.

Mitigation:

- Do not gate portal until plugin shell, helper extraction, and Ambition integration split are done.

## Decision: mostly data-driven content

Good candidates for data:

```text
items, quests, enemy stats, boss tuning, music cues, room/world manifests, rewards
```

Bad candidates for immediate data-driving:

```text
portal transit, ledge grab, collision repair, gravity resolution, projectile collision
```

Risk:

- Over-data-driving creates a custom scripting language and loses Rust's type checking.

Recommendation:

```text
Rust plugins own verbs.
Data/content owns nouns and tuning.
```

## Risk: save compatibility

Moving item IDs, portal gun ownership, quest flags, and content registries can break saves.

Recommendation:

```text
During refactor branches, allow save migration breakage.
Before merge, either add a save version bump/migration or explicitly reset dev saves.
```

## Risk: LDtk feature behavior

If portal is disabled but LDtk maps contain portal entities, choices are:

```text
A. fail loudly
B. silently ignore
C. parse inert data and validate later
```

Recommendation: fail loudly in development. Silent ignore creates confusing authoring bugs.

## Risk: tests become feature-dependent

Portal, gravity-room, inventory, and reachability tests may need feature annotations.

Mitigation:

```text
core_runtime_tests
portal_tests
content_pack_tests
visible_tests
headless_tests
```

## Risk: presentation stays in simulation

Optional plugins only help build times if presentation/audio/UI dependencies are actually gated out.

Watch for render/UI/audio types imported by simulation modules.

## Risk: message API sprawl

Cross-plugin messages are good boundaries, but too many bespoke messages can become a hidden API mess.

Rule:

```text
Cross-plugin communication uses stable semantic messages.
Intra-plugin communication can use private components/resources.
```

## Risk: large-file agent failures

The previous bad overlay failure is a warning: broad replacement of large hotspot files is unsafe.

Mitigation:

```text
- move-only patches for file splits
- no stale full-file replacement of hotspot files
- compile-error-driven fixes
- one seam per patch
```
