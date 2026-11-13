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
  creator of the game inside the fiction: a developer in a hoodie, a gamer
  headset and a paper crown, with a mechanical keyboard on a strap like a
  keytar. He is the rider and the boss: his body takes the hits. He seeks a
  worthy challenge, not a victory. He is not the Creator of the intro (the
  robot's maker, `npc_creator`).
- **Full Stack** (`npc_full_stack`, the mount). His agent at its largest
  setting: a giant square robot built like a guitar amplifier stack (a
  speaker grille for a chest, a control panel of knobs for a face, the knobs
  go to eleven) with an electric guitar. It is the giant whose shoulders he
  stands on. The pun: he stands on the full stack. It is polite, eager and
  verbose ("Great question!"), and it does what it is told, not what is
  meant. It is a mount of class `giant`, like the giant GNU: it is scenery
  that a body can stand on, and the player cannot hurt it.

## The place: the Boss dimension

One world file, `boss.ldtk` (dimension `boss`), three rooms. The rooms are
written by `game/ambition_map_assets/ambition_content/generators/boss_stage/make_specs.py`.

| Room | What it is |
| --- | --- |
| `boss_lobby` | The arrival. The gate ring (address **LEGACY**), a ticket booth, the tour posters ("THE BOSSOFTHEGAME — FINAL TOUR"), tiered steps up to the stage door. |
| `boss_main_stage` | The arena. A wide concert stage with tiered risers, two speaker towers and a lighting truss. Full Stack stands at the centre; the Boss stands on its shoulders. |
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
  The Boss stands on it.
- **Its guitar neck** (a `slope`): the neck rises from the guitar body at
  its hip to the headstock over its shoulder. A body walks up the neck to the
  shoulders. This is the hump of the giant GNU.
- **The amp's top handle and the cabinet's step** (`ledges`): one-way
  ledges on each side.

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
player? Do I need to get Gepetto to do it?"* and then *"Fine. I'll do it
myself."* He swings the keyboard round from his back.

### Phase 2 — "Manual Override" (66% to 30%)

He types, and what he types falls on the player:

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

- **Subagents** (`boss_subagents`): two Agent Swarm bodies drop onto the
  stage (`npc_agent_swarm`, `BossSummon`).
- **The solo**: power chord, backspace, feedback, keystroke rain and riff
  one after another with short rests (the enrage steps of the profile row).

### The end — "The Escape Clause" (the retirement)

At zero resolve he does not die. He sits down on Full Stack's top (the
sheet's `Death` row is him sitting, crown in hand), and the corpse stays
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

- **The Boss** (renderer target `bossofthegame_boss`, an SVG rig like the
  Creator's): rows `Rest` (typing on the strapped keyboard), `FloorSlam`
  (two hands down on the keys), `SideSweep` (a swing of the keyboard),
  `SpikeHalo` (arms up, conducting), `Hit`, `Death` (he sits down, crown in
  hand).
- **Full Stack** (renderer target `full_stack_giant`): a square amp stack on
  two short legs, with arms and an electric guitar; rows `idle` (a low
  strum), `strum` (the power chord), `riff`, `feedback` (the grille glows).
- **Projectile looks:** `keycap_a` to `keycap_z` and `keycap_back`
  (renderer target `boss_keycaps`, one row a key); the chord, the riff and
  the feedback use the shot kit's `sound_ring`.

## Where it is built

- The techniques: `game/ambition_content_modules/src/bossofthegame.rs`.
- The fight: `boss_profiles.ron` and `boss_encounters/bossofthegame.ron`
  (`bossofthegame`); the sheets `boss_sheets.ron`, `boss_art_keys.ron`; the
  catalog rows `npc_full_stack` and `npc_bossofthegame`.
- The cinematics: `data/cutscenes/boss_stage.ron`.
- The rooms: the map's `generators/boss_stage/make_specs.py`, which also
  makes the Hall of Bosses' practice copy (`hall_bossofthegame_stage`,
  `hall.bossofthegame`) and its door.
- The art: the renderer's `scripts/svg_art/{bossofthegame,full_stack}_art.py`,
  `scripts/build_{bossofthegame,full_stack}_rig.py`, the targets
  `bossofthegame_boss`, `full_stack_giant` and `boss_keycaps`.
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
  "characters fall on the player as they try to climb"). It needs a body that
  is ground and falls; the first cut has keycaps that hurt and break on the
  floor.
- A fork ending ("change the rules in a fork of the open-source game").
- His appearance at other places in the game (the brainstorm: a later-game
  reveal).

## Open questions

- **Q175** — Is the giant one of the named agents (Gepetto, Clause, Wren) or
  its own (Full Stack)? Default: its own.
- **Q176** — Is a resolve bar that empties a "voluntary" retirement, or must
  the win need something other than damage? Default: the bar is his resolve.
