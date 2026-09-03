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

1. ✔ **Manifest-backed catalog targets — DONE 2026-08-26, and the gap was
   exactly one field.** A catalog row declares TWO files and only one of them was
   checked: `every_catalog_character_names_a_spritesheet_that_exists` pinned the
   PIXELS, and nothing asked about `manifest` — the `.ron` beside them carrying
   every frame rect, anchor and clip. A row naming a manifest that is nowhere has
   no geometry to draw the pixels with, which is the worse failure.
   `every_catalog_character_names_a_manifest_that_exists` is its sibling, in the
   same file, with the same ratchet shape and a premise guard on the row count.
   ⛔ NOT a startup sweep, and not in the catalog validator: that validator lives
   in an engine crate with no filesystem knowledge, and asset existence is a
   COMPOSITION question — which is why the composed-host asset test is the
   boundary that owns this declaration here.
   ⚠ same stated limit as its sibling: generated art is gitignored, so it catches
   the TYPO and cannot answer the fresh-clone question. Measured at the time:
   260 declared paths across the shipped catalog, 0 missing.

2. ✔ **Runtime-composed misses — CLOSED 2026-08-28. Re-measured twice as this
   item asked, and the shape it proposes a diagnostic for is still absent.** Every `Option`-returning resolver in
   `crates/` (14 of them, by `fn *resolve*` returning `Option`) was checked at its
   call sites, and the ones that could discard a miss do not:

   ```text
   resolve_encounter        `let Some(..) = .. else { .. }`   explicit branch
   resolve_surface          four call sites, all `let-else` or `?` on a path
                            whose absence is the answer
   resolve_key              `.or_else(..)` — a fallback CHAIN, which is the
                            resolver's own explanation
   resolve_active_route     no route means no strike; absence IS the value
   resolved_track_handle    `if let Some(..)` — a track not in the library is not
                            a startup dependency, and the music registry is
                            GENERATED so a typo cannot survive to this point
   ```

   ⛔ **AND THE SILENT-DISCARD SHAPE THE ITEM NAMES IS ABSENT:** grep finds NO
   `resolve*(..).unwrap_or_default()` and no `resolve*(..).unwrap_or(..)` in the
   tree.

   ⚠ **so do not build the diagnostic this item proposes without first naming a
   site that needs it.** The July "roughly six" was a count of a pattern the code
   has since stopped using; re-run the same two greps before reopening.

   ⭐ **2026-08-28, the second re-measure, and it is the one that closes this.**
   The resolver population GREW — 42 `Option`-returning resolvers now, up from 14
   — and the silent-discard grep is STILL empty across `crates/` and `game/`. That
   is the interesting result: a growing population with no new instances of the
   defect means the shape is not being reintroduced, so the diagnostic has no
   customer and this item is not "unfinished", it is ANSWERED. ⛔ Reopen only on a
   named site, never on the count.

   ⚠ **Re-ran both greps 2026-09-02: the closure holds.** The silent-discard
   shape is still ABSENT across `crates/` and `game/` — zero
   `resolve*(..).unwrap_or(..)` and zero `.unwrap_or_default()` — and both guard
   tests cited in item 1 still exist in `declared_art_resolves.rs`. Deliberately
   not restating the resolver count: a multi-line-aware scan of `fn *resolve*`
   returning `Option` counts definitions and gives a different number than the
   pass above, and per the line directly above, the count is not what reopens
   this.
   ⭐ **RE-RUN 2026-09-03, exactly as this item instructs — and the conclusion
   holds while one of its supporting numbers does not.**
   ✔ **The load-bearing result reproduces:** both silent-discard greps are still
   empty across `crates/` and `game/` — no `resolve*(..).unwrap_or_default()`,
   no `resolve*(..).unwrap_or(..)`. The shape is still not being reintroduced,
   so the item stays ANSWERED.
   ⚠ **But the resolver POPULATION figure is instrument-dependent and should not
   be leaned on.** Counting `fn *resolve*` returning `Option` gives a different
   answer for every way of asking: **12** matching on one line, **14** with a
   3-line grep window, **29** with an 8-line window, and **34** (28 distinct
   names) from a scan that reads each signature to its body brace while tracking
   paren depth. The recorded trend "14 → 42" therefore compares two numbers that
   may not have come from the same instrument, and no consistent instrument
   reproduces either endpoint today.
   ⇒ **This is evidence FOR this item's own closing rule**, not against it.
   *"Reopen only on a named site, never on the count"* is exactly right: the
   count moves under the measurer's `-A` flag, and the named-site test does not.
   The growth argument was never the reason this item closed — the empty grep
   was.

3. **Typed/generated ids only when the owning pipeline is already open.** If the
   sprite/content pipeline naturally exposes stable generated symbols, use them
   to make impossible references unrepresentable. Do not open a standalone
   symbol-generation campaign merely to replace strings.

> **RE-VERIFIED against `a83a40c1f` (2026-09-03) — nothing to change, and that
> is the finding.** Both closed items name tests, and a ✔ is only worth its ink
> if the test it points at still exists AND still runs. All three named artefacts
> are present at HEAD — `game/ambition_app/tests/declared_art_resolves.rs`,
> `every_catalog_character_names_a_spritesheet_that_exists` and
> `every_catalog_character_names_a_manifest_that_exists` — and both tests carry a
> bare `#[test]` with no `#[ignore]`, so they run in the default gate rather than
> only under `--include-ignored`. ⇒ Items 1 and 2 stay ✔ on evidence, not on
> memory. Item 3 is a standing principle with nothing to measure.

## Exit

A newly declared asset/reference that is absent by mistake fails authoring or is
reported with actionable provenance, while intentionally absent optional content
remains valid. No always-on boot census is added.
