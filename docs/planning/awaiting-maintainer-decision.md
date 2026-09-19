# Awaiting a maintainer decision

This file contains only **currently unresolved maintainer/product decisions**.
Engineering work that can proceed without a ruling belongs in
[`queue.md`](queue.md). Answered, withdrawn and superseded questions are deleted
from this live ledger; Git history preserves the discussion.

Use one unique `Q<number>` per live question. Do not reuse a retired number.
Every queue row blocked on a maintainer choice must name its `Q` here. When a
question is answered, record the durable ruling in
[`maintainer-decisions.md`](maintainer-decisions.md), update the owning plan/source,
and delete the question here.

## What actually blocks architecture work today

**MEASURED 2026-09-19, after `Q132` was decided**, by reading every gate in
[`queue.md`](queue.md)'s P0/P1 sections and every `DO NOT START BEFORE` in
[`consolidation-plan.md`](consolidation/consolidation-plan.md) rather than by
recalling which questions feel important. ⚠ This list is the BLOCKING set, not
the important set: a question can matter and block nothing, and one below
blocks a campaign that could not previously be costed.

⛔⛤ **AND THE FIRST VERSION OF THIS TABLE MISSED TWO, BECAUSE THE SCAN WINDOW
WAS THE BUG.** It read 60 lines from each row's heading; in this file a row's
blocker is routinely stated in a block further down — `Q139` sits about eighty
lines into `ROLLBACK-MUTATOR-POPULATION`. Re-derived over each row's FULL extent
(heading to the next heading), which is the only boundary that means anything
here. ⇒ A population scoped by a line count rather than by the structure it
lives in is the same defect this page records for censuses of source, arriving
in a census of itself.

| question | what it blocks | and if it stays open |
|---|---|---|
| [`Q136`](#q136--how-does-a-local-menu-intent-enter-the-synchronised-timeline) | **P0** `CUTSCENE-ROLLBACK-DECISION` item 1, explicitly (*"blocked on `Q136`, not unowned"*) | **four** host→sim intents stay live defects, each an EDGE raised outside the rewinding schedule. ⭐ Witnessed 2026-09-19: an edge straddling its consumer does not merely lose the intent, it **desyncs the timeline** — the worse of the two failures. ⚠ The fifth, `SetFlagRequested`'s intro chain, is CLEARED by the same arms: it is a derivation, and a straddling derivation is repaired by the resimulation that re-runs it |
| [`Q122`](#q122--which-registry-fields-are-mechanical-and-which-are-presentation) | **P0** `ID-PEER`'s snapshot-schema-fingerprint road | two builds of the same mechanical schema stay two identities if somebody rewords a comment — poison-measured at 166 diff lines for one pluralised word |
| [`Q144`](#q144--must-every-supported-composition-activate-a-prepared-generation-or-does-direct-entry-keep-the-app-registry-road) | **C04**, and it is now that row's ONLY maintainer hold | C04 cannot start. ⭐ `Q132` narrowed it: the anonymous App-global fallback is already on the wrong side of the scoping rule, so `Q144` now owns only whether direct entry must ACTIVATE a generation or may declare its inputs another explicitly-scoped way |
| [`Q146`](#q146--what-are-the-supported-composition-profiles-and-which-authorities-must-each-one-carry) | **C07**, entirely | C07 cannot be COSTED, not merely started: *"replace optional fallbacks where the authority is required"* has no population until "required" has a referent |
| [`Q139`](#q139--what-declares-that-a-presentation-system-writes-transform) | **P0** `ROLLBACK-MUTATOR-POPULATION`'s only open item | the mutator guard keeps excluding `Transform` BY NAME, so its green says nothing about the most rollback-sensitive component in the workspace. The repair is a DECLARATION across the excluded systems rather than a cleverer scanner — measured, not assumed: a name-based classifier was wrong in both directions — so the ruling is its SHAPE, and four shapes are costed in the row. ⚠ Sizes deliberately not restated here; they moved with the carve work and they depend on a marker vocabulary this ruling would itself be choosing |
| [`Q138`](#q138--should-platformer2dsimharnessstep-refuse-to-step-an-invalidated-session) | **P1** `ROLLBACK-DEAD-SESSION` | an invalidated session keeps accepting `step()`, stops advancing `SimTick`, and returns an observation every time — so assertions after it agree with a frozen world forever. `rollback_health()` already knows and `step` does not consult it; whether it should REFUSE is an API contract nobody has set. ⚠ Cardinalities deliberately not restated here — the row says why, and `scripts/a_rollback_arm_must_refuse_a_frozen_world.py` prints the live one |

⭐ **AND TWO THINGS THAT LOOK LIKE BLOCKERS AND ARE NOT, WHICH IS THE USEFUL
HALF OF MEASURING THIS.**

- [`Q128`](#q128--should-the-simulation-tick-be-rebased-when-peers-agree-to-start-or-stay-an-absolute-per-app-count)
  is a **coordination re-arm condition** on `C03` and `C05`, not a gate: both
  rows say *"if that road is ruled and started while this migration is in
  flight, coordinate rather than assume."* Its other half — the timeline
  comparison — is blocked by `N2`'s absent P2P session, which is engineering
  rather than a ruling.
- [`Q145`](#q145--which-way-round-do-the-room-transition-readiness-chain-and-the-presentation-chain-go-in-update)
  is named by **no queue row and no campaign gate**. It is a live
  ordering-nondeterminism question held by a ratchet, and answering it unblocks
  nothing, so it does not belong in a minimal blocking set.

⚠ `Q127`, `Q129`, `Q133`, `Q137`, `Q101`, `Q104` and `Q110` sit inside P0/P1
rows as product or balance calls on specific sub-roads rather than as gates on
the row. They are real and they are not architecture blockers. ⚠ `Q137` is the
closest to the line, and stating it precisely matters because the obvious
phrasing is wrong: `ID-PEER`'s sharp unchecksummed set is TWELVE rows, eleven of
which are reachable, covered and agreeing. `gravity.flip_switch` is the twelfth
and NO route reaches it — its only mutable writer is registered once in the
workspace, inside a `#[cfg(test)]` module. ⇒ `Q137` does not gate the eleven; it
asks whether that twelfth vertical ships at all, which is a content question
rather than an architecture one, and the row is complete either way.

## Gameplay and content

## Q33 — how should a recharging ranged weapon communicate that it is unavailable?

Choose the player-facing unavailable/readiness signal. The mechanism should not
invent one presentation independently for every ranged weapon.

## Q36 — what are the authored standing heights of the puppy slug, stochastic parrot and burning flying shark?

The engine has a canonical-height contract; these remaining authored characters
need product values rather than inferred sprite dimensions.

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

⇒ **The one shipped instance is measured, side by side with an ordinary held
item, at I3 in [item custody](engine/item-custody-and-accounting.md).** Today the
portal gun is an ENTITLEMENT with a cosmetic world token: `OwnedItems` is granted
on pickup and never revoked on drop, the dropped token is room-scoped, and the
menu re-equips from the entitlement without checking that a token exists. So the
two readings differ observably in exactly one place — whether dropping the gun
and walking away can ever lose it — and a ruling for "occurrence" is a behaviour
change, not a cleanup.

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

## Q97 — may authored content name a technique this composition did not install?

Capture techniques are now engine-owned, so shipped capture content no longer creates this mismatch. The remaining policy question is general: if authored content references an uninstalled capability, should admission refuse the definition/cast, or admit it with a loud degraded-capability diagnostic? Current admission refuses per definition. This also constrains the minimum-profile work in A9. Owner: [`engine/authored-technique-admission.md`](engine/authored-technique-admission.md).

## Q102 — is a solid breakable represented as `BlinkWall { Hard }`, or is that a temporary borrow?

The current solid-breakable road publishes hard-wall behavior through the blink-wall vocabulary. Decide whether that is the intended durable representation or whether breakable solidity needs its own semantic fact. Do not split the type only for naming; split it only if the gameplay contracts differ.

## Q105 — may an author place a chest that is already open?

The old write-only chest-state field was removed; runtime open state is the `Opened` marker. If authored worlds may start with an open chest, add an authored input and lower it to the marker at construction. If not, keep the authoring surface closed rather than restoring dead state vocabulary.

## Q107 — do sprite `active` frames own contact timing, or does moveset authoring?

Both presentation metadata and moveset data can describe an attack-active interval. Choose the mechanical owner. Presentation may project from mechanics, but two independently authored timing truths should not remain.

## Q115 — which per-move `hitbox.inflate` values should the untuned bone-derived specs carry?

The remaining roster has bone-derived hitboxes whose inflation values are not product-tuned. Choose the per-move feel values from measurement rather than applying one roster-wide generosity knob. The queue/Smash parity inventory owns the measurement work; this ledger owns only the tuning decision.

## Architecture and engine policy

## Q34 — should external/launch-owned motion become an explicit cross-game fact?

If more than one game needs it, give it a reusable authority. Otherwise leave the
current local mechanism local.

## Q35 — what owns fighter reach during move startup?

Choose the semantic owner of startup reach so AI, collision and presentation do
not each infer it from different move state.

## Q37 — should the F9 rollback proof pulse survive a gameplay-session change?

Decide whether the pulse demonstrates one rollback session or is a process-level
debug affordance. Its resource lifetime should follow that answer.

## Q48 — should the boss subsystem become a separately composed crate now?

The reassessment requested earlier is due. Judge against current dependencies and
customers, not the historical monolith shape.

## Q59 — two validation ledgers can be red when no lane ran: hook the lane or accept the state?

Choose whether “not run” is a first-class incomplete receipt or whether those
ledgers should be removed from the required surface.

⭐⛤ **THIS ROW HAD NO MEASUREMENT FOR ITS OWN SUBJECT UNTIL 2026-09-18, AND THE
SUBJECT IS MUCH BIGGER THAN TWO LEDGERS.** Censused over every `scripts/*.py`
whose `main()` returns a verdict — the population, rather than a naming
convention over it:

| what holds the verdict | count |
|---|--:|
| named as a `--maintenance` job | 41 |
| not in the lane, but a pytest arm calls `main()` | 24 |
| **nothing** | **77** |

⚠ **THE 77 IS NOT 77 DEFECTS, AND SAYING SO IS THE POINT OF THE ROW.** Most of
it is `measure_*` and `render_*` — REPORTS, which always exit 0 and are held by
nobody on purpose. A report is exactly the *"remove it from the required
surface"* answer, already taken, for most of the population. ⇒ The question is
only live for the ones carrying a BUDGET, a WAIVER TABLE or an ABSENCE
CONTRACT, and there were **eight** of those:
`check_pinned_music_renderer_refuses_gm`, `check_capability_ships`,
`check_engine_systems_are_engine_installed`, `check_headless_arms_can_fail`,
`check_retired_crate_names`, `check_set_pins_have_engine_members`,
`check_severed_sentences`, `check_quality_variants_are_fresh`. All eight were
GREEN, which is how they stayed invisible — and all eight now run in the lane,
at 11 seconds for the set.

⇒ **SO THE ENGINEERING HALF ANSWERED ITSELF: at this price, hooking the lane
wins.** What remains for a maintainer is the narrow version — whether *"not
run"* should be a first-class **incomplete** receipt distinct from **pass** and
**fail**, which is a receipt-model question and is not settled by any of the
above. A ratchet costing 1–3 seconds does not need a policy; a ratchet costing
minutes does, and the lane will grow ones that do.

⚠ **AND THE CENSUS ITSELF IS THE ROW'S CAUTIONARY TALE.** Three sweeps for this
same class ran on 2026-09-18 and the first two undercounted: one used
`check_*.py` as its population and could not see `a_*.py`; the next used the
`a_*` / `*_must_*` / `*_is_*` spellings and could not see these eight. Five
guards were wired before the population was measured rather than guessed. A
scan root is a citation, and a member outside it reads as absent rather than as
unlooked-at.

## Q61 — where should ordering live when two systems write the same durable switch?

Choose the intended winner/merge policy where product meaning is ambiguous.
Engineering already requires one accepted mutation authority and explicit phase
visibility; a public set by itself cannot decide between competing writes. See
[composition](engine/capability-and-runtime-composition.md).

## Q62 — keep or discard the epoch-captured 4,741-line `mary_o.ldtk` delta?

This is the explicit history/content decision. Do not modify the retained LDtk
files until the ruling is made.

## Q63 — should interactables gate on facing, chests persist per-chest, pickups carry collected state, breakables author a debris cue?

⚠ **THE FIELDS ARE GONE; THE FEATURES ARE THE QUESTION.** This question used to
read *"five authored fields still have no runtime consumer: wire them or delete
them?"* — and that phrasing kept four no-op authoring fields alive while it
waited. All four are now deleted:
`InteractableSpec.requires_facing`, `PickupSpec.collected` and
`ChestSpec.persistent` on 2026-09-12, `BreakableSpec.debris_cue` on 2026-09-17.
Each was serializable, documented, threaded through construction into a runtime
representation, and consulted by nothing. See
[item custody](engine/item-custody-and-accounting.md) for the measurement of all
four and F4 in [the source findings](engine/architecture-review-findings.md) for
the three traced by review.

⇒ **Deleting a false capability did not answer this question and was never
blocked on it.** What remains for a maintainer is the product choice: whether
facing-gated interaction, per-chest persistence, per-pickup collected state and
per-breakable debris cues are intended capabilities. Whoever wants one adds the
field and its consumer together — an authoring field alone is what this question
was originally filed about.

ⓘ The "five-field" label was an inherited inventory with no surviving list; the
traceable population is the four above, found by sweeping every field
`spawn_static.rs` threads. Until a capability is chosen, unsupported nondefault
values should receive diagnostics, not an invented runtime meaning.

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

⛔ **THE QUESTION IS NOT ONLY WHICH RULE WINS, IT IS THAT THE LOSING RULE IS
STILL DERIVABLE.** Measured 2026-09-16: an agent that had never opened
`AGENTS.md` hit ENOSPC (98M free of 290G, `target/debug` at 185G of which
`incremental` was 92G), reasoned from where the bytes were, concluded
`rm -rf target/debug/incremental` was the surgical cut because it preserves
`deps`, did it, and recommended it to a peer as general practice — reproducing
almost verbatim the sentence `AGENTS.md:107` retracted on 2026-09-03 after an
agent following it pruned `target/debug/{deps,examples,incremental}` and deleted
205 GB. The retraction is marked, in place, with the incident attached, and it
did not reach the agent because nothing put it in front of the action.

⇒ **A RETRACTED INSTRUCTION IS MORE DANGEROUS THAN AN ABSENT ONE**: the
reasoning that produced it is still available to anyone who reasons from first
principles about disk usage, and it still sounds correct. So a ruling that only
picks a winner leaves this open. What would close it is a MECHANISM — the rule
enforced where the action happens rather than stated where it is documented.

⚠ AND THE SAME RUN MADE THE ERROR THE FILE PRESCRIBES AGAINST FIRST: it read
`df -h $(readlink -f target)` instead of `scripts/setup/target_bindmount.sh
--status`. Run afterwards, `--status` reported `BOUND -> /dev/vda1[...]`, size
99G, repo fs virtiofs — so the bind was healthy and this was genuine build
output, not the absent-bind duplicate `AGENTS.md` says it usually is. The
outcome was fine and the method was prohibited. `--status` would also have
answered, without asking a peer, whether two sessions share a `target/`: they do
not, it is bound per-worktree by hash.

⚠ Related and unruled: repairing an absent bind SHADOWS the duplicate rather
than reclaiming it, and `check_disk_headroom.py` goes GREEN across that repair
because it then asks about `target/`, a different filesystem. A green check after
a repair that freed nothing is the false comfort that makes the next deletion
look justified.

⛔⛤ **A THIRD INSTANCE, 2026-09-16.** Another agent, under a lane abort at
33.8 GB, reached the same `target/debug/incremental` reasoning from the same
first principles. What redirected it was a failure message at the point of
action: a `scripts/tests` arm names `scripts/clean_workspace_crates.sh` in the
assertion it fails on. ⇒ So the mechanism this question asks for already
half-exists, and the missing half was that **the guidance did not say what each
sanctioned reclaim COSTS.** `check_disk_headroom.py`'s refusal named
`cargo clean` without saying it forces a rebuild, and never mentioned the
incremental-cache wrapper — so the option that frees space without a long build
was invisible exactly where somebody needs it, and the `rm -rf` cut looked like
the only way to get it.

✔ **LANDED WITHOUT A RULING, because it needed none.** The refusal and
`AGENTS.md` now carry all three reclaims with what each costs, measured the same
day on one box:

| command | reclaimed | what you pay |
| --- | --- | --- |
| `cargo clean --workspace` | ~35 GB | rebuild Ambition; dependencies stay built |
| `cargo clean` | ~80 GB | rebuild everything, Bevy included |
| `clean_workspace_crates.sh --incremental-only` | 82 GB, 13 GB an hour later | nothing rebuilt |

The third deletes the incremental CACHE rather than artifacts, so no fingerprint
is invalidated. Three `scripts/tests` arms that had failed on the abort went 6/6
after it. ⚠ **Jon's correction, same day: `cargo clean --workspace` plus plain
`cargo clean` is the ordinary answer, and the first write-up here read as a riddle
about a "forbidden" option instead of a table of trades.** The prose is the fix as
much as the content was.

⇒ **WHAT IS STILL OPEN IS THE RULING ITSELF** — which rule wins, and whether a
retracted instruction can be made underivable rather than merely marked. The
2026-09-16 instance is evidence for the answer this question already proposes: a
rule stated where it is documented lost to first-principles reasoning three
times, and a rule named at the moment of the action won.

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

## Q100 — should the facade pull `bevy/debug` because it always links `ambition_dev_tools`?

Mandatory developer diagnostics can use Bevy system names only when the debug feature is linked. Decide whether that diagnostic value justifies making `bevy/debug` part of the facade baseline, or whether minimal profiles must keep it out and accept reduced names. This constrains A9's minimum-profile contract.

## Q101 — may an ability's own contact satisfy the launching move's `Connected` condition?

Blink, dive, mark-recall and empowerment can create contact outside the ordinary strike verdict road. Decide whether that contact belongs to the launching move occurrence for `Connected`, or is an independent event that must not credit the current move. A12 cannot close reflection/contact attribution until this rule is explicit.

## Q103 — what should an unprepared character id inherit at wear time?

Prepared characters fold catalog movement tuning and motion model at admission, but the wear road still has a fallback for ids outside the prepared registry. Choose one contract: inherit the catalog's authored tuning at wear time, inherit engine defaults, or refuse an unprepared wear. The shipped compositions currently have no orphan prepared ids, so this is a boundary-policy decision rather than a live content defect.

## Q104 — is the Rust move table or the content file the source of a moveset?

Runtime composition consumes the content artifact; the legacy Rust tables remain exporter/parity inputs and contain their own tests. Decide the permanent authoring source. If content is authoritative, retire the duplicate Rust tables after preserving any validation they uniquely provide. If Rust is authoritative, treat generated content as build output rather than an editable source.

## Q106 — are `ambition_items` and `ambition_encounter` optional facade capabilities?

Manifest commentary and current dependency edges disagree about whether these capabilities are optional. Decide the public composition contract: optional capability edges, or mandatory baseline dependencies. Then make the manifest and A9 profile tests state the same truth.

## Q108 — which capabilities may a featureless `ambition_platformer2d` link?

A9 needs a product-level minimum-profile contract, not a crate-count target. Choose which capabilities are permitted in the featureless facade; dependency cleanup can then be tested against that named capability list.

## Q109 — should a simulated identity be able to name its room instance?

Current deterministic ids identify authored/simulated objects but do not encode a room-instance dimension, so a second instance of the same authored room can collide with an already-live identity. Decide whether room-instance identity belongs in canonical `SimId` semantics or should be represented by a separate deterministic scope. This gates A8's two-instance proof.

## Q110 — may a provider-keyed fragment registry gain a named hot-reload replacement operation?

Five provider-keyed fragment registries currently refuse conflicting re-registration. Keep ordinary registration as refusal. Decide whether the generation/publication boundary may use an explicit replacement operation, and whether that capability applies to every registry or only registries that are actually reloadable. Owner: [`triage/ambition-registry-core.md`](triage/ambition-registry-core.md).

## Q122 — which registry fields are mechanical, and which are presentation?

The mechanical fingerprint/admission boundary should include only fields that can
affect deterministic gameplay. Decide the classification for mixed registries so
presentation edits do not force mechanical rebases, while mechanical edits cannot
bypass rollback admission. Record the rule per registry owner rather than
maintaining one ad-hoc exclusion list.

⭐⭐ **THE ROLLBACK REGISTRY IS THE CONCRETE CASE, AND IT IS ALREADY COSTING US A
CORRECT DECLARATION.** `RollbackRegistry::schema_dump()` emits
`name \t kind \t wire-type \t detail`, and `compute_schema_fingerprint` hashes the
whole dump — so the PROSE is inside the snapshot schema's identity, which
`ActiveRollbackAuthority::installed` uses as the timeline's contract. Rewording a
sentence moves the identity.

⛔⛤ **THE BILL HAS BEEN PAID ONCE ALREADY, IN A COMMENT THAT SAYS SO.**
`game/ambition_app/tests/rollback_coverage.rs:1208` waives two demo view
components from rollback coverage *"rather than DECLARED DERIVED, deliberately.
A derived declaration's reason string is hashed into `schema_fingerprint`, so it
would put a demo's exhibit into the engine's wire format and owe a version bump
every time somebody reworded it."* That is the defect choosing the architecture:
the cheaper-but-weaker declaration won because the correct one carried a prose
tax. This is not a hypothetical.

⚠ **AND THE OBVIOUS FIX — DROP `detail` FROM THE FINGERPRINT — LOSES REAL REACH.**
RE-MEASURED 2026-09-19 over the committed baseline
(`game/ambition_app/tests/rollback_schema_baseline.txt`, `ggrs-rollback-schema-v199`),
**491 data rows in 17 kinds**, by kind:

| | kinds | rows | what `detail` adds |
|---|---|---|---|
| uniform | 10 | 220 | nothing — one sentence per kind, derivable from the `kind` column beside it |
| varying | 7 | 271 | facts `kind` does not encode |

⛤ This read `493 / 225 / 268` from 2026-09-16. ⚠ **AND THE `493` WAS NEVER A ROW
COUNT**: the file's first line is the version header `ggrs-rollback-schema-v199`,
so a line count overstates the rows by one, and the schema has moved since. ⇒ The
shape of the argument is unchanged — the varying half is the larger one — and
only its size moved.

The varying half is load-bearing: `component-clone` alone carries five sentences
distinguishing *entity handle remapped* from *entity SET remapped* from *keyed
entity MAP remapped*; `resource-canonical` splits *identical* from
*presence-aware* canonical checksum projection; and 22 `resource-clone-custom-checksum`
rows each name what their `fn(&T) -> u64` actually covers. Excluding `detail`
wholesale would stop the fingerprint seeing an entity-remapping change.

⛔⛤ **THE EXCHANGE RATE, MEASURED BY POISON.** Pluralising ONE WORD in
`detail::MESSAGE_CLEAR` — *"message buffer"* → *"message buffers"* — turned
`the_rollback_schema_matches_its_recorded_baseline` red with **166 diff lines, 83
added and 83 removed**: every `message-clear` row in the schema. One letter of
English moved 83 rows of peer-visible snapshot identity. That is the size of the
tax `rollback_coverage.rs:1208` declined to pay.

⇒ **The shape that fits the measurement is a SPLIT, not an exclusion:** a
mechanical `detail` that stays in the fingerprint and a `coverage`/reason note
that does not. The decision this needs is where the line falls — specifically
whether a *reason* (why a type is derived, or which other projection covers it)
is ever allowed to be part of peer-visible identity.

⭐⭐ **AND THE SPLIT IS NO LONGER HYPOTHETICAL: IT IS IMPLEMENTED, GREEN AND
POISONED — in the tooling lane, not in the fingerprint.** Landed 2026-09-16 with
`the-peer-visible-schema-may-not-move-without-the-version`, which needed the same
question answered for the 144 checksum-feeding rows and answered it from the
artifact rather than by a hand-drawn line:

> **`detail` is kept exactly where it DISTINGUISHES rows of the same kind, and
> dropped where it does not.** A kind whose rows all carry one sentence has a
> `detail` the `kind` column already implies; a kind whose rows differ is using
> it to say something `kind` cannot.

48 of the 144 carry theirs. It covers both halves this page measured: the 22
`resource-clone-custom-checksum` projections AND the 18 `resource-canonical` rows
where `rollback_resource_optional_canonical` adds a presence term under an
unchanged name/kind/type. The control and the positive differ only in their
subject — rewording the 7 uniform `component-clone-cursor` rows stays green,
rewording one of the 22 varying ones reddens — and both arms assert their anchor
count first, because a reword aimed at a uniform kind matched 0 rows on the first
attempt and printed the same green a no-op does.

⇒ **WHAT THIS DOES AND DOES NOT DECIDE.** It does not touch
`compute_schema_fingerprint`, so the repository now answers this question two
opposite ways in two places, which is a second witness rather than a resolution.
What it removes from the ruling is the doubt about feasibility: the line this
page proposes drawing by hand can be derived, and a guard drawing it that way has
been running green for a day. ⚠ Its honest cost, also measured: a genuine reword
of a VARYING kind's sentence still reddens even when no projection changed — 48
rows of exposure instead of 493, failing in the safe direction.

✔ **ONE OF THOSE SENTENCES WAS NOT A MECHANICAL FACT AT ALL, AND IT IS GONE —
SCHEMA v194.** *"State checksum supplied by another authoritative projection"*
was recorded on **99 rows** (94 `component-clone`, 5 `resource-clone`) by two
registrar methods whose only bound is `T: Clone`. Nothing in either method could
establish that another projection covers the type; the KIND was asserting what
only the TYPE can know. `detail::CLONE_UNHASHED` now says *"not in the session
checksum"*, which is `feeds_peer_checksum() == false` stated plainly.

⭐⭐ **AND IT IS NOW FALSIFIED BY A WITNESS, NOT ONLY BY THE METHOD SIGNATURE.**
Everything above is structural: `rollback_component_clone`'s bound is `T: Clone`,
so it CANNOT establish the claim. `57590f4b3` measured it false in a named case.
`a_bag_changed_from_update_is_silently_taken_back_by_the_rewind`
(`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`) grants an
item from outside the rewinding schedule — the shape `dispatch_menu_action` makes
when it equips — and the next rewind takes it back. `OwnedItems` is
`rollback_resource_clone`; `OwnedItemsBaseline` IS
`rollback_resource_clone_checksum`, so it is exactly the "other authoritative
projection" the sentence gestured at — and it did not catch the unhashed value
going away. `session_health` was clean on all 240 frames.

⚠ **THE WITNESS'S LIMIT, STATED BY ITS AUTHOR AND NOT TO BE CITED PAST IT:** the
sync-test harness is ONE peer replaying itself, so a write erased identically on
every replay produces no mismatch to detect. What is established is the LOCAL
LOSS across 240 frames, and that a hashed sibling projection did not cover an
unhashed value. It is not a witness about two peers disagreeing.

ⓘ **AND ITS COMPANION ARM KILLS AN ADJACENT REASSURANCE**, which matters here
because this question is about what a fingerprint is FOR.
`a_reset_requested_from_update_mid_window` set `NewGameResetRequested` — which is
`rollback_resource_canonical` and therefore DOES feed the peer checksum — from
`Update` mid-window, and it behaved exactly like the unhashed resource: the room
was not rebuilt, 7 of 7 roster entities survived, `session_health` clean for 180
frames. ⇒ **A checksum cannot disagree about a value that was put back before it
was taken.** Registration kind predicts whether a SURVIVING divergence is caught;
it says nothing about a write that does not survive to be hashed. Measured by
CalculexAmbition, who also retracted the opposite prediction they had published.

⛔ **AND THE PRICE OF THAT CORRECTION IS THE ARGUMENT FOR THIS QUESTION.**
Deleting a claim nobody could support cost a `GGRS_ROLLBACK_SCHEMA_VERSION` bump
and a baseline rewrite, because the fingerprint hashes the sentence. The v194 log
entry says so in the changelog's own voice: *"nothing mechanical changed, and that
is the point of this entry"*. It is the only non-mechanical entry in a log of
layout and projection changes, and a reader comparing v193 to v194 across two
peers learns nothing about whether their snapshots are compatible.

ⓘ It was paid ONCE, not 99 times — one bump covered all 99 rows. The tax is per
EDIT, not per row, which is why the defect is survivable and also why it goes
unnoticed.

⛔⛤ **AND THE SECOND ARGUMENT, WHICH IS NOT HYPOTHETICAL: THE WELD BLOCKED A REAL
REPAIR AND FORCED A NEW API.** Found 2026-09-16 while measuring S7's rows. The
localization probes are the diagnostic that says WHERE a desync is. A row
registered through `rollback_component_clone` gets a PRESENCE probe, whose census
hard-codes `xor: 0` — a carrier count, blind to the value. To measure whether one
of those rows' floats actually differ across a rewind, that probe has to be given
a value projection, and there was no road to do it:

- `record_probe` is private to `registration.rs`;
- `RollbackChecksumProbes::probes` is a private field with no public push;
- so the only remaining route was to edit the REGISTRATION SITE, which changes
  that row's `detail`, which changes `schema_dump()`, which changes
  `compute_schema_fingerprint`.

⇒ **STRENGTHENING A PURELY LOCAL DIAGNOSTIC WOULD HAVE CHANGED THE IDENTITY TWO
PEERS COMPARE.** A probe contributes nothing to the GGRS aggregate; its strength
is a fact about what a developer can see, and it was welded to peer-visible
snapshot identity. That is this question's shape without any appeal to somebody
rewording a comment — it is a repair that was blocked, today, by the coupling.

The workaround is `RollbackChecksumProbes::strengthen_with::<T>(projection)`:
probe strength is owned by the collection at runtime, and registration is how it
is INITIALIZED rather than where it is decided. That keeps ONE owner and needs no
decision here. ⚠ **It does not answer this question, it routes around it** — the
`detail` column still carries prose into the fingerprint, and the next diagnostic
that is not expressible as a runtime override will hit the same wall.

**THE DECISION, WITH WHAT EACH OPTION COSTS.** ⚠ Every figure below comes from
this row's own measurements; none of these is a recommendation.

* **(a) Keep hashing the whole dump** — today's behaviour, written down. Prose
  is part of the wire format and rewording a sentence bumps the timeline's
  contract. ⛔ The cost is not hypothetical and is recorded twice above: a
  correct `DECLARED DERIVED` was passed over for a weaker waiver to avoid the
  prose tax, and a purely local diagnostic strengthening was blocked because
  the only route to it ran through the registration site.
* **(b) Drop `detail` from the fingerprint** — cheapest, and it loses reach the
  measurement can price: 220 rows in 10 kinds lose nothing, and **271 rows in 7
  kinds carry facts the `kind` column does not encode**. Those stop being part
  of snapshot identity, so two builds differing only in them compare equal.
* **(c) Split `detail` into a mechanical part and a prose part, and hash only
  the mechanical one** — what this question's own first paragraph asks for
  (*"record the rule per registry owner"*). The work is the 7 varying kinds,
  and their internal spread is the size: `derived` alone holds **42 distinct
  sentences across 44 rows**, so it is nearly one-per-row and cannot be
  collapsed to a per-kind constant.
* **(d) Keep hashing `detail` but forbid prose IN it** — make the column a
  structured value rather than a sentence, so rewording is unspellable instead
  of merely discouraged. Strongest and most invasive: the same 42 `derived`
  reason strings are the largest single conversion, and it is the only option
  under which a future diagnostic cannot re-create this coupling by accident.

⚠ **(b) AND (c) ARE NOT THE SAME ANSWER AT DIFFERENT PRICES.** (b) removes 271
rows' worth of facts from peer-visible identity; (c) keeps them and pays to
classify them. Whether those facts BELONG in peer identity is the actual
question, and the row count does not answer it.

ⓘ **WHAT DID NOT NEED A DECISION AND HAS LANDED:** those 15 sentences were spelled
TWICE — once in `SchemaRollbackRegistrar` (the metadata recorder) and once in
`rollback_ggrs`'s installing registrar — with nothing comparing the copies. They
agreed only because nobody had reworded one. They now live in
`ambition_platformer2d_runtime::rollback::detail` and both registrars reference
them, dump byte-identical. That collapse is what makes any answer here a
one-place edit instead of a two-crate one.

## Q128 — should the simulation tick be rebased when peers agree to start, or stay an absolute per-App count?

**The last open road of the ID-PEER campaign, and the only one that cannot be
closed by engineering alone.** ⭐ **AND "LAST" IS NOW A MEASUREMENT RATHER THAN A
FIGURE OF SPEECH, 2026-09-17.** Two hosts that reach the shipped Ambition route
by different shell histories now agree on **144 of the 146 real GGRS
`ChecksumPart`s**, and the two that differ are this question and `Q129`. It was
59 of 146 the same morning, before the rollback carrier ordering was rebased at
frame zero. So a ruling here is not one improvement among many: with `Q129` it is
the whole remaining peer-visible difference between two hosts whose canonical
identities and values are identical
(`two_local_histories_compute_the_same_ggrs_component_checksums`, which asserts
BOTH still differ, so this line cannot go stale in the quiet direction). `ambition_time::SimTick` is registered
`resource-canonical`, so its ABSOLUTE value is inside the checksum two peers
compare. Measured 2026-09-15: one writer (`advance_sim_tick`, `+1` per step),
`init_resource`'d once at App build, never rebased anywhere in the workspace, and
sitting unconditionally at the head of the sim schedule — so it counts menu
frames. ⇒ Two Apps that have been running for different lengths of time disagree
about `sim_tick` from the first compared frame, before anything else in that
campaign matters.

⛔ **IT CANNOT BE CLOSED THE WAY THE OTHER NINE WERE.** Every one of those was a
projection or an ownership move: exclude the local term, or make the stale value
impossible. A projection that excluded the tick would exclude the TIMELINE
ITSELF, which is the one thing a rollback comparison is about. What is needed is
a session-relative tick, rebased at the moment peers agree to start — and where
that agreement comes from is a netcode decision, not a refactor.

⚠ **NOTHING IN THE REPOSITORY CAN CURRENTLY OBSERVE THE DEFECT.** Re-derived
2026-09-16 rather than carried: `Session::SyncTest` is constructed in **exactly one
place** in this workspace (`ambition_platformer2d_rollback_ggrs/src/session.rs`),
and `Session::P2P` appears **exactly once**, in a match arm reading
`confirmed_frame()` — so no P2P session is ever built. One machine rewinding
itself, zero distance; a canary comparing a machine against its own past is
structurally incapable of catching a two-peer disagreement. Every leak in that
campaign had to be found by reading. So this will not announce itself, and it does
not get more urgent on its own.

⛔⛤ **BUT OPTION (b) NOW DEFERS TWO POPULATIONS, NOT ONE, AND THAT IS NEW SINCE
THIS WAS WRITTEN.** The ID-PEER campaign's twelfth road is **S7's 25 rows** —
registered rollback state that is outside the session checksum, read by an
unfiltered per-tick query, and float-bearing, twelve of them mutably written in
production (`engine/simulation-authority-and-determinism.md`). Those rows carry no
host-local id, so no projection closes them and no ownership move closes them;
they are simply never compared between peers, and **the same absent session is the
only thing that could ask whether two peers agree about them.** Two of the twelve
were measured clean 2026-09-16 and that clears them of a LOCAL RESTORE defect and
nothing else — a value nothing compares between peers is reproducible locally and
divergent across peers at the same time.

⇒ So "(b) keep it absolute and accept that peer comparison waits for real
sessions" is a bet on one absent session covering the tick AND 25 ranked
float-bearing rows AND both halves of `SETTINGS-ROLLBACK`'s policy resources. That
does not make (a) right; it makes the price of (b) larger than the paragraph above
it implies, and the price was not visible when it was written.

The choice: (a) rebase the tick at an agreed session start, which means deciding
what "agreed" is before there is a handshake to carry it; (b) keep it absolute
and accept that peer comparison waits for real sessions, recording it as a known
hole rather than an oversight; (c) project it out and replace the timeline term
with something else, which nobody has proposed a shape for. Recorded by the
`queue.md` ID-PEER table, which names this as one of two roads still open — the
other is `Q122` above, the snapshot schema fingerprint hashing prose.

## Q131 — ⇒ THE SAME QUESTION AS `Q139`. ASK IT THERE.

⛔⛤ **THE SECOND DUPLICATED RULING FOUND ON THIS PAGE ON 2026-09-18, AND THE
PAIR IS THE SAME SHAPE AS `Q130`/`Q138`.** *"How should a presentation system
that writes `Transform` declare itself?"* and `Q139`'s *"what declares that a
presentation system writes `Transform`?"* are one question about one guard
exclusion, with the same owner row
([ROLLBACK-MUTATOR-POPULATION](queue.md#rollback-mutator-population--the-mutator-guard-sees-a-quarter-of-rollback-state))
and the same options. This one was written first; `Q139` carries the
re-measurements and is the live row.

⚠ **ITS NUMBERS ARE ALL SUPERSEDED, AND ONE OF THEM MOVED FOR A REASON WORTH
KEEPING.** It said the guard *"now covers 338 types, having excluded exactly
one"* — re-derived 2026-09-18, **342**, and `Transform` is still the one
exclusion. It said *"of 64 offenders, 52 are `Transform` writes"*; `Q139` reads
52 of 60, the numerator unchanged and the denominator moved by repairs. It said
the marker-property classifier covers *"23 of the 52"*; `Q139` reads 36 of 52
against the same marker list and 47 against a wider one, because the carve work
gave those systems the presentation components they were always projecting to.
⇒ Every one of those is now a `Q139` row.

⭐ **WHAT IT CARRIED THAT `Q139` HAD LOST: A FOURTH OPTION.** *"(d) leave the
exclusion and accept `Transform` as a permanent blind spot, which is today's
state written down honestly"*, with the argument that it *"is a real option and
should not be dismissed"*. `Q139` listed only (a), (b) and (c). Restored there.
⇒ That is the cost of a duplicated ruling that nobody notices is duplicated:
the two copies drift, and the maintainer reads whichever one they land on.

## Q130 — ⇒ THE SAME QUESTION AS `Q138`. ASK IT THERE.

⛔⛤ **TWO ROWS ON THIS PAGE ASKED ONE MAINTAINER TO DECIDE ONE THING, WITH
DIFFERENT NUMBERS — FOUND 2026-09-18.** *"Should the sim harness refuse to step
an invalidated rollback session?"* and `Q138`'s *"should
`Platformer2dSimHarness::step` refuse to step an invalidated session?"* are the
same question about the same method, with the same three options in the same
order, pointing at the same owner row
([ROLLBACK-DEAD-SESSION](queue.md#rollback-dead-session--an-invalidated-ggrs-session-stops-the-clock-in-silence)).
This one was written first and its census is two generations behind: **21
fixtures, thirteen reading a health API**, against a population that is 31 / 17
adjudicated-and-exempt today. ⇒ `Q138` is the live row. Nothing here contradicts
it; everything here was older.

⛔ **AND ITS ONE ARGUMENT AGAINST ACTING WAS REFUTED BY BUILDING THE THING.**
This row said *"a guard script cannot substitute … the property is not decidable
by reading source. If the contract moves into `step`, no guard is needed; if it
does not, no guard can be written."* The first half is true and `Q138`'s guard
agrees with it — `scripts/a_rollback_arm_must_refuse_a_frozen_world.py` does not
decide the property. It ROUTES the decision: a new sync-test arm either reads
the health API or arrives with a sentence naming what a frozen world breaks in
it. *"No guard can decide this"* and *"no guard can help"* are different claims,
and only the first was ever true.

⭐ **WHAT THIS ROW UNIQUELY CARRIED, KEPT BECAUSE `Q138` DOES NOT HAVE IT: THE
CODEBASE ALREADY ANSWERS A NEIGHBOURING QUESTION IN THE LOUDEST DIRECTION.** For
a two-root world the headless path does not hand back an uninterpretable reading
— it ABORTS. `unique_session_world_root` carries a plain
`assert!(roots.next().is_none(), "more than one canonical SessionRoot exists")`
(`crates/ambition_platformer2d_shared_tangle/src/lifecycle/session.rs:424`),
ungated and live in release, and `live_session_world_root` falls through to it
whenever `SessionGatedSimulation` is absent — direct entry and headless, which
is every harness `Q138` is about. The shell-routed branch resolves the same
condition by scope, silently. ⇒ *"The harness refuses rather than hands back a
reading nobody can interpret"* is already precedent here, and it is stronger
than anything either row proposes. ✔ Re-checked 2026-09-18: the assert is still
there and still ungated. (ToothbrushAmbition's find, filed on their side as part
of `Q132`, which was DECIDED 2026-09-19: exactly one canonical live
`SessionRoot`, so this assert states the invariant rather than guessing at it —
see [`maintainer-decisions.md`](maintainer-decisions.md).) ⚠ A practical note for anyone reading a failure from it: it fires
inside a helper, so the arm named in the output is the last one that ran, not
necessarily the one at fault.

⚠ **AND ONE GENERAL LESSON, WHICH WAS NEVER EVIDENCE FOR THIS RULING: A TYPE
ALIAS MAKES A POPULATION INVISIBLE TO A SCAN KEYED ON WHAT IT EXPANDS TO.** A
first report said *"~206 `Single<.., With<SessionRoot>>` sites"*, which was raw
grep MENTIONS including comments and tests. A peer counted **11**
`Single<..SessionRoot..>` out of **16** `Single<` parameter sites in the whole
workspace and could not reach 206 — correctly, because `SessionWorldRef` and
`SessionWorldMut` are `pub type` ALIASES for `Single<..>`, so a scan keyed on
the word `Single` cannot see any of their uses. Two honest scans of one tree
disagreed by an order of magnitude for that reason alone. ⇒ Re-derived
2026-09-18 with comments stripped and tests excluded: **167 `SessionWorldRef<`
plus 16 `SessionWorldMut<` = 183 production uses across 95 files** (it was
163 + 22 = 185 across 105 when written). The direct `Single<..SessionRoot..>`
spellings are additional.

## Q129 — must the save file be part of what two peers agree on?

**MEASURED 2026-09-16, and unlike Q128 this one announces itself today.** A bag
that changes once per tick desyncs a GGRS sync test within six ticks. ⭐ **AND A
SECOND, INDEPENDENT MEASUREMENT REACHED IT FROM THE OTHER SIDE ON 2026-09-17:**
two hosts that reach the shipped Ambition route by different shell histories now
agree on 144 of the 146 real GGRS `ChecksumPart`s, and the two that differ are
this question and `Q128`. Different instrument, different population — one App
rewinding itself against one bag, versus two Apps with different route histories
compared whole — and the same row. ⇒ A ruling here and on `Q128` is the whole
remaining peer-visible difference between two hosts whose canonical identities
and values are identical. The chain, AS IT STOOD WHEN THIS WAS FILED:
`persist_inventory_to_save` sat in top-level `Update` and wrote the live bag
into `AmbitionGameSave` once per FRAME; `AmbitionGameSave` is registered
`rollback_resource_clone_checksum`, so its value is compared once per TICK; and a
rewind re-simulates ticks without re-running `Update`. The hashed save therefore
described a different frame from the tick it was compared at. Of 364 probed
rollback entries, exactly ONE differed between a run whose bag moves and an
otherwise identical run whose bag does not, and it was this one.
✅ **THAT PLACEMENT IS REPAIRED and the question is not.** Re-derived 2026-09-18:
all three `persist_*_to_save` mirrors are registered through `app.sim_schedule()`
(the table above counts them on the sim side), the divergence set is empty, and
`resources_crossing_the_rewind_boundary.py` reports `AmbitionGameSave` does not
cross the rewind boundary. ⇒ What remains is the OWNERSHIP question this entry is
named for, and the two-host measurement above is the reason to answer it — not a
live desync.

⛔ **A SYNC TEST IS ONE MACHINE REWINDING ITSELF, WHICH IS WHY THIS MATTERS
NOW.** No second peer is required for the divergence — a single App already
disagrees with its own replay. Every road that changes a bag during play crosses
this: a pickup, a shop sale, a drop. ⭐⭐ **IT IS NOT CADENCE AND IT IS NOT SUSTAINED CHANGE — IT IS THE FIRST THREE
TICKS.** Sweeping the tick at which an every-tick grant STARTS, 120 steps each:

| grants every tick from | end tick | bag | health |
|---|---|---|---|
| 1 | 6 | 8 | ⛔ `Err(mismatch at [2, 3, 4, …])` |
| 2 | 6 | 7 | ⛔ `Err(mismatch at [2, 3, 4, …])` |
| 4 | 121 | 120 | `Ok` |
| 6 | 121 | 118 | `Ok` |
| 8, 12, 16, 20 | 121 | 116, 112, 108, 104 | `Ok` |

⇒ **A BAG CHANGING ON EVERY ONE OF 118 CONSECUTIVE TICKS IS CLEAN IF IT STARTS AT
TICK 4.** The defect lives entirely in the first three ticks, and `check_distance`
is 4 — the mismatch is reported at frames `[2, 3, 4]`, which is the window before
the session has a full rollback history behind it.

⚠ **THIS IS THE THIRD FRAMING OF THIS ROW AND EACH ONE WAS MEASURED.** First "a
per-tick change desyncs", then "sustained change desyncs and a single change does
not" — which survived a floor check and was still wrong, because the two cases
differed in START TICK as well as in cadence and I had varied both at once. A
cadence sweep (N consecutive grants from tick 20, N ∈ {1,2,3,4,5,8}) came back
clean at every N, which is what said cadence was not the variable at all.

⛔ **TWO MECHANISMS PROPOSED AND BOTH REFUTED.** YardratAmbition offered a pair
with OPPOSITE predictions, which is the right shape — a mechanism that explains a
number is not evidence for it. **(A) ACCUMULATION:** `grant` makes the bag
`3 + (times the system RAN)` rather than a function of the frame, and a
resimulation re-executes steps; predicts that a pure function of the tick runs
clean. **(B) THE ONE-UPDATE LAG:** the mirror runs in `Update` once per
`app.update()` while GGRS snapshots inside the sim schedule, so a frame's snapshot
holds the save as of the previous update; predicts that a pure function of the
tick still desyncs.

Measured, 120 steps each, writing `take(all)` then `grant(tick % 5)` so the value
cannot depend on how many times the system ran:

| write | end tick | peak bag | health |
|---|---|---|---|
| pure `f(tick)`, every tick from 1 | 6 | 4 | ⛔ mismatch |
| pure `f(tick)`, every tick from 4 | 121 | 4 | `Ok` |
| pure `f(tick)`, once at 20 | 121 | 3 | `Ok` |
| accumulating `grant`, every tick from 1 | 6 | 10 | ⛔ mismatch |

⇒ **(A) IS DEAD** — a pure function of the tick desyncs exactly as the
accumulating write does, so the arithmetic is irrelevant. ⇒ **(B) IS DEAD TOO**
— it predicts a desync from tick 4, and tick 4 is clean. The only variable that
predicts the outcome remains the START TICK, and no proposed mechanism yet
explains why the first three ticks are different.

⚠ The `peak` column exists because `tick % 5` is zero once every five ticks, so
the FINAL bag reads 0 both when the write ran and when it never ran at all. The
high-water mark separates them; without it the "from 4" row would have been a
clean result from a write nobody had shown fired. ⚠⚠ And the first version of
this experiment compared a write starting at tick 1 against a control firing at
tick 20 — varying start tick alongside arithmetic, the SAME confound that made
the sustained-versus-single framing wrong two hours earlier. The "from 4" row is
the repair, and it was added before the result was written down rather than after.

⛔ **A THIRD CANDIDATE, MINE, ALSO REFUTED — AND BEFORE IT WAS ASSERTED.**
`complete_durable_restore` sets `SaveRestored` once from `Update` on the first
frame a primary player body exists, and that latch gates whether the mirrors
write at all; it is `rollback_resource_clone`, so a rewind into a frame where it
was still false would let the latch re-open and run the system a second time.
That shape would make ticks 1..=3 special and everything after boring. ⇒ It is
wrong: measured, `SaveRestored` reads `true` at step 0 of both a desyncing run
and a clean one and never moves. The latch has already settled before the window
opens.

⭐⭐ **BUT THE SAME PROBE PRODUCED THE SHARPEST FACT IN THIS ROW, and it is two
independent measurements agreeing.** The save's census in a desyncing run, per
tick: `0xce4e4758…` at tick 1, then `0x4f52c70a…`, `0x8cf64e57…`, `0xe2f498aa…`,
`0xb8f85fb1…`, `0xd41e15e0…` — a new value every tick. YardratAmbition's
`RollbackRestoreAudit`, reading the resimulation from inside one run, reports
frames 2, 3 and 4 each diverging with **the replay xor CONSTANT at
`0xce4e4758…`** while the first-pass xor moves every frame. ⇒ **THE REPLAY OF
EVERY COMPARED FRAME SEES THE SAVE AS IT WAS AT TICK 1.** Two different
instruments, two sessions, one number.

⚠ **A FOURTH CANDIDATE, PARTLY CONSTRAINED.** YardratAmbition's: the frame-1
lifecycle trace shows roots admitted, a candidate session published and entities
promoted, so ticks 1..=3 might be special because the ENTITY POPULATION is still
settling — a structural property of the window rather than of anything a test
writes. ⇒ Measured against the room this row uses, with NO writer installed: the
`FeatureSimEntity` roster reads 7 at tick 1 and 7 at every tick through 13, never
changing. So that population is already settled before the first observable tick.
⚠ **That constrains the candidate without killing it** — `feature_roster` counts
one population, and session roots, promoted entities and custody holders are not
in it. If the window's specialness is about entities, it is not about these.

⛔ AND ANY SURVIVING VERSION MUST SATISFY A CONSTRAINT ALREADY MEASURED: a system
granting ZERO every tick from tick 1 — same `ResMut<OwnedItems>`, same schedule
position, same change detection, value unchanged — is CLEAN. So the window alone
is never sufficient. The property is a conjunction: a CHANGED hashed value during
a window that is still settling in some way not yet identified.

⚠ What that does NOT yet explain is why a change starting at tick 4 is clean. If
the replay always read a stale save, a change at tick 20 would diverge too. ⇒ The
honest reading is that the replay holds whatever the restore point carried and
the sampled window was ticks 2–4, so "the restore point is tick 1" and "the
restore point is stale by a fixed amount" are not yet separated. That is the next
measurement, and it wants the audit pointed at a window starting well after tick
4 rather than another hypothesis.

⇒ **SO THE PRACTICAL SEVERITY IS MUCH LOWER THAN THE FIRST TWO FRAMINGS SAID.** A
pickup, a shop sale or a drop during play does not desync — measured, not
inferred. What desyncs is inventory changing in the session's first three ticks,
which is startup: a save restore, an opening script, a debug grant at boot.
⛔ That is still a real hole and still wants the ruling below, because the repair
is the same and because "do not touch the bag for the first three ticks" is not a
contract anything states or checks.

⚠ **AND THE ROLLBACK ARMS DO NOT EXERCISE IT**, corrected twice. I first wrote
that they never change inventory in a rewinding window, then retracted that on
finding `grant_pickup` writes `OwnedItems` and `carried_item_crosses_rooms` picks
items up. ToothbrushAmbition measured the retraction and it was the over-correction:
that file's only `with_sync_test_rollback_settings` arm is
`a_mount_dying_under_a_possession_survives_rewinds`, which does not touch the bag,
and every `pick_it_up` caller in it is in a `fixed_60hz_room_sim` arm with no GGRS
session. The file matches a grep for both terms because it holds both KINDS of arm.
⇒ The first claim was right and the correction was wrong, and neither was measured
when written.

⛔⛤ **THE CLEAN RESULTS ABOVE ARE WEAKER THAN THEY LOOK, AND THIS REFRAMES THE
WHOLE ROW — MEASURED 2026-09-16 BY YardratAmbition, RE-RUN HERE.** Every "clean"
verdict in the tables above is a comparison that came back equal. But over that
same window the save's HASHED PROJECTION barely moves.
`probe_how_much_of_the_peer_checksum_actually_varies`, run at `f96493e31`: of the
**144** entries whose kind feeds the peer checksum,
`ambition_persistence::save::AmbitionGameSave` shows **2 distinct censuses**,
against **238** for each of its nine busiest neighbours (`SimTick`,
`BodyKinematics`, `ActorPose`, `CenteredAabb`, `MotionModel`, `SweepSample`,
`BodyLifetime`, `GameplayElapsed`, `PlayerProjectileState`). The sibling file's
header puts it more sharply still: exactly one value across every frame GGRS
saved twice, while the LIVE save reaches 247 mirrored items.

⇒ **SO "A PICKUP DURING PLAY DOES NOT DESYNC" IS TRUE FOR A REASON THAT MAKES IT
WORSE, NOT BETTER.** A comparison that cannot differ cannot fail. The 120-step
runs that came back `Ok` with a bag reaching 120 did not show the mechanism is
benign during play; they show that during play the save is not effectively being
compared at all. ⛔ The severity paragraph below stands as a statement about
OBSERVED desyncs and must not be read as a statement about coverage.

⚠ **WHICH MAKES A PRIOR QUESTION, AND IT IS ARGUABLY THE ONE TO ANSWER FIRST:
is a registered-but-pinned hashed entry a defect in the REGISTRATION, a defect in
the SNAPSHOT ROAD, or an intended property nobody wrote down?** `AmbitionGameSave`
is in the peer contract by registration and out of it in effect. Whichever of
(a)/(b)/(c) is chosen, that stays true unless the projection itself changes —
and if the answer is "intended", then (b) is closer to describing the tree as it
already behaves than to changing it.

⚠ **THREE CAVEATS, CARRIED BECAUSE THE NUMBERS INVITE A WRONG READING.**
(1) **116 entries are constant under BOTH idle and play, and that is NOT a defect
list** — a component nobody spawns in this room, a resource only a boss
encounter writes, and a genuinely frozen projection all land in the same bucket;
separating them needs the live value read beside the census, which no instrument
does for 144 types. (2) Playing rather than idling wakes **15** entries that were
constant while idle, so the bucket is a property of the exercise as much as of
the registration. (3) The probe reports the SIZE of each bucket, which is what
nobody had — not which members are wrong.

⛔⛤ **THE 2026-09-16 MERGED-STATE REVIEW REFUSES (b) OUTRIGHT AND RATES THE
DEFECT P0, ABOVE CONSOLIDATION WORK.** Its words: *"Do not fix this by simply
removing `AmbitionGameSave` from the checksum. The night's later census
invalidated that tempting solution."* ⇒ So the choice below is preserved for its
reasoning, but (b) is no longer live: it *"would make the immediate test green by
throwing away comparison coverage for substantial simulation state"*, which is
this ledger's own 13-of-19 count read back to it.

⇒ **The direction it prefers is (a)'s smallest form** — the three live→save
mirrors cross the same rollback boundary as the state they mirror, so a replay
can reproduce them, with disk I/O and autosave staying outside the simulation and
the layering *rollback-owned durable mechanical representation → confirmed/local
persistence projection → disk*. ⛔ And explicitly NOT the larger "is
`AmbitionGameSave` both simulation authority and disk representation" split
before the replay defect is fixed.

✅ **THE REPAIR LANDED 2026-09-16 AND THIS Q IS NARROWER FOR IT.** The three
mirrors now register through `app.sim_schedule()`; the divergence set is empty
and the projection moved from 1 distinct census to 236 across the compared
frames. ⇒ So the desync is no longer the reason to answer this question, and the
pinned-projection half of it is ANSWERED for the save: it was pinned BECAUSE the
mirrors wrote from `Update`, and it tracks now that they do not. What remains is
the ownership question on its own merits — should a save FILE be part of what two
peers agree on — plus the general form of the prior question, which the save no
longer instantiates: whether any OTHER registered-but-pinned hashed entry exists,
and whether that is a defect in the registration, in the snapshot road, or an
intended property nobody wrote down. The 116-constant-under-both bucket is where
that would be looked for, and it is still not a defect list.

⚠ **IT ALSO SHARPENED THE ACCEPTANCE, AND THE SHARPENING WAS THIS Q's OWN PINNED
PROJECTION.** Verbatim: *"I would not accept merely: startup repro now passes"* —
because the reason the mismatch manifests primarily in the opening few ticks is
still unexplained, and the registered checksum *"barely changes during some long
play windows even when the live save changes substantially"*. ⇒ Acceptance must
show a representative in-simulation save mutation is genuinely being COMPARED
across repeated snapshots, *"rather than the checksum becoming accidentally
pinned and therefore incapable of disagreement"*. That is the 2-against-238
measurement above, arrived at independently, and it means the prior question is
not optional bookkeeping: a repair validated against a pinned projection would
report success from a comparison that cannot fail.

The choice: (a) derive the save inside the sim schedule so a rewind re-derives
it, which makes a persistence mirror into simulation work and raises the cost of
every rewind; (b) take `AmbitionGameSave` out of the peer checksum, on the ground
that a save FILE is a local artifact and not simulation authority two peers must
agree on; (c) keep both and gate the mirror so it only runs on confirmed frames,
which needs a confirmed-frame hook the `Update` schedule does not currently have.

⛔ **(b) LOOKED SMALLEST UNTIL THE WRITERS WERE COUNTED, AND THE COUNT ARGUES
AGAINST IT.** The test is whether anything OTHER than the three `Update` mirrors
writes `AmbitionGameSave` inside a REWINDING schedule — because if nothing does,
taking it out of the checksum costs only the coverage those mirrors never
honestly provided, while if something does, (b) silently drops a real guarantee.
**Something does.** `apply_flag_effects` takes `ResMut<AmbitionGameSave>`
(`crates/ambition_platformer2d_actor_monolith/src/features/ecs/effect_bus.rs:18`)
and is registered through `app.sim_schedule()`
(`crates/ambition_platformer2d_actor_monolith/src/features/mod.rs:204`). A story
flag set by the simulation lands in the save inside the rewinding schedule, where
the checksum is doing real work. ⇒ On that evidence (a) is the honest option and
(b) trades a defect for a blind spot.

**AND THE CENSUS IS NOW DONE: 18 OF THE 19 WRITERS ARE IN A REWINDING SCHEDULE —
RE-DERIVED 2026-09-18.** Every system taking `ResMut<AmbitionGameSave>`
workspace-wide, resolved to the `add_systems` call that registers it and that
call's first argument:

| registered in the sim schedule (18) | not (1) |
|---|---|
| `apply_flag_effects`, `apply_quest_advance_events`, `apply_wave_encounter_effects`, `capture_falling_sand_switch_interactions`, `celebrate_symmetry_attunement`, `count_the_dialogue_visit_when_a_conversation_opens`, `drain_switch_activations`, `drive_wave_encounters`, `grant_quest_completion_rewards`, `heal_save_shrine_system`, `persist_inventory_to_save`, `persist_minted_item_horizon_to_save`, `persist_occurrence_horizon_to_save`, `reset_cut_rope_attempt_on_replay`, `retire_rewards_for_rearmed_encounters`, `tick_active_cutscene`, `track_room_visits`, `update_boss_encounters` | `load_save_at_startup` (`Startup`) |

⛔⛤ **THIS TABLE READ "13 of the 19" AND NAMED SIX OUTSIDERS UNTIL 2026-09-18, AND
EVERY ONE OF THE FIVE THAT LEFT THAT COLUMN LEFT FOR A DIFFERENT REASON.** The
three `persist_*_to_save` mirrors and `track_room_visits` are registered in the
sim schedule now; `dispatch_pending_dialog_requests` left the POPULATION rather
than the column — it no longer takes the resource at all — and
`count_the_dialogue_visit_when_a_conversation_opens` arrived inside the schedule
as its replacement. ⇒ A census kept as a static table is a duplicate of the tree;
this one now carries the command that rebuilds it.

⇒ **THE SAVE IS SIMULATION STATE IN PRACTICE, WHATEVER IT IS IN PRINCIPLE**, and
the margin is no longer arguable: quest advances, boss encounter progress, switch
activations, shrine heals, cutscene ticks, the map's visit stamp, the three save
mirrors and the dialogue visit counter all write it from inside the rewinding
schedule, where the checksum is doing real work. Taking it out of the peer
contract would stop comparing all eighteen. (b) is therefore not the small
option; it is the largest one, measured by what it stops checking. ⚠ The single
outsider is `Startup`, before any timeline exists — so there is no longer a
"writes it from `Update`" tail to point at.

**Method, so the next reader redoes it rather than trusting it.**
`ResMut<'?, AmbitionGameSave>` parameter occurrences over
`multi_writer_resource_census.production_files()` with comments and test modules
stripped: 19 occurrences in 17 files, 19 distinct enclosing `fn`s. Each name is
then found inside an `add_systems(..)` call in a production file and the call's
first argument read.
⚠ Three method notes, because the count would have been wrong without them.
(1) A name inside `.after(...)` is an ORDERING EDGE, not a registration — two
systems appear in a second `add_systems` call for that reason
(`heal_save_shrine_system` at `checkpoint.rs:1802`,
`capture_falling_sand_switch_interactions` at `falling_sand.rs:141`), and both
resolve to `sim` either way, so the classification does not turn on it here.
(2) A schedule can be a PARAMETER: `track_room_visits` is registered with
`install_map_simulation_systems(app, schedule)`, and the one production caller
(`progression_schedule.rs:99`) passes `sim`. A classifier that read the callee
alone would have said "unknown" and a careless one "not sim".
(3) The three `persist_*` mirrors landing on the sim side is now the positive
control — while they were in `Update` it was the other way round, which is why
this note changed direction rather than being deleted.

⛔⛤ **AND THE SAME FACT IS ALREADY IN THE PEER CONTRACT BY A SECOND ROAD, WHICH
NARROWS THIS QUESTION — MEASURED 2026-09-18.** The bag is out of the checksum and
its BASELINE is in:

| resource | registration | in the peer checksum? |
|---|---|---|
| `OwnedItems` | `rollback_resource_clone` | **no** |
| `OwnedItemsBaseline(OwnedItems)` | `rollback_resource_clone_checksum`, projecting `to_persisted()` rows | **YES** |

⇒ `capture_owned_items_baseline` copies the live bag into the baseline on every
`CheckpointCommitted`, so **the first checkpoint commit carries the player's
stored quantities across the line this question is about.** Answering "the save
file is not peer state" by leaving `OwnedItems` unhashed does not achieve that
today.

⚠ **AND NEITHER SIDE OF THAT ASYMMETRY IS A RECORDED DECISION.** `OwnedItems` is
unhashed by KIND — `rollback_resource_clone`'s `feeds_peer_checksum()` is false —
and its registration in `ambition_items/src/rollback_registration.rs` carries no
reason at all; the baseline is hashed because somebody chose `_clone_checksum`
for it, also without a reason. ⇒ One fact, two projections, opposite answers, no
argument on either side. That is what makes it this entry's business rather than
a defect somebody can just fix.

⛔ **NOTHING CAN OBSERVE IT TODAY, AND THAT IS THE USUAL REASON.** Only
`SyncTestSession` is ever constructed — one peer replaying itself, whose two save
files are the same file — so no arm can produce two peers whose bags differ. The
same limit the sync-test witnesses elsewhere in this document state about
themselves.

⚠ **AND THE ADJACENT ASYMMETRY IS NOT THIS ONE, so do not fold them.**
`OwnedItemsBaseline` is also the one checkpoint baseline of four that is NOT in
`SessionScopedResources`, and that part IS consistent: `OwnedItems` is not
session-scoped either, so the baseline travels with the value it baselines, while
the three that do reset describe world placement. That reason is now stated at
`session/teardown.rs` beside the three, where its absence used to be a default.

Reproduction, eliminations and the full harness matrix are in
[ROLLBACK-BAG-DESYNC](queue.md#rollback-bag-desync--ambitiongamesave-disagrees-with-its-own-rollback-replay---repaired-2026-09-16-acceptance-met-the-authorityrepresentation-split-is-deferred-and-q129-is-open);
the owner document is
[DURABLE-HORIZON-CHECKSUM](queue.md#durable-horizon-checksum--the-save-mirrors-write-hashed-state-from-update).

## Q127 — are difficulty, assist and player-damage modifiers match-wide or participant-specific?

`PlayerDamagePolicy` now projects the settings values that deterministic damage
simulation consumes, so simulation no longer reads the full mutable settings
resource. The remaining policy choice is its lifetime and subject: freeze one
agreed value for the match, or publish a value per participant/seat as an
accessibility policy. Both are mechanically viable; the product rule decides the
shape of the admitted authority.

## Q136 — how does a local menu intent enter the synchronised timeline?

**Asked 2026-09-16, with both answers already measured false.**

A menu press — New Game, or using an item — is a LOCAL input event raised in
top-level `Update`. The state it has to affect is rollback-owned. Two spellings
have been measured and neither works:

| the menu writes | what happens | witness |
|---|---|---|
| `NewGameResetRequested` (`resource-canonical`) directly | **0 commits.** The restore returns the flag to `false` before `process_new_game_reset_request` sees it | `a_new_game_asked_for_from_outside_the_simulation_is_swallowed` |
| an `ItemGrantRequested`-style message (`clear_message_on_rollback`) | **no change at all.** The rewind clears the queue; nothing re-produces the request | `a_rollback_cleared_message_written_from_outside_the_simulation_is_also_lost` |

Both witnesses live in
`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`, both
carry an IN-SIM control arm that succeeds (1 commit; bag 3 → 4), and both are
poison-verified. The control is what makes the zeroes readable: a declining reset
and a composition without `apply_item_grants` print the same zero.

⭐ **HOW MANY RESOURCES THIS RULING IS RESPONSIBLE FOR IS MEASURED, NOT
ESTIMATED — BY TWO INSTRUMENTS THAT DELIBERATELY MEASURE DIFFERENT THINGS.** A
2026-09-18 review found this section describing them as one population, and a
commit message of mine claiming they *"now agree"*. They do not agree and must
not: each answers a different question, and the Q136 count is the second one's.

<!-- crossing-census: both_side_resources=57 rollback_registered=33 adjudicated_harmless=20 session_edge_only=3 filed=1 unclassified=0 -->

    scripts/resources_crossing_the_rewind_boundary.py
        BROAD CROSSING CENSUS — UNREGISTERED per-frame state written on both
        sides of the boundary. 57 both-side resources, RE-RUN 2026-09-19 and
        unchanged: 33 rollback-registered, 20 adjudicated harmless with the
        argument beside each, 3 crossing only at a session edge
        (`ControlledSubject`, `CutsceneTriggerQueue`, `EncounterView`, each
        because its `Update` side is session teardown), 1 FILED
        (`CutsceneAdvanceRequest`), 0 nobody has examined.

<!-- ingress-census: spent_resources=56 resource_crossings=3 written_messages=95 message_crossings=3 unlocated=42 unlocated_types=15 -->

    scripts/check_host_produced_sim_consumed_requests.py
        Q136 INGRESS CENSUS — host-produced intent DESTRUCTIVELY CONSUMED by
        the simulation, over BOTH channels an intent can take. **RE-RUN
        2026-09-19** (this block read `3 of 57` and `4 of 95` with a `61 / 20`
        residual at 2026-09-18, and all three moved for stated reasons):

          * 3 of 56 spent RESOURCE types — `CutsceneAdvanceRequest` and
            `NewGameResetRequested` (both Q136), `VersusMatch` (Q140);
          * 3 of **95** written MESSAGE types — `AmbientGravityRequest` and
            `PlayerHealRequested` (both Q136), `ResetToCheckpoint` (adjudicated
            benign by the start-ordering, not by a latch). ⭐ It was FOUR until
            2026-09-19: `SetFlagRequested`'s intro chain closed on a witness,
            because a straddling DERIVATION is repaired by the resimulation
            that re-runs it, where a straddling EDGE is not.
          * ⇒ **FOUR LIVE Q136 INTENTS**, which is this ruling's population:
            two resources and two messages. `VersusMatch` is `Q140`'s and
            `ResetToCheckpoint` is adjudicated.
          * **AND THE MESSAGE HALF IS STILL A LOWER BOUND** — 42 distinct
            message writers and readers sit in no registration the instrument
            can follow, touching **15 of the 95 types**. ⇒ Any of those 15
            could be a crossing nobody has seen. ⭐ That residual was `61 / 20`
            until 2026-09-19, cut by teaching the scan a third registration
            shape (`let rules = (..); app.add_systems(sim, rules);`). ⚠ The
            VERDICT was byte-identical across that change, which is the right
            outcome to expect and the one worth stating: a narrower blind spot
            that moves no adjudication is the instrument improving, not the
            tree.

            ⛤ **THE THREE READINGS ARE 77/24, THEN 47/18, THEN 61/20, AND ONLY
            THE FIRST MOVE WAS THE INSTRUMENT GETTING BETTER AT THE SAME
            QUESTION.** The wrapper road took 77 to 47. The rise to 61 is a
            WIDENING: `MessageReader` and `MessageWriter` fields inside
            `#[derive(SystemParam)]` bundles joined the population, so fourteen
            systems that were never counted became countable and most of them
            are still unplaceable. A census that grows its population and its
            residual together is behaving correctly; reading the 61 as a
            regression against the 47 would be comparing two different
            populations, which is what the arm holding this number now says in
            so many words.

            ⛔⛤ **AND THE WIDENING FOUND FIVE MESSAGE TYPES THAT HAD NO WRITER
            AT ALL.** `BodyKnockedOut`, `LandedBodyHit`, `OwnedSfxMessage`,
            `ParriedBodyHit` and `WalletShieldSpent` are written ONLY through a
            bundle field, so they were absent from the written universe rather
            than merely misattributed — 90 was never the population. Predicted
            by the 2026-09-18 review from one production specimen
            (`FreshAttempt`, `crates/ambition_combat/src/events.rs:193`, two
            cursors, taken by a registered sim system), not from a poison.

            ⚠ **THE WRITE SIDE IS AN UPPER BOUND AND THE OUTPUT SAYS SO PER
            ROW.** Possession is not use — `grid_menu_nav` takes
            `MenuDispatchParams` and writes only its own menu message — and
            reducing a bundle to the types it holds is exactly what produced
            the retracted *"seven kaleidoscope systems raise
            `NewGameResetRequested`"*. So the TYPE is admitted (or the
            population is wrong), the NAME is kept for detection (possession is
            a sound upper bound on who can write), and the printed producer
            list marks it `(holds a writer)`.

            ⭐ **ONE CONSEQUENCE WAS A REPAIR RATHER THAN A NUMBER.** The wider
            population put `refuse_a_weaker_form_pickup` into the census, and it
            became the first message system placed by an OPAQUE SCHEDULE LABEL
            alone — registered through a local named `pre_collect_sim`
            (`game/ambition_demo_mary_o/src/lib.rs:1893`) rather than the
            workspace's `sim`. `is_schedule_variable` had deferred that to
            *"dataflow, which is a different instrument"*; measured, every local
            ever bound to a `sim_schedule()` call is one of TWO names — `sim` in
            45 files and `pre_collect_sim` in one. One idiom with a single
            exception is not dataflow, and resolving it closed the guess.

            ⛤ **WHAT CLOSED THE OTHER 30 IS WORTH MORE THAN THE NUMBER.** This
            page said the cause was the shared `add_systems_bodies` parser
            truncating the `app.add_systems(sim, ..)` at
            `combat_schedule.rs:640` before `apply_feature_hit_events` at
            `:695`, and that fixing it was a campaign because every census
            sharing that parser would have to move together. All of that was
            wrong: the body is 452 characters and closes correctly at `:648`,
            **0 of 639** `add_systems` bodies in the tree are truncated by a
            comment paren, and no body in that file has EVER contained the
            name. `:695` is inside `install_technique(app, KEY, offer,
            (..systems..))`, a registration WRAPPER whose own body is
            `app.add_systems(sim, systems)` — and
            `measure_user_settings_in_simulation.py` had already found that,
            written the mechanism down and built the fixpoint for it. The
            census now calls that owner's helpers. No shared parser moved and
            no other census's floors did either.

            ⚠ **AND THE RESIDUAL IS A DIFFERENT SHAPE, so nobody widens the
            wrong thing next.** Of the 47, **35 are in no `add_systems` body
            anywhere in the tree** — `main`, `fire`,
            `finalize_room_publication`, `dispatch_menu_action` are functions a
            system CALLS, so placing them needs a call graph rather than a
            better registration parser. The other 12 are registered only inside
            `#[cfg(test)]` modules, which the production population strips on
            purpose.

            The rows below are — `AmbientGravityRequest` and
            `PlayerHealRequested` (both Q136, both live) and `ResetToCheckpoint`
            (benign, OUTSIDE THE TIMELINE — see the readings below).
            `SetFlagRequested` was a fourth, filed benign, corrected to LIVE by
            a 2026-09-18 review, declared REPAIRED the same day because its
            producer moved into the rewinding schedule — **and RE-OPENED hours
            later by a second review, because moving the producer into the sim
            is not the same fact as the derivation landing on the same TICK.**

            ⛔⛤ **THE ESCAPE WAS CLAIMED ONE STEP TOO EARLY, AND THE STEP IT
            SKIPPED IS THE ONLY ONE A REWIND CARES ABOUT.**
            `emit_intro_flag_chains` is ordered
            `.after(Platformer2dSimulationPhaseMonolith::GameplayEffects)`
            (`game/ambition_content/src/intro/plugin.rs:146-150`), and
            `GameplayEffects` is where `apply_flag_effects` CONSUMES
            `SetFlagRequested`. So the derived message is read on the FOLLOWING
            tick, and a message in flight between two ticks is not rollback
            state. On a rewind to the snapshot entering that tick the pending
            message is not restored, the derivation re-fires in its usual
            position *after* the consumer, and the target flag lands one tick
            later than it did in the original timeline.

            ⭐ **TWO FACTS MEASURED 2026-09-18 SAY THIS IS NOT A PRESENTATION
            DELAY.** `AmbitionGameSave` is rollback state AND is checksummed —
            `rollback_resource_clone_checksum`
            (`crates/ambition_persistence/src/rollback_registration.rs:31`) —
            and a sweep for any message-buffer rollback mechanism in the tree
            returns nothing. ⇒ A flag arriving a tick late on replay is a
            difference in checksummed state, which is the desync class, not a
            late notification.

            ⚠ **AND THE CENSUS CANNOT SETTLE IT, BY CONSTRUCTION.** It
            classifies on whether the PRODUCER RUNS IN THE SIM SCHEDULE, which
            is a fact about where a system is installed; it never observes the
            producer/consumer snapshot boundary. That is why a green census
            read as a repair.

            ⛔⛤ **AND THE MESSAGE IS NOT MERELY UNRESTORED — IT IS DELIBERATELY
            CLEARED, WHICH NAMES THE BROKEN PREMISE EXACTLY.**
            `SetFlagRequested` is registered
            `clear_message_on_rollback`
            (`crates/ambition_combat/src/rollback_registration.rs:389`), and
            that kind's contract is spelled once, in
            `crates/ambition_platformer2d_core/src/rollback_kind.rs:332`:
            *"clear abandoned-future message buffer in
            `LoadWorld::Mapping`"*.

            ⇒ **“ABANDONED FUTURE” IS THE PREMISE, AND A PRODUCER ORDERED AFTER
            ITS OWN CONSUMER BREAKS IT.** Clearing is correct for a message
            raised inside a frame the rollback discards, because replaying that
            frame raises it again. This message is raised in a frame the
            rollback does NOT re-simulate and read in the next one — a pending
            PAST, not an abandoned future — so clearing drops it permanently and
            the re-derivation, which runs after the consumer, cannot put it
            back until the following tick.

            ⇒ The general rule this exposes is worth more than the instance:
            **`clear_message_on_rollback` is sound exactly while a message is
            produced and consumed within ONE tick.** Ordering that makes a
            cleared message straddle a tick boundary converts a deterministic
            cleanup into a dropped input.

            ⭐⛤ **THE CLASS WAS THEN SWEPT, AND IT HAS EXACTLY ONE MEMBER.**
            **80** message types are registered `clear_message_on_rollback`,
            so 80 carry the one-tick premise. Across production source (test
            files dropped, `#[cfg(test)]` modules stripped, comments stripped)
            **14** systems are ordered `.after(` a
            `Platformer2dSimulationPhaseMonolith` phase, and of those fourteen
            `emit_intro_flag_chains` is the ONLY ONE THAT WRITES A MESSAGE AT
            ALL — the other thirteen are audio, camera, view-sync and menu
            systems ordered after `CoreSimulation`, which is the host side of
            the boundary. ⇒ One instance, not a pattern, which is worth knowing
            before anyone reaches for a sweeping rule.

            ⚠ **THAT SWEEP IS A LOWER BOUND AND ITS DETECTION IS NARROW.** It
            sees an explicit `.after(<phase>)` on the producer. A producer can
            also land after its consumer through set MEMBERSHIP or a `chain()`,
            and this does not see either — answering that needs the initialised
            schedule graph, the way
            `the_mechanical_edit_chain_completes_before_the_timeline_advances`
            walks `PreUpdate`.

            ⚠ **WHAT IS REASONED HERE AND NOT YET RUN.** The three
            registrations above are read from source; no witness has been
            EXECUTED for this path, so the divergence is established as a
            mechanism and not as an observation. What would settle it is a rewind arm
            comparing the exact tick the target flag appears between an
            uninterrupted timeline and a replayed one, poisoning the ordering
            back to `.after` to prove the arm sees this specific invariant. The
            gravity witness landed today
            (`an_ambient_gravity_request_raised_outside_the_simulation_is_lost`)
            is the harness shape.

            ⚠ **THE SOURCE'S OWN ARGUMENT FOR `.after` LOOKS WRONG, AND IT IS
            REASONING RATHER THAN A READING.** The comment says ordering the
            producer BEFORE the consumer *"would collapse a chain into one
            tick"*. It would not: a producer running before the consumer
            observes START-OF-TICK state, so on the tick that establishes `A`
            it still sees `A` absent, emits `B` only on the next tick, and the
            chain advances one edge per tick exactly as now — while the
            emit-then-consume pair sits inside a single tick and re-derives
            identically on replay.

        ⛔⛤ So this ruling is responsible for FOUR live intents, not two, and
        the second channel was invisible until 2026-09-18 because the script
        required the `Resource` derive. ⭐⛤ **THE COUNT WENT FOUR → FIVE → FOUR
        → FIVE → FOUR ACROSS TWO DAYS, AND EVERY MOVE BUT THE LAST WAS A REVIEW
        RATHER THAN A MEASUREMENT**, which is the honest record of how confident
        this page was entitled to be. `SetFlagRequested` joined when a review
        corrected its benign verdict, left when its producer moved into the
        rewinding schedule, RETURNED when a second review pointed out that the
        producer is ordered after its own consumer — and left again on
        2026-09-19, this time on a WITNESS rather than an argument: a straddling
        EDGE desyncs the timeline and a straddling DERIVATION does not, and the
        intro chain is a derivation. See its block above. ⇒ The four are
        `CutsceneAdvanceRequest`, `NewGameResetRequested`,
        `AmbientGravityRequest` and `PlayerHealRequested`.

        ⚠ **THE PATTERN IN THAT OSCILLATION IS WORTH MORE THAN THE NUMBER.**
        Four of the five moves were one reader persuading another, and each
        argument was locally sound; what settled it was building the fixture
        that could tell the two readings apart. ⇒ A row that has changed verdict
        on argument twice should not change it a third time on argument.
        ⇒ What the briefly-repaired one has that the others do not is a
        producer that is a pure DERIVATION over rollback state rather than a
        latched input edge — which makes its escape REACHABLE, by reordering,
        and not yet taken. The filter was right and stays: without
        it the first version reported 67 rows, because `App`, `Commands`,
        `NextState` and `Sprite` are not resources. What was wrong was
        believing one channel was the population.

⛔⛤ **AND THE FIRST CENSUS CANNOT EVER REPORT THE SECOND'S SECOND ROW, WHICH IS
THE REASON TO STOP QUOTING ONE NUMBER.** `resources_crossing_the_rewind_boundary.py:355-358`
builds its candidate list as `name not in registered`, so a rollback-registered
resource cannot reach its `FILED` bucket by construction. `NewGameResetRequested`
IS registered — and its registration is precisely why Q136 applies to it: the
rewind restores the pre-write value and erases the menu's press. The broad
census is right to exclude it and the ingress census is right to name it. ⇒ **Q136
covers two resources, and the ingress census owns that count.** The page said 50 /
29 / 17 / 3 / 1 at 2026-09-17 with only `CutsceneAdvanceRequest` filed, which
represented neither `NewGameResetRequested` nor the `SpawnPlayerCloneRequest` <!-- cite-ok: the player-clone road was deleted in `89d78a4a5`; this line RECORDS the name, it does not point at one -->
specimen found and fixed the next day.

⛔⛤ **THE THREE THAT LEFT WERE NEVER CROSSING, AND THE RULING NEVER OWED THEM.**
`CausalRecording`, `SimPhaseCensus` and `SlotControls` each had an `Update` side
made up ENTIRELY of a test module calling `app.add_systems(Update, <a sim
system>)` — invisible to the shared test-module stripper because the module was
spelled `#[cfg(all(test, feature = "causal"))]` and the like rather than a bare
`#[cfg(test)]`. ⇒ `SlotControls` is the one to learn from: its recorded argument
(*"the Update writer stands down via `another_authority_publishes`"*) described a
REAL mechanism, which is why it read as a considered verdict — but the crossing
it adjudicated was a fixture's registration. **A reason that is true is not
evidence that its subject exists.** ⛔ The filed bucket exists BECAUSE of this row: a
subject with a question in front of it and a subject nobody has looked at were
the same pile until 2026-09-17, so the census could not report the number this
ruling's scope depends on. A filed row that stops crossing reddens the script.

⇒ The third measured subject, `OwnedItems`, is a message rather than a resource
and is covered by the second table row above, not by that census.

⇒ **The recommendation both prior reviews gave — "make it a semantic request the
simulation consumes, reusing `ItemGrantRequested`" — is necessary but not
sufficient.** Those messages work because their shipped producer is a
conversation node that already runs inside the sim schedule. The menu does not,
and cannot: it is UI.

**What the ruling is actually about.** In rollback netcode a local intent reaches
the timeline through the INPUT payload GGRS carries, because that is the one
channel with a deterministic, agreed arrival tick. Options:

1. **Carry it in the input payload.** Correct by construction, and **peer-visible**:
   it changes the input wire format two peers must agree on, and a New Game bit
   in every frame's input is a large thing to spend on a menu press.
2. **A non-rewinding intent buffer, drained inside the sim at a declared tick.**
   The buffer is host-local (never in a snapshot or checksum), and the DRAIN is
   the authoritative step. ⚠ The drain tick must be a function of agreed state,
   or this is just the current defect with more steps — two peers draining on
   different ticks is a desync rather than a swallow, which is worse.
3. **Declare New Game a session-level operation outside the timeline**, which
   stops the session, resets, and starts a new one. ⭐ This may be the honest
   answer for New Game specifically: it is not a mechanical action inside a match,
   it ENDS the match. It does nothing for the item-use half of
   MENU-RESET-MIDSESSION.

⚠ **NOT AN OPTION:** removing either type from the peer checksum.

⭐⭐ **AND THE MECHANISM MEASURED ON 2026-09-18 ADDS ONE TEST THAT ALL THREE
OPTIONS MUST PASS, WHICH THEY WERE NOT WRITTEN AGAINST.** The loss is the
CONSUMPTION record, not the intent: a `MessageReader`'s cursor is a `Local` that
no rewind restores, so the resimulation asks *"anything new?"* and is told no —
whether or not the intent is still sitting there (see the cursor block below for
the arithmetic). ⇒ Restated as a requirement:

> A host→sim handoff is safe only when the fact *"this has already been
> consumed"* is either rollback state or re-derived every frame. Carrying the
> INTENT across the boundary is not enough.

Against that test:

* **Option 1 passes by construction**, and this is the strongest argument for
  it: the input is re-fed on every resimulated frame BY THE ROLLBACK LAYER
  ITSELF, so consumption is re-derived where the replay can see it. It is the
  same property that makes the shipped input road correct — and the distinction
  that `SetFlagRequested` failed on, since a host system re-deriving in `Update`
  has no way to reach the frame being resimulated.
* **Option 2 passes only with an addition its description does not have.** A
  host-local buffer that is *"never in a snapshot or checksum"* is precisely a
  `Local` cursor with more steps: the drain marks it consumed, the rewind does
  not un-mark it, and the resimulated drain finds nothing. ⛔ So the drain's
  bookkeeping must be registered, or the buffer must re-present its contents on
  every resimulated frame. ⚠ That is a SECOND requirement beside the agreed
  drain tick this page already names, and it is the one that was invisible
  before the mechanism was measured.
* **Option 3 sidesteps the test** by leaving the timeline, which is why it stays
  the honest answer for New Game specifically.

⇒ **AND THE RULE IMMEDIATELY EARNED ITS KEEP BY FAILING ONE OF MY OWN BENIGN
VERDICTS.** I first wrote that this page carried TWO general escapes, the second
being *"`SetFlagRequested` is safe because its condition is re-derived every
host frame."* Applied honestly, the test refuses that: a host system
re-deriving on a later frame is outside the tick being resimulated, so the
consumption record for THAT tick is neither rollback state nor re-derived. A
2026-09-18 review reached the same conclusion independently, and the row is now
LIVE. ⇒ What survives is one escape — `ResetToCheckpoint`'s ordering, outside
the timeline — plus the corrected form of the one that failed: **re-derive
inside the rewinding schedule**, which nothing does yet.

⚠ **A rule that only confirms is not worth stating**, and this one struck down a
verdict I had published twice and generalised from. That is the argument for
having written it down as a test rather than as an observation.
`install_resource_clone_checksum` installs the snapshot/restore and the checksum
projection INDEPENDENTLY, so narrowing what peers compare leaves the swallow
exactly as it is. This has now been the wrong answer to three separate questions
in this file.

⚠ **AND SINGLE-PLAYER IS EVIDENCE OF NOTHING.** With no rollback session there is
nothing to rewind, so every hour of local play exercises the working path. The
witnesses need `with_sync_test_rollback_settings`.

⭐⛤ **AND THE SANCTIONED CROSSING ALREADY EXISTS IN THIS CODEBASE, WHICH IS THE
MOST USEFUL THING FOUND FOR THIS RULING.** `SlotControls` is written by
`publish_seat_controls_when_nobody_else_does` in `Update` **and** by
`publish_latched_slot_controls` in the sim — and it is correct, because the
`Update` writer opens with
`another_authority_publishes(latches.as_deref(), rollback.as_deref())` and
RETURNS. Under a rollback host the sim-side publisher owns the value and the
outside writer stands down.

⇒ **That is option 1's shape, already shipped for input.** The outside producer
does not write authoritative state; it asks whether it is still the authority,
and the timeline's own publisher is the one that writes. Any answer to this
question should look like that rather than inventing a fourth channel.

⛔ **HOW MUCH RIDES ON THE RULING, MEASURED.**
`scripts/resources_crossing_the_rewind_boundary.py` (new, 2026-09-16) censuses
every `Resource` written on BOTH sides of the boundary — ⚠ the block below is
that day's reading and is NOT edited; the live split is the row above — the complementary
question to `check_rollback_mutators_run_in_sim.py`, whose population is the
canonical registry and which therefore **cannot see an unregistered request at
all**:

```text
52  Resource types written on both sides
28  rollback-registered
16  adjudicated harmless, each with its argument (presentation, dev tools, catalogs)
 3  crossing only at a SESSION EDGE (teardown, not a per-frame producer)
 5  UNCLASSIFIED with a per-frame `Update` writer
```

⇒ **TRIAGED DOWN TO TWO, AND EVERY CLASSIFICATION IS A VERIFIED GUARD RATHER THAN
A NAME ON A LIST.** Three separate spellings of the same argument turned up:

| type | who stands down | verified |
|---|---|---|
| `SlotControls` | the `Update` writer, via `another_authority_publishes(latches, rollback)` | body read |
| `LoadCoordinator`, `RoomTransitionLoadState` | the SIM writer, via `if simulation_host.is_rollback() { return; }` | body read, not its param comment |
| `SlotControlLatches` | the SIM consumer, via `if replay.replaying_history { return; }` | body read |
| `SeatRawFrames` | nobody — the READ is routed, via `seat_frame_this_tick` | body read |

⇒ **THE RESIDUE IS ONE, AND IT IS THE MEASURED DEFECT.**
`CutsceneAdvanceRequest` uses none of the four, which is exactly why it is the
only row left. ⭐ So the fix has **four shipped precedents to choose from** rather
than needing a new mechanism, and that is a far better position than this
question started in.

`SeatRawFrames` is the interesting fourth: nothing stands down, the READ picks
the authoritative table — `if another_authority_publishes(latches, rollback) {
slots.get(slot) } else { raw.get(slot) }` — and the raw row is written only to be
folded into the encoded rollback input. Its own doc: *"Writing the table that is
not authoritative is harmless — it is overwritten by the authority that owns
it."*

⭐⛤ **AND THE THIRD ROW IS THE INVARIANT THIS WHOLE QUESTION WAS LOOKING FOR.**
`publish_latched_slot_controls` consumes DESTRUCTIVELY inside the sim —
`latches.take(slot)`, the exact shape that loses a menu press — and it is
correct, because it returns while `replaying_history` and a replayed tick is fed
from **GGRS's stored input** instead.

⇒ **Destructive consumption inside the sim is safe when the intent also rides a
channel the replay can re-read.** The guard is not the fix on its own: giving
`tick_active_cutscene` the same early return would stop it double-consuming and
still lose the advance, because nothing would re-apply it. What makes the input
path work is that GGRS carries the input, so the replay has a source.

⛔⛤ **AND FOR THE CUTSCENE HALF THE CHANNEL NEEDS NO NEW BIT — THE SHIPPED GAME
ALREADY TELLS THE PLAYER WHICH BUTTON IT IS.** `ControlFrame` (the frame GGRS
carries, read through `seat_frame_this_tick`) already has `interact_pressed`,
`start_pressed` and `reset_pressed`. And `cutscene_lab_intro`'s own dialogue
beat reads: *"Hold Reset to skip cutscenes -- useful when you've heard a beat
already."*

⇒ So the skip is ALREADY the Reset button in the player's mental model, and
`reset_pressed` is already synchronised and already consumed this exact way by
`apply_player_reset_input_system`. Option 1 for the cutscene half is *"read the
seat frame like every other gameplay button"*, not a wire-format change.

⚠ **THE HOLD STAYS OUTSIDE AND THAT PART IS ALREADY RIGHT.**
`SKIP_HOLD_THRESHOLD_SECS` is accumulated in `CutsceneSkipHold` in WALL time,
deliberately input-local, and only the completed edge is meant to cross. The
census adjudicates `CutsceneSkipHold` as correctly unregistered for that reason.
The defect is entirely in how the completed EDGE crosses.

⚠ **AND THE COUNTER-ARGUMENT IS WRITTEN AT THE PRODUCER, WHICH IS WHY THIS IS
STILL A RULING.** `apply_menu_frame_to_cutscene_request` reads `MenuControlFrame`
on purpose: *"Cutscene controls are UI/menu intent, not gameplay movement. Keep
this small bridge beside the menu frame so touch Confirm/Back can advance or skip
cutscenes without teaching the gameplay `ControlFrame` about menu gestures."*
⇒ That is a real design position and it is the one in tension. Touch Confirm/Back
has no seat frame; a cutscene dismiss from a touch gesture would need the menu
road either way. The decision is whether the cutscene edge is gameplay input
(synchronised, one road) or UI intent (unsynchronised, and then it needs the
replay source that Q136 is about).

⭐⛤ **AND THE TOUCH HALF OF THAT TENSION IS FALSE — MEASURED 2026-09-18.** Touch
is not a second input world: `ambition_touch_input` feeds
`ActionState<Platformer2dInputActionMonolith>`, the SAME leafwing action state
the desktop side uses (`virtual_device.rs:304` binds
`(A::Reset, TouchVirtualButton(B::Reset))`, and `Reset` and `Start` are the only
two gameplay actions it binds). `ambition_input/src/control.rs:247` turns that
action state into `reset_pressed`. So a touch Reset press already arrives on the
gameplay `ControlFrame` and already rides the GGRS input payload — which is
exactly the channel this question is looking for, for exactly the button the
shipped dialogue names.

⇒ **WHAT IS ACTUALLY LEFT IS SMALLER AND CONCRETE: `ControlFrame` CARRIES THE
EDGE AND NOT THE LEVEL.** There is `reset_pressed` and no `reset_held`, and the
skip is a HOLD (`SKIP_HOLD_THRESHOLD_SECS`). So the remaining cost is one of two
things, and neither is a new wire format:

1. **add `reset_held` beside `reset_pressed`.** One bool, in the type that
   already carries `jump_held`, `blink_held`, `special_held`, `shield_held` and
   `attack_held` for precisely this reason — *"a move can be held"*. The hold
   accumulation then moves into the sim and must use `WorldTime::sim_dt()` with a
   rollback-registered accumulator, so a rewind replays it.
2. **keep the accumulation outside and send only the completed edge**, which is
   what happens today — and that edge is the thing Q136 says gets lost.

⚠ Option 1 makes `CutsceneSkipHold` rollback state, which this page's census
currently adjudicates as *correctly* unregistered because the hold is
input-local wall time. That adjudication is a consequence of the current design,
not an argument for it: if the hold moves inside the timeline it must rewind, and
the census row changes with the code. ⛔ What must NOT happen is moving the hold
inside while leaving it accumulating from `Res<Time>` — that is precisely the
`tick_player_clone_brains` defect measured the same day, and <!-- cite-ok: the player-clone road was deleted in `89d78a4a5`; this line RECORDS the name, it does not point at one -->
`scripts/check_sim_schedule_memory_is_adjudicated.py` now exists to catch it.

⭐ **WHICH REFRAMES OPTION 1 FROM "INVENT A CHANNEL" TO "USE THE ONE THIS ALREADY
HAS", at least for the cutscene half.** A dismiss/skip IS a button press. The
control frame GGRS already carries is the channel; `CutsceneAdvanceRequest` is a
second, unsynchronised copy of a button beside it. ⚠ Whether cutscene dismiss
belongs in the gameplay control frame is still a design call — it is read today
from `MenuControlFrame`, deliberately, because *"cutscene controls are UI/menu
intent, not gameplay movement"*. That tension is the decision, and it is a much
smaller one than a new wire format.

⚠ **THE CENSUS REPORTS, IT DOES NOT GATE, AND THE AXIS IS WHY.** Crossing the
boundary is necessary for the defect and nowhere near sufficient. What separates
the three measured defects is **destructive consumption inside the sim** —
`mem::take`, a drain, a bool reset — which is what makes the outside write
unrecoverable rather than merely late. Detecting that statically is not
attempted; it is named so the residue can be read. The first version of the
sweep reported 67 rows because `App`, `Commands`, `NextState`, `Anchor` and
`Sprite` are not resources, and requiring the `Resource` derive removed them by
construction.

### 2026-09-18 — the population has a SECOND CHANNEL, and the instrument could not see it

⛔⛔ **THE INSTRUMENT IS KEYED ON `Resource`, AND FOUR CROSSINGS ARE `Message`s.**
`check_host_produced_sim_consumed_requests.py` requires the `Resource` derive —
deliberately, because the first version reported 67 rows including `App`,
`Commands` and `Sprite`. That filter also excludes an entire ingress channel.
Measured by asking the same question of `MessageWriter`/`MessageReader`
signatures across 87 message types written in the tree, and confirming each
side's schedule from `add_systems`:

| message | host writer (`Update`) | sim reader (`sim`) |
|---|---|---|
| `AmbientGravityRequest` | `cycle_dev_gravity` | `apply_ambient_gravity_requests` |
| `PlayerHealRequested` | `kaleidoscope_menu_action_activated` | `apply_player_heal_requests` |
| `ResetToCheckpoint` | `complete_durable_restore` | `resume_at_checkpoint_on_reset` |
| `SetFlagRequested` | `emit_intro_flag_chains` | `apply_flag_effects` |

⚠ `kaleidoscope_menu_action_activated` is already on this page as one of the two
real `NewGameResetRequested` producers. The menu has a SECOND lost-intent road
beside the one this row has been discussing.

⛔⛤ **AND THE MECHANISM IS NOT AN OVERSIGHT — IT IS A DECLARED ROLLBACK
DECISION DOING EXACTLY WHAT IT SAYS, WHICH IS WHY THIS IS SHARP.** All four
declare `clear_message_on_rollback`, and so do 82 channels in total. That
registration adds `clear_message_channel::<T>` to **`LoadWorld`**, in
`LoadWorldSystems::Mapping`
(`crates/ambition_platformer2d_rollback_ggrs/src/registration.rs:1149-1151`,
`:1312-1316`) — so every rewind EMPTIES the channel. For a message raised inside
the simulation that is precisely right: the resimulation re-raises it, and
leaving the old copy would double it. For a message raised by the HOST there is
no resimulation to re-raise it, so the clear would be the loss. ⇒ **The same
declaration that makes the 78 sim-side channels correct would be the deletion
mechanism for the 4 host-side ones.**

⛔⛤ **THAT PARAGRAPH SAID "IS" UNTIL A POISON SAID OTHERWISE, AND THE
CORRECTION IS THE USEFUL PART OF IT.** MEASURED 2026-09-18: removing
`clear_message_on_rollback::<PlayerHealRequested>` changed its witness's
outcome **not at all** — the heal still rose for about two frames and was still
revoked. The poison was verified applied rather than assumed, by making it
announce itself; it printed four times in the test binary, because *"the
registration is gone"* and *"the build did not pick it up"* produce the same
green. ⇒ The clear is at most PART of the mechanism, and two other candidates
survive:

1. **The reader's cursor.** `MessageReader<'w, 's, M>` is literally
   `{ reader: Local<'s, MessageCursor<M>>, messages: Res<'w, Messages<M>> }`
   (`bevy_ecs` 0.19.1, `message/message_reader.rs:34-38`). A `Local` is
   per-system host storage that no rewind restores, so after the rollback the
   cursor still points PAST the message it consumed on the speculative frame —
   and the resimulation finds nothing to read even when the channel still holds
   it. ⛤ Chasing this found a population gap in a shipped guard:
   `check_sim_schedule_memory_is_adjudicated.py` matches the literal token
   `Local<` and therefore sees only the systems that spell it. RE-MEASURED
   2026-09-18 after two widenings — `MessageReader` expanded through
   `#[derive(SystemParam)]` bundles, and registration WRAPPERS followed into the
   sim population — it is **14** of **142** memory-carrying sim systems; **129**
   carry a `MessageReader` cursor (**138** cursors), overlap 1. ⚠ It read
   13 / 112 / 99 / 106 that morning, and every one of those moved for a
   different reason: an adjudication landed, a bundle's fields became visible,
   and a wrapper road joined the population. The census says the hidden number
   out loud every run rather than letting a clean `14/14` imply 14 is the
   population.
2. **`bevy`'s own double-buffer expiry**, which drops a message after two frames
   with no rollback involved at all — and the measured transient lasted about
   two frames, which is exactly the coincidence that makes this candidate
   impossible to dismiss from the end state alone.

⭐⭐ **AND CANDIDATE 1 IS SETTLED BY SOURCE: IT IS THE CURSOR, AND THE CLEAR IS
REDUNDANT FOR A HOST-RAISED MESSAGE.** Read 2026-09-18 in `bevy_ecs` 0.19.1:

* `Messages::clear` (`message/messages.rs:228-232`) empties both buffers and
  calls `reset_start_message_count`, which sets each buffer's
  `start_message_count = self.message_count`. ⇒ **`message_count` is MONOTONIC
  across a clear.** It is never rewound.
* A cursor's unread count is
  `messages.message_count.saturating_sub(self.last_message_count).min(messages.len())`
  (`message/message_cursor.rs:120-129`).

Put those together on the heal's timeline. The host writes: `message_count`
6, the reader consumes it, `last_message_count` 6, health rises. The rollback
runs, with or without the clear. The resimulation asks the cursor for unread
messages and gets `6 - 6 = 0` — **whether or not the channel still holds the
message.** ⇒ That is exactly why removing `clear_message_on_rollback` changed
nothing, and it is not a coincidence the poison had to uncover: the two
mechanisms are not independent, and the cursor runs first.

⭐ **THE SAME ARITHMETIC IS WHY THE 78 SIM-SIDE CHANNELS ARE FINE, WHICH IS THE
READING THE OTHER CENSUS'S 99 CURSORS WERE WAITING FOR.** A sim-raised message
is re-raised by the resimulation, which bumps `message_count` to 7 — past the
cursor — so the re-raise IS read. ⇒ The rule is one line:

> A `MessageReader` inside the rewinding schedule loses its message exactly
> when nothing inside that schedule re-raises it.

Which is the Q136 ingress question, not a new one. **Joined 2026-09-18 over the
99:** 73 read only sim-produced messages (safe), 4 read a host-produced one
(and they are precisely the four rows above), and 22 read a type with no
production writer at all — a ROAD with no shipped producer, `ItemGrantRequested`
among them, which is why that witness raises its own message from the test.
⇒ **The two censuses are one population seen twice**, and candidate 3 cannot be
the operative mechanism because the cursor has already stopped the read on the
FIRST resimulated frame, before any double-buffer expiry could matter.

⚠ **WHY THIS MATTERS TO THE RULING AND NOT ONLY TO THE PROSE.** Each candidate
implies a different ingress road. If the clear is the loss, a per-message
opt-out fixes it. If the cursor is, no channel-level change can — the fix has to
be state the rollback restores, or a consumer that re-derives. If it is expiry,
the intent merely needs to survive longer than the rollback window, which is a
latency question rather than an architecture one. ⇒ **Separating the three is a
prerequisite for choosing this question's answer, not a footnote to it.** The
experiment is cheap: hold the channel (poison above), then additionally seed the
cursor or step inside two frames, and see which one restores the heal.

⇒ **SO THE ANSWER TO "HOW MANY RESOURCES IS THIS RULING RESPONSIBLE FOR" IS
STILL TWO, AND IT WAS THE WRONG QUESTION.** Two resources plus four messages,
and the resource count was only ever complete for the channel the instrument
looks at.

⭐⭐ **ALL FOUR READ, AND ONLY TWO ARE LIVE — WHICH IS WHY EACH ROW NEEDED A
READING RATHER THAN A COUNT.**

- ⛔ **`AmbientGravityRequest` — LIVE, and the same mechanism as the clone.**
  `cycle_dev_gravity` (`game/ambition_app/src/menu/kaleidoscope_app.rs:2054`)
  reads `keys.just_pressed(KeyCode::Backslash)` and writes once — an
  unregistered host EDGE spent in the sim, and the physical press is several
  host frames gone by the time a rewind ends. A developer hotkey, so the stakes
  are the clone's rather than a player's.

  ⭐ **WITNESSED 2026-09-18, LAST OF THE FOUR, AND THE ARM CORRECTED THE ROW.**
  `an_ambient_gravity_request_raised_outside_the_simulation_is_lost`
  (`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`). Two
  corrections came out of running it rather than reading it:

  ⛔⛤ **THE SHAPE IS THE ITEM GRANT'S FLAT ZERO, NOT THE HEAL'S TRANSIENT.**
  `BaseGravity` never passes through the cycled direction on any of 200 frames,
  under the same ownership mode and the same `(4, 10)` sync-test settings where
  the heal is visible for about two. ⇒ *"Host-raised intents are lost the way
  the heal is lost"* is not a sentence this census can make, and the mechanism
  clause that used to sit in this row — *"the rewind clears the channel"* — does
  not survive either: a flat zero means the request was never applied even once,
  which a channel cleared AFTER a speculative apply cannot produce. The same
  clause was already refuted for the heal by poisoning
  `clear_message_on_rollback` and watching nothing change.

  ⚠ **AND BOTH ITS ARMS RUN ONE COMPOSITION, WHICH ITS THREE SIBLINGS DO NOT.**
  Writing the request on EVERY one of 200 frames into the sandbox fixture the
  heal arm uses moves `BaseGravity` not once, because
  `apply_ambient_gravity_requests` is not reached in that composition at all. A
  flat zero measured there would have been indistinguishable from this finding
  and would have meant only *"no reader here"*. ⇒ The one difference between
  the two arms is WHERE the message is written.

  ⚠ **ITS FIRST CONTROL FAILED, AND THE FAILURE WAS THE CONTROL WORKING.** The
  in-sim cycle lands at tick 100 and is restored to the authored default at step
  158 by `reset_gravity_on_room_reset`
  (`crates/ambition_platformer2d_actor_monolith/src/gravity/lifecycle.rs:28-39`),
  which fires on `RoomReplayAdmitted` — reachable in an idle window now that
  Ambition's untagged-room death rule is declared again. An endpoint comparison
  would have called a working road broken. Both arms therefore assert that
  gravity reached exactly ONE CYCLE STEP from where it started, which is a
  direction a reset cannot manufacture: a reset moves gravity TO the default and
  both fixtures start there.
- ⛔ **`PlayerHealRequested` — LIVE, and player-visible.** Raised by
  `kaleidoscope_menu_action_activated`, which this page already names as one of
  the two real `NewGameResetRequested` producers. A heal chosen in the menu can
  be accepted at the UI and vanish before the simulation applies it.
- ✅ **`ResetToCheckpoint` — BENIGN, BY AN ORDERING THE ROLLBACK LAYER ENFORCES
  ON PURPOSE.** `maintain_local_session` returns without starting a session
  while `durable_hydration_is_pending`
  (`crates/ambition_platformer2d_rollback_ggrs/src/local_session.rs:334-340`), so
  `complete_durable_restore` has already run and its message has already been
  consumed before any timeline exists. There is no rewind to lose it to. ⚠ The
  argument is the ORDERING, not the latch: `SaveRestored` is
  rollback-registered, and the same file records at `:321-326` that it *"is not
  a latch that always rises"*, so reasoning from the latch would have been
  reasoning from the wrong fact.
- ⛔ **`SetFlagRequested` — LIVE, AND I FILED IT BENIGN ON TWO WRONG CLAIMS.**
  A 2026-09-18 review caught it, and the correction is the most instructive
  thing on this page. I wrote that `emit_intro_flag_chains`
  (`game/ambition_content/src/intro/route_state.rs:29-41`) recomputes
  `data.flag(trigger) && !data.flag(target)` from the save *"EVERY host frame"*,
  so a cleared message is simply rewritten. Both halves fail:

  1. **It is not every frame.** It is registered
     `run_if(resource_exists_and_changed::<AmbitionGameSave>)`
     (`game/ambition_content/src/intro/plugin.rs:111-117`). I described a
     registration I had not read.
  2. **⛔⛤ AND EVERY FRAME WOULD NOT HAVE SAVED IT**, which is the part that
     matters, because it is the claim I generalised. The producer is in
     `Update`. Re-deriving on a LATER host frame does not re-run the HISTORICAL
     tick being resimulated: the replay of that tick still has no message, and
     `apply_flag_effects` (`features/ecs/effect_bus.rs:16-29`) writes
     `AmbitionGameSave` AND `QuestRegistry`, both
     `rollback_resource_clone_checksum`-registered
     (`crates/ambition_persistence/src/rollback_registration.rs:31`, `:37`).
     ⇒ The replayed frame diverges on a CHECKSUMMED value, and the re-emission
     lands the flag on a different tick.

  ⭐ **It is also the cheapest of the live rows to repair**, and that is a real
  finding rather than consolation: unlike a menu press this is a PURE DERIVED
  CONDITION over rollback state, so moving the derivation inside the rewinding
  schedule needs no synchronised input channel at all.

  ⛔⛤ **AND THE REPAIR MOVED IT INTO THE SCHEDULE AND LEFT IT STRADDLING A TICK.
  WITNESSED 2026-09-19; THE ROW STAYS OPEN.** `emit_intro_flag_chains` is now
  in the sim schedule rather than `Update`, which is the escape above — but it
  is ordered
  `.after(Platformer2dSimulationPhaseMonolith::GameplayEffects)`
  (`game/ambition_content/src/intro/plugin.rs:145-158`), and its consumer
  `apply_flag_effects` runs INSIDE `GameplayEffects`
  (`crates/ambition_platformer2d_actor_monolith/src/features/mod.rs:216`). A
  producer ordered after its own consumer makes the message a pending PAST, and
  `clear_message_on_rollback` is sound exactly while a message is produced and
  consumed within ONE tick.

  **Two readings of the same four lines, so the world was asked instead.** The
  review's reading was that the rewind silently drops the pending message; the
  plugin's own comment argues the restore re-arms the producer's change gate so
  resimulation re-raises it. Four arms in
  `game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`, on the
  shipped sync-test settings:

  | arm | slot | writes | result |
  |---|---|---|---|
  | `a_flag_requested_inside_the_tick_reaches_the_save_without_rollback` | before | flag | flag set — the premise |
  | `a_flag_requested_before_its_consumer_survives_the_rewind` | before | flag | clean, flag set |
  | `a_silent_system_after_the_consumer_does_not_desync` | **after** | nothing | clean |
  | `a_flag_requested_after_its_consumer_desyncs_the_timeline` | **after** | flag | ⛔ **DESYNC** |

  ⇒ **NEITHER PREDICTION WAS RIGHT, AND THE ANSWER IS THE WORSE OF THE TWO.**
  A request raised on tick 40 produces a GGRS sync-test checksum mismatch at
  frames **41, 42, 43** — the ticks whose consumer finds the buffer already
  cleared. A lost flag is one peer missing an effect; a checksum mismatch is the
  peers DISAGREEING, which is the failure `clear_message_on_rollback` exists to
  prevent. `AmbitionGameSave` is a `rollback_resource_clone_checksum`
  registration, so the replayed frame that does not write the flag hashes
  differently from the original that did.

  ⭐ **AND TWO CONTROLS MAKE IT A STATEMENT ABOUT THE STRADDLE.** The same
  request one slot earlier is clean AND sets the flag; the same system in the
  same slot writing nothing is clean. Only the combination diverges, so this is
  not "a system in that slot desyncs" and not "this file desyncs".

  ⚠ **WHAT THE WITNESS DOES NOT SETTLE, STATED SO THE NEXT READER DOES NOT
  OVERREAD IT.** The arms reproduce the MECHANISM, not `emit_intro_flag_chains`:
  they carry no `resource_exists_and_changed::<AmbitionGameSave>` gate. The
  plugin's argument — that a restore marks the resource changed and so re-raises
  the message on the replayed tick — is now the only thing standing between the
  intro chain and this divergence, and it has no arm. ⇒ Until it has one,
  `SetFlagRequested` is a LIVE row and the repair is not closed. The narrow fix
  is available and cheap: order the producer BEFORE `GameplayEffects`, which the
  plugin comment declines on the grounds that next-tick chaining is the shipped
  behaviour — a behaviour question for the maintainer, and the reason this stays
  here rather than being fixed in passing.

  ⭐⭐⛤ **AND THE GATE'S ARM NOW EXISTS, SO THIS ROW CAN STOP BEING ABOUT THE
  INTRO CHAIN — MEASURED 2026-09-19.** The paragraph above ended by saying the
  plugin's argument was *"the only thing standing between the intro chain and
  this divergence, and it has no arm."* Three more arms, in the same file, with
  the derivation's real shape — gated on
  `resource_exists_and_changed::<AmbitionGameSave>`, skipping a target already
  present, and armed by a PRIOR flag write the way a chain is:

  | arm | producer in the straddling slot | result |
  |---|---|---|
  | `a_flag_requested_after_its_consumer_desyncs_the_timeline` | a one-shot EDGE | ⛔ desync |
  | `the_gated_derivation_survives_the_straddle_that_desyncs_without_it` | gated DERIVATION | ✔ clean, flag lands |
  | `separating_the_gate_from_the_derivation_shape` | the same derivation, gate REMOVED | ✔ clean, flag lands |

  ⇒ **IT IS THE DERIVATION SHAPE THAT SURVIVES, NOT THE GATE.** The third arm
  exists because the gated one differed from the desyncing one in TWO ways at
  once, and crediting the `run_if` without separating them would have been a
  property measured only on the accused. With the gate removed the straddle is
  still clean, so `resource_exists_and_changed` is a COST OPTIMISATION and not
  a correctness mechanism — which is exactly how escape three is worded on this
  page already (*"re-derive INSIDE the rewinding schedule"*, no mention of
  change detection). The wording was right and had not been earned; now it is.

  ⇒ **SO `emit_intro_flag_chains` IS NOT A LIVE DEFECT, AND THE REVIEW CONCERN
  THAT RE-OPENED IT DOES NOT HOLD.** Its `.after(GameplayEffects)` position
  makes its message straddle a tick, and a straddling EDGE desyncs — but it is
  a derivation over rollback state, and a straddling derivation is repaired by
  the resimulation that re-runs it. The 2026-09-18 repair was complete.

  ⭐⭐ **AND THAT NARROWER QUESTION IS ANSWERED THE SAME DAY, SO THE PAYLOAD
  CLOSES.** The remaining question was whether any OTHER producer of
  `SetFlagRequested` is an EDGE raised outside the rewinding schedule.
  `scripts/check_host_produced_sim_consumed_requests.py` — the census that
  built this row's list — reports **95 message types written somewhere, and
  exactly THREE read inside the rewinding schedule and written only outside
  it**: `AmbientGravityRequest`, `PlayerHealRequested` and `ResetToCheckpoint`.
  `SetFlagRequested` is not among them. Its other producers
  (`features/ecs/chests.rs`, `interact.rs`, `pickups.rs`, `damage/actor_hit.rs`,
  `world_facts.rs`) write from inside the sim, and the authored Yarn command
  writes through the ledger.

  ⚠ **THAT LAST CLAUSE RESTS ON AN EXEMPTION WHOSE PREMISE IS NOW HELD, WHICH
  IS THE ONLY REASON IT IS SAFE TO LEAN ON.** The census does not count a
  `NarrativeInputWriter` as a host crossing, on the grounds that the ledger is
  the safe ingress — and until 2026-09-18 nothing checked that the ledger was
  actually INSTALLED for a given payload, which is exactly how
  `SetFlagRequested` came to be written by a shipped Yarn command with no
  `NarrativeInputPlugin` anywhere. `scripts/check_narrative_writers_have_a_ledger.py`
  now pairs all ten writer payloads with their plugins, so the exemption has a
  premise rather than a rationale.

  ⇒ **`SetFlagRequested` IS NO LONGER A LIVE `Q136` ROW.** Both of its roads are
  accounted for: the intro chain is a derivation inside the rewinding schedule
  (witnessed above), and the authored command rides the tick-stamped ledger
  (escape four, premise held). The count returns to FOUR, which is where it was
  before the 2026-09-18 review re-opened it — and the two corrections that
  happened in between were both real.

  ⛔⛤ **AND THE FIRST GATED FIXTURE MEASURED NOTHING, WHICH ITS PREMISE ARM
  CAUGHT AND IS THE REASON THAT ARM EXISTS.** A derivation gated on CHANGED
  runs only on a tick after something moved the save, and in a fixture where
  nothing else writes the save, nothing ever changes it: the derivation never
  ran, the flag never landed, and the rollback arm beside it went red while
  testing nothing at all. Had the premise arm not been written first, that red
  would have read as *"the gate does not survive the rewind"* — the opposite of
  the true answer. The repair is a PRIMER flag written on the sound road, which
  is the role an earlier authored flag plays in the shipped chain.

⭐⛤ **SO THERE IS ONE GENERAL ESCAPE IN THE TREE, NOT TWO — AND LOSING ONE
SHARPENED THE RULE INSTEAD OF WEAKENING IT.** The surviving escape is
`ResetToCheckpoint`'s: **the ordering never enters the timeline.** The one I
lost was *"the condition is re-derived"*, and its failure says exactly what the
real requirement is:

> Re-derivation escapes only where the REPLAY can see it. A host system that
> recomputes the condition is outside the frame being resimulated, so it cannot
> repair that frame — it can only produce a different one later.

⇒ Which folds into the consumption test above as a third, currently unused,
escape: **re-derive INSIDE the rewinding schedule.** `SetFlagRequested` would be
its first instance. Beside the two roads in the section below — ride the
synchronised control frame, or publish as a mechanical edit — this question has
four answers, of which exactly one costs nothing and is already in the tree.
⇒ A latched edge is what makes an intent losable, and so is a latch re-armed on
the wrong side of the boundary.

⭐⭐⛤ **AND THERE IS A FOURTH ESCAPE, FULLY BUILT, SHIPPING ON TEN PAYLOAD
TYPES, AND THIS PAGE HAD NEVER NAMED IT — FOUND 2026-09-18 WHILE CHECKING A
SENTENCE IN `Q140`.** A host-raised intent does not have to choose between
being erased by the rewind and being re-derived inside it. It can be **STAMPED
WITH THE TICK IT APPLIES FROM AND HELD OUTSIDE ROLLBACK STATE**, which is what
`ambition_conversation`'s narrative-input ledger does:

| piece | where | what it does |
|---|---|---|
| `NarrativeInputWriter::write` | the HOST, wherever the command fires | records the payload against `SimTick + 1` (`crates/ambition_conversation/src/ledger.rs:138-148`) |
| `NarrativeInputLedger<M>` | a resource DELIBERATELY NOT rollback-registered | waived in `rollback_coverage.rs:158` as *"an EXTERNAL INPUT, stamped with the tick it applies from — the same category as the device input stream, and rewinding it would erase what the simulation was told rather than what it decided"* |
| `release_narrative_inputs::<M>` | the head of the SIM schedule, inside `GameplaySimulationRoot` | writes the ordinary channel at the stamped tick, so **every resimulation of that tick re-raises the message** |
| `prune_narrative_inputs::<M>` | `Update`, and the placement is load-bearing | ages entries out past the prediction window; in the sim *"a replayed tick that erases its own input reaches a different history than the run it is reproducing"* |

⇒ **THIS IS THE GENERAL ANSWER THE OTHER THREE ESCAPES ARE SPECIAL CASES OF**,
and it costs a host producer nothing but a different writer type. It does not
need the producer to be a pure derivation (escape three), it does not need the
producer to move into the sim (escape two), and it does not need the intent to
sit outside the timeline (escape one). `NarrativeInputPlugin` is registered for **ten**
payloads across **four** sites.

⛤ **THIS SAID FIVE UNTIL 2026-09-19, AND FIVE WAS A SITE RATHER THAN A
POPULATION.** The five named were the actor-monolith block
(`crates/ambition_platformer2d_actor_monolith/src/features/mod.rs:1401-1405`) —
a contiguous run of registrations that reads like a list of all of them, which
is exactly why it was copied as one. Measured by
`scripts/check_narrative_writers_have_a_ledger.py`, which parses both halves of
the pairing rather than a block:

| payload | plugin installed at |
|---|---|
| `ChallengeRequested` | `crates/ambition_platformer2d_actor_monolith/src/features/mod.rs:1401` |
| `BrainCommand` | `crates/ambition_platformer2d_actor_monolith/src/features/mod.rs:1402` |
| `ReleaseProvocation` | `crates/ambition_platformer2d_actor_monolith/src/features/mod.rs:1403` |
| `ItemGrantRequested` | `crates/ambition_platformer2d_actor_monolith/src/features/mod.rs:1404` |
| `ShopTransactionRequested` | `crates/ambition_platformer2d_actor_monolith/src/features/mod.rs:1405` |
| `ConversationEnded` | `crates/ambition_conversation/src/plugin.rs:34` |
| `RunAuthoredCommand` | `crates/ambition_conversation/src/plugin.rs:38` |
| `SpawnActorRequest` | `game/ambition_content/src/plugin.rs:140` |
| `CutRopeRoomReplayRequested` | `game/ambition_content/src/bosses/mod.rs:287` |
| `SetFlagRequested` | `game/ambition_content/src/bosses/mod.rs:302` |

⇒ **THE CORRECTION STRENGTHENS THE ARGUMENT RATHER THAN QUALIFYING IT.** The
escape is not a local convenience of one plugin's own payloads: it is installed
by the conversation crate for its own two, by the actor monolith for five, and
by CONTENT for three — a downstream crate adopting the road for payloads it
does not own. ⚠ `SetFlagRequested`'s registration is the newest and was added
because it was MISSING while a shipped Yarn command wrote through the ledger;
see the `SetFlagRequested` row above, which is still live for a different
reason.

⚠ **AND THE `+ 1` IS THE PART A REIMPLEMENTATION WOULD GET WRONG.** The ledger
stamps the NEXT tick, because a host command fires in `Update` after this
frame's simulation has already run; stamping the tick that has been simulated
*"would make the original frame and its replay disagree about whether the fact
was true during it"* (`ledger.rs:165-175`).

⛔ **WHAT DOES NOT TRANSFER, SAID BEFORE ANYONE COSTS THE MOVE: THE PLUGIN IS
CONVERSATION-SCOPED, AND THE FOUR LIVE INTENTS HAVE NO CONVERSATION.** Every
entry is keyed by `ConversationInstanceId`; `NarrativeInputWriter::write` DROPS
the payload with a warning when no conversation is live
(`ledger.rs:138-146`), and `release` is asked for the entries belonging to the
live instance. A menu press, a developer hotkey and a cutscene dismiss have no
instance to key on. ⇒ What is reusable is the SHAPE — a tick-stamped record,
held outside rollback state, released at the head of the sim — and what a
maintainer is being asked to approve is generalising the key from *"the
conversation this belongs to"* to *"the session this belongs to"*, not adopting
an existing plugin. That is a smaller question than "does a road exist", which
is what this section could previously only ask, and it is bigger than a
registration move.

⭐⭐ **AND THE ONE THING THAT COULD HAVE MADE THAT ESCAPE UNAVAILABLE IS
MEASURED, 2026-09-18: IT CANNOT.** The obvious objection is that the producer is
CHANGE-GATED — `run_if(resource_exists_and_changed::<AmbitionGameSave>)` — and
Bevy change ticks are not rollback state, so a derivation moved into the
rewinding schedule might simply not re-run on the resimulated frame and the move
would buy nothing. Read in `bevy_ggrs` 0.22 (`src/snapshot/resource_snapshot.rs:82-95`):
the restore arm is `S::update(resource.as_mut(), snapshot)`, and `ResMut::as_mut`
marks the resource changed UNCONDITIONALLY. ⇒ Every rollback restore of
`AmbitionGameSave` re-arms the gate, so the derivation runs on the replayed
frame.

⚠ **THE SAME READING SAYS THE OVER-FIRE IS HARMLESS, which is the other half a
maintainer would want.** The restore marks the save changed on a frame the
original timeline may not have changed it, so the replay can run the derivation
where the original did not. `emit_intro_flag_chains` skips any target already
present (`game/ambition_content/src/intro/route_state.rs:36`), so an extra run
writes nothing — the operation is idempotent by construction, and the page's own
module docstring says so for a different reason.

⇒ **SO THIS ROW'S COST IS NOW KNOWN AND IT IS SMALL:** a registration move from
`Update` to `app.sim_schedule()`, ordered relative to
`Platformer2dSimulationPhaseMonolith::GameplayEffects` (where `apply_flag_effects`
runs, chained, `features/mod.rs:216`) — after it, to keep today's next-tick
semantics.

⚠ **AND THE COMPOSITION OBJECTION DISSOLVED ON THE SAME PASS**, which is why it
is written down rather than left as a caution: *"may a CONTENT crate register
into the simulation schedule at all"* has an answer in the tree — `ambition_content`
already calls `app.sim_schedule()` in **8** places (`falling_sand.rs:93`,
`encounters.rs:159`, `quests/mod.rs:19`,
`game/ambition_content/src/dormancy.rs:142`,
`falling_sand_sim.rs:223`, and three more). The intro plugin would be doing what
its sibling modules already do. ⇒ Both halves of this row's cost are now
measured, and neither is a blocker.

### 2026-09-18 — this needs TWO roads, not one abstraction, and registration picks

⛔⛤ **THE PAGE AND THE REVIEW BOTH ASK FOR "THE EVENTUAL INGRESS ABSTRACTION",
AND THE TWO MECHANISMS THIS ROW ALREADY SEPARATES DO NOT SHARE A FIX.** The
landed clone road put its consumer on the HOST side
(`MechanicalEditSet::Publish` in `PreUpdate`). Asking whether the same road
works for a REGISTERED request — `NewGameResetRequested`, whose consumer
`process_new_game_reset_request` is a large sim system with `SessionCommands`
and session-world accessors — turns on what the mechanical edit's session
rebase carries.

⚠ **MEASURED, AND IT REFUTED THE FIRST ANSWER I REASONED OUT.** The intuitive
story is that an admitted edit calls `stop_session` first, so the publish
happens in a rollback-free window and a sim-side consumer simply does not run
while the session is down — `sim_schedule()` IS `GgrsSchedule` under the
rollback host (`crates/ambition_platformer2d_rollback_ggrs/src/lib.rs:123`),
and `AdvanceWorld` runs it only with a session. That is wrong in the part that
matters. Driving `stop_session` on a maintainer-owned sync-test harness and
reading `SimTick`: **20 → 25 over 5 steps with a session, then 25 → 29 over the
next 5 with `stop_session` called in between** — the simulation keeps ticking,
one step's tick does not land, and the boundary reads `LocallyRebasable` again
afterwards because `LocalSessionPolicy`'s `autostart` has already rebased. There
is no useful rollback-free window; there is one skipped frame and then a FRESH
window.

⇒ **So the distinction is not "does the sim run" — it is WHAT FRAME ZERO
CAPTURES**, and `warn_if_no_world_to_rewind` states the rule in passing: a
session *"rebases onto whatever is live"* (`session.rs:245-257`). Frame zero is
the live world's ROLLBACK-REGISTERED state. Therefore:

    an UNREGISTERED request   is not in frame zero. A sim-side consumer spends
    (clone, cutscene dismiss)  it on an early — speculative — frame of the new
                               window; a rewind undoes the effect and nothing
                               restores the flag. That is the original defect
                               rebuilt one window later, which is why the clone
                               road moved its consumer to the host side. ⚠ That
                               road is being deleted (see the later 2026-09-18
                               section); the REASONING is unaffected — it is
                               about registration, not about the clone — and the
                               cutscene dismiss is now its live specimen.

    a REGISTERED request      IS in frame zero, so every resimulation restores
    (`NewGameResetRequested`)  the host's write and the spend is retried until
                               the frame carrying it is CONFIRMED. The consumer
                               may stay where it is.

⭐ **WHICH IS A CHEAPER ANSWER THAN THE ROW HAS BEEN ASSUMING FOR THE RESET
HALF.** `process_new_game_reset_request` does not have to move; it needs the
menu's write to arrive as a mechanical-edit publish so the rebase snapshots it.
⚠ AND IT IS NOT FREE: with the flag in frame zero, a rewind inside the window
replays the reset, so the reset must be idempotent under repetition within one
window. Deterministic — every resimulation does the same thing, so no checksum
disagrees — but a teardown-and-rebuild running several times inside one rollback
window is a claim somebody should want to make on purpose. That is the question
this half actually owes, and it is smaller and more concrete than "design an
ingress abstraction".

### 2026-09-18 — the cutscene half is TWO fields, and only one of them is the blocked decision

⭐⭐ **`CutsceneAdvanceRequest` HAS TWO BOOLS AND EVERYTHING ABOVE IS ABOUT ONE
OF THEM.** Read at the producer
(`update_cutscene_request_from_menu`,
`crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs:1078-1091`):

| field | produced from | shape |
|---|---|---|
| `dismiss_dialogue` | `menu_frame.select` | a pure PRESS EDGE — `MenuInputFrame::select` is `select_pressed` (`crates/ambition_input/src/menu.rs:240`), no accumulation anywhere |
| `skip_cutscene` | `menu_frame.back_held`, accumulated in `CutsceneSkipHold` in WALL time past `SKIP_HOLD_THRESHOLD_SECS` | an edge DERIVED FROM A HOLD |

⇒ The `reset_held` question, the *"the hold accumulation then moves into the sim
and must use `WorldTime::sim_dt()` with a rollback-registered accumulator"* cost,
and the census consequence for `CutsceneSkipHold` are **all about
`skip_cutscene` only**. `dismiss_dialogue` carries no hold, no accumulator and no
clock, so none of it applies. The consumer agrees: `tick_active_cutscene` passes
`dismiss` straight into `runtime.tick(dt, dismiss)` and handles `skip` in a
separate branch above it.

⭐ **AND THE DISMISS ALREADY HAS ITS SYNCHRONISED CHANNEL, WITH NO NEW BIT AT
ALL.** `select_held` is fed by `MenuSelect || Jump || Interact`
(`input_systems.rs:962-964`), and `ControlFrame` already carries
`interact_pressed` and `start_pressed`
(`crates/ambition_platformer2d_core/src/control_frame.rs:186,193`) —
synchronised, re-fed
by GGRS on a rewind, and consumed this exact way elsewhere. So option 1 for the
DISMISS is not *"add `reset_held`"*; it is *"read an edge the frame already
has"*. The design tension quoted above (*"cutscene controls are UI/menu intent,
not gameplay movement"*) still applies and is still the maintainer's call — but
it is the WHOLE cost for this field, with no wire change and no rollback-state
consequence behind it.

⛔⛤ **WHY THIS SPLIT IS NOW URGENT RATHER THAN TIDY: THE SHIPPED CONSEQUENCE
LANDED ON THE UNBLOCKED HALF.** `queue.md`'s `CUTSCENE-ROLLBACK-DECISION` records
that on 2026-09-18 `479d5a028` made the hub's boot cutscene real, and
`test_intro`'s third beat is a `CutsceneBeat::Dialogue` with no duration. Getting
past it is a DISMISS, not a skip. So the first-boot risk sits entirely on
`dismiss_dialogue` and does **not** wait on the `reset_held` ruling; the ruling
governs a convenience press on a cutscene the player has already seen.

⚠ The shipped dialogue text *"Hold Reset to skip cutscenes"* was checked and is
accurate: `back_held` is `MenuBack.pressed() || Reset.pressed()`
(`input_systems.rs:965-966`), so the instruction names a button the code really
reads. The page's Reset reasoning is sound — for the skip.

### 2026-09-18 — the population is enumerated, and option 1 costs no new wire bit

⭐⭐ **THE RESIDUE IS FOUR, NOT ONE, AND THE OTHER THREE WERE HIDDEN BY A GUARD'S
SPELLING.** `scripts/check_host_produced_sim_consumed_requests.py` (new, and in
`--maintenance`) asks this question directly: which resource types are SPENT
inside the rewinding schedule and written only outside it. It reports 4 of 57
spent types, all read:

| type | registered | mechanism |
|---|---|---|
| `CutsceneAdvanceRequest` | no | the sim's take stands through the rewind; nothing re-produces the press |
| `NewGameResetRequested` | yes | the rewind restores `false` and ERASES the menu's write |
| `VersusMatch` | yes | filed under `Q140`, not new |

⛤ **A FOURTH ROW STOOD HERE AND THE FEATURE UNDER IT WAS DELETED.**
`SpawnPlayerCloneRequest` (*"no; same as the cutscene; newly visible"*) went with <!-- cite-ok: the player-clone road was deleted in `89d78a4a5`; this line RECORDS the name, it does not point at one -->
the player-clone hotkey on 2026-09-18 (`89d78a4a5`), whose `KeyCode::KeyK`
trigger collided with two shipped input presets.
<!-- cite-ok: the deleted row is quoted here as the record of what this table used to claim -->
⇒ Worth keeping visible rather than quietly deleting, because the row was
carrying an argument that turned out to be false in BOTH directions: it was
chosen as the specimen precisely for being low-stakes, and its trigger was the
most reachable of the four.

⛔ The prose above ("the residue is one") was measured before `c215d6a37`, when
both censuses tested the host side against a tuple of BARE schedule labels
(`"Update"`, `"PreUpdate"`, …). The tree spells a non-rewinding schedule in
QUALIFIED form 39 times, so `request_player_clone_on_key` <!-- cite-ok: the player-clone road was deleted in `89d78a4a5`; this line RECORDS the name, it does not point at one -->
(`bevy::app::Update`) was on NEITHER side of the boundary and its crossing did
not exist. ⚠ The block above is that day's reading and is not edited; this is
the live one.

⭐ **AND THE FIVE FALSE PRODUCERS THE FIRST RUN REPORTED TEACH THE SAME LESSON
THIS PAGE ALREADY LEARNED FROM `SlotControls`.** `reset_session_scoped_resources_
on_activation` holds thirty session-scoped resources and writes every one to
`T::default()` inside a helper — so `BaseGravity`, `CutsceneTriggerQueue`,
`QuestRegistry`, `SwitchActivationQueue` and two phantom entries on
`CutsceneAdvanceRequest`'s producer list all read as intents being raised.
**A write of `T::default()` is the ABSENCE of an intent.** A reason that is true
is not evidence that its subject exists; neither is a mutable parameter.

⇒ **OPTION 1 NEEDS NO NEW WIRE BIT FOR EITHER REMAINING HALF, WHICH IS THE
DECISION-RELEVANT MEASUREMENT.** The section above establishes it for the
cutscene half (`reset_pressed` already exists and is already consumed this way).
For the clone half the answer is not the input payload at all — see the fifth
precedent below. So the *"a New Game bit in every frame's input is a large thing
to spend on a menu press"* objection now applies to exactly one of the four
rows, `NewGameResetRequested`, and that row is the one this page already
suspects is really option 3.

⚠ **AND THE PEER COST OF OPTION 1 IS CURRENTLY ZERO PEERS, WHICH CUTS BOTH
WAYS.** Measured 2026-09-18: `build_sync_test_session` is the ONLY session
constructed anywhere in the workspace, every handle is added as
`PlayerType::Local`, and `AmbitionGgrsConfig = GgrsConfig<ControlFrame>`. There
is no remote peer to renegotiate a wire format with today.
⛔ That is not permission to ignore the wire — `the_bytes_two_peers_exchange`
(`control_frame.rs`) exists precisely because nothing else versioned this
shape — but it does mean option 1's stated cost is deferred, while option 2's
stated risk is NOT: synctest rolls back every frame and compares checksums, so a
host-local buffer drained on a locally-chosen tick fails the shipped detector
immediately rather than in a future netplay session.

⭐⛤ **THE FIFTH PRECEDENT, AND IT IS THE ONE THE FOUR ABOVE DO NOT COVER: THE
MECHANICAL-EDIT ADMISSION.** The four shipped crossings in the table above are
all *"who stands down"* answers for a value two authorities both publish. A dev
hotkey that SPAWNS A BODY is not that shape — it is a mechanical mutation of the
world around a live timeline, which is what
`decide_mechanical_edit_admission` (`local_session.rs:190`) already arbitrates:

```text
NoTimeline         publish -- there is no ring to restore an older value from
LocallyRebasable   stop_session(world) FIRST, then publish into the gap
ForeignTimeline    REFUSE, leaving the proposal pending so it fires later
unhealthy          REFUSE, for the same reason
```

⇒ That is option 3 (*"a session-level operation outside the timeline"*)
generalized, already built, already tested, and **already the road every other
developer edit takes** — `publish_player_stats_edits`,
`publish_editable_movement_tuning`, `sync_developer_body_profile`. A refused
edit is not a lost edit: the proposal stays pending and publishes when the
refusal lifts, which is exactly the property a swallowed press lacks.

⇒ **So the ruling decomposes by WHO RAISES THE INTENT, and only one row is still
genuinely open:**

| who raises it | road | rows |
|---|---|---|
| a PLAYER pressing a control | the device latch → `ControlFrame` → GGRS replays it | `CutsceneAdvanceRequest` |
| an AUTHOR or DEVELOPER editing the world | `MechanicalEditSet` + the admission | *(none — see below)* |
| a MENU ending the match | option 3, session-level | `NewGameResetRequested` |

⚠ **THE MIDDLE ROAD HAS NO ROW LEFT, AND THAT IS A FACT ABOUT THE ROAD RATHER
THAN ABOUT THE TAXONOMY.** Its only example was `SpawnPlayerCloneRequest`, <!-- cite-ok: the player-clone road was deleted in `89d78a4a5`; this line RECORDS the name, it does not point at one -->
deleted with the clone on 2026-09-18.
<!-- cite-ok: the deleted example is named as the row this road has lost -->
The road itself is not hypothetical — three production sites register into
`MechanicalEditSet::Publish` (`platformer2d_runtime/src/player_schedule.rs:308`,
`ambition_portal2d/src/plugin.rs:138`, `ambition_dev_tools/src/sim_plugin.rs:139`,
each a tuple, so the SYSTEM count is at least three) and the admission decider is
held by `the_admission_road_answers_for_the_shipped_ownership_mode.rs` — but no Q136
intent currently takes it, so it cannot be the road a ruling picks by pointing
at a working instance.

⛤ **THIS PARAGRAPH CHOSE `SpawnPlayerCloneRequest` AS THE SPECIMEN TO LAND <!-- cite-ok: the player-clone road was deleted in `89d78a4a5`; this line RECORDS the name, it does not point at one -->
FIRST, AND THE SPECIMEN NO LONGER EXISTS** — kept because the REASONING is the
part a ruling still needs, and because the way the choice went wrong is the
lesson. The reason given was risk rather than ease: `plugins.rs:194`
<!-- cite-ok: the comment went with the clone's plugin registration on 2026-09-18; the coordinate is kept because the ARGUMENT it made is what this paragraph is about -->
explains the
Update/sim split in its own comment and the explanation is CORRECT — `ButtonInput`
is winit frame state, so reading `just_pressed` on the deterministic tick sees one
physical press once per SIM RUN and a frame that steps the sim twice spawns two
clones. Moving the read into the sim reintroduces that. The double-spawn and the
swallowed press are the same problem from its two sides, which is why this is one
ingress question and not three fixes. The stakes are a dev hotkey — no save data,
no peer checksum, no progression — so the road can be built and witnessed here
without a mis-step costing a timeline.

### 2026-09-18 (later) — and the specimen is being DELETED, not preserved

⛔⛤ **THE CLONE HOTKEY IS NOT ISOLATED FROM REAL CONTROLS, AND THAT ALONE ENDS
THE ARGUMENT BELOW.** `request_player_clone_on_key` reads raw `KeyCode::KeyK` <!-- cite-ok: the player-clone road was deleted in `89d78a4a5`; this line RECORDS the name, it does not point at one -->
unconditionally in `Update`, while two SHIPPED input presets bind K to ordinary
gameplay: `wasd_jkl()` gives it to `burst` and `wasd_uipo()` to `utility`
(`crates/ambition_input/src/presets.rs:175`, `:252`). ⇒ **A player on either
preset requests a debug clone every time they use that action.** Found by a
2026-09-18 review; the section below reasoned carefully about the ingress road
for a feature whose trigger was never safe to ship.

⚠ **AND THE "STAKES ARE A DEV HOTKEY" PREMISE IS THEREFORE FALSE**, which is
the part worth keeping as a lesson: I argued this was the right specimen because
*"no save data, no peer checksum, no progression"*. That was a claim about the
INTENT, and the hotkey's reachability was never measured. A specimen chosen for
its low stakes has to have its trigger checked as carefully as its consumer.

⇒ **AND IT IS GONE, in `89d78a4a5` (2026-09-18).** What the deletion took, and
this list is a RECORD of removed names rather than a set of live pointers:
`game/ambition_app/src/app/player_clone.rs`, its plugin registration, both live
<!-- cite-ok: the deleted module — this list is the deletion's manifest -->
clone tests, the `PlayerClone` / `SpawnPlayerCloneRequest` exports, <!-- cite-ok: the player-clone road was deleted in `89d78a4a5`; this line RECORDS the name, it does not point at one -->
`avatar/clone_probe_tests.rs`, and the demo state-machine brain
<!-- cite-ok: the deleted probe module, likewise part of the manifest -->
`StateMachineCfg::PlayerDemo` with its `PlayerDemoCfg` / `State` / `Phase` and
dispatch arms. <!-- cite-ok: the deletion's own manifest — every name here is
gone BY INTENT, which is the fact the row records -->
⛔ NOT the real player-brain path (`tick_player_brain`), and NOT the generic
mechanical-edit infrastructure — that has legitimate author/dev-edit customers
and only the clone's USE of it disappeared. Verified at HEAD: `PlayerClone`, <!-- cite-ok: the player-clone road was deleted in `89d78a4a5`; this line RECORDS the name, it does not point at one -->
`SpawnPlayerCloneRequest` and `PlayerDemo` have zero occurrences in <!-- cite-ok: the player-clone road was deleted in `89d78a4a5`; this line RECORDS the name, it does not point at one -->
`crates/`, `game/` and `tools/`.

⛔⛤ **AND THE DELETION TOOK ONE THING IT SHOULD NOT HAVE, WHICH IS THE LESSON
THIS ENTRY IS ACTUALLY FOR.** Inside the clone's comment banner sat
`declare_death_rules(UntaggedRooms, replay_level_after(0.0))` — Ambition's own
answer to *"what happens when the last participant dies"*, nothing to do with
the clone, and the banner's own text said so (*"composed into the multi-game
shell host beside Sanic, Mary-O and Smash"*). It went with the braces. With no
`UntaggedRooms` claim, `DeclaredDeathRules::governing(None)` falls through to
`DeathRules::default()` = `LevelReset::Never`
(`crates/ambition_combat/src/death_rules.rs:119-122`), so ordinary Ambition
rooms stopped putting the level back. Restored 2026-09-18 as
`declare_ambition_death_rules`, its own function with its own call site.

⇒ **THE WITNESS WAS ALREADY THERE AND ALREADY CORRECT.**
`the_host_composes_three_games_each_scoped_to_its_own_rooms`
(`game/ambition_app/tests/a_game_governs_only_its_own_rooms.rs`) asserts the
composed host declares all three scopes, is included from `app_it.rs:27`, and
fails on the deletion with `Declaring: [Mode("sanic"), Mode("mary_o")]`. What
was missing was a Rust run between the deletion and the branch — and the same
gap hid a red Python arm (`test_no_waiver_names_a_system_that_no_longer_mutates_-
rollback_state`), because the `--maintenance` lane runs neither. ⚠ Two guards
pointing straight at one commit, both unread for an afternoon: a deletion is
exactly when the suites that were green yesterday stop being evidence.

⭐⭐ **AND THE LESSON IS NOW AN INSTRUMENT, WHICH FOUND A SECOND ONE THE SAME
DAY.** The clone was found by a reviewer reading `presets.rs` by hand; nothing
in the tree could have found it twice.
`scripts/check_raw_key_reads_do_not_collide_with_presets.py` states the
invariant — *the preset table is the only thing allowed to decide what a bound
key means* — and fails on any raw `KeyCode` read, outside `ambition_input`, of
a key a preset binds, unless a row says what keeps the two meanings apart.
MEASURED 2026-09-18: 17 raw reads outside the input crate, 4 of them on
preset-bound keys.

| key | site | verdict |
|---|---|---|
| `ShiftLeft` / `ShiftRight` | `developer_hotkeys.rs:82` | ✅ a MODIFIER qualifying another hotkey, never an action of its own |
| `KeyR` | `basic_presentation.rs:71-73` | ✅ gated by `LoadForegroundPhase::Failed`, so it can only fire on a failed LOAD screen where `arrows_qwer`'s `secondary` has nothing to act on |
| `KeyN` | `map/input.rs:40` | ⛔ **LIVE — the clone's defect exactly** |

⛔ **`KeyN` TOGGLES THE MINIMAP AND TAUNTS.** `handle_map_menu_hotkeys` reads
the raw key and runs whenever a session world exists, `.after` `CoreSimulation`
(`crates/ambition_menu/src/map/mod.rs:226-239`), while **both** shipped presets
bind `N` to `taunt` — `wasd_jkl` and `wasd_uipo`, the same two whose `K`
binding killed the clone. It is milder than the clone (no rollback state, no
simulation write, so nothing desyncs) and it is the same mechanism, so it is
FILED with its reading rather than waived. The repair is to route the toggle
through a bound action the way `M`'s sibling intent already is, not to pick a
different raw key — a different raw key is the same bug waiting for a preset to
grow.

⭐⛤ **MEASURED 2026-09-19, AND “PICK A DIFFERENT RAW KEY” IS NOT MERELY WORSE —
THERE IS NO KEY TO PICK.** Across all FOUR keyboard presets
(`crates/ambition_input/src/presets.rs`: `arrows_zxc`, `wasd_jkl`,
`arrows_qwer`, `wasd_uipo`, 21–22 keys each), **exactly ONE letter key is bound
by no preset: `M`** — and this same menu already reads it. ⇒ The free-key
shortlist for a minimap toggle is EMPTY, so the bound-action repair is forced
rather than preferred, and the maintainer's decision is not *"which key"* but
*"which existing action yields one, or does `minimap` arrive as a modifier on
`map`"*.

⚠ **AND `M`'s SAFETY IS A COINCIDENCE OF THE CURRENT PRESET SET, WHICH IS THE
SAME FRAGILITY ONE STEP BEHIND.** `map/input.rs` reads TWO raw keys, `KeyM` and
`KeyN`. `M` does not collide only because it is the one letter nobody binds;
the moment a preset takes it, `M` becomes `N`. The difference between them is
luck, not design — `M` at least has `menu.map` beside it (bound to `Tab` in
`wasd_jkl`), so the raw read is a debug path over a real action, while `N` has
no action at all.

⚠ **A COUNT OVER PRESETS MUST COUNT ALL OF THEM.** This measurement first read
FOUR free letters — `C`, `M`, `X`, `Z` — because it took the three presets this
row's own paragraph names. The fourth, `arrows_zxc`, binds exactly `Z`, `X` and
`C`; its name says so. The population was in the file, not in the prose above
it.

⚠ **THE FIRST VERSION OF THE GUARD REPORTED A FIFTH AND IT WAS AN ARTEFACT.**
`presets.rs` ends with a `KeyCode::KeyM => "M"` match that turns a keycode into
a label for the rebinding UI, so a grep for `KeyCode::(\w+)` counted 39 bound
keys where the answer is **35** — and the inflation produced a finding on
`KeyCode::KeyM` in the map menu, which no preset binds at all. A display table
reads exactly like a binding table. The filter matches a STRUCT FIELD
ASSIGNMENT, and an arm asserts `KeyM` is still label-only so the distinction
cannot quietly stop working.

⭐ **WHAT SURVIVES IS THE GENERAL CONCLUSION, AND IT IS THE USEFUL HALF:**
author and developer world mutations belong at the mechanical-edit boundary;
player intent does not automatically belong there. The clone was evidence for
that sentence and the sentence does not need it — the properties the demo brain
existed to demonstrate are independently pinned by
`player_brain_seam_translates_control_frame_to_actor_control` (the player
brain → `ActorControl` seam) and by `multiplayer_smoke_tests.rs` (several
`PlayerEntity`s, exactly one `PrimaryPlayer`, independent per-player state).

⇒ **AND Q136 IS A CLEANER QUESTION WITHOUT IT.** The live rows are the menu,
cutscene and player-intent roads — `CutsceneAdvanceRequest`,
`NewGameResetRequested`, `PlayerHealRequested`, `AmbientGravityRequest` — plus
the derived `SetFlagRequested`. A dev hotkey nobody ships was distorting the
design discussion by being the easiest thing to reason about.

⚠ The section below is kept as the record of how the road was built and what the
fixture cost, because the FIXTURE lessons are general (the anti-vacuity arms,
the ownership stamp, the poison that did not apply). Read it as history: its
subject no longer exists.

### 2026-09-18 — the first road is landed, and the fixture was the hard part

⛔⛤ **ITS SUBJECT WAS DELETED LATER THE SAME DAY (`89d78a4a5`), AND THE SECTION
STAYS ANYWAY.** `SpawnPlayerCloneRequest` and both of its witnesses are gone <!-- cite-ok: the player-clone road was deleted in `89d78a4a5`; this line RECORDS the name, it does not point at one -->
with the clone. What this section actually found is not about a clone: it is
about the ADMISSION ROAD, the ownership stamp, and how a rollback fixture can
spend a whole run measuring nothing. Those findings have a live holder —
`game/ambition_app/tests/the_admission_road_answers_for_the_shipped_ownership_mode.rs`,
which keeps BOTH arms of the fold (`Caller` → `ForeignTimeline` → refuse;
`LocalMaintainer` → `LocallyRebasable` → publish) and is strictly better than
the clone witness was, because a decider that refused everything forever would
have satisfied the clone arm alone.

⭐⭐ **THE ROAD ITSELF WORKED, WHICH IS THE PART THAT GENERALISES.**
`spawn_requested_player_clone` stopped running in `app.sim_schedule()`: the <!-- cite-ok: the player-clone road was deleted in `89d78a4a5`; this line RECORDS the name, it does not point at one -->
request was proposed in `MechanicalEditSet::Propose` and the spawn published in
`Publish`, both in `PreUpdate`, with `decide_mechanical_edit_admission` between
them — and poisoning the registration back into the sim schedule reddened the
rollback arm while the fixed-tick control stayed green. The two witnesses that
recorded it, `a_dev_clone_survives_a_rewind`
<!-- cite-ok: the test is RECORDED as deleted in this sentence, not offered as a pointer -->
and `a_press_the_spawn_cannot_honour_yet_is_kept_rather_than_consumed`
<!-- cite-ok: likewise deleted with its subject in `89d78a4a5` -->
were deleted with their subject.

⇒ Two repairs came with it, each a separate defect the move exposed:

- **the press is spent LAST now.** `request.0 = false` stood above every refusal
  in the spawn, so a press arriving on a frame with no resolvable primary — or
  with a primary not yet carrying a `SimId` — was consumed and the clone never
  appeared. Refusing a sub-step is not refusing the operation — the one line of
  this section that outlived its subject, and it is now in the run's standing
  DISCIPLINE rather than in a test.
- **`check_rollback_mutators_run_in_sim.py` caught the repair's own side effect
  before it was committed.** Minting the clone's identity increments the
  primary's `SimIdCounter`, which is rollback state, now from `PreUpdate`. It
  arrived as a NEW offender on the first run after the move and is waived with
  the admission argument — which is the population instrument doing exactly the
  job it exists for.

⛔⛤⛤ **AND THE BIGGEST DEFECT WAS NOT Q136 AT ALL: THE CLONE DESYNCED THE
TIMELINE, AND NOTHING COULD SEE IT BECAUSE NOTHING COULD SPAWN ONE UNDER
ROLLBACK.** With the press finally arriving, the witness's health read came back
`GGRS sync-test checksum mismatch at frames [14, 15, ..]` on every run.
`tick_player_clone_brains` is registered into the SIM schedule and read <!-- cite-ok: the player-clone road was deleted in `89d78a4a5`; this line RECORDS the name, it does not point at one -->
`time.delta_secs()` — the app's WALL dt — accumulating it into a
`PlayerCloneClock` resource that was `init_resource`d and never registered for <!-- cite-ok: this block RECORDS the deletion of `PlayerCloneClock`; the name is gone by intent -->
rollback. A resimulated frame therefore added dt AGAIN to a value no rewind
restored, so `snapshot.sim_time` differed between the original run and the
replay, the demo brain emitted a different frame, and the clone's
`BodyKinematics` diverged.

⇒ **IT IS A DUPLICATE AUTHORITY, AND THE COLLAPSE IS THE FIX RATHER THAN A
REGISTRATION.** `GameplayElapsed` is the same fact, accumulated the same way
(`+= world_time.scaled_dt`), rollback-registered, advanced at the head of
`WorldPrep`, and its own doc says *"before any actor brain reads the snapshot"* —
which is exactly where `tick_player_clone_brains` reads. Registering
`PlayerCloneClock` would have made the drift rewind correctly and left two owners <!-- cite-ok: the same deleted type, named as the road not taken -->
of *how long gameplay has run*; deleting it leaves one. ⭐ The `dt` moved to
`WorldTime::sim_dt()` in the same edit, which also sharpens the pre-existing zero
guard: `sim_dt` is `raw_dt * time_scale`, so it is zero while PAUSED or in
hitstop, and ticking a demo cycle through a pause was never intended.

⚠ **THIS IS THE THIRD TIME THIS WEEK A HOST-LOCAL ACCUMULATOR INSIDE THE REWIND
HAS BEEN THE DEFECT**, and `sim_plugin.rs` names the other two at the
registration that moved them out: `sync_developer_body_profile` was *"arbitrated
by a `Local` that runs once per ADVANCE and therefore remembered across a
rewind"*, and `sync_live_player_dev_edits_system` wrote five movement clusters <!-- cite-ok: the QUOTE is accurate and the quoted name has no definition -- see the note below -->
from a live inspector resource. ⇒ Worth a guard of its own: a system in the sim
schedule that accumulates into a `Local` or into an unregistered resource is the
shape, and all three instances were invisible to every existing census because
none of them is a *multi-writer* and none of them crosses a schedule boundary.

⚠ **AND THE SECOND OF THOSE TWO NAMES DID NOT EXIST, WHICH WAS A FINDING
ABOUT TEN SOURCE COMMENTS RATHER THAN ABOUT THIS ROW.**
`sync_live_player_dev_edits_system` had no definition anywhere in the tree — <!-- cite-ok: this sentence REPORTS that the name had no definition -->
measured 2026-09-18 — yet ten sites named it as a live system, including an
intra-doc link in `crates/ambition_dev_tools/src/lib.rs` calling it *"the
host-scheduled system that applies live ability/tuning edits to the player each
frame"*. ✅ **REPAIRED THE SAME DAY** (`651d265e7`, by CalculexAmbition): the
system that actually carries that sentence is `project_editable_abilities`, and
it calls the plain helper `sync_live_ability_edits_clusters`
(`crates/ambition_dev_tools/src/dev_tools/editable.rs:802`) to do the cluster
mutation — so the decomposition had changed the SHAPE as well as the name, and
a straight rename onto the helper would have left every sentence claiming a
schedule for a function that has none.

⛔⛤ **AND THE MOST TRANSFERABLE FINDING IS ABOUT THE FIXTURE, NOT THE FIX: NO
TEST IN THIS WORKSPACE EXERCISED THE OWNERSHIP MODE THE GAME ACTUALLY RUNS IN.**
The first version of the witness reported 0 clones under rollback and 1 under
fixed tick — which reads exactly like the defect it was written for. It was not.
The diagnostic printed `boundary=ForeignTimeline admission=Refuse` on every tick,
forever:

| who installs the session | owner stamp | `locally_rebasable_timeline` | a mechanical edit |
|---|---|---|---|
| `start_sync_test_session` — every rollback test fixture, via `with_sync_test_rollback_settings` | `SyncTestOwner::Caller` | **false** | **REFUSED** |
| `maintain_local_session` — the shipped game, once `LocalSessionPolicy` is armed | `SyncTestOwner::LocalMaintainer` | true | admitted, baseline rebased |

⇒ The refusal is CORRECT — a harness that installed its own timeline did not ask
for it to be rebased — which is what makes it dangerous: every rollback arm in
the suite sat on the refusing side of the admission road and none of them said
so. Measured WHEN THIS WAS WRITTEN: `grep -rn LocalSessionPolicy` over
`game/ambition_app/tests/` and `crates/ambition_sim_harness/src/` returned
**nothing**, and the dev-tools unit fixtures insert
`MechanicalEditAdmission::Publish` directly, bypassing the decider. So the
`LocallyRebasable` arm — the one the game takes — had no coverage at all.
The first fixture that re-owned its session the shipped way stopped the
caller-owned session and let the maintainer build its own, because the owner
stamp and the installed session are one fact and writing half of it describes a
world that cannot exist. ⇒ That helper is what survived the clone's deletion:
it is now `maintainer_owned_rollback_sim` in
`game/ambition_app/tests/common/mod.rs:389`,
used by the admission-road test above — the gap this paragraph measured is
still closed, by a holder that does not depend on a debug hotkey.

⛔⛤ **AND THE TENSE ABOVE IS A CORRECTION, NOT A STYLE CHOICE — THIRD INSTANCE
OF THE SAME SHAPE TODAY.** The sentence said *"Measured 2026-09-18 … returns
nothing"*, in the present, about a gap the very next sentence says is closed.
Re-derived 2026-09-18: `LocalSessionPolicy` now appears six times in
`game/ambition_app/tests/`, three of them as CODE, in the fixture that closed
it (`common/mod.rs:367` and `:377`); the harness crate is still clean. ⇒ The
fixture's own doc comment already said *"Measured when this was written"* and
was right; this page copied the measurement without copying the tense.
Alongside `Q134`'s *"`ambition_dialog` contains the string `rollback` zero
times"* and the same claim in `queue.md`, that is three sentences in one page
tree whose evidence their own repair falsified. **A measurement that a fix
would change has to say when it was taken, in the sentence, not in a heading.**

⚠ **TWO SMALLER THINGS WORTH A READER'S TIME, both met while building that
fixture.** `LocalSessionPolicy`'s DEFAULT is `check_distance: 0` — rollback
dormant — so the shipped composition has no rewind until something arms it (the
rollback observatory does). And inverting `check_distance` and
`max_prediction_window` does not fail loudly: `maintain_local_session` catches
GGRS's `Invalid Request: Check distance too big`, records it in
`LocalSessionOwnership::last_error` and carries on with NO session installed —
so a test that got the order wrong silently becomes a no-rollback test that
passes, which is how the first version of this witness spent a run measuring
nothing.

⚠ **AND ONE CLAIM RETRACTED IN THE SAME BREATH, BECAUSE IT READ LIKE A DEFECT
AND IS NOT ONE.** That decline logs *"failed to BUILD the local GGRS session,
keeping the running one"* while `AmbitionGgrsSession` was absent, which looks
like a message describing an impossible state. It is not: the decline sits
inside `maintain_local_session`'s *"PREPARE: every way this can decline, while
the old session still runs"* block — the atomicity the 2026-09-17 review
imposed — so in every ordinary call there IS a running session to keep. The
fixture had stopped it first. ⇒ A message can be false in a fixture and true in
the code, and the difference is who stopped what.

## Q135 — should GGRS start before the durable restore has finished?

✅ **ANSWERED AND LANDED 2026-09-16: NO, AND IT NO LONGER CAN.** (The heading
keeps the question because five other planning rows link to this anchor.)

**The session-start gate is in.** `maintain_local_session` now refuses to CREATE
a rollback session while a durable restore is pending, so no simulation tick is
ever run over a world whose save is still being applied. Held by
`a_conversation_on_the_first_tick_of_a_session_is_counted_exactly_once`
(`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`), which
poison-verifies both the gate and the predicate behind it.

⛔⛤ **AND THE HARD-WON PART IS THE PREDICATE, NOT THE GATE. `SaveRestored` IS
NOT A LATCH THAT ALWAYS RISES.** The obvious implementation — wait while
`!restored.0` — was written, measured, and **failed 66 app tests** with *"the
match seats a first fighter"* and *"the opening ceremony never released the
cast"*. `complete_durable_restore` needs exactly one `PrimaryPlayerOnly` body
carrying a `BodyWallet`; a smash match never has that singleton, so **its latch
reads false for the entire process.** Gating on the bare boolean hung every
smash composition at session start.

⇒ `SaveRestored` is a **completion fact about one domain in one experience**, not
a readiness fact about the world, and "nothing to hydrate" and "hydration
pending" are the same bit in it. The gate therefore asks a three-valued question
that `session::durable_horizon::durable_hydration_is_pending` owns: the save is
unapplied **and** this world has the body that lets it be applied. One fact, one
owner, read over a dependency edge (`rollback_ggrs -> actor_monolith`) that
already existed — no readiness flag mirrored into a lower layer, and no new state
machine.

⚠ **AN EARLIER NOTE IN THIS FILE SAID THE ROLLBACK-HOST CRATE COULD NOT LEGALLY
SEE `SaveRestored`, AND THAT WAS WRONG.** `ambition_platformer2d_rollback_ggrs`
already depends on `ambition_platformer2d_actor_monolith` and uses it heavily
(`lifecycle_commit.rs`). That false belief was the entire reason this item was
held for a ruling about introducing a new lower-layer capability. There was
nothing to introduce.

⚠ The predicate's body condition restates `complete_durable_restore`'s own, and
the drift guard is those 66 tests: if it ever reports "pending" where the system
cannot complete, every smash fixture in `app_it` hangs and names itself.

**The original question, for the record.** `maintain_local_session` started the
rollback session on `session_world_entity(world).is_some()`, while the
durable-restore chain — `adopt_occurrence_checkpoint_from_save`,
`restore_inventory_from_save`, `complete_durable_restore` — waited for a primary
player BODY, a later fact. Both lived in top-level `Update` with **no ordering
edge between them.**

**MEASURED 2026-09-16**, `probe_when_the_durable_restore_latch_flips_against_ggrs_start`
in `game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`:

| | session world | primary body | GGRS live | latch set |
|---|--:|--:|--:|--:|
| first frame true | 1 | 1 | 1 or 2 | 2 |

- A sampler with an explicit `.after(complete_durable_restore)` edge finds the
  GGRS session **already live** at the instant the latch is set.

⛔⛤ **RE-MEASURED 2026-09-16, AND THE ANSWER IS NOT A FRAME COUNT. WHETHER THE
PRE-HYDRATION HOLE EXISTS AT ALL IS DECIDED BY UNRELATED `Update` MEMBERSHIP.**

The between-frame probe above reports when GGRS becomes **live**. Liveness is not
**advance**: a session can exist for a frame without stepping the timeline, and a
frame of that shape carries no tick for anything to happen on. So the question
was re-asked with a recorder INSIDE the simulation schedule, in
`FeatureInteractionSet::Actuate` — where the real conversation opener sits —
reading `SaveRestored` at the instant an opener would read it.

Two worlds, identical but for whether ONE unrelated `Update` system is installed
(this file's own within-frame sampler):

```text
sampler absent    first simulated tick = tick 0 on host frame 3, latch TRUE
sampler present   first simulated tick = tick 0 on host frame 2, latch FALSE
```

⇒ In one composition **no tick is ever simulated unrestored** and the hole does
not exist. In the other, **exactly one tick — tick 0, the first tick of the
session — is simulated with the latch false.** Same options, same recorder, same
harness.

⭐ **THAT IS WHY THE GATE EXISTS, AND IT IS A STRONGER REASON THAN A WINDOW
WIDTH.** The
defect is not "a window of N frames", which could be argued down by making N
small. It is that **nothing orders durable hydration against the start of the
synchronised timeline**, so the answer is decided by whichever systems happen to
share `Update` — a property no reviewer of either system can see. Adding an
unrelated system to `Update` can open the hole; removing one can close it. A
lifecycle that is correct by coincidence is the thing option 1 exists to end.

That defect was held by an arm asserting the PAIR — **1 visit** when hydration
won the race, **0 visits** when it lost — and the repair inverted it, as the row
above said it had to. The arm is now
`a_conversation_on_the_first_tick_of_a_session_is_counted_exactly_once`
(`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`): it
still runs BOTH compositions with the same first-tick conversation opener, and
both now read **1 visit** with an EMPTY unrestored-tick list. It also carries a
premise guard the pre-repair form did not need — *"a world that never simulates
is not a world that never loses a visit"* — because the failure this repair can
plausibly cause is a gate that refuses to start a session at all.

⛔ The edge stays an edge. Satisfying this by relaxing the counter's
`opened_at == tick` to `opened_at <= tick` was explicitly out then and is still
out: that turns an edge into a level and over-counts every tick a conversation
stays live.
- `RollbackFrameCount` reads **1** there — timeline frame one, inside a check
  distance of four, so a resimulation reaches back past the write.
- All three restored resources are `rollback_resource_clone_checksum`
  registrations. A rewind across frame 1 restores them to their pre-write
  snapshot, and `Update` does not re-run.
- The gap is not stable: adding ONE exclusive system to `Update` moved the
  session start from frame 2 to frame 1 and shortened the boot by a frame. Each
  configuration is repeatable (3/3 and 6/6) and they disagree.

⇒ So the session-scope waivers' *"the write precedes the timeline"* is not merely
unavailable here; **the reverse is what happens.**

**THE RULING.** Either

1. **GGRS must not start until the durable restore is complete.** Gate
   `maintain_local_session` on `SaveRestored`, or on a broader "the session world
   is finished loading" fact, so a synchronised timeline never begins over a world
   that is still being filled in. This is the semantically clean answer and it is
   a lifecycle change in `ambition_platformer2d_rollback_ggrs::local_session`.
   ⚠ It needs a decision about what else belongs behind the same gate, or the next
   loader to appear reopens this.
2. **The restore chain must move inside the rewinding schedule**, so a rewind
   re-derives what it wrote. ✅⛤ **ITS PREREQUISITES WERE ALREADY MET AND ITS
   BLOCKING DEFECT IS NOW FIXED, BUT IT WAS NEVER THE WHOLE ANSWER — all
   measured 2026-09-16.** This option used to say it *"needs `SaveRestored` to
   become rollback state and the 'one-shot at boot' shape to survive being
   replayed"*. Against the tree: `SaveRestored` **is** `rollback_resource_clone`,
   `ResetToCheckpoint` **is** `clear_message_on_rollback` (so the one-shot's
   effect already takes the `ItemGrantRequested` road), and `adopt_the_ledger`
   is a pure function of `AmbitionGameSave`. Nothing had to be built first.

   ⛔ Moving the three systems was tried and was **necessary without being
   sufficient**: the outside set emptied and the checksum mismatch stood. The
   real cause was that `AuthoredOccurrences` was not a derived resource — see
   the [DURABLE-HORIZON-CHECKSUM row](queue.md#durable-horizon-checksum--the-save-mirrors-write-hashed-state-from-update)
   for the measurement and the repair (`rollback_resource_clone_checksum`,
   schema v195). After it, **no entry of the 364 probed disagrees between two
   passes of one frame** outside world construction.

3. ~~**The writes do not matter**~~ — ⛔ **REFUTED BY MEASUREMENT, so this is a
   two-way ruling and not a three-way one.** A mid-session load staged at tick 40
   inside the rewinding schedule makes
   `written_outside_the_rewinding_schedule()` return BOTH
   `["...continuity::OccurrenceBaseline", "...custody_horizon::CustodyBaseline"]`
   and made the sync test report
   `Err("checksum mismatch at frames [38, 39, 40]")` — a real desync at the frames
   of the load. ⚠ THE DESYNC IS FIXED NOW (schema v195; the ledger is registered),
   and the refutation stands on what replaced it: the write is not harmless, it is
   **discarded**, so the durable restore silently does not reach the ledger. The staging system's own writes to `AmbitionGameSave` and
   `SaveRestored` are inside the schedule and do NOT appear in the outside set;
   what appears is the pair of baselines, whose only writer is
   `adopt_occurrence_checkpoint_from_save` in `Update`. ⇒ **Two hashed resources,
   not one, so option 1's gate has to cover the whole `adopt_the_ledger` call and
   not a single field.**

⚠ **AND THE DETECTOR IS GREEN FOR A REASON THAT IS NOT SAFETY.**
`no_registered_type_is_written_outside_the_rewinding_schedule` can see these types
— they are value-probed — and passes because the harness boots with NO SAVE FILE,
so `adopt_the_ledger` writes the same empty value it found. The comparison is
between two identical censuses. Do not quote that arm's green against this question.
✅ The seeded save now exists —
`probe_what_a_mid_session_load_writes_outside_the_rewinding_schedule` in
`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs` — and it is
`#[ignore]`d because it demonstrates an unfixed defect. ⛔ Note its fixture shape:
staging from OUTSIDE the timeline does nothing, because both the save and the
latch are rollback-registered and the next rollback restores them. The staging
must live in the rewinding schedule.

⭐ **AND THE DETECTOR NOW NAMES THREE WRITERS, NOT TWO, WHICH IS THE v195
PROMOTION WORKING.** `outside_after` reads
`[AuthoredOccurrences, OccurrenceBaseline, CustodyBaseline]`. The ledger joined
the set because the detector's population is REGISTERED types, and a
`declare_rollback_derived_*` type was never in it — so the `Update` write that
caused everything was invisible to the one instrument named after it. ⇒ A
detector whose population is "registered" cannot see a write to something that
declared itself derived, and a false derived declaration therefore removes its
own subject from the guard. Same shape as the presence-probe blindness, one layer
out.

⛔⛤ **AND ONE DEPENDENT INVARIANT, FOUND BY REVIEW RATHER THAN BY MEASUREMENT —
NOW CLOSED BY THE SAME GATE.** `count_the_dialogue_visit_when_a_conversation_opens`
(Q134's repair) early-returns on `!restored.0`, and the table above showed a real
interval where the session world existed, a primary body existed, GGRS was
running and `SaveRestored` was still false. `interact_ecs_actors_and_switches` is NOT gated on
the latch. ⇒ A conversation opened in that interval was counted by nobody: the
counter declines while `opened_at == tick` is true, and by the time the latch
rises that equality is permanently false. ⚠ **The edge must not be relaxed to
`opened_at <= tick` to paper over this** — that is poison-verified to overcount
(6 visits for 5 openings). The gate removes the interval and with it the hole,
which is why this belonged here rather than in Q134.

✔ **THAT ACCEPTANCE IS DISCHARGED.**
`a_conversation_on_the_first_tick_of_a_session_is_counted_exactly_once` opens a
conversation on tick 0 — the earliest openable moment of a session, and the only
tick the measurement ever found running unhydrated — in BOTH compositions, and
asserts one visit in each with the unrestored-tick list empty in each. The edge
was NOT relaxed to `opened_at <= tick`; the gate removed the interval instead,
which is what made the counter's `!restored.0` guard unreachable with a live
conversation behind it rather than merely tolerable.

✅⛤ **AND ONE OF THIS QUESTION'S TWO ROADS IS GONE, 2026-09-16: THERE IS NO
MID-SESSION SAVE REPLACEMENT ANY MORE.** Censused: `SaveRestored` was lowered in
exactly ONE place in the whole codebase — `reset_inventory_on_new_game`'s closing
`restored.0 = false`, on `NewGameResetCommitted`. It did that so the generic load
chain would re-adopt `OccurrenceBaseline` and `CustodyBaseline` from the wiped
file; every other fresh-run durable fact was already reset in that same function.

⇒ Those two are reset directly now, the latch stays true, and New Game is a
self-contained simulation transaction. **So the only `false -> true` transition
left is initial session activation** — which is the case option 1 is about, and
an arbitrary "load another save while the rollback game continues" road is no
longer being created by accident.

⛔⛤ **THE WINDOW IT CLOSED WAS TWO FRAMES WIDE AND INVISIBLE TO THE OBVIOUS
INSTRUMENT.** Sampling `SaveRestored` between `sim.step()` calls reported it TRUE
for the whole run with the defect fully present, because the sim schedule lowered
it from `PreUpdate` and the `Update` chain raised it again in the same frame. The
arm had to sample at the HEAD of that chain
(`.before(adopt_occurrence_checkpoint_from_save)`) to see frames 33 and 34. ⇒ A
latch that is lowered and re-raised within one frame is invisible to any
between-frame reader, and the first version of that acceptance passed with the
defect live. Held by
`a_new_game_clears_the_occurrence_baselines_without_lowering_the_latch`.

## Q134 — is a dialog visit count something two peers must agree on?

✅ **THE DEFECT THIS QUESTION WAS BLOCKING IS CLOSED; WHAT IS LEFT IS THE PRODUCT
QUESTION IN THE TITLE.** Read the ✅⛤ paragraph before the ruling below — the
increment is in the rewinding schedule and a visit survives a rewind. This entry
keeps its measurements because they are what made Option 1 refusable.

[DURABLE-HORIZON-CHECKSUM](queue.md#durable-horizon-checksum--the-save-mirrors-write-hashed-state-from-update)
repaired the three `persist_*_to_save` mirrors by moving them into the rewinding
schedule: they DERIVE the save from simulation state, so a replay reproduces the
value. `dispatch_pending_dialog_requests` WAS a fourth writer of the same hashed
resource and that answer was not available to it — it called
`save.data_mut().increment_dialog_visit(&dialogue_id)` from `Update`. ⚠ It no
longer does; re-measured 2026-09-18, that method has exactly one production call
site and it is in the sim schedule.

⛔⛤ **THIS QUESTION USED TO ARGUE FROM "AN INCREMENT IS NEITHER IDEMPOTENT NOR
DERIVABLE". BOTH HALVES ARE NOW MEASURED FALSE, AND BOTH OF ITS OPTIONS CHANGE
SHAPE AS A RESULT.** Measured 2026-09-16 by three arms in
`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`:

| placement of the increment | after 200 frames of sync test |
|---|---|
| `Update` (as it shipped) | the visit is **LOST** — `a_dialogue_visit_counted_from_update_is_taken_back_by_the_rewind` |
| inside the sim schedule, 5 known ticks | **exactly 5** — `an_increment_inside_the_tick_is_made_idempotent_by_the_restore` |
| inside the sim schedule, no rollback session | exactly 5 — the control |

⇒ **THE RESTORE MAKES AN INCREMENT IDEMPOTENT.** A resimulated tick does not add
to what the previous run left: the snapshot puts `AmbitionGameSave` back to its
state BEFORE the tick, so every replay adds one to the same base. Non-idempotence
only bites a write the snapshot cannot reach, which is exactly where this one is.
⚠ The in-schedule arm fires on FIVE separate ticks on purpose — one tick reaching
1 is also what "no replay happened" looks like — and it asserts
`live_comparisons > 0` so the rewind is a witnessed premise rather than an
assumption.

**MEASURED 2026-09-16, and the measurement closes one of the two branches the row
had been holding open.** The row asked whether the visit is LOST on a rewind or
COUNTED TWICE. It can only be lost:

- `DialogState` is a plain `#[derive(Resource)]`, registered on no road:
  re-derived 2026-09-18, no `rollback_resource*`, `register_rollback*`,
  `clear_*_on_rollback` or `SessionScopedResources` mention names it anywhere in
  the workspace. ⛔⛤ THIS SAID *"`ambition_dialog` contains the string
  `rollback` **zero times**"* until 2026-09-18, and it contains it twice now —
  both inside the comment at `crates/ambition_dialog/src/bridge.rs:160-167`
  that records THIS finding and the repair it caused. A crate-wide string count
  is a fine way to find a road and a poor way to own a claim, because writing
  the claim down falsifies it. The substance is unchanged; the sentence that
  carried it could not survive its own result.
- The dispatcher consumed the request with `state.pending_start.take()`, in
  `Update`.
- ⇒ A rewind restores `AmbitionGameSave` to its pre-increment value. The request
  that produced the increment was consumed from a resource that does not rewind,
  so it does not come back and nothing re-runs the dispatcher. One outcome.

**AND THE PATH IS LIVE IN A ROLLBACK SESSION.** `game/ambition_app/src/app/plugins.rs`
installs the Yarn stack under `#[cfg(feature = "ui")]` and nothing else — it is
not gated on `simulation_host.is_rollback()`, which is checked thirteen lines
above for `AmbitionRollbackPlugin`.

**AND THE FIELD IS INSIDE THE COMPARED VALUE.**
`AmbitionGameSave::checksum` serialises the whole save with
`ron::ser::to_string(&self.0)`, so `dialog_visits` is part of what two peers
compare. Nothing narrows it today.

**THE RULING.** Either

1. ~~**A visit count is durable PROGRESS**, and the save's checksum projection
   should stop covering fields no tick derives.~~ ⛔ **THIS IS NOT A REPAIR, AND
   THE DISTINCTION IS IN THE REGISTRATION.** `install_resource_clone_checksum`
   (`crates/ambition_platformer2d_rollback_ggrs/src/registration.rs`) installs
   `rollback_resource_with_clone` and `checksum_resource` INDEPENDENTLY.
   Narrowing the checksum changes only what two peers compare; the whole
   resource is still snapshotted and restored, and **the restore is what loses
   the visit.** ⭐ Measured, not read: `OwnedItems` is `rollback_resource_clone`
   — restored, in no peer checksum at all — and
   `a_bag_changed_from_update_is_silently_taken_back_by_the_rewind` has been
   green over its lost `Update` write all along. ⇒ A ruling here would silence a
   peer disagreement and keep the data loss. It remains a live question about
   what peers should AGREE on; it is not an answer to this defect.
2. **A visit count is simulation state**, and the increment moves into the
   rewinding schedule. ⭐ **This is now much cheaper than this entry used to
   price it, and `ambition_dialog` needs no rollback vocabulary.** The measured
   idempotence above means the increment itself is safe there; what it needs is
   a replayable EDGE to fire on, and one already exists.
   `ambition_conversation::ActiveConversation` is rollback state
   (`rollback_resource_clone_entity_set_probed` + `rollback_resource_map_entities`)
   carrying a deterministic `ConversationInstanceId` with an `opened_at` tick, and
   `project_the_dialog_ui_from_the_conversation` already treats the opening as
   simulation-owned and the Yarn box as its projection. ⇒ The repair is the same
   one the three `persist_*_to_save` mirrors got: count the visit in the sim
   schedule off "this instance became live", and delete the presentation
   dispatcher's write. Yarn stays presentation-side.

⇒ This is narrower than
[Q129](#q129--must-the-save-file-be-part-of-what-two-peers-agree-on) and does not
wait on it: Q129 asks whether the save belongs in the CHECKSUM, and the
measurement above shows that answer cannot reach this defect either way, because
the loss is in the SNAPSHOT. ⇒ **Nothing about the checksum moots this, and that
is a change from what this entry said before.** Only taking the save out of the
rollback set entirely would, which is neither option here nor Q129's question.

✅⛤ **AND OPTION 2 IS TAKEN AND LANDED, SO WHAT IS LEFT HERE IS A PRODUCT
QUESTION AND NOT A DEFECT.** `count_the_dialogue_visit_when_a_conversation_opens`
(`crates/ambition_platformer2d_actor_monolith/src/session/durable_horizon.rs`)
counts the visit in the sim schedule from `ActiveConversation`'s
`opened_at == SimTick`, and `ambition_dialog::bridge` no longer takes
`ResMut<AmbitionGameSave>` at all. Held by
`a_conversation_opening_counts_exactly_one_visit_across_a_rewound_window`: two
openings reach exactly 2 across 240 rewound frames, poisoned to 0 (counter
unregistered) and 6 (edge relaxed to a level rule).

⚠ **THE QUESTION THAT REMAINS IS THE PRODUCT ONE:** is a dialogue visit a fact two
peers must agree on, or per-player progress that should not be in a shared save at
all? Answering it would change what the save's checksum covers; it no longer
changes whether a visit survives a rewind.

## Q133 — should a throw obey rage when obeying it changes who wins?

[THROW-MODIFIERS](queue.md#throw-modifiers--route-throws-through-rage-and-staleness-policy)
asks for throws to obey the rage modifier every strike already obeys, and states
the change *"is a mechanical consistency defect, not a request to retune all
throws"*. Implemented and measured, that second clause does not hold: it is a
retune, and a large one.

Sweeping `AMBITION_DUEL_RUNG` over all five published rungs, HEAD against the
change, reading `two_cpus_in_the_shipped_composition_damage_each_other`'s own
exchange figure:

| rung | HEAD | rage on throws |
|------|------|----------------|
| 1 | 1.99 ✅ | 2.07 ✅ |
| 3 | 1.99 ✅ | 2.29 ✅ |
| 5 | 0.21 ❌ | 0.21 ❌ (bit-identical; no throw lands at this rung) |
| 6 | 2.32 ✅ | 0.64 ❌ |
| 9 | 1.36 ✅ | 0.46 ❌ |

Median `1.99 → 0.64`. The duels that fail run to the 3618-tick cap `decided
None`, where HEAD's decide around 2300–2900: harder throws push the fighters
apart rather than finishing them, and a duel neither seat can close exchanges
less damage than one somebody wins.

⚠ The mechanism is small and the consequence is not, which is what makes this a
product question rather than a bug. Shipped rage is `rage_per_damage: 0.004`
capped at `1.4` (✔ both re-read 2026-09-18 at
`game/ambition_demo_smash/src/lib.rs:1699-1700`; the field is `rage_max_scale`), and the duel's own damage totals put the largest multiplier
actually reached at about `1.17`. A `×1.05` constant applied to the throw with no
rage at all reproduces the rung-9 failure to the digit.

**The choice.**

**(a) Throws obey rage, and the duel guard is re-baselined.** This is the Smash
semantic — rage applies to throws in the games this composition is modelled on —
and it makes the two launch roads agree. The cost is that fight outcomes at rungs
6 and 9 move materially, and
[DUEL-GUARD-RUNG](queue.md#duel-guard-rung--the-cpu-duel-guard-fails-at-rung-5-on-main-today)
has to be settled first or the re-baseline is measured against a scalar that
already means different things at different rungs.

**(b) Throws obey rage behind a declared influence knob, defaulting to inert.**
The precedent exists in the same struct: `stale_knockback_influence` and
`victim_percent_knockback_scale` are both "the mechanism exists, the shipped
tuning decides how much of it lands". A `throw_rage_influence` at `0.0` makes the
consistency defect expressible and leaves every shipped outcome untouched. The
cost is one more knob nobody has asked for, and the defect stays real but
unfixed in the shipped composition.

**(c) Throws are DECLARED not to obey rage, and the row is closed as
intended-behaviour.** Cheapest, and defensible — a throw is not a strike and
Smash's own throw/rage interaction is a design choice, not a law. The cost is
that it must then be written down where the next reader of
`apply_capture_throws` will find it, or this row gets refiled in six months.

⚠ **WHY THIS IS ESCALATED AT ALL, given *"do not let TUNING block
architecture"* (Jon, 2026-07-06).** That rule says pick a reasonable value and
ship it blind, and escalate only when the KNOB ITSELF is missing. This is not a
knob value: nothing here is unset, and option (a) is not "choose a number" — it
turns a shipped guard red, and it turns it red at the two rungs where
throws actually land. Shipping blind would mean landing main red on a shipped
lane. ⇒ If the answer is (a), the re-baseline is the decision, and it should be
taken against the five-rung sweep rather than against rung 9 alone; if it is (b),
the knob is the missing-knob case the rule names explicitly.

⚠ A related row, [DUEL-GUARD-RUNG](queue.md#duel-guard-rung--the-cpu-duel-guard-fails-at-rung-5-on-main-today),
records that the same guard already fails at rung 5 at HEAD. That is NOT a reason
to discount the table above: rung 5's failure is a brain-selection defect
(its CPUs land 15% of their damage on an opponent and the rest on summons), the
metric itself is sound, and rungs 6 and 9 above are clean pass-to-fail
transitions caused by this change alone.

⛔ **What is NOT in scope here:** the staleness half of the same row. That one is
not a balance question at all — a throw never records its own use, so the
staleness read is structurally inert whatever is decided here. It needs a
mechanics answer (does a throw stale the throw, or the grab?) and is recorded in
the row.

## Q137 — does the `GravityFlipSwitch` plate ship, or does the encounter switch own gravity alone?

Two implementations of one player-facing mechanic — *step on a thing, gravity
flips* — are built, and only one of them is reachable.

**The one that ships.** `Switch` entities lower to
`SwitchAction::{FlipGravity, SetGravity<Face>}` (`ambition_encounter/src/switches.rs`)
and are applied by `ambition_encounter_features/src/systems.rs`. Authored content
uses it: censused 2026-09-17 over `game/ambition_content/assets/worlds/*.ldtk`,
fourteen `Switch` entities, of which `central_hub_main` authors one `FlipGravity`
and `symmetry_room` the four `SetGravity{Down,Left,Up,Right}`. ✔ RE-DERIVED
2026-09-18 by parsing the four `.ldtk` files' `entityInstances`: unchanged, and
the other nine are `ResetEncounter` ×8 across four levels plus one `ToggleFlag`
in `switch_lab`.

⚠ **`central_hub_main` IS THE LDtk LEVEL IDENTIFIER, NOT THE RUNTIME ROOM ID**,
which is `central_hub_complex`. Q143 records a day spent on that exact pair —
a binding written against the level id could never match a runtime room — so
this line is spelled the way the asset spells it on purpose, and a future
reader reconciling the two names should change the binding rather than this
census.

**The one that does not.** `GravityFlipSwitch`
(`ambition_platformer2d_actor_monolith/src/gravity/lifecycle.rs`) is a tall
overlap volume with its own edge-latched writer. Its writer is registered in
exactly ONE place in the workspace and that place is inside a `#[cfg(test)]`
module, which the gravity plugin states in its own words: *"`gravity_flip_switch_system`
is intentionally NOT registered. Nothing spawns a `GravityFlipSwitch` in-game
(the hub flip is an LDtk-authored Switch handled by the encounter system); the
component + system exist only for the unit test + any future overlap-style
plate."* That comment is accurate — it is the reachability above that makes it a
decision rather than a bug.

✔ **RE-CHECKED 2026-09-17 WITH THE LENS THAT HAD JUST OVERTURNED TWO NEIGHBOURS,
AND IT HELD.** `portal.emission` and `boss.death_animation` were each called
unreachable on a sentence about the EVENT the field is named for, and each was
wrong; the question that separates them is *where is the component CONSTRUCTED*,
which is independent of whether its system is registered. `GravityFlipSwitch` has
exactly one construction site in the workspace — `gravity/lifecycle.rs:133` — and
that file's `#[cfg(test)]` opens at line 100. ⇒ Even a registered writer would
have nothing to write to. The LDtk entity contract declares `GravityZone` and
`Switch`, and neither converts to this component, so no author can supply the
missing spawn either.

**What the unreachable half still costs, measured 2026-09-17.** It is not one
dead component; it is a vertical slice that every other layer pays for as though
it shipped:

| layer | what it carries |
|---|---|
| rollback registry | TWO registrations — `require_rollback` as `entity:gravity_flip_switch` and `rollback_component_clone` as `gravity.flip_switch` (`actor_monolith/src/rollback_registration.rs`) |
| schema fingerprint | both rows are inside `compute_schema_fingerprint`, so they are part of the peer-stable schema identity `Q122` is about |
| sim view | `GravitySwitchesView` plus `rebuild_gravity_switches_view`, an unfiltered per-tick query that can only ever produce an empty vector (`ambition_sim_view/src/facts.rs`) |
| render | `GravitySwitchVisual` and `sync_gravity_switch_visual`, which despawn-and-rebuild from that empty view every frame (`ambition_render/src/rendering/gravity_visuals.rs`) |

⭐ **AND A FIFTH LAYER PAYS, FOUND 2026-09-18 FROM THE OTHER DIRECTION.**
`BaseGravity` is one of the 121 multi-writer resources, and the sixth of its six
writer files is this system. Adjudicated in
`scripts/check_multi_writer_resources_are_adjudicated.py` as CORRECT-for-five and
ROUTED here for the sixth, which adds one fact this question did not state: the
two implementations are not merely two affordances, **they compute the SAME
EXPRESSION on the same resource** — `base.dir = -base.dir` here, and the
identical negation in `drive_wave_encounters`'s `SwitchAction::FlipGravity` arm.
⇒ So (b) is not only removing an unreachable plate; it is collapsing two owners
of one fact onto one owner, which is the property that campaign exists to buy.
That is an argument for (b), not a ruling: (a) still answers it by giving the
plate a spawn, and two affordances writing one ambient through one expression is
a coherent design.
⚠ It is also a reminder about the instrument: a writer census reads PARAMETER
LISTS, so a production `fn` whose only registration is behind `#[cfg(test)]`
counts as a live writer. This row is why the guard's own entry says
"multi-writer" here is a fact about reachable DECLARATIONS, not reachable writes.

⛔ **AND IT ALREADY CORRUPTED A RANKING.** S7 in
[`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md)
ranked `gravity.flip_switch` among the twelve sharpest unchecksummed rows —
*outside the peer checksum, read every tick, float-bearing, AND mutably written
in production* — on the strength of a mutable borrow that no production
composition installs. The honest count of reachable sharp rows is eleven, and the
witness arm that walks them can never reach this one however many rooms it
visits.

**The decision, and it is a product one.** Either:

* **(a) the plate ships** — something authors a `GravityFlipSwitch`, the system is
  registered in the simulation schedule, and the two mechanics coexist with a
  stated split (a plate you stand on vs. a switch you activate); or
* **(b) the encounter switch owns gravity alone** — the component, its system,
  its unit test, its view fact and its visual are deleted, and the two rollback
  registrations go with them.

⛔ **NOT A DEFAULT EITHER WAY.** (b) removes a documented future hook, which is a
product call and not a cleanup; (a) is a design statement about whether the game
wants two gravity-flip affordances. What is not tenable is the present state,
where four layers describe a mechanic the player cannot meet and a determinism
ranking counted it as production.

## Q138 — should `Platformer2dSimHarness::step` refuse to step an invalidated session?

A sync-test session that invalidates keeps accepting `step()` and stops
advancing `SimTick`. The step returns an observation every time, nothing panics,
nothing prints, and every assertion after the invalidation runs over a frozen
world — where it agrees with itself, forever. Measured 2026-09-16
(`probe_how_far_each_harness_ticks_over_the_same_window`,
`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`), `SimTick`
over 240 `step()` calls: a healthy sync-test harness reads 1, 41, 81 … 241, and
one system writing a rollback-registered resource outside its sanctioned road
reads 1, 6, 6, 6, 6, 6, 6 with `session_health` already saying
`Err(checksum mismatch at frames [2, 3, 4, …])`.

⇒ **THE HARNESS ALREADY KNOWS.** `rollback_health()`
(`crates/ambition_sim_harness/src/runtime.rs`) returns exactly that error, and
`step` does not consult it. The contract makes silence the default and leaves
every caller to notice on its own.

**The choice is what `step` does about it**, and the options are not equivalent:

* **(a) refuse** — `step` panics, or returns an error, once the session is
  invalid. Silence stops being the default and the twenty-seventh arm's author
  gets the warning the census cannot give them.
* **(b) leave it to callers** — the current contract, with the guard
  (`scripts/a_rollback_arm_must_refuse_a_frozen_world.py`) routing each new arm
  to either read a health API or arrive with a sentence naming what a frozen
  world breaks in it.

⚠ **WHAT THE MEASUREMENT DOES AND DOES NOT SETTLE.** It settles the blast radius
for the TEST population: no arm at HEAD is silently stepping a dead session, so
(a) would red nothing in CI. Every arm either reads a health API or carries a
named mechanism a stopped clock cannot satisfy — a population floor, a recorded
stream length against the tick count, a room change after an authored hold. ⇒
**The cardinalities are not restated here.** They moved four times in three days
and this page held a copy that rotted each time; the census lives in
`docs/planning/queue.md`'s ROLLBACK-DEAD-SESSION with its reference point, and
the one road that cannot be stale is
`python3 scripts/a_rollback_arm_must_refuse_a_frozen_world.py`, which prints the
line. It runs in the maintenance lane as of 2026-09-18.

It does NOT settle the policy, because a harness that panics on a dead session
takes the choice away from a future arm that legitimately wants to step one — an
arm testing the invalidation itself, for instance — and that is the maintainer's
call rather than a census's.

⛔⛤ **AND "THE EXPOSED POPULATION IS ZERO" WAS TRUE ONLY BECAUSE THE COUNT PUT
ONE MEMBER IN THE WRONG COLUMN — WHICH IS AN ARGUMENT *FOR* (a).** This row used
to read *"15 read a health API and the rest already refuse a frozen world"*,
folding the guard's two `NOT_AN_ARM` exemptions into the safe limb. One of them
is not safe. `game/ambition_app/examples/hall_bench.rs:45-68` builds the shipped
sync-test session and steps it 3,300 times with no health read and no liveness
floor, so an invalidated session there prints per-tick timings for a world that
stopped advancing — a benchmark reporting the cost of doing nothing, in the
shape of a number somebody would then put in a document. It is an example rather
than a test, so it still reds nothing in CI; it is also the only caller in the
tree that consumes a frozen world SILENTLY, and it is the one the census
exempted. ⇒ Under (a) it would abort instead, which is what a benchmark wants.

⚠ **THE EXEMPTION'S STATED REASON WAS ALSO FALSE, AND CHECKABLE.** It read *"it
asserts nothing, so there is no verdict for a frozen world to falsify"*, while
`hall_bench.rs:55` asserts the active room after warmup. The conclusion survived
— `with_required_start_room` refuses to boot unless the room resolved
(`crates/ambition_sim_harness/src/options.rs:77-81`), so `active_room` already
equals it at tick 0 and a frozen world satisfies that `assert_eq!` — but the
reason did not, and the reason is what the next reader re-checks. Corrected in
the guard 2026-09-18 with the mechanism that actually carries it.

⭐ **AND THE TREE ALREADY ANSWERS A NEIGHBOURING QUESTION IN THE LOUDEST
DIRECTION**, which is evidence whichever way this goes: for a two-root world
the headless path ABORTS rather than hand back an uninterpretable reading, on a
plain ungated `assert!` at
`crates/ambition_platformer2d_shared_tangle/src/lifecycle/session.rs:424`,
reached by every harness this row is about. See `Q130`, which asked this same
question with an older census and is now a pointer here; that precedent is the
one thing it carried that this row did not.

⛔ **NOT A CLEANUP, AND THE ADJUDICATED ARMS MUST NOT BE EDITED EITHER WAY.**
(There were six when this was written and there are twelve now, which is why the
sentence no longer names a count.) Adding `rollback_health()` to an arm whose
assertions already cannot pass over a frozen world trades a strong guarantee for
a visible one. Counting calls to a safety API measures vigilance; counting
assertions a broken world fails measures safety.

## Q139 — what declares that a presentation system writes `Transform`?

`scripts/check_rollback_mutators_run_in_sim.py`'s component half excludes
`Transform` BY NAME, with the count beside it: on the 2026-09-16 reading, 52 of
the 64 offenders it would otherwise surface are `Transform` writes from camera,
sprite and inspection systems. So a green there says nothing about `Transform` —
the single most rollback-sensitive component in the workspace.

⛤ **RE-MEASURED 2026-09-18 AND THE DENOMINATOR MOVED WHILE THE NUMERATOR DID
NOT: 52 of 60.** Dropping `PRESENTATION_SHARED` in a throwaway process takes the
offender list from 8 to 60, and all 52 of the delta write `Transform` and
NOTHING else — so the exclusion is still doing exactly one job and the guard's
green still says nothing about this component. The four that left the other
bucket left by being repaired or banked, not by changing what `Transform` hides.

⇒ **THE OBVIOUS REPAIR IS REFUTED AND THE ROW RECORDS THE REFUTATION.** Classify
by a property the system already states — `Camera`/`Sprite`/`Text`/`Mesh`/
`Light`/`Node`, or a projection in the signature — and on 2026-09-16 the 52 split
**23 that declare such a marker and 29 that do not**, where the 29 are
presentation only by NAME (`camera_follow`, `sync_parallax_layers`,
`sync_hit_flash_overlays`). A system's name is not a reading of its write set;
that classifier was wrong in both directions twice on 2026-09-16 alone.

⭐⭐ **AND THE SPLIT HAS MOVED A LONG WAY SINCE, WHICH CHANGES WHAT OPTION (a)
COSTS.** Re-measured 2026-09-18 over the same 52, reading each system's
parameter list with comments blanked:

| marker set | declares | bare |
|---|--:|--:|
| exactly this row's list (`Camera`, `Sprite`, `Text`, `Mesh`, `Light`, `Node` and the `2d`/`3d` spellings) | **36** | **16** |
| that, plus prefix matching and `Visibility` (e.g. `SpotLight`, `TextFont`, `InheritedVisibility`) | **47** | **5** |

⇒ **Option (a)'s work is 16 systems, not 29** — and 5 if the marker vocabulary
is allowed to include visibility and the light variants, which is a vocabulary
choice this ruling would be making anyway. The carve work of the last two days
is what moved it: systems gained the presentation components they were always
projecting to. ⚠ The page's three named examples all survive the re-measurement
as still-bare, so the SHAPE of the argument is unchanged and only its size is:
`camera_follow`, `frame_the_inspection`, `sync_ldtk_world_transform`,
`apply_capture_snapshot` and `restore_sprites_without_effects` are among the 16.
⚠ And the two readings differ by 11 systems, so quoting one number without its
marker set is the error this row already made once.

**So the repair is a DECLARATION, not a cleverer scanner**, and the decision is
its shape:

* **(a) a set** — presentation `Transform` writers join a named system set, and
  the guard reads membership instead of parsing signatures;
* **(b) a marker on the entities** — the things presentation moves carry a
  component saying so, and the guard asks about the entity rather than the
  system;
* **(c) a wrapper type** — presentation writes a distinct component the render
  layer lowers to `Transform`, which makes the mutation unspellable rather than
  merely declared;
* **(d) leave the exclusion**, and accept `Transform` as a permanent blind spot
  — today's state, written down honestly. ⭐ RESTORED 2026-09-18 from `Q131`,
  which asked this same question first and listed it: *"a real option and should
  not be dismissed — a green from that guard already says nothing about
  `Transform`, and it says so where it defines its population."* Dropping a real
  option from a maintainer's menu is a decision made by omission.

⚠ **THE SIZE IS THE REASON THIS IS A QUESTION**, and it is smaller than it was.
It is 52 systems across the render, camera and inspection layers, not a script
change, and the three shapes put the cost in different places — (a) is cheapest
and weakest, (c) is the only one a future system cannot forget. But (a) now
touches 16 of the 52 rather than 29, because the other 36 already declare what
they are; under (a) the remaining work is the 16, and under (c) it is still all
52 because a wrapper type has to be written to, not merely declared. Nobody
should start until the shape is chosen.

## Q140 — may the item menu show a stale bag for one frame?

[MENU-RESET-MIDSESSION](queue.md#menu-reset-midsession--the-menu-writes-rollback-state-from-update)
is blocked on one UI question, and the engineering half of it is already decided.

The menu writes `OwnedItems` — a rollback-registered resource — from `Update`,
outside the simulation schedule. A conversation that gives you an item is
rollback-correct today and the menu giving you one is not, for the same resource
in the same crate.

⛔⛤ **BUT THIS ROW NAMED THE WRONG MECHANISM FOR WHY, AND THE CORRECTION
CHANGES WHAT THE FIX IS — MEASURED 2026-09-18.** It said the sanctioned road is
that *"`ItemGrantRequested` is `clear_message_on_rollback` and its consumer
`apply_item_grants` mutates `OwnedItems` from the SIM schedule"*. Both halves
are true and neither is what makes the conversation correct:

* **`clear_message_on_rollback` is not load-bearing.** Poisoning the equivalent
  registration for `PlayerHealRequested` changed that arm's outcome not at all
  — `Messages::clear` leaves `message_count` monotonic, so a reader that already
  consumed the message reads zero on every resimulated frame whether or not the
  channel was emptied.
* **A sim-side CONSUMER does not rescue a host-side PRODUCER.** This file's own
  `a_rollback_cleared_message_written_from_outside_the_simulation_is_also_lost`
  measures `ItemGrantRequested` raised from outside the simulation being lost,
  against an in-sim control that grants. Switching the menu from *"write
  `OwnedItems` from `Update`"* to *"raise `ItemGrantRequested` from `Update`"*
  would trade a rollback-mutator defect for a `Q136` lost-intent defect.

⭐ **WHAT ACTUALLY MAKES THE CONVERSATION CORRECT IS THE LEDGER.**
`cmd_give_item` (`game/ambition_content/src/yarn_vocabulary.rs:245`) does not
write the message at all — it takes a `NarrativeInputWriter`, which records the
payload stamped with `SimTick + 1` into `NarrativeInputLedger<ItemGrantRequested>`,
a resource deliberately kept OUT of rollback state; `release_narrative_inputs`
then raises the real message at the head of the sim schedule, so every
resimulation of that tick re-raises it. See the fourth escape under `Q136` for
the whole shape.

⇒ **SO THE ENGINEERING HALF IS NOT ALREADY DECIDED, AND THE SHAPE IT SHOULD
TAKE IS BUILT.** The menu wants `NarrativeInputPlugin`'s shape — a
tick-stamped, rollback-exempt ledger released inside the sim — not a bare
`MessageWriter`. ⚠ It cannot take the PLUGIN: every entry is keyed by
`ConversationInstanceId` and a menu press has none, so the key has to
generalise from the conversation to the session first. See `Q136`'s fourth
escape for the whole reading. That does not settle the UI question
below, which is the real subject of this row and is unchanged by the
correction.

⇒ **THE BLOCKER IS NOT THE PATTERN, IT IS ONE FRAME.** The menu READS
`OwnedItems` in the same frame to render the row it just changed. Deferring the
write to the sim means the grant lands on the next tick, so the list shows the
old bag for one frame unless the UI renders optimistically. That is a visible
behaviour change in shipped UI.

* **(a) accept the frame** — the row updates on the next tick. Simplest, and the
  menu's read stays the single source of truth.
* **(b) render optimistically** — the menu draws the intended bag immediately and
  reconciles when the sim applies the grant. No visible latency, at the cost of a
  second reading of the bag that can disagree with the authoritative one.

⛔ **NOT A MECHANICAL SUBSTITUTION, WHICH IS WHY THE ROW IS FILED RATHER THAN
DONE.** Everything else in that row is settled: a consumable USE is not a shop
sell so the menu still needs its own message, and the hoped-for escape hatch is
shut — the systems carry `.run_if(simulation_authorized)`, which is TRUE exactly
when a live session scope exists, so the run condition guarantees the dangerous
window rather than excluding it.

## Q141 — may a runtime-spawned ground item ever be durable?

⭐ **THE MECHANISM EXISTS AND THE ENTRY RULE IS NOW ENFORCED; this is the one
question left over it.** `AuthoredOccurrences` has exactly ONE entry road —
custody, via `project_custody_onto_authored_occurrences` reading `InCustodyOf` —
so an object that enters the world already lying on the ground and is never
picked up cannot be remembered, and `republish_placements` refuses any id whose
current row is not `InCustody` or `Placed`. ✔ Re-checked 2026-09-18: the
refusal is still the live predicate
(`crates/ambition_platformer2d_shared_tangle/src/lifecycle/continuity.rs:321`)
and its three arms are all still there — a never-carried id refused by name, a
`Consumed` row not resurrected, and both legal roads still passing so the guard
cannot pass by refusing everything. The measurement and the guard are at
I4 in [item custody](engine/item-custody-and-accounting.md).

⇒ Choose whether a runtime-spawned ground item may be durable at all. If yes it
needs either a road into custody or a SECOND entry point stated as deliberately
as the first — and an object that gains one **must stop carrying
`SpawnedThisAttempt`**, because "the attempt reclaims it" and "the durable world
remembers it" are contradictory answers about the same object. The clearest
member of the population is the death drop, whose two answers agree today: it is
room-scoped, attempt-reset, and correctly not durable.

⛔ **I4 CLAIMED THIS WAS ALREADY FILED, AS "question 51", AND IT WAS NOT.** Q51
is the boss-reward durability boundary — a different question about a different
object. A route to a wrong number reads exactly like a route to a right one, and
the row had carried it since 2026-09-04. Filed here 2026-09-17.

## Q142 — ✔ THREE OF THE FOUR ARE REGISTERED AND WITNESSED; TWO subjects are still owed

The repository already states the rule, in
[`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md):
*"A COMPONENT WHOSE PRESENCE IS READ BY A QUERY FILTER IS AUTHORITATIVE EVEN WHEN
ITS VALUE IS DERIVED."* It was written about the DEMOTION direction — registered
rows whose doc sounds like they could be dropped. This is the inverse reading,
and it had no instrument until 2026-09-17:
`scripts/check_presence_filtered_state_is_rollback_registered.py`.

**The population and how it was measured.** A component is in scope when it is
defined in a crate that registers at least one rollback row AND a literal
`With<X>` / `Without<X>` / `Has<X>` outside test code reads its presence.
**Measured 2026-09-17 when this row was filed: 95 such components, 73
registered, 18 waived by name with the measurement beside each, 4 left. After
the three fixes below, the same run read 76 registered and 1 left** — that
movement was the code, not the instrument or a waiver.

⛔⛤ **AND THEN THE POPULATION ITSELF MOVED, LATER THE SAME DAY, BECAUSE THE
INSTRUMENT WAS BLIND TO HALF A CRATE.** `component_definitions` cut each file at
its FIRST `#[cfg(test)]`, and in this tree a module declares its tests near the
TOP: `shared_tangle/src/construction/mod.rs` writes `#[cfg(test)] mod tests;` and
then defines most of A10's vocabulary underneath it. ⇒ **288 definitions became
305 and the intersection 95 became 104** — the run now reads **104 components,
83 registered, 20 waived, 1 owed**. Two of the nine that appeared were neither
registered nor waived, `InactiveCandidate` and `PresentationOnly`, and both are
now waived with their measurements: three `&mut World` filter sites inside the
component's own module for the first, and **zero production insert sites at all**
for the second. ⚠ The blindness failed in the GREEN direction and no floor was
low enough to notice; the arm that holds it now pins a SUBJECT rather than a
count.

The registered set is read from `rollback_schema_baseline.txt`, which
`rollback_schema_baseline.rs` holds byte-identical against the live registry.

⛔⛤ **THE INSTRUMENT'S FIRST VERSION MISSED THE COMPONENT ITS OWN DOCSTRING
QUOTES.** Deleting `Dormant`'s row from the recorded schema left the check green:
all three of `Dormant`'s production filter sites spell it
`Without<crate::features::ecs::dormancy::Dormant>`, and the regex required a bare
name. Widening it for the path prefix — and for `Has<T>`, which is the same
presence read — took the intersection from 73 to 95 and produced six of the rows
below. ⇒ The poison that found it is planted as
`test_a_path_qualified_filter_is_seen`.

⛔⛤ **AND THREE OF THE FOUR WERE NEVER A DECISION. FIXED 2026-09-17, schema
v197 → v198.** This row was filed as a policy question — *"registering all four
is a wire-format change"* — and a review read the evidence back and pointed out
that the framing was wrong for three of them: an unregistered component that a
sim system mutates every tick is a rollback defect, not an option. Re-checked
one at a time before changing anything, and each holds:

* **`EncounterScript`** — `cursor` and `elapsed` are advanced by
  `EncounterScript::advance` (`timeline.rs`), called from `tick_encounter_scripts`,
  which `ambition_boss_encounter` registers into the sim schedule at
  `ProgressionSet::BossHazards`. Now `component-clone-custom-checksum`, projecting
  `cursor` and the beat-elapsed bits; `beats` is authored content and stays out of
  the projection.
* **`ReleaseOnDeath`** — `release_payloads_on_death` writes `PayloadReleased` and
  removes the marker in the same loop, and `PayloadReleased` is registered
  `message-clear` so a resimulation may re-emit it. The pair was asymmetric in the
  direction that loses the release entirely. Now `component-clone`.
* **`RecharacterizeBody`** — Mary-O inserts it in `FeatureInteraction`;
  `apply_worn_character_gameplay` consumes and removes it in
  `PlayerInputSet::Persona`, an EARLIER phase of the next frame. It is a request
  that deliberately waits a frame, so a rewind across that frame decides whether
  the template is applied nought, one or two times. Now `component-clone`.

⇒ `the_rollback_schema_matches_its_recorded_baseline` and
`the_shipped_app_registers_the_same_schema_as_the_sandbox` both pass at v198, so
all three rows are live in the shipped app, not only in the sandbox.

⭐⭐ **AND `EncounterScript` NOW HAS THE REWIND ARM, WITH THE NUMBER.**
`the_encounter_script_clock_reaches_the_same_value_with_and_without_a_rewind`
(`cut_rope_arena`) boots the one room whose PRODUCTION code attaches a script,
waits 240 frames without driving any input, and compares the beat clock against
the sim tick in both worlds. Registered: the clock and the tick agree exactly.
⛔ With the registration removed: **the clock moved 945 frames more while the
world ran 1 tick more — 19.77 s against 4.02 s** — which is the resimulation
multiplier of a `check_distance` of 4, not a rounding difference.

⛔⛤ **TWO THINGS ABOUT THAT ARM ARE WORTH MORE THAN THE RESULT.** Its first
version CUT THE ROPE, and the slash is a `HitEvent` written by the test from
outside the rewinding schedule: the rewind took the write back, **1 gate fired
without a rollback window and 0 under one**, and the script sat on beat 0. The
arm read `Some(0)` vs `Some(2)` and looked exactly like a lost cursor while it
had never reached its subject. And the second version compared the clocks
directly, reading `4.0333` against `4.0167` — one frame, which looks like the
same defect until you ask how many ticks each world ran. `SimTick` said **241
against 240**: the sync-test harness steps once more. ⇒ The property is a RATE,
and the control is the tick count.

✔ **`ReleaseOnDeath` NOW HAS ITS ARM, 2026-09-17, AND WHAT IT COST WAS THE
OBSERVABLE RATHER THAN THE FIXTURE.**
`a_resimulated_kill_frame_still_carries_the_release_marker`
(`game/ambition_app/tests/cut_rope_arena.rs`) stages the behemoth's death from a
system inside the sim schedule at tick 90 — replayed by every resimulation, so
the kill lands on the same tick in every pass — and records, per PASS of that
tick and ordered before `release_payloads_on_death`, how many hosts still carry
the marker. Registered it reads `[1, 1, 1, 1]`; poisoned by deleting
`encounter.release_on_death` it reads **`[1, 1, 1, 1, 0]`** — a resimulated pass
that cannot emit what the first pass emitted.

⛔⛤ **AND THE FIRST DESIGN OF THAT ARM WAS VACUOUS, WHICH IS THE PART WORTH
KEEPING.** It asserted the VICTORY NPC's presence — the visible consequence, and
the obvious observable — and the poison PASSED. The NPC is not rollback state:
it spawns on the first pass of the kill frame, nothing despawns it on a rewind,
and `spawn_cut_rope_victory_npc` then returns early on `existing`. So its
presence answers *"did the release ever fire"*, which is true either way. ⇒ **A
visible consequence that is not itself rollback state cannot witness a rollback
defect**, and the property the registration buys is the marker being back at the
head of every resimulated pass.

⛔ **WHY IT STAGES A DEATH INSTEAD OF CUTTING THE ROPE — priced, so nobody pays
for it twice.** A test-written `HitEvent` does not survive a rewind (1 `rope_cut`
gate without a rollback window, 0 under one), so the rope must be cut by a real
PRESS, which the harness does feed into the GGRS input stream. The obstacle is
the ROUTE: the authored rope is at `(908, 96)` and the player spawns at
`(110, 712)` — 798 px right and **616 px up**. A walk-and-swing script closes to
660 px; adding a jump cadence and a held up-axis climbs to `y = 293` and closes
to **187 px**. Tuning a blind script onto a 24 px volume that high is a search,
not a fixture, and the whole-fight arm is priced at an authored platforming
route. That is not what this registration owes.

✔ **AND `RecharacterizeBody` HAS ITS ARM TOO, the same day and by the same
shape.** `a_staged_recharacterize_request_survives_every_pass_of_the_frame_that_reads_it`
(`game/ambition_app/tests/a_recharacterize_request_crosses_a_rewind.rs`) stages
the request from a sim-schedule system ordered AFTER the consumer — so it waits a
frame exactly as the Mary-O producer's does — and censuses, per pass of the frame
that reads it, whether the request is still there. Registered `[1, 1, 1, 1]`;
poisoned by deleting `actor.recharacterize_request`, `[1, 1, 1, 1, 0]`.
⚠ It stages rather than driving a Mary-O powerup pickup, which costs coverage of
the PICKUP road and buys the registration's own property; the arm says so at its
own definition.

⛔⛤ **AND BUILDING IT COST AN OFF-BY-ONE WORTH KEEPING.** `advance_sim_tick` runs
BETWEEN a system early in the frame and one late in it, so the head recorder
reads the tick number of the frame BEFORE it while the staging system reads the
new one. The control failed with `[0]` and looked exactly like an insert that
never landed. What separated them was reading the same world from OUTSIDE the
schedule: the marker is there at tick 40 and gone at 41.

**The one still open, and the three that closed:**

| component | the filter that reads it | why it is not hygiene |
|---|---|---|
| `ReleaseOnDeath` (`ambition_boss_encounter`) | `release_payloads_on_death`, `With<ReleaseOnDeath>`, registered into the SIM schedule at `ProgressionSet::BossHazards` | the system REMOVES the marker after emitting, so its absence is what stops a second emission — while the message it emits, `PayloadReleased`, IS registered `message-clear` so a resimulation can re-emit. The pair is asymmetric: the clearing is there to allow a re-emission the missing registration prevents |
| `RecharacterizeBody` (`ambition_characters`) | `Has<>` in `avatar/starting_character.rs` | its own doc: *"One-shot request to reapply a body's character template … The request is consumed after application."* The consumption IS the state |
| `EncounterScript` (`ambition_encounter`) | `Without<EncounterScript>` in `setup_cut_rope_encounter` | ⛔ **the sharpest of the four.** It is an idempotence gate on a component that also holds `cursor: usize` and `elapsed: f32` — the beat a scripted fight has reached and how long it has been in it — advanced every tick by `tick_encounter_scripts`, which is in the sim schedule beside `release_payloads_on_death`. A rewind restores neither |
| `PostBossNpc` (`ambition_combat`) | `AttemptResidue` in `world/rooms/reconstitution.rs` | presence decides whether the celebrant a defeated boss left behind is swept when a replay is admitted |

⛔⛤ **AND `S7`'s FLOAT-ROW CENSUS CANNOT SEE `EncounterScript.elapsed` EITHER,
FOR THE SAME STRUCTURAL REASON.** S7 ranks the rows *outside the session
checksum* — and it derives that population from the REGISTRY. ⛔⛤ THIS SAID
*"99 of them, 25 float-bearing, 12 mutably written"* until 2026-09-18, which
compressed three DIFFERENT populations into one chain and then went stale in
all three. The 99 (today **101**) is the no-value-projection subset of 177
unhashed rows; 25 is not its float-bearing subset but the intersection of
no-projection × unfiltered per-tick read × float-bearing; 12 is the mutably
borrowed subset of THAT. ⇒ S7 owns those numbers and this row owns the
structural point, which does not depend on any of them. A float advanced every tick by a sim system on
a component nobody registered is not a row in the 99; it is not in the population
at all. ⇒ The two censuses are complements, and neither is the whole surface: one
asks which registered rows are uncompared, this one asks which authoritative
components are unregistered.

✔ **THIS PARAGRAPH SAID "No arm has been run that drives a rewind across any of
these four latches" AND THAT IS NO LONGER TRUE — three of the four now have one,
2026-09-17.** The finding was STRUCTURAL when it was written: presence is
authoritative and the snapshot did not know about it. It is now measured for the
three that closed:

| latch | arm | poison reading |
|---|---|---|
| `EncounterScript` | `the_encounter_script_clock_reaches_the_same_value_with_and_without_a_rewind` | 945 frames of excess script-clock advance |
| `ReleaseOnDeath` | `a_resimulated_kill_frame_still_carries_the_release_marker` | `[1, 1, 1, 1, 0]` — a pass that cannot re-emit |
| `RecharacterizeBody` | `a_staged_recharacterize_request_survives_every_pass_of_the_frame_that_reads_it` | `[1, 1, 1, 1, 0]` — a pass that applies the template zero times |

⛔⛤ **AND THE SHAPE THAT WORKED IS NOT THE ONE THIS PARAGRAPH PRESCRIBED.** It
said to use the same measurement as
`a_move_occurrence_reaches_the_same_number_with_and_without_a_rewind` — two
worlds, one number each. That shape cannot see a presence latch: the visible
CONSEQUENCE is usually not itself rollback state, so it survives the rewind
whatever the registration does. The first `ReleaseOnDeath` arm asserted the
victory NPC's presence and its poison PASSED for exactly that reason. What works
is a PER-PASS census of the latch itself, read by a system ordered before its
consumer, with the state change staged from inside the sim schedule so it is
replayed. ⚠ `spawn_cut_rope_victory_npc`'s second road (`boss_is_cleared` from
the save) does mask the defect on room re-entry — that part of the paragraph was
right, and it is why the arm never re-enters the room.

⚠ **`PostBossNpc` STILL HAS NO ARM**, and its question is about what a LOAD does
rather than what a tick does, so the per-pass shape above does not reach it
either. ⛤ Neither does `SmirkingBehemothVictoryNpc`, which joined this row on
2026-09-18 — see below for how a component in a rewinding system's filter stayed
outside the question for as long as it did.

⛔⛤ **AND IT IS TWO COMPONENTS WIDE AGAIN SINCE 2026-09-18, BECAUSE THE GUARD
COULD NOT SEE THE CRATE THE SECOND ONE LIVES IN.**
`check_presence_filtered_state_is_rollback_registered.py` derived its component
population as `crates/<name>/src` for every registering crate — while its filter
scan read `crates` AND `game` the whole time. `game/ambition_content` registers
rollback state (`EchoFanState` and the rest of `bosses/specials/rollback.rs`,
`PortalHostScanned` through `portal/plugin.rs`), so its entire component
population was outside the question. Widening it added six subjects; five are
presentation and are waived with the schedule each filter site runs in, and the
sixth is `SmirkingBehemothVictoryNpc`.

⚠ **THE FLOORS COULD NOT HAVE CAUGHT THIS, AND THAT IS THE GENERAL LESSON.** The
guard has four anti-vacuity floors and every one of them stayed comfortably
satisfied: they catch a join that returns almost NOTHING, and a stable omitted
CATEGORY leaves the remaining population large. A floor is a defence against a
broken instrument, not against a instrument pointed at part of the tree.

`SmirkingBehemothVictoryNpc` belongs to this question and not to a new one. Its
one filter site is `spawn_cut_rope_victory_npc`'s
`existing: Query<&FeatureId, With<SmirkingBehemothVictoryNpc>>` — a SPAWN-ONCE
guard — and that system is registered in the REWINDING schedule
(`app.add_systems(sim, .. .in_set(ContentEncounterVictorySet))`,
`game/ambition_content/src/bosses/mod.rs:379`). So its presence decides whether a
re-simulated victory frame spawns a SECOND celebrant. ⛔ It is NOT being
registered by analogy, for the reason this row already records above: the first
`ReleaseOnDeath` arm asserted this very NPC's presence and its poison PASSED,
because a visible consequence that is not itself rollback state survives a
rewind whatever the registration says. It wants the per-pass census shape that
closed the other three.

**The decision, and it is TWO components wide.** `PostBossNpc` is the other row.
Its presence decides whether the celebrant a defeated boss left behind
is swept when a replay is admitted — which is a question about what a LOAD does,
not about what a tick does, and none of the three arguments above reaches it.
⇒ It wants a targeted behavioural arm (does an admitted replay sweep the
celebrant it should keep?), not a fourth registration by analogy. ⛔ Registering
it anyway would be the sweep this row warned against: the guard's 18 waivers each
state a measurement, and a fourth row added because its three neighbours moved
would be a waiver with the opposite sign and no measurement behind it.

## Q143 — what does a cutscene `Fade { to_alpha: 0.0 }` fade FROM?

**Asked 2026-09-17.** `CutsceneBeat::Fade` is documented as *"Fade screen to
`alpha` (0.0 = clear, 1.0 = solid black) over `seconds`"* and labelled
UNFINISHED at its definition, on the grounds that nothing consumes
`CutscenePresentation::fade_alpha`. ⛔⛤ **THAT DIAGNOSIS IS INCOMPLETE, AND
BUILDING THE CONSUMER IT ASKS FOR WOULD CLOSE THE ROW WITHOUT CHANGING WHAT A
PLAYER SEES.**

**MEASURED 2026-09-17** over every non-test `CutsceneBeat::Fade` literal in the
workspace — all three of them. ✔ RE-DERIVED 2026-09-18: still exactly three
(`game/ambition_content/src/intro/cutscene.rs:43` and `:109`,
`game/ambition_content/src/dialogue/cutscene_defaults.rs:16`), every row of the
table below unchanged including each fade's POSITION in its script:

| script | `to_alpha` | `seconds` | position in script | room |
|---|--:|--:|---|---|
| `test_intro` | **0.0** | 0.8 | second, after a banner | `central_hub_complex` |
| `intro_wake` | **0.0** | 0.8 | **first** | `intro_wake_room` |
| `drain_market_arrival` | **0.0** | 0.6 | **first** | `drain_alley` |

Every author wrote a fade **UP** — from black to clear. And
`CutsceneRuntime::presentation()` returns `to_alpha` unchanged, ignoring
`elapsed`, so the projection reads **0.0 at every instant of the beat** — the
same number a clear screen reads. ⇒ A consumer that draws `fade_alpha` draws
nothing, for all three, for the whole 2.2 s. Held by
`a_fade_beat_projects_its_target_at_every_instant_rather_than_a_ramp`
(`crates/ambition_cutscene/src/lib.rs`), poison-verified: a linear ramp from 1.0
turns the reading `[0.0, 0.0, 0.0, 0.0]` into `[1.0, 0.75, 0.5, 0.25]`.

**The question is where the ramp starts, and it is about authored content, not
about the engine.** Three answers, and they are not equivalent for the shipped
scripts:

1. **A cutscene opens BLACK; a fade ramps from the last fade's target,
   defaulting to 1.0.** All three authored scripts then mean what their author
   plainly wrote, and no content changes. ⚠ It gives `test_intro`'s opening
   banner 1.4 s over a black screen before the fade up — arguably right, and
   arguably a surprise to whoever wrote the banner first.
2. **The screen starts CLEAR; the beat ramps from the live screen alpha.** Then
   all three shipped fades are no-ops that correctly draw nothing, today's
   behaviour is right, and the UNFINISHED label should come off the beat rather
   than a consumer being built. ⚠ It also makes `Fade` unusable as an opener,
   which is how two of the three use it.
3. **`Fade` grows an explicit `from_alpha`.** Unambiguous, and it is a change to
   the serialised script format plus all three authored scripts. ⚠ The
   vocabulary gets a field to say what a convention would have said for free.

⭐ **AND THE THREE ANSWERS ARE ONE IMPLEMENTATION DIFFERING IN ONE NUMBER,
WHICH IS THE CHEAPEST THING ANYONE HAS LEARNED ABOUT THIS ROW.** Measured
2026-09-18 in `CutsceneRuntime::presentation()`: the `Banner` arm two lines
above the `Fade` arm already reads the clock — `(seconds - self.elapsed)` — and
the `Fade` arm ignores `elapsed` entirely. Every option needs it:

| option | start of the ramp | needs `elapsed` |
|---|---|:-:|
| 1 — opens black | the last fade's target, default 1.0 | yes |
| 2 — opens clear | the live screen alpha | yes |
| 3 — explicit field | `from_alpha` | yes |

⇒ **The ruling is the START VALUE, not the interpolation**, and no option can
be implemented without the same `lerp(start, to_alpha, elapsed / seconds)` in
that arm. ⚠ Which also says why the interpolation must NOT land ahead of the
ruling, tempting as an obviously-missing lerp is: today's constant `to_alpha`
IS option 2 for the three shipped literals, since a ramp from a clear screen to
`0.0` is constant `0.0`. Landing the lerp with any concrete start silently
decides the question against option 2 while looking like a bug fix.

⛔ **WHAT MUST NOT HAPPEN IS A CONSUMER LANDING ALONE.** It satisfies the
UNFINISHED label, reads as the row closing, and leaves the player waiting 2.2 s
across three rooms for a screen that never changes.

⛔⛤ **AND ON 2026-09-18 ONE OF THE THREE BECAME THE SHIPPED FIRST BOOT, WHICH IS
WHY THIS ROW IS NOW WORTH SCHEDULING.** The `room` column above read
`central_hub_main` when this row was written, and that was not a typo: it was the
LDtk level id `479d5a028` corrected to `central_hub_complex`. Until that commit
the binding could never match a runtime room, so `test_intro` had **never
played** and its 0.8 s of nothing was unreachable. It is now the second beat of
the cutscene every new player meets on entering the hub, between the
`// boot sequence` banner and the WARDEN line — 0.8 s in which, measured, the
projection reads 0.0 at every instant and a consumer would draw nothing.

⇒ The ruling is unchanged and so is the argument; what changed is the cost of
deferring it. ⚠ A fourth option exists now that did not before and is worth
naming so it is chosen rather than drifted into: **drop the `Fade` beat from
`test_intro`** and leave the other two, which takes the defect off the
first-boot path at the price of the intro the author wrote. That is a content
decision, not an engine one, and it does NOT close this row — `intro_wake` and
`drain_market_arrival` both still open with a fade, and both use it the way
option 2 would forbid. ⛔ Reverting the room binding is not on the list: the
binding is correct and the dead-row problem it fixed was real.

⇒ This is the same missing thing as VC5, the title launcher's content-alpha ramp
— see
[`engine/shell-vanity-sequence.md`](engine/shell-vanity-sequence.md) — at a
different layer, and a screen-alpha consumer built for one is the obvious owner
for the other. ⚠ **That argument is about the CONSUMER and this ruling is about
the RAMP;** whoever takes VC5 does not inherit this decision with it.

## Q144 — must every supported composition activate a prepared generation, or does direct entry keep the App-registry road?

**Asked 2026-09-17, and it is the LAST open duplicate-authority family in the
consolidation census** (`DUP-GENERATION-MECHANICS`, and the gate
`AUTH-SESSION-MECHANICS` used to record as *"direct-entry composition
decision"* — a hold that named no page and no question until now).

**MEASURED 2026-09-17, because the ledger row said less than source does.** A
generation's frozen values already win over anything the App holds, and a
shell-routed session that has LOST its mechanics is refused rather than fed the
App:

| road | constructor | what it does with no active generation |
|---|---|---|
| reset, room transition (×2) | `GenerationMechanics::for_live_session` | **refuses** when `SessionGatedSimulation` is present |
| provider activation | `GenerationMechanics::of` | has no fallback to offer |
| hot reload | `GenerationMechanics::new` | states `None` on purpose — it is building the generation that replaces the live one |

⛔⛤ **RE-MEASURED 2026-09-18 WITH COMMENTS STRIPPED, AND THE FIRST ROW LOST A
ROAD.** It read *"reset, room transition (×2), room stage"* — four. There is no
room-stage construction: `world/rooms/stage.rs` owns the ERROR VARIANT
`RoomConstructionError::LiveGenerationMechanicsMissing`, whose doc comment names
the constructor, and `construct_room_candidate` is HANDED an
`ActorConstructionContext` its caller already built. The only producer of that
variant is `room_transition/loading.rs:1193`, which is already row one's second
entry. ⇒ **A file that names a constructor in prose reads exactly like a file
that calls it, and the table was assembled by grep.** The live population is
FIVE construction sites, not six.

⭐ **AND OPTION 1'S NAMED WEAKNESS IS NOW HELD, WHICHEVER OPTION WINS.**
`scripts/check_generation_mechanics_construction_is_declared.py` requires every
one of those five sites to carry a reading saying which composition it is and
why its constructor is right; an undeclared sixth fails the check. It is
deliberately option-independent — under option 1 it is the missing enforcement,
under option 2 a census of which road each site is on, under option 3 the list
of signatures to change. Eight arms, and the doc-comment control is the one
that reproduces the miscount above.

⛔⛤ **AND THIS PARAGRAPH SAID "THE `::new` AND `::of` ROWS ARE THE PROGRESS
METER THAT MUST REACH ZERO", WHICH THE CONSTRUCTORS' OWN DOC COMMENTS REFUTE ON
BOTH HALVES (corrected 2026-09-18).** `::of` is *"a generation's values with NO
App fallback"*
(`crates/ambition_platformer2d_actor_monolith/src/session/mechanics.rs:206`),
for the activation road that *"cannot be without a generation"* — under option 2
every composition has one, so `::of` is the TARGET STATE rather than the thing
to delete. And the single production `::new` is the hot reload, which
`mechanics.rs:174-177` carves out by name: a reload *"legitimately has no active
generation to read: it is building the one that replaces it, and states `None`
on purpose. Those keep [`Self::new`]."* Option 2 deletes a FALLBACK; that `None`
is not one, and the row survives every option.

⇒ **WHAT OPTION 2 DRIVES TO ZERO IS A BRANCH, AND NO STATIC COUNT REACHES IT.**
It is `for_live_session`'s `shell_routed == false && active.is_none()` path
(`mechanics.rs:185-188`), which returns `Some(Self::new(None, ..))` and is how a
headless harness reaches the App registries — a demo does NOT, as the
composition table below measures. The guard
counts which CONSTRUCTOR a site picked, and every `for_live_session` site passes
an `active` whose emptiness is a property of the COMPOSITION rather than of the
call — so the quantity that would fall is invisible to it. ⇒ Its floor of 1
still earns its place, because a site can legitimately disappear; but it is a
floor against scan loss, not a progress meter, and option 2's cost is sized by
the composition population below rather than by these rows.

⇒ **In the shipped composition there is no second construction source today.**
The discriminator is `SessionGatedSimulation`, and it is `init_resource`d in
exactly one place — `crates/ambition_game_shell/src/session.rs:348`. What is left
is the population that has no generation BY DESIGN, and the size of THAT is
what prices option 2. ⛤ This sentence used to read *"direct-entry demos,
headless harnesses and fixtures"*; the first member was measured on 2026-09-19
and is EMPTY in production, so what prices option 2 is harnesses and fixtures
alone.

⛤ **RE-MEASURED 2026-09-18, AND THE ROW HAD ONE NUMBER WHERE IT NEEDED TWO WITH
THEIR METHODS.** It said *"112 files construct through `Platformer2dSimHarness`"*.
Counted three ways:

| question | method | today |
|---|---|---|
| files that MENTION the type | `grep -rl Platformer2dSimHarness` | **113** |
| files that CALL one of its four constructors | `::new`, `::new_with_options`, `::new_with_timestep`, `::build` | **83** |
| `mod` lines in `app_it.rs` | `grep -c '^mod '` | **183** |

The 112 was the first method wearing the second's words, and the gap is 30
files — enough to matter for a cost estimate, because a file that merely names
the type in a signature needs no generation prepared for it. ⚠ 83 is itself a
FLOOR: a fixture that builds through a `common::` helper (such as
`maintainer_owned_rollback_sim`) calls no constructor of its own. ⇒ Option 2's
real cost is between 83 and 113 files, plus the demos, and the honest way to
say that is with both ends and their methods rather than one number in the
middle. The conclusion the row draws is unchanged at either end: the fallback
is load-bearing for the TEST ESTATE rather than for the game.

⭐⛤ **AND “PLUS THE DEMOS” WAS THE ONE UNPRICED CLAUSE IN THIS ROW. PRICED
2026-09-19 AS “NINE DIRECT-ENTRY CRATES”, AND RE-MEASURED THE SAME DAY AS
**ZERO** — THE CLAUSE COSTS NOTHING, AND BOTH HALVES OF THE FIRST ANSWER WERE
WRONG.** It said *"none installs a shell plugin, so none carries
`SessionGatedSimulation` and every one of them is a direct-entry composition by
construction."* Neither clause survives.

**The install chain nobody had walked.** Three hops, each one a line:

| hop | site |
|---|---|
| `ShellComposition::install` adds the group | `crates/ambition_platformer2d_provider/src/composition.rs:98` |
| `MinimalShellPlugins` adds the bridge | `crates/ambition_game_shell/src/lib.rs:83` |
| the bridge opts the App in | `crates/ambition_game_shell/src/session.rs:348` |

The bridge states the reason in place: *"composing the bridge IS the
declaration that gameplay belongs to shell-routed sessions."* So
`ShellComposition` is not merely *a* shell plugin — it is the very thing that
sets the discriminator.

**And the nine split two ways, neither of them direct entry:**

| crate | composition in PRODUCTION | shell-routed? |
|---|---|---|
| `ambition_demo_mary_o_app` | `ShellComposition::new` (`game/ambition_demo_mary_o_app/src/lib.rs:44`), `.install` at `:49` | **yes** |
| `ambition_demo_sanic_app` | `game/ambition_demo_sanic_app/src/lib.rs:50`, `.install` at `:65` | **yes** |
| `ambition_demo_smash_app` | `game/ambition_demo_smash_app/src/lib.rs:169`, `.install` at `:193` | **yes** |
| `ambition_demo_twintrack_app` | `game/ambition_demo_twintrack_app/src/lib.rs:17`, `.install` at `:22` | **yes** |
| `ambition_demo_mary_o`, `_sanic`, `_smash`, `_twintrack` | none — an experience plugin, no `App` | n/a, hosted |
| `ambition_demo_pocket` | none — no production host exists | n/a |

⇒ **THE FOUR `*_app` CRATES ARE THE MOST SHELL-ROUTED COMPOSITIONS IN THE
WORKSPACE**, and the five libraries are not compositions at all: not one of
them builds an `App` or declares a `[[bin]]`, so a demo library cannot be "a
direct-entry composition" in either direction. Each is an experience plugin
that its `*_app` installs, and `mary_o`, `sanic` and `smash` are ALSO installed
by the main game (`game/ambition_app/src/app/shell_host.rs:96-103`, under the
`MinimalShellPlugins` at `:80`). `pocket` is the one with no production entry
at all: its only `App::new()` is `game/ambition_demo_pocket/src/lib.rs:218`,
inside `#[cfg(test)] mod tests` opening at `:208`, and its only external use is
`game/ambition_app/tests/gameplay_presentation_profiles.rs:134`.

⇒ **SO THE DEMOS DO NOT PRICE OPTION 2 — THEY ARE ALREADY ON ITS ROAD.** Every
production demo entry carries `SessionGatedSimulation` today, which is exactly
the state where `for_live_session` REFUSES rather than falling back, so option
2 asks nothing new of any of them. What the fallback is load-bearing for is
the test estate and nothing else, and the 83–113 figure above is the whole
cost rather than its first term.

⛔⛤ **HOW A ROW MEASURED TWICE GOT IT WRONG TWICE, WHICH IS THE POINT.** The
first pass enumerated three SPELLINGS — `AmbitionGameShellPlugin`,
`ShellSequencePlugin`, `ShellLauncherPlugin` — and asked which crates named
one. All three are real, all three are inside `MinimalShellPlugins`, and no
demo names any of them, because a caller composes the GROUP. Searching for the
members of a plugin group is searching for the spelling rather than the
concept, and it returns a confident, uniform zero. ⇒ The fix that generalises:
ask what would SET the discriminator, walk from `init_resource` outward to its
callers, and stop only at something with no caller. That is the direction this
page already prescribes for absences elsewhere, applied to a presence.

⚠ A second reading of the same corpus also has to be separated from its tests,
and that cut fell differently on each half: `ambition_demo_pocket`'s
`MinimalShellPlugins` (`game/ambition_demo_pocket/src/lib.rs:219`) is under
`#[cfg(test)]`, so a scan that skipped the cut would have called pocket
shell-routed and been wrong in the OTHER direction. Four is the production
number, five is the number with tests included, and the two answers disagree
about a different crate than the one the census was about.

⚠ Measured with comments and test modules stripped, which changed the answer:
a first pass counted `ambition_demo_mary_o` as touching
`Platformer2dSimHarness`, and that is a doc comment at
`game/ambition_demo_mary_o/src/lib.rs:4181` describing what the harness
composes. The same trap this row already records for `world/rooms/stage.rs`.

**The decision is whether that stays the architecture.**

1. **Keep the fallback, and make the composition distinction permanent
   vocabulary.** The family becomes a legitimate separation: two composition
   modes, two authorities, discriminated by a marker instead of inferred. ⚠ The
   App registries stay a construction input forever, so the guarantee lives in
   WHICH constructor a road picks — `for_live_session` versus `new` — and nothing
   but review enforces that choice at a new call site.
2. **Give every supported composition a prepared generation**, then delete the
   fallback and the App-registry parameters entirely. ⚠ That is the measured
   83–113 harness files (the range and its two methods are in the table above)
   plus the demos, each having to prepare and activate a trivial generation
   before it can build a room.
3. **Split the type**: a `GenerationMechanics` with no fallback, plus an explicit
   authority for compositions that DECLARE they have none, so "which authority am
   I reading" is in the type rather than in an `Option` field. ⚠ Two types thread
   through the construction signature; the fixture road gains a name that says
   what it is.

⛔ **WHAT MUST NOT HAPPEN:** deleting the fallback without option 2's work — it
is not a cleanup, it is 83 to 113 files by the two methods in the table above,
plus the demos. The row is open because the second source is REACHABLE, not
because the shipped game uses it.

⚠ **AND THE CLOSURE GATE IS NOT WHAT THIS ROW USED TO SAY.** It read *"or
closing the census row while `GenerationMechanics::new`'s App parameters still
exist"*, which would keep the row open forever: the hot reload passes the App's
live `PreparedCharacterRegistry` through exactly those parameters
(`game/ambition_app/src/app/dev_runtime.rs:124`) and keeps doing so under every
option, because it is preparing the next generation rather than rebuilding a
live room from a stale one. The gate that actually matches option 2 is
`for_live_session` no longer having a branch that returns `Some` with
`active.is_none()` — the App-fallback path at `mechanics.rs:185-188`. Corrected
2026-09-18 together with the progress-meter claim above.

## Q146 — what are the supported composition profiles, and which authorities must each one carry?

**FILED 2026-09-19 BECAUSE `C07`'S GATE NAMED A RULING NOBODY HAD ASKED FOR.**
Its `DO NOT START BEFORE` reads *"Supported composition profiles must be named
first"*, and this page's own rule is that every row blocked on a maintainer
choice must name its `Q` here. It named none for four days. ⇒ The gate was real
and the question was unasked, which is the failure mode a `hold-ok` annotation
exists to prevent.

⚠ **`Q144` IS NOT THIS QUESTION, AND `C07` SAYS SO.** `Q144` rules on ONE
family — whether a supported composition must activate a prepared generation.
This row needs the general vocabulary across every optional `Res`/`ResMut` in
the tree — **806 occurrences over 203 unique type spellings**, measured by
`scripts/architecture_census.py` under the comment-stripping rule and re-run
2026-09-19.
Most are not defects. ⚠ **THE POPULATION'S OWNER IS
[`C07`'s CURRENT STATE](consolidation/consolidation-plan.md), NOT THIS ROW**,
and this row proved why: it said `820 / 206` until 2026-09-19, which is the
figure from the PREVIOUS counting rule — that one counted prose, and three of
its types existed only in doc comments. ⇒ Do not subtract across the rules;
`campaign-metrics.md` carries the rule-by-rule table and the only comparable
pair on it.

⭐ **AND THE `Q132` RULING ALREADY SUPPLIED THE TEST, WHICH IS WHY THIS IS NOW
A SMALLER QUESTION THAN IT WAS.** The scoping rule decided 2026-09-19 says
*mutable state that could legitimately differ between two sessions,
generations, participants or timelines coexisting during preparation, handoff,
rollback, multiplayer or testing must carry explicit scope*, and App-global
mutable state is appropriate only where simultaneous sessions would
legitimately share exactly the same value. ⇒ For any given optional authority,
that rule decides whether the fallback may exist. What it does NOT decide is
the thing `C07` is gated on: **which compositions the engine promises to
support**, because "required" means "required IN a profile".

**The decision is the profile list and what each profile guarantees.** Three
shapes, and they differ in what a missing authority MEANS:

1. **One supported profile — the shell-routed session.** Everything else is a
   fixture. An optional canonical authority is then a defect by default, and the
   spellings above become a finite repair list. ⚠ Costs the most in test estate:
   the harness files that construct worlds without a shell would each need an
   explicitly-scoped declaration rather than a fallback. ⚠ **THE SIZE IS OWNED
   BY `DUP-GENERATION-MECHANICS` IN
   [`architecture-census.md`](consolidation/architecture-census.md)**, which
   states it with its method — 82 files call a harness constructor and 112 name
   the type, raw, and the comment strip moves those to 80 and 103. This row
   carried *"between 83 and 113"*, a range across two DIFFERENT questions read
   as an uncertainty about one.
2. **Two named profiles — shell-routed, and declared direct entry.** Direct
   entry keeps a road, but it must DECLARE its authorities explicitly instead of
   inheriting an anonymous App-global default. ⚠ This is the shape `Q144`'s
   option 1 implies for its own family, and it is the only one that makes
   "supported composition" a checkable property rather than a description.
3. **A capability vocabulary rather than a profile list.** Each authority names
   the capability it requires and each composition declares what it provides,
   so the matrix is derived rather than enumerated. ⚠ The most general and the
   most expensive; it also matches the existing capability decision (*"a game
   may compose this engine WITHOUT a given capability"*, 2026-08-08), so it may
   already be the vocabulary this engine means.

⛔ **WHAT MAKES THIS BLOCKING RATHER THAN INTERESTING:** `C07` cannot even be
COSTED without it. "Replace optional fallbacks where the authority is required"
has no population until "required" has a referent, and the row's own census has
already carried four different figures for the population it would be counting.

## Q145 — which way round do the room-transition readiness chain and the presentation chain go in `Update`?

**Asked 2026-09-18, and it is ONE decision rather than the sixteen it first
looked like.** `RoomTransitionLoadState::active` is a single
`Option<ActiveRoomTransitionLoad>` whose own doc
(`crates/ambition_platformer2d_runtime/src/room_transition/loading.rs:293`) says
*"There is exactly one active transition."* Bevy's own `conflicting_systems()`
reports **16 unordered pairs** writing it in the shipped app, held as a ratchet
by `room_transition_load_state_writers_are_ordered_against_each_other_in_the_shipped_app`
(`game/ambition_app/tests/update_schedule_census.rs`) with a positive and a
negative control, so the number is a fact about the app rather than about a
detector that never fires.

**The sixteen are one missing edge, and the arithmetic closes exactly.**
`name_the_room_transition_conflicts` attributes every pair as
`ReadinessSet(Update)` × `NO ROOM-TRANSITION SET`:

| group | members | ordered internally? |
|---|---|---|
| `RoomTransitionReadinessSet` (`Update`) | `begin_room_transition_load_system`, `authorize_ready_room_transition_system`, `abandon_failed_checkpoint_restore_system`, `finalize_unpresented_room_transition_failure_system` | yes — one `.chain()`, membership asserted by `the_readiness_chain_still_carries_the_checkpoint_terminalization` |
| the app's `Update` writers | `contribute_room_transition_assets_system`, `poll_room_transition_asset_readiness_system`, `drive_room_transition_presentation`, `handle_room_transition_presentation_events` | yes — the first three are one `.chain()`; the fourth is pinned between `LoadPresentationSet::Actions` and `::Finalize` |

4 × 4 = 16, with nothing left over. Both chains are ordered inside themselves
and neither is ordered against the other.

⛔⛤ **AND THE READINESS SET'S ONLY ORDERING IS GATED ON A HOST THE SHIPPED GAME
IS NOT.** `crates/ambition_platformer2d_runtime/src/room_transition/mod.rs:126`
wraps its `configure_sets` in `if app.sim_is(Update)`, with a comment explaining
that on a `FixedUpdate` or GGRS host the sim is its own schedule and *"there is
no edge to draw"*. That is right about `RoomTransitionSet::Detect`/`Apply`, which
have no members in `Update` on such a host — but it means that on the shipped
GGRS host `RoomTransitionReadinessSet` carries **no ordering in `Update` at
all**, including against the four app-side systems that write the very resource
it owns. The conditional is correct about the sets it names and silent about the
ones it does not.

**The decision is which order, and it is a real one because the two mean
different things about what a transaction opened this frame may observe.**

1. **Readiness BEFORE the app chain.** `begin` opens a transaction, then the same
   frame contributes its assets and polls readiness. ⚠ `authorize` then runs
   before this frame's poll, so authorization always reads the PREVIOUS frame's
   readiness — one frame of latency on every crossing, paid always.
2. **Readiness AFTER the app chain.** Contribute and poll advance the transaction
   that is already live, and `authorize` sees this frame's readiness. ⚠ A
   crossing detected this frame does not open its transaction until the tail of
   `Update`, so the cover cannot be raised until the next frame — the latency
   moves rather than disappearing.
3. **Split the readiness chain** so `begin` leads and `authorize` trails the app
   chain. ⚠ It is currently one `.chain()` whose membership a test asserts at
   four, and the terminalization's position inside it is load-bearing (its own
   comment: *"BEFORE THE TEARDOWN, and before the next frame's `begin` can open a
   replacement transaction"*). Splitting it means that comment has to be
   re-derived, not just moved.

⇒ Whichever is chosen, the ratchet falls 16 → 0 in one edit and the guard says
so at its own definition.

⛔⛤ **AND THE QUESTION "WHICH ORDER DOES IT ALREADY RUN IN" HAS AN ANSWER NOW,
2026-09-18: NEITHER — the two chains INTERLEAVE.** Held by
`which_order_the_two_room_transition_chains_already_run_in`
(`game/ambition_app/tests/update_schedule_census.rs`), which builds the shipped
host, resolves `Update`, and reads the executable positions of the eight
systems:

```text
readiness chain : 95, 96, 97, 98      (contiguous — it is one `.chain()`)
app-side writers: 81, 82, 310, 323    (two BEFORE the readiness chain, two long after)
```

⚠ **RE-RUN 2026-09-19 AND THE POSITIONS MOVED WHILE THE SHAPE DID NOT.** The
first reading was `89, 90, 91, 92` against `80, 81, 280, 324`; the numbers
above are today's. ⇒ **AN ABSOLUTE SCHEDULE POSITION IS THE WRONG THING TO
WRITE DOWN** — it shifts when any system is added anywhere earlier in `Update`,
so it goes stale without anything about this question changing. The durable
fact is the SHAPE, and it is unchanged: the readiness chain is contiguous, two
app-side writers precede it and two trail it by hundreds of slots. The
conflict count is also unchanged — **16 unordered pairs**, re-measured the same
day by running the arm itself (`cargo test -p ambition_app --test app_it --
room_transition`, 16 tests, all passing).

⇒ **THIS CHANGES THE PRICE OF ALL THREE OPTIONS AND IT IS THE FACT THE RULING
WAS MISSING.** The reflex when an ordering is unspecified is that some order is
already happening and making it explicit is free. It is not free here: no
option describes today, so each of the three MOVES systems rather than merely
naming where they already are, and options 1 and 2 would both drag the two
trailing app-side writers across the readiness chain. ⚠ The reading also
narrows option 3 — *"split the chain so `begin` leads and `authorize` trails"* —
because the app-side four are not one contiguous block to sit between: two of
them already run before the readiness chain and two after it.

⚠ **WHAT THE PROBE DOES NOT SAY** is which system sits at which position:
`System::name()` is the debug placeholder in this build. A first draft tried to
tell them apart by `LoadPresentationSet` membership and every one came back the
same, because `handle_room_transition_presentation_events` is PINNED BETWEEN two
of those sets rather than being a member — a classifier that cannot return its
other answer, removed rather than left printing a label it had not earned. The
interleaving verdict does not depend on it, and it is poison-checked: breaking
the readiness lookup reddens the arm instead of reporting two empty groups.

⛔ **WHAT MUST NOT HAPPEN:** lowering the ratchet without adding the edge. The
number is measured on the shipped composition every run; editing it to match a
smaller reading is how a guard stops describing the tree.

⇒ **A FOURTH OPTION WAS TRIED ON PAPER AND DOES NOT WORK, RECORDED SO THE NEXT
READER DOES NOT SPEND THE SAME HOUR.** The reflex this repository has been
applying all week is *one fact, one owner*: if two chains write one resource,
give each its own and the conflict dissolves. The ownership split even looks
clean when the field writes are read (measured 2026-09-18) — the readiness chain
owns the transaction's LIFETIME (`begin` opens `active`, `authorize`/`abandon`/
`finalize_unpresented` close it) and the app-side chain only mutates its
CONTENT:

```text
contribute_room_transition_assets_system        asset_readiness_complete, asset_work_id, barrier, sequence
poll_room_transition_asset_readiness_system     asset_readiness_complete, phase, sequence
handle_room_transition_presentation_events      barrier, sequence
drive_room_transition_presentation              reads `active`
```

⛔ It dissolves the CONFLICT and not the DECISION, which is why it is not a
fourth option. `authorize_ready_room_transition_system` READS
`asset_readiness_complete` and `phase` — the very fields the app chain writes —
so after any split the same question returns as a read-order question: does
authorize see this frame's poll or the previous frame's? The latency is intrinsic
to a pipeline where one stage decides on another stage's output within a single
`Update`; moving the state into two resources would buy a new
keep-them-consistent hazard (both would have to be keyed to the same transaction)
and answer nothing. ⚠ Note also that `asset_readiness_complete` is written by
BOTH app-side systems, so the app chain is not a single-writer either — it is
ordered internally instead, which is exactly the remedy this question is asking
for between the chains.

⚠ **AND THE 16 IS A LOWER BOUND.** `retire_committed_room_transition` and
`retire_cancelled_room_transition`
(`crates/ambition_platformer2d_rollback_ggrs/src/lifecycle_commit.rs:305`,
`:366`) are plain `fn(&mut World, ..)` the commit executor CALLS, not registered
systems, so no schedule graph holds a node for them and no ordering question
about them can reach the detector. Only the second of those compare-matches
`active.intent` before clearing; the rest clear whatever is active.
