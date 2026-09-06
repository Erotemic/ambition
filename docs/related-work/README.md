# Related work

**How other engines and systems solve the problems Ambition is solving, with
citations and source-level comparison.**

## Why this section exists

Jon, 2026-08-07, on Ambition's shell vocabulary: *"I wonder if we should start a
related work section in the docs to document how everyone else does it, and give
us a better reference for how we compare."*

Ambition is written from first principles on purpose, and that is a strength
until it becomes an excuse. A design argued only against itself has no way to
find out that a concept it invented already has a name, that a distinction it
collapsed is one three other engines kept, or that a subsystem already present
in source is quietly mature enough to benchmark against a much larger system.
This section is the outside view.

The second case matters now. Related work is not only for **future architecture**.
When the source already contains a real movement kernel, rollback-aware causal
facts, stable simulation identity, an observation boundary, loading barriers,
content compilation, app-local persistence roots, or an external-consumer SDK,
we should compare those implemented contracts directly. Otherwise the docs
understate what the engine is already attempting and miss the most useful design
pressure.

## What belongs here

A page per QUESTION, not per engine. The useful unit is *"how is this problem
solved elsewhere"* — a page per engine would be a tour, and nobody reads a tour
while making a decision.

Each page owes:

* **The Ambition concept it is about**, named in our terms, with a pointer to
  where we implement it.
* **What each engine/system calls it**, and — more importantly — *whether they
  have the concept at all*. An engine that does NOT split something we split is
  often the most informative comparison.
* **Citations that were checked**, with the date. See the rule below.
* **What it changed**, if anything. A related-work page that changed no decision
  should say so; that is a finding too.
* **The remaining design pressure exposed by the comparison**, grounded in what
  the source actually implements today rather than an imagined greenfield API.

## ⛔ The citation rule

**Every external claim carries a link, and the link was FETCHED, not remembered.**

This is not bureaucracy. The first draft of the vocabulary page below asserted an
Unreal URL option from memory; the Epic page it was attributed to does not
document it, and the real citation turned out to be a different page entirely
(the claim was true, the source was wrong). A confidently-wrong citation is worse
than no citation, because the next reader stops checking.

So: fetch the page, quote/check the relevant claim, record the URL, date the
check. Prefer first-party documentation and primary papers. Mark third-party
sources as third-party when they are genuinely the best available source.

⚠ **APIs move.** A citation is a point-in-time observation, same as everything
else in this repo. Re-check before acting on a detail, and prefer claims about
CONCEPTS over claims about spellings.

## Pages

### Vocabulary and future-facing design frontiers

* [Shell vocabulary: provider, experience, route](shell-vocabulary-in-other-engines.md)
  — what Unreal, Unity and Godot call the things our shell calls providers,
  experiences and routes. Checked 2026-08-07.
* [Participant input, control authority, and possession](participant-input-control-and-possession.md)
  — per-user devices, contexts, possession, spatial interpretation, local-N, and
  why view ownership should remain a separate relation. Checked 2026-08-07.
* [Actions, abilities, and temporal ownership](actions-abilities-and-temporal-ownership.md)
  — Unreal GAS, input interactions/combo recognition, Ambition's landed action
  and motion-technique seams, and the minimum shared lifecycle still worth
  designing. Checked 2026-08-07.
* [Deterministic simulation, rollback, and replay](deterministic-simulation-rollback-and-replay.md)
  — Photon Quantum, Unity and Unreal prediction compared with Ambition's
  headless/rollback contract and scenario-tooling frontier. Checked 2026-08-07.
* [Diagnostics, causality, and frame inspection](diagnostics-causality-and-frame-inspection.md)
  — Whyline, rr, OpenTelemetry and general profilers compared with Ambition's
  already rollback-aware typed causal log; cross-tick and `why-not` queries are
  the next distinction. Checked 2026-08-07.
* [Authoring, world composition, and deterministic preparation](authoring-world-composition-and-preparation.md)
  — prefabs/scenes/world layers versus Ambition's implemented content compiler,
  `PreparedContentPack` and transactional construction. Checked 2026-08-07.

### Implemented engine subsystems we should benchmark now

* [Movement kernels, character controllers, and collision](movement-character-controllers-and-collision.md)
  — Godot `CharacterBody2D`, Rapier KCC and Box2D compared with the existing
  `step_motion` kernel, sibling `MotionModel`s, gravity-relative frame laws,
  swept triggers and portal-aware casts. Checked 2026-08-07.
* [Reference frames, portals, and relativity](reference-frames-portals-and-relativity.md)
  — ROS tf2, ordinary game transforms and MIT OpenRelativity compared with
  Ambition's gameplay-level frame transforms, portal transit and proper/causal
  relativistic systems. Checked 2026-08-07.
* [Time domains, proper time, and simulation clocks](time-domains-proper-time-and-simulation-clocks.md)
  — Bevy/Unity/Unreal clock facilities compared with `SimClock`, `PlayerClock`,
  wall/presentation time, fixed `SimTick`, and entity proper-time scaling.
  Checked 2026-08-07.
* [Headless simulation and agent environments](headless-simulation-and-agent-environments.md)
  — Gymnasium and Unity ML-Agents compared with the real
  `Platformer2dSimHarness` build/step/reset seam and shared runtime composition.
  Checked 2026-08-07.
* [Simulation observation and read models](simulation-observation-and-read-models.md)
  — Bevy render extraction and UI MVVM/data binding compared with
  `ambition_sim_view` as one simulation observation boundary for rendering, RL,
  netcode, brains and tools. Checked 2026-08-07.
* [Stable identity, provenance, and reconstruction](stable-identity-provenance-and-reconstruction.md)
  — Bevy entities, Unity GUID/GlobalObjectId, Unreal PrimaryAssetId and Godot
  ResourceUID compared with Ambition's separate content identity, `SimId`,
  runtime handles and `SpawnOrigin`. Checked 2026-08-07.
* [Engine extension and SDK boundaries](engine-extension-and-sdk-boundaries.md)
  — Bevy plugins, Unreal modules/plugins and Unity packages compared with the
  curated `ambition_platformer2d` semantic facade and executable external
  consumer boundary. Checked 2026-08-07.
* [Asset addressing, runtime profiles, and publication](asset-addressing-runtime-profiles-and-publication.md)
  — Bevy assets, Unity Addressables and Unreal Asset Manager compared with
  logical `AssetId`, runtime profiles, source policy and generated-art
  publication hygiene. Checked 2026-08-07.
* [Loading coordination, activation barriers, and supersession](loading-coordination-activation-barriers-and-supersession.md)
  — Bevy/Unity/Unreal/Godot async loading compared with `ambition_load`'s
  contributor-neutral required/degradable/speculative work, open discovery,
  supersession and one-shot activation commit. Checked 2026-08-07.
* [Persistence, save compatibility, and confirmed side effects](persistence-save-compatibility-and-confirmed-side-effects.md)
  — Unreal/Godot/Unity persistence mechanisms compared with Ambition's explicit
  migration verdicts, non-destructive future/corrupt-file handling, durable
  replacement, app-local roots and confirmed-history autosave. Checked
  2026-08-07.
* [World IR, level authoring, and backend adapters](world-ir-level-authoring-and-backend-adapters.md)
  — LDtk/Tiled external formats and Unity/Godot editor-native tilemaps compared
  with Ambition's backend-neutral `RoomSpec`/placement/room-graph IR and adapter
  boundary. Checked 2026-08-07.

## Source-grounded comparison map

This pass through the source exposed several areas where the implementation is
already more specific than the competitive roadmap's feature labels. These are
not claims that Ambition is "done"; they are the contracts that deserve direct
benchmarks now.

| Existing Ambition contract | Compare against now | Why it matters |
|---|---|---|
| one frame-aware `step_motion` kernel with sibling motion policies | Godot CharacterBody2D, Rapier KCC, Box2D queries/CCD | benchmark controller ergonomics without surrendering policy plurality or gravity/frame laws |
| portal-aware transforms + relativity/proper-time crates | ROS tf2, engine transform spaces, OpenRelativity | treat frames as typed gameplay semantics, not ad-hoc vector conversions |
| explicit `ClockDomain`, `SimTick`, presentation time and proper-time scale | Bevy fixed time, Unity scaled/unscaled time, Unreal time dilation | make every timer/state machine name the clock it belongs to |
| real build/step/reset headless harness over the same composition | Gymnasium, Unity ML-Agents | turn deterministic game simulation into a stable agent/testing product surface |
| `ambition_sim_view` pure tick read models | Bevy extraction, Unreal MVVM, Unity data binding | one observation contract for rendering, RL, netcode, brains and inspection |
| `ContentId` + `SimId` + runtime `Entity`, with `SpawnOrigin` | Unity GUID/GlobalObjectId, Unreal PrimaryAssetId, Godot ResourceUID | keep authoring identity, simulation identity and allocator handles distinct and reconstructible |
| compiled canonical `PreparedContentPack` with lowered artifacts and fingerprint | prefab/scene import + cook/build pipelines | validation/runtime consume one semantic artifact instead of reparsing authored bytes independently |
| contributor-neutral `ambition_load` barriers | Addressables handles, Unreal Streamable Manager, Godot background load | loading completion is not the same fact as destination activation permission |
| rollback-labeled typed causal facts | Whyline, rr, OpenTelemetry, engine profilers | build semantic why/why-not/time-travel diagnostics rather than another flame graph |
| versioned save data + future-file preservation + confirmed autosave | Unreal SaveGame, Unity/Godot serialization/file APIs | persistence correctness includes compatibility and rollback side-effect policy, not only serialization |
| curated semantic facade + real external consumers | Bevy PluginGroup, Unreal modules/plugins, Unity packages | make accidental internal knowledge an executable SDK failure |
| logical asset IDs/profiles + publication hygiene | Bevy AssetSource, Addressables, Unreal Asset Manager/cook | add platform/content policy above Bevy without reimplementing its loader |
| backend-neutral world IR with LDtk and RON inputs | LDtk/Tiled, Unity/Godot tilemaps | prove authoring backends are source languages rather than runtime object authority |
| open motion-technique catalog over a rolling semantic direction buffer | Unreal combo triggers, Unity Interactions, fighting-game command buffers | make complex input recognition deterministic, content-owned and inspectable without becoming gameplay authority |

## Competitive design lens

The pages are not a checklist for copying large engines. They should make three
outcomes explicit:

1. **Integrate** where Bevy or mature engine tooling already solves a general
   problem well: async byte loading, renderer/editor-facing visualization,
   platform profiling, low-level asset storage, ordinary serialization.
2. **Compete** where a 2D platformer engine needs a complete conventional author
   experience: movement tuning, participant-local input, actions, level
   authoring, content reuse, loading feedback, save durability, rollback proof.
3. **Differentiate** where Ambition's architecture already enables a stronger
   contract: persistent participant/control authority, body-owned spatial
   interpretation, policy-plural movement over one kernel, exact clock domains,
   deterministic preparation, stable provenance/reconstruction, loading commit
   barriers, rollback outside networking, and semantic causal explanation.

The competitive roadmap is the binding plan. Related-work pages are evidence and
design pressure: recommendations here become architecture only when the relevant
plan/ADR/source contract adopts them.

## Highest-leverage open comparisons

| Frontier | Design still owed | Competitive/differentiating bar |
|---|---|---|
| Movement/collision | public semantic contact result; controller conformance matrix; declarative tuning; external motion-policy decision | Godot/Rapier ease for normal movement while retaining one frame-aware deterministic kernel for materially different locomotion laws |
| Participant/control | action-specific context arbitration; participant-keyed routing; synchronized frame policy | Unity/Unreal-grade device/context ergonomics without conflating participant, control authority, body, spatial frame, or view |
| Motion gestures/actions | tick-based gesture timing; richer authored tolerance; match/rejection evidence; minimum shared temporal record | combo/interaction ergonomics plus deterministic body-authoritative execution and `why-not` explanation |
| Frames/time | stable frame identity/inspection; portal/frame graph semantics; explicit timer clock ownership; proper-time interaction policy | make reference frame and clock domain ordinary typed engine facts rather than hidden global assumptions |
| Rollback/replay/headless | declarative scenarios; stable step result/schema; multi-agent participant routing; batched sims; correction-aware effect identity | deterministic rollback as normal test/RL/replay/debug contract, not merely multiplayer mode |
| Observation | versioned view schema; diff/change surface; corrected/predicted status; adapters for UI/tools | one stable read boundary rather than render/UI/RL each reaching into live ECS differently |
| Identity/reconstruction | alias/migration policy; lineage queries; reconstruction epoch semantics | explicit content/simulation/runtime identities with provenance that survives rebuilds |
| Diagnostics | cross-tick execution-qualified causal edges; structured rejection facts; corrected-history diff; thread-safe recorder | answer platformer-specific *why* / *why not* questions on the same evidence CI and headless tests use |
| Authoring/content compiler | reusable-definition surface; hot-reload transaction UX; schema evolution; printable source-to-plan lineage | prefab-like convenience lowered through one validated/fingerprinted prepared artifact and transactional construction |
| World backends | second substantially different importer; IR compatibility/versioning; source provenance through geometry | Unity/Godot authoring immediacy without making one editor's object graph simulation authority |
| Assets/loading | authoritative dependency/readiness graph; cancellation policy; barrier acceptance cases; cache/integrity semantics | mature async loading underneath, explicit cross-subsystem activation authority above it |
| Persistence | generic SDK payload/slot boundary; migration registry/goldens; durable host tests; shared confirmed-effect seam | preserve unknown/future state and never commit speculative history while remaining simpler than object-graph persistence |
| SDK | explicit compatibility/versioning policy; third-party capability seam; minimal feature composition; structured install failures | Bevy-native internals with a smaller semantic platformer surface and real external-consumer conformance |

## A useful review heuristic

When a new subsystem or refactor becomes substantial, ask four questions before
calling its design "ours":

1. **What existing system is the closest semantic comparison?** Not the most
   famous engine — the system with the same authority/lifecycle problem.
2. **What does Ambition already implement that changes the comparison?** Read the
   source before writing the related-work page.
3. **Where should we integrate rather than compete?** General machinery is not a
   differentiation opportunity merely because we can write it.
4. **What stronger contract could Ambition plausibly prove?** Prefer executable
   conformance, deterministic identity/provenance, causal explanation and
   headless parity over marketing adjectives.

That keeps related work tied to architecture and prevents both NIH design and
cargo-cult copying.
