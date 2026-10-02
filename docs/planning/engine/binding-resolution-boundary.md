# Binding resolution boundary - remaining work

**Status:** residual defects only.

The original binding-resolution campaign landed the core mechanism:
`Ref<N>`, `Resolver<N>`, `Bound<N>`, structured unresolved diagnostics, item-art
bindings, and construction-time refusal for several authored identities. Git
history has the full record.

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

Provider identity is not reachable at the call site. `WorldItemArtEntry` and
`HeldItemArtEntry` are `{ sprite_id, asset_path, size }`, and
`WorldItemArtManifest::effective()` is a last-wins merge keyed by `sprite_id`.
`GroundItemFact` is `{ pos, half_extent, item_id }`. A fix is one of two
different choices:

1. attribute the art: a source on the art entries, carried through
   `effective()` (this also makes the last-wins merge auditable);
2. attribute the content: the declaring source on `GroundItemFact`.

First find a real case where a diagnostic was lost. Two providers failing on one
id can be one authoring defect, and then one report per process is correct.

Do not add another global reporting registry.

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

## Prepared resolution and runtime authority

Explicit binding resolution separates known support from installed support. A11 couples the
technique support declaration to its actual handler installer; a validator lookup
alone cannot establish either existence or availability.

Bindings prepared against one immutable content revision must not resolve through
an unrelated later mutable catalog during simulation. Stale prepared work needs
revision/scope rejection at activation. Do not generalize this into an executable
service locator or make display names stand in for semantic identity.
