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

⛔ **The fourth was the hole, and it is invisible to the other three by
construction.** `world.flag_set` takes an author-typed flag NAME, so a misspelt
one parses, names a published condition, evaluates successfully — **and answers
NO for the rest of the game**, because nothing will ever set a flag by that
name. ⇒ A wall gated on it never opens and a dialogue branch behind it is
unreachable, and both look like content nobody wrote rather than content that is
broken. The first three guards are all about the CONDITION; only this one looks
at the FACT.

⭐ **Measured before the rule: four authored reads over three distinct flags,
every one written** — `bob_field_survey_received` (yarn + `quest.rs`, and the
one LDtk gate), `kernel_guide_demo_flag` (yarn, set and cleared),
`p1_stabilizer_received` (`quest.rs`). It is a ratchet, not a repair.
⚠ **"Writable" is deliberately loose** — the name appearing in any `.yarn`
`set_flag` or as a string literal in any `.rs`. That does not prove anything
sets it at runtime, and the looseness is in the safe direction: it under-reports
rather than over-reports, and the failure it exists for is a TYPO, where the
misspelling appears in exactly one place in the repository.

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
- ✔ ANSWERED FOR ROUTES (2026-09-04): when possession changes bodies, which
  theorem abilities transfer and which do not? **None transfer, because a route
  asks the body a participant is DRIVING** — see the possession ruling above.
  ⚠ Still open for PROGRESSION: whether a participant carries anything across
  bodies at all is a different question and this does not touch it.
- Can an item temporarily satisfy a capability requirement without becoming a
  body capability?
- How expressive should compound requirements be before they become an
  accidental scripting language?
- Should "knowledge" ever be an engine fact, or remain Ambition/social AI data?
- ⛔ LIVE, NOT HYPOTHETICAL, and filed as **awaiting-maintainer-decision #54**
  (2026-09-04): how should co-op gates behave when one participant can traverse
  and another cannot? The code answers "the party" today, by an existential
  nobody chose.
- What constitutes a soft gate that AI/navigation should still consider
  reachable?
