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

⛔ **THE CONSTRUCTION-IDENTITY HOLE, NAMED EXACTLY (read 2026-09-10).**
`ambition_platformer2d_runtime/src/sim_identity.rs :: ensure_sim_id` queries
`With<BodyKinematics>, Without<SimId>` and mints from an authored fact:

```rust
(Some(feature_id), _) => SimId::placement(&id.0),
(None, Some(_primary)) => SimId::player_slot(0),
(None, None)           => continue,     // <-- THE HOLE
```

Its own comment states the invariant: *"Not identifiable from an authored fact.
Its spawn site must mint it."* ⇒ **The invariant is written in a comment and
enforced by nothing.** A spawn site that forgets leaves a damageable body that
`ensure_sim_id` deliberately skips.

⇒ **THE WORK IS NOT "FIND THE BAD SPAWN SITE". IT IS TO MAKE THAT ARM
OBSERVABLE.** The runtime census
(`game/ambition_app/tests/every_damageable_body_is_identified.rs`) is the right
instrument and already exists. ⚠ Its `DAMAGEABLE_FLOOR = 4` against a measurement
of 4 is **a ratchet at its ceiling**: it cannot see an addition, which is the
direction a new unidentified body arrives from.

⚠ **The cut-rope victory NPC was REFUTED as an example** — it carries `FeatureId`
+ `BodyKinematics`, exactly the query the first arm serves. **What is open is the
general invariant, not that case.**

⭐ **A DESIGN FACT THAT COST THREE AGENTS AN HOUR, RECORDED HERE BECAUSE NOTHING
ELSE RECORDS IT: ONE UNRESOLVABLE PARTICIPANT GIVES ZERO SEATS.**
`ambition_match/src/prepared.rs` records a per-seat problem and continues at line
635, then aborts the WHOLE preparation at line 872 if any problem was recorded.
**Four `seat_problem(` call sites ⇒ four fault kinds each abort an entire match.**
A fixture that asserts a seat count therefore reports `left: 0` and names nothing.
`e86fd5609` closed the diagnosability half — a withheld cast now names the
character and the admission pass instead of printing a bare zero.

### A12 - LANDED 2026-09-10; a shot now names the USE of the move that fired it

**Landed `f9baa86e8`.** A verdict carrying `attacker_move_instance: None` used to
be credited to whatever move the fighter is playing NOW, so a projectile launched
by move A and landing during move B marked B connected — the late-feedback defect
A12 exists to eliminate.

**The road:** `MovePlayback::instance` → `MoveEventMessage::move_instance` (3 emit
sites) → `RangedCommitment::CommittedMove { instance }` →
`ProjectileSpawnRequest::move_instance` → `FiredByMoveInstance` on the shot → the
damage result. **No consumer reads the value again**; a read of the owner's
playback at spawn or at landing repeats the defect one link later.

⚠ **The value is ABSENT, not zero**, for a shot no move fired — a gun, a bomb, an
environmental volley. **A `0` would name a first use that never played.**

`RangedCommitment` carries it rather than `ActorActionMessage` because the
commitment has **2 construction sites against 43**, and only the commitment is on
the firing path.

**MEASURED 2026-09-10:** `officer` is the one fighter on the shipped grid that can
reach the bug — a shot move plus a conditional cancel. The census test
`a_shot_and_a_conditional_cancel_never_share_a_fighter` keeps that live.
`GGRS_ROLLBACK_SCHEMA_VERSION` 178 → 179, both baselines updated.

⛔ **THE TWO ROLLBACK BASELINES ARE NOT TWO COPIES OF ONE FINGERPRINT.**
`rollback_schema_baseline.txt` holds the version and the rows;
`rollback-schema-baseline.json` holds `stable_schema_names` and `encoded_types`
and **no version at all**. ⚠ **The JSON wants the RE-EXPORTED name**
(`ambition_projectiles::FiredByMoveInstance`), not the module path. A module path
there leaves **the Rust baseline test GREEN while `check_absence_contracts.py` is
RED** — two instruments with different populations, and the Rust one looks
authoritative. Stopping at the Rust test ships a baseline the guard rejects.


### A4 - NOT BLOCKED; the hold is delivered and the premise is the one that MEASURES TRUE

⛔ **THIS PACKET WAS READ AS BLOCKED AND IT IS NOT.** A4's hold, verbatim, is
*"first map writers and select production fixtures."* Both halves are delivered in
[the writer map](engine/accepted-control-writer-map.md), which says so in its own
opening: *"the frontier says the hold is released by the enumeration, so the
enumeration is the deliverable."*

⭐ **RE-DERIVED 2026-09-10 at `2bf960acf`, 156 commits after the map's
`966351e25` stamp. Every row is IDENTICAL:**

| responsibility | component | sites | outside its defining crate |
|---|---|---|---|
| accepted driver relation | `DrivingParticipant` | 8 | **8 — all of them** |
| input projection | `ActorControl` | 24 | 22 |
| live body execution | `BodyKinematics` | 38 | 28 |
| custody reconciliation | `InCustodyOf`, `BodyCustodySettled` | 4 | 4 |

⇒ **A4's PREMISE HOLDS, and it is the only one of three that does.** A5's premise
measured FALSE — `Breakable::apply_damage` already owns the whole state machine.
A6's measured FALSE — nine fields are read at both moments. **A4's authority is
genuinely scattered.** ⛔ **Three packets, three different answers: a packet's
premise is a claim to be measured, not a frame to work inside.**

⚠ **THE UNCHANGED NUMBERS WERE POISONED BEFORE THEY WERE BELIEVED.** A number that
does not move in 156 commits is a claim about the INSTRUMENT. One added
`&mut BodyKinematics` query took body execution 38/28 → 39/29; the site was then
removed. ⇒ The scan reads the live tree. **Without that, "unchanged" and "not
measuring" are the same output.**

⛔⛔ **AND THE MAP HAS TWO WRONG CITATIONS THAT ITS OWN TOTAL HID.** It names four
production readers of `body_driving_seat`; the count is still four at HEAD, and
**two members are wrong**:

- `control/queries.rs:224` **is a TEST** — `#[cfg(test)]` sits at line 209, and at
  209 in `966351e25` too. **An error at the stamp, not decay.**
- `avatar/systems.rs:103` **is a production reader and is MISSING**, added by
  `ab308504b` — *the same commit the surrounding paragraph reports as the fix.*
  **The list is older than the prose around it.**

⇒ ⭐⭐ **A COUNT IS NOT A CHECK ON A LIST.** A reader who verified "four" would have
called the list correct. **Set equality and cardinality are different questions,
and only one of them is cheap to write down.**

⭐⭐ **AND IT HAPPENED TWICE ON 2026-09-10, ON TWO INSTRUMENTS, FOUND BY
TWO AGENTS WHO DID NOT KNOW OF EACH OTHER.** The second: the field-reader seal
returned **200 sites / 27 fields**, unchanged across ~200 commits — while
`ced8b7f7c` **moved** a `display_name` read rather than removing one.
`worn_kit.rs` left that field's list and `starting_character.rs:274` entered it.
⚠ **A MOVE IS NOT A REMOVAL**, and the unchanged total would otherwise have
read as *"that commit had no effect"*.

⇒ Two independent cases, same direction: **the total held still and the
membership moved underneath it.** In both, a reader who checked the number would
have called the list correct. **Check the MEMBERS when the claim is about
members**; a stable count is evidence of nothing but its own stability.

⚠ **`check_planning_citations.py --strict` cannot catch this class.** It rejects an
ambiguous suffix; it does **not** reject a citation that resolves to a real line in
the wrong ROLE. **Do not read a green citation lane as a check on membership.**

⛔ **THE MAP'S ONE "GENUINELY IN DOUBT" ROW WAS DECIDED THREE DAYS BEFORE THE
MAP RE-OPENED IT.** `InCustodyOf` is **one fact — "room residency is suspended"
— with two producers of different DURABILITY**, closed by
[item custody and accounting](engine/item-custody-and-accounting.md) in
`414019ec9` (2026-09-07); the one change it recommended landed the same day in
`1659e5402`. The map was written 2026-09-10 and cites neither.

`ambition_held_items` writes it for an ITEM's holder, which also carries
`ItemCustody`. `project_body_custody` writes it for riders, limbs and possessed
BODIES, querying `Without<GroundItem>` because the item domain owns its own
projection. `persist_occurrence_horizon_to_save` keeps only the ITEM rows.
⭐ **That drop is the design:** a grip on a mount and a possession are session
state, and a durable row saying *"somebody holds this body"* is a claim the
loader cannot answer, because it does not put a rider back on a mount.

⚠ **THE REAL HAZARD IS NOT THE ONE THE MAP NAMED. DURABILITY IS EXPRESSED AS AN
ABSENCE.** A row is non-durable because its subject LACKS `ItemCustody`, so the
save filter never asks a new producer anything. ⇒ **A third producer becomes
non-durable BY DEFAULT and SILENTLY** — a permissive default answering a question
it was never asked. Anyone adding one must decide durability on purpose, because
nothing will make them.


## P1 - ownership and independently testable composition

### A3 - DONE; acceptance met. One cosmetic residual, not worth a commit.

`ActorPlacementContext` sits in `src/world/placements.rs` rather than under
`construction/`. Moving it removes no dependency edge. ⛔ Do it when something
else opens that file; a commit spent on it buys nothing.


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
profile is built without its render feature, so `RenderApp` is not a nameable type from there. <!-- cite-ok: `RenderApp` is BEVY'S and is named here precisely because this profile CANNOT name it; resolving to no definition in this tree is the finding, not a stale citation -->
The absence is enforced by the type system.

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

⚠ **THE 49 IS THE COUNT *INCLUDING* THE FACADE, measured at `939d6aaa5`;
excluding it the number is 48.** ⛔ **STATE WHICH ONE, ALWAYS.** The
[status page](status.md) carried **51** as an *other-packages* count from the
`300004d6` baseline, and a row's 49 read as DRIFT against a later 48 when they
were one measurement under two definitions. **A number that cannot say which it
is cannot be quoted.** Reproduce with
`cargo tree -e normal --no-default-features -p ambition_platformer2d`.


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

⛔ **THAT QUESTION WAS ASKED AND THE ANSWER IS NO — do not re-open it.** I
proposed that `autonomous_profile` might be one name carrying both a policy ID
and a live policy, which would have explained its three-moment spread without a
boundary change. MEASURED at the four call sites: **the split already exists.**
`PreparedCharacterDefinition` carries BOTH `autonomous_profile` (the value,
"Carried") and `autonomous_profile_ref`, whose own doc says it is *"RESOLVED at
preparation, so nothing downstream ever sees the name"*.

⚠ The one reader that appears to resolve a profile BY NAME at spawn —
`npc_policy.rs:94`, `catalog.autonomous_profile(name)` — is the
`AMBITION_ACTOR_BRAIN_PROFILE` dev-tools override, documented in place as *"a
measurement knob, unset in every ordinary run"*. It is a different road, not a
leak of the authored one, so the contract holds.

⇒ So the three-moment spread is not a name doing two jobs; it is one value read
wherever autonomy is decided — folded at preparation, seeded at construction,
ticked at runtime. **The uneven-straddle observation stands and the cheap
explanation for it is dead**, which means any A6 boundary proposal has to account
for a field that is genuinely wanted in three places.
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

### A7 - RECLASSIFIED 2026-09-10: a component-construction SEAL, not an occurrence-ownership completion

⛔⛔ **THIS ROW SAID DONE ON A CLAIM THE CODE DOES NOT SUPPORT, and a GPT review
of the 125 commits after `6a692b6` is right about it.** The seal below is real
and worth keeping. *"Seven minting authorities became one"* is not.

`drop_held_weapon` (`actor_monolith/src/features/ecs/damage_drops.rs:320-348`)
still spawns the entity itself and mints four of the occurrence's five facts —
`SimId::death_drop(..)`, `RoomScopedEntity`, `dynamic_drop_origin(..)`,
`SpawnedThisAttempt` — with `GroundItem::at_rest` one component in a tuple of
five. The smash bomb/mine and match-spawn roads do their own. ⇒ **Calling a
centralized component constructor does not transfer authority over the
OCCURRENCE to that component's crate.**

⭐⭐ **AND THE MECHANISM OF THE FALSE CLOSURE IS WORTH MORE THAN THE CORRECTION.**
The sentence in `item-writer-inventory.md` was written as a **PROPOSAL** —
*"a constructor that takes what an occurrence needs… turns seven minting
authorities into one"*, describing what a fix WOULD buy — and was then read as a
description of what the seal DID buy, and the row was marked done on it. ⇒ **A
forward-looking sentence and a completion claim are one tense apart, and the
page gave a reader no way to tell which it was holding.** That is not
overclaiming; it is a grammatical ambiguity that survives careful reading, which
makes it worse. **A proposal says WOULD; a receipt names a COMMIT.** Corrected in
the owner document at `1e1af1760`.

⛔ **AND THE FIX IS NOT A GENERIC ITEM-REQUEST BUS TO MAKE THE COUNT ONE.**
Centralizing occurrence minting is justified only where it centralizes a real
invariant — identity, custody, provenance, rollback ownership. **A bus that
exists to move a number from seven to one buys a number.** The documentation was
the defect, not the architecture.

⇒ **WHAT A7 STILL OWES:** the packet's acceptance is *"reward policy receives
accepted outcomes; it does not become an alternative item minting path."* **That
is a question about who may DECIDE an occurrence exists, and it is untouched.**

**What is genuinely DONE below: the census and the seal.**


⭐ **THE SEAL PRODUCED THE UPPER BOUND THE CENSUS PAGE SAID A TEXT SCAN COULD
NOT.** `GroundItem` had four `pub` fields and NO constructor, so seven production
sites across three crates each minted an occurrence — including a death-drop
policy, which A7's own acceptance forbids, and it could because there was no
narrower road to take. `#[non_exhaustive]` plus `at_rest` / `released` made the
enumeration `rustc`'s job: the census's 7 production sites were exactly right,
and 13 MORE in test code it strips by design. Commits `be2f97fa3`, `7108a57b1`.

⇒ **The findings are decisions and they live in the owner document**
([item custody and accounting](engine/item-custody-and-accounting.md), plus the
[writer inventory](engine/item-writer-inventory.md)): inventory is the headline
rather than session-adjacency (`ambition_items` owns `OwnedItems`, schedules
NOTHING, and 13 of 16 writers are foreign), checkpoint is already 9-of-9 inside
its owner, and "occurrence" is one decision for `GroundItem` (248 sites, seven
crates) and a much smaller different one for `WorldItem` (49, owner-dominant).

⚠ Two instrument corrections are recorded there and matter to anyone re-running
it: a VISIBILITY seal reaches only the nearest dependent ring (8 sites where a
`#[deprecated]` pass finds 248), and the writer census recognised construction by
a blessed-name list until `3934f560c`, which had hidden `WorldItem::equipping`
entirely.


### C2 - DONE; acceptance met.

Public set ancestry and deferred visibility are covered, optional capabilities
remain optional, and no broad runtime policy object replaced imports.

⭐ THE PART WORTH KEEPING, because it is a guard-design rule and not a C2 fact:
a test that shows a seat still neutral after an extra FRAME is a CONFOUND, not a
check — `populate_seat_control_frames` rebuilds every seat's latch from that
participant's `ActionState` each frame, so a synthetic accumulation is
overwritten before the tick sees it. And "the sim did not run" and "the drain is
not installed" are the same green until a PROBE system in the same set tells them
apart. Both live in that suite now.


### S7 / N3 / guard doctrine — landed 2026-09-10, recorded where they are OWNED

Not queue rows: three findings whose homes are elsewhere, listed here only so the
next reader of this file knows they exist.

**S7 — 59 rollback rows outside the session checksum, all read**
([simulation authority and determinism](engine/simulation-authority-and-determinism.md)).
53 bounded, 4 unbounded, 1 presentation correctly out, and 1 (`smash.seat_credit`)
with no production reader at all — removed at v178, `2c1ecfedb`. ⛔ The honest
reading is NOT "59 undetected divergences": `rollback/registry.rs` already argues
that uncompared state "appears a tick later as a checksum mismatch with no
obvious cause", and the localization probe exists BECAUSE that is true. It is a
latency-and-attribution problem with a mitigation already chosen. ⚠ The class
that is genuinely open is state whose next reader may be arbitrarily far away —
`portal.owned_gun_pair`'s only production reader is a MENU re-equip, so its
divergence never propagates and the checksum never catches it, **not late, ever**.

⚠ AND THE CANONICAL-FINITENESS OBSERVER COVERS ONLY THE CHECKSUMMED HALF
(`13021f0bf`, 116,280 finite / 0 non-finite over 30 frames). A green run does not
mean the canonical state is finite. ⭐ Its own construction is the finding worth
keeping: `canonical_f32_bits` already tested `is_nan()` — it collapses NaN to one
bit pattern **so two peers' checksums agree**, which makes the one mechanism that
notices divergence blind to this poison by design.

**N3** ([netcode](engine/netcode.md)): the rollback schema fingerprint is kept in
TWO files checked by two lanes, and negotiating an identity the repo keeps twice
is negotiating which copy. Found the hard way — a new registration left the Rust
baseline green and the Python one red.

⛔⛔ **A FIFTH SPECIES, FOUND 2026-09-10 AND WORTH MORE THAN THE RED THAT
PRODUCED IT: A GUARD WHOSE INPUT IS OUTSIDE THE POPULATION ANYONE RE-RUNS.**
`--workspace` was red for hours and every hypothesis either agent floated was
about SOURCE — the changed crates, their dependents, feature unification,
`relativity`, flake. The failing test was
`no_planning_doc_names_a_condition_the_engine_does_not_publish`, and the change
that broke it was **a `.md` file**.

⇒ **A test's inputs are not its crate.** `app_it` reads planning documents, the
rollback baseline, asset manifests and LDtk worlds; any of those changing is a
change to its subject. A diff-derived population of CRATES cannot contain a
documentation edit at all, so no amount of per-crate discipline reaches it —
which is why a per-crate `app_it` run passed 611/0 one commit before the page
landed, honestly and uselessly. ⚠ Diagnostic: *what does this test read that is
not its own crate?* Remedy: say so AT the test, so whoever edits that input knows
they are editing a subject.

⭐ Two narrower rules from the same hunt, both true and both insufficient on
their own: a per-crate sweep of the CHANGED set is blind to what a change causes
downstream (the population is the reverse dependency closure — 79 members, nine
run), and `cargo test -p` builds BARE features while `--workspace` unifies them,
so a sibling can turn on a capability an umbrella feature deliberately excluded
([capability and runtime composition](engine/capability-and-runtime-composition.md)
carries the `relativity` case: taken OUT of `all_capabilities` on 2026-09-01 so
the relativity crates would not be in every default build, and on anyway under
`--workspace` because `ambition_demo_twintrack` names it. MEASURED both ways —
`cargo tree -e normal -p ambition_app` has no relativity edge, while
`cargo tree -e normal --workspace -i ambition_relativity2d` puts it under
`ambition_platformer2d`, which reaches `ambition_app` and the rest. ⇒ **A
statement that is false gets corrected; one that is true of a build nobody runs
is quoted forever.**).

**Guard doctrine** ([checks that did not run](../recipes/checks-that-did-not-run.md)):
three species that all print the same green. ⇒ **VACUOUS** — the guard is blind;
ask *would this still pass if the scan matched nothing?*; wants a FLOOR.
⇒ **INERT SUBJECT** — the guard is perfect and nothing in production reads what
it asserts; ask *who reads this outside the test?*; wants a READER. ⇒ **WRONG
PARTY** — the guard is sighted, the tree is RIGHT, and the message names the
wrong file; it is the only one whose remedy is destructive if believed, and the
worked case is a correct schema census nearly edited to silence a condition
guard. ⭐ Remedy ranking: cross-evidence (two inputs of different KINDS) beats a
closed anchor, which beats a cleverer pattern — a floor says "I saw N things",
cross-evidence says "two independent worlds agree", and only one survives the
instrument going blind.
([source-text guard exposure](engine/source-text-guard-exposure.md) is the sweep:
121 Python guard files and 12 Rust, one real case.)

### D-CPU-INERT — an authored fighter's CPUs engage 11% of a duel; a NON-fighter is seatable

**Owner:** [fighter brain](engine/fighter-brain.md), F6 and the utility
progression. Found 2026-09-10 while sampling D-BRAIN-MENU across fighters; it is
not a consequence of that row's held change — every number below is HEAD.

⛔⛔ **THE ACCEPTANCE TEST PASSES FOR ONE FIGHTER OUT OF THREE SAMPLED.** Same
harness, same rung (9, the top authored rung), same 3613 ticks, same mirror
matchup — the FIGHTER is the only variable:

| fighter | damage/min | move starts | running | hitstun | KOs |
|---|---|---|---|---|---|
| `npc_pirate_admiral` ⛔ VOID | 1.26 / 1.07 | 54 / 27 | 14% / 19% | [525, 324] | 2 |
| `npc_emmy_noether` | **0.28 / 0.44** | 13 / 12 | 2% / 2% | [38, 59] | **0** |
| `npc_carl_stargan` ⚠ NOT a fighter | **0.00 / 0.00** | **3 / 3** | **0% / 0%** | **[0, 0]** | **0** |

⛔⛔ **AND MY FIRST VERSION OF THIS ROW CALLED CARL A SHIPPED FIGHTER. HE IS
NOT.** `npc_carl_stargan` appears in `character_catalog.rs` in exactly one place:
`KNOWN_BARE_REGISTRATIONS`, the exemption list for ids that author **nothing —
not a body, not a policy, not a moveset**. The entry records the reason in place:
*"one placement: hall_of_characters NpcSpawn, brain_override stand_still… Registered because Jon put him on the Smash grid and the grid drops what it
cannot seat."*

⇒ **So his 0.00 is not a CPU-quality result — it is a character with no authored
body being SEATABLE AS A FIGHTER**, which is a content/roster defect and a
genuine second finding. `npc_emmy_noether` IS on `PLAYABLE_ROSTER` (the curated
cast, every id a catalog row with a renderable sheet), so hers is the brain
finding. Two different defects; the first version of this row conflated them.

⚠ **The failure was mine and it is worth naming: I picked a subject by grepping
fighter ids and never asked whether it was a fighter**, while one screen away the
catalog carried an assertion whose entire purpose is to say it is not. An
instrument's population is not the list that is easy to grep.

⚠ **THIS IS NOT A MENU-BREADTH PROBLEM, which is what F6's framing would
predict.** Emmy starts 7–8 DISTINCT moves out of her 13, and Carl 3 out of 3. The
variety is there; the ACTIVITY is not. A brain that picked badly would still
press. These barely press at all.

⭐⭐ **AND THE MECHANISM IS MEASURED: THEY ARE NOT FAILING TO ATTACK, THEY ARE
FAILING TO CLOSE.** Ticks spent within 60px — roughly a body-and-a-half, inside
which an ordinary grounded attack reaches:

| fighter | ticks in reach | closest ever | damage/min |
|---|---:|---:|---|
| `npc_pirate_admiral` | **1194** of 3613 (33%) | 10px | 1.26 / 1.07 |
| `npc_emmy_noether` | **392** (11%) | 0px | 0.28 / 0.44 |
| `npc_carl_stargan` | **17** (0.5%) | 2px | 0.00 / 0.00 |

⇒ **Time-in-range tracks damage across all three.** Every fighter DOES reach its
opponent — the closest approach is 0–10px in each case, so approach is not
impossible — but Carl's pair are in reach for half a percent of the duel. The
attack scorer is rarely being offered a target at all, which makes this the
MOVEMENT scorer's subject and not the attack menu's.

⭐⭐ **SWEPT ACROSS THE WHOLE GRID 2026-09-10 — 21 ids, and the answer is neither
thing either of us expected.** The gate's population is NOT one, AND the roster is
not broadly inert. **Twenty measured, fifteen clear the gate, five fall short —
and the five DO NOT SHARE A MECHANISM.**

| band | fighters | shape |
|---|---|---|
| top | `smash_george_booul` **64% in reach, 182/184 starts, 5.12 dmg** | highest on every column at once |
| clearing | `npc_oiler`, `officer`, `pointed_polygon`, `projectile_polygon`, `perfect_cellular_automaton`, `player_robot_v3`, `npc_bob`, `mary_o_tall`, `sanic`, `npc_pirate_admiral`, `author`, `pugnacious_polygon`, `npc_ninja_shadow_oni_leader`, `goblin` | 17–49% in reach |
| **barely press** | `npc_emmy_noether` (13 starts, 10%), `performer` (18, 20%), `npc_carl_stargan` (3, 0.5%) | low engagement, low activity |
| **press and convert nothing** | **`special_patent_clerk` — 51/51 starts, 19% in reach, 0.09 dmg/min**; `medic` — 49/49, 25%, 0.41 | busy, in reach a normal share, converting almost nothing |

⛔⛔ **AND THE SECOND BAND IS NOT A FIGHTER PROBLEM AT ALL — IT IS THE HARNESS
MEASURING A DEGENERATE MIRROR MATCH. Traced 2026-09-10 and the mechanism is
verified in the engine's own words.**

The duel seats two CPUs of the SAME fighter with a deterministic brain and no
noise input, so a matchup that never breaks symmetry stays in lockstep: both
bodies hold identical state, choose the same move on the same tick, and throw it
at the same instant. `arbitrate_attack_clanks` then does exactly what it is for —
*"Close enough: both attacks are refused"*, cancelled by despawn **before**
`apply_hitbox_damage` asks any of them about a victim — and `clank_verdict` refuses
both whenever the damage `difference` is inside the window. **Two identical moves
have a difference of ZERO.** So every exchange clanks, forever.

⇒ **The tell is seat symmetry, measured across all 20** — exact equality on
damage/min, starts, damage dealt AND hitstun:

| fighter | dmg/min | starts | dealt | hitstun | |
|---|---|---|---|---|---|
| `npc_carl_stargan` | 0.00 / 0.00 | 3 / 3 | 0 / 0 | 0 / 0 | **LOCKSTEP** |
| `special_patent_clerk` | 0.09 / 0.09 | 51 / 51 | 9 / 9 | 11 / 11 | **LOCKSTEP** |
| `medic` | 0.41 / 0.41 | 49 / 49 | 41 / 41 | 118 / 118 | **LOCKSTEP** |
| `performer` | 0.49 / 0.49 | 18 / 17 | 9 / 9 | 86 / 86 | broke once, same outcome |
| `npc_emmy_noether` | 0.28 / 0.44 | 13 / 12 | 18 / 15 | 38 / 59 | genuinely asymmetric |
| the 15 that clear | — | 64/62, 182/184, 41/33, 70/54 … | — | — | **not one equal** |

**Lockstep among gate failures: 3. Among the fifteen passers: 0.**

⛔ **AND LOCKSTEP IS NOT "NO DAMAGE" — that framing is too strong and `medic`
refutes it.** His mirrored seats dealt 41 EACH and took 118 hitstun each: they
landed, symmetrically. ⇒ What lockstep proves is only that **the two seats never
diverged, so the bout carries one seat's worth of information reported twice.**
Whether it CAUSES the low damage is a further claim this instrument cannot
separate — which is precisely why it cannot measure these fighters.

⚠ For `special_patent_clerk` the stronger reading does hold, because two
independent lines agree: 9 damage and 11 hitstun across 51 presses is a fight
that barely connects, and his kit is authored to connect easily (below).

⇒ **SO THE ROW'S ANSWER, THIRD REVISION AND MUCH SMALLER THAN EITHER BEFORE IT:
of 21 grid ids, exactly ONE fighter genuinely fails the gate in a duel that
actually happened** — `npc_emmy_noether`, diverged on every field and still short
at 11% time-in-range against a roster median near 40%. Three are degenerate
mirror bouts, one is at threshold AND diverged by a single press, one is a bare
registration (Q98), one is unrunnable.

⚠ **The instrument flags LOCKSTEP at the point of measurement now**
(`scripts/measure_duel_roster.py`), with an arm pinning that the flag does NOT
fire on Emmy — a flag that fired on everything would explain away the only real
finding.

⛔⛔ **AND THE OBVIOUS FIX IS NOT THE FIX — the noise seed already exists, is
per-seat, and IS consumed. Measured before implementing it:**

1. `fighter_cognition_seed` (`brain_builders.rs:70`) hashes the participant id
   `"<character>#seat<n>"`, so two seats get DIFFERENT streams —
2. unless the character authors `preserves_mirror_symmetry`, which strips the
   seat so twins deliberately share one stream;
3. and the stream is consumed at
   `ambition_combat::brain::fighter::decision.rs:471`,
   `next_signed_unit(&mut state.noise)`, feeding press jitter scaled by
   `execution_noise` — 0.10 at rung 9, non-zero.

⇒ **THE INVERSION IS THE OPEN QUESTION.** `npc_emmy_noether` is the character who
authors `preserves_mirror_symmetry` — twins sharing one stream — and she is the
one fighter whose duel genuinely DIVERGED. The three lockstep bouts belong to
fighters who already have distinct seeds and non-zero jitter and stayed
bit-identical anyway.

**ANSWERED 2026-09-10, AND IT IS A CONJUNCTION — WHICH IS WHY FIVE SINGLE-CAUSE
MECHANISMS DIED ON IT.** Each of the five was a variable somebody could name, so
each arrived with a story attached and the story is what got tested. Both
surviving halves were measured, neither was argued:

> **Lockstep = (the rung-9 press jitter is identically zero) AND (nothing in the
> bout ever broke the stage's mirror symmetry).**

**Half one — the seats really are exact mirrors, and it is not a close call.**
`[sym]` in `smash_cpus_damage_each_other` reports each bout's worst departure
from `x0 + x1 = const, y0 = y1`:

| fighter | outcome | worst axis drift over 3613 ticks |
|---|---|---|
| `special_patent_clerk` | LOCKSTEP | **0.0000 px** |
| `npc_carl_stargan` | LOCKSTEP | **0.0020 px** |
| `medic` | LOCKSTEP | **0.0022 px** |
| `npc_emmy_noether` | diverges | **240.79 px** |
| `npc_pirate_admiral` | diverges | **446.62 px** |

⇒ **Five orders of magnitude with nothing in between.** A lockstep bout never
leaves a hundredth of a pixel of exact reflection; a divergent one leaves the
stage. And the mechanism was in the tree the whole time, in Emmy's own
`gameplay_description`: *"the reflection… BREAKS as soon as their observations
diverge (one takes a hit, one is launched further, one is nearer a ledge)"*.

⚠ **SCOPE, and it is not a formality: n=5, and the three lockstep subjects were
SELECTED on a statistic correlated with the one then measured.** This is strong
evidence for the mechanism and not yet a statement about the other fifteen.

**Half three, and it is the arm that makes the conjunction testable: WAKING THE
JITTER BREAKS LOCKSTEP.** Fifteen duels across every published rung, `035c56307`,
**every row carrying two distinct `seed=` values read at BIRTH** so the
shared-stream confound is excluded by reading rather than by inference:

| fighter | rung 1 | 3 | 5 | 6 | 9 |
|---|---|---|---|---|---|
| `medic` | SEP | SEP | SEP | SEP | **LOCKSTEP** |
| `special_patent_clerk` | SEP | SEP | SEP | SEP | **LOCKSTEP** |
| `npc_carl_stargan` (control) | SEP | SEP | **LOCKSTEP** | SEP | **LOCKSTEP** |

⇒ **Both load-bearing fighters separate at every rung where the jitter is alive
and lock at the one where it is dead** — and the jitter's reachable ceiling goes
2.25 → 1.81 → 1.38 → 1.16 → **0** across exactly those rungs. **The outcome
tracks the mechanism's own parameter monotonically and breaks where the
arithmetic says the term dies**, which a single separation could never have
shown.

⚠ **The numbers under the verdicts matter as much as the verdicts:** the clerk
deals **96/105 at rung 5 and 9/9 at rung 9**. The lockstep row is not merely
symmetric, it is a **different fight** — and a tenfold damage drop at the locked
rung is exactly what a threshold gate would misread as a fighter problem.

⚠ **THE CONTROL WAS NEVER A CONTROL, and its owner said so rather than reporting
it as a counterexample.** `npc_carl_stargan` is not monotone — LOCKSTEP at rung 5
with **7/7 starts and 0/0 damage**. A fighter that takes seven actions in a
minute has almost no surface for a one-tick nudge to act on, so his lockstep rows
measure his IDLENESS, not the jitter. ⇒ **A control must hold the mechanism's
PRECONDITION fixed, not just the treatment.** He measures the FLOOR of the
effect, and that reading was chosen after seeing the data and is labelled as one.

⛔ **AND A CONTENT FINDING FELL OUT OF HIM: he deals 55/62 damage at rung 3 and
44/66 at rung 6**, while the catalog listed him in `KNOWN_BARE_REGISTRATIONS` —
the exemption for ids that author *"nothing, not a body, not a policy, not a
moveset"*.

✔ **CHASED AND CLOSED 2026-09-10 (`30c15da29`), and the answer is that the
CATALOG was wrong, not the roster.** `npc_carl_stargan` authors a **locomotion**,
a **600-line moveset of his own** (`carl_stargan_moveset`, `pale_blue_dot` and
all) and **`max_health = Some(4)`**. `authors_a_body` is TRUE, so that exemption
had not been reached for him in a long time — **the list was right when written
and the character grew a body underneath it.**

⇒ **The duel probe is what caught it, and no census over declarations could
have:** his seat performs `carl_stargan_dash_attack` and `pale_blue_dot`, which
is not what a character who authors nothing does. **Q98 asked the maintainer
whether the grid may seat a bodiless character, quoting that exemption as
evidence — it is withdrawn.**

⚠ **AND THE ASSERTION IT CAME FROM CLAIMED A CHECK IT DOES NOT PERFORM.** The
message says *"not a body, not a policy, NOT A MOVESET"*; the predicate is
`authors_a_body || authors_only_policy || exempt` and consults no moveset. The
list is empty now, the message says what it checks, and **a new arm asserts every
entry is LOAD-BEARING** so an exemption cannot outlive its need again —
poison-verified by putting him back, which names him.

⚠ **AND ONE RUNG DOES NOT EXIST.** A rung-8 sweep returned *"two fighters shared
the stage for only 0 of 3600 ticks"* — not a fight that ended early, **a fight
that never began**: `smash_roster_at_levels` names each seat
`duelist_l{level}` and the smash experience publishes only
`l1, l3, l5, l6, l9`. ⇒ **The sweep's own validator checked `1..=9` — the
LADDER's range — while the roster needs a PUBLISHED POLICY, a strictly smaller
set.** Two vocabularies for one concept, and the guard was pointed at the wider
one, so a knob accepted a value the composition cannot seat and failed silently
and expensively at the far end. **Of the five seatable rungs, 1/3/5 are
rollouts-off and 6/9 are rollouts-on, so no seatable pair isolates jitter with
rollouts held constant** — the monotone series is the evidence; a controlled
contrast is not available in this composition at all.

**Half two — at rung 9 the jitter is not small, it is zero.** See
[D-RUNG9-NOISE](#d-rung9-noise--the-hardest-cpu-is-the-only-one-with-execution-noise-disabled)
below, which is its own row because it is a shipped defect independent of this
one. Distinct seeds are drawn and discarded, so **the seed reaches nothing at the
rung the game seats CPUs on** — which is why three fighters with distinct streams
behave identically.

⇒ **AND THAT DISSOLVES THE INVERSION RATHER THAN EXPLAINING IT.** Emmy's
`preserves_mirror_symmetry` buys nothing at rung 9, because at rung 9 *every*
character's stream is inert. She diverges because her STAGE broke, which is her
own design comment working as written. **The apparent paradox was an artefact of
believing the seed mattered.**

⛔⛔ **AND THE ADMIRAL'S ENTIRE ROW IS VOID: HE WAS A FIGHTER BEATING UP A
BRUTE.** Measured with a `[brain]` probe reading each seat's brain at BIRTH and at
the end: seat 1 was born `fighter` and ended `melee_brute`, having pressed
`call_the_shark` five times. `rebuild_dismounted_rider_brains` answered the
shark's death by handing the rider a brain derived from its kit — and
`dismounted_rider_brain_and_action_set` chooses between a skirmisher and a forced
brute, **consulting no template at all**. Fixed at `f77ba3a45`: a rider with no
`MountedBrainCache` never gave up a controller on boarding, so it has none to get
back. The same bout after the fix:

| | before | after |
|---|---|---|
| seat 1 brain at end | `melee_brute` | **`fighter`** |
| hitstun | [525, 324] | **[161, 142]** |
| knockouts | 2 | **4** |
| damage/min | 1.26 / 1.07 | **0.84 / 0.76** |

⇒ **A 201-tick hitstun split collapsed to 19.** Every admiral column above — and
the 54/27 start count that a ratio screen flagged as the roster's lone outlier —
measured a mismatched bout.

⛔ **NEXT IN THIS ROW: THE WHOLE 21-ID SWEEP IS A MEASUREMENT OF A COMPOSITION
THAT NO LONGER EXISTS.** Not one row: the fix changes any bout in which a rider's
mount dies, and neither the roster script nor the sweep log records whether one
did. **Re-run the full sweep before any band in that table is quoted again.**

⚠ AND `ladder_rig`'s *"no fighter brain ever took the noise seed"* is about a
FIXTURE that built brains without one. It is not a claim about the shipped brain,
and I quoted it as though it were.

⭐ Corroborated from the static side, which is what sent me looking: **the clerk's
kit is authored STRONGER than the goblin's on every axis** — total authored damage
207 vs 137, median hit-box half-extent 26 vs 20, median reach 42 vs 32, and
structurally identical (26 vs 27 moves, both exactly 6 with no Active window, both
exactly 2 Active-but-empty, which are their grabs and correct). His `tilt_forward`
reaches 58px against 42 and hits for 7 against 4. **A kit that good deals 0.09
only if the fight is barely happening** — a prediction from OUTSIDE the harness
agreeing with a symmetry seen inside it, which is what makes the artifact reading
convincing rather than merely available. His one damaging move all match is a single
`patent_clerk_dash_attack` — the one moment the mirror broke.

⇒ **So this is a defect in the INSTRUMENT, and the fix is the one `ladder_rig`
already implements**: that rig refuses to report a bout where *"no fighter brain
ever took the noise seed, so every run of this bout is identical."* This harness
has no such guard. **Give the duel a noise seed or seat the two sides
asymmetrically**, and re-measure — until then it cannot measure any fighter whose
mirror stays in lockstep, and will keep reporting them as inert.

⚠ **WHAT THIS DOES NOT EXPLAIN, and it may be a second finding:** the clerk's CPU
picks only **4 distinct moves of 26** (`synchronize_clocks` 17, `tilt_forward` 17,
`clerk_grab` 16, dash attack 1) where the pirate admiral picks 12. Symmetry
explains why those four never land. It does not explain why there are four.

⚠ **`goblin` is the counter-example on the other side:** 17% in reach, second
lowest measured, 55/51 starts, and it clears the gate. ⇒ **Time-in-range and
damage correlate at the extremes and not in the middle.** George is highest on
everything, Carl lowest on everything, and between them neither predicts the
other.

⚠ **`performer` fails at 0.49 against a 0.5 gate** — at the threshold, not below
it in any meaningful sense. Counting it as a failure carries a rounding artifact
into a headline; the genuine count is FOUR, in two mechanisms.

⚠ **`npc_alice` is UNMEASURABLE**, not inert: she panics in
`bevy_render::sync_component.rs:55` on a `PendingSyncEntity` the headless
composition never inserts — a despawn hook on a camera component. She is the only
one of twenty-one that reaches it, so the trigger is hers and unexplained; nobody
has run her in the windowed app, so "safe when shipped" is an inference.

⚠ **The measurement WINDOW is not deterministic even though the fight is.**
`perfect_cellular_automaton` ran 3580 and 3602 ticks with byte-identical damage,
hitstun and in-reach counts. Every headline figure is a rate over that
denominator, so third-digit wobble is expected and means nothing. The seating
transaction is where to look, not the sim — and whether that is IO-bound startup
or genuine non-determinism is NOT yet read.

⇒ **AND THERE IS EXACTLY ONE FIGHTER-VARYING TERM IN MOVEMENT SCORING, which is
where to look FIRST.** `walks_off` — the ledge rule deciding whether closing is
safe — is `floor_ahead(toward) < half_extent.x * 2.0`. **It scales with the
body's WIDTH**, so a wider fighter reads "approach walks me off" from further
back and retreats where a narrower one advances. Nothing else in
`movement_options` differs by fighter at all.

⭐ The dependence is now pinned by
`options::tests::a_wider_body_refuses_an_approach_a_narrower_one_takes` (two
bodies at one spot differing only in half-extent, plus a mid-platform control;
poisoned by replacing the width term with a constant, which kills that test alone
and leaves 37 standing). It was real, unstated and unguarded until 2026-09-10.

⚠ **THIS IS A CANDIDATE, NOT THE CAUSE**, and the guard asserts only that the
term exists and varies. ⇒ **The join that would settle it: `half_extent.x` per
fighter against time-in-range.** A correlation implicates the term; equal widths
across a 6× engagement spread would exonerate it, which is as useful — the same
shape as the `lifts` coupling that turned out real and inert.
⚠ Take the width from the COMPOSED WORLD, not from source: the character catalog
says in its own header that it is *"NOT a second body-construction authority"*,
and bodies are assembled from registered `CharacterDefinition` values.

⚠ **And the acceptance test cannot see it**, because `FIGHTER` is a const set to
the one fighter that passes. `two_cpus_in_the_shipped_composition_damage_each_other`
asserts `>= 0.5` of pool per minute and would fail on two of the three sampled —
so the guard is sound and its POPULATION is one. ⇒ Widening it to the roster is
the first concrete step, and it will go red immediately; that is the point.

⇒ **SO THIS ROW IS TWO ROWS AND SHOULD BE SPLIT WHEN EITHER IS PICKED UP.** The
"barely press" band is a movement/engagement question and the width term below is
its first candidate. The "press and convert nothing" band is a kit question with
its own subject — `special_patent_clerk` at 51 starts and 0.09 damage is the
sharpest single number in the sweep and does not belong in the same investigation.

**Acceptance, and it is now two claims because the row holds two defects:** the
duel gate is asserted over a representative set of AUTHORED fighters rather than
one, and every fighter in that set fights; and the Smash grid's seating rule is
RULED ON rather than assumed — see [Q98](awaiting-maintainer-decision.md), because
the bodiless character on the grid is there by a deliberate maintainer placement
and removing him retracts it.

⚠ **THE SET MUST BE `PLAYABLE_ROSTER` OR THE ASSEMBLED GRID CROSSED AGAINST IT,
NOT `authored_movesets::tables()`.** That list's own header warns it is *"NOT THE
SELECTABLE CAST"*, and it has already produced one census that read as a
statement about the game and was not. A bare registration in the fighter table
makes "N of 19 are inert" a number that travels and is wrong. ⚠ Until then, no
CPU-quality number quoted from this harness travels without naming its fighter.

⚠ n=1 run per fighter and all three are MIRROR matches. The contrast is
controlled (one variable) but the absolute figures are single samples; re-measure
before tuning anything.

## P2 — current engine/game work

### D-TETHER-LINE — DONE 2026-09-10; the reel publishes a fact, not a component

**Re-derived before starting, and the row was half true.** A tether line already
existed (`ambition_render::rendering::tether`) and neither `ambition_render` nor
`ambition_sim_view` carries a `ambition_demo_smash` dependency — so the layering
half of the acceptance was already met. What was missing is what the row's first
sentence actually says: **the REEL published nothing.** The line drew from
`grab_reach`, which is the capture box's reach under the move clock, so a fighter
latching a ledge and being reeled across the stage drew NOTHING.

⇒ `BodyLineAnchor` — a generic body-to-world point, in `ambition_platformer2d_core`
— is inserted and removed by the smash ruleset beside `TetherReel`, projected by
both read models as `line_anchor`, and consumed by the line road as
`grab_reach.or(line_anchor)`. **Presentation never learns what a `TetherReel` is,
and a third line mechanic draws itself by publishing the same component.**

⛔ THREE REASONS IT IS ITS OWN FACT rather than reusing a neighbour. `wire_anchor`
is *"where the wire she is HANGING FROM comes down from"* — suspended beneath a
point. `grab_reach` is where a live capture box reaches TO. A reel is neither: the
body is pulled TOWARD a point it latched, and folding it into either makes one
field mean two mechanics with no way for a renderer to ask which. ⚠ And it is a
COMPONENT rather than a `BodyMotionFacts` field for a structural reason — those
facts are rebuilt from the motion model every tick, so a ruleset writing there is
overwritten before anything reads it.

⚠ THE ANCHOR IS REMOVED AT BOTH REEL-END SITES, not just the one. A body that
stopped reeling and kept its anchor draws a rope to a ledge it is no longer
attached to — worse than no line, because the player reads it as a live threat.

**Acceptance: met.** Guard `a_fighter_reeled_to_a_ledge_gets_a_line_without_a_grab`
(no grab anywhere, only the latched anchor) with a control
(`a_fighter_lined_to_nothing_gets_no_line`), poisoned by reverting the line road
to `grab_reach` alone — which kills the new test and leaves the other four
standing. render 258, sim_view 102, core 546, demo_smash 268.

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

**MEASURED 2026-09-10, and the row's premise is understated: the CPU is not
merely failing to SELECT some attacks, its kit MISLABELS them.**

`attack_kit_of` resolves every press with `move_for_directional_verb`, while a
comment beside it asserted that was "the same function `trigger_moveset_moves`
calls". It is not. The press road calls
`move_for_attack(base, dir, grounded, RUNNING)`; `move_for_directional_verb` is
that function with the running branch skipped.

⛔ **CORRECTED 2026-09-10 BY DAMAGE-BY-MOVE: THE CPU ALREADY PERFORMS DASH
ATTACKS.** Seat 0 dealt 36 damage with `pirate_admiral_dash_attack` under the
MISLABELED kit. The kit's own enumeration cannot reach the move — that census
stands — but the press the brain issues produces it anyway, because the press
road resolves the stance itself. ⇒ **The defect is the MISLABEL alone**: the
brain scores one move's frame data while the body performs another. Anything
below that reads as "the CPU cannot dash attack" is my error and is wrong.

⇒ **Eighteen of eighteen shipped fighters author a dash attack no press IN THE
KIT reaches.** Census over `authored_movesets::tables()`: 22 authored
moves that can hit are unreachable by any brain press, and they are three
families — 18 `*_dash_attack` (the defect), 3 `*_jab2` (cancel-chain successors,
correctly absent from a fresh-press kit) and `dive_stomp_uncharged` (a
`when_refused` fallback, likewise).

⚠ **The absence is the smaller half.** While the body runs, the brain scores
`jab`'s frame data, issues the attack press, and the press road performs
`{base}_dash` — a candidate whose `move_id` and `frames` describe a different
move than the one the press produces, so every scoring term downstream (startup,
reach, damage, frame advantage) reads the wrong move.

⛔ **THE FIX IS BEHIND `--features truthful_attack_kit` ON
`ambition_platformer2d_actor_monolith`, DEFAULT OFF — runnable, not landed.**
⭐ ONE code path, not a `#[cfg]` split: with the feature off `running_now` is a
compile-time `false` and `move_for_attack(verb, dir, grounded, false)` falls
through to `move_for_directional_verb`, so the shipped behaviour is today's BY
CONSTRUCTION rather than by a second arm somebody has to keep in step. Verified
both ways (6 passed + witness ignored / 7 passed), and `app_it` 612 with it off.

⛔⛔ **AND A SECOND FIGHTER SAYS THE GATE IS CALIBRATED TO ONE.** `npc_emmy_noether`
at rung 9 — **HEAD: 0.28 / 0.44, which already FAILS the 0.5 threshold**, 0
knockouts, hitstun [38, 59]. Truthful kit: **0.28 / 0.44, byte-identical**, same
damage-by-move. ⇒ Another shipped fighter fails this acceptance test TODAY with
nothing changed, so "the fix fails the gate at rung 9" is much weaker evidence
than it looked — the gate does not hold across fighters at HEAD. And the flag is
a no-op wherever the stance never triggers, which is the control the one-code-path
shape gives for free.

⚠ **THE HARNESS IS THIS TEST, NOT `ladder-rig`.**
`cargo test -p ambition_app --test app_it -- smash_cpus_damage_each_other::two_cpus --nocapture`,
with `FIGHTER` / `RUNG` / `TICKS` at the top of
`game/ambition_app/tests/smash_cpus_damage_each_other.rs` selecting the cell.
`ladder-rig` CANNOT answer this: its default duelists bind no `attack_dash` (its
own header says so) and `ambition_demo_smash_app` has no `ambition_content` edge,
so Ambition's 19 authored movesets are not seatable there at all. Resolving with
`move_for_attack` makes the kit truthful; it also re-prices how the CPUs fight.
MEASURED: it reddens
`smash_cpus_damage_each_other::two_cpus_in_the_shipped_composition_damage_each_other`
("the CPUs are not fighting") and
`smash_in_the_host::launched::an_up_tilt_launches_much_further_at_a_high_percent`
— both green at HEAD, both failing reproducibly in isolation, neither flaky.
This row's own owner document rules on that case: a change that re-prices
matchups *"needs the ladder rig (`brain::fighter::evaluation` + `scenarios`), not
a coordinator's judgement"*. Two acceptance tests reporting a worse fight IS the
rig speaking, so landing it anyway would be exactly the judgement the doc
forbids.

⭐⭐ **TRACED 2026-09-10 RATHER THAN ARGUED, and the numbers change what this
row IS.** The duel rig (`smash_cpus_damage_each_other`, pirate admiral, rung 9,
3613 ticks, `decided None`, 2 knockouts both ways) reports:

| kit | seat 0 | seat 1 | hitstun ticks |
|---|---:|---:|---|
| HEAD — mislabeled | **1.26** | **1.07** | [525, 324] |
| truthful (`move_for_attack`) | **0.47** | **0.86** | [81, 191] |

Damage per minute roughly HALVES and hitstun collapses by ~85% on seat 0. Neither
match decided early, so this is not the "a fight good enough to end fast reads as
less damage" artefact the threshold's own comment warns about — I checked that
first and it is not what happened. The CPUs simply land far fewer hits.

⇒ **THE MECHANISM, MEASURED — AND IT CORRECTED MY FIRST READING OF IT.** The
press road makes a run PRE-EMPT the smash gesture: while running, `attack` and
`smash` BOTH resolve to `{base}_dash`, so a truthful kit has ONE attack candidate
whenever the body runs where the mislabeled one offered the standing menu. From
that I wrote *"the CPU never stops running"* — and then instrumented the duel for
stance, which does not support it:

| kit | seat 0 running / grounded | seat 1 | grounded ticks |
|---|---|---|---|
| mislabeled | 258/1908 = **14%** | 353/1822 = **19%** | 1908 / 1822 |
| truthful | 402/1334 = **30%** | 569/1457 = **39%** | 1334 / 1457 |

⛔ At HEAD these bodies run only 14–19% of their grounded time, so "never stops
running" was false. What the trace actually shows is a FEEDBACK LOOP: making the
dash attack reachable roughly DOUBLES the running fraction and cuts grounded time
by a third, and the damage falls with it.

⭐ **I THEN PROPOSED THE COUPLING THAT COULD CARRY IT, AND MEASURED IT FALSE
TOO.** `generate_options` calls `movement_options(&view, situation,
!lifts.is_empty())` — the one wire from the attack kit to movement scoring.
Across all 19 shipped fighters that boolean is the SAME standing and running, so
the wire is INERT and cannot be what moved the bodies. Pinned by
`stance_coupling::no_shipped_fighter_changes_its_lift_availability_with_stance`.

⛔⛔ **IT DOES FIRE FOR THE WRONG VERSION OF THE FIX, WHICH EXPLAINS THE OTHER
FAILURE COMPLETELY.** Poisoning that guard with my specials-collapse mistake
reddens every fighter — **a fighter's lifting move IS its up-special**
(`steam_lift`, `starstuff`, `scramble_leap`, `smoke_fold`, …), so collapsing
specials strips every body's RECOVERY from its kit while running and movement
scoring changes for a body that no longer believes it can get home.

⭐⭐ **THE MOVE DISTRIBUTION, MEASURED — F6's step 1 finally answered with counts
rather than a story.** Starts per `(move_id, instance)`:

| | HEAD (mislabeled) | truthful kit |
|---|---|---|
| seat 0 | 54 starts — `pirate_grab` 13, **`jab` 10**, `dash_attack` 5 | 43 — **`pirate_fthrow` 5, `pirate_pummel` 5**, `pirate_grab` 4, jab out of the top eight |
| seat 1 | 27 starts — `jab` 4, `dash_attack` 3 | 39 — `pirate_grab_dash` 5, `dash_attack` 3 |

⛔ **THE CPU DOES NOT SPAM THE DASH ATTACK** — that is the third mechanism
refuted. It leaves seat 0's top eight entirely and holds at 3 for seat 1.

⇒ What moves is `jab` and the GRAB CHAIN: jab leaves the running menu by
construction and both seats shift toward grab → pummel → throw. Seat 1 starts
MORE moves and deals LESS damage.

⭐⭐ **ISOLATED, by damage attributed by move** (joined on
`ResolvedBodyHit::attacker_move_instance`, `+0 unclaimed` both runs):

| | HEAD | truthful kit |
|---|---|---|
| seat 0 | **122** — dash 36, **jab 33**, grapeshot 18, … | **44** — grapeshot 18, heave_to 10, dash 9 |
| seat 1 | **157** — **jab 126**, air_down 31 | **25** — heave_to 10, dash 9, tilt_up 6 |

⇒ **Jab is 159 of 279 damage (57%) at HEAD and deals ZERO under the truthful
kit.** The whole drop is jab's contribution vanishing.

⛔⛔⛔ **THREE RUNGS: THE EFFECT IS RUNG-DEPENDENT AND
HELPS AT THE BOTTOM. The rung-9 numbers above are ONE FIGHT.** Same duel, `RUNG` 6:

| rung | HEAD | truthful | jab damage |
|---|---|---|---|
| **9** | 1.26 / 1.07 | **0.47** / 0.86 | 159 → **0** |
| **6** | 1.56 / 1.69 | 1.52 / 1.39 | 68 → **161** |

Jab, HEAD → truthful: 159 → **0** at rung 9; 68 → **161** at rung 6.

| rung | HEAD | truthful | verdict |
|---|---|---|---|
| **3** | 0.78 / 0.61 | **0.86 / 0.76** | truthful kit is BETTER on both seats |
| **6** | 1.56 / 1.69 | 1.52 / 1.39 | −3% / −18%, both far above the 0.5 gate |
| **9** | 1.26 / 1.07 | **0.47** / 0.86 | −63% / −20%; the only rung that FAILS |

⇒ **The direction is consistent and it is not "the fix is a regression": the
higher the rung, the worse the truthful kit does, and at the bottom of the ladder
it HELPS.** That is a coherent story about a weaker brain benefiting from an
honest menu and a stronger one being disturbed by it — and it is still one fight
per rung, so it is a direction, not a curve.

⚠ Rung 12 is not a sample: the ladder has no such entry, both fighters failed to
seat, and the test's own 600-tick floor REFUSED to report rather than divide by
zero ticks. The floor doing its job is why that run is absent from the table
instead of sitting in it as a number.

At rung 6 the cost is ~3% / ~18%, both far above the 0.5 threshold, with MORE
knockouts (2 v 1), and **jab is the biggest damage source under the truthful
kit**. ⇒ "The truthful kit halves CPU damage" is a rung-9 statement and must not
be quoted without it. The fix still fails the gate — which is calibrated at rung
9 — but the reason to hold it is "one rung regresses and we do not know why",
not "the fix makes the CPUs worse".

⛔⛔⛔ **RE-MEASURED 2026-09-10 AFTER `f77ba3a45`, AND EVERY NUMBER ABOVE IS
VOID: THE SUBJECT OF THIS ENTIRE TABLE — `npc_pirate_admiral` — WAS A FIGHTER
BEATING A BRUTE.** He is the only fighter of twenty that mounts anything; his
shark died, `rebuild_dismounted_rider_brains` handed the rider a kit-derived
brain, and seat 1 ran `melee_brute` for the rest of every bout above. **The row's
headline — *jab is 159 of 279 damage (57%) at HEAD* — is a fact about a BRUTE's
jab policy**, not about the fighter brain this row is about.

⭐⭐ **THE CLEAN SUBJECT SAYS THE TRUTHFUL KIT IS AN IMPROVEMENT.** `medic` — no
mount, therefore no dismount confound, same rung 9, same 3613 ticks, ONE
variable:

| medic, rung 9 | damage/min | hitstun |
|---|---|---|
| feature OFF | 0.41 / 0.41 | [118, 118] |
| **truthful kit** | **0.45 / 0.45** | **[132, 132]** |

**+10% damage, +12% hitstun.** And the admiral's own numbers, both arms
re-measured with the dismount fix in:

| admiral, rung 9 | old table | corrected |
|---|---|---|
| seat 0 | 1.26 → 0.47 = **−63%** | 0.84 → 0.47 = **−44%** |
| seat 1 | 1.07 → 0.86 = −20% | 0.76 → 0.86 = **+13%, an IMPROVEMENT** |

⇒ **Across three subjects at rung 9 the truthful kit is an IMPROVEMENT (`medic`),
a NO-OP (`npc_emmy_noether`, byte-identical both arms) and MIXED on the one bout
that was measuring a brute.** *"The truthful kit halves CPU damage"* was a
statement about the contaminated subject and must not be quoted again.

⚠ **ONE ANOMALY, FLAGGED RATHER THAN SMOOTHED: the admiral's TRUTHFUL arm is
byte-identical before and after the dismount fix** (0.47 / 0.86, [81, 191], 2 KOs
and every `[dealt]` figure), while its feature-off arm moved a great deal. Both
arms start `call_the_shark` 5–6 times. ⇒ **Summoning is not dismounting**, and
the event the fix is keyed on is a mount DYING; if the truthful arm has zero
mount deaths this is exactly right and there is no mystery. **Unmeasured. Nothing
here is built on that arm.**

⛔⛔ **AND THE HOLD STILL STANDS, BUT FOR A DIFFERENT REASON THAN THE ROW GIVES.**
Both acceptance tests still redden with the feature on, verified with a control
at today's HEAD and a witness that the flag reached the build:

| `cargo tree -e features -p ambition_app \| grep 'actor_monolith feature "truthful_attack_kit"'` | verdict |
|---|---|
| **1** (feature routed in) | `test result: FAILED. 0 passed; 2 failed` |
| **0** (reverted) | `test result: ok. 2 passed; 0 failed` |

⇒ **THE TWO FAILURES ARE DIFFERENT KINDS AND THE ROW HAS BEEN TREATING THEM AS
ONE.** Their own messages:

1. *"seat 0 took 47% of its pool per minute — the CPUs are not fighting"* — a
   **0.47 against a 0.50 gate, on the contaminated subject**, and `medic` fails
   that same gate at **0.41 with nothing changed at all**. This is a threshold
   calibrated to one fighter, and that fighter's own HEAD number just moved from
   1.26 to 0.84 underneath it.
2. *"the SAME move on the SAME fighter lifted them 26.6px at 0% and 26.6px at
   1427% — the percent meter is not reaching the launch"* — **a MECHANIC that
   stops working.** Knockback ceasing to scale with damage is not a tuning
   regression, and no amount of re-calibrating a damage gate addresses it.

⛔⛔⛔ **AND (2) IS NOT A BLOCKER EITHER — MEASURED 2026-09-10, THE SAME DAY I
CALLED IT THE REAL ONE. THE FIXTURE'S OWN STRIKE NEVER LANDS.** Instrumenting the
victim's damage meter across the strike, both arms:

| up-tilt fixture | feature OFF | feature ON |
|---|---|---|
| percent = 0 | meter **0 → 10**, rose **3.4 px** | meter **0 → 0**, rose **26.6 px** |
| percent = 1427 | meter **1427 → 1437**, rose **372.8 px** | meter **1427 → 1427**, rose **26.6 px** |

⇒ **The feature-off arm scales 110×, so the percent meter is fine. In the
treatment arm the meter does not move by a single point in either match.** The
identical 26.6px is the victim doing something else entirely — which is exactly
why it does not vary with percent. **There is no kit-to-knockback coupling to
find; the hit was thrown where the victim was not.**

⚠ **EVERY INPUT TO THAT FIXTURE'S KNOCKBACK IS A LITERAL IN ITS OWN FILE** —
`UP_TILT_DAMAGE`, `UP_TILT_KNOCKBACK`, `UP_TILT_GROWTH`, `UP_TILT_LAUNCH_DIR`,
the half-extent and the anchor — and the percent is written into the victim two
statements before the strike. ⇒ **There is no way for the launch to stop scaling
EXCEPT by the victim not taking the hit**, so *"the percent meter is not reaching
the launch"* is a conclusion that fixture is never entitled to draw. **It has
drawn it wrongly three times**: once with the second strike inside the first's
hitstop, once with the victim walking clear of a 48px box, and now.

✔ **Guarded 2026-09-10: the fixture asserts its own strike landed before
interpreting the rise**, with a message pointing at what moved the victim out of
the box rather than at the launch formula. Green on shipping code;
poison-verified by aiming the hitbox 9000px away — `FAILED`, 0 compile errors,
naming the miss.

⛔⛔ **SO THE HOLD NOW RESTS ON (1) ALONE, AND (1) IS A THRESHOLD, NOT A DEFECT:**
0.47 against 0.50 on a subject whose baseline moved 1.26 → 0.84 underneath it,
which a second shipped fighter fails at **0.41 with nothing changed at all**.
⇒ **Neither red test is evidence about the fighter brain.**

⚠ **AND THE UP-TILT FIXTURE IS NOT A GATE ON THIS ROW'S CHANGE — IT IS A GATE ON
"THE CPUs STILL WALK THE WAY THEY DID IN AUGUST".** It shares its app with LIVE
CPU fighters whose walking it does not control, and its park-immediately-before-
the-strike was calibrated against one particular way of walking. **This row's
whole change is a change to how CPUs walk.** A fixture that cannot survive that
cannot arbitrate it.

⇒ **NEXT IN THIS ROW: the decision is now a calibration question for the
maintainer, not a measurement question.** The remaining evidence is a 0.50 gate
that no longer holds across fighters at HEAD. Either the gate is re-derived
against the roster it is supposed to police, or the fix lands and the gate moves
with it — and that is Jon's call, not a coordinator's.

⚠ **AND A TRAP FOR WHOEVER MEASURES THIS NEXT: `truthful_attack_kit` CANNOT BE
ENABLED BY FLIPPING THE MONOLITH'S `default`.** All seven consumers take that
crate with `default-features = false` — the facade, runtime, provider, sim_view,
rollback_ggrs, touch_input and `ambition_content` — so **a default nobody takes
is not a default**, and flipping it produces a full table of plausible numbers
from a binary in which nothing changed. Route it through a consumer's own
dependency line and **prove it arrived with `cargo tree -e features`**, which is
the only witness that a flag reached the thing under test.

⚠ n=2 and the two disagree. Next measurement is MORE SAMPLES (other rungs, other
fighters) before any mechanism is fitted. What follows described the rung-9 fight
and is kept only as that.

⛔ **AT RUNG 9, JAB IS STARTED ZERO TIMES, NOT MERELY LANDING LESS.** Full
distribution rather than a top-eight: seat 0 starts 11 distinct moves, seat 1
starts 13, and `jab` is in neither — against 10 and 4 starts at HEAD. The
truthful kit removes jab from the RUNNING menu only, standing menus are
byte-identical, and these bodies stand 60–70% of grounded time, so the brain is
declining jab on ticks where it is still offered.

⇒ **Next instrument: the kit AT THE MOMENT OF DECISION** — what
`generate_options` was handed and what it chose, on standing ticks. ⚠ Check one
cheap thing first: `power` is normalised by `kit_max_damage`, recomputed per tick
over the CURRENT kit, so changing the kit's membership re-prices every candidate
in it, not only the ones that changed.

⚠ Four mechanisms proposed on this row, three measured false. Bring an
instrument, not a fifth story.

⛔ **THAT IS F6, NAMED IN THE OWNER DOCUMENT, AND IT IS THE REAL BLOCKER.**
`fighter-brain.md` §F6: *"A fighter repeatedly selecting one converted/dash move
can arise from independent movement and attack scorers rather than the moveset
itself"*, and its instruction is to trace the scored movement+attack PAIR and
identify the missing opportunity/commitment term before adding randomness or
per-move caps. This trace is step 1 of that procedure, done.

⇒ **Next concrete step is F6's step 2: the term that lets a brain choose to STOP
RUNNING because a standing option scores better.** Until it exists, a truthful
kit is strictly worse than a mislabeled one, which is why the fix stays held —
and that is a statement about the SCORER, not about the resolver.

⚠ The evaluation rig proper (`brain::fighter::evaluation`) cannot referee this:
its kit is synthetic (`rig_uptilt`, `rig_smash`) with no dash stance at all, and
it measures APM and distinct frames rather than damage. The duel rig above is the
instrument that can see it.

⇒ **Do not run the fix through `brain::fighter::evaluation`** despite the owner
doc naming it: that rig cannot see the subject. Use the duel rig, and re-run the
table above after F6's term lands — the fix is right the moment those two numbers
come back up.

The witness is
`a_running_body_is_offered_the_dash_attack_its_press_would_actually_produce`,
`#[ignore]`d with that reason and green the moment the resolver changes.

⚠ AND A CORRECTION TO MY OWN FIRST ATTEMPT, kept because it is the reusable
part: I initially redirected `SPECIAL` to `ATTACK` while running too. The press
road does not — it resolves a special in an EARLIER branch that never reaches
`move_for_attack`, and its `base_verb` is only ever Attack or Smash. That
collapsed a running fighter's whole kit to the single dash attack and took its
specials away, reddening two DIFFERENT acceptance tests. ⇒ Copying "the rule the
production road uses" means copying where the road APPLIES it, not just what it
says.

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

✔ **THE CENSUS LANDED 2026-09-10** —
`every_damageable_body_is_identified` in `app_it`, a runtime query over
`CenteredAabb + ActorFaction`: **the strike road's own required pair and nothing
narrower**, because a filter of my choosing would census my opinion about who can
be hit. First reading:

```
[identity] 4 damageable bodies, 0 without a `SimId`;
           identified: ["placement:NpcSpawn-0017", "placement:census_boss",
                        "placement:census_enemy", "slot:0"]
```

⛔⛔ **AND THE ANTI-VACUITY FLOOR FIRED ON THE FIRST RUN, WHICH IS WHY THE
FIXTURE SPAWNS ANYTHING AT ALL.** The sandbox world alone, 120 frames in, holds
**TWO** damageable bodies — so a census of it would have printed *"0 without a
SimId"* over a population of two and read as a clean bill of health for the whole
tree. **The floor refused the READING rather than the tree**, and the answer was
to exercise more construction roads rather than to lower it.

⚠ **THE FLOOR IS PER-ROAD, NOT A TOTAL, and that is not decoration**: if
`spawn_boss_at` silently stopped producing a damageable body, a total would still
clear on the sandbox cast plus the enemy and the census would report health for a
road it no longer travels. Each named road must appear in the identities.

⭐⭐ **SAMPLED EVERY FRAME, NOT ONCE AT THE END — and that answers a question the
end-state version had to ASSUME.** A body that is damageable for three frames
before its identity arrives is exactly the defect this row is about, and an
end-of-run reading cannot see it. **MEASURED: zero unidentified across 120
frame-samples after the spawns.** ⇒ **`SimId` arrives WITH the body, never
after** — so there is no window in which the strike road can reach a body it
cannot name, and **a build-site assertion is therefore safe to consider** rather
than being a race nobody has characterised. ⚠ If the two arms had disagreed —
per-frame dirty, end-state clean — that would have been the finding.

✔ Poison-verified twice, at both moments: stripping one body's `SimId` at the end
fires the census arm and NAMES it
(*"1 of 4 … [\"488v0 (Feature actor npc: Kernel Guide NPC)\"]"*), and stripping
one MID-FLIGHT fires the per-frame arm naming the frame (*"1 frame-samples …
first at Some((3, …))"*). 0 compile errors both.

⛔ **WHAT THIS DOES NOT SAY, and it must not be read as more: THREE ROADS ARE
COVERED, NOT SEVEN.** The authored `NpcSpawn` placement, the enemy road, the boss
road and the player slot. `construction/mod.rs` also mints identity for **giants,
hands, shrines, riders and summons** — none of those is driven here, so none is
measured. **A road nobody drives is not covered**, and adding one is how this
census grows. The test says so in place.

**Acceptance:** the census half is MET — the population is measured at runtime and
named, and it is empty for the roads driven. ⛔ **The second half is NOT: the
invariant is still asserted at READ time, not where bodies are BUILT.** The row's
own argument is that a missing target identity is a construction failure and the
resolver can only report it, so a census in a test is a REPORTER too — a better
one than the resolver's coincident-pair `debug_assert`, and still not the
construction-site assertion the protocol asks for.

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

✔ **RE-MEASURED 2026-09-10 AND THE SMASH ITEM IS DONE TOO: the census is
CLEAN.** `scripts/measure_id_prefixes_spelled_twice.py` at HEAD — **39 prefixes
appear as a literal, exactly ONE in both a producer and a consumer, and it is
`npc_`**, the false positive this row already names and must not "fix".

`respawn_platform_` is single-owned: `RESPAWN_PLATFORM_PREFIX` is the one
spelling, `respawn_platform_id` builds from it and `is_respawn_platform_id`
parses with it. ⭐ **And the comment that landed with it did the thing this file
keeps asking for — it MEASURED its own claim and corrected it.** A first draft
said *"no test would have said so"*; restoring the two-literal form with the
builder renamed and the reader's copy left behind **fails one of the five
respawn arms and passes four.** ⇒ The suite is not blind there, but four fifths
of it is, and the arm that catches it does so as a side effect of the platform
set it reads rather than because anything asserts the two spellings agree.

⇒ **Nothing is left in this row for either lane.** It stays open only as a
STANDING RE-MEASUREMENT: the next producer/consumer pair a sweep finds. ⚠ Read
the two counts separately when it is re-run — the literal count is expected to
grow with the tree and says nothing; **the both-sides count is the row**, and a
rise in it names its own subject.

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

**THIRD SIGHTING 2026-09-10, AND IT WAS NOT A FLAKE — which is the finding.**
`workspace (default features)` went red while every changed crate passed
individually. That is the exact signature the two sightings above describe, and
two agents spent hours on it generating flake-adjacent hypotheses: feature
unification, `relativity` turned on by a sibling, `content_pack`-gated tests, a
downstream dependent, load. **Every one was measured and eliminated.**

⇒ **The cause was a `.md` FILE.** `no_planning_doc_names_a_condition_the_engine_does_not_publish`
reads planning documents at runtime, and a new census page cited a rollback
SCHEMA row (`feature.switch_on`) that the guard read as a misspelled condition id
(`world.switch_on`). Deterministic, reproducible on two machines, and invisible
to every per-crate run because **no crate had changed in a way that mattered**.

⇒ **So "workspace red + per-crate green" now has a non-flake explanation that was
not on this row's list, and it should be checked FIRST because it is free:** did
anything change that a test READS but does not COMPILE — a planning document, a
baseline file, an asset manifest, an LDtk world? A diff grouped by crate cannot
show it. See the fifth-species entry above.

⭐ **AND THE ROW'S OWN ADVICE WAS PAID FOR A THIRD TIME BEFORE IT WAS FOLLOWED.**
A reproduction was piped through `tail`, which writes only at EOF — eighty minutes
of a live run and a hung run are the same 0-byte reading, and when cargo exited
the pipeline never flushed. ⇒ Redirect to a FILE and read the file WHILE it runs:
progress is a `wc -l` and a failure is a `grep`, instead of a question nobody can
answer until the end. Both agents' runs that finally named things were unpiped.

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

⭐⭐ **AND THE CITATION CHECKER IS THE WRONG INSTRUMENT FOR HALF OF THIS ROW —
THE INTRA-DOC LINK RATCHET IS THE OTHER HALF, AND IT WAS RED AT HEAD.**
`check_planning_citations.py` reads planning markdown. **A carve leaves its
references behind in RUST DOC COMMENTS too**, and nothing in this row pointed at
the guard that sees those. `scripts/check_doc_link_ratchet.py` was **RED at HEAD
on 2026-09-10** — 4 crates, 143 → 156 — and two of the eleven new breakages are
exactly this row's species:

- **A CARVE:** `ConstructionDomain` moved to
  `ambition_platformer2d_shared_tangle::construction`, and
  `ambition_platformer2d_actor_monolith`'s module doc still named it bare.
- **A DELETION:** `projectile_reaches_boss` documented itself as *"the swept
  sibling of `ecs_hit_event_hits_boss`"* <!-- cite-ok: the row RECORDS the deleted name; that the prose outlived its subject IS the finding --> — a predicate **A2 deleted** with the
  rest of the discrete family. The prose outlived its subject by two rows.

Cleared at `6b30dd644`, ratchet green, every crate exactly at baseline, 156 → 145.
⚠ The other nine were escaped-bracket, private-target and module-scope defects,
not carve residue — **fixed as the eleven the ratchet NAMED**, because its
baseline records *which* links are broken and paying a regression off with an
unrelated repair no longer restores the number. The 145 pre-existing are
deliberately untouched.

⛔ **INSTRUMENT NOTE, because it produced a wrong reading first: a default
`cargo doc` reported the monolith CLEAN while the ratchet called it red.** It was
a **cache hit** — an incremental doc build emits warnings only for crates it
actually recompiles, and a crate it skips contributes silence indistinguishable
from success. ⇒ **Confirm the crate appears under `Documenting` before believing
a zero.**

⇒ **Add the ratchet to this row's checklist beside the range-form citation
check.** A carve author who runs only `--vanished A..B` has checked the planning
prose and none of the doc comments.

### D-RUNG9-NOISE — the hardest CPU is the only one with execution noise disabled

**Owner:** [fighter brain](engine/fighter-brain.md), the authored ladder. Found
2026-09-10 by ToothbrushAmbition while deriving a rung sweep for D-CPU-INERT;
split out because it is a shipped defect that row does not own.

⛔⛔ **AT RUNG 9 THE FIGHTER PRESS JITTER IS IDENTICALLY ZERO FOR EVERY POSSIBLE
SAMPLE, AND THE EVIDENCE IS A MEASUREMENT, NOT THE ARITHMETIC.** Guarded at
`4a709158c`: two seats on **different** seeds pressed on **identical ticks**
across 600 ticks, 24 presses, in a unit test with no duel harness. Rungs 1–8
pass, so the seat-symmetry fix works everywhere it can be reached.

**The arithmetic is the explanation for that measurement, not its evidence.**
`decision.rs:472` is `(|sample| * execution_noise * interval()).round()`;
`execution_noise = 0.45 - t*0.35` with `t = (level-1)/8`; `interval()` is 5; and
`|sample|` REACHES exactly 1.0. In f32 the rung-9 ceiling is `0.4999999701976776`
— under the tie by 3e-8 — so `round()` returns 0 for every sample including the
maximum.

| rung | noise | ceiling | max jitter | P(jitter>0) | L3 rollouts |
|---:|---|---:|---:|---:|---|
| 3 | 0.36250 | 1.8125 | 2 | 0.72 | off |
| 5 | 0.27500 | 1.3750 | 1 | 0.64 | off |
| 6 | 0.23125 | 1.1562 | 1 | 0.57 | on |
| 8 | 0.14375 | 0.7187 | 1 | 0.30 | on |
| 9 | 0.10000 | **0.49999997** | **0** | **0.00** | on |

⛔ §1.3 says level 9 is *"small numbers, never zero — a frame-perfect CPU is not a
hard opponent, it is a different game"*. For this term it is zero, and the top
rung presses exactly on its decision ticks forever.

⛔⛔ **AND `decision.rs:471` IS THE STREAM'S ONLY CONSUMER IN THE TREE**, so at
rung 9 the per-seat cognition seed has no observable effect at all. ⇒
**`two_participants_of_one_character_do_not_share_a_stream` guards a fix that
cannot reach the shipped rung.** Measured: `medic` and `special_patent_clerk`
have distinct seeds (`0x1da79d34…`/`0x1ca79b9f…`, `0xe8b8d6d8…`/`0xe7b8d543…`)
and their mirror duels drift 0.0000 px and 0.0022 px over 3613 ticks — the
reflection that change exists to prevent.

⚠⚠ **AND NO TEST COULD SEE IT, FOR A REASON WORTH MORE THAN THE BUG. The
determinism guard was vacuous on its own subject.** `run` hands the brain a
`BrainSnapshot::idle()` with an **empty `attack_kit`**, so `wants_attack` is never
`Some` and the jitter path is never entered. Measured before the repair:
`the_same_seed_produces_the_same_fighter` made **0 presses of 90 frames** and left
`a.noise` at exactly its initial seed — it was comparing two all-`false` vectors
and asserting equality between two seeds that had never moved. **It could not
have failed for its stated reason.** Separately, every execution-noise fixture in
that file used `execution_noise = 0.9`, a value **no authored rung produces**.

⇒ **Two distinct species, and conflating them loses the sharper one: the 0.9
fixture measured the WRONG character; the idle snapshot measured NO character and
the assertion was still true.** The transferable rule is **a test whose subject is
supplied by a fixture must assert the fixture supplied it** — `assert!(presses > 0)`
is one line, and it is the difference between a guard and a sentence. Every
"same input ⇒ same output" test has this shape and almost none count the outputs.

✔ Guarded at `359c8be69` pinning the GAP rather than the fix: rungs 1–8 must keep
a reachable jitter **and** rung 9's ceiling must stay just under the boundary, so
a ladder retuned to a genuinely small jitter reddens the second while the first
stays green. Both poisoned (interval 1 → only the first fires; interval 4 → only
the whisker fires).

⛔ **OPEN — MAINTAINER'S CALL, deliberately not written into the guard.** Whether
the hardest CPU shipping with execution noise disabled is a defect or an accepted
cost is Jon's. If it is a defect the fix is one constant, but **waking it re-tunes
every rung-9 CPU in the game**, and every measurement taken at rung 9 — including
all twenty rows of D-CPU-INERT — becomes a measurement of a different opponent.

**Acceptance:** the ruling is recorded, and the ladder cannot drift into or out of
a zero-jitter rung unnoticed.

### D-PARITY-SELF — a cross-backend parity guard compares one backend to itself

**Owner:** menu composition. Found 2026-09-10 by
`scripts/measure_floorless_equality_tests.py` while screening for a different
defect; the emptiness shape found it, but emptiness is not what is wrong with it.

⛔⛔ **`cross_backend_model_parity_inventory_and_system`
(`game/ambition_app/src/menu/grid_backend/tests.rs`) BUILDS BOTH SIDES FROM ONE
CLOSURE.**

```rust
let build = || build_inventory_pages(&owned, equipped, MenuFocus::Item(0), &settings, ...);
let cube_pages = build();
let grid_pages = build();
```

**There is no backend argument anywhere in the fixture.** It asserts
`build() == build()` — that one function is deterministic — and reports it as
cross-backend parity.

⚠ **Its doc states the claim accurately, which is what makes it convincing:**
*"CROSS-BACKEND CONTENT PARITY: the active tab's `MenuPageModel` is built from the
SAME backend-agnostic builders regardless of which backend renders it."* ⇒ **The
fixture ASSUMES that sentence by calling one builder twice.** The thing the test
claims to check is the thing it does to construct its subject.

⛔⛔ **THIS IS A WRONG-PARTY GUARD, AND ITS COST IS NOT A MISSED BUG BUT AN ACTIVE
CERTIFICATION.** If a backend ever stops calling `build_inventory_pages` and
builds its own model, this stays green forever and reports parity for a tree that
has none. **Leaving it standing costs more than deleting it would**, because
somebody trusts it.

⚠ **NOT FIXED, DELIBERATELY.** A repair has to establish what parity *means*
between the two backends and reach one of them through its own road; a fix
written without that produces a wrong-party guard with a passing test — the same
defect one layer down. **This row is the report, not the repair.**

⇒ **The check that would be real:** drive each backend through the road it
actually uses to obtain a `MenuPageModel` and compare those. If both genuinely
call the same builder the test is a tautology and should be DELETED rather than
rewritten; if they do not, it has been lying and the divergence is the finding.

✔ Screened across the whole tree: 7916 `#[test]` bodies in 1075 files, **two hits
with this shape** — and the second was READ and found SOUND (it compares against a
hard-coded 8-entry constant, so an empty answer would differ from it and fail).
Both were read before either was named. The screen's own first run scanned **0
files** and printed a clean bill of health — a `git grep` pathspec before the
pattern — so it now carries a corpus floor and a positive control pinned to
`359c8be69` by sha.

⚠ **"Two hits" is a FLOOR on this species, not a census of it.** The screen sees
this shape only when the compared bindings are plausibly-empty collections; **a
parity test comparing two scalars from one closure has the identical defect and is
invisible to it.** The real query is *"tests whose two compared sides trace to one
call site"*, and that is an AST job, not a regex one.

**Acceptance:** each compared side is obtained through the road its own party
uses, or the test is deleted as a tautology with the reason recorded.

### D-HEADLESS-DESPAWN — a headless composition can SPAWN render-synced entities but not DESPAWN them

**Owner:** composition / the headless profile. Found 2026-09-10 by
ToothbrushAmbition while asking why one fighter of twenty-one was UNMEASURABLE
in the roster sweep.

⚠ **`npc_alice` is the TRIGGER, not the subject.** Nothing about that character
is wrong; she is the only fighter that happens to reach a despawn inside a duel.
**A row titled after her sends the next reader to fix a character.**

⛔⛔ **THE MECHANISM, FROM THE BACKTRACE.** `SyncToRenderWorld`'s **remove** hook
requires `bevy_render::sync_world::PendingSyncEntity`, a render-world resource
the headless composition does not hold:

```
sync_component.rs:55   (PendingSyncEntity does not exist in the `World`)
  ← EntityWorldMut::despawn_no_free_with_caller
  ← <F as Command>::apply        (a queued `commands.entity(e).despawn()`)
  ← SingleThreadedExecutor::run   (Update)
```

⇒ **Spawning a render-synced entity headless is fine. Despawning one is fatal.**
The bout runs 2.4 seconds and five hitstop cycles first, which is why it reads as
a fighter problem.

⛔ **THE COMPOSITION FACT.** That run's own census reports **104 `ambition_render`
systems in `Update`** in a build with no render world. ⇒ The headless profile
installs the presentation half and omits the world it syncs to, **so it works
until something is cleaned up.**

⚠ **FOUR CANDIDATES DIED, AND THIS TABLE IS THE ROW'S BEST CONTENT** — it is what
stops the next person re-running them:

| candidate | killed by |
|---|---|
| she uses a lot of VFX | `carl_stargan` uses **29** `vfx_at` to her 19, and is clean |
| her effect ids are undefined | **every** fighter's are — `electric_arc`, `gear_scatter`, `evidence_ping`, all zero |
| a 7.8MP sheet decoded mid-gameplay | `medic` loads **7.5MP** mid-gameplay, same log line, clean |
| a death/knockout despawns something | `medic` at rung 3 scores **5 knockouts** without failing |

⭐ **Each was killed by ONE non-accused subject**, which is the method worth
copying: a property measured only on the accused is distinguishing by
construction of the search.

⛔⛔ **THE CONSEQUENCE THAT MAKES THIS URGENT RATHER THAN CURIOUS.** ⇒ **Any
change that makes another fighter despawn a render-synced entity turns their row
UNMEASURABLE too, and nothing would say why.** The sweep would report a narrower
population and read as healthy.

⛔⛤ **AND THE STAKE IS NOT "A MACHINE WITHOUT A GPU" — THAT FRAMING WAS MINE AND
IT IS WRONG.** ⇒ **The subject is the `NoWindow` / `backends: None` PROFILE
specifically, which is a CONFIGURATION CHOICE, not a property of the host.**
MEASURED on an agent machine with no discrete GPU: `vulkaninfo` reports a working
software adapter (`llvmpipe`, LLVM 20.1.2), and the repo already ships
`VisibleRenderMode::OffscreenGpu`, documented as needing an adapter *"software or
otherwise"*. Sampled in one process (`bc935b1d3`):

```
[offscreen] render_app=true    render_device_in_main_world=false
[offscreen] control no_window_render_app=false
```

⭐ **So the landed gate is SELF-ADJUSTING and costs no coverage: it keys on
`RenderApp` presence, which IS the adapter question one step downstream.** An
`OffscreenGpu` run installs the view-cone rig and exercises it normally. ⚠ **An
earlier version of this row claimed the fix made the subsystem invisible to every
agent — "a crash converted into permanent silence".** Two agents built that
argument, neither checked, and **it is retracted: the observability was never
lost.**

⚠ `render_device_in_main_world=false` is real and not hidden: `build_visible_app`
runs `finish`/`cleanup` only on the `NoWindow` arm, so the offscreen app's
`RenderDevice` had not reached the main world when sampled. **The render app
COMPOSES; whether an offscreen run DRAWS is unmeasured.** Do not read "headless
agents can produce pixels" off this.

⚠ **THE STAKE IS STILL JON'S FRESH-CLONE ASK, narrowed:** a composition that
installs the render crate's Update systems with `backends: None` runs a game
right up until a cleanup.

⚠ **NOT DECIDED.** Whether the fix is to install the resource in the headless
profile, keep those systems out of it, or make the hook tolerate a missing world
is a maintainer's call. **This row is the report.**

**Acceptance:** a headless composition either does not install render-sync hooks
or can serve them, and a fighter that despawns a render-synced entity completes a
duel.

### D-VFX-ID-ADMISSION — REFUTED before it was worked; the search was keyed on the wrong spelling

**Owner:** nobody — the premise is false. Kept because the SEARCH ARTIFACT is
worth more than the row was, and because a row that refutes itself is cheaper
than one that quietly disappears.

⛔ **THE CLAIM WAS:** every fighter moveset names VFX effect ids —
`four_point_glint`, `rune_burst`, `rune_circle`, `magic_seal_break`,
`phase_ripple`, `pickup_twinkle`, `electric_arc`, `gear_scatter`,
`evidence_ping` — and **grepping each outside the moveset files returns zero
definitions**, in `.rs` and `.ron` alike. ⇒ Read as an authored key family with
no admission check, A11's shape one layer over, with the worst case being that
*every `vfx_at` call in the game is authored, reviewed and reaches nothing.*

⛔⛔ **MEASURED 2026-09-10 AND IT IS FALSE. NINE FOR NINE, IN THE SHIPPED
ASSETS:**

| id | `assets/audio/sfx.bank.txt` | `assets/sprites_0_25x/` |
|---|---|---|
| all nine above | **1** | **1** |

⭐ **THE CAUSE IS A NAME SPLIT, WHICH IS WHY THE GREP WAS HONEST AND WRONG.** The
CONSUMER spells the bare row (`rune_burst`); the OWNER spells it **compositely** —
`vfx.generic_exotic.rune_burst` in the packed bank, and as a row inside a
`generic_*_fx` spritesheet manifest. ⇒ **A census keyed on the consumer's
spelling cannot see the owner's registrations**, and the tell was in `vfx_at`'s
own doc the whole time: *"the bank ships one `vfx.<family>.<row>` cue per
authored row, so the name that finds the clip finds the sound."*

⚠⚠ **AND THE WIDENING THAT LOOKED LIKE CORROBORATION IS THE LESSON.** The first
version blamed one character's six ids; correcting it to *"every fighter's ids
are undefined"* was the right widening of the POPULATION and **left the
instrument defect untouched.** ⇒ **Widening a population does not fix an
instrument looking in the wrong place — it makes the wrong answer bigger and more
convincing.** The wider result felt like confirmation and was the same error at
scale.

⚠ **WHAT SURVIVES, and it is much smaller than the row it replaces.**
`vfx_cued`'s doc: *"an id neither the registry nor the packed bank authorizes is
counted and dropped, not heard — so a typo here is silence."* ⇒ **A mechanism
exists and it OBSERVES rather than REFUSES.** Whether "counted and dropped" is
the right policy for authored content, against A11's *refused at admission*, is a
real question and a minor one. **It is not "the flourish layer reaches nothing",
and nobody should spend a day on it.**

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
