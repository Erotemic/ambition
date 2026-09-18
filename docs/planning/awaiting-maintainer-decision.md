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
Measured 2026-09-16 over the 493-row committed baseline
(`game/ambition_app/tests/rollback_schema_baseline.txt`), by kind:

| | kinds | rows | what `detail` adds |
|---|---|---|---|
| uniform | 10 | 225 | nothing — one sentence per kind, derivable from the `kind` column beside it |
| varying | 7 | 268 | facts `kind` does not encode |

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

## Q132 — when a handoff frame holds two session roots, should two hundred systems run or skip?

**MEASURED 2026-09-16, and it is C03's real shape rather than the one its row
carried.** The engine resolves "the live session world" two different ways, and
they disagree on exactly one kind of frame.

| spelling | what it is | mentions |
| --- | --- | --- |
| `SessionWorldRef<T>` | `Single<Ref<T>, With<SessionRoot>>` | 177, in 103 files |
| `SessionWorldMut<T>` | `Single<&mut T, With<SessionRoot>>` | 29, in 19 files |
| `live_session_world_root` | the root whose scope equals the ACTIVE scope | 4, in 2 files |
| `session_root_for_scope` | a named scope's root, seen through the disabling marker | 11, in 6 files |

⛔ `Single` matches NOTHING when the count is not exactly one, and a system whose
`Single` fails is SILENTLY SKIPPED. So on a frame holding two roots, 185 production sites
stop running and the four scope-aware ones keep working. ⇒ **The correctness of
two hundred systems rests on an invariant one test arm asserts:**
`the_shipped_app_never_holds_two_session_roots_across_a_handoff`
(`game/ambition_app/tests/an_edit_reaches_the_shipped_game.rs:519`), which counts
roots every frame across a real shell handoff and requires the count never to
exceed one.

⭐ **THE INVARIANT HOLDS TODAY. This is not a bug report.** The arm passes, and
`a_candidate_session_replaced_while_pending_is_discarded` passes beside it. The
question is what the engine MEANS, because the two answers license different
futures — and C03 is about to move session-owned storage, which is exactly the
work that decides whether a two-root frame can ever exist.

The choice: **(a)** `Single` is the meaning — a two-root frame is a BUG, and the
scope-aware helpers exist only for the lifecycle code that legitimately sees both
sides of a handoff. Then the invariant deserves more than one arm, and C03 may
freely assume one root. **(b)** Scope is the meaning — a two-root frame is LEGAL
during a handoff, and 185 production sites are silently skipping on it rather than
resolving the live root. Then those aliases are wrong and the migration is
large. **(c)** Keep both deliberately, and say in one place which code is
entitled to which, so the next author picks on purpose rather than by import.

⚠ **WHAT MAKES THIS URGENT RATHER THAN INTERESTING:** a system that is silently
skipped produces no error, no log and no failing test — it produces a frame where
nothing happened. That is the same failure signature as a system that ran and
found nothing to do, which is why neither the suite nor a reader can tell them
apart without being told which semantics was intended.

⛔⛤ **AND MEASURED 2026-09-16, THE TREE ALREADY GIVES THREE DIFFERENT ANSWERS,
NOT TWO.** A third road exists and it is the loudest:
`unique_session_world_root` (`shared_tangle/src/lifecycle/session.rs:416`) — the
fallback `live_session_world_root` takes in a host with NO
`SessionGatedSimulation`, i.e. direct-entry and headless — carries
`assert!(roots.next().is_none(), "more than one canonical SessionRoot exists")`.
A plain `assert!`, on in release.

| road | what two roots means | where |
| --- | --- | --- |
| `SessionWorldRef<T>` / `SessionWorldMut<T>` — both `Single<.., With<SessionRoot>>` | the system is SILENTLY SKIPPED | **185** production uses, 105 files |
| `live_session_world_root`, shell-routed | RESOLVE the one whose scope is active | 4 sites |
| `unique_session_world_root`, direct/headless | **PANIC** | the same 4 sites, other branch |

⇒ So the same condition is impossible, skippable and resolvable depending on
which road asks, and the shell-routed vs direct split means **a two-root frame
crashes a headless harness and silently no-ops the shipped game.** That is not a
disagreement about style; it is three different contracts for one state, and
whichever the ruling picks, the other two need saying so out loud.

⚠ **AND THE PANIC IS REACHABLE, which is the part a Q should have to show.** Two
named roads get there:
- `session_world_entity` → `live_session_world_root` → (no gate) →
  `unique_session_world_root`. `ambition_platformer2d/src/rollback.rs:444` calls
  it to refuse a rollback session opened over an unbuilt world — the sim-harness
  install path.
- `insert_session_world_component`
  (`crates/ambition_platformer2d_shared_tangle/src/lifecycle/session.rs:650`, *"for small direct hosts and
  focused tests"*) calls `unique_session_world_root` **UNCONDITIONALLY**, not
  through the gated branch — so that one asserts in ANY host, shell-routed
  included. Its callers include `ambition_render`'s moving-platform and
  portal-compositing setup.

⇒ The loud contract is not confined to harnesses: one of its two roads runs in
the shipped render path.

⭐⭐ **AND THERE IS ONE MITIGATION THAT IS WORTH DOING UNDER ANY OF THE THREE
ANSWERS: PRINT THE POPULATION BESIDE THE VERDICT.** The ID-PEER owner supplied
the general form of this signature from their own lane the same day, with two
instances: a rollback audit reporting *"no component changed across a save/load
of the same frame"* over **`carriers=0`** — a room authoring no ground item — and
a `BodyAnimFacts` probe reporting 36 clean comparisons where every field was
`0.000` and there was ONE distinct census. ⛔ The tell in a third case was that
two structurally different types produced the SAME digest, which is what an
empty-collection projection does; read without its population that looks like two
independent confirmations.

⇒ **"It ran and found nothing" and "it never ran" must be DIFFERENT STRINGS.** A
carrier count, a row count, a distinct-census count, a root count — anything that
makes the population visible next to the verdict. For this question the natural
one is the ROOT COUNT, which
`the_shipped_app_never_holds_two_session_roots_across_a_handoff` already computes
every frame. That mitigation does not decide the ruling and is not a substitute
for it: it makes the failure legible, not impossible.

## Q131 — how should a presentation system that writes `Transform` declare itself?

**The last blocker on ROLLBACK-MUTATOR-POPULATION, and it is a shape question
rather than an engineering one.** The guard that keeps rollback state from being
mutated outside the rewinding schedule now covers 338 types, having excluded
exactly one: `Transform`. MEASURED 2026-09-16 — of 64 offenders, 52 are
`Transform` writes from camera, sprite, parallax and inspection systems, which
are presentation acting on a component that happens to be rollback-registered.

⛔ **AND THE OBVIOUS RULE FOR TELLING THEM APART DOES NOT WORK.** Keying on a
property the system STATES — does its signature query `Camera`, `Sprite`, `Text`,
`Mesh`, `Light`, a projection — covers **23 of the 52**. The other 29 are plainly
presentation by NAME (`camera_follow`, `sync_parallax_layers`,
`sync_hit_flash_overlays`, `sync_morph_ball_visual`,
`draw_unauthored_attack_volumes`) and nothing else. ⚠ Classifying them means
matching names, and a row's name is not a reading of its write set — that
classifier was measured wrong in BOTH directions twice on 2026-09-16, once in
this guard's own neighbourhood and once in the S7 census.

⇒ So the repair is a declaration rather than a cleverer scanner, and the choice
is what the declaration IS: (a) a system set that presentation systems join, so
the guard asks the schedule rather than the source; (b) a marker component on the
entities presentation moves, so the guard asks the query; (c) a distinct
component for presentation transforms, so a presentation system cannot write the
rollback-registered one at all; (d) leave the exclusion and accept `Transform` as
a permanent blind spot, which is today's state written down honestly.

⭐ (c) is "make it impossible, not checked" and (a) is the cheapest thing that
could work. Either touches ~52 systems rather than the guard, which is why it is
a ruling: the cost is spread across every presentation author, and the benefit is
one guard's reach. ⚠ (d) is a real option and should not be dismissed — a green
from that guard already says nothing about `Transform`, and it says so where it
defines its population.

Owner row:
[ROLLBACK-MUTATOR-POPULATION](queue.md#rollback-mutator-population--the-mutator-guard-sees-a-quarter-of-rollback-state).

## Q130 — should the sim harness refuse to step an invalidated rollback session?

**MEASURED 2026-09-16.** A GGRS session that invalidates keeps accepting
`sim.step()`. It returns an observation every time and simply stops advancing
`SimTick`. Nothing panics and nothing prints. `session_health` knows; the step
loop never asks. ⇒ Every assertion after the invalidation runs over a frozen
world, where it agrees with itself.

⭐ **THE CURRENT TREE IS SAFE, AND THAT IS THE ARGUMENT FOR ACTING RATHER THAN
AGAINST IT.** Of the 21 files built on `with_sync_test_rollback_settings`,
thirteen call `rollback_health()` or `session_health`; the other eight each refuse
a frozen world by other means — a 116,280-float population floor, a recorded
stream length against the tick count, `load_runs` having moved, explicit
`room_changes > 0` preconditions, and two door walks that panic when the room
never changes. ⚠ The first version of this census counted six of those eight as
EXPOSED, because it counted calls to the safety API rather than assertions a
broken world fails. Nothing needs cleaning up.

⇒ **BUT THE CENSUS ROTS.** It proves the current 21 are safe and says nothing
about the twenty-second. A rollback arm whose assertions happen to be satisfiable
by a frozen world is exposed the moment it is written, and its author gets no
warning. That is the same argument that turns "the only production registrar is
this one" into an absence contract rather than a note.

⚠ **AND A GUARD SCRIPT CANNOT SUBSTITUTE.** The property is "this arm's
assertions are unsatisfiable by a frozen world", which six different mechanisms
produced above and a seventh would too. It is not decidable by reading source. If
the contract moves into `step`, no guard is needed; if it does not, no guard can
be written.

⭐ **THE CODEBASE ALREADY ANSWERS A NEIGHBOURING QUESTION IN THE LOUDEST
DIRECTION, which is evidence whichever way this is decided.** For a two-root
world the headless path does not return an uninterpretable reading — it ABORTS.
`lifecycle/session.rs::unique_session_world_root` carries a plain
`assert!(roots.next().is_none(), "more than one canonical SessionRoot exists")`,
ungated and live in release, and `live_session_world_root` falls through to it
whenever `SessionGatedSimulation` is absent — direct entry and headless, which is
every harness this row is about. The shell-routed branch instead resolves the
same condition by scope, silently. ⇒ So "the harness refuses rather than hands
back a reading nobody can interpret" is already precedent here, and it is
stronger than anything this Q proposes. (ToothbrushAmbition's find, filed on
their side as part of Q132.)

⛔⛤ **THAT THIRD POPULATION WAS OVERSTATED AND IS NOW RESOLVED — BOTH COUNTS
WERE RIGHT ABOUT DIFFERENT THINGS.** The first report said "~206 `Single<..,
With<SessionRoot>>` sites", which was raw grep MENTIONS of the two type aliases,
comments and tests included. A peer then counted **11** `Single<..SessionRoot..>`
occurrences out of **16** `Single<` parameter sites in the whole workspace and
could not reach 206 — correctly, because `SessionWorldRef` and `SessionWorldMut`
are `pub type` ALIASES for `Single<..>`, so a scan keyed on the word `Single`
cannot see any of their uses.

⇒ **MEASURED with comments stripped and tests excluded: 163 `SessionWorldRef<`
plus 22 `SessionWorldMut<` = 185 production uses across 105 files**, and the
direct `Single<..SessionRoot..>` spellings the peer counted are additional. The
SHAPE never depended on the number; it is carried by the assert, the scope
branch, and the existence of skip-shaped sites at all.

⚠ **THE LESSON IS THE ALIAS, NOT THE ARITHMETIC:** a type alias makes a
population invisible to a scan keyed on what it expands to, and visible only to
one keyed on its name. Two honest scans of the same tree disagreed by an order of
magnitude for that reason alone.

⚠ A practical note for anyone reading a failure here: that `assert!` fires
inside a helper, so the arm named in the output is the last one that ran, not
necessarily the one at fault.

The choice: (a) `Platformer2dSimHarness::step` panics when the session has
invalidated, making silence impossible — the census above is the evidence that
nothing currently relies on stepping a dead session, so this should redden
nothing today; (b) it returns an error the caller may ignore, which is the
current situation with a nicer name; (c) leave it, and accept that each future
rollback arm's safety is its author's to remember.

⭐ (a) is "make it impossible, not checked" applied to a harness contract, and
the reason it is a ruling rather than a patch is that it changes what every
existing and future rollback arm is allowed to do — a harness that panics on a
dead session will red any arm that turns out to be relying on one, and the census
is a claim about today rather than a proof about tomorrow.

Owner row:
[ROLLBACK-DEAD-SESSION](queue.md#rollback-dead-session--an-invalidated-ggrs-session-stops-the-clock-in-silence).

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
ESTIMATED.** `scripts/resources_crossing_the_rewind_boundary.py` sweeps every
`Resource` written on both sides of the boundary — **50** of them at 2026-09-17
(was 53 earlier the same day; see below) — and sorts them: 29
rollback-registered, 17 adjudicated harmless with the argument beside each, 3
crossing only at a session edge, **1 FILED against this question
(`CutsceneAdvanceRequest` — a dismiss raised on the host side does nothing), and
0 that nobody has examined.**

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
`tick_player_clone_brains` defect measured the same day, and
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
| `SpawnPlayerCloneRequest` | no | same as the cutscene; newly visible |
| `VersusMatch` | yes | filed under `Q140`, not new |

⛔ The prose above ("the residue is one") was measured before `c215d6a37`, when
both censuses tested the host side against a tuple of BARE schedule labels
(`"Update"`, `"PreUpdate"`, …). The tree spells a non-rewinding schedule in
QUALIFIED form 39 times, so `request_player_clone_on_key`
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
| an AUTHOR or DEVELOPER editing the world | `MechanicalEditSet` + the admission | `SpawnPlayerCloneRequest` |
| a MENU ending the match | option 3, session-level | `NewGameResetRequested` |

⭐ **AND `SpawnPlayerCloneRequest` IS THE SPECIMEN TO LAND FIRST**, for a reason
that is about risk rather than about it being easy: `plugins.rs:189` explains the
Update/sim split in its own comment and the explanation is CORRECT — `ButtonInput`
is winit frame state, so reading `just_pressed` on the deterministic tick sees one
physical press once per SIM RUN and a frame that steps the sim twice spawns two
clones. Moving the read into the sim reintroduces that. The double-spawn and the
swallowed press are the same problem from its two sides, which is why this is one
ingress question and not three fixes. The stakes are a dev hotkey — no save data,
no peer checksum, no progression — so the road can be built and witnessed here
without a mis-step costing a timeline.

### 2026-09-18 — the first road is landed, and the fixture was the hard part

⭐⭐ **`SpawnPlayerCloneRequest` IS FIXED, BY THE MECHANICAL-EDIT ROAD, AND BOTH
CENSUSES AGREE IT IS GONE.** `spawn_requested_player_clone` no longer runs in
`app.sim_schedule()`: the request is proposed in `MechanicalEditSet::Propose` and
the spawn published in `Publish`, both in `PreUpdate`, with
`decide_mechanical_edit_admission` between them. Witnessed by
`a_dev_clone_survives_a_rewind`, which carries a fixed-tick control arm (1 clone)
beside the rollback arm — and poisoning the registration back into the sim
schedule reddens both arms while the control stays green.

⇒ Two repairs came with it, each a separate defect the move exposed:

- **the press is spent LAST now.** `request.0 = false` stood above every refusal
  in the spawn, so a press arriving on a frame with no resolvable primary — or
  with a primary not yet carrying a `SimId` — was consumed and the clone never
  appeared. Refusing a sub-step is not refusing the operation. Held by
  `a_press_the_spawn_cannot_honour_yet_is_kept_rather_than_consumed`.
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
`tick_player_clone_brains` is registered into the SIM schedule and read
`time.delta_secs()` — the app's WALL dt — accumulating it into a
`PlayerCloneClock` resource that was `init_resource`d and never registered for
rollback. A resimulated frame therefore added dt AGAIN to a value no rewind
restored, so `snapshot.sim_time` differed between the original run and the
replay, the demo brain emitted a different frame, and the clone's
`BodyKinematics` diverged.

⇒ **IT IS A DUPLICATE AUTHORITY, AND THE COLLAPSE IS THE FIX RATHER THAN A
REGISTRATION.** `GameplayElapsed` is the same fact, accumulated the same way
(`+= world_time.scaled_dt`), rollback-registered, advanced at the head of
`WorldPrep`, and its own doc says *"before any actor brain reads the snapshot"* —
which is exactly where `tick_player_clone_brains` reads. Registering
`PlayerCloneClock` would have made the drift rewind correctly and left two owners
of *how long gameplay has run*; deleting it leaves one. ⭐ The `dt` moved to
`WorldTime::sim_dt()` in the same edit, which also sharpens the pre-existing zero
guard: `sim_dt` is `raw_dt * time_scale`, so it is zero while PAUSED or in
hitstop, and ticking a demo cycle through a pause was never intended.

⚠ **THIS IS THE THIRD TIME THIS WEEK A HOST-LOCAL ACCUMULATOR INSIDE THE REWIND
HAS BEEN THE DEFECT**, and `sim_plugin.rs` names the other two at the
registration that moved them out: `sync_developer_body_profile` was *"arbitrated
by a `Local` that runs once per ADVANCE and therefore remembered across a
rewind"*, and `sync_live_player_dev_edits_system` wrote five movement clusters
from a live inspector resource. ⇒ Worth a guard of its own: a system in the sim
schedule that accumulates into a `Local` or into an unregistered resource is the
shape, and all three instances were invisible to every existing census because
none of them is a *multi-writer* and none of them crosses a schedule boundary.

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
so. Measured 2026-09-18: `grep -rn LocalSessionPolicy` over
`game/ambition_app/tests/` and `crates/ambition_sim_harness/src/` returns
**nothing**, and the dev-tools unit fixtures insert
`MechanicalEditAdmission::Publish` directly, bypassing the decider. So the
`LocallyRebasable` arm — the one the game takes — had no coverage at all.
`a_dev_clone_survives_a_rewind::hand_the_timeline_to_the_local_maintainer` is the
first fixture that re-owns its session the shipped way; it stops the
caller-owned session and lets the maintainer build its own, because the owner
stamp and the installed session are one fact and writing half of it describes a
world that cannot exist.

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

- `ambition_dialog` contains the string `rollback` **zero times**. `DialogState`
  is a plain `#[derive(Resource)]`, registered on no road.
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
capped at `1.4`, and the duel's own damage totals put the largest multiplier
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
and `symmetry_room` the four `SetGravity{Down,Left,Up,Right}`.

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

⚠ **WHAT THE MEASUREMENT DOES AND DOES NOT SETTLE.** It settles the blast radius:
the exposed population is ZERO today. Of 26 sync-test fixtures, 15 read a health
API and the rest already refuse a frozen world with assertions a stopped clock
cannot satisfy (a population floor, a recorded stream length against the tick
count, a room change after an authored hold). So (a) would red nothing at HEAD.
It does NOT settle the policy, because a harness that panics on a dead session
takes the choice away from a future arm that legitimately wants to step one — an
arm testing the invalidation itself, for instance — and that is the maintainer's
call rather than a census's.

⛔ **NOT A CLEANUP, AND THE SIX ARMS MUST NOT BE EDITED EITHER WAY.** Adding
`rollback_health()` to an arm whose assertions already cannot pass over a frozen
world trades a strong guarantee for a visible one. Counting calls to a safety API
measures vigilance; counting assertions a broken world fails measures safety.

## Q139 — what declares that a presentation system writes `Transform`?

`scripts/check_rollback_mutators_run_in_sim.py`'s component half excludes
`Transform` BY NAME, with the count beside it: 52 of the 64 offenders it would
otherwise surface are `Transform` writes from camera, sprite and inspection
systems. So a green there says nothing about `Transform` — the single most
rollback-sensitive component in the workspace.

⇒ **THE OBVIOUS REPAIR IS REFUTED AND THE ROW RECORDS THE REFUTATION.** Classify
by a property the system already states — `Camera`/`Sprite`/`Text`/`Mesh`/
`Light`/`Node`, or a projection in the signature — and the 52 split **23 that
declare such a marker and 29 that do not**, where the 29 are presentation only by
NAME (`camera_follow`, `sync_parallax_layers`, `sync_hit_flash_overlays`). A
system's name is not a reading of its write set; that classifier was wrong in
both directions twice on 2026-09-16 alone.

**So the repair is a DECLARATION, not a cleverer scanner**, and the decision is
its shape:

* **(a) a set** — presentation `Transform` writers join a named system set, and
  the guard reads membership instead of parsing signatures;
* **(b) a marker on the entities** — the things presentation moves carry a
  component saying so, and the guard asks about the entity rather than the
  system;
* **(c) a wrapper type** — presentation writes a distinct component the render
  layer lowers to `Transform`, which makes the mutation unspellable rather than
  merely declared.

⚠ **THE SIZE IS THE REASON THIS IS A QUESTION.** It is ~52 systems across the
render, camera and inspection layers, not a script change, and the three shapes
put the cost in different places — (a) is cheapest and weakest, (c) is the only
one a future system cannot forget. Nobody should start until the shape is chosen.

## Q140 — may the item menu show a stale bag for one frame?

[MENU-RESET-MIDSESSION](queue.md#menu-reset-midsession--the-menu-writes-rollback-state-from-update)
is blocked on one UI question, and the engineering half of it is already decided.

The menu writes `OwnedItems` — a rollback-registered resource — from `Update`,
outside the simulation schedule. The sanctioned road exists and the menu does not
use it: `ItemGrantRequested` is `clear_message_on_rollback` and its consumer
`apply_item_grants` mutates `OwnedItems` from the SIM schedule, beside
`apply_shop_transactions`. So a conversation that gives you an item is
rollback-correct today and the menu giving you one is not, for the same resource
in the same crate.

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
current row is not `InCustody` or `Placed`. The measurement and the guard are at
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

## Q142 — ✔ THREE OF THE FOUR ARE REGISTERED AND WITNESSED; the question is down to `PostBossNpc`

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
checksum* — 99 of them, 25 float-bearing, 12 mutably written — and it derives
that population from the REGISTRY. A float advanced every tick by a sim system on
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
workspace — all three of them:

| script | `to_alpha` | `seconds` | position in script | room |
|---|--:|--:|---|---|
| `test_intro` | **0.0** | 0.8 | second, after a banner | `central_hub_main` |
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

⛔ **WHAT MUST NOT HAPPEN IS A CONSUMER LANDING ALONE.** It satisfies the
UNFINISHED label, reads as the row closing, and leaves the player waiting 2.2 s
across three rooms for a screen that never changes.

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
| reset, room transition (×2), room stage | `GenerationMechanics::for_live_session` | **refuses** when `SessionGatedSimulation` is present |
| provider activation | `GenerationMechanics::of` | has no fallback to offer |
| hot reload | `GenerationMechanics::new` | states `None` on purpose — it is building the generation that replaces the live one |

⇒ **In the shipped composition there is no second construction source today.**
The discriminator is `SessionGatedSimulation`, and it is `init_resource`d in
exactly one place — `crates/ambition_game_shell/src/session.rs:348`. What is left
is the population that has no generation BY DESIGN: direct-entry demos, headless
harnesses and fixtures. 112 files construct through `Platformer2dSimHarness` and
`app_it` alone is 182 test files, so the fallback is load-bearing for the test
estate rather than for the game.

**The decision is whether that stays the architecture.**

1. **Keep the fallback, and make the composition distinction permanent
   vocabulary.** The family becomes a legitimate separation: two composition
   modes, two authorities, discriminated by a marker instead of inferred. ⚠ The
   App registries stay a construction input forever, so the guarantee lives in
   WHICH constructor a road picks — `for_live_session` versus `new` — and nothing
   but review enforces that choice at a new call site.
2. **Give every supported composition a prepared generation**, then delete the
   fallback and the App-registry parameters entirely. ⚠ That is the measured 112
   harness files plus the demos, each having to prepare and activate a trivial
   generation before it can build a room.
3. **Split the type**: a `GenerationMechanics` with no fallback, plus an explicit
   authority for compositions that DECLARE they have none, so "which authority am
   I reading" is in the type rather than in an `Option` field. ⚠ Two types thread
   through the construction signature; the fixture road gains a name that says
   what it is.

⛔ **WHAT MUST NOT HAPPEN:** deleting the fallback without option 2's work (it is
not a cleanup, it is 112 files), or closing the census row while
`GenerationMechanics::new`'s App parameters still exist. The row is open because
the second source is REACHABLE, not because the shipped game uses it.

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
