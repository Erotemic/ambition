# Portable preparation and load explainability — consolidated

**State:** the preparation campaign has landed; remaining asset lifecycle work is
owned elsewhere.

Current forward work:
[`asset-preparation-and-residency.md`](asset-preparation-and-residency.md).
Durable loading/asset semantics:
[`../../concepts/asset-management.md`](../../concepts/asset-management.md).

The surviving rule is that preparation/readiness is a semantic transaction:
identify required work, validate/prepare it without partially mutating the live
world, expose structured unresolved evidence, then authorize the consumer that
owns the commit. Presentation reports readiness; it does not invent it.

The recent hitch work added a second distinction that this old campaign did not
originally model well: source/decode readiness and render/device materialization
are separate stages. That current architecture now lives in the asset-residency
plan rather than in this historical execution ledger.

Keep this forwarding receipt until Phase 2 can safely update the queue/tracks
links, then remove it.
