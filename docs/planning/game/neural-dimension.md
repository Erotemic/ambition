# The Neural dimension: the Cortex and the Weights (started 2026-11-24)

**State:** BEGUN. Four rooms, two biomes, two enemies, four residents and the
gate are in the game. What is under "Next" is not built.

Jon (2026-11-24): "The Neural Dimension (both biological and artificial)".
The roster note (`docs/brainstorms/story_and_world_directions_2026-11-18.md`)
puts Neural with the worlds that emphasize encounters while still holding a
society.

The concept art (2026-11-22) is in the assets submodule,
`assets/concept_art/misc/`: `Neural Cathedral_ Memory and Bias-1.png` (the
living half: flesh-pink arches full of holes, dendrite trees, synapse caps
that spark, a signpost to MEMORY, ATTENTION and BIASES, memories as framed
pictures and film) and `Neural Dimension_ Cognitive Circuitry-2.png` (the
artificial half: memory banks, logic gates, a classifier sure that a rabbit
is a RED HERRING at 99.7 %, an echo chamber of screens, eye drones). They
are a reference for the look, not a layout.

| | |
| --- | --- |
| world file | `neural.ldtk` (`data/worlds.ron`: `world.neural_ldtk`) |
| biomes | `neural` (the Cortex, living), `neural_circuit` (the Weights, built) |
| gate address | `LATENT`, on the Callosum (`data/gate_network.ron`, posted `LEGAL`) |
| generator | `game/ambition_map_assets/ambition_content/generators/neural/make_specs.py` |
| dialogue | `dialogue/sandbox/neural.yarn`; the Hall's nodes in `hall.yarn` |
| art | renderer `backgrounds/neural.py`, `targets/props/neural_props.py`, `targets/characters/neural_*.py`, `data/townsfolk/neural_*.chargen.yaml` |
| music | `the_algorithm_knows_youre_lonely` |

## The shape of it

The dimension is one mind in two halves. The living half grew; the built
half was trained. The gate stands on the bridge between them, the Callosum
(the corpus callosum is the bundle of fibres that joins the two halves of a
brain).

```
                                                  door
                                          [the Echo Chamber]
                                                  |
 [ the Neural Cathedral ] ==== [ the Callosum ] ==== [ the Weights ]
```

- **The Callosum** (`neural_callosum`): the gate and terminal on a bridge of
  fibre over the dark, the signpost (MEMORY west, ATTENTION up, BIASES
  east), and the Grandmother Cell, who recognizes exactly one person.
- **The Neural Cathedral** (`neural_cathedral`), west: living arches and
  dendrite branches, synapse caps to climb, a gallery of memories, pools of
  fluid. Intrusive Thoughts drift in. Donna Hebb wires together the
  neurons that fire together.
- **The Weights** (`neural_weights`), east: racks of memory and the
  classifier's board. Logic gates cross the training spikes. Ladders at
  both ends lead to a maintenance tunnel under the spikes. Attention Heads
  watch from the air. Percy the Perceptron is on the floor.
- **The Echo Chamber** (`neural_echo_chamber`), by a door in the Weights: a
  round room of screens and Echo, who agrees with you.

## The people and the things that fight

| Id | Name | Where | Note |
| --- | --- | --- | --- |
| `npc_neural_grandmother_cell` | the Grandmother Cell | Callosum | A neuron that fires for one person only. She is sure it is you. |
| `npc_neural_hebb` | Donna Hebb | Cathedral | A matchmaker: cells that fire together, she wires together. |
| `npc_neural_perceptron` | Percy the Perceptron | Weights | An old machine of one layer. It answers yes or no, and it cannot do XOR. |
| `npc_neural_echo` | Echo | Echo Chamber | Says what you said, louder. |
| `intrusive_thought` | Intrusive Thought | Cathedral | A purple cloud of talk that drifts at you and will not leave. |
| `sunk_cost_fallacy` | Sunk Cost Fallacy | Cathedral | A diver beside the pool who swings his salvage chest. |
| `attention_head` | Attention Head | Weights | An eye on rotors that watches and fires at what it looks at. |

## Next

- The Memory Bank (MEMORY BANK A on the concept sheet) and the gallery of
  memories as a place to walk through.
- Biases as rooms: pattern recognition that sees a cat in everything, the
  red herring the classifier is sure of.
- Training: a room where the floor is learned, wrong at first.
- Its own music.
