# Ambition Ability System

This is a draft design for optional movement/combat upgrades.

The endgame sandbox should usually enable every implemented ability so we can
judge whether the final movement kit is fun. The engine still represents each
ability explicitly because the real game will unlock them gradually, some future
abilities may be incompatible, and automated tests need to isolate one mechanic
at a time.

## Engine shape

Abilities live in `ambition_engine::AbilitySet`.

Current flags:

- `move_horizontal`
- `jump`
- `variable_jump`
- `double_jump`
- `wall_jump`
- `wall_cling`
- `wall_climb`
- `dash`
- `double_dash`
- `attack`
- `pogo`
- `rebound`
- `reset`

The sandbox constructs players with `AbilitySet::sandbox_all()`. Future story
states should use smaller sets or build a set from an upgrade graph.

## Dependency warnings

`AbilitySet::compatibility_warnings()` returns warnings for unusual ability
combinations, for example:

- `double_dash` enabled while `dash` is disabled
- `wall_climb` enabled while `wall_cling` is disabled
- `pogo` enabled while `attack` is disabled

These are warnings instead of hard errors because some story beats or challenge
rooms may intentionally violate ordinary upgrade dependencies.

## Dash resources

Dash is now charge-based:

- `dash = true`, `double_dash = false`: one dash charge
- `dash = true`, `double_dash = true`: two dash charges
- `dash = false`: zero dash charges

The old `dash_available` field still exists as a convenience/debug mirror, but
new code should prefer `dash_charges_available`.

## Wall abilities

`wall_cling` slows vertical slide while the player presses into a wall.

`wall_climb` allows vertical movement while clinging. It depends on
`wall_cling` in the normal upgrade graph, but the engine only warns rather than
rejecting that combination.

## Sandbox policy

The sandbox is an endgame feel lab, so it should keep all abilities enabled by
default unless a test preset or debug menu says otherwise.
