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
closed by engineering alone.** `ambition_time::SimTick` is registered
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
(`game/ambition_app/tests/an_edit_reaches_the_shipped_game.rs:435`), which counts
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
`unique_session_world_root` (`shared_tangle/src/lifecycle/session.rs:398`) — the
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
  `unique_session_world_root`. `ambition_platformer2d/src/rollback.rs:443` calls
  it to refuse a rollback session opened over an unbuilt world — the sim-harness
  install path.
- `insert_session_world_component`
  (`crates/ambition_platformer2d_shared_tangle/src/lifecycle/session.rs:588`, *"for small direct hosts and
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
that changes once per tick desyncs a GGRS sync test within six ticks. The chain:
`persist_inventory_to_save` sits in top-level `Update` and writes the live bag
into `AmbitionGameSave` once per FRAME; `AmbitionGameSave` is registered
`rollback_resource_clone_checksum`, so its value is compared once per TICK; and a
rewind re-simulates ticks without re-running `Update`. The hashed save therefore
describes a different frame from the tick it is compared at. Of 364 probed
rollback entries, exactly ONE differs between a run whose bag moves and an
otherwise identical run whose bag does not, and it is this one.

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

**AND THE CENSUS IS NOW DONE: 13 OF THE 19 WRITERS ARE IN A REWINDING SCHEDULE.**
Every system taking `ResMut<AmbitionGameSave>` workspace-wide, resolved to the
`add_systems` call that registers it:

| registered in the sim schedule (13) | not (6) |
|---|---|
| `apply_flag_effects`, `apply_quest_advance_events`, `apply_wave_encounter_effects`, `capture_falling_sand_switch_interactions`, `celebrate_symmetry_attunement`, `drain_switch_activations`, `drive_wave_encounters`, `grant_quest_completion_rewards`, `heal_save_shrine_system`, `reset_cut_rope_attempt_on_replay`, `retire_rewards_for_rearmed_encounters`, `tick_active_cutscene`, `update_boss_encounters` | `dispatch_pending_dialog_requests`, `load_save_at_startup`, `track_room_visits`, and the three `persist_*_to_save` mirrors |

⇒ **THE SAVE IS SIMULATION-ADJACENT STATE IN PRACTICE, WHATEVER IT IS IN
PRINCIPLE.** Quest advances, boss encounter progress, switch activations, shrine
heals and cutscene ticks all write it from inside the rewinding schedule, where
the checksum is doing real work. Taking it out of the peer contract would stop
comparing all thirteen. (b) is therefore not the small option; it is the largest
one, measured by what it stops checking.

⚠ Two method notes, because the count would have been wrong twice without them.
A name inside `.after(...)` is an ORDERING EDGE, not a registration — excluding
those is why `heal_save_shrine_system` is counted from its real `add_systems` and
not from `checkpoint.rs:1755`. And the three `persist_*` mirrors landing on the
`Update` side is the positive control: a classifier that put them anywhere else
would be wrong about the very systems this Q is named for.

⚠ It also applies to more than the bag: `persist_occurrence_horizon_to_save` and
`persist_minted_item_horizon_to_save` write the same resource from the same
`Update` chain, so a ruling here settles three systems, not one.

Reproduction, eliminations and the full harness matrix are in
[ROLLBACK-BAG-DESYNC](queue.md#rollback-bag-desync--ambitiongamesave-disagrees-with-its-own-rollback-replay);
the owner document is
[DURABLE-HORIZON-CHECKSUM](queue.md#durable-horizon-checksum--the-save-mirrors-write-hashed-state-from-update).

## Q127 — are difficulty, assist and player-damage modifiers match-wide or participant-specific?

`PlayerDamagePolicy` now projects the settings values that deterministic damage
simulation consumes, so simulation no longer reads the full mutable settings
resource. The remaining policy choice is its lifetime and subject: freeze one
agreed value for the match, or publish a value per participant/seat as an
accessibility policy. Both are mechanically viable; the product rule decides the
shape of the admitted authority.

## Q135 — should GGRS start before the durable restore has finished?

`maintain_local_session` starts the rollback session on
`session_world_entity(world).is_some()`. The durable-restore chain —
`adopt_occurrence_checkpoint_from_save`, `restore_inventory_from_save`,
`complete_durable_restore` — waits for a primary player BODY, which is a later
fact. Both live in top-level `Update` with **no ordering edge between them.**

**MEASURED 2026-09-16**, `probe_when_the_durable_restore_latch_flips_against_ggrs_start`
in `game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`:

| | session world | primary body | GGRS live | latch set |
|---|--:|--:|--:|--:|
| first frame true | 1 | 1 | 1 or 2 | 2 |

- A sampler with an explicit `.after(complete_durable_restore)` edge finds the
  GGRS session **already live** at the instant the latch is set.
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
   re-derives what it wrote. That needs `SaveRestored` to become rollback state
   and the "one-shot at boot" shape to survive being replayed.
3. **The writes do not matter**, which needs an argument this row could not find:
   they are hashed, they are inside the window, and nothing re-applies them.

⚠ **AND THE DETECTOR IS GREEN FOR A REASON THAT IS NOT SAFETY.**
`no_registered_type_is_written_outside_the_rewinding_schedule` can see these types
— they are value-probed — and passes because the harness boots with NO SAVE FILE,
so `adopt_the_ledger` writes the same empty value it found. The comparison is
between two identical censuses. **Whoever takes this needs a SEEDED save; nobody
has built one.** Do not quote that arm's green against this question.

## Q134 — is a dialog visit count something two peers must agree on?

[DURABLE-HORIZON-CHECKSUM](queue.md#durable-horizon-checksum--the-save-mirrors-write-hashed-state-from-update)
repaired the three `persist_*_to_save` mirrors by moving them into the rewinding
schedule: they DERIVE the save from simulation state, so a replay reproduces the
value. `dispatch_pending_dialog_requests` is a fourth writer of the same hashed
resource and that answer is not available to it — it calls
`save.data_mut().increment_dialog_visit(&dialogue_id)`, and an increment is
neither idempotent nor derivable.

**MEASURED 2026-09-16, and the measurement closes one of the two branches the row
had been holding open.** The row asked whether the visit is LOST on a rewind or
COUNTED TWICE. It can only be lost:

- `ambition_dialog` contains the string `rollback` **zero times**. `DialogState`
  is a plain `#[derive(Resource)]`, registered on no road.
- The dispatcher consumes the request with `state.pending_start.take()`, in
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

1. **A visit count is durable PROGRESS, not simulation state**, and the save's
   checksum projection should stop covering fields no tick derives. That is
   cheap here and expensive in general: it changes what "the peers agree on the
   save" means for every other field at the same time, and it needs a rule for
   deciding which side of the line a field is on, not a list.
2. **A visit count is simulation state**, and the dialogue request has to become
   rewinding state so the increment can live in the sim schedule. That is the
   honest version of "the save is part of the shared world", and it is a real
   piece of work: `ambition_dialog` has no rollback vocabulary at all today.

⇒ This is narrower than
[Q129](#q129--must-the-save-file-be-part-of-what-two-peers-agree-on) and does not
wait on it: Q129 asks whether the save belongs in the checksum, and this asks what
to do with a field that **no tick can reproduce** whatever the answer to Q129 is.
A ruling of "exclude the save entirely" on Q129 would moot this; any other ruling
does not.

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
