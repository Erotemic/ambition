# Characters & the Hall

Every character is an actor (see [`../engine/unified-actors.md`](../engine/unified-actors.md));
this is how a character's **identity** — sprite, behavior, and *voice* — is authored as
data, and the Hall of Characters that showcases it.

---

## The catalog is the single source of truth

> A character's voice lives on its identity — the catalog row — not scattered across
> hardcoded tables.

`character_catalog.ron` is the durable identity record. Each row carries:

- **sprite metadata** (id, display name, gameplay sheet/manifest, and an
  optional independent portrait-sheet reference),
- a **brain preset + action set** (the behavioral template),
- **bark pools** — one-liners keyed by occasion (`on_hit`, `provoked`, `idle`, `hall`),
- a **`hall_dialogue_id`** referencing a Yarn node.

There is no hand-maintained multi-table registry; identity, behavior, and voice all
hang off the one row.

## Barks

Authored on the catalog row, fired by the situation — a combat hit (`on_hit`), a
peaceful→hostile flip (`provoked`), an idle tick (`idle`), a Hall inspection (`hall`) —
read through `bark_line_for_character_id(id, situation, rotation)`. The reader is
generic; the lines are content.

## The Hall of Characters

A gallery room **generated from the catalog**: each character stands on a pedestal,
inspect-to-converse via a branching Yarn conversation (one node per `hall_dialogue_id`).
The Hall is a playable index of the cast — and a living test of the catalog pipeline
(if a row is malformed, the Hall shows it).

## Remaining work (content + deletion)

The architecture, migration, deletion, and Hall voice pass are complete:

- ✅ DONE (R3.4, `248eb9cc`): barks migrated to the catalog; the legacy
  named-bark tables + registry deleted, not bridged. The catalog `barks` field is
  now the single source of truth via `bark_line_for_character_id`, with only a
  single engine-generic `GENERIC_HIT_BARKS` anonymous default remaining.
- ✅ DONE (2026-07-17): every generated Hall exhibit, including provider-owned
  Sanic and Mary-O forms, has at least one `hall` bark and a catalog-backed
  `hall_dialogue_id` with an authored Yarn node. The Hall spec and LDtk world are
  regenerated from those bindings.
- ✅ GUARDED: the full-host Hall integration test walks every generated
  `NpcSpawn` and rejects missing bark pools, missing or drifted LDtk dialogue
  bindings, and references to absent Yarn titles.
- ✅ FULL DEFAULT PORTRAIT COVERAGE (2026-07-18): every Hall character now
  publishes an independent default portrait sheet. Config-driven generators,
  procedural-Python families, SVG/rig families, multipart bosses, faction
  lineups, and exceptional prop-like speakers all meet the same PNG/RON product
  contract without sharing an artistic representation. Portrait paths derive
  from each catalog gameplay-sheet path, regeneration fails on missing or
  malformed Hall products, and a generated contact sheet supports visual review.
  Named expression/animation playback remains follow-up work.
