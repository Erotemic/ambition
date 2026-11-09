# Android Smash startup and CPU efficiency - agent handoff (2026-11-08)

**Status:** OPEN. Android autoplay profiling works. The high-volume character-sheet demand during Smash startup has been reproduced, and the first targeted image-loading change did not reduce that population. **Next action: identify who first requests the unrelated sheets, then remove that demand at its source.**

**Owner:** Next available optimization agent. This is a self-contained handoff, separate from shared queues. Coordinate with ongoing changes before editing the same source files. The existing investigation plan is [`android-smash-startup-assets.md`](android-smash-startup-assets.md); the broader architectural contract is [`asset-preparation-and-residency.md`](asset-preparation-and-residency.md). Do not require either document to understand the tasks below.

## 1. Goal and boundaries

The user's objective is to reduce **actual Android CPU use, energy, peak image memory and time spent stuttering at match startup**, while preserving normal interactive Smash gameplay. Desktop profiles are useful development proxies, but use Android evidence to assess mobile savings.

- Keep the installed Android APK normally playable. Autoplay is selected only for a one-shot CLI launch (`AMBITION_AUTOPLAY=smash_4p` with `AMBITION_BOOT_ROUTE=smash_select`); never bake autoplay into a general APK.
- Keep authored animation behavior, quality-tier fallback, image page indices, presentation readiness, and deterministic GGRS simulation intact. Do not make simulation depend on GPU/asset readiness.
- Prefer eliminating semantic demand for unneeded image products before deferring or throttling those same requests. Keep asset registration cheap and hot paths free of routine integrity validation.
- Do not introduce an LRU, global cache scanner, broad compatibility layer, or new streaming architecture without evidence that remaining required images need one.
- Do not modify generated assets or merge this handoff into a task queue concurrently maintained by another agent.

## 2. Evidence ledger - three Android capture artifacts

Stored as local archives in the user's checkout under `dev/ambition_dev_measurements/profiles/` (share/upload them with the agent when practical):

| Capture archive (basename) | Role | Sampling window | Autoplay outcome |
| --- | --- | --- | --- |
| `android-all-smash_4p-auto-20261108T221326Z.tar.gz` | Pre-selective-page comparison | 35 s warmup + 60 s CPU capture | Live match was recorded; old validator incorrectly returned `no-live-marker` |
| `android-all-smash_4p-auto-20261108T230722Z.tar.gz` | After selective-page patch; cold-ish startup | 0 s warmup + 90 s CPU capture | `live` |
| `android-all-smash_4p-auto-20261108T233358Z.tar.gz` | After selective-page patch; warmed capture | 35 s warmup + 60 s CPU capture | `live` |

The two newer archives report source revision `64181c88b6d6` in the prior review; check the *current* checkout and capture metadata before using that identifier for a binary comparison. Both new runs used the same Android code, `smash_4p`, four level-3 CPUs, and 40 profiling stocks. **The four CPU seats on Android alternate two character identities** (see `profile_roster` in `game/ambition_demo_smash/src/autoplay.rs`), unlike the desktop mixed-character benchmark.

Observed image census from the archived logs/reports:

| Metric | Pre-patch profile | New profiles |
| --- | --- | --- |
| Character-sheet images decoded | 609 | 609 in both |
| Character-sheet decoded area | ~305.9 megapixels | ~305.9 megapixels |
| Character-sheet images reported as ever drawn in census | 0 | 0 |
| Total images after broad loading burst | ~855 | ~859 in warmed run |
| Reported decoded image bytes after burst | ~1.49 GB | ~1.49 GB |

The `609 * 4`-byte pixel-area estimate is roughly 1.22 GB. These are image-census decode/ownership numbers, **not** measured instantaneous GPU memory or process RSS; do not equate APK size, decoded bytes, handles and GPU residency.

The latest warmed run's `game-log.stdout` includes actual runtime lines for unrelated room products on the `character-sheet` road, including:

```text
[image] ... content://ambition_shared/sprites/bakery_room_spritesheet.png ... via character-sheet ... DECODED DURING GAMEPLAY
[image] ... content://ambition_shared/sprites/eureka_observatory_room_spritesheet.png ... via character-sheet ... DECODED DURING GAMEPLAY
[image] ... content://ambition_shared/sprites/sky_lookout_deck_spritesheet.png ... via character-sheet ... DECODED DURING GAMEPLAY
```

These entries span multiple seconds (e.g. sky lookout reports `demand->insert 3060ms`); that interval measures demand-to-insertion latency, not necessarily a continuous 3-second GPU stall. This evidence establishes **broad, unrelated room-sheet demand during Smash**; the *producer* of that demand still needs tracing.

Other timing observations:

- The earlier startup had 481 ms and ~991 ms frame spikes during broad loading. The new captures still show substantial opening-period stalls. A warmed run shows an approximately 24.8-second frame interval spanning an **Android suspend/resume**, so exclude that interval from game CPU/stall attribution.
- The original 60-second post-warmup CPU capture averaged about 49 FPS and approximately two aggregate CPU-cores over its window. Later runs used different warmups and one was suspended; do not assign the CPU difference to the loading patch.
- The native simpleperf library frames are largely unsymbolized. Symbolization with the matching unstripped `libambition_app.so` remains open.
- A `--warmup 0` autoplay capture still waits for match readiness before attaching its native CPU sampler. Logcat covers earlier startup, but **simpleperf does not yet measure process launch through pre-live initialization**.

**Strong result:** The hybrid baked-page optimization is a valid localized change, but it did **not** reduce the 609-image character-sheet population in these captures. Do not repeat that optimization as the solution to the global burst.

## 3. Work already implemented (verify in CURRENT checkout)

- Fighter AI/combat allocation cleanups: avoid idle attack-kit reconstruction, redundant perception clones, premature geometry for ineligible strike targets, and sheet-record clones in attack geometry lookup. The sheet clone/drop hotspots disappeared from the native desktop samples.
- Faster page-local atlas indexing and a switch to disable the background sprite-sheet watcher. Android defaults to watcher-off; retain that power-saving behavior.
- Reproducible desktop CPU-match profiles with 40 stocks, uncapped frame pacing, optional mixed roster/debris toggles, and frame-wait/focus diagnostics. Desktop 4-CPU runs exceeded 300 FPS in several *focused* captures. A ~60-Hz plateau in some other captures is consistent with focus/presentation throttling; do not treat it as proven physics cost.
- Android CLI `smash_4p` autoplay, 40-stock real match, live-readiness checks, and one-shot launch environment cleanup. The `no-live-marker` false failure was fixed, and both subsequent Android archives report `live`.
- Selective baked-page demand for hybrid rigged/baked characters plus readiness tests, in `ambition_sprite_sheet`, `ambition_platformer2d_assets`, and `room_transition_assets.rs`. This can avoid baked pages used only by part-rendered rows; it is not the root fix for the Android full-roster/room demand.

These entries describe work from this sprint, not a guarantee that every prior Rust test passed. The source snapshot used for earlier reviews (`ambition-agent-source-20261108T153646Z-8976daf1c089.tar.gz`) predates several overlays. **Read the actual current repository HEAD, status, and diffs before changing anything.**

## 4. Immediate assignment: attribute and remove excess image demand

### P0 - Trace the first request of the unrelated sheets

**First files/code locations:**

- `crates/ambition_asset_manager/src/image_stages.rs`: existing demand-stage accounting, `note_demand`, road labels, first-arrival/decode observability.
- `crates/ambition_platformer2d_assets/src/character_sprites/assets.rs`: sheet/part materialization, `load_sprite_pages`, shared-pack and per-target demand roads.
- `crates/ambition_platformer2d/src/world_flow/room_transition_assets.rs`: room-manifest demand, `materialize_character_demand`, and `prefetch_neighbor_room_preparation_system`.
- `game/ambition_demo_smash/src/autoplay.rs`: roster construction and match activation.
- Search for `demand_rostered_character_sheets` and `materialize_character_demand` in the current checkout; review callers and activation conditions, not just definitions.

**Important candidate, not a conclusion:** `prefetch_neighbor_room_preparation_system` can prepare **up to four neighbor rooms**. It may demand the frozen session cast for each prepared room. Check if it even runs in the Smash composition and whether that mechanism can account for unrelated room sheets. A four-room neighbor budget alone does not prove why hundreds of character sheets load.

**Proposed tracing:** At the point where a demand is *first registered* (before `AssetServer::load` and prior to decode), stamp a small semantic `requester`/`scope` identity, with current room ID, selected cast/roster ID, image path or stable product ID, quality tier, and exact callsite/road. Aggregate once per unique product/requester instead of logging once every frame. Keep prefetch distinct from visible-room, active-roster, FX and global-registration demand. Retain existing `[image]` accounting, and join it to first request, first decode, first draw and owner lifetime.

**Proof artifact:** One ranked table (CSV + concise summary) with columns:

```text
product_id,path,road,first_requester,scope_or_room,quality_tier,
first_request_time,first_decode_time,first_draw_time,decoded_bytes,
currently_owned_or_resident,reason_required_for_smash
```

Report counts/bytes for active fighters, active stage, legitimate shared FX/UI, neighbor prefetch, global catalog/default initialization and unknown. Explain the origin of the `bakery_room_spritesheet.png` request in particular. Avoid conflating a declared catalog entry with an image handle that was actually loaded.

**Done when:** The large `character-sheet` population has specific first-demand producers, and each unexplained >100-MB group has a testable call path or a named missing instrumentation field. Report whether the 609 images are 609 independently demanded products or a smaller number of multi-page products.

### P1 - Correct the owning demand producer, with a regression test

**Only after P0 identifies the cause.** If a global roster/cast preload leaks into active Smash, request only seats/current match presentation plus actually required stage/FX resources. If a neighbor-room preparation road is implicated, make its speculative demand conditional on a corresponding live navigable world, rather than disabling room preparation globally. Reuse existing demand/materialization ownership and readiness APIs.

Preserve catalog registration and asset publication, which must remain available for later characters, rooms and editor workflows; the optimization should avoid unnecessary decode/materialization, not delete definitions or break subsequent re-demand.

**Regression scenarios:**

1. Register many unrelated characters/rooms; start a four-CPU Smash match; assert no `AssetServer::load`/materialization for unrelated baked or part pages.
2. Assert all actual CPU cast art, stage art and required FX/portraits are ready, including quality fallback and the active character's baked/rigged hybrid animations.
3. Change room/session/quality context and verify that newly needed products re-demand correctly. Retain prefetch coverage for legitimate room traversal.
4. Compare `image` census by cause, decoded count, bytes and time-to-live on the same Android device. The target is *fewer requests and fewer decoded bytes*, not just better frame averages.

### P2 - Make the startup sample cover launch-to-live

The current `scripts/measure/autoprofile_android.sh smash_4p --warmup 0` is useful but not a true launch-to-live native CPU trace. Implement or add a **separate opt-in cold-launch mode** to `scripts/measure/profile_android.sh` that starts the relevant tracing as soon as Android launches/assigns the PID and also retains early logcat. Record timestamps for process launch, first frame, `smash-autoplay LIVE`, request/decode/upload bursts, app suspend/resume, and steady match. Keep the existing `--warmup 35` steady-state mode unchanged. Ensure the one-shot launch env is removed and the installed game remains normal when launched from its icon.

Do not require root access. If `simpleperf` cannot attach early enough on the target device, document the earliest supported attach point and use available ADB/trace instrumentation for the pre-attach interval.

### P3 - Native CPU symbols and sustained power

Use the archive's `symbol-candidates*` files and the **exact Build ID** of the APK's unstripped `libambition_app.so` to symbolize the recorded `perf.data`; avoid mismatching recent host builds with an older APK. Rank CPU work by actual function/module and distinguish application code from Bevy executor, allocator, asset decode, driver and rendering.

For Android CPU/power, track average CPU-seconds per wall-second, major source groups, memory use, frame p95/p99, context switches, page faults, frequency and thermal state over a stable comparable window. Logcat volume and asset watcher activity are secondary, known opportunities. Do not turn a 1-2% desktop FPS difference into an Android energy claim.

### Conditional follow-up - Materialization pacing or streaming

Only pursue if P0/P1 show that the remaining image demand is necessary and P2 still shows intolerable decode/upload spikes. First bound/defer expensive materialization while keeping a usable prior representation; add residency budgets or animation-level streaming only if the measurements justify it. See `asset-preparation-and-residency.md` for quality, readiness, publication and owner-lifetime rules.

## 5. Reproduction and commands (run from repository root)

```bash
cd ~/code/ambition

git status --short
git log -5 --oneline

# Existing one-shot Android autoplay; APK remains ordinarily playable.
# This includes launch logs, but native CPU samples attach AFTER live readiness.
scripts/measure/autoprofile_android.sh smash_4p \
  --warmup 0 --duration 90

# Stable gameplay CPU sample, do not compare its averages directly to cold startup.
scripts/measure/autoprofile_android.sh smash_4p \
  --warmup 35 --duration 60
```

Check the generated profile under `dev/ambition_dev_measurements/profiles/`. The archive includes `metadata.txt`, `game-log.stdout`, `smash-autoplay-readiness.txt`, `smash-autoplay-capture-status.txt`, `frame-census.txt`, `simpleperf-device-*`, `perf.data`, symbol candidates, CPU counter output, thermal/device metadata, and any available summary. `gfxinfo` may report zero rendered frames for the Rust-rendered surface; use engine frame census and GPU diagnostics as appropriate.

For quick comparison of existing runs (no code change):

```bash
cd ~/code/ambition
for archive in \
  dev/ambition_dev_measurements/profiles/android-all-smash_4p-auto-20261108T221326Z.tar.gz \
  dev/ambition_dev_measurements/profiles/android-all-smash_4p-auto-20261108T230722Z.tar.gz \
  dev/ambition_dev_measurements/profiles/android-all-smash_4p-auto-20261108T233358Z.tar.gz; do
  echo "=== $archive"
  tar -tzf "$archive" | grep -E 'metadata.txt|game-log.stdout|frame-census.txt|smash-autoplay-capture-status.txt'
done
```

Capture comparison hygiene: same revision/build ID, quality setting, device, CPU cast, stock count and thermal baseline; record when the app is backgrounded/suspended and separate launch, loading, live steady-state and post-match windows. A fresh install may change device caches, so preserve install policy in metadata. Compare multiple runs if savings are small.

## 6. Proposed agent execution order and stop criteria

1. **Survey current HEAD** and the existing planning documents; establish which prior overlays are present. Avoid edits overlapping active agents.
2. **Reproduce/explain one unrelated image request** (start with `bakery_room_spritesheet.png`) and trace its first semantic producer. Check neighbor-prefetch activation before changing it.
3. Implement the **smallest request-ownership fix** and a non-rostered/no-unrelated-image-load regression test. Include a reason and validation for any load that remains.
4. Run targeted Rust/unit tests plus the Android cold-ish and warmed CLI captures; compare image demand, decoded bytes, first-usable frame, active-match p95/p99 and CPU-seconds/wall-second.
5. If excessive demand remains, improve the first-request census and repeat. Defer headless physics, event-loop/presentation diagnostics and streaming redesign unless the new evidence implicates them.
6. Update this handoff or the existing standalone plan with measured results and concrete next actions, **without overwriting any shared task queue**. Deliver source changes as a ZIP overlay with relative repo paths and ready-to-copy extraction, `git add` and `git commit` commands. Every authored commit command must include `Co-authored-by: GPT-6 <noreply@openai.com>` when GPT-6 is the contributor.

**Sprint completion condition:** A named, tested demand producer explains the unrelated room-sheet requests; redundant requests are removed or consciously retained with documented owners; a new Android capture shows the before/after counts and startup CPU/frame behavior; and the one-shot autoplay build still launches normally from the Android icon. If all remaining demands are justified, close P1 and open the conditional pacing/residency design only with the measured workload.
