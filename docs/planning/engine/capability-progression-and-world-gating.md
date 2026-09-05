# Capability progression and world gating — Engine 1.0 program

**State:** OPEN — systemic gating is the preferred direction; capability ownership details are intentionally unresolved.

> **RE-MEASURED against `bc85c059d` (2026-09-02). ⭐ THE INPUTS TO FOUR GATE FAMILIES
> ALREADY EXIST; THE GATE THAT READS THEM DOES NOT.** That is a materially
> different starting position from "design the whole thing", and it says where
> the first slice is.
>
> | gate family | vocabulary at HEAD |
> |---|---|
> | body capability | ⭐ `AbilitySet` (`platformer2d_core/src/abilities.rs`) with **14** fields: `move_horizontal`, `jump`, `variable_jump`, `double_jump`, `fast_fall`, `wall_jump`, `wall_cling`, `wall_climb`, `dash`, `double_dash`, `fly`, `fly_toggle`, `blink`, `precision_blink` |
> | body property | ⭐ `mass` (`Option<f32>`, read at spawn, rollback-registered as `mount.mass`), `standing_height` (authored; "IS the body's height — not a hint"), `Locomotion` |
> | item/equipment | `crates/ambition_items`, `crates/ambition_inventory_ui` |
> | world mechanism | partial — `PersistedSwitch`, `GravityFlipSwitch`, `Empowered`; no general "mechanism is in state X" fact |
> | soft systemic pressure | nothing route-facing |
> | social/knowledge | nothing route-facing |
> | story gate | the authored road, as intended |
>
> ⛔ **AND NOTHING GATES A ROUTE ON ANY OF THEM.** The types named `*Gate` in the
> workspace are `ActionGate` (`entity_catalog/src/action_scheme.rs`),
> `EncounterGate` (`ambition_encounter/src/timeline.rs`) and `OutOfShieldGate`
> (`platformer2d_core/src/movement/abilities.rs`) — action and encounter
> sequencing, not world traversal. A search for a route requirement reading a
> body capability finds only cooldown and brain-action gating.
>
> ⇒ **So the open question is narrower than the page's framing suggests.** It is
> not "how do we represent seven gate families"; it is "what reads `AbilitySet`
> and `mass` when a body meets a route, and who owns that predicate". The
> capability-ownership detail the page leaves unresolved is exactly the thing
> that first slice would have to decide, and it can now be decided against real
> types rather than proposed ones.

> **RE-MEASURED AGAIN against `e685f7982` (2026-09-03), and the block above
> overstates the gap.** ⭐ **A ROUTE-GATING MECHANISM EXISTS, IS AUTHORED, AND
> ALREADY SERVES TWO OF THE SEVEN FAMILIES END TO END.** Seal walls are derived
> onto the collision overlay's `gate_solids` in `WorldPrep` and read by three
> consumer domains — body collision
> (`platformer2d_world/src/collision.rs`), projectiles
> (`ambition_projectiles/src/collision_world.rs:56`) and rendering. Two systems
> write it, registered together in `WorldGatingSchedulePlugin`
> (`platformer2d_runtime/src/world_gating.rs`):
>
> | writer | gate family it serves |
> |---|---|
> | `contribute_encounter_lock_walls` | story gate — encounter phase |
> | `sync_authored_gated_lock_walls` | story gate — an authored `gated_by` condition |
>
> ⇒ So "nothing gates a route" was wrong as stated. What is true is narrower and
> more useful: **nothing gates a route on a BODY fact.** The gap is not a missing
> mechanism, it is a missing input to one that already works.
>
> ⛔ **AND THE INPUT IS BLOCKED BY ONE HARDCODED ARGUMENT, WHICH IS THE REAL
> FIRST SLICE.** The authored road resolves its question through the shared
> `ConditionCatalog`, which is extensible and already carries three published
> production conditions (⚠ three when this was written; **five** at
> `4b8ef1b04` — `body.can` and `body.fits` joined below):
>
> | published condition | publisher | family |
> |---|---|---|
> | `flag_set` | `actor_monolith/src/world_facts.rs:139` | story gate |
> | `is_held` | `ambition_held_items/src/lib.rs:60` | item/equipment |
> | `holds` | `actor_monolith/src/items/conditions.rs:92` | item/equipment |
>
> But a gated wall cannot ask for two of them. `prepare_question`
> (`world/gated_lock_walls.rs:295`) calls
> `catalog.prepare(flag_set.clone(), &[wall.gated_by.as_str()])` — the condition
> **id is fixed to `flag_set`**, and the authored `gated_by` string is only its
> ARGUMENT. The module says so deliberately: *"the current authored field
> intentionally names only a flag even though the condition mechanism is
> extensible."*
>
> ⇒ **Hence the item/equipment family is published but unreachable from a
> route**, and a body-capability condition would be too, however well it was
> written. Publishing an `AbilitySet`-reading condition is the easy half and
> lands dead unless the authored field can name it.
>
> ⇒ **The first slice is therefore a data-shape decision, not an architecture
> one:** let an authored wall name a condition id plus arguments instead of a
> bare flag name. That decision is small, has a precedent for every piece it
> needs, and — usefully for this page — it can be taken WITHOUT first answering
> any of the open design questions below, because it changes what an author may
> ask, not who owns a capability.
>
> ⭐ **AND THE MIGRATION IS TWO ROWS.** `gated_by` is an LDtk entity field
> (`platformer2d_ldtk/src/conversion/entity_converters.rs:64` →
> `platformer2d_world/src/rooms/specs.rs:202`), and across all five authored
> worlds it is set exactly **twice**, both in `intro.ldtk`, both to the same
> value — `"bob_field_survey_received"`, a story flag. So widening the field
> costs either a two-row edit or a "bare string still means `flag_set`"
> fallback, and either is small. ⚠ I did not check what the LDtk *editor* needs
> to define a second field, which is the remaining unknown.
>
> ✔ **THE FIRST SLICE IS LANDED (2026-09-04). `gated_by` IS AN AUTHORED
> CONDITION LINE.** A wall may now write `"inventory.holds axe"`; a bare value
> still means `world.flag_set <value>`, which is what both shipped rows say, so
> the two-row migration was not needed at all and the LDtk editor question never
> arose. ⭐ **AND THE MECHANISM WAS ALREADY BUILT** — `ConditionCatalog::prepare_line`
> parses exactly this form, and `CommandCatalog::prepare_line`'s own doc had
> already declared it *"the form an authored FIELD carries, because a level
> author writes one string and the number of arguments is the verb's business
> rather than the field's."* The slice was one call site choosing between two
> existing entry points, not a data-format design. ⇒ **The item/equipment family
> is reachable from a route now** (`inventory.holds`, `custody.is_held`), guarded by
> `a_wall_may_be_gated_on_an_item_the_player_carries` — empty bag, wall up; grant
> the axe, wall opens — poison-verified against a discriminator stuck at `false`,
> which reddens that arm alone and leaves the flag arm green.
> ⛔ **The discriminator is SYNTACTIC and must stay so:** a first token shaped
> like `domain.question` names a condition, so a MISSPELT condition reaches the
> catalog's diagnostic and leaves the wall standing rather than being demoted to
> a flag lookup that would never be satisfied either. A flag id containing a `.`
> is not addressable in the bare form and must be written out in full.
> ✔ **THE BODY-CAPABILITY CONDITION IS PUBLISHED AND THE ROAD IS WALKED
> (2026-09-04).** `body.can(verb)` reads the EFFECTIVE `BodyAbilities`, not
> `AbilityBase` — a session mask or a story lockout that turns a verb off must
> close the route it opens, or the wall and the world disagree about what the
> player can do. Its `verb` lookup is an exhaustive destructure of `AbilitySet`
> under `deny(unused_variables)`, so adding a capability is a compile error at
> the one moment its author is still looking, rather than a verb that is silently
> unaskable.
>
> ⭐ **And the ROAD is what was actually missing, not the condition.** It was
> published, registered by `BodyCapabilityConditionsPlugin`, and unit-tested
> against a hand-built world — none of which says an authored wall can reach it.
> `a_wall_may_be_gated_on_what_the_body_can_do`
> (`world/gated_lock_walls/tests.rs`) walks the whole chain from an authored
> `gated_by = "body.can wall_climb"` through the syntactic discriminator,
> `prepare_line`, the `Name` parameter and `AbilitySet`, and changes exactly one
> bool on the body between its two assertions. ⛔ The body arrives WITHOUT the
> verb and the closed arm is asserted first: a wall that was never up cannot be
> observed opening, and "nobody to ask" is a different answer from "the body
> cannot do it". Poison-verified by making every body satisfy every verb — that
> arm reddens alone.
>
> ✔ **AND BODY PROPERTY LANDED THE SAME DAY — `body.fits(height)`, the sixth
> family.** It is the gate this page's Goal names first (*"gate routes through
> body size"*), and the crawlspace is the shape:
> `a_wall_may_be_gated_on_the_body_being_small_enough_to_pass` changes nothing
> between its two assertions but the body's height, and the route changes with
> it.
>
> ⭐ **It reads `BodyKinematics::size`, the CURRENT size, not `BodyBaseSize`** —
> the same choice `body.can` makes for the same reason. The kinematic size is
> what the stances write; the base size is the authored standing baseline they
> derive FROM. A gate that asked the baseline would refuse a body that physically
> fits, which is the world disagreeing with itself about a hole in it. So
> crouching and morphing count, and that is a consequence of the existing
> effective-versus-authored rule rather than a new ruling.
> ⚠ **CORRECTED the same day: I first wrote that the kinematic size is "what the
> collision doctrine sweeps", and it is not.** The doctrine sweeps
> `aabb_oriented(gravity_dir)`, which SWAPS width and height under sideways
> gravity because the body lies along the wall; `BodyKinematics::size` is the
> body's OWN-frame size. So under flipped gravity `body.fits` reads a different
> number than the collision footprint does.
> ⭐ **And gravity-independent is the right rule for a route, which is why the
> code says so now instead of implying the other thing.** A world-space reading
> would make one authored wall open and close as gravity flipped, so an author
> could not say what their own crawlspace means without knowing which way gravity
> pointed when the player arrived. The body and the passage rotate together; "how
> tall is this creature" does not. Pinned by
> `the_opening_is_measured_against_the_bodys_own_height_not_its_world_footprint`,
> whose body is 30 tall and 64 wide so the two readings disagree — a square
> fixture would have passed either way.
> ⛔ ONE parameter, deliberately: a width question is a different condition, not
> a second argument. In a side-on platformer you traverse an opening
> horizontally, so "am I short enough" is the physical question and "am I narrow
> enough" is a different route shape that should be published and named before
> it can be authored.
> ⚠ A non-positive opening is `Unanswerable`, not false — no body has a
> non-positive height, so `false` would be right for the wrong reason and would
> hide an authoring mistake behind a wall that correctly never opens.
> Poison-verified twice: `<=` weakened to `<` reddens the equal-height arm
> alone, and a predicate that fits everything reddens the wall arm alone.
>
> ⚠ **AND THE WORLD-MECHANISM ROW WAS WRONG WHEN I WROTE IT AN HOUR EARLIER —
> `world.switch_on` landed the same day.** I recorded that family as "a missing
> FACT rather than a missing condition… a seam to negotiate", which was a
> consequence inferred from this page's own older *"no general 'mechanism is in
> state X' fact"* rather than a reading of the save layer. Reading it:
> `AmbitionGameSaveData::switch(id) -> bool` and `set_switch(id, on)` have been
> there all along, durable and by id, and **the input is already flowing** —
> completing a wave encounter latches every switch linked to it
> (`crates/ambition_encounter_features/src/systems.rs:496`, all of them and not
> the first, with authored ids beside the encounter). So the boolean half was a
> predicate to write, and it is written.
>
> ⭐ **It is not `flag_set` with a different name, and the distinction is what
> the family IS rather than where the bit lives.** A flag is a story fact
> something recorded about the player; a switch is a mechanism's own state,
> flipped by the world doing what the world does. They share the save because
> both are durable, and they are two questions because "the arena has been
> cleared" and "you have been told about the survey" are different design.
> Poison-verified by pointing `switch_on` at the FLAG namespace: the wall test
> reddens alone, which is the proof the two namespaces are separate and this
> condition reads the right one.
> ⚠ An unrecorded switch is `NotSatisfied`, and that is not the concession
> `flag_set` makes — a latched switch nothing has flipped IS off, so `false` is
> the true answer rather than the tolerable one.
>
> ⛔⛔ **AND THE HONEST SENTENCE AFTER ALL OF THIS: NO SHIPPED LEVEL AUTHORS ANY
> OF THE FIVE NEW CONDITIONS.** `gated_by` is set exactly twice across the five
> authored worlds, both in `intro.ldtk`, both to `bob_field_survey_received` — a
> story flag. So five families are reachable and none is *reached*.
>
> ⚠ **That is a real tension with this track's own rule** — `tracks.md`: *"Grow
> the authoring vocabulary only from concrete progression needs"* — and it should
> be named rather than left for a reader to notice. Two of the five had the need
> written down before the work (`body.can` is what this page said was missing;
> `body.fits` is the Goal's own first example, "gate routes through body size").
> The other three — `custody.is_held`, `world.switch_on`, `encounter.cleared` — were
> published because the FACT already existed and had a producer, which is a
> weaker justification than a level that wants them.
>
> ⇒ **The risk is the dormant-cluster shape this repository has hit before**: a
> registry with no production producer, correct in every test, reached by
> nothing. What protects against it here is that each condition has an end-to-end
> wall test walking the authored road, so the vocabulary is proven usable rather
> than merely present. What does NOT protect against it is anything in the
> engine: only a level author choosing one of these does that. ⇒ **Which routes
> should stop being story flags is a design call, not an engineering one**, and
> the next slice of this track is a level, not a condition.
>
> ⭐⭐ **AND THE DENOMINATOR, MEASURED 2026-09-04, CHANGES WHAT THAT CALL IS.**
> `scripts/authored_route_gates.py` (committed) walks every `.ldtk` in the repo:
> **six worlds, THREE `LockWall` instances — two gated, one encounter lock.**
> Both gated walls are in `intro.ldtk` and both name
> `bob_field_survey_received`.
> ⇒ **So "five families reachable and none reached" is not a migration backlog;
> there is nothing to migrate.** Converting both existing walls would empty the
> story-gate family — the one this page says should stay available *"when
> sequencing is actually the design"* — and still leave four families unused.
> **The vocabulary is not unused because authors chose flags. It is unused
> because the world has almost no gates at all.**
> ⇒ Filed as **awaiting-maintainer-decision #55**: grow the world, leave it and
> accept the dormant-cluster risk explicitly, or convert the two (not
> recommended, and recorded so it is not tried).
> ⚠ **Do not answer it by adding walls to a demo world to make the count go up**
> — a gate authored to exercise the vocabulary is the dormant cluster wearing a
> level's clothes.
>
> ⛔⛔ **AND THE COUNT ABOVE WAS ITSELF ONE CONSUMER SHORT — corrected the same
> hour, by the rule it had just established.** `ConditionCatalog` has a SECOND
> authored road and it is the busier one:
> `ambition_conversation/src/dialog/authored_conditions.rs` installs a Yarn verb
> `condition(id, arg)`. Both roads, measured:
>
> ```text
> LockWall instances: 3  (2 gated, 1 encounter)
> condition uses in .yarn: 18  (7 inventory.holds, 5 boss.cleared,
>                               3 quest.active, 3 world.flag_set)
> TOTAL authored uses: 20  (2 route gates + 18 dialogue lines)
> published but authored NOWHERE: 5 of 9
> ```
>
> ⚠ **AND THE FIFTH IS `custody.is_held`, NOT `held.is_held`** — the id this
> page and its siblings spelled by hand for a day. `ambition_held_items` declares
> `DOMAIN = "custody"`, so the hand-written spelling names a condition nobody
> published. Found by deriving the list from the source instead of keeping it.
>
> ⛔ **THOSE FIGURES WERE 12 / 5-of-7 UNTIL THE SCRIPT LEARNED THE SECOND
> SPELLING.** A condition is reachable from `.yarn` two ways — the generic
> `condition(id, arg)` verb and a NAMED function bound to it (`boss_cleared`,
> `quest_active`) — and counting one reported the two newest conditions as
> unauthored on the day they shipped *because* they had authored callers.
>
> ⇒ **Five times more authored uses are DIALOGUE than routes.** So this page's
> framing — everything as a route question — is what made the first count look
> complete. ⭐ *"You look like you could climb that"* in a `.yarn` line is a
> `body.can` customer that costs no level geometry, and it is invisible from
> here. **The dormant-cluster number that survives is five of NINE published
> conditions authored nowhere by either road** — `world.switch_on`,
> `custody.is_held`, `body.can`, `body.fits`, `encounter.cleared` — measured
> across both consumers and both spellings. ⚠ The denominator moved from seven
> when `boss.cleared` and `quest.active` shipped; the numerator did not.
>
> ✔ **AND THE VOCABULARY IS SAFE TO AUTHOR AGAINST NOW, which is what that slice
> needed from engineering (2026-09-04).**
> `every_authored_gate_condition_prepares_against_the_composed_catalog`
> (`game/ambition_app/tests/the_engine_can_be_asked_questions.rs`) walks every
> `gated_by` in every shipped world and prepares it against the SAME catalog the
> game composes, so a misspelt condition or a wrong argument count fails a build
> instead of a playthrough. ⛔ That failure is the risk the condition-line
> widening introduced: an unpreparable gate leaves the wall STANDING, which is
> correct behaviour and indistinguishable in play from a gate that is simply not
> satisfied yet.
> ⭐ It calls `prepare_authored_gate` — the production function the wall system
> calls — rather than restating its two-road rule, so it cannot drift into
> validating a rule the game stopped applying.
> ⚠ It asserts a FLOOR on its corpus (2 today) and caught its own vacuity on the
> first run: it read `embedded_text`, which is `None` without `static_map`, so it
> had validated nothing at all.
>
> ⛔⛔ **FIVE of the seven, and I first wrote "seven of the seven" — an
> arithmetic error, caught the same day by counting the list against this page's
> own `Gate families` section.** The sentence contradicted itself in its own next
> clause: it named five families and then said two were empty.
>
> ⇒ **Reachable from a route now (5 of 7):** story gate (two writers),
> item/equipment (`inventory.holds`, `custody.is_held`), body capability
> (`body.can`), body property (`body.fits`), world mechanism
> (`world.switch_on`).
> ⛔ **Route-facing-empty (2 of 7): soft systemic pressure and
> social/knowledge.** Neither has a FACT to read, which makes them a different
> kind of gap from the five and not a smaller version of one — a condition
> written for either would have nothing to ask.
> ⚠ And within the reachable five one residual is genuinely UNEXPRESSIBLE: a
> mechanism in a state that is not a bool — a valve at half, a lift at floor
> three. That is the seam, and it is one family's residual rather than a
> family's absence.
>
> ✔ **AND POSSESSION IS RULED ON FOR ROUTES (2026-09-04), which answers one of
> this page's own open questions in the direction the world already believed.**
> `body.can` and `body.fits` asked *"any body holding `PlayerEntity` OR
> `DrivingParticipant`"*, and possession moves the seat OFF the home avatar and
> onto the target (`control/authority.rs:39-45`) while the home KEEPS
> `PlayerEntity`. ⛔ So while the player drove a vessel that cannot climb, a wall
> gated on climbing opened on the strength of the body they had left behind — and
> the reverse refused a possessed vessel that could climb because the resting
> avatar could not. Both directions are now pinned, and the production-path half
> drives the disagreement through an authored `GatedLockWall` rather than through
> the condition function.
>
> ⇒ **The ruling, stated so a later condition inherits it rather than re-deciding
> it: a route asks the body a participant is DRIVING.** Nothing "transfers" on
> possession — which is why this answers *"when possession changes bodies, which
> theorem abilities transfer and which do not"* for routes without answering it
> for progression: the route never asked about the participant in the first
> place, it asked about a body, and possession changes which body that is.
> ⚠ It does NOT settle *"can an item temporarily satisfy a capability
> requirement"*: an item that grants a verb does so by writing the driven body's
> effective `AbilitySet`, which the condition already reads, so that question is
> about who may write the set rather than about who is asked.
>
> ⛔⛔ **AND THE CO-OP QUESTION ON THIS PAGE IS NO LONGER HYPOTHETICAL** — the
> fallback is an existential over every `DrivingParticipant` holder, so with two
> seats a wall opens when EITHER driver qualifies. Named at the call site and
> filed as **awaiting-maintainer-decision #54**, deliberately not settled here:
> `gate_solids` is ONE `Vec<Block>` on one overlay read by body collision,
> projectiles and rendering, so a wall that stands for one player and not another
> is a mechanism change rather than a stricter predicate.
> ⚠ The same OR existed under the `PlayerEntity` predicate and was equally a
> ruling. Widening the population made a latent decision live; it did not create
> one.
>
> ⚠ **REPAIRING IT REDDENED TWO PRE-EXISTING ROUTE TESTS**, which is composition
> repair rather than regression: both fixtures held `PlayerEntity` alone, and
> `actor_monolith/src/features/ecs/dormancy.rs:96` already records the identical
> trap one domain over — *"a fixture that spawned `PlayerEntity` alone would find
> NO OBSERVERS AT ALL"*. Both grew a seat.
>
> ⓘ **A body-capability predicate is already written, for actions rather than
> routes.** `ActionSet::gated_by(AbilitySet)`
> (`ambition_characters/src/brain/action_set/mod.rs:115`) narrows a brain's
> action set on `abilities.attack` and `abilities.shield`. Whoever publishes the
> route-facing condition should read it first — not to share code, since it
> answers a different question, but because it is this workspace's existing
> answer to "how does a body capability narrow what is possible", and the two
> should not disagree about what `AbilitySet` means.

## Goal

Make exploration and progression primarily emerge from **what the controlled
body can do, what it carries/equips, and what the world has physically become**,
rather than from a long chain of story-stage flags.

Ambition should be able to gate routes through body size, movement abilities,
portal use, environmental resistance, tools, keys, powered machinery and other
mechanical facts. Explicit narrative gates remain available when sequencing is
actually the design.

## ⭐ The authored-gate integrity story is now complete, and the last piece was
## about the FACT rather than the condition (2026-09-04)

Four guards stand between an author and a gate that can never open, and they
fail at four different moments:

| guard | catches | when |
|---|---|---|
| `every_authored_gate_condition_prepares_against_the_composed_catalog` | a `gated_by` naming a condition no domain publishes | build |
| `every_condition_an_authored_yarn_file_asks_is_published_by_the_engine` | a `condition("…")` naming one, in dialogue | build |
| `no_planning_doc_names_a_condition_the_engine_does_not_publish` | a fabricated id spreading through the planning prose | build |
| `test_every_gated_flag_has_a_writer.py` | a flag NAME nothing can ever set | build |
| `test_alias_arguments_name_something_real.py` | an ARGUMENT to a named alias that names no real boss or quest | build |
| `every_authored_item_id_resolves_to_a_real_item` | an ITEM id no catalog spelling resolves | build |

✔ **ALL SIX VERIFIED LIVE 2026-09-04, by name, after the last of them landed —
seven test functions, all passing:** `test_every_gated_flag_has_a_writer` (1),
`test_alias_arguments_name_something_real` (2),
`every_authored_gate_condition_prepares_against_the_composed_catalog` +
`every_condition_an_authored_yarn_file_asks_is_published_by_the_engine` +
`no_planning_doc_names_a_condition_the_engine_does_not_publish` (3 together),
`every_authored_item_id_resolves_to_a_real_item` (1).
⭐⭐ **AND WHAT THE SIX BUY, STATED AS A PROPERTY: "authored" now means
"reachable" for every family except the one with a filed decision.** Checked
across the five conditions with authored callers, 2026-09-04:

| condition | authored | arguments verified by |
|---|---:|---|
| `wallet.can_afford` | 10 | numbers; `shop::authored_price` accepts them and the action shares it |
| `inventory.holds` | 5 | `every_authored_item_id_resolves_to_a_real_item` |
| `boss.cleared` | 3 | ✔ resolvable since 2026-09-05 — `every_authored_boss_cleared_call_names_a_real_boss_placement` |
| `quest.active` | 1 | `test_alias_arguments_name_something_real` |
| `world.flag_set` | 3 | `test_every_gated_flag_has_a_writer` |

⛔⛔ **AND FIVE OF THE GUARDS BEHIND THIS TABLE ARE INVISIBLE TO A PER-CRATE
RUN — measured 2026-09-05, on my own work.** `yarn_condition_aliases.rs` is
`#![cfg(feature = "ui")]`, and `ambition_content`'s manifest says `default = []`.
So:

```text
cargo test -p ambition_content --test content_it yarn_condition_aliases
  → running 0 tests
cargo nextest run --workspace   (the same tests)
  → PASS at 2604/7150
```

⇒ **They compile only because ANOTHER workspace crate turns on
`ambition_content/ui`, and cargo unifies features across the graph.** The guards
are real and they run in every lane anyone actually uses — but their existence
depends on a feature edge in a crate that is not theirs. If that edge ever goes,
five authored-integrity guards vanish and a `-p` run has been reporting `0 tests`
the whole time rather than a failure.
⚠ The gate is right where it says *"a crate built by itself is not compiled by
`check_no_warnings.py` either"*; this is the same blind spot pointed at TESTS.
⭐ ⓘ The honest split: the interpreter arms genuinely need `ui` (they drive real
Yarn through `bevy_yarnspinner`). **`the_boss_fixture_id_is_not_a_name_any_shipped_dialogue_uses`
does NOT** — it only reads `YARN_SOURCES` strings — so it is gated by where it
happens to live rather than by what it needs, and moving it to an ungated file
would make it survive the edge going away. That is the change to make.

⚠ **THE COUNTS IN THIS TABLE WERE RAW TEXT UNTIL 2026-09-05 and are now
EXECUTABLE calls** — `inventory.holds` 7→5, `boss.cleared` 5→3,
`quest.active` 3→1. The difference is characters SPEAKING a call in dialogue,
which four whole-file scanners counted as code until `executable_regions` landed.
✔ And `boss.cleared`'s row no longer says *"none can be true"*: Jon ruled
question 57 and the placement carries an authored encounter id the dialogue
names.

⇒ **Exactly one family is authored-but-dead, and it has a decision waiting.**
The other four have every argument checked at build time by a guard that fails
on a typo. ⚠ That is the useful form of the claim: not *"six guards exist"* but
*"a misspelling in any authored condition argument now reddens a build, and the
one remaining unreachable call is a design question rather than a spelling
one."*

✔ **ALL SIX RUN IN THE DEFAULT GATE, which is not automatic and was checked.**
Three are pytest guards (`scripts/tests/`), which every lane keeps including
`--rust`. The other three live in `app_it` and `content_it` in files carrying
NO `#[cfg(feature)]`, verified by running them with no `--features` at all.
⚠ **Worth stating because the sibling case exists**: the Yarn-interpreter
acceptances for `boss_cleared` / `quest_active` / `can_afford` are
`#![cfg(feature = "ui")]` and run ONLY under the exhaustive union — the right
home, since `bevy_yarnspinner` exists only there, but it means a regression in
those aliases passes the ordinary gate. ⇒ A guard's lane is part of what it
guards.

✔ **AND EVERY FLOOR CLEARS ITS LARGEST SINGLE SOURCE — audited and poisoned
2026-09-04, not assumed.** An anti-vacuity floor of "non-empty" is worthless
when the corpus spans two files, because losing either leaves the other. Both
guards whose corpus splits were poisoned by removing ONE file's spelling:

```text
  boss_cleared   kernel 3 + cove 2   floor raised >= 4   one file lost -> RED
  world.flag_set kernel 1 + intro 2  floor >= 3 distinct one file lost -> RED
```

⛔ The alias floor was `assert asked` (non-empty) until this audit and would
have survived losing `cove.yarn` entirely — which is exactly how a poison of
mine passed earlier the same day and read, for a minute, as a broken guard
rather than a mis-aimed poison.

⚠ **Run by name rather than trusted from the table**, because a table can list
a test that no longer executes — which is not hypothetical here: a stolen
`#[test]` attribute left `mirroring_a_bout_swaps_every_per_seat_reading` dead
for a day earlier the same day, and it read as a test in every listing.

⛔ **The fourth was the hole, and it is invisible to the other three by
construction.** `world.flag_set` takes an author-typed flag NAME, so a misspelt
one parses, names a published condition, evaluates successfully — **and answers
NO for the rest of the game**, because nothing will ever set a flag by that
name. ⇒ A wall gated on it never opens and a dialogue branch behind it is
unreachable, and both look like content nobody wrote rather than content that is
broken. The first three guards are all about the CONDITION; only this one looks
at the FACT.

⛔⛔ **AND THE FIFTH ONE LANDED ON A DEFECT THAT WAS ALREADY LIVE, which none
of the other four did.** `boss_cleared("mockingbird")` — **three** executable
authored calls (five raw: two of the five are the Kernel Guide SAYING the call in
prose, counted by four whole-file scanners until `ea71c83a8`) —
passed the BEHAVIOR id while `boss_encounter/src/systems.rs:259` writes the save under the
PLACEMENT (`BossSpawn-4308`). Exact lookup, no bridging, so those branches had
never been able to open. ⇒ Filed as question 57 rather than repaired: making
the dialogue pass `BossSpawn-4308` puts an LDtk-generated identifier into an
authored script, and the alternatives change what existing saves mean.

✔✔ **RULED AND CLOSED 2026-09-05 (`dff7c908c`), and the objection above is what
the answer had to get past.** Jon: *"Boss progress is keyed only by stable
authored encounter/placement IDs."* ⇒ Neither horn of the dilemma was taken —
the dialogue does NOT carry `BossSpawn-4308`, and no existing save key was
reinterpreted. A third option: the placement gets an AUTHORED id
(`BossSpawn.encounter_id` = `cove.mockingbird`), so the id an author types IS the
placement's id, and `ldtk::fields::boss_placement_id` is the one definition of
it. The LDtk iid stops being exposed rather than being written into a script.
⚠ Worth keeping as a pattern: *"an authored identifier the author can type"* is a
third road whenever a durable key and an authoring surface disagree, and it was
invisible while the choice looked like "use theirs or use ours".
⚠ **So that guard accepts EITHER spelling on purpose.** It asks *"does this name
a real boss at all"*, which is answerable today and catches the typo class;
which id the API should take is 57's to answer. When 57 lands, narrow it and
the losing spelling becomes a red.
⭐ **The six now cover every author-typed string family the dialogue vocabulary
has** — the condition id, the fact (flag) name, the alias argument, and the item
id. Between them a misspelling in authored content stops being invisible.
⚠ **And the item guard ASKS the resolver rather than listing the items**
(`Item::from_dialog_id`), because a table of valid spellings would be a second
authority on normalisation — the exact defect `normalize_item_id` was deleted
for, a second copy that agreed until it did not.
⛔ **The argument family was the last one nobody was looking at, and it was the
only one already broken.** Worth remembering when the next vocabulary lands: the
ID gets checked because it looks like an identifier; the ARGUMENT looks like
data and does not.

✔✔ **THE SWEEP IS FINISHED — every author-typed string in the dialogue
vocabulary is accounted for (2026-09-04), and the two that are NOT guarded are
recorded here so nobody sweeps again:**
- `<<spawn_chest "kernel_demo_chest">>` — ⛔ **the verb is a STUB.**
  `cmd_spawn_chest` logs *"(stub; chest spawn consumer pending)"* and returns.
  There is nothing to validate the id against because nothing consumes it.
  ⚠ **And it is called from the DEVELOPER command menu, not from shipped
  narrative** — `kernel.yarn`'s `-> Command: spawn_chest` /
  `hub_guide__test_chest`, one of a row of `hub_guide__test_*` nodes that exist
  to exercise each verb by hand. ⇒ So this is a stub whose only caller is the
  stub-tester, which is the correct state and not a player-facing gap. Checked
  rather than assumed: my first note here said "authored content calls a verb
  that does nothing", which is true and reads as worse than it is.
- `<<play_sfx "ui.notification.discovery">>` — ✔ **the one authored id
  RESOLVES** (`ambition_sfx/src/ids.rs:228`, `UI_NOTIFICATION_DISCOVERY`), and a
  guard over a corpus of one would be vacuous by any honest floor. ⚠ The failure
  mode is real though — `SfxId::new` hashes at the call site, so a misspelling
  plays SILENCE rather than erroring — so this is a "not yet", not a "never".
  Revisit when the authored sfx corpus is big enough for a floor to mean
  something.

⭐ **Measured before the rule: four authored reads over three distinct flags,
every one written** — `bob_field_survey_received` (yarn + `quest.rs`, and the
one LDtk gate), `kernel_guide_demo_flag` (yarn, set and cleared),
`p1_stabilizer_received` (`quest.rs`). It is a ratchet, not a repair.
⚠ **"Writable" is deliberately loose** — the name appearing in any `.yarn`
`set_flag` or as a string literal in any `.rs`. That does not prove anything
sets it at runtime, and the looseness is in the safe direction: it under-reports
rather than over-reports, and the failure it exists for is a TYPO, where the
misspelling appears in exactly one place in the repository.

## ⚠ `body.fits` answers from POSTURE, not from capability — for the first
## author of one (found 2026-09-04, LATENT: no shipped level authors it)

**MEASURED.** `fits` compares `BodyKinematics.size.y` against the authored
opening (`body_conditions.rs:275`), and `size` is the body's CURRENT size:
`BodyBaseSize` exists separately and is documented as *"the player's authored
STANDING body size — the baseline the morph / crouch / slide stances… read
from"* (`body_clusters.rs:222`). ⇒ So the condition answers *"is this body
shorter than the gap right now"*, which is exactly what its own summary says
(*"no taller than this opening"*) — this is not a defect against its contract.

⛔ **And the WALL is re-evaluated EVERY TICK — measured, not inferred from the
function's name.** `sync_authored_gated_lock_walls` is registered on the SIM
schedule in `Platformer2dSimulationPhaseMonolith::WorldPrep`
(`platformer2d_runtime/src/world_gating.rs:42`), after the feature overlay set
and before hazard update — so it runs on every simulation tick rather than once
at room load. ⇒ A `body.fits` gate therefore tracks posture frame by frame: the
solid appears and disappears as the player stands and crouches.
⚠ **What is still REASONED and not measured** is the consequence — what a
re-appearing solid does to a body occupying its space. Both inputs are now
confirmed in source (per-tick re-evaluation, current-size read); the collision
outcome would need a run, and no shipped level authors `body.fits`, so there is
nothing to run it against without writing the level first.

⇒ **The question for whoever authors the first one:** should `body.fits` ask
about the body's POSTURE (as now) or its CAPABILITY — *"could this body get
through, crouching if it can"*? Both are defensible and they gate different
things: posture makes the crouch a required ACTION, capability makes the ability
to crouch a required UNLOCK. ⭐ The second is what
[`../game/systemic-progression.md`](../game/systemic-progression.md)'s
*"why can I go there now?"* is asking for, and the first is what is built.
⛔⛔ **AND ITS SIBLING ANSWERS THE OTHER KIND — the two body conditions are not
the same sort of question, which is the finding rather than the posture itself.**
Measured the same day: `body.can(verb)` reads `BodyAbilities.abilities`
(`body_conditions.rs`, `ability_named(&set.abilities, verb) == Some(true)`) — a
GRANTED capability, persistent, unchanged by what the body is doing this frame.
`body.fits(height)` reads the body's current size.

| condition | reads | changes when |
|---|---|---|
| `body.can(verb)` | `BodyAbilities` — a granted ability | the player ACQUIRES something |
| `body.fits(height)` | `BodyKinematics.size` — current posture | the player CROUCHES |

⇒ **An author reading "the body family" would reasonably expect both to ask
what a body IS ABLE to do.** One does; one asks what it happens to be doing.
⭐ **And only one of them serves the product criterion.** *"Acquire materially
different traversal capabilities"* means a door that opens because you learned
something and stays open — `body.can` does that, `body.fits` opens while you
hold a button. Both are legitimate designs; they are just not the same design,
and nothing currently says which the family is for.

ⓘ Recorded rather than changed, because nothing authors `body.fits` **or**
`body.can` yet — both are among the five conditions the census measures as
authored NOWHERE — so this is a decision the first author should make
deliberately rather than inherit. ⚠ If the answer is "the family asks about
capability", `body.fits` wants to read `BodyBaseSize` (or ask whether any
reachable stance fits) rather than `BodyKinematics.size`.

## ⭐ What "world mechanism (partial)" means, measured 2026-09-04

The family reads as half-built and the half is precise: **a route can be gated
on a LATCHED SWITCH and on nothing else in the room**, because a switch is the
only mechanism with a durable row.

- ✔ `world.switch_on(switch)` reads `save.data().switch(name)` —
  `AmbitionGameSaveData.switches` is one of the fourteen durable families.
- ⛔ **A BROKEN BREAKABLE IS NOT A DURABLE FACT.** `BreakableFeature` is
  rollback-registered (`ambition_combat/src/rollback_registration.rs:191`) and
  appears in the reconstitution census as ECS state — but the save has no
  breakables field, so a smashed wall is SESSION state restored by re-authoring,
  not something a later room can ask about.
- ⛔ Same for an opened door: no durable row, so no question.

⇒ **So "partial" is not "the condition is incomplete" — the condition is
complete over the facts that exist.** What is missing is a FACT, which is the
same sentence [`../roadmap.md`](../roadmap.md)'s P2 tier reaches for the soft
systemic and social/knowledge families: *"What is left is FACTS, not
predicates."* Three of the seven gate families are waiting on durable state
rather than on vocabulary.
⚠ **And that is a deliberate boundary rather than an oversight**: making
breakables durable means every smashed crate becomes save content, which
[`construction-and-reconstitution.md`](construction-and-reconstitution.md)'s
whereabouts ledger explicitly declines for `SpawnOrigin::Dynamic` — *"the
running simulation minted this"*. ⇒ A breakable gate wants an authored,
identified mechanism, not a persisted debris field, and that is a content-model
question rather than a condition-vocabulary one.

## Gate families

- **body capability:** climb, fly, morph, blink, portal use, attack/tool ability;
- **body property:** size, mass class, locomotion type, damage/resistance facts;
- **item/equipment:** physical key, tool, wearable or held capability source;
- **world mechanism:** bridge repaired, machine powered, door physically opened;
- **soft systemic pressure:** danger, difficult traversal, hostile population;
- **social/knowledge:** character cooperation or discovered information;
- **story gate:** explicit authored sequencing, used deliberately rather than as
  the default progression representation.

## Engine/game boundary

The engine owns reusable requirement/capability facts and queries. Ambition owns
which theorems, characters, areas and progression meanings those facts represent.

Do not turn every gate into a generic quest condition. Conversely, if world
interaction, navigation, AI and authoring all independently need the same typed
requirement expression, promote that common vocabulary rather than duplicating
flag checks.

## Candidate crate / Bevy shape

A small capability/requirement vocabulary may eventually deserve its own crate,
but only if body construction, world interactions and reachability genuinely
share it. Prefer typed data and queries over a stringly universal expression
language.

## Open design questions — deliberately unresolved

- Which capabilities belong intrinsically to a body versus participant-level
  permanent progression?
  ⭐⭐ **THE LAYERS THAT EXIST, MEASURED 2026-09-05 — and there is no
  participant one at all:**

| layer | home | kind |
|---|---|---|
| authored defaults | `Platformer2dGameplayDefaults` (asset resource) | what a body spawns with |
| intrinsic | `AbilityBase` (component) | captured at spawn, held constant |
| effective | `BodyAbilities` (component) | what the movement kernel reads |
| session mask | `EditableAbilitySet` (resource) | a RESTRICTION — `effective = base ∩ mask` |
| **participant progression** | — | **does not exist** |

  ⇒ **Everything is body-scoped.** The one non-body layer is an intersection, so
  it can only take verbs AWAY; nothing in the tree can grant one. There is no
  per-participant store, and no `PlayerSlot`-keyed ability anything (searched
  `ParticipantAbilities`, `ParticipantProgress`, `PlayerProgress`,
  `ParticipantCapabilit`, `ProfileAbilities` — zero files each).
  ⇒ ⭐ **So this question and the item-capability one above are the SAME missing
  mechanism**: the `∪ gear/upgrades` union term the `AbilityBase` doc already
  promises. A temporary item grant and a permanent participant unlock are both
  "something outside the body contributes a verb to the effective set", differing
  only in lifetime.
  ⚠ **And Jon's ruling on decision 57 argues against the alternative shape**: a
  participant-level capability STORE would be a second authority over *"can this
  body do X"* at a broader grain, which is exactly what that ruling says not to
  build — breadth composes from the narrow authority. The composition-shaped
  answer is that capabilities stay on the BODY and participant progression GRANTS
  to a body, rather than being a parallel truth something has to reconcile.
  ⓘ That is an argument, not a ruling; it is recorded because the code currently
  has no opinion and the first implementation will set one.
- ✔ ANSWERED FOR ROUTES (2026-09-04): when possession changes bodies, which
  theorem abilities transfer and which do not? **None transfer, because a route
  asks the body a participant is DRIVING** — see the possession ruling above.
  ⚠ Still open for PROGRESSION: whether a participant carries anything across
  bodies at all is a different question and this does not touch it.
- Can an item temporarily satisfy a capability requirement without becoming a
  body capability?
  ⭐⭐ **THE ANSWER IS ALREADY STATED IN THE CODE'S OWN CONTRACT, and the
  structure to do it ships. Measured 2026-09-05.** `AbilityBase`
  (`platformer2d_core/src/body_clusters.rs:124`) is the intrinsic set and
  `BodyAbilities` the effective one, and the doc states the derivation:

  > `effective = base ∩ session_mask` (∪ gear/upgrades once those land)

  ⇒ So yes — an item contributes to the EFFECTIVE set without touching the base,
  which is exactly *"satisfies the requirement without becoming a body
  capability"*. The two-layer split already exists and already earns its keep:
  the F3 dev mask gates a verb off without destroying the authored kit.

  ⛔⛔ **BUT THAT FORMULA HAS THREE AUTHORITIES AND ONLY ONE OF THEM IS THE
  FORMULA.** Nothing computes it; two systems each restate a PIECE of it by
  writing `BodyAbilities` directly:

  - the F3 re-sync (`crates/ambition_dev_tools/src/lib.rs:90`, the `intersect`
    itself; the system is `sync_live_player_dev_edits_system` at `:61`) — primary-only, applies
    `base ∩ mask` every frame;
  - `restore_wall_abilities_after_transit`
    (`ambition_content/src/portal/ability_adapter.rs:83`) — writes
    `effective.wall_verbs = base.wall_verbs` for four verbs, for every body,
    *"restoring from the BASE (not a saved copy) keeps this stateless"*.

  ⇒ **The day a gear term lands, the portal restore silently erases it.** It
  implements `effective = base`, not `effective = base ∩ mask ∪ gear`, so a
  transit would strip an item-granted `wall_climb` and nothing would report it —
  the player would simply stop being able to climb after using a portal.
  ⚠ And the pair is ALREADY reconciled by a sync rather than by structure: the
  restore's own comment says *"if a session mask also gates one of these verbs
  off for the primary, the F3 re-sync re-applies the mask on the next frame"* —
  a one-frame window where the mask is wrong, guarded by nothing.

  ⇒ ⭐ **The elegant precondition to answering this question is to make the
  derivation a FUNCTION every writer calls** — `effective(base, mask, gear)` —
  rather than a sentence in a doc comment that three places re-implement. That
  removes a restatement which exists TODAY (two writers, no gear needed to
  justify it) and makes the gear term impossible to forget rather than a thing
  the next author must remember to add in every writer. That is the change to
  make BEFORE gear, not after.

  ⭐⭐ **AND THE CONCRETE SHAPE, now that both writers have been read.** The
  formula has exactly ONE real implementation today —
  `ambition_dev_tools/src/lib.rs:90`, `base.abilities.intersect(editable.as_engine())`
  — and the portal pair is not a second implementation of it so much as a
  DIFFERENT MECHANISM for the same job: suppress by overwriting the effective
  set, restore by copying the base back.

  ⇒ **The change that removes the class, rather than syncing it: the transit
  should be a MASK, not a write.** `effective = base ∩ session_mask ∩
  transit_mask (∪ gear)`, recomputed by one function. Then
  `suppress_ledge_grab_during_transit` contributes a mask while the latch is
  held, `restore_wall_abilities_after_transit` DELETES ITSELF — there is nothing
  to restore, because nothing was overwritten — and the one-frame window where a
  session mask is wrong closes by construction rather than by the F3 re-sync
  catching up next frame.
  ⚠ **The obstacle is ownership, and it is worth stating before anyone starts:**
  `EditableAbilitySet` lives in `ambition_dev_tools`, so today the mask term is
  owned by a dev-tools crate that `ambition_content` should not depend on. Making
  the derivation shared means the MASK CONCEPT has to move to
  `platformer2d_core` beside `AbilityBase`, with the dev editable becoming one
  contributor to it rather than the definition of it. That is the actual cost of
  this change — not the function, which is three lines.
- How expressive should compound requirements be before they become an
  accidental scripting language?
  ⛔⛔ **NO LONGER PURELY HYPOTHETICAL — a ruling now DEPENDS on an answer that
  is only half true. Measured 2026-09-05.** Jon's decision 57 says breadth
  composes rather than getting its own mechanism, and gives the shape:

  > `boss.cleared("cove.mockingbird") OR boss.cleared("tower.mockingbird")`

  That composes in **dialogue** — a `.yarn` `<<if …>>` is evaluated by the Yarn
  interpreter, which has `or` and `and`, and our conditions are registered
  functions returning bools. ⇒ It does **NOT** compose in a **route gate**:
  `prepare_authored_gate` (`world/gated_lock_walls.rs:355`) reads `gated_by` as
  EITHER one `domain.question` line OR a bare flag name, and `prepare_line`
  (`authored_logic/prepared.rs:131`) splits it into exactly one id plus args.
  **There is no operator anywhere in that road.** A wall asks one question.

  ⇒ **So "any Mockingbird cleared" can gate a CONVERSATION today and cannot gate
  a DOOR.** That is not a defect in the ruling — the ruling is about keys, not
  operators — but it means the composition it points at is unavailable in one of
  the two places gates live, and the first author who needs it will discover that
  from a wall that will not open.
  ⇒ **This question therefore has a due date now**, and the cheapest honest
  answers are worth naming: (a) leave it, and say in the gate's own error that
  compound gates are not authorable; (b) allow exactly `and`/`or` over prepared
  conditions and nothing else — no nesting, no negation, no values — which is the
  smallest step that does not become a language; (c) let a gate name a flag that
  a content system sets from a composed rule, which keeps the gate road simple
  and moves the composition into code where it is already expressible.
  ⚠ I did not pick one; the reason this bullet exists is that picking is Jon's.
- Should "knowledge" ever be an engine fact, or remain Ambition/social AI data?
  ⭐⭐ **HALF OF THIS IS ALREADY ANSWERED BY SHIPPED CODE, in the "engine fact"
  direction. Measured 2026-09-05.** `WorldMemory`
  (`crates/ambition_characters/src/perception.rs:785`) is *"the per-controller
  belief that outlives the viewport (invariant I6)"* — keyed by actor id,
  refreshed for what is seen, DECAYED for what has left view, and forgotten below
  a confidence floor. It is engine code, in `ambition_characters`, not Ambition
  content, and its `update` is pure so it is replay-deterministic.
  ⇒ **The engine already owns TRANSIENT PERCEPTUAL knowledge** — who I have seen,
  where, and how sure I am.
  ⛔ **What has no home at all is DURABLE SOCIAL knowledge** — *"this NPC knows
  you stole the thing"*, *"the village heard about the boss"*. It is in none of
  the fourteen durable save families (version, encounters, switches, bosses,
  quests, flags, dialog_visits, items, wallet, inventory_saved, checkpoint,
  occurrences, custody, minted_items), and `WorldMemory` cannot carry it: it
  decays by construction, which is the correct behaviour for sight and the wrong
  one for a grudge.
  ⇒ ⭐ **So the question splits, and only the second half is open:** perceptual
  knowledge is settled (engine, transient, per-controller); durable social
  knowledge is unowned, and the reason it looks answered is that the word
  "knowledge" covers both. ⚠ Worth splitting the bullet before anyone answers it,
  because an answer aimed at the whole word would either make sight durable or
  make grudges decay.
- ⛔ LIVE, NOT HYPOTHETICAL, and filed as **awaiting-maintainer-decision #54**
  (2026-09-04): how should co-op gates behave when one participant can traverse
  and another cannot? The code answers "the party" today, by an existential
  nobody chose.
- What constitutes a soft gate that AI/navigation should still consider
  reachable?
  ⓘ⭐ **THIS HAS NO CONSUMER TODAY — measured 2026-09-05, and that is the useful
  answer rather than a design.** There is no world-space route planner in the
  tree: no navmesh, no nav graph, no A*. Every one of the 38 files mentioning
  *"navigation"* is MENU navigation (`ambition_ui_nav`, `ambition_input`,
  `ambition_menu`, `ambition_settings_menu`, `ambition_touch_input`,
  `game_shell`, `menu_kaleidoscope`). Perception does line-of-sight and
  line-of-fire against the real geometry, and `WorldView::reachable` — the one
  name that sounds like a route query — is cited in
  `crates/ambition_platformer2d_actor_monolith/src/features/ecs/perception.rs:877`
  as a
  thing that USED to exist.
  ⇒ So an AI sees geometry and nothing else. A standing `GatedLockWall` is
  geometry that blocks; an opened one is absent. **There is no layer that could
  classify a gate as "soft"**, because nothing plans a route that would need the
  classification.
  ⇒ ⭐ The question is therefore PREMATURE rather than unanswered, and the
  actionable form is: *when a route planner lands, the gate classification is
  part of its design, not a thing bolted onto it afterwards.* Leaving it on this
  list without that note invites someone to answer it in the abstract and build a
  taxonomy nothing consumes.
  ⚠ **Method, because this is a NEGATIVE claim and my first search was too
  narrow.** Grepping `navmesh|NavGraph|pathfind|path_to` returned nothing, which
  would have been right by luck. Widening to `astar|a_star|waypoint|navigation|
  reachability` returned APPARENT refutations — 5 files with `a_star`, 8 with
  `reachability` — every one a substring false positive
  (`a_starting_character…`, `unreachable!`, prose). ⇒ A widened search can
  manufacture false POSITIVES that talk you out of a true negative, exactly as a
  narrow one manufactures false negatives. Both halves need reading, not
  counting.
