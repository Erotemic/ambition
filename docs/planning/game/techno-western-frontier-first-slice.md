# Techno-western frontier: first playable slice

**Status:** Implementation handoff, not approved canon. Read `docs/brainstorms/2026-11-20-hobford-gates-and-frontier.md` for intent and ownership of ideas. This task is separate from the cowboy robot chicken boss already under construction.

## Goal

Build a playable, connected area in the frontier dimension. The area should stand on its own before the later damage, scrapyard, Fia, and Oiler story is fixed. Its style draws from Western frontier adventures, industrial science fiction, and comic absurdity. Build a place with local life and credible reasons for conflict, not a row of unrelated boss rooms.

## First slice

1. **Auxiliary gate town:** A small station, a gate terminal, a recovery inn, a rail bridge, and access to a local mechanic district. Use existing room, terminal, and dialogue capabilities. Do not place the main GA metropolis here.
2. **Departure:** Alice and Bob need rest after travel. They can direct Scout toward an ordinary mechanic. Do not force Scout to join or abandon them. Keep the proposed Hobford dialer damage as story context until the required terminal mechanics are designed.
3. **Train robbery:** An outlaw locomotive attempts to steal freight cars from another train. Scout falls or is knocked from a bridge into the event. The train's goal is freight, not Scout. Fighting begins when Scout gets in its way. Make a small playable sequence of train-top traversal, cargo hazards, and a clear exit far from the gate town. A staged moving-scene illusion is acceptable if it uses existing engine capabilities.
4. **Arrival district:** Connect the train sequence to an industrial frontier settlement or outlying district. Provide at least one alternative route or loop and space for later NPCs and adventures. Avoid a long flat corridor.

## Scope and ownership

- Another agent owns the **cowboy robot chicken boss**. Do not change its encounter, animations, behavior, or directly owned files. Inspect the live checkout for ongoing changes and coordinate shared content files before touching them.
- Do not select the second major boss yet. Do not force Scout into the scrapyard in this slice. The chicken boss may or may not become part of that story.
- Use the repo's LDtk and content-pack patterns. See `docs/planning/game/world-composition.md`, `docs/authoring/level-design-guide.md`, and `docs/planning/game/gate-network.md`. Follow `AGENTS.md`. Do not create or switch worktrees.
- Keep named content in authored data rather than bespoke engine cases. Build a dense, connected room cluster with a readable default route; allow later manual LDtk edits.
- A broken dialer that leaves one selectable destination is **not** the current gate engine behavior. Do not silently change the global dialing contract to implement a story beat. Defer or implement it through an explicitly reviewed local terminal state.
- Keep other agents' files and unrelated working changes untouched. Stage only files owned by this task and commit those files with the required co-author attribution.

## Done for this iteration

A player can arrive at the frontier terminal, explore the inn and bridge area, enter a playable train robbery, and emerge in a different district. Connections and collisions work in the current game. Only add focused verification for this path; do not expand the regression suite for unrelated features. Record the exact authored files and any missing assets or deferred dialogue in the handoff.

## Built (2026-11-20): a rough assembly

A first pass at the slice with its assets, so the pieces can be seen on the
stage (Jon, 2026-11-20: "a rough outline of the world existing with the
assets needed to bring the world to life ... and a basic assembly into a
level"). It has no gameplay scripting: no robbery sequence, no fight, no
scrapyard, no Fia or Oiler. Queue row: `SIDING` in `queue.md`.

**Decisions.** The auxiliary terminal is a gate of its own, address
`SIDING` (facility `halfway_siding`, in the Frontier dimension): an
auxiliary terminal is a separate facility, and an address reaches one gate.
Its terminal posts `LEGAL` and `LEADENSADDLE`. The dialing contract is
unchanged; the broken dialer is only in Alice's lines. The slice joins Dry
Dock by the rail line: Clinker Flats' railhead leads east onto Pad Three,
whose west side is now an edge.

**The route.** All rooms are in `frontier.ldtk`, west of Dry Dock, made by
`generators/frontier/make_specs.py` (its first-slice section):

| room | what |
| --- | --- |
| `frontier_siding_station` | The gate `SIDING` and its terminal. Ramps connect the apron and the town to the platform. The baggage awning and the trunk stack give access to the station roof. Augusta Pell is at the ticket window. |
| `frontier_siding_street` | Ramps connect the inn, the mercantile, and the telegraph office along the street. Visitors can reach the windmill deck by its ladder or from either porch. |
| `frontier_lie_down_inn` | Pearl Featherstone is at the desk. Alice and Bob are by the cots. Visitors can reach the gallery above the cots from the linen chest or the far stair. |
| `frontier_siding_trestle` | The road crosses the gulch. Two hatches lead to the inspection walks below it. There is a jump between the walks. Drop through the loose planks in the middle to board the freight. |
| `frontier_sprocket_row` | Hettie Gimbal is in the garage. A transfer walk links the garage hoist to the boiler in the parts yard. The street passes below the work platforms to the closed workshop gate. |
| `frontier_freight_tail` | Hollis Teague is on the caboose. Gangways connect the two loading cars at deck height. Ladders connect the gangways to the roofs. The decks lead to the flat car. |
| `frontier_freight_cars` | a tank car, an ore car, a box car, a flat car with a loose crate |
| `frontier_rustler_head` | the head of the train and the Rustler, the outlaw engine; off its pilot into Clinker Flats |
| `frontier_clinker_yard` | Clinker Flats' yard: the dock (Lou Shunter), the tipple road up to the works |
| `frontier_clinker_works` | the smelter works: furnace house, slag pots, crane gantry (Magda Ferro) |
| `frontier_clinker_row` | the Row under the works: shacks, the pump on a cinder hump (Ida Soames) |
| `frontier_clinker_railhead` | the old tramway grade down from the works, the line east to Pad Three |

Clinker Flats is a loop: the yard's tipple road and the railhead's grade
join the works' level (yard, works, railhead) and the street level (yard,
Row, railhead), so either way leads to the pad. The freight stands still
in its rooms and its backdrop runs past (`sky_scroll`); a fall between two
cars is a `HazardBlock` that puts the player back at the room's start.

**Assets.**
- Backdrops and skins (renderer `backgrounds/frontier_slice.py`, published
  with `backgrounds.sh`): `frontier_siding` (the rim, mesas, the town's
  backs, a windmill), `frontier_rails` (the gulch at speed), `frontier_works`
  (stacks, slag heaps, shacks under smoke).
- Scenery: 36 pieces in `targets/props/frontier_slice_props.py`, drawn on
  each room's walk lines (`generators/frontier/props.py` places them).
- People: seven chargen residents (`data/townsfolk/frontier_*.chargen.yaml`,
  catalog `npc_frontier_*`), Alice and Bob from the catalog; their lines in
  `dialogue/sandbox/frontier.yarn`, their wing in the Hall of Characters.
- Music: `halfway_siding` (platform, lie_down, trestle, sprocket),
  `the_rustler` (boxcars, rustler) and `clinker_flats` (works, row) from the
  music renderer's `dev/compose_halfway_siding.py`, bound in
  `music_cues.ron`. Jon's review of the first renders (2026-11-20): the town
  cue opened on silence and was too schmaltzy, the train cue was unbalanced,
  every cue must loop, and the Frontier needs more western tracks. The second
  version starts on its first section, sets the town on a dry boom-chuck
  guitar and a harmonica (an upright piano alone at the inn), rebalances the
  train by measured per-section loudness, and gives Clinker Flats its own
  cue. The game plays them once `scripts/regen/music.sh` publishes them
  where the registry is.

**The outlaw (2026-11-21).** Peyote Lean, Marshal Gallus's partner
(`leadensaddle.md`), keeps the Rustler's tender and shoots at anyone who
comes up the train.

**Missing or deferred.** The robbery is staged, not played: one outlaw
fights (Peyote Lean), the Rustler's grapples do not move, Scout is not knocked off the
trestle (the player drops through the planks). Sprocket Row's workshops are
shut and the ordinary mechanic has nothing to do yet. No new gate-sickness
or dialer rule. The trestle is one room: the train is not seen passing under
it.
