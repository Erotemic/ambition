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
| `frontier_refinery` | The refinery on the flank of the east mesa: terraces cut into the slope, pipe racks, tanks, the cracking tower, catwalks. The Marshal's posse are here (chickens). | Ruth Okafor, the night-shift foreman. |
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

The encounter is `marshal_gallus` (profile and encounter rows, placement
`frontier.marshal_gallus`). His resolve: 64 HP. Thresholds: phase 2 at 60%,
enrage at 25%. He walks the yard (`AnchorSway` on the floor, no bob, a chase
toward the player). His body hurts to touch.

His moves are `Special` keys that a Rhai script performs
(`game/ambition_content/rhai/marshal_gallus.rhai`, `boss_special` triggers),
and two generic body strikes:

| Move | Key | Tell | What it does | Dodge |
| --- | --- | --- | --- | --- |
| Quick draw | `gallus_quickdraw` | He drops his hand to the holster; a click. | Three shots aimed at the player, one after the other. | Move across the line, or jump. |
| Spur kick | `Strike("side_sweep")` | He rears back on one leg. | A wide kick in front of him. | Keep away, or jump over it. |
| Lasso | `gallus_lasso` | He twirls a loop over his head; a ring shows where you stand. | The loop drops on that spot. A body in it is yanked to his feet. | Leave the ring. |
| Dynamite | `gallus_dynamite` | He lights three sticks (a fuse spark). | Three sticks in arcs; each bursts where it lands. | Stand between the bursts. |
| Tumbleweeds | `gallus_tumbleweeds` | He whistles a short note; dust at the far wall. | Burning tumbleweeds bounce across the floor from the far wall. | Jump each one, or stand on a dock. |
| Steam whistle | `gallus_whistle` | He puffs his chest; the plates shake. | He crows. A ring of steam bursts round him and hurts at close range. | Keep out of his reach. |
| High noon | `gallus_high_noon` | He steps back and squares up; a bell tolls twelve times (a tell of 1.6 s). | Six shots across the yard, high and low by turn. | Duck the high ones, jump the low ones. |
| Reload | (a `Rest` after high noon) | | He breaks open the cylinder; empty shells fall. | The long punish window. |

Phases:

1. **Morning Shift (100% to 60%).** Quick draw, spur kick when you are near,
   lasso when you are far. Each move is followed by a rest that is long
   enough to hit him.
2. **Overtime (60% to 25%).** Adds dynamite and the tumbleweeds; the steam
   whistle comes when you stand next to him. Shorter rests.
3. **High Noon (25% to 0).** He loses his hat in the change (the hat is
   blown off by his own whistle), and the high noon volley joins the cycle,
   always followed by the reload.

**Defeat.** His whistle cracks. He drops to one knee, and the grille of the
corral opens (`boss.cleared frontier.marshal_gallus`). On the lookout he sits
on an ore cart and talks in a hoarse voice; without the whistle, the shift
starts at sunrise.

**The arena.** `frontier_ore_corral`, about 1600 x 640: a level floor, a
loading dock at each side one jump up (64), a high catwalk over each dock a
double jump up (128 over the dock), the steam whistle's stack at the back.
The docks are where the tumbleweeds cannot reach. The camera holds the yard.

**The posse.** Chickens (`chicken`, `chicken_hen`) on the refinery terraces.
They are his flock. They do not join the fight in the corral.

### What the scripts can and cannot do

The scripted seams (`boss_special` in `technique_execution`) can request
projectiles, effects, sounds, blasts, drawn rows, module entities (the lasso's
loop; its tick reads its touchers and requests `launch_toucher`) and body
velocity. They cannot request `held_damage_box`, `conducted_pose`,
`summon`, `burst`, `camera_shake` or `speech_bubble`: the runtime's script
alias table (`ambition_platformer2d_runtime::extension_composition::script_ports`)
does not name them. So:

- the spur kick and the stomp use the generic body strikes of the boss
  pattern (`Strike(...)`), not a script;
- the whistle's hurt ring is a `blast`, and its steam is effects;
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
| Ruth Okafor | dog | the refinery | the night-shift foreman; the Marshal crows at four in the morning and the shift has to start. |
| Marshal Gallus | (the boss) | the corral; the lookout after | |

## The music

Four original cues, rendered by the music renderer with the full instrument
set (no General MIDI fallback; `optional: false` on every sampled instrument).

| Cue | Where | What it is |
| --- | --- | --- |
| `leaden_saddle` | the pad, Main Street, the livery, the lookout | The town: a slow shuffle; a fingerpicked acoustic guitar, a harmonica, whistling, an upright bass, brushes, an anvil on the off beat far back. |
| `wet_whistle` | the saloon | A honky-tonk upright piano with a banjo and a fiddle. |
| `refinery_shift` | the refinery | A steady train beat; a baritone guitar with tremolo, low strings, metal percussion (anvil, pipes), a harmonica that wails over it. |
| `marshal_gallus_showdown` | the fight | A boss theme in sections (intro, phase 1, phase 2, enrage): a solo trumpet over a whistled call, a choir's "ah", a tremolo guitar, timpani and anvils, a bell on high noon. |

## The gate

A gate (a triangle) on the apron of Pad Three. Zone `frontier_pad_gate`,
facility `dry_dock_pad` ("Dry Dock Pad Three", dimension `frontier`),
terminal `frontier_gate_terminal`, address `LEADENSADDLE`, posted `LEGAL`.
Not hidden. Its row in `gate_network.ron`, its portal in `gate_portals.ron`,
the world in `worlds.ron`, an area card ("Dry Dock" / "The Frontier
Dimension"; the corral "The Ore Corral").

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
