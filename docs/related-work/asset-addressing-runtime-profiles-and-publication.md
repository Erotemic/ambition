# Asset addressing, runtime profiles, and publication

**Checked 2026-08-07.** Ambition already has a logical asset catalog above Bevy's
loader and a separate publication boundary for generated runtime art. That is
mature enough to compare directly with Bevy asset sources, Unity Addressables
and Unreal Asset Manager.

## The Ambition capability that already exists

[`ambition_asset_manager`](../../crates/ambition_asset_manager/src/lib.rs) owns a
logical asset layer while deliberately delegating byte loading, handles,
dependency loading and hot reload to Bevy where appropriate.

Its current vocabulary includes:

- stable logical `AssetId`;
- `AssetManifest` entries with logical path, kind, source candidates, missing
  policy, cache policy, preload group, optional content hash and declared
  dependencies;
- `AssetProfile` runtime personas such as desktop loose/install, mobile bundle,
  web, bundled static, no-assets and headless;
- resolution of `(AssetId, AssetProfile) -> AssetLocation`;
- caller/game-owned asset sources layered over engine assets;
- a generated-sprite **publish/install boundary** that classifies generated
  outputs and refuses human-only diagnostics under runtime asset roots.

The crate explicitly says what it does **not** own: async loading, handles,
dependency machinery and hot reload belong to Bevy's asset system. This is a
good example of the engine adding policy without forking substrate machinery.

---

## Bevy `AssetServer` / `AssetSource` — the loading substrate Ambition should keep using

Bevy's asset system loads runtime values from asset sources, supports multiple
sources/readers and optional file watching/hot reload. `AssetServer` coordinates
loading and strongly typed asset storage.

Sources:

- [`AssetServer`](https://docs.rs/bevy/latest/bevy/asset/prelude/struct.AssetServer.html)
  (Bevy API docs).
- [`AssetSource`](https://docs.rs/bevy/latest/bevy/asset/io/struct.AssetSource.html)
  (Bevy API docs).
- [Hot asset reloading example](https://docs.rs/bevy/latest/src/hot_asset_reloading/hot_asset_reloading.rs.html)
  (Bevy example source).

### Comparison

Ambition's current division is sound:

```text
Ambition: semantic ID + profile/source policy + runtime publication hygiene
Bevy:     bytes + readers + handles + typed asset storage + watching/reload
```

The related-work lesson is therefore **do not grow an Ambition `AssetServer`**.
The useful engine work is everything Bevy intentionally leaves game-specific:
logical identity, platform/profile policy, required/optional behavior, content
provenance and packaging conventions.

---

## Unity Addressables — logical address plus location/dependency-aware async loading

Unity Addressables lets a developer request assets by an address and supports
asynchronous loading from different locations together with dependencies.

Source: [Addressables](https://docs.unity3d.com/6000.0/Documentation/Manual/com.unity.addressables.html)
(Unity, official).

### Comparison

Addressables is the closest product analogy for `AssetId` plus profile-based
resolution. It sets a higher bar in one major area: **dependencies participate in
runtime loading**, whereas Ambition's `AssetEntry.dependencies` are currently
informational.

Ambition should not reproduce Addressables' API wholesale. Its content compiler
already knows exact prepared-content dependencies and capabilities, which creates
an opportunity to unify asset readiness with content preparation instead of
having two unrelated dependency graphs.

---

## Unreal Asset Manager — semantic Primary Asset IDs plus bundle/load state

Unreal distinguishes Primary Assets that can be addressed by
`FPrimaryAssetId`. Asset Manager APIs load primary assets and selected bundles,
can preload recursively, and track load/bundle state.

Sources:

- [Asset Management](https://dev.epicgames.com/documentation/en-us/unreal-engine/asset-management-in-unreal-engine)
  (Epic, official).
- [`LoadPrimaryAssets`](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/Engine/UAssetManager/LoadPrimaryAssets)
  (Epic, official).
- [`PreloadPrimaryAssets`](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/Engine/UAssetManager/PreloadPrimaryAssets)
  (Epic, official).

### Comparison

Unreal demonstrates a useful separation between semantic asset identity and
runtime load state. Ambition already has the first half and coarse preload
buckets. The missing competitive product is a first-class **readiness contract**:
“this prepared experience requires exactly these logical assets; here is their
state; activation cannot commit until policy says the set is ready.”

That should integrate with the existing shell preparation transaction rather
than becoming a second startup state machine.

---

## Publication hygiene is also an asset-system concern

Ambition's generated sprite toolchain emits runtime images/records *and*
human-only canonical poses, labeled previews and debug artifacts. The
`asset_publish` layer explicitly classifies and installs only runtime artifacts,
and hygiene tests fail if diagnostic images leak back into runtime roots.

This has a parallel in mature engines' **import/build/cook** distinction: source
and editor artifacts are not automatically runtime payload. Ambition's current
implementation is small, but the architectural principle should be generalized:

```text
author/generator outputs
    -> classify / validate / fingerprint
    -> publish manifest
    -> runtime asset roots / packaged sources
```

The runtime should consume the publish result, not rummage through a generator's
workspace.

The separate [loading-coordination comparison](loading-coordination-activation-barriers-and-supersession.md)
explains why logical asset readiness should feed a contributor-neutral activation
barrier rather than making the asset subsystem own route/session commit.

## What Ambition already distinguishes

| Concern | Bevy / mature-engine precedent | Ambition's current answer |
|---|---|---|
| logical asset identity | Addressables address, Unreal PrimaryAssetId | `AssetId` |
| storage/source | Bevy `AssetSource`, remote/bundled locations | profile-ordered `AssetLocation` candidates |
| runtime persona | build/platform-specific configuration | explicit `AssetProfile` incl. headless/no-assets |
| byte loading | AssetServer/Addressables/Asset Manager | delegated to Bevy; Ambition resolves policy only |
| dependency readiness | mature systems load dependency sets | declared today, not yet authoritative |
| generated-output install | import/cook/build pipeline | explicit sprite publish manifest + runtime hygiene checks |
| headless asset policy | engine-specific | `Headless` can intentionally tolerate missing required visual assets |

## Design work the comparison exposes now

### 1. Unify asset dependencies with prepared-content readiness

Do not make `AssetEntry.dependencies` a second graph that can disagree with
content schemas. Preparation should produce the exact logical asset requirement
set for an experience/content epoch, then the asset layer resolves it for the
active profile.

### 2. Add an explicit load/readiness result

A caller or shell should be able to query stable states such as:

```text
Unresolved -> Resolving -> Loading -> Ready
                         -> Degraded(policy permits)
                         -> Refused(reasons)
```

Each refusal should name logical asset ID, resolved source/profile and content
provenance. This is more useful than leaking Bevy handle/load-state details to
game code.

### 3. Make publication manifests general

The sprite publisher is evidence for a broader generated-asset contract. Audio
banks, shaders, generated atlases and future processed content should be able to
use the same notions of artifact class, install manifest, integrity hash and
runtime-root hygiene.

### 4. Decide cache/integrity semantics for remote sources

The manifest already carries cache policy and optional content hash, including an
IPFS gateway placeholder. Before remote/CDN delivery becomes public, define who
verifies hashes, how cache invalidation interacts with prepared-content identity,
and whether a content epoch may resolve to changed bytes under the same logical
ID.

### 5. Preserve “game owns its art” as an SDK invariant

The consumer-source code already exists because a real external consumer exposed
this leak. Keep testing layered game-owned asset sources across desktop, web and
headless compositions so the engine's own asset tree never becomes an accidental
requirement for third-party games.

## What this comparison changed

The asset manager should appear in related work as an implemented engine policy
layer. The right competitive split is:

- **reuse Bevy for loading/hot reload;**
- **match Addressables/Unreal on logical identity and dependency/readiness UX;**
- **differentiate by binding readiness to deterministic prepared-content epochs
  and by making generated-art publication/hygiene an explicit build contract.**
