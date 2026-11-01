# Asset library, editor and content delivery

**State:** planning, 2026-10-31. This plan does not schedule implementation.
**Source review:** superproject `5ec96e4e3152` (2026-10-31).
**Related work:** [asset authoring, caches and delivery](../../related-work/asset-authoring-caches-and-delivery.md).

## Scope and owners

This plan owns the lifecycle of a file-backed asset, from authoring to a
selected runtime product and then to a distributed release. It also owns the
asset library's future editor-facing queries. It does not create a second
runtime asset loader, content compiler or game package system.

- [Asset manager](../../systems/asset-manager.md) owns `AssetId`, profile
  resolution and the Bevy asset-source boundary.
- [Content generation and reload](content-generation-and-reload.md) owns
  semantic content generations and their activation.
- [Asset preparation and residency](asset-preparation-and-residency.md) owns
  demand, CPU preparation, device upload and residency.
- [Authoring and tools](authoring-and-tools.md) and
  [inspection and workbench](inspection-diagnostics-and-workbench.md) own tool
  discovery and user interfaces. The current [asset inspector](../../tools/asset-inspector.md)
  is the first asset editor surface.
- [Project build and distribution](project-build-and-distribution.md) owns the
  complete executable and game release workflow. This plan owns only the
  asset-store and content-update part of that workflow.
- [Prevalidated content artifacts](prevalidated-content-artifacts.md) is a
  separate, deferred proposal for prepared simulation content. It does not
  authorize deep asset scans during ordinary startup or development.

## Product contract

A developer can add a sprite, change a music cue, replace an SFX product or
register a new asset, publish the result, and restart the native game. That
work must not require a Rust rebuild when Rust code and schemas did not change.
A static web or installed release may rebuild or embed its selected content.
Hot reload is optional. A restart is an acceptable content activation boundary.

An old binary, old image, old audio file or old published selection remains
usable when it can do its job. Staleness and missing provenance are diagnostic
facts. They must not cause an automatic rebuild or a refusal in the normal
native-development path. A developer must be able to list stale items and
learn why each item is stale without a full asset rebuild.

Normal asset selection and reuse must trust local published metadata. Do not
hash all selected files during game startup, inspector queries, or every
publication. A targeted publish can compute evidence for the new product. A
high-stakes release can run a separate, explicit deep audit before it is
released. Reuse that audit result. Do not repeat the audit on every local read.

A genuinely unreadable asset may fail to load. The engine should keep the
active version or use a suitable placeholder when the owning domain permits
it. Do not turn a local media error into a demand for a clean asset store.
Authoritative mechanical content has its own semantic admission rules; this
policy does not remove those rules or authorize incompatible simulation state.

## Existing implementation and limits

The implementation already separates several concerns:

| Concern | Current owner or mechanism | Limit to remember |
| --- | --- | --- |
| Library declaration | `game/ambition_content_library/library.json` | Names producer families, paths and the catalog. |
| Asset roles and metadata | `authored/catalog.json` and `AssetManifest` | This is not a complete editor database. |
| Selected game libraries | Game `pack.ron`; `ContentLibraries` | A game selects zero or more libraries. |
| Selected runtime files | `ContentLibrary::files()` and `LibrarySnapshot` | The selected publication owns managed generated roots. |
| Immutable products | Python `publication_store.Store` | Acceptance, selection and deep audit currently overlap. |
| Runtime bytes | Bevy `AssetSource` and `AssetServer` | Do not copy this loader into the inspector. |
| Editor read model | `tools/asset_inspector/asset_index/` | Some media paths do not yet use publication selection. |
| Audio products | `music` and `sfx` producers in `library.json` | Music uses cue-level output; SFX exposes a bank. |
| Packaging | `scripts/package_asset_guard.py` | The complete game release is a separate concern. |
| Freshness reports | `scripts/content_status.py` | Its normal mode is advisory; strict mode is opt-in. |

The store has accepted bundles and one `published.json` selection. Runtime
selection creates a file map for the App. A selected sprite or audio file does
not require a Rust rebuild. The initial library load parses the manifest and
catalog. The sprite-sheet preparation step also runs at startup. This can cost
time even though registration needs no compilation. Measure each stage before
adding a new cache or an incremental editor database.

In `publication_store.Store.publish()`, selection currently calls deep
`integrity()` on selected bundles, including unchanged bundles. This can make a
small publication read many old files. Change this behavior only through a
focused publication-path implementation and measurement. Do not turn the
performance fix into a stronger acceptance gate.

## Four independent operations

### 1. Author

An author changes source files, recipes, logical roles and references. A source
may be an SVG, RON, LDtk world, sound definition or another file that a producer
owns. Preserve its identity separately from its path and from the bytes of a
published product. Do not make the runtime game catalog a general editor
mutation database.

The editor should query a **derived, read-only index**. Source owners remain
authoritative for their own metadata. The index can include source records,
products, active selections, stale status, use sites and prior publications.
Build it in parts when a tool needs the information. It is not a mandatory
startup dependency for the game.

### 2. Produce

A producer turns source inputs into one or more runtime products. It records
its inputs and outputs when that evidence is available. It can accept the new
products into immutable storage without invalidating earlier usable products.

A normal focused publish should:

1. Run only the requested producer and affected shared work.
2. Write new products to a private staging area.
3. Copy a product into its immutable bundle and hash the bytes at most once
   while copying, if a digest is required for that product.
4. Publish a new selection by reading bundle manifests, without rereading the
   bytes of unchanged bundles.
5. Keep the old selection if the requested publish does not complete.

These steps describe the target, not a claim that every current producer does
all five. Source freshness may be unknown; an unknown value is not a refusal.

### 3. Select

A selection maps logical paths and quality/profile choices to accepted product
versions. The runtime reads one coherent selection when a game or session
starts. The inspector uses the same selection to answer "what will load?".
It can also show old or loose products in a separate workspace view.

Do **not** require loose-file fallback as a permanent policy. Cheap focused
publication may be simpler. Keep the source resolver independent from the
store format so that an optional mutable development overlay can be added later
if measurements justify it. A future overlay must have visible precedence and
must not silently change a pinned release.

### 4. Deliver

A release freezes a selection of game content and library products for one
target. Distribution transfers the missing bundles, then changes the local
selection only when the required transfer is complete. Old selections remain
available for rollback while retained.

The first transport candidate is **rsync** over immutable bundle directories.
A later HTTP, CDN or IPFS transport should supply the same logical files and
selection data. Transport addresses and product identities are different
fields. Do not put `rsync`, HTTP or IPFS syntax into game asset references.
The installed game must be able to start offline with its local selection.

## Identity and metadata boundaries

Do not merge these concepts:

| Concept | What it identifies | Stability rule |
| --- | --- | --- |
| Source identity | Editable source object or recipe | A rename should not have to change its identity. A future editor may require explicit IDs. |
| Logical asset ID | A role the game requests | Remains stable across generated versions and physical moves. |
| Product identity | Exact generated artifact or bundle | Immutable after acceptance. A digest can identify its bytes. |
| Selection identity | A set of selected product versions | Can be pinned for a session or release. |
| Content epoch | Mechanical data admitted to a game session | Owned by the content-generation contract, not by the file store. |
| Location | Local file, static bundle or remote transport | Can change without changing the logical asset ID. |
| Residency | Loaded CPU or device representation | Bevy and domain owners manage its lifetime. |

These are design distinctions, not a mandate to add seven identifiers now.
Reuse current identifiers where they already have the required semantics. Add
an explicit stable source ID only when an editor needs a reliable rename or
reference-update operation.

A future editor-facing record should be able to report `logical_id`, `family`,
`owner`, `source`, `selected_product`, `selection`, `quality`, `uses`, `status`
and `provenance` as applicable. Unknown values are allowed. Its view must not
be a second authority that games need at startup.

## The editor surface

The asset inspector can grow into an asset browser and editor workbench. Keep
its data API usable by command-line tools and agents. A graphical view is one
client of that API.

The first useful workflows are:

- Browse, filter and search by family, source, owner, quality and game use.
- Preview selected images and audio without materializing unrelated products.
- Show the selected version beside other available versions and loose files.
- List stale products and reasons using recorded inputs or cheap metadata.
- Show inbound and outbound references where the domain owners expose them.
- Publish a selected target and show the resulting selection change.
- Compare two selections, including added, replaced and removed products.
- Show store size, retained selection roots and garbage-collection candidates.

Later workflows can add safe rename/move operations, drag-and-drop bindings,
batch import settings, undo and edit transactions. Do not make runtime loading
wait for those features. Do not invent a second reference graph if the content
compiler or another domain owner already exports the needed relation.

## Startup and iteration cost model

Measure these costs separately. Do not call them all "asset registration":

| Phase | Expected scaling factor | Desired behavior |
| --- | --- | --- |
| Register one new catalog entry | Small authored edit | No Cargo build. |
| Accept a new bundle | New product bytes and staged files | Do not scan old payload bytes. |
| Change the selected publication | Manifest entries and changed units | Atomic metadata work. |
| Native startup | Selected manifests, catalog and prepared sheet metadata | No deep asset audit; record where time goes. |
| First visible use | Needed decode, device upload and audio work | Prepare on demand under residency policy. |
| Inspector search | Indexed metadata | Do not decode every asset or render on status requests. |
| Release package | Selected closure and target conversion | Deliberately more expensive; may embed or recompile. |

Measure cold and warm launches, changed and unchanged launches, release and
debug profiles, catalog entry counts, sprite-table size and selected file count.
Report wall time by phase, bytes read, file metadata operations, and bytes
hashed. Do not infer a startup regression from publication time alone.

## Cleanup and history

The store should eventually retain immutable selection snapshots and explicit
pins for released builds. A garbage collector can mark all bundles reachable
from the current selection, pinned releases and chosen rollback history. It can
then list unreferenced old bundles.

Start with a **dry-run report** that names each candidate, its size, age and
reason for retention or deletion. Actual deletion is an explicit action. Use
a grace period and account for active readers or package jobs before deletion.
Do not verify every stored file to decide if it is reachable. Keep the current
selection and every pinned release usable after cleanup.

Retention defaults, pin syntax and a concurrent-reader protocol are not fixed
by this plan. Set them when GC has a real operator and storage measurements.

## Delivery and update seams

An initial content-only update can use this order:

1. Select an immutable release manifest for the target game and platform.
2. Compare bundle names/identities with the installed release.
3. Transfer missing bundles into a staging root with rsync.
4. Use an explicit release-time or download-time audit policy when needed.
5. Atomically install the new selection or release pointer.
6. Restart or activate the new content under the owning session policy.
7. Retain the previous selection for rollback.

Start with a full selected-game content closure, not the whole shared library.
The closure can include fonts, backgrounds, music, SFX, sprite metadata, worlds
and game-owned files. Do not assume everything belongs to one library. Keep
bundle size and dependency shape visible. One large SFX bank can make a small
sound change expensive to transfer; many small bundles also cost metadata,
files and open operations. Measure before changing producer granularity.

Remote or user-supplied content is a different trust boundary from a local
accepted bundle. Validate untrusted inputs at installation or admission as
needed. Avoid full deep checks during each ordinary trusted local read. A
publisher signature or compact validation proof may be useful for higher-risk
releases, but neither is required for the first rsync workflow.

## Work slices and promotion triggers

These are possible work slices. They are **not** a second execution queue.
Only [the live queue](../queue.md) selects implementation work.

| Slice | Promote when | Completion evidence |
| --- | --- | --- |
| L0. Cost inventory | Startup or publish time obstructs a real iteration. | Trace launch parsing, sheet preparation, publish hashing, catalog scans and first visible use separately. |
| L1. Metadata-only selection | A focused publish rehashes accepted unchanged bundles. | Publish touches new products and metadata; unchanged bundle payloads are not read. An old selection remains usable. |
| L2. Common inspector/runtime selection | The inspector reports a product differently from the game's selected files. | Both resolve the same selection for images, audio and fonts; workspace-only files remain visible separately. |
| L3. Selection snapshots and GC | Historical bundles consume useful storage or rollback needs pins. | Dry-run reports correct reachability and savings; an explicit cleanup retains current and pinned releases. |
| L4. Rsync content updates | An external game or second machine needs content-only updates. | Transfer only missing bundles, install selection last, restart offline, and roll back. |
| L5. Editor mutations | A real authoring operation needs stable rename, reference edits or batch changes. | One owner performs the mutation; inspector queries update without becoming authoritative. |
| L6. Other transports | A real release needs HTTP/CDN/IPFS or remote streaming. | Same logical identities and selections; no transport-specific references in game content. |

L0 does not authorize a new binary cache by itself. If existing parsing is
cheap, do not add one. L1 must not introduce extra integrity checks. A separate
prevalidated simulation-content project retains its own execution gate.

## Acceptance and forbidden regressions

- Native registration, regeneration and focused publication can finish without
  a Cargo rebuild when Rust code and schemas are unchanged.
- A stale but usable binary or media product does not block a developer demo.
  A query can name its stale status and reason.
- Ordinary startup and publication do not deep-hash unchanged selected media.
- Restarting a game selects a coherent published snapshot. Hot reload is not
  required to satisfy the edit-to-play contract.
- A status or browse query does not decode the whole library or rerender art.
- The inspector distinguishes selected, other available and absent products.
- Music, SFX, fonts and any other loaded file family use the same selected-file
  rules as sprites, or document a real separate source owner.
- Packaging can embed content and rebuild for a static distribution.
- Cleanup cannot remove a current or pinned release. Remote updates install a
  selection only after the required files are present.
- No design step requires a central service or an online connection for a
  local game or local authoring session.

## Deferred choices

- Whether focused immutable publication is fast enough to avoid a mutable
  workspace overlay. Measure it before choosing.
- How much of the editor index to cache, and when to update it. Start from the
  current inspector's derived queries.
- Stable source-ID and rename policy. A first transactional rename customer
  should determine it.
- Default bundle granularity and automatic dependency closure for transport.
  Start from actual change and update-size measurements.
- Selection pin/retention policy and concurrent GC handling.
- Package compatibility, publisher trust, optional signatures, remote
  credentials and network recovery behavior.
- HTTP, CDN, IPFS and on-demand streaming. The transport interface can be
  designed later without changing logical asset identity.
