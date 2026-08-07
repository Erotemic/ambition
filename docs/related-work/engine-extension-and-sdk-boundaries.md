# Engine extension and SDK boundaries

**Checked 2026-08-07.** Ambition already has an unusually explicit distinction
between internal crates and the API a game is allowed to depend on. That should
be compared to Bevy plugin groups, Unreal modules/plugins and Unity packages as a
current engine-product concern, not left only in ADR 0031.

## The Ambition capability that already exists

[ADR 0031](../adr/0031-public-facade-is-the-compatibility-boundary.md) makes
[`ambition_platformer2d`](../../crates/ambition_platformer2d/src/lib.rs) the
semantic compatibility surface rather than a mirror of internal crates.

The implemented contract includes:

- public modules named for game-facing roles rather than crate topology;
- a compatibility promise at the facade, while inner crates remain engine
  implementation details;
- an executable module allowlist for the external consumer fixture;
- `PlatformerApp` assembly that owns engine-internal plugin/order constraints;
- installation into a caller-owned Bevy `App` rather than taking process
  ownership;
- a real external-consumer fixture whose failures have already exposed API and
  composition leaks;
- a generated/cross-checked SDK API reference and blind-agent acceptance tests.

This is already a serious SDK design method: **consumer evidence is allowed to
reshape the facade without requiring the internal crate graph to be the public
architecture.**

---

## Bevy plugins and `PluginGroup` — composable app logic in a caller-owned `App`

Bevy's `Plugin` trait packages app logic/configuration, and `PluginGroup` groups
plugins that can be added, reordered, disabled or configured through the app.
The caller remains the owner of the `App`.

Sources:

- [`Plugin`](https://docs.rs/bevy/latest/bevy/app/trait.Plugin.html)
  (Bevy API docs).
- [`PluginGroup`](https://docs.rs/bevy/latest/bevy/app/trait.PluginGroup.html)
  (Bevy API docs).

### Comparison

This validates `PlatformerApp`'s mechanical shape. Ambition should remain a good
Bevy citizen rather than creating a second application/runtime abstraction.

The distinction is **semantic curation**. Bevy intentionally exposes a broad
engine API and lets plugins depend on it. Ambition's public facade adds a tighter
contract for platformer games: a game should not need to know which internal
crate owns a room type, which plugin must precede an asset plugin, or which
resource a rollback driver writes.

That curated layer is product value, not anti-Bevy abstraction.

---

## Unreal modules and plugins — public/private source boundaries are explicit

Unreal modules distinguish public and private headers/dependencies; headers in a
module's `Public` directory are exposed to dependent modules, while plugins
package one or more modules as project/engine extensions.

Sources:

- [Unreal Engine Modules](https://dev.epicgames.com/documentation/unreal-engine/unreal-engine-modules)
  (Epic, official).
- [Plugins in Unreal Engine](https://dev.epicgames.com/documentation/en-us/unreal-engine/plugins-in-unreal-engine)
  (Epic, official).

### Comparison

Unreal demonstrates the mature-engine expectation that **visibility and
dependency direction are build-system concepts**, not just documentation.
Ambition is applying the same lesson through Cargo crates plus executable
allowlists/absence contracts.

Ambition should continue to differ in one useful way: the facade's public module
names should reflect gameplay concepts, not simply expose every internal module
that is technically linkable. A clean SDK can therefore coexist with aggressive
internal decomposition.

---

## Unity Package Manager — extension distribution and dependency resolution are product surfaces

Unity Package Manager is the engine's package-management system for project
dependencies, custom packages, editor/runtime libraries and content. Custom
packages have manifests and can be versioned/distributed independently.

Sources:

- [Creating custom packages](https://docs.unity3d.com/current/Documentation/Manual/CustomPackages.html)
  (Unity, official).
- [Package Manager](https://docs.unity3d.com/current/Documentation/Manual/Packages.html)
  (Unity, official).

### Comparison

Ambition does not yet need to clone Unity's registry/distribution ecosystem, but
this comparison separates **API boundary** from **package ecosystem**. ADR 0031
has made major progress on the first; a competitive third-party engine eventually
needs the second:

- versioned engine/facade compatibility;
- capability and content-pack dependency metadata;
- discoverable install/update workflow;
- examples/templates that compile against only supported surfaces;
- migration notes for facade changes.

The content-pack compiler already has capability IDs and schema versions that can
become part of that ecosystem instead of inventing a disconnected plugin
manifest format later.

---

## What Ambition already distinguishes

| Concern | Mature-engine precedent | Ambition's current shape |
|---|---|---|
| App composition | Bevy plugins/groups | `PlatformerApp` installs into caller-owned Bevy `App` and owns its internal order |
| Public/private implementation | Unreal module public/private surface | facade is the compatibility boundary; inner crates explicitly are not |
| Extension package | Unreal plugin / Unity package | content/capability modules are emerging, but distribution/versioning is not yet a full product |
| API enforcement | build/module visibility | external-consumer module allowlist + dependency contracts + SDK-reference checks |
| API acceptance | sample projects/docs | real external fixture plus blind-agent tests record the first internal file a consumer had to open |

The blind-agent gate is particularly distinctive. It treats “can a new consumer
build with only the public docs/API?” as an executable usability test rather
than relying on maintainers who already know internal crate names.

## Design work the comparison exposes now

### 1. Version the facade explicitly

ADR 0031 already names this obligation. Before calling the engine SDK stable,
define compatibility/version policy for:

- Rust facade symbols;
- content-pack schema versions;
- prepared-content fingerprints/session compatibility;
- save/replay schema;
- provider/capability contracts.

These versions need not all move together, but their relationship should be
machine-readable.

### 2. Define the third-party capability boundary

The content compiler has `CapabilityId`; `PlatformerApp` has deferred install
closures/modules. Specify what a third-party capability may contribute:

- systems/schedules;
- content schemas and lowering;
- actions/control prompts;
- observation rows;
- construction recipes or only higher-level declarations;
- diagnostics/causal fact kinds;
- presentation adapters.

The answer should preserve engine-owned ordering and deterministic construction,
not expose arbitrary insertion points because Bevy technically allows them.

### 3. Add minimal SDK distributions/features

A movement-only game, headless simulator or tools process should not need every
presentation/content subsystem. Measure feature sets against the consumer matrix
from ADR 0031 and provide supported minimal compositions without weakening the
single semantic facade.

### 4. Make composition failures structured

`HostStatus::Refused` already improved a silent startup failure. Extend that
pattern: composition/capability failures should carry stable diagnostic IDs,
provider/content identity and corrective hints so external consumers never need
to inspect internal resources to learn why installation failed.

### 5. Treat external consumers as a permanent conformance suite

Keep adding deliberately different consumers — noncombat, movement-only,
headless/RL, alternate content backend — and prevent the SDK from becoming shaped
only like Ambition-the-game. This is the strongest mechanism the repo currently
has for discovering accidental architecture leaks.

## What this comparison changed

Ambition's facade campaign is not merely cleanup. It already resembles the
public/private extension disciplines of mature engines while exploiting Bevy's
composable app model.

The competitive target should be stated as:

> Bevy-native composition underneath, a smaller semantic platformer SDK on top,
> and executable consumer tests that make internal implementation knowledge a
> failure rather than an undocumented prerequisite.
