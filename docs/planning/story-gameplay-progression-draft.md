# Story + gameplay progression rough draft

Status: rough planning draft, meant to guide the next playable slice. It should
change as rooms are implemented, tested, and made fun.

## Design target

Build a small, coherent Ambition slice where the story is not a cutscene pasted
on top of the platformer. Each beat should map to a room, a verb, a quest flag,
an NPC interaction, an encounter, an audio cue, or an authored visual motif.

The current repo already supports enough to make a playable draft with mostly
existing tools:

- LDtk-authored rooms, loading zones, NPC spawns, debug labels, switches,
  lock walls, chests, pickups, encounters, boss spawns, and camera zones.
- Dialogue nodes and choices, including intro-specific dialogue.
- Quest state, save flags, room-entered conditions, encounter-cleared
  conditions, boss-defeated conditions, and one post-quest dialogue redirect.
- Platformer verbs: jump, dash, wall tech, blink, fastfall, slash, pogo,
  shield/parry, projectile, body modes, water, ladders, moving platforms, and
  breakables.
- Generated or manifest-driven sprites, backgrounds, SFX, and music.

The next slice should spend most of its engineering effort on playable rooms
and quest/progression hooks. New sprites and tunes matter, but they can ship as
rough generated placeholders until the mechanics prove the scene.

## Working premise

The player wakes in a basement lab as a small embodied AI. Two violent factions
raid the wrong lab while pursuing a larger target. The creator dies before
explaining the actual question the player was built to ask. The player escapes
through maintenance spaces under an interdimensional transit system, learns that
humans use stable gates while small AIs can exploit unstable ripples, and then
lands in The Kernel: a hub where movement labs, faction spaces, and bosses are
both gameplay tests and story evidence.

Alice and Bob become the first recurring quest pair: they are not just crypto
jokes, but the game's first example of trust, communication, interception, and
collaboration as mechanics. Their quest should teach how Ambition handles
non-combat objectives while still being a platformer.

## Progression spine

### 0. Wake room: embodiment before exposition

Existing anchor:

- `intro_wake_room`
- Creator NPC with `creator_intro`
- Diagnostic cart, neural console, genesis vat props
- Door/edge transition into `intro_raid_corridor`

Gameplay goal:

- Teach basic movement by giving the player space to move rather than a modal
  tutorial.
- Let the player ignore or talk to the Creator.
- Establish that observation and motion are both data.

Story goal:

- The Creator is not a grand wizard; he is a tired researcher in a bad basement
  who realizes something is wrong.
- The player is not told they are special. The player is only told to move.

Graphical direction:

- Cold basement lab, bright diagnostic equipment, cart rails, unstable lights.
- The player sprite should feel small and newly assembled.
- Props can remain placeholders, but the room should read as a basement lab at
  silhouette scale.

Music/audio direction:

- Sparse hum, low fluorescent buzz, light startup arpeggio as control begins.
- No full heroic theme yet.

Implementation hooks:

- Existing dialogue and prop rendering are enough.
- Optional polish: add a simple `first_move` or `wake_room_exited` flag for
  later quest memory.

### 1. Raid corridor: the wrong-list incident

Existing anchor:

- `intro_raid_corridor`
- `Framebreaker` enemy, `Nazi Salvage Guard` enemy. (NOTE: The Flamebreaker name is not set in stone), and we might change the other faction so Nazis are introduced later.

- Creator Final NPC and fast/impossible dialogue variants
- Debug labels already plant the manifest/wrong-list joke

Gameplay goal:

- Turn basic movement into pressure: run, jump, slash, and keep moving.
- Let speed change how much of the Creator's final line the player hears.
- Keep the room readable enough that a new player can clear it without mastery,
  but reward speedrunners with extra story fragments.

Story goal:

- The factions are not symmetric. Both are dangerous here, but the authoritarian
  salvage force is explicitly evil and bureaucratic; the Framebreakers are
  violent anti-machine hardliners driven by fear and grievance.
- The lab was not the actual target. The victim is retroactively filed into the
  target list because institutions prefer valid paperwork to truth.

Graphical direction:

- Broken lab corridor, alarm strips, smoke, signs that contradict each other.
- Faction silhouettes should be readable even with placeholder sprites: boots
  and lists for the salvage force, improvised iron tools for Framebreakers.

Music/audio direction:

- Alarm pulse enters, percussion is uneven and anxious.
- Creator line should duck music slightly so the player understands it matters.

Implementation hooks:

- Existing enemies, dialogue, and cutscene variants can carry a rough version.
- Needed to complete the room: a progression check or cutscene timer that
  selects normal/fast/impossible Creator variants reliably.

### 2. Escape shaft: movement as the only answer

Existing anchor:

- `intro_escape_shaft`
- Service corridor labels
- Edge transition to `drain_alley`

Gameplay goal:

- A low-combat movement space after the raid.
- Use climbable/wall/jump verbs, simple hazards, and maybe a rebound pad if the
  room needs an explicit movement lesson.

Story goal:

- The player leaves the lab before understanding what happened.
- The environment becomes infrastructure rather than home.

Graphical direction:

- Pipes, shafts, cable trays, wet concrete, maintenance signs.

Music/audio direction:

- Alarm fades into industrial rhythm; water and pipe knocks enter.

Implementation hooks:

- Mostly LDtk collision and labels.
- Optional: add a one-way route so returning later feels like sequence-breaking
  rather than normal commute.

### 3. Drain Alley: first social contact

Existing anchor:

- `drain_alley`
- Oiler NPC with `oiler_intro`
- News Board with `news_board_lab_incident`
- Signs about drain market, gates, swordguns, ripples

Gameplay goal:

- Slow down after the escape.
- Let Oiler repair or frame the first ability upgrade.
- Let the News Board reframe the incident with dark institutional humor.

Story goal:

- Oiler becomes the first non-creator human who treats the player as a person
  without needing a philosophy lecture.
- The board reveals the world can edit truth after the fact.

Graphical direction:

- Market-under-transit vibe: pipes, cheap food stalls, warning signs, steam.
- Oiler should be practical, grease-stained, and visually distinct from lab
  authority.

Music/audio direction:

- First warm motif. Street percussion, pipe clanks, soft bass.
- Oiler can have a small wrench hit/stinger.

Implementation hooks:

- Existing NPC/dialogue is enough for a rough pass.
- Good first unlock candidate: repaired dash, repaired blink, or a named
  traversal calibration. If all verbs remain unlocked for sandbox convenience,
  present the unlock as a quest flag/label first and enforce later.

### 4. Gate Stack: roads versus cracks

Existing anchor:

- `gate_stack_lower`
- Gate Janitor with `gate_janitor_ripple`
- Manifest Clerk with `manifest_kiosk_wrong_list`
- Portal switch and `intro_portal_zone` back to `central_hub_complex`
- Signs about delayed gates, Nazi Dimension service, tolls, and shark traffic

Gameplay goal:

- Teach that stable gates are doors, but ripples are traversal opportunities.
- Make the player do a small action chain: talk/read, toggle gate power, use the
  unstable route or portal.
- Introduce the idea that some destinations are legal but limited, while the
  player can reach unofficial cracks.

Story goal:

- Humans use stable gates. AIs can fit through ripples.
- The player is not an authorized traveler; they are an edge case the transit
  system does not know how to classify.

Graphical direction:

- Big stable gate rings above/behind small unstable shimmer cracks.
- The current gate ring/portal sprites can carry the first pass.

Music/audio direction:

- Transit station ambience: bells, delay chimes, crowd-like filtered noise.
- Ripple should have a delicate unstable synth cue, not the same sound as a
  stable gate.

Implementation hooks:

- Existing switches/loading zones can approximate the gate power chain.
- Needed to complete the room: a reusable `Ripple`/small portal entity, or a
  switch-gated loading zone with special visuals and audio.

### 5. The Kernel: hub as playable index

Existing anchor:

- `central_hub_main` and `central_hub_basement`
- Kernel Guide NPC, basement labs, many doors to prototype areas
- Feature rooms for hazards, enemies, boss, breakables, treasure, NPCs, mob lab,
  water, crawl, morph, ladder, cutscene, quest, switch

Gameplay goal:

- Turn the hub into a playable table of contents.
- The player can follow the main quest, but the room arrangement should also
  invite experimentation.
- Each lab teaches one mechanic and produces one story or quest consequence.

Story goal:

- The Kernel is not just a debug hub. It is the maintenance layer of a world
  where mechanics, law, memory, and story are inspectable.
- If the player is an AI, debug is a sense organ rather than a cheat.

Graphical direction:

- Stitched world: hub above, maintenance basement below, visible routing labels
  in early builds.
- Keep debug labels diegetic where possible: signage, terminal displays,
  handwritten maintenance notes.

Music/audio direction:

- Hub loop should be comfortable and expandable, with stems that can add lab,
  transit, or boss hints.

Implementation hooks:

- Existing rooms and dialogue already support this.
- Needed to complete the story version: reorder or gate some doors so the next
  slice has a deliberate path rather than a pure playground.

## Alice/Bob cryptography-gang quest

This should be the next story/gameplay slice after the intro/Gate Stack path.
Alice and Bob should be recurring characters who begin as a joke about crypto
examples and become the first emotional long arc about communication, consent,
interception, and trust.

### Quest 1: Handshake

Premise:

- Alice and Bob are separated by incompatible gate routes after the wrong-list
  incident. They can talk only if the player carries a challenge and response
  through spaces that do not preserve identity cleanly.

Gameplay shape with existing tools:

1. Alice NPC gives the player a `challenge` flag or quest step.
2. Player traverses a small room that tests one movement verb.
3. Bob NPC reads the challenge and gives a `response` flag.
4. Returning to Alice completes the quest and opens a route.

Existing hooks:

- NPC dialogue choices can set flags with a small extension, or the first draft
  can use room-entered/chest/switch flags as stand-ins.
- Quest conditions can sequence `FlagSet`, `RoomEntered`, and later
  `EncounterCleared` steps.
- Lock walls and loading zones can represent routes that open after the
  handshake.

Placeholder implementation:

- Put Alice in a small Gate Stack side office or Kernel terminal alcove.
- Put Bob in a nearby relay room reachable through one movement test.
- Use debug labels for `challenge`, `response`, and `verified` until item/UI
  work exists.

Needed to complete:

- Alice sprite and Bob sprite.
- A simple quest item/message payload representation, even if it is just a save
  flag plus inventory text.
- A short `handshake_theme` or motif that can later recur in love-story routes.

### Quest 2: Eve listens

Premise:

- Eve is not automatically evil; she is a listener, archivist, packet sniffer,
  or reporter. The player learns that communication has observers, and that
  observers can be benign, exploitative, or both.

Gameplay shape:

- A room with projectile or moving-platform timing where the player must carry
  a message without letting interceptors touch them.
- If touched, the message is not lost, but it becomes `observed`, altering
  later dialogue.

Existing hooks:

- Damage volumes or projectiles can stand in for interceptors.
- Save flags can record `message_observed`.
- Dialogue can branch later once flag-gated dialogue is supported.

Needed to complete:

- Flag-gated dialogue variants beyond the current specific pirate redirect.
- Lightweight non-damaging contact trigger, or use damage volumes as placeholder
  and phrase it as noisy interference.

### Quest 3: Mallory modifies

Premise:

- Mallory actively tampers with messages. This introduces a mechanic where the
  player must verify path integrity, not merely survive traversal.

Gameplay shape:

- A short puzzle-combat room with two routes: fast but tampered, slow but
  verifiable.
- The player can still proceed after tampering, but Alice/Bob's relationship
  changes because the message content was altered.

Existing hooks:

- Switches, lock walls, and multiple loading zones can emulate route choice.
- Quest flags can record `tampered_route_taken` versus `verified_route_taken`.

Needed to complete:

- More general conditional quest/dialogue consequences.
- A compact route-integrity UI or terminal display.

### Quest 4: Key exchange becomes movement

Premise:

- Alice and Bob stop treating the player as a courier and teach the player a
  traversal verb: not because cryptography is magic, but because protocol is a
  way to coordinate safely across hostile space.

Gameplay reward candidates:

- `Blink Key`: blink through a soft wall only after completing a challenge /
  response in the room.
- `One-Time Pad`: one safe traversal through a dangerous field; consumed or
  refreshed at terminals.
- `Public Key Door`: anyone can send power into it, but only the matching route
  can open it.
- `Commit-Reveal Platform`: step on a switch to commit, then reach another
  place before revealing to align moving platforms.

Mostly-existing first pass:

- Use soft/hard blink walls, switches, lock walls, and moving platforms.
- Represent key state as labels/flags, not a full UI.

Long-term payoff:

- Alice and Bob's relationship can become a love story, a friendship, a failed
  trust route, or a professional partnership depending on whether the player
  preserved, leaked, or modified their messages.

## Main-slice quest chain

This is the rough sequence to build first.

| Step | Beat | Existing room(s) | Gameplay verb | Progress hook | Asset/music need |
|---|---|---|---|---|---|
| 1 | Wake and move | `intro_wake_room` | basic movement, interact | optional `wake_room_exited` flag | lab hum, creator/lab props |
| 2 | Escape the raid | `intro_raid_corridor` | run, jump, slash | Creator final variant | raid percussion, faction silhouettes |
| 3 | Leave through service spaces | `intro_escape_shaft` | climb/wall/jump | room entered | industrial transition loop |
| 4 | Meet Oiler | `drain_alley` | talk, optional repair | `met_oiler` / ability flag | Oiler sprite/tune motif |
| 5 | Learn gates vs ripples | `gate_stack_lower` | switch + portal/ripple | `first_ripple_used` | ripple SFX, gate motif |
| 6 | Enter The Kernel | `central_hub_main` | hub routing | `entered_kernel` | hub loop |
| 7 | Alice/Bob handshake | new side rooms near Gate Stack/Kernel | courier movement test | `alice_bob_handshake_done` | Alice/Bob sprites, handshake motif |
| 8 | Clear first combat lab | `mob_lab` or `basement_enemies` | encounter combat | `EncounterCleared` | encounter stem |
| 9 | Defeat first prototype boss | `basement_boss` | boss pattern reading | `BossDefeated(clockwork_warden)` | boss theme |
| 10 | Choose first faction side path | pirate/ninja/military/etc. | focused movement + dialogue | route flag | faction sprites/tunes |

## Room-completion checklist

A room is complete enough for the draft if it answers these questions:

1. What verb does the player practice here?
2. What new fact about the world does the player learn here?
3. What state changes when the room is cleared, entered, or revisited?
4. What visual motif makes the room recognizable at thumbnail scale?
5. What musical or SFX cue tells the player the room belongs to this arc?
6. What placeholder is acceptable now, and what asset/mechanic is needed later?

## Minimum new systems that would help

Avoid building all of these before the slice is playable. These are the likely
places where placeholders will start to hurt.

1. General flag-gated dialogue variants, not just one-off redirect code.
2. A reusable `Ripple` entity: small portal, can be locked/unlocked, has distinct
   visual/audio treatment, can set `first_ripple_used`.
3. Message/quest payload display for courier quests: challenge, response,
   observed, tampered, verified.
4. Ability gating that can be enabled for story mode while sandbox/debug mode
   keeps all verbs available.
5. Route-choice consequences in quest state: not a morality meter, just durable
   facts about what happened.

## Tone rules

- Keep the opening playable and heavy, then let Oiler and the Gate Stack bring
  warmth and absurdity back.
- Do not make authoritarian logic sound reasonable. Make it bureaucratic,
  cowardly, cruel, and absurd.
- Do not let the player being an AI solve ethics. The world should treat
  uncertainty as the central pressure.
- The crypto gang should start funny and become emotionally useful. Alice and
  Bob are examples in textbooks, but in Ambition they should eventually become
  people whose trust the player can protect or damage.
- Every lore reveal should correspond to a new movement route, quest state,
  room transform, boss unlock, or dialogue consequence.

## Suggested next development slice

1. Create rough Alice and Bob sprites.
2. Add two small LDtk rooms near Gate Stack or The Kernel: Alice Relay and Bob
   Relay.
3. Add a three-step `alice_bob_handshake` quest using placeholder flags.
4. Add dialogue for Alice, Bob, and optionally Eve as a silent observer terminal.
5. Add one new generated tune: `handshake_motif`, sparse and warm.
6. Add a placeholder `challenge/response` visual using debug labels or terminal
   props.
7. Validate that the quest appears in the inventory quest log and advances
   across save/load.
