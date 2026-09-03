# Bevy system-parameter architecture — distilled

**Status:** DISTILLED 2026-08-30.

The investigation established a durable rule rather than a standing migration
campaign. It now lives at
[`../../architecture/bevy-system-boundaries.md`](../../architecture/bevy-system-boundaries.md).

Key conclusion:

> Do not pack the Bevy parameter ceiling. Name stable entity roles with
> `QueryData`, cohesive world capabilities with small domain-owned `SystemParam`
> values, pure kernel contracts with ordinary structs, and split systems only at
> real phase or mutation-authority boundaries.

The old counts of systems at the ceiling, `SystemParam`/`QueryData` totals and
candidate-by-candidate migration phases were investigation snapshots. They are
not active architecture targets.

> **MEASURED 2026-09-03. ⭐ THIS RECEIPT HAS NO LIVE REFERRER, WHICH IS THE
> OPPOSITE OF ITS SIBLING.** Nothing outside `docs/archive` links here — the only
> three citations are archived documents, and `scripts/check_doc_links.py`
> excludes `docs/archive` on purpose (*"archives preserve stale paths on
> purpose"*). ⇒ So those archive links do NOT hold this file in place; deleting
> it would break nothing any gate checks.
>
> ⚠ **Which does not make deletion obviously right, and the difference is worth
> naming.** [`../engine/architecture.md`](../engine/architecture.md) is a receipt
> kept alive by 16 live policy citations — it has a *link* job, and a measurable
> exit condition. This one has no link job left. What it still carries is a
> WARNING: *"do not run a mechanical workspace-wide wrapping campaign from this
> receipt"*, and the reason that sentence exists is that the investigation's
> counts read like a migration backlog. ⇒ **A receipt whose remaining value is a
> prohibition is not the same object as one whose value is a redirect**, and it
> should be retired by someone deciding the warning is no longer needed — not by
> a sweep that notices nothing links here.

Promote a focused queue slice only when a concrete system exposes mixed
authority or a recurring unnamed entity/world seam. Do not run a mechanical
workspace-wide wrapping campaign from this receipt.
