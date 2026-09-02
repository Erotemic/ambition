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

⛔⛔ **THE FIX AS WRITTEN IS NOT IMPLEMENTABLE, MEASURED 2026-09-02.** "Fix the
declarer at the call site" assumes provider identity is reachable there. It is
not, on either side of the join:

- **the art side** — `WorldItemArtEntry` is `{ sprite_id, asset_path, size }`
  and `HeldItemArtEntry` matches it. Providers construct them with exactly those
  three fields (`ambition_demo_mary_o/src/provider.rs:151,161,174`), and
  `WorldItemArtManifest::effective()` is a LAST-WINS merge keyed by `sprite_id`,
  so not even the winning entry records who contributed it;
- **the content side** — `GroundItemFact` is `{ pos, half_extent, item_id }`
  (`ambition_sim_view/src/facts.rs:215`). The renderer iterates
  `GroundItemsView` and holds the id and nothing else.

⇒ Qualifying the declarer with the ID buys nothing: `ReportedOnce` already keys
by (namespace, declarer, id), so the id is in the key. Only PROVIDER identity
separates two providers' reports, and it does not exist to be named.

A real fix is a design choice between two DIFFERENT questions, and this row does
not say which it wants:

1. **attribute the ART** — a source on `WorldItemArtEntry` / `HeldItemArtEntry`,
   carried through `effective()`. Answers "whose art binding is missing", and
   incidentally makes the last-wins merge auditable: today one provider can
   silently override another's sprite and nothing records it;
2. **attribute the CONTENT** — the declaring source on `GroundItemFact`. Answers
   "whose level authored an item with an unbound id", which is closer to this
   row's wording.

⚠ **AND SETTLE THE PRIOR QUESTION FIRST.** This row asserts the suppression is a
defect but cites no case where a real diagnostic was lost. Two providers failing
on one id may be ONE authoring defect seen twice, in which case reporting it once
per process is correct and there is nothing here to fix. Find the case before
building either shape.

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
