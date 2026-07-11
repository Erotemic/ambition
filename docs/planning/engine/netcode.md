# Netcode — determinism as a contract, multiplayer as a ladder

**Authored by fable, 2026-07-05.** How multiplayer enters the engine without
ever being a rewrite: each rung of the ladder is independently shippable,
each hardens an invariant the next rung needs, and the first rung is nearly
free. Super Smash Siblings ships on rung N1; online play is post-1.0 but its
SEAMS are paid for now, exactly like slower-light's Tier-0.

Standing architecture facts this builds on: the two-port body (controllers
attempt via `SlotControls`/`ActorControl`; bodies enforce) means a "remote
player" is just another controller backend; bit-identical replay fixtures
already pin determinism as a canary; time domains (ADR 0010/0011) separate
sim time from feel time; the E4 read-model split gives presentation a
confirmed-state boundary rollback needs.

---

## Q4 — THE DECISION BRIEF (Jon: this is the context you asked for)

**The question:** how strong a determinism promise does the ENGINE make?
Determinism = "same inputs ⇒ same simulation states." The strength of the
promise decides how much discipline every future sim system must carry.

**Three levels, with what each buys and costs:**

1. **Canary only (status quo).** Bit-identical replay tests exist but may
   be re-baselined freely; nothing is promised.
   *Buys:* zero ongoing discipline. *Costs:* rollback netcode, Braid-style
   rewind, and reproducible RL training all become fragile or impossible —
   each would need its own ad-hoc determinism audit later, against code
   that was never held to the rule. Retrofitting is the expensive path.
2. **Same-build contract (what M20 proposes).** Promise: the SAME binary,
   on the SAME platform, fed the SAME per-tick input stream, produces
   identical sim states — forever, enforced by CI (two sims, hash per
   tick, first-divergence report). Requires: a fixed-tick sim option
   (presentation interpolates), serializable per-tick input streams, and
   the standing hygiene rules we already follow (stable iteration order,
   no wall-clock/HashMap-order in sim). f32 math is FINE at this level —
   floats are deterministic on one binary; we just can't reorder ops
   between "runs," which same-binary guarantees.
   *Buys:* replay/RL reproducibility as a product feature, desync
   forensics, same-build online lockstep AND rollback (both peers run the
   same binary — the normal case for an indie game), rewind mechanics,
   and the fighter brain's forward rollouts. *Costs:* the N0 slices below
   (~small), plus a permanent-but-light discipline tax on new sim code
   (the lint set makes it mostly automatic).
3. **Cross-platform bit-exact.** Same states across different OS/CPU/
   compiler builds. Requires software-float or fixed-point math, no std
   trig, audited transcendentals — a deep tax on every kernel.
   *Buys over level 2:* only cross-platform lockstep between DIFFERENT
   binaries and platform-independent replay files. *Not worth it* for
   this engine's goals; explicitly a non-goal.

**Recommendation (M20): level 2.** It is the knee of the curve — nearly
all the value, small cost, and it must be chosen NOW because every sim
system written after the choice either respects it cheaply or violates
it expensively.

### ✅ Q4 RESOLVED — "same-build now, cross-platform later" (Jon, 2026-07-06)

**Level 2 (same-build) is ACCEPTED for now.** Same binary / platform /
input stream ⇒ deterministic enough for tests, replay, and desync
canaries. Cross-platform bit-exactness (level 3) is NOT promised now — but
**do NOT code the architecture into a corner against eventual
cross-platform determinism.** Concretely, every sim system carries these
guardrails from now on (they cost ~nothing at level 2 and keep level 3
reachable without a rewrite):

- Stable, behavior-affecting iteration order (sort by stable id, never
  `Entity`; no `HashMap`/`HashSet` iteration driving sim outcomes).
- Deterministic, seeded RNG STREAMS (no global/thread RNG; per-owner or
  per-tick seeded streams — a seed is reproducible and portable later).
- No wall-clock reads in the sim (`WorldTime`/proper-time only — never
  `Instant::now`/system time in a sim system).
- No accidental hash-order semantics anywhere sim state depends on it.
- Snapshot + input-stream FORMATS (N0.2, N3.1) chosen so they do not
  preclude cross-platform determinism later (explicit field order, no
  platform-width-dependent encoding, versioned).

Stable authored/spawn IDs "where practical" (see decomposition [Q-FABLE
W-d]) support this — they are the portable identity level 3 and rollback
both want.

## N0 — determinism is a MANAGED contract (Q4 CONFIRMED: level 2)

The ladder needs the level-2 **scoped guarantee**: *same build, same
platform, same inputs ⇒ same sim states.* (Cross-platform float
determinism is explicitly NOT promised — but the N0.2/N0.3/N3.1 formats +
the guardrails above keep level 3 reachable.)

Obligations (each a slice, all [opus]):

- **N0.1 Fixed-tick sim mode — ✅ LANDED (opus, 2026-07-09).** The two-clocks
  design below was ruled by fable and is implemented as ruled. What shipped:
  `SimSchedule` + `App::sim_schedule()` in `platformer_primitives::schedule`;
  every engine sim plugin, `configure_sandbox_sets`, the content plugins, and
  the app-local sim residue register into it; `PlatformerEnginePlugins {
  fixed_tick }` hosts the sim in `FixedUpdate` on `Time<Fixed>` at
  `SIM_TICK_HZ = 60`; `SimTick` (in `ambition_time`) is the canonical timeline;
  `ControlFrameLatch` (in `engine_core`) is the frame→tick input latch, owned
  by the DEVICE layer (`ambition_host`). Exit check met: the rl_sim
  `player_phase_split` / `actor_phase_split` suites pass with the label
  threaded BOTH ways, plus a split-brain guard in
  `ambition_host/tests/demo_shell_smoke.rs` that fails if any sim system is
  stranded in `Update` under fixed tick.

  **Executor deviation from the ruled mechanism (vision §7 — the case, not a
  silent drift).** Fable ruled: *"each engine-group schedule plugin (and
  `configure_sandbox_sets`) gains a `schedule: InternedScheduleLabel` field
  defaulting to `Update`; `PlatformerEnginePlugins` becomes a struct with a
  `fixed_tick: bool` knob that threads the label to every member."* The plugin
  group's knob shipped exactly as ruled. The per-plugin FIELD did not: plugins
  read the label from a `SimSchedule` resource via `app.sim_schedule()`
  instead. Why:
  1. **The field is viral past the engine group.** ~14 of the ~25 sim
     registrations that must move are NOT engine-group members — they are
     content (`ambition_content`: bosses, falling sand, intro, portal adapters,
     quests) and the app-local residue. A field-threaded label makes every
     downstream game's content plugin grow a `schedule` field and every demo
     crate re-thread it. That is a tax on the reusability oracle ("could
     another platformer be built by ADDING a content crate without editing
     core?"). The resource is one call, `app.sim_schedule()`, from anywhere.
  2. **Content builds BEFORE the group** in Ambition's own app, so a field on
     the group could never have reached content anyway. Some app-level channel
     was always required; having two mechanisms is worse than having the one.
  3. **The implicit-read hazard is closed structurally, not by documentation.**
     `SimSchedule` seals on first read: changing the label after any plugin has
     committed systems panics, naming both labels. The failure mode the field
     was protecting against (half the sim in `Update`, half in `FixedUpdate`)
     is now a startup panic and a schedule-graph guard test, not a silent
     ordering loss.

  Everything else is as ruled — the two clocks, bullet-time inside the tick,
  per-tick input latching, `FixedUpdate` hosting, and the exit check.

  **Known remainder (not blocking N0.2/N0.4), recorded honestly:**
  - *Presentation interpolation is not implemented.* Under fixed tick,
    presentation reads the last completed tick's read-model with no overstep
    interpolation. Nothing ships fixed-tick with a window yet, so this is dead
    code today; it lands with the first fixed-tick visible app (velocity
    extrapolation from `BodyPoseView`, per the mechanism note below).
  - *`WorldTime::wall_dt()` means "unscaled SIM dt", not wall dt.* Under fixed
    tick `refresh_world_time` runs inside the tick, so `raw_dt == TICK_DT`.
    For sim readers that is correct and MORE deterministic (possession's
    hold timer, the OOB trace). For the three presentation readers
    (`render::fx`, `render::deep_dream`, `actors::audio::environment`) it is
    wrong at any refresh rate ≠ 60 Hz. Splitting a real `wall_dt` field off
    `WorldTime` is the fix; it only bites a fixed-tick *windowed* app.
  - *One frame of device→tick input latency.* `RunFixedMainLoop` runs before
    `Update` in Bevy's `Main`, so a device sample taken in `Update` reaches the
    tick on the next frame. Standard for fixed-tick, and the latch is what
    makes it lossless. Moving the device→latch bridge to `PreUpdate` would
    remove it if it ever matters.

  **The ruled design (fable, 2026-07-06 night) — do not re-derive:**
  - **The two clocks are:** (1) the **SIM TICK** clock — fixed 60 Hz, the
    only clock sim systems advance on in fixed-tick mode; the tick COUNT
    is the canonical timeline (N0.2 streams and N0.4 hashes key on it);
    (2) the **FRAME/FEEL** clock — the render frame's raw dt, driving
    presentation interpolation, device sampling, and the per-player
    feel-time effects (the ADR 0010/0011 split — this is what it was for).
  - **Bullet-time composes INSIDE the tick, never with the tick rate:**
    `time_scale` (ClockScaleRequest pipeline) scales `scaled_dt =
    TICK_DT × time_scale` while the tick cadence stays fixed. Never
    scale the accumulator; a slowed world still ticks 60 Hz (determinism
    + netcode need the fixed timeline; the sim just moves less per tick
    — which is ALREADY the semantics of `WorldTime::scaled_dt`).
  - **Input latching:** devices sample per FRAME (feel); the input bridge
    LATCHES frame samples into ONE per-tick `SlotControls` frame — axes
    take the latest sample, edge/press events OR across the frames inside
    a tick so a sub-tick tap is never lost. The per-TICK frame is what
    N0.2 records and what the sim consumes.
  - **Mechanism (Bevy-native):** fixed-tick mode hosts the SIM sets in
    `FixedUpdate` (Bevy's `Time<Fixed>` accumulator; presentation stays
    in `Update` and interpolates read-model states by the overstep
    fraction). Execution is a mechanical THREADING slice: each
    engine-group schedule plugin (and `configure_sandbox_sets`) gains a
    `schedule: InternedScheduleLabel` field defaulting to `Update` —
    `PlatformerEnginePlugins` becomes a struct with a `fixed_tick: bool`
    knob that threads the label to every member plugin. *(Executed as a
    `SimSchedule` resource read by `app.sim_schedule()` instead of a
    per-plugin field — see the deviation case above. The group's knob is
    as ruled.)* Default stays
    frame-stepped (Ambition today, byte-parity); SSB/demos opt in.
    Presentation interpolation reads previous+current tick pose from the
    read-model (BodyPoseView carries pos+vel — velocity extrapolation is
    the cheap v1; a two-tick pose buffer is the v2 if extrapolation
    visibly jitters). **Not yet implemented** — see the remainder above.
  - **Ordering guard:** the rl_sim schedule-shape tests must pass with
    the label threaded BOTH ways (parameterize one suite run over
    Update/FixedUpdate) — that's the exit check. ✅ met: `SandboxSimOptions
    ::with_fixed_tick` parameterizes `player_phase_split` and
    `actor_phase_split`.
- **N0.2 Input-stream capture as a first-class type — ✅ LANDED (opus,
  2026-07-09).** `ambition_engine_core::InputStream` — versioned
  (`INPUT_STREAM_VERSION`), serde, per-tick `SlotControls` keyed by `SimTick`,
  contiguous, `validate()`d on load. Explicit field order, `u64`/`u32` only (no
  `usize`), so it does not preclude level 3. `ControlFrame` gained
  `#[serde(default)]`, so ADDING a field never bumps the version: an older
  stream loads with the new field neutral, which is what it meant.
  `ambition_runtime::InputStreamRecorder` is the ONE capture path, recording
  `SlotControls` after the input phase finalizes them — the frame the SIM
  consumed, not the one the device produced (gestures, portal warp, and the
  fixed-tick latch all rewrite it in between). `SandboxSim::step_frame` drives a
  raw `ControlFrame`, so replay no longer round-trips through `AgentAction`,
  which silently drops `shield_held` / `aim_*` / the projectile verbs.

  Exit: `game/ambition_app/tests/input_stream_replay.rs` records a scripted
  session (run, jump, reverse, dash), validates, JSON round-trips, and replays
  the DECODED stream into a FRESH sim with zero divergence at every tick —
  capture, transport, and replay, which is N0.4's comparison in miniature.
  `replay_fixture_regression.rs` was promoted onto the type: its untyped
  `serde_json::Value` field-pokes are gone. Noted while doing so: that fixture's
  60 ticks are entirely NEUTRAL input, so on its own it only ever proved that a
  falling body falls the same way.
- **N0.3 Determinism lint set — 🟢 LANDED (2026-07-10; re-audit finding 1 closed).** The four rules
  (no ambient randomness; no wall-clock reads; no std-hash-order semantics; no
  `Entity` as an ordering key) are greps over non-test source under `crates/*` AND
  `game/{ambition_content,ambition_demo_sanic,ambition_demo_smb1}`,
  in the `engine.determinism` policy (`tests/ambition_workspace_policy/src/custom/determinism.rs`; migrated 2026-07-10 from `crates/ambition_runtime/tests/determinism_lints.rs`), with an
  auditable `AMBITION_REVIEW(determinism)` escape hatch. The doc page is
  **ADR 0023**. Each lint is poison-tested (a violation injected into a real sim
  source makes it fail), so none of them passes vacuously.

  It did not merely codify accidentally-true properties — it found a REAL
  violation: `features::ecs::attack::start_body_melee` iterated a
  `std::collections::HashSet<Entity>` and, inside that loop, spawned strike
  entities and wrote sfx/vfx/hit messages. `RandomState` is seeded per PROCESS,
  so two runs of the same binary on the same inputs could swing two bodies in
  opposite orders — a level-2 violation on the hottest combat path. Fixed by
  deduping in message order. Two commutative-but-hash-ordered sites became
  `BTreeMap`s (`compute_holding_positions`, the smash variety metric), and four
  genuinely-unobservable ones carry the marker.

  **Rule 3's detector had a hole, and a real bug was living in it (2026-07-10).**
  The lint required the fully-qualified `std::collections::HashMap` *on the binding
  line*. Every idiomatic Rust file imports the name and then writes it bare, so the
  rule saw almost nothing it was written to see. Widening it to the bare-but-imported
  spelling — while still exempting `bevy::platform::collections`, whose `FixedHasher`
  is legal at level 2 — immediately found
  `ambition_characters::perception::WorldMemory`:

  > `actors: HashMap<String, RememberedActor>`, and `last_known_hostile` takes the
  > `max_by` confidence over `.values()`. **Two hostiles in view are both at
  > confidence `1.0`**, so the tie was broken by `RandomState` — the enemy chased a
  > different player on every run of the same binary on the same inputs.

  Now a `BTreeMap`, so `max_by` keeps the greatest id. Not a tiebreak anyone would
  *choose*; a tiebreak that EXISTS, which is the whole requirement.
  `rule_three_sees_a_bare_hashmap_and_not_a_bevy_one` poison-tests both spellings and
  both exemptions, because *a lint that only sees the spelling its author had in mind
  is not a lint.* `ambition_world::placements::registered_kinds` picked up the marker:
  it sorts the keys on the very next line, at room load, outside the tick.

  **Scope widening — DONE (2026-07-10).** N0.3 now scans `game/ambition_content` and
  the demo-rule crates (which schedule portal, falling-sand, boss, and other simulation
  systems), and its widening immediately caught two real `falling_sand.rs` rule-3
  violations. A self-check asserts each `game/` root is actually reached, so the widened
  scan cannot pass vacuously.

  **Manifest-dependency scan un-vacuumed (re-audit finding 1, 2026-07-10).** When the
  `game/` roots were added, the RNG-dependency half still joined `root/crates/<full-path>`,
  producing `crates/crates/..` and `crates/game/..` — every manifest read failed, every
  crate was silently skipped, and the dependency scan read ZERO manifests while passing
  green. Fixed: `root.join(krate)`, PANIC on an unreadable listed manifest, and an assert
  that one manifest is read per listed crate. N0.3 is now LANDED.
- **N0.4 Desync canary rig — 🟢 LANDED as the canary mechanism (2026-07-10).**
  `game/ambition_app/tests/desync_canary.rs`: two `SandboxSim`s, one seeded input
  stream, the registered sim state hashed every tick, first-divergence report that
  names the offending REGISTRY ENTRY (a desync you cannot name is a desync you
  cannot fix). **3 rooms × 240 ticks, in sync.** Poison-tested both ways — a
  different input stream must diverge, and moving one body must change the hash —
  because a canary that cannot cry proves nothing.

  **Fixture-coverage correction — DONE (2026-07-10).** Required-room construction no
  longer skips. `try_sim` returns `Result` and refuses a room that fails to build OR
  silently falls back to another room; `sim()` is `unwrap_or_else(panic!)`, a HARD
  failure that takes the gate down rather than passing vacuously; and
  `a_missing_required_room_is_a_hard_failure_not_a_skip` is the poison test that pins
  it. The earlier skip/return paths that collapsed measured peak debt to zero are gone.

  The canary MECHANISM is landed and defended. What it can PROVE is still bounded by
  the registered set (decision (1)) and by N3.2's exact-restore work — the two are
  built together, so the canary strengthens as `SimId` and the codecs reach the rest
  of the sim. That is the honest reading of decision (1), not a loophole.

  Built on N3.1's registration seam, as this section required
  (`ambition_runtime::snapshot`). The hash is FNV-1a, never
  `std::hash::DefaultHasher`: `RandomState` is seeded per process, and a canary
  that changes its mind between runs is the bug class ADR 0023 exists to prevent.
  Entity rows are sorted by stable key before hashing, because Bevy's `Query`
  order follows archetype layout and would otherwise cry desync on every run.

  **What it covers is what is registered**, and today that is the sim tick, the
  scaled clock, and every body with a stable id (`FeatureId`, or slot 0). *What
  the canary cannot see, it cannot defend* — and the set grows as `SimId` reaches
  the rest of the sim. That is the honest reading of decision (1), not a loophole.

## N1 — local multiplayer (ships with Super Smash Siblings)

The engine was built for this: `PlayerSlot(n)` exists, possession proves
any body is drivable, `SlotControls` routes per-slot intents. Missing is
only the host wiring:

- **N1.1 Input device → slot binding.** N gamepads/keyboard-splits map to
  slots (leafwing already distinguishes devices); a small binding resource +
  join/leave flow (press-start-to-join is demo UI, the BINDING is engine).
- **N1.2 N controlled bodies.** Spawn per-slot bodies (the guest-player
  composition path already sketched in `PlayerIdentityBundle` docs); the
  `ControlledSubject` seam generalizes from "the one subject" to per-slot
  subjects — audit the ~dozen `ControlledSubject` readers for primary-player
  assumptions (this is the real work; grep-driven, mechanical).
- **N1.3 Camera policy.** Shared-arena framing (bounding box of subjects +
  zoom clamps) as a `CameraZoneSpec` policy — SSB needs it; authored data.

## N2 — deterministic lockstep (first online rung, post-1.0 candidate)

Same-build peers exchange input streams with a small input delay; no
snapshots needed. Needs N0 complete, plus:

- **N2.1 Transport trait + session shell** — a thin `InputTransport`
  (send/recv tick-stamped `SlotControls`) with a loopback impl for tests;
  matchmaking/lobby is out of engine scope.
- **N2.2 Ecosystem evaluation** (use-existing-packages rule):
  `bevy_matchbox` (WebRTC transport), `bevy_ggrs` (see N3 — it also does
  lockstep-with-delay). Adopt, don't rebuild, unless the evaluation
  documents a rejection.

## N3 — rollback (the real thing, explicitly post-1.0)

- **N3.1 Snapshot/restore keystone — 🟡 LANDED, exactness OPEN in N3.2
  (audit correction 2026-07-10).** The registry, `SimId` vocabulary, shared
  snapshot/hash bytes, take/restore mechanics, coverage measurement, and replay
  oracle are valuable. The surrounding prose overstated what they prove:
  uniqueness, complete mutable-state coverage, codec failure semantics,
  active-room ownership, and dynamic-spawn reconstruction remain open.

  - `ambition_runtime::snapshot`: `SnapshotRegistry` (opt-in per plugin, decision
    1), `StateHasher`, `hash_entities_by_key` (the stable-order rule),
    `register_engine_sim_state`. N0.4 rides it.
  - `ambition_platformer_primitives::sim_id`: **`SimId`**, the one identity
    vocabulary — `placement(id)` (an LDtk iid / `FeatureId`), `player_slot(n)`,
    and `spawned(spawner, counter)`. It is a *`String`* on purpose: a desync report
    that says `placement:BossSpawn-4308/3` names a projectile fired by a boss;
    `9f3ac21e` names nothing. `SimIdCounter` lives on the SPAWNER, never global —
    a global counter couples unrelated spawners.
  - `ensure_sim_id` covers the two authored identities; `mint_spawned_sim_ids`
    covers projectiles, ordered by the existing `ProjectileSeq` (a global counter
    is forbidden for *identity* but is a perfectly good *total order*).

  **`the_sim_id_migration_ledger` currently measures zero anonymous bodies**
  across `gap_run` / `portal_lab` / `mockingbird_arena` / `gnu_ton_arena` when
  those fixtures load. It is not yet a trustworthy gate because a failed room
  construction is skipped, and duplicate `SimId`s are not rejected. Series 1
  hard-fails the fixtures; Series 2 adds the uniqueness invariant and its poison
  test. A real rise still means a spawn site shipped without minting identity.

  **`take` / `restore` — ✅ LANDED 2026-07-10** (the sketch below, executed).

  - **One serialization, two consumers.** N0.4's line *"hash = the snapshot
    serialization of N3.1 — build them together"* is taken literally: a component
    implements `SnapshotState` once, and its bytes are BOTH what the canary hashes
    and what `take` stores. There is no second encoder to drift. A codec that drops
    a field is caught by `every_engine_codec_round_trips_exactly` (the property is
    `encode ∘ decode ∘ encode == encode`, on bytes) and by `Reader::finish`, which
    rejects a decoder that leaves bytes on the floor.
  - `restore` **reconciles by `SimId`**: an entity in both worlds is *patched* in
    place (every registered component overwritten from its blob; one the snapshot
    lacks is *removed*), one only in the snapshot is *respawned* from blobs, one only
    in the world is *despawned*. All three fall out of *"the snapshot is the truth"*
    rather than out of a diff. `take` after `restore` returns the snapshot it
    restored from.

  **DEVIATION from decision (3), and the case for it.** The sketch rules *"restore =
  despawn-registered + respawn from blobs (no in-place patching — simpler, and
  room-reset already proves the world can rebuild)"*. Despawn-everything shipped
  first, and it is wrong for the case a rollback is made of.

  A sim body carries two kinds of component. **Authored config** — its brain, its
  moveset, its action set, its faction — is immutable for the body's life and is
  created by the room spawner from content. **Mutable state** — kinematics, meters,
  timers, cooldowns — is what the sim advances. Rewinding must restore the second and
  must not disturb the first. Despawn-and-respawn destroys *both*, and then obliges
  the registry to carry authored config in every blob of every tick of the rollback
  buffer so respawn can put it back. That is not simpler; it is a serialization of the
  entire content pipeline, sixty times a second.

  Patching the survivors is no more complex — the despawn and respawn paths still
  exist, for exactly the entities whose EXISTENCE changed, which is the case decision
  (3) was really reasoning about and the one where *"room-reset proves the world can
  rebuild"* actually applies. Measured on `gap_run`: the difference between a restore
  that destroys **53 component types** and one that destroys **none**.

  **Deviation from the sketch, stated rather than drifted.** The sketch has
  `SimSnapshot { tick, blobs: Vec<(StateTypeId, Box<[u8]>)> }` — one flat byte
  string per entry. Entity rows stay STRUCTURED (`Vec<(SimId, Vec<u8>)>`) instead,
  because decision (3) makes `restore` group rows by `SimId` across entries to
  respawn one entity carrying all of its components; a flat blob would be re-split
  on `restore`'s first line, and that parse could fail. This one cannot. The wire
  format — where `Box<[u8]>` and a version tag earn their keep — is N3.3's, and it
  serializes exactly this, which is why the per-entry bytes are already canonical,
  explicitly ordered, and free of `usize`.

  **What restore cannot REWIND, it reports — and the report is a gate.**
  A patched entity keeps every component the registry does not know about. An
  immutable authored fact is *correct* left alone; a timer is **stale**, still reading
  the tick we rewound FROM, and it is that timer that makes a replay diverge.
  `SnapshotRegistry::unclaimed_components` cannot tell the two apart, so it reports
  both: every component on a `SimId` entity that is neither registered nor
  `declare_derived`'d. `RestoreReport` returns that set at every call as
  `stale_components`, alongside `unidentified_survivors` — bodies with no `SimId`,
  which `restore` cannot touch at all and which therefore *walk out of a rollback*.
  A projectile in that set outlives its own un-firing.

  `the_snapshot_coverage_ledger` in `ambition_app` prints and pins the debt:

  | room | component types a rewind leaves stale | rewind is exact? |
  |---|---|---|
  | `gap_run` | 28 | ✅ **yes** |
  | `mockingbird_arena` | 49 | ✅ **yes** |
  | `gnu_ton_arena` | **59** | ✅ **yes** |
  | `portal_lab` | 54 | no |

  **`gnu_ton_arena` carries the LARGEST stale count of any room and rewinds exactly.**
  That is the ledger's own disclaimer, demonstrated: for an immutable authored fact,
  stale and correct are the same thing. The number is an upper bound on the debt, and
  the exit oracle is the only thing that measures the debt.

  Pinned at 59 — the **peak over the run**, not the count at its end. The first
  version of this ledger measured once, after 120 ticks, by which time the arena
  bosses were dead and despawned; `gnu_ton_arena` duly reported the same 35 types as
  `gap_run`, which is the count of a world containing only the player. The debt was
  real the whole time. The instrument was looking at the wrong tick, and a ledger that
  under-reports is worse than no ledger. It samples every 20 ticks now and keeps the
  worst. It may fall; it may not rise. The count is an *upper bound* on the
  debt, not the debt: for an immutable authored fact, stale and correct are the same
  thing. The exit oracle is what measures whether stale state actually leaks.

  The ledger keys on `TypeId` (always exact); component NAMES need `bevy_ecs/debug`,
  which `ambition_app`'s test graph happens to enable and `ambition_runtime`'s does
  not — so the counts are trustworthy in both and the names are readable where it
  matters. Lower it by registering a component, or by `declare_derived::<C>()`, which
  is a *promise* that the same per-frame system that maintains `C` rebuilds it —
  N3.1's own no-restore-only-code rule, made into an API call.

  **`a_restored_sim_replays_the_future_it_was_rewound_from` is N3.1's exit oracle,
  and `gap_run` PASSES IT.** Take, run K ticks hashing each, restore, replay the same
  K inputs, demand identical hash streams. `body_kinematics` is in the hash, so
  "unregistered state leaked" and "anything moved differently" are the same event. A
  plain platformer room now rewinds and replays bit for bit, 60 ticks deep.

  The other three rooms do not, and the oracle **asserts that they do not** — fix one
  and the test fails, telling you to promote it. A ledger you can only satisfy by
  lowering it is not a ledger. Its sibling,
  `a_restore_of_a_real_room_is_exact_where_it_is_registered_and_honest_where_it_is_not`,
  pins the other half: `restore` reproduces the registered hash bit for bit, leaves
  zero unidentified survivors, and names every type it left stale.

  The current keystone records **60 registry entries across five kinds**
  (component, cursor, resolved, resource, resource-cursor), plus registered message
  channels and declared-derived state. The coverage ledger, not an inline hand count,
  is the inventory authority. *A coyote timer that survives a rollback is a jump the
  player did not earn; an attack cooldown that survives one is an attack the enemy did
  not pay for.*

  `snapshot_pod!` writes a codec from a field list, so the failure mode is a field
  OMITTED — which `encode ∘ decode ∘ encode` cannot see, because it round-trips its
  own bytes perfectly. `every_registered_component_survives_a_world_round_trip` wrecks
  a world and demands its hash back; the exit oracle runs the sim forward and notices
  what a hash cannot.

  ### The ledger had a blind spot the size of a `Resource`

  `unclaimed_components` walks entities. **A `Resource` sits on no entity**, so for the
  whole of this chain the ledger never saw one, and `restore` never touched one — while
  this section's own checklist names them explicitly: *"`WorldTime` + every sim clock"*,
  *"every seeded RNG resource"*, *"active room + spawn state"*, *"falling-sand grids
  (ONE resource blob)"*.

  `SnapshotRegistry::unclaimed_resources` now measures it, filtered to types whose
  name CONTAINS `ambition_` (Bevy's asset servers and render device state are not sim
  state and never will be). **It reads 181 in the audited tree**, and it is pinned.

  It read 135 until the filter said `starts_with`. **Forty-five of them are `Messages<T>`
  buffers**, named `bevy_ecs::message::Messages<ambition_..::HitEvent>` — hidden twice
  over, once by sitting on no entity and once by wearing Bevy's module path. And they are
  not empty: at a tick boundary `Messages<ActorActionMessage>` holds the actions the
  brains just emitted. *A message written before a snapshot and read after a restore is
  an event that happens twice.*

  `SnapshotRegistry::register_message_channel` names one, and **`restore` clears every
  registered channel**. The content of a buffer at snapshot time cannot affect the future
  — every system that runs in that tick has already read it — but a message from the
  future we are *abandoning* must not be read in the past we are returning to. Four
  channels are registered (`actor_action`, `hit_event`, `on_hit_effect`, `move_event`);
  `pending_messages` reports the rest. The channels are NOT hashed: two sims of N0.4's
  canary hold the same pending messages, a rewound sim holds none, and hashing that
  difference would fail the exit oracle for the one thing it is trying to fix. Most of that is presentation or derived —
  `ActorRenderIndex`, `CameraShakeState`, `DeveloperTools` — and comes off with
  `declare_derived`. Some of it is not:

  - `ambition_encounter::state::EncounterState` — the live encounter phase, the wave
    run, the spawn counter. **This was why `mockingbird_arena` diverged** before the missing state was registered; the current replay table below records it clean.
  - `ambition_projectiles::enemy::state::EnemyProjectileState`.
  - `ambition_actors::encounter::switches::SwitchActivationQueue`.

  *What the canary cannot see, it cannot defend* — and for one whole chain of commits,
  it could not see a resource. The number is the point: it was zero because nothing
  looked, not because nothing was there.

  ### `SnapshotCursor` — a component that is half authored, half mutable

  `ActorMotionPath` owns a patrol path (authored, immutable, large) and a
  `(segment, dir)` cursor (mutable, tiny, and the whole reason a rollback touches it).
  Serializing the waypoints sixty times a second to rewind two integers is absurd, and
  `SnapshotState::decode` cannot rebuild the component without them.

  So `register_cursor::<C>()` **applies the cursor onto the entity that already has
  it**. That is sound *precisely because* `restore` patches survivors: an entity
  present in both worlds still carries its authored half. A **respawned** entity does
  not, and the cursor correctly refuses to invent one — one more reason a rollback
  window must not span a spawn, and why `RestoreReport::respawned` is a number you are
  meant to look at.

  This is the general shape of the authored/mutable split, and it is why the coverage
  ledger is an upper bound rather than a debt: many of the 59 want a *cursor*, not a
  codec.

  ### Replay status after the resolved blockers

  Pointing `hash_by_entry` — the per-entry hash the canary already had — at each dirty
  room named three different diseases wearing one symptom:

  | room | first divergence | what restore did | remaining cause |
  |---|---|---|---|
  | `gap_run` | — | all patched | ✅ **CLEAN** |
  | `gnu_ton_arena` | — | all patched | ✅ **CLEAN** |
  | `mockingbird_arena` | — | all patched | ✅ **CLEAN** |
  | `portal_lab` | tick 0 | 1 **respawned** | a naked respawn |

  **Three rooms rewind and replay bit for bit, two of them boss fights.** Take a
  snapshot, run 60 ticks, restore, replay the same inputs, identical hash stream.

  The last two blockers were both *mirrors of state that lives somewhere else*:

  - `gnu_ton_arena` broke on `perception_memory` alone. The cause was `GameplayElapsed`,
    an accumulating sim clock a brain stamps `RememberedActor.last_seen` with. This
    section's checklist says *"`WorldTime` + every sim clock"*; I had registered one of
    the two, and could not have found the other, because a `Resource` sits on no entity
    and the coverage ledger walked entities.
  - `mockingbird_arena` broke at tick 21: the replay telegraphed `wing_sweep` while the
    original stood still, with every clock, seed, and cooldown identical. The boss was
    already awake. `BossEncounter.encounter_phase` is a MIRROR that
    `sync_boss_encounter_phase` copies out of `BossEncounter.encounter:
    Option<BossPhaseState>` every tick. **Rewinding only the mirror is rewinding a
    thermometer.** The cursor now carries the `BossPhaseState` — its `phase`,
    `phase_elapsed`, `transition_lock` — and leaves its authored `triggers` alone: *a
    snapshot carries what the fight has become, never the rules it became it by.*

  **Two hypotheses died on the way, and both fixes were kept because both were right on
  their own terms**: a stale `Messages<ActorActionMessage>` (buffers really are non-empty
  at a tick boundary; `restore` clears the registered channels now) and a stale
  `CombatSlotsRes` slot assignment (a `ResourceCursor`; its `assigned_to` is a stable id,
  not an `Entity`). Neither moved tick 21. `ProperTimeScale` was registered on the way,
  too. A falsified hypothesis is the cheapest thing a later reader can be handed.

  ### `portal_lab`: CONFIRMED diagnosis — a rollback window that spans a room transition (NOT a leak)

  Traced (S2.2, 2026-07-10), and it overturns the earlier "cross-room leak"
  hypothesis. **There is no `central_hub_main` → `portal_lab` entity leak.**

  - `central_hub_main` is the LDtk *level* id; `central_hub_complex` is that
    level's runtime `activeArea` / room id.
  - `NpcSpawn-0017` is authored by `central_hub_complex` (via its
    `boss_spawns`/`placements`/`enemy_spawns` — a boss uses `boss_spawns`, an NPC a
    placement, but both carry a `placement:<iid>` id), and it is alive **only while
    that room is active**: the traversal policy bounces the player through a shared
    loading zone between `portal_lab` and `central_hub_complex`, and the NPC is
    despawned the instant the player transitions away.
  - A per-tick check (`every_placement_entity_is_owned_by_the_active_room_every_tick`)
    proves the roster is healthy: no `placement:<iid>` is ever alive while a room that
    does *not* author it is active. The leak hypothesis is permanently dead.

  The defect is **temporal**: the snapshot is taken while `central_hub_complex` is
  active (so it captures `NpcSpawn-0017`, correctly), execution transitions to
  `portal_lab`, and restore leaves `portal_lab` active. `respawn_from_the_room` then
  reconciles against the wrong current `RoomSpec`. **The active-room context is
  omitted from snapshot state.**

  Fixing this by restoring only `RoomSet.active` + `RoomGeometry` would be *worse*
  than the current failure: a room transition also tears down and rebuilds
  room-scoped entities, moving platforms, and clocks, so a partial cursor restore
  produces a more internally-inconsistent world than a clean refusal. Two valid
  outcomes:

  1. **Support rollback across transitions** by atomically restoring the complete
     room context *before* entity reconciliation.
  2. **Define room transitions as rollback boundaries**: record the snapshot's active
     room, and make restore explicitly REJECT a cross-room snapshot rather than
     partially restore it.

  **S2.3 lands (2) as the honest boundary** (`SimSnapshot::active_room` +
  `RestoreError::CrossRoomBoundary`, detected before reconciliation). Outcome (1) —
  the full atomic room transaction — is the bounded-window work below, and is what
  moves `portal_lab` to CLEAN.

  A separate, un-conflated case surfaced by the ownership invariant: a
  dynamically-spawned child (a boss hand) receives a `FeatureId`, so `ensure_sim_id`
  would promote it into the `placement:` namespace even though **no room authors
  it**. **RESOLVED (2026-07-11):** `spawn_giant_hand_limbs` now derives the hand's
  `FeatureId` from the giant's AUTHORED id (not `giant.index()`, an allocator slot)
  and mints its `SimId::spawned(giant_placement_sim_id, ordinal)` at spawn, so the
  hand is a deterministic spawned child (`placement:<giant_iid>/<ordinal>`) and
  `ensure_sim_id` (`Without<SimId>`) skips it. Pinned by
  `giant_hand_identity_tests` in `spawn_actors.rs`.

  The landed N3.1 keystone currently contains 60 registry entries across five kinds (component,
  cursor, resolved, resource, resource-cursor), four message channels, three declared
  derived, and an exit oracle that three rooms pass and the fourth is *asserted* to fail.

  A note on where the codecs live. `ambition_runtime` implements `SnapshotState` for
  other crates' types today because it sits above them all. That is the bootstrap, not
  the destination: this section asks that *"each sim crate registers its components'
  serialization"*, which needs the trait to move down to `platformer_primitives`. It
  is a mechanical move, and it is worth doing before the third crate wants a codec.

  Still useful long before netcode: Braid-style rewind, RL tree search, and the
  fighter brain's FB6 rollouts all want it — and all three want `lossless()` first.
- **N3.2 Exact-restore substrate + resim discipline — OPEN, ordered.**

  1. **Identity invariant + snapshot validation:** DONE (S2.1; third-pass findings 1 + 2).
     Reject duplicate live/snapshot `SimId`s and duplicate registry names before any lookup
     map. The live-identity check is a PANIC (a running world with a duplicate id is a
     spawn-site bug); the SNAPSHOT check is a mutation-free `validate_snapshot` phase that
     runs before the first despawn and RETURNS `RestoreError::MalformedSnapshot` (corrupt
     wire input, not a program bug). It establishes canonical roster order, registry/kind
     agreement, unique rows, roster membership, AND exact top-level entry order (fourth-pass
     re-audit: restore iterates `snapshot.entries` directly, so a permuted deserialized
     snapshot could resolve a component before a registered dependency is restored — the entry
     order is now required to match registry order, which also subsumes unknown/missing/
     duplicate entries). `duplicate_ids` sorts before scanning, so a non-adjacent collision in
     an unsorted roster no longer evades it. Duplicate-`SimId`, kind-mismatch, and reordered-
     snapshot poison tests landed with it.
  2. **Active-room ownership + room boundary + authoritative roster:** DONE (S2.2/S2.3;
     third-pass findings 1 + 5). Traced the mechanism — no leak; the rollback window spans a
     room transition and the active room is omitted from snapshot state. Enforced per-tick
     authored ownership (proves the roster is healthy); captured `SimSnapshot::active_room`
     AND folded it into `hash_world`/`hash_by_entry`/`size_bytes`; and made restore REJECT a
     cross-room snapshot (`RestoreError::CrossRoomBoundary`) before reconciliation —
     comparing the FULL `Option<String>` on both sides, so a `Some`/`None` presence mismatch
     is refused, not just two different ids (third-pass finding 5). The identity `roster` is
     now AUTHORITATIVE and HASHED (third-pass finding 1): restore reconciles against the full
     roster, not the component-derived id set, so a `SimId` entity with zero registered
     components is preserved/reconstructed rather than silently despawned/dropped; and the
     roster is a named hash pseudo-entry, so two worlds differing only by such an entity no
     longer hash equal. REMAINING: the full atomic room-context restore (entities +
     platforms + clocks), which moves `portal_lab` to CLEAN — see the bounded-window item.
     (The boss-hand case that once routed to the dynamic-spawn item — dynamic children
     wrongly in the `placement:` namespace — is RESOLVED: they mint `SimId::spawned(..)`
     from the giant's authored id, 2026-07-11.)
  3. **Reconciliation ordering:** DONE (S2.4). Stale components and unidentified
     survivors are computed AFTER reconciliation, over the final restored roster.
  4. **Codec failure semantics:** DONE for the ordinary path TRANSACTIONALLY, and every
     codec now reports an OUTCOME rather than a bare success (S2.5; third-pass findings 3 + 4).
     `restore` returns `RestoreError::DecodeFailed` in every build. Standalone codecs (plain
     component, plain resource) carry a decode `probe` and are validated in a mutation-free
     preflight BEFORE the first despawn, so a corrupt ordinary blob refuses with the world
     untouched (a would-be-despawned future entity is asserted to survive). Cursor and
     resolved component codecs now return `ApplyOutcome::{Applied, DecodeFailed, Unapplied}`:
     a cursor with no live target and a resolve whose content vanished report `Unapplied`
     (counted, `lossless()` denies it) instead of the old bare `true`. Resource cursors carry
     a PRESENCE TAG (`Some`/`None`), so absence and an empty cursor no longer encode
     identically, and a shape mismatch (`CombatSlotsRes`) refuses loudly rather than silently
     zipping. **Every codec path now also asserts it consumed the WHOLE blob** (fourth-pass
     re-audit): the resolved insert checks `r.finish()` after `resolve` returns `Some`, the
     resource-cursor absence path checks `finish` before removing, and `Reader::bool` decodes
     only canonical `0`/`1` — so a valid prefix plus trailing garbage, or a non-canonical tag
     byte, is `DecodeFailed` rather than a silent success. RESIDUAL: cursor/resolved codecs
     decode into a live target, cannot be probed standalone, so a genuine decode failure can
     still surface mid-reconciliation; and a resolved codec still cannot DISTINGUISH a decode
     failure (e.g. a truncated blob) from authored absence (`resolve` returns `None` for both)
     — both deny `lossless()`, but making `SnapshotResolve::resolve` return a `Result` to
     separate the two is the remaining work (now ONLY that distinction, not trailing bytes).
  5. **Positive, self-measured losslessness:** DONE (S2.6/S2.7/S2.8; third-pass findings 3 + 4
     + 6). `lossless()` is argless: `restore` MEASURES the resource term itself
     (`unregistered_sim_resources`), so the caller can no longer claim `lossless(0)` against a
     world with debt. It requires `resource_census_reliable` (resource names need
     `bevy_ecs/debug`; without them the count is a spurious 0), so it cannot succeed blind. It
     now ALSO denies any restore with an unapplied component row (`unapplied_rows`) or an
     unresolved resource cursor (`resource_cursors_unresolved`) — the false-success the old
     bare-`true` insert produced. Registered `Messages<M>` channels are CLAIMED (restore
     clears them), not counted as false debt. The sim-resource universe is a NAMED exclusion
     policy (`SIM_RESOURCE_EXCLUSIONS`), per-TYPE for mixed-purpose crates (`ambition_ldtk_map`)
     so a new resource there is a review event, namespace-form only for wholly-presentation
     subtrees. The coverage ledger pins the debt by TYPE NAME against reviewed inventory files
     (`tests/known_{resource,component}_debt.txt`), not just by count, so a substitution that
     holds the count constant is a review event (third-pass finding 6).
  6. **Dynamic reconstruction refusal (NOT a general bounded window):** ONE refusal enforced
     (S2.9/S2.10; re-audit finding 4). `restore` refuses `RestoreError::UnsupportedDynamic`
     `Reconstruction` when a `SimId::spawned(..)` entity (id contains `/`) is in the
     snapshot, gone from the world, and unauthored — it EXISTED at the snapshot tick and
     cannot be rebuilt from blobs alone. This is a reconstruction refusal, not a "birth"
     (an entity spawned after the snapshot is future-only and simply despawned), and it is
     preflighted before any mutation. It establishes ONE reconstruction refusal, not a
     general bounded-window guarantee. REMAINING for exact-across-a-birth: register
     reconstruction recipes per dynamic spawner; then tag confirmed read-model ticks and
     deduplicate resim side effects. No suite room yet spawns AND kills a dynamic child
     inside a window, so the recipe path is unexercised. **The boss-hand identity
     sub-item is DONE (2026-07-11):** `spawn_giant_hand_limbs` derives the hands'
     `FeatureId` from the giant's AUTHORED id and mints `SimId::spawned(giant, ordinal)`
     at spawn, so they are deterministic spawned children instead of entity-index-derived
     `placement:` ids (`giant_hand_identity_tests`).

  Poison-test atomicity is binding: do not land a known-red future test or pin the
  current bug. Full rationale and the three-series order live in
  [`../../archive/reviews/static-audit-response-2026-07-10.md`](../../archive/reviews/static-audit-response-2026-07-10.md).
- **N3.3 `bevy_ggrs` integration spike** against the SSB demo scene.

## N3.1 design sketch (pre-solved; sim components conform to this NOW)

```rust
// ambition_runtime (owner):
pub struct SnapshotRegistry { /* built at plugin init */ }
impl SnapshotRegistry {
    /// Each SIM crate's plugin calls this for every component/resource that
    /// constitutes sim state. T: Component + Serialize + DeserializeOwned.
    pub fn register_component<T: SnapshotState>(&mut self);
    pub fn register_resource<T: SnapshotState>(&mut self);
}
pub struct SimSnapshot { tick: u64, blobs: Vec<(StateTypeId, Box<[u8]>)> }
pub fn take(world: &World, reg: &SnapshotRegistry) -> SimSnapshot;
pub fn restore(world: &mut World, snap: &SimSnapshot, reg: &SnapshotRegistry);
```

Decisions fixed here so opus never re-derives them: (1) registration is
OPT-IN per plugin — un-registered state is by definition presentation or
derived, and the desync canary (N0.4) hashes exactly the registered set,
which keeps the two features honest against each other; (2) `Entity`
references inside sim components are FORBIDDEN in favor of the stable-id
vocabulary (spawn ids / slot ids) — where one exists today it gets a
migration slice listed in tracks.md when N3.1 implementation starts;
until then the RULE binds new code (write the doc-comment snapshot story
at every exception); (3) restore = despawn-registered + respawn from
blobs (no in-place patching — simpler, and room-reset already proves the
world can rebuild); (4) the fighter brain's rollouts call
`take`/`restore` on a SCRATCH copy of the app's sim world (the headless
`SandboxSim` embeds fine — it's the same App shape the RL path builds).

**Identity & scope (pinned 2026-07-06, answering the contract review):**

- **One identity vocabulary, shared with SimView.** ✅ **LANDED 2026-07-10.** Every
  snapshot-registered entity carries a `SimId` — the EXISTING stable ids,
  not a new system: actors use `ActorConfig.id` (== LDtk iid; placement
  identity), player bodies use their slot, dynamically-spawned sim
  entities (projectiles, dropped items, spawned adds) get a
  deterministic sequence id minted at spawn (`(spawner SimId, per-spawner
  counter)` — deterministic because the sim is; wall-clock/Entity-index
  ids are forbidden). Snapshot blobs key by SimId; restore despawns every
  registered entity and respawns from blobs, so an entity spawned AFTER
  the snapshot simply ceases to exist on restore (correct), and one
  despawned since is recreated (correct). `Entity` values never appear in
  a blob.
- **Included** (the registration checklist per sim crate): body kinematics
  + transforms, health/combat/damage meters, move playbacks + cooldowns,
  brain memory (habit models, timers), `WorldTime` + every sim clock,
  portal placements + transit/cooldown state, flags/save-derived liveness,
  active room + spawn state, falling-sand grids (ONE resource blob), and
  every seeded RNG resource (sim randomness MUST be a registered seeded
  resource — an unregistered RNG is a determinism bug N0.4 will catch).
- **Excluded, structurally:** `SimView` and all view indexes (rebuilt every
  tick by construction), the composed-world overlay + carve output
  (derived — restore triggers the same recomposition that runs per frame),
  asset handles, presentation entities (never registered), and caches.
  Rule: DERIVED state is never snapshotted; if restoring something
  requires a rebuild pass, the rebuild must be the SAME system that
  maintains it per-frame (no special restore-only code paths).
- **Presentation reconciliation is free by E4:** render rebuilds from
  `SimView` each frame, so a restore that removes/revives sim entities
  needs no render-side fixup protocol — the next view rebuild reflects it.

## Who does what, and when

| Rung | When | Grade |
|---|---|---|
| N0.1–N0.4 | before/with SSB demo | [opus] (N0.1's two-clocks review is RULED above — mechanical threading) |
| N1.1–N1.3 | the SSB demo's engine prerequisites | [opus] |
| N2 | post-1.0, evaluation first | [opus] |
| N3.1 design | NOW (fable window) — implementation later | **[fable design]** → [opus execute] |
| N3.2–N3.3 | post-1.0 | [opus] |

Exit for the doc: SSB runs 4 local slots at fixed tick with the desync
canary green over recorded matches, and `SimSnapshot`'s design is written
(even if unimplemented) so no new sim state is authored in a
snapshot-hostile shape (Rule: sim components are plain data; anything
holding `Entity` references or interior mutability documents its snapshot
story at the definition site).
