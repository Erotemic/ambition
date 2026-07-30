# Ambition SDK

**Building a game on this engine? Start here — and you should not need to open
anything under `crates/`.**

That is the acceptance test, not a courtesy. ADR 0031 makes the blind-agent run
one of two mechanical gates on the public API: *can an agent implement a
character, a room and a mechanic with only `docs/sdk/` and `ambition::prelude`
in context, never opening a file under `crates/`?* The recorded result includes
**which engine file it had to open first**, because that field names the next
leak. If you had to open one, that is a bug in this directory.

## Status: slice A, in progress

This SDK is being built one leak at a time by
[the API 1.0 campaign](../planning/engine/api-1.0-campaign.md). Being honest
about what is not here yet is part of the method — a doc that implies coverage
it lacks sends a reader into `crates/` with no warning.

| Area | Status |
|---|---|
| Host composition — standing up a game, visible and headless | **[api-prototype.md](api-prototype.md)** — designed, not yet implemented (A2 done, A3 next) |
| Declaring content — characters, rooms, packs | not started (slice B) |
| Capabilities and rollback schema | not started (slice C) |
| Revising content at runtime | not started (slice D) |

Until a row says implemented, the engine is composed by hand. The worked
example of the hand-composed form is `fixtures/external_consumer` — the
external-consumer fixture, a complete tiny game built from outside the
workspace through the `ambition` umbrella alone.

## The compatibility promise

A game depends on **`ambition`** and nothing else from this workspace (plus
`bevy`, because derive macros resolve `::bevy_ecs` through the consumer's own
manifest).

The promise is made at that surface and nowhere else. Inner `ambition_*` crates
stay independently usable by engine developers and carry **no stability
promise** — if your imports name one, you are depending on our implementation
topology and we will move it.

That is enforced, not asked for: `scripts/check_absence_contracts.py` carries a
module allowlist over consumer code, with a frozen baseline that may only
shrink. Run it to see what the reference consumer still names and how often:

```bash
python3 scripts/check_absence_contracts.py
python3 scripts/check_absence_contracts.py --allowlist-open-count
```

Every module in that output is a leak this SDK has not closed yet. Eighteen at
the start of slice A.
