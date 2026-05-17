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
last_verified: 2026-05-17
---

# Engineering memory

## Definition

Engineering memory is Ambition's long-running record of hard-won lessons. It lives in `dev/`, not `docs/`, because it is evidence and lookup memory rather than current architecture documentation.

## Core invariants

- `dev/journals/` is for symptom-driven postmortems.
- `dev/benchmark-candidates/` is for invariant traps and hard questions distilled from real mistakes.
- Agents should search `dev/` before non-trivial code changes.
- Durable rules discovered in `dev/` should be promoted into concept pages, recipes, or ADRs.
- Do not duplicate canonical lessons between `dev/journals/lessons_learned.md` and `dev/journals/lessons_learned.md`.

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
