# Dark rooms and tunnels

**State:** DONE 2026-11-16 (planned 2026-11-14): DR1, DR2, DR3, TN1, TN2 and TN3 landed. Left for a customer: per-item pickup art (the salt run's Lantern draws as the generic ability gem, so its hanging lanterns read as the thing to take), and a tunnel painted in `Terrain` (the salt run is blocks). Done 2026-11-19 ([below](#per-pixel-light-and-the-towns-night-2026-11-19)): the light at full resolution, and the same lights in a sunlit room at night. Asked by Jon on 2026-11-14: "we have to
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

What did not exist when this was planned: darkness (nothing made a room
darker than its art), lights that light a room, any tunnel room, and
navigation over painted terrain (`NavGraph` planned over blocks and
`SurfaceChain` slopes only; see [Hobford polish](../game/hobford-polish.md),
"The land"). All four exist now (below).

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
  **Landed 2026-11-16.** A still chain's flat is a standing surface when it
  has ground under it (painted earth or a solid outline just below, or a
  filled ground line) and no earth over it (a cave roof's underside is no
  floor); painted earth and solid outlines take room from a body as blocks
  do. The parts of a chain between its flats are its slope pieces, and a
  slope leg walks a piece from one surface to the next, as it walked a
  whole slope chain before (`navigation::surfaces::chain_walks`). A bare
  authored line stays a line: counting the Hanoi room's 64 stacked disk
  tops took its graph from 23 ms to 4.9 s (walker) and 9.6 s (dog), so a
  flat needs ground under it. Witnesses: `a_body_routes_over_painted_ground`
  (a painted outline with its earth, a roof over it; travelled both ways)
  and `the_navigation_graph_walks_painted_ground` (ground painted cell by
  cell and lowered by the loader). The census of the shipped rooms
  (`nav_graph_census`, 339 rooms, two bodies) is unchanged in surfaces,
  links and groups but for the four Hobford hill rooms, whose slope legs now
  start where a filled line's flat ends (each route about a second
  cheaper).
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
  **Landed 2026-11-16, as a key item, not a held one** (a held item takes
  the hand a weapon needs; the light is the point, not the holding). The
  Lantern is `Item::Lantern` (the Sealed Note's old slot, `lantern`), found
  by the salt run's hatch (a `PickupSpawn` `ability:lantern`, kept once
  taken). The engine has a `PlayerLight` (`presentation::PlayerLight`:
  each player's light radius and colour, default 200 and the room's own
  dark), read by the hole veil and the light map; Ambition sets it while
  the bag holds the Lantern (`ambition_content::presentation::lantern`:
  360, warm), so the renderer names no item. The pickup draws as the shared
  item orb (pickups have three looks; per-item pickup art is not done).
  The salt run's ways out are now its smugglers' ladders up to a trapdoor
  (`hill_salt_run_ladder`, `SceneArt`), not doors in the rock.

## Per-pixel light and the town's night (2026-11-19)

Jon, 2026-11-19, on the salt run: "when I saw lighting in the cave it was
very blocky". The light map was 160x90 texels over the view at every tier,
about 12 world units a texel, and a shadow's edge falls off over one 16-unit
cell: stretched over the screen, every rock edge was a staircase.

- **The light is summed per pixel on the GPU at High and Ultra**
  (`ambition_render::rendering::lamps`, `lamps.wgsl`; the dark room's
  `DarknessMaterial`, `darkness.wgsl`). The lights in view (at most 64, the
  nearest the view's centre past that) are a uniform; at Ultra each light's
  reach along its directions (`SolidGrid::reach`, unchanged) is a row of an
  R32 float texture, read with two loads a pixel and interpolated between
  directions as `lit_part` does. The sum is the CPU light map's
  (`lamp_texels`), so the look is the one DR2 and DR3 tuned, at the
  screen's resolution. The reach rows are written into the texture the GPU
  already has (`ReachWrites`, a render-world `write_texture`): changing the
  image asset made a new texture each frame a light moved, and the material
  drew nothing while it bound it again (measured: the night veil vanished
  at Ultra). Where no 2D material can be built (no renderer, Android), the
  CPU light map is drawn as before.
- **A sunlit room's lamps at night** (Jon, 2026-11-19, on the Math Town
  night filter: "Is there an \"ultra\" quality shader that could give a much
  more realistic effect?"). The daylight veil multiplies the world by the
  hour's light; at High and Ultra it also reads the room's `LightSource`s and
  what casts light, and where they reach multiplies by their colour instead,
  never darker than the hour, falling off from the source with no flat core.
  Over it an added glow (`lamp_glow.wgsl`, `LampGlow`) makes each source
  brighter than its paint, as strong as the hour is dark: a lit window
  shines. By day the lamps change nothing and are not gathered. Lower tiers
  keep the hour's colour. At Ultra the lamps' light stops at the room's
  ground, as in a dark room.
- **The Math Town street's lights** (`generators/math_town/make_specs.py`,
  `HOUSE_LIGHTS`, `CLIFF_HOME_LIGHTS`, `BAY_LAMP`): 36 `LightSource`s on the
  windows the facade art paints (the cafe all lit, the observatory dim and
  red), the cliff homes' lit windows, and each passage bay's vault lamp and
  window. `math.ldtk` gained the `LightSource` definition. Captured at hour 23:
  Medium is the moonlit multiply; High and Ultra light the street's windows
  and walls round them warm.
- **The salt run's lanterns** (Jon, 2026-11-19: "it wasn't possible to
  navigate to the cage the way you had it next to the water, which is just
  weird placement"): two of the smugglers' lanterns hung over the middle of
  the mill race, out of any reach, and read as something to take. They hang
  now over the dry floor at either bank (`LAMPS`), a little over a body's
  head.

