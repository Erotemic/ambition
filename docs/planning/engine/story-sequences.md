# Story sequences: authoring scripted scenes without a recompile

**State:** SS1, SS2, SS4's first form and SS3's death record landed
2026-11-14 (below, "What landed"); SS3's `present_when` landed 2026-11-17
with its first customer, the game show's goat and car (below); SS5 landed
2026-11-14; SS6 is covered by SS1 and SS4 (below). Asked by Jon on 2026-11-14 after playing level 1: "do we have
an expressive enough way to codify these authored sequences that doesn't
require recompile of the engine? Because now that we are starting to author
story sequences, it likely will start to become more important."

This page answers that question from what the code has today, names the
gaps level 1 ran into, and proposes the order to close them. It follows the
rule of [authored gameplay logic](authored-gameplay-logic-and-orchestration.md):
no universal sequencer. Each domain that owns a kind of sequence keeps its
own cursor, and they all share one vocabulary of prepared conditions and
commands.

## What exists (all of it content: no engine build)

| Piece | Where it is authored | What it can do |
|---|---|---|
| Dialogue | Yarn (`assets/dialogue/**.yarn`) | branches on conditions (`condition("encounter.cleared", ...)`), choices, `<<wait>>`, variables, `<<command ...>>` with prepared commands, speakers and portraits. It reloads live (`content_watch::YarnSourceWatch`) |
| Cutscenes | `assets/data/cutscenes/*.ron` | a straight list of beats: `Dialogue`, `Wait`, `Fade`, `Banner`, `SetFlag`, `Fall` (a scripted death). `CameraPan` advances its timer and moves no camera (unfinished). A `seen_flag` plays it once |
| Triggers | LDtk entities (generators) | `CutsceneTrigger` (a volume; `after_encounter`), `EncounterTrigger`, `Switch` (`on_activate` command line), `LockWall` (`gated_by` condition) |
| Room lines | LDtk level fields | `entry_cutscene`; `while_live` (a command line asked each tick) and its condition `while_live_when` |
| Encounters | `assets/data/encounters/*.ron` | waves of spawns, each with a position and a delay; the encounter's own lock wall; a linked switch to restart it |
| Conditions and commands | registered by domains (`ConditionCatalog`, the command catalog) | prepared and checked at load: `world.flag_set`, `world.set_flag`, `encounter.start`, `encounter.cleared`, quest steps, and others |

So a scene like the workshop raid is all content today: the talk (Yarn)
sets a flag, the room's `while_live_when` lets its `while_live` line start
the raid, the encounter walks the raiders in, the bar breaks on a flag, and
the Creator's last words are a cutscene whose `Fall` beat kills him.

## Gaps level 1 ran into

1. **Actors cannot be directed.** No beat or command makes a character walk
   to a place, leave through a door, turn, take a pose or appear. The raid
   staging had to place the raiders where their own notice distance would
   bring them in through the door. Jon's note, "there are goblins that need to
   escape" (the raid's survivors running off with the case), cannot be said
   at all.
2. **A cutscene cannot use the shared vocabulary.** Its beats are its own
   list: it cannot run a prepared command (start an encounter, open a wall)
   or wait for a condition (a body arrived, a fight was won). Dialogue can do
   the first and not the second.
3. **A consequence does not stay in the world by itself** (a death does now;
   see SS3). The Creator's
   death is a flag and a fall in one cutscene. Nothing yet says "this NPC is
   present, or lying here, when this condition holds", so each lasting
   consequence needs its own handling.
4. **Triggers are volumes** (a cutscene can start on a condition now; see
   SS4). A scene that should follow an event (the raid
   won) has to be a volume that a body stands in when the event happens. The
   final fragment's volume was a strip by the door, and a fight carried Scout
   past it, so the Creator never died (fixed 2026-11-14: the volume is the
   whole workshop).
5. **The camera cannot be directed** (it can now; see SS5).

## Proposal, in order

Each step is content-facing vocabulary that a domain owns, prepared and
refused at load like the existing commands. None of them is a scripting
language.

- **SS1. Actor commands.** `actor.walk_to <character> <zone|point>`,
  `actor.leave_through <character> <zone>` (walk to an exit and go),
  `actor.face`, `actor.pose <character> <clip>` (on the existing animation
  override), and the condition `actor.arrived`. The navigation graph and the
  body's errand already walk a body to a point; these commands give that to
  content. Yarn can use them at once through `<<command>>`.
- **SS2. Cutscene beats over the shared vocabulary.** `Command { line }` runs
  a prepared command; `WaitFor { condition, seconds_at_most }` holds the beat
  until the condition is true. The cutscene keeps its own cursor, as now, so
  rollback and skip stay where they are. A skip applies the commands it
  passes, as it applies `SetFlag` and `Fall` today.
- **SS3. Lasting presence.** `NpcSpawn` takes `present_when` and
  `fallen_when` conditions, so the Creator lies where he fell on every later
  visit, and a villager who has left is gone. **First part landed
  2026-11-14:** the Creator's case needed no new field. An NPC placement's
  death was already kept (`DeadStaysDead`; `npc_fate` builds a recorded
  death lying down), but only the hit road wrote the record, so a
  cutscene's `Fall` was forgotten when the room was built again. A `Fall`
  now writes the same record under the body's own policy
  (`enemy_death_flag`). That makes `fallen_when` unnecessary for any death
  that happens in play. **`present_when` landed 2026-11-17** for its first
  customer: the goat and the car behind Monty's doors in Hylbert's Hotel
  (`math_hotel_door_1..3`), which were `LockWall` pictures and are actors
  now. An `NpcSpawn` takes `present_when: "world.flag_set <flag>"` (or
  `not ...`; anything else is refused by the converter). It is a plan-time
  filter beside the population cap (`RoomFeatureConstructionPlan::prepare`),
  so an absent NPC has no row, no root and no id, and it spends no cap. The
  flags come from the durable save: `ActorConstructionContext::world_flags`,
  stated by the door (`begin_room_transition_load_system`) and by a new
  session's first room (the candidate's own save, `build_candidate`). The
  plan keeps the answers it read, and the prefetch promotes a plan only
  while the save still gives them (`matches_presence`): with no flag
  changed the closet's prefetched plan is a hit; with the goat's flag set
  in the ballroom it is a miss, and the door prepares a fresh one
  (`capture_scene math_hotel_ballroom player --flag monty_open_1 --flag
  monty_goat_1 --player-at 384,488@40 --interact-at 70`). **Limits:** the
  answer is taken when the room is built, so a flag that changes while the
  player stands in the room changes nothing until it is built again; and
  the developer LDtk hot reload (`reload_ldtk_world_from_disk`) states no
  flags, so it rebuilds such a room as a new game would.
- **SS4. Event triggers.** A cutscene can start on a condition becoming true
  (`starts_when`), in a room or anywhere, without a volume.
- **SS5. The camera.** Finish `CameraPan` and add `CameraHold`. **Landed
  2026-11-14**, with the hold folded into the pan: a `CameraPan { target,
  seconds }` eases the camera to a point in the room (in the room's own
  pixels, as bodies are placed) and holds it there through the beats after
  it, so a line of dialogue does not snap the view back. A second pan goes
  from the first target, and `CameraRelease { seconds }` eases back to the
  view's follow. The end of a cutscene gives the camera back at once. The
  presentation is a pure function of the cutscene's place, like its fade
  (`CutsceneCamera`). The render camera applies it downstream of the
  resolved snapshot, as it applies the shake and the finishing zoom: the
  views of the cutscene's room only, kept inside the room, and with the
  follow still easing underneath, so a release lands where the player is
  now. Witnesses: `a_pan_holds_until_it_is_released` (ambition_cutscene),
  `a_cutscene_pan_frames_its_target_in_its_own_room` (ambition_render). In
  content: the raid's escape goes ahead of the raider to the yard gate.
  Not checked on screen (this VM has no display).
- **SS6. Retreat in an encounter.** An encounter field `retreat_at: <count>`:
  when that many of its mobs remain, they `leave_through` a named zone (with
  what they carry), and the encounter is won. That is the raid's survivors
  running off with the case. With radiant objectives
  ([RADIANT-NPC-HTN](../queue.md#radiant-npc-htn--authored-npc-lives-and-replaceable-planning))
  the same exit becomes an objective a goblin plans for. SS6 is the authored
  form that works before that. **Covered 2026-11-14 without a new field:**
  a retreat is a cutscene that starts on the count (`starts_when:
  "actor.fallen_is <mob> <n>"`) and runs `actor.leave_through` on the rest.
  The raid's escape is exactly that, and the encounter is won when the last
  one has gone. An encounter field would be a second spelling of the same
  thing; add one only if an encounter needs a retreat with no scene at all
  and the cutscene form proves awkward.

## What landed (2026-11-14)

- **SS1, actor commands** (`crates/ambition_platformer2d_actor_monolith/src/body/direction.rs`):
  `actor.walk_to <character> <place> <brain>`, `actor.leave_through
  <character> <zone> <brain>`, `actor.release <character>`, and the
  conditions `actor.arrived`, `actor.gone`, `actor.fallen_is <character> <n>` and `actor.count_is <character>
  <n>`. A body is sent with the lives' navigation (`Visit`); its brain is
  switched to the named preset through the one brain authority
  (`BrainCommand`), so content chooses the gait (`keeps_to_its_tasks` walks,
  `runs_where_sent` runs; a raw preset name is qualified in the
  character's provider). A body an encounter spawned has no brain binding;
  directing it binds it to its own character's policy first, which is what
  `actor.release` restores. A leaving body is sent to the point of its
  zone nearest it and goes through on touching it (an edge zone is a wall
  for every undriven body, so it never gets inside). A leaving body goes through its zone into the
  next room (live: it comes out there; not live: the ledger books it there
  when it keeps durable whereabouts, and it is despawned). Fallen bodies are
  neither directed nor counted. A body on its way somewhere is not held by a
  playing cutscene, so a scene can move its actors. `actor.face` and
  `actor.pose` landed later the same day: `actor.face <character>
  <left|right|other character>` turns the body once (a held body stays
  turned), and `actor.pose <character> <row> <loop|hold>` draws it with a
  named row of its sheet until `actor.release` (`Posed`, rollback state;
  `pin_poses` derives the drawn pin each tick, and a pose wins over a rest
  row). In content, the Creator turns to the escaping raider and points
  (`gesture`), and the scene lets him go at its end; the raid witness
  checks the pose while he runs and its end. The turn itself is not
  witnessed: his facing was not shown to change. `actor.hold <character>
  <held item>` (2026-11-17) puts a held item of the registry in each body's
  empty hand (a hand that holds something else is refused). It is the
  body's own, not a world object: it goes where the body goes and out of the
  world with it. The raid's last raider takes the Creator's case
  (`creator_case`) that way before he runs, and is drawn carrying it.
- **SS2, cutscene beats** (`crates/ambition_cutscene`):
  `Command { line }` runs a prepared command in the cutscene's room (a skip
  still runs it), `WaitFor { condition, seconds_at_most }` holds until the
  condition is true or for its longest hold. Every `Command`, `WaitFor` and
  `starts_when` line is prepared against the installed catalogs whenever the
  library changes (at start and at each reload) and refused there, naming
  the cutscene and beat (`CutsceneLineRefusals`, the log). A misspelt verb or
  condition is refused then; a misspelt character or zone is a name the
  catalogs cannot check without a room, and is refused when the command runs
  (logged).
- **SS4, first form:** `starts_when: Some("<condition line>")` on a script
  plays it for the whole world on the first tick its condition holds (it
  needs a seen flag).
- **The acceptance scene** (all content): `intro_raider_runs_with_the_case`
  starts at `actor.fallen_is npc_lab_raider 2` (not "one live": the raiders
  come in one at a time, so that is true as the first walks in), opens the yard gate (now gated
  on `intro_case_taken`; the Creator's last words set it too, for a raid whose
  last two fall together), sends the raider out through the lane edge with
  `runs_where_sent`, and waits for `actor.gone`. His going wins the raid.
  Witness: `game/ambition_app/tests/the_raids_last_raider_runs_with_the_case.rs`.
  The case itself is not yet an object he carries; the siding's crate as the
  case is SS3/SS6 work.
- **SS3, a death in a scene stays:** a cutscene `Fall` writes the death
  record a hit writes. Witness:
  `the_creator_lies_where_he_fell_when_scout_comes_back` (same file): his
  last words, out to the lane, back in; the workshop is built again, a new
  body, lying where he fell.

## Acceptance for the first slice (SS1 + SS2)

The raid's last raider, at one left, runs out of the barn door with the
case (a cutscene with `Command` beats using `actor.leave_through`). The
Creator's last words then play, and the siding's crate is the case the
goblins carried. It is written entirely in content files, with no engine
change after the slice lands. A misspelled actor or zone is refused when
the pack loads, naming the file.
