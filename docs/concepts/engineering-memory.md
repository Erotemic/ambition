---
id: engineering-memory
aliases:
  - dev memory
  - journals
  - benchmark candidates
  - lessons learned
implemented_by:
  - dev/README.md
  - dev/SEARCH.md
  - dev/journals/
  - dev/benchmark-candidates/
last_verified: 2026-09-03
---

# Engineering memory

## Definition

Engineering memory is Ambition's long-running record of hard-won lessons. It lives in `dev/`, not `docs/`, because it is evidence and lookup memory rather than current architecture documentation.

## Core invariants

- `dev/journals/` is for symptom-driven postmortems.
- `dev/benchmark-candidates/` is for invariant traps and hard questions distilled from real mistakes.
- Agents should search `dev/` before non-trivial code changes.
- Durable rules discovered in `dev/` should be promoted into concept pages, recipes, or ADRs.
- ⛔ **THIS INVARIANT IS VACUOUS AS WRITTEN AND HAS BEEN SINCE IT WAS WRITTEN.**
  It reads *"do not duplicate canonical lessons between
  `dev/journals/lessons_learned.md` and `dev/journals/lessons_learned.md`"* — the
  same path twice, so it forbids nothing. Present in `ecc107fb9`, the commit that
  created this knowledge base, and unchanged since; there is exactly one
  `lessons_learned.md` in the tree, so the second name was never a real file.
  ⇒ **Flagged rather than rewritten, because guessing an invariant is worse than
  an obviously broken one.** The reading the rest of this page supports is *"do
  not duplicate between `dev/journals/lessons_learned.md` and
  `dev/benchmark-candidates/`"* — the two collections defined above, one for
  postmortems and one for distilled invariant traps, which are exactly the pair a
  lesson could land in twice. ⚠ That is a RECONSTRUCTION from context, not the
  original intent recovered; it wants a maintainer's yes before it becomes the
  rule. Note it cannot mean "between `dev/` and concept pages": promoting durable
  rules into concept pages is the invariant two lines down.

## Edit protocol

1. Add a journal when the diagnosis took real effort and future symptom search will help.
2. Add a benchmark candidate when the failure is a transferable invariant another agent could miss.
3. Update indexes so future agents can find the entry.
4. Promote stable rules to `docs/concepts/` when they become current project policy.

## Validation

```bash
rg -n "<symptom>|<failure class>" dev/journals dev/benchmark-candidates
```

The test is findability: future agents should be able to rediscover the memory by the words they would naturally search while stuck.
