# Camera zones and room visual profiles

LDtk owns authored camera zones and room presentation choices. Conversion
lowers these fields into typed `CameraZoneSpec` and
`RoomMetadata::visual_profile` values; simulation geometry does not depend on
them.

## Camera zones

Place a `CameraZone` entity on the `Ambition` layer. Conversion adds it to
`RoomSpec::camera_zones`. The supported fields are:

| Field | Meaning |
| --- | --- |
| `id`, `name` | Stable ID (LDtk iid when absent) and descriptive name |
| `priority` | Higher priority wins overlapping zones; ties choose the larger zone |
| `zoom` or `camera_zoom` | Zoom multiplier; when omitted, the zone's default is `1.15` |
| `target_offset_x`, `target_offset_y` | Offset from the normal camera target in world units |
| `easing_hz` | Optional easing-rate override |
| `cinematic_lock` or `lock_to_zone` | Target the zone center instead of the followed subject |
| `clamp_mode` | `room_bounds` (default), `zone_bounds`, or `none` |
| `scroll_policy` | `free` (default) or `forward_only_x` (`no_backtrack` alias) |

`forward_only_x` keeps the camera's X target from moving backward while the
zone remains active. Its high-water mark resets when the zone is left. It
does not itself stop the player walking left; a game's level rules handle that.
The implementation lives in
`ambition_platformer2d_world::rooms::camera` and the LDtk entity converter.

`CameraViewState` exposes the selected zone and overlap count for debugging.
On active-room changes, presentation resets the easing target instead of
panning through the space separating LDtk room coordinates.

## Room-level visual fields

The LDtk level's fields lower through
`ambition_platformer2d_ldtk/src/conversion/level_fields.rs`:

| Field | Meaning |
| --- | --- |
| `visual_profile` or `visual_profile_id` | Named presentation profile |
| `parallax_theme` | Explicit theme for background layers, terrain skin, and motes |
| `palette` | Named architectural look (`clean_corrupted` or `debug_beautiful`) |
| `lighting_hint`, `foreground_treatment` | Optional presentation hints |
| `sky_scroll` | Optional nonzero sky-scroll rate |
| `block_color` | Default block colour (`#RRGGBB` or `#RRGGBBAA`); an individual block's `color` overrides it |

**Scenery theme selection is explicit.**
`ParallaxTheme::named_by_room_metadata` reads only
`visual_profile.parallax_theme`. It does not infer a theme from `biome`,
`visual_profile.id`, `music_track`, or `visual_theme`.
`from_room_metadata` returns `default` when no theme is named. A room without an
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
