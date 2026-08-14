# Binding resolution boundary - remaining work

**Status:** residual defects only, re-verified against HEAD on 2026-08-13.

The original binding-resolution campaign landed the core mechanism:
`Ref<N>`, `Resolver<N>`, `Bound<N>`, structured unresolved diagnostics, item-art
bindings, and construction-time refusal for several authored identities. Its full
history, including corrected overclaims, is archived at
[`docs/archive/planning-superseded/2026-08-13/engine/binding-resolution-boundary.md`](../../archive/planning-superseded/2026-08-13/engine/binding-resolution-boundary.md).

Do not reopen a campaign to convert every string ID to the same wrapper. Keep a
binding slice only when it removes a real silent-failure or duplicate-authority
path.

## Remaining defects

### 1. Source-qualify per-frame item-art diagnostics

`ReportedOnce` correctly keys a report by namespace, declarer and id, and clears
when its backing art resource changes. The ground-item presentation path still
passes generic declarers such as `"ground item"` rather than provider/source
identity. Two providers with the same unresolved id can therefore suppress one
another's diagnostic in one process.

Fix the declarer at the call site so a failure names the provider/content source
that authored it. Do not add another global reporting registry.

### 2. Extend failed-file detection beyond item art where invisibility is real

`report_unloadable_item_art` handles the important case that namespace resolution
cannot see: a registered art id whose file never loads. Character sheets, props
or projectile art that can fail in the same invisible way should use the same
principle when there is a concrete silent-failure path.

Prefer a shared small asset-materialization primitive if multiple consumers need
identical polling/failure semantics; do not create a universal asset census.

## Deferred trigger, not current work

`Bound<N>` proves that an id resolved in some authority of namespace `N`; it does
not encode which resolver instance assigned the slot. `SheetRecord::row` therefore
keeps a release `assert!` that rejects a `Bound<AnimRow>` minted by another sheet.

That is adequate while `AnimRow` is the only slot-bearing bound value that escapes
its resolver. Add resolver/authority branding only when a second real namespace
has the same escaping-slot problem. Do not implement the abstraction ahead of
that trigger.

## No standing migration list

Recipe ids, music ids, dialogue ids and move clips each already have domain-specific
registries/validation or lookup semantics. Their use of strings is not by itself a
defect. Promote one to this plan only when HEAD shows a concrete unresolved typo,
ambiguous authority, repeated lookup cost that matters, or silent fallback that a
binding boundary would actually eliminate.
