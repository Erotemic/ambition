# Awaiting a maintainer decision

This file holds only open maintainer and product questions. Engineering work
that can proceed without a ruling goes in [`queue.md`](queue.md).

- Give each question one unique `Q<number>`. Do not reuse a retired number.
- Use the heading shape `## Q<number> — <question>`. Other documents link to
  these anchors, so do not change the text of a heading.
- Each entry states the question, the context needed to choose, the owner, and
  what changes for each answer.
- A queue row that waits on a question names it in a `**Blocked by:**` field.
- When a question is answered, record the ruling in
  [`maintainer-decisions.md`](maintainer-decisions.md), update the owner plan or
  source, and delete the question here.
- Before you file a question, make sure that no ruling already decides it. A
  decided direction with unfinished implementation is a queue row, not a
  question.
- Do not file "nothing uses X: keep or delete?". Current usage is not a deletion
  criterion (Q74 ruling, 2026-10-03); ask about meaning, authority or cost.

## What actually blocks architecture work today

This is the blocking set, not the important set. A question can matter and
block nothing.

**The blocking set holds one question (2026-11-17).** It blocks one small-screen
row, not architecture work. The previous one, `Q157`, was ruled on
2026-10-19.

| Question | Blocks |
|---|---|
| [`Q178`](#q178--on-a-small-screen-does-the-touch-overlay-give-way-to-a-menu) | SMALL-VIEWPORT-UI: the touch overlay on small menus (the dialogue box and other screens go on) |

`scripts/checks/check_blocking_set_names_every_gate.py` reads the queue's
`**Blocked by:**` fields and fails if a gate has no row in a table here. Each
table row links a `Q` that has a section on this page with at least two
options, written `* **(a) …`. Derive gates from the field, not from row prose.

## Open questions

`Q168` and `Q170` were decided on 2026-11-06 and are recorded in
[`maintainer-decisions.md`](maintainer-decisions.md). Their remaining
engineering and content work is indexed in [`tracks.md`](tracks.md), not held
as a product-decision gate.

## Q171 — Does holding down crouch at once, or look down first?

**Asked 2026-11-11 (Jon, unsure):** *"pressing down maybe shouldn't
immediately put you into a crouch, maybe it just has your head look down. And
after a bit of time it also translates the camera a little bit down so you can
actually 'look down'. The same 'look up' should happen for when you press and
hold up."*

**What exists:** up held while standing turns the head up (`idle_look_up`,
over a walk too). The camera already leans: a full look up held still, or a
full crouch held still, for `LOOK_DELAY_SECS` (0.8 s) leans the view by
`LOOK_SHARE_OF_VIEW` (0.3) of its height over `LOOK_RAMP_SECS` (0.5 s)
(`ambition_sim_view::camera_snapshot::camera_look`). Down crouches at once:
a smaller hurtbox, the crawl, the down+jump drop through a one-way, and the
crouching attacks all key on it.

**Options:**

* **(a) Keep the crouch at once** (now). Down is the crouch; the camera's
  look down is the crouch held still. Nothing changes.
* **(b) Down tilts the head first, and crouches after a hold** (as long as
  the camera's delay, or shorter). A crouch to dodge comes later by that
  hold, and every crouch-keyed verb (crawl, drop through, crouch attacks)
  waits for it or keys on the down input instead.
* **(c) Down tilts the head, and the crouch moves to another input** (or to
  down while moving). The largest change: the crouch's verbs are re-bound.

**Owner:** the movement kernel's crouch (`update_body_mode`) and the pose
view (`next_gaze_secs`). Engineering for (b) or (c) is a queue row once ruled.

## Q172 — Do melee strikes and guards need swept (continuous) contact?

**Asked by:** the portal plan's remaining acceptance (PSIM, "swept guard
contact" and "moving-target hurt contact"), 2026-11-12.

**What exists:** projectiles and loose items meet moving targets on the
world clock, through portal cuts, scale and reflection (`b3c975837`,
`70c9fbf75`). A melee strike and a guard are tested by overlap once a tick,
in each body's admitted material through the cuts, and the deflect spark and
knockback read the midpoint of the two touching pieces. No shipped strike
has been seen to pass a guard or a body in one tick: a strike that reaches a
body's hurt parts also meets the guard in front of them that tick, and the
fastest measured limb (GNU-ton's portal blows, 41–60 world units a tick)
moves less than its own volume plus the smallest body.

**Options:**

* **(a) Keep overlap contact for melee** (now). The swept rules stay with
  projectiles and items, where a tick's travel exceeds the volume. The PSIM
  rows for melee CCD close as not required.
* **(b) Sweep melee strikes against hurt parts and guards** over the tick,
  with contact time and point, as projectiles do. The cost is one swept
  query per active strike and recipient; it changes no shipped outcome seen
  so far, and would matter for a faster or thinner strike authored later.

**Owner:** `ambition_combat::hitbox` (`apply_hitbox_damage`, `turned_aside`).
Engineering for (b) is a queue row once ruled.

## Q173 — Does the creator always die in the raid?

**Context (2026-11-12, INTRO-RAID, now LEVEL-1).** Jon: the creator follows the robot
into the second room, "where he dies, though he doesn't actually die now".
Built: he fights beside the robot (he can fall in the fight, 12 HP), and
when the robot reaches the corridor's far end his last words play and end
on a scripted `Fall`. Default taken: **he always dies there**; if he fell in
the fight already, the same words play as the ones he had left.

**Options:**

* **(a) Always, at the way out** (now).
* **(b) Only if he falls in the fight**; at the way out, a living creator
  says something else and stays behind (or goes another way).
* **(c) Never in the intro**; his death moves later in the story.

**Owner:** `data/cutscenes/intro.ron` (`creator_final_fragment`) and the
volume at the corridor's far end in `intro.ldtk`. Each option is content
only.

## Q174 — What does a death cost in Ambition?

**Context (2026-11-12, PLAYER-DEATH).** Jon asked for a presented death and
a rule for its cost, and named the options below; "it would be nice if we
could put a twist on it". Ambition's death beat is 0 s today and costs
nothing: the room goes back to the checkpoint. Default taken: **(b) with
(c)'s fee**: just before the blow lands, a portal opens and yoinks the
body away ("You're too valuable to be destroyed"); the gate network's
insurance charges a safety fee from the currency, and the fee is left at
the spot as a recoverable cache (a sealed parcel at the rescue portal's
mouth) that going back returns. Too little to pay: the bigger penalty is
that the parcel holds what was carried and the rescue lands at the last
checkpoint in debt (the next currency earned pays the debt first).

**Options:**

* **(a) Hollow Knight style:** a penalty undone by finding your way back to
  your body and fixing it.
* **(b) The rescue portal:** a portal yoinks you away before the death
  (now, with (c)'s fee).
* **(c) A lost currency you can win back,** with a bigger penalty when you
  die without enough to pay the safety fee.
* **(d) No cost:** the presentation only; the checkpoint restore is the
  penalty.

**Owner:** `session/death.rs` (the beat), the bag (currency) and the
presentation. Engineering is the PLAYER-DEATH row.

**Built under the default (2026-11-13):** the fee is 50 coins
(`room_rules.ron`, `death_fee`); the parcel lies at the last safe ground
where the body was taken; one parcel a participant (a second death before
the fetch loses the first, as Hollow Knight's shade); the debt is a negative
wallet of at most one fee. The rescue portal's own look is still to come.

## Q175 — Is the final boss's giant one of the named agents?

**Context (2026-11-12, FINAL-BOSS).** Jon: the bossofthegame stands on the
shoulders of a giant square robot with an electric guitar. The brainstorm
has an agent carry the Boss on its shoulders, and names the agents Gepetto,
Clause and Wren. Default taken: **(b)**, the giant is its own agent, **Full
Stack**, built like a guitar amplifier stack (he stands on the full stack).

**Options:**

* **(a) A named agent at its largest setting** (Gepetto, Clause or Wren,
  "extra high"), so the fight is that agent's temple boss too.
* **(b) Its own agent, Full Stack** (now).

**Update (2026-11-13).** Jon: "Add Gepetto, Clause and Wren to the fight",
with Full Stack kept. Taken: (b) stands, and the three named agents join
the fight beside the giant: the Boss calls Geppetto in as his second phase
opens and Clause and Wren as his encore does (`npc_geppetto`, `npc_clause`,
`npc_wren`, drawn from `assets/concept_art/misc/agent-concepts/`). Still
open: the spelling. The game uses the concept sheets' **Geppetto**; Jon's
note and the endgame brainstorm spell it **Gepetto**. One rename of a
catalog id, a renderer target and the Boss's bark either way.

**Owner:** `docs/planning/game/final-boss.md`; the catalog rows
`npc_full_stack`, `npc_geppetto`, `npc_clause`, `npc_wren` and their barks.
Each option is content only (a name, barks and art).

## Q176 — Does emptying the final boss's bar count as his voluntary retirement?

**Context (2026-11-12, FINAL-BOSS).** The endgame brainstorm: "the only
definitive in-game victory is getting the Boss to retire voluntarily (not
depleting a health bar)". The first cut is a boss fight with a bar. Default
taken: **(a)**, the bar is his **resolve**; when it empties he does not die,
he sits down and chooses to retire (the green room's scene).

**Options:**

* **(a) The bar is his resolve** (now): a worthy challenge is what makes him
  retire.
* **(b) A condition that is not damage**: for example, survive his encore
  for a time, or make him laugh, while the bar only gates the phases.
* **(c) Both**: the bar empties, and then a last beat that is not damage
  (he offers the crown; the player must take it or refuse it).

**Owner:** `boss_encounters/bossofthegame.ron`, the stage's rooms and the
cutscene library. (b) needs an encounter trigger that is not health (an
`External` gate); (a) and (c) are content only.

## Q177 — Does a living exploding mite go off?

**Context (2026-11-12, MATH-POLISH report 5).** Jon: "The exploding mites
should only trigger their explode fuse when they die. Right now they just
explode when I enter the room." The mite ran at its foe, planted a 0.9 s
fuse in reach and ended itself, and the blast came at that kill. Default
taken: **(a)**. Under both options the blast comes at the end of the death
row, which is the fuse.

**Options:**

* **(a) It never goes off alive** (now): it snaps with its mandibles
  (`exploding_mite_snap`); killed, its fuse lights and it goes off when the
  death row ends. Its threat is the corpse: kill it at range or step away.
* **(b) It still goes off alive**: in reach it ends itself at once (the
  `Detonate` move with a short plant), so its death row, the fuse, starts
  there and the blast follows.

**Owner:** `character_catalog.ron` (`npc_exploding_mite`, its action set).
(b) is content only: `melee: Some(Detonate(fuse_s: 0.15, reach_px: 40.0))`.

## Q178 — On a small screen, does the touch overlay give way to a menu?

**Context (2026-11-17, SMALL-VIEWPORT-UI).** The Smash select screen now
lays itself out at 640x360: six columns paged with arrows, START and the four
player cards (`capture_scene --route smash_select ... 640x360`). The touch
overlay still covers it. The movement stick never shrinks (its footprint is
210 px square at `TOUCH_SCALE` 0.7, which is 58% of a 360 px height). It stays
visible in Menu and Dialogue contexts on purpose, because it steers menu
cursors (`sync_touch_stick_visibility_from_context`). The action cluster shows
Confirm and Back as "Choose" buttons. At 640x360 the stick hides P1's card
and the page arrow, and the cluster sits over START and P4. If the menu kept
clear of both bottom corners, about 260 px would be left for four player
cards.

**Options:**

* **(a) A screen whose every control is tappable hides the stick and the
  action cluster** and keeps the menu row. Touch players tap cards, tokens
  and START directly (the select screen already takes taps). Menus that are
  not tappable keep the stick. This needs a declaration from the screen
  ("taken by direct touch").
* **(b) The overlay shrinks with the viewport**, the stick included, down to
  a touch-target floor. Every screen keeps both controls, and they are
  smaller on a phone.
* **(c) Each menu keeps clear of the published `ControlFootprints`**, and
  re-lays itself out in what is left. At 640x360 the select screen's player
  cards would no longer fit in one row.

**Owner:** `ambition_touch_input` (visibility and placement) and each menu's
layout. Engineering follows the ruling, screen by screen (queue row
SMALL-VIEWPORT-UI).

## Q179 — What does a quick press of Precision Blink do, if two quick presses are the directional blink?

**Context (2026-11-18, ABILITY-VARIANTS, [loadout open 2](game/ability-loadout.md#open)).**
Jon's notes: "a quick press does the intuitive thing, and two quick presses
give the directional blink". Today a quick press (released before
`BLINK_HOLD_THRESHOLD`) already is a directional blink: 190 px along the
movement stick, or forward along the facing with no stick
(`movement::control::handle_blink_clusters`). A hold aims (levels 2 and 3).
So the notes ask for a NEW single-press behaviour, and what "intuitive"
means is not in the notes. Contextual Blink (behind the nearest foe) is its
own row of the grid, so it is not the answer here.

**Options:**

* **(a) The quick press keeps the body's travel** (recommended default): it
  blinks 190 px along the body's current velocity when it moves, else along
  the facing, ignoring the stick; a double press within 0.25 s is the
  stick-directed blink of today. A running or falling body goes on the way
  it was going, and a deliberate direction takes a double press.
* **(b) The quick press finds a footing**: it blinks forward to the nearest
  place the body can stand within 190 px (a ledge ahead, the floor below a
  gap), else forward; the double press is the stick-directed blink.
* **(c) Keep the quick press as it is** (stick-directed) and give the double
  press a different meaning (for example the full 430 px precision distance
  along the stick, with no aim).

**Owner:** the movement kernel's blink control (`ambition_platformer2d_core`)
and the loadout plan. Engineering adds the double-press window as body
maneuver state (rollback, codec) whichever option is chosen.

## Q180 — What should the LDtk worlds' PlayerStart icon point at, so the player robot can publish without a sheet page?

**Context (2026-11-18, [PART-FIRST](queue.md#part-first--publish-characters-as-parts-not-frames---done-2026-11-20)).**
Every character with a part flipbook now publishes no baked sheet page; the
game draws it from parts. `player_robot_v3` cannot yet: every LDtk world in
`game/ambition_map_assets` declares the tileset `sprite_player_robot_v3` with
`relPath ../sprites/player_robot_v3_spritesheet.png` (3072×2468, 7.6 MP) only
so the editor can draw the `PlayerStart` entity, and `bevy_ecs_ldtk` loads
every declared tileset, so the runtime decodes that page at boot and a
missing page would fail the world load
(measured 2026-11-18: no level layer draws it).
Until this is answered, `authored/sprites.yaml` `baked_pages` keeps the robot
painted. Any target below is a mechanical rewrite of the tileset and its rects.

**Options:**

* **(a) The robot's portrait page** (recommended default):
  `../sprites/player_robot_v3_portraits.png`, with `PlayerStart`'s
  `tileRect` / `uiTileRect` on its neutral portrait cell. The editor shows the
  robot's face at the start; the page is already published at every tier.
* **(b) A small editor icon** published as a static prop (a `PlayerStart`
  marker drawn for the editor), so no character art is bound to the worlds.
* **(c) Keep the sheet page:** the robot stays painted on every publish
  (`baked_pages`) and the runtime keeps decoding 7.6 MP at boot.

**Owner:** the map submodule's worlds (Jon) and the part-first plan
([part-first publication](engine/part-first-publication.md)). After (a) or
(b), `player_robot_v3` leaves `baked_pages`.

**Assumed (a), 2026-11-20, reversible.** Every world that declared the sheet
(15 content worlds, the 3 Sanic demo worlds, and the `pulse_forge` and
`bridge_warden` example projects) now points `sprite_player_robot_v3` at
`../sprites/player_robot_v3_portraits.png` (2048×640, grid 256), and every
`PlayerStart` rect (definition, UI and each instance's cached `__tile`) frames
the page's `portrait` clip, (0, 320, 256, 320). `player_robot_v3` left
`baked_pages`; `library.json` embeds the portrait page and the robot's part
atlas in place of the sheet page. To choose (b) or (c) instead, retarget the
same 20 tilesets (or restore the `baked_pages` row); nothing else reads the
choice.

## Q181 — Should Sanic's roll be a commitment (Sonic's roll), or stay the run in a ball?

**Context (2026-11-19, [GROUNDED-ATTACK-MOMENTUM](queue.md#grounded-attack-momentum--measure-before-changing-the-slide---done-2026-11-19-q181-is-jons)).**
Measured in the demo ([table](engine/unified-movement-kernel.md#grounded-attack-momentum-grounded-attack-momentum-q169)):
Sanic's only roll is the spin dash, and while it rolls the body follows the
run's surface-momentum law in a smaller box. Hands off it loses 400 px/s²
(the run's `friction`), the stick speeds it up to top speed, and the stick
back brakes it at the run's `brake`. In classic Sonic a roll has half the
friction and the stick can only slow it, so a roll is a commitment: fast,
long, and hard to steer.

**Options:**

* **(a) Sonic's roll** (recommended default): two fields on the `spin_dash`
  row of `sanic_rules.ron`, a friction factor while `Rolling` (0.5) and input
  that only brakes; the kernel reads them as an effect on the body's
  `MomentumParams`, as the speed shoes do. Act routes that script a roll are
  re-measured.
* **(b) Keep the run in a ball:** the roll stays steerable at full speed.

**Owner:** the Sanic demo's feel (`game/ambition_demo_sanic`) and the
movement kernel's momentum law.

## Q182 — Where in the Nazi Dimension is the Supreme Leader fought?

**Context (2026-11-21, [the Supreme Leader](game/bosses.md#the-supreme-leader--total-control-built-2026-11-21)).**
His fight is built and playable in its own room, the War Room
(`hall_supreme_leader_war_room` in `evil.ldtk`), reached only from the Hall
of Bosses. Castle Godwin is not the dimension's capital, and the capital is
not designed yet. Until a story room exists the Hall copy is his one
placement (`hall.supreme_leader`): a real encounter, not a practice copy.

**Options:**

* **(a) Keep him Hall-only until the capital is designed** (recommended
  default): nothing changes; the War Room stays reachable only from the
  Hall.
* **(b) Name the capital and a route now:** a generator for the capital
  joins the War Room (or a story copy of it) to the dimension's rooms. The
  Hall copy becomes a practice copy at its own placement, as the other
  bosses' are.

**Owner:** the Nazi Dimension's map (`generators/war_room/`,
`generators/castle_godwin/`) and `docs/planning/game/bosses.md`.

## Q183 — Who is the Supreme Leader?

**Context (2026-11-21).** He is shown and spoken of only by his title. The
art, his barks and his docs settle no name and no personal history (his
relation to the regime's founding, how he came to lead it). The regime
fought the Gate Authority to a stalemate; that is the one fact the fight
uses.

**Options:**

* **(a) Keep the title only** (recommended default): "The Supreme Leader"
  on his card, in the catalog and in the Hall.
* **(b) Give him a name and a history:** the catalog's `display_name`, the
  encounter's `name`, his Hall barks and any dialogue change with it.

**Owner:** `character_catalog.ron` (`npc_supreme_leader`),
`boss_encounters/supreme_leader.ron`, `docs/planning/game/bosses.md`.

## Q185 — Should LEGAL, the address every terminal posts home, reach the Gate Authority's capital?

**Context (2026-11-25, [the Central Hub dimension](game/central-hub-dimension.md)).**
The lore puts the public network's centre in the Gate Authority's capital:
"the gate authority generally doesn't let anyone know any gate address
except for the location to the central hub". In the game today `LEGAL`
reaches the sandbox's central hub (`central_hub_complex`), which also holds
the halls, the gate stack and the developer wing, and a new game starts
there. The capital was begun as its own dimension, the Concourse at
`LICENSED`, and its terminal posts `LEGAL` like every other.

**Options:**

* **(a) Keep them apart** (recommended default): `LEGAL` stays the sandbox
  hub's; the capital keeps `LICENSED`. Nothing changes.
* **(b) Move home to the capital:** `LEGAL` goes to the Concourse ring and
  the sandbox hub's public ring takes another address. Every posted `LEGAL`
  then leads to the capital; the halls and the developer wing are reached
  from it (or only from the sandbox). Tests that dial `LEGAL` home follow.

**Owner:** `game/ambition_content/assets/data/gate_network.ron`,
`docs/planning/game/gate-network.md`,
`game/ambition_map_assets/ambition_content/generators/authority/`.
