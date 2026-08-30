# Bevy plugin and reusable crate strategy — durable authority moved

**State:** durable doctrine distilled; apply it from focused plans.

The current package/plugin rules are in
[`../../architecture/package-and-capability-boundaries.md`](../../architecture/package-and-capability-boundaries.md).

For active architecture use:

- [`actor-monolith-decomposition.md`](actor-monolith-decomposition.md) for current
  extraction pressure;
- [`capability-and-runtime-composition.md`](capability-and-runtime-composition.md)
  for optional capability composition;
- [`public-sdk-1.0.md`](public-sdk-1.0.md) for external-consumer pressure.

The short rule is:

1. establish a coherent domain/plugin internally;
2. extract a workspace crate when ownership, dependency or test isolation pays;
3. make it independently consumable only after a real consumer validates the
   public contract.

Do not create crates to improve a package-count metric, and do not promise
frame-time/startup wins from decomposition without current measurements.
