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

Promote a focused queue slice only when a concrete system exposes mixed
authority or a recurring unnamed entity/world seam. Do not run a mechanical
workspace-wide wrapping campaign from this receipt.
