# `ambition_demo_mary_o` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_demo_mary_o** — Super Mary-O demo content home — the M-track's world half.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`ai_slop`](src/ai_slop.rs) | Mary-O's AI Slop — the plain stompable walker, authored as pure content. |
| [`autoplay`](src/autoplay.rs) | Mary-O plays herself, for unattended profiling: `AMBITION_AUTOPLAY=mary_o`. |
| [`bricks`](src/bricks.rs) | Mary-O's breakable bricks — the SECOND consumer of the reactive-block primitive. |
| [`castle_end`](src/castle_end.rs) | The end of a castle: a switch, the fall of the bridge, a gate, and who waits past it. |
| [`cosmic`](src/cosmic.rs) | Eternal Cosmic Mary-O: her ultimate form. |
| [`cosmic_aura`](src/cosmic_aura.rs) | The light about Eternal Cosmic Mary-O. |
| [`cosmic_barrier`](src/cosmic_barrier.rs) | The barrier of Eternal Cosmic Mary-O. |
| [`death`](src/death.rs) | The death beat. |
| [`fire_shader`](src/fire_shader.rs) | The glow of a fire in Mary-O: her spark blossom, and each orb of a bar of fire. |
| [`flag`](src/flag.rs) | — the flagpole sequence. |
| [`forms`](src/forms.rs) | The `mary_o_forms` schema: her power ladder, as data in the demo's pack (`assets/data/mary_o_forms.ron`). |
| [`ldtk_vocabulary`](src/ldtk_vocabulary.rs) | Mary-O-specific LDtk vocabulary. |
| [`level_1_2`](src/level_1_2.rs) | World 1-2: the level under the ground. |
| [`movement`](src/movement.rs) | Mary-O Classic presentation/control glue. |
| [`pack`](src/pack.rs) | The Mary-O demo's content pack. |
| [`pipe`](src/pipe.rs) | The pipe transit — the scripted slide into one tube and out of the other. |
| [`pipe_signs`](src/pipe_signs.rs) | The name of a level over the pipe that goes to it. |
| [`plane`](src/plane.rs) | Flying Mary-O enemy archetypes. |
| [`plant`](src/plant.rs) | The Snake Plant: the plant that comes up out of a pipe. |
| [`powerups`](src/powerups.rs) | Super Mary-O's powerups — the equipment chain, authored as A3 data. |
| [`provider`](src/provider.rs) | The Mary-O experience provider. |
| [`quasar_shader`](src/quasar_shader.rs) | Mary-O's invincible rainbow-quasar presentation pass. |
| [`rules`](src/rules.rs) | The `mary_o_rules` schema: the demo's own tuning, as data in its pack (`assets/data/mary_o_rules.ron`). |
| [`scenery`](src/scenery.rs) | Mary-O scenery props — the flagpole and the warp pipes, drawn from their generated construction sheets instead of flat placeholder quads. |
| [`shimmer_shader`](src/shimmer_shader.rs) | The shimmer in the air of a Mary-O room: sparkles and ribbons of light in the starlight rooms, embers and a low mist in the starless room, sparks and red heat in the castle. |
| [`snake`](src/snake.rs) | Mary-O's Solid Snake walker and shell state machine. |
| [`star`](src/star.rs) | Mary-O's pocket-quasar super state. |
| [`stomp`](src/stomp.rs) | The ONE top/side contact authority for Mary-O's enemies, in the engine's box type. |
| [`swallow`](src/swallow.rs) | The Laden Swallow: the bird that stays over the player and lets coconuts go. |
| [`test_course`](src/test_course.rs) | A level for the TESTS to play, which nobody authors. |
| [`void_lizard`](src/void_lizard.rs) | The Void Lizard: the boss of the castle. |
| [`void_look`](src/void_look.rs) | The field of the void about each void thing of Mary-O: the Void Star, the Void Lizard, and the orb the Void Lizard throws. |
| [`void_star`](src/void_star.rs) | The Void Star: the enemy that no stomp can hurt. |

_33 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
