# Level design guide

Rules for laying out rooms and staging scenes in them. They apply to every
generator (`game/ambition_map_assets/ambition_content/generators/<dim>/`)
and to rooms edited in LDtk. The rulings are Jon's; the date says when.

## Moving between rooms

**Edges join connected rooms; doors lead into interiors** (Jon,
2026-11-14). When two rooms are parts of one connected place (a lane and a
meadow, a street and the next street, the floor of a cave and its next
chamber), the way between them is an edge loading zone: the player walks
off one side of a room and arrives at the matching side of the next. A door
(`activation: "Door"`, a pressed exit) is for going *into* something: a
house, a shop, the barn, a station hut, a cave mouth seen as an entrance.

- An edge keeps the world continuous. The rooms sit beside each other on
  the flat map as they are walked, and LDtk placement follows from that.
- A door is a change of space, and the player should see what it opens
  into from the outside (a building, a hatch, a stair head).
- A stair, a ladder or a path down a slope between two outdoor rooms is an
  edge on the room's top or bottom, or a side edge at the level where the
  path leaves, not a door.

## Calling things by their shape

The gates of the gate network are **triangles**: call each one a *gate*
(the Hobford gate, the gate clearing), never a "ring" (Jon, 2026-11-14).
This is about player-facing text: signs, dialogue, room titles and zone
names. Engine identifiers that say `ring` (`RingKind`, `GateRingPositions`)
are a separate rename.

## Staging a scene

Place things so that the camera and the dialogue show the scene without
being steered (Jon, 2026-11-14).

- **Nothing important appears from nowhere.** Enemies who arrive walk in
  through a door or off an edge the player can see. They are not spawned
  in the middle of the room.
- **Frame the scene by placement, not by panning.** Put the speakers, and
  the door the interruption comes through, close enough together that the
  camera already holds all of them when the moment comes. A camera that
  pans over to find the arrival is a sign that the room was laid out
  without the scene in mind.
- **Frame an interior with a camera zone.** A `CameraZone` with
  `clamp_mode: "zone_bounds"` over a building keeps the camera inside it, so
  whatever waits outside (raiders in the yard) is out of shot until it comes
  through the door. The workshop's barn is the example.
- **Arrivals start within notice distance.** An enemy notices from its own
  brain profile's `aggro_radius` (the lab raider's is 460 px), so an arrival
  that must walk in starts no farther than that from where the player
  stands, and out of the frame.
- **Give dialogue time.** A line that starts before the previous beat has
  landed, or a fight that starts under a line still being read, is a
  pacing defect. Each beat finishes before the next begins.
- **No soft locks.** A one-way device (a barrier, a gate, a lever-held
  door) must be operable from every side the player can reach, or stay
  open once opened. Check where a player who drops, falls or arrives from
  another door ends up.

## Backgrounds

**A landmark is in the background or the foreground, never both** (Jon,
2026-11-14). A parallax background that paints a distant landmark (the gate
on its hill above Hobford) is right for the rooms that see it from afar.
The room where the landmark stands in the play plane uses a background
without it, so the player never sees the gate twice at once. Give such a
room its own look (a variant of the region's background) rather than
reusing the region's.
