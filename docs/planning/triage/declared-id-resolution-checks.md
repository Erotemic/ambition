# Declared-id resolution — remaining authoring diagnostics

> **Verified against `cecd01ca` (2026-08-13).** The original silent-resolution
> triage is mostly implemented. The complete investigation is archived at
> [`../../archive/planning-superseded/2026-08-13/triage-declared-id-resolution-checks.md`](../../archive/planning-superseded/2026-08-13/triage-declared-id-resolution-checks.md).

## What already exists

The shipped host now checks declared world-item art, projectile image paths, and
music paths against real files in `game/ambition_app/tests/declared_art_resolves.rs`.
Character art and summoned-character/body resolution have their own composed-host
checks, construction reports unresolved refs with provenance, and several runtime
resolvers already explain/report misses rather than silently treating them as
"feature absent".

Do not recreate the old boot-time validation proposal or duplicate these tests.

## Remaining work

1. **Manifest-backed catalog targets.** Identify any still-live catalog `manifest`
   declarations whose target can name a missing generated asset without a useful
   preparation/authoring diagnostic. Add the check at the authoring/preparation
   boundary that owns that declaration, not as a generic startup sweep.

2. **Runtime-composed misses.** At concrete resolver call sites that still discard
   an unexpected `None`, make the miss observable once with the resolver's own
   provenance/explanation. Re-measure before changing anything; the July count of
   "roughly six" sites is stale.

3. **Typed/generated ids only when the owning pipeline is already open.** If the
   sprite/content pipeline naturally exposes stable generated symbols, use them
   to make impossible references unrepresentable. Do not open a standalone
   symbol-generation campaign merely to replace strings.

## Exit

A newly declared asset/reference that is absent by mistake fails authoring or is
reported with actionable provenance, while intentionally absent optional content
remains valid. No always-on boot census is added.
