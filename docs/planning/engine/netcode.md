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

## N0 — determinism becomes a MANAGED contract (the Q4 answer we act on)

Today determinism is a test canary. The ladder needs it as a **scoped
guarantee**: *same build, same platform, same inputs ⇒ same sim states.*
(Cross-platform float determinism is explicitly NOT promised — same-binary
lockstep/rollback and replay/RL reproducibility don't need it.)

Obligations (each a slice, all [opus]):

- **N0.1 Fixed-tick sim mode.** The sim already steps headlessly at fixed
  dt (`SandboxSim.step`); the windowed app steps on frame time. Add the
  runtime option: sim at fixed Hz + presentation interpolation reading the
  read-model. Feel-sensitive input timing stays on the per-player FEEL
  clock (the existing time-domain split — this is what it was FOR).
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

## Who does what, and when

| Rung | When | Grade |
|---|---|---|
| N0.1–N0.4 | before/with SSB demo | [opus] (N0.1 wants a fable/opus-specced review of the two clocks) |
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
