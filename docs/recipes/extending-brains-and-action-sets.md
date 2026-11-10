---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/systems/actors-brains-and-character-content.md
  - docs/mechanics/abilities.md
---

# Extend brains and action sets

Brains decide intent. Action sets/schemes translate semantic slots and requests
into reusable actions. Body/combat/world systems execute those actions. Keep
those responsibilities separate.

## First decide what is actually missing

```bash
python scripts/agent_query.py "brain action set <desired behavior>"
python scripts/agent_query.py tests "<desired behavior>"
python scripts/agent_query.py crate ambition_characters
```

Classify the change:

- **New tuning/preset:** provider data only.
- **New composition of existing actions:** provider data plus existing brain
  vocabulary.
- **New decision policy:** extend the reusable brain domain.
- **New action meaning/capability:** extend action resolution and the owning
  mechanic domain, then expose it to all controllers.
- **New named character behavior:** keep the name and defaults in provider data.

## Brain rules

A brain consumes the same stable observation/control vocabulary available to
other controllers and emits actor-local intent/action requests. It must not:

- mutate movement/combat/world state directly;
- read presentation-only entities or pixels when a simulation observation exists;
- bypass cooldown/resource/action-scheme gates;
- depend on wall-clock time or unstable entity iteration order;
- encode an Ambition character name in a reusable engine policy.

Prefer deterministic state machines, planners, or policies with explicit memory
that can be reset, snapshotted, and tested.

## Action-set/scheme rules

- Semantic device slots are stable; character-specific meanings are resolved in
  simulation.
- The live scheme derives from body capabilities/equipment/mode/authority.
- The shared resolver produces both execution and `ControlPrompt` meaning.
- Rejection/gating is visible and deterministic.
- Equivalent player/enemy actions execute through one domain seam.

## Implementation loop

1. Add the smallest reusable data or policy variant the gameplay needs.
2. Route the action through the existing body/combat/projectile/interaction
   authority rather than creating a character-specific executor.
3. Register the provider preset and play the behavior in an actual encounter.
4. Check reset, restore, or replay when the new state participates in them.
5. Retain a test only when it protects an important decision invariant or a
   demonstrated cross-controller failure. Prefer extending an existing scenario
   to adding another assembled-app test.

For a targeted check, choose the owner that changed:

```bash
./run_tests.sh -p ambition_characters -k <new_policy>
./run_tests.sh -p ambition_platformer2d_actor_monolith -k <new_action>
```

The examples are alternatives, not a requirement to run both or the entire
workspace suite. See `AGENTS.md` under **Avoid bullshit guardrails**.
