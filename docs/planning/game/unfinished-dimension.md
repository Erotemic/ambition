# The Unfinished dimension: the site the Boss's agents are building (started 2026-11-25)

**State:** BEGUN. Three rooms, two biomes, three residents, one enemy and
the gate are in the game. What is under "Next" is not built.

Jon (2026-11-24): "The Unfinished Dimension (the dimension that the boss's
agents are still working on)", with a note on its concept art: "the concept
is a bit too colorful, probably not the greatest". The colour here is held
down on purpose: blueprint paper, unpainted greys and browns, muted greens,
blueprint ink for what is only drawn, and one accent, the yellow of a hard
hat.

The agents are the Boss's ([final boss](final-boss.md): Geppetto, Clause
and Wren, one family with dark visors and two glowing eyes). The endgame
brainstorm (`docs/brainstorms/good_llm_ideas/creator_agents_and_endgame.md`,
non-canonical) calls Clause's smaller instances **subclauses**; the workers
here are subclauses in hard hats. The same brainstorm's "unfinished world"
(abandoned concepts surviving in the Void) is a direction for later, not
what this slice builds.

The concept art (2026-11-24) is in the assets submodule,
`assets/concept_art/misc/`: `Unfinished Dimension_ Building Tomorrow.png`
and four sheets named `ChatGPT Image Nov 24, 2026, 11_28_*.png` (workers in
hard hats with glowing eyes, a painter rolling colour onto the sky,
surveyors with theodolites and blueprint easels, wireframe trees and
castles beside painted ones, cranes lowering chunks of land, gauges of
gravity, weather and layers, and the void with a galaxy under it). They are
a reference for the look, not a layout, and not for its colours.

| | |
| --- | --- |
| world file | `unfinished.ldtk` (`data/worlds.ron`: `world.unfinished_ldtk`) |
| biomes | `unfinished` (the building site), `unfinished_void` (where nothing is built yet) |
| gate address | `INSTALLING`, in the Staging Area (`data/gate_network.ron`, posted `LEGAL`) |
| generator | `game/ambition_map_assets/ambition_content/generators/unfinished/make_specs.py` |
| dialogue | `dialogue/sandbox/unfinished.yarn`; the Hall's nodes in `hall.yarn` |
| art | renderer `backgrounds/unfinished.py`, `targets/props/unfinished_props.py`, `targets/characters/unfinished_subclause.py`, `targets/characters/glitch_bug.py`, `data/townsfolk/unfinished_placeholder.chargen.yaml` |
| music | `something_worth_building` |

## The shape of it

```
      door
 [the Site Office]
        |
 [ the Staging Area ] ==== [ the Draft ]
                             (the void under it)
```

- **The Staging Area** (`unfinished_staging`): the gate on the first
  ground the agents laid; the site cabin (the Site Office's door); the
  survey post and Subclause 4(b); three trees painted on one side and
  still a drawing on the other; the scaffold up to a chest.
- **The Draft** (`unfinished_draft`), east: the built ground ends and a
  stream runs off it into the void. Slabs hang from cranes' cables over the
  void, a jump apart; between them, the outlines of platforms not built yet
  (they are drawings: they do not hold you). Subclause 4(c) paints the sky
  from a hanging platform. A Bug and Feature Creep guard the built ground.
- **The Site Office** (`unfinished_site_office`), by a door: the gauges of
  GRAVITY, WEATHER and LAYERS, the backlog, and the Placeholder.

## The people and the things that fight

| Id | Name | Where | Note |
| --- | --- | --- | --- |
| `npc_unfinished_subclause_surveyor` | Subclause 4(b) | the Staging Area | Surveys ground that is not there yet; can tell you what the spec says, not what it means. |
| `npc_unfinished_subclause_painter` | Subclause 4(c) | the Draft | Paints the sky: one coat of blue, two of afternoon. |
| `npc_unfinished_placeholder` | Placeholder | the Site Office | A grey stand-in for "a wise old someone, to be decided", since the second sprint. |
| `glitch_bug` | Bug | the Draft | A beetle with a missing texture for a shell; a slice of it slips sideways. Known issue. Won't fix. |
| `feature_creep` | Feature Creep | the Draft | A heavy prototype that strikes with a piston. |

## Next

- Gameplay from the unfinished: platforms that are built while you watch
  (a crane lowers one), drawings that become solid when painted, the gauges
  of gravity and weather as switches.
- The Void as a place: the abandoned concepts of the brainstorm, earlier
  drafts of things that exist elsewhere.
- Wrens (Wren's small instances) on the site, once their look is settled.
- Its own music.
