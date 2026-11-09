# The queue — live execution order

This file contains **executable, unresolved engineering work**, not a work log.
An item remains here only with a reproducible defect or concrete customer,
a next action, and a checkable acceptance condition. Re-measure the source
before starting a row; retire it upon completion. Focused owner documents
carry architecture, measurements and implementation history; Git history
carries closed campaigns. Unselected or customer-gated plans live in
[`tracks.md`](tracks.md), and product questions in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

## Current order and selection

1. **Network architecture (maintainer priority, updated 2026-11-06):** [NET-AUTHORITY-EXPERIMENT](#net-authority-experiment--compare-hosted-world-replication-with-ggrs) compares player-hosted authoritative Ambition (with client prediction) against existing GGRS; Smash keeps bounded rollback. The [NET-INPUT-WIDTH](#net-input-width--remove-idle-room-consensus-from-every-frame) GGRS hot-path defect is still concrete; minimize that backend's packets without presuming it is the flagship's final protocol. Keep [GGRS island-history partitioning](tracks.md#conditional-ggrs-island-rollback) deferred until that experiment chooses a backend; `activeArea` physical continuity, loading-zone custody and local live portals remain relevant either way, and are covered by the authority experiment and existing multiplayer owners.
2. **Multiplayer correctness:** [MULTIPLAYER-SEAMS](#multiplayer-seams--make-local-multiplayer-safe-for-netplay) feature/tooling validation;
   then LOADOUT-AUTHORITY and LOCAL-PLAY-MODES menu leave. Honor the
   2026-10-19 ruling order for CHARACTER-FORM, COOP-RECOVERY,
   PARTNER-RECOVERY and DIALOGUE-CHECKPOINT; do not re-implement
   already completed morph-ball visibility, joining, party travel, baseline
   split-view or peer room-commit mechanics.
3. **Selected product/asset fixes (2026-11-01 triage):** ASSET-SELECTION,
   SMASH-DEFECTS, ITEM-CONTENT, ASSET-PUBLISH and ASSET-PREVIEW in that order.
   A9's completed profile proof is not an automatic invitation to rewrite the
   capability architecture. Continue AP14 ownership work before lower-priority
   ENEMY-NOTICE, WORLD-LAYERS and POSE-CONTINUITY; then ABILITY-VARIANTS and
   CHARGEN-SPECIES.
4. **NPC life and boss product work:** [RADIANT-NPC-HTN](#radiant-npc-htn--authored-npc-lives-and-replaceable-planning) tests text-authored HTN planning and persistent schedules. [BOSS-CINEMATIC-COVERAGE](#boss-cinematic-coverage--give-every-boss-a-wolfenstein-class-name-intro), [GNUTON-GNU2-FINALE](#gnuton-gnu2-finale--summon-a-second-gnu-through-a-real-portal), and [GNU-MOTION-PARTS](#gnu-motion-parts--diagnose-and-fix-non-smooth-giant-motion) are selected implementation customers; follow their owner plans and preserve existing shipped encounter phases until replacements pass.
5. **Developer iteration and inspection:** [GEOMETRY-OBSERVABILITY](#geometry-observability--render-the-effective-geometry-behind-gameplay) makes the effective geometry behind collisions, attacks, interactions, portals and navigation visible and machine-readable; it is a concrete inspection customer, not a broad spatial rearchitecture. [TEST-LANES](#test-lanes--keep-required-test-lanes-executable), CONTENT-AS-DATA, TOOL-DISCOVERY and
   EDITOR-SHELL remain selected at their stated boundaries. Portal, rendering,
   combat and map rows below are independent bounded work, not a second
   priority list; select them after correctness constraints are respected.

Current source may be ahead of these dated ordering decisions. Do not treat
old measurements as an obligation to redo completed work. Detailed
architecture proposals in
[the reassessment](engine/architecture-reassessment-and-composable-multiplayer.md)
do not by themselves authorize networking/physics/rollback rewrites.

**Automation/link contracts:** `scripts/checks/check_blocking_set_names_every_gate.py`
reads each `**Blocked by:**` field through its next blank line, plus
`**Acceptance:**` clauses. A heading containing `✅`, `DONE` or `CLOSED` marks
a closed row for the consolidation/hold guards. Keep the **ID-PEER** and
**A10** closed receipts below: the consolidation checks use their IDs and
state. Inbound `queue.md#...` links require retaining exact heading anchors
or updating their source. Check with
`python3 scripts/checks/check_planning_anchors_resolve.py`.

## P0 — architecture and correctness

### NET-AUTHORITY-EXPERIMENT — compare hosted-world replication with GGRS

**Customer:** persistent Ambition multiplayer, NPC lives, late join and different-room participation have different cost/authority needs than Smash rollback. The current 168-byte GGRS input and App-wide rewind may be unnecessary for the flagship's intended player-hosted experience.

**Owner:** [authority-model experiment](engine/hosted-world-networking-experiment.md) and [network topology](engine/network-topology-and-authority-research.md).

**Next action:** NA-1 benchmark the working GGRS path; NA-2 construct a minimal player-hosted authoritative two-region/guest fixture using existing body/seat authorities; NA-3 exercise prediction, interest subscriptions, NPC persistent state and late join; NA-4 decide per-game backend and update the applicable ADRs. Evaluate compatible Lightyear and `bevy_replicon` APIs without declaring a dependency before the prototype.

**Acceptance:** runnable headless comparison, byte and CPU measurements, stable guest input/reconciliation, region-scoped replication, persistent host-owned NPC facts, no service dependency, and an explicit per-game synchronization decision. Do not remove GGRS or rewrite working multiplayer merely from prose.

### NET-INPUT-WIDTH — remove idle room consensus from every frame

**Failure:** `PeerInput` is 168 fixed-width bincode bytes per player per tick: 72 bytes of `ControlFrame` plus 96 bytes of `PeerVerdict`, including full plan/content hashes even when no operation is underway. This increases every input stream's traffic irrespective of active room crossings.

**Owner:** [island rollback and compact peer input](engine/island-rollback-and-compact-peer-input.md#net-input-width-reduce-serialized-per-frame-input-first), [netcode](engine/netcode.md#the-input-payload-two-peers-exchange) and [peer room commits](engine/peer-room-commit-protocol.md).

**Next action:** NPI-1 measure and trace all verdict consumers; NPI-2 move rare room-preparation data out of the per-frame hot path while retaining frame-stamped, replay-stable agreement; NPI-3 migrate the wire identity, exact-byte tests and multi-handle fixed width. This task is **independent** of region-scoped rollback and should land first.

**Acceptance:** normal input is at most 80 serialized bytes per player per frame (72 if no small in-band token is required), without the repeated 96-byte verdict; every delayed/corrected/concurrent peer crossing and failure case still agrees on the exact frame with no reintroduced global hold. Source and actual link traffic are measured separately; a changed wire identity refuses incompatible peers. Checkpoint/rebuild barrier behavior is preserved.

### RADIANT-NPC-HTN — authored NPC lives and replaceable planning

**Customer:** civilian NPCs need schedules, commitments, social interactions, smart-object affordances and meaningful offscreen life; the current `CharacterBrainTemplate` policy catalog is closed. Nikobolt and Coulomba share one laboratory and are the initial integrated customer.

**Owner:** [Radiant NPC planner](engine/radiant-npc-planning.md), [agentic character runtime](engine/agentic-character-runtime.md).

**Next action:** compare `bevy_htn` authoring and `bevy_bae` execution to a standalone HTN core with the same benchmark; choose an engine-independent typed planner boundary. Implement an exclusive workbench, goal/plan interruption, and offscreen persistence without bypassing world facts or typed actor actions.

**Acceptance:** both scientists follow different authored schedules, share an exclusive lab affordance, react/replan, persist on departure and reentry, explain their plans and remain correct under save/reload and hosted authority. Change their activity policies without an engine-host compile. Do not require LLM inference or all-peer rollback.

### BOSS-CINEMATIC-COVERAGE — give every boss a Wolfenstein-class name intro

**Failure:** a generic boss name card and intro cutscene trigger exist, but name-card opacity is driven by individual `BossStage` requests (Wolfenstein/Inugami explicit), and the shared cutscene library does not provide every boss an intro. A roster-wide guarantee is absent.

**Owner:** [boss cinematic and GNU2 plan](game/boss-cinematics-gnu2-and-motion.md), [boss content](game/bosses.md).

**Next action:** census instantiated boss IDs and intro pathways; add an encounter-owned default title-card beat that leaves unique conductor cinematics intact; correct view/room ownership for multiview and repeated boss attempts. Test all configured bosses rather than only Wolfenstein.

**Acceptance:** each active boss gets a readable cinematic name/title reveal, at correct intro phase and view, with safe skip/replay/cleanup; bespoke intros still work without double presentation. Roster census and rendered witnesses cover multiple non-Wolfenstein fights.

### GNUTON-GNU2-FINALE — summon a second GNU through a real portal

**Failure:** GNU-ton's phase-3 well uses a real floor/overhead portal pair but changes the existing hall's sky and gravity to represent space. The preferred new finale is a large live portal summoning physically present GNU2, followed by simultaneous GNU and GNU2 combat. GNU2 art is currently an unpublished art-only prototype.

**Owner:** [boss finale and acceptance](game/boss-cinematics-gnu2-and-motion.md) and [GNU-ton debts](game/gnu-ton-fight-debts.md).

**Next action:** register published GNU2 character and combat geometry; author a same-active-area portal staging source, actual through-aperture entrance, combined fight and multi-boss victory policy. Replace obsolete sky/space-phase authority only after replay, reset and exit gating pass.

**Acceptance:** physically observed GNU2 portal emergence, simultaneous independently damageable GNU/GNU2 giants with readable counters, either defeat order supported, door gating on both plus rider yield, no duplicate spawn on retries, and both bosses safely cleaned on teardown. Keep intra-island portal semantics and existing phase 1/2 gameplay.

### GNU-MOTION-PARTS — diagnose and fix non-smooth giant motion

**Failure:** GNU-ton's giant appears to move non-smoothly. It already has part-sheet assets, but the conductor pins mount animation rows separately from its `MountWalk` motion, rider and fist poses. Root/interpolation/part-pose timing has not been measured; missing runtime part use is a hypothesis, not a confirmed cause.

**Owner:** [motion diagnosis and fix](game/boss-cinematics-gnu2-and-motion.md), [semantic-part rendering](engine/semantic-part-rendering-and-ragdolls.md).

**Next action:** capture frame-by-frame mount motion, row clocks, rendered part transforms, portal clipping, rider and fists. Identify root, animation or interpolation discontinuity; unify the responsible pose authority or shared render adapter rather than applying per-boss correction offsets.

**Acceptance:** walking, turning, buck, rider movement and portal strikes are visually continuous and align with collision, footholds, hits and camera, across fixed ticks, relevant render rates, reset/replay and quality levels.

### LOADOUT-AUTHORITY — equipment changes enter through simulation input

**Failure:** abilities equip through `ItemEquipRequested`, but the menu still
puts held items in the hand and stows them outside the simulation tick.

**Owner:** menu effects, item input, held-item custody and body capabilities.
Design: [ability loadout](game/ability-loadout.md#open-for-bombs).

**Next action:** route held equips and stows through the existing stamped
item-intent path. Attribute the request to its participant/body and apply it
at the custody owner. Remove the host-side simulation writes; do not add a
second equipment channel for each menu or item.

**Acceptance:** Grid and Kaleidoscope use the same operation. Each participant
changes only its own hand/loadout. Replay and checkpoint restore preserve
custody, stock and re-arming without duplicated objects. A menu cannot equip
an item before the corresponding simulation input is accepted.

### ITEM-CONTENT — remove retired items without letting fixtures retain them

**Failure:** Javelin and Sealed Note remain in authored content partly because
older tests and the intro handover use them. Axe and Map Fragment removal is
already reflected in the grid. The maintainer requested removal and an item
usage list on 2026-10-30.

**Owner:** item content, held specs, world placements, intro dialogue and their
behavioral fixtures. Design: [ability loadout](game/ability-loadout.md).

**Next action:** list each remaining item and its real consumers. Remove the
retired named content from every owner. Replace custody/checkpoint fixtures
with an existing supported item and update the intro handover consistently;
tests are not a reason to retain unwanted named content. Recheck saved-data
handling before removing a serialized identifier.

**Acceptance:** the equipment and quest pages reflect actual content. No
placement, dialogue, recipe or save reader names a removed item. The retained
custody and persistent-world witnesses still exercise real production paths.

### ABILITY-VARIANTS — finish the authored loadout through shared capability seams

**Failure:** the loadout plan still has placeholder variants, no authored
upgrade fact, and hard-coded eye-laser origin measurements.

**Owner:** equipment preparation, body control slots, content modules and
character landmarks. Design: [ability loadout, open work](game/ability-loadout.md#open).

**Next action:** after LOADOUT-AUTHORITY, recheck the published eye-laser row
and add its visor landmark and sound at their source owners. Then support
Blink and stance replacement through the existing technique/equipment seam,
with one authoritative upgrade fact. Author Contextual Blink, Precision Blink
tiers and the remaining variants in that model, not named engine branches.

**Acceptance:** each real variant changes its declared slot on any supported
body. Replacing or removing equipment restores the prepared intrinsic kit.
A placeholder stays visibly unavailable until it has an admitted implementation.
Tiers, equipment and checkpoint/save state agree. Keep quality selection out
of gameplay authority.

### CHARGEN-SPECIES — species parameters and asymmetric overlay views

**Failure:** the overlay mechanism is built, but species heads still expose
mainly muzzle and ear choices. Mirrored left cells move asymmetric details
to the wrong side.

**Owner:** renderer `chargen` specification, species geometry, overlay data and
studio applicability. Design: [species knobs and overlays](game/chargen-species-knobs-and-2d-overlays.md).

**Next action:** add meaningful dog and pigeon parameters first, then the
other listed species. Keep parameter defaults, generation and studio controls
at their canonical owners. Add explicit left cells for asymmetric details;
reuse the existing overlay mechanism. New random draws follow existing draws.

**Acceptance:** head crops and posed clips show distinct valid shapes in every
view; heads stay attached and asymmetric features keep their side. Existing
seeds retain their established fields. Publish each changed product and its
quality variants through the current pipeline. No runtime shader redraws the
character or changes gameplay geometry.

### ASSET-SELECTION — one selected-file contract for runtime and tools

**Failure:** the inspector reads loose audio and font paths while the runtime
uses selected publications. Python packaging and Rust also differ on missing
publications and stale generation identities.

**Owner:** `ambition_asset_manager::content_library`,
`scripts/lib/content_libraries.py`, `tools/ambition_editor/asset_index`, and
provider library installation.
Design: [asset library review, A and E](engine/asset-library-implementation-review.md#a-selection-is-correct-in-the-runtime-but-not-uniform-in-the-tools).

**Next action:** define the read-only selected-file contract in the existing
Rust and Python readers. Separate selected membership from workspace inventory.
Use it for published-only sprites, music, SFX and fonts. Keep local staleness
advisory; preserve path-safety and unreadable-file errors. State any stricter
package requirement at the packaging boundary.

**Host registration landed in `fb1621179`:** providers import identities and retain the
host's selected source. Explicit conflicting selections still refuse. Guard:
`a_provider_import_keeps_the_host_selection_but_an_explicit_conflict_refuses`.
The native app with served-provider support and standalone served Mary-O step
successfully. Keep provider imports separate from explicit source selection.

**Acceptance:** shared behavioral fixtures produce the same selected membership
and origin in both languages. Published-only assets resolve in the inspector;
loose unselected copies remain visible as workspace products. A usable old
publication can run without a rebuild or a deep audit. A native host with
served-content support enabled admits its shared library once; standalone
served games still run.

### SMASH-DEFECTS — select-screen, respawn and result lifecycle

**Failure:** source review found off-page portraits that retain their old
rectangle, a default jump budget at respawn, a result clock without a pause
check, stale mouse state at route entry, and invisible page controls.

**Owner:** Smash select UI and route lifecycle, stock placement, and the shared
body reset API. Evidence: [SD-01 through SD-07](demos/smash-defect-review-2026-10-31.md).

**Next action:** reproduce and fix SD-01 and SD-05 through the one page layout;
then SD-02 through the body's authored jump policy; then SD-03 and SD-04 through
the existing pause and route lifecycle. Measure idle component change ticks and
portrait demands for SD-06 and SD-07 before selecting a performance change.
Keep the existing S0 combat work separate.

**Acceptance:** small-view mouse, touch and pad input reaches every visible
page; off-page children are hidden. Real stock loss preserves each fighter's
jump budget. Pause preserves the result delay. The first click after route
re-entry uses its actual pointer position. Report idle demand/change counts
and frame cost separately from correctness.

### ASSET-PUBLISH — selection does not audit old bundle payloads

**Failure:** acceptance hashes the source and copied product; selection calls
`integrity()` for every selected bundle, including unchanged bundles.

**Owner:** renderer publication store and library producer scripts.
Design: [asset library review, B](engine/asset-library-implementation-review.md#b-publication-works-but-checks-the-past-on-every-change).

**Next action:** record a small and a many-bundle baseline of time, bytes read,
hashes and stat calls. Accept new bytes in one copy/hash pass. Select from
accepted manifests, check ownership conflicts, and keep the atomic swap.
Move deep payload inspection to an explicit audit.

**Acceptance:** focused publication does not read or hash old payloads.
Concurrent independent publications survive the merge; a failed producer
leaves the previous selection usable. Stale parents remain advisory. Report
measured cost; do not add a new binary cache or store service.

### ASSET-PREVIEW — local composition and release audit are separate operations

**Failure:** linked web preview composition hashes every selected input and
then audits the composed tree as if it were a release.

**Owner:** `scripts/package/package_asset_guard.py` and web preview setup.
Design: [asset library review, D](engine/asset-library-implementation-review.md#d-the-web-development-packager-does-release-level-work).

**Next action:** measure the current preview operation. Add an explicit local
composition policy that uses selected manifests and links without a deep
payload audit. Keep the explicit strict release path.

**Acceptance:** local preview uses the same selected file set without hashing
all media. Missing required files have a useful diagnostic. Android, Steam
Deck and static web release packages still pass their final asset contracts.

### GEOMETRY-OBSERVABILITY — render the effective geometry behind gameplay

**Failure:** F1 currently draws a useful but hand-assembled set of world, body, combat, portal and feature gizmos. Some outlines represent base authored blocks or coarse AABBs rather than the resolved, subject-specific geometry that simulation actually tests; interaction reach and rejection conditions lack comprehensive visibility. New geometry domains do not automatically become discoverable debug layers.

**Owner:** [geometry observability and F1](engine/geometry-observability-and-f1.md), under the [inspection/workbench program](engine/inspection-diagnostics-and-workbench.md).

**Next action:** GOBS-0 enumerate producers and reproduce a missing/misleading outline; GOBS-1 install composable read-only geometry providers and a shared F1/headless view; GOBS-2 render effective per-body collision and contact decisions; GOBS-3 cover combat, interaction, triggers, navigation and portal pieces; GOBS-4 provide practical F1 selection/presets and interpolation comparison; GOBS-5 close coverage and cost regressions. Preserve the already working F1 and combat-view paths.

**Acceptance:** a single representative two-room lab exposes actual collision/hit/hurt/interaction/portal/navigation geometry with source identity, state and why-not reasons; a new provider becomes discoverable in F1 and headless inspection without modifying a central game-specific draw switch; overlays are room- and view-correct, off by default, and do no per-frame diagnostic work when disabled. Tests distinguish absent, filtered, intangible and unsupported geometry. Do not confuse an authored block outline with a subject's effective collision world.

### TOOL-DISCOVERY — find supported commands by developer intent

**Failure:** the generated tool index lists immediate tool directories. It does
not identify supported script commands or Cargo binaries, so a workflow still
requires broad source search.

**Owner:** `scripts/generate_agent_index.py`, `scripts/agent_query.py`, and the
packages that own each command.
Design: [tooling layout, T0-T1](engine/tooling-layout-and-editor-consolidation.md).

**Priority:** after the selected asset-contract and A9/AP14 ownership work.

**Next action:** inventory supported capture, moveset, LDtk, asset-inspection
and ECS workflows on current main. Record their canonical command, help,
inputs, outputs, mutation policy and build or binary-reuse requirement.
Identify path consumers before any move. Extend the existing generated index
from tool-owned descriptions and existing entry points. Keep internal and
experimental commands distinct; do not add a second command registry.

**Acceptance:** intent queries for screenshots, deterministic capture, move
tuning, room editing, published assets and ECS inspection return runnable
commands and their owner documents. Missing commands have useful diagnostics.
Discovery stays fast and does not require a GUI, Cargo build or asset audit.
No tool or submodule move is part of this slice.

### EDITOR-SHELL — promote the existing inspector without a second authority

**Failure:** the Asset Inspector and specialist tools have separate entry
points, with no common navigation for existing inspection workflows.

**Owner:** the existing `tools/ambition_editor` implementation and its CLI.
Design: [tooling layout, T2](engine/tooling-layout-and-editor-consolidation.md).

**Depends on:** TOOL-DISCOVERY and its path-consumer inventory.

**Next action:** promote the inspector as Ambition Editor under
`tools/ambition_editor/`, with minimal internal renaming. Preserve its asset
browsing, JSON commands, cached previews, serve behavior and authored review
notes. Link to at least one existing specialist tool through its supported
command. Update known callers in the same change; keep only deliberate stable
launchers. Do not merge tool environments or reimplement domain operations.

**Acceptance:** one editor opens the existing asset and review workflows and
reaches another inspection capability. Agents can obtain the same facts and
invoke that capability without browser automation. Review IDs, provenance,
selected libraries and stale-preview access survive. Inspection does not gain
a mandatory build, regeneration or full-library audit.

T3-T5 remain proposed in the owner document. Select one relocation or cleanup
batch only when the discovery inventory shows its owner, benefit and callers.
Keep unique capabilities regardless of use counts; do not add forwarding
wrappers for every moved script.

### ENEMY-NOTICE — complete first-sight beats on the existing brain paths

**Failure:** the dilophosaurus and other skirmishers have authored notice art
or sound but no notice interval in `SkirmisherCfg`. The enemy-pattern plan's
claim that all fighter-template enemies lack notice is stale: fighter
engagement already implements it.

**Owner:** `ambition_characters::brain::state_machine`, combat brain emission,
and authored character profiles. Design: [enemy patterns](game/enemy-patterns.md#not-yet).

**Next action:** extend the existing first-sight/beat contract to skirmishers.
Audit the named pirate and ninja profiles against fighter engagement; author
missing values rather than another fighter notice implementation.

**Acceptance:** first sight produces one authored pose and cue before attack;
loss and reacquisition follow the declared policy. Hurt interruption and
ranged volley timing still work. A body without notice data keeps its declared
policy. Use the same beat projection for every controller.

### WORLD-LAYERS — fold the remaining long dimensions into connected regions

**Failure:** the current Jungle world is 25792 by 1888 pixels; Magical is
17408 by 2688. Both still have long horizontal layouts. Travel portal kinds,
reach checks and same-dimension rules already landed in `c818c0801`.

**Owner:** `tools/ambition_ldtk_tools/specs` generators and
`game/ambition_map_assets`. Design: [world layers](game/world-layers-and-travel.md),
[region composition](game/world-composition.md),
[remaining room detail](game/world-detail-24h.md),
[Art dimension follow-up](game/art-dimension.md#ideas-for-later), and
[History dimension follow-up](game/history-dimension.md#the-road-ahead).

**Next action:** fold Jungle into surface, underground and canopy regions,
then add Magical's sky and local arch network. Reuse the existing portal and
gate machinery. Keep authored rooms editable in LDtk. Recheck old room-detail
counts before selecting missing props, secrets or region links. Then recheck
Art's remaining publication, cues and places against the live assets. Keep its
completed first region.

**Acceptance:** the flat map matches room placement; neighboring regions have
loops and distinct vertical connections. Real movement reaches their exits,
shortcuts and arches. Hidden rooms and permanent breaks survive save/load.
Review the semantic LDtk diff and capture the changed places. Do not add a
second travel implementation or turn the target aspect ratio into a test.

### LEVEL-TRUTH — authored level data in the map submodule, one owner per room

Jon, 2026-11-04: "we should avoid putting the specs for levels and dimensions
in the main repo... I also want to avoid the issue of having more than one
source of truth for level data... we should likely be putting the authored
content into that submodule."

**Done:** a generator no longer overwrites a room edited in LDtk
(`<world>.generated.json`, `level ownership status|adopt|release`,
[region composition](game/world-composition.md#contents-one-owner-per-room)).
The History generator moved to
`game/ambition_map_assets/ambition_content/generators/history/`, and its
area specs are no longer committed.

**Failure:** every other generator (`tools/ambition_ldtk_tools/specs/*/make_specs.py`:
art, castle, eureka, evil moor, magical, ninja village, pirate port,
primeval, gate network and the rest) and its committed area specs are still
in the main repo, about 2.5 MB with 238 commits of history. Their worlds have
no record yet, so the guard is inert for them until their next apply.

**Next action:** move each generator to `generators/<dimension>/` as History
did (its specs to the temp folder; `district_kit` stays in tools). Apply each
once so its rooms are recorded. Hand-written one-off specs (`*_area.ron`,
`*.yaml` edits) that already ran become LDtk-owned and are deleted.

**Acceptance:** `tools/ambition_ldtk_tools/specs/` holds only shared code and
checks; `level ownership status` names an owner for every room of every world.

### POSE-CONTINUITY — one visual transition path after the completed base poses

**State:** the foundational Smash art and wiring gates are complete. Phase 3
continuity and campaign-only coverage remain open.

**Owner:** sprite-sheet animation and rig presentation, with renderer-authored
visual bindings. Design: [expressive poses, Phase 3](demos/smash-expressive-fighter-poses.md#phase-3---improve-continuity-and-body-weight-polish).

**Next action:** measure clip transitions on one reference fighter. Establish
stable visual bindings only where that experiment needs them, then implement
compatible transitions and planted contacts on the shared presentation path.
Measure campaign sheet gaps before authoring more rows.

**Acceptance:** human and CPU bodies share the path; contact timing and sharp
attack poses remain readable. Check interruptions, rollback resets, quality
tiers and portals at game scale. Preserve Mary-O's classic home-game tall
poses. Visual rig work does not enable gameplay rig geometry.

### LOCAL-PLAY-MODES — two local modes, explicit join and menu leave

**Owner:** `ambition_input` (seating, `sources.rs`), Ambition's seating
declaration (`ambition_content::provider::declare_ambition_seating`), the join
road (`session/join.rs`) and the settings owner
([multiplayer](game/multiplayer.md), ruling Q153 2026-10-19).

**Built 2026-10-19 (slices 1 and 2):** Ambition's route decides a channel
plan, so devices never size the session (`ambition_route_seating`).
Single-player is `[Shared]`: the keyboard and every unclaimed pad drive player
one, the pad in use, also a pad connected late. The gameplay setting
`accept_local_joins` makes it `[Shared, Claimed]`: the second handle is
reserved from the start and idle until a pad that player one is not using
presses Start (`ambition_input::claim_local_seats`, `LocalSeatClaims`). The
claimed pad drives seat 1 only. The simulation joins a seat with no body on its
Start press (`session/join.rs`); Jump is not the gesture. A change of plan
rebuilds the local rollback session, with no restart. A standing claim keeps
its channel when the setting goes off. Witnesses: `local_play_modes.rs`,
`a_second_seat_joins_the_session.rs` (the rewound join; Jump builds nothing),
`a_joined_seat_has_a_view_and_a_hud.rs` (a real pad's Start, with two pads,
keyboard plus one pad, and a pad connected late; control: setting off),
`provider::tests::ambition_is_single_player_until_the_setting_accepts_a_join`.

**Built 2026-10-24 (first integration correction,
[local-coop integration](engine/local-coop-integration.md)):** the shipped
host builds a camera for each view the adaptive split opens
(`SplitViewRigsPlugin`, one capability for every host), and every player body's
picture follows its own pose and live room (`sync_visuals`). Witnesses:
`local_play_modes.rs` (left stick through the raw axis event; the split
camera's view, layers, screen effects and its own half of the display),
`room_placement_tests::each_player_visual_follows_its_own_body_in_its_live_room`.
The maintainer reports the co-op baseline now works in the main game
with two players (2026-10-24); see `MULTIPLAYER-SEAMS` for new playtest defects.

**Built 2026-10-28 (join placement, `1f0956abb`):** the common body recipe
now joins at a clear nearby position in the primary body's live room.
The search has a fixed limit and declines a blocked join. See the guards in
[local-coop integration](engine/local-coop-integration.md).

**Next action, in slices:**

3. **Leave:** pause-menu actions "remove Player 2" (Player 1) and "leave"
   (Player 2). Leaving releases the claim, retires the body by an explicit
   lifetime policy, drops or returns what it holds with no duplicate and no
   loss, closes its view and HUD, and resets no room. A later join works.
   Measured 2026-10-19: Player 2 cannot open a menu. Ambition's grid menu
   reads `MenuControlFrame`, which only the primary seat fills, against the
   2026-08-06 ruling "any seat may pause, and the seat that paused drives the
   menu". The leave slice makes the menu answer to the seat that opened it.
   Lifetime policy chosen: the body retires, and what it holds or carries is
   put down where it stood (the bag is session-wide, `OwnedItems`), through
   `HostIntentWriter` so the leave is a stamped simulation tick.
4. **Presentation:** the join opens the two-player presentation; the adaptive
   merge stays (Q162). Nice to have: a portal that Player 2 drops out of,
   presentation only.

**Acceptance:** the controller and multiplayer tests of the 2026-10-19
assignment (section 11), each a simulation-level witness with a negative
control.

### MULTIPLAYER-SEAMS — make local multiplayer safe for netplay

**Owner:** [local-coop integration](engine/local-coop-integration.md) and
[peer-room-commit protocol](engine/peer-room-commit-protocol.md), across
input, dialogue, view/UI, combat, room lifecycle, and rollback owners.

**State (2026-11-04):** the local isolation slices are built: participant-owned
dialogue, viewport routing, blink clocks, fixed/adaptive split, friendly fire,
shot allegiance, damage-box scope, and per-view diagnostics. Ordinary online
peer crossings now commit on the shared input timeline **without a global hold
or rebase**. Delayed-peer, corrected-combat, concurrent-loading, arrival-reset,
rendered combat, and input-timing witnesses pass (latest `dc303e611`).

**Next action:** complete the broader **feature and tooling verification** for
input-driven peer room commits. Reproduce a normal crossing and a failed or
invalidated preparation with two peers; audit exact content/plan identity,
participant-specific interactions, replay, view/UI ownership, input ordering,
and cleanup. Keep full-party restores/rebuilds on their distinct global
barrier. Do not replace the native room constructor or reinstate a hold for
ordinary travel. The [protocol owner](engine/peer-room-commit-protocol.md)
contains the case-by-case gates and prior witness names.

**Acceptance:** both participants can continue independent movement, combat,
dialogue, menu and view presentation while one crosses/loads; two delayed
peers agree on application frames and outcomes across rollback, and refuse
mismatched or changed preparation without corrupting source rooms. Run the
focused harnesses and a rendered two-controller main-game playthrough.

**Blocked by:** none.

**Separate work:** menu leave is LOCAL-PLAY-MODES; participant recovery is
COOP-RECOVERY and PARTNER-RECOVERY. Do not duplicate their lifecycle rules here.

### Music playback gaps — diagnosis ready; death gap cause open

**Owner:** [music playback diagnostics](../systems/music-playback-diagnostics.md)
and the game/audio integration path.

**State:** request-versus-Kira instance observations and audio-context warnings
exist (`534a92549`, `b170a9238`). Dinosaur treetops and tar pits adaptive
binding gaps were fixed (`0f5b40487`). The death/respawn gap has **not** been
explained by these repairs.

**Next action:** reproduce a visible death/respawn music gap with the
instrumented run; correlate request, selected source, decoder, Kira instance,
audio context and speaker-audible playback. Separate intentional silence,
suspension, mute and one-shot completion from defects.

**Acceptance:** a reproduced unintended death/respawn gap is repaired with a
regression witness, or the observed event is classified with concrete playback
evidence. Keep diagnostics independent of playback repair.

### GATE-STARTUP-FLICKER — diagnose the wormhole at startup

**Owner:** [Gate network](game/gate-network.md#startup-rendering).
**State:** OPEN. Jon reports flicker on desktop and Steam Deck. Warmed High and
Ultra captures show an opening flare and continuous steady rendering. They do
not prove that cold startup is correct.
**Next action:** capture the reported gate from cold startup and locate the
first incorrect frame. Check sprite readiness, animation ownership and the
shader layer before changing art.
**Acceptance:** the gate renders continuously after its opening on desktop and
Steam Deck. Keep intended opening animation separate from missing frames.

### CHARACTER-FORM — a change of form is the same body in the new form's geometry

**Owner:** the form application (`avatar/character_execution.rs`, the re-wear
road), `ambition_body_seed::PhysicalBaseline` / `BaselineBoundary`, the
construction paths (home body, NPC placement, summons, match fighters, joined
seats), and control projection (`control/authority.rs`). Ruling: Q159
(2026-10-19). Plans:
[controlled-character actor kernel](engine/controlled-character-actor-kernel.md),
[control authority](engine/control-authority-and-ai-policy.md).

**Current failure (measured 2026-10-06, Q159's table):** the player (a 30 by
48 box) wearing the companion dog is drawn 36 by 30 in the player's box,
while the dog NPC is built at 108.6 by 90.6 (the NPC's drawn quad; its
collider follows `standing_height` 64). `BaselineBoundary::Replacement`
applies no geometry.

**Read 2026-10-19 (the construction roads):** actors size through one
resolver, `BuiltGeometry::resolve` (`ambition_body_seed`): the posed sheet's
Idle box (`SpriteAuthored`), else the sheet's `body_metrics` scaled by
`standing_height`, else the placement box. The contradictions:

1. `BodySource::Explicit` sizes the home body, is only the placement box of a
   match seat (a sheet overrides it), and is ignored by the NPC, minion and
   enemy roads. No shipped character authors it (the catalog can express only
   `posed_body`).
2. A character with no body source (the dog, `npc_kernel_guide`, the
   pirates) is sized from its sheet as an NPC, and is 30 by 48 as a player,
   at the start and after a re-wear: the home body never reads the sheet.
3. At a re-wear, the prepared-body grant
   (`actor_spawn::character_body::grant_prepared_character_body`) resizes a
   `SpriteAuthored` body and keeps the old box for the others; their art is
   fitted to the kept box, and the hurtbox fallback and attack volumes follow
   that box, so they are undersized.
4. Two triggers: the gameplay half of a re-wear (`apply_worn_character_gameplay`)
   needs `RecharacterizeBody`, and the projection runs on
   `Changed<WornCharacter>`.
5. A re-wear never writes `AbilityBase`, so the new form's abilities do not
   arrive.
6. Keep worn form, home identity, driving authority, and custody separate.
   A form change must not replace the participant's driving or custody claims.
   Possession records and handoffs now apply to each participant seat.

Tests that pin the old behaviour and change with slice 1:
`a_body_drawn_from_its_box_states_the_quad_of_the_character_it_wears`
(`character_runtime/presentation/tests.rs`, "a re-wear keeps the box") and
`a_reworn_body_states_the_quad_it_is_drawn_at` (app tests). The doc of
`match_activation`'s `Explicit` seating test calls a projection that resizes a
live body "a second geometry authority"; the ruling makes the form's
configuration the input and the live body the authority.

**Built 2026-10-19 (slice 1, 86ae35083):** the prepared-body grant sizes a
form with no posed body from its sheet at its standing height
(`catalog_join::sprite_body_at_standing_height`, the NPC road's numbers), at
construction and at each change of form, and retracts it; damage stays; a home
body's base abilities are `HomeBodyAbilities::base_for` at build and at a
change of form. Witness `a_player_that_changes_form_takes_the_size_of_that_form`
(guide, dog, robot, guide; poison red at the guide's 30 by 48 against a built
40.4 by 65.8). Contradictions 2, 3 and 5 above are closed for forms with a
standing height. Review finding #8 (2026-10-19): the grant no longer sizes an
`Explicit` body from its sheet (`sheet_body_for`); it keeps its box, as before
slice 1. Explicit half-extents on a change of form are still slice 1b.

**Next action, in slices:** (1b) a form with no standing height (scaled by a
placement box today) and `BodySource::Explicit` get one answer on every road
(contradiction 1). (2) The construction paths
agree on intrinsic geometry for the same form. (3) Distinct operations over
shared infrastructure: transformation (same body, new form), possession
(another existing body, same participant), skin (presentation only), body mode
(same body and form). Investigate the rename `WornCharacter` →
`CharacterForm` after the semantics are right.

**Acceptance:** the robot becomes the dog: the body is the dog's built size,
with the dog's moves and abilities, the same entity and identity, and its
damage kept; back to the robot restores the robot's geometry. A skin change
moves no collider, hurtbox or movement value. A possession leaves the original
body in the world, and the possessed body keeps its geometry and state.

### COOP-RECOVERY — independent respawn and a full-party wipe

**Owner:** `session/death.rs`, `sandbox_reset.rs`, the checkpoint restore
(`session/checkpoint.rs`). Rulings: Q151 amended and Q153 (2026-10-19, the
later row: independent multiplayer).

**Current state (the Q153 default, 2026-10-05):** a death sends back only the
primary's room, and only when nobody is in play there; a fallen seat 1 comes
back beside the primary; a fallen primary waits while a seat plays in its room.
The restore's subject is the primary body (`PrimaryPlayerOnly`), so the
operations are told apart by seat, not by the active set, and there is no
full-party-wipe condition as such. Ambition's death beat is 0 s
(`room_rules.ron`), so an individual return is immediate and a wipe needs two
deaths on one tick.

**Next action:** one recovery state machine over the active participant set,
symmetric for every participant:

- **Individual recovery (the default):** the participant respawns by the
  single-player respawn-location policy, for itself. With nobody else active
  in its room, that is a local room recovery: its room goes back to its
  checkpoint and the other participants' rooms and wins are spared (the Q151
  restore, with the dying participant as its subject, not the primary). With
  a living participant in its room, the room does not go back; only the
  participant returns.
- **Full-party wipe:** every active participant is out of play; the
  checkpoint restore spares nothing.
- A participant that left is not counted. The wipe reads death state now,
  not past deaths.
- **The wipe stays reachable:** the respawn waits a declared delay while
  another participant plays (a `room_rules` field); a partner that falls in
  that time makes the wipe.

**Acceptance:** same room, no room reset and the survivor untouched;
different rooms, room B and its wins unchanged; both dead, the checkpoint
restores; a removed participant not counted; a local recovery duplicates no
object.

### PARTNER-RECOVERY — an optional respawn beside a living partner

**Owner:** as COOP-RECOVERY, plus the settings owner. Ruling: Q153
(2026-10-19, the later row).

**Next action (after COOP-RECOVERY):** a configurable co-op policy, off by
default, that respawns a fallen participant beside a living partner (the
2026-10-05 return beside the primary is this policy, generalized to any
partner). The assisted-respawn presentation and UI follow it. Teleporting while
alive is a separate possible feature, not in this row.

### DIALOGUE-CHECKPOINT — story facts and their objects share one horizon

**Owner:** the checkpoint (`OwnedItemsBaseline`, Q124), the world fact store
(`world.set_flag`, quest steps) and the dialogue effects. Ruling: Q164
(2026-10-19).

**Current failure (measured 2026-10-09, `a_playthrough_of_the_persistent_world`):**
the player gives Alice's note to Bob, gets the survey, and dies before any
checkpoint: the survey leaves the bag (objects go back), and
`bob_field_survey_received`, the quest step and the wall it opened stay (facts
have no checkpoint copy). The object is lost for good while the world says it
was received.

**Next action:** (1) the checkpoint copies the participant's unbanked facts
(flags, quest steps) with the bag, and a solo death restores both; (2) an
authored dialogue effect that banks (a checkpoint action through the existing
persistence road, one transaction with the facts and objects it moves);
(3) in co-op, shared facts a survivor holds are not rolled back, and their
objects stay consistent; (4) dialogue history (seen, per participant, Q134)
kept apart from the world facts, so a seen marker grants nothing.

**Acceptance:** the dialogue arms of section 11: solo rollback consistent; an
authored checkpointing dialogue banks its consequences; a single co-op death
does not roll shared facts behind the survivor; a seen marker grants no reward
or quest step.

### GROUNDED-ATTACK-MOMENTUM — measure before changing the slide

**Owner:** [unified movement kernel](engine/unified-movement-kernel.md) and
the moveset (`MoveFrameData::roots_steering`). Ruling: Q169 (filed as Q166,
2026-10-19).

**Current state (2026-10-13):** a rooted grounded move takes the stick; each
law then brakes at its hands-off rate. The axis laws stop in about 3 px from
270 px/s; Sanic's surface-momentum law coasts 351 px from 530 px/s.

**Next action:** measure, for representative laws and moves: whether the move
takes the stick; whether it keeps horizontal speed; which braking law applies
while it plays; whether authored move motion adds travel; whether it can carry
its user off a ledge. Compare with Smash Ultimate (traction, dash cancels) and
Hollow Knight (Cyclone Slash, Dash Slash). Recommend; implement only what the
evidence justifies, as per-move data if a law cannot express it. The AI's
move commitment and ledge risk are separate work.

**Acceptance:** a measurement table in the kernel page, one witness per
distinguished fact (coast, brake, authored displacement, ledge carry), and a
recommendation.

### SMALL-VIEWPORT-UI — screens fit a phone in landscape

**Owner:** the menu backend (`crates/ambition_menu/src/render/bevy_ui/`) and
each pixel-laid screen. Ruling: Q165 (2026-10-19), screen by screen.

**Current failure (measured 2026-10-09 at 640x360):** the Smash select screen
shows 3 of 6 roster columns and no START; the dialogue box covers half the
screen; the touch stick covers each menu's left side.

**Next action:** one screen at a time, state what it does when small (scroll,
fewer lines, a floor for text and touch targets), with a `capture_scene`
witness at 640x360 and 1280x720.

### CONTENT-AS-DATA — content still written as Rust

**Owner:** [fast iteration implementation](engine/fast-iteration-implementation.md)
and the owning game pack. The major migration is built: game text, item and
rules tables, stages, world manifests, props, published art references, demo
module specials, and ordinary pack reload now have authored sources. Keep
engine-owned defaults and code-derived structure in Rust; do not move them
only to increase a census count.

**Remaining executable slice:** on a *confirmed* game-authored value still
read once at startup, give its schema a `Restater` or move the reader to use
time. First reproduce the edit/reload gap; change one schema at a time and
delete any duplicate Rust value. The [packet catalog](engine/fast-iteration-implementation.md)
owns candidate ports and checks; recheck the current source rather than using
the 2026-10-16 declaration count as a new inventory.

**Deferred:** game-specific movement VFX overrides have an agreed optional
`room_rules` policy but no game currently authors a different look. Promote
only for such a customer; do not copy the engine's defaults into every pack.

**Acceptance:** changing the authoritative data reaches the next applicable
session without a Cargo rebuild; a malformed value names the source and field;
no duplicate Rust reader remains. Report edit-to-test time for the slice.

### TEST-LANES — keep required test lanes executable

**Owner:** [test consolidation action plan](test-suite-consolidation-action-plan-2026-10-26.md)
and [test corpus evidence](test-corpus-static-audit-findings-2026-10-26.md).

**Selected next action:** reduce duplicated Smash/Versus fixture setup while
preserving distinct behavior witnesses, phase boundaries and negative controls.
Remove obsolete diagnostics only after checking their actual callers. Use
current `AGENTS.md` verification policy: no test-count/source-count floors or
new meta-test machinery as a proxy for behavior.

**Uncleared verification (2026-11-04):** the last full feature run recorded
30 failures in five targets. Subsequent selected app cases (seven) and spawn
owner cases (14) passed after repairs; **those focused passes do not clear the
full matrix**. Re-run/triage on a machine with the required published art and
rendered music. On the sampled VM, Smash and Sanic first rooms could not
start because their music tracks were not rendered; classify missing assets
separately from code failures. Keep failure evidence in the lane ledger.

**Other precise follow-ups:** reconcile the inconsistent compile-ratchet
baseline (`ambition_geometry`, actor monolith, core) by deliberate measurement,
not `--update` bookkeeping; capture an assertion if the historical
`composes_through_the_sdk` in-company failure recurs. These are not grounds
for retries or enlarging fixed-frame waits. Optional broader cleanup belongs
in the owner plan, not as a second chronological task list here.

**Acceptance:** selected duplicated setup is removed without reducing tested
behaviors; required checks run with honest PASS/FAIL/NOT RUN classification;
the outstanding feature-matrix failures are reproduced, repaired or explicitly
classified against their prerequisites.

### AP14 — actor kernel decomposition

**Owner:** [`actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md)
and [`actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md).
Jon's current instruction selects the remaining decomposition. Move each
responsibility to its owner and keep one body, control, and action path.

| # | Item | Next step |
|---|---|---|
| AP14 | Complete actor-monolith responsibility separation | The kernel module graph is a DAG. Lifecycle, publication, materialization, items, world objects, device input, observation, and global schedules have explicit owners. The public `body` API owns execution; `control` owns accepted control. Room technique grants follow each driven body. Real simulation covers two-seat possession, reaction clocks, controlled-body removal, action continuity, one-step free motion before and after seat releases, and mounted motion through timed dismount under rollback. Audit residual responsibilities and public operations. Keep accepted control and body execution together. Preserve rollback, publication barriers, and the number of active systems. See the [decomposition plan](engine/actor-monolith-decomposition.md#dependency-first-extraction-sequence). |

**Built:** `d541442ba` installs gate and lock-wall effects through
`SessionRoomVisualsPlugin`. It removes the game-host registrations and their
obsolete exceptions. The owner tests and installation scan pass; the kernel
module graph remains a DAG. Do not add host-installation exceptions for these
effects.

**Acceptance:** each cut moves state, behaviour and installation together and
deletes the old edge; `python3 scripts/measure/measure_kernel_module_graph.py --scc --cuts --edges 80`
is run after it.

### I2/I3 — finish independent content authoring and safe reload

**Owner:** [extension model](engine/extension-model.md) and
[fast iteration implementation](engine/fast-iteration-implementation.md).

**Built:** app-scoped pack selection; admitted generation publication;
no-op/stale/superseded edit handling; reload for moves, characters, audio,
cutscenes and quests; native/WASM procedural modules; `actor_think` S1-S3;
held traversal modules; and the `body_contact` extension route. Do not
re-enqueue the old missing-reload-family list or repeat the completed brain
migration.

**Next action:** finish the specifically named `body_contact` customers:
Mary-O snake-shell behavior and the Sanic badnik contact behavior, including
toucher roll/contact harm through their existing owners. The plant is a brain
and pose customer, not a contact rule. Recheck which customers are still native
before migrating them.

**Acceptance:** authored changes publish exactly once at an admitted
generation; stale generations are refused; these customers run through their
supported module road without a native duplicate or second source of truth.

### A7 — make item occurrence authority explicit where it carries a real invariant

**Owner:** [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md).

**Current state:** `GroundItem` construction is sealed, but that seal does not own
occurrence creation. Death drops, match spawns and other roads still mint
identity/provenance/custody facts at their occurrence sites. Two mint sites
degrade on purpose and belong to the `ItemCustody` inventory leg:
`ambition_held_items`'s thrown-item mint and `puppy_slug_gun`'s minion mint.

**Concrete inventory customer:** [bomb pocketing](game/ability-loadout.md#open-for-bombs)
needs one occurrence-to-quantity exchange that the checkpoint can restore.
Record the received quantity and the object's retirement at the existing
custody/accounting owner. Prove conservation through throw, pickup, pocket,
death and save/load; do not add an unrelated grants mirror beside spends.

**Next implementation:** centralize only the occurrence decisions that share an
actual invariant (identity, custody, provenance, rollback ownership). Do not add a
generic request bus merely to reduce writer count.

**Acceptance:** reward policy consumes accepted occurrence outcomes and cannot
become an alternative minting authority; every remaining occurrence creator has
an explicit ownership reason.

### BRAIN — finish truthful fighter attack selection

**Owner:** fighter combat options and [brain behavior](engine/fighter-tactical-intent.md)
only where the current production option resolver needs geometry, not a
reactivation of the paused tactical-intent experiment.

**State:** whiff memory, startup-as-first-live-hit, pure-counter valuation,
stop-distance and coast-distance features are implemented. The 2026-10-13
Sanic rung-5 run still left the stage under grounded attack momentum; measure
and resolve that in GROUNDED-ATTACK-MOMENTUM (Q169), not by repeating a
fixed-range or stale-view brain workaround.

**Remaining roads, remeasure before implementing:**

1. Projectile launch value: a ranged move has `launch: 0` although its hit
   uses `HitKnockbackMagnitude::FeelScale`. Clarify the resolved launch and
   hazard-coverage features against real kit choices.
2. Placed traps: resolve the hazardous region relative to the attacker,
   rather than treating placement radius as attack reach.
3. Teleports: expose the resolved destination (`behind_nearest_foe`,
   `behind_gap`, aim) to the attack offer.
4. Difficulty ladder: measure `--rungs 3,4,6,7,8` and distinguish step size
   from the missing L3 planning step; shipped rungs set `rollout_depth: 0`.

**Acceptance:** representative CPUs choose movement-compatible attacks
across their authored kits without replacing attack admission with a second
rule, a reach floor, or fighter-specific evaluator exceptions.

### D72 — continue Smash parity from the inventory

**Owner:** [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md).

**Current state:** the inventory is the source of feature/parity truth. Do not
turn this queue row back into a chronological parity diary.

**Next implementation:** take the next inventory row whose policy is settled,
implement it on the production path, update that inventory row and add the
production acceptance witness.

**Blocked where applicable by:** product rows named by the inventory. (Q62
and Q89 were ruled on 2026-10-04: Q62 is the LDTK-SEMANTIC-DIFF task, and a
stand-in may keep an incomplete kit.)

### PORTAL-SIMILARITY — canonical parts and scaled mirror portals

**Failure / customer:** linked apertures currently collapse to their smaller
size so transit can remain an isometry; the N-piece partitioner and several
consumers still use rigid maps or specialized presentation/collision paths.
A GNU-ton multi-portal limb and a body partially crossing unequal apertures
need correct scaling, reflection and physically consistent clipping.

**Owner:** [portal parts, similarity, chirality, and numerical failure plan](engine/portal-parts-similarity-and-chirality.md).

**Next action:** continue implementation and testing on main (Jon, 2026-11-08).
Jon released main and requested the tested slot 1 work to be merged. All VM
CPUs are available. Finish live root transit, exact contact projections and
source admission for every mover. Follow the owner's
[implementation plan](engine/portal-parts-similarity-and-chirality.md#implementation-plan)
and [draft status](engine/portal-parts-similarity-and-chirality.md#draft-status).
PSIM-1A and PSIM-1B: prove canonical-material partition invariants,
add bounded similarity composition/parity, and make budget refusal stop the
source motion before extending transit, combat and presentation. Reuse the
existing `frame::thread` worklist. This is local to one live simulation room;
cross-island live portals are not an implicit dependency.

**Acceptance:** all five PSIM slices have production-path and deterministic
witnesses; authored mismatched apertures scale correctly in both directions;
part geometry, collision, combat and masking agree; reflection persists as
correct actor chirality after transit; bounded rejected numeric transforms
show the typed IEEE 754 effect without poisoning simulation or replay.

### PORTAL-STRUCTURAL-FX — sprite and particle effects ignore the panes

Jon, 2026-10-17: "it should not be structurally possible to add something
into the game that doesn't work with portals." Meshes built on the CPU now
have one road that folds them through pairs and out of far-side panes
(`ambition_render::folded_mesh`, used by every line look). The `fx.rs`
effects have none: particles, beams, impacts and slash sprites are drawn
whole at z 30 and up, over a pane on its far side, and are not cut at a
crossing. Measured the same day by reading `portal_compositing.rs` (they
carry no candidate marker). Next: give them that one road (a sprite effect
through the far-side and transit pieces by construction, or the folded
writer for quads), not a per-effect opt-in.


**Far side, done 2026-10-18:** a pane composites every top-level drawable of
its room drawn at or above the window (`publish_portal_compositing_candidates`),
not only the three marked kinds, so a particle, a beam or a slash behind a
pane is hidden and redrawn in its uncovered parts by construction. Two kinds
opt out: the compositor's own copies and the parallax layers (placed against
the view). Witness: `a_far_side_effect_is_composited_with_no_marker_of_its_own`
(red under the old marker filter, without the parallax opt-out, and without
the portal-room check). Cost (release, 2000 sprites, 1000 over the window,
one pane): 64.9 µs a frame, against 34.7 µs for the old rule, which took
only the 100 marked ones; with no pane 16.3 µs against 24.7 µs. A candidate
that stays one is written in place: an insert command for each was 179 µs. **Open:** an effect is still not cut at a crossing
(the transit pieces, `sync_portal_body_pieces`, take only bodies).

### MOUNT-RIDER-CUSTOMER — a shipped rider controls a shipped mount

**Owner:** `body/brain_command.rs` (`MountedBrainCache`, the source-only
arm) and the dismounted-rider road (`body/dismounted_rider.rs`).

**Ruling:** Q76 (2026-10-03): keep mount/rider support and give it an early
customer (a character riding the dog, or the robot commandeering a shark).

**Current state:** `MountedBrainCache` has no production constructor, so
`ControlClaimant::Mount` has no production writer. Nothing resumes the mount's
recorded brain when a ride ends; a mount death rebuilds a solo brain from
config.

**Next action:** author the customer, then specify and repair, each with a
witness: control transfer, the mount's previous brain, dismount and its
restoration, body and room lifetime, damage and death, participant ownership,
animation composition.

**Acceptance:** the customer mounts, the mount obeys the rider, and on
dismount the mount runs the brain it had before; the same after a rewind.

### RIG-IMPOSTOR-CONTAINMENT — a part-drawn body is drawn whole or refused

**The invariant:** every pixel a composited body draws lies in its cell. One
camera draws a whole page with no per-cell scissor, so a part past its cell
is cut from its own body and drawn into its neighbour's.

**Owner:** `ambition_render::rendering::actors::rigged` (`impostor_margin`,
`posed_reach`), `ambition_sprite_sheet::character::rigged` (`art_overhang`)
and `scripts/measure/measure_rigged_parity.py`. Plan:
[`engine/mary-o-part-realization.md`](engine/mary-o-part-realization.md).

**Scope since 37497a444 (2026-10-05):** a part-drawn body draws its parts in
the world, and uses an impostor cell only while something composes it
(`ComposedBodyDemand`: hit flash, portal pieces, deep dream, quasar) or its
frame fades as one picture. So the invariant applies to that composed road
only.

**Solved, by kind of draw:**

- A published frame (2026-10-05). The cell is the frame plus
  `impostor_margin`, which covers the flipbook's stated `art_overhang`. The
  oni leader's banner is drawn whole (`a_cell_holds_every_draw_of_its_body`).
- A tween's in-between (2026-10-06, review P2). `art_overhang` is a
  conservative envelope of every draw `tween_into` can make, taken when the
  flipbook is read: each moving draw is sampled finely enough that no corner
  moves more than half a pixel between samples, and that slack is added
  (`tween_overhang`). A part that turns between two frames reaches where
  neither frame does (`the_overhang_of_a_tweened_clip_covers_its_in_betweens`;
  the published corpus: `no_in_between_of_a_published_flipbook_reaches_past_its_overhang`).
  Measured on the 123 published flipbooks with a tweened clip: the in-betweens
  move the stated overhang of 12 of them, by 0.21 px at most; no cell changed.
- Parts placed by a `PartPose` (2026-10-06, review P2). A pose is in no file,
  so its reach is measured each frame (`posed_reach`). While it is more than
  the margin, the body is drawn directly, as a body whose frame fits no cell
  is: whole, and its readers see no image, with one warning. The hold that
  smooths composition demand smooths this too
  (`a_part_pose_that_reaches_past_its_cell_is_drawn_directly`).

**Open:**

1. `scripts/measure/measure_rigged_parity.py` still clips its oracle to the cell, so
   a cut part cannot read as a parity failure there. The clip was added when
   the cell was the frame plus 16 px (the oni leader's banner read as 1153
   wrong pixels of a correct draw). With the margin from `art_overhang` the
   clip should remove nothing. Remove it and run the harness on a GPU
   (`rigged_sprite_parity`); acceptance: the oni leader passes unclipped, and
   with `impostor_margin` poisoned back to 16 px it fails.
2. A posed body that reaches past its cell loses its composed readers (its
   hit flash overlay, its portal pieces) while it does. If a ragdoll must keep
   them, give the presentation a cell chosen from the pose's reach, or let a
   pose declare an envelope. No shipped body carries a `PartPose` today.
3. A two-body shared-page test that reads the pixels of both cells needs a
   GPU; the three proofs above are on the draw geometry.

### DENSE-MELEE-ROOM — author the dense-melee development room

**Owner:** content; first measured customer
[`engine/bounded-perception-and-attention.md`](engine/bounded-perception-and-attention.md#acceptance-for-the-open-increment).

**Ruling:** Q93 (2026-10-04): keep or create a deliberate dense-melee
development/stress room. It is a real scenario, not polished content.

**Current state:** no such room; the density script fakes density by widening
the viewport over the Hall's `stand_still` cast.

**Next action:** after the P1 correctness rows above, author one room (LDtk or
a generated spec) with many tactical-brain fighters in close melee, reachable
by an ordinary route and by `measure_perception_density.sh`.

**Acceptance:** in the room `kept` tracks population until the budget caps it,
and the census, frame time and camera framing are readable from one run.

### ASSET-PRODUCT-LAYOUT — runtime roots by tier; editor products apart

**Owner:** `ambition_asset_manager` path builders, `ambition_sprite_sheet`,
the publish scripts. Plan:
[`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md#products-are-laid-out-by-what-they-are-q82-q83).

**Ruling:** Q82, Q83 (2026-10-04).

**Current state:** full quality is the bare `sprites/` root and the reduced
tiers are suffixed siblings. ✅ 2026-10-15, the ultrapack's runtime product
is cut by runtime unit: only the pack-plan groups a runtime reader loads ship
(`runtime_groups`), 1.8 MB, was 695 MB; the whole pack is a source in the
renderer's cache (owner page).

**Next action:** audit the listed path builders, then move to
`<root>/{full,half,quarter,potato}/` in one change.

**Acceptance:** every tier is a named directory; the packager selects roots
and has no new exclusion; the packaged ultrapack holds only reachable pages;
the game draws the same at each tier.

## Receipts

Closed rows retained only where an open row, a script or an inbound link still needs their identity.

### PARTY-TRAVEL-POLICIES — configurable group crossings and a leader view

Done 2026-11-04 (`4cb723b8c`, `5ed9bb73a`): independent/leader/any/all-member crossing policies, bidirectional gate idle timing, native body transfer and leader view are implemented. Default remains independent adaptive play; keep camera geometry out of travel authority. Owner: [game multiplayer](game/multiplayer.md).

### SHOT-END-REPLAY — a brain that saw its shot end replays differently — ✅ DONE 2026-10-11

Done 2026-10-11: the replay inconsistency was a stale body-motion read on rewind, not shot-end identity. See [integration audit](triage/2026-10-11-integration-source-audit.md).

### A4 — separate control authority from body execution on the real schedule — ✅ DONE 2026-10-09

Done 2026-10-09: accepted participant control is separated from body execution on the real simulation schedule. See [simulation authority](engine/simulation-authority-and-determinism.md).

### AUTHORITY-POLISH — one owner per mechanical fact, and no mirror in the rollback kernel — ✅ DONE 2026-10-09

Done 2026-10-09 (C11): no second mirror is retained as an authority for the surveyed mechanical facts. Reopen only for a concrete duplicate. See [consolidation plan](consolidation/consolidation-plan.md).

### SETTINGS-ROLLBACK — a settings change reaches simulation only at an admitted rebase — ✅ DONE 2026-10-08

Done 2026-10-08: settings reach simulation only through admitted rebase. Keep per-scope settings out of unvalidated mid-session writes.

### AUTHORED-INTERACTABLE-STATE — facing gates, per-chest and per-pickup persistence — ✅ DONE 2026-10-08

Done 2026-10-08: facing gates and chest/pickup state use explicit per-object persistence. See [authoring and tools](engine/authoring-and-tools.md).

### ID-PEER — remove host-local lineage from peer-stable mechanical identity — ✅ DONE 2026-10-03

Done 2026-10-03: peer-stable mechanical identity no longer inherits host-local lineage. Preserve the local/canonical identity boundary; consolidation ledger and checks depend on this closed row.

### A10 — candidate world / last-good-world publication — ✅ DONE, DEMOLITION CLOSED 2026-09-16

Done 2026-09-16: candidate-world and last-good-world publication are complete. A failed candidate cannot mutate the live published world. Keep this closed row for the consolidation-ledger guards.

### A12 — finish move-contact attribution and reflection identity — ✅ DONE 2026-09-19

Done 2026-09-19: move-contact and reflection attribution use occurrence identity; an outcome for another/no occurrence grants no move credit. This A12 is not the separate frontier A12 flow-validation task.
