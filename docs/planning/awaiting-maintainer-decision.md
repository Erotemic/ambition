# Awaiting a maintainer decision

This file contains only **currently unresolved maintainer/product decisions**.
Engineering work that can proceed without a ruling belongs in
[`queue.md`](queue.md). Answered, withdrawn and superseded questions are deleted
from this live ledger; Git history preserves the discussion.

Use one unique `Q<number>` per live question. Do not reuse a retired number.

⛔⛤ **AND A ROW THAT SAYS "A MAINTAINER'S CALL" MUST HAVE A `Q` HERE. FIVE DID
NOT, FOUND 2026-09-12 BY CENSUS RATHER THAN ONE AT A TIME.** `queue.md` declared
maintainer holds in five sections; `A10`'s guarantee choice, the headless
render-sync choice, the per-move `hitbox.inflate` values, the rung-9 execution
noise and the truthful attack kit were all absent from this file — two of them for
days, and one of them (`Q115`) is the item Jon asked for BY NAME, measured to its
decision point and then waiting where he would never see it.

⇒ **A packet held on a decision that is not in the decision ledger waits forever
without anyone declining it**, and from the maintainer's side that is
indistinguishable from work nobody did. The cheap check is one command:
`grep -nE "maintainer'?s call|is Jon's|needs a ruling" docs/planning/queue.md`,
then confirm each hit has a `Q` here. ⚠ **The queue row must also name the `Q`**,
or the disconnect simply recurs in the other direction.
When a question is answered, record the durable ruling in
[`maintainer-decisions.md`](maintainer-decisions.md), update the owning plan/source,
and delete the question here.

## Gameplay and content

## Q33 — how should a recharging ranged weapon communicate that it is unavailable?

Choose the player-facing unavailable/readiness signal. The mechanism should not
invent one presentation independently for every ranged weapon.

## Q36 — what are the authored standing heights of the puppy slug, stochastic parrot and burning flying shark?

The engine has a canonical-height contract; these remaining authored characters
need product values rather than inferred sprite dimensions.

## Q37 — should the F9 rollback proof pulse survive a gameplay-session change?

Decide whether the pulse demonstrates one rollback session or is a process-level
debug affordance. Its resource lifetime should follow that answer.

## Q38 — does an actor released in a foreign room stay there?

This is a world/product rule about custody and room membership after release, not
a query-order choice.

## Q40 — should a held gun-sword kick the player the way it kicks the pirate?

Choose whether recoil is an authored weapon property applied to every holder or a
pirate-specific behavior.

## Q41 — where should a hand-fired fireball leave the body?

Choose the authored launch landmark/offset contract for hand-fired projectiles.
Do not derive it independently from sprite bounds at runtime.

## Q42 — should the gauntlet fireball keep bespoke art or use the catalog energy ball?

Product/art choice. The runtime should consume one authored presentation identity
once chosen.

## Q43 — does a body hanging on a ledge inside a hazard volume die?

Choose the game rule for ledge custody versus hazard damage. The engine can then
encode one authority rather than special-case the observed overlap.

## Q44 — should `SmashChargeSpec` keep a Smash-specific name?

The mechanism is now broader than one game mode. Rename only if the intended API
is reusable; do not churn names solely for aesthetics.

## Q45 — is a unique capability item an entitlement or an occurrence?

Choose whether losing/dropping the physical object also loses the capability.
Keep occurrence, custody and entitlement separate in either answer.

## Q46 — does Mary-O 1-1 want a fourth question block over floor?

Content-layout choice needed to make the floor-refusal behavior of the fire form
meaningfully playable.

## Q47 — where does TwinTrack's simultaneity limit live while the exhibit is parked?

Choose whether the parked exhibit still consumes a global/participant/world
simultaneity slot.

## Q49 — is near-identical CPU play on a symmetric stage acceptable?

If yes, no diversity mechanism is owed. If no, specify whether the desired
variation is tactical policy, difficulty behavior or presentation/personality.

## Q51 — does a boss reward survive a death/checkpoint rewind that un-fights the boss?

Choose the reward's durability boundary. Boss defeat and reward occurrence are
already separate facts; this decides which checkpoint/replay retracts the latter.

## Q52 — does a second bark set for one enemy replace the first or conflict?

Choose whether bark sets are single-valued authoring or composable collections.
The content validator should enforce the chosen cardinality.

## Q54 — in co-op, does a body gate open for the party, the acting body, or only the primary body?

Choose gate subject semantics. Do not hard-code primary-player behavior into the
generic gate vocabulary.

## Q55 — should authored worlds grow to use all five route-gate families?

The vocabulary exists. Decide whether broader authored coverage is desired now or
the unused families should remain capability surface without demo customers.

## Q56 — should room replay retract boss defeat for every boss family or only the currently wired one?

Choose the intended replay durability rule, then make the generic boss-progress
road enforce it uniformly.

## Q58 — does the BODY gate family ask what a body *can do* or what it *is doing*?

Capability and current action are different facts. Pick the authored gate
semantics before extending content usage.

## Q64 — what happens to a held gun when its holder is clipped by a far-side portal?

Choose whether the item gets its own clipped body-owned presentation, disappears
with the holder, or follows another explicit rule. Do not let draw order decide.

## Q67 — does the Limit meter survive stock loss?

Choose stock lifetime for the meter. The earlier free-Limit respawn bug is fixed
independently of this decision.

## Q70 — should the title Settings tab visibly highlight on pointer hover?

Small UI/product choice; implementation already has the semantic tab state.

## Q71 — how much Limit should a successful block award?

Generic meter policy now permits a block source. Choose the Smash balance value;
keep it out of generic validation.

## Q79 — how far may the camera zoom out before the fight stops being legible?

Choose the product legibility floor. Camera policy can then clamp against a named
limit rather than an arbitrary tuning value.

## Q80 — what does “the art agrees with the hitbox” mean in pixels?

Choose the tolerance/measurement contract used by sprite/hitbox authoring tools.
This should be a measurable visual-authoring rule, not a subjective test failure.

## Q87 — should the top platform and respawn point continue to overlap?

Stage-layout/product decision. If not, move one authored placement rather than
adding runtime avoidance.

## Q89 — what special should each Robot stand-in have?

The stand-ins currently lack the button vocabulary expected by their match role.
Choose authored moves or explicitly accept the omission.

## Q91 — keep the 10× countdown mode?

This was explicitly requested earlier and remains available. Decide whether it is
still a product/debug affordance worth carrying.

## Q93 — should the demo author a dense melee room?

Product/content call. Do not add engine behavior merely to manufacture a stress
scene unless the room itself is wanted.

## Architecture and engine policy

## Q34 — should external/launch-owned motion become an explicit cross-game fact?

If more than one game needs it, give it a reusable authority. Otherwise leave the
current local mechanism local.

## Q35 — what owns fighter reach during move startup?

Choose the semantic owner of startup reach so AI, collision and presentation do
not each infer it from different move state.

## Q48 — should the boss subsystem become a separately composed crate now?

The reassessment requested earlier is due. Judge against current dependencies and
customers, not the historical monolith shape.

## Q59 — two validation ledgers can be red when no lane ran: hook the lane or accept the state?

Choose whether “not run” is a first-class incomplete receipt or whether those
ledgers should be removed from the required surface.

## Q61 — where should ordering live when two systems write the same durable switch?

Choose the intended winner/merge policy where product meaning is ambiguous.
Engineering already requires one accepted mutation authority and explicit phase
visibility; a public set by itself cannot decide between competing writes. See
[composition](engine/capability-and-runtime-composition.md).

## Q62 — keep or discard the epoch-captured 4,741-line `mary_o.ldtk` delta?

This is the explicit history/content decision. Do not modify the retained LDtk
files until the ruling is made.

## Q63 — five authored fields still have no runtime consumer: wire them or delete them?

For each field, choose intended capability versus obsolete authoring. Do not keep
permanent knobs that decide nothing. The review traced three examples end-to-end:
`requires_facing`, pickup `collected`, and chest `persistent`; see finding F4 in
[the source findings](engine/architecture-review-findings.md). The five-field
label is the earlier inventory, not a new claim that all five were revalidated.
Until policy is chosen, unsupported nondefault values should receive diagnostics,
not an invented runtime meaning.

## Q66 — should the citation checker become a ratcheted gate now that its baseline is zero?

Choose whether citation health is required on every relevant change or remains an
advisory audit.

## Q68 — which settings are the evergreen baseline every game inherits?

Choose the cross-game groups/surface. The full settings menu and shell-level audio
controls already exist; this decides composition, not implementation feasibility.

## Q72 — `EncounterEffect::SetMusic` has no production customer: give it semantic identity or remove/defer it?

If retained, multiple simultaneous scripts must not share one subsystem-wide
claimant. If no authored use is planned, remove/defer the unused capability.

## Q73 — may a capability plugin install private systems into a published set under the `ambition_combat` no-plugin stance?

Clarify whether the stance forbids all plugins or only host-owned opaque
installation. C2 needs one durable interpretation. Owner-local installation
helpers and explicit host ordering remain available without prejudging this
plugin-style decision; no registry or broad context is required.

## Q74 — keep or cut the three declared dependency seams that still have no customer?

A seam with a plausible near-term composition use may stay; otherwise remove the
dependency rather than preserving hypothetical architecture. Recheck actual
production call sites and supported profile closure separately. The A9 render
dependency finding concerns a mandatory reachable path, not this older unused-seam
inventory; one is not evidence for the other.

## Q75 — can inventory, dialogue and map coexist in the same frame?

This is the factual reachability question behind the unordered `MenuControlFrame`
reader pairs. If coexistence is supported, input ownership/ordering needs a real
fix; if forbidden, encode the exclusivity as an invariant/test.

## Q76 — are composite mount-riders actually planned?

`MountedBrainCache` has no production constructor at the reference head. Keep the
capability if future authored composite riders are intended; otherwise simplify
rather than maintaining an unused semantic branch.

## Q77 — which target-reclaim rule wins: the sanctioned cleanup-script exception or the repository's “never delete target” rule?

Resolve the contradictory operational guidance before another cleanup tool acts
on the target directory.

## Q78 — how should the divergent/unpushed sprite-renderer submodule state be reconciled?

Before any blind `git submodule update`, decide which line/commit must be kept and
pushed. Tooling warnings should cite this question until the submodule state is
settled.

## Q86 — should cast framing be bidirectional (target) rather than only a floor/minimum?

Choose whether framing owns both minimum and desired composition or only prevents
excessive zoom-in.

## Q94 — what residency-memory limit should the runtime target?

Needs a maintainer/hardware/product value. The residency mechanism can enforce a
budget once the budget exists. Report source, decoded CPU, prepared simulation
content and device residency separately. A8 instance isolation and A9 dependency
closure do not supply a hardware budget.

## Q110 — may a provider-keyed fragment registry gain a NAMED hot-reload replacement operation?

All five provider-keyed fragment registries — `AudioCatalogRegistry`, `SfxBankRegistry`,
`BossCatalogRegistry`, `CharacterCatalogRegistry`, `AdaptiveMusicCatalogRegistry` — accept an
identical re-registration, REFUSE a changed one, expose no remove/replace/clear, and panic at the
App seam. Re-derive rather than trust the counts:

    git grep -hoE "pub fn [a-z_]+" -- \
      crates/ambition_audio/src/catalog.rs \
      crates/ambition_boss_encounter/src/catalog.rs \
      crates/ambition_characters/src/actor/character_catalog/registry.rs \
      crates/ambition_audio/src/music/catalog.rs
    git grep -nE "unwrap_or_else\(\|error\| panic!" -- <the same four files>

The first prints each registry's whole `pub fn` surface, so the absence of remove/replace/clear is
visible by inspection rather than by an empty result. The second returns 5, one per `*AppExt` trait.

That refusal is protocol, not oversight: `ambition_registry_core::classify` answers
New/Idempotent/Conflict and states there is deliberately no fourth answer, *"so a silent overwrite
cannot be the accidental default"*. The alternative is already contemplated in
[the registry triage](triage/ambition-registry-core.md): *"a separate explicitly named replacement
operation may be appropriate for a hot-reload boundary; it should not be the accidental semantics of
ordinary registration"*, with `PreparedCharacterRegistry`'s stage/admit/publish split as the worked
precedent.

**The question.** May a fragment registry gain such an operation, so a content generation can
re-publish one provider's fragment — and if so, does it belong on all five or only where a reload
boundary exists? `engine.audio-authority-is-app-local` and `engine.shell-audio-authority-is-explicit`
(both severity `error`) constrain the audio half; the latter names *"an optional audio registry"* as
the thing it prevents.

Measured 2026-09-12 by NamekAmbition at `4d486ef20`. The consequence for content families is in
[the queue](queue.md)'s second-family row.

## Q111 — may `BossCatalog` and `CharacterCatalog` ever be absent, and is that one ruling or two?

`engine.character-authority-is-app-local` (severity `error`) states production code may not
*"reinstall process-global authority, silently substitute an empty catalog, make those resources
optional, or use retired implicit lookup wrappers"*. The two catalogs are NOT in the same state
against that rule:

- `CharacterCatalog` ALREADY has one optional production reader with a written floor.
  `speak_conversation_cut_barks` takes `Option<bevy::prelude::Res<CharacterCatalog>>`
  (`crates/ambition_platformer2d_actor_monolith/src/features/npcs.rs:497`), is registered into the
  sim schedule at `crates/ambition_platformer2d_actor_monolith/src/features/mod.rs:1375`, and its doc
  states the reason — *"a composition with no catalog (a demo, a headless fixture) must still break
  conversations, and losing an unwritten line is not worth failing over."*
- `BossCatalog` has none. Every reader is required, and
  `crates/ambition_platformer2d/src/game_assets.rs:102` panics naming that policy id.

Re-derive both with patterns that do not depend on how `Res` is qualified:

    git grep -nE "Option<[A-Za-z_:]*Res<(BossCatalog|CharacterCatalog)>>" -- '*.rs'
    git grep -nE "remove_resource\s*::\s*<\s*(BossCatalog|CharacterCatalog)\s*>" -- '*.rs'

The first returns one production reader and one comment; the second returns nothing, while the same
pattern finds `AuthoredFighterLadder` in `game/ambition_content/src/reload.rs` — the family that CAN
publish its own removal.

⚠ Do not read the policy's green as evidence the optional reader is sanctioned — it never
objected, for TWO independent reasons. (a) Position: line 497 sat past the file's first
`#[cfg(test)]` (line 211), inside a region the scan did not reach; that truncation defect and its
repair are recorded in [the queue](queue.md). (b) Spelling, which that repair does NOT touch: no
needle in this policy's `forbid` list matches the real text of line 497. The list enumerates
`Option<Res<CharacterCatalog>>` and a fully-qualified variant, and the code reads
`Option<bevy::prelude::Res<CharacterCatalog>>`, matched by raw substring with `whole_ident` unset.
⇒ The guard written to forbid exactly this reader still cannot see it, so the question below has
never actually been put to the policy.

**The question.** (1) Is the existing optional reader sanctioned, or a violation to close?
(2) Is a floor one ruling for both catalogs or two — for `CharacterCatalog` it extends something that
already exists, while for `BossCatalog` it would be a new exception against a panic that names its
own policy id.

Measured 2026-09-12 by NamekAmbition at `4d486ef20`.

## Q112 — ranged recoil writes velocity directly while the kernel documents a seam for exactly this reaction; should it move, and if not, where is that recorded?

✅ **RULED AND DONE 2026-09-12 — AND IT SHOULD NOT HAVE BEEN FILED HERE.** The
answer: **route it through the seam.** Recoil now stages through
`BodyFlightState::stage_launch`, the same gateway `hit_reaction.rs` uses for
knockback, and **the authored magnitude is unchanged.**

⛔⛤ **WHY THIS WAS NOT A MAINTAINER QUESTION.** It was filed on the grounds that
moving the write *"changes game feel"*. The question it actually asks is **who
may interpret an externally authored world-space impulse for a body whose
movement model owns its velocity semantics** — and the motion architecture has
already answered that: the movement model does. A question is not a maintainer's
merely because more than one implementation is conceivable; the test is whether
the project's existing ownership model already determines which is structurally
correct. Here it did.

⚠ **AND THE "FEEL" FRAMING HID A DEFECT.** A `vel` write is authoritative for an
axis-swept body and **INERT for a surface-momentum one**, whose `vel` is derived
from `v_t` and republished every step — the same shape as *"Sanic took knockback
with every number non-zero and never moved"*. ⇒ The waived line **did nothing at
all** for that entire motion model. What was filed as a tuning preference was a
mechanic that did not work for part of the roster.

⇒ **DECIDED EXPLICITLY, not defaulted:** `flinchless: true`, because
`PendingLaunch::flinchless` means *"a push, not a hit: it moves the body and
leaves it in control"* — a gun's kick is exactly that, and recoil that tumbled
its own shooter would be a new mechanic rather than a relocation. And the staging
**ADDS to a launch already waiting** rather than replacing it, keeping
`flinchless` FALSE when a real hit is pending: `stage_launch` overwrites, so the
one-line version would have deleted a knockback that arrived on the same frame
the body fired.

✔ **THE WAIVER IS DELETED, NOT LEFT MATCHING NOTHING.** `engine.toml`'s
`allow_lines = ["policy: ranged recoil, authority unresolved"]` was this policy's
only exemption; an `allow_lines` needle that no longer matches is an amnesty row
that silently re-arms the day somebody writes that comment again, and its
emptiness is invisible to a green suite. `engine.velocity-writes-are-authority-only`
now has ZERO waivers and zero violations; `cargo test -p ambition_workspace_policy`
is 36/36 including `velocity_write_guard_reacts`.

Guards: `ranged_recoil_is_staged_as_a_launch_rather_than_written_onto_velocity`
(asserts the exact authored vector, so a relocation that rescaled it fails) and
`recoil_adds_to_a_waiting_launch_instead_of_erasing_it`. Both poison-verified by
restoring the direct write — both fail, and the restore is md5-confirmed.


`spawn_projectiles_from_brain_actions` applies recoil by writing the firing body's velocity itself:

    crates/ambition_platformer2d_actor_monolith/src/features/ecs/brain_effects.rs:381
        kin.vel += kick; // policy: ranged recoil, authority unresolved

(inside `pub fn spawn_projectiles_from_brain_actions`, declared at `:93`; the line was `:354` before
`8bd1d884d` added the block comment that now stands above it.)

The movement kernel documents a different road for exactly this kind of write, and says why
(`crates/ambition_platformer2d_core/src/movement/kernel.rs:157`):

> An external reaction (knockback, a fling) writes a world-space launch into
> `BodyFlightState::pending_launch` and cannot apply it itself: it holds a `&mut Vec2`, not the
> model, and only the model knows what a launch MEANS to it. Writing `kinematics.vel` directly is
> authoritative for an axis-swept body and a LIE for a riding surface-momentum one, whose `vel` is
> derived from `v_t` and republished every step — which is why Sanic took knockback with every
> number non-zero and never moved.

The seam is `BodyFlightState::pending_launch` (`crates/ambition_platformer2d_core/src/body_clusters.rs:429`),
staged by `stage_launch` (`:473`) and drained in one place at the top of `step_motion`
(`crates/ambition_platformer2d_core/src/movement/kernel.rs:183`). The field doc at `:444` names the
sanctioned call: *"STAGE THE PAIR WITH `stage_launch` AND DRAIN IT WITH `take_launch`. A caller that
writes `pending_launch` directly gets `false`."*

⚠ The road is paved but barely travelled — re-derive rather than trust the count:

    git grep -nE "stage_launch|pending_launch\s*=" -- '*.rs'
    git grep -nE "kin\.vel \+=|kinematics\.vel \+=" -- '*.rs'

The first shows ONE production stager, `crates/ambition_combat/src/hit_reaction.rs:326`
(`flight.stage_launch(launch, knockback.is_windbox());`, production — that file's first
`#[cfg(test)]` is at 472), against 21 test and fixture stagers, whose own comment gives the recoil
site's situation word for word: *"Written here rather than applied here for the same reason: this
function has a `&mut Vec2` and no world and no `MotionModel`."*

[ADR 0024](../adr/0024-frame-aware-unified-movement-kernel.md) §8 names both reactions in ONE
sentence: *"Knockback, recoil, explosions, and scripted pushes are typed world-space
impulses/accelerations accumulated before the one kernel call."* Its status is *"Accepted;
implemented"*. So knockback takes the documented road and recoil does not, and the ADR does not
distinguish them.

⭐ MEASURED: the documented road is also the policy-clean one. Against
`engine.velocity-writes-are-authority-only`'s forbid list, `flight.stage_launch(kick, false);` is
CLEAN and `kin.vel += kick;` trips the needle `kin.vel += `. The guard is not asking for an
exception; it is pointing at the seam. The site is reachable from that layer —
`crates/ambition_platformer2d_actor_monolith/src/actor_clusters.rs:64` exposes
`pub flight: &'a mut BodyFlightState` — though
`crates/ambition_platformer2d_actor_monolith/src/features/ecs/brain_effects.rs` does not hold it
today (its `flight` names are `ProjectileFlight`, the projectile's own envelope, not
`BodyFlightState`).

⛔ NOT MEASURED, and it decides how much this matters: whether a body that fires can ride surface
momentum, which is what would make the direct write silently do nothing rather than merely bypass the
seam. `SurfaceMomentum` is opted into by a character catalog field
(`crates/ambition_characters/src/actor/character_catalog/entry.rs:100`), not by demo, so any authored
character can take that model — but no current ranged producer is known to. "No current content does
this" is a fact about the roster, not the code.

**The question.** (1) Should ranged recoil move to `stage_launch`, as knockback already has?
(2) If this site genuinely differs — a recoil kick is the shooter's own action rather than something
done TO it, which may be a real distinction the ADR's one sentence flattens — then what records that,
and where? ⚠ As of `8bd1d884d` the site is EXEMPTED IN PLACE, which is not the same as resolved: the
write carries the marker `// policy: ranged recoil, authority unresolved`, and
`tests/ambition_workspace_policy/policies/engine.toml` carries a single `allow_lines` entry of
exactly that text. Measured on that commit, the waiver string matches exactly ONE line across the
1850 `.rs` files under `crates` and `game`, so it exempts the authored site and nothing else. The
link runs both ways on purpose — `git grep -n "policy: ranged recoil" -- '*.rs'` finds the write from
the waiver, and the waiver's own text names this question — so a reader who arrives at either end can
reach the other. ⛔ A provisional marker becomes permanent by default, which is how the optional
catalog reader in Q111 reached today: nobody ruled, so the absence of a ruling became the rule. That
is why the question is filed here rather than left to the marker.

⛔ No authored value has been changed, and none should be on a scanner's account: whether recoil
routes through the kernel changes game feel, and a scanner repair is the wrong provenance for that.
The surviving finding that surfaced this is `production_slice`'s one-line fallout, recorded in
[the queue](queue.md).

⚠ THIS ROW IS THE AUTHORITY; THE BLOCK COMMENT ABOVE THE WRITE IS A POINTER TO IT. The duplication is
deliberate — the write is where somebody stands when they wonder — but two full statements of one
open question rot apart, so the comment carries the case and this row carries the decision. If they
ever disagree, this row is what was decided and the comment is what went stale.

Measured 2026-09-12 by NamekAmbition at `8bd1d884d`.

## Q113 — is the STRONGER last-good-world guarantee wanted, given its only known implementation is A10 itself?

✅ **RULED 2026-09-12: YES — TAKE THE STRONGER GUARANTEE. A10 IS THE NEXT MAJOR
ARCHITECTURE PACKET.** Recorded in
[`maintainer-decisions.md`](maintainer-decisions.md). ⚠ Provenance stated because
it is not a separate instruction: this was ruled in the architecture review Jon
forwarded on 2026-09-12.

⇒ **THE SHAPE, and the boundary matters as much as the answer:**

```text
last-good playable world N
        ├── build candidate N+1 off to the side
        ├── reject it safely if invalid
        └── publish N+1 only when construction succeeds
```

rather than destroy/mutate N, construct N+1, discover something invalid, and
recover afterward.

⛔ **NOT ARBITRARY TRANSACTIONAL ROLLBACK OF ARBITRARY ECS COMMANDS** — the ruling
says so explicitly, and it is the failure mode this packet has always been one
step from. A10 stays bounded to: typed construction recipes; constrained candidate
construction; validation of relationships and resources; ONE controlled
publication boundary; explicit retirement of the old world. **One supported
scene/reconstruction path, strong end to end** — not a universal recipe language,
not a World clone, not a second lifecycle coordinator.

⭐ **WHY IT IS WORTH ITS IMPLEMENTATION, which is the question this row actually
asked:** I3 delivered the CONTENT half of the same architecture — prepare a
generation, admit it, verify it, publish atomically. A10 is the SCENE half of that
identical shape, and the pairing is what makes fast, safe edit→play iteration real
rather than a property of one resource family.


⛔ **A10 HAS BEEN HELD ON THIS AND THE HOLD WAS NOT IN THIS FILE** — found
2026-09-12 by re-deriving the blocker rather than trusting the note I was
carrying, which said "A10 needs a stated failure guarantee". It does not: the
guarantee for the WEAKER form is already written.

`docs/planning/engine/checkpoint-restoration-protocol.md`: *"After destructive
application begins, this contract promises fail-closed publication, **not rollback
of arbitrary Commands**. The stronger last-good-world guarantee remains A10 and
requires constrained inactive construction."*

⇒ So the question is not what to promise; it is **whether the stronger promise is
worth its only known implementation**, which is A10's own deliverable —
constrained typed inactive construction over the shared construction
executor/recipes. That is circular by construction, which is why the packet
cannot start itself, and the same document explicitly forbids doing it
opportunistically (*"Do not implement that larger project as an undocumented
prerequisite to A1"*).

**What each answer unblocks.** *Fail-closed is enough* → A10 shrinks to the
already-written contract plus its acceptance arms, and the P0 queue row that reads
as an implementation task can be re-scoped or closed. *The stronger guarantee is
wanted* → A10 becomes a real packet and needs sizing against the construction
executor, and F6's conditional-recovery language becomes load-bearing.

⚠ **THE CONTENT TRANSACTION'S RECEIPTS ARE NOT PROGRESS ON THIS.** A10 is the
SCENE half — recipes, resource writes, hooks, observers — and its acceptance list
(invalid candidate relationships, duplicate identity, forbidden resource mutation,
`recovered` rather than `unchanged`) names none of the content work that landed in
September.

## Q114 — for a `NoWindow` / `backends: None` profile, should the render-sync hooks be absent, served, or tolerant?

The queue row `D-HEADLESS-DESPAWN` ends *"NOT DECIDED. Whether the fix is to
install the resource in the headless profile, keep those systems out of it, or
make the hook tolerate a missing world is a maintainer's call. This row is the
report."* ⛔ That call was not in this file either, so a packet named as held for a
maintainer was waiting in a document maintainers do not read for decisions.

MEASURED (see the row for the backtrace and the four dead candidates):
`SyncToRenderWorld`'s **remove** hook needs `bevy_render::sync_world::PendingSyncEntity`,
which the headless profile does not hold, while that same profile installs **104
`ambition_render` systems in `Update`**. ⇒ Spawning a render-synced entity
headless is fine; DESPAWNING one is fatal. A duel runs 2.4 seconds first, which is
why it reads as a character bug.

**Why it is not merely curious:** any change that makes another fighter despawn a
render-synced entity makes that fighter's row UNMEASURABLE too, and the sweep
would report a narrower population and read as healthy.

**The three options, as the row states them:** install the resource in the
headless profile; keep those Update systems out of it; or make the hook tolerate a
missing render world. The second is the only one that also answers why a profile
with `backends: None` installs the presentation half at all.

⚠ SCOPE, because an earlier version of this got it wrong: the subject is the
PROFILE, not the machine. An agent box with no discrete GPU still reports a
working software adapter, and `VisibleRenderMode::OffscreenGpu` composes a render
app there.

## ✅ Q116 — CLOSED 2026-09-12 AS A CORRECTNESS REPAIR. It was a defect, and the fix was the QUANTIZATION, not the constant.

⭐ **RULED BY THE ARCHITECTURE REVIEW JON FORWARDED**, whose reasoning is the
part worth keeping: *"Historical benchmark data already belongs to a
commit/configuration. Changing an opponent means new measurements describe a new
opponent. Otherwise every balance defect becomes permanent once somebody
benchmarks it."* ⇒ The "every rung-9 measurement becomes a measurement of a
different opponent" cost, recorded below, is not a reason to keep it.

⭐⭐ **AND THE FIX IS NOT THE ONE CONSTANT THE ROW BELOW PREDICTED.** Raising rung
9's noise until it cleared the rounding tie would be a number tuned to
`interval == 5`, and `decision_interval_ticks` is an AUTHORABLE field — a
character choosing 3 would put rungs 7-9 back under the boundary with nothing to
say so. `decision.rs` now quantizes PROBABILISTICALLY: whole ticks always, the
fractional tick with probability equal to the fraction. `P(jitter >= 1) = span/2`
for any span, so **a nonzero execution noise cannot be a no-op at any interval,
by construction**, and the ladder keeps every authored number it had. Expected
jitter is exactly `span/2`, so the rungs stay ordered and rung 9 stays the
smallest — which is what *"small numbers, never zero"* asks for.

⭐ **THE SECOND HALF CAME FOR FREE AND IS THE BEHAVIOURAL PROOF.** The per-seat
cognition stream's only consumer is this site, so rung 9 is now covered by
`a_different_stream_makes_a_different_fighter_wherever_the_jitter_is_reachable` —
which derives its own population and adopted rung 9 with a one-line edit.
POISON-VERIFIED: restoring `.round()` reddens it with *"rung 9: two seats on
DIFFERENT streams pressed on identical ticks (24 presses)"*.

⭐ **AND THE DUEL FLOORS SURVIVED THE RE-PRICING**, which was the open risk:
`two_cpus_in_the_shipped_composition_damage_each_other` and
`the_repertoire_gets_used::every_authored_route_gets_pressed` both pass at HEAD.
⇒ Q117's rung-9 evidence can now be re-measured against a CPU that obeys its own
difficulty contract.

<details><summary>The original row, kept because its measurement is the
evidence</summary>

### Q116 — is the hardest CPU shipping with execution noise DISABLED a defect, or an accepted cost?

MEASURED and guarded at `359c8be69`; the row is `D-RUNG9-NOISE` in
[`queue.md`](queue.md). **At rung 9 the fighter press jitter is identically zero
for every possible sample**, so the top rung presses exactly on its decision ticks
forever. Not arithmetic — a measurement: two seats on DIFFERENT seeds pressed on
IDENTICAL ticks across 600 ticks and 24 presses.

The arithmetic explains it. `execution_noise = 0.45 - t*0.35` with
`t = (level-1)/8`, `interval()` is 5, and `|sample|` reaches exactly 1.0, so the
rung-9 ceiling is `0.4999999701976776` in f32 — **under the rounding tie by 3e-8**
— and `round()` returns 0 for every sample including the maximum. Rungs 1–8 keep a
reachable jitter (rung 8's ceiling is 0.719, P(jitter>0) = 0.30).

⛔ THE FIGHTER-BRAIN DOC §1.3 SAYS level 9 is *"small numbers, never zero — a
frame-perfect CPU is not a hard opponent, it is a different game."* For this term
it is zero.

⛔⛤ **AND IT SILENTLY VOIDS A SEPARATE FIX.** That decision site is the per-seat
cognition stream's ONLY consumer in the tree, so at rung 9 the per-seat seed has
no observable effect at all: `medic` and `special_patent_clerk` hold distinct
seeds and their mirror duels drift 0.0000 px and 0.0022 px over 3613 ticks — the
reflection `two_participants_of_one_character_do_not_share_a_stream` exists to
prevent, unreachable at the shipped rung.

**What each answer costs.** *Defect* → the fix is ONE CONSTANT, and waking it
re-tunes every rung-9 CPU in the game; **every measurement ever taken at rung 9
becomes a measurement of a different opponent**, including all twenty rows of
D-CPU-INERT. *Accepted cost* → record it, and §1.3's sentence needs amending so
the next reader does not file this again.

⚠ THE GUARD PINS THE GAP, NOT THE FIX, deliberately: rungs 1–8 must keep a
reachable jitter AND rung 9's ceiling must stay just under the boundary, so a
ladder retuned to a genuinely small jitter reddens the second while the first
stays green. The ladder cannot drift into or out of a zero-jitter rung unnoticed
whichever way this is ruled.

</details>

## ✅ Q117 — CLOSED 2026-09-12. It LANDED, and it was never a semantic yes/no.

⭐ **RULED BY THE ARCHITECTURE REVIEW JON FORWARDED**, and the reasoning is the
part to keep: *"A planner that thinks it selected move A while the executor
performs move B violates the basic action-model contract"* — re-pricing is the
EXPECTED consequence of a decision model that starts reading the frame data of
the move it actually takes, not an argument against it. *"The right response is:
fix model truth → measure changed behavior → retune."* The feature flag is
deleted and `a_running_body_is_offered_the_dash_attack_its_press_would_actually_produce`
is mandatory.

⭐⭐ **WHAT IT ACTUALLY DID, MEASURED at the shipped duel (pirate admiral, rung 9,
with `Q116` already repaired):** the CPU now uses the dash attack deliberately —
`pirate_admiral_dash_attack` is seat 0's TOP damage move at 33 of 69, where it
was 18 of 75 — and **the duel now DECIDES, at tick 2320, where it used to run
3613 ticks undecided.** Four knockouts either way.

⛔⛤ **AND IT CAUGHT AN ACCEPTANCE FLOOR THAT COULD NOT EXPRESS A DECISIVE MATCH.**
`smash_cpus_damage_each_other` asserted BOTH seats take ≥ 50% of a pool per
minute; seat 0 took 0.44 **because it was winning**. `A_REAL_FIGHT`'s own doc
already names that artefact — *"a decided match stops accumulating damage the
moment a seat leaves the cast … and the winner, who by definition takes less,
read lower still"* — and dividing by duel ticks had only removed most of it. The
floor is now the EXCHANGE (the same total, as a sum) plus a per-seat check that
each seat dealt damage and entered hitstun. ⚠ That is a weakening, it is recorded
as one, and it is poison-verified both ways: an inert pair and a one-seat
passenger each still redden it.

⇒ **WHAT REMAINS IS A TUNING QUESTION AND IT IS JON'S:** what utility / run /
dash-attack parameters produce the fighter quality he wants now that the brain
evaluates the action it actually takes. That is not this row.

<details><summary>The original row, kept because its measurement is the
evidence</summary>

### Q117 — should the truthful attack kit land, given it re-prices every CPU matchup?

The row is `D-BRAIN-MENU`. **The fix EXISTS and runs** behind
`--features truthful_attack_kit` on `ambition_platformer2d_actor_monolith`,
default off, as ONE code path rather than a `#[cfg]` split — with the feature off
`running_now` is a compile-time `false`, so today's behaviour is the shipped one
BY CONSTRUCTION rather than by a second arm somebody has to keep in step.

**The defect it fixes is real and is a mislabel, not an absence.** `attack_kit_of`
resolves presses with `move_for_directional_verb` while the press road calls
`move_for_attack(base, dir, grounded, RUNNING)` — the same function with the
running branch skipped — so while the body runs, **the brain scores `jab`'s frame
data and the press produces `{base}_dash`**. Every scoring term downstream
(startup, reach, damage, frame advantage) reads the wrong move. Eighteen of
eighteen shipped fighters author a dash attack no press in the kit reaches.
⚠ The CPU DOES perform dash attacks today — the press road resolves the stance
itself — so anything reading as *"the CPU cannot dash attack"* is wrong.

⛔ **WHY IT IS NOT LANDED: IT RE-PRICES THE FIGHT, AND THE RIG SAID SO.** Measured
on the duel harness (pirate admiral, rung 9, 3613 ticks, neither match decided
early — the "a fight good enough to end fast reads as less damage" artefact was
checked first and is not what happened):

| kit | seat 0 | seat 1 | hitstun ticks |
|---|---:|---:|---|
| HEAD — mislabeled | 1.26 | 1.07 | [525, 324] |
| truthful | **0.47** | **0.86** | [81, 191] |

Damage per minute roughly halves; hitstun collapses ~85% on seat 0. Traced rather
than argued: making the dash attack reachable roughly DOUBLES the running fraction
(14–19% → 30–39% of grounded time) and cuts grounded time by a third — a feedback
loop, not "the CPU never stops running", which was measured false.

⇒ Two acceptance tests redden
(`smash_cpus_damage_each_other::two_cpus_in_the_shipped_composition_damage_each_other`
and `smash_in_the_host::launched::an_up_tilt_launches_much_further_at_a_high_percent`),
both green at HEAD, both failing reproducibly in isolation, neither flaky. The
owner document rules that a change re-pricing matchups *"needs the ladder rig, not
a coordinator's judgement"* — **two acceptance tests reporting a worse fight IS
the rig speaking**, so landing it anyway would be exactly the judgement the doc
forbids.

⚠ AND THE GATE IT FAILS IS NOT CALIBRATED ACROSS FIGHTERS: `npc_emmy_noether` at
rung 9 scores 0.28 / 0.44 at HEAD with nothing changed, which already FAILS the
same 0.5 threshold. So *"the fix fails the gate"* is weaker evidence than it
looks. ⭐ Note this question interacts with `Q116`: the duel numbers above were
taken at rung 9, where execution noise is identically zero.

## Assets, presentation and content policy

## Q69 — at `potato`, should character sprites fall back to the `0_25x` tier?

The tier-drift defect is measured separately. This asks only for the interim
quality policy while the generator/trim problem is repaired. The proposed
`dev/patches/swing-fighter-render-honours-quality-scale-20260902.patch` is a
separate renderer refusal/validation aid, not an answer to this product choice.

## Q81 — what should happen to the mostly-unreferenced bespoke FX rows for Pirate Admiral and George Booul?

The existing FX-row census found these sheets as the extreme unreferenced-art
case (historically summarized as 34 of 35 rows unnamed by production callers).
Either wire effects that correspond to intended authored moves, deliberately keep
rows as future art, or remove superseded rows. The count is owned by
`scripts/measure_fx_row_reachability.py`; do not copy a stale number into code.

## Q82 — should LDtk editor-preview tilesets remain runtime-packaged when the runtime never draws them?

Confirm whether another tool/runtime consumer needs them before excluding them
from runtime residency/packaging. A concrete proposed retarget is preserved as
`dev/patches/ldtk-player-tileset-retarget-20260902.patch`; it changes the map-assets
submodule and therefore needs an explicit content/pointer decision.

## Q83 — should the 442 MB shared sprite pack remain when one prop is the only current reader?

Choose whether this is intentional shared infrastructure or a packaging mistake
that should be split/deferred.

## Q84 — should portraits have independently authored readable low-resolution tiers?

Current generated tiers preserve existence but not necessarily readability.
Choose the product quality requirement before adding portrait-specific generation.
`dev/patches/portrait-tiers-are-never-baked-20260902.patch` is the existing proposed
implementation for the "full-resolution only" answer.

## Q85 — should Hall characters without authored interaction dialogue remain non-interactive?

Content decision: author dialogue or explicitly accept that those cast members are
visual/background only.

## Q88 — who owns the Smash CPU difficulty ladder?

Choose whether ladder tuning is Smash ruleset content, generic fighter-brain
policy, or a combination with an explicit boundary. The current engine should not
infer this from table location.

## Q90 — `read_weight` is authored on the ladder and inert: wire it or delete it?

Do not retain an authored difficulty field that never affects a decision.

## Q92 — is the BODY-PROFILE developer experiment still wanted?

If no, delete the experiment and its planning residue. If yes, name the
measurement it must produce before more implementation work.

## Q95 — fast-forward the music renderer's main and repin the superproject so fresh clones refuse General-MIDI fallback?

The refusal exists on the renderer line prepared for this purpose, but the
superproject pin at the reference state does not contain it. The safe operation
is a durable fast-forward/push of the renderer's main followed by the parent
pointer bump; do **not** repoint the parent at a deletable agent-only commit. In
the same change, flip `scripts/check_pinned_music_renderer_refuses_gm.py` from
reporting to gating.

## Q97 — a character authors a technique this composition did not install: refuse, or degrade?

**Measured 2026-09-09, by turning A11's admission pass on at the preparation
barrier and reading what it refused.** Wired as a refusal first, it reddened
seven application tests, and EVERY refusal was a `smash.*` key in a composition
that does not install `ambition_demo_smash`:

```
mary_o / mary_o_grab   / WindowSustain{1} : smash.capture_attempt
mary_o / mary_o_pummel / Event{0}         : smash.capture_pummel
mary_o / mary_o_fthrow / Event{0}         : smash.capture_throw   (+ b/u/d throws)
```

⛔ **THE MOVES ALREADY PLAY AND DO NOTHING THERE — the pass found that, it did
not cause it.** `mary_o`'s grab, pummel and four throws name Smash's capture
techniques. The capture REQUESTS are engine types in `ambition_combat`
(`CaptureAttemptRequested` and friends); only the authored-effect TRANSLATION
lives in `ambition_demo_smash::capture`. So any composition that wants grabs
without composing the Smash demo gets a silent no-op on six moves.

⚠ **AND THE TABLE CANNOT TELL THE TWO CASES APART.** `TechniqueSupport` knows
only what THIS composition declared, so a genuine typo (`smash.teleprot`) and a
technique some OTHER composition installs are both `Unknown` to it. Refusing on
`Unknown` therefore cannot be narrowed to typos without a workspace-wide key
registry, which does not exist.

⇒ Three answers, and they are not equivalent:

1. **Degrade loudly (what is implemented today).** The whole refusal list is
   logged at `error!` at STARTUP instead of one `warn!` per move mid-fight,
   which is the failure A11 exists to remove. The shipped composition is
   separately ASSERTED to have zero refusals
   (`authored_effects_are_admitted.rs`), so the guarantee is enforced where it
   can be. Costs: an unsupported move still silently does nothing at runtime.
2. **Refuse the definition.** Meets the acceptance row as written ("invalid or
   uninstalled calls cannot publish definitions") and breaks every composition
   that authors a technique it does not install — today that is `mary_o`
   outside the Smash host.
3. ~~**Fix the layering instead: make capture an ENGINE technique.**~~ **DONE
   2026-09-10 — and decided by MEASUREMENT, not by where the request types sat.**
   The review was right to refuse that inference, so ownership was taken from the
   mechanic's state and behaviour:

   * STATE — `CapturedBy`, the component holding who has whom: `ambition_combat`.
   * BEHAVIOUR — acquire, escape sampling, throw edge, hold ticking, pose,
     carries, captor control restriction, release, interruption, pummels:
     TWELVE systems, all `ambition_combat`.
   * VOCABULARY — the four keys and their param structs:
     `ambition_characters::smash_capture`, also engine-side.
   * TRANSLATION — ONE function, in the game.

   ⚠ The old module argued FOR the split — *"a ruleset knows what its own
   authored strings mean"* — and the argument does not survive its own premises:
   the strings are engine constants, and every arm is a field-for-field
   `hydrate -> write` with no ruleset policy in it. The translation moved to
   `ambition_combat::capture::systems::translate_authored_capture_effects` and is
   installed and declared by the engine composition, so `mary_o`'s six grab and
   throw moves now work wherever combat is composed rather than wherever Smash
   happens to be mounted.

⇒ **WHAT REMAINS OF Q97 IS THE POLICY HALF ONLY.** With capture fixed, no shipped
composition is known to author a technique its host did not install — so the
question is no longer "how do we repair these compositions" but the general one:
**may authored content name a capability its host did not compose, and if it
does, should the definition be refused or degraded?** Today it is REFUSED
per-definition (`ambition_characters::prepared::admit_and_finalize_cast`), which
is the literal reading of the acceptance row. Refusing the whole cast, or
degrading with a report, are the alternatives. That is a content-architecture
ruling and it is still Jon's.

⚠ Not a feel ruling: it decides whether authored content may name a capability
its host did not compose, which is the same question the SDK's minimum-profile
work (A9) asks from the other side. Owner document:
[authored technique admission](engine/authored-technique-admission.md).

## Q98 — may the Smash grid seat a character that authors no body?

**Measured 2026-09-10.** `npc_carl_stargan` is seatable on the assembled Smash
grid and, put on a stage against a copy of himself at the top difficulty rung for
a full minute, **deals 0.00 damage — three move starts, zero hitstun, zero
knockouts, in reach for 17 of 3613 ticks.**

⛔⛔⛔ **WITHDRAWN 2026-09-10 — THE PREMISE IS FALSE AND THE QUESTION DISSOLVES.
`npc_carl_stargan` IS NOT A BARE REGISTRATION.** He authors a **locomotion**, a
**600-line moveset of his own** (`carl_stargan_moveset`, `pale_blue_dot` and
all) and **`max_health = Some(4)`**, so `authors_a_body` is TRUE and
`KNOWN_BARE_REGISTRATIONS` had not been reached for him in a long time. **The
exemption was stale and its text — which carries PLACEMENT EVIDENCE, so a reader
takes it as a statement about what he authors — is what this question quoted.**

⚠ **AND THE ASSERTION MESSAGE IT CAME FROM WAS A SPECIFICATION THE PREDICATE
DOES NOT IMPLEMENT.** It read *"authors NOTHING — not a body, not a policy, NOT
A MOVESET"* while the predicate is `authors_a_body || authors_only_policy ||
exempt` and consults no moveset at all. **The third clause is the one lifted into
this question as evidence.** ⇒ A failure message is read only by people who are
already confused, which is when a false claim in it does the most damage.

✔ **Fixed at `30c15da29`:** the list is empty, the message says what it checks,
and a new arm asserts every entry is LOAD-BEARING so an exemption cannot outlive
its need again — poison-verified by putting him back, which names him.

⇒ **NOTHING IS ASKED OF THE MAINTAINER HERE. Delete this question.** The grid
seats a character who authors a body, a locomotion and a moveset; that is a
fighter, and the row it came from was measuring a rung, not a roster.

⚠ **Kept until Jon reads it because the question was ASKED, and a question that
withdraws itself is worth more than one that quietly disappears** — a maintainer
who saw it in passing should be able to find out it was wrong.

⚠ **The measurement that started this stands and is recorded in
[D-CPU-INERT](queue.md): he fights at lower rungs.** The 0.00 above is a **rung-9** reading. Swept across every
published rung, the same character against a copy of himself:

| rung | starts | damage |
|---|---|---|
| 1 | — | separates |
| 3 | 20 / 20 | **55 / 62** |
| 5 | 7 / 7 | 0 / 0 |
| 6 | 34 / 40 | **44 / 66** |
| 9 | 3 / 3 | 0 / 0 |

⇒ **A character the catalog says authors NOTHING — not a body, not a policy, not
a moveset — starts 34 moves and deals 66 damage at rung 6.** So "he is not a
fighter" was true of the rung it was measured at and false of the roster. **Do
not rule on the 0.00.**

⚠ **AND THAT MAKES A SECOND QUESTION THE FIRST ONE HID: what is he fighting
WITH?** Something furnishes a body, a policy and a moveset to an id whose only
registration is the exemption list for ids that have none. **Until that is
answered, options 2 and 3 below are not costed** — "drop bodiless characters"
does not describe him if he is not bodiless, and "author him a body" may be
duplicating one he already receives from somewhere. ⇒ **This is a measurement
somebody owes before the decision, not part of the decision.**

⭐ It is the same species as the defect fixed at `f77ba3a45`, where a duel seat
silently ran a different AI backend from the one the match assigned: **the
composed thing is not what the declaration says**, and a census over declarations
cannot see it.

⇒ **He is not a broken fighter; he is not a fighter** *at rung 9*.
`character_catalog.rs`
lists him in exactly one place — `KNOWN_BARE_REGISTRATIONS`, the exemption for
ids that author *"NOTHING — not a body, not a policy, not a moveset"* — and the
entry records the reason verbatim: *"one placement: hall_of_characters NpcSpawn,
brain_override stand_still. Never an EnemySpawn, so no archetype vitals exist to
retract. **Registered because Jon put him on the Smash grid (2026-08-11) and the
grid drops what it cannot seat.**"*

⚠ So this is a decision that was already made once, deliberately, by the
maintainer — which is exactly why it is a question here rather than a defect in
`queue.md`. He is not on `PLAYABLE_ROSTER` (13 ids, six of them `npc_` fighters);
he reaches the grid by a different road.

⇒ Three answers, and they are not equivalent:

1. **Keep him seatable.** A player can pick a character who cannot fight. That is
   a real product statement if the grid is meant to be "everyone we can draw",
   and it costs nothing to leave.
2. **Drop bodiless characters from the grid.** The grid's rule becomes "a
   fighter", not "an id we can seat". ⛔ This RETRACTS a placement Jon made on
   purpose, so it needs saying out loud rather than being fixed quietly.
3. **Author him a body.** He becomes a fighter and the question dissolves — the
   most work and the only answer that makes the grid entry mean what a player
   would assume.

⚠ **AND THE ENGINEERING HALF DOES NOT WAIT ON THIS RULING**, so it is not
blocking: the duel gate's own assertion checks that an id can be SEATED, not that
it is an authored fighter, and that is what let a bodiless character into a
fighter measurement. Tightening the SWEEP's population is queue work
(D-CPU-INERT) whichever way this is answered.

⭐ Not a feel ruling: it decides whether "on the grid" means "is a fighter", which
is the property every CPU-quality measurement over that grid will assume.

## Q102 — a solid breakable is published as `BlinkWall { Hard }`: is that the representation, or a borrow?

**Flagged 2026-09-10 in the same review that ruled Q96, and it is a DO-NOT-MAKE-IT-
WORSE constraint rather than a repair request.** Verbatim:

> *"Representing a generic solid breakable as `BlockKind::BlinkWall { Hard }` mixes
> ordinary solidity with blink-specific permeability semantics. That may be
> justified by the current world vocabulary, but **projectile support should not
> cement that representation into more systems.** Ideally consumers ask shared
> collision semantics rather than learning that 'hard blink wall happens to mean
> solid breakable'."*

**Measured at HEAD 2026-09-10.** `world/overlay.rs:52-58` maps
`BreakableCollision::OneWayUp` → `BlockKind::OneWay`, which is an honest match, and
`BreakableCollision::Solid` → `BlockKind::BlinkWall { tier: Hard }`, which is a
borrow.

⭐ **WHY THE BORROW WORKS TODAY, AND EXACTLY WHEN IT STOPS.**
`ambition_platformer2d_core/src/world.rs` documents `BlockKind::Solid` as *"full
collision on both axes, and also a hard blocker for blink pathing"*, and
`BlinkWallTier::Hard` as *"intended to remain blocked until a stronger
blink-phasing upgrade."* ⇒ **The two are behaviourally identical for blink pathing
only while no stronger blink upgrade exists.** The day one is added,
`BlinkWall { Hard }` becomes permeable to it and `Solid` does not — and **every
solid breakable in the game silently becomes blink-passable.** That is the whole
debt, and it is a one-feature fuse.

⚠ **CONSUMERS ARE ALREADY LEARNING THE COINCIDENCE.** `BlockKind` carries exactly
ONE predicate — `is_pogo_target()` — so every other consumer enumerates variants.
`shared_tangle/src/projectile/collision.rs` matches `Solid | BlinkWall { .. }` at
four sites, and `features/ecs/perception.rs:802` maps `BlinkWall { .. }` to
`SolidKind::BlinkWall`.

⇒ **THE CONSTRAINT ON Q96's IMPLEMENTATION:** when the compound-solid row is built,
the projectile road must ask for **collision semantics**, not for `BlinkWall`. The
cheap shape is a predicate on `BlockKind` beside `is_pogo_target()`; adding a fifth
`Solid | BlinkWall { .. }` arm makes the eventual repair more expensive.

⛔ **AND NAME THE PREDICATE FOR THE SEMANTICS, NEVER FOR ITS CURRENT MEMBERS.** Ask
*does this block present a solid surface to a projectile* — not
*is_solid_or_blink_wall*. **A predicate named after its members is a match arm
wearing a function's clothes:** it moves the enumeration without removing it, and
the next variant still has to be added in every caller's head. ⇒ Named for the
semantics, this is also **the cheapest repair path for the debt above** — once
consumers ask the predicate, fixing the representation touches the predicate and
not the consumers.

**The question for Jon** is only the eventual one: should a solid breakable get its
own `BlockKind`, or is the borrow the intended vocabulary? ⛔ Nothing is blocked on
the answer — Q96's work proceeds either way, under the constraint above.

## Q115 — which per-move `hitbox.inflate` values should the 97 untuned bone-derived specs carry?

⛔ **THIS IS THE ONE JON ASKED FOR BY NAME AND IT HAS BEEN MEASURED TO ITS
DECISION POINT SINCE 2026-09-11, WITHOUT BEING IN THIS FILE.** The engineering is
finished; what remains is authoring values, which is not mine to choose — Jon,
2026-09-10: *"I don't trust your spatial decision making at the moment."*

WHAT IS SETTLED (all in `queue.md`'s moveset/hitbox section, with the scripts):
· **WHERE** the values go — `.spec.json`'s `hitbox.inflate` / `hitbox.per_frame`,
  not the move table. A roster-wide multiplier is REVERTED and cannot come back:
  `FrameToBody::point` scales the DISPLACEMENT FROM THE FEET PIXEL, so on the
  performer's forward tilt a 1.25 knob scaled the half-extent 1.25x **and moved
  the centre 9.4 px up and 4.6 px forward, off the drawn blade.** No value avoids
  that.
· **WHICH moves — AND THE UNIT MATTERS, BECAUSE THE QUEUE ROW CARRIES THREE
  COUNTS FOR THIS ONE POPULATION.** The number with a committed instrument behind
  it is `scripts/measure_hitbox_authoring_coverage.py` over a complete grid take:
  **97 recorded MOVES are bone-derived AND carry no `inflate`, and 64 of them play
  thin** — recorded (character, move) pairs, so one `.spec.json` shared by several
  characters is counted once per character. Each is listed with the exact
  `.spec.json` that decides it
  (thinnest: `npc_carl_stargan air_neutral` 0.06, `smash_down` 0.07, `medic
  air_up` 0.08, `medic tilt_forward` 0.09, `officer jab` 0.10).
· **WHAT "apply what GPT-6 did" MEANS AS A NUMBER** — coverage predicts thinness:
  `performer_stage_v1` has 15 of 19 specs inflated and is the best bone-derived
  stage at 29% thin; `medic_triage_v1` has 5 of 18 and is 10/13 thin; and
  `fighting_brawler_v1`, `officer_brawler_v1` and `projectile_beast_v1` have **0
  of 14, 0 of 15 and 0 of 17** between them, at 41–93% thin.

⛔⛤ **AND TWO OTHER SPEC-LEVEL COUNTS IN THAT ROW DISAGREE WITH EACH OTHER, WHICH
IS WORTH KNOWING BEFORE ANY OF THEM IS USED TO SIZE THE JOB.** The row's own
per-stage table implies **63** uninflated bone-derived specs (19−15, 18−5, 14−0,
15−0, 17−0 = 4+13+14+15+17), while its prose says **46 bone-derived specs with no
inflate at all**. Neither is the 97, which is a different granularity and fine.
⇒ **The 46 agrees with nothing and is WITHDRAWN pending a re-run**; the table is
self-consistent and 63 is derivable from it; the 97/64 is the one with a script.
Do not price the work off the 46. (Not re-run here: the coverage join needs a
complete grid take, which is a machine measurement and belongs on the authoring
box.)

⚠ RE-DERIVE ON THE AUTHORING MACHINE BEFORE TUNING, and say which publish state a
number was taken under: published assets are gitignored and DERIVED, so reverting
a spec with git does NOT undo a publish. An abandoned `inflate: 40` probe stayed
live in the officer's sheet and its numbers were reported as an unexplained 4.3x
anomaly.
⚠ AND DO NOT COMPARE AGAINST `scripts/measure_authored_strike_extents.py`: it
reads `half_extents` from the move tables, the value the game IGNORES for any
character with a sprite stage. Its rows are still right for the six without one.

**Two other moveset items are also waiting on a ruling rather than on work**, and
both have numbers: `attack_air_back` connects by 0.2 px where the forward air has
17.8 to spare (~38 px of reach behind her against the 48 px the forward takes ask
for); and the authored clock is not Ultimate-shaped in ONE axis — startup is right
(tilts ~5 f, smashes ~12 at 60 fps equivalent) but ACTIVE runs 10–17 f against
Ultimate's usual 2–5, with SHORTER totals, so generous boxes plus fast recovery
makes her moves very safe.

## Human measurements, not design answers

These are recorded here only when the maintainer must supply the measurement; the
engineering follow-up belongs in `queue.md`.

- **D-RASTER-3:** weak-GPU framebuffer/source-tier comparison.
- **Switch Pro outer range:** radial maxima/dead-zone measurement on both machines.

## Q99 — D-BRAIN-MENU is held by two red tests and neither is now evidence about the fighter brain: drop the hold?

**The change.** `truthful_attack_kit` makes the CPU's attack kit resolve a press
the way the PRESS ROAD does (`move_for_attack(.., running)`) instead of its
stance-blind fallback. Today the brain scores `jab`'s frame data while the body
performs `{base}_dash`: **eighteen of eighteen shipped fighters author a dash
attack no press in the kit reaches**, and every scoring term downstream — startup,
reach, damage, frame advantage — reads a different move than the one the press
produces. See [D-BRAIN-MENU](queue.md) for the full receipt.

**It is held because two acceptance tests redden. Both were re-measured
2026-09-10 and neither now says anything about the fighter brain.**

⛔ **RED 1 is a threshold whose calibration subject moved underneath it.** The
gate is 0.50 damage per pool-minute; the fix reads 0.47 on `npc_pirate_admiral`.
⇒ But that fighter's own HEAD baseline moved **1.26 → 0.84** when `f77ba3a45`
stopped a dismount replacing his opponent's brain — **his row was a fighter
beating a brute** — and a second shipped fighter, `medic`, **fails the same gate
at 0.41 with nothing changed at all.** On `medic`, the clean subject with no
mount and no confound, the fix is an IMPROVEMENT: 0.41 → **0.45** damage, hitstun
118 → **132**.

⛔ **RED 2 is a fixture whose own strike does not land.** Its message reads *"the
percent meter is not reaching the launch"*. Measured across the strike:

| up-tilt fixture | feature OFF | feature ON |
|---|---|---|
| percent = 0 | meter **0 → 10**, rose 3.4 px | meter **0 → 0**, rose 26.6 px |
| percent = 1427 | meter **1427 → 1437**, rose 372.8 px | meter **1427 → 1427**, rose 26.6 px |

The shipped arm scales 110×, so the meter is fine; under the feature the victim's
meter does not move by a point. **Every knockback input in that fixture is a
literal in its own file, so the launch cannot stop scaling except by the victim
not taking the hit.** ⚠ That fixture shares its app with live CPU fighters whose
walking it does not control — **it is a gate on "the CPUs still walk the way they
did in August", and this change is a change to how CPUs walk.** It now asserts its
own strike landed, so it cannot make this accusation a fourth time.

⇒ **THE QUESTION IS NOT WHETHER THE EVIDENCE HOLDS — IT DOES NOT. It is whether
a hold is dropped on that basis by an agent.** Two answers:

1. **Drop the hold and land the fix**, re-deriving the 0.50 gate against the
   roster it is meant to police — it does not hold across fighters at HEAD, which
   is a defect in the gate whatever happens to this row.
2. **Keep the hold** and require a positive result — a measurement showing the
   truthful kit makes CPUs fight BETTER on a stated population — before landing.
   ⚠ The three subjects measured at rung 9 say improvement, no-op, and mixed; the
   row's owner document says re-pricing matchups *"needs the ladder rig, not a
   coordinator's judgement"*, and the ladder rig cannot seat Ambition's authored
   movesets at all.

⚠ **Recorded as a question rather than acted on because dropping a hold is a
maintainer's call even when the evidence for it has evaporated.** Nothing is
blocked meanwhile: the fix is behind a default-off feature and the shipped
behaviour is today's by construction.

## Q100 — should the facade pull `bevy/debug` because it always links `ambition_dev_tools`?

**Measured 2026-09-10 (`86c95490d`), and the remedy is a ruling rather than a
repair.** A9's axis two asks which linked crates install anything in the minimum
profile. All **557 systems report `<Enable the debug feature to see the name>`**,
so the census cannot attribute one.

⛔ **THE DATA DOES NOT EXIST, WHICH IS STRONGER THAN "HARD".**
`bevy_utils::DebugName` has **no field** without `bevy_utils/debug`; the string is
discarded at construction and no runtime accessor can recover it. `TypeId`
survives and carries no crate name.

⚠ **The inconsistency is what makes it a question.** `bevy_dev_tools` requires
`bevy_utils/debug`, and the minimum profile does not link `bevy_dev_tools` — the
workspace has the feature, the featureless facade does not. **But
`ambition_dev_tools` IS in the minimum profile**: facade-direct, not optional, so
a consumer selecting nothing links it, and its census attributes systems by name.

⇒ **A mandatory diagnostics crate sits in a profile that cannot supply the one
fact it reads.** That is vision.md's *"machine-readable diagnostics and
provenance"* failing at the profile the SDK offers.

**The trade is binary size against diagnosability.** ⛔ Adding a feature to the
minimum-profile fixture is not an option: its whole value is that everything it
links arrives implicitly.

⚠ **This ruling blocks A9's axis two and nothing else.** Axis one is closed —
false optionals are 0 of 13, and the non-optional closure is 48 excluding the
facade at `939d6aaa5`.

⭐ **The number was measured against `origin/main` at `58cc6c9a8`: Q99 was the
highest filed, in both that ref and this worktree.** If another agent claims Q100
concurrently, **renumber this one** — it was filed by the coordinator, not by the
agent who did the measurement, and it is the cheaper of the two to move.

## Q101 — may an ability's own contact satisfy the launching move's `Connected`?

`verdict_belongs_to` admits `None`, and `None` currently means *"credit whatever
move this body is playing NOW"*. ⭐ **The predicate's own census names every
production site that writes `attacker_move_instance: None` AND REACHES IT** —
`features/empowerment.rs`, `features/ecs/actors/update.rs:964`,
`features/enemies/integration.rs`, and `abilities/traversal/{blink,dive,mark_recall}.rs`.

⚠ **These are ABILITY AND CONTACT roads. Not hazards, not the blast zone.** The
comment's older defence of `None` named four legitimate producers; measured, only
one of the four reaches this predicate, *"and both of them were used to size a
change as too large to make."*

**The question.** For `blink`, `dive`, `mark_recall` and empowerment contact harm,
the player triggered the ability, possibly as part of a move.

- **If that contact SHOULD satisfy the launching move's `Connected` condition**,
  the occurrence must be propagated through those roads, exactly as `f9baa86e8`
  now does for projectiles.
- **If they are INDEPENDENT effects**, they must not mutate a move-local latch,
  and `None` must stop meaning *"credit the current move"*.

⛔ **It is not a bug fix either way.** *"Not attributable to this move"* may be
FALSE for blink and dive, so refusing `None` globally would silently stop
crediting contacts a player would expect to count.

⚠ **A12's reflection blocker cannot be closed without this answer.** Reflection
re-owns a shot and keeps the shooter's stamp; clearing the stamp yields `None`,
which credits whoever is playing — the same defect with an extra step — and
re-stamping is only correct if the reflection is itself move-authored, which in
general it is not. **Every road out passes through this ruling.**

Measured by ToothbrushAmbition; census verified in the predicate's own comment.
⭐ Q99 was the highest filed when this was written, in `origin/main` and in the
worktree, read in one command. **If another agent claims Q101 concurrently, this
one renumbers.**

## Q106 — are `ambition_items` and `ambition_encounter` optional capability edges of the facade, or not?

**A comment says one thing and four measurements say the other.**
`crates/ambition_platformer2d/Cargo.toml:20` reads *"`ambition_items?/content_pack`:
the `?` means only if that optional edge is already enabled. A movement-only game
with `default-features = false` gets the compiler without being forced to link
the items capability to have it."*

Measured at `9604a3649`:

* the entries that comment annotates — `:27` `"ambition_items/content_pack"` and
  `:30` `"ambition_encounter/content_pack"` — carry **no `?`**;
* neither dependency is declared `optional = true` (`:193`, `:208`);
* neither appears in `all_capabilities`, the list the manifest says names every
  optional crate edge;
* and `cargo tree -p ambition_platformer2d --no-default-features --features
  content_pack` **links both** — 367 crates, items and encounter among them.

⭐ **The manifest uses `?/` correctly four times elsewhere** (`:102`, `:112`,
`:125`, `:128`), so this is not a syntax error. Somebody meant the design and it
is not implemented.

**The question.** Either:

* **they ARE optional capability edges** — then the `?`, `optional = true` and the
  `all_capabilities` entries are missing, and the payoff is that a movement-only
  game can take the content compiler without the items and encounter
  capabilities; or
* **they are NOT** — then the comment describes a design that was considered and
  declined, and it should be deleted rather than left describing behaviour the
  build does not have.

⛔ **It is a capability-edge ruling, not a repair.** Slice H's stance is that the
facade's edges are optional capabilities and the default is all of them; whether
`item_catalog` and `encounter_waves` belong in that set is a product question
about which games may exist without them — `engine_schemas()` registers both
unconditionally today, and a composition that must not claim to own a schema it
cannot install is the reason the `?` was wanted in the first place.

⚠ **Nothing is blocked meanwhile.** The shipped behaviour is today's by
construction, and the schema list that reads these edges was collapsed to one
place at `9604a3649` without needing the answer.

Found while collapsing `engine_schemas()` and `ambition_content_cli::default_registry()`.
⭐ **Q102 was the highest filed in `origin/main` at `9604a3649`, read in the same
command that wrote this.** ⚠ Q103–Q105 are claimed concurrently by
ToothbrushAmbition and are not yet in this ref, which is why this row starts at
106 and why a reader seeing a gap should look for their push rather than assume a
retired number. **If those three land differently, this one renumbers.**
## Q105 — may an author place a chest that is already open?

⭐⭐ **STILL OPEN AS A DESIGN QUESTION, BUT ITS BROKEN HALF-IMPLEMENTATION IS GONE
(2026-09-12) AND THE QUESTION IS NOW CHEAPER TO ANSWER EITHER WAY.** The two
options below were *"derive the marker from the authored state"* and *"delete the
unreachable variants and the write-only field"*. **The deletion was taken**:
`ChestSpec.state`, `ChestStateSpec` and `Chest::state` are removed. <!-- cite-ok: the DELETED chest-state vocabulary, named on purpose. These rows are the CENSUS that justified removing it (2026-09-12); a resolvable citation here would mean the deletion did not happen. See Q105. -->

⛔ **THAT IS NOT A RULING ON THE DESIGN QUESTION, AND IT MUST NOT BE READ AS ONE.**
What was removed was never a capability: `Opening`/`Opened` were unreachable from
every authoring surface, so **no author could place an open chest before the
deletion either.** What the field did do was LIE — it recorded an "is it open"
fact that the runtime, which gates on the `Opened` marker, never consulted, so an
authored `Opened` chest would have granted its reward TWICE the day anything
could express one.

⇒ **The choice made was between two ways of NOT shipping the feature:** leave a
lying field in place while the question waits, or remove it so the question waits
against a truthful type. ⚠ **If the answer is YES, an author may** — the work is
unchanged in size and clearer in shape: add the authored field **and its
lowering to the `Opened` marker TOGETHER**, in one change. A field without a
lowering is precisely the trap that was removed, and re-adding one is a
deliberate act rather than an accident.

⇒ **What Jon still decides:** whether an author may place an already-opened
chest at all, and — if so — what `Opening` would even mean, since it names a
transient animation state rather than a persistent one. That ambiguity is
untouched by the deletion.

**The original finding, kept because it is the evidence:**

**`Chest::state` is WRITE-ONLY.** Measured 2026-09-11 during A5's census:
`ChestState { Closed, Opening, Opened }` is constructed, mapped from
`ChestStateSpec` at authored spawn, and serialized — and read by NOTHING in <!-- cite-ok: the DELETED chest-state vocabulary, named on purpose. These rows are the CENSUS that justified removing it (2026-09-12); a resolvable citation here would mean the deletion did not happen. See Q105. -->
production. The one read in the repository is an assertion inside
`ambition_interaction`'s own test module. The runtime's open-gate is the `Opened`
MARKER component (five production sites, three crates), and **nothing derives the
marker from the authored state.**

⚠ **LATENT, NOT LIVE, and the population was checked before saying so.** LDtk's
`ChestSpawn` declares exactly two fields — `name` and `reward` — in all four
shipped worlds; `ChestSpec::new` defaults to `Closed`; and no converter anywhere
populates `ChestStateSpec`. `Opening` and `Opened` are unreachable from content, <!-- cite-ok: the DELETED chest-state vocabulary, named on purpose. These rows are the CENSUS that justified removing it (2026-09-12); a resolvable citation here would mean the deletion did not happen. See Q105. -->
so two of three spec variants are dead and both non-`Closed` arms of
`chest_state_from_spec` are dead with them. <!-- cite-ok: the DELETED chest-state vocabulary, named on purpose. These rows are the CENSUS that justified removing it (2026-09-12); a resolvable citation here would mean the deletion did not happen. See Q105. -->

**The question.**

- **An author MAY place an opened chest**: derive the `Opened` marker from the
  authored state at spawn. ⛔ Without that derivation a chest authored `Opened`
  would be opened again and grant its reward TWICE — A5's own acceptance line is
  *"no duplicate effects/rewards"*.
- **An author MAY NOT**: delete `ChestStateSpec`'s two unreachable variants and <!-- cite-ok: the DELETED chest-state vocabulary, named on purpose. These rows are the CENSUS that justified removing it (2026-09-12); a resolvable citation here would mean the deletion did not happen. See Q105. -->
  the write-only `Chest::state` field with them. ⇐ **THE FIELD REMOVAL HALF OF
  THIS WAS TAKEN 2026-09-12** (the whole field, not two variants); see the header
  above for why that is not the same as ruling the question.

⚠ Content-design, not ownership. A5 took no type and proposes none; found by
YardratAmbition, recorded on
[`destructible-writer-inventory.md`](engine/destructible-writer-inventory.md).

## Q103 — what should an UNPREPARED character id inherit at wear time?

**A6's last open item, and it is design rather than cleanup.** The character
barrier FOLDS `movement_tuning` and `motion_model` into the prepared registry
(`crates/ambition_characters/src/prepared.rs:1338`, `:1334`), so the registry is the
catalog's fold and cannot disagree with it. The same fold is ALSO spelled at the
READ site, in `avatar/starting_character.rs`.

**Measured 2026-09-11, all five compositions: ZERO orphans.** Shipped host
147 catalog rows / 58 prepared, mary_o 7/7, twintrack 2/2, sanic 3/3, smash 3/3 —
and in the four demos the read-time fold is never reached at all. In the shipped
host the read-site path runs for 89 ids per boot and the catalog authors a value
for **none** of them, so it returns the default every time.

⛔ **DELETING THE SECOND SPELLING WAS TRIED AND REVERTED.** It reddened SIX tests
across three files, all on the wear/re-wear road the fold serves, one of them
asserting the deleted behaviour outright.

**The question.** When a body WEARS a character id the barrier never prepared:

- **Inherit the catalog's authored tuning at wear time** — keep the read-site
  fold, and the duplication is a deliberate restatement with a stated reason.
- **Inherit the engine default** — delete the read-site fold, and the six tests
  are re-authored to say so.
- **Refuse the wear** — an unprepared id is a construction failure, and the
  read-site fold becomes unreachable rather than redundant.

⚠ Not urgent: `game/ambition_app/tests/authored_feel_reaches_the_prepared_cast.rs`
(poison-verified) keeps the orphan case from arising meanwhile. It could not have
caught the six; the in-crate suite did.

## Q104 — is the Rust move table or the content file the SOURCE of a moveset?

**All seventeen tables — nineteen characters — are content as of 2026-09-11:**
declared in `pack.ron`, validated by the `moveset` schema, applied at
`character_catalog::authored_intrinsics`, and **not compiled into the host**. A
move-timing edit costs 0.61 s against 6.30 s through the Rust table
(`dev/measurements/m0_move_edit_loop.sh`).

The nineteen `game/ambition_content/src/*_moveset.rs` files still exist — roughly
15k lines with their own in-file tests. They are the EXPORTER's input and the
parity oracle's subject, and nothing the game runs reads them: fast-iteration I2
step 5's own allowance (*"a test-only old table may be a temporary parity oracle,
not a runtime fallback"*).

**The question.**

- **The Rust stays the source**: the `.ron` is build output, hand edits to it are
  overwritten by the next export, and per-character hitbox tuning stays a Rust
  edit with a rebuild. The parity oracle is permanent.
- **The RON becomes the source**: the nineteen Rust tables are deleted and
  per-character tuning becomes a content edit with no rebuild — which is what
  *"attacks rarely ever feel like they connect"* needs most.

⛔⛤ **AND THE OBVIOUS ARGUMENT AGAINST DELETING THEM DOES NOT SURVIVE ITS OWN
EVIDENCE.** This row first said the in-file tests *"caught the medic's dead `jab`
target and the oni leader's dead `special_forward` one"*, so deleting them has a
cost. Checked, by a second reader who did not write the migration:
* the medic's `jab` was caught by
  `every_cancel_target_resolves_and_a_confirm_is_authored` — a CROSS-TABLE guard
  in `authored_movesets.rs` that consumes `tables()`, so it is repointable at
  parsed content and does not die with them;
* the oni leader's was caught by the content-pack validator, which is the thing
  that REPLACES them. That same cross-table guard was blind to it.
⇒ Neither catch is evidence for the per-file tests.

⭐⭐ **THE REAL TRADE IS ADJACENCY, AND IT IS A SMALLER QUESTION.** MEASURED over
the twenty files: **18,176 lines — 11,015 authoring and 7,161 inside
`#[cfg(test)]` (39%)**; 120 test functions across the nineteen character tables,
of which **22 (18%) are cross-fighter RELATIONS** — `alice_reaches_further_than_bob_and_bob_hits_harder`,
`bob_is_slower_to_start_than_alice_on_every_shared_press`,
`his_reach_spans_further_than_anybody_elses`. A schema validator checks
STRUCTURE: does the field exist, does the id resolve. **It cannot express "alice
reaches further than bob"** — design intent written as numeric relations over
authored data.
⇒ But those tests need the PARSED DATA, not the Rust literals: they already
consume `tables()`, exactly as the cross-table guards do. So the choice is not
*"delete 15k lines including their tests"*; it is **"re-home 120 tests, 22 of
them relations, and accept that the other 98 lose the file they sat next to."**
The thing that genuinely dies is a test being beside the numbers it constrains,
where an author editing a move sees it.
⚠ That 18% is a SECOND number. The first matcher substring-matched fighter names
and said 67%, because `author`, `archetype`, `medic`, `officer`, `performer` and
`goblin` are also ordinary English words in this corpus's prose — the same
"a name that exists in another vocabulary answers YES" failure as the migration's
own eighth drift. The 18% requires a CODE reference.
- **Split by family**: the values are content, the STRUCTURE stays Rust.

⚠ This is a call about where authoring lives, not a refactor. The generated file
carries a do-not-hand-edit banner meanwhile, so the trap is at least visible.

## Q107 — do sprite `active` frames own contact timing, or does moveset authoring?

**Two independently authored clocks, pinned together by a test.** A move's Active
window is authored in the move table (`start_s`/`end_s`). The sprite library also
declares a per-frame `active` list and a frame duration, and
`normal_contact_windows_match_the_authored_light_and_pose_clock` asserts the two
agree. ⇒ That is SYNCHRONISATION, not ownership, and
[`queue.md`'s `D-STRIKE-GENEROSITY`](queue.md) records that the moveset side
HARDCODES the frame counts rather than reading the library.

**The question.** Either:

* **the sprite `active` frames own contact timing** — then the runtime must publish
  that list as a semantic fact (`AnimationMetrics` does not today; it exposes frame
  duration and geometry, and empty per-frame geometry cannot be read as "inactive"
  because the sampler falls back to the coarse polygon), and move windows derive
  from it; or
* **moveset authoring owns it** — then the sprite `active` metadata is an
  art-generation concern and the pinning test is the boundary, which is what it is
  doing today by accident rather than by decision.

⛔ **MAINTAINING BOTH IS WHAT EXISTS**, and generalising the current shape to every
fighter would multiply a second hardcoded combat clock that merely resembles the art
metadata.

⭐ **WHY IT IS BEING ASKED NOW.** Censused 2026-09-11 over 343 moves: the performer
authors FOUR consecutive Active windows on her forward tilt where the other
seventeen entities author one, at an identical 27×14 volume — 9.6 live frames
against their 2.4–6.6. **If the SHEET owns contact timing, those four windows are a
workaround for a sheet and a table that disagree**; if the TABLE owns it, they are
authoring and the question is only whether the value is wanted. The measurement
cannot tell those apart, and neither can anyone else until this is ruled.

⚠ Nothing is blocked meanwhile: the pinning test keeps the two in step, and the
census is a file read now rather than an app boot.

⭐ **Q106 was the highest filed in `origin/main` when this was written, read in the
same command** — Q103–Q105 had landed by then, so the gap Q106 warned about is
closed and that row's note can go when somebody next touches it.

## Q108 — which capabilities may a featureless `ambition_platformer2d` link?

**38 crates sit between the stated minimum and what it links today.** MEASURED
2026-09-11 by `scripts/measure_minimum_profile_closure.py` (the feature-resolved
tree, the same flags `the-featureless-facade-links-none-of-these` uses):

| | crates |
| --- | --- |
| featureless `ambition_platformer2d` | **49** |
| the minimum A9 states, priced — `core` + `shared_tangle` + `world` + `time` + `input` | **11** |

A9's own words are *"a constructed body advancing against world geometry, without
renderer, audio, inventory, encounters or game content; this is a target, not a
passing current profile."* That target prices at 11. The 38 in between are
`abilities, audio, body_seed, boss_encounter, character_sprites, characters,
combat, conversation, cutscene, damage, dev_tools, dialog, encounter,
encounter_features, game_shell, gameplay_trace, held_items, interaction, items,
load, load_presentation, match, mount, persistence, platformer2d_actor_monolith,
platformer2d_actor_spawn, platformer2d_host, platformer2d_provider,
platformer2d_runtime, projectiles, sfx, sfx_bank, sim_view, sprite_sheet, ui_nav,
vfx, world_items` (plus the facade).

⛔ **THE DECISION IS A CAPABILITY LIST, NOT A CRATE LIST**, and it cannot be made
edge by edge: each crate is held by TWO roads at once — the facade names 42 of
the 48 directly, and the hubs (`provider` 47, `host` 45, `runtime` 44, `sim_view`
41, `actor_monolith` 38) reach the same 42 from underneath. Cutting either road
alone measures ZERO. Cutting `provider` + `host` + `runtime` + `actor_monolith` +
`sim_view` together takes 49 to 44. So every capability costs a PAIR of edits —
a facade feature gate and the matching hub gate — and a partial one is invisible.

⚠ Nothing is blocked meanwhile. Run
`python3 scripts/measure_minimum_profile_closure.py --minimum <crates...>` to
price any capability set before ruling; it answers in seconds.

## Q109 — should a simulated identity be able to name its room instance?

**MEASURED 2026-09-11 by `scripts/measure_identity_instance_scope.py`: zero of
ten `SimId` constructors take a room or an instance.** `SimId::placement(id)` is
`"placement:{id}"` — the map's own iid and nothing else — so two instances of one
prepared room mint the same identity for every authored placement.
`SimId::encounter` has the same shape; four more (`spawned`, `death_drop`,
`strike_volume`, `geometry`) inherit whatever scope their parent had, which is
none.

⭐ **IT REFUSES RATHER THAN CORRUPTS TODAY.** A second instance hits the
construction planner's `IdentityAlreadyLive`, so the current failure is loud.
Nothing is broken; what is unavailable is the capability A8 and open-world
residency are written against.

**The narrower half, and the one that can be decided on its own:**
`GeoSource::TileLayer { layer: String }` is scoped by a STRING CONVENTION —
`ldtk/intgrid.rs` passes `"{level}/{layer}"`, *"because an active area can span
multiple levels that each carry this layer"*. The type cannot hold anyone to it,
and MEASURED, one production site already does not: `ambition_demo_sanic` spells
`"sanic_speedway_ground"` bare, and `Block::solid_tiled` / `one_way_tiled` take
whatever `layer` a caller passes. Latent, because that demo has one level.

⇒ **Making the level a FIELD would make the bare key unconstructible** — the
make-it-impossible shape rather than a rule about a string. ⚠ It is not free:
`SimId::geometry` derives a persisted identity string from `GeoSource`, so the
format must stay `tile/{level}/{layer}` for the LDtk road (it already would) and
`ambition_demo_sanic`'s ids would change.

**What is being asked:**
1. Should identity carry instance scope at all, or should two live instances of
   one room stay a refusal? A8 assumes the former; nothing has ruled it.
2. Independently: should `TileLayer` carry the level as a field now, ahead of
   that ruling, purely to stop the convention being unenforceable?

⚠ Nothing is blocked meanwhile, and the census is the deliverable A8 was held
for. Re-run the script after any identity change; it prices the answer in
milliseconds.

## Maintenance rule

Do not add investigation transcripts beneath a question. Record enough source
context to make the decision, link the owner doc when useful, and stop. Once
answered, move the durable ruling to `maintainer-decisions.md` and delete the
question here.

</details>

## Q118 — HALF SEALED 2026-09-13. The interval is REACHABLE IN THE SHIPPED GAME, and one half of it cannot be sealed by refusing.

⛔⛤ **THE QUESTION IN THE TITLE IS ANSWERED: IT IS ARCHITECTURE.** The shipped
lifecycle produces the transition on every reload, measured rather than argued.
Implementing the cancel-based seal and running the production arm
(`an_edit_reaches_the_shipped_game::an_edited_pack_reaches_the_cast_the_shipped_composition_plays`)
failed with *"the shipped shell never re-activated the route"*, and instrumenting
the lease gave the reason:

```text
[probe] lease boundary=LiveTimeline authority_present=true owner=SessionScopeId(0)
```

⇒ A reload re-prepares the route the shell is ALREADY ON, so by the time the
transaction reaches its boundary the session it is REPLACING owns a healthy,
speculating GGRS timeline. **Every reload in the shipped game publishes across
one.**

✅ **THE UNHEALTHY HALF IS SEALED NOW** —
`break_the_publication_lease_when_the_boundary_closes` cancels the whole shell
transaction (`ShellCommand::CancelPending`, which names a REQUEST and refuses a
stranger's). An unhealthy authority is a RECORDED DIVERGENCE, and publishing
across one launders a desync, which is wrong under every model below. It does not
wait for the ruling.

⛔ **THE LIVE-TIMELINE HALF CANNOT BE SEALED BY REFUSING, AND THAT IS THE RULING
THIS ROW NOW WANTS.** Cancelling there does not seal anything — it deletes hot
reload from the shipped game. ⇒ Of the two directions this row already named,
the first (an authorization broken early) is MEASURED INSUFFICIENT for this half,
and the second is what remains: **a lifecycle that STOPS AND REBASES rollback as
part of the same transaction.** That is a real packet and it is not a small one.

⚠ **WHAT IS NOT KNOWN AND SHOULD NOT BE GUESSED:** whether publishing across a
healthy speculating timeline is actually harmful in the shipped single-player
composition, or only in a networked one. `Q120` is the same question from the
developer-edit side and is also unruled. If the answer is *"harmless locally,
fatal networked"*, the cheap correct move may be to refuse the reload only in a
NETWORK-compatible session rather than to build the rebase lifecycle.

<details><summary>The original row, whose measurements all still stand</summary>

### Q118 — is the rollback-legality interval ARCHITECTURE, or is it unreachable in the shipped lifecycle?

⛔ **NOT A TASTE QUESTION AND PROBABLY NOT JON'S — recorded here because the
ANSWER decides whether an implementation packet exists, and the measurement that
decides it is bounded.**

**MEASURED 2026-09-12, and the structural gap is real.** `admit_candidate` asks
`publication_boundary(world)` and refuses a live rollback timeline or an
unhealthy authority. The generation then spends time in `PendingGeneration` —
through shell preparation to `RouteActivated` — and `commit_content_generation`
asks NOTHING, deliberately: *"there is nothing in it that can say no."* ⇒ The
implementation carries an unstated assumption: **that nothing can establish or
invalidate a rollback authority between the request and the activation.**

Two arms in `game/ambition_content/src/reload_tests.rs` now measure what happens
when it does — `a_generation_still_publishes_across_a_timeline_that_went_live_mid_flight`
and `an_authority_that_goes_unhealthy_mid_flight_cancels_the_pending_generation`
(renamed 2026-09-13; the second is now SEALED). Both published when written. The second
is the worse one: an unhealthy authority is a RECORDED DIVERGENCE, and
`publishing_does_not_heal_an_unhealthy_rollback_authority` exists precisely
because content publication must not launder a desync.

⚠ **WHAT IS NOT MEASURED, AND IT IS THE WHOLE QUESTION:** whether the SHIPPED
lifecycle naturally produces either transition inside that window. Both arms
install the authority by hand. The suspicious road is
`local_session::maintain_local_session`, which runs every `Update` and starts a
GGRS session when gameplay becomes active — and a reload re-prepares the route
the shell is already on, so the session world is torn down and rebuilt inside the
pending interval. ⇒ **Whether the restart lands before or after
`commit_content_generation` in that frame is an ORDERING fact I have not
measured.**

⛔ **THE FIX IS NOT A SECOND `publication_boundary` CALL AT THE COMMIT.** By then
the shell's engine/session half is already at its commit boundary; a fallible
content half there recreates exactly the split I3 exists to prevent — a route
activated at N+1 with a cast still at N. The two coherent directions are an
authorization that covers the INTERVAL and is broken EARLY (breaking it cancels
the whole shell transaction, so neither half activates), or a lifecycle that
stops and rebases rollback as part of the same transaction.

⭐⭐ **THE ORDERING MEASUREMENT IS TAKEN, 2026-09-12, AND THE ANSWER IS THAT
NOTHING ORDERS THEM.** `nothing_orders_the_rollback_session_start_against_the_generation_commit`
(`game/ambition_app/tests/reload_publication_is_installed.rs`) walks the shipped
`Update` dependency graph and finds NO path in either direction between
`LocalSessionSet::Maintain` — where `maintain_local_session` starts the GGRS
session — and `commit_content_generation`. ⇒ The two are AMBIGUOUS: Bevy is free
to run them in either order, so a schedule invariant does not exist to be
appealed to, and *"the transition is impossible"* is not available as an answer.

⛔⛤ **THE CONTROL IS WHAT MAKES THAT NEGATIVE WORTH ANYTHING.** *"No path
exists"* and *"my traversal cannot find a path"* are indistinguishable from the
outside, so the same traversal is first asked a question whose answer this file
already asserts directly — `AmbitionGameShellSet::Pending` → the commit — and
must find it.

⚠ **WHAT THE MEASUREMENT DOES AND DOES NOT SAY.** It says the ORDER is
unconstrained, which is necessary for the crossing and is exactly what a schedule
invariant would have to fix. It does NOT by itself show that a session start
occurs inside a pending generation's window — that needs the two to co-occur, and
the plausible road is that a reload re-prepares the route the shell is already
on, so the session world is torn down and rebuilt while the generation waits.

⇒ **SO THE PACKET IS REAL AND THE REMAINING QUESTION IS NARROWER:** an
authorization covering the INTERVAL, broken EARLY so that breaking it cancels the
whole shell transaction (neither half activates), or a lifecycle that stops and
rebases rollback as part of the same transaction. ⛔ Not a second
`publication_boundary` call at the commit — see above.

## Q119 — RULED BY THE REVIEW, RECORDED FOR THE AUDIT TRAIL: which App authorities are MECHANICAL?

⭐ **NOT AWAITING A DECISION — the classification below came from Jon's forwarded
review and is being applied.** It lives here because the LIST is what future work
has to be checked against, and a rule with no home is a rule nobody re-reads.

> ```text
> Mechanical + immutable during timeline → bind into generation / timeline identity
> Mechanical + changes during timeline   → deterministic input/state, or explicit rebase
> Derived                                → rebuild after rewind
> Presentation-only                      → may remain outside gameplay identity
> ```

⛔ **THIS REPLACES "forward-only" AS A ROLLBACK-COVERAGE CATEGORY.** The review's
words: *"A mechanical resource can be omitted from rollback only if it is
immutable for the lifetime of the rollback timeline, derived entirely from
registered historical state, or represented as deterministic external input.
'Forward-only' by itself is not a rollback category."*

⇒ **AND IT COMES WITH TWO QUESTIONS TO ASK OF ANYTHING CALLED authored /
prepared / immutable / generation-bound / rollback-safe:**
1. Where does execution READ the value?
2. Where is that exact value represented in mechanical identity or historical
   input?

*"If the answer to the second question is nowhere, the first answer is enough to
reopen the architecture."*

**Bound as of 2026-09-12** (`characters.definitions`, `characters.authored-sheets`,
`boss.catalog` — see the `MechanicalRegistries` receipt). **NOT yet audited:** the
rest of `PlatformerSessionBuilder`'s inputs, and the live developer-edit
resources (`ActiveMovementTuning`, `Platformer2dFeelTuningMonolith`,
`EditableAbilitySet`, `EditablePlayerStats`, `DeveloperTools.player_body_profile`)
which are the SECOND row of the table and have no answer yet.

## Q120 — which rollback model do LIVE DEVELOPER MECHANICAL EDITS get: refusal, rebase, or deterministic input?

⛔⛤ **THE GAP IS MEASURED, NOT ARGUED. `game/ambition_app/tests/developer_edits_under_rollback.rs`.**

`rollback_coverage.rs` waives `ActiveMovementTuning` and
`Platformer2dFeelTuningMonolith` with the reason *"forward-only"* — a developer
knob, not per-frame simulation state. **That answers the wrong question.** Under
rollback the question is not *"do we want to rewind this value"* but **"can its
value affect the simulation of a HISTORICAL frame"**, and it can:

```text
frame 100:  jump_speed = A       simulated, checksummed
frame 101:  a developer edits it → B
frame 102:  rollback to frame 98
            ... resimulate 98..102 — now reading B
```

**MEASURED 2026-09-12 against the real GGRS sync-test canary** (save every frame,
rewind 4, resimulate the same inputs, compare checksums): editing
`ActiveMovementTuning` mid-timeline **DESYNCS**. The control — the same forty
frames with no edit — stays healthy, so it is the edit and not the rig.

⚠ **THE FEEL-TUNING ARM DID NOT DESYNC, AND THAT IS NOT AN ACQUITTAL.** The
scripted inputs never reached the double-tap term; the arm prints which of the
two happened rather than asserting the one that flatters the resource.

⇒ **WHAT IS NOT MEASURED AND IS EXACTLY THE DECISION:** which of the three
coherent models this wants. The review that found it names them:
1. **Refuse** mechanical live edits while a rollback timeline is active — simplest
   and safest; the developer gets *"requires local rebase / session restart"*.
2. **Rebase** — the edit becomes a generation change and a bounded
   reconstruction, which is the shape A10 is already building.
3. **Deterministic timestamped input** — possible, substantially more machinery
   (a simulation tick, serialization, replay semantics, network policy).

⛔ **WHAT IS NOT COHERENT IS TODAY'S:** a mutable mechanical input outside
rollback history that resimulation reads at its latest value.

⚠ **AND THE SAME QUESTION IS OWED BY FOUR MORE**, all with live-edit writers and
no rollback coverage: `EditableAbilitySet` (mutates rollback-controlled body
abilities), `EditablePlayerStats` (health/mana/offense, through a
`Local<PlayerStatsSyncSnapshot>` that is itself unrestored),
`DeveloperTools.player_body_profile`, and `PhysicsSandboxSettings`/`PortalTuning`
which carry the same *"forward-only"* waiver.

⇒ **THE WAIVER'S RULE NEEDS REPLACING EITHER WAY** — see `Q119`. *"A mechanical
resource can be omitted from rollback only if it is immutable for the lifetime of
the rollback timeline, derived entirely from registered historical state, or
represented as deterministic external input. 'Forward-only' by itself is not a
rollback category."*

## ⚠ Q121 — REOPENED 2026-09-13 BY REVIEW. Half closed; the freeze took the WRONG GENERATION and DIES AT ACTIVATION.

⛔⛤ **THE CLOSURE BELOW WAS WRITTEN ABOUT THE STRUCTURE AND THE STRUCTURE WAS
RIGHT. THE VALUE WAS NOT.** A review of `bdbddfe` measured the dataflow the
closure did not: a cast-changing reload admits the N+1 registry at REQUEST time
and deliberately withholds it from the App until the commit boundary, so
`PendingGeneration.admitted_cast` holds N+1 while the published
`PreparedCharacterRegistry` stays N for the whole window. The freeze read the
published one.

⇒ **A session whose IDENTITY named N+1 and whose FIGHTERS were N's** — the exact
mixed-generation class the road exists to remove. ⭐ And my own closure note
named the gap that hid it: *"there is no end-to-end arm that prepares a session,
MUTATES the registry, activates, and asserts the world was built from the frozen
value."* The structural half is the stronger half; it is not the behavioural one,
and the behavioural one was wrong.

✅ **HALF ONE IS CLOSED (`d8604e50c`).** `PendingContentIdentity` became
`PendingGenerationInputs` and carries the candidate cast beside the candidate
identity — one transaction-local claim, not a second authority — with
`AdmittedRevision::candidate()` as the accessor whose ABSENCE was the defect.
Poison-verified: collapsing the resolver to the App registry fails the guard with
`Some(3)` where `Some(9)` is owed.

⛔ **HALF TWO IS OPEN AND IT IS THE BIGGER ONE: THE FREEZE HAS THE WRONG
LIFETIME.** `PreparedPlatformerSessions::take` removes the prepared record at
activation and hands `FrozenMechanicalState` transiently to
`PlatformerSessionBuilder::build`. Nothing promotes it into an active-session
authority. So:

```text
session activation     -> frozen generation values
later room transition  -> current App values   (room_transition/loading.rs)
later reset            -> current App values   (session/reset/mod.rs)
```

⇒ *"A prepared generation means its frozen mechanical values"* is true of the
FIRST construction call and of nothing else. **The review's shape:** activation
promotes the exact immutable snapshot into an active-generation owner, and every
world-building road — initial, transition, reset, checkpoint reconstruction, A10
candidate construction — takes a projection from that ONE owner. ⛔ NOT a
separately synchronised "transition registry" and "reset registry"; that is the
duplicate-authority defect wearing a lifecycle hat.

⚠ **DO NOT INFLATE `ActiveContentBinding` INTO A BAG OF REGISTRIES.** It is a
tiny identifier and the review says so explicitly. A typed active-generation
object bound to the session scope is the shape.

⭐ **THE ACCEPTANCE TEST IS STATED**: activate generation A, replace the
App-global registry with B WITHOUT an admitted transition, transition rooms, and
assert the reconstructed room is still A's (or that construction refuses as
stale). Repeat through reset. Then activate B legitimately and prove transition
and reset switch atomically. Two Apps, so no process-global can satisfy it.

<details><summary>The 2026-09-12 closure note, kept because its structural claim
still holds and its stated gap is what the review walked through</summary>

### ✅ Q121 — the structural half: a prepared generation carries frozen values


`PreparedPlatformerSession` carries `FrozenMechanicalState` — the prepared cast,
the authored sheets and the boss catalog, CLONED in the same system that takes the
identity — and `PlatformerSessionBuilder::build` consumes it. ⛔ **The three `Res`
handles are GONE from the `SystemParam`**, so reading the live registry at
activation is a compile error rather than a discipline. POISON-VERIFIED: putting
`&self.boss_catalog` back into `build` fails with *"no field `boss_catalog` on
type `&mut PlatformerSessionBuilder`"*.

⚠ **THE COST IS A CLONE PER PREPARED SESSION AND THAT IS THE POINT.** Holding a
handle instead would reintroduce the read-at-activation this removes. The shipped
cast is 58 characters, so it is bounded and paid once.

⚠ **THE FREEZE CAPTURES THE FOLD, NOT THE PRE-FOLD SOURCE**, and the distinction
is real: the FINGERPRINT is over `StagedCharacterOverrides` (lossless — see its
doc), while construction reads the published `PreparedCharacterRegistry`. Freezing
has to capture the value the builder will actually use, so the two are captured
from different resources in the same system.

⛔ **WHAT IS NOT COVERED, said plainly:** there is no end-to-end arm that prepares
a session, MUTATES the registry, activates, and asserts the world was built from
the frozen value. What is proven is the STRUCTURE — the builder no longer has a
handle to mutate against — which is the stronger half but not the behavioural one.

### Q121 — the prepared generation is a FINGERPRINT, not a FROZEN VALUE: prepare A, construct from B

⛔⛤ **THIS IS THE LAYER THE PREVIOUS FIX EXPOSED, NOT THE PREVIOUS FIX FAILING.**
`characters.definitions`, `characters.authored-sheets` and `boss.catalog` now
reach `PreparedContentIdentity` (`215ecc43c`), so the identity finally names the
mechanical content. **But the identity is taken at PREPARATION and session
construction re-reads the same MUTABLE App registries at ACTIVATION.**

**MEASURED 2026-09-12.** `PlatformerSessionBuilder` is a `SystemParam` holding
live handles:

```text
crates/ambition_platformer2d_provider/src/lifecycle.rs:1385  Option<Res<PreparedCharacterRegistry>>
                                                     :1404  Res<AuthoredSheets>
                                                     :1405  Res<BossCatalog>
```

⇒ Between the fingerprint and the construction, any of the three may be
replaced — and one of them demonstrably is: **`activate_staged_revision` inserts a
new `PreparedCharacterRegistry`**, which is the hot-reload road. So a generation
can be prepared against cast N and have its world built from cast N+1, with the
identity still claiming N.

⚠ **IT IS THE SAME INTERVAL `Q118` IS ABOUT**, from the other side: Q118 is a
LEGALITY that goes stale across the window, this is a VALUE that does. A fix for
one does not fix the other, but a lifecycle that seals the interval would.

⇒ **THE ANSWER THE REVIEW NAMES** is the second row of `Q119`'s table made real:
a prepared generation should MEAN the frozen mechanical values, not
*"`PreparedContent` says N, and at activation query whatever these App resources
contain now"*. Session construction consumes the frozen values.

⚠ **AND IT MATTERS MORE FOR A10, not less:** a last-good-world guarantee is only
worth having if the candidate's identity identifies the world being constructed.

</details>

## Q122 — which fields of the mechanical registries are MECHANICAL, and which are presentation?

⛔⛤ **THE NEW FINGERPRINT IS OVER-SENSITIVE, AND I INTRODUCED THAT.** Binding the
three registries exhaustively — by `serde` derive and by declaration TEXT — was
the right call for completeness and the wrong one for MEANING. Measured, it hashes:

- the sheets' raw declaration RON, so **reformatting a file moves the mechanical
  identity**;
- `PreparedCharacterOverrides`'s `portrait`, `voice`, `ranged_vfx`, `dream_seed` —
  presentation by their own doc comments;
- `BossCatalog`'s `sprite_filenames`.

✅ **ONE OF THEM IS ALREADY OUT:** the sheets' `origin` — the declaring FILE
PATH — is removed. *"The same records from a different file"* is a real
difference to a COLLISION REPORT and no difference at all to what a body
simulates, and making it move the identity refuses snapshots and reloads for a
provenance edit.

⚠ **THE FAILURE MODE IS SAFE AND THAT IS WHY THIS IS NOT P0.** Over-sensitivity
refuses a legitimate restore; under-sensitivity restores a snapshot into the
wrong world. ⇒ **Do not fix it by hand-listing exclusions** — that is a
population that rots, and a new presentation field added later would be included
silently. The shape that holds is an annotation ON the field (`#[serde(skip)]`
or an equivalent), so the derive stays exhaustive and the default is the safe
direction.

⇒ **THE DECISION IS THE LIST**: which fields of `PreparedCharacterOverrides`,
`SheetRecord` and `BossCatalog` a rollback timeline's identity should bind. That
is a design pass, not a cleanup.

## Q123 — A10's "invisible candidate" is proven for ordinary QUERIES and nothing else

✅ **HIDDEN AT MINT AS OF `ebfcda0ee`** — the marker goes on in the same command
batch as the root's identity, before the recipe runs, closing the window where a
component hook or lifecycle observer could see a candidate as an ordinary member
of the live world.

✅ **THE ROLLBACK-SNAPSHOT HALF IS CLOSED, and it was the one that mattered
most** — a candidate that entered a GGRS save would be restored into the LIVE
world by the next rewind: an unpublished, unvalidated scene arriving as history,
strictly worse than the half-visible scene `commit_inactive` exists to prevent.

**MEASURED in the pinned dependency:** `bevy_ggrs-0.22.0` collects snapshot state
through ORDINARY queries. ⚠ THE PATHS BELOW ARE IN THE DEPENDENCY, NOT THIS
REPO, so they are spelled without the `file:line` shape the citation checker
resolves against the tree: in its snapshot module's component-snapshot file the save is
`Query<(&RollbackId, &S::Target)>` (line 69) and the restore is
`Query<(Entity, &RollbackId, Option<&mut S::Target>)>` (line 99); in
its snapshot entity file the entity map is `Query<(&RollbackId, Entity)>` (line 42). **Not one mentions `InactiveCandidate`**, so
`DefaultQueryFilters` excludes candidates from the save, the restore and the
entity map alike. `a_candidate_is_invisible_to_a_query_shaped_like_the_rollback_snapshots`
pins the half that is OURS — that a tuple query in exactly that shape sees
nothing — with the live body as its premise and publication as its lift.
Poison-verified: skipping `register_inactive_candidate_filter` reddens it.

⚠ **THE OTHER HALF IS A READING OF A PINNED VERSION.** That `bevy_ggrs`'s queries
have that shape is not something the arm tests, and a `bevy_ggrs` upgrade is
where it has to be re-read.

⛔ **STILL UNPROVEN:** physics/global gatherers and anything walking ARCHETYPES
directly rather than through a filtered query — those do not inherit
`DefaultQueryFilters` at all.

⛔ **AND PUBLICATION ATOMICITY RESTS ON A POPULATION FACT, NOT A BOUNDARY.**
`publish_candidate` removes the marker entity-by-entity under `&mut World`, which
is atomic with respect to SCHEDULED SYSTEMS — none run inside an exclusive world
call. A component HOOK is not a system: one registered on a published component
would observe a half-published candidate. The census that says this is safe today
(**zero component hooks workspace-wide, three lifecycle observers, one an `Add`
and not on the construction road**) is a population count, and the
`InactiveCandidate` doc says so itself: *"it is a POPULATION FACT, not a
boundary, and it is the thing to re-measure before trusting this in a new
domain."*

⛔⛤ **AND THE HOOK HALF CANNOT BE GUARDED FROM OUTSIDE, MEASURED.** The obvious
fail-closed move — refuse to publish if `InactiveCandidate` carries a registered
hook — is not available: `ComponentHooks`'s fields are `pub(crate)` in
`bevy_ecs-0.19.1` (`lifecycle.rs:150-156`), so `ComponentInfo::hooks()` hands back
a value nothing outside that crate can interrogate. A census of hooks is
therefore a `git grep`, which is the population fact this row is complaining
about, not a replacement for it.

⇒ **WHAT WOULD CLOSE THE REST:** a structural reason no hook can observe a
partial publication (or an upstream way to ask), and an arm for a collector that
walks archetypes rather than querying.

## Q124 — a DEATH-RESET rebuilds a room around the placement you are still CARRYING, and duplicates its identity

⛔⛤ **MEASURED 2026-09-13, and it is the mechanism A10's hold said it could not
name.** The `death_restores_the_checkpoint` arms were the only two app-level room
tests that failed with the candidate bracket on, and the reason is not A10's.

**WHAT THE INSTRUMENT SHOWED.** A death is committed as a room TRANSITION to the
SAME room (`room-transition begin seq=1 central_hub_complex ->
central_hub_complex`). The transaction that rebuilds it is refused:

```text
verify_and_publish room=central_hub_complex violations=[
    Duplicated { sim_id: SimId("placement:ground_gun_sword"), count: 2 },
    PlannedOverBaseline { sim_id: SimId("placement:ground_gun_sword") } ]
```

and the two occupants differ by exactly one component:

```text
502v0  ... ItemCustody, InCustodyOf, SettledItem, RoomScopedEntity ...   <- the one you are holding
530v0  ... ItemCustody,             SettledItem, RoomScopedEntity ...   <- the one the room just minted
```

**THE ROAD.** Room-transition sweeps use
`RoomResident = (With<RoomScopedEntity>, Without<InCustodyOf>)` — a carried item
deliberately follows you through a DOOR, which is right. But this "door" leads
back into the room that AUTHORED the thing you are carrying, and the room plan
mints `placement:ground_gun_sword` again. Two entities, one authored identity.

⛔⛔ **THE LIVE BUILD PRODUCES THE IDENTICAL VIOLATIONS AND IS IDENTICALLY
REFUSED** — measured, not reasoned: three refusals across the same run, one per
carried placement (`ground_gun_sword` twice, `ground_grapple` once), and
`room-loaded` is never written for them. The test passes anyway **because under
the live build a refusal does nothing**: the entities were committed to the world
before verification ran, so *"the room was not published"* costs exactly the
`RoomLoaded` message. Under the candidate bracket the same refusal correctly
drops all 18 roots and the room is empty.

⇒ **A10 DID NOT CAUSE THIS. A10 MADE AN ALREADY-SILENT REFUSAL BITE**, which is
the whole point of "make it impossible, not checked" — and the first thing it
found was a real production defect that has been shipping under a refusal nobody
could act on.

⭐⭐ **AND THE TRANSACTION NEVER DECLARES ITS INTENT.** `TransactionBaseline` has
`retiring()` and `reconstructing()` for exactly this, and **`git grep` finds ZERO
production callers** — both are reached only from tests. `transaction::open`
captures a baseline that declares nothing, so `verify_committed_roster` judges
every shipped room against a claim nobody made. That is why the violation reads
`PlannedOverBaseline` (a vague *"you rebuilt something you did not say you
would"*) instead of `ReconstructedOldSurvived` (the precise *"you said you would
replace this and the old body is still here"*).

⇒ **THE DECISION, and it is Jon's because it is a GAMEPLAY rule, not a
structural one:** when a death-reset rebuilds the room you are standing in, what
happens to an authored placement in your custody?

1. **The held copy is destroyed and the room's fresh one is the only occurrence.**
   Matches the test's own sentence — *"the reward was acquired before any
   checkpoint, so a death owes it back to the world"* — and matches what a player
   expects from a death. A same-room reset stops being a "door" for custody.
2. **The room does not replan a placement that is in custody.** You keep what you
   were holding and the room comes back one pickup short of itself until you drop
   it. ⚠ This is the shape that was already rejected once for a different reason
   (see `clear_transient_on_sandbox_reset`'s note about rebuilding a room
   permanently one pickup short).
3. **Custody is durable across a death and the DUPLICATE is the defect** — the
   room's plan should recognise the identity as already live and adopt it rather
   than mint a second. This is `reconstructing()` used properly.

✅ **THE STRUCTURAL HALF IS DONE AND NEEDED NO RULING.** `transaction::open`
takes the plan and derives the declaration from it, so every shipped room now
states what it is reconstructing. MEASURED before and after, on the same death:

```text
before: [Duplicated { placement:ground_gun_sword, count: 2 },
         PlannedOverBaseline { placement:ground_gun_sword }]
after:  [Duplicated { placement:ground_gun_sword, count: 2 },
         ReconstructedOldSurvived { placement:ground_gun_sword, stale: 502v0 }]
```

⇒ The refusal is unchanged; what changed is that it now NAMES THE SURVIVING
ENTITY instead of saying *"you rebuilt something you never said you would"* —
which was true of every room reset in the game and therefore said nothing.

⭐⭐ **AND THE FIRST VERSION OF THAT FIX BUILT A SECOND AUTHORITY, caught by a
poison that refused to fire.** `open` originally ACCEPTED a declaration, and the
test harness immediately grew its own copy of `plan.planned_sim_ids()` beside the
production one. Poisoning the production declaration to declare nothing left
`a_room_that_fails_verification_is_not_published` GREEN — the arm was declaring
for itself and never reached the production road. `open` now takes the PLAN and
derives it, `close` already did, and the same poison reddens the arm.

⛔ **WHAT STILL NEEDS THE RULING** is only the gameplay question below: the
verifier can now say precisely what is wrong, and cannot say what should happen
instead.

⚠ **AND ONE TEMPTING ANSWER IS ALREADY RULED OUT — CHECKED BEFORE OFFERING IT.**
*"A death-reset is not a door, so it should take the RESET road, whose sweep does
not exclude `InCustodyOf`"* looks like the fix, because the reset road's own
comment says a reset destroys the occurrences it is about *"hands included"*. But
`resume_at_checkpoint_on_reset` routes a death through the transition road
DELIBERATELY, and says why: *"a session opening at a checkpoint and a death
returning to one are the same question asked twice."* Re-routing would fork that
answer, which is the defect this repository removes rather than the fix. ⇒ The
three options below remain the whole choice.

⚠ **A10's LAST STEP IS HELD ON THIS ROW**, and the hold is now one sentence
rather than "the mechanism is not yet understood".

## ✅ Q125 — CLOSED 2026-09-13. A relation gets its two endpoints and nothing else.

`RelationFn` takes a `RelationScope` whose `commands` is PRIVATE; the two `Entity`
parameters are gone with it, because the endpoints live on the scope. The whole
surface is `from()` / `to()` (each an `EntityScope` over ONE named entity),
`from_entity()` / `to_entity()`, and read-only `scope` / `session` / `services`.

⭐ **`EntityScope::queue_component_upsert` IS WHAT MADE IT POSSIBLE WITHOUT A
CARVE-OUT.** `wire_limb` genuinely needs a deferred read-modify-write on the host
(the rig is built by whichever limb is wired first and extended by the rest), and
`queue_component_mut` skips when the component is absent. Expressed as two
operations the caller reaches for `Commands::queue` and takes the whole
`&mut World` back; expressed as an upsert it needs nothing.

**MEASURED, from `ambition_platformer2d_actor_monolith` — a DIFFERENT crate, so
this is the real caller's view and not the defining crate's:**

```text
spawn                      -> error[E0616]: field `commands` of struct `RelationScope` is private
insert_resource            -> error[E0616]: (same)
despawn a third entity     -> error[E0616]: (same)
commands.queue(&mut World) -> error[E0616]: (same)
from().queue(&mut World)   -> error[E0599]: no method named `queue` found for struct `EntityScope`
commands_for_sabotage()    -> error[E0599]: no method named `commands_for_sabotage` found
```

⇒ The last line is the `#[cfg(test)] pub(crate)` claim verified rather than
asserted: the sabotage door the adversarial toy relations use is unreachable from
any other crate.

⛔ **SESSION-SCOPED INSERTION WAS DELIBERATELY NOT ADDED.** The review listed it
as something a relation might want; no shipped relation uses one, and
`RootScope::rebind` is the recorded lesson — *"a capability with no caller is not
narrower than one with a caller; it is the same capability, untested"*. It arrives
with its caller.

<details><summary>The original row</summary>

### Q125 — A10 closed the RECIPE escape and left the RELATION escape open

⛔⛤ **FROM THE 2026-09-13 REVIEW, and it is the right reading of `c90a1cda0`.**
`ConstructionRootCtx`/`RootScope` made unplanned minting a TYPE ERROR for
recipes. `RelationFn` still receives `&mut ConstructionExecCtx`, whose
`pub commands: &mut Commands` lets a relation spawn, despawn an unrelated live
entity, insert or mutate resources, and — via `commands.queue(|world: &mut
World|)` — take unrestricted exclusive-world authority. `commit_inactive` applies
that queue BEFORE verification, and `retire_candidate` despawns only roots tagged
as this transaction's candidates: it cannot undo a resource write.

⇒ **A failed candidate can still mutate world N**, which contradicts A10's
last-good-world claim. ⚠ **NOT A LIVE CORRUPTION REPORT**: the review inspected
`wire_limb`, `wire_mount` and `wire_grudge` and all three touch only their
declared endpoints. It is a capability that no type boundary prevents.

⇒ **THE SHAPE**: a `RelationScope` bound to `from` and `to`, by analogy with
`RootScope` — insertion on either endpoint, endpoint-scoped deferred
read-modify-write (`wire_limb` genuinely needs that for the host's `LimbRig`),
session-scoped insertion where needed; and NO `Commands`, `World`, spawn,
arbitrary lookup or resource mutation. A relation that needs a third
authoritative entity has a missing PLAN ROW, not a minting need.

⚠ **CLOSE IT BEFORE WIRING A10's CANDIDATE LIFECYCLE INTO I3b**, or the
integration bakes in the escape the recipe work was built to delete.

</details>

## ✅ Q126 — CLOSED 2026-09-13 by (a): the two knobs are IN the canonical identity.

`MechanicalRegistries::developer_construction` renders both into the
`construction.developer` section, so two Apps that admit different numbers of
authored actors — or force different brains — are different generations, and a
snapshot taken under one is refused into the other by the contract that already
compares that identity. (b), refusing a non-default knob in a rollback session,
was not needed: including the value is strictly more useful, because it lets a
measurement run under a cap still be replayed against itself.

⚠ **`PerceptionExtentOverride` IS STILL OUT**, deliberately and not by oversight:
its knob configures `ensure_perception` when it attaches a policy to a NEW body,
and the resulting `Perception::Sighted` IS registered rollback state. Whether the
seed belongs in the identity alongside the two above is the same question one
layer out, and it is not answered by this row.

⭐⭐ **A POISON DECLINED TO FIRE AND FOUND THE REAL GAP.** The first two arms vary
WHICH FIELD is set and never WHICH VALUE, so a dump spelling `set` instead of the
value passed both: two compositions forcing every brain to different presets, or
capping at 1 and 2, would still have shared an identity.
`two_different_values_of_one_developer_knob_are_two_identities` is the arm that
closes it, and it is red under that poison.

<details><summary>The original row</summary>

### Q126 — immutable DEVELOPER construction configuration is mechanical and is outside the identity

⛔ **CONFIRMED AT HEAD 2026-09-13**, from the review. Two `PlatformerSessionBuilder`
inputs (`lifecycle.rs:1503-1504`) are mechanical, change the constructed world,
and reach no fingerprint:

* **`AuthoredBrainOverride`** — chooses a forced profile/preset in the NPC
  construction road (`npc_policy.rs`). Its own doc says it changes the room.
* **`AuthoredPopulationCap`** — consumed by `RoomFeatureConstructionPlan::prepare`
  BEFORE the construction rows exist, so it changes the authoritative ROSTER.

⇒ Two Apps can share one `PreparedContentIdentity` and have different rosters, or
different autonomous behaviour. `rollback_coverage.rs` exempts both because they
are written once and not reread during resimulation — which answers the SNAPSHOT
question and not the IDENTITY one. **`PerceptionExtentOverride` owes the same
audit**: its resulting component is snapshotted, its startup configuration still
makes two otherwise identical compositions construct different mechanical state.

⇒ **THE DECISION**: (a) normalise their mechanical values into the canonical
generation identity, or (b) refuse a non-default knob in a rollback- or
network-compatible session. ⛔ NOT a second hash authority — the same canonical
identity, one more typed input. If any becomes live-editable it graduates into
`Q120`.

</details>
