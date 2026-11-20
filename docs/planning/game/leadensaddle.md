# The Frontier dimension: Dry Dock and Marshal Gallus (started 2026-11-20)

**Queue row:** [LEADENSADDLE](../queue.md#leadensaddle--the-frontier-dimension-and-marshal-gallus).
**Gate address:** `LEADENSADDLE` (Λ Ε Α Δ Ε Ν Σ Α Δ Δ Λ Ε, every letter on the ring).

Jon, 2026-11-20 (relayed by the coordinator): a robot chicken boss, "Foghorn
Leghorn meets DC's Cyborg": a middle ground between a scaled-up chicken and a
person with a chicken head, a big swaggering rooster-shaped cyborg with a
cowboy hat and other cowboy kit. A western biome for him to live in, with its
own environment and western music, a boss theme too. The feel is a cross of
Cowboy Bebop, Red Dead Redemption, Firefly and Space Dandy, with industry in
it and a spaceport-town look: a frontier town on a dusty world, with a
refinery, pipes and docked ships beside a saloon and a livery.

**Originality rule.** The named works are mood references only. Nothing here
copies their characters, names, likenesses, catchphrases, logos, melodies or
layouts. The boss has no voice, catchphrase or design of either reference
character. The music is original western writing.

## The names

| What | Name | Why |
| --- | --- | --- |
| The dimension | **the Frontier** (`frontier`, world file `frontier.ldtk`) | A frontier world at the edge of the network. |
| The region | **the Leaden Saddle** | A saddle is a low pass between two hills. This one is between two mesas, and the first survey found lead in its rock. The gate address is its name. |
| The town | **Dry Dock** | A spaceport in a desert: the ships dock on dry land, and the yard repairs them, as a dry dock does. |
| The saloon | **The Wet Whistle** | Where the town wets its whistle. The Marshal's voice is a steam whistle (below). |
| The boss's yard | **the Ore Corral** | The fenced ore yard of the refinery, where the ore carts stand. |
| The boss | **Marshal Gallus** | *Gallus gallus* is the chicken's Latin name, and it sounds like "gallows". |
| His card's title | *Cock of the Walk* | The old name for the strongest rooster of a yard. |

## The town and its rooms

The flat map is the pass, west to east. The town is in the low middle of the
saddle; the refinery climbs the east mesa; the corral is on a shelf of that
mesa, and the lookout is on its top. Outdoor rooms join by edges; the saloon
and the livery are interiors, so their ways in are doors
([level design guide](../../authoring/level-design-guide.md)).

```
                          door           door
                   [The Wet Whistle] [the livery]
                                |       |
 [Pad Three] ==== [      Main Street      ] ==== [the refinery] ==== [the Ore Corral] -grille-> [the lookout]
  the gate                                       (terraces up         (Marshal Gallus)          (after him)
                                                  the mesa)
```

`==` is an edge. The grille of the corral opens when the Marshal falls
(`boss.cleared frontier.marshal_gallus`).

| Room | What it is | Who |
| --- | --- | --- |
| `frontier_landing_pad` | Pad Three: the gate on the pad's apron, its terminal and dialer, the clerk's booth. A docked freighter stands on the next pad; stacked cargo boxes give a high way east. | Dolores Pike, the pad clerk. A gate-sick traveller. |
| `frontier_main_street` | The town's main street: a boardwalk under the shop awnings (a low tier), the roofs of the shops (a high tier), the water tower. The saloon door, the livery door. The Marshal's notices on a board. | Gus Bell (outside his livery), townsfolk. |
| `frontier_wet_whistle` | The saloon: the bar, the piano, the stage; the balcony is a tier up. | Mae Calloway (behind the bar), Elias Crane (a prospector), the piano player. |
| `frontier_livery` | The livery: stalls of hover-mules, a hayloft over them. | Gus Bell's daughter June, who shoes the mules. |
| `frontier_refinery` | The refinery on the flank of the east mesa: terraces cut into the slope, pipe racks, tanks, the cracking tower, catwalks. The Marshal's posse are here (chickens). | Ruth Harlan, the night-shift foreman. |
| `frontier_ore_corral` | The arena: a level yard on a shelf of the mesa, ore carts, the corral fence, the steam whistle on its stack. Two raised loading docks at the sides, one jump up. | Marshal Gallus. |
| `frontier_saddle_lookout` | The top of the east mesa, behind the corral's grille: the whole town below, the pads and the ships. The Marshal sits on an ore cart there, hoarse. The reward chest. | Marshal Gallus, beaten. |
| `hall_marshal_gallus_corral` | The Hall of Bosses' practice copy of the corral (placement `hall.marshal_gallus`). | |

Each room has a title and a blurb (two Room DebugLabels) that says where it
is and one fact about it.

### The look (biome `frontier`)

A new biome in the sprite renderer: a hot amber sky with a pale ringed planet
and two moons by day, the two mesas with the saddle between them, the town's
false fronts, a water tower, the refinery's stacks with a flare, and the
gantries of the pads with ships on them. The near layer has heat shimmer,
telegraph poles and dust. The ground is packed red earth over sandstone, with
boards for the boardwalk. Decor: barrels, a hitching rail, cacti, ore carts,
crates, pipe ends, a tumbleweed. The door is a plank door under a small
awning. Motes are blown dust.

## Marshal Gallus

**Who he is.** The Marshal of Dry Dock and the boss of its refinery. He says
he was a rooster of the old yard who was rebuilt with refinery parts after an
accident at the cracking tower; nobody in town knows if that is true. He made
himself marshal, and he writes the town's law as he likes it: his ordinances
are nailed to the board on Main Street and he quotes them by number. His voice
box is a steam whistle. He crows it at dawn, and the shift at the refinery
starts when he crows, so dawn is when the Marshal says it is. He is loud,
vain about his hat, slow to admit a mistake, and generous with other people's
money.

**The middle ground.** He is not a big chicken and not a man with a chicken's
head. He has a rooster's body plan: a deep keel of a chest, a short body tipped
up, a rooster's legs with backward knees and spurs, a fan of tail feathers. He
stands upright like a man, wide-legged, chest out. His wings are arms: the
left is a wing of real feathers whose long primaries close like fingers round
the grip of a revolver; the right is a machine arm (pistons, a three-fingered
claw of a hand). The right half of his face and his chest are plated.

| Part | What it is |
| --- | --- |
| Hat | A wide, flat-brimmed cowboy hat, sand-coloured, with a band of brass studs. The comb comes up through a slit in its crown. |
| Head | A red comb, half of it a serrated steel plate; red wattles; a yellow beak; a natural left eye; the right eye a round brass lens with a red light. |
| Neck | A red bandana. |
| Chest | A riveted chest plate in rust-red enamel, like the end of a boiler, with a brass marshal's star on it; a bandolier of brass cartridges across it. |
| Waist | A gun belt with a holster on the left hip. |
| Arms | Left: a feathered wing with a long-barrelled revolver. Right: a machine arm, with a six-chamber cylinder in the forearm (a hand cannon that turns). |
| Tail | Sickle feathers in dark green-black; among them, three curved exhaust pipes that puff smoke when he exerts himself. |
| Legs | Yellow scaled rooster legs; the right shin is a piston. Cowboy spurs (a rowel on a strap) on his heels, over his own spurs. |
| Palette | cream and wheat feathers, a rust-red chest, red comb, yellow legs, brass, gunmetal grey, a sand hat. |

**Size.** About two and a half times the robot's height: a body box of about
96 x 128 world units, drawn larger.

**His voice (dialogue).** Proclamations and ordinances, long boasts, plain
threats. He never uses the reference character's verbal habits. Sample beats
for the arena's entry:

> **Marshal Gallus:** Ordinance Four. No travellers past the refinery after the whistle.
> **Marshal Gallus:** I blew the whistle an hour ago. You did not hear it? Everybody hears it.
> **Marshal Gallus:** Hold still. This is the good hat.

## The fight

The encounter is `marshal_gallus` (`boss_profiles.ron`,
`boss_encounters/marshal_gallus.ron`), placed as `frontier.marshal_gallus`
in the corral and as `hall.marshal_gallus` in the Hall of Bosses' copy. His
resolve: 60 HP. Phase 2 at 65%, enrage at 30%. He walks the yard
(`AnchorSway` on the floor, no bob, a chase toward the player) and plants his
feet to shoot (`strike_speed_scale` 0.15). His body hurts to touch.

His moves are `Special` keys that the Rhai script
`game/ambition_content/rhai/marshal_gallus.rhai` performs (one entry per
move on the `boss_special` port), and one generic body strike:

| Move | Key | Tell | What it does | Dodge |
| --- | --- | --- | --- | --- |
| Quick draw | `gallus_quickdraw` | the `quickdraw` row; a ping at the muzzle. | Three shots, 0.14 s apart, each aimed where you are as it leaves. | Keep moving across the line. |
| Spur kick | `Strike("side_sweep")` | the `spur_kick` row. | A kick in front of him. | Keep away, or jump it. |
| Lasso | `gallus_lasso` | the `lasso` row; a ping where you stand. | The loop (a module entity, 88 x 80) drops on that spot; a body in it is thrown toward him, into the spur kick that follows. | Leave the spot. |
| Dynamite | `gallus_dynamite` | the `dynamite` row; a ping on the floor under you. | Three sticks lobbed to land round you (96 apart), each a burst (`splash_half_extent` 52). | Stand between the bursts, or on a catwalk. |
| Tumbleweeds | `gallus_tumbleweeds` | the `call` or `whistle` row; dust at both docks. | A pair of tumbleweeds bounds in from the docks' inner faces each second of the strike. | Jump each one, or stand on a dock. |
| Shift whistle | `gallus_whistle` | the `whistle` row; steam at his chest. | A ring of sound runs the floor both ways from his feet. | Jump it. |
| High noon | `gallus_high_noon` | the `high_noon` row; he marks where you stand and the mark pings until he fires. | Six shots down the line to the mark, 0.05 s apart. | Be off the line when the warning ends. |

Each move is classified in `boss_seeds.ron` (the quick draw a spread
volley, the dynamite a slam, the tumbleweeds projectile rain, the whistle a
sweep, the lasso and high noon beams), inside the bands those seeds already
measure.

Phases:

1. **Morning Shift (100% to 65%).** The quick draw, then by where you stand:
   the spur kick (near), the dynamite (far) or the draw again (above, on a
   catwalk).
2. **Overtime (65% to 30%).** The whistle, the tumbleweeds, the lasso into
   the spur kick, then the dynamite, the draw or the kick by where you stand.
3. **High Noon (30% to 0).** High noon, then the long rest (1.4 s, his
   reload) that is the punish window, then everything faster. His art for
   the change is the `enrage` row: the hat pushed back and the boiler glowing
   (the art does not knock the hat off).

**Defeat.** The sundown section of his theme plays, he falls (the `death`
row), and the corral's gate to the mesa top opens (`boss.cleared
frontier.marshal_gallus`); the chest he drops holds 300.

**The arena.** `frontier_ore_corral`, 1600 x 640: a level yard (floor 576), a
loading dock at each side one jump up (64), a catwalk over each a double jump
up (128 over the dock). The docks are where the tumbleweeds start, so they
are behind them.

**The posse.** Game chickens (`chicken`, `chicken_hen`) on the refinery
terraces. They do not join the fight in the corral.

### What the scripts can and cannot do

The scripted seams (`boss_special` in `technique_execution`) request
projectiles, effects, sounds and module entities here (the lasso's loop;
its `module_entity_tick` entry reads its touchers and requests
`launch_toucher`). They cannot request `held_damage_box`, `conducted_pose`,
`summon`, `burst`, `camera_shake` or `speech_bubble`: the runtime's script
alias table (`ambition_platformer2d_runtime::extension_composition`) does
not name them. So:

- the spur kick is the generic body strike of the boss pattern
  (`Strike("side_sweep")`), not a script;
- the whistle hurts with projectiles (rings along the floor), not a burst;
- the Marshal does not walk under script control. His walk is the pattern's
  `AnchorSway`.

If a later slice needs a scripted melee volume or a scripted walk, the
missing capability is a script alias for `held_damage_box` and
`conducted_pose` in the boss phases.

## The people

Ordinary names. The residents are made by the character generator
(`chargen`), with species heads where it suits a spaceport: a cat, a horse, a
dog, an alligator. A new chargen hat, `cowboy`, is added for them.

| Person | Species | Where | What they want |
| --- | --- | --- | --- |
| Dolores Pike | human | Pad Three, the clerk's booth | to log you in; to complain that the Marshal charges a toll on the gate and keeps it. |
| Gus Bell | horse | the livery, and its door | to rent you a hover-mule. He does not rent horses. |
| June Bell | horse | the livery | she shoes the mules, and she wants a new file. |
| Mae Calloway | cat | the saloon, behind the bar | to sell you a drink; she knows the Marshal's habits. |
| Elias Crane | alligator | the saloon | a prospector who pans the creek for lead; he talks too much. |
| Ruth Harlan | dog | the refinery | the night-shift foreman; the Marshal crows at four in the morning and the shift has to start. |
| Marshal Gallus | (the boss) | the corral; the lookout after | |

## The music

Two original cues, rendered by the music renderer with the full instrument
set (no General MIDI fallback; `optional: false` on every sampled
instrument), generated by `tools/ambition_music_renderer/dev/compose_leaden_saddle.py`.
The game binds their sections to the rooms and the fight
(`game/ambition_content/assets/audio/music_cues.ron`).

| Cue / section | Where | What it is |
| --- | --- | --- |
| `leaden_saddle` / `dry_dock` | the pad, Main Street, the livery | 96 BPM, A minor: a boom-chick acoustic guitar, upright bass, a clip-clop rim; the saddle motif (A C D E, G E D C D) on a fiddle and a trumpet; a Rhodes for the spaceport. |
| `leaden_saddle` / `wet_whistle` | the saloon | Stride upright piano, the saddle turned major, brushes, a walking bass. |
| `leaden_saddle` / `refinery_shift` | the refinery, the corral, the hall's copy | A heavy half-time kit, a pulse of effected strings (String Cyborgs), the saddle in the low brass, a twang guitar. |
| `leaden_saddle` / `mesa_top` | the mesa top | The guitar alone, a distant trumpet, strings. |
| `marshal_gallus_showdown` ("Cock of the Walk") | the fight | 132 BPM, E minor, on the crow motif (E E B, A G E): `standoff` (intro), `duel` (phases 1 and 2: a galloping snare, a twang ostinato, trumpets, a male choir), `high_noon` (the enrage, up a semitone), `sundown` (the defeat: the crow slowed into the saddle, ending on E major). |

## The gate

A gate (a triangle) on the apron of Pad Three. Zone `frontier_pad_gate`,
facility `dry_dock_pad` ("Dry Dock Pad Three", dimension `frontier`),
terminal `frontier_gate_terminal`, address `LEADENSADDLE`, posted `LEGAL`.
Not hidden. Its row in `gate_network.ron`, its portal in `gate_portals.ron`,
the world in `worlds.ron`, area cards ("Dry Dock", "The Refinery", "The
Mesa Top", each "The Leaden Saddle"; the corral shows the Marshal's own
card).

## Slices

1. This page and the queue row.
2. The boss's art: renderer target `marshal_gallus` (a paper doll, like the
   Gram Reaper's), stills checked.
3. The environment: the biome, the rooms (generator
   `game/ambition_map_assets/ambition_content/generators/frontier/make_specs.py`),
   the gate, the world.
4. The fight: catalog rows, profile, encounter, sheet rows, the Rhai script,
   the hall copy; played headless.
5. The music.
6. The people, their talk, signs, captures of the main rooms.

## Open

- The boss's art is a baked sheet. PART-FIRST
  ([part-first publication](../engine/part-first-publication.md)) may move it
  to parts later.
