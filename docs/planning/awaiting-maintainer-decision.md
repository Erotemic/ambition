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

⚠ **NOTHING IN THE REPOSITORY CAN CURRENTLY OBSERVE THE DEFECT.** The only
sessions in use are `SyncTestSession` — one machine rewinding itself, zero
distance — and a canary that compares a machine against its own past is
structurally incapable of catching a two-peer disagreement. Every leak in that
campaign had to be found by reading. So this will not announce itself, and it
does not get more urgent on its own.

The choice: (a) rebase the tick at an agreed session start, which means deciding
what "agreed" is before there is a handshake to carry it; (b) keep it absolute
and accept that peer comparison waits for real sessions, recording it as a known
hole rather than an oversight; (c) project it out and replace the timeline term
with something else, which nobody has proposed a shape for. Recorded by the
`queue.md` ID-PEER table, which names this as one of two roads still open — the
other is `Q122` above, the snapshot schema fingerprint hashing prose.

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
[ROLLBACK-BAG-DESYNC](queue.md#rollback-bag-desync--a-per-tick-change-to-an-unhashed-resource-desyncs-the-sync-test);
the owner document is
[DURABLE-HORIZON-CHECKSUM](queue.md#durable-horizon-checksum--the-save-mirrors-write-hashed-state-from-update).

## Q127 — are difficulty, assist and player-damage modifiers match-wide or participant-specific?

`PlayerDamagePolicy` now projects the settings values that deterministic damage
simulation consumes, so simulation no longer reads the full mutable settings
resource. The remaining policy choice is its lifetime and subject: freeze one
agreed value for the match, or publish a value per participant/seat as an
accessibility policy. Both are mechanically viable; the product rule decides the
shape of the admitted authority.
