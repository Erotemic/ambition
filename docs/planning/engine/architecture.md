# Engine architecture - retained planning entry point

The current implementation reference is
[the durable engine architecture](../../architecture/engine-architecture.md).
Forward decisions and source corrections are in the
[architecture reassessment](architecture-reassessment.md),
[responsibility map](architecture-responsibility-map.md) and
[decomposition contract](decomposition.md). The [queue](../queue.md) alone selects
execution priority.

This path remains because workspace-policy metadata and source comments cite it.
Do not delete it or mechanically redirect every citation. A rule's destination
must actually state the argument that supports that rule. Recount inbound
references when performing that migration; historical counts are not a gate.

The reassessment changes planned boundaries, not implemented source. Promote each
new contract into durable architecture when its implementation and tests land.
Explicit maintainer rulings continue to constrain both documents. Retain typed
construction, one body/movement authority, domain-owned rollback state, public
phase ordering, scoped ruleset policy, explicit composition and capability absence
tests. Do not infer independence from a crate name or an SCC count.
