# Dark rooms and tunnels

**State:** OPEN, planned 2026-11-14; DR1, TN2, DR2 and DR3 landed 2026-11-16. Asked by Jon on 2026-11-14: "we have to
start thinking about how to model "dark" or "dim" rooms with limited
visibilities and effectively tunnels. We don't have any rooms that are based
on tunnels, and tunnels in a metroidvania are fun and how we make good use of
vertical space."

The same message asked for the player's glow by quality tier. That part is
done (2026-11-14) and is the first piece of light this plan builds on.

## What exists

| Piece | What it does | Where |
|---|---|---|
| Actor glow | A light about a body, drawn by quality: Off (Potato), a mask sprite (Low, Medium), a dithered falloff shader (High), a lamp's falloff added to the scene (Ultra). The player's highlight is one | `ambition_render::rendering::actor_glow`, `actor_glow.wgsl`; the tier is `VisualQualityBudget::glow` |
| Fire glow | A quad of added light on each fire | Mary-O's `give_each_fire_its_glow` |
| Daylight veil | One translucent sprite over the world of a sunlit room, coloured by the hour. Rooms under the ground or indoors have none | `ambition_render::rendering::daylight` |
| Painted terrain | The `Terrain` IntGrid layer draws caves, overhangs and tunnels exactly (Sanic's acts use it) | the LDtk conversion |

What does not exist: darkness (nothing makes a room darker than its art),
lights that light a room, any tunnel room, and navigation over painted terrain
(`NavGraph` plans over blocks and `SurfaceChain`s only; see
[Hobford polish](../game/hobford-polish.md), "The land").

## Rules

- **Light is presentation.** No simulation fact reads light: rollback,
  checksums and the navigation graph never see it. A room that is hard to
  cross in the dark is hard to read, not mechanically different, so a
  replay and a peer agree whatever their quality.
- **Every tier can play every room.** Potato draws the darkness flat and
  caps it (no room is darker than half at Potato), so what must be seen
  (hazards, ledges, the way on) is still seen. Higher tiers add the light
  that makes it beautiful, not the information that makes it playable.
- **The place gives the darkness.** A tunnel is dark because it is under the
  hill; a cellar because its door is shut. No arbitrary "dark room" gate.

## Darkness and lights, by tier

A room states how dark it is: a level field `darkness` (0 none, 1 black) and
an `ambient` tint. Lights are a `LightSource` entity (radius, colour,
intensity, an optional flicker) placed in LDtk, and the actor glows; a held
item can carry a light (a lantern).

| Tier | Darkness | Lights |
|---|---|---|
| Potato | one flat veil, at most 0.5 | none drawn |
| Low, Medium | one veil sprite with a soft hole around the player (the player's light only) | the player's |
| High | a light map: a 160x90 texture over the view (computed on the CPU, drawn smooth), the ambient darkness less each light's falloff | every light in view |
| Ultra | the light map, and occlusion: a light does not pass through the room's solid ground (its solid cells, a light's reach marched over them), so a tunnel's bends cast shadow | every light, shadows |

The light map is the "real lighting" the Ultra glow points at: at Ultra the
player's glow becomes a light in the map, and lights the tunnel around it.

## Tunnels

Tunnels are rooms whose ground is around the player, not only under them:
winding passages under the hills, crawlspaces (`crouch`), sumps (`swim`),
chimneys (wall climb), and drops between levels. They are how the world uses
its height: down from the millstream under the village, up through the roots
of the woods' hollow oak.

What a tunnel needs:

1. **Geometry.** Blocks first (navigation already plans over them); then
   painted terrain once the graph plans over it (TN1 below).
2. **Camera.** Tight camera zones that frame the passage, zoomed in, so the
   darkness and the walls close in.
3. **Art.** An earth interior skin for the walls, timber props, roots,
   lamps (`LightSource`), an underground backdrop (not sunlit, so the daylight
   veil is already clear).
4. **Darkness.** The room's `darkness` (DR1).

## The first tunnel: under Hobford

Level 1 already names both ends: the cracked wall behind the mill wheel
(`intro_millstream`, a hollow) and the woods' hollow oak (`intro_woods`). A
tunnel joins them under the village: down behind the wheel, a passage of
old mill race and roots, up the oak's roots into the woods. It is a way, not
a gate: the street above is always open. Its reward is the shortcut and what
is down there (coins, a note).

## Order

- **DR1. Room darkness.** The `darkness` and `ambient` level fields; the veil
  (flat at Potato, with the player's hole at Low and Medium). Acceptance: a
  dark room at each tier, captured; Potato no darker than half.
  **Landed 2026-11-16.** `darkness` is a level field in percent (Int, 0 to
  100; `RoomVisualProfile::darkness_pct`) and `ambient` a `#rrggbb` colour
  (unset: a deep blue-black); the area tool writes both
  (`OPTIONAL_LEVEL_FIELDS`). The tier's `GlowBudget::darkness`
  (`DarknessQuality`) is `Flat` at Potato (one veil, capped at half) and
  `AroundThePlayer` above it: a soft round hole of light (radius 200, clear
  to a third of it) on the first player, and four flat pieces round it
  (`ambition_render::rendering::darkness`, `darken_the_room`, over the
  daylight veil, on its layer so no portal capture darkens a window twice).
  High and Ultra draw the hole veil until DR2's light map. The first dark
  rooms are the catacombs under the moor and the ossuary behind them
  (`evil_moor` generator, 70%). The percent is what the eye sees: the
  veil is blended in linear light, where an alpha of 0.7 took a bright
  skull from sRGB 211 only to 125, so the veil's alpha is
  `1 - (1 - pct)^2.2` (captured: 211 to 64 at 70%, to 108 at Potato's
  half).
- **DR2. The light map (High).** `LightSource`, the per-view target, actor
  glows and fire glows drawn into it. Acceptance: a tunnel lit by its lamps
  and the player; the cost measured (one target per view at half size).
  **Landed 2026-11-16.** `LightSource` is an
  LDtk entity (`radius`, `color` `#rrggbb`, `intensity`; in the entity
  contract), carried in the room spec (`RoomSpec::lights`, presentation
  only). At High and Ultra (`DarknessQuality::LightMap`) the veil is one
  sprite over the view (1.25 times it) whose 160x90 texture is the darkness
  left where the lights reach, each light the hole's falloff over only the
  texels it reaches, combined as `1 - prod(1 - light)`, then the perceptual
  alpha (a 256-entry table); computed on the CPU and drawn with linear
  filtering, redrawn only when the camera, a light or the darkness moved.
  Where a lamp's light fades the veil takes a third of its colour (the
  authored `color`): a warm rim, not a haze. That is not the GPU target this
  page planned: at 160x90 the CPU map needs no render-graph pass. Measured
  on this VM (release, six overlapping lamps of radius 260 over a 1600x900
  view): 0.28 ms a frame, 0.17 of it the colour pass every map pays; the
  first, unbounded version cost 1.03. Every player's light is in it, and
  the salt run's four lanterns (`hill_salt_run_lantern`) and the daylight
  down its well shaft; none past the well, as its sign says. Captured at
  High (lantern pools along the race, dark between) and Low (the player's
  hole only). Glows and fires: anything that carries `CastsLight` (radius,
  intensity, colour; `presentation::CastsLight`) is a light in the map, at
  its place while it shows. Each actor glow casts its own (its radius, hue
  and strength on the tick; today the player highlight, too faint to
  matter beside the player's light), and Mary-O's fires cast twice their
  halo in orange; a game's own lamp adds the component and names no
  renderer type. The lantern a player carries is TN3.
- **DR3. Occlusion (Ultra).** Lights stopped by solid blocks. Acceptance:
  a lamp round a corner leaves the far side dark; the frame cost measured.
  **Landed 2026-11-16.** Ultra's `DarknessQuality::ShadowedLightMap`. The
  room's solid ground becomes a grid of 16-unit cells once a room
  (`ambition_render::rendering::occlusion::SolidGrid`): solid blocks, solid
  outlines, painted earth and the earth under a filled ground line; a line
  a body only walks on stops no light. Each light's reach is marched along
  about one direction a cell round its rim (32 to 256), measured once a
  room for its lamps and each redraw for lights that move; a texel past the
  reach in its direction is dark, the face a cell deep lit, the edge a cell
  soft. A lamp hung into the ceiling shines from where it leaves the rock.
  The direction is a "diamond" turn, not an angle (no trigonometry per
  texel). Measured on this VM (release, six lamps of radius 260 over a
  1600x900 view): the map 0.38 ms unshadowed and 0.46 shadowed, a moving
  light's reach 0.03 ms more; the first per-texel `atan2` version cost
  0.81. Captured: the salt run's hatch at Ultra (the rock dark, the faces
  the lantern sees lit) beside High (light through the walls). Not done
  from the plan: the light map at full resolution at Ultra (160x90 there
  too).
- **TN1. Navigation over painted terrain.** The graph's surfaces from the
  `Terrain` layer, so a tunnel can be painted and still walked by NPCs and
  the census.
- **TN2. The first tunnel** (above), in blocks if TN1 is not done.
  **Landed 2026-11-16 in blocks** (TN1 is not done): `intro_salt_run`, THE
  SALT RUN, an old smugglers' passage (salt under the gate's toll), 6240
  wide under the millstream, the village and the woods; a hatch in the
  millstream hollow's floor and the gap between the hollow oak's roots are
  its doors (`presentation: SceneArt`, drawn as `hill_salt_run_hatch` and
  `hill_woods_hollow_oak`, which also draws the oak round its climb, which
  was boards in the air). Dark at 75%. Its lamps wait for DR2.
- **TN3. The lantern.** A held light that widens the player's light: the
  first item whose worth is seeing.
