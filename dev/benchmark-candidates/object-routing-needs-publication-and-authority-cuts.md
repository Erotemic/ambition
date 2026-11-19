# Object routing needs publication and authority cuts

An authored placement can have a `SimId` and `SpawnOrigin` without an
`ObjectOccurrence`. A script lookup must not allocate the missing address.
The construction owner must assign addresses after initialization and before
scripted reads. Room publication assigns its constructed roots. Session
publication also assigns completed bodies that were built beside the room.
Outer candidate barriers must fall before native birth observers run.

A provenance registry must include unpublished roots. A Bevy `try_query` can
return no query when an optional component type has not been registered.
Query the required provenance, then read the optional occurrence from each
entity. Do not let an optional capability erase the lookup population.

Source occurrence and current rollback authority serve different purposes.
Capture the source occurrence and room when the native domain applies a fact.
A later notification can retain that provenance after the source retires.
Allocation room is not current residency.

Independent room replay hides foreign entities with `Disabled`. A foreign
endpoint can therefore be visible in the live pass and absent in replay.
Direct-call refusal codes and caller-owned lifetime decisions must not depend
on that difference. Refuse a target outside the visible local authority with
one outcome. End a pending local wait when its endpoint is no longer local or
visible. A live local callee keeps its own active-time deadline.

Native birth publication must return before it builds an occurrence index when
there are no routes. Building an N-object index for each of N births adds
quadratic startup work. A useful benchmark varies room population and the
number of subscribed birth sources separately. Include the generated Hall in
that engine workload; do not change the Hall to hide the cost.
