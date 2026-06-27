# Characters & the Hall

Every character is an actor (see [`../engine/unified-actors.md`](../engine/unified-actors.md));
this is how a character's **identity** — sprite, behavior, and *voice* — is authored as
data, and the Hall of Characters that showcases it.

---

## The catalog is the single source of truth

> A character's voice lives on its identity — the catalog row — not scattered across
> hardcoded tables.

`character_catalog.ron` is the durable identity record. Each row carries:

- **sprite metadata** (id, display name, animations keyed by action),
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

The architecture is in place; the job is authoring and cleanup:

1. Migrate the legacy hardcoded bark sources into catalog `barks`.
2. Author `hall` bark lines + a `hall_dialogue_id` Yarn node for every character.
3. **Delete** the legacy fallback code + registry (don't bridge it).
4. Regenerate the Hall spec via the Python tool and re-embed it into LDtk.
