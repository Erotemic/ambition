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

- **N0.1 Fixed-tick sim mode — ✅ the two-clocks review is RULED (fable,
  2026-07-06 night). Opus executes; do not re-derive:**
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
    knob that threads the label to every member plugin. Default stays
    frame-stepped (Ambition today, byte-parity); SSB/demos opt in.
    Presentation interpolation reads previous+current tick pose from the
    read-model (BodyPoseView carries pos+vel — velocity extrapolation is
    the cheap v1; a two-tick pose buffer is the v2 if extrapolation
    visibly jitters).
  - **Ordering guard:** the rl_sim schedule-shape tests must pass with
    the label threaded BOTH ways (parameterize one suite run over
    Update/FixedUpdate) — that's the exit check.
- **N0.2 Input-stream capture as a first-class type.** `SlotControls`
  per-tick, serializable, versioned — the SAME artifact serves replay
  fixtures, RL trajectories, desync forensics, and the wire format later.
- **N0.3 Determinism lint set.** Codify the known rules (stable iteration
  order — sort by stable id, never `Entity`; no `HashMap` iteration in sim;
  no wall-clock reads in sim) as clippy-style greps in CI + a doc page.
- **N0.4 Desync canary rig.** Two sims, same input stream, state-hash per
  tick (hash = the snapshot serialization of N3.1 — build them together),
  first-divergence report. This is the tool that keeps N0 true forever.

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

- **N3.1 Snapshot/restore of sim state** — the big one, and useful long
  before netcode: Braid-style rewind, RL tree search, and the fighter
  brain's forward rollouts all want it. Scope: the SIM world only (the
  read-model boundary means presentation never snapshots). Shape: a
  `SimSnapshot` component-set registry (the crates each register their
  sim-state components; the runtime owns serialize/restore) — design work,
  [fable-hard], and the reason N3 design starts now even though online
  ships later.
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

- **One identity vocabulary, shared with SimView.** Every
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
