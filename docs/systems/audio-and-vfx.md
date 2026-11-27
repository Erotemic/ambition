---
status: current
---

# Audio and VFX

Audio and visual effects consume simulation facts. Their providers can be
absent in headless mode without changing gameplay outcomes.

## Audio content and ownership

- Providers author named music, SFX cues, voiceprints and encounter bindings.
- `ambition_audio` owns App-local catalogs, selection, mixing, adaptive music,
  web unlock and playback.
- `ambition_sfx` owns shared emitter, spatial attenuation and message types.
- The host supplies settings and a device/backend when available.

The audio content schemas are in `ambition_audio::content_schema`:

| Schema | Purpose |
| --- | --- |
| `music_registry` | Rendered tracks and their asset requirements |
| `sfx_registry` | Procedural sound specifications |
| `music_cue_catalog` | Adaptive sections/layers/states plus room and encounter bindings |

Ambition's cue catalog is
`game/ambition_content/assets/audio/music_cues.ron`. Content preparation
validates references and required audio files. Generated audio is rendered and
published during authoring, not regenerated when the game starts.

## Adaptive music

An encounter binding selects music by fight phase or wave. A room binding
selects a state of a cue instead of starting its ordinary `music_track`. Rooms
sharing a cue retain playback when moving between bound states.

States on the same section change layer gains without restarting the track.
Switching sections starts the new section from its beginning. For a continuous
song spanning rooms or combat states, keep those states on the same looped
section and bind each room explicitly. An optional entry state may play a
one-shot section before the normal loop; an encounter can select a different
fight state. See [room music](../recipes/room-music.md).

## Spatial sound

`SfxEmitter` is either global or spatial. Spatial emitters describe a point,
circle or axis-aligned rectangle (`SfxShape`) and an inner/outer attenuation
range (`SfxFalloff`). A listener inside an extended source hears full volume;
gain falls toward zero beyond the inner radius and reaches zero at the outer
radius. World-space sources must use simulation coordinates, not screen
coordinates.

`SfxListeners` contains the local views' listener positions. The shared audio
output takes the **maximum** attenuation gain across listeners rather than
adding gains. A global sound needs no listener; `(0, 0)` is a valid world
position and must not stand in for global audio.

`SfxMix` carries effective user/environment gain. The voice mixer combines it
with spatial attenuation for each playing voice. Do not use a global Kira
channel-volume replacement for SFX, since it would override per-voice gains.
A loop may update its emitter without restarting, and moving listeners update
playing voices. Hosts without an output device may still calculate and record
these decisions for diagnostics.

## One-shot playback limits

The one-shot player limits each cue in each listening space. Sounds with
different cue IDs can share a limit when the emitter supplies a named
`SfxGroup`. Combat uses one group for impact sounds, including the different
materials in one crowd. The player considers audible requests first. In that
set, a higher priority wins, followed by the larger gain.

After a voice starts, its group waits 45 ms before another voice can start.
Each group can hold two voices at once. The one-shot player holds at most 24
voices across all groups. It discards excess requests before clip resolution.
The time limit uses real time, so combat hitstop does not extend it.

Combat uses the longest eligible hitstop on each tick. A room that already
has an impact hold can start a new hold after the current one ends.

## Visual effects

`ambition_vfx` provides presentation-neutral effect messages. Render/content
plugins turn those messages and read models into particles, trails, flashes,
shakes and provider-specific art. Hit/hurt geometry, shield state, movement,
projectile trajectories and action timing remain simulation authority.

Held rays can carry a set of game-defined damage type IDs. A jump carries
the types of the ray that caused it. The damage receiver publishes the types
with its decision. Ambition uses `ambition.electric` on Scout's lightning
and draws a shock on each body that takes the damage. The shock places
painted bone parts from the `shock_bones` atlas at the body's landmarks.
The atlas uses the same painters as the Barrow Skeleton. The character's
body kind and tags select whether the shock shows bones or only arcs.

## When adding an effect

1. Identify the semantic event or read model that already represents the
   gameplay fact. Prefer that over a second side-effect channel.
2. Author/register the provider's cue or art and publish generated assets.
3. Connect presentation to the existing consumer, preserving headless
   behavior and provider selection.
4. Verify the actual cue/visual in the affected scene. For browser audio,
   follow [web audio testing](../recipes/web-audio-manual-test.md).

Missing or degraded presentation must not alter gameplay, and one gameplay
transition should not cause duplicate sound/event emissions.
