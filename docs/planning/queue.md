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

### A7 - DONE 2026-09-10; the census and the occurrence seal both landed.

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

### D-CPU-INERT — two CPUs of a shipped fighter deal ZERO damage in a minute

**Owner:** [fighter brain](engine/fighter-brain.md), F6 and the utility
progression. Found 2026-09-10 while sampling D-BRAIN-MENU across fighters; it is
not a consequence of that row's held change — every number below is HEAD.

⛔⛔ **THE ACCEPTANCE TEST PASSES FOR ONE FIGHTER OUT OF THREE SAMPLED.** Same
harness, same rung (9, the top authored rung), same 3613 ticks, same mirror
matchup — the FIGHTER is the only variable:

| fighter | damage/min | move starts | running | hitstun | KOs |
|---|---|---|---|---|---|
| `npc_pirate_admiral` | 1.26 / 1.07 | 54 / 27 | 14% / 19% | [525, 324] | 2 |
| `npc_emmy_noether` | **0.28 / 0.44** | 13 / 12 | 2% / 2% | [38, 59] | **0** |
| `npc_carl_stargan` | **0.00 / 0.00** | **3 / 3** | **0% / 0%** | **[0, 0]** | **0** |

⇒ **Carl's two CPUs stand on a stage for a full minute at the hardest difficulty
and land NOTHING.** Three move starts each — a dash attack, `pale_blue_dot` and
a forward smash — and zero hitstun on either side. Both seats are identical
because the fight never diverges: nothing happens to diverge it.

⚠ **THIS IS NOT A MENU-BREADTH PROBLEM, which is what F6's framing would
predict.** Emmy starts 7–8 DISTINCT moves out of her 13, and Carl 3 out of 3. The
variety is there; the ACTIVITY is not. A brain that picked badly would still
press. These barely press at all.

⚠ **And the acceptance test cannot see it**, because `FIGHTER` is a const set to
the one fighter that passes. `two_cpus_in_the_shipped_composition_damage_each_other`
asserts `>= 0.5` of pool per minute and would fail on two of the three sampled —
so the guard is sound and its POPULATION is one. ⇒ Widening it to the roster is
the first concrete step, and it will go red immediately; that is the point.

**Acceptance:** the duel gate is asserted over a representative set of shipped
fighters rather than one, and every fighter in that set fights. ⚠ Until then, no
CPU-quality number quoted from this harness travels without naming its fighter.

⚠ n=1 run per fighter and all three are MIRROR matches. The contrast is
controlled (one variable) but the absolute figures are single samples; re-measure
before tuning anything.

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
