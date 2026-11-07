# Performance and composition: findings and proposals

**Scope:** what makes the game cost more CPU than it must on a weak device, and
what keeps Mary-O and Sanic from paying for Ambition. **Asked by:** Jon,
2026-10-15/16 ("optimize by finding more elegant patterns, instead of chasing
milliseconds"; "run [approximations] by me first"; "Mary-O and Sanic [must] not
get bogged down by everything else that is in Ambition"). **Status:** dated measurement record, not a live run diary. The 2026-10-24 run closed.
Re-measure against the current build before selecting CPU work; follow-up choices and
target-device checks are below.

**Related pages (they own their plans; this page ranks across them):**
[system-composition-audit](system-composition-audit.md) (S1-S6),
[capability-and-runtime-composition](capability-and-runtime-composition.md),
[performance-and-iteration](performance-and-iteration.md),
[simulation-tick-rate-compatibility](simulation-tick-rate-compatibility.md),
[consolidation plan](../consolidation/consolidation-plan.md).

Each claim is **MEASURED** (a run, a census, a profile, or a read of source)
or **REASONED** (inferred). A number here has the run that made it; run it
again before you quote it.

## Run outcome and remaining checks (2026-10-24)

**Completed run:** main-world serial scheduling, dormancy of unused game/capability
systems, material and presentation reductions, room-look shader reductions, and
actor-kernel carve packets. The October 24 desktop A/B found a Mary-O main-world
frame wall of 12.98 → 6.74 ms and an Ambition main-world wall of 13.91 → 5.12 ms.
These are **dated desktop measurements**, not a present phone-performance claim.
The completed change-by-change run log is in Git history. Remaining
acceptance and conditional follow-ups (not a new active optimization campaign):

1. **Reprofile on a phone** with the run's changes, including the hub look and
   the `android` versus `android_dev` product build. Do not infer a device win
   from a desktop system-count reduction.
2. **Resolve the shipping tooling policy**: the October 23/24 census found
   developer tools still installed in the play build; `android_dev` includes
   `dev_tools`. A play build without debugging tools needs a product choice.
3. **Measure and decide material/gizmo costs** on a target device before a
   Bevy fork, removal of debug gizmos, or consolidation of Ambition's room-look
   materials. There is no standing generic optimization campaign.
4. **Repair multiroom moving-platform presentation**: the 2026-10-24 witness
   `the_visual_follows_its_set_while_a_second_room_is_live` was ignored because
   `sync_moving_platform_visuals` used `Single<…, With<RoomInstanceRoot>>`.
   With two live room roots, the pass was skipped. Each platform visual must
   resolve its own room. Revalidate the witness against current code, fix if
   still reproducible, and remove `#[ignore]` when accepted.

### F49 — The run's end, desktop A/B: main-world wall halved, the hub frame halved (2026-10-24)

Measured in the desktop VM. Mary-O main-world wall 12.98 → 6.74 ms; Ambition
13.91 → 5.12 ms. The Ambition hub's rendered frame p50 went 51.1 → 27.2 ms,
with a large room-look component. A phone/GPU A/B remains necessary.

### F48 — After the content gates: what is left is Bevy's and Jon's (2026-10-24)

The October 24 desktop census found 919 systems running in a Mary-O frame
and 993 in an Ambition frame (profiling build with census). Remaining
choices: Bevy gizmo groups (24 systems in the older census), Ambition-owned
material types on other games' routes, and debug tooling in the shipping
Android profile (38 systems). These counts must be re-measured before a
tradeoff decision. The multiroom platform-visual defect above was found in
this pass and is an independent correctness task.

### F45 — A run condition can eat a message its system never reads (2026-10-23)

**Standing safety rule:** a consuming `on_message` condition can advance the
message cursor while an enclosing schedule set is asleep. The completed fix
uses a non-consuming predicate (`a_message_waits`) where the message must
survive until the consumer runs. Keep the replay and reconstitution witnesses.

## Historical performance findings (not open work by default)

The October 16-24 profiles were deliberately collected to motivate the
completed dormancy, change-filter, presentation, terrain and scheduling cuts.
The run's underlying per-system traces, dated experiments, abandoned candidates
and code-review receipts remain in Git history. The table below retains the
mechanisms needed to interpret a **new** profile, not an assertion that every
old defect is still present.

| Historical findings | Evidence and shipped outcome | Reopening condition |
| --- | --- | --- |
| F1-F4, F8, F30-F31, F34-F37: foreign-game work | Capabilities and other games were running in profiles that did not use them. Scoped systems, dormancy sets, encounters, dialogue, held-use, fighter special gates and the `RulesScopeSet` reduced the running census through the October run. `bevy_light` visibility now sleeps when no light exists. | Current census shows a specific nonparticipating provider/capability still running |
| F3, F22-F23, F41: executor policy | `apply_executor_policy`/`serialize_main_schedules` completed the single-executor policy for main-world frame/fixed schedules on 2026-10-17. The standalone tick A/B improved markedly, but the GGRS hub was already serial in its simulation path. Do not redo D12. | New profile isolates an executor cost; preserve rollback schedule ordering |
| F6-F7, F13-F18, F20-F21, F26-F29, F43: identity/allocator/change churn | Leafwing `PressAll` clash allocations, false `Changed` writes, UI/rig slot refresh, parallax and gravity-visual respawns, wielded overlay churn and terrain sprites were addressed during the run. Terrain mesh chunks reduced Mary-O 1-2 sprite entities 3,423 → 1,250, but the phone median was not improved in that trial. | A fresh allocation/churn/change probe reproduces a named producer rather than reopening all old work |
| F5, F12, F19: repeated read-model and room lookups | In the October phone sample, read-model rebuilding was ~0.9% and live-room lookup paths ~1.9% of samples. No current cost claim; repeated simulation-tick presentation derivations remain a candidate. | Measured current draw-frame/replay/headless cost; approve the read-model lifetime policy (historical D2) |
| F10: composition hygiene | Named content in reusable engine crates is a dependency/ownership issue independent of measured CPU cost. | A real capability/customer boundary still violates composition ownership |
| F11, F40: phone bottleneck | The October phone was main-thread-bound, with the executor/conditions ~16% of that main thread in a pre-cut build. Renderer scaling was not justified by that sample. | Profile a current phone/game scene with CPU, GPU and render thread split |
| F14-F16, F20: change semantics | Rebuilding room masks or writing unchanged resources woke downstream change filters. Compare before mutating; do not gate a rewinding schedule solely on `Changed` because restored snapshots mark values changed. | New `changes_by_site.csv` identifies a current producer and rollback-equivalent fix |
| F17, F21: immediate-mode presentation | Room parallax, gravity-zone and wielded overlays were changed to persistent reconciliation. Other candidate presentations were identified (`item_visuals`, `projectile_visuals`, `mark_beacon`, `hud/declared`, portal visuals), but were not all measured as per-frame churn. | A new churn trace demonstrates a remaining concrete offender |
| F24, F33, F38: collision-world and body stepping | A 1-2 phone sample showed collision-world copies at ~1.6% of all samples, vs ~0.8% in 1-3. Desktop 1-2 stepped 26 actors at ~13.2 us each; a tried indexed block-query rewrite changed 420 → 412 us (noise) and was reverted. | A fresh body/kernel/collision profile justifies composition caching or exact at-rest body skipping |
| F25, F28, F39: hub and tooling overhead | Hub costs included per-frame systems, LDtk/Yarn and presentation; Tracy's system spans made a hall simulation tick ~5x more expensive in a dated profile. The later hall instruction reduction was 10.0 → 8.5-8.8 M/tick. | Use no-Tracy for absolute timing; use Tracy to rank systems, not as the shipping baseline |
| F32: terrain entity reduction | Chunk meshes reduced entities and transform work; the original phone median was unchanged. | Further static batching requires a current GPU/render proof, not an entity-count target |
| F45: message cursor safety | `on_message` under a sleeping set can consume an event without a reader. The completed `a_message_waits` repair is a standing semantic contract. | Keep regression witnesses when modifying run conditions or event consumption |
| F46, F49: hub look | Shaders/materials were simplified during the run; desktop hub frame p50 was 51.1 → 27.2 ms. | Target GPU capture shows a remaining shader or material bottleneck without a visual regression |

### F0 — A third of the phone's CPU is the executor's work for each system, outside any system

**Historical measurement, October 16:** a large share of the Pixel 5 CPU was
Bevy's per-system scheduling/parameter/archetype overhead rather than a slow
single Ambition system body. Standalone Mary-O/Sanic ran about 1,040 systems
per frame, including ~170 systems for unrelated capabilities. This motivated
scope sets, selective capability installation and the completed executor policy.
The *number* is not the current phone percentage; recensus before a new cut.

## Still-actionable candidates (unselected until measured)

These are **not** an active optimization queue. Keep each proposed lever
available for a reproduced target-profile hotspot and bounded A/B. Related
selected tasks continue to live in `queue.md`, with conditional work in
`tracks.md`.

| Historical proposal | Current disposition / specific work to preserve |
| --- | --- |
| P1 / C1-C7, P3: capability-specific installs | Continue provider-local profile work only where a real customer still runs irrelevant extensions: boss/wielded phase ownership, narrative ledgers/dialog view, mounts, stocks, capture, game profiles, standalone Dialogue/Smash dependency decisions. Profile and source-check before using the old ~55-115 systems/tick estimates. See [system composition](system-composition-audit.md) and [capability composition](capability-and-runtime-composition.md). |
| P2, P11, P18 | **Completed** for the serial main-world policy, in-repo input clash bypass, and Mary-O/Sanic `RulesScopeSet` move, respectively. An optional upstream Leafwing PR is an external improvement, not a pending local optimization. |
| P4: identities | Intern identity handles or use entity keys only if current string cloning/sorting is costly and lifetime-safe. Preserve `SimId` and live-room identity semantics. |
| P5 / D2: read-model lifetime | Consider building render read-models per drawn frame/by change instead of per sim tick; first define headless, rollback and trace reader behavior and verify no changed simulation result. |
| P6, P12: change-driven writes | Reconcile by input change; avoid hidden snapshot/load changes. Probe first, include the per-game `virtual_joystick::update_ui` compare-before-write follow-up if reproduced. |
| P7, P8: tools, gizmos and upstream costs | Measure installed shipping debug tools, material/gizmo systems and active LDtk/Yarn/Bevy query-state costs. Use the product decision on Android dev versus play builds; consider forks only after a measured bounded proof. |
| Other: renderer feature overhead | The old Mary-O route ran `bevy_pbr`/OIT work with no active 3D camera, and some Lunex UI groups had no public sleep set. GPU preprocessing was disabled on the profiled Pixel 5 but active on the desktop VM. Profile the **current** product/renderer and platform before touching shared Bevy plugins or removing visuals. |
| Other: idle hit-flash materials | A Sanic probe found `HitFlashMaterial` prepared frequently even at intensity zero because its `uv_rect` followed body animation. Before suppressing updates or hiding the overlay, prove flash and portal compositing remain correct for every visible view. |
| Other: hub repertoire and loading | A hub repertoire re-fold cost was seen mostly around room loads/power-ups, not steady state. Likewise loading presentation and unrelated UI systems were identified; do not optimize either without a new span trace demonstrating an actual target-path burden. |
| P9: content as data | Follow the existing content/authority owners. Do not charge a composition migration to CPU optimization without measured cost. |
| P10 / D10: phone render scale | Change visible resolution only if a current target GPU/render-thread profile requires it. |
| P13: keyed presentation reconcile | Extend retained keyed entity updates to a currently measured spawn/despawn churn site. |
| P14: collision-world composition | Share one composed world per room/tick or a layered no-copy view when repeated copies are proven costly and replay-equivalent. |
| P15: static visuals | Batch fixtures/unskinned blocks or hidden sprites only with measured rendering/visibility cost and preserved art equivalence. Terrain skin chunks are done. |
| P16: repeated allocations | Reuse buffers in demonstrated top allocation sites, including `Layered::near` if a fresh trace supports it. |
| P17: navigation | Bake/cache graphs by room geometry and body when current autoplay/NPC load customers need it; the older 1-2 phone graph took ~11 seconds to build. |
| Other: exact idle bodies | Preserve the F38 proposal: fingerprint a body's complete stepping inputs and skip only when a tick is provably a fixed point; compare skipped and stepped actors across rollback and all components. This is **not implemented** by the historical failed block-index experiment. |
| Other: moveset/roster cache | F39 suggested deriving `MoveSpec::frame_data` once per moveset and reducing `CharacterCatalog`/`Layered::near` work; require a fresh trace and benchmark before implementation. |

### Historical C1-C7 profile prerequisites (check before selection)

The original P1 proposal had seven bounded steps; these are preserved as
**candidate contracts**, not blindly re-opened implementation tasks:

1. **C1:** Boss/wielded extension phases (`technique_execution`,
   `boss_conduct`, `wielded_use`, `module_entity_tick`) should be installed by
   their capability owner; no omitted phase port may admit content. Avoid a
   second `BossCatalog` initializer and registry-free boss animation.
2. **C2:** Dialogue owns the seven narrative payload ledgers and
   `rebuild_dialog_view`, instead of a core SimView obligation.
3. **C3:** Mount pose, mount control and mount ports install only with mounts.
4. **C4:** Match stocks and match-finish zoom install only for a match profile.
5. **C5:** Grab/capture control chain and ports install with capture, preserving
   their schedule order and independent admission schema.
6. **C6:** Demo hosts use a declared profile (`ENGINE_PROFILE`/`for_profile`)
   whose supported schemas are checked at pack admission; benchmark any
   smaller profile against the shipped one.
7. **C7:** Remove standalone Dialogue/Grabs/Smash kit dependencies only after
   their authored content and behavior are accounted for (D3/D4).

For every currently missing capability, reuse the supported-profile census,
`Capability::ALL`, absence/admission probes, session-edge guards and full
rollback witnesses; compare with the completed
[system-composition audit](system-composition-audit.md) before making changes.

## Historical product choices (not new open maintainer questions)

The October run proposed visible or content-changing decisions. None is
promoted into the current maintainer-question list by retaining it here.
The current [decision ledger](../maintainer-decisions.md) and scoped owner
plans supersede older requests.

- **D1:** 30 Hz simulation would affect one-way/platform tolerance, hitstop,
  region size, NPC prediction and AI cadence. Use the dedicated
  [tick-rate compatibility plan](simulation-tick-rate-compatibility.md) and an
  actual target game before selecting; do not silently reduce tick rate.
- **D3/D4:** Omitting Dialogue or Smash/capture from standalone Mary-O/Sanic
  requires handling authored fallback dialogue, state exit and kit capture
  verbs. Keep their profile/customer dependency explicit; do not strip content
  just to reduce registered systems.
- **D5:** LDtk respawn delay would change reload timing; prefer the existing
  fixed query-state path unless a new upstream failure is reproduced.
- **D6:** Kira dynamic channels alter crossfades; require a music-authoring
  customer and listening/behavior acceptance.
- **D7:** Unknown boss IDs should be handled at content admission rather than
  by inventing a fallback identity; verify the current catalog boundary.
- **D8:** Observer-minted `SimId`s change the deterministic spawn point in
  the tick; prove replay/identity stability first.
- **D9:** Per-room gizmo trails should be retained or dropped by explicit
  local view policy, not by a cross-room global fallback.
- **D10/D11:** Internal render scale and an overrun tick catch-up limit affect
  visible rendering/time; select only with measured device behavior and an
  explicit gameplay policy.
- **D12** is completed (serial executor). **D13** was withdrawn as a local
  fork requirement; the input-clash bypass landed in-repo.

## Selection rule for further performance work

The October 17 priority order is obsolete: some items were implemented,
others were superseded by later probes. Use the newest run outcome above and
`queue.md` for active work. For an unselected optimization, reproduce the
cost on a current machine/profile, identify its source and compare the
smallest bounded change before reopening an old P-number.

## How to answer a cost question

| Question | Tool |
| --- | --- |
| Which systems run in a frame, by crate? | `AMBITION_PROFILE_CENSUS=1` + `AMBITION_SYSTEM_LIST=PATH`; `scripts/measure/system_census_table.py` |
| Which Bevy system takes the time (desktop)? | `scripts/measure/profile_desktop.sh` → `tracy_summary.md` |
| Where does the phone spend its CPU? | `scripts/measure/profile_android.sh all ...`, then `scripts/measure/symbolize_android_profile.py DIR` |
| Which line of OURS caused framework time (phone)? | `symbolize_android_profile.py DIR --ours` (needs `simpleperf-device-samples.txt`, which a profile made after 2026-10-16 has) |
| Which functions of ours cost the most, framework time under them included (phone)? | `symbolize_android_profile.py DIR --inclusive [--thread TID]` |
| How much phone CPU passes through one function (inclusive)? | `symbolize_android_profile.py DIR --under NAME` (repeatable; with `--thread TID`) |
| Which thread is saturated, and what does it do (phone)? | the `threads` table of `symbolize_android_profile.py DIR` (busy % of one core, by thread id), then `--ours --thread TID` |
| Which system allocates, and which code makes archetype churn? | `profile_desktop.sh --perf-probe` → `alloc_by_span.csv`, `churn_by_site.csv` (recipe `profiling.md` §1d) |
| Which code marks a component or resource changed every frame (wakes every `Changed<T>` reader)? | the same run → `changes_by_site.csv` |

## Measured baseline

**Phone (Pixel 5, hub with every game, Mary-O 1-1, commit 97ff2541e,
2026-10-16; `profiles/android-all-maryo-1-1`).** MEASURED.

- Frame time, 5 s windows: p50 24-46 ms, p95 37-131 ms.
- Threads: main 67%, the Bevy compute pool 22% (shares of the process; see F11
  for how busy each one is).
- Crates by own time: `core` 21%, `bevy_ecs` 13.2%, `mimalloc` 5.2%, `alloc`
  3.1%, `concurrent_queue` 2.2%, `fixedbitset` 1.2%, `async_executor` 1.0%;
  each crate of ours under 1%.
- Reading (REASONED): the time is in the framework and the allocator, and the
  framework does what our composition asks: about 1000 systems a frame, their
  access checks (`fixedbitset`), the task executor, and allocations. Which
  line of ours causes each part is the next measurement (`--ours` with the
  sample dump, and the `perf_probe`).

**System census (desktop debug, VM, 2026-10-16).** MEASURED.

| run | registered | ran in one frame |
| --- | --- | --- |
| standalone Mary-O, 1-1 | 1277 | 1036 |
| standalone Sanic, act 1 | 1264 | 1043 |
| hub Ambition, `basement_enemies` | 1964 | 1529 |

Systems that ran in each standalone game for capabilities it does not use
(Mary-O / Sanic): boss encounters 41/41, conversation 36/36, Smash grabs and
throws 14/14, Smash moveset 13/13, mounts 7/7, held items 9/9, match stocks
7/7, gizmos 36/36, Kira audio channels 30/30, developer tools and traces 18/18,
extension lowering 32/34. About 170 systems a frame (17%).

## Historical run receipts and how to interpret them

- October 16 Pixel 5 (`97ff2541e`, incremental profile): p50 24-46 ms,
  p95 37-131 ms per 5-second window. Some samples were confounded by
  scene/build differences; `android-profile` was incremental until October 16.
- October 17 Mary-O 1-2: 26 physics actor steps/tick vs 20 in 1-3,
  and higher collision-world copying. Indexing block queries into
  `Layered::near` yielded no significant change and was reverted. Deferred
  exact-at-rest skipping remains conditional, not a completed speedup.
- October 18 hall: Tracy instrumentation inflated a simulated tick sharply;
  use a `--no-tracy` capture for absolute costs. Several safe hash/read/early
  out changes reduced instructions ~13% in an interleaved headless A/B.
- October 23/24: the latest desktop main-world comparisons and hub-frame
  results are in F49 above. F48 is the later per-frame system census. Neither
  is a November phone/GPU acceptance result.

The removed F0-F49 per-probe narratives, code-review diary and historical P1
implementation sketch remain in Git history. Their obsolete line numbers and
exact old path references should not be used to select work without a current
source/profile check.
