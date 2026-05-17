# Camera zones and room visual profiles

This document records the authored-room contract for presentation data that is
spatial or room-scoped. The goal is to keep real gameplay rooms and the intro
room declarative: LDtk tells the sandbox what camera / art mood a room wants,
and runtime systems consume typed specs instead of inferring intent from names
or music ids.

## CameraZone entities

`CameraZone` entities on the `Ambition` layer lower into
`RoomSpec::camera_zones: Vec<CameraZoneSpec>`.

Supported fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | string | Stable zone id. Falls back to the LDtk entity iid if empty. |
| `name` | string | Display/debug name. |
| `priority` | int | Higher priority wins when zones overlap. Defaults to `0`. |
| `zoom` or `camera_zoom` | float | Requested camera zoom multiplier. Empty preserves the legacy `1.15` breath-out while inside the zone. |
| `target_offset_x` | float | World-space camera target X offset. Defaults to `0`. |
| `target_offset_y` | float | World-space camera target Y offset. Defaults to `0`. |
| `easing_hz` | float | Optional camera target easing speed while active. Defaults to the normal camera easing rate. |
| `cinematic_lock` or `lock_to_zone` | bool | When true, the camera targets the zone center instead of the player. |
| `clamp_mode` | string | `room_bounds` (default), `zone_bounds`, or `none`. |

When multiple zones overlap the player, the highest priority zone wins. Ties use
larger zone area as the stable tie-breaker. `CameraViewState` exposes both the
number of active zones and the winning zone id for HUD/debug readers.

Room transitions also reset the presentation-only camera target on active-room
change. This is a conservative mitigation for LDtk rooms connected by side doors
but placed far apart in world coordinates: the camera snaps to the new room's
first valid target instead of easing through unrelated space.

## RoomVisualProfile level metadata

LDtk level fields lower into `RoomMetadata::visual_profile`. These are optional;
older rooms keep working with the legacy `biome` / `visual_theme` fallback.

Supported level fields:

| Field | Meaning |
| --- | --- |
| `visual_profile` or `visual_profile_id` | Stable authored profile id, such as `intro_wakeup_room`. |
| `parallax_theme` | Explicit generated parallax/background theme. Prefer this over inferring from `biome`, `music_track`, or `visual_theme`. |
| `palette` | Palette / color-grading hint for future renderer passes. |
| `lighting_hint` | Lighting mood hint for future post-process or shader passes. |
| `foreground_treatment` | Foreground/atmosphere treatment hint. |

`ParallaxTheme::from_room_metadata` now checks `visual_profile.parallax_theme`
first, then `visual_profile.id`, and only then falls back to the legacy loose
metadata heuristic. New story rooms should set `parallax_theme` explicitly.

## LDtk schema note

The runtime parser tolerates these fields even when older LDtk files do not yet
have editor-visible field definitions. When updating the LDtk project schema,
add the fields above to the `CameraZone` entity definition and level-field list
so designers can edit them in the LDtk UI.
