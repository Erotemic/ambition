# Immutable content and transactional construction

**State:** DISTILLED — the core architecture is implemented. Remaining lifecycle
convergence is owned by
[`construction-and-reconstitution.md`](construction-and-reconstitution.md).

Prepared content is immutable authority for one content identity. Runtime room
construction lowers prepared content through typed, domain-owned construction
lanes under one transaction:

```text
plan -> preflight -> commit -> verify -> publish
```

The room adapter translates world data into domain vocabulary; the domain does
not depend upward on the authored-world spec merely to build itself. Construction
schema metadata is for validation/fingerprinting and must not become a
string/`TypeId` callback registry.

Current room transitions, including the rollback host, consume the prepared
readiness/authorization transaction. The rollback host commits only after
confirmation and then rebases to a new frame-zero baseline; rollback snapshots do
not cross room boundaries.

The old campaign's remaining useful external/P2P proof is trigger-based and is
owned by [`netcode.md`](netcode.md). The old same-room/reset convergence is owned
by [`construction-and-reconstitution.md`](construction-and-reconstitution.md).

Execution history is available in git. Do not reopen a universal construction
enum, custom snapshot engine, or executable type-erased registry because older
planning prose mentions them.
