# Projectile ownership and parry rules

The runtime keeps three different facts:

- `SpawnOrigin::Dynamic` records the original spawner and its sequence. A
  reflection does not change creation provenance or the shot's simulation ID.
- `ProjectileOwner` is the current credited live entity. It is mapped on restore.
- `ProjectileCredit` records the current semantic body and reference room. The
  projectile's session qualifies that reference. It survives loss of the live
  link and is canonical rollback state.

Reflection changes the live owner, credit, and combat allegiance together. It
removes the old shooter's move occurrence and weapon-shot marker. Presentation
source stays with the shot. A reflected shot does not become the original
shooter's work again when its deflector retires.

A valid live link carries the reference through travel. A missing link can be
restored from one matching identity in the same session and reference room.
No match, or an ambiguous match, leaves it unresolved. Creation provenance does
not supply current credit. A missing live owner does not erase semantic credit.

The native `ambition_combat::projectile_spawn::ProjectileWriter` captures each
request before deferred construction. It reserves a descendant ID from the
emitter's counter and captures session, room, round, credit, allegiance, and
presentation. Materialization does not read these facts from a live source.
Source travel, control changes, or retirement cannot change a queued shot's
birth context. Both first-step classes use this path.

`write` uses the owner as the emitter. `write_from` can use a separate emitter
and credit body. A credit body in another session is refused before an ID is
spent. An unidentified source cannot supply a semantic descendant ID. Native
scratch hosts can still emit unidentified shots. Environmental requests with
no emitter capture the optional active lifetime at emission.

The projectile model owns physical construction and material admission. Combat
owns emission and source capture. `PendingProjectileEmissions` retains requests
that cross a tick boundary. It is cloned on rollback, remaps native owner
links, and has a semantic checksum. The emission message is an observation;
clearing that transport does not discard pending work. Session activation and
retirement clear the queue.

A room that rewinds alone (the room rollback host, or a guest that predicts
its own body) sets its rows of the queue back to what the queue carried into
the saved tick. The rows are the emissions whose credit names that room. A
tick-head record keeps what the queue carried into each recent tick
(`CarriedProjectileEmissions`), and the room's retractor reads it. While the
room replays, the materializers take only its emissions; other rooms' rows
wait for the live tick.

A `hit_reaction` entry can request `projectile`. That phase runs after the
early materializer. Its shot becomes available at the next early materializer.
The native pending queue preserves this timing across rollback. Early technique,
wielded-item, module-entity, and boss-conduct requests use the same service.

## Observe creation

The `projectile` request accepts an optional `reply`:

```rhai
this.requests.push(request("projectile", #{
    origin: this.trigger.position, dir: [1.0, 0.0], speed: 200.0,
    damage: 1, max_lifetime: 2.0, half_extent: [4.0, 4.0],
    reply: #{ request: 1, selector: "my_game::weapon/created" },
}));
```

Declare `created` as an `action_result` entry in `actor_think`. The reply
selector is the full entry path in the submitting module. Admission reports
`projectile_queued`. Actual construction reports terminal `applied` with code
`projectile_created` and one semantic shot ID in `subjects`. This result does
not promise that the shot remains alive when the callback runs. Invalid shape,
callback, scope, or emitter identity produces a native refusal.

A retry with the same request number and data does not create a second shot.
Use a new request number for each new shot. See [action results](action-results.md).

Explicit cancellation removes a queued emission before construction starts.
After the materializer takes the emission, cancellation cannot revoke its
birth. Creation completion does not retire the shot. Tracking is optional:
an emitted shot can still be born after its observer retires. Session teardown
clears pending emissions.

## Author a parry window

A script can request `parry_window` in `technique_execution`:

```rhai
#{ key: "guard", phase: "technique_execution",
   trigger: #{ port: "status_tick", selector: "my_game.reflecting" },
   requests: ["parry_window"], limit: 1 }
```

```rhai
fn guard() {
    this.requests.push(request("parry_window", #{ window_s: 0.15 }));
}
```

The duration must be finite and positive. The native guard uses the longer of
its existing and requested windows. This does not raise a held shield. Reissue
the request each active tick to hold the window open. `absorbs_projectiles:
true` consumes caught projectiles instead of reflecting them. Melee and
projectile contacts still use their native guard and hit rules.

Any controller can hold the status. The native body must carry guard state.
The request uses the existing native parry service; the script does not run a
second contact solver.

## Witness and remaining work

[`scripted_projectile_credit.rs`](../../game/ambition_app/tests/scripted_projectile_credit.rs)
loads one Rhai module. It fires a projectile, arms an NPC's parry, and retires
the deflector through native requests. The fixture checks original lineage,
current credit, and GGRS replay across reflection and retirement.

It also checks direct and splash reception after the deflector retires. The
loaded rule receives the same shot ID and retained credit for both native
decisions, including the actual applied damage. Native hit publication uses
`ambition_combat::HitWriter`. It captures source identity before deferred
reception. Detached strikes retain `HitOrigin`; this also covers non-projectile
sources whose owners retire.

[Projectile outcomes](projectile-outcomes.md) report native interception and
retirement. Accepted hit batches identify the shot and its current credit.
Request submission is not proof of creation or damage.

Native tests in
[`projectile_spawn/materialize.rs`](../../crates/ambition_combat/src/projectile_spawn/materialize.rs)
check delayed construction after source retirement, control and team changes,
travel, and session or round changes. They also check separate timing classes,
room-root emitters, and refusal of credit from another session.

The loaded reception fixture also requests a projectile from `hit_reaction`.
It checks native construction and GGRS replay across the pending interval.
The native pending tests check transport clearing, queue restore, one-time
consumption, and a checksum that excludes host session and entity counters.
