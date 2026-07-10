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
- **N0.3 Determinism lint set — ✅ LANDED (opus, 2026-07-09).** The four rules
  (no ambient randomness; no wall-clock reads; no std-hash-order semantics; no
  `Entity` as an ordering key) are greps over every non-test source in the sim
  crates, in `crates/ambition_runtime/tests/determinism_lints.rs`, with an
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
- ~~**N0.4 Desync canary rig.**~~ ✅ **LANDED 2026-07-10.**
  `game/ambition_app/tests/desync_canary.rs`: two `SandboxSim`s, one seeded input
  stream, the registered sim state hashed every tick, first-divergence report that
  names the offending REGISTRY ENTRY (a desync you cannot name is a desync you
  cannot fix). **3 rooms × 240 ticks, in sync.** Poison-tested both ways — a
  different input stream must diverge, and moving one body must change the hash —
  because a canary that cannot cry proves nothing.

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

- **N3.1 Snapshot/restore of sim state** — 🟡 **registry + identity + `take`/`restore`
  landed 2026-07-10. What remains is the per-crate registration checklist, and it
  is now a NUMBER.**

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

  **`the_sim_id_migration_ledger` is a GATE, and it reads ZERO** across
  `gap_run` / `portal_lab` / `mockingbird_arena` / `gnu_ton_arena`: every
  simulated body carries a `SimId`. A rise means a spawn site shipped without
  minting one, and restore would silently lose whatever it spawned.

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

  | room | component types a restore leaves stale |
  |---|---|
  | `gap_run` | 53 |
  | `portal_lab` | **88** |
  | `mockingbird_arena` | 77 |
  | `gnu_ton_arena` | 53 |

  Pinned at 88; it may fall, it may not rise. The ledger keys on `TypeId` (always
  exact); component NAMES need `bevy_ecs/debug`, which `ambition_app`'s test graph
  happens to enable and `ambition_runtime`'s does not — so the counts are trustworthy
  in both and the names are readable where it matters. Lower it by registering a
  component, or by `declare_derived::<C>()`, which is a *promise* that the same
  per-frame system that maintains `C` rebuilds it — N3.1's own no-restore-only-code
  rule, made into an API call.

  **`a_restored_sim_replays_the_future_it_was_rewound_from` is N3.1's exit oracle**:
  take, run K ticks hashing each, restore, replay the same K inputs, demand identical
  hash streams. It is `#[ignore]`d and it fails at tick 0 — not from a bug in
  `take`/`restore`, whose unit oracles are green, but because the player's ground
  timers, jump state, and action buffer are unregistered and so are left stale.
  **Un-ignore it when the ledger reads zero.**
  Its live sibling,
  `a_restore_of_a_real_room_is_exact_where_it_is_registered_and_honest_where_it_is_not`,
  pins today's truth: on `gap_run` the restore reproduces the registered hash bit for
  bit, leaves zero unidentified survivors, and admits every type it destroyed. That
  test is what stops the `#[ignore]` from being a shrug, and it goes red — on
  purpose — the day the ledger hits zero.

  Registered today: `sim_tick`, `world_time`, `body_kinematics`, `body_health`,
  `sim_id_counters`. The checklist below (move playbacks + cooldowns, brain memory,
  portal transit, falling-sand grids, every seeded RNG) is what the 88 is made of.

  Still useful long before netcode: Braid-style rewind, RL tree search, and the
  fighter brain's FB6 rollouts all want it — and all three want `lossless()` first.
- **N3.2 Resim discipline** — bounded rollback window; presentation reads
  confirmed ticks only (read-model tick-tagging); side-effect suppression
  during resim (sfx/vfx event facts carry the tick; presentation dedups).
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
