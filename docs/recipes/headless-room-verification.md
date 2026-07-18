---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/systems/headless-simulation.md
  - docs/concepts/testing-and-validation.md
---

# Headless room verification

Use the real provider/runtime construction path. A hand-built `World` that skips
loading, lifecycle, control, or lowering can prove a unit invariant, but it does
not prove the assembled game.

## Localize the existing seam

```bash
python scripts/agent_query.py "headless room load provider"
python scripts/agent_query.py tests "<room id or behavior>"
python scripts/agent_query.py crate ambition_sim_harness
```

Prefer extending an existing harness/helper over adding a second miniature app.

## Verification ladder

1. **Pure/domain test** — prove the narrow transformation or policy.
2. **Owning-crate assembled test** — construct the smallest real plugin/domain
   composition.
3. **Provider/runtime test** — prepare the provider, load the room through the
   transaction, step simulation, and observe stable read models/traces.
4. **Geometry render** — inspect authored collision/entities without a GPU.
5. **Visible smoke** — reserve for presentation feel that cannot be inferred
   from authoritative state.

## Commands

```bash
python scripts/agent_query.py tests "<invariant>"
./run_tests.sh -k <test_substring>
./run_tests.sh -p <owning_package> -k <test_substring>
cargo run -p ambition_actors --example render_room_geometry -- <ROOM_ID>
```

`game/ambition_app` integration tests are aggregated through the current
`app_it` test surface; do not invoke deleted historical test binaries. Let
`./run_tests.sh` and `agent_query.py tests` choose the current package/test shape.

## What to assert

Assert durable outcomes:

- provider/session/room construction commits atomically;
- expected stable IDs resolve and scoped entities exist;
- the actor receives semantic control and uses the shared body/action path;
- movement, interaction, hit, transition, or progression facts occur;
- cleanup/reset removes the old scope and derived views converge;
- gravity rotations or equivalent symmetries preserve the mechanic;
- the test is non-vacuous (the target event/state was actually reached).

Avoid pinning exact frame counts, velocities, coordinates, or visual assets unless
they are the contract under test.
