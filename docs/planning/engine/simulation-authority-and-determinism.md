# Simulation authority and determinism

**State:** OPEN — rollback registration/backend ownership is largely settled;
remaining work is runtime authoritative-state correctness, deterministic
composition, lifetime boundaries, and explicit phase ownership.

> **Guard pointer, added ce25540b1 (2026-09-02).** This program has a mechanical
> guard that no planning file previously named:
> `scripts/check_rollback_mutators_run_in_sim.py` — *"does anything mutate
> ROLLBACK state from a schedule that never rewinds?"* A component registered
> for rollback is restored on every rewind, so a system mutating it must run in
> the schedule GGRS resimulates (`app.sim_schedule()`), not a literal `Update`.
> ⛔ Its own docstring names why the bug is invisible locally: **under a
> fixed-tick host the two are the same schedule**, so the mistake costs nothing
> and shows nothing — it only diverges under GGRS, where the value rewinds and
> the mutation does not replay with it. Green at `ce25540b1`: 4 systems mutate
> rollback state, none registered into a non-rewinding schedule.
>
> **And a second, for the ordering half of this program:**
> `scripts/check_set_pins_have_engine_members.py` reports engine ordering edges
> that target sets with no engine-owned members — a Bevy ordering edge against an
> empty set is VACUOUS in that schedule, and reads exactly like an active
> constraint. It compares engine set pins against engine-installed systems so
> host/game-only extension sets are not mistaken for real ordering. Green at
> `0ac499bb1`: **2 app-filled sets, all waived with a reason.**
>
> **And a THIRD, an absence contract rather than a script:**
> `central-rollback-does-not-enumerate-domains` in `check_absence_contracts.py`
> pins `platformer2d_runtime/src/rollback/mod.rs` — the host may compose a
> domain's ONE public rollback offer (`register_rollback_state`) and may NOT
> reach through that seam to name any concrete gameplay type from
> `ambition_platformer2d_actor_monolith`, `ambition_boss_encounter` or
> `ambition_characters`. That is this program's "central rollback does not own
> domain censuses" rule, enforced by grep rather than by review, and green.
> ⚠ `rollback-wire-format-changes-are-declared` is a fourth, in the same file,
> and a wire-format change that does not declare itself fails it. ⛔ **DO NOT
> RETYPE ITS TALLY — the gate prints it:**
> `python3 scripts/check_absence_contracts.py | grep rollback-wire-format`.
> This line said "406 stable names across 123 encoded types in 11 crates"; on
> 2026-09-03 it is **409 names, 123 types, 12 crates**, because five crates were
> carved out of the actor monolith that day and registered types follow their
> code. ⇒ The CRATE count is the half that moves under a carve — the type count
> did not change at all, which is the point: a carve relocates a registered type
> and must leave its owner string and short name alone, so the ledger stays
> byte-identical while the crate tally does not.

## Goal

A simulation result should be determined by explicit authoritative data and
semantic phase/composition rules, not by:

- whether a component happened to be registered but its entity was not on the
  rollback timeline;
- Bevy query/entity iteration order;
- a `Local<T>` or process-global value remembering a future state across rewind;
- ambiguous gameplay-session ownership;
- multiple mutable representations of one fact;
- scheduler topology among otherwise unordered writers;
- tuple/SystemParam packing that hides one system owning unrelated domains.

## Current model

An authoritative dynamic object can require five independent guarantees.

### 1. Rewind codec

What mutable component/resource state is saved, restored, remapped and included
in deterministic checksum policy?

### 2. Rollback participation

Does the authoritative entity itself participate in GGRS entity
creation/destruction history? A registered component on an entity without the
appropriate authoritative-family `Rollback` anchor is not enough.

### 3. Stable semantic identity

Which logical object is this across construction/reconstruction and when a rule
needs to compare peers? `SimId` is Ambition's semantic identity;
`bevy_ggrs::RollbackId` remains internal frame-history identity.

A stable identity is not required merely because an entity exists. It is required
when behavior, relationships, reconstruction, diagnostics or deterministic
selection depend on that logical identity.

### 4. Deterministic selection and composition

If multiple valid entities can affect one result, define the rule. Sorting by a
stable key is appropriate for deterministic selection, but not every operation
is order-independent. Where effects do not commute, define precedence or a
canonical state-first/identity-last composition rather than laundering ECS query
order into gameplay semantics.

### 5. Lifetime ownership

Which gameplay session and rollback timeline may treat the state as authority?

Current rollback lifetime is:

```text
process
  -> gameplay session           SessionScopeId
       -> rollback timeline     RollbackTimelineGeneration
```

`ActiveRollbackAuthority` owns the current rollback contract/status together.
Health carries across timeline generations only when the gameplay-session owner
is the same. A foreign session reads `Unavailable`, never the other session's
health. Historical diagnostics are process-lifetime evidence and gate no
simulation/lifecycle work.

ADR 0027 is the durable authority for this rule.

## Landed architecture that should not be reopened

### Domain-owned rollback declarations

Every gameplay domain owns its `register_rollback_state` declaration beside the
types it owns. The generic runtime collects backend-neutral declarations without
naming every concrete gameplay component.

The concrete GGRS schedule/session/backend lives in
`ambition_platformer2d_rollback_ggrs`; the generic runtime has no `bevy_ggrs`
dependency. Do not reconstruct the old runtime `rollback/domains/*` census.

### Explicit schedule phases

`GgrsSchedule` uses explicit Ambition phase sets and the single-threaded
executor. Current measurement found parallel dispatch expensive for the present
many-small-system workload; parallelization is not a determinism or performance
objective without new evidence.

### Controlled-actor observation/decision decomposition

Large actor decision code has already shed several unrelated authorities:
observation, target selection and pre-decision maintenance have clearer owners;
old primary-player combat-slot arbitration was deleted after proving it had no
production consumer.

Continue phase decomposition only when it removes real authority coupling.

### Gameplay-session rollback ownership

`26ec7b19` fixed the demonstrated Smash -> title -> Ambition contamination by
making rollback confirmation session-owned, resolving contracts against their
own session root, and re-establishing session-mirrored resources at activation.

Do not replace this with ad hoc "clear health on quit" calls. Same-session poison
must survive a timeline rebase; different gameplay sessions must not inherit it.

## Current work

### ✔ S-AUDIT — the repo's OWN one-authority sentences, audited (2026-09-05)

⭐⭐ **ELEVEN CLAIMS CHECKED, THREE NEEDED FIXING** (the first sweep of seven is
below; a second pass over the phrasings it missed — `the one place`, `the only
consumer`, `the only executor`, `one authority` — added four more):

```text
encounter/lifecycle.rs:284   EncounterCommand: "the reducer is the only consumer"    HOLDS (1 reader)
authored_logic/prepared.rs   "the ONE AUTHORITY on what an authored value means"     HOLDS
settings/video/quality.rs    "THIS IS THE ONE AUTHORITY", below every consumer       HOLDS
construction/mod.rs:1034     "This is the only executor"                             MISPLACED
```

⭐ **The `prepared.rs` and `quality.rs` survivors are scope phrases again.** A
SECOND truth-parser exists (`gameplay_trace/policy.rs` accepts `1|true|yes|on`)
but it parses an ENVIRONMENT VARIABLE, not an authored value — a different
domain, so the qualifier "an authored value" is what makes the sentence true.
`quality.rs` is stronger still: the bypass it describes (three consumers, one
reading the resource and two calling `resolved_budget()` directly, so a forced
`potato` materialized half a room's cast at each tier) is GONE — the only
callers left are inside the authority's own implementation, plus a comment
warning against it.

⛔ **`construction/mod.rs` was the third fix, and a new shape: the claim was
not false, it named the WRONG SUBJECT.** "This is the only executor" sat on
`commit_subset`. There IS one executor and it is the private `execute`;
`commit` and `commit_subset` are its two thin shapes. The structure was right
the whole time — an authority sentence pointing at one of two callers invites
someone to add a third beside it.

⭐⭐ **FIRST SWEEP — SEVEN CLAIMS CHECKED, TWO WERE FALSE.** The tree states its authority
invariants in prose -- *"the sole writer"*, *"the single definition"*, *"the
only writer of X"*. Those sentences are the closest thing this program has to a
declared invariant, and **nothing checks them**. Grepped
`sole caller|sole writer|single definition|only writer|sole authority` across
this session's lane and tested each by reading the code it sits on:

```text
camera_ease.rs:224   only writer of CameraShakeState "on behalf of the simulation"   HOLDS
camera_ease.rs:513   only writer of FinishZoomState  "on behalf of the simulation"   HOLDS
asset_manager:367    resolve_local_file_path is the sole caller                      HOLDS
audio_registries:45  sole authority for Ambition's audio catalog fragment            HOLDS
external_effects:128 "simulation is not the only writer" (a NEGATIVE claim)          n/a
boss_encounter:351   "the single definition of the cleared predicate"                FALSE
encounter/lifecycle  "the reducer is the only writer of `phase`"                     FALSE
```

⭐ **The two survivors of the qualifier are the interesting part.** Both camera
claims hold ONLY because of the phrase *"on behalf of the simulation"* -- each
state has a second writer (its own `tick_*` decay). An unqualified version of
either sentence would have been false. ⇒ **a scope phrase in an authority claim
is load-bearing, and dropping it while "tidying" a comment turns a true sentence
into a false one.**

⛔ **AND A FALSE ONE INVITES THE WRONG REFACTOR, which is why these are not
cosmetic.** `boss_is_cleared` claimed to be the only reading of its fact while a
second reading sat 60 lines away. I started to unify them before finding the
reason the second must stay: the authored road needs the STATE, not the
predicate, because it must say WHY a false answer is false. The sentence was the
invitation to make the code worse.

**Both false ones are closed, and one structurally** — `EncounterLifecycle::phase`
is now `pub(crate)` behind a `phase()` reader, so no consumer crate can assign
it (poison-verified: a foreign-crate write fails E0616).

⚠ **AND THE SEAL FOUND A WRITER THE GREP COULD NOT.** Sizing said "zero
production writes outside the file", from a grep for `lifecycle.phase =`. The
compiler then named `snapshot_impls::decode`, which rebuilds `phase` on a
rollback restore -- invisible because a codec builds a STRUCT LITERAL, not an
assignment. ⇒ **a name-based grep cannot answer a question about a field; seal
it and read the compiler's list.** That is why the seal is `pub(crate)` and not
private, and it is a caution for every other "sole writer" row in this program:
the rollback codec is a writer of nearly everything, and it never appears in a
grep for assignments.

✔ **THE CODEC CAVEAT WAS APPLIED BACK TO THIS AUDIT'S OWN SURVIVORS, 2026-09-05.**
If a type is rollback-registered, its codec's `decode` writes every field it
rebuilds, so an "only writer" claim on it is suspect by construction. Checked
the two closest survivors: **neither `CameraShakeState` nor `FinishZoomState` is
rollback-registered or has a `SnapshotState` impl** — they are presentation
state, correctly outside rollback — so those verdicts stand unqualified. ⇒ the
caveat bites only on types with a snapshot impl; `git grep -l 'impl SnapshotState'`
is the suspect set for any future sweep.

⭐⭐ **AND THAT TRIAGE FOUND A REAL FALSE CLAIM IN THE OTHER LANE THE SAME DAY,
which is what makes this a class rather than one file's habit.** The fighter
session ran it over `crates/ambition_combat` / `ambition_characters` — five
types with snapshot impls (`BodyCombat`, `ActorPose`, `SmashHoldState`,
`MovePlayback`, `BodyMelee`) — and `project_moveset_melee_to_body_melee`
declared itself *"the SOLE writer of a `MovesetMelee` body's swing"*,
unqualified. `BodyMelee` is rollback-registered as `actor.body_melee` and its
`decode` rebuilds `swing` wholesale, so the codec writes it on every rewind;
corrected to *"during live simulation"*. ⇒ **two independent lanes, one query,
one false claim each — and in both the claim had been checked the only way
anyone would check it, with a grep that could not fail.**

ⓘ Not proposed as a gate. These sentences are prose about six different kinds of
authority; a checker would need to understand each. The cheap discipline is the
one this audit used -- when you touch a comment claiming sole authority,
re-derive it, and keep the scope phrase.

### S1 — scenario-populated rollback coverage — the timeline half landed 2026-09-02

Boot-time/static registration checks cannot prove runtime-created authoritative
families. Representative scenarios should create the populations real gameplay
creates, then assert the required combination of:

- rollback participation;
- registered mutable state/remapping;
- stable semantic identity where needed;
- deterministic composition/selection;
- correct gameplay-session ownership.

Prefer domain-specific authoritative constructors/request types that make the
required invariants hard to omit. Do **not** introduce a universal
`spawn_sim_entity` wrapper merely so a source scanner can ban raw `spawn`.

Where it stands (re-verified 2026-09-02):

- **participation + registration, populated**: `rollback_coverage.rs` builds a
  live match, a boss arena, a strike volume, a mounted pair, the falling-sand
  room and the event-created set (sentry, vortex well, temporary gravity zone,
  falling hazard, portal shot) and sweeps each for unaccounted components and
  INERT registrations (a registered type on an entity with no anchor).
- ✔ **rewind stability while those families STEP**:
  `rollback_populated_timeline.rs` makes that same event-created population
  (plus a held-item bolt created by play) a fresh SyncTest baseline and
  resimulates 150 frames at check distance 7 — 852 replay comparisons — under
  TWO oracles. ⛔⛔ The session checksum alone was not one: 47 registrations are
  probed-only ("not in the session checksum"), and a sentry stepping from a
  process-global counter stayed GREEN under `rollback_health` for all 150
  frames. `RollbackRestoreAudit` is the oracle that sees them; the same poison
  fails it at frame 2 naming `Sentry`. A poison periodic in the check window
  (`n % 7` at distance 7) cancels and proves nothing — measured.
- ✔ **semantic identity across a rewind** is covered by the same timeline, and
  measured rather than inferred: `SimId` is canonical AND checksummed, the
  fixture fires a bolt every nine frames inside seven-frame check windows, so
  every mint is replayed — and minting from a process-global counter instead
  of the spawner's `SimIdCounter` (poison in `sim_identity.rs`) desyncs the
  session at frame 2. **Session ownership** is covered by
  `rollback_lifecycle_reset.rs` / `session_ownership_tests.rs`;
  **selection/composition** is S3.

### S2 — remove authoritative non-rewinding memory — ✔ CLOSED 2026-09-02

Every authoritative case this row named is now registered state, and the
remaining `Local<T>`s on sim-schedule systems were censused and classified:

```text
cutscene trigger room      ambition_cutscene::LastCutsceneRoom        registered (cutscene.last_room)
Mary-O active-room follower LevelDeparture.seen_room (component)     registered; rollback_room_memory.rs
quest room-entry edge      ambition_persistence::quest::LastQuestRoom registered+checksummed (2d73d9d94, schema v149)
```

Census of `Local<T>` on systems in the sim schedule after that (grep, sim-side
crates, tests/dev/census excluded): `tick_npc_idle_barks`'s bark timers —
presentation cadence that writes only `VfxMessage` (cleared on rollback), so a
resim re-emits bubbles and mutates nothing authoritative; `contact_scratch` and
`empty_relations` — per-run scratch cleared before use; `gated_lock_walls`'s
cached query state; everything else is host/presentation (`time_control`,
audio, menu input, the runtime's prefetch memory, the causal recorder's epoch).
None remembers an authoritative future.

⚠ Known limit of the quest fix's PIN: a confirmed room transition rebases GGRS
onto a new frame-zero baseline, so today no rewind crosses the room flip and the
old `Local`'s divergence was unreachable (the mary-o file records the same for
its follower). The guard is therefore structural — the resource restores the
producer's behaviour (`restoring_the_last_room_makes_the_producer_announce_the_
room_again`, red with the `Local` back) — not a desync reproduction. The move is
still right: a correctness only the rebase provides moves when the rebase does.

The original row, for the record:

Current review evidence still identifies gameplay logic whose `Local<T>`/edge
memory can remember a future state after rollback. Examples include the Mary-O
active-room follower and quest/room-visit edge detection.

For each case, decide whether the memory is:

- authoritative history that must rewind;
- derivable from rewound state and should be recomputed;
- presentation/diagnostic memory that must not mutate authoritative state during
  historical resimulation.

Move authoritative memory into registered state or eliminate it. Do not add it
to checksums while leaving the actual restore semantics unchanged.

### S3 — close remaining deterministic selection/composition sites — ✔ the three named sites, 2026-09-02

Current known sites include projectile-victim ties, possession candidates and
pickup-magnet ownership. Use the existing deterministic-selection vocabulary
where the operation is a true selection.

For composition problems, first state whether the operation is commutative. A
stable sort is not a semantic answer when reversing the same valid influences
changes the result.

Re-verified 2026-09-02:

- possession candidates (`abilities/traversal/possession.rs:226`) and the pickup
  magnet / collector (`features/ecs/pickups.rs`, and
  `crates/ambition_world_items/src/world_item.rs` since `69641a83f` carved the
  touched collectible out of the kernel) go through `sim_selection::winner_by`,
  whose final key is `SimId`;
- ✔ projectile victims: `step_projectiles` ordered its first-wins loop by
  distance-along-the-leg then the victim's position — and two bodies on one
  spawn point tie on all of it, so a stable sort handed the decision back to
  query order. `StrikeVictim` now carries the victim's `SimId` and the sort ends
  on it. Guard: `two_stacked_victims_are_struck_in_identity_order_whatever_the_
  archetype_order` (spawns `[a, b]` and `[b, a]`; red without the key). The
  boss/breakable arms are `any()` predicates, order-free by construction.
- censused the same day: every `.iter().find/min_by/max_by` in the sim-side
  crates (11 sites) is a lookup by unique key (`enc.id == target`, `entity ==
  body`, the portal partner) or a walk over an AUTHORED `Vec` (a boss's
  damageable parts, `world.blocks`) — none selects among peers by query order.
  The melee victim loop hits every overlapping body and has no first-wins to
  order. `for … in query { … break }` shapes outside these were not swept; the
  grep for them is noisy and the next one found should be added here.

### S4 — dynamic identity and provenance — the census half landed 2026-09-02

Runtime-spawned authoritative entities must use the runtime identity/provenance
road rather than borrowing authored-placement identity. Required component
relationships should make paired facts difficult to forget where the type system
can express the invariant.

Do not attempt to enforce semantic authored-vs-runtime provenance with a grep of
all `SimId::placement` call sites. A future typed `PlacementId` seam may be
appropriate when the refactor is justified by a concrete failure/customer.

✔ **Every rollback-anchored entity on the populated timeline carries one
unique `SimId`** — `every_rollback_anchored_entity_has_a_unique_sim_id_on_the_
populated_timeline` (`rollback_populated_timeline.rs`), with no waiver list.
The first run found four anonymous anchors and one collision, all repaired at
their seams: the grenade's gravity well and the encounter script's falling
hazard now mint `SimId::spawned(spawner, counter)` under the grenade / the
encounter (`open_temporary_gravity_well` and `drop_hazard` take the id); a
placed portal and the gameplay session's world root carry the new DERIVED
`SimId::singleton(kind, key)` (`portal:blue`, `session:<activation>`) — at
most one per key, so re-placing is the same object. The collision was the
fixture's: hand-minted `slot:0/0` met the subject's first bolt, which is why
the fixture now draws from the subject's own `SimIdCounter` like every
production spawner. Poison: drop the portal's id and the census names it.
The semantic half — a resimulation re-mints the SAME ids — is proven by the
S1 timeline (see S1: a process-global mint counter desyncs it at frame 2).

⛔ **"NO WAIVER LIST" IS TRUE OF THE ASSERTION AND NOT OF THE POPULATION, and
the difference cost a real defect (2026-09-04, yardrat).** The census's
`populate()` spawns five things — a sentry, a vortex well, a temporary gravity
well, a falling hazard and a portal shot — and **no `GroundItem` at all**. A
`GroundItem` *is* a rollback anchor (`ambition_held_items/src/lib.rs:1162`, and
`crates/ambition_platformer2d_actor_monolith/src/rollback_registration.rs:304`
says so again), and
`drop_held_weapon` was spawning one with provenance and no `SimId` on every
death of a body holding a weapon — every boss's signature gauntlet included. The
census was green because that class was outside what it populates, not because
the invariant held. Fixed at the drop site; see I2 in
[`item-custody-and-accounting.md`](item-custody-and-accounting.md).
⭐ **The general rule this row now owes its reader: a census with no waiver list
is only as strong as the population it walks, so widening `populate()` is the
way to strengthen it — not sharpening the assertion.**

⛔⛔ **AND THERE IS A SECOND AXIS THE RULE ABOVE DOES NOT NAME: *WHEN* THE CENSUS
LOOKS. Widening `populate()` could never have found what this one hid**
(2026-09-04). `every_rollback_anchored_entity_has_a_unique_sim_id_on_the_populated_timeline`
walked the world **only after sixty frames of play**, so its real population was
*"whatever survives sixty frames"* — and a transient anchor is outside it BY
CONSTRUCTION, at any fixture width.

⇒ **What was hiding there: a `PortalShot` is an anonymous rollback anchor.**
`require_rollback::<PortalShot>` puts it in the envelope
(`ambition_portal2d/src/rollback_registration.rs:48`) and it carried no `SimId`,
so it rewound by entity index while being the entity that decides *where a
portal opens*. Every shot has fizzled or placed long before frame 60.

⭐ **THE PROOF IS THE SECOND POISON, and it is worth more than the fix.** With
the shot anonymous AND the census restored to looking only at the end — the code
exactly as it shipped that morning — the run is **2 passed, 0 failed, GREEN**.
A real defect, present in the world, sailing through an assertion that has no
waiver list. ⇒ **An anti-vacuity floor is only as honest as the moment it is
asked at.** (Named jointly with the peer session, whose `GroundItem` find is the
same class on the WHAT axis: they widened what the fixture creates, this widens
when the census looks, and neither widening implies the other.)

⇒ The census now runs at the populated BASELINE and again after play. The class
floor is required at the baseline, where all six classes exist, and only for the
DURABLE classes at frame 60 — requiring a vortex well there would be asserting
that it never expires. ⚠ Its first failure said exactly that
(`["vortex well", "portal shot"]` missing at 60) and was the instrument
reporting a badly-timed question, not a defect; reading it as a defect would have
produced a guard pinning the wrong thing.

⇒ Fixed on the road the repo already had: `PortalFireIntent` carries the shot's
`SimId`, minted by the FIRER from its own `SimIdCounter` — the same
`Some(mint())` shape `deploy_sentry`, `open_vortex_well` and `drop_hazard` take,
so a resimulated tick re-mints the same id and two seats firing on one tick
cannot collide. ⚠ It costs the message its `Copy` (the id's payload is a
`String`).

✔ **AND THE MINT'S RESIM STABILITY IS MEASURED (2026-09-04) — but read S1 first,
because IT ALREADY SAID SO and I did not look.**
⛔ **This page's own S1 row has recorded since 2026-09-02:** *"semantic identity
across a rewind ... measured rather than inferred: `SimId` is canonical AND
checksummed ... minting from a process-global counter instead of the spawner's
`SimIdCounter` (poison in `sim_identity.rs`) desyncs the session at frame 2."*
⇒ **My run below is a RE-DERIVATION, not a new fact, and it cost a near-total
rebuild** — `shared_tangle` sits near the root of the graph — plus a live poison
in a tree shared with a peer. The recipe's rule fired and I missed it:
**re-reading an open row beats building.** Before poisoning anything, read the
owner doc's other rows for the same claim.
⚠ **What the second run does add is small and worth exactly its size:** a
DIFFERENT poison site (the counter's `next()` rather than the mint call) reaching
the same verdict, which makes the two independent rather than one result quoted
twice. Frame 9 here against S1's frame 2, because the poison is further from the
first mint.

⇒ The re-derivation itself, for the record: "A resimulated tick re-mints the same id"
follows from the counter being rollback state — but this program's own S1 row
records that REGISTERED ≠ CHECKSUMMED and that a real desync once read clean, so
the registration is not the proof. Poisoned `SimIdCounter::next()` with a
process-global `AtomicU64` drift term: the populated timeline reds at **frame
9**, naming *"GGRS sync-test checksum mismatch at frames [2, 3, 4, 5, 6, 7]"*.
⇒ The id is in the session checksum and this timeline sees an unstable one.
⭐ It is a MID-WINDOW mint that is exercised, not just the fixture's own five:
`busy` presses attack every ninth frame, so the subject spawns bolts from its
counter throughout the rolled-back window.

⛔ **A THIRD AXIS WAS PROPOSED AND DOES NOT SURVIVE CHECKING, and the retraction
is worth more than the caveat was.** The proposal (mine, adopting the peer
session's mechanism) was *"which SCHEDULE the census runs in — a world walked in
`Update` and a world walked in `FixedUpdate` are different populations on the
same frame"*. That is true of a census implemented as a SYSTEM. **This one is
not.** It is a direct world walk —
`world.query_filtered::<(Entity, Option<&SimId>, Option<&Name>), With<Rollback>>()`
on `sim.world_mut()` between steps — and a world walk sees every entity present
at that instant whatever schedule created it. Schedule membership cannot
partition it.
⚠ **I wrote the caveat into this page before checking it against the census's own
implementation**, which is the trap this program's own recipe names: a coherent
mechanism from a credible source is not a measurement, and a plausible caveat
sends the next reader somewhere there is nothing to find.

✔ **THE REAL RESIDUAL — the same `WHEN` problem at finer grain — IS CLOSED, and
closing it needed a THIRD census.** An anchor that never survives to a step
boundary is invisible to any between-steps walk. So
`no_anchor_rewinds_anonymously_on_any_frame_it_exists` runs the scan as a SYSTEM
in the sim schedule, observing the population every simulated frame (including
the frames a rewind resimulates) and accumulating what it finds — a panic inside
a GGRS schedule reports as a desync without naming the row, so the finding is
collected and asserted outside.

⛔⛔ **ITS REACH IS MEASURED, AND THE FIRST POISON FAILED — which is the finding.**
* **Poison A**: spawn an anonymous anchor and despawn it in the next chained
  system, both through `Commands`. **The scan does NOT see it** — and neither
  does anything else, because both commands apply at the SAME sync point and the
  entity never exists at a system boundary at all.
* **Poison B**: the same with an `ApplyDeferred` between the spawn and the scan,
  so the entity provably exists at a boundary. **RED**, naming
  `"poison transient anchor"`.

⇒ **The honest scope: this scan sees every anchor that exists at a system
boundary inside a step — which is every anchor any OTHER system can see.** The
wording it shipped with first ("anything shorter-lived than a step") over-claimed
and poison A is what caught it. ⭐ A guard's stated REACH is a claim like any
other and wants a poison of its own; the failing poison was worth more than the
passing one.

⛔ **A "the rest may be vacuous" note stood here and is WITHDRAWN.** It reasoned
that save/load are themselves schedule work, so an entity that never reaches a
sync point is never saved and cannot rewind at all. The peer session measured the
premise: production orders the portal step `.after(portal_fire_system)` rather
than `.chain()`, and switching the test to `.after()` still passes, **because
Bevy inserts a sync point automatically when a `Commands` writer is ordered
before a reader.** ⇒ Such an entity DOES reach a boundary and DOES get saved. My
reasoning was wrong in the direction that dismisses a real gap — the second time
today a note of mine labelled as reasoning turned out false while everything
labelled as measured held.

✔✔ **THE WITHIN-A-STEP GAP IS CLOSED — and this heading said "REAL AND STILL
OPEN" until 2026-09-05, directly above the measurement that reversed it.** The
body below already recorded the resolution; only the heading kept the old verdict,
so a reader skimming for open work found one that was not there.
⚠ That is the failure mode this whole section is about, wearing its own clothes:
a NEGATIVE result written down as a standing gap, and then not un-written when it
was overturned. **The retraction has to reach the sentence a reader arrives at
first.**

ⓘ The original finding, kept because the reversal is only legible beside it: A portal shot travels ~31.7px per 60Hz step, so one
fired within that of the fizzle line lives and dies inside a single `sim.step()`
— the sharpest known instance, and not hypothetical: it needs a player firing
about 32px from a wall, and the shot carries a minted `SimId` precisely so it
rewinds. I put that shot into the fixture and counted the frames the in-schedule
scan saw it on, by its own id:

| shot | scanned frames seen |
|---|---:|
| fired into open space (x=300) | **371** |
| fired ten pixels short of the fizzle line | **0** |

✔✔ **AND THE THIRD MEASUREMENT REVERSED THE FIRST TWO: the class IS walked, once
the scan is ORDERED.**

| shot | scanned frames it was seen on |
|---|---:|
| fired into open space (x=300) | 371 |
| fired at the fizzle line, scan UNORDERED | **0** |
| fired at the fizzle line, scan `.after(portal_fire_system)` | **1** |

⇒ **What was unreachable was the SCAN's position, not the entity's lifetime.**
One frame is the whole life of the shot. Bevy inserts a sync point when a
`Commands` writer is ordered before a reader of what it wrote, so naming the edge
is what makes the spawn observable at all.

⭐ **THE LESSON IS ABOUT THE FIRST RESULT, and it is the reusable half: a
NEGATIVE result is a claim about the instrument until you have varied the
instrument.** "0 frames" read as *the class is beyond this census* and was
written into this page as a standing open gap; it was really *the census was one
ordering edge away from working.*

⛔ **The edge is load-bearing and the fixture says so**: drop
`.after(portal_fire_system)` and the one-step counter reads zero — poison-verified,
and the failure names the edge rather than passing quietly.
⚠ The counter caught its own first error too: it counted ANY `PortalShot` and
reported 380 frames, because `populate` already fires a long-range one — an
instrument answering a wider question than the one asked.



### S5 — phase and ownership decomposition

Continue breaking high-authority systems where a split produces a real semantic
phase or domain owner. Useful criteria are:

- independent authoritative domains read/written;
- ordering constraints;
- query breadth;
- rollback participation;
- mutation during what should be proposal/decision;
- duplicated derivations of the same authority.

See [`../../architecture/bevy-system-boundaries.md`](../../architecture/bevy-system-boundaries.md) for the durable ECS-boundary rule.

A cohesive `SystemParam` or `QueryData` is good when it names one concept. It is
not a fix when it only hides the parameter ceiling.

### S6 — session-scoped process-resource residue

The current engine still has process resources that mirror one live gameplay
session. `SessionScopedResources` now re-establishes the known set on activation,
so skipped/misordered retirement is not the correctness boundary.

Treat additions to this set as migration pressure: decide whether the fact is
truly process-global, should remain a session mirror with activation semantics,
or belongs structurally under session-owned state. Do not mandate one storage
shape without a concrete ownership case.

The remaining `LocalSessionPolicy::check_distance` F9-proof-pulse question is a
maintainer/productivity decision, not evidence that all such resources need an
entity-based rewrite.

✔ **THE SET'S RESET IS STRUCTURALLY GUARDED SINCE 2026-09-04.** `reset` was
seventeen `*resources.field = Default::default()` lines — a hand-kept list in the
shape of code, where adding a resource to `SessionScopedResources` and forgetting
it here COMPILED and leaked that resource into the next session silently, which
is the one failure this whole set exists to prevent. It destructures the
`SystemParam` exhaustively now, so the omission is `error[E0027]: pattern does
not mention field`, verified by adding one. ⚠ And
`retirement_clears_every_session_scoped_mirror` asserts a SUBSET by hand: "every"
is the compiler's claim, not that test's, and its doc now says so rather than
inviting the list to grow.

⭐⭐ **THE COMPLEMENT WAS NEVER MEASURED, AND MEASURING IT FOUND ONE REAL
CANDIDATE (2026-09-05).** This row says to treat additions as migration
pressure, but the guard covers "in the set and not reset" — it says nothing
about "should be in the set and is not". Compared the runtime's
`sim_core_resources.rs` against the set:

```text
runtime init_resource types          36
session-scoped reset set             17  -> 18 after this finding
init'd but NOT session-scoped        31  -> 30
```

⚠ REASONED classification of the 31 (by name and by reading, not by a tool):
seven tuning/settings, six clock/tick, three content catalogs, two instruments,
one derived-per-frame, two save-backed and re-established through
`SaveRestored`, three presentation, four transient per-frame, two portal, one
overlay. Most are legitimately process-global.

⛔⛔ **ONE WAS NOT: `ProjectileSeqCounter`.** MEASURED — it is
`rollback_resource_canonical`, so its value is INSIDE THE STATE CHECKSUM, it is a
process-global monotonic counter, and nothing reset it. ⇒ session B's first
projectile id depended on how many projectiles session A fired, so two hosts
with different local histories could disagree at frame 0 in a checksummed value
while every entity matched. ✔ Added to the set; the exhaustive destructure then
forces its reset.
⚠ Not reproduced as a live desync, and whether a netplay session seeds initial
state from one peer is not established here. The asymmetry is what is measured.

⭐ **AND THE ONE THAT LOOKED WORSE WAS NOT A CANDIDATE AT ALL.** `SimIdCounter`
is a COMPONENT, not a resource — per-spawner, dying with its entity — so it is
absent from a resource set by construction. It also must NOT be naively reset:
`SimId`s reach the save (`PersistedOccurrence.id`), so a restarted sequence
could collide with saved ids. The distinction that makes the projectile counter
safe to scope is exactly that projectile ids are transient.

⛔⛔ **AND THE SET HAS A SECOND, UNGUARDED HALF.** `reset` is exhaustively
destructured (E0027 on omission, poison-verified twice now), but the TEST
HARNESS that must `init_resource` the same set is a hand-kept list of seventeen
lines. A missing entry is not a compile error — it is a runtime *"Parameter
`SessionScopedResources<'_>::<field>` failed validation: Resource does not
exist"* in three tests at once. ⇒ **adding a field here is TWO edits and only
one is guarded**, because the type system cannot enumerate a `SystemParam`'s
fields. The harness now carries that warning where it is read; a guard is the
fallback and here there is not even one.

Addition 2026-09-02: the two room-entry edge memories S2 moved out of `Local`s
(`LastQuestRoom`, `LastCutsceneRoom`) are in the set now. Each remembers "the
room I last announced" and fires only on a change, so inherited across sessions
a new game starting in the room the previous session ended in — quit at the
start, start over — skipped its first room's quest events and cutscene
trigger. The `Local`s had the same defect; S2 made it registered state and S6
makes it session-scoped. Guard: `retirement_clears_every_session_scoped_mirror`.

## State projection rule

Read models are allowed. They must be one-way projections from authoritative
state. If a projected component contains fields that another system mutates as
authority, split the representation or move the authority; do not preserve
exceptions by saving/restoring selected fields around a projection rebuild.

## Test topology

Use the host needed by the invariant:

| Property | Required proof shape |
|---|---|
| deterministic simulation | headless real `GgrsSchedule` / `SyncTestSession` |
| runtime-created rollback population | scenario that actually creates it before rewind |
| cross-game/session isolation | shell/app host that creates, retires and creates real sessions |
| physical input/rebinding lifecycle | real input/session host |
| rendered materialization/raster behavior | rendered hardware measurement |
| durable persistence | fresh-process reconstruction |
| capability combinations | explicit supported feature/product matrix |

A simplified host is useful only if it still contains the composition property
being asserted.

## Acceptance

- adding a rewindable domain type changes its domain declaration, not a central
  concrete-type census;
- representative dynamic authoritative families survive rewind/recreation with
  correct identity and deterministic behavior;
- no known authoritative edge/history uses non-rewinding `Local<T>` memory;
- peer selection/composition does not depend on raw ECS iteration order;
- one gameplay session cannot consume another session's rollback health or live
  session mirror state;
- same-session rollback rebases cannot clear a real desync by accident;
- major simulation systems have named authority/phase contracts rather than
  parameter packing disguising breadth.

## Explicit non-goals for the current program

- another custom rollback/snapshot engine;
- pushing `bevy_ggrs` into every leaf domain;
- a universal raw-spawn wrapper;
- exhaustive pairwise schedule edges duplicating semantic phase structure;
- scheduler parallelism as an architecture objective;
- source-text policy where a type/API/runtime test can make the invariant
  structural.
