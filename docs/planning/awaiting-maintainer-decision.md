# Awaiting a maintainer decision

Only questions whose next step is **Jon's product/authoring judgement** belong
here. Engineering work goes to [`queue.md`](queue.md) or [`tracks.md`](tracks.md).
Answered rulings belong in [`maintainer-decisions.md`](maintainer-decisions.md);
the investigation that led to an answered question remains available in git
history.

This file intentionally does not retain answered decision transcripts.

> ✔ **PREMISES RE-MEASURED 2026-09-03. Every question here still rests on a fact
> that is still true**, which is the thing that decides whether answering them is
> a good use of your time. A decision resting on a premise the code has moved past
> costs you the answer AND the discovery that it was moot.
>
> Checked against HEAD: the windbox vocabulary is still authored by nothing (39);
> the puppy slug, stochastic parrot and burning flying shark still carry no
> `standing_height` (36); `REACH_TOLERANCE` is still one global `2.0` at
> `ambition_characters/src/brain/fighter/options.rs:183` (35);
> `BodyMelee::ranged_cooldown` is still the implemented half with presentation
> unresolved (33); the `"gauntlet_fireball"` visual still reproduces the old
> sprite rather than the catalog energy ball (42). ⇒ **And none of the fifteen has
> quietly been answered in [`maintainer-decisions.md`](maintainer-decisions.md)**,
> which this file's own rule would make a defect.
>
> ⚠ Two entries gained a fact worth having before you answer: 36 needs FOUR rows
> rather than three (the shark has a separate hall variant), and 39's answer costs
> one authored field on one move rather than an implementation.
>
> ✔ **AND THE QUOTED NUMBERS WERE RE-RUN, not just the premises.** Several entries
> name the script that produced their figure, which makes checking them one
> command each:
>
> | instrument | entry's figure | re-run 2026-09-03 |
> |---|---|---|
> | `measure_character_data_coverage.py` | 23 of 149 | **23 of 149** ✔ |
> | `measure_fx_row_reachability.py` | 34 of 35 rows unnamed | **34 of 35** ✔ (`george_booul_vfx` 20/21, `pirate_admiral_vfx` 14/14) |
> | `measure_orphan_shipped_pages.py` | 487 files, 14.2 MB | **438 files, 13.4 MB** — drifted, corrected in place |
>
> ⇒ Two of three exact after a day, one drifted by 49 files. **The entries that
> name their instrument are the ones that could be checked at all**, which is the
> argument for naming it every time a number goes into this file.

⛔ **NUMBERING: TAKE ONE ABOVE THE HIGHEST NUMBER PRESENT, and check first.**
The entries are NOT in numeric order — the top block runs newest-first and an
older ascending block follows it — so the number nearest the insertion point is
not the highest. Two sessions have now collided by assuming it was
(`cce4f764b` "Renumber my decision to 30: I collided with an existing 28", and
again on 2026-09-03). ⚠ And because answered entries are DELETED rather than
archived, gaps are normal and a missing number is not a free one: re-using it
silently re-points every page that cited the original.

    grep -oE '^### [0-9]+\.' awaiting-maintainer-decision.md | tr -d '#. ' | sort -n | tail -1

## Open decisions

### 50. May a fighter leave the frame by 16 units for one body-frame, or must the camera always contain the cast?

`the_stage_kills::every_live_fighter_stays_inside_the_frame`
(`game/ambition_demo_smash_app/tests/the_stage_kills.rs:1094`) is RED under the
gate's feature union, and it is the only survivor of the smash target — thirty-odd
failures went to one. Its message:

> *"a live fighter was drawn OUTSIDE the frame on 1 body-frames, worst 16 units
> past the edge — the knockout that decides the match happens off-screen:
> t3 seat 1 at (416,204) is 16 units outside a 800x450 frame centred (0,0)"*

⇒ **The engineering half is not in question.** The test measures what it says it
measures, one body-frame is genuinely outside, and 16 units on a 800-wide frame
is 4% of the half-width. What is unanswered is whether that is a defect at all.

Two readings, and they lead to different work:

- **A containment CONTRACT** — the camera must always contain every live
  fighter, so one frame outside is a bug in the camera's follow/zoom and the
  test is correctly red.
- **A tuning MARGIN** — a platform fighter's camera is allowed to lag a fast
  body briefly, the assertion's tolerance is the thing that is wrong, and the
  test should carry a stated allowance instead of zero.

⚠ **Why it is yours rather than mine**: which one is right depends on how a
knockout should READ, and the test's own text says the stake — *"the knockout
that decides the match happens off-screen"*. That is a feel judgement about the
moment the match is decided on, not an engine fact. ⛔ And the cheap fixes are
both wrong-shaped without the ruling: widening the tolerance answers it by
accident, and chasing the camera answers it by assuming.

⚠ Related but separate: entry 49 is also a `the_stage_kills` question. They are
independent — that one is about CPU divergence, this one about framing.

### 49. Is near-identical CPU play on a symmetric stage acceptable, or a defect?

A test has been deferring this to you since before its queue row was pruned, and
the deferral is the only record left of it.

`two_cpus_wearing_one_character_stop_being_a_perfect_reflection`
(`game/ambition_demo_smash_app/tests/the_stage_kills.rs:1806`) seats two CPUs on
the SAME character on a mirrored stage and asserts they diverge by more than a
pixel. It passes. Its failure text says what a pass means and hands you the
question in the same breath:

> *"⛔ this is NOT one mind played twice — the two seats draw from different
> streams, and the sibling guards listed above prove it. What it says is that a
> symmetric stage plus symmetric information leaves two different streams almost
> nothing to diverge ON at this difficulty. Whether that is acceptable is a
> product decision (queue D167); do NOT answer it by unmirroring the spawns or by
> adding noise."*

⇒ **The engineering half is settled and guarded** — the determinism is real, the
divergence is real, and the test forbids the two cheap fixes that would hide the
question. What is unanswered is whether two CPUs that play almost identically on
a symmetric stage read as a broken AI or as a fair mirror match.

⛔ **Queue row `D167` no longer exists.** It is in no live planning document — not
`queue.md`, not `tracks.md`, not this file — and survives only in the archived
pre-prune queue. So the question was never answered and never re-filed; it fell
out of the planning system and its only trace is an assertion message nobody
reads unless the test fails. Filed here 2026-09-03 to put it back where a
decision can be made.

### 47. TwinTrack's simultaneity limit: where does it live while the exhibit is parked? (re-filed 2026-09-03; was 28)

Both TwinTrack panes render ONE instant of the simulation's coordinate time, so
they can disagree about optics (light delay, aberration, Doppler) and NOT about
simultaneity — which is what the twin paradox actually is. This was question 28
here and the entry vanished — not answered and archived, just absent (yardrat,
2026-09-03: no `### 28` in this file and no row in `maintainer-decisions.md` names
it; the parked demo page `demos/twintrack.md` was the only record). Re-filed so
it has a home again. **The question:** while TwinTrack is parked, does the limit
belong in `engine/relativity.md` beside the spacetime-diagram design (which uses
"simultaneity" in a different sense and otherwise reads as though the exhibit
already shows it), or does the parked page stay the record? Default if nobody
rules: a one-line "known limit" note in `engine/relativity.md` pointing at the
demo page.

⇒ **Convention from the same finding, applied from here on:** when a question
leaves this file, its answer row in `maintainer-decisions.md` NAMES THE NUMBER
it closes ("closes 47"). Only 2 of that file's 100 rows do today, which is why a
dropped question and an answered one look the same; with the number on the
answer, a dropped one is a set difference.
### 48. The boss-crate reassessment you asked for on 2026-07-16 is now due

Your ruling that day (`maintainer-decisions.md`, 2026-07-16) was *"defer any boss
crate carve until boss behavior converges onto the canonical moveset/action
path"*, with a follow-up in the same row: *"reassess afterward whether a separate
boss crate still exists as a coherent subsystem."*

⭐ **The carve landed on 2026-08-17** — `725de8c26`, *"Carve the boss domain out
of the actor monolith into ambition_boss_encounter"* — so "afterward" arrived
seventeen days ago and nothing asked the follow-up question.

Measured at HEAD, as the input to it:

| | |
|---|---|
| size | **47 files, 14,635 lines** |
| in-tree consumers | **9** — `actor_monolith`, `runtime`, `provider`, `damage`, `sim_view`, `abilities`, `content_cli`, the `platformer2d` facade, and `ambition_content` |
| closure | in `never_asked_for`: a movement-only game links it |

⇒ **The engineering reading is that it does cohere** — a domain with nine
consumers is not a grab-bag someone forgot to delete. ⚠ But nine consumers is
also a lot of surface for one domain, and whether that breadth is the boss
domain being genuinely central or the carve having taken too much with it is a
judgement about what a boss IS, which is why this is here rather than in
`tracks.md`.

⛔ **Related and stale:** `tracks.md`'s trigger list still carries *"Boss crate
extraction — wait until boss vocabulary/ownership is coherent"* under the heading
**"Do not promote these until the trigger exists"**. The trigger fired a month
ago. That row wants deleting or rewriting as the reassessment; it is in another
agent's hot file, so it is reported rather than edited.

### 46. Does 1-1 want a fourth ?-block over floor, so the fire form's floor-refusal can be played?

`refuse_a_weaker_form_pickup` is Mary-O's rule that a form on the FLOOR may not
replace a stronger one. It now has a played acceptance test on the shipped app
(`weaker_form_refusal.rs`), but only for the EQUAL rung — a tall Mary-O meeting
a second wand. The strictly-weaker rung (fire meeting a wand) is **unreachable
in 1-1 as authored**, and the obstacle is level geometry rather than engineering.

Reaching fire spends TWO of 1-1's three `Toward(Lantern)` ?-blocks
(small→wand→tall, tall→lantern→fire), so a wand left walking needs the third.
⛔ **The third, at x=1920, stands over a pit.** Measured by dropping a body into
its column: from above she lands ON the block (centre 256, feet on its top
face); from below the face she falls to y≈969 in a 448-tall room, dies, and
respawns at the level start. No body can stand under it to bonk it.

⚠ **This is not a hole in the rule and not a missing test of it.** The equal
rung is the stronger arm for the comparison the rule actually makes — a `<`
written where `<=` belongs still refuses a wand offered to a fire Mary-O, and
lets a second wand re-equip a tall one — so the boundary is covered and the
interior is not. The question is only whether the interior is worth authoring
for.

Choose one:

- **author a fourth ?-block over floor in 1-1** — smallest change, and it also
  gives a player a place to reach fire without crossing the pit;
- **build the fixture in 1-3 instead** — it already authors a `Question` at 256
  AND an `AlwaysWand` at 1696, so the scenario exists there with no new content;
- **leave it** — the boundary arm covers the comparison, and the interior arm
  would cost a scripted pit-edge jump, which is a test about jump tuning wearing
  this rule's name.

⚠ **AND THIS QUESTION IS OLDER THAN THIS ROW.**
[`demos/super-mary-o.md`](demos/super-mary-o.md) has carried a product question
since 2026-08-14 about the fire form's DISCOVERABILITY — *"should the beacon
walk to you like the wand, should 1-1 place a reachable second block earlier, or
is 'the reward waits up there and you must climb for it' the intended feel?"* —
reached from the other direction, a player who could not find the reward. Same
level, same three blocks, and answering either answers both. The new fact this
row adds is that the third block has no standing position under it at all.

⛔ No content was authored to answer this: authored levels are Jon's.

### 37. Should the F9 rollback proof pulse survive a gameplay-session change?

`LocalSessionPolicy::check_distance` is raised by the F9 proof pulse and returns
to normal only when that pulse finishes. If the player quits to title during the
pulse and launches another game, the elevated verification distance currently
survives.

The recent session-ownership fix deliberately did **not** decide this because the
value is developer tuning rather than gameplay authority.

Choose one policy:

- **session-scoped:** a new gameplay session always starts with the ordinary
  check distance; or
- **process-scoped developer intent:** the proof pulse deliberately spans a
  relaunch until it completes/cancels.

This is primarily a developer-iteration/expectation decision. The gameplay
rollback authority itself is already session-owned by ADR 0027.

### 36. What are the authored standing heights of the puppy slug, stochastic parrot and burning flying shark?

These are the remaining characters whose old size derivation cannot be replaced
by preserving one existing placement size because their authored spawn boxes
disagree substantially across rooms.

✔ **PREMISE RE-MEASURED 2026-09-03 — still open, and here are the exact rows to
fill.** None of them carries a `standing_height` in
`game/ambition_content/assets/data/character_catalog.ron`:

| your name for it | catalog id(s) |
|---|---|
| puppy slug | `npc_puppy_slug` |
| stochastic parrot | `stochastic_parrot` |
| burning flying shark | **two** — `npc_burning_flying_shark` and `hall_npc_burning_flying_shark` |

⚠ The shark is the one to watch: it has a hall variant as a separate catalog
entry, so a single number either goes in twice or the two are deliberately
different sizes — which is itself part of the answer rather than a detail below
it.

Representative placement variation:

```text
npc_puppy_slug            (48,22), (32,48), (64,32), (48,32),
                          (64,16), (52,66), (42,42), (28,44)
stochastic_parrot         three different boxes
npc_burning_flying_shark  mostly (108,96), also (32,48)
```

The needed value is one character-authored `standing_height` in world units for
each. Do not choose by majority box size: the box was editor/layout data, not a
stature authority.

Decision 32 still applies: there is no standard adult/humanoid height and no
bulk normalization. Character stature is authored individually.

Related visual followups that should be judged by playtest rather than another
population average: the cove pirates relative to Robot v3, slop size, and the
Mary-O snake's post-rescale size.

### 35. What should own fighter reach during move startup?

The current fighter brain uses one global `REACH_TOLERANCE = 2.0`, effectively
allowing a move to remain viable out to roughly three times its authored reach.
The bug that exposed this proxy is fixed; the constant is not currently known to
cause a product defect.

The design choices are:

1. keep the proxy until platform-fighter option ranking has its own capability
   boundary;
2. add a per-move tolerance field;
3. derive reachable distance from move startup plus the body's movement
   capability, which requires threading capability/top-speed information into
   perception;
4. for moves with authored startup impulse/travel, derive that part directly
   from the move and keep a fallback for ordinary movement.

**Default if no change is desired:** leave it. Do not widen generic actor data for
a proxy that is not currently hurting play merely to close a planning row.

### 34. Should external/launch-owned motion become an explicit cross-game fact?

Three shared movement decisions have historically inferred "this velocity belongs
to a launch rather than locomotion" from speed magnitude: initial-dash settling,
shield braking and body-contact resistance. The approximation fails when a launch
has decayed below ordinary run speed.

Smash already has a live tumble mechanic and therefore a genre-specific fact that
can represent external/launch-owned motion. Ambition does not necessarily want
Smash tumble semantics for ordinary bodies.

The decision is whether to:

- keep the current thresholds until a visible defect requires more;
- introduce a generic carried/external-motion ownership fact with Smash tumble as
  one producer; or
- let the platform-fighter capability own the richer rule while the shared kernel
  keeps the simpler behavior for other games.

Do not solve this by simply reading Smash's `tumble_speed` from the generic
kernel; the question is exactly whether that game-specific semantic is shared.

### 33. How should a recharging ranged weapon communicate that it is unavailable?

The firing cadence is implemented: `BodyMelee::ranged_cooldown` follows the
weapon's authored refire interval, and an early press is refused before spending
the proposer so ordinary combat buffering can retry when the weapon becomes
ready. The unresolved part is presentation.

Choose the product channel when this becomes important in play:

- character/muzzle VFX driven by recharge fraction;
- a presentation treatment on the firing limb/body; or
- a HUD indicator.

The mechanic does not need to block architecture work while the unavailable
state is merely invisible rather than incorrect. Prefer character-local
presentation if it reads clearly; do not add another gameplay authority to show
the cooldown.

### 38. Does an actor released in a foreign room stay there?

Today an actor moved away from its authored home and then left in another room is
retired when that room unloads and is authored again at home when encountered
later. The current construction road honestly refuses to claim the actor was
persistently relocated.

Two valid product policies:

- **go home:** authored home placement is restored when the actor is no longer
  live/resident;
- **stay where left:** persist a `Placed`/relocation occurrence for actors as is
  already done for relevant item occurrences, and teach reconstruction to honor
  it.

If choosing “stay,” the producer and reconstruction consumer must land together;
recording a moved placement that construction refuses would only add warnings and
still teleport the actor home.

This decision feeds
[`engine/construction-and-reconstitution.md`](engine/construction-and-reconstitution.md)
and [`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md).

### 39. Which authored move, if any, should adopt the dormant windbox/armor vocabulary?

The windbox mechanic is implemented and can express outward gust or inward
suction. `WindowTag::Armor` also exists but has no shipped authored customer.
There is no engine defect merely because the vocabulary is currently unused.

If one should become product-visible, name a fighter/move. Otherwise leave the
mechanism dormant until a character design asks for it. Do not invent a customer
to make an adoption count nonzero.

✔ **RE-MEASURED 2026-09-03 — still exactly true, and here is what is waiting.**
The windbox vocabulary is real API across **16 files**: `WindboxVolume`
(`ambition_entity_catalog/src/lib.rs:415`), `MoveSpec::windbox`
(`ambition_combat/src/strike.rs:94`) and `is_windbox`
(`platformer2d_core/src/hit_response.rs:112`). **Zero** authored movesets under
`game/ambition_content/src/*moveset*.rs` name it. ⇒ So the question is unchanged
and the answer costs one authored field on one move — not an implementation.
⚠ Measured because "dormant" is the premise this decision rests on, and a
premise that has quietly acquired a customer would make the question moot without
anyone noticing.

### 40. Should a held gun-sword kick the player the way it kicks the pirate?

The K2 fold puts the player's held gun-sword and fireball on the ONE projectile
road, and that road applies the weapon's authored `Discharge`.
`gun_sword_discharge()` authors **380 px/s of recoil**, written for the pirate
who carries it.

The held-shot path that was deleted applied **no recoil at all** when the player
fired. That difference was a property of the second code path, not an authored
decision — nobody chose it. The fold preserved the old feel by zeroing recoil
for hand-fired held items, so today the same weapon kicks its NPC wielder and
not its player wielder.

Choose one:

- **the weapon kicks whoever fires it:** delete the zeroing; a player firing the
  gun-sword takes the authored 380 px/s. One weapon, one authored number.
- **recoil is a wielder property:** keep the zeroing, and say so in the weapon
  vocabulary rather than as a special case — an NPC braces, a player does not.
- **the player number is simply different:** author a separate player recoil
  value; name it.

⚠ This is a FEEL ruling on a shipped weapon, not an engineering question. The
engineering is done either way; the fold currently encodes "no kick for the
player" only because that is what the deleted path happened to do. The zeroing
is `fire_held_ranged_system` in `crates/ambition_held_items/src/lib.rs` (it was
`items/pickup/mod.rs` <!-- cite-ok: the pre-carve path, kept as the record -->
until the pickup carve moved the domain on 2026-09-03 — the old path still
EXISTS as the kernel's schedule residue, so that citation resolved while
pointing at code that had left); the guard is
`a_hand_fired_gun_sword_bolt_flies_the_one_projectile_road`
(`game/ambition_app/tests/hand_fired_held_shot.rs`) — retarget it with the
ruling.

✔ **ANSWERED by Jon, 2026-09-02, verbatim:** *"Weapons can have recoil and they
will kick whoever is firing it. The kick might depend on the mass property of
the actor doing the firing."*

⇒ **The first sentence settles this entry: delete the zeroing.** One weapon, one
authored number, and it kicks its wielder whoever that is. `fire_held_ranged_system`
stops special-casing hand-fired items and the guard retargets to expect recoil.

▢ **The second sentence opens a NEW question and must not be smuggled into this
one.** "Might" is a direction, not a ruling. Recording what it would have to
attach to, because the input already exists and is not vestigial:
`ActorDefinition.mass` is `Option<f32>`, read at spawn as
`definition.vitals.mass.unwrap_or(1.0)`, merged by
`ambition_body_seed/src/physical_baseline.rs` (it was
`character_runtime/physical_baseline.rs` <!-- cite-ok --> until D33 cut 2a moved
the file whole, `7ba40886e`), and rollback-registered under the
stable name `mount.mass`. So a mass-scaled recoil would read a live,
rollback-safe value rather than needing a new authored field. ⛔ What is NOT
decided: the curve (linear in mass? inverse? clamped?), whether an unauthored
mass of 1.0 means "average" or "unscaled", and whether this generalises to all
knockback or only to discharge recoil. Do not pick one by inference — a shipped
feel change to every weapon is a bigger ruling than the one asked here.

### 41. Where should a hand-fired fireball leave the body?

The deleted held-shot path spawned the fireball from a side muzzle at
`(size.x / 2 + 8, -0.12 * size.y)` — offset forward and slightly toward the
head. The projectile road's default is `Muzzle::BodyOrigin`, a few pixels away
from that point.

The difference is small and entirely cosmetic, but it is visible on a wide body
and it changes where the shot clears the player's own silhouette.

Choose one:

- **body origin:** accept the road's default and retire the old offset; one
  spawn rule for every projectile.
- **keep the authored muzzle:** register the old offset as an authored
  `Muzzle` on the fireball so the fold preserves the shipped look exactly.
- **a muzzle is a per-weapon authored fact:** every hand-fired weapon names its
  own, and the fireball's is the old offset.

The fireball's spec (`held_item_by_id("fireball")`, `action_set/mod.rs`) authors
`Muzzle::default()` today.

### 42. Should the gauntlet fireball keep its own sprite, or become the catalog's energy ball?

The deleted path drew the fireball as the 30 px `gauntlet_fireball.png` sprite.
The projectile catalog's `"fireball"` is a tinted energy ball — a different
look, shared with every other fireball in the game.

To avoid changing a shipped visual inside a refactor, the fold registers a
`"gauntlet_fireball"` Image visual that reproduces the old sprite.

Choose one:

- **keep the gauntlet's own sprite:** the registered Image visual stays, and the
  gauntlet reads as its own weapon.
- **adopt the catalog energy ball:** delete the registration; one fireball look
  across the game, and the gauntlet loses its distinct art.
- **the sprite is right but the catalog should own it:** promote
  `gauntlet_fireball` into the shared catalog as a first-class projectile visual
  other weapons may also use.

⭐ **THE SIZE/ANCHOR COMPARISON IS DONE, IN CLOSED FORM, 2026-09-02 — and it
found the difference somewhere else.** A capture was queued for this; reading the
two paths answers it more exactly than a screenshot could.

- **SIZE: exact parity, at every quality tier.** The deleted renderer drew
  `Vec2::splat(30.0)`. The new registration is
  `ProjectileRenderSize::FixedWidth(30.0)`, which resolves as
  `Vec2::new(w, w / frame_aspect)`. `gauntlet_fireball.png` ships at **64 x 64**
  (base), 32 x 32, 16 x 16 and 8 x 8 (`sprites_potato`) — **all square**, so the
  aspect is 1.0 and the height is 30.0 whichever tier the quality budget loads.
  Identical, not merely close.
  ⚠ An earlier revision of this note said the sprite was 16 x 16. That was the
  `sprites_0_25x` copy — one of four, read without checking the others. The
  conclusion was right and is now stronger, but the number was a sample quoted as
  the population.
- **ANCHOR: exact parity.** The old sprite took `..default()` (centre); the new
  Image path passes `anchor: None`, which is the same centre.
- ⛔ **DEPTH CHANGED, and nobody asked about depth.** The old fireball drew at
  `z = 9.5` — below `WORLD_Z_DUMMY` (10.0) and well below `WORLD_Z_PLAYER`
  (20.0), so it passed BEHIND the player and behind enemies. The projectile road
  draws every shot at `projectile_z()` = `WORLD_Z_PLAYER + 2.0` = **22.0**, in
  front of the player. `world_to_bevy` passes `z` straight through, so these are
  the same scale and directly comparable.

⇒ The fold moved the fireball from behind the cast to in front of it. That reads
like a repair rather than a regression — a thrown fireball vanishing behind a
body is hard to defend — but it IS a visible change, it was not authored as part
of the fold, and it is not what decision 42 asks about. Say whether the new
layering is what you want; if it is, nothing to do.

⚠ **What a capture would still add, and only this:** that the asset RESOLVES at
runtime rather than drawing the magenta placeholder. The geometry above needs no
picture.

The registration is `GAUNTLET_FIREBALL_VISUAL` in
`game/ambition_content/src/projectiles.rs`.

All three (40–42) were opened by the K2 fold on 2026-09-02; the fold itself is
landed and none of them blocks it.

### 43. Does a body hanging on a ledge inside a hazard volume die?

Spikes under a ledge lip is an authored shape, and a hanging body's box can
overlap one. Today it does not die: an active ledge grab consumes the simulation
frame before the hazard/OOB gate runs, so the gate never judges it.

That was not authored — it is a consequence of where the gate sits. It surfaced
2026-09-02 when the gate moved to fix an ordering bug and would have started
judging three populations it never had; the move was constrained back to its
original population rather than deciding this by accident. See
[`engine/collision-and-ccd.md`](engine/collision-and-ccd.md) §1.

- **stays immune:** hanging is a committed state, and a body that cannot act
  cannot be asked to escape; or
- **dies:** the hazard is a volume and being inside it is being inside it,
  whatever the body is doing.

⚠ Recorded, not recommended. Whichever way it goes it is a FEEL ruling and wants
authoring deliberately, not acquiring from a refactor.

✔ **ANSWERED by Jon, 2026-09-02, verbatim:** *"They get hit by the spikes (as
long as they are not immune - e.g. they might have iframes from a ledge grab).
Spikes may or may not insta-kill, they could just do damage."*

⇒ **"Dies" was the wrong framing of the second option and the ruling corrects
it.** The body is HIT; whether that kills it is the hazard's authored damage,
not a property of hanging. Both halves are already expressible and need no new
vocabulary: `HazardSpec` carries `damage: i32`, `knockback`, `kind`, `team`,
`hitstop_seconds` and `respawn`, so a spike that merely hurts is authored, not
built.

⛔ **AND THE EXEMPTION MOVES.** It is no longer "hanging is a committed state" —
a hanging body is judged like any other, and if it survives that is because it
is IMMUNE, through the ordinary invulnerability road. Jon names a ledge grab
granting iframes as an example, not as a fact about today's code.

▢ **So what remains is one question this entry did not ask:** does a ledge grab
grant iframes, and if so for how long? ⭐ It has a home already — invulnerability
is a REASON SET, not a flag (`features/empowerment.rs` delegates
`Empowerment::UNTOUCHABLE` to the body's invulnerability-reason set, beside
`Invulnerability::EMPOWERED`), so "hanging on a ledge" would be another reason
rather than a new mechanism. That is a separate authoring decision
from the gate ordering, and it is the one that decides whether the visible
behaviour actually changes. Until it is answered, moving the gate makes hanging
bodies take spike damage — which is now the intended behaviour, so the
constraint that held the gate back is lifted.

### Two fighters' bespoke effect art is never requested (2026-09-02)

`npc_pirate_admiral` has a 14-row effect sheet and `smash_george_booul` a 21-row
one, and **nothing in the repository names 34 of those 35 rows** — measured by
`scripts/measure_fx_row_reachability.py`, which asks which fx row names any
tracked `.rs`/`.ron`/`.yarn`/`.json` mentions (an effect is drawn by name, so a
row nothing names cannot be requested).

⛔ IT IS NOT DEAD ART, which is why this is a question and not a slice. The rows
were drawn FOR those kits: `grapeshot_cloud`, `heave_to_anchor`,
`heave_to_brake`, `cutlass_wake`, `boarding_wake`, `captains_mark` sit beside an
admiral moveset whose moves are named `grapeshot`, `heave_to`, `gun_sword` — and
that moveset asks for `muzzle_flash` and `air_slice` from the GENERIC sheets
instead. George is the same: `bivalence_weak`/`bivalence_strong`,
`excluded_middle_windup/launch/ascent/gate/tail`, `modus_ponens_*`, `reductio_*`,
beside moves called `bivalence`, `excluded_middle`, `commitment`.

✔ **RE-MEASURED 2026-09-02 evening and the claim reproduces exactly**:
`pirate_admiral_vfx` 14 rows / 0 named, `george_booul_vfx` 21 / 1 named — 34 of
35, unchanged. ⓘ Wider context the row did not have: across all 13 fx sheets it
is **196 rows, 120 named, 76 named by nothing**, and `pirate_admiral_vfx` is the
only sheet with NO row named by anything. So the two named here are the extreme
of a spread, not isolated cases — which strengthens "wire them" and weakens
"they were superseded", because eleven other sheets are partly wired.

⇒ **Wire them, or were they superseded?** The row→move pairing is unambiguous
from the names, but WHEN in a timeline each fires and at what scale is a feel
ruling, which is why it is here rather than done. Residency's interest was the
size (9.4 MP resident in every room); since 2026-09-02 the two sheets are never
decoded, because no realized character's moveset names a row of them.

### The LDtk editor-preview tileset is 7.6 MP the runtime never draws

**FIVE** world files declare `sprite_player_robot_v3 = ../sprites/player_robot_v3_
spritesheet.png` so the editor can draw entity previews, and `bevy_ecs_ldtk`
decodes every tileset of a project when the project loads — on every boot and
every world load, at FULL tier, beside whatever tier the game actually realizes.

⭐ **MEASURED 2026-09-02 (`scripts/measure_ldtk_tileset_usage.py`), and the two
things that were assumed are now checked.** NO LEVEL LAYER in any of the five
uses the tileset (`layerInstances[].__tilesetDefUid`, across 1/11/60/1/1
levels); its only consumer is one entity definition per world, `PlayerStart`,
cropping the top-left tile. So "editor previews only" is measured — which
matters, because a layer using it would have made a cheaper tier a QUALITY
decision instead of a free one. And it is five worlds, not four: the four
`ambition_content` ones plus `ambition_demo_sanic/worlds/sanic_speedway.ldtk`.
`mary_o` does not reference the sheet.

⛔⛔ **AND IT IS NOT "one line per world".** `tileRect`, `uiTileRect`,
`tileGridSize`, `pxWid` and `pxHei` are all TILESET PIXEL coordinates. Change
only the `relPath` and the 256×256 crop that framed one animation frame spans a
third of an 832-pixel image — the preview breaks while the JSON still looks
plausible, and nothing in the game reports it. The tiers are also not exact
fractions and the x and y factors differ (`sprites_0_25x` of 3072×2468 is
832×653, not 768×617), so there is no constant to scale by.

⇒ **A prepared patch is waiting: `dev/patches/ldtk-player-tileset-retarget-20260902.patch`**
(`patch -p1 <` or `git apply`, both verified; regenerate with
`scripts/propose_ldtk_tileset_retarget.py --tier <tier>`). It recomputes every
pixel field from the real PNG header and preserves each crop as a FRACTION of
the image. Boot decode for this tileset goes 7.6 MP → ~0.54 MP.

ⓘ It also fixes two stale declarations found by reading the real header: the
four content worlds declare `pxHei 2484` against a 2468-pixel file, and
`sanic_speedway` declares `1681×1728, tileGridSize 224` for that same file. ⚠ The
patch preserves sanic's framing as a fraction, i.e. whatever that stale
declaration was already showing.

⚠ **IF THE SUITE TELLS YOU THIS PATCH NO LONGER APPLIES, CHECK THE INDEX BEFORE
BELIEVING IT.** The targets live in the `game/ambition_map_assets` SUBMODULE, and
on 2026-09-03 `test_a_patch_against_this_tree_still_applies` failed on it —
"either the tree moved under it or the patch was never test-applied" — while
`git apply --cached --check` inside that submodule accepted it cleanly. Four
`.ldtk` worlds were simply dirty from another session's work in progress. Acting
on the failure as written would have withdrawn a correct patch, or regenerated
it on top of somebody's unfinished edits. The checker now separates the two
causes and SKIPS with the dirty files named, so this should not recur; the note
stays because the wrong reading was one sentence away from a bad decision.

⛔ **Jon's submodule, so it waits** — applying it needs a commit in
`game/ambition_map_assets` plus a pointer bump. ▢ And it is untested against the
LDtk editor itself, which needs Jon opening a world.

### How tall is a puppy slug? The base is unauthored, its variants are not (2026-09-02)

⭐ Found while re-measuring the pirate-sizing report's *"zero rows author
`standing_height`"* claim, which is now **23 of 149**
(`scripts/measure_character_data_coverage.py`). Two of the twenty-three are
puppy slugs, and the third member of the family is not:

```text
npc_puppy_slug            (falls to the 48.0 default — nobody chose it)
npc_puppy_slug_variant2   41.7
npc_puppy_slug_velvet     30.9
```

⛔ **So the BASE crawler stands taller than both of its own variants, at exactly
the player robot's height** — which is the symptom Jon reported for the pirates
(*"as tall as the player robot who is supposed to be chibi"*). `npc_puppy_slug`
and `npc_puppy_slug_velvet` publish the SAME 128 px frame, and land at 48.0 and
30.9: a 55% difference between two characters drawn at one size, where one side
is a decision and the other is a default.

⭐ **AND IT IS THE ONLY FAMILY IN THE CATALOG LIKE THIS.** Swept every
base/variant pair (one row name a strict `_`-prefix of another) across all 149
rows: `npc_puppy_slug` is the *sole* case where one side authors a height and
the other does not. Every other family is all-authored or all-default. ⇒ That
uniqueness is what makes it read as an omission rather than a convention — if
leaving the base to `body_kind` were the house style, it would be visible
somewhere else.

⚠ **It is still not obviously a bug** — a base breed may be meant to be larger —
which is why this is a question and not a fix. What is odd is the SHAPE: the two
variants were deliberately sized and the thing they are variants OF was left to
`body_kind`. ⇒ **What height should `npc_puppy_slug` be?** It is placed in
`hall_of_characters`, `intro` and five levels of `sandbox`, so whatever it is
now is visible in play.

### The shared sprite pack is 442.6 MB and one prop reads it (raised 2026-09-02)

⭐ **MEASURED** (`scripts/measure_pack_reachability.py`).
`build_prop_sprite_asset_packed` is the ultrapack's ONLY production consumer; it
has ONE call site, the intro prop loop; and it runs only for
`intro_prop_sprite_rows()` entries whose 4th tuple element is `Some(target)`.
**Exactly one row is: `intro_cart`.** Characters have no pack road at all —
`load_character_sprites_in` takes the per-target `*_spritesheet.ron` every time.
All four tiers pack the same 197 targets. ⚠ **Re-run on calculex 2026-09-03 the
script reports 164 targets, and the pack directory measures 318 MB rather than
442.6.** The load-bearing claim is unchanged and verified — *"1 target(s) opt
into the pack — intro_cart"* — but BOTH size figures are generated-artifact
numbers, and `measure_orphan_shipped_pages.py` says of its own kind: *"these are
gitignored generated files, and this is ONE machine's tree."* ⇒ So treat 197/442.6
and 164/318 as two machines' trees rather than a change over time; the argument
this entry makes does not turn on which. On one machine: **442.6 MB of pack
pages, 5.2 MB on a page any consumer can reach — 98.8% unreachable.**

⚠ **NOT A DEFECT REPORT.** Packing every target is what a packer should do; the
finding is that adoption never followed. Reachability is a SOURCE fact and reads
the same on any checkout; the megabytes are generated, gitignored, per-machine.

⇒ **Three answers are all reasonable and none is an agent's call:** adopt the
pack for characters (it was built for that, and `project_ultrapack` design intent
says the two roads should converge); narrow the generator to pack only what a
consumer opts into; or leave it, on the grounds that a packer that packs
everything is correct and the cost is disk nobody is paying attention to. ⛔ What
is NOT reasonable is dropping the per-target PNGs to "save" the duplication —
they are every character's only road.

### Portraits are generated at four tiers and only full resolution is readable (raised 2026-09-02)

`bake_portrait_manifests` collects portrait manifests from `assets/sprites` ONLY
and says why: *"Portraits are presentation products and currently have no
quality-tier variants"*. The generator emits the PNGs at all four tiers anyway —
**438 files, 13.4 MB, with no road**
(`scripts/measure_orphan_shipped_pages.py`, re-run 2026-09-03; its
`REDUCED-TIER PORTRAITS` section). ⚠ This entry read **487 files, 14.2 MB** when
raised on 2026-09-02 — the figure drifted by 49 files in a day, which is what a
generated population does. ⇒ The decision is unaffected; the drift is only worth
noting because the entry quotes a size to argue the cost is worth acting on, and
that size is a moving number with a one-command instrument beside it.

ⓘ The missing `.ron`s are POLICY, not a bug —
`check_quality_variants_are_fresh.py` records that portraits are *"published
SELECTIVELY"*. ⛔ But the 9 that ARE published per reduced tier cannot be read
either: `PortraitSheetRegistry` is built `from_baked_table(BAKED_PORTRAIT_RONS)`
and `build.rs` bakes from `assets/sprites`. A deliberate selective publication
produces files no build can load.

⭐⭐ **AND THE MEASUREMENT NARROWS THE ANSWER — 2026-09-02,
`scripts/measure_portrait_tier_headroom.py`.** Portrait draw size is chosen by
VIEWPORT, never by quality tier: `DialogLayoutProfile::for_viewport` picks
**56×62** (phone landscape), **82×94** (phone portrait / small tablet) or
**104×120** (everything else), consulting no quality setting. So no quality tier
can select a portrait resolution — the window size does. Against `alice`:

```text
tier              frame     @1x display          @2x display
sprites          256x320    covers every box     covers every box
sprites_0_5x     128x160    covers every box     smallest box only
sprites_0_25x      64x80    smallest box only    UPSCALES ALWAYS
sprites_potato     16x20    UPSCALES ALWAYS      UPSCALES ALWAYS
```

⇒ **Nothing wants a Potato portrait**: at 16×20 it is under even the 56×62 box
at 1×. `sprites_0_25x` is defensible only on a phone-landscape viewport at 1×
display scale — the least likely combination, since phones are high-DPI. Only
`sprites_0_5x` has a real case, and only at 1×. ⚠ A tier under the box it is
drawn into is not a cheaper portrait; it is a blurrier one, which is the
failure Jon's standing rule forbids.

⭐ **THE ANSWER IS ALREADY WRITTEN — `dev/patches/portrait-tiers-are-never-baked-20260902.patch`**
(`git apply` it from the repo root). It stops
`scripts/generate_visual_quality_variants.py` copying `*_portraits.png` into the
three reduced tiers, which is 487 files / 14.2 MB that nothing can load, since
`bake_portrait_manifests` collects from `assets/sprites` only. It is a patch and
not a commit because it changes generated assets on the next unfiltered regen —
that is Jon's call, not mine. ⓘ It touches a MAIN-REPO script, so applying it
needs no submodule commit and no pointer bump.

⚠ Filed as a pointer 2026-09-03 because the patch existed for a day with NOTHING
in the repository naming it — the decision row asked the question and the answer
sat in a directory nobody had reason to open. The other three patches in
`dev/patches/` are each named by a doc; this one was not.

ⓘ Residency is already bounded independently: `RetainedHudImages` holds one
entry per portrait ACTUALLY SHOWN (~1.3–2.0 MP each), not the 163 baked
manifests — so the tiers would save package size, not runtime memory.

⭐ **AND THEY ARE STILL BEING PRODUCED, not left over.** Age signal
(`measure_orphan_shipped_pages.py`): 439 of 475 comparable portrait files were
written in the same run as their full-resolution twin or after it, median +3.07
days — against the stranded sheet pages, where 44 of 44 predate their manifest.
⇒ **A clean regen on another machine will reproduce these**, so the "wait for
yardrat" answer that covers the stranded pages does not cover this row.

⇒ **Stop generating them, start baking them, or leave them?** The measurement
says at most one tier (`0_5x`) could ever be wanted and two certainly cannot.
The comment says portraits have no tier variants *currently*, which reads as
intent that may change — and that is the part an agent cannot know.

### 44. Should `SmashChargeSpec` keep a game-mode name for a general mechanism?

Jon raised the general shape of this on 2026-08-28, about a different type
(*"it might be a good idea to rename the actor given its conflation with a very
core concept in the architecture. But we can do that in a different pass"*). The
`Actor` half was done — it is `Performer` now. This half was never put to him and
should not be decided by an agent, because a rename is Jon's vocabulary call.

⭐ THE CONFLATION IS MEASURED, not assumed. `SmashChargeSpec` is named for one
game mode and its own doc comment describes something general: *"How a chargeable
move HOLDS: where on its own timeline the charge waits, and how long it may wait
before it fires itself."* It carries `roots`, `sustain`
(`WhileHeld` / `UntilPressedAgain`) and two seconds-valued clocks in the owner's
proper time — none of which is Smash-specific — and the Trap (the Performer's
down-B, an Ambition move, not a Smash one) uses it for a three-second
subterranean beat, which is what made the name visible.

⚠ THE SIZE, so the answer can be costed: **36 references** across `crates/` and
`game/`. A rename is mechanical and touches authored content, so it wants to
happen in one pass or not at all.

⇒ Three answers are all reasonable and the choice is not an agent's: keep the
name (the mechanism was authored for Smash and the association is useful), rename
to something like `TimelineHoldSpec` / `ChargeHoldSpec`, or keep it and let a
future Smash-specific type take the name back. ⛔ No engineering is blocked
either way — this is recorded so it stops being asked and forgotten.

### Interact dialogue for the characters the Hall's authoring did not cover (raised 2026-09-02)

`triage/character-dialogue-from-suggestions.md` re-measured: 149 catalog rows,
124 with a `hall_dialogue_id`, 131 authored `hall_*` Yarn nodes. The Hall was
solved by hand-authoring, the escape the 2026-07-26 decision left open, so a
generator built to that decision would generate over 124 characters that
already have nodes. What remains is ~25 rows with no hall id, and every room
that is not the Hall, where a character with a real `fallback_dialogue` voice
still opens `generic_npc` on interact.

Needed: keep authoring by hand (then the triage closes as superseded), or
build the generator for the remainder only (a per-character node synthesized
from `fallback_dialogue`, overridden by any authored node of the same title).
Content call; the engine side is unchanged either way.

### 45. Is a unique capability item an ENTITLEMENT or an OCCURRENCE? (2026-09-02)

`item-custody-and-accounting.md` I3 asks special pickup roads to "converge
toward the same occurrence/custody model as ordinary held items **when that
model can express their semantics**". Measured, it cannot express the portal
gun's, and the difference is a product decision rather than an engineering one.

An ordinary held item is an OCCURRENCE: a `GroundItem` with a `SimId` that
persists through pickup and drop, its location remembered by custody and the
whereabouts ledger. The portal gun is an ENTITLEMENT: picking it up despawns the
world token, grants `PortalGun` on the body and `Item::PortalGun` in
`OwnedItems`, and **dropping never revokes the grant** — it unequips and spawns
a fresh, room-scoped token. The menu re-equips straight from `OwnedItems`. The
code states the intent where it is decided: *"The gun is a single item: it
doesn't exist until you pick it up — picking up the one world item IS getting
the portal gun."*

✔ Nothing is broken: `OwnedItems::grant` clamps a unique item to 1, so the two
roads cannot inflate a count, and the measured behaviour is self-consistent.

⇒ **The question is what you want a unique capability item to MEAN**, and the
two readings differ observably in exactly one place: **can dropping the portal
gun and walking away ever lose it?**

- **Entitlement** (what ships today): no. Once acquired it is yours; the world
  token is a convenience for re-equipping in place. Zelda's hookshot.
- **Occurrence**: yes. The gun is a thing that exists somewhere, can be left in
  a room, stolen, or lost down a pit, and the durable record is where it IS.

⛔ Not an agent's call — it decides whether a whole category of future item is
losable. Recorded rather than implemented; I3 stays open behind it.

## Waiting on maintainer measurement, not a decision

### The residency limit open work 4 needs

A budget policy that keeps the last room's cast resident needs a ceiling, and the
ceiling is a host number: `resident_mb` at Full on the 3090 after a
hub→hall→hub walk. Everything else in that section is measured; this is the one
input that cannot be taken on a software rasteriser.

> ⭐ **THE NUMBER, MEASURED ON CALCULEX 2026-09-03 — no GPU, and it was already
> being produced every gate run.** At the hall entry, through the real authored
> door, at FULL texture resolution:
>
> | run | images | megapixels | **resident** |
> |---|---|---|---|
> | `hall_transition_cover` (control) | 224 | 363.1 | **1452.3 MB** |
> | same, `AMBITION_QUALITY_PROFILE=ultra` | 225 | 363.9 | **1455.8 MB** |
> | `leaving_the_gallery…` — the RETURN leg, hub→hall→hub, at 5.0s | 236 | 503.0 | **2012.0 MB** |
> | same run at 10.0s | 244 | 507.7 | **2030.9 MB** |
>
> `gpu +0 … awaiting gpu N` on every one — nothing uploaded, which is the point:
> `resident_mb` is decoded CPU-side bytes and needs no adapter.
>
> ⓘ **Why these are the FULL-tier figures, stated rather than assumed.** The test
> composition does not seed from the adapter, so it takes
> `default_visual_quality_profile()` = `High` on non-Android, and
> `VisualQualityBudget::for_profile` maps `High` to
> `TextureResolutionScale::Full` — the unsuffixed sprite tree. The hall obeys the
> same tier by Jon's 2026-09-02 ruling, pinned in
> `the_halls_cast_is_realized_at_the_users_tier_never_lower`: *"the hall draws at
> the user's tier, never lower … not want a lower quality tier for gallery
> previews."* ⚠ Open work 6 still describes this leg as *"the gallery (Quarter)
> for the hub (Full)"*, which was the PRE-repair behaviour that same section
> reports fixing.
>
> ⚠ **The 10.0s row said "after the retire" until it was checked, and that was an
> assumption, not an observation.** Residency GREW between the two samples — 236
> images to 244, 2012.0 MB to 2030.9 — and a retire drops pages, so both readings
> are almost certainly BEFORE the retire lands. ⇒ Which makes ≈2.03 GB a PEAK
> while both rooms' casts are held, not a settled steady state, and the settled
> figure is a third measurement nobody has taken. The peak is the right input for
> a budget ceiling either way, which is why the correction does not change the
> answer — only what the number is called.

> ⭐ **SO THE ASK IS ANSWERED: ≈2.03 GB is `resident_mb` after a hub→hall→hub
> walk at full texture resolution, and it needed no 3090.** The round trip peaks
> ~580 MB above the hall entry alone, because it holds both rooms' casts before
> the retire — which is precisely the pressure the "keeps the last room's cast
> resident" policy has to budget for, and precisely why the entry asked for the
> walk rather than the room.
>
> ⚠ **Read the detail below before quoting these.** The first two rows differ
> only by an env var that turns out to be INERT — one configuration measured
> twice, not a tier comparison — and `capture_scene` reports 119.4 MB for the
> same room because ITS composition seeds quality from the Cpu adapter and loads
> `sprites_potato`. The number is a property of the composition as much as the
> room.

ⓘ **2026-09-03, calculex — the ADAPTER may not be what blocks this, which would
make the ask smaller than 3090 time.** Two things were checked, and neither is a
claim that the number has been taken:

* **`resident_mb` is adapter-independent by construction.** The census computes
  it as `width * height * per_pixel`, where `per_pixel` comes from the image
  FORMAT's `block_copy_size` (`crates/ambition_render/src/asset_census.rs:218`).
  It is decoded-image arithmetic, not anything the GPU reports, so llvmpipe and a
  3090 should agree for the same assets and the same walk.
* **The census DOES emit on this host.** `capture_scene` renders offscreen
  through lavapipe and prints the full `[image-census]` line including
  `…MB resident`; the headless room harness does not, because it composes no
  render app. So the missing piece is a WALK DRIVER, not an adapter:
  `scripts/profile_desktop.sh` is the documented driver and its windowed path
  refuses without a display *("a windowed run needs a display, and failing here
  is the point")*.

⚠ **Two caveats that keep this an ⓘ and not a ✔.** The walk has not been driven
here — `capture_scene` takes `--press`/`--route` but room-to-room transitions
through doors were not attempted. And quality seeds itself to `potato` on a Cpu
adapter, so a Full measurement needs `AMBITION_QUALITY_PROFILE=Full` and would
have to be checked on the `[census] visual_quality` row rather than assumed.
⇒ The decision this entry asks for is unchanged. What changed is the cost of the
input: possibly a driver on any host rather than time on the one 3090.

> **MEASURED 2026-09-03, and the two caveats above resolve in opposite
> directions.**
>
> ✔ **The census is takeable here, and the number is real.** `capture_scene
> hall_of_characters player … 640x360 --warmup 60` on lavapipe prints
> `[image-census] total 235 images, 29.9MP, 119.4MB resident | gpu +235 | awaiting
> gpu 0 | re-decodes 0`. So `resident_mb` after a room is loaded needs no GPU.
>
> ✔ **AND THE WALK DRIVER EXISTS — the first caveat was simply wrong.** I checked
> `capture_scene` and `profile_desktop.sh` and concluded no driver could cross a
> door here, without looking at the test suite.
> `game/ambition_app/tests/hall_transition_cover.rs` builds
> `build_visible_app(VisibleRenderMode::NoWindow, …)` and drives *"the REAL
> transition, resolved through the room graph rather than synthesised: stand in
> the Hall door and press interact"*. It is a module of `app_it`, so **that hub →
> hall crossing already runs on this machine in every gate run**, and it reads the
> ledger in process (`resident_character_pages`) rather than parsing a printed
> line. Its own comment records `22 → 226 resident at the hall entry`.
>
> ⛔ **The second caveat is CONFIRMED, and by measurement rather than the reason I
> gave.** Two runs, one with `AMBITION_QUALITY_PROFILE` unset-equivalent and one
> with a VALID `ultra`, both log *"visual quality seeded to `potato` for a Cpu
> adapter (llvmpipe)"* and produce a byte-identical census — same 235 images, same
> 29.9 MP, same 119.4 MB. So the tier does not move through that lever in this
> tool, and **a high-tier residency figure is not takeable this way**. ⇒ That is
> the one thing still genuinely blocked, and it is a quality-selection question,
> not an adapter one.
>
> ⚠ **My own error, recorded because it cost two runs:** the caveat above said
> `AMBITION_QUALITY_PROFILE=Full`. There is no `Full`. The labels are
> `potato|low|medium|high|ultra` (`settings/video/quality.rs`), and
> `capture_scene`'s own help already says `AMBITION_QUALITY_PROFILE=ultra`.
> `from_label` returns `None` on an unparseable value *"so a typo boots the user's
> OWN setting instead of silently substituting a tier they did not choose"*.
>
> ⓘ **And the warning for that case EXISTS and is well written** —
> `log_quality_profile_override` (`ambition_render/src/quality.rs:182`) warns
> *"…is not a profile; using the saved setting instead. Expected one of: potato,
> low, medium, high, ultra"*. It simply never fired: **neither** run printed it,
> the valid one included, and neither printed the success message either. So
> `VisualQualityPlugin::build`'s logger has nowhere to write.
>
> ⇒ **AND THE MECHANISM IS EXACT.** `build_visible_app` drops `LogPlugin` for
> `NoWindow`/`OffscreenGpu` — its comment says why: *"tests build several Apps
> per process; the tracing subscriber is process-global."* `capture_scene` adds
> it back, but **after** the plugin group
> (`capture_scene.rs`, `app.add_plugins(bevy::log::LogPlugin::default())`
> following `build_visible_app_with`). So every line a plugin emits during
> `build()` is written with no subscriber installed and is lost, while anything
> logged later from a SYSTEM survives — which is exactly the pattern observed
> here: no override message, but the adapter-seeding line printed normally.
> ⚠ This is not specific to quality. **Any** build-time log in any plugin is
> silent in this tool, which is worth more than the one message that led me to
> it.
>
> ⛔ **AND THE LEVER IS INERT HERE, NOT MERELY UNREPORTED.** Both runs loaded a
> byte-identical asset set — 2 images from `sprite_packs/full/` and 14 from
> `sprites_potato` — so `AMBITION_QUALITY_PROFILE` changed nothing about what
> became resident, valid label or not.
>
> ⛔ **AND RESIDENCY IS TIER-DEPENDENT BY DESIGN, so that inertness is the whole
> blocker.** `VisualQualityBudget::for_profile`
> (`ambition_persistence/src/settings/video/quality.rs`) sets
> `resolution_scale` per tier: the potato/low tiers take
> `TextureResolutionScale::Potato`, medium takes `Half`, and **high and ultra
> take `Full`**. Each maps to a different sprite tree — `sprites_potato`,
> `sprites_0_5x`, or the unsuffixed base — so the tier decides which pixels
> become resident, and `resident_mb` must move with it.
>
> ⇒ **So `capture_scene`'s 119.4 MB is the POTATO figure** — it seeds from the
> Cpu adapter and loads `sprites_potato`. The full-resolution figure in the table
> at the top comes from `hall_transition_cover`, whose composition does NOT seed
> from the adapter and loads the base tree: **363 MP against 29.9 MP, twelve
> times the pixels, for the same room.**
>
> ⛔ **AND THE ENV VAR EXPLAINS NEITHER.** Running the test WITH and WITHOUT
> `AMBITION_QUALITY_PROFILE=ultra` gives 1455.8 MB and 1452.3 MB — the same
> configuration twice. The lever is inert in both tools; the twelve-fold gap is
> which composition seeds quality from the adapter, not which tier was asked for.
> ⇒ **Two tools on one machine disagree about residency by 12× for reasons that
> have nothing to do with hardware**, which is worth more to open work 4's budget
> policy than either number alone: a ceiling is meaningless until the composition
> that produced it is named beside it.
>
> ⚠ **Recorded because I got this wrong once in the other direction too:** an
> earlier version of this note said the asset set "does not move with the tier at
> all", inferring an invariant from two runs that were both potato. Two identical
> measurements of the same configuration are one measurement.

### D-RASTER-3's remaining half

Splitting the weak-GPU 2.54× between framebuffer scale and MSAA needs an
interleaved A/B on real weak-GPU hardware with the independent
`AMBITION_MAX_SCALE_FACTOR` and `AMBITION_MSAA` knobs, multiple reps per arm,
build/features/profile held constant. ⛔ Explicitly not lavapipe: the row says so
and the substitution is what made the original result unattributable.

### Switch Pro outer stick range

The remaining cross-machine controller question needs the actual hardware
measurement: run the existing `Shift+F6` axis probe on both machines, push the
Switch Pro to each extreme/corner, and compare reported peak magnitude.

The proposed shared outer-saturation fix should be judged only after that number
exists. This is tracked in the execution queue as an external measurement, not a
maintainer design decision.

### ~~Which character owns each per-fighter FX sheet~~ (raised 2026-09-02, withdrawn 2026-09-02 — no decision was needed)

The question assumed the demand seam needed a sheet → CHARACTER ID table. It
does not: a realized character carries its own prepared moveset
(`PreparedCharacterDefinition.kit.projectable_moveset()`), and the moveset's
`Vfx` events name the rows. `character_sprites::demand_character_fx_sheets`
asks that moveset the frame the character realizes and decodes whichever
character-owned sheets its rows live on (`fx::owned_fx_sheets_named_by`) —
ownership is read off the content that fires the effect, not off a name.
Landed in `asset-preparation-and-residency.md` §2. The two never-wired sheets
below are unaffected: a sheet no moveset names is now never decoded, which is
the correct residency for art nothing can request, and the wiring question
stays the owner's.

## ✔ WITHDRAWN 2026-09-02 — "is 8% of the floor crate worth an encoder split?"

Raised here the same day and withdrawn the same day: it is an ENGINEERING
question, not a product one, so it is answered in
[`engine/control-authority-and-ai-policy.md`](engine/control-authority-and-ai-policy.md)
instead. Short answer: do not split. Of the three available shapes, two are
refused by rules the repository already holds (no service locator; the orphan
rule again one crate up) and the third fails the plan's own acceptance criterion
4, because `ambition_mount` holds a `Brain` by value and does not depend on
`ambition_combat` — so moving `Brain` into a combat crate makes a movement-only
game link one. Recorded here only so the question is not raised a third time.
