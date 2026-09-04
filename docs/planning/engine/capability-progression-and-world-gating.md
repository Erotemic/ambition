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
> is reachable from a route now** (`inventory.holds`, `held.is_held`), guarded by
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
> what the collision doctrine sweeps; the base size is the authored standing
> baseline stances derive FROM. A gate that asked the baseline would refuse a
> body that physically fits, which is the world disagreeing with itself about a
> hole in it. So crouching and morphing count, and that is a consequence of the
> existing effective-versus-authored rule rather than a new ruling.
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
> ⇒ **Six of the seven gate families are reachable from a route now**: story
> gate (two writers), item/equipment (`inventory.holds`, `held.is_held`), body
> capability (`body.can`) and body property (`body.fits`). What remains
> route-facing-empty is **soft systemic pressure**, **social/knowledge**, and the
> world-mechanism family's general *"mechanism is in state X"* fact — the last of
> which is a missing FACT rather than a missing condition, and is therefore the
> only one of the three that is a seam to negotiate rather than a predicate to
> write.
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
- When possession changes bodies, which theorem abilities transfer and which do
  not?
- Can an item temporarily satisfy a capability requirement without becoming a
  body capability?
- How expressive should compound requirements be before they become an
  accidental scripting language?
- Should "knowledge" ever be an engine fact, or remain Ambition/social AI data?
- How should co-op gates behave when one participant can traverse and another
  cannot?
- What constitutes a soft gate that AI/navigation should still consider
  reachable?
