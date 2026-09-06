# Why did this actor change on this tick?

Operational recipe for the causal inspector. You should not need to open an
engine source file to follow it.

## TL;DR

```rust
use ambition_causal::{CausalRecording, RecordingPolicy, SubjectKey, domains};

// 1. turn it on for the domains you are investigating
app.world_mut()
    .resource_mut::<CausalRecording>()
    .set_policy(RecordingPolicy::only([domains::MOVEMENT, domains::ROLLBACK]));

// 2. drive the sim
for _ in 0..120 { app.update(); }

// 3. ask
let log = app.world().resource::<CausalRecording>();
let why = log.explain(tick, &SubjectKey::Seat(1));
println!("{}", why.render());
```

`explain` returns everything published about that subject on that tick, plus
everything published about the WORLD on that tick (a rebase, a rules change) —
because those explain a body too.

⛔ **It returns ONE execution of that tick, not a merge of several.** A tick
number is not a moment: frames restart at zero on every session, so generation 1
tick 20 and generation 2 tick 20 are different things, and under a rollback host
the same tick runs originally and again as resimulation. `explain` gives you the
LATEST; `explanations(tick, subject)` gives every one, oldest first, and is what
to reach for when the question is about the rewind itself.

```rust
for one in log.explanations(20, &SubjectKey::Seat(1)) {
    println!("{} {:?}", one.key.generation, one.execution());
}
```

⚠ two resimulations *within* one generation still group together, because
nothing publishes an attempt counter yet.

## Recording is OFF by default

Installing `CausalPlugin` makes recording *possible*. It never turns it on. An
instrument that is on by default is one somebody turns off, and then it is not
there when it is needed.

| policy | keeps |
|---|---|
| `RecordingPolicy::Off` | nothing. The shipped default. |
| `RecordingPolicy::only([..])` | just those domains. The usual answer — the expensive domains are rarely the ones under investigation. |
| `RecordingPolicy::All` | everything. |

The log is a bounded ring (`DEFAULT_CAPACITY` 4096). When it wraps,
`Explanation::truncated` is true — so a gap the *buffer* caused is
distinguishable from a gap the *simulation* caused.

## What you can ask today

| question | domain | fact kind |
|---|---|---|
| Why did this body move this tick? | `MOVEMENT` | `movement_intent` |
| Was this tick original or rollback resimulation? | `ROLLBACK` | `tick_execution` |
| Why did this fighter choose this action? | `BRAIN` | `fighter_decision` ⚠ headless only, see below |

Not yet published: damage, lifecycle, move playback, contacts. `Explanation`
tolerates their absence — a movement-only composition gets a shorter answer, not
an error.

## Read fields, not prose

```rust
let intent = why.first("movement_intent").expect("a seated body");
assert_eq!(intent.get("locomotion_x"), Some(&FactValue::Float(-1.0)));
```

`FactDetail::summary` is for humans. **Every value a tool would want is a
field**, so nothing has to be parsed back out and improving a message never
breaks a test.

## Subjects

```rust
SubjectKey::Seat(1)          // a seat — stable across death and respawn
SubjectKey::Sim("boss_a")    // a stable sim id
SubjectKey::Unstable(idx)    // ⚠ a recorded API leak, not a design
```

Never key on an `Entity`: indices are recycled, so a later body would inherit an
earlier one's explanation.

## Chains

A fact may name the fact it followed from, which turns a list into a chain:

```rust
let chain = why.chain_to(why.first("playback_began").unwrap());
// [action_pressed, move_scored, playback_began] — oldest first
```

## Publishing from your own capability

**From an ECS system — the sound way:**

```rust
pub fn record_my_thing(
    log: Option<ResMut<CausalRecording>>,
    things: Query<(&Thing, &Whatever)>,   // immutable: an observer by signature
) {
    let Some(mut log) = log else { return };
    if !log.is_recording() { return }
    for (thing, whatever) in &things {
        log.record(
            CausalFact::new(domains::MOVEMENT, 0, FactDetail::new("my_kind", "…"))
                .about(SubjectKey::Seat(thing.seat))
                .field("value", whatever.0),
        );
    }
}
```

Leave the tick `0` and leave the execution alone — the host stamps both
(`stamp_causal_frame`, at the head of the sim schedule). A domain five hops
below the ECS does not know the world's clock, and it certainly does not know
whether the host is replaying this frame; a fact that guessed `Original` would
make a resimulated tick indistinguishable from its original.

**Order your publisher after the stamp**, in `RecordingSet::Publish`:

```rust
app.add_systems(sim, my_publisher.in_set(ambition_platformer2d_runtime::causal::RecordingSet::Publish));
```

A publisher outside that set can run first and carry the *previous* frame's
identity. That is not hypothetical — it is what the parallel-schedule proof
found the first time the plugin was written.

**⛔ From pure code deep in a call tree — thread-local, and it has a contract:**

```rust
let (log, ()) = ambition_causal::with_sink(log, || run_the_pure_thing());
```

`with_sink` collects facts published on the **same thread**. That is sound for a
pure call tree driven from one thread (a headless probe, a test) and **not sound
under Bevy's multithreaded scheduler** — a system on a worker thread publishes
into a sink the collector never sees.

It does not vanish silently: `ambition_causal::facts_lost_offthread()` counts
exactly that case, and `assert_no_offthread_loss()` turns it into a failure. If
it fires, that domain needs `ResMut<CausalRecording>` instead.

## Rules that are not negotiable

* **⛔ the simulation must never READ a fact.** The ring is lossy, it is not
  rewound by a rollback host, and anything that branched on one would desync the
  moment history replayed. The thread-local sink's `record` returns `()` on
  purpose — there is no read path.
* **Observers take everything immutably.** Make it a property of the signature,
  not a promise in a comment.
* **Quote the compiler's prepared identity** (`ambition:character/goblin`) in
  `CausalFact::content`, never a name reconstructed from a runtime internal.

## Turning it on in a build

`causal` is an optional, default-OFF feature on `ambition_platformer2d_runtime` and
`ambition_platformer2d_actor_monolith` — a game that never opens an inspector must not link one.

```bash
cargo test -p ambition_platformer2d_actor_monolith --features causal
cargo test -p ambition_platformer2d_runtime --features causal
```

## Deterministic dumps

`CausalLog::dump()` is ordered by `(tick, fact id)` — insertion order within a
tick, stable across runs — so it can be a CI artifact and a diff between two
runs means something.
