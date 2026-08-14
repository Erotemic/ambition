# Public API — remaining work

> **Verified against `cecd01ca` (2026-08-13).** The API 1.0 campaign's slices
> A–G and the first optional-capability cut are implemented. The full campaign
> record is archived at
> [`../../archive/planning-superseded/2026-08-13/engine/api-1.0-campaign.md`](../../archive/planning-superseded/2026-08-13/engine/api-1.0-campaign.md).

This file contains only work that remains. The durable method for growing the
public surface is [`../../concepts/api-growth.md`](../../concepts/api-growth.md).

## 1. Finish optional-capability closure

The facade already has `default = ["all_capabilities"]` and optional capability
dependencies. Re-measure the facade-only transitive closure and remove remaining
capability leakage caused by internal composition edges, especially edges pulled
through the actor monolith/runtime. Keep the easy all-capabilities default.

`ambition_audio` is still intentionally unconditional today; change that only if
a real consumer benefits from excluding it.

## 2. Make rollback participation one declaration

The current registration surface can still declare a rollback component/codec
without making entities carrying it actually participate in rollback; callers
also use `require_rollback`. Replace that two-step possibility with a declaration
shape where an inert rollback registration is unrepresentable, without pushing
GGRS knowledge into every domain crate.

This work should align with the post-D73 rollback ownership inversion campaign,
not create a second registry architecture.

## 3. Prove the facade from another real consumer before freezing more API

`fixtures/external_consumer` already proved the first external-workspace slice.
Before treating additional construction/content APIs as stable, exercise them
from another meaningfully different consumer or from an in-repo game that uses
only the supported facade. Improve the API only where the consumer exposes real
friction.

## Exit

- a minimal consumer links only the capabilities it selected, modulo explicitly
  documented always-on substrate;
- rollback state cannot be registered-but-inert by construction;
- a second consumer can author/use the relevant engine capability without
  importing internal implementation topology;
- no blind-agent run, source-text allowlist, or evidence bundle is required just
  to declare the campaign complete. Those were historical discovery tools.
