# Same-room replay / reset

**State:** MERGED into
[`construction-and-reconstitution.md`](construction-and-reconstitution.md).

The surviving architectural finding is simple: a replay/reset path that
hand-maintains its own list of populations to clear and rebuild is a second room
constructor and will drift from fresh construction.

The target is now expressed in the canonical reconstitution program: replay
chooses an explicit retention policy from lifetime/provenance, then uses the same
domain construction semantics as a fresh room. Do not make the legacy reset list
larger as the final architecture.

Historical investigation and the repaired examples remain in git history.
