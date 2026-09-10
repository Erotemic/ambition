# The queue — live execution order

This file is the **current executable engineering queue**. It is not a work log,
review transcript, campaign archive or place to preserve completed investigations.
Git history owns those records, including intentionally retired epochs in
[the cold history store](repository-history.md).

A row stays here only when an engineer can act on it without first reconstructing
weeks of context. Durable design belongs in the linked owner document. Product
questions belong in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

**Architecture review source:** `300004d601af1e633cfaee969f079cf9bb368ca8`
(2026-09-08 committed archive). Revalidate before changing a newer head. The
review made no Rust execution claim; see the coverage receipt.

## P0 - characterize and repair current correctness gaps

### A1a - DONE 2026-09-08

`restore_checkpoint_on_session_start` latched `routed_for` before asking the
lifecycle slot and threw the `Admission` away, so a refused crossing spent the
session's one resume and stranded the player in the room the session opened in.
Fixed by latching only on `Admission::admitted()`; guarded by
`a_refused_slot_leaves_the_checkpoint_resume_retryable` (poison verified) and the
missing-subject arm beside it. A1b then moved the state, systems, installation
and tests out of `shrine`/item-pickup into `session::checkpoint`. F9's executed witness landed
with it: on a refused reset the entitlement ledger rolls back **and the object
acquired after the checkpoint is destroyed outright**, because custody
restoration and the room reconstruction that would re-author it fall on opposite
sides of an admission neither consults. Measurements and the A1c obligation are
in the [protocol](engine/checkpoint-restoration-protocol.md#f9-measured-2026-09-08-executed-full-checkpoint-horizon-composition).

**Standing prohibition:** no consequence of a lifecycle request may be written
before the slot has said yes.

### A2 - unify projectile contact geometry and obstruction semantics

**Owner:** [projectile contact protocol](engine/projectile-contact-protocol.md),
packet A2 and findings F2/F3.

Establish shared geometry, actual travel legs and finite-shape obstruction order
before replacing family dispatch. Preserve synchronous interception and later
hit reception. Carry collider contributor identity: a destructible's own wall
and its hurt shape can be one compound contact, not competing unrelated targets.

**A2a landed 2026-09-09** (one boss hurt geometry, published once at the
damage-facing sample; the catalog and the attack/animation inputs left the
consumers with the derivation). **A2b's obstruction half landed**: the travel leg
is captured rather than reconstructed, and both branches ask one swept question
with the shot's own box and world-hit policy. Receipts in the owner document.

**Swept target contact landed 2026-09-09**, with candidate ordering by time of
impact and the targeted hit event carrying the box AT CONTACT — the delayed
applier re-tests that volume, so a shot that genuinely crossed its target used to
fail its own re-test and land nothing.

**The boss/breakable branch is swept too, 2026-09-09**, and the receiver-neutral
returning-shot lifetime landed with it as its own semantic change: that branch
despawned every shot that reached a boss or a breakable, so the same boomerang
came back from a body and vanished into a crate.

**Direct-before-splash and targeted delivery both landed 2026-09-09.** A
projectile's direct contact now names its boss or breakable recipient
(`HitTarget::Feature`, schema 177) instead of broadcasting a volume the applier
re-scans — which damaged every breakable that volume overlapped and let query
order pick which part of a multi-part boss was credited.

**The family-predicate deletion gate closed 2026-09-09**: all three discrete
`ecs_hit_event_hits_*` predicates had no production caller once contact became
swept, and are gone. One of them had none beforehand either — reachable, tested,
unreached — so its rule moved onto the function every consumer actually calls.

⚠ That paragraph and the ones above it once read as though every A2 correction
had landed. They had not: world-versus-target ordering was still split when they
were written, which is the defect the review found. The claim is scoped to the
deletions and the swept-contact corrections it actually names.

**A2b was REOPENED and re-closed 2026-09-09 (GPT review #7).** The ordering the
protocol asks for was still split three ways, and the review was right:

- the BOSS/BREAKABLE branch compared nothing. It computed a swept contact and, if
  it found one, emitted the targeted hit and the splash and `continue`d — so the
  world sweep ran only when NO feature was reached. The ordering was *"feature
  contact, else world"*, and a crate or a boss standing behind an unrelated solid
  was struck through it;
- the BODY branch asked the right question of the wrong geometry: it swept from
  the muzzle to the victim's CENTRE, which answers *"is a wall before the
  victim's middle?"*. On a wide body the shot reaches the near face first, and a
  wall between that face and the centre refused a hit that physically happened;
- and the world branch swept a third time for its own pull-back.

⇒ ONE sweep over the shot's actual travel leg, its finite `time_of_impact` kept
in the leg's own [0, 1] parameter and compared directly against every body's and
every feature's contact time. The pull-back reuses the same result instead of
re-sweeping.

⛔ THE TIE RULE WRITTEN HERE WAS WRONG AND IS CORRECTED BELOW (review #8). It
said a tie goes to the TARGET and called that "the strict comparison the body
branch already used rather than inventing a policy". Inventing a policy is
exactly what it was: the protocol awards an equal-time tie to an independent
blocking surface. Three witnesses, each poison-verified
against the exact code it replaced: a crate behind a wall is not broken, the same
crate with the wall moved PAST it still is (the anti-vacuity floor moves the wall,
not the crate, so an arm that broke nothing would fail it), and a wall standing
inside a wide body past its near face no longer saves it.

**A2's remaining item is deliberately not taken.** A cohesive
`projectile/contacts.rs` <!-- cite-ok: proposed module path --> inside the same
crate removes no authority and no dependency edge — the code would import what it
imports now and be reachable by the same callers — and a same-crate move that
names neither is churn by this repository's own test. It becomes worth doing when
it enables a deletion: a `pub(crate)` boundary, or a split that lets flight state
leave for `ambition_projectiles` without the victim queries following. The semantic corrections and
deletion gates the owner document requires have landed, INCLUDING the finite-time
ordering rule reopened by review #7 above.

**A2b's FINITE ORDERING is closed; compound-contact acceptance is DEFERRED to
Q96/A5, and two narrow holes stay open (review #9).** Stated that way because
this row previously read as closed and not-closed at once — it said "RE-CLOSED",
then that the compound-solid row was not closed, and then listed compound solid
object under acceptance anyway.

**CLOSED 2026-09-10 — the tied world witness is authoritative (`1967a03d4`,
ToothbrushAmbition).** `resolve_world_collision` re-answered "what stopped this
shot" by scanning `world.blocks` for an endpoint overlap, solids before one-ways,
after `first_body_sweep` had already ordered every candidate over the leg. The
two priorities are different, so ordering and physics could name different
colliders. Fixed by carrying the whole `SweepHit` — the ordering wants the time,
the pull-back wants the centre, the response wants the collider — and giving the
resolver the selected collider to dispatch on, with no re-admission. Guard:
`a_shot_resolves_against_the_collider_the_sweep_selected_not_the_harder_one`,
measured RED (0 surviving bodies) before the fix and green after.

⭐ **AND THE FIXTURE NEEDED A SELF-VERIFYING ARM, which is the transferable
lesson.** A bouncing fireball's half-extent is `(12, 9)`, not square, so
hand-placed faces missed the intended tie by 3px and the wall won outright at
t=0.42 — the test would have gone green over a tie that never happened.
Asserting that the sweep picks the platform BEFORE asserting behaviour is what
caught it. Any tie fixture wants that arm.

**CLOSED 2026-09-10 — an unidentified victim beat an identified one
(`c2188fa7a`, ToothbrushAmbition).** `StrikeVictim.sim_id` documents "a body
without one still gets hit, it just cannot win the tie"; both resolver sites
implemented it as a bare `Option` comparison, and `Option` orders `None` FIRST —
so the comparison said the opposite of the sentence written above it. Fixed with
ONE authority, `ambition_combat::hitbox::victim_identity_key`, read at
systems.rs:892 and :930, ordering absent identity last, with rank as a leading
field rather than a sentinel string so an authored id cannot collide with the
stand-in. `sim_id` stays `Option`: as a Bevy `QueryData` field, requiring it
silently drops unidentified bodies out of the query rather than failing
construction — a gameplay change hidden inside a determinism fix. Guard:
`an_unidentified_victim_does_not_beat_an_identified_one`, measured RED under both
spawn orders; the existing stacked fixture could not express it because it
hand-installs `SimId` on both bodies.

⭐ **AND THE `debug_assert` WAS THE CENSUS.** The genuinely undecidable case — two
coincident victims, neither identified — is a `debug_assert` naming both
entities, chosen over a runtime census because an assert that never fires IS the
measurement, and answers the question that matters ("does this happen on any road
we exercise?") rather than the one a static scan can answer. It fired ONCE across
1117 tests, in a fixture (`a_seated_fighters_shot_hits_a_same_faction_body_on_another_team`,
two bodies on one point with no `SimId`), now identified. ⚠ It fires on ORDERING
ambiguity, not OUTCOME ambiguity — that fixture's team filter made the order moot
— so a future firing is a prompt to look, not proof of a live defect. A
bundle-shaped static scanner had reported "2 of 2" and was describing its own
method; it was deleted rather than committed.

**A2 REOPENED AND RE-CLOSED AGAIN 2026-09-09 (GPT review #8), `dc2fe7ce7`.**
Three defects, one cause: the road had a contact order that was not the
protocol's, and then did not use its own answer.

- **The tie went to the target, with an epsilon.** `wall < contact - f32::EPSILON`
  is the opposite of the protocol's rule AND the epsilon comparator it forbids by
  name; at ~1.19e-7, twice the float spacing at 0.5, it also swallowed walls that
  genuinely were first. Now an exact `is_le`, extracted as `wall_reaches_first`
  so the tie can be asserted directly — two swept code paths producing
  bit-identical `f32` is not something a fixture can promise.
- **Body-vs-feature was family knowledge wearing a comparison.** Same epsilon,
  and at a tie the body won because its loop owns the equality case. Both
  families now share ONE order: time, then position, then stable authored
  identity. Neither is privileged; only the WORLD is, and only at an exact tie.
- **Obstruction and response disagreed about one-ways.** The sweep excluded every
  `OneWay` for a `Bouncing` shot while the response bounces off one the shot
  descends onto. Both now read `shot_policy_admits`; poisoning it reddens the new
  descending test and the OLD bounce fixture together.
- **The selected witness was not authoritative.** The pull-back was skipped
  whenever the endpoint overlapped anything, so a shot could order targets by
  wall A and physically resolve at wall B.
- **The splash detonated at the tick endpoint**, not the contact — a
  gameplay-visible area attack centred in the wrong place.
- `first_body_sweep` handed its own equal-TOI ties to `self.blocks` order; now
  time, then position, then the durable `GeoId`.

⚠ WHAT IS DELIBERATELY NOT CLOSED, stated rather than implied: the acceptance
matrix's COMPOUND SOLID OBJECT row. A genuine compound contact — a destructible's
own collision surface and its damageable volume as ONE contact rather than two
competitors — needs stable collider-contributor identity. That is A5
infrastructure, and Q96 has to be decided first.

⭐ **AND THE CASE IS NOT REACHABLE ON THIS ROAD AT ALL, measured 2026-09-09 —
which is WHY the protocol's tie rule could be implemented exactly, with no
contributor identity: every block this sweep can return is by construction an
INDEPENDENT blocker.** A
breakable authored `BreakableCollision::Solid` DOES publish a `BlinkWall` block —
`world/overlay.rs` writes it into `FeatureEcsWorldOverlay::blocks` — but
`ambition_projectiles::collision_world::ProjectileCollisionWorld::solids()`
composites only `gate_solids`, `portal_carves` and `removed_block_names`.
`overlay.blocks` (every ECS breakable surface and every pogo orb) is not in the
world a projectile sweeps. So a solid crate's own surface is not a wall this road
can hit, and the tie rule has no compound case to get wrong yet.
⛔ A first attempt to witness the compound case here produced a test that passed
under a deliberately broken comparison TWICE — once because the fixture never
published the surface, and once because the shot's landing splash broke the crate
whether or not the direct hit landed. It was deleted rather than kept as a green
row that measures nothing. Whether a projectile SHOULD collide with an ECS
breakable's published surface is a separate open question, not this packet's.

**Acceptance:** authored-empty geometry, thin wall/target, equal-time ties,
reflection/absorption, returning shots and rollback have explicit production-road
outcomes. ⚠ COMPOUND SOLID OBJECT is deliberately NOT in this list — it is
deferred to Q96/A5 and was previously named here while the same row said it was
not closed. The initial sampled-target sweep is not a
claim of full moving-target CCD. No second family query chooses the victim.

### A11/A12 - make authored technique admission truthful and bounded

**Owner:** [authored technique admission](engine/authored-technique-admission.md).

**A12a landed 2026-09-09.** The flow validator now rejects an infinite timeout
(`f32::INFINITY > 0.0` is true, so the mandatory-timeout check admitted the exact
value it forbids), a graph past 256 nodes (a cursor bound: `flow_node` is a `u16`
written with a narrowing cast, so node 65,536 silently becomes node 0), any cycle
reachable from node 0 (`reaches_finish` is existential, so a branch that
terminates on one road and loops on the other passed), and a node nothing arrives
at. The flow-owned-lifetime prose is corrected against the runtime: the timeline
ends the move, so none of these traps a fighter — they lose authored steps, and a
cycle fires a technique N times where the author wrote one.

**A11a's support authority landed 2026-09-09.** `TechniqueSupport` refuses a
second claim on a technique key and refuses an authored key nothing installed
declares; `install_technique` adds the handler system and declares its key in one
statement, so a capability cannot install one without the other. Four handlers
converted. It replaces `ParamSchemaRegistry`, which admitted unknown keys by
design, overwrote duplicates silently, and had zero production callers — so a
misspelled effect key reached the runtime and became a warning mid-fight.

**A11b's exhaustive visitor landed 2026-09-09.** `MoveSpec::effect_refs` returns
every authored effect with the path it sits on, destructuring without `..` at
every level so a fifth reference site is a compile error rather than a silent gap.
It replaced two hand-kept lists that each read two of the four sites — including
the bare-specials census whose own doc records it having already missed a site
once, and the held-item guard, where the miss means shipping a placeholder quad.

**Key declaration is COMPLETE, 4 -> 22 (`a596abd25`, `42766c8c1`).** Every
technique the shipped composition installs declares the key it answers, engine
and game alike; `install_techniques` (plural) exists because one handler can
answer several keys. The last of them was `pogo_bounce`, an ENGINE technique
authored by 36 characters and declared by nothing — found by walking the prepared
corpus, not by grep, which could not see it (not in the `smash.` namespace,
authored through prefabs, reached at an on-hit site).

**ADMISSION IS NOW REAL, 2026-09-10 — the three things GPT review #9 named are
done.** The review rejected `6a692b6d2` correctly; what replaced it:

1. **Rejection GATES publication.** A refused definition is WITHHELD, per
   definition, with the rest of the cast still published — the literal reading of
   "invalid or uninstalled calls cannot publish definitions". Refusals carry the
   character that owns them, which is what makes withholding possible rather than
   merely reportable. Poisoned: stop withholding and only the "not published"
   test dies; all three controls, including "the rest of the cast is still
   published", hold.
2. **`InstalledTechniques` is back in `ambition_combat`.** Moving it into
   `ambition_characters` was dependency convenience wearing ownership's clothes —
   my commit message admitted it ("THE TABLE HAD TO MOVE DOWN A CRATE FOR THE
   CHECK TO EXIST AT ALL"). The runtime now passes `TechniqueSupport` in as an
   ordinary argument, and the checked barrier is ordered explicitly
   `.before(close_preparation_barrier)` — the `finalized` guard makes ORDER the
   guarantee, so leaving it to plugin-registration order would be a silent bypass.
3. **The predicate carries the site.** `admit_at` takes the `EffectSite` the
   exhaustive visitor already returned, checked against a `TechniqueDelivery`
   road per declaration — verified for all 23 declarations against the
   `MessageReader` their handler actually uses, not assumed. Two declarations
   swapped `check_hydrates::<T>` for the domain's own validator, so `{scale: 2.0}`
   is refused at admission rather than mid-fight. `TechniqueRefusal::Disabled` is
   deleted as unreachable vocabulary, with Q97 named as what would restore it.

⭐ **AND CAPTURE MOVED TO THE ENGINE, which is what made refusal affordable.**
Turning refusal on reddened seven application tests, every one a `smash.*` key in
a composition without the Smash demo. Measured by state and behaviour ownership
rather than by where the request types sat: `CapturedBy` and TWELVE behaviour
systems are `ambition_combat`'s, the vocabulary is `ambition_characters`', and
the demo owned ONE function that is a field-for-field `hydrate -> write` with no
ruleset policy in any arm. The old module argued for the split — "a ruleset knows
what its own authored strings mean" — and the argument does not survive its own
premises, because the strings are engine constants. `mary_o`'s six grab and throw
moves now work wherever combat is composed.

⛔ **TWO MISTAKES IN THAT MOVE, both worth keeping.** Moving the translator
WITHOUT its chain put it in a different set from its consumers, so a request
written after `acquire_captures` would be consumed a tick late — the mechanic
moves together or not at all. And the four request channels were registered ONLY
by the demo (the `ambition_combat` registrations are `#[cfg(test)]` fixtures), so
installing the systems without them panicked twelve application tests with
"Message not initialized" — a composition fact living in a ruleset, from the
registration side. The demo's own comment had already written the rule: a system
that writes four messages does not run in a world that registers three.

**A11c LANDED 2026-09-10, and with it the acceptance row that could not be
written.** "Rejection leaves the active generation unchanged" was untestable
because there was no production republication road at ALL:
`PreparedCharacterRegistry` has one production writer, guarded to run once, and
`stage_authored_character` PANICS after the barrier closes — so nothing could
produce a second generation to leave unchanged. Three attempts at that fixture
stopped on three different obstacles; this was the prerequisite all three were
missing.

The panic's own sentence was the design: *"a later cast change is a separate
explicit transaction."* `stage_character_revision` contributes an edit;
`activate_staged_revision` folds it over the LIVE cast, admits the whole
candidate, and either publishes under a new generation or refuses.

⚠ **AND THE REVISION RULE IS NOT THE BARRIER'S.** At initial activation there is
no last-good, so a refused definition is withheld and the rest of the cast still
publishes. A revision is a TRANSACTION over a cast that is already live: applying
half of it would leave a session in a state no author asked for, so the whole
edit is refused and the previous registry — generation included — stays
published. That is the last-good-prepared-definition retention the packet
requires. It is NOT last-good-WORLD retention, which the packet explicitly does
not ask for.

⚠ Admission runs against the whole CANDIDATE, not the edit alone: a summon in an
edited move may name a character the edit did not touch, and an edit may remove
the definition an untouched move was naming.

Four tests, two of them controls. Poisoned: let a refused revision publish and
only `a_refused_revision_leaves_the_active_generation_unchanged` dies, with the
premise arm (an admitted revision DOES bump the generation) and both controls
holding.

⭐ The panic also cited `docs/archive/planning-superseded/…`, a directory deleted
2026-09-05 — a production message pointing a developer at nothing, read at the
exact moment they hit it. It now names the mechanism instead.

**NESTED REFERENCES LANDED 2026-09-10, closing A11.** Three technique params
name another authored definition and preparation resolved none of them:
`SummonRideParams::character_id` and the two `item_id`s. Each one was a move that
hydrates, declares its key, passes every guard in the tree, plays its animation,
and summons a character or drops an item that does not exist. `NestedReferences`
is declared BESIDE the handler like the key and the params already are —
`Characters(fn)` / `HeldItems(fn)` / `None` — so the barrier resolves what a
technique names without knowing what any technique is.

⛔ **I RECORDED THE TWO `item_id`s AS UNCHECKABLE AND THAT WAS WRONG.** The note
that stood here said `ambition_characters` "cannot see the item vocabulary — the
same layering wall the support table had to cross." There is no wall:
`held_item_by_id` is a static registry in `ambition_characters::brain::action_set`,
the SAME CRATE as the barrier. I inferred the obstacle from the shape of an
earlier problem instead of measuring it, and a deferral written that way expires
the moment someone believes it — this one would have parked a real gap behind a
reason that was never true. ⭐ A deferral's REASONS expire like a measurement
does: re-derive them, don't re-read them.

Guards: three unit tests in `prepared_tests::held_item_references` (extractor,
admitted, refused-at-preparation) — poisoned by making `held_items()` return
empty, which kills the refusal test alone while both controls hold. Plus
`installed_techniques_are_declared::the_techniques_that_name_other_definitions_declare_that_they_do`,
asked of the BUILT APP, because a key can be declared while the thing it names
goes unchecked and no other guard in the tree can see the difference.

⛔ SUPERSEDED — what review #9 named, kept for the shape of the errors:

1. **Rejection must gate publication.** The pass currently logs refusals and then
   publishes the offending definitions anyway, so the moves "play and do nothing"
   exactly as before. The owner contract requires the checked result before
   active-definition publication; the implementation must preserve the last-good
   registry or refuse initial activation.
2. **`InstalledTechniques` must go back to composition/runtime ownership.** It was
   moved into `ambition_characters` because preparation could not see upward —
   dependency convenience, not ownership, and the same category of error as the
   earlier `actor_spawn` mistake. The fix needs no new abstraction: runtime/combat
   keeps the resource and invokes a FALLIBLE preparation/publication function with
   the support table as an ordinary input.
3. **The predicate is weaker than the contract.** `admit` takes only an
   `EffectRef`, so it discards the `EffectSite` the exhaustive visitor returns —
   `pogo_bounce` authored in a timeline/sustain/flow slot passes validation and is
   consumed by nothing. And most declarations check `check_hydrates::<T>` rather
   than the domain's own validator (`TimeDilationParams::problems` rejects
   `scale >= 1`; the declaration does not, so `{scale: 2.0}` is admitted and
   refused only at fire time). `TechniqueRefusal::Disabled` is dead vocabulary
   for the same reason: an absent key is always `Unknown`.

**A12b's structural half landed 2026-09-09.** `FlowNode`'s edges are the runtime
cursor's own `u16`, so the interpreter's narrowing `as u16` — which turned an
authored edge of 65,536 into node 0, a terminating flow into a per-tick loop, with
nothing in the pipeline able to report it — is gone with the type that allowed it;
the 256-node bound is a budget again rather than a stand-in for cursor safety. The
witness sits at the deserialization boundary because that is the only road the
defect was reachable on: every in-repo flow is Rust literals. `MovePlayback::spec`
is an `Arc<MoveSpec>`, so a move can no longer edit the definition it is executing
(the contract's rule, previously unenforced) and the per-tick deep clone of the
authored graph is deleted. `TechniqueFlow::successors` is now the one edge
enumeration its own doc claimed it was — `reaches_finish` and the dangling report
each had a second and third copy.

**A11b's rejection half was RE-DERIVED 2026-09-09, and most of it is already
true or not yet reachable.** The three parts, separated because they have
different answers:

1. **At the registration door it holds, and is witnessed.**
   `stage_authored_character` assembles a CLONE of `StagedCharacterOverrides` and
   publishes it only on success, so a `DuplicateId` whose `candidate.insert`
   already mutated that clone drops it. `two_providers_cannot_author_the_same_stable_id`
   and `two_characters_cannot_present_under_the_same_display_name` both assert
   the previous authority survives, and the first would redden if the candidate
   became an in-place `ResMut`. Nothing to add.
2. **"Generation unchanged" is not writable as a non-vacuous test yet.**
   `PreparedCharacterRegistry` has exactly ONE production writer —
   `finalize_cast`, at the barrier, guarded to run once — SEALED by grep for
   `insert_resource`/`ResMut` of that type across the workspace. `insert_prepared`
   is `#[cfg(any(test, feature = "test-support"))]` and every non-test caller of
   it turned out to be inside a `#[test]`. So the generation cannot move during a
   session at all, and a rejected registration happens BEFORE the barrier where no
   registry exists to be unchanged. The assertion would be true of a harness, not
   of the architecture.
3. ⇒ **The acceptance row "edit rejected during active play" needs A11c FIRST,
   not after.** There is no production republication road: `stage_authored_character`
   PANICS after `finalized`, and nothing else writes the registry. The candidate/
   activation mechanism A11c describes is the prerequisite that fixture has been
   waiting on, which is why three attempts at it stopped on different obstacles.
   ⚠ `project_prepared_character_definitions` DOES compare generations in
   production — a reader for a transition only a test can cause.

Still open in this lane: domain nested-reference policies (measured: three
technique params name another authored definition — `SummonRideParams::character_id`,
`DropBombParams::item_id`, `PlaceMineParams::item_id` — and preparation checks
none of them; a summon naming an unknown character is refused at FIRE TIME by
`preflight_planned_bodies` with an error log, which is the runtime failure this
packet exists to move to preparation), the public production insertion paths, and
A12b's remainder — of which "constructors private" is already effectively true
(`PreparedCharacterDefinition` has five private fields, so no external struct
literal exists, and its one construction site is inside `prepared.rs`), while
"fallible" and the pinned REVISION both wait on the same A11c mechanism.

**BLOCKER #3's SECOND HALF CLOSED 2026-09-10: the declarations were weaker than
the domain in two different ways, and only one of them was per-technique.**

⛔⛔ **THE GENERAL ONE FIRST, because it is not a fact about any technique.**
`NaN`, `inf` and `-inf` are valid RON and hydrate cleanly, so
`check_hydrates::<T>` — which is what TWENTY of the twenty-three shipped
declarations use — admits all three. MEASURED before the fix: `(amount: NaN)`
parses, hydrates to `FillMeterParams { amount: NaN }`, and is admitted.

⇒ What that buys is not one misbehaving move. A non-finite float reaching
gameplay state POISONS IT PERMANENTLY: `ResourceMeter::refill` is
`(current + amount).clamp(0.0, max)` and `f32::clamp` returns `NaN` for a `NaN`
input, so one authored fill leaves the meter `NaN` forever — every later
comparison against it false — and `body.mana` is ROLLBACK-CANONICAL, so the
poison is snapshotted and restored across every rewind. The same holds for any
authored position, velocity or radius.

⭐ The refusal is STRUCTURAL and sits in `admit_at` BEFORE the declaration's own
predicate, because "no authored field may hold a non-finite number" is not
something each declaration should have to remember — the same mistake as making
each one remember its key. It walks the `ron::Value`, so it covers nested tuples
(`half_extents[1]`), `Some(NaN)`, and a technique added tomorrow.
⚠ Integers cannot fail it: `Number::into_f64` maps every integer variant to a
finite `f64`, and there is a test pinning that, because a bug there would reject
`damage: 4` and take the whole roster down.

⛔ **THE PER-TECHNIQUE ONE: a domain rule that only the Rust authoring road asks
is not a rule.** `SteeredBoltParams` had three — a bolt must draw something, its
trail must be redrawn at some interval, and it must be steerable — and all three
were `assert!`s inside `author_steered_bolt`, the helper Rust content calls. A
bolt arriving as an ordinary `EffectRef`, which is every other way a technique is
authored, was checked only for hydration. `SteeredBoltParams::problems` is now the
one authority and BOTH roads ask it: the helper asserts on it, the declaration
refuses on it, and a test holds them to the same answer so a rule cannot be added
to one and forgotten in the other.
⚠ `at_s` stayed an assert deliberately — a bolt fired past its move's duration is
a fact about the TIMELINE, and the params road has no move to compare against.

⭐ The three declarations that ALREADY asked their domain — time dilation,
riposte, pogo bounce — are unchanged. `smash_limit`'s `problems()` turned out to
be on `LimitMeterFill`, not on the params type, so it was never the missing
validator it looked like; `FILL_METER` has no shipped customer at all, so its
negative-amount question (a "fill" that drains, when `MoveGates::meter_cost`
already owns spending) is recorded here and not invented into a rule.

**A12's REMAINING ROW, MEASURED AND OPEN 2026-09-10: "late contact feedback
cannot mutate another move occurrence" IS NOT HONOURED, AND CANNOT BE.**
`mark_move_playback_resolved_hits` keys on the ATTACKER ENTITY alone, and
neither `ResolvedBodyHit` nor `BlockedBodyHit` carries an occurrence — so a
verdict lands on whichever use is wearing the playback when it arrives. Witness:
`a_late_connect_is_not_credited_to_the_move_that_replaced_the_one_that_earned_it`,
`#[ignore]`d as a known-open defect with a passing control beside it proving the
verdict does arrive.

Both ends of the window are the code's own statements: the verdict is ONE FRAME
LATE against a player victim (`connected_hit`'s doc), and a replacement playback
can appear IN THE SAME UPDATE (`instance`'s doc).

⛔ **THE ROAD I FIRST BLAMED CANNOT DO IT, AND CHECKING THAT IS THE FINDING.** I
had the mechanism as an OnHit cancel firing on the overlap frame and stranding
the verdict on its successor. `CancelCondition::OnHit` is `contact.connected` —
an OnHit cancel WAITS FOR the very verdict it would strand, so by the time it
can fire the credit has already landed correctly. The reachable replacements are
the ones that do not consult the verdict: a `CancelCondition::Always` window, or
jab #1 simply ENDING before the verdict drains. ⇒ Had I shipped the first story
it would have been a fabricated mechanism in a delivered doc, sitting on top of
a real defect.

⚠ **STILL UNPROVEN: that a shipped configuration reaches this in a running
match.** The fixture measures the MECHANISM. Nobody should quote it as a live
in-game defect until a composed harness reaches it.

What a stale connect buys, if it lands: `connected_hit` feeds
`MovePlayback::contact()`, which feeds BOTH the cancel road (`cancel_permits` /
`cancel_successors`) and `FlowSignal::Connected` — so the successor is
OnHit-cancellable having touched nothing, and an authored flow takes its "it
worked" road on a strike that never landed.

⭐ The repository already fixed this bug once, on the other side of the glass:
`instance` exists because the inspector "credit[ed] the FIRST instance's contact
to the second". The read model learned it; the runtime did not.

**THE FIX, SCOPED — and its cost is why it is its own item.** Occurrence
identity has to reach the verdict, and the carriers are not free:

| hop | change | literals |
|---|---|---|
| the box that struck | a SEPARATE component beside `Hitbox`, so no `Hitbox` literal moves | 0 (+1 spawn site) |
| `HitEvent` | `attacker_move_instance: Option<u32>` | 57 |
| `ResolvedBodyHit` / `BlockedBodyHit` | same field, via `publish_resolved_hit` / `publish_blocked_hit` (one production writer each) | 13 |
| `mark_move_playback_resolved_hits` | compare against `pb.instance` | 1 |

⚠ Neither `Hitbox` nor `HitEvent` derives `Default` and NO literal uses
`..Default::default()`, so every one is a hand edit — measured, not estimated.
⛔ AND A PARTIAL FIX IS WORSE THAN THE HONEST GAP: gating on `pb.landed_hit`, or
stamping "a verdict is pending for use N" on the body, both pass the fixture
above and both still misattribute when the SUCCESSOR has overlapped something of
its own. The channel has to carry the occurrence.

**Acceptance:** invalid/uninstalled calls cannot publish definitions; rejection
leaves active generation unchanged; existing 3-/4-node flows retain their traces.
Finish does not remove recovery, Wait does not extend the move, and late contact
feedback cannot mutate another move occurrence. No generic execution registry.

## P1 - ownership and independently testable composition

### A1c - DONE 2026-09-08; A1 CLOSED by review #7, 2026-09-09

**Owner:** [checkpoint restoration protocol](engine/checkpoint-restoration-protocol.md).

**Signed off 2026-09-09.** Review #7 accepts A1 as closed: the rollback host
checks `LocalSyncTest` ownership before terminalizing a host-local preparation
failure, respects the confirmed-frame boundary, and the abandonment note carries
enough to reject a stale note from a rewound branch rather than matching a reused
operation key. The unconfirmed-abandonment and key-reuse tests are named as the
right witnesses. Delete this row when the next queue pass compresses it.
A1b (ownership move) and A1c subcommits 1-2 landed 2026-09-08: no domain reads
the raw `ResetToCheckpoint` any more, a refused request changes no domain state,
a refused request is remembered rather than lost, and a no-item checkpoint
composition works.

Subcommit 3's preparation half landed 2026-09-08: the accepted operation now
outlives its frame, carries the occurrence/minted inputs pinned at admission, and
room loading derives both the prefetch cache key and the fresh plan from it
instead of from a live ledger the restore had swapped in order to be read.

Subcommit 3b landed 2026-09-08: destructive domain application left ordinary
speculative simulation for `CheckpointDomainApply`, a schedule only a commit
executor runs, with its inputs installed for that schedule's duration and removed
on every path. Both executors are witnessed and poison-verified. The transitional
`AdmittedCheckpointRestore` token was deleted rather than given a longer
lifetime: authorization is structural now.

The accepted operation now has an identity — `CheckpointOperationKey`, the
session's ownership stamp plus an admission-only sequence — and every stage after
the transaction opens matches on it rather than on intent equality.

Verification, terminal outcomes and startup unification landed 2026-09-08: the
commit checks the applied world against the snapshots the operation was accepted
with, publishes exactly one outcome per key, blocks gameplay on failure, and
startup routing is now the same operation mechanism —
`CheckpointResumeProgress` is deleted rather than renamed. <!-- cite-ok: the name is here BECAUSE it is gone; this sentence records the deletion -->

Verification now covers the ledger, the bag, the room the operation claimed to
reconstruct, the subject it restores around, and — per banked custody row — that
the occurrence is carried, carried ONCE, and carried by the custodian the
checkpoint names. Each has a case that fails for it alone. The operation
counter's overflow path refuses the lifecycle slot rather than the identity
(poison-verified), the key has one canonical projection, and the terminal outcome
is a closed `RestoreFailure` rather than a free-form string.

The acceptance matrix was audited row by row on 2026-09-08 and every one of its
**seventeen rows now has a witness that fails for that row's property**; the
per-row receipts are in the protocol.

⛔ **TWO ROWS WERE CLOSED ON A STRUCTURAL ARGUMENT AND BOTH ARGUMENTS COVERED
HALF A ROW.** "Nothing destructive runs before the commit" proves *retain live
state* and says nothing about terminalization or permanent retry — both were
broken. "Every cached plan carries the default outlook and `promote` refuses a
different one" was sound about the comparison and silent about whether the
comparison was ever reached: it was not, because the prefetch cache's identity
had two keepers and only the host's copy was ever set, so the first transition of
every session cleared every warm plan before looking one up. **A structural
argument is evidence; a row closes on a test that fails for it.**

⭐ Repaired with the witness: `PrefetchIdentity` is one value that `publish`
requires, so a plan cannot enter the cache without the cache knowing the world it
was prepared for, and there is no public reset.

⛔ **THE TERMINAL ROAD'S ROLLBACK BOUNDARY WAS THE THIRD HALF-REPAIR IN THIS
PACKET.** Ending a failed preparation moved off `Update` onto a commit boundary,
which fixed the schedule but not the authorization: the host-side abandonment
note could still spend rollback state before the session-ownership gate and
before its operation's admitted frame was confirmed, and a note held only a KEY
while the sequence counter that mints keys rewinds. Repaired and witnessed on
both properties. The P2P road is deliberately inert — the terminalization sits
below `commit_confirmed_lifecycle`'s `LocalSyncTest` gate, because ending an
operation on a local asset failure is a lifecycle decision only an owning host
may make. A coordinated peer-level rule is a new packet.

**Recorded as deliberate non-goals, not open work** — each needs a NEW DECISION
to become a task:
- ⛔ verification's remaining omissions are DELIBERATE, not pending: body
  placement has no comparand but the arrival, which transit legitimately
  reconciles off, and clocks/portals have no accepted snapshot at all. Checking
  either needs a new decision (a transit postcondition; a snapshot of what a
  checkpoint means for a clock), not another assertion. Population completeness
  landed with its own failing case;
- the terminal outcome has no presentation consumer. When one is wanted, publish
  a message at completion rather than polling the session's single-latest-outcome
  resource.

**Acceptance:** preparation/prefetch read the pinned snapshot rather than a
modified live ledger; a checkpoint change invalidates a prefetched plan that
would produce a different population even at the same target room; final
verification and rollback rebase include restored custody/occurrences. A failed
destructive native apply remains fail-closed, not an invented undo guarantee.
Retain the corrected actor-spawn boundary.

### A3 - relocate actor-specific world placement lowering

**Owner:** prepared construction integration; packet A3.

**Re-measured 2026-09-09, and the row's edge was already gone; a different one
was not.** `ambition_platformer2d_world`'s manifest names no actor crate at all
and its `LoweringCtx<C>` is generic, so the queue's stated acceptance — *"world
no longer imports actor preparation solely to lower a placement"* — held at HEAD
before this row was touched. The frontier's "world region" means the MONOLITH's
`src/world/`, where the coupling measured as two files: `placements.rs` (119
lines: the context struct and three type aliases) and `rooms/stage.rs` (two
production signatures). The actor-specific lowering implementations the frontier
says to move alongside are already under `features/ecs/spawn/**`.

**The real defect the preflight found, and it landed 2026-09-09:** the five
fields of one authored snapshot reached their single assembly point by TWO
carriers. `prepared`, `brain_profiles` and `forced_brains` arrived on
`ActorConstructionContext` — whose own doc says the authorities are parameters of
one value precisely so a road cannot forget one — while the CHARACTER CATALOG and
the AUTHORED SHEETS were bare positional arguments threaded through
`RoomConstructionPlan::prepare_from_parts`, `prepare_spec` and
`RoomFeatureConstructionPlan::prepare` for the sole purpose of being assembled
beside them. They ride on the context now; three signatures each lost two
parameters (9→7, 9→7, 7→5) and all three shed
`#[allow(clippy::too_many_arguments)]`. `SimulationSetup` lost both fields too —
setup reads `construction.characters`, so one value carries the catalog where
three spellings did. Poison-verified: emptying the catalog at the assembly point
reddens 17 `app_it` tests (0 in the monolith's own 1,108, which is a fact about
where the coverage is).

**What remains is a file move that removes no edge:** `ActorPlacementContext`
sits in `src/world/placements.rs` rather than under `construction/`. Do it when
something else opens that file; do not spend a commit on it.

**Acceptance:** met — provider validation, body construction and failure behavior
remain covered, and no executable type-erased recipe registry replaced the direct
adapter.

### A9 - establish truthful minimal engine profiles

**Owner:** public SDK and composition; packet A9.

Record the Cargo feature closure of real external fixtures. Separate compiler
reachability, runtime installation and public-import ergonomics; repair one
dependency path at a time.

**The render path closed 2026-09-09, and it was one edge.** Re-measured at HEAD
with `cargo tree -e normal --no-default-features -p ambition_platformer2d`: 51
other workspace packages, matching the number this row already carried, and
`-i ambition_render` named exactly ONE path — `ambition_platformer2d ->
ambition_platformer2d_host -> ambition_render`, non-optional in the host's
manifest. The host's camera, projectile-visual and fx-pipeline plugins are behind
its own `render` feature now (which carries `ambition_menu` and
`ambition_sprite_sheet` with it, since nothing outside that feature's code named
them); the crate's own default stays `render` so building it alone still builds
the windowed face its description promises, and the facade takes it
`default-features = false` and forwards it from its `ambition_render` feature.
Closure 52 → 50: `ambition_render` and `ambition_sprite_fx` left.

⚠ **The existing capability-footprint sentinel cannot see this**, and that is why
a second contract exists rather than a wider baseline: `fixtures/minimal_game`
ASKS for the renderer (its exit criterion draws a windowed face), so the renderer
is legitimately in its closure. `the-featureless-facade-links-none-of-these` in
`scripts/check_absence_contracts.py` walks the feature-resolved tree instead —
the manifest walk the other dependency contracts use counts optional edges and
would report a renderer no feature enables. Poison-verified twice: restoring the
facade's `default-features` on host reddens it naming both crates, and pointing
its `cargo tree` at a package that does not exist trips the anti-vacuity floor
rather than printing `ok` on an empty measurement.

**Five dead dependency declarations removed the same day.**
`scripts/measure_unreferenced_workspace_dependencies.py` (committed with the
change, poison-verified by putting one back) found four crates declaring an
`ambition_*` dependency their whole source tree never names. All five lines were
genuinely removable — the compiler is the judge and it agreed:
`ambition_abilities -> ambition_boss_encounter` (which said "an abilities crate
needs a boss system" to every SCC measurement and dragged encounter, persistence
and cutscene behind it in a manifest walk) and `-> ambition_gameplay_trace`;
`ambition_touch_input`'s `mobile_touch -> ambition_cutscene`; the facade's
`all_capabilities -> ambition_sfx_bank`, a capability name activating a crate the
facade never re-exports.

⛔ **And one of them was a FEATURE THAT PUBLISHED NOTHING.**
`ambition_characters`' `causal` said *"publish this capability's causal facts
(brain decisions, for now)"* and the crate contained no `cfg(feature = "causal")`
and no reference to `ambition_causal` at all — a composition could turn it on,
pay the compile, and receive no brain decisions, with the manifest comment
asserting otherwise. Deleted along with the monolith's forwarding of it; the
other three `causal` forwards (`combat`, `damage`, the monolith's own) are real.
⚠ The feature-resolved closure did not move: every one of those crates is also
reached through `ambition_platformer2d_actor_monolith`, which is the packet's own
thesis rather than a reason to leave a false edge standing.

**And the map became a capability, 2026-09-09.** Tracing every crate in the
featureless closure to its activating parents (the frontier's "trace every
alternate path") found exactly ONE single-parent capability edge:
`ambition_menu <= ambition_platformer2d_runtime`, which installed
`MapStatePlugin` and `install_map_simulation_systems` unconditionally. The facade
already OFFERED `ambition_menu` as a named capability — an optional capability
with one unconditional installer is not optional, and that is what kept the menu
crate in a movement-only game's closure. It is behind the runtime's `map` feature
now, forwarded from the facade's `ambition_menu`, so naming the capability
installs it rather than linking a crate nobody steps.
`ProgressionSet::Map` is `shared_tangle`'s and stays configured either way.
Closure 50 → 49, and RATCHETED: `ambition_menu` joined
`the-featureless-facade-links-none-of-these`'s forbidden set, so the promised
absence is checked and not merely claimed — poison-verified by giving the runtime
a `default = ["map"]`, which reddens it naming the crate. ⚠ Scoped to THIS
PROFILE: `ambition_platformer2d_host` legitimately links the menu crate under its
`render` feature for the `MenuFont` handoff, so "the menu is never linked without
the map" would be false. The positive wire has its own witness — dropping the
facade's forwarding reddens
`every_room_the_map_calls_visited_has_its_visit_on_the_save`.

⚠ The trace's real finding is the shape, not the win:
`ambition_platformer2d_runtime` and `ambition_platformer2d_actor_monolith` are
parents of nearly everything, and `ambition_platformer2d_core` has 33 parents.
Every remaining crate has two or more parents, so no further SINGLE-EDGE closure
decrement exists.

⛔ That is a statement about the graph and nothing else, and it must not be read
as "the monolith carve is next". A closure measurement cannot say which ownership
change is semantically correct — the `actor_spawn` carve is this repository's own
receipt for that, where a green SCC number sat beside a live view the extraction
had taken with it. Any further reduction here has to establish STATE, BEHAVIOUR
and INVARIANT ownership first and let the closure follow, not the other way
round.

**The headless consumer fixture landed 2026-09-09**, and it is the third fact
neither the closure contract nor a compile check can state: that the profile
RUNS. `fixtures/headless_profile/` names `ambition_platformer2d` with
`default-features = false` and NO feature list, in its own workspace with its own
lockfile, so everything it links is something the engine supplies implicitly. Its
closure is exactly the 49 the featureless facade has — `ambition_render`,
`ambition_menu` and `ambition_sprite_fx` absent — and its tests compose the app
and step a body until the room's one authored block stops it. Wired into the lane
twice: a seconds-long `cargo check` beside outlander's, and the full test run in
the exhaustive plan.

⛔ **THE RENDERER'S ABSENCE IS NOT ASSERTED IN THAT FIXTURE, AND THE REASON IS
BETTER THAN AN ASSERTION.** A first version tried
`app.get_sub_app(bevy::render::RenderApp)` and did not compile: the consumer has
no `bevy` dependency of its own and the `bevy` the facade re-exports at this
profile is built without its render feature, so `RenderApp` is not a nameable
type from there. The absence is enforced by the type system.

⛔⛔ **AND THE FIRST BODY TEST WAS VACUOUS, which the fixture's own poison
caught.** Its room was copied from `minimal_game`, whose floor block sits at the
bottom of a 640x360 room — where a body rests at y=296, also `room_height -
body_height`. Asserting "it moved, then it stopped" survived moving that block
100,000px away: the body fell THROUGH to y=583 and settled there, satisfying both
arms while touching nothing the room authored. The floor is RAISED clear now, so
the resting height names the surface, and the poisoned run reddens. ⚠ The same
geometry is in `minimal_game`, which `docs/sdk/README.md` tells consumers to copy.

**The four capabilities the frontier's minimum EXCLUDES were traced 2026-09-09,
and there is no further accidental edge among them.** The packet names the
intended minimum as a body against world geometry *"without renderer, audio,
inventory, encounters or game content"*. Renderer: gone. Of the rest, measured
with `cargo tree -e normal --no-default-features -i <crate>` against the
featureless facade:

- **`ambition_inventory_ui` is ALREADY ABSENT** — no parent in the closure at
  all. The frontier's list is stale on that one; do not spend a gate on it.
- **`ambition_encounter`, `ambition_boss_encounter`, `ambition_cutscene`,
  `ambition_dialog`, `ambition_conversation`, `ambition_items`,
  `ambition_persistence`** all arrive through
  `ambition_platformer2d_actor_monolith` and/or `ambition_platformer2d_runtime`,
  most through three or more parents. These are the hub, not a gate.
- **`ambition_audio` has one path that does NOT cross the monolith**:
  `ambition_platformer2d_provider -> ambition_load_presentation ->
  ambition_game_shell -> ambition_audio`. Both edges on it are GENUINE USES, not
  residue: the provider's authoring surface carries
  `ambition_load_presentation::LoadExperienceSpec` (a loading screen is part of
  an experience's declaration, 8 references), and `ambition_game_shell::session`
  reads `AudioCatalogRegistry` / `FrontendAudioRegistry` to select the audio
  context per route. Making either optional carves a PUBLIC authoring surface or
  moves route audio selection — a design decision, not a dependency cleanup, and
  it is not taken here.

⇒ Every remaining reduction needs an ownership change first. That is a statement
about these four edges, measured, and still not a licence to read a closure number
as a mandate — see the note above the trace.

**Next in this row:** which of the 49 a minimum profile has a RIGHT to expect is
still unestablished for the crates the frontier does NOT name; the fixture makes
that question askable rather than answering it.

**Acceptance:** a supported profile constructs and steps a real subject, its
promised absent capability is absent from both installation and resolved closure,
and the full Ambition composition continues to work.

### A6 - MEASURED 2026-09-10; the two-way split the packet assumes does not exist

**Owner:** [prepared-definition field census](engine/prepared-definition-field-census.md).
200 use sites, 27 fields, 9 consumer crates.

⛔ **PREPARATION AND MATERIALIZATION ARE NOT ALREADY SEPARATED.** Nine fields are
read by BOTH the spawn road and the runtime — `autonomous_profile`,
`death_traits`, `id`, `kit`, `motion_model`, `mount`, `movement_tuning`,
`provider`, `sheet` — which is nine of the thirteen `actor_spawn` touches at all.
They are not homogeneous either: `id`/`provider`/`sheet` are identity and asset
keys legitimately read at both moments, while `kit`, `movement_tuning` and
`motion_model` are MECHANICAL VALUES read twice.

⭐ **POLICY STRADDLES UNEVENLY, and that is what a two-way split cannot absorb.**
`autonomous_profile` is read by four crates at three moments; its siblings
`provoked_profile` and `provoked_profile_id` are runtime-only.

⇒ **THE QUESTION TO ASK BEFORE ANY BOUNDARY IS PROPOSED, and it is cheaper than
a split: is `autonomous_profile` one field doing two jobs?** A field whose
siblings live in one place while it lives in three is not a straddle — it is a
name carrying both a policy ID and a live policy. Four call sites answer it.
Same shape, smaller: `display_name` sits inside `ambition_combat`'s otherwise
clean execution slice {`authored_moveset`, `kit`, `ranged_execution`}, and one
presentation field in an execution slice is usually a read that belongs
elsewhere.

⚠ **THE BINDING CONSTRAINT, and it belongs at the top of any proposal rather
than in a caveat: `ambition_app_tools` reads ELEVEN fields including `kit`,
`vitals` and `movement_tuning`.** Tool binaries reach into mechanical values, so
narrowing the surface breaks the tools first — and being binary roots, that is
where a change is noticed last. The cheap end is real: nine fields have at most
one consumer outside the owner and five have none, but `kit` has six.

⭐ The clean slices exist and are small: `ambition_body_seed` reads exactly
{`body`, `locomotion`, `vitals`} — pure materialization, no policy.

### A7 - item custody/accounting: the writer census and the occurrence seal

**Owner:** [item custody and accounting](engine/item-custody-and-accounting.md).

Four populations kept separate, because they are four questions: an occurrence
exists, a holder is a relationship, an inventory is an aggregate, a checkpoint is
a snapshot of all three. **Checkpoint is already fully owned (9 sites, 0 outside
`actor_monolith`); custody is nearly (10 sites, 1 outside).**

⛔ **The headline is inventory: `ambition_items` owns `OwnedItems`, holds three
`&mut` delegation seams, and schedules NOTHING** — all 13 foreign writers are in
`ambition_app` (5), `ambition_content` (5) and `actor_monolith` (3), and they
mutate through methods on an already-private `counts`. **Its problem is not
encapsulation, which a seal would not touch; it is that the domain has no road of
its own.** Occurrence: 25 write sites, 9 outside the owning crates, and eight of
the nine MINT a `GroundItem` directly rather than asking the domain to.

**The occurrence seal (`89984d166`) splits "occurrence" in two.** `GroundItem`
has **248 use sites across seven crates** (93 production) and
`ambition_held_items` is not even the biggest user of `spec` — actor_monolith 7,
held_items 3. `WorldItem` has **49**, with its owning crate the dominant user of
every field. ⇒ Whatever A7 does about occurrence is one decision for
`GroundItem` and a much smaller, different one for `WorldItem`.

⭐⭐ **AND THE INSTRUMENT HAD TO CHANGE, WHICH IS A RESULT ABOUT SEALS
GENERALLY: A VISIBILITY SEAL IS STRUCTURALLY ONE CRATE DEEP.** Private fields
are an ERROR at every use site, so the nearest dependent crate fails to compile
and everything downstream is never built — measured on `GroundItem` at 8 sites,
all in `ambition_abilities`, while the writer census had already named three
other crates constructing the type. `--keep-going` recovers crates INDEPENDENT
of the failure and there were none, so it is not a flag problem. `#[deprecated]`
is a WARNING: one pass, whole workspace, every use site, including same-crate
ones — which also closes the blind spot `pub(crate)` could not. 8 sites against
248 is the size of the difference.

Instruments: `scripts/measure_state_writers.py --domain item` (a lower bound;
blind spots printed in its own output) and
`scripts/measure_field_readers_by_seal.py` (the deprecation seal; the visibility
seal stays available behind `--visibility-seal`). Commits `bd7756511`,
`966351e25`, `66fe66395`, `89984d166`.

⚠ Four matcher defects were corrected mid-census, every one INFLATING the crate
the packet is about (occurrence 37→25, actor_monolith 17→6), plus a citation
defect: line numbers were counted in text with blocks spliced out, so any
`--sites` line quoted before `66fe66395` must be re-derived. Counts and per-crate
splits were never line-derived and stand.

### C2 - capability-owned installation, only where ownership is established

**Owner:** [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).

Fresh source instruments report 0 capability/ruleset private orderings,
73 composition private orderings, 174 foreign installations, and 3 mechanically
reducible versus 38 irreducible installation blocks. These are locator metrics.

Move a reducible block only after establishing one implementation owner. Keep
cross-capability policy in explicit composition. Resolve Q73 before adopting a
plugin form that conflicts with the recorded combat convention; an owner helper
can be sufficient. Zero foreign installations is not the target.

**Re-measured and one block moved, 2026-09-09.** The instrument reports 2
reducible / 39 irreducible at HEAD (was 3/38 when this row was written; the
counts drift with the composition, not with progress). Both reducible blocks were
in `ambition_platformer2d_host`, both installing `actor_monolith` input systems.

The frame-to-tick latch drain moved:
`install_latched_slot_publication` sits beside `install_roster_seating`, whose own
doc already made the argument — *"the order is this crate's fact, not the
composition's"*. The host had spelled the system, its sim phase, the
`InputSet::Route` edge it must precede, and the fixed-tick condition, and reached
for `app.sim_schedule()` to do it; it names one function now. Reducible 2 → 1.

⛔ **The last one is DECLINED, and the reason is the row's own rule.** It is
`.add_systems(Startup, spawn_primary_input_participant)` — one system, no
ordering anchors, which is exactly the shape the instrument cannot classify: it
sees "one capability could install this" and cannot see that what the composition
is deciding is *that this app has a person in front of a controller*. A headless
or RL composition installs the monolith and wants no leafwing participant. No
implementation owner is established, so it stays.

⛔⛔ **AND A POISON THAT PASSED FOUND SOMETHING BIGGER.** Making
`install_latched_slot_publication` a no-op leaves all 602 `app_it` tests green.
`ambition_platformer2d_runtime::input_drive::drive_slot_frame` — the helper every
scripted test drives input through — writes the latch *if the latch table
exists*, and otherwise falls through to writing `SeatRawFrames` and `SlotControls`
DIRECTLY. So a scripted press is delivered whether or not the drain ever runs:
the fallback makes the drain unfalsifiable from the road every test takes. The
gap predates this move (the move is registration-identical).

⭐ **CLOSED 2026-09-09 (GPT review #8):** `game/ambition_app/tests/latched_input_reaches_the_tick.rs`.
It accumulates into `SlotControlLatches` the way a device bridge does and never
calls `drive_slot_frame`, so the fallback that made the drain unfalsifiable is
not on its road. Making `install_latched_slot_publication` a no-op now reddens
both of its tests — the same poison that leaves all 602 `app_it` tests green.
Engine + `PlatformerHostPlugins` + `Fixed60Hz` + a live `SessionRoot`: NOT the
shell frontend, because the shell boots to a launcher with no session and the
claim under test belongs to the host input stack. `Fixed60Hz` and not `Rollback`
deliberately — a rollback host drains the same latches at `ReadInputs` through
`capture_latched_local_input` and would pass with the installer deleted.

⚠ TWO THINGS THE FIRST DRAFT GOT WRONG, both recorded in the file because each
would otherwise be rediscovered as a defect in the installer. (1) The gameplay
phase needs `simulation_authorized` to find a live session scope; against the
shell composition it never ran, and the test failed with the seat neutral while
the installer was fine. There is now a PROBE system in the same set, so "the sim
did not run" and "the drain is not installed" are separate answers. (2) Showing
the seat still neutral after an extra FRAME is a confound, not a check:
`populate_seat_control_frames` rebuilds every seat's latch from that
participant's `ActionState` each frame, so a synthetic accumulation is
overwritten before the tick sees it.

**Acceptance:** public set ancestry and deferred visibility are covered, optional
capabilities remain optional, and no broad runtime policy object replaces imports.

The remaining A4-A8/A10 packets have evidence-based holds in the frontier. They
are not executable queue commitments. SCC counts do not release those holds.

## P2 — current engine/game work

### D-TETHER-LINE — give the ledge tether a readable generic reach line

**Owner:** [`engine/expressive-move-capabilities.md`](engine/expressive-move-capabilities.md).

The reel is mechanically visible through movement but has no attachment line.
Do not teach `sim_view` about the Smash-specific `TetherReel` component. Publish a
generic body-to-world reach/attachment fact from gameplay and let the existing
presentation line road consume it.

**Acceptance:** diagonal ledge tether draws from body to actual anchor, Performer
flyline/grab reach remain correct, and no engine/view crate imports Smash ruleset
state.

### D-POTATO-ASPECT — resolve tier-dependent character trim/aspect drift

**Owner:** [`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md).
**Maintainer choice:** Q69 in the decision ledger covers the interim `0_25x`
fallback policy.

**Do:** keep generated-tier measurement explicit; do not turn missing ignored
manifests into a pass. Fix generation/trim semantics so a selected tier preserves
the promised frame geometry.

**Two generation defects repaired 2026-09-09, measured before and after.** A
frame's drawn quad is `authored_render * (trim_w / frame_w, trim_h / frame_h)`,
so the TRIM FRACTION decides the quad's shape and must be tier-independent. Two
things in the fallback generation path made it a function of the tier:

- **the packer re-measured an alpha bounding box on the DOWNSCALED image.**
  `build_sheet_variant` packed with `trim=True` and then overwrote the scaled
  record's `w`/`h`/`off` with the placement's, throwing away geometry
  `_scale_rect_struct` had already computed correctly. It now crops each frame to
  the scaled base box, packs with `trim=False`, and writes only WHERE the frame
  landed — so the manifest and the pixels cannot disagree;
- **and `min_frame_px` was applied twice**, which was the larger half.
  `effective_scale` already raises a whole sheet's scale so no LOGICAL frame
  falls below the floor; `_scaled_frame_crop` then applied the same floor to each
  TRIMMED CROP, inflating every small trim box up to it. `mary_o_v2` idle is the
  recorded case: base 63x86 in 160x192 (0.394 x 0.448) against potato 7x5 in
  10x12 (0.700 x 0.417) — the aspect flipped from portrait to landscape, which is
  the measurable half of Jon's report that *"the size of the snake has seemed to
  vary depending on the global game state"*. The state was the quality profile. A
  crop's only real floor is 1px.

MEASURED with `scripts/measure_sprite_tier_trim_drift.py`, whole tree
regenerated: potato rows drifting past 0.05 went **2966/3708 (80.0%) → 621/3708
(16.7%)**, worst drift **0.823 → 0.132**, and `mary_o_v2` idle is 0.400 x 0.417
against a base of 0.394 x 0.448. `sprites_0_5x` (1 row) and `sprites_0_25x` (146)
are unchanged, and their residue is a different cause — integer rounding of small
rects, bounded at 0.078 — not the alpha-retrim gradient.

⚠ **THE SECOND CLAUSE NEEDED ITS OWN REPAIR, and it caught a regression the first
change introduced.** Keeping the base box means a frame no longer shrinks onto
whatever survived downscaling — and NEAREST at 1/16 deletes thin content
outright. Measured over 60 sheets: the authored sheets carry 90 genuinely blank
frames of 7,313 and `0_5x` reproduces exactly 90, while the first version of the
fix produced 172 at potato. A frame that HAD content and lost it is now resampled
with an area filter; the count is 91. The tier's crunchy nearest look is untouched
wherever it still has something to show.

Residual, stated rather than implied: 16.7% of potato rows still drift past 0.05,
all of it small-integer rounding in frames of 9-12px. `check_quality_variants_are_fresh`
and `measure_tier_variant_scaling` are green, and 7,349 regenerated potato rects
were checked to lie inside their page — 0 out of bounds.

**Acceptance:** met for the systematic half — same authored frame at
full/half/quarter/potato has bounded anchor/aspect drift, and potato no longer
loses drawable pixels the base sheet has. Rounding drift at extreme downscale is
bounded and is not the tier-dependent aspect flip this row was opened for.

### D129 — finish authored-geometry sprite clipping repair

**Owner:** [`engine/character-authoring-package.md`](engine/character-authoring-package.md)
and current renderer geometry contracts.

Work from the current measured clipped-sheet population, not historical counts.
Fix per character/sheet where authoring is genuinely wrong; do not introduce a
global scale heuristic to erase asset mistakes.

**Acceptance:** render-time clipping warning population decreases for intentional
repairs and unchanged composited/tiling cases stay classified rather than hidden.

### D-BRAIN-MENU — make the fighter brain able to order from its authored move menu

**Owner:** [`engine/fighter-brain.md`](engine/fighter-brain.md).

The generic fighter brain can have legal authored attacks that its scoring shape
never selects. Implement the owner doc's current scoring/menu packet; do not add
per-character special-case button scripts.

**Acceptance:** representative CPU can select movement-compatible attacks,
smashes/charged options become live customers where the authored menu permits
them, and easiest difficulty remains intentionally poor rather than suicidal.

### D72 — continue Smash parity from the inventory, not a campaign diary

**Owner:** [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md).

Choose the highest-priority remaining parity row whose primitive is not blocked by
a maintainer decision. Implement through reusable engine capability when the move
class is reusable; demo-only policy stays in Smash.

**Acceptance:** update the inventory row and add production-path acceptance. Do
not append another chronology to a retired expressive-moves campaign.

### D166 — make character authoring boundaries load-bearing

**Owner:** [`engine/character-authoring-package.md`](engine/character-authoring-package.md).

Continue only from the owner doc's measured census. Migrate a field when there is
a real duplicate/competing authoring authority, not because a struct looks large.

**Acceptance:** one authoritative authored value, all runtime projections derive
from it, and the old duplicate path disappears.

### D-SCENARIO-IDENTITY — finish scenario identity transport/cache ownership

**Owner:** performance/scenario tooling.

The report-side identity distinction is complete; remaining work is transport and
cache identity. Unsupported geometry must continue to refuse rather than staging
Flat data under a different scenario name.

**Acceptance:** two scenario geometries with the same benchmark knobs do not
share a cache/result identity; unsupported geometry exits as unsupported.

### Closed by re-derivation, 2026-09-09

⛔ **D-PORTAL-INTERACT-SEAT.** Re-derived against HEAD rather than inherited, and
all three of its acceptance clauses already hold:

- *two driven bodies can independently interact in the same tick* —
  `two_driven_bodies_each_flip_their_own_switch`, beside
  `a_second_seat_spends_its_own_buffered_interact` and
  `a_seat_that_pressed_nothing_does_not_interact_on_another_seats_press`;
- *portal presence does not change which semantic interaction intent exists* —
  vacuously, and measured: `grep -c 'portal\|Portal'` over the whole 275-line
  interaction system is ZERO. There is no portal dependency to remove;
- *no query-order `.next()` arbitration returns* — the one `.next()` left is the
  documented startup fallback for the frame before any seat is attached, not an
  arbitration among candidates.

⚠ RECORDED RATHER THAN SILENTLY DELETED, because a row that describes a defect
nobody can reproduce costs the next reader the same re-derivation. The rule this
follows is the file's own: a row stays only when an engineer can act on it.

### Closed 2026-09-09 — D-RESET-ROAD-RESIDUE

`ActorMutIntegrationExt::reset_to_spawn` restored an enemy's spatial baseline,
health and respawn policy in place on a surviving entity, and had no production
caller: `git grep` found its definition and its own four `#[cfg(test)]` tests.
It is superseded rather than merely unwired — a room transition retires every
`RoomResident` and rebuilds the destination from its construction plan, and a
same-room replay reconstructs the population at the confirmed lifecycle boundary,
so no entity survives for a reset-in-place to restore.

⭐ **THE REPLACEMENT WITNESS ALREADY EXISTED AND IS BETTER THAN THE FOUR.** The
row asked for the respawn-policy claims to be asserted against the shipped
reconstruction before the false witness was deleted.
`a_body_that_never_persists_its_death_ignores_a_flag_bearing_its_name` does
exactly that, through `sync_ecs_actors_with_save` — the reconstruction's own
liveness reader — with both halves: an `OnRoomReenter` body ignores a flag its
kind never writes, and a `DeadStaysDead` body under the SAME flag stays dead, so
a green cannot be bought by the flag being absent. Deleted with its tests.

⭐ **AND THE `OnRest` HALF WAS A LIVE DEFECT, not residue.**
`clear_dead_until_rest_flags` had zero callers in the workspace, so an `OnRest`
death wrote `enemy_<id>_dead_until_rest`, nothing cleared it, and `OnRest`
behaved exactly like `DeadStaysDead` across the 14 shipped placements that author
it (`scripts/measure_persisting_enemy_placements.py`: `pirate_sky_arena` and
`pirate_sky_lookout`). A rest clears them now, at the shrine seam rather than on
`CheckpointCommitted` — that message is "whatever a game decides a checkpoint
is", including an autosave, and reviving a corpse because the game autosaved is
not the mechanic. The suffix has one owner instead of two spellings kept in sync
by a comment.

⇒ `check_no_warnings` is GREEN, which it had not been all session: the dead
method was its only finding until two more of my own appeared beside it.

### D-FOREIGN-ORDER-SPELLING — DONE 2026-09-10

**The ratchet at 0 counted one spelling of two.**
`scripts/measure_foreign_system_ordering.py`'s `PATH` regex required at least one
`::`, so `.after(camera_follow)` through a `use` import was invisible while the
identical qualified edge scored as a violation. `capture_scene.rs` had been
writing the bare form all along; `moveset_render.rs` (`24ccc544e`) wrote the
qualified form and made a pre-existing violation VISIBLE rather than creating it.
Census now resolves each file's `use` tree before matching (`6db1dd495`).

Classifier keyed on the SITE — a binary root is a composition whatever its
package is called — rather than widening `COMPOSITION_SUFFIXES`, which cannot
survive a capability crate named `*_tools` and is the classifier-collapse move
the guard's own comments warn about. Both ceilings now count EDGES rather than
written occurrences. `CAPABILITY_ORDERING_CEILING` 0 → 10 as a **debt ledger,
ratchet-down only**; `TOTAL_ORDERING_CEILING` 75 → 77 (**the instrument changed,
not the tree**; 93 under the old counting). Commits `6db1dd495`, `5f3f75508`.
Guard: `scripts/tests/test_foreign_system_ordering.py`, 5 passed.

⛔⛔ **AND A CEILING CANNOT PIN THE CENSUS CORRECTION — this nearly shipped as a
check that cannot fail.** Revert `expand_aliases` and the capability count falls
10 → 2, comfortably UNDER the new ceiling, with the anti-vacuity floor still
satisfied by the two survivors: every assertion stays green while the instrument
goes blind again to the population it was just corrected to see. That is exactly
how the original zero happened. A ceiling only ever sees a number GROWING. So the
correction is pinned by a positive control naming an edge that exists ONLY in the
bare spelling (`ambition_content -> ambition_portal2d::portal_transit`), plus a
pin on the site classifier. Poison-verified both ways, tree restored
byte-identical.

⚠ **THE 10 ARE THREE JOBS, NOT ONE NUMBER**, and a single figure hides that:
- **(a)** `ambition_content/src/portal/plugin.rs` — SIX orderings into
  `ambition_portal2d`'s private systems from one file. One owner, one
  published-set problem.
- **(b)** `ambition_content/src/moveset_sound.rs:38` — a two-owner `.chain()`
  over `ambition_combat` and `ambition_render`; the architecture note's own named
  poison shape.
- **(c)** `actor_monolith` ×3 — `.before(select_actor_targets)`
  (`ambition_combat`), `.after(project_boss_attack_state_from_move)`
  (`ambition_boss_encounter`), `.before(audio_play_sfx_messages)`
  (`ambition_audio`); the first two through the crate's own `pub use` re-export.

Publishing a set in `ambition_platformer2d::render::rendering` for both capture
bins, and unpicking the portal block, are SEPARATE rows — deliberately not
smuggled into a census correction.

### D-DAMAGEABLE-BODY-IDENTITY — is every damageable body identified where it is BUILT?

**Owner:** [projectile contact protocol](engine/projectile-contact-protocol.md);
opened by A2's ordering half, 2026-09-10.

`construction/mod.rs` mints `SimId::placement(..)` for enemies, bosses, giants,
hands, shrines, riders and summons, so the placed roads are covered. What is NOT
established is whether any damageable body reaches the world without one. The
protocol calls missing required target identity a construction/verification
failure rather than a sort fallback, so the invariant belongs where bodies are
BUILT — the resolver can only report it.

⚠ The projectile resolver's `debug_assert` (`c2188fa7a`) is a PARTIAL census: it
flags only the COINCIDENT pair, so a lone unidentified body passes it silently.

⚠ **A STATIC SCAN CANNOT ANSWER THIS.** Bodies receive `CenteredAabb` and
`ActorFaction` from separate inserts, so a bundle-shaped scanner reports 2 of 2
and is describing its own method rather than the tree. A runtime census over the
`StrikeVictim` query is the instrument (precedent:
`ambition_dev_tools/src/runtime_census.rs`).

**Deliverable is the census, not a refactor.**

**Acceptance:** the population of damageable bodies without a stable identity is
measured at runtime and named; if it is empty, the invariant is asserted where
bodies are built so it stays empty.

### D-ID-CONVENTION-DRIFT — keep shared semantic key builders single-owned

**Owner:** registry/identity owners.

Continue only when a producer and consumer still construct the same semantic ID
with separate format strings. Move spelling into the semantic owner and update all
customers in one change.

**Acceptance:** grep finds one constructor for the migrated key family and both
producer/consumer tests use it.

**Re-measured 2026-09-09** with `scripts/measure_id_prefixes_spelled_twice.py`:
38 prefixes appear as a literal, TWO in both a producer and a consumer.

- `_dead_until_rest` — **migrated**, and it was a live defect rather than drift.
  See the D-RESET-ROAD-RESIDUE receipt above.
- `respawn_platform_` — `game/ambition_demo_smash/src/lib.rs`, the Smash lane.
- ⚠ `npc_` — **A FALSE POSITIVE, and it must not be "fixed".** The sweep matches
  a PREFIX, so three unrelated conventions that share four characters read as one
  drifting id: `npc_{id}_hostile` and `npc_{dialogue_id}_talked` are save FLAGS
  built in `features/npcs.rs`; `character_id.strip_prefix("npc_")` in
  `ambition_sprite_sheet` is a PLACEMENT-ID convention for finding a sheet
  record; and `npc_talked:{id}` in `ambition_persistence::quest` is a quest
  objective key. Verified by grep: each flag has exactly one production
  constructor, and the remaining literals are test spellings, which are
  deliberate — an independent spelling in a test is what catches a rename.

⇒ Nothing is left in this row for the engine lane. It stays open for the Smash
one, and for the next producer/consumer pair a re-measurement finds.

### D-BUILD-GRAPH-BLINDNESS — DONE 2026-09-09

**Owner:** build/architecture tooling.

Do not interpret declaration count as capability reachability. Measurements must
resolve definitions, feature conditions and actual closure.

**The row's own failure happened while it was open, twice in one day, and both
are now instrumented.** A `cargo metadata` walk of the facade's closure reported
61 ambition crates where `cargo tree -e normal --no-default-features` reports 49;
the 12-crate gap is entirely OPTIONAL edges no feature enables, and the planning
row's existing figure was right while the "correction" would have been the error.
A guard built on the resolve graph would demand the deletion of a dependency that
already costs nothing — which is this row's sentence, met from the wrong side.

The three answers now have three instruments and fixtures that tell them apart:

- **declared-but-unused** — `measure_unreferenced_workspace_dependencies.py`'s
  `unreferenced_in`, split out to be testable, with
  `scripts/tests/test_unreferenced_workspace_dependencies.py`: a source use, a
  strong `dep/feature` forward, a WEAK `dep?/feature` forward, a dependency
  nothing names, a prefix-sibling false negative that would otherwise make the
  count permanently zero, and a live-tree floor so a classifier that reported
  nothing for every input could not pass;
- **feature-gated** — the same forwarding cases, which are a real use of the
  dependency and must not be reported;
- **actually linked** — `the-featureless-facade-links-none-of-these`, which walks
  the FEATURE-RESOLVED tree, with red probes in
  `scripts/tests/test_absence_contracts.py` for a forbidden crate that is
  present, for an instrument that measured nothing (the failure mode an ABSENCE
  contract has by construction), and for a truncated census.

**Acceptance:** met — fixtures distinguish declared-but-unused, feature-gated and
actually linked dependencies.

### D-LANE-UNRUNNABLE / D-APPIT-FLAKE — preserve executable test lanes

**Owner:** test runner / app integration lane.

When a lane cannot run because the environment lacks a precondition, report
**incomplete**, not pass. For flakes, isolate the production ordering/state source
instead of increasing retries.

**Acceptance:** missing Cargo/target/GPU prerequisites are explicit receipt states;
known deterministic fixtures do not depend on wall-clock or entity order.

**The first half landed 2026-09-09, from a run that produced the defect.** A
`--rust` lane reported `5/6 jobs passed` with `workspace doctests` FAILED, and
the whole content of that failure was `error: extern location for bevy does not
exist: …libbevy-<hash>.rlib` — a stale artifact left by the same commit's own
manifest feature changes. `cargo test --workspace --doc` immediately afterwards
was clean. The job never compiled a doctest, so "FAILED" was a claim about the
repository that nothing had measured, and a reader could not tell it from a real
red. `run_tests.py` now scans a bounded tail for a narrow table of PRECONDITION
signatures and reports those jobs as INCOMPLETE — named in the summary with their
remedy, listed under `unrunnable` in the status file and on the per-job row, and
still non-zero, because incomplete is not pass. Four tests, each the others'
control (a stale artifact is incomplete; an ordinary red is still a red; a clean
run says nothing about either; the signature table is not empty), poison-verified
in both directions: widening the pattern to `FAILED|error` reddens the real-red
arm, and classifying nothing reddens the incomplete arm and the floor.

⚠ Still open: the GPU and missing-Cargo prerequisites have no signature yet — the
table is deliberately narrow, one entry per signature actually observed, because
a pattern broad enough to swallow a genuine compile error converts real reds into
shrugs. And the D-APPIT-FLAKE half below is untouched.

**Sighting 2026-09-09, measured rather than guessed.**
`composes_through_the_sdk::a_host_that_omits_boss_encounters_still_builds_and_steps`
failed in 2 of ~8 full `app_it` runs and **0 of 25 consecutive runs of its own
module**, plus 3 consecutive clean full runs. ⇒ it fails only under full-suite
load, which points at parallel execution or process-global state rather than at
the test's own logic. The panic text was never captured, because a run that fails
prints it only for the failing test and every attempt to capture reproduced a
green run — that is the row's own lesson restated: **a flake with no message is a
sighting, not a diagnosis.** Whoever sees it next should run the full suite with
output redirected to a file so the message survives the run that produced it.

**Second sighting 2026-09-09**, and it followed that advice too late: a `--rust`
run reported the `workspace (default features)` job failed, an identical
`cargo nextest run --workspace` immediately afterwards was 7619/7619 passed with
34 skipped, and the failing test's NAME was lost because that run had been piped
to `tail`. Same shape as the first sighting — full-suite load only — and the same
lesson, now paid for twice: redirect the whole run to a file, not to `tail`.

⚠ The obvious suspect was ruled out: the test's own doc says "the only thing that
would make it fail is a boss system that some other capability turns out to
require", and A2a had just made the damage-facing publication require
`Res<BossCatalog>`. If that were it the failure would be deterministic in the
disabled arm; 25 clean module runs say it is not.

### POST-CARVE-DOC-SWEEP — update moved-source references in the same carve

**Owner:** the carve author.

Run citation/link/source-reference guards on the **diff** after a move. Re-tense
historical prose where useful; delete live directions to old paths. Do not retain a
huge global post-carve diary.

⛔⛔ **AND "ON THE DIFF" MEANS THE RANGE FORM, WHICH IS A DIFFERENT CHECK.**
`check_planning_citations.py --vanished HEAD` compares HEAD against the WORKING
TREE and says so in its own output — *"⚠ that is REF→WORKING TREE, not REF→a
carve — pass `A..B` to attribute a range"*. Ran after every commit on 2026-09-09
it stayed green all day, because each commit's deletions were already at HEAD by
the time it ran. The RANGE form over the same session
(`--vanished <first>..HEAD`, 21 names left between them) found one immediately:
`projectile-contact-protocol.md` still cited A2a's witness as
`the_boss_hit_test_answers_only_from_the_published_volumes`, <!-- cite-ok: the row RECORDS the vanished name --> renamed to
`a_boss_is_reached_only_through_its_published_volumes` by the A2c predicate
deletion two commits later. Repointed. ⇒ A carve author who runs only the
working-tree form has not run this row's check at all.

## P3 — human-gated measurements and local-machine work

These rows cannot be completed from an ordinary headless source review. Keep the
measurement here; keep analysis/results in the owning tool or owner document.

- **D-RASTER-3:** run the remaining weak-GPU framebuffer-scale versus source-tier
  experiment on the intended GPU. Owner: `engine/performance-and-iteration.md`.
- **Switch Pro outer range:** run the controller diagnostic on both machines and
  record the measured radial maxima/dead-zone behavior before changing stick
  thresholds.
- **Web reveal branch:** validate the existing reveal-barrier branch on the real
  browser/GPU target before merge; do not infer from native first-draw behavior.
- **Kaleidoscope Bevy-0.19 flash:** reproduce interactively before filing a fix;
  stale no-repro descriptions are not a queue substitute.
- **LDtk preview tilesets:** measure whether editor-preview assets still decode the
  full player sheet on current boot before changing residency policy.
- **Capture stays alive after window close:** reproduce with current capture
  tooling and identify the live owner keeping the process alive before patching.
- **External consumer/platform checks:** use the SDK/external-consumer owner docs;
  do not claim portability from in-workspace fixtures alone.

Product/content choices such as dense-room composition, evergreen settings,
camera legibility limits and asset policy live in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md), not here.

## Replenishment rule

Add a queue row only when all of these are known:

1. the current production failure or missing capability;
2. the semantic owner;
3. the next concrete edit or measurement;
4. an acceptance test/receipt that can falsify the work.

If one is unknown, put the question in the relevant owner document or maintainer
decision ledger instead. When a row is complete, delete it from this file. Git
history is the completion log.
