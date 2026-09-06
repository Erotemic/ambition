# Loading coordination, activation barriers, and supersession

**Checked 2026-08-07.** Ambition already has a contributor-neutral loading
coordinator that is meaningfully more than an asset progress bar. It should be
compared to mature async-loading systems now, because the current source has
explicit activation requirements, discovery, cancellation, supersession,
prefetch promotion, failure semantics, progress confidence and one-shot commit
authorization.

## The Ambition capability that already exists

[`ambition_load`](../../crates/ambition_load/src/lib.rs) deliberately does not
load bytes. Bevy and subsystem-specific loaders perform work; the coordinator
owns the semantic question **"is this destination allowed to become active?"**

The current model includes:

- `LoadPlanSpec` with explicit `supersedes` relation;
- named `LoadBarrierSpec`s whose work discovery can remain open or be closed;
- work classified as `RequiredFor(barriers)`, `Degradable`, or `Speculative`;
- per-work priority and optional authored weight;
- `Planned`, `Running`, `Complete`, `Failed`, `Cancelled`, and `Skipped` states;
- retryability plus separate player/developer failure messages;
- progress estimates that record their basis, confidence, provenance and whether
  the fraction may decrease as new work is discovered;
- promotion of streamable/speculative work into an activation barrier without
  discarding already-made progress;
- cancellation and supersession that make late completions irrelevant;
- a one-shot `authorize_commit` check that refuses unknown, unready, cancelled,
  superseded or already-authorized barriers.

See [`model.rs`](../../crates/ambition_load/src/model.rs) and
[`coordinator.rs`](../../crates/ambition_load/src/coordinator.rs). The tests pin
important semantics such as "superseded load cannot authorize commit",
"streamable work does not block until promoted", "forecasts keep facts and
estimates separate", and "late completion is ignored after cancellation".

This is already an **activation transaction coordinator**, not merely loading UI
state.

---

## Bevy `AssetServer` — byte/resource readiness substrate

Bevy loads assets asynchronously and exposes load state for assets and their
dependencies through `AssetServer`.

Sources:

- [`AssetServer`](https://docs.rs/bevy/latest/bevy/asset/struct.AssetServer.html)
  (Bevy API docs).
- [`LoadState`](https://docs.rs/bevy/latest/bevy/asset/enum.LoadState.html)
  (Bevy API docs).

### Comparison

Bevy answers whether an asset or dependency tree loaded. Ambition's coordinator
should continue to sit **above** that answer. An experience activation may also
wait for content compilation, world conversion, save restore, shader/pipeline
warmup, network/session facts or generated runtime artifacts that are not one
Bevy asset tree.

The current source therefore has the right split:

```text
contributors say what work exists and report state
        ↓
ambition_load derives barrier readiness + commit permission
        ↓
shell / room transition / restore decides what activation means
```

Do not make `ambition_load` another `AssetServer`.

---

## Unity Addressables `AsyncOperationHandle` — operation lifetime and progress

Unity Addressables returns `AsyncOperationHandle` values for asynchronous work.
Handles expose completion/failure and progress; `PercentComplete` is an equally
weighted view over sub-operations, while download status can report byte
progress.

Source: [Asynchronous operation handles](https://docs.unity3d.com/Packages/com.unity.addressables%401.21/manual/AddressableAssetsAsyncOperationHandle.html)
(Unity, official).

### Comparison

Unity is strong prior art for explicit operation identity/lifetime and for the
fact that **progress is not one universally meaningful scalar**. Ambition has
already reached the same conclusion more explicitly: its estimate says whether
it came from equal steps, authored weights or mixed weights, carries provenance
and confidence, and may be absent while discovery is open.

That distinction is worth keeping. An unknown amount of future work should not
be rendered as fake precision just because a loading screen wants a percentage.

What Unity does *not* supply as the central Addressables abstraction is
Ambition's contributor-neutral activation barrier spanning arbitrary engine
subsystems. That is the interesting engine-level layer here.

---

## Unreal Streamable Manager / Asset Manager — async handles, priorities, and retained loads

Unreal's Asset Manager uses a Streamable Manager for asynchronous object loading.
`RequestAsyncLoad` accepts priorities and returns a `FStreamableHandle`; active
handles can retain loaded objects. Asset bundles provide named sets associated
with primary assets.

Sources:

- [Asset Management](https://dev.epicgames.com/documentation/unreal-engine/asset-management-in-unreal-engine?lang=en-US)
  (Epic, official).
- [`FStreamableManager::RequestAsyncLoad`](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/Engine/FStreamableManager/RequestAsyncLoad)
  (Epic, official).
- [`FStreamableHandle`](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/Engine/Engine/FStreamableHandle)
  (Epic, official).

### Comparison

Unreal sets a high bar for actual asynchronous asset scheduling and lifetime
management. Ambition should integrate rather than duplicate such substrate-level
concerns in Bevy terms.

The Ambition-specific value is the **separation between completed work and
permission to commit a destination**. A superseded route can finish every
underlying request and still be forbidden to activate. Likewise, speculative
prefetch can become required later without starting a parallel loading state
machine.

That semantic separation is especially valuable for room transitions and shell
routes where user intent can change faster than asynchronous work finishes.

---

## Godot `ResourceLoader` — background request/status/progress

Godot's `ResourceLoader` supports threaded load requests, status polling and
optional progress reporting.

Sources:

- [`ResourceLoader`](https://docs.godotengine.org/en/stable/classes/class_resourceloader.html)
  (Godot, official).
- [Background loading](https://docs.godotengine.org/en/stable/tutorials/io/background_loading.html)
  (Godot, official).

### Comparison

Godot provides a compact API for the common "request resource, show progress,
retrieve it when complete" workflow. Ambition should make ordinary loading just
as approachable, but its internal coordinator is solving a broader orchestration
problem: multiple contributors, late-discovered work, route supersession,
activation-critical versus degradable work and explicit commit authorization.

The author-facing UX should hide that complexity when a single resource is all
that matters without deleting the richer semantic model.

---

## What Ambition already distinguishes

| Concern | Mature async-loading precedent | Ambition today |
|---|---|---|
| actual byte/resource load | Bevy AssetServer, Addressables, Unreal Streamable Manager, Godot ResourceLoader | delegated to contributors/substrates |
| operation identity | handles/async operations | stable `LoadId` + `LoadWorkId` |
| required dependency set | asset dependency graphs/bundles | barrier-specific `RequiredFor` work spanning any contributor |
| background/degradable work | preload/streaming | first-class `Degradable` work |
| speculative prefetch | project-specific scheduling | first-class `Speculative` work promotable without losing progress |
| route replacement | cancellation APIs/project policy | explicit plan supersession; stale plan cannot commit |
| readiness | operation complete / dependency loaded | closed discovery + all required work complete + active plan |
| progress | scalar/operation progress | evidence-bearing estimate with basis/confidence/provenance |
| activation | often caller convention | explicit one-shot commit authorization |

## Design work the comparison exposes now

### 1. Give contributors a stable adapter protocol

Asset, content, world, persistence, networking and pipeline warmup contributors
should all be able to describe work and report progress/failure through a small
public protocol without importing shell policy. The coordinator should remain
contributor-neutral.

### 2. Connect prepared-content and asset readiness through barriers

The content compiler can tell us what exact content/assets an epoch requires;
the asset layer can resolve/load them; the load coordinator can determine when
that requirement set authorizes activation. These should compose rather than
maintain three partly overlapping startup graphs.

### 3. Make cancellation/supersession propagate to expensive work where safe

The coordinator correctly ignores late completion today. The next product-level
question is which contributor operations are cancellable and which should be
allowed to finish into a reusable cache. Cancellation policy should be explicit
per work type rather than inferred from plan state.

### 4. Define barrier scopes for room streaming and restore

The source says shells, room transitions, save restores and future streaming can
all use the same readiness authority. Prove that claim with separate acceptance
fixtures. In particular, distinguish "enough to activate destination" from
"everything nearby is fully prefetched".

### 5. Expose progress as evidence, not decoration

Keep the current honesty rules in any public UI/API: no percentage when discovery
is genuinely unknown; show why the estimate exists; allow estimates to move
backward when new work is discovered. This is a small but meaningful quality
advantage over fake monotonic loading bars.

### 6. Feed load refusal/supersession into causal diagnostics

A failed or superseded barrier is exactly the kind of event authors will ask
"why didn't this route activate?" about. Stable `LoadFailure` and
`LoadCommitRejection` values should become causal/diagnostic facts without
making `ambition_load` depend on a presentation UI.

## What this comparison changed

`ambition_load` should be documented as an implemented engine subsystem, not as
loading-screen plumbing. Mature engines already solve async bytes and asset
handles well; Ambition's competitive opportunity is the layer above them:

> contributor-neutral work + honest progress evidence + explicit activation
> barriers + supersession-safe one-shot commit.

That model can unify shell routes, room changes, restore and future streaming
without turning any one loader into global authority.
