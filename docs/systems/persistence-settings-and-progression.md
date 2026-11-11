---
status: current
last_verified: 2026-11-10
---

# Persistence, settings, and progression

`ambition_persistence` owns durable save and settings vocabulary and save I/O.
Game/provider content owns named quests, rewards, and authored progression.
Simulation owns live entities and temporary state. Presentation reads these
facts; it does not own persistent truth.

## Three different lifetimes

| State | Examples | Owner |
| --- | --- | --- |
| Durable history | Rewards, switches, quest progress, seen events, saved NPC-life facts | Save data plus provider progression integration |
| User preferences | Audio mix, input settings, display options | Persistence settings model and host adapters |
| Live simulation | Entities, movement, timers, captures, active attacks | Simulation and its rollback/construction authority |

Derived menu or HUD models are neither saves nor authoritative simulation.
Persist provider-qualified stable identities rather than raw Bevy `Entity`
handles. Renaming an ID referenced by saved data requires deliberate migration
or compatibility treatment.

## Content and runtime

A provider's `quest_book` source defines quests with ordered steps and optional
`auto_start` behavior. The schema is owned by
`ambition_persistence::quest::content_schema`; Ambition authors its quests at
`game/ambition_content/assets/data/quests.ron`. Preparation checks duplicate
IDs, empty quests, and invalid condition identifiers. Completion producers
emit typed facts; persistence records durable outcomes.

NPC-life `LifeLedger` preserves a character's facts and last planned hour.
`body::life::persist_lives_to_save` publishes changed ledger rows into the
save, and restore reconstructs active lives from those values and the authored
`LifeBook`. See [NPC lives](npc-lives.md).

The **room snapshot prototype** at
`ambition_platformer2d_runtime/src/rollback/room_snapshot.rs` is a separate,
experimental mechanism for restoring registered component rows of selected
entities. It is not yet an independent room rollback host: it does not capture
resources or checksums. Do not describe experimental simulation-island
reconstruction as shipped independent rollback. The active GGRS authority and
save/checkpoint lifecycle remain separate concerns.

## Settings

```text
persisted preferences
    -> settings and menu models
    -> host/domain adapters
    -> effective runtime configuration
```

Gameplay consumes effective values without doing disk I/O. File locations
remain platform-owned rather than hardcoded in simulation.

When changing persistence, check the relevant load/restore path and a focused
real usage scenario. Avoid routinely running every settings, quest, rollback,
and save suite for a localized change.
