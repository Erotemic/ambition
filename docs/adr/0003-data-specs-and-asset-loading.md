# ADR 0003: Use data specs and embedded fallbacks while asset loading matures

## Status

Accepted.

## Context

Rooms, movement tuning, ability flags, audio specs, and room transitions are moving toward RON data. Bevy asset loading and hot reload are useful, but early patches benefited from embedded fallbacks that kept the sandbox running when asset paths or loaders were wrong.

## Decision

Keep content authoring data-driven where practical, with canonical sandbox data under:

```text
crates/ambition_sandbox/assets/ambition/sandbox.ron
```

Use embedded fallback logic until the Bevy asset-loading path is robust enough to own startup. `bevy_asset_loader` may provide asset collections and future loading states, but it should not remove the fallback path prematurely.

## Consequences

Startup remains reliable during iteration. Future loading-state work can migrate one asset family at a time. Docs should avoid claiming full hot-reload or asset-loader ownership until that is true.
