# Current next good moves

Keep this file short and actionable.

Related split:

- [`state.md`](state.md)
- [`risks.md`](risks.md)
- [`next.md`](next.md)

## Current next good moves

1. Fix compile/runtime issues from user logs before adding new features.
2. Convert one enemy to the `seldom_state` path instead of migrating everything at once.
3. Build a small first-level vertical slice rather than only adding isolated labs.
4. Expand tests around room graphs, blink/collision, input buffering, and generated schedules.
5. Add a render/preview lab for procedural visuals before committing to a final style.
6. Keep updating ADRs when decisions supersede older notes.

## Documentation cleanup follow-ups

After this split, the next documentation work should be incremental:

1. Move current subsystem docs into clearer buckets only when updating those docs anyway.
2. Promote durable rules from `dev/` into concept pages, recipes, or ADRs.
3. Add generated `.agent/` indexes and retrieval/localization evals in a later chunk.
4. Keep `docs/brainstorms/` active and linked; do not archive it as cleanup.
