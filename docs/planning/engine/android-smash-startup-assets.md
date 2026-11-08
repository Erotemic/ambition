# Android Smash startup asset demand

**State:** OPEN — selective baked-page loading is implemented in this packet. The cause of the full Android image-demand burst is still open.

**Scope:** Android Smash startup, image decode, materialization, and memory use. The established lifecycle contract is in [asset preparation and residency](asset-preparation-and-residency.md). This document is a standalone investigation and implementation plan. Do not merge it into a shared task queue while other agents update that queue.

## Evidence and target

The 2026-11-08 four-CPU Android autoplay capture (`android-all-smash_4p-auto-20261108T221326Z`) reported:

- 855 decoded images and about 1.49 GB of reported image bytes by 26 seconds after engine startup.
- 761 of those images had not been drawn at that point. The image report attributed about 1.36 GB to that population.
- 609 decoded images were classified as character-sheet images. This is far more than the four CPU seats need as distinct fighter identities.
- The early match included frame times of about 481 ms and 991 ms. The 35-second profiler warmup missed much of this period.
- After warmup, the 60-second capture averaged about 49 FPS. This is a different measurement window from cold startup.

The counters above describe the capture's reported decode and draw activity. Verify live residency and ownership before treating the reported image-byte total as simultaneous GPU memory use. An image that has not been drawn may still be needed by a planned animation or by a different active presentation owner.

**Goal:** Reduce requests, decode, and device materialization for image products that the current Smash session does not need. Keep simulation behavior, animation row selection, quality transitions, and visual readiness correct.

## Implemented: pages for hybrid rigged/baked characters

A character can draw some rows from part textures and other rows from baked sheet pages. Before this change, the character materializer loaded the baked pages referenced by every row even when the runtime flipbook drew most rows from parts. The loading rule now selects the pages used by baked rows in a mixed realization. A fully baked realization still loads all pages used by its frames. A fully part-drawn realization retains its existing parts-only path.

The runtime keeps all page slots for the animator but leaves omitted baked pages without image handles. Room readiness now includes the baked pages required by a mixed character **and** its part pages. Tests cover page selection across frames, a mixed realization, and refusal of an incomplete baked realization.

This optimization can only remove a baked image when no baked row shares its page. It may save no images for a one-page hybrid. Do not credit the full Android image burst to this path without a road-and-owner census.

## Next work, in priority order

### A1 — Find the image-demand owners

**Why:** Four CPUs cannot explain hundreds of independently demanded sheet images without tracing the callers or a large number of pages per sheet.

**Inspect:** `image_decodes.csv`, `image_arrivals.csv`, `asset_activity.csv`, and the stage demand ledger. Join image path/ID, first request, road, owner/character, requested tier, resolved tier, first draw, and live residency. Inspect `demand_rostered_character_sheets`, `materialize_character_demand`, room prefetch, core FX, and any asset-server loads outside the character materializer.

**Deliverable:** A ranked table of distinct image products by decoded bytes and no-first-draw count, with the first requesting call site and the active residency owner. Separate character-sheet, character-parts, FX, LDtk/editor, background, and unknown loads.

**Acceptance:** Every major 100-MB-or-greater population has a named demand owner or a specific missing tracing field. The report must distinguish a one-time decode from a still-resident image.

### A2 — Restrict demand at the semantic boundary

**Why:** A startup request for an unrelated character or room is a demand defect. Fix it before building streaming or eviction.

**Implementation:** If A1 finds gallery-wide or global roster preloads, change the owning demand producer to request only current seat characters, stage needs, current presentation FX, and the selected quality tier. Preserve the prepared catalog and declarations; do not decode every declared character. Use the existing materializer and room manifest. Add a test with many registered characters but only four active seats; assert that only owned characters produce image requests.

**Acceptance:** Non-rostered, non-visible characters produce no baked-sheet or part-image load requests in a fresh Smash session. Re-entry, quality changes, and room transitions still re-demand correctly.

### A3 — Reduce unused part-page requests

**Why:** A rigged flipbook currently requests every published part atlas page. Some may be unreachable from the rows that a given presentation uses, or may be in a large shared product.

**Implementation:** Measure page references through `RiggedSpriteAsset` draws and clips, then add a page subset policy only if unused pages are confirmed. Keep the flipbook part-page indices stable. Use the same readiness and residency rules as baked pages. If shared part pages are a single indivisible publication unit, plan a runtime product split rather than introducing an index-rewrite workaround.

**Acceptance:** The part renderer never reads an absent page. Tier fallback, part/baked transitions, and hot reload remain correct.

### A4 — Cold-start profile and comparison protocol

**Why:** A 35-second warmup mostly measures the recovered game, not the observed startup lag.

**Procedure:** Use the Android CLI autoplay launch for each fresh run, with `--warmup 0` (or the script's smallest supported warmup) and enough capture time for the startup-to-live transition. Keep the installed APK playable. Record app start, first presented frame, first live match, first-demand/first-decode/first-draw, 1-second frame-time distributions, main/process CPU time, memory, thermal state, and file/package revision. Repeat one steady-state capture using the established warmup to avoid mixing phases.

**Acceptance:** A before/after pair on the same device and quality settings reports startup lag duration, peak 1-second p95 frame time, decoded images, decoded bytes, live image residency, and mean CPU-seconds per wall-second. No speculative speedup claim from a desktop-only run.

### A5 — Pace materialization, then consider streaming

**Trigger:** A1/A2 show that the remaining early image requests are necessary and A4 shows that decode/upload concurrency still causes visible frame spikes.

**Design:** Reuse the existing declared → prepared → device-materialized → resident stages. Put budgets on expensive decode/upload completion, not on semantic registration. Keep a usable placeholder or prior tier until a required representation is ready. Never change simulation or GGRS rollback based on render readiness. Define explicit owner lifetime before any eviction policy.

**Acceptance:** No gameplay-dependent divergence; required images appear by first use or the presentation fallback is documented and tested. Loading is bounded without a cache that retains unrelated sessions indefinitely.

## Scope limits

Do not rebuild the asset system solely to solve the four-CPU startup test. Do not add a generic LRU or per-frame global catalog scan. Do not require every packaged image to be loaded, and do not treat the 3.8-GB APK's stored assets as equivalent to live RAM. Package composition belongs in its existing runtime-product planning track.

The current sprint can close after the selective-page patch passes its Rust and Android checks and A1 produces a demand-owner explanation. Proceed to A2 only when A1 identifies a concrete redundant demand path. Keep A3–A5 as conditional work.
