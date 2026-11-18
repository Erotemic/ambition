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

The runtime publishes identified shots before their first contact, including
shots that retire during that step. Both materialization timing classes use
this publication path. The original source must have a simulation identity and
counter. An unidentified source cannot supply a semantic descendant ID.

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

This does not complete the projectile outcome API. Per-shot callbacks and
accepted damage correlation remain open. Spawn requests also still need
captured emission context and downstream replies. A source that retires before
materialization cannot supply missing source facts at that later boundary.
Request submission is not proof of creation or damage.
