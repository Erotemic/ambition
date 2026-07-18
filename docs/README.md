# Ambition documentation map

Use this page to choose the smallest trustworthy packet. Do not read the whole
documentation tree.

## Ten-minute orientation

1. Read [`../README.md`](../README.md) and [`../AGENTS.md`](../AGENTS.md).
2. Read [`.agent/README.md`](../.agent/README.md), then run:

   ```bash
   python scripts/agent_query.py overview
   python scripts/agent_query.py "<the user's task words>"
   ```

3. Read [`concepts/engine-mental-model.md`](concepts/engine-mental-model.md).
4. Read [`planning/vision.md`](planning/vision.md) and the relevant entry in
   [`planning/tracks.md`](planning/tracks.md).
5. Inspect one crate packet, one focused source owner, and the narrowest tests.

That is normally enough to begin useful work.

## Authority ladder

| Need | Authority |
|---|---|
| User intent for this task | the current user request |
| Project direction and live queue | [`planning/`](planning/README.md) |
| Accepted architectural decisions | [`adr/`](adr/README.md) |
| Durable vocabulary and invariants | [`concepts/`](concepts/index.md) |
| Current subsystem implementation notes | [`systems/`](systems/index.md), then source |
| Repeatable procedures | [`recipes/`](recipes/index.md) |
| Author-time tools | [`tools/`](tools/index.md) |
| Current implementation fact | source, manifests, and tests |
| Localization | generated `.agent/` indexes |
| Failure history and engineering lessons | [`../dev/`](../dev/README.md) |
| Superseded evidence | [`archive/`](archive/README.md) and git history |

Generated indexes locate likely owners; they do not override source. Planning
wins for intended direction. Source and tests win for current implementation.

## Durable engine concepts

Start with:

- [`concepts/engine-mental-model.md`](concepts/engine-mental-model.md) — the
  stable layer/data-flow picture.
- [`concepts/content-and-provider-boundaries.md`](concepts/content-and-provider-boundaries.md)
  — what providers own and how sessions activate.
- [`concepts/input-and-game-modes.md`](concepts/input-and-game-modes.md) — one
  control/action/prompt path.
- [`concepts/sim-presentation-seam.md`](concepts/sim-presentation-seam.md) —
  authoritative simulation versus derived presentation.
- [`concepts/testing-and-validation.md`](concepts/testing-and-validation.md) —
  how to prove a change.

Use [`concepts/index.md`](concepts/index.md) for the full focused list.

## Current system docs

System docs are intentionally fewer and shorter than the source. They explain a
current cross-crate flow or authority boundary that cannot be discovered from a
single module map. Exact symbol inventories belong in `.agent/`, not prose.

See [`systems/index.md`](systems/index.md). If a system page reads like a
migration ledger, future plan, or dated audit, move it to `archive/` or delete it.

## Procedures and tools

- [`recipes/index.md`](recipes/index.md) — commands and repeatable workflows.
- [`tools/index.md`](tools/index.md) — author-time generators, validators, and
  reports.
- [`mechanics/index.md`](mechanics/index.md) — stable gameplay-mechanic contracts
  and expressibility tests.

A recipe must work on the current tree. A tool doc must name its real launcher,
dependencies, outputs, and whether it mutates checked-in content.

## Planning, incubation, and history

- [`planning/`](planning/README.md) is the master plan and live queue.
- [`brainstorms/`](brainstorms/) is Jon's active design-incubation space;
  agents do not write there.
- [`vision/`](vision/index.md) contains auxiliary vision notes; binding direction
  lives in `planning/vision.md`.
- [`archive/`](archive/README.md) preserves superseded reviews, migrations, and
  handoffs as evidence, not authority.

`docs/current/` is retired and should not be recreated.

## Freshness rule

Prefer stable contracts over path-heavy inventories. A durable doc should state:

- who owns authority;
- the data flow and invariants;
- how another provider/controller/platform participates;
- how reset/restore/headless execution behaves;
- how to localize the current implementation and tests.

When exact paths are useful, verify them in the same patch and include a
`last_verified` date. Delete completed migration narratives rather than keeping
them in `systems/`.

Run after documentation changes:

```bash
python scripts/check_doc_links.py
python scripts/generate_agent_index.py
```
