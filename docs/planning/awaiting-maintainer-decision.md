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

The mechanical fingerprint/admission boundary should include only fields that can affect deterministic gameplay. Decide the classification for mixed registries so presentation edits do not force mechanical rebases, while mechanical edits cannot bypass rollback admission. Record the rule per registry owner rather than maintaining one ad-hoc exclusion list.

## Q127 — are difficulty, assist and player-damage modifiers match-wide or participant-specific?

`PlayerDamagePolicy` now projects the settings values that deterministic damage
simulation consumes, so simulation no longer reads the full mutable settings
resource. The remaining policy choice is its lifetime and subject: freeze one
agreed value for the match, or publish a value per participant/seat as an
accessibility policy. Both are mechanically viable; the product rule decides the
shape of the admitted authority.
