# Same-room replay is a second, incomplete room constructor

**State:** OPEN — the defect class is measured and one instance is repaired; the
convergence onto canonical reconstruction is not started.

**Opened** 2026-08-14, from Jon's retest of the Smirking Behemoth replay and
GPT's reading of it. The falsifier that made it legible is his: *"leaving the
room and re-entering restores its movement."*

## The claim

There are two constructors for a room's authoritative population.

```text
room transition / new game        same-room replay
        │                                 │
RoomConstructionPlan              ResetRoomFeaturesEvent
  prepare                                 │
  retire outgoing contents        reset_ecs_room_features
  commit prepared construction      mutate every surviving entity
        │                            back toward a presumed spawn
        ↓                                 ↓
   a constructed room               a room that LOOKS constructed
```

`reset_ecs_room_features`
(`crates/ambition_platformer2d_actor_monolith/src/features/ecs/reset.rs`) is a
hand-kept reconstruction ledger: pickups, chests, breakables, actor spawn state,
dispositions, aggression, pinned poses, boss health, boss encounter phase, boss
brain cursor, control frames, anim frames, hazard positions, switches, encounter
entities, falling hazards, lure overrides. Every row is a fact somebody noticed
was missing. Nothing enumerates what a constructed room actually contains, so
nothing can notice the next missing row — the ledger only ever grows, one bug
report at a time.

**The rule this file exists to enforce: a fix here that adds one more component
to that ledger is not a fix.**

## The measured instance (repaired 2026-08-14)

A replayed Smirking Behemoth came back alive, woke through Intro into Phase 1,
and its brain commanded `velocity_target = (-141.75, 0)` every tick — while its
body stood at exactly its spawn, forever.

The two constructors disagreed about ONE boolean:

| | writes `flight.fly_enabled` as |
|---|---|
| construction (`actor_clusters.rs`) | `ActorTuning::is_aerial` → `true` |
| reset (`core::body_clusters::reset_body_clusters`) | `abilities.fly && !abilities.fly_toggle` |

The boss's kit declared `fly_toggle: true`, so the reset derived `false`. A boss
steers ONLY by commanding an exact velocity, and only the flight limb reads that
command — so a grounded boss is an inert boss. Leaving the room ran the other
constructor and gave it its legs back.

The repair was to stop the body from lying about itself: a boss is a PERMANENT
flier (nothing has ever toggled one), so `fly_toggle: false` makes both
constructors compute the same thing. Pinned by
`a_replayed_boss_behaves_like_a_freshly_constructed_one`
(`game/ambition_app/tests/boss_lifecycle.rs`), which compares *behaviour* —
does it wake, does it leave its spawn — against a freshly constructed boss of
the same archetype, and never names a component.

⚠ note what the repair did NOT do: the divergence is still there for the next
body that spawns with a toggled ability already on. `reset_body_clusters`
re-derives an identity fact that construction sets directly — the same trap its
own `base_size` comment already documents at length, three lines above the one
that bit us.

## What convergence needs

```text
RoomReplayRequested
    ↓
content clears durable per-attempt facts        (exists: ContentRoomReplayResetSet)
    ↓
prepare canonical construction for current room  (exists: RoomConstructionPlan)
    ↓
retire/reconstruct the room's authored + attempt-scoped population   ← MISSING
    ↓
reset controlled-body attempt state              (exists: the host consumer)
```

Only the third step is missing, and it is missing because nobody has written
down what a replay is allowed to destroy.

## The open decision (this is the blocker — do not guess it)

A replay must not simply despawn every runtime entity. Four populations, four
different answers, and only the first is obvious:

1. **Authored room population** — reconstruct. (Bosses, enemies, props, hazards.)
2. **Attempt-scoped residue** — disappear. Already named by `SpawnedThisAttempt`
   and the in-flight-volley despawn, so this population has a marker to build on.
3. **Session/world-persistent instances** — survive per their lifetime contract.
   A weapon dropped in this room and expected to still be here is the case
   `SpawnedThisAttempt`'s doc already argues about; a replay and a room exit are
   different questions and one scope cannot answer both.
4. **Participant-controlled / body-owned state** — needs an explicit replay
   policy. Held items, mounts, summons still under command, a possessed body.

That is exactly the taxonomy
[`instance-lifetime-provenance-and-persistence.md`](instance-lifetime-provenance-and-persistence.md)
exists to give types to (`spawn provenance` + `lifetime policy`). **This work
should not invent a parallel one.** Until an instance carries its own lifetime
policy, "reconstruct the room" cannot be expressed without a hand-kept list —
which is the thing being deleted.

⇒ **Sequencing: the lifetime/provenance model lands first; this converges onto
it.** Anything done before that is a third constructor.

## What may be done before that

- Repair measured disagreements between the two constructors one at a time, each
  pinned by a fresh-vs-replayed BEHAVIOUR comparison rather than a component
  assertion. (One done.)
- Add more fresh-vs-replayed comparisons for the populations most likely to
  diverge — an ordinary enemy, a morphing enemy, a hazard on a path — so the
  convergence has acceptance tests waiting for it rather than a rewrite with no
  oracle.
- ⛔ do NOT extend the ledger to make a symptom disappear. If a new divergence
  is found and the honest fix is a ledger row, record it here instead and leave
  the test red-listed.
