---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/systems/headless-simulation.md
  - docs/concepts/testing-and-validation.md
---

# Gameplay trace recorder

Gameplay traces are structured observations of authoritative simulation facts.
They support debugging, replay comparison, headless acceptance, and agent/human
handoff without making logs part of gameplay authority.

## Ownership and flow

The focused trace domain owns event vocabulary, bounded collection, stable
serialization/dump behavior, and comparison helpers. Simulation domains emit
semantic facts at canonical seams; provider/runtime composition chooses which
traces are enabled and where artifacts are written.

```text
simulation fact
    -> typed trace event
    -> deterministic ordering/bounded buffer
    -> snapshot/dump/report
    -> tests, replay comparison, or diagnostics
```

Use `python scripts/agent_query.py "gameplay trace recorder"` for current crate,
resource, event, and system names.

## Trace design rules

- Emit once at the authoritative transition, not once per presentation consumer.
- Prefer stable provider IDs and semantic values over debug strings or raw Bevy
  entity numbers.
- Include enough context to prove non-vacuity and explain rejection/outcome.
- Ordering that matters is explicit and deterministic.
- Collection may be disabled without changing simulation.
- Dumps are versioned or tolerant enough for intentional schema evolution.
- Avoid tracing every frame when a state transition or aggregate suffices.

## Tests

A trace test should first prove the scenario occurred, then compare relevant
semantic events. Exact full-trace equality is a canary and may be re-baselined
when intentional pre-release behavior changes; invariant assertions are stronger.

```bash
python scripts/agent_query.py tests "gameplay trace"
./run_tests.sh -k trace
./run_tests.sh -k replay
```
