# Asset library implementation review

**Review date:** 2026-10-31. **Source baseline:** `5ec96e4e3152`.
**Review type:** code trace and focused Python tests. No runtime code changes.
**Planning owner:** [Asset library, editor and delivery](asset-library-editor-and-delivery.md).
**Engine comparison:** [Asset authoring, caches and delivery](../../related-work/asset-authoring-caches-and-delivery.md).

## Decision summary

Keep the current content libraries, immutable bundle store, and Bevy asset source.
They provide a sound base for an editor and content-only updates. Improve the
boundaries between them before adding a database, a mutable store, or remote
loading.

The most important gap is **selection agreement**. The runtime, inspector,
status tool, and packager each have their own view of the selected files.
Their views can disagree about music, SFX, fonts, loose files, and missing
publications. Make the selected file set an explicit contract that each tool
can consume or verify. Do not put the inspector or its query cache on the game
startup path.

**Standing development rule:** an old usable product is usable. Stale state,
missing evidence, and an old build date are reasons for a warning, not for a
rebuild or refusal. No routine deep integrity check belongs in native startup,
asset browsing, publication selection, or a web preview. A release or a new
remote trust boundary may use an explicit, stronger check.

## Current data flow

```text
Authored files and recipe inputs            Game pack and library manifests
       |                                                  |
       v                                                  |
Producer scripts and renderer submodules                  |
       |                                                  |
Private staging tree                                      |
       |                                                  |
       v                                                  |
Store.accept() -> bundles/<identity>/manifest.json        |
       |                                                  |
       v                                                  |
Store.publish() -> published.json -------------------------+
       |                                                  |
       +--> Rust ContentLibrary::files() --> snapshot --> Bevy AssetReader
       |                              |                    |
       |                              +--> sheet RON       +--> image/audio on demand
       |                                   preparation
       |
       +--> Python Library.files() --> package_asset_guard.py
       |
       +--> content_status.py (separate freshness view)
       |
       +--> asset inspector (separate inventory + previews + usage)
```

**Authority and ownership:**

| Fact | Current owner | Source evidence |
| --- | --- | --- |
| Which libraries a game imports | Game `pack.ron` | `crates/ambition_content_pack/src/embed.rs:40-55` and game packs |
| Library paths, producers, and catalog | `library.json`, `authored/catalog.json` | `game/ambition_content_library/library.json`; `content_library.rs:114-131` |
| Exact produced bundle members | Accepted bundle manifest | `publication_store.py:86-125` |
| Selected bundle versions | `published.json` | `publication_store.py:145-202` |
| Native runtime file identity and bytes | `ContentLibraries` and `LibraryAssets` | `content_library.rs:217-259,315-330`; `library_source.rs:45-75,90-134` |
| Prepared sprite metadata and its activation | Sprite sheet subsystem | `platformer2d/src/content/libraries.rs:8-27`; `baked_sheet_rons.rs:258-339` |
| Inspector entries and use sites | Inspector read model | `tools/ambition_editor/asset_index/` |
| Complete release asset tree | Game-owned files plus selected library files | `scripts/package/package_asset_guard.py:350-374` |

## Code findings, ordered by leverage

### A. Selection is correct in the runtime, but not uniform in the tools

**Confirmed:** Rust `ContentLibrary::files()` reads the selected publication,
excludes loose copies under publication-managed roots, and adds published
members. `ContentLibraries::files()` gives each library a qualified name and
adds an unqualified alias only when it is not ambiguous
(`crates/ambition_asset_manager/src/content_library.rs:217-253,315-330`).
The snapshot passed to Bevy is consistent for a given installation.

The inspector takes another route. The music, SFX and font providers inspect
loose `generated/runtime/full` paths, rather than every selected member.
Published-only media can therefore be marked missing
(`tools/ambition_editor/asset_index/kinds.py:150-230,297-307,352-380`).
The image scanner also inventories loose products under managed roots.
That is useful workspace information, but such files are not automatically
runtime-selected (`asset_index/paths.py:96-132`).

The Python packaging resolver has its own selection and root rules. In
`scripts/lib/content_libraries.py:28-61` it *requires* a publication when a
store is declared. Rust treats an empty selection as usable
(`content_library.rs:153-158`). This is a difference between a permissive
runtime and a stricter packaging consumer. Preserve that distinction only
where a package truly cannot be formed; report the difference to the user.

Publication metadata also has inconsistent acceptance rules. Python
`Store.selected()` refuses a selection when its recorded generation identity
does not match the JSON (`publication_store.py:60-72`). Rust checks the schema
and safe paths, but not the generation digest
(`crates/ambition_asset_manager/src/art_publication.rs:48-65`). A stale or
mismatched *identity field* should be reported without hiding usable local
members in normal development. Keep path-safety checks and unreadable-data
errors distinct from a freshness or digest diagnostic. A release audit can
make stronger claims before shipping.

**Design action:** define a small read-only `selected files` contract, with
library ID, logical name, selected product or file, generation, physical source,
role and origin. Consumers may implement it in Rust or Python. Use shared
fixtures to check the same semantics; do not create a cross-language runtime
service. Report loose, old, and unselected products as a *separate* workspace
view. An absent publication should not force native runtime validation.

**Done 2026-11-10 (the contract and its readers).** The contract is six
shared cases under `crates/ambition_asset_manager/tests/data/selected_files/`:
a library, and the members it selects with origin (`published` or `loose`)
and generation, or the word its error must contain. The runtime reader
(`tests/selected_files_contract.rs`) and the packaging reader
(`scripts/tests/test_selected_files_contract.py`) both read them. The cases:
a publication owns the sprite roots and the roots it names; a store that has
published nothing is an empty selection; a library without a store manages
nothing; a stale recorded identity still selects; an authored file colliding
with a product, and a member path outside the store, refuse. Rust already
agreed with all six. Python disagreed on two, and now agrees:
`Library.selected_files()` treats an empty store as an empty selection and a
stale identity as an advisory (`Library.advisories()`, through
`Store.selected(verify_identity=False)`). The stricter requirement is stated
where it belongs: `package_asset_guard.collect_source_files` refuses a
library whose store published nothing (`Library.requires_publication()`).
`art_publication.resolve` resolves through a stale identity too. Section E's
inspector part is done the same day.

### B. Publication works, but checks the past on every change

`Store.accept()` computes a digest of each source, copies it into a new bundle,
then computes a second digest of the copy
(`tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/publication_store.py:86-125`).
`Store.publish()` merges replacements under a file lock, then visits *all*
selected bundle manifests and calls `integrity()` before writing one atomic
`published.json` (`publication_store.py:145-202`). It also reports parent
staleness without rejecting the new selection; retain this useful policy.

A controlled three-file reproduction saw six direct digest calls during
acceptance. In the same reproduction, an unchanged selection with a populated
`Digests` cache did not recompute file content hashes. Without that cache,
`publish()` did hash the three members. The precise cost therefore depends on
cache state. It is wrong to claim that *every* publish always rereads every
member. It still iterates all selected members and does file metadata work.

**Design action:** keep immutable acceptance and atomic selection. Compute a
new product digest while copying its bytes once; use the recorded digest in
the bundle manifest. On selection update, use the accepted manifests and check
ownership/conflict rules. Do not call deep `integrity()` on unchanged bundles.
If a bundle is damaged, a requested file may fail; retain the previous usable
selection or report the broken file. A high-stakes release runs a separate
one-time audit before distribution.

**Done 2026-11-10 (measured).** `Store.accept` copies each member once
while hashing it (`copy_digest`), so the recorded digest is of the bytes the
bundle holds; an existing bundle with the same key gets an existence check,
not a re-read. `Store.publish` reads accepted manifests, checks ownership
(`Two producers own`), checks only that a bundle it newly selects has its
members, and keeps the lock, the merge of concurrent publications and the
atomic write; its inert `hashes` parameter is gone. The deep check is the
explicit `Store.audit()`. Measured with
`scripts/measure/measure_publication_cost.py` (synthetic store, members of
256 KiB, one new unit of 8 members; two runs each, the same both times):

| step | before | after |
|---|---|---|
| accept one 2.1 MB unit | 6.29 MB read, 4.20 MB hashed, 59 stat calls, 15-17 ms | 2.10 MB read, 2.10 MB hashed, 19 stat calls, 9-10 ms |
| publish onto 4 bundles (8.4 MB) | 10.5 MB read and hashed, 44 stat calls, 31-35 ms | 0.02 MB read, 12 stat calls, 3.3-3.8 ms |
| publish onto 200 bundles (419 MB), no digest cache | 423 MB read and hashed, 1612 stat calls, 1123-1156 ms | 1.1 MB read (manifests), 12 stat calls, 59-67 ms |
| the same, warm `Digests` cache | 1.8 MB read, 110-112 ms | (no cache taken) |
| explicit `audit()`, 200 bundles | (inside every publish) | 423 MB read, 1096-1218 ms |

Witnesses in the renderer's `tests/test_publication_store.py`: a focused
publication reads no payload digest while an old member is damaged, and
`audit` reports that member (a poison restoring the per-publish deep check
reddened it); a newly selected bundle missing a member refuses and leaves the
previous selection; accept records the digest of the bytes it copied. The
existing concurrent-publication, failed-accept and stale-parent tests pass.

**Scope caution:** the music publisher checks cue dependencies and renders
per-cue units, while SFX currently produces an audio bank. It can refuse to
*certify a new render* when required inputs are not current
(`scripts/regen/generate_library_content.py:223-240`). That must not turn into a
refusal to run the last usable published version.

### C. Startup scans the library and prepares sheet data

Native installation calls `ContentLibrary::read()`, constructs a library
snapshot, reads selected sprite metadata, and calls `prepare_sheet_table()`
**synchronously** (`crates/ambition_platformer2d/src/content/libraries.rs:8-27`).
The manifest parser deserializes the whole semantic catalog
(`content_library.rs:114-131`). `ContentLibrary::files()` enumerates declared
asset roots and selection entries (`content_library.rs:217-253,359-377`).

`SheetTexts::from_snapshot()` traverses the alias-expanded snapshot for each
library and reads RON and other metadata
(`crates/ambition_sprite_sheet/src/library.rs:23-105`). The sheet preparation
code says that parsing the full table costs about one second in a debug build
(`baked_sheet_rons.rs:258-260`); that is a code comment, not a measurement of
this user's current build. It also prepares body rigs, portraits, and part
flipbooks. This path does not eagerly decode every registered PNG.

One missing unqualified asset lookup can scan the entire file map to detect
ambiguity (`crates/ambition_asset_manager/src/library_source.rs:90-100`).
Directory queries also walk key sets. This matters if Bevy makes many missing
file and `.meta` requests, but frequency is not measured.

A separate sprite watcher runs in the background. It may reconstruct a file
inventory on a poll and prepares changed tables off the frame
(`crates/ambition_platformer2d/src/content/sprite_reload.rs:21-49,93-180,244-265`).
Do not count its work as synchronous initial library preparation without
profiling it.

**Design action:** instrument catalog parse, root enumeration, selected-index
composition, sheet text reads, sheet preparation, first frame, asset reads,
missed lookups, and watcher polls separately. Only then decide whether a
precomputed index, lazy sheet preparation, or alias/directory lookup table
pays for itself. A per-user no-rebuild benchmark is more useful than a new
cache without measurements.

### D. The web development packager does release-level work

`collect_source_files()` computes SHA-256 for each selected game or library
file (`scripts/package/package_asset_guard.py:350-374`). `audit_tree()` hashes the
composed files again (`package_asset_guard.py:734-764`). The script documents
that even `compose --materialize link`, used for served-web development,
performs the full audit (header lines 25-36).

**Design action:** retain the deep copy-and-audit path for release packages.
Add a development composition mode that trusts local selections and uses
metadata for the file list. It can warn about missing inputs and continue when
possible. Do not make link mode alone imply a release audit. Measure this
path separately from native launch and static compilation.

**Done 2026-11-10 (measured).** `compose --policy local` composes the same
selected file set from metadata: no input is hashed, the composed tree is
checked by membership (every selected file, nothing else), a declared input
that is absent, or a library whose store published nothing, is reported and
left out, and the contract records `"policy": "local"` with no digests, so
`audit-tree` and `audit-zip` refuse it. `--policy release` stays the default
and every package's. `build_for_web.sh --served` composes a linked tree
under `local` and a `--copy-assets` tree under `release`.

A profile of the old compose on this checkout (web profile, 14,827 files)
found the largest cost was neither hash: `resolve_declarations` searched
every file for each declaration's library alias (75 million `split` calls,
60 of 105 profiled seconds). The aliases are now indexed once, for both
policies. Wall time, two runs each, warm content compiler:

| compose, web profile, link | before | after |
|---|---|---|
| release | 58.8, 59.3 s, then refuses (14 declared assets absent on this VM) | 38.5, 38.7 s, the same refusal |
| local | (no such policy: the preview could not be composed here) | 21.3, 22.3 s, composed, the 14 reported |

Witnesses: `scripts/tests/test_package_preview_policy.py` (local hashes
nothing and composes the release file set, which a poison making it hash
reddened; a missing declared input is a warning locally and a refusal in
release; no audit accepts a local contract). The Android and Steam Deck
packagers and a static web release pass no `--policy` and keep the release
path; their final asset contracts were not run on this VM, which lacks four
generated families.

### E. The audio publication is integrated; inspector resolution lags

The library manifest declares `music`, `sfx`, `fonts`, `backgrounds`, `sprites`
and quality variants. `scripts/regen/generate_library_content.py` publishes them.
Music uses per-cue bundle units and a registry; SFX currently ships a bank.
At runtime, music handles load through Bevy on first use
(`crates/ambition_audio/src/library.rs:341-365`); SFX banks load through Bevy
(`crates/ambition_audio/src/bank_asset.rs:329-375`).

**Design action:** do not start a new audio asset subsystem. Fix inspector
resolution and release closure for the existing one.

**Inspector resolution done 2026-11-10.** `asset_index/kinds.py` reads the
library's selected file set once (`selected_files`) and locates the music
registry, each track, the SFX bank and each font through it. A loose copy
under a root the publication owns is still listed, with
`details.selected = false` and a reason saying the game does not select it;
sprites already worked this way (published roots, and loose roots labelled
`unmanaged`). Measured on this checkout, where every product is published and
nothing is loose: before, all 19 published history cues had no audio located;
after, all 19 do. They stay `missing` here for a true reason, now stated: the
selection has no `audio/music_registry.ron`, so the game cannot name them.
Witness: `test_published_only_media_resolve_and_a_shadowed_workspace_copy_says_so`
(a poison that ignored the selection reddened it). Release closure and bank
update amplification are not measured yet. Measure bank update
amplification before deciding whether the SFX bank needs smaller units.
Include fonts, backgrounds, sprite metadata, LDtk worlds, and game-owned
content in each release closure even when their files come from different
owners.

### F. Selection and GC must account for active readers

`published.json` stores one current selection. Old immutable bundles remain on
disk; the store has no persistent release-selection history or GC policy.
A selected sprite can also have a version-specific physical `content://.../__art/...`
asset path (`content_library.rs:244-249`; `sprite_sheet/src/library.rs:47-52`).
`LibraryAssets::immutable()` resolves such paths directly to the bundle
folder (`library_source.rs:45-52`). An open game may therefore hold a path to
a bundle after `published.json` moves to a later selection.

**Design action:** save immutable selection receipts and release pins *before*
GC is allowed. Start with a report-only mark-and-sweep tool. Treat a running
session, an active packager, and an in-progress publication as possible roots.
An initial offline-only, explicit cleanup with a grace period is safer than
implementing leases now. Never delete a bundle solely because it is not in
the current selection. No file-content hashing is needed for reachability.

### G. Native code/data separation should not change

`crates/ambition_content_pack/src/embed.rs:40-44` stops dependency registration
unless `CARGO_FEATURE_STATIC_CONTENT` is set. This is the required behavior:
normal native development does not recompile Rust because an asset was
registered or republished. Embedded or static distributions may recompile.
Do not make the editor, publication status, or selected bundle identity a
Cargo input for non-static development builds.

## Source agreement matrix for a future editor

| Operation | Runtime | Inspector | Status | Packager |
| --- | --- | --- | --- | --- |
| Selected library IDs | Game pack | Currently profile + optional override | Game packs | Game pack |
| Selected file membership | Rust snapshot | Independent root/media scans | Selected bundle status | Python library resolver |
| Loose unselected files | Not runtime-selected under managed roots | Shown in scan | May count loose family fallback | Not included when managed |
| Stale source product | May still load | Different per family | Reported as diagnostic | Not normally distinguished |
| File-content audit | Not routine | Not required for browse | Cached input evidence | Currently full SHA during compose |
| Source use sites | Domain-specific | Cross-game scan | Not needed | Pack references |

A shared contract must define *which files will load*, not force every tool to
share the same query model. A fast editor index can cache the selected view,
other available products, use sites, and provenance. It must not add work to
native startup. Keep game-scoped uses and cross-game uses as separate queries;
the inspector currently scans all `game/*/assets/*.ron` and fixed LDtk roots
(`tools/ambition_editor/asset_index/usage.py:66-72,117-121`).

## Decisions and alternatives

| Decision | Preferred now | Why | Revisit when |
| --- | --- | --- | --- |
| Asset store | Keep immutable bundles and atomic selection | Works across media and enables rollback | Measured focused publish is too slow |
| Development override | No new mutable store yet | Another authority adds precedence and invalidation | Publication latency obstructs edit-to-play |
| Selection contract | Share semantics, not a new daemon or database | Rust and Python have different customers | A tool truly needs cross-process transactions |
| Editor index | Derived, optional, incremental | Query-heavy; runtime does not need it | Initial inspector queries exceed budget |
| Startup cache | Instrument first | Parsing and scanning costs are distinct | Measured startup cost exceeds target |
| Publication checks | Manifest-only on trusted reuse | Immutable accepted objects do not change by policy | New object acceptance or explicit release audit |
| GC | Dry-run, pins, offline-first cleanup | Avoids deleting active version-specific resources | Multi-process cleanup is an actual need |
| Distribution | Frozen release selection + rsync | Simple and local-first | First remote update customer |
| Other transport | Keep transport outside logical IDs | Makes HTTP/IPFS possible | Need streaming or a public CDN |

## Ranked next steps and evidence

**R0 — Cost trace, no behavior changes.** Add opt-in timing and operation
counters to publication and native startup. Capture cold and warm starts,
first visible frame, selected file counts, bytes read, hashes, stat calls,
metadata preparation, and packager composition. Report actual milliseconds
and filesystem operations; do not infer them from the source alone.

**R1 — Remove routine publication audits.** Focused one-pass acceptance,
manifest-based selection, unchanged bundle reuse, atomic swap, and a separate
audit command. Check the small-unit case and a many-bundle case. Keep the old
selection available if new work fails.

**R2 — Make the inspector selection-correct.** Supply published-only audio,
fonts, and sprites through the same selected-file contract. Keep stale and
loose artifacts visible with clear selection status. Retain read-only fallbacks
if a provider or the optional content-check binary is missing; never force
rebuilding a usable old binary.

**R3 — Separate preview composition from release audit.** Keep explicit release
verification and strict shipping policy. Make linked web preview and local
iteration avoid full-library hashing.

**R4 — Record release snapshots, then GC.** Start with reachability reports
and protected roots; add explicit offline deletion after the report is
useful. Ensure old handles and rollback releases remain usable.

**R5 — Add content-only distribution.** First freeze a full game-specific
closure, not every asset in a shared library. Use rsync to stage missing
bundles, then install the selection. Test offline restart, failed transfer,
rollback, and version compatibility. Do not build an IPFS client now.

**R6 — Expand editor writes on demand.** When an actual rename, drag-and-drop
registration, or batch edit needs stable identity and transactions, add them
at the source owner. Keep the inspector read model derived.

These are roadmap proposals, not a new active work queue.

## External engine lessons, tied to these seams

- **Godot:** a filesystem-first editor and imported products help authors,
  but automatic import can cost time. Ambition should keep explicit focused
  publication and make the selected file easy to browse. Godot PCK patches
  demonstrate simple mount-based updates.
- **Unity:** its source GUID/imported artifact split supports editor refactors;
  Addressables separates a remote catalog from bundle bytes. Ambition already
  has an analogous logical name, immutable bundle, and mutable selection.
  Delay editor-wide GUIDs and complex content-group restrictions.
- **Unreal:** Asset Registry metadata lets the editor browse without loading
  assets; DDC serves derived outputs. Ambition's inspector should use derived
  metadata but should not impose a cooked-data pipeline on native startup.
- **Defold:** independently delivered content and explicit mounts show a path
  to small installations. Ambition can first make a frozen release closure
  and transfer it with rsync.
- **Bevy:** AssetServer and AssetReader already own runtime bytes and handles.
  Keep them. Ambition needs to own the higher-level editor and distribution
  policy, not a second runtime cache.

See [the source-linked engine comparison](../../related-work/asset-authoring-caches-and-delivery.md)
for the relevant primary documentation and known tradeoffs.

## Verification and limits

- Source-only snapshot at `5ec96e4e3152`. Generated media and a built native
  executable were not part of the review. No claim of current startup time or
  publish wall time is made.
- Focused Python suites: 27 passed, one failed because the environment did
  not provide the sprite render dependency `resvg_py`. That failure was an
  import error before the targeted assertion, not evidence of a publication
  regression. No Cargo build was run.
- A controlled three-file publication reproduction counted six direct file
  digests on acceptance. Cached and uncached selection paths behaved
  differently, as described above. It was not a throughput benchmark.
- The plan states observed behavior, identifiable scaling costs, and design
  recommendations separately. Measure actual workstation costs before
  selecting caching, lazy initialization, new store types, or bundle sizes.
