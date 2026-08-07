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

`ambition_platformer2d_core::ConfirmedFrameBoundary` is the host's published answer to
"which frames can never be simulated again", and
`ambition_platformer2d_runtime::external_effects` is the mechanism that keys irreversible
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
  `ambition_platformer2d_actor_monolith` build graph dropped `bevy_pbr`, `bevy_gltf`,
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
- deleted the dead `ambition_platformer2d_world → ambition_time` edge and made actors→ui_nav
  an optional feature-conduit. **Correction:** `sprite_sheet → interaction` is
  NOT dead (8 real path references) — the deep review was wrong there;
- deleted the vestigial `rl_sim` feature chain (actors' and the facade's copies
  gated nothing once the RL surface moved to `ambition_sim_harness`; the app's
  switch is the real one). `headless` stays: it is an intentional empty
  composite whose value is what it leaves off;
- **the trim exposed three crates that only ever compiled by accident**, because
  the untrimmed dep was donating features workspace-wide:
  `ambition_platformer2d_shared_tangle` (needs `bevy_input_focus` for `KeyCode`),
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

`ambition_platformer2d_runtime::sandbox_reset` now owns `reset_sandbox` and the ONE
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
every one of which `ambition_platformer2d_runtime` already depended on; it was unmovable
only because it sat in `app::world_flow::room_flow`, which also composes
`load_room` with `ambition_platformer2d::render` spawns. Splitting the reset out of that
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
  `ambition_platformer2d_core`; engine-wide, not Mary-O's;
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
live in `ambition_platformer2d_runtime::room_transition`, carried by `PlatformerEnginePlugins`
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

`ambition_platformer2d_shared_tangle::construction` is the content-free planner;
`ambition_platformer2d_actor_monolith::construction` puts the three real origin families through it
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
  `ambition_platformer2d_shared_tangle::gameplay_presentation`: fixed-aspect
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

---

## K2b direct-entry activation — execution narrative (archived 2026-08-07)

**Moved verbatim from `docs/planning/tracks.md`, losslessly.** The row is `✔ DONE
2026-08-06`; this is the 319 lines of how it went. Its two OPEN rows (K2b-i, the
`SessionScopedEntity` asymmetry; K2b-ii, the blocked edits 2-4) stayed in the live
file and are not reproduced here.

⚠ Nothing here is edited. If something below turns out to be wrong, reopen it in
the live ledger rather than amending this file.

- ✔ **K2b direct-entry activation — DONE 2026-08-06.** The oracle is met: the
  hand-built `SessionRoot` is deleted. `compose_ambition_gameplay_host` is the
  one composition, and the whole `ambition_app` suite (471 tests) plus the
  workspace policy job are green without it.
  ⭐ **the ORDER inside that function is the part worth keeping.**
  `AmbitionShellHosted` → simulation plugin → shell. Getting the last two
  backwards panics inside Bevy parameter validation naming a system the caller
  never heard of, because the shell is an ADAPTER over a composed game rather
  than a composition of one.
  ⭐ **nine call sites had the recipe by hand**, which is why deleting the
  publisher turned every one into a failure at once — 33 tests — and the blast
  radius got MEASURED instead of estimated.
  ⛔⛔ **the deletion's real value was the four defects it exposed**, none of
  which K2b was about. Each had been invisible because the rollback harness
  composed the simulation plugin alone, so no checker had ever seen the shipped
  host:
  - **A global countdown with no owner froze every fighter.**
    `settle_versus_round` ran unconditionally and `VersusMatch` defaults to
    `Starting`, whose arm marks EVERY fighter `ScriptedControl` each tick. Any
    composition that installed the versus stage without being on its route
    gagged its own bodies. ⭐ same rule `track_versus_roster` learned about the
    roster one campaign earlier: not "a fighter exists", MINE.
    ⚠ **the symptom pointed at the wrong end.** Seat two authored 40 frames of
    right and moved 0.00px, which reads as input routing — and the input was
    measured intact through `PendingSeatInputs` → `LocalInputs` →
    `SlotControls[1]` → the brain tick, 190 times. Two plausible fixes (a
    missing `SlotControlLatches`, a clobbering host writer) were probed and
    REFUTED, which is the only reason neither shipped.
  - **18 rollback registrations were presence-only**, across mary_o, sanic,
    relativity and versus — each now carries a value projection.
    ⛔ `SpentPowerBlocks` is a `HashSet`, so an order-dependent one would have
    been nondeterministic between peers; all four set-shaped checksums XOR
    per-element hashes.
  - **`PortalGunPickup` and `HealShrine` had no rollback anchor**, so `SimId`,
    `SpawnOrigin` and `TransactionId` were inert on them. Invisible because
    direct entry built its world UNSCOPED and the sweep never looked.
  - **The schema baseline recorded a composition no player runs** — 50 rows
    added, ZERO removed, when the harness started composing the shipped host.
  ⭐ **the replay fixture is the cleanest evidence nothing else moved**: the
  regenerated trace is the old one shifted by exactly one frame
  (`new[i] == old[i+1]` for all 59, same landing), because activation takes
  frames. State what changed about a regenerated fixture or it proves nothing.

  **⚠ SCOPED 2026-07-21 (opus, structural trace done — the card badly
  understated this).** The blocker is not the `SessionRoot` spawn; it is that
  the spawn happens **at plugin-build time, before tick 0**, and activation
  happens **asynchronously over several `Update` frames**. Everything below is
  anchored and SCOPED — file:line trace, five numbered edits, two named
  structural risks, a three-stage plan. None of it is compiled or tested, so
  "pre-solved" (the earlier wording) overstated it: the settlement behavior is
  designed, not demonstrated.

  *Today (direct entry):* `publish_direct_prepared_session_root`
  (`app/resources.rs:295`, called from `app/plugins.rs:132` at the END of
  `add_simulation_plugins`) spawns `SessionRoot(SessionScopeId(0))` + live
  world + content + identity. The player comes later from
  `setup_simulation_system` (`app/setup_systems.rs:35`, `run_if(direct_entry)`),
  which calls the SAME `session::setup::simulation_world` the shell builder
  calls — but `UNSCOPED`, with a hardcoded
  `PLAYABLE_ROSTER[0]` default character, and it never inserts
  `GameplayInputOwner`.

  *Target:* direct entry is just **a shell host whose initial route is the
  gameplay route** — the recipe `ambition_demo_sanic_app/src/lib.rs:79-84`
  already proves (`ShellHostSpec::new(<gameplay_route>, <home_route>)`).
  No new API; `PlatformerExperienceAuthoring::install` already registers the
  preparation plan.

  **All five edits landed 2026-08-06.** 3 deleted 265 lines of the entry path
  (`spawn_ldtk_world_root`, both feature arms of `setup_presentation_system`,
  and the private chain under them — `presentation_world`,
  `presentation_world_inner`, `session_presentation`, `PresentationSetup`,
  `SessionPresentationSetup`), plus the `direct_entry` run condition itself.
  4 deleted the direct audio branch. 5 composed the shell in `headless.rs`,
  `rl_sim/mod.rs` and `bin/capture_scene.rs`.
  ⭐ **verified by CAPTURE, twice, because both deletions are silent-failure
  classes.** A deleted presentation builder passes every test and draws nothing;
  a deleted audio selection passes every test and plays nothing. The 640x360
  `--include-ui` capture of `central_hub_complex` is BYTE-IDENTICAL before and
  after edit 3, and reports `first owned SFX play attempt
  (owner=Some(Gameplay(0)))` after edit 4 — `Gameplay(0)`, not `Direct`, which
  is what makes deleting the other selection safe rather than merely compiling.
  ⚠ **`build_visible_app`'s `shell_hosted` parameter is KEPT, name and all.**
  Both arms are shell-hosted now and it only picks the initial route; renaming
  it would touch 33 call sites to restate a boolean whose two values are
  unchanged. The doc says so instead.

  **The edits** (in order):
  1. `app/cli.rs:790-818` — stop using `AmbitionShellHosted` as the
     discriminator; always compose the shell host + visuals, and in direct mode
     set the initial route to `AMBITION_GAMEPLAY_ROUTE` and skip the startup
     vanity sequence.
  2. Delete `publish_direct_prepared_session_root` + its call at
     `app/plugins.rs:132`.
  3. Drop four `run_if(direct_entry)` registrations and un-gate the host one:
     `sim_resources.rs:44` (`setup_simulation_system` — the system itself
     likely dies), `plugins.rs:288` (`spawn_ldtk_world_root`), `plugins.rs:515`
     (`setup_presentation_system`), `plugins.rs:525` (`spawn_map_menu`);
     `plugins.rs:517-523` becomes unconditional.
  4. Delete the direct audio branch `app/resources.rs:80-98` —
     `select_shell_audio_context` (`game_shell/src/session.rs:399`) owns
     selection + `SfxEmissionContext` on activation.
  5. **This is where the actual work is:** `headless.rs:124`,
     `rl_sim/mod.rs:64`, `bin/capture_scene.rs:143` add only
     `AmbitionGameSimulationPlugin` and get their root for free at build time. They
     must compose the shell and **settle N frames until the world exists**.

  **Two risks, both structural:**
  - **sync→async.** ~35 integration files behind `tests/common/mod.rs` +
    `Platformer2dSimHarness` (`rl_sim/mod.rs:64`), plus `run_headless`
    (`headless.rs:137-142` `.expect("active session RoomSet")`), do
    `App::new(); …; update(); read_the_world()`. After the migration the root
    exists only after the load barrier reaches `Ready` and all 8 preparation
    work items complete (`game_shell/src/preparation.rs:27-40`). **Do the
    settle helper FIRST, as its own commit, before deleting anything.**
  - **`SessionGatedSimulation` semantics flip.** Composing the shell installs
    `GameplaySessionBridgePlugin` → `SessionGatedSimulation`
    (`game_shell/src/session.rs:306`), flipping `simulation_authorized`,
    `session_world_exists`, `session_world_entity`, and
    `declare_gameplay_input_context` from "one root is enough" to "root scope
    must equal `ActiveSessionScope::current()`" — for the headless/RL harnesses
    too. The root also becomes `SessionScopedEntity`-tagged and therefore
    despawnable by `despawn_retired_session_entities`, a teardown-bug class
    that structurally cannot occur today.

  Note `SessionScopeId(0)` at `resources.rs:316` is an arbitrary placeholder,
  not special — the first shell activation also mints 0
  (`ActiveSessionScope::begin`), and they do not collide today only because
  direct entry never installs `SessionScopePlugin`. It disappears entirely.

  No test asserts on `publish_direct_prepared_session_root` or
  `SessionScopeId(0)` directly — the coverage is all implicit, which is exactly
  what makes risk 1 dangerous.

  ◐ **K2b.1 STARTED 2026-08-06: the settle helper is landed and in front of
  `run_headless`.** `settle_until_session_world(app, frames)` advances until
  `session_world_entity` answers and returns how many frames it took —
  `Ok(0)` for a build-time root, so putting it in front of a direct-entry
  caller changes nothing and the migration is stageable exactly as this row
  planned. Three tests pin the three cases: immediate, late, and never (an
  `Err(budget)` rather than a panic three lines later about a missing
  `RoomSet`).
  ✔ `run_headless` and `Platformer2dSimHarness::build` both settle now — the
  harness one matters most, because every caller reads the world IMMEDIATELY
  after construction (`room_ids()`, an observation, a `RoomSet`), which works
  today only because the root is there at build time. Best-effort while the
  build-time root still exists, a hard error in K2b.2.
  ⭐ `bin/capture_scene.rs` needs NOTHING: checked rather than assumed — it
  names no `session_world*` and holds no `.expect`, because it already waits
  through its own warmup and camera-adoption gate. The row listed it because it
  composes the plugin, and composing is not the same as reading.
  ✔ **the ~35 integration files behind `tests/common/mod.rs` need no edits** —
  they all construct through `Platformer2dSimHarness::build`, so the settle
  inside it covers every one. Risk 1 is retired at the seam rather than at 35
  call sites.
  ✔ **AND BOTH PATHS ARE NOW PROVEN TO AGREE** —
  `game/ambition_app/tests/direct_and_shell_agree.rs` builds direct entry and a
  shell host booted straight to `AMBITION_GAMEPLAY_ROUTE`, settles both through
  the same helper, and asserts they start in the SAME ROOM. Measured: direct
  settles in **0** frames, the shell in **2**. That is the coverage this row
  called *"all implicit, which is exactly what makes risk 1 dangerous"*.
  ⛔ **two things the test found that the plan did not say:**
  1. the shell composer is an ADAPTER, not a composition —
     `compose_ambition_shell_host` without `AmbitionGameSimulationPlugin` panics
     on frame one (`settle_versus_round` wants `Res<WorldTime>`);
  2. `AmbitionShellHosted` must be inserted BEFORE the sim plugin builds, or
     `publish_direct_prepared_session_root` runs anyway and the app gets TWO
     canonical roots — the `SessionScopeId(0)` collision this row predicts in
     prose, reproduced.
  ⭐ edit 1 landed additively: `compose_ambition_shell_host_booting_to(app,
  route)`, with the launcher default untouched.
  ✔ **edit 1 LANDED (2026-08-06): the shell host is composed either way**, and
  the mode only chooses the route — launcher when hosted, `AMBITION_GAMEPLAY_ROUTE`
  when `--direct`/`--start-room`. `AmbitionShellHosted` is now inserted
  unconditionally (before the sim plugins, per the collision above), so
  `publish_direct_prepared_session_root` never runs in a CLI-built app. 300 app
  tests green.
  ✔ **`run_headless` MIGRATED (2026-08-06)** — it composes the shell and boots
  to the gameplay route, so the headless report now runs the same activation a
  player does instead of a second way to start a game. 14 headless tests green.
  ⛔ **the HARNESS flip was TRIED and REVERTED, and the measurement is the
  scope.** Composing the shell inside `ambition_sim_composition` turns risk 2
  from prose into eight red tests, in two distinct families:
  * **the subject vanishes** — `desync_canary` panics at
    `the sandbox session has a controlled subject`. Under
    `SessionGatedSimulation` the root must belong to the ACTIVE scope, and the
    harness's first read happens against an activation that has not seated a
    body yet;
  * **the rewind stops agreeing** —
    `GGRS sync-test checksum mismatch at frames [2, 3, 4]`, plus
    `effect_quarantine`'s pair. A shell activation performs work DURING the
    frames the sync test is comparing, so the two runs diverge on activation
    rather than on gameplay.
  ⭐ **that second family is the real content of K2b.3**, and it is not a
  test-fixture problem: it says a rollback session must not begin until
  activation has settled.
  ✔ **the rule is STATED now (2026-08-06)**: `RollbackRefused::NoSessionWorld`,
  checked in `rollback::start` after activation and settle. A session opened
  over a world that does not exist yet is measuring CONSTRUCTION, and its
  checksum mismatch reads as a desync in the game — so refusing is the honest
  answer and the message names the helper to wait with. ⚠ it had no teeth to
  grow before: with a build-time root "the world exists" was true before frame
  one, so nothing could notice the rule was missing. 46 rollback tests green.
  ⭐ **CORRECTION (checked, not assumed): the lower-level road already stated
  the rule.** `install_rebased_sync_test_session` calls
  `warn_if_no_world_to_rewind`, whose text is the same diagnosis in almost the
  same words — *"frame zero is an EMPTY world … the frames that build the room
  will mismatch on every resimulation and GGRS will report it only as a checksum
  difference."* The row above overstated the gap; what was missing was the
  refusal at the higher authority, which is what landed.
  ✔ **but that check asked a NARROWER question, and the gap only opens under a
  shell host.** It accepted any `SessionRoot` entity, while
  `session_world_entity` also requires the root's scope to equal the active one
  whenever `SessionGatedSimulation` is installed — which the shell installs. A
  root left by a RETIRED activation satisfied the old check while every reader
  in the engine correctly saw no world, so the warning stayed silent for exactly
  the case it exists to catch. Both roads ask the same question now.
  ◐ **ATTEMPT TWO (2026-08-06): two of the three families are FIXED, and the
  third says the shape is wrong.**
  * ✔ *the subject family* — `settle_until_controlled_subject` waits for a
    seated body as well as a world, because every harness caller drives an actor
    on the next line. The `"the sandbox session has a controlled subject"` panic
    is gone.
  * ✔ *the checksum family* — and the cause was ORDERING, not the shell: the
    settle sat AFTER `start_sync_test_session` in `Platformer2dSimHarness::build`,
    so the session still opened over an activating world. Moved above it; all 8
    `desync_canary` tests pass with the harness composing the shell.
  * ⛔ *a THIRD family the plan never named, and it is the blocker*: composing
    the shell drags in EVERY PROVIDER, and providers register RENDER material
    state. `rollback_coverage` starts reporting
    `bevy_render::…PreparedMaterial2d<MaryOQuasarMaterial>` and
    `EntitiesNeedingSpecialization<…>` as unrewound resources in the harness
    world, plus an inert-registration failure. Waiving render materials into a
    rollback census would be lying about what a headless RL/oracle harness is.
  ◐ **ATTEMPT THREE (2026-08-06): the render family is GONE and ONE class
  remains.** `Material2dPlugin::<MaryOQuasarMaterial>` was being installed
  whenever an `EmbeddedAssetRegistry` existed — which every headless composition
  has — so the material's render-world resources landed in any app that merely
  had `AssetPlugin`. That is the same *"a proxy answers the question next
  door"* mistake the module's own doc already records about its RUN condition,
  repeated one line up at install time. It is gated on the render sub-app now,
  and Mary-O still draws (captured to check).
  * ✔ render resources: gone from the harness world.
  * ✔ `ContentEpoch` and `Messages<SessionScopeRetired>`: two honest waivers,
    identity and lifecycle respectively — a rewind cannot rebuild a session from
    other content, and must not un-retire a scope.
  * ▢ **ONE class left, and it is a real question rather than a waiver.**
    `no_snapshot_registration_is_inert_*` reports an archetype
    (`SpawnOrigin + TransactionId + RoomScopedEntity + SessionScopedEntity +
    SimId + Name`) whose components are REGISTERED as rollback state while the
    entity carries no rollback anchor — so the registration is a claim the
    engine does not honour. It exists only under the shell.
    ⭐ **MOVED TO [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md)
    (2026-08-07)** — *"Is a SESSION scope marker construction provenance, the way
    a ROOM scope marker is?"* It stopped being an engineering question the moment
    it narrowed to one component, and the full write-up lives there so it is not
    restated in two places that can drift. Kept here in summary because the
    sweep is what found it.
    ⭐⭐ **NARROWED to one component, 2026-08-07.** Five of the six are in
    `PROVENANCE_ONLY` — `SpawnOrigin`, `TransactionId`, `SimId`,
    `RoomScopedEntity`, `Name` — so they are skipped by the RULE rather than by a
    waiver. The archetype is reported at all because of exactly ONE component:
    **`SessionScopedEntity`**, which is registered rollback state
    (`scope.session`, `primitives.rs:110`) and is NOT in that list.
    ⚠ **so the real question is an ASYMMETRY, and it is a short one**: both
    `RoomScopedEntity` and `SessionScopedEntity` are write-once SCOPE MARKERS
    stamped at construction, and the provenance rule's own argument — *"written
    ONCE and never again … a rewind that does not restore them restores exactly
    the values they already hold"* — reads identically for both. Is the omission
    deliberate? ⛔ **there is a reason to think it might be**: the sibling waiver
    for `Messages<SessionScopeRetired>` says a rewind *"must not un-retire a
    scope"*, so session lifetime has a rewind rule that room lifetime does not.
    Whether that rule reaches the MARKER as well as the retirement message is the
    whole question, and it is Jon's or the rollback owner's.
    ⛔ **and the sweep cannot currently see it.** Both
    `no_snapshot_registration_is_inert_*` assertions PASS, so the archetype does
    not appear in the boot world or a live match — it is in a composition no test
    sweeps. Naming the entity (done today) helps only once a test reaches that
    composition; until then the sweep is green about a class it never looks at.
    ✔ **the instrument NAMES the entity now (2026-08-07), so that probe never
    has to be written.** `inert_registrations` keyed each archetype to
    `names.intersection(&anchors)` — which is PROVABLY EMPTY at that point, since
    the loop `continue`s a few lines up whenever that intersection is non-empty.
    Every reported archetype carried an empty set beside it: the failure named a
    SHAPE and could never name a THING. It reports the entity's `Name` (deduped,
    so 40 copies of a prop stay one line), which is exactly what the next
    investigation below was told to go and print.
    ⚠ **narrowed by probe (2026-08-06, throwaway, not kept):** the CLI-built
    app — which composes the shell as of edit 1 — has **ZERO** construction
    roots without a body after settling. So the archetype is NOT a plain
    consequence of shell composition; it appears under the HARNESS's composition
    specifically (rollback enabled, a named room). The next investigation should
    probe inside `Platformer2dSimHarness` with the flip applied and print the
    entity's `Name`, rather than re-deriving that the shell is involved.
  ⭐ **the remaining work after that is a HEADLESS SHELL COMPOSITION** — routing
  and activation without the provider presentation — if the anchor question does
  not dissolve it.
  `compose_ambition_shell_host` is already headless in the sense that visuals
  are separate (`install_ambition_shell_visuals`), but the PROVIDER plugins it
  adds are not. That is the next design step, and it is a smaller and much more
  precise question than "migrate the harness".
  ▢ **edits 2-4 remain BLOCKED, and now for a measured reason.**
  The build-time root is not dead: `run_headless` and
  `Platformer2dSimHarness::build` compose `AmbitionGameSimulationPlugin`
  WITHOUT the CLI, so they never insert `AmbitionShellHosted` and still get
  their root at build time — and the four `direct_entry` gates are still true
  for exactly those apps. Deleting either now changes headless behaviour rather
  than removing a dead path. **The remaining work is migrating those two
  composers to compose the shell**, which the settle helper already prepared
  them for; the agreement test then says the room did not change.

  **Suggested staging:** (K2b.1) land the settle helper + migrate
  `headless`/`rl_sim`/`capture_scene` to compose the shell and settle, keeping
  the build-time root as a fallback and proving both paths agree; (K2b.2)
  delete the build-time root and the four `direct_entry` gates; (K2b.3) delete
  `AmbitionShellHosted` / `shell_host::direct_entry` once nothing reads them,
  and fold `PlatformerExperienceAuthoring::with_world_manifest` in (the K2a
  remainder above) while that builder is already open.

---

## 0. Pay down the GGRS correctness debt — **LARGELY LANDED 2026-07-19** (archived 2026-08-07, verbatim from tracks.md)

## 0. Pay down the GGRS correctness debt — **LARGELY LANDED 2026-07-19**

Spec: deep-review §2. Landed:

- registered the unregistered sim state: `WornEquipment`, `SwitchOn`,
  `SwitchFeature`, `SwitchActivationQueue`, breakable/hazard/respawn/stand
  timers, portal-gun runtime (`PortalTransitCooldown`/`PortalEmission`/
  `PortalShot`/`PortalGun`), and `RoomVisual`;
- `MovePlayback.live_boxes` is now VALIDATED against the world every tick, so a
  cloned cache slot naming a dead entity is dropped and the window respawns its
  volume. Mechanism-agnostic: it fixes the GGRS clone case and any future path
  that despawns a volume out from under a playback;
- `possession_trigger_system`'s `Local` hold/edge state moved onto the
  registered `PossessionState`;
- target selection no longer compares raw `Entity` (candidates are sorted by
  `SimId`, ties go to canonical order); slot requests are sorted by `actor_id`
  before `assign_slots`;
- **the coverage forcing function exists**:
  `game/ambition_app/tests/rollback_coverage.rs` boots the real sim and asserts
  every component ON a simulated entity is registered, derived, or waived with a
  reason. It is computed, not a checked-in ledger, and it found the last two
  gaps (`SwitchFeature`, `RoomVisual`) on its first run. NOTE it checks entity
  COMPOSITION, not system access — Bevy 0.18 does not expose per-system
  `FilteredAccessSet` publicly — so **resources still need review by hand**.

Remaining:

- the demo-content state composed into the app shell (`BallDash`,
  `BallDashInput`, `SanicActState`, `MaryOLevelState`, `FlagSequence`) — these
  live in `game/` crates and need the content-side registration seam;
- `FactionRelations`/`FriendlyFire` are unregistered and latent-safe (only
  `Default` writes today); register them when anything mutates them in-session;
- the exit oracle below.

**Exit:** a sync-test run that lands a melee hit, spends armor, flips a switch,
and breaks a brick across a forced rollback window stays checksum-identical.

---

## ✅ **FS2/FS3 sand slice** (`574550a6d`, Jon-directed 2026-07-20; ruling + (archived 2026-08-07, verbatim from tracks.md WAVE 1)

- ✅ **FS2/FS3 sand slice** (`574550a6d`, Jon-directed 2026-07-20; ruling +
  full card in `falling-sand.md` §4). The one-CA-step experiment resolved
  REPLACE from `bevy_falling_sand`'s source (private PostUpdate systems, a
  step signal that fires twice, DirtyAdvance starvation, parallel+RNG core).
  Sand now runs on a bespoke deterministic grid in
  `ambition_content::falling_sand_sim` (UNGATED; proofs run in every content
  test): one solver step per ordinary sim tick, conservation
  `loose + settled == emitted` asserted every tick, fixed-point settling
  proved, FS3 atomic transfer into a persistent `SettledSandLedger` that owns
  collision (kills the transient flicker), authored-room regression green in
  2.9s. ⛔ The falling-sand room is **not a netcode acceptance surface**:
  water/oil are SHELVED on the frame-driven external crate, and the bespoke
  sand grid/ledger are not rollback snapshots (the authoritative-pass gate
  stops duplicate stepping; it does not reconstruct historical material
  state). Per Jon's 2026-07-20 hard blocker in `falling-sand.md`, the unblock
  is an explicit rewrite/fork decision — no further correctness work on the
  bfs path until Jon calls it. The vestigial bfs-side sand plumbing dies with
  that rewrite.


---

## ✅ **PARTICIPANT-CENTERED INPUT, startup/launcher vertical slice** (archived 2026-08-07, verbatim from tracks.md WAVE 1)

- ✅ **PARTICIPANT-CENTERED INPUT, startup/launcher vertical slice**
  (GPT 5.6-directed 2026-07-20, fable; live doc =
  `docs/planning/engine/participant-input.md`). Four commits on main:
  `e7cc2be14` the persistent `InputParticipant` owns ActionState/InputMap
  (never on actors; `attach_player_input_components` deleted) + explicit
  `ContextClaim`/`ActiveInputContext` contexts declared by their owning
  surfaces + the `InputSet` pipeline
  Collect→ResolveActions→ResolveContext→Route→PublishCues→Consume;
  `2296ce6f6` the shell reads NO raw devices (semantic `MenuControlFrame`
  only, always live from the participant), vanity cards are tap-anywhere
  through the same semantic command as confirm, and the open
  `UiCue`/`ActiveUiCues` vocabulary replaces `MenuConfirmPrompt` (launcher
  cues "Play"/exit label, cards cue "Continue", inventory cues Equip/Use —
  one `ControlPrompt` writer per frame, decided by the resolved context);
  `fc37545b2` touch is a VIRTUAL DEVICE (leafwing input kinds over
  `MobileTouchState`, bound in the participant's InputMap; both folds and
  every GameMode routing branch in touch deleted; declared double-bindings
  replace the secret Jump-as-confirm); plus the assembled
  `app_it::participant_input` acceptance (no-actor startup/launcher, source
  ownership across sessions, held-edge transition safety, three-device raw
  screen-axis parity — it caught a real Update-schedule cycle before launch).
  Reference-frame seam untouched by construction (axes stay raw ScreenAxes
  until `AccelerationFrame::resolve_control`). NOT in slice: rebinding UX
  (P1/P5 stand), dialogue/pause/vehicle contexts, multi-participant frames,
  loading-context migration (its retry keeps a local raw read). The complete
  forward architecture and executable PA1–PA7 migration now live in
  [`engine/participant-action-system.md`](engine/participant-action-system.md).

**Keystone slices**


---

## ✅ **K1a movement tuning** — exit criterion MET. `ae::ActiveMovementTuning` (archived 2026-08-07, verbatim from tracks.md WAVE 1)

- ✅ **K1a movement tuning** — exit criterion MET. `ae::ActiveMovementTuning`
  is the neutral authority every sim system reads (damage, actor update,
  gravity resolve, player tick, room flow, session setup/reset, the provider
  session builder, both demos, the host smoke fixture);
  `EditableMovementTuning` is now only an inspector mirror pushed through
  `apply_editable_movement_tuning` in `DevEditApplySet`. `ambition_platformer2d_core`
  promoted dev-dep → dep in `ambition_platformer2d_provider` (no new graph edge;
  actors already pulled it in). Remaining `EditableMovementTuning` references
  are editor paths only (inspector registration, settings/kaleidoscope
  writers, seeding, test fixtures) — verify with
  `rg EditableMovementTuning -g '*.rs' | grep -v ambition_dev_tools`.
  A live-edit test pins that F3 still reaches the sim; an adapter bug it
  caught (Bevy counts insertion as a change, so the mirror's defaults
  stomped authored tuning on frame one) is fixed and poison-tested.
  **LATER K1 completion** (unchanged, NOT done): deleting the
  `ambition_dev_tools` dep from actors/runtime still needs `DeveloperRuntimeState`,
  `EditableAbilitySet`, the schedule sets, and profiling hooks evicted.


---

## ✅ **K2a world-manifest parameterization** (opus, 2026-07-21). The (archived 2026-08-07, verbatim from tracks.md WAVE 1)

- ✅ **K2a world-manifest parameterization** (opus, 2026-07-21). The
  `OnceLock`, `install_world_manifest`, the free `world_manifest()` accessor,
  and the implicit `cfg(test)` fixture branch inside it are all DELETED.
  `WorldManifest` is now an ordinary owned value with two delivery routes,
  both carrying the same value from one owner:
  - **`&WorldManifest` argument** for readers that run pre-`App`, at
    plugin-build time, or as pure functions — `load_default`,
    `load_default_for_dev`, `merge_secondary_worlds`, `load_from_disk_at`,
    `to_room_set`, `LdtkHotReloadState::from_catalog`, the whole
    `build_platformer2d_asset_catalog*` family, and `AmbitionAssetSourcePlugin::for_profile`
    (a plugin VALUE built before `add_plugins`, so a `Res` genuinely cannot
    reach it).
  - **`Res<WorldManifest>`** (it derives `Resource`) for the in-schedule
    readers: `load_ldtk_asset_handle`, `spawn_ldtk_world_root(s_scoped)` on
    both the direct-entry and shell-host paths, `handle_ldtk_hot_reload`, and
    `setup_host_presentation_system`. `AmbitionContentPlugin::build` publishes
    it where `worlds::install()` used to sit; `init_sandbox_resources` threads
    the same value by reference through every preparation-time reader.

  **Oracle** (`app_it::world_manifest_parameterization`, 5 tests): two
  declarations with disjoint world files AND disjoint entry rooms compose in one
  process, in both orders, each keeping its own rooms and its own start room.

  ⚠ **STRENGTHENED 2026-07-21.** The original three tests said "two providers
  prepare" but built two bare `WorldManifest` VALUES and called pure functions
  over them — no `App`, no provider, no plugin. That is near-tautological as an
  isolation proof (a function taking the manifest by reference and reading
  nothing else cannot leak between callers) and it was blind to the route K2a
  actually changed: `insert_resource` at provider-build time, read as
  `Res<WorldManifest>` in schedule. Its poison test only bit because it stubbed
  those two pure readers directly. Two App-level oracles now cover the real
  boundary: `two_apps_keep_their_own_manifest_through_in_schedule_readers`
  builds two `App`s in one order, steps them INTERLEAVED in the other, and
  asserts each App's own scheduled reader saw its own entry room every frame;
  `the_real_content_provider_publishes_into_its_own_app_only` builds the actual
  `AmbitionContentPlugin` beside a second App and checks neither learned the
  other's declaration.

  Still uncovered, found while doing this: a live first-wins `OnceLock`
  (`EXTRA_ENTITY_CONVERTERS`, `ldtk_map/src/conversion/mod.rs:633`) sits one
  call away from `to_room_set`, with the same silently-dropped `Err` this track
  condemns. It is dormant — `install_ldtk_entity_converters` has zero callers —
  so it is a latent hazard, not a live bug.

  Two things fell out that the card did not predict:
  - `LdtkProject::load_static_map` had ZERO callers. Deleted.
  - `build_sandbox_catalog_without_worlds` +
    `sandbox_catalog_inputs_without_worlds` existed only because the global
    could not express "this game ships no worlds" except as a panic. With the
    manifest an argument that is just `&WorldManifest::default()`
    (`is_world_less()`), so the twin is deleted and the two procedural demos
    call the ordinary builder.
  - ⚠ Found by the change, not fixed by it:
    `ambition_platformer2d_actor_monolith/examples/render_room_geometry.rs` loaded through the
    global while never installing one, so it panicked the moment it ran. The
    explicit parameter turned that into a compile error; the example now
    builds its own manifest and works.

  **NOT done (K1-style remainder):** `PlatformerExperienceAuthoring` still has
  no `with_world_manifest` builder — the engine-level provider seam next to
  `with_presentation_profiles` is the natural owner, but Ambition's content
  plugin is today's publisher and adding a second writer for one user would be
  speculative. Fold it in with K2b, which touches that builder anyway.
  ⛔ **RE-EXAMINED 2026-08-06 when K2b landed, and DECLINED — measured, not
  assumed.** `WorldManifest` appears 80 times across the tree and NO game but
  Ambition declares one: `ambition_demo_sanic`, `ambition_demo_mary_o` and
  `ambition_demo_smash` name the type zero times between them. A builder method
  would therefore be a seam with exactly one caller, which is the
  pre-generalization the engine direction forbids outright. The row's own
  reasoning was right; folding it into K2b did not change the count.
  ✔ **the TWO-WRITERS half is DONE — verified 2026-08-07, the row was stale.**
  It read: the manifest has two writers of the same value, `ambition_content::
  plugin` and `ambition_app::app::resources`. There is exactly ONE
  `insert_resource` now (`ambition_content/src/plugin.rs:77`), and the app side
  carries the fix's own note at `resources.rs:91` — *"ONE writer, and it is the
  CONTENT plugin's (2026-08-06) … The provider that OWNS the worlds publishes the
  declaration; the host reads it."*
  ⚠ **the local `let world_manifest = …` in `resources.rs` is NOT the duplicate**
  and should not be "cleaned up" by a later reader: it is threaded BY REFERENCE
  into the preparation-time readers (catalog rows, the LDtk load, the room-set
  conversion, the hot-reload watcher) which run before any schedule and so cannot
  take a `Res`. That is the K2a no-process-global shape, and it is orthogonal to
  who inserts the resource. ⭐ recorded because the surviving `let` looks exactly
  like the defect the row described.


---

## ✅ **RESOLVED 2026-08-07, and two of its three parts are no longer the right (archived 2026-08-07, verbatim from tracks.md WAVE 1)

- ✅ **RESOLVED 2026-08-07, and two of its three parts are no longer the right
  work.** The row prescribed three things for "the smallest inventory smoke worth
  keeping" (`fable-reply-2026-07-19-b.md` §3–4). Checked each against the tree
  before doing any of them:
  * **widen the population by `BodyKinematics` — LANDED**, and went further than
    prescribed. `simulated_population` has THREE sources now, the third being
    vocabulary-derived (anything carrying a type the rollback registers). The
    review explicitly dropped registry-derived queries as a non-goal; the code's
    own comment justifies it with a gap the two tags provably cannot reach — a
    moveset strike volume lives six frames and carries neither tag, so no number
    of extra rooms reaches that family.
  * **per-filter anti-vacuity — LANDED TODAY, at a different granularity, and the
    difference is a measurement.** The review asked that each of the two filters
    assert ≥1 match. That was right when the helper served ONE boot room, where a
    filter matching nothing could only mean a broken filter. It serves TEN rooms
    now, and asserting it revealed that **`portal_lab` authors no
    `FeatureSimEntity` at all** — a legitimate room, not a broken fixture. So the
    floor asserts what is true of every fixture: a body exists, and the union is
    non-empty. ⛔ this was load-bearing and unguarded: poisoning the population to
    empty fails **11 of the 17** tests in the file, and every one of them would
    have passed silently before, because an empty population produces an empty
    unaccounted-list which is exactly what a clean sweep produces.
  * ⛔ **sequester into `tests/ambition_agent_guardrails/` — NO LONGER CORRECT, and
    doing it would invert the rule it came from.** Guardrails are agent tooling and
    are sequestered; product architecture is not. When this row was written the
    thing was one smoke test. It is now **19 tests in 2130 lines** verifying the
    ADR-0023 determinism contract across ten rooms, a live match, a mounted pair
    and a transient strike volume. Filing that under agent tooling would move the
    rollback correctness sweep out of the product's own verification. The rename
    to `rollback_inventory_smoke` goes with it: the file stopped being a smoke.

