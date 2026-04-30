# AI Generation Contract

Ambition should be AI-first without allowing arbitrary generated code to destabilize the engine.

The engine should execute validated symbolic specs, not raw unchecked AI-written gameplay code.

## Good generated content

```ron
MovementTheorem(
    name: "Exponential Lift",
    prerequisites: ["Vector Dash"],
    input_pattern: ["jump", "hold_up", "dash"],
    curve: Exponential { base: 1.08, duration: 0.65, max_velocity: 900.0 },
)
```

```ron
RoomSpec(
    seed: 184467,
    theme: TangentSpace,
    tests: ["dash_pogo_order", "wall_bank", "recoverable_spike_channel"],
)
```

## Validation rules

Generated specs should be:

- deterministic from seed + spec;
- bounded;
- serializable;
- inspectable;
- reachable/beatable according to automated tests;
- mechanically distinct from existing content;
- readable to a human player.

## Hard boundary

AI may generate:

- room specs;
- shape specs;
- motion curves;
- music motifs;
- enemy behavior specs;
- tutorial text;
- test cases.

AI should not directly mutate core Rust engine code during normal content generation.
