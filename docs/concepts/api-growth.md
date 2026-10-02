---
id: api-growth
aliases: []
status: current
authority: durable-concept
last_verified: 2026-08-13
related_docs:
  - docs/planning/engine/public-sdk-1.0.md
  - docs/planning/engine/capability-and-runtime-composition.md
  - docs/adr/0031-public-facade-is-the-compatibility-boundary.md
---

# Grow the public API from real consumers

Ambition's public engine surface should grow from **consumer friction**, not from
trying to predict a complete 1.0 SDK in advance.

## Method

1. Pick a real consumer that exercises the capability.
2. Use the supported public surface as far as it naturally goes.
3. Identify the rule, ordering requirement, ownership detail, or internal import
   the consumer is being forced to rediscover.
4. Improve the narrow public abstraction that removes that friction.
5. Re-run the consumer and an appropriate behavioral/integration check.
6. Delete transitional/internal exposure when the public seam replaces it.

Prefer evidence that corresponds to actual developer cost: imports the consumer
must name, boilerplate it must repeat, engine-core edits it must make, diagnostics
it receives when content is invalid, and whether visible/headless hosts can share
the same authored composition.

Source allowlists, absence scans, leak logs and blind-agent trials found the first
facade boundary. They are **not mandatory ceremony for every API change**.
Enforce architecture with crate dependencies, visibility, types and the consumer
itself.
