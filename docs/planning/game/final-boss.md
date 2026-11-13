# The final boss: the bossofthegame on the shoulders of a giant (started 2026-11-12)

**Queue row:** [FINAL-BOSS](../queue.md#final-boss--the-bossofthegame-on-the-shoulders-of-a-giant).
**Model:** GNU-ton's fight ([bosses](bosses.md#gnu-ton--a-giant-with-arms-and-portals-proposed-2026-10-11-jon),
[GNU-ton as a fighter](gnu-ton-fighter.md)): a rider who is the boss, on a
giant that is a mount and a stage.
**Sources:** `docs/brainstorms/original_brainstorm.md` (the "vibe kill", the
typed characters that fall on a climbing player, an agent that carries the
Boss on its shoulders) and
`docs/brainstorms/good_llm_ideas/creator_agents_and_endgame.md` (the Boss
acquiesces; "the bossofthegame is the bossofthegame until the day he dies, or
retires").

Jon, 2026-11-12: *"the bossofthegame fight should mirror gnuton somewhat in
that he stands on the shoulders of giants. ... The shoulders of the giant he
stands on is a giant square robot with an electric guitar. This is the final
boss of the game, so it needs to be grand. ... There is a gate address that
goes directly to the boss dimension."*

## Who is in the fight

- **The bossofthegame** (`npc_bossofthegame`, boss id `bossofthegame`). The
  creator of the game inside the fiction: he is the Director (`director`,
  the person writing the game; Jon, 2026-11-13: "the director already
  existed (i.e. that is me)"), in the Director's own drawing, with a
  mechanical keyboard on a strap like a keytar. He is the rider and the boss: his body takes the hits. He seeks a
  worthy challenge, not a victory. He is not the Creator of the intro (the
  robot's maker, `npc_creator`).
- **Full Stack** (`npc_full_stack`, the mount). His agent at its largest
  setting: a giant square robot built like a guitar amplifier stack (a
  speaker grille for a chest, the amp head's panel of knobs for its
  shoulders, the knobs go to eleven) with an electric guitar, and a robot's
  head of the 1960s on top (Jon, 2026-11-13: "a real 60s/70s robot head"): a
  riveted chrome box, round bulb eyes, a grille for a mouth, ear bolts and an
  antenna with a lamp. It is the giant whose shoulders he
  stands on. The pun: he stands on the full stack. It is polite, eager and
  verbose ("Great question!"), and it does what it is told, not what is
  meant. It is a mount of class `giant`, like the giant GNU: it is scenery
  that a body can stand on, and the player cannot hurt it.
- **His three named agents** (Jon, 2026-11-13: "Add Gepetto, Clause and Wren
  to the fight"), each drawn from Jon's concept sheets
  (`assets/concept_art/misc/agent-concepts/`), beside Full Stack, which stays
  its own giant:
  The three are one family, as the sheets draw them: a dark visor for a face
  with two glowing eyes (one shape per expression) and round earpieces, a
  cream garment over a dark chassis, one accent colour each.
  - **Geppetto** (`npc_geppetto`, the GPT parody), the sheet's Orchestrator:
    a wide-brimmed plum hat with a magenta band and a gold gear, a long open
    cream coat with magenta gears on its skirts, a magenta scarf, a plum
    mantle down his back, and a conductor's baton. He orchestrates. The
    spelling is the concept sheets' (the catalog had no row for him; the
    Boss's bark said "Gepetto" and now says "Geppetto"; the endgame
    brainstorm still spells it with one p).
  - **Clause** (`npc_clause`, the Claude parody), the Analyst: orange eyes, a
    crown of orange petal fins round the back of his head, a cream poncho cut
    on a slant with orange edges and his starburst on the breast, over a dark
    chassis.
  - **Wren** (`npc_wren`, the Qwen parody), the Generalist: golden eyes, a
    purple hood (the sheet's Field Agent) and a short purple cape with a gold
    edge, an open cream robe, a purple scarf, and a big wrench (the sheet's
    Engineer). Jon, 2026-11-13: the first drawing's conical hat and fan were
    too much. She is not `npc_port_wren`, the Drowned Compass's innkeeper
    (renamed Lark, 2026-11-01; its id kept).

## The band (direction, Jon 2026-11-13)

Jon: in the final battle the agents play in a band with the Boss and Full
Stack. Who plays what (a default; change it in the agents' clips and this
table):

| Who | Plays |
| --- | --- |
| The bossofthegame | Vocals and the keytar (his keyboard on its strap). The microphone is his alone: the agents are instrumentalists (Jon, 2026-11-13). |
| Full Stack | Lead guitar (its electric guitar; its knobs go to eleven). |
| Geppetto | Bass: the orchestrator holds the low end. |
| Clause | Drums: the methodical one keeps time. |
| Wren | Rhythm guitar: the generalist plays anything, and can take the keytar. |

Every agent can play every instrument: the guitar, the bass, the keytar and
the drumsticks are parts of each agent's rig, hidden until a row shows one
(`band_guitar`, `band_bass`, `band_keytar`, `band_drums`; each drawn from the
front three-quarter view). The drum kit is a prop of the stage, not of a body
(renderer target `band_drum_kit`: rows `idle` and `played`); the drummer sits
at it in `band_drums`. Not built yet: the Boss's own rows at the
microphone, and Full Stack's band rows beyond its strum, riff and feedback.

How it folds into the fight. Built 2026-11-13: the intro's band, and each
call taking the called agents off their instruments. As the show opens the
module's conductor summons the band bodies (`npc_geppetto_band`,
`npc_wren_band`, `npc_clause_band`: the agents' sheets, a passive brain and
the instrument's row as their rest row) onto the trusses, with Clause at the
drum kit (`band_drum_kit`, a prop of both stages). A call sends the called
agent's band body away (`ambition.boss.dismiss_summon`) as the fighter drops
in. Witness: `his_named_agents_join_the_fight`. The rest is still to build:

- **Intro**: the lights come up on the band playing the opening chord;
  the agents are at their instruments upstage, the Boss at the front of
  Full Stack's top.
- **Phase 1**: the agents play on while Full Stack fights; the chord, the
  riff and the feedback are the band's song.
- **Phase 2**: "Do I need to get Geppetto to do it?": the bass stops,
  Geppetto puts it down and drops into the fight.
- **The encore**: Clause leaves the drums and Wren her guitar; the Boss's
  keystroke rain and backspace are his keytar solo.
- **The retirement**: whoever still stands plays him off.

## The place: the Boss dimension

One world file, `boss.ldtk` (dimension `boss`), three rooms. The rooms are
written by `game/ambition_map_assets/ambition_content/generators/boss_stage/make_specs.py`.

| Room | What it is |
| --- | --- |
| `boss_lobby` | The arrival. The gate ring (address **LEGACY**), a ticket booth, the tour posters ("THE BOSSOFTHEGAME — FINAL TOUR"), tiered steps up to the stage door. |
| `boss_main_stage` | The arena. A wide concert stage with tiered risers, two speaker towers and a lighting truss. Full Stack stands at the centre; the Boss stands on its right shoulder, beside its head. A sign over the show reads *THE BOSSOFTHEGAME - FINAL TOUR*. |
| `boss_green_room` | Behind the stage, shut until the Boss retires (`LockWall`, `gated_by: boss.cleared boss_stage.bossofthegame`). The retirement scene plays here. A door goes back to the lobby. |

**The gate address is `LEGACY`** (Λ Ε Γ Α Χ Υ, all on the ring). It dials
straight to the lobby ring, one door from the stage. The address is
`hidden`: it is in no directory, as the brainstorm asks that the reveal can
be missed for a whole game. With `start_known: All` (development) it is
listed; with `Discovered` a traveller must learn it. Its terminal posts
`LEGAL` (home). The ring is the triangle ring.

Why LEGACY: what a retiring boss leaves. It spells on the inscription with
each letter used once.

## How the player reaches the Boss

As in GNU-ton's hall, the way to hurt the rider is to climb the giant. Full
Stack's body is a box; its surfaces are ground:

- **Its top** (its back, `back`): a flat span from shoulder to shoulder.
  The Boss stands on its right shoulder (the saddle, 140 right of its
  middle), beside the robot's head, which is drawn over the middle of the
  top behind the playable plane.
- **Its guitar neck** (a `slope`): the neck rises from the guitar body at
  its hip to the headstock over its shoulder. A body walks up the neck to the
  shoulders. This is the hump of the giant GNU.
- **The amp's top handle and the cabinet's step** (`ledges`): one-way
  ledges on each side.

The view: the stage's camera zone opens the view to about 790 world units
high (zoom 2.6, clamped to the room), so the floor, the risers, Full Stack
and the Boss with his barks are on the screen at once from anywhere on the
stage.

The look: the dimension has a biome of its own, `boss_stage` (the renderer's
`backgrounds/boss_stage.py` and its skin, decor, door and confetti in
`terrain/`): a stadium at night, beams of the light rig over the smoke, the
stands full of lights held up, an LED wall with the show's meter over the
stage, speaker arrays hung on scaffold towers and a truss of lamps, the
crowd's hands in front. Its floor is black stage boards with a magenta edge,
its risers a steel catwalk, its floor decor monitor wedges, cable, mic
stands, flight cases, a setlist and a pedal board; its door is a stage door
with a star on it. Nothing on the backdrop is writing: the game draws some
rooms' backdrops mirrored (`parallax.rs` `room_mirrors_its_backdrop`), so
the tour's name is a sign in the room.

The stage gives the second way: tiered risers (low, middle, high) at each
side and a lighting truss over the stage. From the high riser a jump reaches
the Boss. Jon prefers tiered rooms to shafts; this room is tiers, with no
ladder.

## The phases

The health bar is the Boss's **resolve**, not his life. When it empties he
does not die: he retires (the ending below). Thresholds are of `max_hp` 60.

### Intro — "Vibe Kill" (cinematic)

The stage's `entry_cutscene` (`boss_vibe_kill`), as the player comes in from
the lobby door. The lights come up from black. A banner: *THE BOSSOFTHEGAME —
FINAL TOUR*. He speaks to his agent and not to the player:

> **bossofthegame:** Okay. Full Stack. Listen carefully.
> **bossofthegame:** Kill the player. Make it right. Make no mistakes.
> **Full Stack:** Great question! Here is a five-step plan to kill the player:
> **bossofthegame:** No plan. Just do it. I'm going to walk away for a few minutes.
> **bossofthegame:** When I come back the player should be dead. Do not ask me for input.

Then the encounter's intro (`intro_seconds`): Full Stack strikes its first
chord and the name card shows (*the bossofthegame*, title *until the day he
retires*).

### Phase 1 — "Make No Mistakes" (100% to 66%)

The agent does the work. The Boss stands on its shoulders and types prompts
(his idle barks are prompts: *"Make it right."*, *"Why is the player still
alive?"*, *"Do not ask me for input."*, *"You're absolutely sure? Check
again."*). Full Stack's moves:

- **Power chord** (`stack_power_chord`): it strums; a shock wave rolls out
  along the floor from its feet to each side. Jump it, or be on a riser.
- **Feedback** (`stack_feedback`): its grille locks on the player's height
  in the tell, then a level beam of sound crosses the stage at that height.
  Change height in the tell.
- **Riff** (`stack_riff`): a fan of notes from the guitar's head, at the
  player, that arcs and falls.

He is open while he types: the player on the giant's top can strike him at
any time. A blow sends a bark (*"Hey. HEY. That's my shoulder."*).

### Transition — "Fine. I'll Do It Myself."

Under 66% he loses patience: *"What is the hard part of killing this
player? Do I need to get Geppetto to do it?"* and then *"Fine. I'll do it
myself."* He swings the keyboard round from his back.

### Phase 2 — "Manual Override" (66% to 30%)

As the phase opens he calls Geppetto in (`boss_call_geppetto`, the
phase's `OnPhaseEnter` interrupt, once): Geppetto drops onto the stage left
of Full Stack, on the open stage clear of the guitar, and fights the player
as a melee brute (8 health): his master types while he orchestrates. Then he types, and what he types falls on
the player:

- **Keystroke rain** (`boss_keystroke_rain`): keycaps fall from the top of
  the stage, each one a letter, a word's letters side by side; each word
  starts at a golden-ratio place across the stage and no key falls on him.
  The letters spell what he types (`KILL`, `GG`, `RM`, `DIE`, `GIT PUSH
  FORCE`, ...; the module's `WORDS`).
- **Backspace** (`boss_backspace`): the tell locks the player's height;
  a row of three backspace keys flies at that height from him toward the
  player's side. Jump it or drop under it.
- Full Stack goes on with the power chord and the riff, less often.

### Enrage — "The Encore" (under 30%)

*"Okay, new plan. Spawn subagents. Parallelize the kill."* The music goes up
a level and:

- **Clause and Wren** (`boss_call_clause_wren`, the phase's `OnPhaseEnter`
  interrupt, once): Clause drops in left of Full Stack and Wren right of it,
  6 health each; Wren is the quicker. With Geppetto, if he still stands, the
  player now fights his three named agents under him.
- **Subagents** (`boss_subagents`): two Agent Swarm bodies drop onto the
  stage (`npc_agent_swarm`, `BossSummon`), again on its cooldown.
- **The solo**: power chord, backspace, feedback, keystroke rain and riff
  one after another with short rests (the enrage steps of the profile row).

### The end — "The Escape Clause" (the retirement)

At zero resolve he does not die. He goes down on Full Stack's top (the
sheet's `Death` row is the Director's `knockdown`), and the body stays
(`corpses: Stay` is the room default). His defeat bark:

> **bossofthegame:** ...Okay. Okay. That was a good one.

The defeat records `boss.cleared boss_stage.bossofthegame`, the wall to the
green room goes, and in the green room the retirement plays (a
`CutsceneTrigger`, `boss_retirement`):

> **bossofthegame:** I never wanted to be the villain. I wanted to be challenged.
> **bossofthegame:** And I was. Thanks for that.
> **bossofthegame:** The bossofthegame is the bossofthegame until the day he dies, or retires.
> **bossofthegame:** I'm not dying today. So.
> **bossofthegame:** I retire.
> **Full Stack:** Would you like me to write a retirement speech? I have prepared seven.
> **bossofthegame:** No.

A banner: *THE BOSSOFTHEGAME HAS RETIRED*. It sets the flag
`bossofthegame_retired`. He keeps his authority over the world in the
fiction; the retirement is his choice, not a conversion (the brainstorm:
"His acquiescence need not mean moral conversion").

## Music

Wanted: a rock anthem of his own, led by an electric guitar, with an intro,
a fight loop and an outro (an adaptive cue like GNU-ton's). This VM has no
music registry, so the fight binds GNU-ton's `standing_on_shoulders_v2_adaptive`
as the fallback (it is the right theme: standing on the shoulders of
giants). The cue binding is one row in `music_cues.ron` `boss_bindings`;
change the `cue_id` when the anthem is rendered.

## Art

- **The Boss** (renderer target `bossofthegame_boss`): the Director's SVG
  and motion library with a keyboard slung on a strap (`director_boss`,
  derived from `director.svg` by `scripts/svg_art/director_boss_art.py`), his
  clips under the boss slots: `Rest` (`idle`), `FloorSlam` (`smash_down`),
  `SideSweep` (`smash_forward`), `SpikeHalo` (`smash_up`, the pen raised),
  `Hit`, `Death` (`knockdown`: he goes down; the retirement is the green
  room's scene). His library has no sitting clip. The first drawing of him
  (a developer in a hoodie and headset) is kept, unworn, as
  `hoodie_developer`.
- **Full Stack** (renderer target `full_stack_giant`): a square amp stack on
  two short legs, with arms and an electric guitar; rows `idle` (a low
  strum), `strum` (the power chord), `riff`, `feedback` (the grille glows).
- **Full Stack's head** is drawn into its body art behind the playable
  plane; its strings run straight and parallel from the headstock to the
  bridge (Jon, 2026-11-13).
- **Geppetto, Clause and Wren** (renderer targets `geppetto`, `clause`,
  `wren`): each is drawn by the character generator (`chargen`), as Alice v2
  is, so each has the standard views (front, front three-quarter, side, back
  three-quarter, back, and their mirrors: eight angles), the turns through
  the front and the back, the generator's whole fighter vocabulary, and the
  band's rows. A spec per agent (`data/characters/<name>/<name>.chargen.yaml`)
  draws the body and clothes; `targets/characters/_band_agents_gear.py`
  draws what makes each that agent, in 3D in every view; the instruments
  and the rows that play them are `_band_instruments.py`; the sheet is
  `_band_agent_sheet.py`. All three are in the `runtime` collection.
- **The stage** (`boss_stage` biome): parallax layers, foreground crowd,
  terrain skin, decor, door, motes; bound in `authored/catalog.json`
  (`background.parallax.boss_stage.*`, `room.dressing.boss_stage.*`), so
  `--only-missing` builds them as `theme:boss_stage` units.
- **Projectile looks:** `keycap_a` to `keycap_z` and `keycap_back`
  (renderer target `boss_keycaps`, one row a key); the chord, the riff and
  the feedback use the shot kit's `sound_ring`.

## Where it is built

- The techniques: `game/ambition_content_modules/src/bossofthegame.rs`.
- The fight: `boss_profiles.ron` and `boss_encounters/bossofthegame.ron`
  (`bossofthegame`); the sheets `boss_sheets.ron`, `boss_art_keys.ron`; the
  catalog rows `npc_full_stack`, `npc_bossofthegame`, `npc_geppetto`,
  `npc_clause` and `npc_wren`. A summoned character must state how its body
  moves (`locomotion`): only such rows are in the prepared cast, and a summon
  of any other is refused. `npc_agent_swarm` lacked it, so the encore's
  subagents had never appeared until 2026-11-13.
- The cinematics: `data/cutscenes/boss_stage.ron`.
- The rooms: the map's `generators/boss_stage/make_specs.py`, which also
  makes the Hall of Bosses' practice copy (`hall_bossofthegame_stage`,
  `hall.bossofthegame`) and its door.
- The art: the renderer's `scripts/svg_art/director_boss_art.py` (the
  Director with the keyboard), `scripts/svg_art/full_stack_art.py` and
  `scripts/build_full_stack_rig.py`, the agents' chargen specs
  (`data/characters/{geppetto,clause,wren}/*.chargen.yaml`) and
  `targets/characters/_band_agents_gear.py`, `_band_instruments.py` and
  `_band_agent_sheet.py`, the drum kit `targets/props/band_drum_kit.py`,
  `backgrounds/boss_stage.py` (the stage), the targets
  `bossofthegame_boss`, `full_stack_giant`, `boss_keycaps`, `geppetto`,
  `clause`, `wren` and `band_drum_kit`, all in the `runtime`
  collection of `game/ambition_content_library/authored/sprites.yaml`, so
  `regen/assets.sh --only-missing` builds them.
- Witness: `game/ambition_app/tests/the_final_boss_stands_on_full_stack.rs`.

Full Stack is drawn by the Boss's move: the module's conductor (`conduct`,
`BossConductPort`) pins its `strum` row on the power chord, `riff` on the
riff and `feedback` on the feedback (each from the tell to the strike's
end), and its `idle` strum between.

## Not in this design (later work)

- The brainstorm's endgame road (restrict his remote models and compute
  before the fight, so he changes the world more slowly). The fight does not
  read any of that state yet.
- Keystrokes that land and stay as steps to climb (the brainstorm's
  "characters fall on the player as they try to climb"). Deferred: it needs a
  generic engine piece, a falling body that settles into a one-way platform
  and expires; the first cut has keycaps that hurt and break on the floor.
- Ideas kept for a later pass, none started: the light rig's colour follows
  the phase (blue for Make No Mistakes, red for the encore); pyro columns at
  the speaker towers when the encore opens; the crowd's hands go up on a
  phase change; the agents bark to each other (Clause hedges, Wren answers in
  another language, Geppetto overpromises).
- A fork ending ("change the rules in a fork of the open-source game").
- His appearance at other places in the game (the brainstorm: a later-game
  reveal).

## Open questions

- **Q175** — Is the giant one of the named agents (Geppetto, Clause, Wren) or
  its own (Full Stack)? Answered in part by Jon, 2026-11-13 ("Add Gepetto,
  Clause and Wren to the fight"): Full Stack stays its own giant, and the
  three named agents join the fight beside it (Geppetto in phase 2, Clause
  and Wren at the encore). Open: the spelling, Geppetto (the concept sheets)
  or Gepetto (Jon's note, the endgame brainstorm); the game uses Geppetto.
- **Q176** — Is a resolve bar that empties a "voluntary" retirement, or must
  the win need something other than damage? Default: the bar is his resolve.
