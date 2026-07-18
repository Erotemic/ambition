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

1. Add the smallest reusable data/policy variant.
2. Add pure tests for decision or resolution.
3. Route the request through the canonical body/combat/projectile/interaction
   seam—never a new character-specific executor.
4. Register provider presets/data.
5. Add an assembled test with at least two controller/body kinds when
   unification is the invariant.
6. Verify reset/snapshot/replay behavior.

```bash
./run_tests.sh -p ambition_characters -k <new_policy>
./run_tests.sh -p ambition_actors -k <new_action>
./run_tests.sh -k action_scheme
```
