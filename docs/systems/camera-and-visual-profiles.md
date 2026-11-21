# Camera zones and room visual profiles

LDtk owns authored camera zones and room presentation choices. Conversion
lowers these fields into typed `CameraZoneSpec` and
`RoomMetadata::visual_profile` values; simulation geometry does not depend on
them.

## Camera compositions

A `CameraZone` has an activation box and a composition. Its activation box
selects the view. It does not have to be the box that limits the visible world.
Place these entities on an entity layer that permits `CameraZone`, such as
`AmbitionCameras` in Mary-o. Conversion adds them to `RoomSpec::camera_zones`.

The shared resolve has four steps:

1. Select regions from physical body positions. A jump anchor does not change
   this test. All bodies watched by a shared view can select a region.
2. Compose the target, zoom, look and bounds from the selected regions or a
   policy intent.
3. Move the view toward that composed goal on the presentation clock.
4. Apply visibility recovery and transition bounds. A room cutscene can then
   direct this same snapshot. The renderer consumes that director result.

| Field | Meaning |
| --- | --- |
| `id`, `name` | Stable ID (LDtk iid when absent) and descriptive name |
| `priority` | Higher priority wins; equal priorities use the ID in lexical order |
| `selection` | `exclusive` (default), `blend_x`, `blend_y`, or `blend_edge` |
| `sample` | `center` (default), `feet`, or `full_body` for activation |
| `enter_margin`, `exit_margin` | Entry inset and exit extension, in world units |
| `latched` | Keep an exclusive region until another region wins or the view resets; a continuous crossing can activate a thin region |
| `blend_width` | Positive spatial blend width, in world units; independent of zoom |
| `zoom` or `camera_zoom` | View multiplier; default `1.15` |
| `target_offset_x`, `target_offset_y` | Offset from the normal follow target |
| `lock_x`, `target_x`; `lock_y`, `target_y` | Fix one target axis |
| `cinematic_lock` or `lock_to_zone` | Use the activation box center as the target |
| `tracking` | `follow` (default) or `grounded` |
| `easing_hz` | Exponential response rate after composition |
| `transition_seconds` | Optional timed transition on region entry, from the current view |
| `cut` | Apply the new composition at once |
| `allow_look` | Allow held look input; default true |
| `clamp_mode` | `room_bounds` (default), `zone_bounds`, or `none` |
| `view_bounds` and `view_bounds_min_x/min_y/max_x/max_y` | Enable and set a viewing box separate from activation |
| `bounds_space` | `footprint` (default) limits the full view; `centers` limits only its center and permits a line or point |
| `fit` | `center` (default) centers an oversized view; `fit_view` can reduce its size to fit footprint bounds |
| `transition_bounds` and `transition_bounds_min_x/min_y/max_x/max_y` | Optional parent viewing box during movement between compositions |
| `visibility` | `recover` (default) protects subject visibility; `preserve_bounds` keeps the authored restriction |
| `scroll_policy` | `free` (default) or `forward_only_x` (`no_backtrack` alias) |

Portal continuity can apply its chart offset when no director owns the center.
Shake and finishing zoom remain presentation effects.

Timed transitions control the center. Zoom uses the presentation profile's
zoom-in and zoom-out rates. A cut applies both position and zoom at once.

Exclusive selection changes the desired composition at entry. Temporal response
moves the view to that goal. Spatial selection varies influence with position;
it must be requested explicitly. Regions at one priority share their influence. Spatial influence at a higher
priority covers the lower-priority composition; the remainder stays below it.
Blend X and Y increase from the activation box's minimum edge. Blend edge spans
each boundary, with half its width outside and half inside. Its midpoint is
the authored edge. The width is fixed in world units.

A footprint smaller than the view cannot contain it. `center` centers the view
on that axis; `fit_view` changes the view size instead. Visibility recovery can
relax a local composition to keep a subject visible. Use `preserve_bounds`
when the restriction is part of a discovery gesture. The final parent box still
limits the view. Author a parent box that contains the intended transition.

`grounded` holds the last supported focus during an ordinary jump. It resumes
follow during a fall or a large rise. This changes tracking only. Activation
always uses the physical body, including when a platform carries it upward.
Mary-o authors a low-priority grounded-follow region in each room. Local
compositions replace it. For a roof with a fixed composition, use an upper activation region and a Y
lock. Mary-o 1-2 uses this form: entry starts in the air, the transition runs
for a short duration, and later roof jumps retain the same vertical goal.

`forward_only_x` keeps the X target from moving backward during a visit. Leaving
the region clears its high-water mark. It does not restrict body movement.

## Native and scripted policy

`ambition_world_port::camera` defines typed `CameraPolicyInput` and
`CameraIntent` ports. The default policy is `ambition::camera/follow`. The
LDtk level field `camera_policy` can name another admitted entry. A native
entry, a Rhai entry, and a Rust-Wasm entry use the same contract and solver.

Input includes stable body IDs, live room identity, physical and presented
positions, body geometry, viewport and safe area, authored regions, continuity,
the previous presented center, and the active cutscene director. Output can
select region weights or supply target, axis locks, zoom, bounds, tracking,
visibility, response and transition settings. `honor_director` defaults to
true; an explicit policy can take control during a cutscene.

The host calls a policy once per view per rendered frame. `this.frame` and
`this.dt` use presentation time, including while gameplay is paused.
`this.tick` is absent in this domain. Simulation entries have a tick and no
presentation frame. A view record (`per view` in Rust, `attachment: "view"`
in Rhai) retires with the view. It is neither saved nor rewound. Room or
subject changes and placement reset it. Portal chart transit has a separate
continuity value and does not reset it on each frame. Code replacement resets
its records without replacing live gameplay.

Gameplay owners can install typed observations with
`ExtensionAppExt::install_view_policy_observation`. The supplier receives the
view entity and reads native state. The host collects these facts before any
view policy runs. A policy must declare each observation port it reads.
Discovery flags remain with their gameplay owner; a camera record stores only
presentation memory. Presentation entries can write only view records and can
submit only ports admitted in the camera phase.

A camera-only code reload does not change `ExtensionGeneration` or rebuild
the session. Keep camera code in its own artifact. A binary that also changes
simulation modules uses the normal mechanical reload path. Invalid code keeps
the admitted code. Invalid output commits neither view records nor intent; the
view retains its last valid intent and reports the fault.

`CameraPolicyState` exposes the last input, intent, frame and fault per view.
`CameraEaseState` exposes transition progress and the underlying follow center.
`ResolvedCameraSnapshot` exposes the composed goal and the presented center.
Use these values to distinguish selection from response in a headless trace.
See [iterating without a rebuild](../recipes/iterating-without-a-rebuild.md#camera-policies)
for runnable Rhai and Wasm examples.

## Room-level visual fields

The LDtk level's fields lower through
`ambition_platformer2d_ldtk/src/conversion/level_fields.rs`:

| Field | Meaning |
| --- | --- |
| `visual_profile` or `visual_profile_id` | Named presentation profile |
| `parallax_theme` | Explicit theme for background layers, terrain skin, and motes |
| `backdrop` | The background layers' theme alone, when it is not `parallax_theme`'s (the skin, dressing, doors and motes stay with `parallax_theme`) |
| `palette` | Named architectural look (`clean_corrupted` or `debug_beautiful`) |
| `lighting_hint`, `foreground_treatment` | Optional presentation hints |
| `sky_scroll` | Optional nonzero sky-scroll rate |
| `block_color` | Default block colour (`#RRGGBB` or `#RRGGBBAA`); an individual block's `color` overrides it |

**Scenery theme selection is explicit.**
`ParallaxTheme::named_by_room_metadata` reads only
`visual_profile.parallax_theme`. It does not infer a theme from `biome`,
`visual_profile.id`, `music_track`, or `visual_theme`.
`backdrop_of_room` gives the background's theme: the room's `backdrop` when it
names one, else its own theme, else `default`. A room without an
explicit theme can retain its own authored block art rather than acquiring
another game's terrain dressing. See
[parallax backgrounds](parallax-backgrounds.md).

A named `palette` selects the room look independently of the parallax theme.
For example, the two-state hub uses `palette=clean_corrupted` with
`parallax_theme=hub_clean`. See [room looks](room-looks.md).

## Camera look held by the player

`ambition_sim_view::camera_snapshot` applies a held gaze in the subject's
reference frame after framing and before room clamping. The current thresholds
are `LOOK_DELAY_SECS` (0.8 s), `LOOK_RAMP_SECS` (0.5 s) and
`LOOK_SHARE_OF_VIEW` (30% of visible height). The source owns those values;
do not duplicate gaze-time state in camera presentation.

## Authoring

Use the LDtk editor or `ambition_ldtk_tools` to set the fields. When a field
is absent from the LDtk project definitions, add its schema definition before
setting a value. The loader's tolerance for optional fields does not make an
undeclared editor field editable.
