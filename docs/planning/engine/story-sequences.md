# Story sequences: authoring scripted scenes without a recompile

**State:** SS1, SS2, SS4's first form and SS3's death record landed
2026-11-14 (below, "What landed"); SS3's `present_when` landed 2026-11-17
with its first customer, the game show's goat and car (below); SS5 landed
2026-11-14; SS6 is covered by SS1 and SS4 (below). **SS7 and SS8 landed
2026-11-21** (reviewed that day against production `b91b29d35935`):
authored calls bind to their actual participants, and a skip ends the
presentation without completing the gameplay work for it. SS7 step 1 is
conversations (below, "SS7 progress"), step 2 scene roles and the brute
scene, step 3 the doorway scene. SS8 is below ("SS8 landed"). The open
items each records are listed with it.
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
  state. A skip runs the remaining beats in order and still waits on each
  `WaitFor` (SS8, landed 2026-11-21).
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
  `Command { line }` runs a prepared command in the cutscene's room (a skip
  still runs it, in order; SS8), `WaitFor { condition, seconds_at_most }` holds until the
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

#### Own each admitted playback before binding its roles

**Source follow-up at `f41218979625`, 2026-11-21:** an explicit cutscene
request is currently answered from the presence of its script id and room,
not from the run admitted for that request. This is a source-derived failure
sequence; no new game run or timing measurement was made in this review.

[`lower_cutscenes`](../../../crates/ambition_platformer2d_runtime/src/story_script.rs)
queues a trigger and separately stores `CutsceneAsk { id, room, ticket,
began }`. `answer_cutscene_asks` scans active and queued definitions. Presence
means started; absence after `began` means ended. The
[`queue`](../../../crates/ambition_cutscene/src/lib.rs)
coalesces identical waiting triggers, but does not join active playback.
The observer runs after queue drain and before playback tick.

For a repeatable script S in room R, let request A be playing when request B
arrives. B queues a later run, but the observer sees A and reports B started.
After A ends, queue drain starts B before the observer runs again. A's reply
then remains pending through B's run. The queue asks for one operation while
the observer answers for another. Reordering the observer does not restore
the missing identity.

**Decision:** the cutscene owner assigns an occurrence to each admitted run.
Pending and active playback retain it. Admission identifies which occurrence
the caller created or joined; owner transitions establish its progress.
Reuse that same occurrence for SS7 role/beat bindings and SS8 obligations.

| Value | Meaning and policy |
| --- | --- |
| Script definition | Reusable content. It does not identify one run. |
| Trigger coalescing key | Existing policy for equivalent waiting demand, such as entry and volume triggers for the same script and room. |
| Playback occurrence | One admitted run through queueing, start and terminal disposition. Its owner retains it through rewind. |
| Action ticket | One caller's request and reply destination. Multiple tickets can follow one occurrence only because admission joins them to it. |
| Seen flag | Durable narrative policy. It is neither a playback identity nor evidence that this caller's operation started. |

Preserve waiting-only coalescing in the first slice. A request for S/R while
A is active joins an already waiting S/R occurrence or creates one. It does
not implicitly join A. Native entry/volume producers can keep the inexpensive
fire-and-forget adapter; it uses the same admission. A queued occurrence that
is later passed over by the seen check reports not-played for its own
interested tickets, never completion inferred from an earlier run.

Repeatable uses of one definition must be independent in different rooms.
The current
[`drain_cutscene_triggers`](../../../crates/ambition_platformer2d_runtime/src/cutscene.rs)
adds global `active.plays(id)` exclusion to room capacity. Remove that
definition exclusion for repeatable scripts. Preserve it for scripts with a
seen flag in this slice: otherwise two rooms can start the same globally
once-only scene before either writes the flag. Keep room capacity and
world-scene exclusivity. Do not introduce a new reservation policy across
different definitions that happen to share a flag.

Emit start and terminal facts at the actual playback transitions. The runtime
bridge binds existing
[`ActionReply`/`ActionMailbox`](../../../crates/ambition_platformer2d_shared_tangle/src/action_results.rs)
destinations to the admitted occurrence. Keep `CutsceneAsks` as bounded
caller-owned occurrence/ticket associations, without `began`. A narrow
runtime bridge can publish into the existing native action journal at start,
natural end, skip and queue refusal; a returned domain transition value is
enough where borrowing needs separation. Keep reply types and extension host
dependencies out of the pure cutscene owner. No new event bus or persistent
transition journal is needed. Prune retired ticket associations on admission
and transition so that removing polling does not leave the ask limit full.

Preserve ordered start and end facts when an empty scene does both in one
tick. Recording end at its actual transition removes today's extra observer
tick. Deliver through the established later action-read cut; never execute
script handlers inline. Existing action journals carry delivery through
replay; a non-rewinding message drain is not a replacement for that contract.

This is semantic state, not an address cache. Full snapshots already encode
queue and playback, while room restoration through `CutscenesInRooms`
currently restores active cutscene state. The occurrence allocation state,
pending queue membership and reply associations must follow the same room
rewind as their playback. A new process-wide counter, or a counter restored
only on a whole-world rewind, would couple independent room histories. Use
the existing room/session occurrence scope and deterministic owner-local
sequence state, or an allocation rule with the same demonstrated properties.
Keep world-scoped playback explicit. A reply from an old run must not reach
a replacement caller with the same authored name.

Extend the existing room owner log/retractor, or use registered state on its
room root; adding a key only to the active-playback codec is insufficient.
Restore room-owned pending membership and its order without changing other
rooms or the relative order around world-scene queue barriers. Do not copy
future admissions from the live frontier queue into a replay. Native entry,
volume and world-condition producers are currently disabled during room
replay; this packet does not claim to fix every missed native trigger. Prove
the replayable scripted A/B request across the pending boundary. If that
requires a native-trigger policy change, state and select it separately.

A playback occurrence is a cutscene's run key, not another identity for an
actor. Role references keep the existing SimId/ObjectRef boundary below.
Likewise, ticket retirement does not end the scene; current cutscene request
cancellation is unsupported. This slice does not add cancellation or change
the current presentation-end meaning of `cutscene_ended`. SS8 states the
separate gameplay settlement change.

**First delivery:**

1. Give admitted waiting and active cutscenes the owner occurrence, and make
   queue admission return created/joined/refused. Preserve the stated queue,
   seen-flag and world-scene policies.
2. Bind current play tickets to it and answer from owner transitions. Delete
   `CutsceneAsk.began`, the per-tick join of cloned active script ids and
   queued triggers, and the requirement that presence polling sit between
   queue drain and playback tick. Reuse bounded action delivery and its
   existing replay/retirement rules.
3. Feed that owner occurrence into the role and condition work below.
   Preserve it in both whole-world and independent-room restore. Do not make
   the current request-correlation fix wait for all SS8 settlement work.

Extend the existing
[`scripted_cutscene_outcomes`](../../../game/ambition_app/tests/scripted_cutscene_outcomes.rs)
witness with overlapping A/B requests: B cannot start early, and A cannot
wait through B. Include empty-scene start/end, duplicate waiting native
triggers, independent repeatable use in two rooms, and the retained seen-flag
exclusion. Rewind across queue/start/end and rewind one room while another
continues; each ticket must retain its admitted occurrence. These are bounded
semantic cases, not a matrix over every authored scene.

The work removed is explicit: replies no longer clone active names and the
whole queue each tick and compare each ask against them. Work follows actual
admissions/transitions and their interested replies. Cutscene counts can be
small; no frame-time saving is claimed without measurement.

#### Qualify a conversation by its opening scope

**Source refinement at `00bf92c22142`, 2026-11-22.** Preserve the existing
conversation occurrence, role bindings, ledger and continuations. Complete
its identity with the opening live-room scope; do not replace it with a
new task or conversation framework.

[`ConversationInstanceId`](../../../crates/ambition_conversation/src/instance.rs)
contains opening tick, node, bare initiator/talker `SimId` values and the
dialogue context. [`open_between`](../../../crates/ambition_conversation/src/opening.rs)
does not capture the opening room. Existing `LiveBodyId` names a body by
its `SimId` and live room precisely because separate instances can contain
the same authored placements.

Two different seats can control equivalent authored initiator placements
in two live instances of one room and open the same node with equivalent
talkers/context on the same tick. Their body entities and seats differ, so
[`ActiveConversation::open`](../../../crates/ambition_conversation/src/authority.rs)
keeps both. Their minted IDs are equal: `of_instance` finds the first,
and `close_instance` removes both. Runner routing and the
[narrative ledger](../../../crates/ambition_conversation/src/ledger.rs)
also depend on that identity. This is a source-derived counterexample,
not a newly run two-room reproduction.

Capture the opening scope in the identity, or use the existing qualified
endpoint identities. Keep that scope stable through redirection and
continuation; do not re-key from participants' current location or mutable
`InputOwner`. Opening scope distinguishes the conversation occurrence.
It does not replace the existing live endpoint resolver or prevent a
supported participant from moving. No UUID, new global counter or extra
per-tick room scan is needed.

Prove this through the real opening and narrative-end path: use duplicate
authored placements in two rooms, distinct driving seats, equal tick/node/
context, and assert that ending one conversation leaves the other's runner,
input owner and pending work intact. Different home `slot:0`/`slot:1`
identities are a control, not the failing arm. Preserve replay equality and
the existing rule that a control handoff does not create a new conversation.
Update the existing identity codec/checksum where its representation changes.

#### Bind participants and calls

**Current source:** actor commands select bodies by character name through
[`direction::cast_in`](../../../crates/ambition_platformer2d_actor_monolith/src/body/direction.rs),
optionally in the command's room. `arrived`, `gone` and `beaten` independently
query the matching population without that room context. The
[`cutscene adapter`](../../../crates/ambition_platformer2d_runtime/src/cutscene.rs)
carries a room into commands, but its wait answers are keyed by raw condition
text. The same line cannot have independent answers for two bound scenes.

The original Yarn binding gap was addressed by the progress receipts below. The
[`conversation instance`](../../../crates/ambition_conversation/src/instance.rs)
names its initiator and talker, with the opening-scope refinement above still
open. The generic
[`command`](../../../crates/ambition_conversation/src/dialog/authored_commands.rs)
and [condition](../../../crates/ambition_conversation/src/dialog/authored_conditions.rs)
bridges now bind authored `@roles` through the existing instance and shared
call boundary. They still refuse raw reference parameters, which must not
become an alternate string-to-identity route. The command bridge's missing
conversation room context is a separate remaining qualification issue;
do not reopen the completed role-binding receipt to address it.

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
2. Migrate the Creator dialogue and brute scene. Address cutscene condition
   answers by the admitted playback occurrence above and prepared call/beat;
   conversation calls retain their existing conversation occurrence, qualified
   by its captured opening scope as above.
   Delete raw-line answer conflation and repeated individual population
   selection. Preserve the
   narrative ledger and current command/action-reply runner.
3. Bind the doorway raiders as a cohort **after** their spawn milestone. They
   do not exist at scene start. State whether each group binding captures
   members at that point or is deliberately a live population query. Keep
   existing broadcast forms as explicit adapters with their old meaning.

**SS7 progress (2026-11-21): roles through the shared boundary and Yarn.**
`ParamKind::Individual` is a parameter that takes a character id (its
population) or `@role` (one occurrence its invocation binds).
Preparation turns `@talker` into `AuthoredArg::Role`. `RoleBindings` and
`PreparedCondition::bound` / `PreparedCommand::bound` replace each role
with the bound `SimId`. A role the invocation did not bind is refused,
naming the roles it did bind. The catalogs refuse an unbound role that
reaches evaluation; it is never read as the population. The actor verbs'
`character`, `target` and `way`, and the conditions `arrived`, `gone` and
`beaten`, take `Individual`. The counts (`count_is`, `fallen_is`,
`fallen_in`) stay population names.

A bound body resolves by its `SimId` wherever it lives, since a directed
body can leave into another live room. The command's room only tells apart
two bodies that carry the identity. A fallen bound body refuses, and so
does a retired one. It is never replaced by another body of the same
character.

The Yarn condition and command bridges bind `@talker` and `@initiator` from
the speaking `ConversationInstanceId` (new `talker()` / `initiator()`
accessors). The Creator's `intro.yarn` now walks and asks about `@talker`
instead of spelling `npc_creator`.

**SS7 step 2 (2026-11-21): scene roles and the brute scene.** A
`CutsceneScript` declares `roles: [("brute", Character("npc_raid_brute")),
...]` (`RoleCast::Character` or `RoleCast::Placement`). When the scene
starts, `drain_cutscene_triggers` binds each role to one live body: the one
body of that character in the scene's room (in the world, for a scene of the
whole world), or the placement's body. With none, or with more than one, the
role stays unbound and every call naming it is refused; it is never handed
to another body.

`PlayingCutscene.roles` holds the bound identities. It is rollback state, in
`ActiveCutscene` and in the room log's `CutsceneState` (schema 523 to 524,
codec shape re-recorded). Each `Command` line and each `WaitFor` condition
is prepared and then bound with its own scene's roles.

`CutsceneWaitAnswer` is keyed by `CutsceneWaitKey` (room, script, beat)
instead of the raw line. Two scenes waiting on one line, or one line at two
beats, are answered apart.

`check_cutscene_lines` refuses a line that names an undeclared role. The
brute scene in `intro.ron` now binds `@brute` and `@creator` once and uses
them throughout.

Unit witness: `cutscene::tests::each_scene_binds_its_roles_in_its_own_room`.
Two rooms' scenes each bind the body in their own room. A scene of the whole
world, seeing two bodies of the character, binds neither. The wait answers
of two rooms are told apart.

Still open:
- One script still plays in one room at a time (`ActiveCutscene::plays`).
  "Two live rooms with the same script" therefore remains blocked by that
  rule, not by the keys.
- Conditions carry no room, so a bound condition with two live bodies of
  one identity is unanswerable rather than room-resolved.
- The conversation command bridge still supplies no room.
- Reload rebinding is not exercised.

**SS7 step 3 (2026-11-21): the doorway scene.** `intro_raid_at_the_door`
binds `@creator` at its start and directs the Creator through it. The raiders
are deliberately not bound: they do not exist when the scene starts.
`npc_lab_raider` stays a live population query of the scene's room, the
crowd the scene forms up and turns, which is the broadcast form's old
meaning. It is asked only after the scene waits for their spawn
(`actor.count_is npc_lab_raider 3`).

Capturing a cohort at that milestone would need a mid-scene binding beat.
That waits for a scene that must act on those particular members after the
crowd has changed. SS8's dependency on the spawn before directing the cohort
is unchanged.

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

Attach those obligations to the SS7 playback occurrence. Keep the occurrence
after optional presentation ends while its stated gameplay work is pending.
A later run of the same script cannot inherit its replies, role bindings or
obligations. Distinguish presentation dismissal from gameplay settlement in
the existing action contract when this slice changes the current completion
meaning; do not infer either event from absence in `ActiveCutscene`.
For a play request whose contract includes required gameplay, dismissal
cannot terminally complete its ticket. Settle or fail that ticket from the
stated obligations; a retired ticket cannot receive a second completion.

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

**SS8 landed (2026-11-21).** A skip ends the presentation and keeps the
script's cursor. `CutsceneRuntime::begin_skip` marks the playback skipping
(rollback state: schema 525). From then on the scene shows nothing, holds no
player (`PlayingCutscene::holds`), and passes every beat at once, in order,
except a `WaitFor`. A `WaitFor` still waits for its answer or its longest
hold. The seen flag is written at completion, by the one ending
(`end_cutscene`), not when the skip is asked. The blanket rule is deleted:
`CutsceneRuntime::skip`, which emitted every remaining command at once and
ended the scene, and its `Skipped` event. One authored sequence serves both
the played and the skipped scene.

A plain `Wait` is a delay, and a skip drops it. A scene whose next beat
depends on a result must wait on that result. The brute scene had one such
dependency written as a delay: the bomb must burst before the Creator is
released to his own mind, which would dodge it. The 1.4 s delay after the
throw is now `WaitFor actor.throw_landed @brute`.

A second dependency is easy to break by "fixing" it. A body a playing scene
holds is invulnerable (`project_cutscene_hold`, `Invulnerability::CUTSCENE`)
unless it is on its way somewhere (a pending `Visit`). The bomb hurts the
Creator in normal play because it is thrown while he is still walking to the
bench. Waiting for his arrival before the throw (tried first) made him held,
and so immune, when the bomb burst. The skipped scene holds no body, so only
the normal-play arm caught it.

`actor.throw_landed` is new. It answers from the thrown items themselves: an
item minted by `SimId::spawned` of the thrower, `ReleasedAs::Throw`, still
moving or with a lit fuse. So a thrower that has left still answers for its
throw. A thrower that still holds what it could throw has not thrown yet,
and the answer is no. That clause is load-bearing: a scene's command is a
request applied after the scene answers its next wait, so the first answer
after a throw beat can come before the bomb exists. Without the clause,
"nothing in the air" held at once, the Creator was let go before the blast,
and he dodged it in normal play. The same timing applies to any wait that
follows a command: the condition must be false before the command takes
effect, not only after. The doorway scene already waited on its results:
`actor.arrived @creator`, the raiders' spawn (`actor.count_is
npc_lab_raider 3`) before `form_up`, and their arrival. So a skip before
`intro_raiders_coming` still directs the cohort only after it exists.

Witnesses:

- The unit test `a_skipped_scene_still_waits_for_what_it_waits_for`, and
  `a_skip_keeps_the_flags_and_falls_it_skipped` rewritten over `begin_skip`.
- `a_skipped_brute_scene_ends_the_raid_as_a_played_one_does`. It holds
  cancel from the scene's first tick and asserts normal play's outcome: the
  case taken, the brute gone, the three raiders fallen, and the Creator
  wounded by the bomb but alive.
- Still open: a scene's harm to a body it holds depends on that body being
  on its way somewhere. A scene that means to hurt a body standing still has
  no way to say so.

Still open:

- A `WaitFor` that times out proceeds, as before. No scene yet needs a
  required result with its own failure policy (refusal and unanswerable told
  apart from timeout).
- A skip cancels no scene-owned work, because none is pending as a
  scene-owned operation. A launched bomb is committed world work and flies on.
- The shell's own sequence skip (`ShellSequenceCommand::Skip`) is another
  runtime, with no gameplay commands, and is unchanged.

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
