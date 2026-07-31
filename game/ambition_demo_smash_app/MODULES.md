# ambition_demo_smash_app

The smash demo's thin shell. `ambition` + `ambition_demo_smash` + `bevy`, never
`ambition_app` — that is the demo gate.

| module | what it owns |
|---|---|
| `lib.rs` | `build_demo_app()` — foundation, engine group, host group, the smash experience |
| `main.rs` | headless by default, windowed under `--features visible` |
| `tests/the_stage_kills.rs` | the claims no unit test in the content crate can make |

## Why this crate exists

Until something RAN the stage, every claim about the stocks loop was a unit
test. Spend, respawn, eliminate and end were each covered and each correct; what
nothing covered was whether a fighter knocked off THIS platform, with THIS blast
margin, reaches the world's edge at all.

It earned that on its first boot, three times over — each a failure no compile
and no content test could produce:

1. **`frontend audio provider 'smash' registered no audio fragment`.** The
   content installer was empty. Declaring SILENCE is a registration, not the
   absence of one.
2. **`character_catalog: Resource does not exist`.** The demo had declared a
   starting character from Ambition's robot lineage — which lives in
   `game/ambition_content`, ABOVE the facade, so naming it would have broken the
   oracle rule outright. The demo authors its own two duelists; the crossover
   claim moves to where Ambition hosts this experience alongside its own.
3. **A malformed action-set preset.** `melee: Some(...)` takes an authored swipe
   shape, not a damage/cooldown pair.
