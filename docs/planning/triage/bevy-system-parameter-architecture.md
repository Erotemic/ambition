# Bevy system parameters and mutation boundaries

The durable contract is
[Bevy system boundaries](../../architecture/bevy-system-boundaries.md).
A stable entity role can use `QueryData`; a cohesive domain can use a small
owner-defined `SystemParam`; a pure kernel can use an ordinary struct. Split a
system at a real phase or mutation-authority boundary, not at an arbitrary
parameter or line-count threshold.

The [architecture reassessment](../engine/architecture-reassessment.md) adds a
knowledge test: can the consumer be understood while knowing less about the
producer? A context containing every query, catalog, session resource and callback
fails that test even if a function now takes one parameter. Rust's borrow checks
establish access safety, not semantic authority or lifetime correctness.

Distinguish grouping borrows for one operation from transferring ownership.
`ActorPlacementContext` is an actor/catalog preparation bridge and A3 moves it
with that operation; it should not become the generic world service interface.
`ActorMut` and live actor queries stay outside spawn builders. A parameter wrapper
must not reintroduce the corrected spawn boundary error indirectly.

Promote a focused change only when it names the stable role, accepted writes,
phase visibility and behavior fixture. Old parameter-count censuses are not a
workspace migration backlog. Public scheduling sets still require real ancestry,
run conditions and deferred-buffer visibility tests.
