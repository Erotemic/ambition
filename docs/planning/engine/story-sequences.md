# Story sequences: authoring scripted scenes without a recompile

**State:** SS1, SS2, SS4's first form and SS3's death record landed
2026-11-14 (below, "What landed"); SS3's `present_when` landed 2026-11-17
with its first customer, the game show's goat and car (below); SS5 landed
2026-11-14; SS6 is covered by SS1 and SS4 (below). **SS7–SS8 are planned**
(reviewed 2026-11-21 against production `b91b29d35935`): bind authored calls
to their actual participants, and distinguish skipping presentation from
completing gameplay work. Neither is implemented by this planning update.
Asked by Jon on 2026-11-14 after playing level 1: "do we have
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
| Cutscenes | `assets/data/cutscenes/*.ron` | a straight list of dialogue, waits, authored commands/conditions, flags and scripted deaths; camera pan/release, fade and banner presentation. A `seen_flag` plays it once |
| Triggers | LDtk entities (generators) | `CutsceneTrigger` (a volume; `after_encounter`), `EncounterTrigger`, `Switch` (`on_activate` command line), `LockWall` (`gated_by` condition) |
| Room lines | LDtk level fields | `entry_cutscene`; `while_live` (a command line asked each tick) and its condition `while_live_when` |
| Encounters | `assets/data/encounters/*.ron` | waves of spawns, each with a position and a delay; the encounter's own lock wall; a linked switch to restart it |
| Conditions and commands | registered by domains (`ConditionCatalog`, the command catalog) | prepared and checked at load: `world.flag_set`, `world.set_flag`, `encounter.start`, `encounter.cleared`, quest steps, and others |

So a scene like the workshop raid is all content today: the talk (Yarn)
sets a flag, the room's `while_live_when` lets its `while_live` line start
the raid, the encounter walks the raiders in, the bar breaks on a flag, and
the Creator's last words are a cutscene whose `Fall` beat kills him.

## Historical gaps from level 1 (2026-11-14)

These motivated SS1–SS6; the receipts below describe what subsequently
landed. Current work is SS7–SS8, after those receipts.

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

Each step is content-facing vocabulary that a domain owns. Static calls are
prepared against installed descriptors; live references and state have their
own binding/admission point (SS7). None needs a new scripting language.

- **SS1. Actor commands.** `actor.walk_to <character> <zone|point>`,
  `actor.leave_through <character> <zone>` (walk to an exit and go),
  `actor.face`, `actor.pose <character> <clip>` (on the existing animation
  override), and the condition `actor.arrived`. The navigation graph and the
  body's errand already walk a body to a point; these commands give that to
  content. Yarn can use them at once through `<<command>>`.
- **SS2. Cutscene beats over the shared vocabulary.** `Command { line }` runs
  a prepared command; `WaitFor { condition, seconds_at_most }` holds the beat
  until the condition is true. The cutscene keeps its own cursor and rollback
  state. Current skip emits remaining commands alongside flags and falls;
  SS8 replaces the assumption that this also completes their gameplay work.
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
  ([RADIANT-NPC-HTN](../queue.md#radiant-npc-htn--authored-npc-lives-and-replaceable-planning---done-2026-11-17))
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
  world with it. The raid's brute takes the Creator's case
  (`creator_case`) that way before he runs, and is drawn carrying it.
  2026-11-17 also: `actor.shove <character> <radius>` throws every other
  body in the room within the radius away from the character's;
  `actor.fight <character> <target>` sets each body on the nearest body of
  the target character as a provocation does (hostile, with its provoked
  mind and a grudge); `actor.throw <character> <target>` throws the item in
  its hand at the nearest body of the target, on an arc that lands there
  (a thrown bomb goes off as any thrown bomb); and the condition
  `actor.beaten <character>` holds while a body of the character stands at
  its `beaten_at` floor (a death trait: blows take it no lower, so a body
  that the story needs alive is beaten there rather than killed).
- **SS2, cutscene beats** (`crates/ambition_cutscene`):
  `Command { line }` runs a prepared command in the cutscene's room (current
  skip still queues it; the completion gap is SS8), `WaitFor { condition, seconds_at_most }` holds until the
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
- **The acceptance scene** (all content): `intro_brute_takes_the_case`
  (2026-11-17; it replaced the last raider running at two fallen, which
  left the Creator's death unexplained). The raid's second wave is a brute
  (`npc_raid_brute`, `beaten_at: 1`). The scene starts at `actor.beaten
  npc_raid_brute`: he roars and throws everyone back, goes to the bench, the
  Creator is set on him, and he throws a bomb that wounds the Creator (the
  Creator is `beaten_at: 1` too, so the blast cannot kill him before his
  last words). Then he takes the case, opens the yard gate
  (`intro_case_taken`; the Creator's last words set it too), runs out
  through the lane edge with `runs_where_sent`, and the scene waits for
  `actor.gone`. His going wins the raid.
  Witness: `game/ambition_app/tests/the_raid_brute_takes_the_case.rs` (the
  Creator's health falls when the bomb lands and stays above zero). The
  siding's crate as the case is SS3/SS6 work.
- **SS3, a death in a scene stays:** a cutscene `Fall` writes the death
  record a hit writes. Witness:
  `the_creator_lies_where_he_fell_when_scout_comes_back` (same file): his
  last words, out to the lane, back in; the workshop is built again, a new
  body, lying where he fell.

## Remaining work: bound calls and gameplay completion

### SS7 — bind roles once for an authored invocation

**Current source:** actor commands select bodies by character name through
[`direction::cast_in`](../../../crates/ambition_platformer2d_actor_monolith/src/body/direction.rs),
optionally in the command's room. `arrived`, `gone` and `beaten` independently
query the matching population without that room context. The
[`cutscene adapter`](../../../crates/ambition_platformer2d_runtime/src/cutscene.rs)
carries a room into commands, but its wait answers are keyed by raw condition
text. The same line cannot have independent answers for two bound scenes.

Yarn has the same missing binding stage: the
[`conversation instance`](../../../crates/ambition_conversation/src/instance.rs)
already identifies its initiator and talker, while the generic
[`command`](../../../crates/ambition_conversation/src/dialog/authored_commands.rs)
and [condition](../../../crates/ambition_conversation/src/dialog/authored_conditions.rs)
bridges refuse reference parameters and ask through character-name arguments.
The command bridge does not supply the conversation's room. Its refusal of
raw strings as identities is correct; the missing operation is binding an
authored role to an identity the caller already owns.

**Decision:** prepare a reusable call, then bind its invocation's explicit
roles and context. A character kind, a population query, a bound occurrence
and an operation issued to that occurrence are distinct. The shared
authored-call boundary carries the bindings/context into both commands and
conditions; cutscenes and conversations retain their own cursor, ledger and
authority. This page owns the first delivery through both frontends.

Use existing occurrence references and lexical construction bindings from
[reusable authored composition](reusable-authored-world-composition.md).
`AuthoredArg::Reference` currently carries SimId and native object ports use
ObjectRef: adapt explicitly through the existing live resolver at the domain
boundary, without creating a third lifetime identity. A room-local place
name remains a place; static content names remain content names. Role lookup
does not grant a command permission to mutate the object.

The first two customers are the Creator's
[`intro.yarn`](../../../game/ambition_content/assets/dialogue/sandbox/intro.yarn)
conversation and the brute's
[`intro.ron`](../../../game/ambition_content/assets/data/cutscenes/intro.ron)
choreography. Instead of repeatedly spelling `npc_creator`, the conversation
can address its talker and ask about that same body. A scene binds its Creator
and brute roles and uses those references throughout. An illustrative
`actor.arrived(@talker)` is a proposed binding meaning, not current syntax.

1. Prepare one individual reference parameter through the shared call
   boundary, and bind it from the existing conversation talker and one scene
   placement. A required unique binding refuses absent/ambiguous targets with
   source context. Do not silently choose the first body of that character.
2. Migrate the Creator dialogue and brute scene. Address condition answers by
   playing occurrence and prepared call/beat, deleting the raw-line answer
   conflation and repeated individual population selection. Preserve the
   narrative ledger and current command/action-reply runner.
3. Bind the doorway raiders as a cohort **after** their spawn milestone. They
   do not exist at scene start. State whether each group binding captures
   members at that point or is deliberately a live population query. Keep
   existing broadcast forms as explicit adapters with their old meaning.

Bindings are rollback-owned invocation state, resolved through current live
objects at the invocation cut. Retirement refuses the old reference; it must
not discover a replacement of the same character. Reconstruction of the same
occurrence retains its role. Reload follows the admitted-program lifetime;
it cannot silently rebind active roles by name. Binding identity never freezes
health, position, form, friendship or permission. Reuse the existing read
index; do not rescan/rebind every scene on every tick.

Two live rooms with the same script and character kinds must act and wait
independently. A form change must not hand a role to another body. Keep
population counts, nearest-enemy selection and intentional crowd commands as
queries. “Departed this room,” “retired,” and “no matching live body exists”
remain different questions: a directed actor can leave into another live room.
Likewise, observing an actor's current arrived Visit is not proof that a
particular requested walk completed; correlated completion belongs to the
existing [domain action-outcome protocol](extension-domain-contracts.md).

### SS8 — skip presentation; settle stated gameplay obligations

**Current source:** `CutsceneBeat::consequence` and `CutsceneRuntime::skip`
in [ambition_cutscene](../../../crates/ambition_cutscene/src/lib.rs) emit all
remaining Command beats together. The runtime adapter queues them, writes
the seen flag and removes the scene. `WaitFor` also advances identically on
satisfaction or timeout. These describe the implementation, not a guarantee
that skipping preserves its intended gameplay outcome.

The brute script exposes the distinction. `direction::send` applies a Visit
and returns `Done`; `leave_through_zones` later performs the actual transfer
or departure. `direction::throw` launches a held item with a 0.45–0.85 second
flight; item/contact/combat owners later determine its impact. Removing all
time issues walking, throwing, releasing and leaving together. A release can
remove the just-issued Visit; the throw starts from the current positions,
not the positions the skipped walk would have reached. The normal-play
[raid witness](../../../game/ambition_app/tests/the_raid_brute_takes_the_case.rs)
checks departure and a wounded but living Creator, not skip equivalence.

**Decision:** a skip ends optional presentation; it does not prove remaining
work completed. Keep the cutscene's existing cursor and explicit obligations
until their owning domains settle them. Immediate flag/fall consequences can
retain their simple path. A cosmetic delay can disappear; a dependency on
arrival, creation, departure or impact needs the relevant result. A deliberate
best-effort wait may time out and proceed. A required result must distinguish
satisfaction from refusal, an unanswerable condition and timeout, with an
explicit failure/abort policy; none silently becomes successful completion.
A flag's applied result establishes that fact, not completion of a population
change triggered by it. The doorway scene still needs the actual spawn/binding
milestone before it directs the new cohort.

The first operation owners are navigation/transfer for **leave through a
zone**, and held-item/contact for **throw**. Identify the specific result each
scene requires before adding vocabulary: command application, departure,
launch and impact differ. Use existing typed requests, action replies and
cancellation policies; do not add a generic completion enum or pending-work
registry. A fast settlement is available only if its domain supplies an
admitted operation with defined semantics. The scene may not infer a teleport,
damage event or inventory transfer by executing commands with time removed.

An already-launched projectile is committed work: ending the presentation
neither erases it nor launches it again. Cancel pending scene-owned work only
under its domain's policy. Release presentation/control contributions under
the [source-lifetime contract](actor-effects-and-modifier-composition.md),
without dropping an outstanding gameplay obligation or retaining a whole-room
presentation hold merely because work is pending. Do not advance the whole
World extra ticks to finish a skipped scene.

Delivery uses two shipped cases. For the brute, compare normal play and skip
before/after the throw and before departure against the stated story outcome;
define the result if the required movement or impact fails. For the doorway
scene, skip before `intro_raiders_coming` must preserve the dependency on
spawning/binding the raiders before directing them. An arbitrary longer delay
does not establish that dependency.

Delete the blanket skip-all-commands consequence rule and “seen on queue
emission” for scenes carrying outstanding gameplay work. Keep one authored
sequence interpretation; do not maintain a second skip script that must be
kept in sync. Rewind while pending, target retirement and skip after acceptance
must preserve the existing action/room-lifetime guarantees. Completion and
presentation-dismissal state must be distinguished where the seen flag gates
later story. This work adds neither a universal sequencer nor a global story
transaction.

## Acceptance for the first slice (SS1 + SS2)

The raid's last raider, at one left, runs out of the barn door with the
case (a cutscene with `Command` beats using `actor.leave_through`; met
2026-11-14, and since 2026-11-17 the brute does it, above). The
Creator's last words then play, and the siding's crate is the case the
goblins carried. It is written entirely in content files, with no engine
change after the slice lands. Static verb/argument errors are diagnosed when
the library loads. Current actor/zone names are checked at execution; SS7
adds explicit invocation binding and its diagnostics without claiming that
all live objects exist at pack-load time.
