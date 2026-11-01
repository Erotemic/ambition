# Asset authoring, import caches, editor indexes and content delivery

**Checked:** 2026-10-31 against first-party engine documentation.
**Ambition owner:** [asset library, editor and content delivery](../planning/engine/asset-library-editor-and-delivery.md).
**Scope:** the developer edit loop and the route from authored assets to a
shipped content update. This is distinct from the existing comparison of
[asset addresses and runtime profiles](asset-addressing-runtime-profiles-and-publication.md).

## The question

Ambition already has authored content, producer tools, an immutable asset store,
a publication selection, Bevy asset readers and an asset inspector. How can an
asset browser grow into an editor without forcing all game assets into a large
engine database or making a native development restart expensive?

Five engines expose different useful designs. The comparison separates their
**documented behavior** from an **engineering judgment** about Ambition. A
feature in one engine is not evidence that Ambition must copy it.

## Comparison at a glance

| Engine | Editor discovery and import | Delivery and updates | Useful lesson | Cost to avoid |
| --- | --- | --- | --- | --- |
| Godot | Project files and a managed import cache | PCK/ZIP packs and patch packs | Easy file-first authoring | Automatic reimport and editor/cache coupling |
| Unity | Source database, GUID metadata and imported artifact cache | Addressables catalogs and AssetBundles | Stable references and content-only updates | Group rules, rebuild coupling and dependency complexity |
| Unreal | Async Asset Registry and Derived Data Cache | Cooked, platform-specific chunks and patches | Browse metadata without loading media; share derived work | Large cook/packaging pipeline for a simple dev edit |
| Defold | Editor-owned resources and a compiled game archive | Optional live-update archives and mounted packs | Small base install and later content delivery | Some update behavior needs a bundled build to test |
| Bevy | AssetServer, source readers, loaders and optional processors | Application-owned package policy | Reuse the runtime data path | Expecting a runtime framework to supply an editor database |

The sources for each row are linked below. The "cost to avoid" column is our
assessment, not an engine benchmark.

## Godot: direct files and a hidden imported copy

**Documented design.** Authors place image, audio, font and other sources in
the project or drag them into the FileSystem dock. Godot stores imported
products under `.godot/imported/` and keeps import settings in source-adjacent
`.import` files. Source changes can cause an automatic reimport. A game uses
`ResourceLoader` for imported assets rather than opening the source file
itself. See Godot's [import process](https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/import_process.html)
and [import plugins](https://docs.godotengine.org/en/stable/tutorials/plugins/editor/import_plugins.html).

Godot can export PCK/ZIP packs and add update or mod packs without replacing
the original executable. Its resource-pack approach also permits a loose
runtime-file route, but that route has different loading semantics. See
[exporting packs, patches and mods](https://docs.godotengine.org/en/stable/tutorials/export/exporting_pcks.html).

**Benefits for Ambition.** A file is easy to find and edit. Import settings
stay near the source. The editor and game can access the same logical resource
without forcing the user to manage generated cache paths. A standalone pack is
also a simple update unit.

**Tradeoffs for Ambition.** Automatic reimport can add work to an edit loop.
The editor must keep its import state and the filesystem in sync. A game may
have to distinguish imported resources from arbitrary runtime files. Ambition
already has specialized sprite and audio generators; it should not replace
them with a single file-extension-based importer just to imitate Godot.

**Decision pressure.** Keep source files visible and producer-owned. Make
focused import/publish actions easy. Make editor metadata refresh cheap and
separate from expensive regeneration.

## Unity: source identities, an artifact database and Addressables

**Documented design.** Unity assigns asset GUIDs and keeps import settings in
`.meta` files. The GUID lets references survive moves or renames. Unity tracks
source data separately from imported artifacts in the project's `Library`
directory. It refreshes changed sources and dependencies; code-related files
can also cause script compilation and domain reload. See Unity's
[asset metadata](https://docs.unity.cn/Manual/AssetMetadata.html),
[asset database contents](https://docs.unity.com/en-us/engine/6000.6/manual/assets-and-media/asset-database/contents)
and [refresh process](https://docs.unity.com/en-us/engine/6000.7/manual/assets-and-media/asset-database/refreshing).

Addressables maps logical keys through catalogs to local or remote AssetBundles
and tracks dependencies. Content update builds reuse unchanged bundles and
replace changed bundles in the catalog. Group update restrictions and the
previous build's content-state file affect this workflow. See the official
[Addressables overview](https://docs.unity.cn/Packages/com.unity.addressables@2.0/manual/AddressableAssetsOverview.html)
and [content update workflow](https://docs.unity.cn/Packages/com.unity.addressables@2.0/manual/content-update-build-create.html).

Unity documents how bundle-level dependencies can affect residency and memory:
a dependency can cause more than the requested asset to be loaded. See
[Addressables memory management](https://docs.unity.cn/Packages/com.unity.addressables@2.3/manual/MemoryManagement.html).

**Benefits for Ambition.** Separate logical identity from file location and
product bytes. Keep source identity stable through editor moves. A content
catalog allows updates without replacing game code. A shared import cache can
avoid work on other machines when that work is costly.

**Tradeoffs for Ambition.** Asset groups, build-state files, bundle dependencies
and update restrictions impose decisions on authors. Import refreshes and
script reloads can also make iteration more expensive. These are features with
costs, not requirements to adopt before we have real customers.

**Decision pressure.** Borrow the identity split and selected publication
catalog. Delay automatic dependency-based repackaging, source database
transactions and bundle-group policy until editing or delivery needs them.

## Unreal: metadata discovery, derived cache and cooked release units

**Documented design.** Unreal's Asset Registry collects searchable metadata
about unloaded assets. The editor can list and filter them without loading
each object's full data. The registry updates from changes in memory and on
disk. See Epic's [Asset Registry](https://dev.epicgames.com/documentation/en-us/unreal-engine/asset-registry-in-unreal-engine).

Unreal stores rebuildable derived work in a Derived Data Cache (DDC). It can
use local, shared or cloud caches. A cache miss generates the missing derived
product. Cooked builds convert assets for the target platform and can assign
assets to independently distributed chunks. See
[Derived Data Cache](https://dev.epicgames.com/documentation/unreal-engine/using-derived-data-cache-in-unreal-engine?lang=en-US),
[cooking and chunking](https://dev.epicgames.com/documentation/en-us/unreal-engine/cooking-content-and-creating-chunks-in-unreal-engine)
and [project patching](https://dev.epicgames.com/documentation/en-us/unreal-engine/updating-unreal-engine-projects-with-patches-after-release).

**Benefits for Ambition.** A metadata-only asset browser can scale without
opening all images or sounds. Derived work can live outside source control and
can be shared. The release cook and its chunks can be separate from editor
loading and native development.

**Tradeoffs for Ambition.** A complete asset registry, cook graph and shared
cache service add format and ownership work. Large or platform-specific chunks
can increase update and build cost. Ambition's current source and publication
manifests are simpler. Preserve that advantage until measurements show a need
for a larger pipeline.

**Decision pressure.** Make the inspector a derived read model, not a second
source of truth. Keep native restart free of target cooking. Add shared caches
or computed indexes only for measured repeat work.

## Defold: small archives and explicit live-update mounts

**Documented design.** Defold packages resources into a base application
archive. Authors can exclude selected content from that archive and ship it
later as ZIP, folder or hosted resources. The game mounts optional content with
a specified priority. Defold documents that editor Build mode does not test
Live Update; a bundled application is needed. See Defold's
[Live Update manual](https://defold.com/manuals/live-update/).

**Benefits for Ambition.** The base game can be small. A release can fetch more
content later. Explicit mount order gives a clear answer when packs overlap.

**Tradeoffs for Ambition.** The export workflow needs to preserve each optional
content closure, and test runs may use different resource paths from a final
bundle. A pack priority system needs precise conflict rules.

**Decision pressure.** Plan for optional installed content and explicit
selection priority, but keep the default development path local and offline.

## Bevy: keep the loader, provide Ambition's policy on top

**Documented design.** Bevy has an asset source/reader and loader pipeline.
Bevy 0.18 also supports file-type default asset processors and processors
selected through asset metadata. See [Bevy 0.18's asset processor notes](https://bevy.org/news/bevy-0-18/).

**Current Ambition implementation.** `crates/ambition_asset_manager/src/library_source.rs`
registers Bevy asset sources that resolve selected library files. The App-local
`LibraryAssets` snapshot defines those files. The inspector does not need to
reimplement Bevy's runtime loading and caching.

**Benefits for Ambition.** The game keeps Bevy handles, asynchronous loading,
platform readers and device lifetime. Ambition can focus on authored metadata,
semantic names, publication, selection and editor operations.

**Tradeoffs for Ambition.** Bevy's asset APIs are not a complete Unity-like
editor database, versioned artifact store or remote update service. Ambition
must provide those parts when its products need them.

**Decision pressure.** Keep one Bevy runtime loader. Add focused tooling at the
selection and publication boundaries, rather than a competing asset runtime.

## Decisions this comparison informs

1. **Keep authoring, production, selection and delivery separate.** Godot's
   source/import split and Unity's artifact/catalog split both support this.
2. **Do not use a content digest as a logical asset ID.** Stable editor
   references, immutable product identity and physical transport have
   different jobs.
3. **Use metadata for browsing.** Unreal's Asset Registry shows why an
   inspector should avoid loading all media just to list assets.
4. **Use immutable bundles as distribution units for now.** Unity and Defold
   show that changing the selected content can be cheaper than rebuilding
   the executable. Bundle granularity is a measured tradeoff.
5. **Keep import, preparation and release work independent.** A desktop demo
   may use a usable old artifact. A release build may run deeper validation
   and platform conversion, then reuse the result.
6. **Make the editor an optional client of domain-owned data.** Do not make a
   GUI or an editor database necessary for a headless run or native restart.
7. **Start delivery with an ordinary file transport.** A release selection
   can identify the required immutable bundles. Rsync can transfer them now;
   a future HTTP or IPFS source can use the same logical references.

These are design directions, not a new implementation mandate. The
[planning owner](../planning/engine/asset-library-editor-and-delivery.md)
records the staged work, acceptance criteria and unresolved choices.
