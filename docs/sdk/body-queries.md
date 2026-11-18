# Bounded body queries

Use `body_query` to select bodies in the caller's live room and session. It is
available in `actor_think`, `technique_execution`, `wielded_use`, `boss_conduct`,
`module_entity_tick`, `body_contact`, and `hit_reaction`. Declare it in the
entry's `requests` list.

```rhai
this.requests.push(request("body_query", #{
    reply: #{ request: 0, selector: "my_game::ability/result" },
    origin: this.trigger.position,
    radius: 600.0,
    limit: 3,
    relation: "foe",
}));
```

Declare the reply entry with the `action_result` trigger in `actor_think`,
and with `reads: ["body_query_result"]`. The full selector must name an entry
in the submitting module. See [action results](action-results.md) for request
numbers, retry, cancellation, scope lifetime, and delivery.

```rhai
fn result() {
    if this.trigger.stage != "applied" { return; }
    let found = this.observe.body_query_result;
    if found == () { return; }
    for target in found.targets {
        // Use the native identity for a later domain request.
    }
}
```

`body_query_result` is optional. It contains facts only for the query named by
the current applied `action_result`. It does not expose another entry's query
history. A successful query reports code `body_query_ready` and ends the action.
An empty selection is a successful query with an empty target list.

## Selection and scope

- `origin` and `radius` use room world units, with +Y down. The radius includes
  body centres. It is not a shape-overlap or line-of-sight test.
- `limit` is positive. Results use distance order, then simulation identity for
  equal distances. `truncated` is true when more matching bodies existed.
- `relation` can be `"foe"`, `"ally"`, `"neutral"`, or absent. It uses the shared
  combat relation, including team, faction, and grudge rules. It does not apply
  friendly-fire damage policy. A relation filter needs a combat source.
- `include_self` defaults to false. Disabled, dead, out-of-play, and untouchable
  depth-plane bodies are excluded. Controlled and autonomous bodies use the
  same path.
- `identities` can restrict selection to retained body identities. It defaults
  to an empty list, which searches the area.
- `require_all: true` requires a nonempty identity list. All named bodies must
  still pass the scope, life, radius, and relation filters and fit the limit.
  Otherwise the request ends with `rejected` and code `target_missing`.

Each target supplies its semantic `id`, physical centre `position`, physical
full extent `size`, `velocity`, whether it is `driven`, optional `relation`,
and centre `distance`. These are physical body facts, not hit or hurt geometry.
Keep an identity in a declared `body` or `opt body` record field when it must
survive invocations and rollback.

The result includes the matching `ticket`, `observed_at` tick, live `room`,
and producer `phase`. Facts describe that phase's request-lowering read cut.
They can change before the reply runs. A query does not reserve a target;
the domain that applies a later action must validate it again.

## Game budgets and replay

A game's `room_rules` content can set:

```ron
(body_queries: Some((max_targets: 16, max_radius: 2000.0)))
```

The default limits are 32 targets and a radius of 8192 world units. Requests
can choose smaller bounds. Invalid geometry or an exceeded budget produces a
terminal rejection. Content reload replaces or removes the scoped budget.
Removing it restores the engine defaults.

The body retains at most 64 successful query results for correlated replies.
Results and action delivery cursors are canonical rollback state. Retrying a
request does not rerun the search at a later tick. Use a new request number
for a new read.

The portable types are in
`ambition_platformer2d::extension::combat::body_query`. The assembled
[headless fixture](../../game/ambition_app/tests/scripted_body_queries.rs)
uses the loaded Rhai host for a controlled body and an NPC. It checks bounded
selection, retained identities, relationship changes, refusal, and replay.
