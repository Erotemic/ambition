# Archived: LANDED tracks from `docs/planning/tracks.md`

**Moved verbatim on 2026-08-01, losslessly.** `tracks.md` opens by stating what
it is: *"This file is the live queue, not a completion ledger."* These sections
were marked landed IN THEIR HEADINGS and had no open (`▢`) rows, so by the
file's own definition they had stopped being queue.

The selection rule is deliberately narrow and checkable — a heading carrying
`**LANDED` / `**COMPLETE` / `GATE CLOSED`, no `first item` qualifier, and no open
row in the body. Two sections that LOOK landed were kept for exactly that
reason: *"0. Pay down the GGRS correctness debt — LARGELY LANDED"* and
*"8. Combat unification batch — (first item LANDED)"*. Partially landed is not
landed, and a rule that could not tell them apart would be the wrong rule.

⚠ **Nothing here is edited**, and nothing here should be. Reopen in `tracks.md`.

---

## 1. Quarantine external effects to confirmed GGRS frames — **LANDED 2026-07-21**

`ambition_engine_core::ConfirmedFrameBoundary` is the host's published answer to
"which frames can never be simulated again", and
`ambition_runtime::external_effects` is the mechanism that keys irreversible
work to it. **Deferral, not suppression** — the distinction is the whole track.

The sim's effect channel became an **outbox**: cleared at the start of each
advance, journaled at the end under the frame that produced it, released back
into the same channel once that frame confirms. Presentation consumers are
unchanged and unaware. Re-simulating a frame REPLACES its intents *including
with nothing at all*, which is the half a boolean gate structurally cannot
express and is what erases a phantom.

- ✅ **Classification** (`quarantine_presentation_effects` — the list IS the
  classification, pinned by `only_presentation_facing_effects_are_quarantined`):
  `OwnedSfxMessage`, `VfxMessage`, `ExplosionRequest`, `FireworksRequest`,
  `DebrisBurstMessage`. ⚠ **The work-list was wrong about `EffectRequest`** —
  all three of its readers are sim-side (`apply_effects` spawns hitboxes,
  `apply_summon_effects` spawns minions, `apply_enemy_projectile_effects`), as
  is `SpawnProjectile`'s. Deferring one would not quarantine an external effect;
  it would change what the simulation computes. The split is "who reads it", not
  "effect-shaped name", and erring permissive is a desync, not a duplicate sound.
- ✅ **Buffer by frame + session identity**; ✅ **release exactly once, in
  simulation order**; ✅ **discard the abandoned branch at `LoadWorld`** and
  invalidate on session replacement (a generation counter on the boundary).
- ✅ **`SfxEmissionGate` DELETED**, and the deletion is load-bearing rather than
  tidying: suppressing at emit time destroys the corrected sound before anything
  downstream can decide whether the prediction it replaces was ever heard.
- ✅ **Persistence** (`385a165ee`): the autosave is gated on the world holding no
  predicted state, and change detection is replaced by a comparison against what
  was last committed. The second is what makes the first safe — `is_changed()` is
  consumed by a system that ran and declined to write, so any run condition in
  front of it silently swallows real changes. Settings deliberately get the value
  comparison but NOT the gate (not rollback state, all writers menu-side); the
  reasoning and its expiry condition are recorded at the call site.
- ✅ **Forensic trace** (`2eb14ef9e`): rows keyed by `sim_frame`, corrections
  replace predictions in place. The old `simulation_pass_is_authoritative` gate
  was *neither* option the review offered — "authoritative" meant FIRST PASS, so
  a mispredicted frame kept its guess permanently. Anomaly detection and dump
  arming stay first-pass (a file write must happen once) while the rows inside
  the dump still get corrected, since the flush runs in `PostUpdate`.

**Two traps worth keeping.** `Messages::drain` takes both of Bevy's
double-buffers, so without the start-of-advance clear the previous render
frame's already-released effects get journaled again and replayed — poison-tested
by `without_the_clear_the_effect_would_be_replayed`. And registering the release
in `PreUpdate` is not enough: with no edge against `RunGgrsSystems` Bevy may
release *before* the advances, and the next clear then wipes what was just
handed to presentation, silently, because the journal already counted it. The
integration oracle found that one; the unit tests structurally could not.

**Exit (met, with one clause narrowed).** `app_it::effect_quarantine`: the same
input script on the same GGRS host, once never rewinding and once rewinding
every step, must deliver the same effects in the same order. Poison-tested —
disabling the quarantine yields 46 effects against 10, each sound roughly five
times over, the original bug in its observable form.

⚠ **Not claimed: a live mispredicted remote input.** A sync test resimulates
with the *same* inputs, so its correction always equals its prediction and
A-versus-B cannot arise there. The A≠B rule is proven against the real systems
in `external_effects/tests.rs`
(`a_corrected_frame_replaces_what_the_prediction_produced` plus the
produces-nothing variant). Proving it end to end needs two peers, and ggrs's
handshake is wall-clock gated (200ms sync-retry interval), so a live two-peer
test would be timing-flaky in a repo whose determinism doctrine forbids exactly
that. **Owed when the Matchbox transport lands** — the transport and the proof
are the same piece of work, and that is the honest place for it.

**Still open from this track, DEFERRED with #3 (Jon, 2026-07-24):** attaching a
Matchbox transport through the existing `install_session` seam (unchanged by this
work — no simulation system was touched) and landing the two-peer
predicted-A/corrected-B oracle with it. Both wait for the Super Smash Siblings
era; the seam is documented and stays untouched until then.


## 2. Build-graph hygiene (compile-time wins) — **LANDED 2026-07-19**

Deep-review §6. Landed:

- `ambition_menu` and `ambition_menu_kaleidoscope` now declare
  `default-features = false` with minimal feature sets (`bevy_ui` +
  `bevy_picking`; `bevy_pbr` + `bevy_ui`). **Measured result:** the
  `ambition_actors` build graph dropped `bevy_pbr`, `bevy_gltf`,
  `gltf_animation`, `bevy_audio`+`vorbis`, `mesh_picking`, `smaa_luts`,
  `tonemapping_luts`, `ktx2`, `sysinfo_plugin`, and `bevy_light` — the whole 3D
  stack that plain `bevy = "0.18.1"` was pushing into every build via feature
  unification, headless and CI included;
- `ambition_menu_kaleidoscope` is optional on the app, wired to the existing
  `kaleidoscope_menu` feature (its module is now cfg-gated to match), so
  bevy_lunex + bevy_rich_text3d leave non-cube builds entirely;
- `[workspace.dependencies]` adopted for serde/ron/thiserror across 30
  manifests, ending the ron 0.11-vs-0.12 split in our own tree. **Honest
  result:** the duplicate COMPILES remain, because ron 0.12 comes from
  `bevy_animation` and thiserror 1 from `bevy_ecs_ldtk` — transitive, not ours;
- deleted the dead `ambition_world → ambition_time` edge and made actors→ui_nav
  an optional feature-conduit. **Correction:** `sprite_sheet → interaction` is
  NOT dead (8 real path references) — the deep review was wrong there;
- deleted the vestigial `rl_sim` feature chain (actors' and the facade's copies
  gated nothing once the RL surface moved to `ambition_sim_harness`; the app's
  switch is the real one). `headless` stays: it is an intentional empty
  composite whose value is what it leaves off;
- **the trim exposed three crates that only ever compiled by accident**, because
  the untrimmed dep was donating features workspace-wide:
  `ambition_platformer_primitives` (needs `bevy_input_focus` for `KeyCode`),
  `ambition_game_shell` and `ambition_load_presentation` (need a windowing
  backend for the winit that `ui_api` pulls). All three now declare what they
  use — see `dev/journals/lessons_learned.md` (2026-07-19) for the pattern to
  expect on the next trim.

Remaining (own pass): `ui_api` is a bundle that pulls `bevy_animation` (unused
here) into ~6 crates; replacing it with explicit feature lists would drop that
plus ron 0.12. And `bevy` itself is still pinned in ~46 manifests — a
workspace-dependency conversion is mechanical but wants its own review, since
the per-crate feature sets legitimately differ.


## 2.5 Make `RoomReplayRequested` a real seam — **LANDED 2026-07-21** (`cf5095576`, `7743d224f`)

`ambition_runtime::sandbox_reset` now owns `reset_sandbox` and the ONE
`apply_room_replay_request_system`, carried to every host by
`RoomReplaySchedulePlugin` in `PlatformerEnginePlugins`. The two content anchors
(`ContentDialogueFollowupSet`, `ContentRoomReplayResetSet`) moved with it, since
the engine now owns the consumer they order against. Ambition keeps only its
reset-INPUT system (the button binding is Ambition's) with an explicit `.before`
edge to the consumer — the old `.chain()` was the only thing making those two
unambiguous.

**The blocker was a MODULE, not a dependency.** The card said the consumer was
stuck app-side because it called `reset_sandbox`, "a host/reset concern". In
fact `reset_sandbox` names only `engine_core`/`actors`/`characters`/`sfx`/`vfx`,
every one of which `ambition_runtime` already depended on; it was unmovable
only because it sat in `app::world_flow::room_flow`, which also composes
`load_room` with `ambition::render` spawns. Splitting the reset out of that
module is the entire unlock.

**Exit met.** Nine tests across the three hosts
(`ambition_demo_{mary_o,sanic}_app/tests/room_replay.rs`,
`ambition_app/tests/app_it -- room_replay_seam`): the seam itself, Mary-O's
TIMEOUT beat end to end, Sanic's act clear past the FULL `ACT_CLEAR_DWELL`, and
a one-request-one-reset count per host. Poison-tested both directions — dropping
the plugin fails all nine (Mary-O stays at her full 600px displacement, Sanic at
1060 vs a spawn of 160); re-adding a duplicate app-side registration fails the
hosted pair.

Two findings recorded rather than fixed:
- Sanic can clear the act and then coast off the end of the speedway into a pit
  death, inside his own 4s results dwell. That is why the act-clear proof stamps
  the cleared phase under controlled conditions instead of extending
  `act_completion.rs`: the death respawn rebuilds the room by itself, so that
  run cannot isolate a replay. Logged in `code_smells.md` (2026-07-21).
- A duplicate consumer is a hard Bevy panic in `ambition_app` (the reset-input
  `.before` edge cannot resolve against a twice-registered system) but SILENT in
  the demo apps, which have no such edge. Hence the count assertion.

Mary-O's open acceptance run can now assert the replay clause it was written
against ("waits through an actual replay into a fresh level").


## 3. Close Super Mary-O level 1 — **LEVEL-1 GATE CLOSED 2026-07-21** (`d92791435`)

The acceptance run landed: one state-aware controller plays spawn → ?-block →
milk → pit A → secret pipe → vault → 8 coins → return pipe → surface →
re-power → pits B and C → stair pyramid → pole → tally → a real replay back to
spawn, with no positional set-up and all three lives intact. **Nothing in the
codebase previously proved any pit was crossable.** Full clause-by-clause
account in [`demos/super-mary-o.md`](demos/super-mary-o.md) — single source,
do not copy back here.

Three bugs fell out of writing it, all invisible to the existing tests because
every prior proof either set her position past the terrain or asserted a value
the emitter wrote:
- the secret vault had **no working exit** (return pipe block derived from its
  interact band, floating it 48px clear of the floor). FIXED `cbc6902d2`. Its
  own "sealed vault" test stayed green by checking a body at the band's centre —
  inside solid rock — and the scripted seam run stayed green by teleporting her
  to exactly that unreachable point;
- **a body reset redefined the body** — `reset_body_clusters` hardcoded the
  default size into `base_size`, so any identity-driven size (a worn form, a
  mount, a boss phase) was silently unmade on every reset. FIXED `4e4bd0fd8` in
  `ambition_engine_core`; engine-wide, not Mary-O's;
- **pit B opens into the secret vault**. REPORTED, authoring call:
  [`triage/room-replay-followups-2026-07-21.md`](triage/room-replay-followups-2026-07-21.md) §5.

**Exit (met):** visible and headless customers use the same provider, body,
item, and level state with no Mary-O-only engine path.

**Now unblocked:** additional authored levels, which were gated behind this.


## 3.5 Room transitions are an ENGINE capability — **LANDED 2026-07-25**

`RoomTransitionRequested` had exactly one consumer, registered only by
`ambition_app`, so no demo host could change rooms — a direct hit on the oracle
and the reason Mary-O's secret vault had to be dug into the same `RoomSpec` as
her surface. The readiness transaction, one-shot authorization, and commit now
live in `ambition_runtime::room_transition`, carried by `PlatformerEnginePlugins`
into every host. The §2.5 shape repeats exactly: **the blocker was one CALL, not
a dependency** — the commit drew the new room itself (`spawn_room_visuals`), so
it named `ambition_render`; it now writes `RespawnRoomVisualsRequested`, the
channel the sandbox reset and the room stager already used.

Two host-owned contributors stay, both marker-gated so absence is honest rather
than silent: `RoomTransitionAssetContributor` (a headless host cannot answer
"did the room's art arrive", so its work item is `Skipped`, not pending forever)
and the existing cover gate. The neighbor prefetch split along the same line —
the prepared `RoomConstructionPlan` is an engine artifact promoted by engine
identity; the asset manifest belongs to the host that can name it.

Customer: Mary-O **World 1-2** ([`demos/super-mary-o.md`](demos/super-mary-o.md)),
proven by a run that PLAYS into it and by a ferry-ride proof. ⚠ The engine group
now supplies `AmbitionLoadPlugin`; five hosts that added their own copy were
panicking on a duplicate.

Open: cross-room continuity (score/coins/lives/worn power).


## 5. Provenance + three-origin `ConstructionPlan` vertical slice — **LANDED 2026-07-22**

Full account in
[`engine/immutable-content-and-transactional-construction.md`](engine/immutable-content-and-transactional-construction.md)
Phase 3 — single source, do not copy back here. Headlines:

`ambition_platformer_primitives::construction` is the content-free planner;
`ambition_actors::construction` puts the three real origin families through it
(authored `GroundItemSpec`, provider-staged `SpawnActorRequest`, `Effect::Summon`
minion). Every exit clause met, each with a named test.

**The result worth remembering is that provenance stopped being a spelling.**
`SpawnOrigin` is a snapshot-registered component; the one place in the tree that
parsed a `SimId` (`heal_projectile_owners`, `rsplit_once('/')`) is deleted. Two
stale claims fell out of doing it and are corrected: `ProjectileOwner`'s
registered derived-state justification named a field that is EMPTY for every
player projectile, and `SimId::as_str` documented itself as "never parsed" while
being parsed one crate away.

Three failures that were silent skips are now preflight failures — an authored
ground item naming an unregistered held item, a staged duellist grudging an
actor outside its batch, and two summons colliding on one authored id. Each had
been invisible because the spawner swallowed it.

⚠ **Deliberately partial, and that is the card, not a shortfall.** Only ONE
family per origin kind is migrated; authored placements, enemies, bosses,
shrines, gravity zones and portal guns still take the family-specific loops in
`RoomFeatureConstructionPlan::spawn`. Those are Phase 4's migration order.
`apply_spawn_actor_requests` also survives on purpose — programmatic scene setup
(RL reset, demo crony spawns) legitimately wants a message.

⚠ **A SECOND review round found four of those five repairs incomplete, and one
encoding a new wrong invariant** — the relation rule permitted cutting a
relation's target, which strands the untouched source on a dead `Entity` handle
(proven: `Grudge(1v0)` vs a rebuilt `1v1`). All now repaired: symmetric relation
rule + `relation_closure`; executor allocates the root via `ConstructionRoot` so
a recipe cannot commandeer or nominate one; `AcceptsFn` and the request's
`recipe` field deleted in favour of derived `recipe_of` + exhaustive
`construct`; counter advance queued as part of the commit; `ContentBinding`
replaces the thrice-overloaded epoch-zero sentinel.

⚠ **Checkpoint 1 of the third review round landed:** restored four relation
tests my own previous commit silently deleted (an edit truncated the file and the
reported count was never re-derived) and extended them to six cases; collapsed
`recipe_of`+`construct` into one `dispatch` so identity and behaviour cannot
drift; the construction registry now genuinely reaches the prepared-content
fingerprint as `construction.recipes` (it was documented as doing so for two
commits before it did); summon counter reservations carry the value planning read
and refuse a stale or missing counter BEFORE spawning. Actor recipes are now
documented as a CLOSED domain — providers register metadata, not executable
behaviour. **There is no enforced plan-to-world roster parity and the docs no
longer claim one.**

⚠ **Checkpoint 2 (substrate) landed:** the prepared plan now freezes its
resolved constructor (commit no longer re-dispatches — proven against a domain
whose `dispatch` flips on an atomic); summon reservation check+build+advance are
one exclusive-world boundary with the `max()` recovery deleted; relations carry
schema metadata that reaches the fingerprint; `verify_committed_roster` counts
identities and flags unplanned roots, with six adversarial recipes proving it.
⚠ It DETECTS, it does not prevent — Bevy commands do not roll back.

⚠ **Checkpoint 3 (substrate) landed:** verification became something a
transaction has to pass rather than a function tests could call. The baseline
holds entity + provenance per identity, not a `BTreeSet<SimId>` (which could not
tell an original from a replacement), refuses capture on a pre-existing
duplicate, and takes retirement/reconstruction as DECLARED rather than inferred
from the plan. Authoritative scope is now gathered by querying the world and
classified by component — this transaction's `TransactionId` stamp, another's,
an explicit `PresentationOnly` opt-out, or no ownership at all — so a caller can
no longer make the check incomplete by forgetting a root. Relations carry a
frozen `verify` beside their `wire` and are checked against committed components
(a receipt only proves the wiring function was CALLED). `fn_addr_eq` is gone from
registration semantics: it made a registry contract depend on codegen.
**`RoomFeatureConstructionPlan::spawn` no longer writes `RoomLoaded`** — a
queued capture runs before construction and a queued verify-and-publish after
it, and a fatal violation withholds publication.
⚠ Still a detector: there is no staging world, so nothing rolls back.
⚠ `Severity::Unmigrated` is a deliberate temporary hole — an identity with no
ownership stamp is reported, not fatal, because nine families still build roots
outside the planner.

⚠ **Phase 4 STEP 1 landed:** `ambition.limb` and `ambition.mount` are registered
relation kinds with bidirectional wiring AND bidirectional postcondition checks.
Relations gained a typed `RelationPayload` because `Limb`'s `slot` and
`home_offset` are both stated relative to the HOST — facts about the pairing, not
about either body — so the dump gained a payload column and the plan schema is
**v3**. One function writes both ends, which is what makes the half-write
unspellable; the rig case accumulates in canonical relation order because
`fan_out_limb_intents` reads it positionally. Reverse verification is not
redundant: a limb outside its host's rig is INERT and a mount whose `MountSlot`
does not point back stops obeying (`steer_mount_from_rider` queries
`With<MountSlot>`), while every forward-only assertion passes in both cases.
✅ **Production callers landed 2026-07-23** (superseding the two ⚠s that stood
here): giant hosts + hands are explicit plan rows joined by `ambition.limb`
relations for EVERY plan origin (authored and provider-staged share
`giant_cluster_rows`; runtime origins REFUSE giant specs rather than spawn
handless), and authored mount links are planned `ambition.mount` relations —
`PendingMountLinks` and its frame-later resolver are DELETED, link-named
enemies/bosses are plan rows, and both ends of the weld are wired at commit and
verified before publication. The same push closed the outer-artifact holes: the
authoritative roster derives from the completed plan (`planned_ids()` ∪ the
enumerated non-plan families, one `SimId` spelling), `RoomConstructionPlanId`
hashes the complete frozen plan (`deterministic_dump()`: derived rows, relation
payloads, recipe ids, content epoch), exact rig composition is a fatal boundary
check (`verify_rig_composition`), and reconstruction accepts any stable `SimId`
(a hand's `SimId::spawned` included). Campaign doc: Checkpoints A, C, and
"C step 2". Commits `77e4ce03f`, `e164f22e2`, `89b10c5a4`, `c8cee911a`.

⚠ **Phase 4 proper is the remaining work.** Nine authoritative families and the
parallel `apply_spawn_actor_requests` path remain outside the planner (table in
the campaign doc; the enemy and boss families now MINUS their giant/mount-link
members). Verification still DETECTS rather than PREVENTS — no staging world,
Bevy commands do not roll back.

⚠ **An earlier same-day review found five transactional gaps the tests could
not see** — a counter spent before validation, an unchecked recipe/parameter
pairing, an executor trusting the `Entity` a recipe returned, a parent stored
twice, and `construct_one` silently dropping relations. All closed (plan schema
is now **v2**); the Phase 3 account lists them. **Read that list before starting
Phase 4** — every one was a boundary described as atomic with nothing enforcing
it, which is exactly the risk Phase 4 multiplies.

**Phase 4 LANDED 2026-07-23** (`637797649`, `58c43e900`, `d1e26aa79`,
`37e041810`; accounts in the campaign doc §Phase 4a–4g). All three parts below
executed: every family migrated (the two ⚠s above dissolved —
`non_plan_authoritative_ids` is deleted, the roster is exactly
`planned_ids()`), the five lifecycle paths audited onto one transaction, and
the commit boundary enforces content-binding staleness
(`ContentBindingMismatch`, fatal). `apply_spawn_actor_requests` is explicitly
scoped to programmatic scene setup. Recorded-open in 4g: the live identity
index (no consumer yet) and the staging world (detection → prevention).
The executed shape, kept for the record:

1. **Family migration, largest loops first** — fold the remaining spawn loops
   into plan rows using the twice-proven pattern (giants, mount links): enemy
   family, boss family, authored placements→NPCs, then the static families
   (hazard, pickup/chest/breakable/switch, shrine, gravity zone, portal,
   portal-gun). Each family keeps its populate function; being planned changes
   who allocates the root and wires/verifies relations, not what the actor is.
   Delete each family's loop as it migrates; `non_plan_authoritative_ids`
   shrinks to empty and the "incomplete visibility" caveat in the campaign
   doc's Checkpoint C statement dissolves with it.
2. **Lifecycle unification** — activation → reset → transition → hot reload →
   snapshot reconstruction become variations of ONE prepared transaction
   (today they share the planner only for the migrated families). Hot reload
   builds candidate `PreparedContent` + candidate room state before activating
   either.
3. **The commit boundary** — the recorded-not-solved limits: enforce
   `ConstructionScope::content_epoch` at commit; the live identity index so a
   relation can target an entity outside the plan; and the staging/disposable
   world (or equivalent) that turns the boundary verifier from a detector into
   a preventer. `RoomLoaded` publication ordering is already correct.

**Exit:** every authoritative root in a shipped room is a plan row; the
enumerated legacy list is empty; `apply_spawn_actor_requests` either lowers
through the planner or is explicitly scoped to programmatic scene setup; a
failed preparation cannot partially despawn or replace the active room.


## 10. Gameplay presentation profiles — **LANDED 2026-07-20**

Design of record:
[`triage/gameplay-presentation-profiles.md`](triage/gameplay-presentation-profiles.md)
(promoted and implemented 2026-07-20 — `077d3108a`, `5ac381d72`. All nine
promotion questions are resolved against source in its "Resolved questions"
section, and its "Implementation status" section records what landed, the three
things the implementation learned, and the four items deliberately left out).
GP1–GP5 are all ✅, plus a review-driven correction pass (`8a545077b`,
`77c788c2a`, `ce283d6bf`, `54892bb26`) that repaired the schedule handoff for
fixed-tick and GGRS hosts, made reserved surrounds real control placement
regions with an explicit fallback ladder, derived screen occlusion from
computed UI layout, and completed the canonical occlusion ordering key. What
remains is listed in the design doc as scoped follow-ups, not as unfinished
cards: the platform safe-area bridge (nothing exposes insets yet), overlap
fallback steps 2–4 (need a device to tune against), a participant-facing layout
preference (gated on product testing), authored surround art, and a
non-compactible movement stick (its art is owned by `virtual_joystick`).

Landscape phones are much wider than the gameplay composition, so virtual
controls cover the controlled actor. One subsystem, four independent policy
axes (viewport / framing / screen occupancy / activation), configured per
provider. **No engine branch may select behavior by game name.**

- ✅ **GP1** — pure policies + layout resolver in
  `ambition_platformer_primitives::gameplay_presentation`: fixed-aspect
  fitting, safe-region ∩ occlusion composition, three presets, no runtime
  camera change. Tested over 4:3 / 16:9 / 16:10 / 19.5:9 / 20:9 plus
  asymmetric safe-area insets;
- ✅ **GP2** — fixed-aspect runtime slice: host resolves one
  `ResolvedGameplayPresentation`, applies `Camera.viewport` to `MainCamera`,
  keeps `FrontHudCamera` full-screen, and feeds `CameraViewport` from the
  gameplay rect instead of the window. Proves `fixed_four_by_three()` for
  Super Mary O;
- ✅ **GP3** — soft subject framing: one new pure input (`CameraScreenFraming`)
  turns the normalized safe region into a per-axis deadzone on the camera
  target, *before* the existing room/zone clamp. Proves
  `high_speed_full_bleed()` for Sanic;
- ✅ **GP4** — occupancy-aware framing: `ScreenOccluder` (content-free, anchored
  like the existing `TouchExclusionZone`), published by the touch controls,
  composed into the safe region. Proves `adaptive_platformer()` — normal
  desktop framing, occlusion-aware only when touch-primary;
- ✅ **GP5** — named surround/HUD/control regions + Mary O surround
  presentation. Turned out to be REQUIRED rather than polish: a
  viewport-clipped camera never clears outside itself.

**Guardrails:** presentation only — profile selection must never change
simulation results, and camera composition must not flip on the last input
device (glyphs may, framing may not). The provider declaration is a field on
`PlatformerExperienceAuthoring`, *not* a neighboring registration API;
`ambition_demo_pocket` stays undeclared on purpose.

**Exit (met):** 16 pure resolver tests over the required aspect matrix, 11 host
runtime tests, 6 camera-deadzone tests, 6 real-provider pins in `app_it`, 3
surround tests, 2 touch-occupancy tests; `cargo test --workspace` fully green.
Oracle 4 has no subject in this codebase (there is no gameplay pointer→world
conversion) — see the design doc.

