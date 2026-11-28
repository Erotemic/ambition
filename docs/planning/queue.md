# The queue — live execution order

This file contains **executable, unresolved engineering work**, not a work log.
An item remains here only with a reproducible defect or concrete customer,
a next action, and a checkable acceptance condition. Re-measure the source
before starting a row; retire it upon completion. Focused owner documents
carry architecture, measurements and implementation history; Git history
carries closed campaigns. Unselected or customer-gated plans live in
[`tracks.md`](tracks.md), and product questions in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

How to read this page: **Current order** says what to pick next. The
**active rows** each have a next step an agent can take now. **Waiting** rows
have a next step only a person or a particular machine can take: look again
when that changes. **Receipts** keep a closed row only while something links
to it. A finished row is deleted, not marked; its history stays in its owner
page and in Git.

## Current order and selection

Take the first item with an actionable next step. Skip a gated step without
calling it done, and go on to the next.

1. **Portals (Jon, 2026-11-26 and 2026-11-27).** [PORTAL-SIMILARITY](#portal-similarity--canonical-parts-and-scaled-mirror-portals):
   P3 is done (2026-11-27): aperture-rim contact is one cast primitive
   (`material_cast/rim.rs`), with adapters for character and ground-item
   movement, and the post-step snag writer is deleted. T3 landed the same
   day: a room's replay keeps the hidden rooms' portal publication. Water
   membership, chest/switch touch and climbing read the body's admitted
   parts, and `body.regions` observes that query with its cost counted
   (all 2026-11-27). A deforming line reaching or leaving an aperture's
   plane reads its own end travel (2026-11-27). A line on a turning plane
   keeps the rigid fit, which is exact when it turns with the plane. RV3 is
   handled separately.
2. **Hobford polish (Jon, 2026-11-14).** [Hobford polish](game/hobford-polish.md)
   with [LEVEL-1](#level-1--the-lab-raid-and-the-first-level-after-it) and [LEVEL-TRUTH](#level-truth--authored-level-data-in-the-map-submodule-one-owner-per-room) for the level data. Story
   sequences SS1–SS8 (SS7/SS8 landed 2026-11-21) and dark rooms and tunnels
   (DR1–DR3, TN1–TN3) are done. The open story item (no general policy
   when a required command is refused) is kept in
   [story sequences](engine/story-sequences.md).
3. **Script-first gameplay (Jon, 2026-11-20).** New flagship gameplay follows
   the [capability program](engine/script-first-ambition-gameplay.md); the
   [capability census](engine/script-first-capability-census.md) records the
   contracts and the next gaps, and its P0/P1 gaps are closed.
   [CONTENT-AS-DATA](#content-as-data--content-still-written-as-rust) and [I2/I3](#i2i3--finish-independent-content-authoring-and-safe-reload) hold the remaining authoring
   work. Keep working native mechanics, and do not use port counts as
   acceptance. The [authoring recipe](../recipes/scripted-stage-attacks.md)
   describes the built-host edit loop. **Retained receipt:**
   [first-class procedural game authoring](engine/first-class-procedural-game-authoring.md)
   proves A1–A10 on desktop Linux (`9c1ccf62d`); keep its fixtures. The older
   [runtime authoring audit](engine/runtime-authoring-audit-2026-11-14/README.md)
   proposes migrations that need their own product benefit first.
4. **Architecture and correctness.** [ARCHITECTURE-COMPOSITION](#architecture-composition--link-owned-facts-and-preserve-causal-boundaries) (the
   2026-11-21 plan), [EXECUTION-COST](#execution-cost--specialize-gameplay-work-at-its-actual-inputs), [BUILD-BOUNDARIES](#build-boundaries--share-implementation-after-typed-selection),
   [AP14](#ap14--actor-kernel-decomposition), [A7](#a7--make-item-occurrence-authority-explicit-where-it-carries-a-real-invariant) and [TEST-LANES](#test-lanes--keep-required-test-lanes-executable). The
   [reassessment](engine/architecture-reassessment-and-composable-multiplayer.md)
   does not by itself authorize networking, physics or rollback rewrites.
5. **Network (direction set 2026-11-10).** A hybrid world: rollback inside a
   room, hosted authority around it. [SIMULATION-ISLANDS](#simulation-islands--room-level-rollback-inside-a-hybrid-world) and
   [NET-AUTHORITY-EXPERIMENT](#net-authority-experiment--compare-hosted-world-replication-with-ggrs); Smash keeps its bounded rollback.
6. **Gameplay and presentation.** [GNUTON-POLISH](#gnuton-polish--the-gnu-ton-fight-presentable-jon-2026-11-21), [FINAL-BOSS](#final-boss--the-bossofthegame-on-the-shoulders-of-a-giant),
   [ABILITY-VARIANTS](#ability-variants--finish-the-authored-loadout-through-shared-capability-seams) (Q179's press-hold-release blink),
   [POSE-CONTINUITY](#pose-continuity--one-visual-transition-path-after-the-completed-base-poses), [PRESENTATION-COMPOSITION](#presentation-composition--order-appearances-and-add-a-25d-view),
   [MOUNT-RIDER-CUSTOMER](#mount-rider-customer--a-shipped-rider-controls-a-shipped-mount), [BRAIN](#brain--finish-truthful-fighter-attack-selection) and [ASSET-AUTHORING](#asset-authoring--edit-owned-sources-and-prepare-the-meaning-once).
   These are bounded rows, not a second priority list.

Moved to [`tracks.md`](tracks.md#moved-from-the-queue-2026-11-27) on
2026-11-27: Smash parity (D72), which stays behind the main Ambition work
(2026-11-22 review); small-viewport UI, which waits (Jon, 2026-11-17); and
the rig-impostor remainder, which waits for a body that carries a `PartPose`
or a GPU lane.

Cross-version saved-content compatibility is deferred to 2027; current-session
rollback and native reconstruction keep their obligations. The source may be
ahead of a row: re-measure before starting, and do not redo finished work
because an old measurement says so.

**Automation/link contracts:** `scripts/checks/check_blocking_set_names_every_gate.py`
reads each `**Blocked by:**` field through its next blank line, plus
`**Acceptance:**` clauses. A heading containing `✅`, `DONE` or `CLOSED` marks
a closed row for the consolidation/hold guards. Keep the **ID-PEER** and
**A10** closed receipts below: the consolidation checks use their IDs and
state. Inbound `queue.md#...` links require retaining exact heading anchors
or updating their source. Check with
`python3 scripts/checks/check_planning_anchors_resolve.py`.

## P0 — architecture and correctness

### ARCHITECTURE-COMPOSITION — link owned facts and preserve causal boundaries

**State:** PLANNED (architecture review, 2026-11-21). Source baseline
`6895e516863872eb6d38093a27225ed8e2973d70`; no implementation or new
performance result is claimed. This row selects bounded architecture work
within the product order above. The anchor, role and interaction follow-up
below reviews production source `b91b29d35935`. The implementation follow-up
at `3ed50e318392` keeps the CW1, CW2 and Effects 6a receipts and identifies
the remaining authority boundaries below. It reports no new runtime result.
The follow-up at `f41218979625` supplies the missing cutscene playback
occurrence for SS7/SS8. The SDK publication completion is selected in
[I2/I3](#i2i3--finish-independent-content-authoring-and-safe-reload).
The further review of that same production source adds A7 item recipes and
native continuity, O6 merchant subject/verdict completion, and native module
room-time completion under SIMULATION-ISLANDS. These are planning changes;
they do not claim newer engine source or runtime results.
The 2026-11-22 review of the same source adds AC1/AC2 native action completion
and cancellation, and CR1/CR2 combat reception and limb injury. Preserve the
earlier implementation receipts; these packets address composition cases
those receipts did not prove.
The later review at `34b4ab0b7f4f` adds ACT1/ACT2 below. It preserves
the now-landed CR1 and AC2 first slices, E2's prepared clip facts and E4's
navigation work. No new runtime result is claimed.
The deeper review of the same source refines these owners: ACT1's offer
must name the action execution can accept; ACT2 separates live maintenance
from elapsed progression; CR1 settles static hit reactions at native
reception. CLIP3 retains source motion under selected move retiming.
The endpoint review at `00bf92c22142` adds REL1–REL3 beside the completed
anchor slices and REL4 under A7. These share native admission and accepted
transition cuts; they do not promote a general relationship framework or
Smash-only work.
The observation review of the same source adds OBS1–OBS3: native exact
evidence admission, memory/report semantics and declared knowledge demand
for module controllers. It also refines SS7's opening scope. Durable social
knowledge and generated dialogue remain unselected.
The shared-observer/control review of the same source adds IN4/IN5 and
MV1: physical edge observation survives routing changes, pending
input-dependent work sees admitted interruption, and each view derives
framing/continuity from its selected target. P2's provider-action final hop
and broader M3 shared-view control choices remain customer-gated.
The static-comprehension review of the same runtime source adds SC1 below
and refines CR1 strike construction. E2 receives the named animation
binding under its existing owner. These are required operation relations,
not a general readability rewrite; no native run or performance result is
claimed.
The authored-composition review of the same source adds explicit empty
geometry under combat, attack-family selection under L1, and FORM-REBIND
below. These close lost decisions at existing native boundaries. Keep the
completed scale compiler and CHARACTER-FORM geometry receipts.

**Observed:** schema values can lower before their imports resolve; body and
candidate contexts collect optional domain catalogs; several live-contribution
APIs discard application identity; native shared-world undo does not account
for decisions made by another room. The
[architecture owner](engine/architecture.md#structural-direction-from-the-2026-11-21-review)
links the source evidence and the revised plans.

**Decision:** link typed domain contributions against exact selected inputs,
keep their interpreters optional, preserve independently revocable application
identity, and cross rollback authorities through committed domain operations.
Reuse existing candidate construction, Bevy plugins, action receipts and
world-post admission.

**Next bounded work:**

The [implementation review of `1bef25200e00`](engine/implementation-review-2026-11-23.md)
checks the agents' work against the native paths. Its corrections stay in
ACT1, IN2, OBS3 and the music row below; it adds no separate priority order.
Read the relevant counterexample before treating a shared helper or retained
value as completion of that owner's contract.

1. At the active independent-room boundary, run
   [CW1](engine/causal-world-commit.md#cw1--prove-the-dependent-reader-case-and-complete-held-outcomes)
   (done 2026-11-21: the dependent-reader case is a measured failing baseline
   on the shared bag, and a held world-value write reports `Admitted`, then
   its real decision under the same ticket). The first CW2 cell grant/use
   slice is implemented with four hosted cases. Completion remains open:
   [CW2 follow-up](engine/causal-world-commit.md#cw2-follow-up--preserve-authority-under-pressure)
   removes eager grant, spend and refund on queue refusal, and keeps each
   committed allocation owned through use or return (implemented 2026-11-21:
   typed admission, per-room credits, settlement at the confirmed cut; three
   pressure cases added to the hosted witness; source and recipient retirement witnessed 2026-11-24 in `committed_bag::retirement_tests`). Source-owned admission
   credits replay at their recorded cuts; today's foreign queue cannot
   change an old outcome. Prove pressure, destination replay and source
   retirement before extending this road to more domains. Unrelated local
   work continues.
2. [Effects phase 6a](engine/actor-effects-and-modifier-composition.md#6a-preserve-issuer-identity-in-control-and-pose)
   preserves independent hold and pose issuers (pilot implemented 2026-11-21:
   holds and poses are claims keyed by application; a transform's expiry
   withdraws only its own pose). Complete the
   [occurrence and restore boundary](engine/actor-effects-and-modifier-composition.md#6a-follow-up--bind-the-live-occurrence-and-restore-the-authority)
   before phases 6b/6c expand it (identity and withdrawal implemented
   2026-11-21 for the body-contact producer; the restore projection is
   stated as snapshot-resident; same-tick status re-application closed
   2026-11-24: an ended status's claims are withdrawn at its end). Retiring a source cannot revoke a replacement
   at the same authored address; derived masks and winners rebuild from the
   restored claims before their next read. Then share one interval across
   native and numeric/capability contributions. (6b landed 2026-11-22: Mary-O's pocket
   quasar is one application, `PocketQuasar`, whose one interval feeds
   defense/contact through a held grant and music/shader by observation;
   retirement withdraws only its own grant. 6c's numeric slice too: an
   effect source is one application with one interval over any terms, and
   Speed Shoes are one grant.)
3. [Linking L1/L2](engine/capability-linking-and-prepared-facets.md#bounded-implementation-slices)
   removes the repeated equipped-repertoire interpretation, then binds a
   mixed mechanical candidate. L3's pure cutscene consumer is an independent
   dependency experiment; L4 measures each actual cut. BOSS-CHARACTER's
   common seed continues without waiting for these packets. (L1 first slice
   landed 2026-11-22: `scale_moveset` is the one all-or-nothing scale rule
   that `WornKit::of` and `moveset_in` share, and the spawn Grant asks
   `WornKit::of`, and finalization checks each scale by that rule. Open: the
   link-context binding and equipment/hand specialization.)
   The [family-selection refinement](engine/capability-linking-and-prepared-facets.md#select-attack-families-before-materializing-moves)
   preserves an untouched family's authored/scaled moves and retains the
   selected ranged executor, including explicit absence. Equal preset
   numbers do not erase replacement meaning; shared move IDs cannot make
   one family's replacement overwrite another. Keep direct attempts and
   the existing character-scale order. (Family selection landed
   2026-11-24: `effective_repertoire` selects each family by its source,
   the hand or a worn row's grant, and materializes only those; and the
   charge owner reads the hand, so an explicit empty slot no longer reads as
   native charge.)
   FORM-REBIND
   supplies L2's live form/generation customer and can be corrected before
   broader linking. (L2, 2026-11-25: body construction reads only the
   admitted cast; the mechanical-body tag is prepared, and no spawn road
   takes the App's `CharacterCatalog` for a body fact; a two-owner candidate
   publishes both changes at activation or neither; activation refuses a
   candidate whose sheet mechanics are no longer live.) The independent
   [authored-empty geometry slice](engine/combat-model.md#preserve-an-authored-empty-geometry-sample)
   carries Unauthored/Empty/Shape through the sprite/combat boundary.
   An empty selected strike sample contributes no strike or AI strike
   region; its window entry is completed work without repeated per-tick
   resolution. Keep capture and push coverage separate. Preserve CLIP3's
   time map and the current geometry sampling cut, and the existing pulse
   plan's memory ownership where an empty intermediate segment needs it.
4. **Done 2026-11-27.** [World composition W1/W2](engine/reusable-authored-world-composition.md#implementation-slices)
   uses existing local references, then a different mechanism. General format
   work follows only if the operation is shared. (W1 landed 2026-11-22: the Bridge Warden is made with its binding to the
   posts' local keys; no result indexing or follow-up attach. W2's native
   binding landed 2026-11-24: a `lift_called` lift waits at both stops and a
   `MoveLift` switch in its room sends it. `create_mechanism` landed the same
   day: one scripted request builds lifts and the switches that send them,
   bound by local key, with two copies in one room kept apart. Lift occurrences
   landed 2026-11-25: each lift is an object of the batch that a caller
   retires alone (schema 543). Door occurrences landed the same day: a
   scripted door stands in its own room's gate solids while shut, and a
   `ToggleDoor` switch of the same request opens it (schema 544). The
   second-instance witness is not applicable (Q188, ruled (a) 2026-11-27):
   in Ambition a seat entering an authored room joins its live instance.
   Authored room identity and live instance identity stay distinct, so a
   later game may instance a room; that waits for a gameplay customer. Rollback through one child's retirement is witnessed.) Producer-validity delegation
   follows the existing [asset owner](engine/asset-library-editor-and-delivery.md#dependency-evidence-has-three-meanings).
5. [Body anchors A1/A2](engine/body-anchor-bindings.md#delivery-packets-and-proof)
   replace private saddle/capture point conversion with two independently
   mapped endpoints, then bind sampled named points to their admitted rig.
   Keep mount, capture and custody laws separate. (A1 landed 2026-11-22:
   `root_placing` plus each owner's own map; the saddle's combined offset and
   capture's private conversion are gone. A2's capture slice landed too: a
   named hold point is
   bound by name, rebinds on a reordered rig, and ends the hold when the rig
   loses it. Pose provenance and the rollback witness landed 2026-11-24:
   the pose names its rig, and `BodyRig` requires its pose, so a body a
   restore builds again is solved again. The authored seat point waits for
   a rig that states one.)
   [REL1/REL2](engine/relationship-endpoints-and-transitions.md#rel1--capture-owns-admission-at-every-ingress)
   close native admission bypasses: capture reserves both endpoints;
   mount admission establishes common scope and an active pair, and accepts
   one end before publishing it. Preserve reset associations separately from
   active riding and delete adapter-only end deduplication. (Landed
   2026-11-23: `CaptureOccupancy` is the one capture law at both ingresses;
   `BoardCase::refusal` with `ExecutionScope` is the one board law;
   `ActiveRides` is the active pair; a ride ends once per tick and the
   adapter's deduplication is gone.)
   [REL3](engine/relationship-endpoints-and-transitions.md#rel3--prepare-a-transfer-cohort-and-native-relation-dispositions)
   prepares companions from each transfer's actual roots, including an NPC
   rider's mount and a possessed body's inactive home. Admit the required
   destination configuration and native relation disposition after final
   preflight. A split capture releases only on success. Prove a retained
   source with two seats, an NPC rider arrival without test-side mount
   placement, and a late refusal with no relation changes. Use the existing
   publication and direct NPC departure owners; keep unsupported dormant
   cohorts refused. A7 owns the item exit/handoff refinement below.
   (Direct roads landed 2026-11-24: direction and errand take an NPC
   rider's mount and limbs, stand the mount under the arrived rider, and end
   split rides and holds natively. Publication road and possession home
   landed the same day, with the late-refusal witness, and the captured
   body's automatic exit from a room another seat keeps. ISL-3a landed
   2026-11-25: the host scopes a crossing to its footprint, suspends
   lifecycle work during replays, retracts an undecided crossing a replay
   decides again, and retires history at one cut. See the
   [receipt](engine/island-rollback-and-compact-peer-input.md#isl-3a--retain-the-transfer-footprint-and-its-membership-cut).)
6. [Story SS7/SS8](engine/story-sequences.md#own-each-admitted-playback-before-binding-its-roles)
   first gives each admitted cutscene run an owner occurrence. A request
   follows that run's transitions; it cannot start from the presence of an
   earlier run or remain pending through a later one. Preserve waiting-only
   trigger coalescing, seen policy and independent room rewind. Bind roles
   and per-beat answers to that same occurrence, then settle required
   gameplay after skip, replacing the blanket skip-all-commands promise with
   domain settlement for the scene's actual required outcomes. Use Creator
   dialogue and intro choreography for roles and settlement. No universal
   sequencer. (Landed 2026-11-21: roles in conversations, the brute and
   doorway scenes; a skip keeps the cursor and its `WaitFor`s, and the brute
   scene waits on the bomb's impact, `actor.throw_landed`. A required
   result's own failure policy waits for a scene that needs one.)
   Complete the [conversation opening scope](engine/story-sequences.md#qualify-a-conversation-by-its-opening-scope):
   identical authored participants in different live room instances must
   not share a conversation identity, runner or end result. Preserve the
   existing ledger and independence from mutable input ownership.
   (Landed 2026-11-23: the instance id carries its opening live room.
   2026-11-24: one cutscene script plays in two live rooms at once, each
   run with its own roles and waits.)
   [O6a/O6b](engine/authored-gameplay-logic-and-orchestration.md#o6--bind-invocation-roles-through-the-existing-call-boundary)
   adds the existing merchant as an independent first context customer:
   wallet questions and purchase requests select the same payer policy.
   Preserve three verdicts through supported Yarn gameplay gates; an
   unanswered condition must not become an answered negative.
   (O6a landed 2026-11-23: the merchant's `can_afford`/`wallet_balance`
   ask the purse `buy_item` charges, by one payer rule. O6b landed the same
   day: gates with a negative arm ask `verdict(…)`, compare its word and
   refuse an unanswered question.)
7. [Interaction I1/I2](engine/interaction-opportunities-and-activation.md#bounded-delivery)
   moves availability/selection below the prompt and shares one attempt
   across competing contextual consumers. Preserve gesture provenance,
   eligible windows, domain acceptance and explicit read/consume cuts.
   (I1 landed 2026-11-22: one conversation decision for the prompt, the
   door and the opening; selection is gameplay's `InteractSelection`, which
   the prompt projects and the mount and portal consumers read. I2's first
   slice landed the same day: the talk/switch and chest executors and the
   door act on the selected target, revalidated at their own cuts. The
   mount followed: it boards or leaves only what is selected
   (`MountPresses`). The portal toggle followed on 2026-11-23: the
   generator answers each toggle, and only a toggle that turned it spends
   the press.)
8. [Native completion AC1/AC2](engine/native-action-completion.md) reuses the
   actual construction receipt for summon identity and initialization, and
   separates cancellation identity from pending-work capacity. Check the
   existing unsupported limbed-summon refusal before allocation. Delete ID
   prediction, next-tick existence polling and post-commit identity scans.
   These packets can ship independently of the earlier composition work.
   (AC2 landed 2026-11-22: a cancellation is a control request with its own
   identity and result and no pending slot, so a full caller can cancel.
   AC1 landed 2026-11-23: a summon request carries its reply, and the
   executor answers at its commit from the receipt. A limbed-host summon is
   refused in preflight before allocation. The ride adapter's counter
   prediction, `RideSummons` and its next-tick pass are gone.)
   [Combat CR1](engine/combat-reception-and-injury.md#cr1--finalize-one-native-hit-outcome)
   finalizes reception, connect and actual reaction from one captured
   operation, including damaging boss and lethal home-body hits. Then
   [CR2](engine/combat-reception-and-injury.md#cr2--route-limb-injury-without-an-hp-mailbox)
   replaces GNU's missing-HP transfer with one optional surface-to-recipient
   relation. Keep one injury per opening, explicit injury cap, distinct clash
   acceptance and the stated same-cut timing change. No general damage graph.
   (CR1 first slice landed 2026-11-22: the hit reporter owns connect and
   block; a damaging boss hit and a lethal home-body strike now connect once,
   with zero hitlag. The strike-origin follow-up landed 2026-11-23: a
   module's body strike credits the move use that emitted it, and early
   contact marks only that use.)
   The [strike-origin refinement](engine/combat-reception-and-injury.md#carry-attack-origin-through-strike-creation-and-contact)
   requires origin at native strike creation and carries it through early
   contact. Matching occurrence can mark/stale its move; independent or
   older contact cannot credit a replacement. Preserve the early overlap
   cut and the strike's independent lifetime. The wider ground-slam shock
   is the first module customer; keep the primary fists out of its proof.
   This can ship without ACT/CR2 or the remaining receiver finalizer.
   The [reaction-data refinement](engine/combat-reception-and-injury.md#bind-a-hits-reaction-to-its-accepted-reception)
   moves the four static hit-reaction choices into prepared native hit data.
   A contact callback must not install a status from an ignored strike or
   replace a different hit's launch. Preserve physical-contact effects and
   independently admitted status capabilities.
   (Landed 2026-11-23: `HitVolume.status` carries the choice to
   `apply_body_hit_reaction`, which both receivers call only for a hit they
   accepted; the four forwarding ports and `smash.*` modules are gone.)

9. [Body action ACT1/ACT2](engine/body-action-execution.md) makes current
   body permission reach every applicable start/spend cut, settles landing
   before replacement admission, and retires a move on its actual interrupting
   reaction. CR1's narrow actual-reaction follow-on supplies the result; CR2 is
   independent. Resolve the execution interval once through the current time
   owners so local hitlag/proper time reaches movement, move playback and
   physical pose sampling, with separate expiry clocks. Keep live sustain
   contributions distinct from cadence and flow progression. The same
   admission owner supplies a non-spending resolved-action offer to the
   brain; acceptance rechecks it and publishes actual move identity.
   An accepted ranged emission retains its
   [admission basis](engine/body-action-execution.md#a-committed-emission-retains-its-admission-basis)
   across a form/equipment change; a replacement source needs its own
   admission. Include authored brandish selection and preserve live aim.
   Remove the primary-body freeze shortcut that
   bypasses room policy. First customers are ordinary Ambition reaction and
   landing, plus Smash local hitlag. ACT3 floor choreography remains a later
   Smash customer; it is not part of this row's required completion.
   (ACT1 slices landed 2026-11-23: an interrupting reaction retires the
   victim's move use it captured at reception; armor, gusts, damage-only
   ticks, deaths and boss hits do not. Ordinary move starts ask the body's
   reaction and guard locks through one rule shared with the input gate,
   and a refused press keeps its buffer. A ranged use retains its selected
   source on its fire event (schema 534). Boss attack
   starts wait out the same lock. A brain's attack kit shares the trigger's
   affordability and fallback selection (`MoveAdmission::offer`), and its observed use
   names the started move (`OwnMoveFact.spec`). An attack acts out of a
   tumble only through its accepted move. The
   [ACT1 follow-through](engine/body-action-execution.md#act1-follow-through--resolve-the-request-before-spending)
   completes prospective-source admission/spend, request-versus-fallback
   cancel eligibility, and use facts carried by a final-step shot. (Landed
   2026-11-23: the landing settles in the trigger set before any start is
   admitted; a jump/burst escape ends the move only on the verb the body's
   step spent; a shot whose drawn weapon left the hand is interrupted before
   it can fire. ACT1's listed steps are done.) The 2026-11-23
   review follow-through landed: a ranged event carries its use's facts
   (`RangedUse`), each candidate's weapon gate and refire spend use its own
   `emission_source`, and the attack kit asks the cancel window about the
   request before the fallback.)
   (ACT2 slices 2026-11-23: the reaction cut resolves one action share per
   body per step before the timers decay (`BodyActionStep`, unit-correct
   `released_action_dt`). Travel on both roads, move timeline/charge/flow
   and the physical pose clock spend it, and stun thaws on it, so all four
   resume on the same step. A held move fires no event and runs no flow; its
   strike volume and hit memory stay. The primary-hitstop world-clock rung
   is gone; a world freeze is the room's impact policy. Later the same day
   the floor game (tumble, prone, getup) moved onto the action interval,
   with tech press/lockout left on input time, and a sustained effect's
   technique now spends its move's action interval (`trigger_over`), and a
   time-scale status is its own `ClockDilation` (schema 537) that travel
   and every action clock read, apart from the relativistic proper-time
   rate. The CPU-only and human-seat Smash acceptance landed 2026-11-27:
   `local_hitlag_holds_only_the_struck_bodies_with_cpus_and_with_a_human_seat`
   holds travel and move time through every whole-hitstop step, and
   releases only the remainder at expiry, in both compositions.)

10. [Input IN1/IN2](engine/participant-action-system.md#input-history-and-body-interpretation)
    defines one admitted input cut for recording and replay, then resolves
    body coordinates before a locomotion-only emergence constraint. Preserve
    held-direction continuity and each verb's policy. This is shared engine
    work from the source `34b4ab0b7f4f` audit; it does not promote Smash-only
    polish. IN1's first slice landed 2026-11-24: the recorder runs at
    `AdmittedSeatInput`, before reset and interpretation; streams are
    version 2; a recorded reset and a held portal transit replay exactly.
    The raw-row dual writes are removed (2026-11-24): an interpretation edits
    only this tick's input authority. IN1 is done. IN3 records the smaller qualified-press coalescing correction
    (done 2026-11-23: a tick's attack press is chosen whole from one sample).
    IN2's first slice landed 2026-11-23: the portal guards act on the driven
    body's resolved control, in its own frame; the warp keeps the accepted
    transit's orthogonal map. The
    [IN2 follow-through](engine/participant-action-system.md#in2-follow-through--retain-direction-meaning-through-the-guard)
    retains actual aim-source selection and keeps posture outside physical
    travel projection (landed 2026-11-23, schema 536:
    `ActorControlFrame::attack_axis_from_aim_stick`, and a walker's travel is
    only its side axis). The gravity-turn composition is proved: a held stick
    through a transit into turned gravity is mapped once (2026-11-23). IN2 is
    done.
    [IN4/IN5](engine/participant-action-system.md#input-continuity-across-routing-and-control-changes)
    refine the same owner at `00bf92c22142`. Observe valid physical samples
    before context filtering and treat rebind reset samples as invalid;
    reopening gameplay must not create analog presses. Carry current
    supply eligibility and a retained interruption through the admitted
    input cut, including capture/resume between ticks. Withdraw a pending
    fireball charge without firing at supply/driver loss, before accepting
    a later fresh edge. Keep committed body moves and ACT2 action holds
    distinct. [Same-body binding replacement](engine/participant-action-system.md#same-body-binding-replacement-is-a-local-handoff)
    also settles a pending charge whose release route is withdrawn. Keep
    unchanged bindings and the incoming action's activation policy. This
    local simulation change needs no new peer-wire representation. IN4
    can ship independently (its capture and rebind crossings landed
    2026-11-23: `SeatSourceEdges` follows valid samples and resyncs after an
    invalid one; the retired seat's raw row and latch go with it: IN4 is
    done). IN5's driver handoff landed 2026-11-24: a pending charge keeps
    the seat that began it and is withdrawn without a shot when the body's
    driver changes, or when its body's binding no longer routes the press
    to it (Eye Laser equipped, a weapon in hand). IN5's supply interruption
    (UI/focus capture) landed 2026-11-24: `ControlFrame::input_interrupted`
    rides the admitted seat frame (wire identity 10), a capture withdraws a
    pending charge without a shot, and a Down+Interact held through a capture
    does not release a possession. Do not select a new provider transport
    merely to complete P2's unused publisher.

11. [Observation OBS1](engine/bounded-perception-and-attention.md#obs1--make-attention-the-native-evidence-admission-boundary)
    landed 2026-11-23 (`6fe56fb12`): a `TacticalWorld` body admits,
    refreshes, acquires and reports exactly its attended set; the cheap
    road and omniscient target are unchanged. Done 2026-11-27: a
    `TargetBelief` body admits the same attended set (no view built), and
    memory keeps at most `WorldMemory::RETAINED_AT_MOST` (32) rows, living
    foes first.
    [OBS2](engine/world-facts-observations-and-memory.md#obs2--preserve-evidence-meaning-through-memory-and-communication)
    landed 2026-11-23 (`4488b2d65`, schema 531): memory carries observed
    liveness and evidence source/`SimTick` cut; a hit keeps its hostility
    beside a better sighting; only a preceding-step sight is reported; a
    report cannot name its listener. Settle timing, crew assistance and
    saved hostility are unchanged.
    Then [OBS3](engine/agentic-character-runtime.md#obs3--resolve-observation-demand-across-controller-providers)
    (first slice landed 2026-11-23: `ambition.actor.think_sighted` carries a
    sighted binding; its demand maintains the body's belief and the module
    reads that projection, never the live target; the
    [OBS3 follow-through](engine/agentic-character-runtime.md#obs3-follow-through--publish-availability-for-this-decision)
    makes its availability current when native decision skips a body;
    landed 2026-11-23: a per-step decision product withdrawn at Prepare and
    written only by the native decision, required by both think routes)
    resolves native plus module observation demand before one domain
    projection is built. Prove a selected sighted module with a native
    `None` fallback and an explicitly authoritative module as control.
    Keep today's world-rule ports and controller gate. Streaming top-K is
    an owner-local measured option after admission is correct; neither a
    new collection package nor a durable social-memory system is required.

12. [View ownership MV1](engine/multiplayer-and-multiview.md#mv1--resolve-the-selected-view-before-its-framing-and-continuity)
    resolves each view's requested subject/population before deriving
    framing, reference anchor and placement/cast continuity. A no-driver
    explicit spectator must not depend on an unrelated session default;
    another cast's spread or teleport must not change its frame. Bind
    user preferences explicitly so authored per-view policies survive
    Update. Keep default possession-follow behavior, single-view/cast
    receipts and portal continuity. Resolve only requested populations;
    do not repeat the full old cast prelude for every view. This can ship
    before broader M3 adaptation and the 2.5D adapter. M3 still owns the
    product choice of shared orientation and logical input reference when
    bodies disagree on gravity. (Preference binding landed 2026-11-23: a
    view's `ReferenceFrameSource` keeps an authored frame from the setting.
    Selected-subject framing landed the same day: each view selects its
    subject before framing, and placement history is kept per view. MV1 is
    done; M3's shared-orientation choice stays with M3.)

13. [Room scope SC1](engine/capability-and-runtime-composition.md#sc1--select-rule-subjects-inside-the-work-gate)
    keeps the coarse any-room work gate and makes Mary-O's gait writer
    select each subject through existing room access and the installer's
    scope. A foreign room does not inherit the walk throttle because its
    body lacks a run edge. Preserve `EveryRoom`, body-local control and
    native integration; no new registry or scoped-query interpreter.
    This caller repair can ship independently of input and view work.
    (Done 2026-11-23: the gait writer takes the installer's scope and selects
    each body by its live room; `RulesScopeSet` is described as a work gate.)

**Acceptance:** body permission governs the actual action spend; a refused
press has no accepted-action side effects; an applicable hitlag or held
reaction suspends move execution and movement on the same action interval;
authored world freeze remains
available through its room policy. A selected domain fact has one binder;
independent holds and poses retire separately; another room cannot act on a world cause that can
still disappear; a rejected mechanical candidate leaves the active selection
and pinned product versions intact; a narrow authoring consumer excludes its
native adapter. Two anchor spaces align without merging relation laws;
play requests follow only their admitted run; authored commands and waits
keep their selected participants; skipping does
not equate a queued request with its completed gameplay outcome; removing
the prompt does not change the same installed interaction domains' rules.
The merchant's offer and action use the same subject policy and item-domain
exchange law. A required unanswered question cannot authorize its negative
gameplay arm.
Mixed summons report the exact created body, and a full pending mailbox can
still cancel admitted work. A hit's native reception and move confirm agree
without inventing a physical reaction. GNU's fist impact retains its striker
and recipient policy without spending/healing fictitious limb HP or giving
two confirms for one strike.
The tactical cap bounds direct evidence admitted/refreshed, while legitimate
old memory may remain. An observed death disconfirms pursuit without revealing
an offscreen death. Report source/cut and subject survive delivery. A sighted
module receives the chosen belief projection without a live-position fallback;
explicit authoritative controllers keep their contract. Equal authored pairs
in separate rooms keep distinct conversation occurrences.
Capture/resume does not manufacture a physical press or lose an admitted
interruption; a fresh post-capture press survives cancellation of the old
charge, including prediction/correction. A committed move remains on its
body. Explicit view target, cast extent, continuity and preference binding
vary independently of an unrelated default subject.
Room work gates do not substitute for subject selection. A strike retains
origin from birth through overlap and reception without coupling its
lifetime to the launching move.
Equipment replacement preserves each retained attack family's meaning;
explicit empty geometry or a hand slot does not expose an inherited
fallback. A refused form request changes no native grant or beat, and an
accepted form rebinds intrinsic configuration under the same role terms
as construction while preserving the live body's state.
State which axis each packet proves. Code, runtime behavior
and compile-time performance remain separate evidence.

### EXECUTION-COST — specialize gameplay work at its actual inputs

**State:** PLANNED, 2026-11-21 efficiency follow-through. Preserve the product
order above. This selects bounded work; it reports no new runtime measurement.

**Failure / customer:** one projectile leg prepares equivalent source cells
twice; ordinary animation evaluates immutable clip facts during presentation;
addressed behavior repeats static joins and population scans; navigation
discards an arrival position that its own cost law still reads. The follow-up
at `f41218979625` also finds that graph probes turn a physical world size
into canonical geometry and lose material scale, while the two consumers
disagree about stance.
The further review of that source finds another invalidation error: one live
item's position change copies the global occurrence map and rebuilds dormant
recipe/save data that did not change.

**Owner:** [execution specialization and work selection](engine/execution-specialization-and-work-selection.md)
links E1–E6 to their existing domain owners. Portal implementation remains
under [PORTAL-SIMILARITY](#portal-similarity--canonical-parts-and-scaled-mirror-portals).

**Next action:** the duplicate projectile source preparation is removed
(2026-11-21: walls and recipients read one prepared leg). Clip whole-body
eligibility and per-track semantic roles are prepared with the flipbook
(PVS-1 first slice, 2026-11-22), and frame track correspondence is an
index (2026-11-24). The prepared rig and its part binding are retained
from admission (2026-11-24: `AdmittedBodyRig` per rig text, shared across
generations; the renderer rebinds only for a different rig or program). The
static source review adds a [named source-binding customer](engine/runtime-rigged-sprite-animation.md#bind-semantic-roles-to-named-source-rows):
boss roles resolve source rows, timing policy and mechanical geometry
association once. (First slice 2026-11-28: a boss sheet row names its
published row, and the drawn record row is bound once by name; six bosses
name every row, and a record that breaks a named relation is refused. The
other thirteen bosses, the session binding, retiming and the alias law
remain.) Preserve the native cursor, hurt-part sample, death
timing and visual projection under equal-capacity row reordering; keep
explicit retiming, independent rig/move sources and dense runtime handles.
The duplicate fact collectors are removed (one EventMailbox scan for all native
fact ports, 2026-11-22), and `run_phase` buckets its queue by trigger once.
Dispatch spans and entry lookup are prepared at admission (2026-11-24:
`Admitted::dispatch`, read by `run_phase` and the inbox). Call completion,
retirement and publication batches share one occurrence view each
(2026-11-24). Active journal work is derived (2026-11-24: orphans retire
only on an owner cut, and the collectors visit only active lanes). Navigation's cost-state
reduction is fixed (2026-11-22: route searches arrivals and charges the final
walk), and the advisor measures a body's motion once across rooms
(`NavMotion`, 2026-11-22). The graph cache pins the graphs demanded each
tick, and a surface's reach is searched once and kept (2026-11-22).
Seeded waypoint answers are kept by surface and seed (2026-11-24); the
navigation list here is done. The seat autopilot reuses its kept motion across
room graphs, and `next` reads a kept per-goal table over link landings
(anchor-lifted route state, fixed edge costs) instead of a forward search
per body per tick (both 2026-11-24). The newly selected
[scale law](engine/scale-relative-gameplay.md) also changes canonical
gravity response. E4 retains material and the complete resolved frame;
navigation must not create its own gravity multiplier or preserve the old
controls-only product policy.

The `00bf92c22142` [static geometry follow-up](engine/platformer-navigation-and-reachability.md#static-geometry-coverage--native-support-before-finite-preparation)
keeps native support/occupancy separate from finite navigation preparation.
Remove presentation-fill authority and shape-bound approximations at the
demonstrated consumers. Preserve ordinary box paths, bounded edge preparation
and the distinction between unsupported coverage and an exhausted admitted
route. Existing E4 receipts remain closed.

[E6 / IR1–IR4](engine/item-recipes-and-live-continuity.md) first retains the
physical recipe chosen at mint, then removes the live → save → live recipe
loop. Keep current pose in the body and capture domain placement at accepted
retirement/checkpoint/save cuts. Move every native reader before removing the
mirror. Count whole-map copies, baseline/hash work and export rows while one
item moves beside increasing dormant populations. The correctness packet IR1
can land independently of the later storage change. (IR1's extent slice landed 2026-11-22: a
minted item keeps the canonical extent its mint chose, through the save and
both reconstruction roads. IR1 is done 2026-11-24: a shipped regrown
coconut keeps its extent through the room and custody roads end to end, and
the retained/not-retained item fields are named on the owner page.
IR2 is done 2026-11-24: the item owner retains live and dormant recipes
natively, and room preparation, the checkpoint capture and boss retraction
read that record instead of the save codec. IR3 is done 2026-11-24: a moving
item no longer writes the occurrence ledger; its pose is captured at the
checkpoint commit, the operation admission and the room retirement. A moving
tick beside 10,000 dormant rows fell from 2.43 ms to 3.5 µs in the ledger
path. IR4 is done 2026-11-24: the file joins live poses at output, a
pose-only change is written at most once a second, a semantic change at
once, and a switch or a quit writes the outgoing cut.)

The follow-up [data-structure packets K1–K3](engine/data-structure-kernels.md#delivery-and-deletion-criteria)
make bounds queries and strict unique lookup independent optimization targets
in the existing foundations. The range grid is extracted for world and
contact consumers (K1, `ambition_geometry::spatial::RangeGrid`), and the
occurrence/binding ambiguity reduction is shared (K2,
`ambition_binding::cardinality`), both 2026-11-22. Next is K3: compare
direct scans with indexed implementations before selecting target/workload
policies. Preserve all domain lifetime and acceptance rules. (K3 measured
2026-11-24: a per-tick contact grid pays only above 256 pieces, and a call
batch indexes its bindings only from 16 calls; both select the scan below.)

**Acceptance:** ordinary operations select the cheapest supported local case;
adding unrelated portals, installed entries or dormant receivers does not add
exact work to that operation. Preparation preserves order, samples, scope and
refusals; cache availability cannot alter publication. Every claimed saving
names removed work and measures its relevant cost. Retain an offline/reference
comparison without running a shadow general evaluator on every gameplay query.

For K1–K3, interchangeable implementations agree on numeric policy, ordering,
duplicates, unknown/invalid input and snapshot lifetime. Measure setup and
consumption as well as queries; delete the private grid, unused contact
overlay wrapper and repeated ambiguity reducers as their callers migrate.

### BUILD-BOUNDARIES — share implementation after typed selection

**State:** CB1–CB4 cuts LANDED (2026-11-22 to 2026-11-25; the `sim` tools
link no renderer). CB4's build work, one sample each: `headless` builds cold in 455 s under
`sim` against 859 s under `game`, and an app edit rebuilds in 5.1 s against
8.6 s. Open: the conditional curved-search experiment.

**Failure / customer:** the game schema library normally imports the
aggregate CLI that consumes it; the tool package selects desktop/capture
dependencies for offline and headless commands; port and phase generics
carry type selection into substantial work that depends only on values.

**Owner:** [compiler boundaries and build products](engine/compiler-boundaries-and-build-products.md).
CB1 removes the schema-to-checker edge. CB2 converts typed port metadata and
Bevy systems before shared schedule installation. CB3 retains distinct
phase systems and their local state, while sharing projectile and status
reducer bodies. CB4 makes each command select its real dependencies, then
proves the reusable harness and game provider cuts separately.

**Next action:** CB1 is done (2026-11-22: the checker is its own package above
the schema library). CB2's typed entrypoints now hand plain values to one untyped installer
(2026-11-22). CB3's projectile and held-status customers share
one reducer body each across phases (2026-11-22). For CB4, the offline item report builds without the game (2026-11-22,
`--no-default-features`), and the harness no longer inherits the facade's
defaults (2026-11-25: its own graph lost the renderer, menus and LDtk, 626 →
549 crates; no current consumer's graph changes). `game` is now default-off
(2026-11-25). Every launch descriptor names it, and every workspace-wide
lane names `ambition_app_tools/game`, so no bin's tests are silently
skipped. The plain tools graph is 247 crates against 939
([slice](engine/compiler-boundaries-and-build-products.md#cb4--select-command-dependencies-and-the-harnesss-actual-services)).
The first harness-alone consumer landed 2026-11-25
(`fixtures/harness_alone`: it steps and observes a sim, and its own graph
of 372 crates links no renderer, window or product shell).
Persistence no longer asks Bevy for `bevy_window` (2026-11-25: the offline
report's graph 188 → 144 crates). The simulation family is split from
capture (2026-11-25): a `sim` feature that `game` includes, six stepping
commands on it, the app compiling at `rl_sim` alone; `sim` resolves 503
crates against `game`'s 665, and a headless trace is the same under both.
The provider's presentation cut landed (2026-11-25): `ambition_content`
selects its simulation capabilities and a `render` feature for its looks;
its own graph lost `ambition_render`, the menus and the inventory UI (451 →
440 crates), and the LDtk edge after it: the runtime's `ldtk` is the pure
area table and `ldtk_runtime` the `bevy_ecs_ldtk` spine, so content's graph
(408 crates) holds no `bevy_render` or `wgpu`. The app cut landed
2026-11-25: the app and the three demo providers select simulation
capabilities and put their looks under a `render` feature (the app's own
drawing code is gated on it); `sim` 503 → 496 with no `ambition_render`,
and `game` resolves the same crates and features. The `sim` cut
landed 2026-11-25: the LDtk spine, `bevy_dev_tools` and Avian moved under the
drawing features, so `sim` (496 → 443) links no `bevy_render`, `wgpu`,
`bevy_ecs_ldtk` or Avian, `game` resolves the same features, and a headless
trace is the same under both; `scripts/tests/test_sim_tools_have_no_renderer.py`
holds it. CB4's command, harness, provider and app cuts are all landed.
Do not move tool binaries back into app integration-test builds.

**Acceptance:** an independent schema consumer excludes the checker; the
selected offline/simulation command does not activate unrelated capture
features; shared implementations retain port codecs, phase identities,
local state, publication order and domain results. Record surviving paths
when a complete renderer cut is not yet achieved. Use existing compile and
monomorphization tools to distinguish source fanout, emitted code, link cost
and delivered bytes. A simpler source boundary alone is not a speed claim.

The curved-contact callback experiment remains conditional under portal P4.
Do not erase the ordinary collision path, replace domain codecs with a
generic value tree, or redesign rollback from old mono counts. Keep each
experiment only for an observed cost or a simpler boundary with no material
runtime regression; report which reason supports it.

### NET-AUTHORITY-EXPERIMENT — compare hosted-world replication with GGRS

**Customer:** persistent Ambition multiplayer, NPC lives, late join and different-room participation have different cost/authority needs than Smash rollback. The App-wide GGRS rewind (and the per-message link cost measured by NET-INPUT-WIDTH) may be unnecessary for the flagship's intended player-hosted experience.

**Owner:** [authority-model experiment](engine/hosted-world-networking-experiment.md) and [network topology](engine/network-topology-and-authority-research.md).

**Next action:** NA-1 and a first NA-2 are measured (2026-11-10, [results](engine/hosted-world-networking-experiment.md#na-1-and-na-2-2026-11-10)): each GGRS peer spends about 4× the host's update per frame even without rollbacks, and room-scoped replication bytes depend on delta encoding. Half of that is `SaveWorld`: 1,196 systems, about 16 ms per run whatever the body count. The 2026-11-10 direction (hybrid; rollback kept per room while cheap) makes this the hosted half of [SIMULATION-ISLANDS](#simulation-islands--room-level-rollback-inside-a-hybrid-world). NA-2b's first measurement ([result](engine/hosted-world-networking-experiment.md#na-2b-first-measurement-2026-11-10-a-guest-app-that-simulates-nothing)): a guest App with every simulation phase gated off costs about 4.5 ms an update against the host's 9–10, and applying 2–18 body rows costs about 0.1 ms. The gated guest's update split (2026-11-11): alone it costs about 2.2 ms, nearly all `Update`'s 1,030 gated systems, so a guest should be composed from fewer plugins rather than gated. Done 2026-11-11: shots spawned and ended on the host are built and removed on the guest by `SimId` from a spawn row per kind; its ids equal the host's after every snapshot, at about 1.3 link bytes a frame ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). A death on the host is a row change, not a despawn (the killed enemy's body stays), so the body row must carry the defeat; done 2026-11-11 ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)): health by `SimId` (2 bytes a body) puts the guest's enemy down one snapshot after the host's with no disagreement, but what draws a death is gated simulation state; done 2026-11-11 ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)): the `DeathBeat` by `SimId` (3 bytes a body) makes the guest draw the corpse as the host does (without it, never), though the kill's shake and hold stay on the host. Without the gated systems (2026-11-11, [result](engine/hosted-world-networking-experiment.md#na-2b-the-gated-guests-update-split-2026-11-11)): taking the simulation's 557 systems out of a booted guest saves 0.24–0.46 ms of about 2.1, and the rest is 335 composition and presentation systems (other games' demos, the conversation, the shell), so a cheap guest is a narrower composition, not a missing simulation. Done 2026-11-11: the kill as a presentation event: the host's camera shakes, forwarded with each snapshot, make the guest shake with it and the control never does, at 0.11 link bytes a frame ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Measured 2026-11-11: a guest of Ambition alone (`AMBITION_HUB_GAMES=ambition`) has 136 fewer systems and costs no measurably less (1.8–2.2 ms an update against 1.9–2.9, two runs each); its cost is Ambition's own conversation, shell and load presentation ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Measured 2026-11-11: what else a host spawns: in the kill run, only the shots (by `SimId`) and the enemy's bounty coin, which has no `SimId` (drops are anonymous by design), so a guest replicating by `SimId` never sees it ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Done 2026-11-12: a dropped pickup keyed by its `SpawnOrigin` (the parent's `SimId` and the sequence): the guest builds, draws and removes the bounty coin one snapshot after the host, with no difference in what a view draws, at 0.38 link bytes a frame, most of it the magnet's motion ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Measured 2026-11-12: a breaking wall is a row change on a `SimId` both Apps hold, its drop takes the pickup road, and a swing's short-lived strike volumes are the only other new ids (drawn from the body's move, so no row) ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Measured 2026-11-12: a summon is a body descended from its summoner's `SimId` and already in the body rows; its spawn row names its character ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Measured 2026-11-12: a snapshot of the guest's room only is 27 bytes a frame of body rows against 398 unscoped once the players part, and a body that leaves the guest's room is removed by the general rule ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Measured 2026-11-12: the host's sounds and visuals forwarded with the snapshot; only a visual that names a body (a swing's owner) needs translating, by `SimId`, and every one did ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Measured 2026-11-12: a gated guest does not follow its player's crossing (room changes run in the simulation it gates): it keeps the old room and builds the new room's bodies with no room ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Measured 2026-11-12: with a room row (the host's number and the definition) the guest records its player's crossing and its own room construction builds the room two frames later, if the room transition phase stays awake; an unmapped room's bodies are deferred, so only another player's body needs a spawn row ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). NA-2b is measured. NA-3 first measurement 2026-11-12: a guest predicting its own body is exact while no other body acts on it; with Alice swinging beside Bob the corrections average 1.45 units, 0.37 with offset reconciliation ([result](engine/hosted-world-networking-experiment.md#na-3-guest-prediction-first-measurement-2026-11-12)). Measured 2026-11-12: the corrections are Alice's strikes on Bob, and reconciling by replaying the guest's own room since the snapshot (ISL-2's room rewind on the guest) brings the mean to 0.068 units at about 9 ms a frame of replays in a debug build ([result](engine/hosted-world-networking-experiment.md#na-3-guest-prediction-first-measurement-2026-11-12)). Measured 2026-11-12: a late guest that adopts the host's save builds the room as the host has it (a broken wall broken) and the rows agree at once; without the save the wall stands for good ([result](engine/hosted-world-networking-experiment.md#na-3-a-late-join-2026-11-12)). Measured 2026-11-12: the save carries what an NPC life holds, not where it is in its plan, so a guest showing lives needs each life's step on the wire ([result](engine/hosted-world-networking-experiment.md#na-3-a-late-join-2026-11-12)). NA-4 recorded 2026-11-12 in [ADR 0036](../adr/0036-synchronization-is-an-optional-game-capability.md#per-game-synchronization-na-4-2026-11-12): Smash keeps GGRS; Ambition is hybrid as directed on 2026-11-10. Next: production code (with SIMULATION-ISLANDS): a guest App composed from fewer plugins, a transport, the room row and spawn rows, and each life's step as a row. Evaluated 2026-11-13 ([result](engine/hosted-world-networking-experiment.md#library-evaluation-2026-11-13-lightyear-and-bevy_replicon)): prototype on `bevy_replicon` 0.44 (the last line for Bevy 0.19) alone. `Signature` from the `SimId` is the identity, room visibility filters are the interest, and the room rewind stays the prediction; Lightyear's per-component rollback would be a third rewind. First slice done 2026-11-13 ([result](engine/hosted-world-networking-experiment.md#the-prototypes-first-slice-2026-11-13-body-rows-through-bevy_replicon)): crate `ambition_hosted_world`; through replicon's in-process link a gated guest holds the host's bodies exactly by `SimId` `Signature` (53.2 bytes a frame for two bodies). Done 2026-11-13 ([result](engine/hosted-world-networking-experiment.md#the-life-row-2026-11-13)): the life row (health and the death's beat): a host kill shows on the guest on every frame, with the beat it does not run. Done 2026-11-13 ([result](engine/hosted-world-networking-experiment.md#the-room-as-a-visibility-filter-2026-11-13)): the room as replicon's visibility filter (`RoomOf` on bodies and on the guest's client entity): after a crossing the guest holds exactly its room's bodies at 30.6 link bytes a frame against 66.4 unscoped. Done 2026-11-13 ([result](engine/hosted-world-networking-experiment.md#the-spawn-row-2026-11-13)): the spawn row: a body the host spawned mid-run is built on the guest through the staged-actor road into the replicated entity, with the host's `SimId`, and holds the host's rows. Done 2026-11-13 ([result](engine/hosted-world-networking-experiment.md#a-real-link-2026-11-13-renets-netcode-over-udp)): a real link: renet's netcode over UDP (`bevy_replicon_renet` 0.20, feature `renet`); on loopback the guest joins in 3 updates and holds the host's bodies after every frame, at about 66 bytes a frame. Done 2026-11-13 ([result](engine/hosted-world-networking-experiment.md#the-guests-input-2026-11-13)): the guest's input: a `GuestInput` client message drives the seat its `GuestSeat` names on the host; Bob driven only on the guest moves on the host exactly as when driven locally. Probed 2026-11-13: what a room replicates with no character is its ground items, authored placements a guest builds with the room. Done 2026-11-13 ([result](engine/hosted-world-networking-experiment.md#the-room-row-2026-11-13-a-guest-follows-its-players-crossing)): the room row: the guest follows its own player's crossing through its own room lifecycle, reports the room it holds, and only then is sent that room's bodies, which continue into its own (17 hub bodies with the host's rows, none twice); the filter never hides a guest's own player. Done 2026-11-13 ([result](engine/hosted-world-networking-experiment.md#a-players-spawn-row-2026-11-13)): a player's spawn row: a player's body back in a guest's view is built again through the guest's home-body recipe and held where the host has it. Done 2026-11-13 ([result](engine/hosted-world-networking-experiment.md#an-npc-lifes-plan-row-2026-11-13)): each NPC life as a plan row, written only when it changes: the gated guest holds the lab's two lives as the host has them after every snapshot, at 62 link bytes a frame for the room (1,160 when `Life` itself was replicated); the test helper's gate had not stopped lives (a Bevy late-condition drop), now fixed. Done 2026-11-13 ([result](engine/hosted-world-networking-experiment.md#a-guests-gate-and-seat-2026-11-13)): the guest's gate is production code (`put_the_simulation_to_sleep`), and a host with `SeatsGuests` seats each connecting guest and tells it its seat; composing a guest from fewer plugins waits, since the measurements say it buys at most 0.46 ms of 2.1 and the gate settles correctness. Done 2026-11-14 ([result](engine/hosted-world-networking-experiment.md#the-visible-app-as-host-or-guest-2026-11-14)): the visible app as host or guest (`hosted_world` feature, `--host-world PORT` / `--join-world ADDRESS`): on the fixed 60 Hz host, over UDP; a headless host and guest on loopback seat the guest at 1, put its simulation to sleep and hold the host's bodies. Done 2026-11-14: a sleeping guest's camera follows its own seat (`ControlledSubject` from its `LocalSeat`), and its HUD meters read the same subject. Done 2026-11-14: a guest that boots in another room is taken to the host's player's room before it has a body of its own, and its Start joins its seat on the host, builds that body on the guest and moves its camera there (`a_guest_that_boots_elsewhere_is_taken_to_the_host`). Done 2026-11-19: a guest that predicts its own body ([SIMULATION-ISLANDS](#simulation-islands--room-level-rollback-inside-a-hybrid-world), `--predict-own-body`). Next: a windowed play of two Apps (no display on the agent's VM: maintainer), with and without `--predict-own-body`.

**Acceptance:** runnable headless comparison, byte and CPU measurements, stable guest input/reconciliation, region-scoped replication, persistent host-owned NPC facts, no service dependency, and an explicit per-game synchronization decision. Do not remove GGRS or rewrite working multiplayer merely from prose.

**Prediction follow-up, source `34b4ab0b7f4f`:**
[HP1–HP5](engine/hosted-prediction-contract.md) defines the production guest's
native continuation and supported partial simulation. Start with explicit
guest world-commit permission and one body profile corrected at a coherent
cut; then admit moving-surface dependencies, dated control assignment and
native population support. Preserve the motion receipts above and the
sleeping guest. This refines the existing hosted lane, does not replace the
transport, and does not promote networking or Smash polish in product order.

### SIMULATION-ISLANDS — room-level rollback inside a hybrid world

**Customer:** the Ambition metroidvania's multiplayer: players in different rooms must not pay for, wait on or rewind each other's rooms. Maintainer direction 2026-11-10: move away from strict App-wide rollback toward a hybrid model; keep rollback within a room boundary while it is easy to maintain, and evaluate what keeping it at room level costs.

**Owner:** [simulation islands](engine/island-rollback-and-compact-peer-input.md#simulation-islands-independently-rewindable-active-areas); the hosted half is [NET-AUTHORITY-EXPERIMENT](#net-authority-experiment--compare-hosted-world-replication-with-ggrs).

**Done 2026-11-10 (ISL-1, census):** in a two-room scene every rollback entity but four has a room (22 in the hub, 2 in `switch_lab`; the four are the two room roots, the session root and a boss encounter). One `SaveWorld` run costs 16.1 ms over 1,201 systems for those 28 entities, so a history per room on the per-type mechanism would multiply that by the live rooms. Five resources couple rooms (impact hitstop and the clock scale, the projectile id sequence, the one conversation, the one cutscene) ([result](engine/island-rollback-and-compact-peer-input.md#isl-1-result-2026-11-10-what-has-a-room)).

**Done 2026-11-10 (ISL-2a, cost):** a room snapshot through the registered rows (`RoomSnapshotRows`) saves the hub (22 entities) in 1.35 ms and `switch_lab` (2) in 0.31 ms, against 16.3 ms for the App-wide `SaveWorld`; it holds no resources or checksum yet ([result](engine/island-rollback-and-compact-peer-input.md#isl-2a-result-2026-11-10-what-a-room-snapshot-costs)). Room-level rollback is affordable on a snapshot that follows the room's entities.

**Done 2026-11-10 (ISL-2b, first witness):** in a hosted world the hub rewinds to a saved frame and replays Alice's inputs alone, with Bob's room hidden (`Disabled`): the hub ends exactly as the first run left it and Bob's room is untouched; the control without hiding moves Bob ([result](engine/island-rollback-and-compact-peer-input.md#isl-2b-first-witness-2026-11-10-a-room-rewinds-alone)). The window spawns nothing and exercises no coupling resource.

**Done 2026-11-10 (ISL-1b, the freeze):** with several live rooms an impact hold or Player 1's hitlag no longer stops every room's clock; one live room keeps the freeze ([owner](engine/island-rollback-and-compact-peer-input.md#next-isl-2c-and-isl-1b)).

**Done 2026-11-10 (ISL-2c, shots):** a room restore despawns what was spawned after the save and re-creates what is gone (references remapped); in the rewind witness Alice's shots fired in the window are fired again by the replay and end where they did.

**Measured 2026-11-10 (ISL-2d):** room/tick undo removed duplicate grants and restored selected keyed world writes during room replay; the [owner receipts](engine/island-rollback-and-compact-peer-input.md#isl-2d-measured-2026-11-10-a-world-write-in-a-rewound-window) retain the individual witnesses. **Revised 2026-11-21:** these proofs do not cover another room acting on a provisional grant or flag. [Causal world commit](engine/causal-world-commit.md) replaces eager shared-store undo as the target cross-history contract. Keep correct local/private retraction while migrating each externally visible operation.

**Measured 2026-11-10 (ISL-2e):** a hosted world that predicts its guest and rewinds only his room on a late input ends where an on-time run ends, at 9.4–16.3 ms of host update a frame (the late-applying host: 8.4–9.9; each GGRS peer: 33–48) ([result](engine/island-rollback-and-compact-peer-input.md#isl-2e-measured-2026-11-10-what-room-level-rollback-costs-a-hosted-world)).

**Done 2026-11-14 ([result](engine/island-rollback-and-compact-peer-input.md#the-room-rollback-host-in-production-2026-11-14)):** the room rollback host is production code (`ambition_hosted_world::room_rollback`, `RewindsGuestRooms`, on in the visible app's `--host-world`): guest input stamped with the host tick, per-seat input history and prediction, a save of each guest-only room every tick, and on a late input that differs a rewind of that room alone (clock, channels, ISL-2d retraction, other rooms hidden) with a replay to now. In the witness, 26 rewinds over 150 frames of late input end where the on-time run ends, body for body. Done 2026-11-14: a rewind in a room the host's player shares (`a_guests_late_input_rewinds_a_room_the_host_shares`): the host seat replays what its devices gave each tick, and the impact freeze is saved with the clock; Alice fighting Bob ends exactly where the on-time run ends. Done 2026-11-14: a replay presents nothing (its sounds, visuals and shakes were presented when the ticks first ran; measured 21 sounds and 28 visuals against 14 and 16 on time before, equal after). Channel handling now preserves pending live work through a separate replay stream (2026-11-16, `74e86f475`). Done 2026-11-14: a guest's late crossing joins the live room (a replay publishes a boundary with its ticks unconfirmed, the eager crossing path honours it, and no room is rewound while a crossing is recorded or loading); before, it built a second hub. Done 2026-11-17 ([result](engine/island-rollback-and-compact-peer-input.md), the paragraph "Every writer in the rewinding schedule is classified"): every resource writer in the rewinding schedule is classified, with the census (`check_sim_schedule_resource_writers_are_room_classified.py`) at 0 and its ceiling at 0. On the way: gate phases, portal frames, hits, gestures, switch presses, flags, provocations and possession rewind with their room; checkpoint commits and the gate network are live-only; no replay spans a live checkpoint or lifecycle intent; and no room replays alone during a match. Measured 2026-11-18 ([result](engine/island-rollback-and-compact-peer-input.md#a-rewinds-cost-on-a-release-build-over-a-real-link-2026-11-18)): on a release build over UDP through a delaying relay, a rewind costs about 2–2.6 ms plus 1.7–2.4 ms a replayed tick; at a 100 ms round trip (about 7 ticks a rewind) a rewind frame costs 19–22 ms mean, over the 60 Hz budget, against 3.5–4.2 ms without one. Done 2026-11-18 ([result](engine/island-rollback-and-compact-peer-input.md#run-ahead-a-guests-input-arrives-before-its-tick-2026-11-18)): NA-3 run-ahead: the host reports how early each guest's inputs arrive (`InputMargin`) and the guest leads its stamps so they arrive a tick early; over the same link the rewinds fall from about 84 to 3 or 4 in 600 frames at every delay, with the lead settling at 3, 5 and 9 ticks for 0, 25 and 50 ms each way. Done 2026-11-18 ([result](engine/island-rollback-and-compact-peer-input.md#the-live-confirmed-frame-boundary-on-a-hosted-host-2026-11-18)): a host that rewinds guest rooms publishes the confirmed-frame boundary every live tick (a tick is confirmed once each guest's input stamped after it has arrived), so the confirmed-frame observers act only on ticks no late input can replay; no rewind in the late-input witnesses replays a confirmed tick. Done 2026-11-19 ([result](engine/hosted-world-networking-experiment.md#a-guest-that-predicts-its-own-body-2026-11-19)): a guest that predicts its own body in production (`PredictsOwnBody`, `--predict-own-body`). Its own tick is its input's stamp. It saves its room each tick, and when the host's row of its body (dated by `RowTick`) differs from its prediction for that tick, it restores the room and replays (the host's `replay_room`). With 3 frames each way Bob turns on the guest's own frame against 8–9 frames for a sleeping guest. While nothing touches him, 150 of 150 rows agree. Alice's strikes are corrected by replay (42 corrections), and the run ends equal to the host. **Done 2026-11-20:** the then-selected narrative couplings (one conversation and one cutscene per world), in the slices N1–N4 of the [narrative plan](engine/island-rollback-and-compact-peer-input.md#narrative-per-room-the-plan-2026-11-20): one live conversation per seat, logged and rewound per room, each on its own Yarn runner and drawn in its own seat's box; one cutscene per room. **Projectile ids, resolved by reading 2026-11-20:** a shot's identity is `SimId::spawned(owner, owner's SimIdCounter)` (`mint_spawned_sim_ids`), one stream per spawner. The global `ProjectileSeqCounter` is only the total order that minting and the projectile step sort by. A room replay mints the window's shots after every shot the save kept, in their original relative order, so the room's order and its shots' ids are those of the on-time run. ISL-2c's witness (Alice's shots in the rewound window end where they did) is the replay case. No per-room sequence is needed. Meanwhile: ISL-2c (the hit on an NPC is done: `a_rewound_room_hurts_its_enemy_again`; an NPC life's ledger writes are done 2026-11-11: the rows mirror the rewound life, and the bookings a room's bodies add are taken back by its rewind); the previously enumerated ISL-1b couplings have receipts, each with a witness that an event in one room leaves the other unchanged (2026-11-11: bullet time slows Player 1's room only; a room's impact hold and clock scale rewind with it on its root's `RoomPace`; a room's rewind puts back its conversation, `ConversationsInRooms`, and its cutscene, `CutscenesInRooms`; a room's cutscene holds only its own room's players and dates its flags in its room; a conversation's grants and sales are its seat's, paid from that body's wallet and dated in its room). Since N3 and N4, conversations run one per seat and cutscenes one per room. The earlier invocation-clock receipt is narrower than the native module completion below: a module entry's `dt` follows its scope's room (2026-11-11, `ScopePace`), and the match clock, banners, room transitions and passages and the gate network stay on the shared pace by ruling (each is the session's, presentation, or spans two rooms). The fight's own clocks (strike windows, cooldowns, fuses) and, since 2026-11-11, the body-local and room-local ones follow the room through `RoomFreeze` (`ambition_platformer2d_shared_tangle::lifecycle`): items, platforms, breakables, mounts, mana, pets, captures, hazards, death interludes, boss phases, encounter waves and scripts, trails, and a room's cutscene.

**Review fixes, 2026-11-16:** `74e86f475` preserves other rooms' saved history, message buffers and reader positions, and suspends foreign conversation and cutscene work. The six production host tests include two guest rooms with late inputs, pending heals, active or queued narrative work, and seat reuse. `dc98fbc71` separates input sample order from tick estimates, merges button edges within a tick, bounds timestamps, clears retired-seat history, and allocates seats from the session roster with retry. The [room replay contract](../systems/room-replay.md) defines state ownership.

**Narrative per room ([plan](engine/island-rollback-and-compact-peer-input.md#narrative-per-room-the-plan-2026-11-20)):** N1 (2026-11-20) keys live conversations by seat and body; N2 (2026-11-20) gives each live conversation its own Yarn runner on a shared variable store, its commands naming their own conversation. N3 (2026-11-20) draws a dialogue box per seat over its own view and lifts the refusal of a second seat's talk; N4 (2026-11-20) plays a cutscene per room, each on its own seat, pace, hold, rewind and view. The narrative plan is done.

**Acceptance:** per the owner's ISL-2–ISL-4 acceptance, with every step's cost measured against the App-wide baseline; Smash keeps its single rollback domain.

**Native module-clock completion, open at `f41218979625`:**
[ISL-1b](engine/island-rollback-and-compact-peer-input.md#isl-1b-completion--one-room-step-for-module-owned-work)
uses the module entity's room step for native age/lifetime, positive-step Tick
eligibility and pull integration. The host's `ScopePace` already covers the
script side. Use Sentry/Vortex through the actual two-room schedule, including
hold/resume and rewind. Preserve discrete zero-dt operations, one-room
slowdown and the existing session/presentation clock policies. This is a
bounded completion of the clock contract, not a new time framework.

**ISL-3 refinement, source `34b4ab0b7f4f`:** the
[history-membership cut](engine/island-rollback-and-compact-peer-input.md#isl-3-refinement--history-membership-and-scoped-transfer-cuts)
derives affected source/destination histories from the existing transfer
plan. Retain their confirmed barrier first while unrelated rooms keep
history and replay; keep foreign lifecycle work out of hidden-world replay.
Body/custody and input membership change at one publication cut. CW4's
scoped finality query remains a later optimization, not a prerequisite.
The owner also records a smaller late-input edge disposition correction.

### GNUTON-POLISH — the GNU-ton fight, presentable (Jon 2026-11-21)

**Request (Jon, 2026-11-21):** GNU2 comes in upside down; the stage is not
big enough for two giant gnus; GNU2 has a bar and the first gnu has none
("we should probably make both of the gnu invulnerable"); boss bars must not
overlap; arms on by default, also in the dead pose; the apple trees on real
floating platforms; the apples' fall and regrowth used in the fight. The
scaling-portal idea goes in a brainstorm.

**Done 2026-11-21 ([owner](game/boss-cinematics-gnu2-and-motion.md#gnu2s-finale-gnuton-gnu2-finale)):**
both arenas 1024 wider; GNU2's pair ends face each other, so the game's
Rotation convention carries it upright; GNU2 escorts GNU-ton
(`BossEncounterSpec::escort`: unhit, no bar, falls with him); bar panels a
fixed height and stacked apart; the giants die and stomp with their arms on;
a boss's body is its kin box and its frame is drawn from it (GNU2 stood 184
under the floor), with the cells 320 deeper; three apple trees on one-way
islands in `gnu_ton_shoulders`, each with three real apples that ripen,
fall and grow again. Ideas past this: `docs/brainstorms/2026-11-21-gnu-ton-fight-ideas.md`.

**Open:** with the giant placed at the hall's middle it walked at the player
from the first frame, and `gnu_ton_fight::a_gnu_ton_fist_hits_once_after_three_cuts_and_replays_its_scaled_launch`
and `both_gnu_ton_fists_damage_separate_bodies_through_their_paths_in_one_tick`
(GGRS sync test) failed with a checksum mismatch at frame 13; placed where a
player at the door is within `stride::FAR`, it stands and they pass.
Unmeasured: whether a giant walking in the first frames desyncs a rollback
(its walk, `MountWalk`) or only meets those witnesses' premise. Next: a
sync-test witness with the player far from the giant at the build.

### ABILITY-VARIANTS — finish the authored loadout through shared capability seams

**Failure:** the loadout plan still has placeholder variants and no authored
upgrade fact.

**Owner:** equipment preparation, body control slots, content modules and
character landmarks. Design: [ability loadout, open work](game/ability-loadout.md#open).

**Done 2026-11-16 (`c6c886449`, renderer `72bf2dc`):** the laser keeps its
lit visor while crouched and starts at the authored `Eye` landmark in its
firing row. Its lease renews without clearing hit memory. The app regression
covers firing, crouching, and standing again. See the
[loadout receipt](game/ability-loadout.md#open).

**Done 2026-11-14:** the one authoritative upgrade fact: an ability's level
is the bag's count, its rows per level are `items.ron`'s `levels`, and the
worn row follows the count ([loadout, open 3](game/ability-loadout.md#open)).

**Done 2026-11-14:** a worn row withholds verbs of the kit through one
ceiling contribution, and Precision Blink has two levels: the quick blink,
then the whole blink ([loadout, open 3](game/ability-loadout.md#open)).

**Done 2026-11-17:** Precision Blink has its three levels: the aim's time
slow is a verb of its own (`blink_slow`), level 2 withholds it and level 3
is the whole blink ([loadout, open 3](game/ability-loadout.md#open)).

**Done 2026-11-18:** Contextual Blink, levels 1 and 2: a module on the
Blink slot puts the body behind the nearest foe, facing it. A technique's
transit is held on the body and made in the next tick's carry, so every
reader of the travelled path sees it ([loadout, open 1](game/ability-loadout.md#open)).

**Done 2026-11-20:** Contextual Blink, level 3: a second press soon after
the first slows the world while the slot is held, the stick picks the foe,
and letting go blinks behind it. The slow is a technique request
(`ambition.body.observer_slow`), held on the body as rollback state
([loadout, open 1](game/ability-loadout.md#open)).

**Next action:** the laser's own sound waits for an SFX bank republish.
Then Precision Blink's one press-hold-release (Q179, ruled 2026-11-27;
[loadout, open 2](game/ability-loadout.md#open)): a minimum preparation
before the short blink, tuned apart from the hold threshold, with
preparation, aiming, interruption and release in rollback state and a
visible preparation tried in play; no double tap. Then the
stance placeholders through the same technique/equipment seam, not named
engine branches. Dash II and Dash III were built on 2026-11-26 as dash styles
lent by their rows (`EquipmentRow.lends`); see
[the dash styles](game/ability-loadout.md#the-dash-styles-built-2026-11-26).

**Acceptance:** each real variant changes its declared slot on any supported
body. Replacing or removing equipment restores the prepared intrinsic kit.
A placeholder stays visibly unavailable until it has an admitted implementation.
Tiers, equipment and checkpoint/save state agree. Keep quality selection out
of gameplay authority.

### ASSET-AUTHORING — edit owned sources and prepare the meaning once

**State:** PLANNED (source review, 2026-11-21). Baseline `3ed50e318392`.
This row selects bounded authoring work. It does not reorder the maintainer's
intro/network priorities or reopen completed publication/tooling campaigns.

**Owners:** [asset authoring and prepared media](engine/asset-authoring-and-prepared-media.md),
[authoring and tools A5.2](engine/authoring-and-tools.md#a52---carry-context-and-execution-results-across-clients),
and [LDtk L4.1](engine/ldtk-authoring-and-world-tools.md#l41--prepare-the-complete-edit-before-writing-source).

**Evidence:** sprite rebuilds have different rules for keeping GUI edits;
motion-library compatibility is tested through a raster conversion; game
music re-authors producer timing/product facts; SFX backends interpret one
layer-duration field differently. Served preflight loses selected pack/tool
context, repair drops partial outcome information, and map commands can
write before repair/validation succeeds. The owner pages name source paths
and distinguish direct findings from unmeasured effects.

| Packet | First action | Acceptance and removal |
| --- | --- | --- |
| A5.2 | Carry explicit context through CLI/HTTP preflight, then compose sprite/scenery receipts | Same-name rooms in two packs resolve correctly; later failure retains earlier outcomes; remove default-profile reconstruction and inferred `generated` results |
| L4.1 | Finalize an entity move and a level clone before writing | Reviewed candidate is committed; refusal writes nothing; tracked source links survive; delete post-commit repair/validation |
| SA1 | Edit one existing motion-bound character through its source; then separate robot source blocks | A rebuild preserves the edit; inspect does not overwrite source; remove writes to disposable projections and selected merge-back logic |
| SA2 | Bind motion units explicitly for Director/Director Boss | A uniform coordinate change preserves frame-space trajectories and measurements; replace raster-factor equality as compatibility proof |
| MA1 | Export music playback facts and link Hobford, then Dinosaur | One authored clock drives transitions; old audio retains old timing; remove duplicate game timing/paths and the migrated registry parser |
| FX1 | Prepare common layer timing for synth/noise/Faust | Local control spans agree; intentional cropping stays explicit; remove backend duration precedence; review audible corrections |

**Execution:** A5.2 context plumbing and L4.1 can begin independently of media
schema work. SA1/SA2 share the sprite authoring owner and should coordinate
their binding edits. MA1 and FX1 stay in separate audio owners. For each
packet, implement its first customer and remove the displaced interpretation
before expanding to more content. Preserve existing source formats where
they remain the right owner.

**Music refinement, 2026-11-22:** MA1 still owns compiler clock and product
facts. [MUS1–MUS4](engine/music-programs-and-playback.md#implementation-cuts-and-deletion-conditions)
own occurrence selection, accepted playback, transition execution, and
strict cue recipes. The reported Wolfenstein handoff does not need to wait
for a full clock/export migration. Keep Hobford/Dinosaur as MA1's first
customers; add Galvanic/Tritone at the native handoff.

**Acceptance:** use the owner page's bounded proof, including cheap
source-only inspection. Do not substitute a new catalog, full-library render,
render-speed claim, or corpus migration for that proof. No media was rendered
and no cost saving was measured during this planning review. Music's current
Hobford lengths agree; tempo-map support is an expressive gap, not a newly
reproduced playback failure. FX1 may deliberately correct the old cropped
ramps and must label that sonic change.

### LEVEL-TRUTH — authored level data in the map submodule, one owner per room

Jon, 2026-11-04: "we should avoid putting the specs for levels and dimensions
in the main repo... I also want to avoid the issue of having more than one
source of truth for level data... we should likely be putting the authored
content into that submodule."

**Done:** a generator no longer overwrites a room edited in LDtk
(`<world>.generated.json`, `level ownership status|adopt|release`,
[region composition](game/world-composition.md#contents-one-owner-per-room)).
The History generator moved to
`game/ambition_map_assets/ambition_content/generators/history/`, and its
area specs are no longer committed.

**Failure:** every other generator (`tools/ambition_ldtk_tools/specs/*/make_specs.py`:
art, castle, eureka, evil moor, magical, ninja village, pirate port,
primeval, gate network and the rest) and its committed area specs are still
in the main repo, about 2.5 MB with 238 commits of history. Their worlds have
no record yet, so the guard is inert for them until their next apply.

**Survey (2026-11-10):** `level ownership survey <spec>... --ldtk <world>`
applies a generator's specs to a copy of its world and says, per room,
whether the live room is still what they make (`reproduced`) or was changed
after (`DIFFERS`). History's recorded generator reproduces its rooms (12 of
12 sampled): the control. It replays `area create` only, not a generator's
in-place steps after it (painting, `entity add`, `scenery_overlay.py`), so a
DIFFERS may be those steps. Treat it as LDtk-owned: that keeps the room as it
is, and costs the generator only a room it might have rebuilt.

**Jungle done (2026-11-10):** all twelve `dinosaur.ldtk` rooms are LDtk-owned
(`dinosaur.generated.json`). The six trail rooms and three of the six later
ones DIFFER from their specs, as the primeval generator's own note said (the
rooms were tuned in LDtk after it). Its scripts (`make_specs.py`,
`side_exits.py`, `treetops.py`, `tar_pits.py`) were deleted; the last version
is at `a910d3952e`.

**First survey of the rest (bare specs):** art 12 of 15 reproduced; castle
Godwin 3 of 6; castle kitchens 3 of 5; the breakable wall lab 0 of 2. The
loose top-level specs and edits are one-offs, but some are still read:
`hall_of_characters_area.ron` (`generate_hall_of_characters.py`),
`symmetry_room_area.ron` (`gen_symmetry_room.py`) and
`intro_raid_corridor_area.ron` (`test_spec_brains_resolve.py`).

**Every room has an owner (2026-11-10):** `generator_survey <generator>`
runs a whole generator (its `--apply`, with its committed specs and without
the ownership records) on the live worlds, fingerprints every room before and
after, and puts the worlds back from git; `--settle` records the rooms it
left as they were and adopts the rest. Run over the 14 room-making
generators: 103 rooms reproduced and are recorded as generated; every room
that DIFFERS (edited after its generator), that no generator made (the
sandbox's 80, intro, noodle) or whose generator cannot run (hall of biomes:
the sandbox already fails validation at `scroll_lab_door`, a LoadingZone with
no target) is LDtk-owned. `level ownership status` names an owner for every
room of every world but two History rooms, HAND-EDITED by e7ad541's
per-zone loading-zone presentation after History generated them (the guard
refuses to overwrite them).

**Generators moved (2026-11-10):** the 14 room-making generators live in
`game/ambition_map_assets/ambition_content/generators/<generator>/` beside
their worlds, as History's does; their area specs go to a temp folder and are
no longer committed; `district_kit`, `district_scenery` and the gate network
(`gates.py`, `make_gates.py`, which writes the three districts'
`gate_interface.yaml` into their generators' folders) stay in tools as shared
code. Each moved generator writes the same specs as before it moved (compared
with the lenient art box this VM needs: its art publication is missing, so a
generator here cannot `--apply`).

**One-offs deleted (2026-11-10):** 111 hand-written one-off specs and edits
(`*_area.ron`, `*.yaml`, `gnu_ton/`, `pirate_sky/`) are gone; every room they
made is LDtk-owned, and the code and docs that cited one for a number now
name the room. The dog example moved to `specs/examples/`.

**What stays in tools (2026-11-10):** besides shared code
(`district_kit`, `district_scenery`, `scenery_overlay`, `check_flat_map`,
the gate network) and the tool's examples: the hall of characters' spec and
order, written by the package's catalog-driven `generate_hall_of_characters.py`
(its room reproduces and is recorded as generated, so the hall can be made
again when the catalog grows; its test checks the committed spec is current);
`intro_raid_corridor_area.ron`, the fixture of `test_spec_brains_resolve.py`;
and the portal labs' specs (the portal peer's). The symmetry room DIFFERS from
its spec and is LDtk-owned; `gen_symmetry_room.py` and its spec were deleted
(last at `83b5cdfa78`).

**Next action:** none required by the acceptance below beyond the portal
peer's specs; a generator's next `--apply` on a machine with the art
publication is the check that each moved generator still makes its rooms.

**Acceptance:** `tools/ambition_ldtk_tools/specs/` holds only shared code and
checks; `level ownership status` names an owner for every room of every world.

### POSE-CONTINUITY — one visual transition path after the completed base poses

**State:** foundational Smash art/wiring and the recorded continuity slices
are built. CLIP1–CLIP3 address the remaining request/clock/binding
crossings with immediate Ambition and Smash customers.

**Owner:** move preparation/playback for source timing, sprite-sheet animation
and rig presentation for the selected picture, with renderer-authored
visual bindings. Design: [expressive poses, Phase 3](demos/smash-expressive-fighter-poses.md#phase-3---improve-continuity-and-body-weight-polish).

**Next action:** [CLIP1/CLIP2](demos/smash-expressive-fighter-poses.md#clip1--carry-the-beat-through-the-existing-clip-request):
carry occurrence, semantic beat, owner time and authored loop rule through
complete candidates in the existing clip request; align pose-modifier
clocks; bind existing landing and ceiling-tech art. In parallel,
[CLIP3](demos/smash-expressive-fighter-poses.md#clip3--preserve-source-motion-under-ruleset-retiming)
retains a prepared map from selected move time to the authored source beat.
Start with Scout's Ambition landing/recoil and the Admiral's reaction and
retimed forward smash. ACT3 owns the later full tech/getup episode. Do not
repeat completed rigs/blends or require a roster-wide art pass.

**Retained receipts:** the measurement is done (2026-11-14, Scout:
[Phase 3 table](demos/smash-expressive-fighter-poses.md#phase-3---improve-continuity-and-body-weight-polish),
`scripts/measure/pose_transitions.py`): 6 of 34 transitions pop at their
end, all out of the airborne `fall` pose or the ledge climb, and 7 more only
when interrupted. The blend landed the same day: a change between two
carriage poses slides from the pose last drawn over 0.07 s on the shared
rigged path; strikes, move clips and the hard landing stay sharp. Contacts
were measured the same day: natural clip ends already plant the feet (0 to
2 px), a foot glides only on an interrupt, and a ledge catch's hands slide
24 to 31 px. **The visual rig landed 2026-11-18:** Scout publishes
`player_robot_v3_body_rig.ron` (side rows and their mirrors; no hurt parts,
gameplay off as for every rig), which names each part track's joint
(`part_bindings`); `PosedParts::bind_named` places every named draw of the
2028 shared frames from the rig's pose (41707 exactly, 35 half a turn away
on the same raster). **Planted hands landed the same day:** a pose names
the limb ends it plants (`CharacterAnim::planted_contacts`: the ledge
grab's two hands); while a change into it blends, those hands stay where it
draws them and each arm is solved from its blended shoulder (the rig's
`limbs`, `PosedParts::hold`), so a ledge catch no longer slides the hands
24 to 31 px across the ledge. **Follow-through landed the same day:** a
landing, a brake or a blow swings the arms (about the shoulders, the
forearm lagging) and nods or snaps the head for 0.18 s, from and back to
the authored pose; a strike or a clip cancels it
([Phase 3](demos/smash-expressive-fighter-poses.md#phase-3---improve-continuity-and-body-weight-polish)).
**Campaign gaps measured 2026-11-19**
([table](demos/smash-expressive-fighter-poses.md#roster-rollout-jon-2026-10-31-everyone-really),
`scripts/measure/campaign_sheet_gaps.py`): of 510 sheets no Smash fighter
draws, none draws the state poses. 329 had no row named `hit`, so a struck
body on them was drawn `idle`, though most spell that pose `hurt`. Fixed the
same day: a chain name binds the sheet's other spelling of its pose
(`first_bound_row`), and 492 of 504 now draw the struck pose. Done 2026-11-19:
the townsfolk target draws the motion library's reaction anchors (renderer
`f84ac65`; 240 sheets republished part-first; each state's own pose is now
missing from 264 campaign sheets, not about 504), and the replay audit
forgives 96 levels, measured on the two resampling differences 19 of those
sheets refused on. Of the 12 hand-made rigs that drew `idle` struck, the
Creator has `hurt`, `death` and `death_idle` and the four dogs share a
`hurt` (renderer `89a8d71`, `37868ab`, 2026-11-19). The other seven
(burning_flying_shark, full_stack_giant, genghis_can_v2, genghis_cant_v2,
player_robot_fable, smirking_behemoth_boss, void_star) draw a `hurt` row in
their own idiom, and the shark and both Genghis brothers a death (renderer
`5c8f05d`, 2026-11-20). Left: watch the swing amplitudes in the game at 60
and 144 Hz.

**Acceptance:** human and CPU bodies share the path; contact timing and sharp
attack poses remain readable. Check interruptions, rollback resets, quality
tiers and portals at game scale. Preserve Mary-O's classic home-game tall
poses. Visual rig work does not enable gameplay rig geometry.

### LEVEL-1 — the lab raid and the first level after it

**Owner:** the level-1 rooms in `intro.ldtk` and their generator
(`game/ambition_map_assets/ambition_content/generators/intro/make_specs.py`),
`intro.yarn`, `data/cutscenes/intro.ron`, the intro quests and flag chains,
and the intro cast rows. Design: [`game/level-1.md`](game/level-1.md). Jon
2026-11-14: redo everything in the first level after the lab raid, and make
the raid work; a level 1 that meets some of the cast, makes the player learn
the basic mechanics through the place, and starts real challenges. One jump,
melee only; pogo at most an optional extra. Fia is not named in it.

This row took over INTRO-RAID (2026-11-12). Kept from it: one creator (he
no longer follows the robot: in Hobford he stays at his bench, and his
`follows_closely` brain was deleted when nothing named it); a playing room cutscene holds every body in its room
(`ControlHold::Cutscene`); the `Fall` beat; Oiler's `meets_the_player`
brain and `oiler_notices` volume; the message board's fight rows.

**Measured 2026-11-14 (the old raid, a temporary probe walking right from
the wake room):** the creator stopped at the head of the 128 px stair from
the lab into the corridor (x 1025) and never reached the fight; his last
words played and his `Fall` ran about 1400 px behind the player, off screen.

**Rebuilt 2026-11-14 as Hobford** (the brief: a rural region newly on the
gate network, the robot named Scout, the raid as an interruption, Alice the
first friendly character, three directions in any order). Eight rooms by
the generator: the workshop (`intro_wake_room`), Hobford Lane, the high
meadow, the gate clearing with a new ring (`hobford_ring`, TILLAGE), the
freight siding (`intro_freight_yard`), the millstream, Hobford and the
woods. The raid starts at a line of the Creator's conversation
(`encounter.start`) and the raiders join it (`conversation.join`); it also
starts at the barn door. Removed: the basement lab, the service stair,
`intro_escape_shaft`, `alice_relay`, `bob_relay`, the gate stack's
checkpoint. The Drain Market and the culvert stay as town rooms. Decisions
D1-D21 in the design page.

**Walked, the far half (2026-11-17):** `a_walked_route_of_the_persistent_world::the_player_walks_from_the_woods_to_the_siding_and_wins_its_fight`
goes from the woods up the hill path, lifts the barrier by its lever, goes
down the freight track, wins the yard fight (both waves, about 1230 frames,
no death) and opens the goblins' crate for the notes, all by the player's
own inputs. Found on the way and fixed: the raid's runaway scene counted
fallen raiders in every room, so a raid whose last two fell together left it
armed for the siding, where it held the yard fight on a line of the absent
Creator (`actor.fallen_in`, the count in one named room).

**Next action:** the art
(terrain, props, villagers, publication; the three backgrounds are drawn);
the residents (the gatekeeper, the miller, the innkeeper and the trader landed 2026-11-14, Hobford row 9); tune the waves by playing them. The sandbox hub remains the new-game start (D3, Jon, 2026-11-17); the workshop is reached from the hub.

**LEVEL-1-KIT (done 2026-11-14 for verbs):** each level-1 room withholds the
kit past run, one jump, drop through and the strike (the level field
`withheld_verbs`, written into a player's body each tick as the ceiling
contribution `room.withheld_verbs`; `AbilitySet::verb_mut` names each verb).
Witness `level_one_withholds_the_large_kit`, with the hub as its control.
Found on the way: the service stair's flight A rose the wrong
way (a 144 px wall), its west girder left a body's height exactly over
flight B, and the keyboard walker let Jump up between two launch frames,
which cut every jump to a 30 px hop.
**Shots (done 2026-11-14):** `ranged` is a permission every kit has,
withheld by level 1's rooms: no projectile slot, and the charge stream
cleared at the control gate ([level 1](game/level-1.md)). Schema 470.

**MULTI-PARTY-DIALOGUE (done 2026-11-14):** a conversation holds any
number of bodies: the opening pair and whoever joined
(`conversation.join` / `conversation.leave`, or a hostile within earshot
breaking into a node with `<node>__interrupted` authored). Every member is
held and leaves on its own (struck, dead or gone, out of reach); the
conversation ends when the initiator leaves or fewer than two are left.
Membership is rollback state. Design and limits:
`docs/planning/engine/dialogue-continuity.md`. Witness
`a_third_character_joins_a_conversation_and_leaves_it_by_the_rules` (the coil lab) and the
conversation crate's unit tests. **Next action:** the rural opening's raid
(`creator_intro__interrupted`) is its first customer.

**Acceptance:** the level's rooms are crossed by the walked route with the
player's own inputs; every required step is one jump; the creator dies
beside the robot; Alice's reading reaches Bob and opens the survey line;
the freight yard's fight returns the notebooks and opens the shortcut home.

### FINAL-BOSS — the bossofthegame on the shoulders of a giant

**Owner:** the final-boss subagent (Jon, 2026-11-12: "Put a dedicated
subagent on this"); design page `game/final-boss.md`. Request (Jon,
2026-11-12): the bossofthegame fight mirrors GNU-ton's, in that he stands on
the shoulders of giants. The giant is a square robot with an electric guitar.
It is the final boss of the game, so it is grand. A gate address goes directly
to the boss dimension. Brainstorm notes: `docs/brainstorms/original_brainstorm.md`
(the 4th-wall "vibe kill", typed characters falling on a climbing player, an
agent carrying the Boss on its shoulders) and
`docs/brainstorms/good_llm_ideas/creator_agents_and_endgame.md` (the Boss
acquiesces rather than dies; "until the day he dies, or retires").

**Design:** [`game/final-boss.md`](game/final-boss.md) (2026-11-12): the
Boss rides **Full Stack**, a giant amp-stack robot with a guitar; the
dimension `boss` (`boss.ldtk`: lobby, main stage, green room); the hidden
gate address **LEGACY**; phases "Make No Mistakes", "Manual Override", "The
Encore"; the end is his retirement, not his death. Ruled 2026-11-26: Full
Stack is its own agent (Q175), and an empty resolve bar leads to a final
noncombat interaction in which he chooses to retire, not a kill-equivalent
end (Q176).

**Next action (Q176):** read the green room's scene against the ruling and,
where the end still plays as a defeat, make the empty bar lead into a
noncombat interaction he ends by retiring. Acceptance: a headless run that
empties the bar reaches that interaction, with no death beat or kill
flag on the Boss.

**Built 2026-11-12:** the dimension (`world.boss_ldtk`, map
`generators/boss_stage/make_specs.py`): `boss_lobby` with the ring
`boss_lobby_gate` (LEGACY, hidden), `boss_main_stage` (risers and truss,
the green-room grille on `boss.cleared boss_stage.bossofthegame`),
`boss_green_room`.

*The fight (built 2026-11-12):* the art (renderer targets
`bossofthegame_boss`, `full_stack_giant`, `boss_keycaps`), the catalog rows
`npc_full_stack` (a `giant` mount: its top and its guitar are ground) and
`npc_bossofthegame` (his voice), the encounter `bossofthegame` (60 resolve,
three phases, `corpse: Stay`: he sits down), the module
`ambition_content_modules::bossofthegame` (power chord, riff, feedback,
keystroke rain, backspace, subagents), the cutscenes `boss_vibe_kill` (the
stage's entry) and `boss_retirement` (the green room), the music fallback
(GNU-ton's adaptive cue) and the hall's practice copy
`hall_bossofthegame_stage`. Witness: `the_final_boss_stands_on_full_stack`.
The module's conductor draws Full Stack in its `strum`, `riff` and
`feedback` rows on those moves, its `idle` strum between.

*Built 2026-11-13:* the stage's camera zone (zoom 2.6, about 790 units
high: floor, risers, Full Stack and the Boss on one screen); the Boss is the
Director with a keyboard (Jon: "the director already existed"); the
dimension's own biome `boss_stage` (stadium backdrop, crowd foreground,
stage skin, decor, door, confetti); Full Stack's 1960s robot head and
straight strings; the three named agents Geppetto (called as phase 2 opens),
Clause and Wren (as the encore opens), drawn from the concept sheets on the
character generator, so each has the eight standard angles and the fighter
rows (Wren in a hood with a wrench; Q175 updated); the band (design page):
a guitar, bass, keytar and drumsticks every agent can play (`band_*` rows),
and a drum kit prop. The encore's Agent Swarm subagents, refused until then
as an unbuilt character, now drop in.

*Built 2026-11-13:* the band in the fight. As the show opens the Boss's
conductor summons Geppetto on the bass (west truss), Wren on the rhythm
guitar and Clause at the drum kit (east truss), each drawn in the
instrument's row. A call sends the called agent's band body away (a new boss
request, `ambition.boss.dismiss_summon`) as the fighter drops in. Witness:
`his_named_agents_join_the_fight` (rows at the open; Geppetto off at phase
2; the others off at the encore). A capture of the practice stage shows the
three at their places.

*Built 2026-11-13:* the Boss's microphone. It is in his far hand (the pen
stays in the near one), with four rows at it: `mic_sing`, `mic_point`,
`mic_call` and `mic_encore` (renderer b63728c). It also fixed a fairness
finding. Every tell of his but the rain and the backspace was `spike_halo`,
so nothing on screen told six attacks apart (`boss_fight_validator`; his
eight moves were also in no seed). Now each tells by its own row: the
chord by the pen raised, the riff by singing, the feedback by the mic
pointed out, the subagents by `talk`, Geppetto's call by the mic held up,
and the encore by both arms up. The moves are classified in
`boss_seeds.ron`, with three bands widened to what they measure. The
validator's pin holds at 3 errors and 5 warnings, the other bosses'.
Full Stack has two more band rows: `solo` while he holds his beat between
phases, and `outro` once he is down. Jon (2026-11-13): the stage camera
was far too wide, so its zoom is now 1.4 (about 430 units high), down from
2.6. The Boss holds no pen: it is the Director's sword in Smash (renderer
302b54c, map 8743626).

*Played through headless 2026-11-13* (a temporary probe on
`boss_main_stage`, deleted). The intro plays with one confirm per line; the
band stands at its places, and Full Stack plays idle, riff and strum. The
real kill clears him, and Full Stack plays `outro`. Through the grille, the
retirement plays at the green room's trigger. Two things read badly and are
fixed. First, the green room was empty while the Boss spoke his
retirement: he now stands on its dais, as `npc_bossofthegame` himself
(2026-11-15, Jon: one body, one actor, one path; the second catalog row
`npc_bossofthegame_retired` is deleted, and retired is the world's flag). Second, the one-way door to the lobby
stood 88 units from the east wall, so its "ONE WAY: NO RETURN" warning ran
off the room; it now stands 260 in. Full Stack is not in the green room
(he is 480 high and the room is 640), so his line there is a voice.

**Next action:** play it in a window and tune; his own anthem when a music registry is
at hand; the later-work list in the design page.

**Acceptance:** a ring dialed by its address reaches the boss dimension; the
fight runs with the Boss riding the guitar robot, in phases, with its intro,
music cue and art; the ending follows the retirement rule.

### CONTENT-AS-DATA — content still written as Rust

**Owner:** [fast iteration implementation](engine/fast-iteration-implementation.md)
and the owning game pack. The major migration is built: game text, item and
rules tables, stages, world manifests, props, published art references, demo
module specials, and ordinary pack reload now have authored sources. Keep
engine-owned defaults and code-derived structure in Rust; do not move them
only to increase a census count.

**Remaining executable slice:** on a *confirmed* game-authored value still
read once at startup, give its schema a `Restater` or move the reader to use
time. First reproduce the edit/reload gap; change one schema at a time and
delete any duplicate Rust value. The [packet catalog](engine/fast-iteration-implementation.md)
owns candidate ports and checks; recheck the current source rather than using
the 2026-10-16 declaration count as a new inventory.

**Launcher words restated (2026-11-14):** Ambition's launcher row and its
preparation's name (`data/ambition_text.ron`: `launcher.label`,
`launcher.description`, `launcher.preparing`) were read once, when the
provider registered, and kept their start words until the next start; the
reload table said so (`publish_nothing_the_selection_carries`). The text
family now restates them from the candidate at the activation
(`publish_launcher_words`, through `ShellExperienceRegistry::restate_words`
and `ShellRouteCatalog::restate_preparation_label`); the launcher draws its
rows from the registry, so the edited words are the next frame's. Witness:
`a_candidate_that_changes_only_the_launcher_words_restates_the_row` (edit
the file, reload, activate: about 15 s, with no Cargo build of the content).

**Boss art keys restated (2026-11-19):** `data/boss_art_keys.ron` (which sheet
draws each boss, which rows each move claims) is part of the boss catalog
fragment, but it was not a participating domain, so saving it refused the
whole reload (`RefusedUnsupportedChangedDomain`) and the running game kept the
old rows until a restart. It is now in `BOSS_DOMAINS` (`BOSS_ART_KEYS_SCHEMA`),
staged with the candidate catalog like `boss_profiles`. Witness:
`a_boss_art_key_saved_while_the_game_runs_is_played` (mockingbird's snap row
`chomp` → `dive`; failed with no reload before the change). Edit-to-play: 22
frames after the save; the test runs in about 35 s with no Cargo build of the
content. Still refused in Ambition's pack: `world_manifest`, which the LDtk
world reload owns.

**Starting character restated (2026-11-20):** `ambition_rules.starting_character`
reached the candidate catalog on a reload, but the provider's authored default
(`PlatformerAuthoredCatalogRegistry`), which `prepare_platformer_content`
resolves the next session's character from, was read once at provider build.
The next session therefore started as the old row. `publish_ambition_tuning`
now restates it at activation (`restate_starting_character`). Witness:
`a_candidate_that_changes_the_starting_character_restates_the_providers_default`
(`player_robot_v3` → `player_robot_v2`). It failed with the old row before the
change, and it runs in about 33 s with no Cargo build of the content.

**World manifest stays a restart (decided 2026-11-21):** `data/worlds.ron`
is still refused by a running reload (`RefusedUnsupportedChangedDomain`, which
names `world_manifest`), and that is the intended answer. In a dev build the
pack is compiled from the files at startup (`pack::compile_pack` reads them),
so an edit to the world list needs a restart, not a Cargo build. That is the
line Jon drew on 2026-10-17: restart is fine, recompile is not. The worlds
themselves reload live through the LDtk world reload, which owns them. A live
world-list edit would also have to rebuild the room set from a different file
set, with no customer asking for it. Reopen only for such a customer.

**Deferred:** game-specific movement VFX overrides have an agreed optional
`room_rules` policy but no game currently authors a different look. Promote
only for such a customer; do not copy the engine's defaults into every pack.

**Acceptance:** changing the authoritative data reaches the next applicable
session without a Cargo rebuild; a malformed value names the source and field;
no duplicate Rust reader remains. Report edit-to-test time for the slice.

### TEST-LANES — keep required test lanes executable

**Owner:** [test consolidation action plan](test-suite-consolidation-action-plan-2026-10-26.md)
and [test corpus evidence](test-corpus-static-audit-findings-2026-10-26.md).

**Selected next action:** keep the lanes green. Steps 1 to 5 of the action
plan are done, so what is left is the reds the lanes find; each is recorded
below with its cause. Steps 1 to 4:
- step 1, the Smash setup is shared (2026-11-14);
- step 2, the Versus setup is consolidated, its waits named by the fact each
  observes (2026-11-21);
- step 3, the first diagnostic queue is classified (2026-11-21);
- step 4, the repeated inputs of six modules are shared, with no test merged
  (2026-11-21, [record](test-suite-consolidation-action-plan-2026-10-26.md#step-4-consolidate-repeated-inputs-not-the-behaviors)).

Step 5 is read in full. `two_players_two_live_rooms.rs` (71 tests),
`a_room_rewinds_alone.rs` (18 witnesses, 14 hand-run controls) and the three
reload modules (27 tests) gave 0 removals ([record](test-suite-consolidation-action-plan-2026-10-26.md#step-5-record-two_players_two_live_roomsrs-2026-11-21)).
The hosted-Smash half, `smash_in_the_host.rs`, went from 79 to 73 tests
(2026-11-28, [record](test-suite-consolidation-action-plan-2026-10-26.md#step-5-record-smash_in_the_hostrs-2026-11-28)).
Four tests that made the same call as a sibling were merged into it, and two
print-only diagnostics whose questions are asserted elsewhere were deleted.
No assertion was lost, and seven hosted boots are saved per run. Use
current `AGENTS.md` verification policy: no test-count/source-count floors or
new meta-test machinery as a proxy for behavior.

**Ridge vine red (found 2026-11-27, inherited; fixed 2026-11-27):**
`the_tar_pits::a_vine_climbs_from_the_ridge_road_to_the_canopy`. The ridge
data was not the cause: the room from when the vine was added (map
`874cbc2`) failed the same way. A probe of the body passed: pressed at the
apex, the double jump rises 145 px from under the one-way ledge (112 px up)
and lands on it. A trace of the key walker showed the defect.
- `follow_leg` pressed the air jump at the apex and moved to `SecondAir` on
  the same frame.
- The key walker must let a held jump key up to make an edge, so the press
  reached the body two frames later.
- Meanwhile the still-falling body, below the landing, met the early-miss
  check, and the leg failed with its air jump unspent.

A brain writes its control frame directly and saw no delay. The follower
now has `LegPhase::AirJump`: pressed and not yet lifted. It keeps the press
and judges the second arc only once the body rises. The phase is checksum
state only (`SnapshotCursor` writes `phase as u8`), so no codec layout
changed. The vine test passes (lands at frame 139, where it failed at 137).

**Music reds (found 2026-11-27, inherited; fixed 2026-11-28):** both
fixtures were stale, and both cues were rendered and current.
- `shell_host_rendered::a_providers_own_frontend_route_plays_the_score_written_for_it`
  read the playing track as soon as the route settled. A probe at that
  moment showed the intent was already right: provider `smash`, the select
  track, in the library. The track reached the transport 0.2 s
  (wall clock) later, once its media had loaded. Until then the host's theme
  plays, which is the intended handoff. The test now waits with its own
  `settle_base_music` after each route change.
- `two_players_two_live_rooms::a_boss_in_one_of_two_live_rooms_fights_and_drops_its_chest_in_its_own_room`
  moved the harness-spawned boss into #1 but left its `Encounter` in no
  room. Since `6d100f9d9`, a boss's music is published only through the
  in-flight encounter attempt in the boss's own room. A probe showed the
  boss awake (Intro) and the claim missing, so the fixture now stamps both,
  as a room build does. The one-room control passed throughout.
- The bank cue test went green once the missing `sfx` content family was
  generated (`generate_library_content.py sfx --only-missing`).

**Close-up flipbook census red (found 2026-11-28, inherited; fixed
2026-11-28):** `character::rigged::tests::every_published_flipbook_realizes_its_sheet_at_every_tier`
named `director_closeup` and `player_robot_v3_closeup`: "no published
sheet". The `startup_character_parts` target publishes only their part
flipbooks, and the vanity card realizes them with no sheet
(`RiggedSpriteAsset::baked`), as intended since `8c3d3a4b4`. The census
assumed every flipbook ships beside a sheet. It now requires one only for a
flipbook that bakes a clip. Both close-ups still pass the census's tier
checks. On this VM, `mother_lode` and `startup_character_parts` had to be
rendered first (`scripts/regen/sprites.sh --target <t> --only-missing`).

**Hosted-Smash and resource-census reds (red since 2026-11-26, recorded in
[the binding transition](engine/contextual-binding-tables-transition.md#reds-after-the-transition-measured-2026-11-26);
fixed 2026-11-28):** each was the test's own reading, not the game.
- `smash_in_the_host::on_the_smash_pad_y_starts_the_fighters_authored_grab`
  and `quitting_a_smash_match_gives_the_pad_back` read the first
  participant of an unordered query. A probe showed the pad's seat binding
  Grab to `Button(North)` and the keyboard's seat binding only `KeyS`. Both
  now read the seat that binds a button.
- `an_unattended_smash_profile_enters_a_live_all_cpu_round` and its
  four-CPU twin wrote `GoTo(smash_select)` before the first update. A probe
  showed the select route activated and then replaced in the same frame by
  the shell's own `Initialize`, which boots the launcher. The shipped
  profile boots to the route instead (`AMBITION_BOOT_ROUTE`,
  `compose_ambition_shell_host_booting_to`), and the test now does too.
- `every_component_in_a_hosted_match_is_registered_derived_or_waived`
  found `WorldSprite` on an impact visual. The effect carries the live-room
  stamp `InRoomInstance`, which the vocabulary names, and only the renderers
  read `WorldSprite`; it is waived as presentation.
- `rollback_coverage`'s two resource sweeps found `GateWordsResource`,
  `GateDirectoryResource`, `GateAddressRule` (GATE-SPLIT) and `CrushPolicy`.
  Each is authored, written at composition and, for the gate words and
  directory, restated only at a reload's publication boundary. They are
  waived beside `GateNetworkCatalog`.

**Barrows content reds (found 2026-11-28, inherited from `550a7183d`; fixed
2026-11-28):**
- `intro::route_state::tests::the_intro_world_authors_the_flag_that_opens_each_gated_wall`
  pinned three gated walls; the Barrows authored ten more, each with its
  key, switch, encounter or boss. The pin now lists all thirteen.
- `prop_sheets::tests::the_prop_sheets_are_the_packs_rows` was
  environmental: 35 prop targets (the Barrows and others) were not
  rendered on this VM (`scripts/regen/sprites.sh --target ... --only-missing`).
`ambition_content --lib`: 555 passed.

**Trailhead arch red (found 2026-11-20, inherited; fixed 2026-11-20):** `the_gate_network::the_jungles_arches_chain_along_the_trail`. The press measured the door from its centre point, and the arch is 146 px tall. Its centre stood 49 px above a traveller in the opening, so Ranger Brandvold won the press from 33 px away. He had rallied to a kiwi's fight at the arch (x 921, not his authored 744), and the arch never fired. A door is now measured from its long axis (`ambition_platformer2d_world::rooms::door_distance`). One function serves the press, the prompt, the reach observer and the door's own transition (`rooms/systems.rs`). That transition had its own centre-point copy, and changing only the other three made both sides refuse the press. The arch witness passes. Of the gate/route/door/interact/talk app_it filters, 68 pass, and the only other red is the one below.

**Autoplay LEGAL red (found 2026-11-20, inherited; fixed 2026-11-20):** `a_route_over_rooms::autoplay_uses_the_gate_network_and_returns_from_the_primeval_trail`. The arch was not its cause. Probed at each room change: the return reached `primeval_trailhead` with the outbound wormhole (hub → `primeval_trailhead_gate`) still open, with 3.7 s of its 30 s idle timeout left. Autoplay walked home through it, arriving by a Gate passage at `hub_public_gate_zone`, and so never dialed LEGAL. The arch chain (JL-2, 2026-11-18) made the trip that short. The behaviour is right and the test's premise was stale. It now asserts the meaning: GENESIS dialed, and home through the hub's gate, by a LEGAL dial or by a wormhole still open. Passes (271 s).

**Ridden-mount crossing red (found 2026-11-20, inherited; fixed 2026-11-21):** `carried_item_crosses_rooms::the_whole_attachment_closure_is_recorded_as_being_in_custody`. Probed after each crossing: after the first door the possessed GNU-ton rider was no longer riding (`RidingOn` gone, mount alive). At the second door the mount stayed in `gnu_ton_approach` and was despawned with that room, which left 0 occurrences. The cause was MOUNT-RIDER-CUSTOMER slice 1. The rider holds `CanPilot { giant }`, so its Interact at the door also wrote `DismountRequested { RiderBailed }`: one press both left and bailed. Now the interaction owner decides which target the press is for, and the mount crate obeys. `update_nearest_interactable` orders the press as body/switch/chest, then for a rider the door it stands in (`InteractVariant::Use`), then Dismount. For a body on foot it is the mount (Ride), then the door: the door a body has just arrived by does not take the press, and a mount crosses with its rider. `claim_interact_presses` fills `ambition_mount::InteractPressesClaimed` (a derived resource) before `MountsCommandeered`, and `commandeer_mounts` neither boards nor bails for a claimed body.

**Portal-projectile reds (found 2026-11-21, inherited: red at `0960526fe` with the interact-claims change stashed):** `projectile_portal_transit::a_projectile_retains_its_pose_when_a_recipients_partition_is_refused` (pose 1000 against the 980 it should keep) and `a_spinning_exit_carries_a_projectile_into_a_hurt_body_between_matching_poses` ("the far-side wall must stop the shot"). **Fixed 2026-11-21:** `101f6dd87` made a shot reach only recipients in its own session (`CollisionWorld::same_session` compares `SessionScopedEntity`), and both fixtures hand-spawned their recipients with no session scope, so every shot passed through them. The fixtures now copy the owner's session scope onto the target, remote target, ally and hurt body; real bodies were always session-scoped, so the rule stands. `projectile_portal_transit` (and held): 23 passed.

**Catalog reds (found and fixed 2026-11-21):**
- `registered_character_art_resolves`: `projectile_polygon` and its void twin bind `grab_air` (the tether lash). The runtime resolves an airborne grab through `move_for_flat_verb`, which prefers `grab_air`, but the preparation vocabulary had never heard of it. `runtime_verb_vocabulary` now includes it.
- `the_full_hall_validates_with_every_hub_provider_catalog`: 97 catalog rows wrote `hall_dialogue_id` inside `barks`, where `CharacterBarks` dropped it silently. The bard's pedestal named a node its row did not. Every one now sits at row level, and `CharacterBarks` is `deny_unknown_fields`.
- Behind that red, four exhibits had nothing to say: the three band rows and the practice bag. Each has hall lines now.
- Still red on this VM only: the two music checks (no music registry).

**Dev gravity lost under rollback (Q136; fixed 2026-11-21):** the `\` hotkey
and the developer menu's Gravity row wrote `AmbientGravityRequest` straight
into its channel from `Update`. A speculative frame spent it, and the rewind
never raised it again. Both now write it as a host intent, and the applier
runs after the ledger's release. The tracking arm in
`a_bag_changed_mid_window_reaches_the_save.rs` is now a gate; all 14 of the
file's tests pass.

**Velocity-authority policy red (found 2026-11-20, inherited; fixed 2026-11-20):** `engine.velocity-writes-are-authority-only` named four bare `vel = ZERO` writes: the crumple's collapse into its knockdown (`ambition_characters/src/actor/body_step.rs`) and the D72 burial, crumple and seize (`ambition_combat/src/extension/reaction.rs`). All four drop a hit's knockback because a reaction that holds the body in place replaced it, so they share one named kernel authority, `ambition_platformer2d_core::movement::pin_body_in_place`. Its doc names the limit: it is an `AxisSwept` body's velocity, and a surface-momentum body must be pinned through its model. `cargo test -p ambition_workspace_policy`: 35 passed. The reaction tests (`ambition_combat` 26, `ambition_characters` knockdown 1) pass.

**Workspace policy reds (found 2026-11-19, inherited):**
`cargo test -p ambition_workspace_policy` was red in two scopes.

The game scope is repaired. `ambition_app` gained `ambition_hosted_world`
(the `hosted_world` feature) and `ambition_content_schemas` (the shipped
validator) without its allowlist, and both are now listed with their reasons.

The engine scope is repaired too (2026-11-19, 35 of 35 policies pass). Each
boundary was decided at its owner:
- `ambition_abilities` → `ambition_persistence`, allowed: the whereabouts
  supplier reads the save's death record, as the encounter, boss and item
  owners already do;
- `ambition_portal2d` → `ambition_geometry_observation`, allowed: the provider
  registry portal openings register with, as the world owner's geometry does;
- `ambition_platformer2d_runtime` → `ambition_camera_presentation`, allowed:
  the engine group installs the camera request appliers so headless and
  windowed runs apply the same requests (P6 step 5);
- `ambition_abilities/src/group.rs` requires `Res<CharacterCatalog>`, and its
  install runs it only where a catalog exists (2026-11-20): the shipped modules
  bind a group even in a host that omits the catalog
  (`composes_through_the_sdk::a_host_that_omits_cutscenes_still_builds_and_steps`);
- `observed_geometry.rs` no longer names `ControlledSubject`: the default
  subject is `ambition_sim_view::geometry_inspection::default_geometry_subject`;
- `hit_feel.rs`'s attacker recoil is waived as the guard deflection is (the
  knockback road when the body has flight state, set otherwise);
- the hosted world's `BodyRow` moved to `body_row.rs`, waived in the pose and
  velocity rules: a guest adopts the host's row for a body its sleeping
  simulation does not move.

**Sanic red (found 2026-11-19, inherited; fixed 2026-11-19):**
`sanic_it`'s `act_two_routes::the_ledge_spring_throws_him_onto_the_sky_island`
failed at 3ff8d4e39, before the badnik module, ending at (13616, 456) with no
monitor spent. Probed: the script let go of the stick once thrown, and the
unsteered arc came down with his box 2 px short of the monitor (x 13648
against 13650) while his feet were in the stomp band; the next tick he met
the box's side, which an unrolled body bumps off, and a body that lands
beside the box walks through it. The rule is right; the script was a matter
of pixels. He now steers toward the monitor and back over it, as a player
does, and still needs the spring to reach the island and a stomp on the lid.

**Inherited reds found with the GNU2 slice (2026-11-19):** each failed the
same way with the slice stashed.
- `app_it`'s `gnu_ton_in_a_match::each_ground_strike_of_gnu_ton_hits_a_short_foe_beside_him`:
  "the match of special_patent_clerk and sandbag_infinite did not start", as
  every live-round Smash arm here (below). Open.
- Fixed 2026-11-19: `ambition_boss_encounter`'s giant record test asked for
  an alpha-trimmed page, but the giant mount has published page-less since
  PART-FIRST (2026-11-18): its record says `drawn_from_parts` with whole-frame
  rects. It now asserts that (`giant_gnu_baked_record_drives_the_drawn_parts`).
- Fixed 2026-11-19: `check_session_authority_none_arms.py` was red at 12
  sites, one stale row (the projectile writer's read moved from `write_from`
  to its `SystemParam`) and eleven optional reads with no row. Each now has a
  row read from its code: nine `reduced-composition`, two `refuses`.
- Fixed 2026-11-19: `ambition_sprite_sheet`'s
  `a_flipbook_states_the_road_it_is_drawn_by` required a published baked
  flipbook. Since PART-FIRST the cost rule realizes none baked (older bundles
  still hold `realize: baked`), so its census asks only for the parts road.
  The baked road is still read back from a record.
- Open, the same with and without the binding change (`app_it -- pose`):
  `projectile_portal_transit`'s
  `a_projectile_retains_its_pose_when_a_recipients_partition_is_refused` (980
  against 1000) and `a_spinning_exit_carries_a_projectile_into_a_hurt_body_between_matching_poses`
  ("the far-side wall must stop the shot"), both in the projectile replay
  work upstream is landing; `every_composed_character_can_say_at_least_one_line`
  (`npc_clause_band` and four others have no line);
  `hit_he_is_drawn_in_the_pose_his_hurt_parts_have` (no T-rex is drawn from
  parts within its boot cap). The rest are this VM's music registry and the Smash
  match that never starts.

**Shell lifecycle reds (found 2026-11-19, inherited; fixed 2026-11-19):** two of
`shell_host_lifecycle`'s witnesses failed before the snake-shell slice, the
same way with and without it:
- `each_experience_begins_with_its_own_bag`: the Ambition session's bag was
  25 and the witness said 21, a count written on 2026-11-14. The bag was
  right: the starter set (`OwnedItems::starter`, every real non-key item)
  gained Precision Blink's third level, Lightning, the Lantern and Contextual
  Blink. The witness now counts the starter set from the catalog.
- `a_session_prepared_while_another_experience_plays_is_built_from_its_own_save`:
  the playing session's save gained `room_visited_sanic_speedway` and a day
  clock over the 240 frames, which Sanic writes by playing. The refusal arm
  now compares against the same Sanic session playing the same frames
  alone, not against its save before them.
They are listed as witnesses in
[construction and reconstitution](engine/construction-and-reconstitution.md).
Two more fail here for the VM's missing music registry ("authored music is
selected").

**Smash fixture shared (2026-11-14):** `common::smash` holds the Smash
setup `smash_ride` repeated in each test: `shipped_host` (the shipped
composition, 30 frames in), `wait_for_live_round` (a cast exists and none of
it is held), `live_round(roster)` and `seat(app, n)`. Ten copies of the
admiral setup, three of the live wait and twelve seat lookups in
`smash_ride.rs` now call it (3274 → about 2770 lines); each arm keeps its own
premise and assertions, and the three print-only probes that waited without
asserting now refuse a round that never went live. On this VM every
live-round arm of `smash_ride` fails at that premise, before and after
(checked on the unedited file: `the_summoned_shark_carries_no_contact_hazard`
fails at the same "opening ceremony never released the cast"): the VM has no
music registry and the round waits for its entry music. Next candidates:
`smash_reaction_poses::seated_as` and `smash_cpus_damage_each_other`'s
seating, which wait on different conditions and are left as they are.

**Measured 2026-11-10 (boot cost):** in a sample of 152 `app_it` tests, the mean
was 29 s and ten took 100 s or more. A harness boot costs about 11 s, and most
tests boot at least once. Two pure parses ran again on every boot in one
process:
- the sprite sheet table (2.4 s), which now prepares against the process's last
  table (`content::libraries`);
- the LDtk world (about 0.75 s, twice per boot), whose parse is now kept per
  path, size and file time (`LdtkProject::load_from_path`).

Boots after the first now cost 7.0–7.7 s (`boot_budget::probe_what_one_boot_costs`).
What remains is not one hot spot:
- character preparation: 0.9 s over 212 characters;
- sheet text reads: 0.75 s;
- the asset catalog: 0.5 s;
- Yarn: 0.4 s;
- the first update: 0.9 s;
- the settle: 1.6 s.

**Red without the boot change (2026-11-10, this VM; each also fails at the
commit before it):**
- `boot_budget::no_system_is_registered_twice_in_one_schedule`: two copies of
  `ambition_held_items::project_item_body_presence` in `GgrsSchedule`, a
  composition defect.
- `an_edit_reaches_the_shipped_game::a_reload_that_moves_a_tracks_file_plays_the_new_file`
  and `developer_edits_under_rollback::god_blink_in_ambition_does_not_grant_blink_to_maryo`:
  no rendered music on this VM; the Mary-O route never activates a body.
- `ambition_platformer2d` lib, two tests in `game_assets::sheets::tests`: no
  published art on this VM.

**Red source guards (2026-11-10, measured on main):** five checks were red on
other agents' drift. Repaired here: `is_test_path` did not know a file its
parent declares `#[cfg(test)] mod name;` (34 files, among them
`camera_scenarios.rs`, whose scenario App counted as a second in-session
writer of `ControlledSubject`); five adjudicated Kaleidoscope/visual-quality
resources no longer have a writer (their verdicts are gone); `body_seed`'s
borrowed `BodyGeometry<'a>` view collided by name with the shared-tangle
resource (renamed `BodySizes`). The eight NEW
multi-writer resources it then named are adjudicated (`AudioPlaybackConditions`,
`BodyGeometry`, `EditableAbilitySet`, `LocalSeatClaims`, `PlayerGlowCatalog`,
`PromisedRoomPlans`, `SfxLoopPlayback`, `SfxOneShots`: disjoint fields, a
keyed registry, an editor plus the session edge, a player and its mixer).
Then the next layer: thirteen adjudicated resources had gained or lost a
writer since their verdicts; each verdict carries a dated addendum for its new
writer (or its departed one), `SeatMenuFrames` has a new verdict (the per-seat
menu pipeline replaced the tool-only one), and `MapMenuState`'s baseline
follows the file its verdict already named. The guard is green, and so is
`check_session_authority_none_arms` (fifteen rows written, two stale ones
gone, one read made required). Then the other three: the session-root
declarations follow the per-room rules fixture to its new crate and name the
menu's `#[cfg(test)]` hand seam; `FactionRelations` is composed and installed
by Ambition's content plugin rather than written through `resource_mut`
(the guard now sees a typed `insert_resource::<T>` install); `ActorGlow` and
`PresentationCover` are waived as presentation (render-crate `Update`
filters only). `check_level_box_readers` was red with 73 unclassified
`kin.aabb()` reads; its premise had gone (2026-11-10): since the body
material, `BodyKinematics::aabb()` is the material's envelope
(`material.half_extent(canonical_size * 0.5)`), which righting turns toward
the body's DOWN, so it IS the turned box the guard asked readers to use, and
`collision_box(last_step)` no longer exists. The guard is re-aimed at the
level box's present spelling, a world box built from the unturned half
(`Aabb::new(<not the origin>, ..canonical_size * 0.5)`); that population is
zero, so its baseline is empty and the check is green. Evidence that `aabb()`
is turned: `a_body_in_sideways_gravity_collects_the_pickup_its_own_box_touches`
(items) turns a body's material a quarter and collects a pickup only the
turned box touches.

**Red app tests seen in a focused lane (2026-11-10, on ae2e0e8bab, before and
after the ISL-2d bag slice):** `gnu_ton_fight::with_its_arms_on_a_blow_launches_him_and_the_third_blow_throws`
and `gnu_ton_fight::thrown_off_the_back_he_comes_back_and_he_goes_to_a_player_at_its_far_end`
("no blow landed"; the file's last commits are the portal owner's limb-sweep
work, so they are reported to it, not fixed here), and
`two_seats_two_items::two_local_seats_each_pick_up_their_own_bomb` ("the
match seats this fighter": a Smash match, which on this VM waits for
unrendered music). Also red on 316ff3ef7f with no local change (2026-11-11,
stashed and re-run): `smash_ride::a_shark_summoned_into_a_save_that_remembers_a_dead_one_is_still_alive`
("the opening ceremony never released the cast"), the same Smash start.

**Also red with no local change (2026-11-11, my files stashed and re-run):**
`shell_host_rendered::provider_relative_sfx_resolves_the_real_source_and_rejects_stale_work`
("Sanic resolves its authored procedural Dash") and
`smash_cpus_damage_each_other::the_goblin_and_the_pca_do_not_ask_for_the_same_sound_many_times_on_one_tick`
(the match never seated two bodies): the Sanic and Smash starts on this VM.

**Red at f18c6f4875 and every later commit checked (2026-11-11, checked out
and re-run):** `blink_run_reachability::blink_run_blinks_across_the_death_gaps`
("should have blinked across gap 1 onto the mid floor", one reset).
**Classified 2026-11-11, two causes.** (1) An engine defect, fixed: the
player is now 30 by 48 and `blink_run`'s corridor is 48 high, so the body
touches floor and ceiling, f32 rounding put its bottom face 1.5e-5 into the
floor, and `blink_target`'s embed check (an exact `strict_intersects`)
refused every blink in the corridor; it now allows 0.01 px
(`a_body_that_fills_its_corridor_blinks_along_it`). (2) Content against the
body: each gap is 128 px of hazard (8 tiles) and a blink carries 150, so a
30-wide body lands 8 px over the hazard at best (and the test's walk edges,
275 and 595, are past the 273 and 593 where its edge first touches the
hazard). `blink_run` is LDtk-owned (`level ownership status`). **Fixed 2026-11-11:**
each gap is 7 tiles (112 px; the far column is floor again) and the test
blinks from 266 and 586; it passes. Also red in the same lane, environmental:
`director_teleport_blink` (three) and
`smash_ride::a_saddled_body_advances_its_clocks_and_none_of_its_displacement`
("the match seats a first fighter", the Smash start).

**Kernel lift double carry (found 2026-11-11, Jon's "the shark up to the
Mockingbird vibrates"):** `mockingbird_fight::a_rider_rises_with_the_shore_shark_without_bouncing`
(ignored, known red) rides the shore shark (`mockingbird_arena_lift`, 300
px/s, a one-way lift). Of 122 carried ticks, one carries the rider twice: at
the shark's top 485 (tick 69) its feet go 490 to 480 and stay at 480 the
next tick. Every other tick they are within 6.2e-5 px of the top, and the
gap's f32 step changes at 535 and 510, so the second carry is a rounding
edge between the gravity axis' repair and its sweep's carry, not a
geometry contact (the shaft is clear). Not fixed: the collision kernel's
material casts. The steady stutter Jon saw was the platform's picture,
drawn at its tick position under a rider drawn between ticks: fixed
(`a_moving_platform_is_drawn_between_its_ticks`).

**Red with portal LDtk compiled out (2026-11-11, portal agent's area):**
`ambition_content` lib `intro::tests::each_intro_room_says_its_cutscene_in_data_and_the_library_has_it`:
"level 'central_hub_basement' PortalGunSpawn ... the portal LDtk converter
is compiled out (enable the `portal_ldtk` cargo feature)". The shipped
worlds no longer compose for a crate built without `portal_ldtk`. Reported,
not fixed (portals are the portal agent's).

**Red at 392defdb08 (where the test was last written) and every later
commit checked (2026-11-11, checked out with the current map submodule and
re-run):** `companion_dog::the_dog_climbs_to_a_player_who_stands_far_away_on_the_upper_deck`
("the dog's first place was (1335, 1888), not beside the player at
(420, 1608)") and
`companion_dog::a_brute_brain_in_the_dogs_body_climbs_to_its_target_and_one_that_does_not_navigate_stays_below`
("rose 0 px"). Older than the ISL-1b work. Not classified: the checkouts kept
today's map submodule, so a map change under an older test is the first
suspect.

**Red at 13ca8d5c34 and every later commit checked (2026-11-11, checked out
and re-run):**
`death_restores_the_checkpoint::a_death_takes_back_from_bobs_hand_what_alice_banked_in_hers`
fails its control ("Bob holds the gun-sword Alice put down, next door":
`(0, 0, 1, "Placed")`, the sword still placed). Older than the ISL-1b
work; not classified.

**Red guard reported to the portal owner (2026-11-11):**
`check_session_authority_none_arms.py` is red on
`game/ambition_app_tools/src/bin/capture_scene/portal_fixture.rs::place_portals`
(added in 9d6f9da4ff, "Render portal limbs from their physical roots"): it
reads `ActiveSessionScope` optionally with no `none-arm:` row. The portal
owner's file, so it is reported here, not fixed.

**Uncleared verification (2026-11-04):** the last full feature run recorded
30 failures in five targets. Subsequent selected app cases (seven) and spawn
owner cases (14) passed after repairs; **those focused passes do not clear the
full matrix**. Re-run/triage on a machine with the required published art and
rendered music. On the sampled VM, Smash and Sanic first rooms could not
start because their music tracks were not rendered; classify missing assets
separately from code failures. Keep failure evidence in the lane ledger.

**Compile ratchet reconciled (2026-11-10):** the baseline frozen at
`4ed566a24ad1` disagreed with its own crate table for `ambition_geometry`, the
monolith and the core. The gate had been red on a gating finding: the critical
path had grown from 16 crates to 24. The measured path is `ambition_geometry →
core → time → shared_tangle → world → portal2d → projectiles → combat →
held_items → abilities → shrine → construction → items → breakables →
projectile_combat → actor_monolith → session → sim_view → runtime →
rollback_ggrs → platformer2d → fighter_specials → content → app`. The links
it gained are AP14 carves stacked in series under the monolith. The baseline
is re-frozen with that reason at `7c6ca7f71e`, and the gate passes. If build
wall-clock matters, that series is where to cut. Thirty-nine new crates have
no measured compile time; they are priced at the median rate, a placeholder,
until `scripts/measure/compile_collect.py` measures them.

**Other precise follow-ups:** capture an assertion if the historical
`composes_through_the_sdk` in-company failure recurs. These are not grounds
for retries or enlarging fixed-frame waits. Optional broader cleanup belongs
in the owner plan, not as a second chronological task list here.

**Guards re-read 2026-11-20:** five static guards were red on main from
writers that landed 11-13 to 11-19 (`99e368087`, `93e80746d`): the
multi-writer adjudication, the sim-consumed request writers, the session
None-arms, the host-produced ingress census and the rewind crossing census.
Every new writer was read and recorded; one real gap was the portal schedule
plugin's unregistered effect channel (`48b222336`). The camera-centre arm of
the stage-kills test now allows the capped collapse it was designed with
(`a93904370`).

**Acceptance:** selected duplicated setup is removed without reducing tested
behaviors; required checks run with honest PASS/FAIL/NOT RUN classification;
the outstanding feature-matrix failures are reproduced, repaired or explicitly
classified against their prerequisites.

### AP14 — actor kernel decomposition

**Owner:** [`actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md)
and [`actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md).
Jon's current instruction selects the remaining decomposition. Move each
responsibility to its owner and keep one body, control, and action path.

| # | Item | Next step |
|---|---|---|
| AP14 | Complete actor-monolith responsibility separation | The kernel module graph is a DAG. Lifecycle, publication, materialization, items, world objects, device input, observation, and global schedules have explicit owners. The public `body` API owns execution; `control` owns accepted control. Room technique grants follow each driven body. Real simulation covers two-seat possession, reaction clocks, controlled-body removal, action continuity, one-step free motion before and after seat releases, and mounted motion through timed dismount under rollback. Audit residual responsibilities and public operations. Keep accepted control and body execution together. Preserve rollback, publication barriers, and the number of active systems. See the [decomposition plan](engine/actor-monolith-decomposition.md#dependency-first-extraction-sequence). |

**Built:** `d541442ba` installs gate and lock-wall effects through
`SessionRoomVisualsPlugin`. It removes the game-host registrations and their
obsolete exceptions. The owner tests and installation scan pass; the kernel
module graph remains a DAG. Do not add host-installation exceptions for these
effects.

**Built 2026-11-20 (K36):** empowerment is `ambition_combat::empowerment`;
the plan's [residual body inventory](engine/actor-monolith-decomposition.md#residual-body-inventory-measured-2026-11-20)
records why `perception` stays with the brain dispatch. `fade_rows` and
`death_beat` followed to combat (K37, K38).

**Built 2026-11-21:**
- The `ledge_grab` and `swim` adapters are gone. Each held only tests of the
  core's own step, and those tests moved to `ambition_platformer2d_core`.
- `character_policy` is folded into its one caller, `brain_command`.
- The graph is still a DAG; `body` has 20,512 production lines.

- `drawn_ledges` moved to the runtime composition, which installs it; it
  joins the mount's ledge and the sprite sheet's pin. `body` has 20,455
  production lines; still a DAG.
- K39: the fighter ladder projection is `ambition_combat::brain::fighter::ladder`
  with its own plugin.
- K40: the wallet shield projection is `ambition_damage::wallet_shield`.
- The brain-feed owner question is answered from a per-module read: the
  remaining feed and conduct modules form groups tied by sibling and kernel
  reads, so none moves alone; they stay with the dispatch.

`beat_cues` stays: it reads the body's decision frames. The
[inventory](engine/actor-monolith-decomposition.md#residual-body-inventory-measured-2026-11-20)
pass is complete; `transform_beat` is the one module left with no sibling read,
and it waits on an owner for its rollback rows.

**Acceptance:** each cut moves state, behaviour and installation together and
deletes the old edge; `python3 scripts/measure/measure_kernel_module_graph.py --scc --cuts --edges 80`
is run after it.

### I2/I3 — finish independent content authoring and safe reload

**Owner:** [extension model](engine/extension-model.md),
[fast iteration implementation](engine/fast-iteration-implementation.md), and
the [SDK publication cut](engine/content-generation-and-reload.md#sdkdemo-publication--admit-owner-products-before-selection).

**Built in main Ambition:** app-scoped pack selection; admitted generation publication;
no-op/stale/superseded edit handling; reload for moves, characters, audio,
cutscenes and quests; native/WASM procedural modules; `actor_think` S1-S3;
held traversal modules; and the `body_contact` extension route. Do not
re-enqueue the old missing-reload-family list or repeat the completed brain
migration. The SDK/demo path selects each pack's generation in the App that
watched the edit (P5 step 4 cut 2, landed 2026-11-28), and admits every
affected owner product before it selects.

**Contact customer receipt:** the named `body_contact` customers are done (2026-11-19).
The Sanic badnik's contact is a module, and its toucher roll and
contact-harm facts are asked of their owners through
`sanic.contact.touchers`. The Mary-O snake shell is a module that keeps its
phase in the snake's record and holds, withdraws, pins and runs down through
the new `body_contact` requests. The plant is a brain
and pose customer, not a contact rule. Recheck which customers are still native
before migrating them.

**SDK publication cut landed (2026-11-28):** delivery steps 1–4 at the
owner. Step 4 made the selection App-owned: a reload selects into the App
that watched the edit, systems bind `SelectedPacks` once, and helpers in
Mary-O, Smash, Sanic and versus take the row they read. See the owner's
receipt.

**Running-demo world reload (2026-11-28):** F11/F12 reloads the running
session's world through its provider's declared reader; Mary-O's world and
Sanic's three acts reload in their running sessions. See the
[fast iteration](engine/fast-iteration-implementation.md) receipt.

**Next action:** wasmi's per-call cost, the largest loaded-road cost M1
measured. A 128 KiB module stack, instance reuse from the starting image and
the guest's module table built into that image (`amb_prepare`) took a
sentry call from about 89 to 28 µs (2026-11-28; native is 0.8 µs, Rhai
10.6 µs). What remains is the image restore (6.4 µs for 4 pages) and the
sentry's own guest work. I6's native arm is measured and not shipped; I7's
cancellation was measured with no defect; I5's `Checkpoint` eligibility
waits for a module that declares it.

**Acceptance:** a foreign-audio collision plus a rules edit leaves the old
pack and all affected products selected, with one refusal and no published
count. Multi-schema audio/cast edits prepare each product once. Same-frame
watchers retain each other's accepted provider rows. Two Apps with one
embedded source keep independent selections and timelines. Existing local
rebase, remote generation and retained-reader rules continue; no new
live-session restatement policy is implied. The completed module customers
keep their supported road without a native duplicate.

### A7 — make item occurrence authority explicit where it carries a real invariant

**Owner:** [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md).

**Current state:** `GroundItem` construction is sealed, but that seal does not own
occurrence creation. Death drops, match spawns and other roads still mint
identity/provenance/custody facts at their occurrence sites. Two mint sites
degrade on purpose and belong to the `ItemCustody` inventory leg:
`ambition_held_items`'s thrown-item mint and `puppy_slug_gun`'s minion mint.

**Concrete inventory customer:** [bomb pocketing](game/ability-loadout.md#open-for-bombs)
needs one occurrence-to-quantity exchange that the checkpoint can restore.
Record the received quantity and the object's retirement at the existing
custody/accounting owner. Prove conservation through throw, pickup, pocket,
death and save/load; do not add an unrelated grants mirror beside spends.

**Bomb pocketing built (2026-11-21):** a stow of a carried stock pockets it.
It is recorded as a `BagSpend` of exchange `Pocketed`, so the checkpoint keeps
a pocket exactly when it keeps the object's end. The custody row ends as
`Consumed` (`AuthoredOccurrences::end_carried`). See
[ability loadout](game/ability-loadout.md#open-for-bombs).

Conservation:
- Witnessed for the pickup and the pocket.
- For a death and restore, it follows from the shared keep test:
  - with no participant spared, neither the throw's spend nor the pocket is
    kept, and the bag is the checkpoint's;
  - with the room spared, both are kept, and they cancel.
- The death arm was probed in the running app (2026-11-21, probe deleted
  after the run). The floor bomb is picked up and banked in hand at a shrine
  checkpoint (bag 2, row `InCustody`), then pocketed (bag 3, `Consumed`).
  After a death the bag is 2 again, the bomb is back in the hand that banked
  it, and its row is `InCustody`: the pocket was not kept, and nothing was
  minted twice. A floor bomb in a neighbouring live room cannot stand in for
  this: its room unloads once the rest is over.
- Save/load needs nothing new: the bag is saved, and a `Consumed` row is a
  saved row.

**Occurrence creators read (2026-11-21):** the owner page now lists every
runtime creator with its identity, provenance and the reason it owns its mint
([inventory](engine/item-custody-and-accounting.md#occurrence-creators-and-why-each-owns-its-mint-read-2026-11-21)).
Death drops already share one road (`spawn_death_drop`); reward policy only
reads outcomes (pickup grant source, chest rewards). One asymmetry is recorded:
match items have identity and no provenance.

**Next implementation, updated against `00bf92c22142`:**

1. [IR1](engine/item-recipes-and-live-continuity.md#ir1--retain-the-recipe-chosen-at-mint)
   has retained canonical extent and both reconstruction readers since
   2026-11-22. Keep its [first-slice receipt](engine/item-recipes-and-live-continuity.md#ir1-first-slice-done-2026-11-22).
   The shipped native reconstruction witness and the retained-field table
   landed 2026-11-24 ([IR1 done](engine/item-recipes-and-live-continuity.md#ir1-done-2026-11-24));
   IR1 is complete. Do not redo the extent migration.
2. IR2 moves dormant recipe readers off the save codec. (Done 2026-11-24:
   [IR2 done](engine/item-recipes-and-live-continuity.md#ir2-done-2026-11-24),
   `RetainedMintRecipes` is the item owner's native record, and the save's
   minted rows are its export.) IR3 keeps active
   pose in the live body, captures retained state at accepted boundaries and
   removes motion-driven dormant work. (Done 2026-11-24:
   [IR3 done](engine/item-recipes-and-live-continuity.md#ir3-done-2026-11-24).) Preserve the atomic horizon receipt
   below, existing room rewind and domain placement conventions. E6 owns
   the cost evidence; IR3's ledger-path measurement is on its receipt.
   IR4 separately reduces pose-only output frequency. It preserves prompt
   semantic writes and adds the required graceful switch/quit capture.
   (Done 2026-11-24: [IR4 done](engine/item-recipes-and-live-continuity.md#ir4-done-2026-11-24).)
   [REL4](engine/relationship-endpoints-and-transitions.md#rel4--retain-exit-context-and-authorize-endpoint-succession)
   adds held-item exit context to those selected cuts: missing reconstruction
   must not replace the orphan fallback with zero, and accepted transfer
   must give it explicit destination meaning. Keep same-room return-where-taken
   behavior, bind-time material and actual release admission; no per-tick
   hand-pose mirror. Scope custodian resolution to the selected population
   and use accepted succession for publication handoffs, so a same-ID holder
   in another live instance is untouched. Preserve A10's existing receipt
   ownership and refusal behavior.
3. [A7 exchange](engine/item-custody-and-accounting.md#a7-exchange--one-rule-for-an-offer-and-its-accepted-exchange)
   makes shop offer checks and accepted exchanges share the existing pure
   `InventoryExchange` law. Pair it with O6a's actual payer. Keep narrative
   and procedural history retention separate and preserve CW2's open causal
   work. This slice can proceed independently of IR2/IR3. (Landed
   2026-11-24: shop deals lower to the exchange's `check`/`apply`, and the
   merchant offers ask `can_buy`/`can_sell` of the conversation's payer.)

Centralize only a real invariant. The recipe does not require moving all
minting sites behind a generic request bus.

**Acceptance:** reward policy consumes accepted occurrence outcomes and cannot
become an alternative minting authority; every remaining occurrence creator has
an explicit ownership reason.

**Acceptance read 2026-11-21:** both halves hold by read.
- Every creator in the inventory has its reason. The match-item asymmetry is
  now stated at `spawn_match_items` as session-only.
- Reward policy spawns nothing: `collect_ecs_pickups` reads origins, and
  chests grant what an outcome names.
- "Cannot become" is policy, not type. `GroundItem::at_rest`/`released` are
  public, and the census rules out a request bus that would close them.

**Atomic durable horizon done (2026-11-21):** minted items, occurrence rows
and custody are one write (`set_durable_horizon`), made by the session's
horizon writer with rows the item domain builds (`MintedItemRows`). A minted
row is kept only beside its occurrence row. See the
[owner page](engine/item-custody-and-accounting.md#open-work).

**`durably_held` width decided (2026-11-21):** it stays the `ItemCustody`
component; narrowing to `Held` would drop save rows.

**Menu equip re-read (2026-11-21):** already one road. The menu's Equip is
a stamped request applied in the sim by `change_hand`, as a script's `equip`
is, and a bag-backed hand is stated session-only. The owner page said
otherwise and is corrected.

**Agent surface built (2026-11-21), over a save file:** `item_report`
(`ambition_app_tools`) answers `where`, `list --room/--body`, `audit` and
`explain` from what a load would rebuild. Checked on a crafted save: `audit`
named a custody row held over a spent chest and a mint with no minted row,
and exited 1. A live-world view is not built.

The earlier A7 acceptance held by read above. It did not establish recipe
sufficiency, moving-world cost or agreement between shop and native exchange
laws. Those follow-ups are open; the earlier horizon and custody receipts
remain in force.

### BRAIN — finish truthful fighter attack selection

**Owner:** [fighter brain](engine/fighter-brain.md) for action selection and
recovery execution. The [tactical-intent experiment](engine/fighter-tactical-intent.md)
remains paused.

**Next recovery work when this lane is selected:** [REC1](engine/fighter-brain.md#rec1--execute-the-recovery-after-the-move-starts)
must carry the Admiral's selected ride through flight and actual return to
support. Existing Recover presses the ride's bail edge and supplies no
flight target. Use ACT1's actual action offer/identity, effective motor
routing and a checked entry/release region. Reuse the current control and
flight law, then complete the
forecast/execution follow-up needed for that return. Other REC2/REC3 work
stays separately selectable. This corrects execution before more scoring or
ladder tuning. It does not move BRAIN above the
current flagship order.

**State:** whiff memory, startup-as-first-live-hit, pure-counter valuation,
stop-distance and coast-distance features are implemented. The 2026-10-13
Sanic rung-5 run left the stage under grounded attack momentum. That is
answered: a grounded move with `MoveGates::stops_at_ledge` holds at the
floor's end (2026-11-19), and Sanic's roll is the committed roll of Q181
(SANIC-ROLL, 2026-11-26). Re-run rung 5 before treating it as a brain
defect, and do not add a fixed-range or stale-view brain workaround.

**Remaining roads, remeasure before implementing:**

1. Projectile launch value: a ranged move has `launch: 0` although its hit
   uses `HitKnockbackMagnitude::FeelScale`. Clarify the resolved launch and
   hazard-coverage features against real kit choices.
   **Re-measured and half done 2026-11-21.**
   - The launch half: a `Ranged` beat fires the wielder's weapon, and
     `RangedActionSpec::launch` (`base` + `growth` per percent, the volume
     law) already states the shot's launch. Only the PCA's `pca_life_cannon`
     states one (30, 0.76); every other shot deals the fixed nudge. Now
     `MoveHazard::Spawned` carries it as `ShotLaunch` beside its damage
     (`resolve_owners_ranged_action` fills it), and
     `MoveFrameData::strongest_launch` folds it in like `strongest_hit`. The
     kill-potential share reads it (witness
     `a_shot_that_states_its_launch_earns_kill_potential`). Authored bolts,
     seekers and bombs state `launch: None` until their params carry one.
   - The coverage half, in the kill gate: a launcher has no volume, so
     `coverage` is `None` and `reach_fit` is 0. Payoff is gated by frame
     advantage alone, so only kill potential lost out: it was gated by
     `reach_fit.max(frame_advantage)`, so a shot earned it only against a
     committed foe. Now a `hazard_fit` (the led gap inside the hazard's
     reach: admission's own question, with `reach_fit`'s falloff) joins
     that gate. A fused bomb touches nobody on arrival and gets 0.
   - Deliberately not done: `hazard_fit` does not feed the `reach_fit`
     feature. That would reweight every projectile fighter's whole menu
     (reach is the heaviest weight), and it waits on a measured grid
     (move-usage census of the projectile fighters before and after).
2. Placed traps: resolve the hazardous region relative to the attacker,
   rather than treating placement radius as attack reach.
   **Done 2026-11-21:** `ThreatTravel::Placed` carries its region:
   `at` (the authored offset, body-local) and `radius` (the blast). The old
   scalar `reach` (`|offset.x| + blast`) is gone. `MoveHazard::reaches`
   answers admission with that disc for a placed object, and with the reach
   from the body for anything that flies. `reach()` is still "how far from
   the body" (`|at| + radius`). On the polygon's shipped bomb, `(-16, 14)`
   with a 56px blast, a foe 70px in front is no longer admitted and one
   behind her is. Witness
   `a_placed_bomb_reaches_its_own_region_not_a_radius_around_the_body`.
   Traps still earn no strike payoff (`strikes_on_arrival` is false); pricing
   stage control is a separate feature.
3. Teleports: expose the resolved destination (`behind_nearest_foe`,
   `behind_gap`, aim) to the attack offer.
   **Re-measured 2026-11-21: no customer in shipped content.** Every
   teleport is one of two kinds, and neither is an attack the offer prices
   by where it lands:
   - Aimed recoveries (`behind_nearest_foe: false`: robot `phase_shift`,
     duelist A, the Director's up-B). The grid measured these off the
     attack list (`options.rs`: 225%/223% with the move on no list,
     against 27–39% admitted).
   - Conditional responses: the Shadow Oni's teleport is a flow branch
     that fires only after a blocked strike, and the Director's is his
     counter's `response`, which `counter_value` already prices.

   The work comes due when an authored move teleports as its approach (an
   ambush that strikes after arriving). Then its coverage has to be measured
   from the destination, and `options.rs`'s "WHAT IS OWED" note is where to
   start.
4. Difficulty ladder: measure `--rungs 3,4,6,7,8` and distinguish step size
   from the missing L3 planning step; shipped rungs set `rollout_depth: 0`.
   **Measured 2026-11-21** (`smash_tool ladder-rig --rungs 3,4,6,7,8
   --paired --seeds 6 --character smash_george_booul --opponent
   smash_george_booul`, the shipped 480 s clock, 48 bouts). Every row is
   within spread: with 6 paired seeds, a row needs 6 of 6 to clear the bar.

   | Pair | Verdict (per seed) | Survived, median (hi : lo) | Dealt % (hi : lo) |
   |---|---|---|---|
   | 4 vs 3 | higher, 5:1 (83%, p 0.219) | 132.9 : 135.1 s | 301 : 245 |
   | 6 vs 4 | higher, 4:2 (67%, p 0.688) | 126.4 : 123.6 s | 271 : 251 |
   | 7 vs 6 | even, 3:3 | 119.0 : 116.9 s | 316 : 255 |
   | 8 vs 7 | higher, 5:1 (83%, p 0.219) | 93.0 : 92.3 s | 253 : 208 |

   Every row ended 0:0 in stocks left. The higher rung dealt 8–24% more in
   all four rows, the even 7-vs-6 row included, and bouts get shorter up the
   ladder. Reading: each rung is a real but small step, smaller than the
   bout-to-bout spread at 6 seeds. Every rung prints `rollout 0/0`, so the
   disabled planning step (`rollout.rs` exists; the ladder rows switch it
   off) is the same on both sides of every pair. It cannot be what separates
   adjacent rungs. It is a question only about how high the top of the
   ladder can go. The flat 7-vs-6 row matches the smallest authored gap: the
   two rows share `fadv 0.60` and differ by 40 ms reaction, 40 apm, 0.04
   noise and small weight steps.
   Before 4d4b95f27, duelist_l2/l4/l7/l8 had no published policy, so any
   `--rungs` run that named them was refused by the noise-seed guard. An
   earlier short default run on this VM (2 seeds, 60 s) had the higher rung
   outfight in 3v1, 5v3, 6v5 and 9v6.
   **The planning step, measured 2026-11-21:** a ladder copy whose rung 9
   is rung 8 with `rollout_depth: 12, rollout_k: 4` (the values the
   decision tests use), run as `--ladder <copy> --rungs 8,9 --paired
   --seeds 10`, George mirror, 480 s. Result: rollout on outfought off 6:4
   (60%, p 0.754, within spread; with 10 seeds the bar is 9 of 10).
   Survived 86.5 : 88.9 s, dealt 206 : 210%, peak 110.5 : 143.0%. Turned
   on as it stands, L3 buys nothing this rig can see. That matches the
   ladder file's own note: rollout stays zero "until rollout fidelity is
   good enough". The top of the ladder will not rise by flipping the
   field. Inside this George mirror experiment, a useful rollout needs
   better fidelity before activation. That result does not test mounted
   recovery or make rollout a prerequisite for REC1. Road 4 is measured;
   more adjacent-pair seeds would only sharpen the step sizes.

**Acceptance:** representative CPUs choose movement-compatible attacks
across their authored kits without replacing attack admission with a second
rule, a reach floor, or fighter-specific evaluator exceptions.

### PORTAL-SIMILARITY — canonical parts and scaled mirror portals

**Failure / customer:** the material/Sim2 foundation and playable 80/40 slice
are implemented, but several consumers reconstruct scope, trajectory or view
meaning from a smaller projection. The 2026-11-21 source review at
`b91b29d35935` selects corrections to topology/lifetime, accepted motion,
conservative query preparation and complete observer views.

**Owner:** [portal delivery plan](engine/portal-parts-similarity-and-chirality.md).
Its [implementation plan](engine/portal-parts-similarity-and-chirality.md#implementation-plan)
owns P1–P6 and links the detailed contracts. The source counterexamples have
not been reproduced by a new Rust/game run in this review.

**Next action:** P1's first two slices are done (2026-11-22): scoped reset
and same-position held recast; released-pair cooldown cleanup and held lease
release on room transfer. Its owner retains those receipts. P1's third slice
landed 2026-11-26: the held-pairs observation replies with each slot's
outcome (open, pending, closed on use, fizzled, refused); P1 is complete. P2's first
step landed 2026-11-25: generic transit no longer crosses what the kernel's
refusal recovery closed (`SweepSample::restriction`, schema 545; see
[the receipt](engine/portals-are-moving-frames.md#refusal-recovery-can-be-overridden-by-later-crossing-discovery)).
Since landed (2026-11-25, [receipts](engine/portals-are-moving-frames.md)):
transit continues from the crossing for a moving exit, contact reads the
legs before and between crossings, an interpolated view and a feature row
draw a crossing at its time, and a line host binds SUP1's material place.
M1 landed 2026-11-26 (a: the travel record names its step; b: crossings
use the aperture snapshot the step was admitted against; c: a contact keeps
the chart it was met in, also for scripts on `ambition.body.contact@4`).
M2a/M2b landed 2026-11-26: the kernel interrupts a body's step at each root
crossing and continues it in the exit's chart; transit publishes the
kernel's crossings. M2c landed the same day: items and projectiles give
the rest of the tick's change of velocity in the exit's chart. M2d landed
the same day: transit moves no root (a settled item's carry crosses in its
owner), the old completed-endpoint road is deleted, and the crossing reads
the velocity at mid-tick so a floor-to-floor loop keeps its energy. Next in
P2: SUP1 through a portal cell (the deforming line split landed 2026-11-26,
the rider beside the cut, S2's clipped deforming cast and S3's
curve-carried cast 2026-11-27, with a round body in a cut cell, a line
on an aperture's own plane and a line reaching or leaving it the same day;
only a line on a turning plane reads the rigid fit, exact when it turns with
the plane). Host admission landed the same
day as crushing C5. P4's full-turn bounds and repeated-link candidate exclusion landed
2026-11-26 and its shared projectile source leg 2026-11-27 (receipts in the
owner's P4 section), and an unreachable moving portal is witnessed not to
change a straight query's cells 2026-11-27; exact cell dependencies (a
reuse cache keyed by what each cell reads) were measured the same day and not
built: in `portal_lab`, 93 % of portal-cut preparations repeat their inputs,
but all of them cost at most 2.4 % of step time. P3 landed
2026-11-27: the rim is contact inside the accepted motion, and the AABB snag
writer is deleted (M3 had already removed the static velocity
reconstruction). P5/P6 complete observer identity,
chart transport and demand/ready-image capture without replacing working pools
(V1, a window for each view of a room, V2, a window clipped to a rolled
view, V3, a moving opening drawn when its bodies are, and the rest of V4
landed 2026-11-27; V5, the card view's window, was already in place: P5/P6's
V list is done).

The follow-up [SCALE-FEEL](#scale-feel--the-same-body-behavior-at-a-different-scale)
supplies the selected consumer-unit law; [PRESENTATION-COMPOSITION](#presentation-composition--order-appearances-and-add-a-25d-view)
supplies local appearance order and a later 3D rendering consumer. Keep P2/P3
time/contact and P5/P6 observer/capture authority here.

The `00bf92c22142` [query audit](engine/world-geometry-and-spatial-semantics.md)
adds two bounded continuations in this lane. T3 landed 2026-11-27: a
room's replay replaces only that room's portal publication, and keeps each
hidden room's links and each hidden body's entries
([receipt](engine/portal-pair-identity-and-aperture.md#t3-refinement--a-publication-replaces-only-its-declared-population)).
Water membership, chest/switch touch and climbing read accepted parts
(landed 2026-11-27, [receipt](engine/world-geometry-and-spatial-semantics.md#bounded-delivery-and-evidence)).
Touch keeps the part's map for facing and preference. A climb is admitted
only through a map that keeps up. The observers and cost counts (step 4)
landed the same day: `body.regions` keeps the touches the response drops,
and no cache is proposed.
Keep discrete sampling, strict overlap and exact-box fast paths. These cuts
do not wait for all P2–P6 work or select a universal zone redesign.

**Acceptance:** the accepted operation binds its actual scope, topology/time
selection, restrictions, path and final state; later readers do not infer a
different operation. Existing unequal-aperture, material/parity, multi-limb,
refusal and replay behavior remains covered. Each selected packet removes
its replaced interpretation. Capture/device and matched performance evidence
remain separate gates; no new speedup is claimed.

**Scope:** continuous physical portals stay within one live simulation room.
Retiring a held lease on ordinary room travel is current lifecycle work; it
does not require cross-island live portals. Preserve the authored lab and
current boss/common-body work. The older static kernel continuation receipt
is not evidence of moving-map residual time or completed multi-view support.

### PRESENTATION-COMPOSITION — order appearances and add a 2.5D view

**State:** IN PROGRESS: PO1 and PO2 landed 2026-11-25; RV1 landed 2026-11-25 (`RiggedLocalSample`, the 2D driver its first consumer; `ViewCameraSet`: one renderer-free view frame, a 2D adapter that writes only `Camera2d`); RV2's cart pilot landed 2026-11-25 (`rendering::card_view`: an orthographic Camera3d of unlit part cards and a cart mesh beside the 2D view, one sample, one declared depth policy); RV3 planned. Maintainer-selected 2026-11-21; source `f41218979625`.

**Failure / customer:** Scout must appear behind the cart panel inside and
in front outside. Current seating writes root z 14.45 behind panel 14.5,
while a hit flash adds 1.5 and a wielded item uses player z. Equal-root rig
parts can interleave. Numeric z also controls portal carry eligibility.
The future 2.5D consumer must coexist with current 2D authoring/rendering.
These are source deductions; no new image or device measurement was made.

**Owner:** [presentation order and optional 2.5D rendering](engine/presentation-order-and-25d-rendering.md).
PVS-2 retains visual sampling; portal P5/P6 retains projection/capture.

**Progress (2026-11-25):** PO1 landed: the cart's seats sit a whole body
depth band under its near side, scheduled before the drawable writers, and
an actor is matched in its own room; captures show the seated flash behind
the rails (the old depth drew it over them). What PO1 left is listed on the
owner page.

**Next action:** PO2 extends
that authority to common consumers and removes numeric-z classification as
those consumers migrate. RV1 extracts a complete shared sample before sprite
materialization and isolates each view's camera. RV2 draws one orthographic
3D card/cart-mesh view beside 2D; RV3 integrates portals and required effects
before general rollout. Keep 2D the default and 3D optional.

**Acceptance:** actual parts, flash and held art order correctly inside,
jumping within and outside the cart; overlapping appearances, two carts and
same-named actors in two rooms retain exact ownership. Prepared local order
and bounded changed-instance checks avoid an all-room relation solve. Either,
both or no renderer leaves simulation unchanged; equal requested samples
have equal meaning without forcing headless sampling. Inspect both views in
one real window. A no-portal pilot cannot claim full portal support. Measure
sample reuse, draws, CPU/GPU work and retained bytes before claiming savings.

### MOUNT-RIDER-CUSTOMER — a shipped rider controls a shipped mount

**Owner:** `body/brain_command.rs` (`MountedBrainCache`, the source-only
arm) and the dismounted-rider road (`body/dismounted_rider.rs`).

**Ruling:** Q76 (2026-10-03): keep mount/rider support and give it an early
customer (a character riding the dog, or the robot commandeering a shark).

**Customer (2026-11-20, slice 1):** the robot commandeers a riderless shark.
`player_robot_v3` authors `pilotable_classes: ["shark"]`. The licence is
granted with the rest of the character body (`grant_prepared_character_body`),
so the campaign protagonist gets it too; before, only a match seat did.
`ambition_mount::commandeer_mounts` (`MountsCommandeered`, first in the mount
chain) boards the nearest licensed, riderless, unreserved, living mount
within `COMMANDEER_REACH` of the pilot's box when it presses Interact. Interact
while riding writes `DismountRequested { RiderBailed }`. Witnesses:
`interact_commandeers_a_riderless_mount_and_gets_off_again` (mount lib) and
`the_protagonist_commandeers_a_riderless_shark_and_gets_off` (app_it,
sandbox).

**Done 2026-11-20 (slice 2), one witness each:**
1. **The mount's own presses are masked.** While a mount is ridden with
   `ControlGrant::Total`, `steer_mount_from_rider` rebuilds its frame from
   neutral, carrying only the rider's travel: locomotion, velocity, facing and
   the jump edge. Its brain still ticks, so it resumes as it was on dismount.
   A cove shark's `ChargeCrash` bite no longer fires under a pirate (a
   behaviour change for the cove crew: the rider fires from the saddle, and
   the shark's contact damage still stands). The giant's limb strikes go
   through `LimbIntents`, not the mount's frame, so they are untouched.
   Witness: `total_grant_routes_rider_locomotion_to_mount_but_not_fire`.
2. **Damage and death while ridden.** A blow that does not kill leaves the
   rider aboard. A death dissolves the ride. The fix: a rider with no
   `MountedBrainCache` now loses the whole link (`RidingOn` and the saddle).
   Before, the record was kept "so a reset can re-arm the pair", but only a
   cached rider is ever re-armed, so a commandeering pilot kept `RidingOn`.
   `board` then refused its next mount, and its Interact asked to get off a
   corpse. The protagonist's brain is untouched (match seats and non-actor
   bodies are not rebuilt) and it walks under the stick afterwards. Witness:
   `the_prompt_names_the_shark_and_its_death_puts_the_protagonist_down_still_playing`
   (app_it).
3. **Rewind.** Boarding, riding and getting off replay to the same world
   under the sync test. Witness:
   `commandeering_and_getting_off_replay_under_the_sync_test`.
4. **Pose.** A commandeering rider is drawn sitting its saddle, never in
   Jump or Fall (`pose_view::seated` on the player road). This is asserted in
   the death witness above. A dedicated riding row does not exist for any
   sheet; drawing one is art work, not this row's.
5. **Prompt.** `NearestInteractable` answers `Ride(<mount name>)` beside a
   mount the press would board, and `Dismount` while riding. The rule is the
   press's own: `ambition_mount::nearest_commandeerable`. Witness: the same
   app_it test. **Done 2026-11-21:** the Interact button says it too:
   `project_interact_verb` names the prompt's Interact entry from
   `NearestInteractable` ("Talk", "Open", "Ride Burning Flying Shark",
   "Dismount"; "Interact" with nothing in reach; `ByButton` games keep
   "Interact"). The touch overlay reads that label. Witnesses:
   `the_interact_control_is_named_after_what_the_press_would_do` (sim_view)
   and the app_it test above.

**Remaining:** `MountedBrainCache` still has no production constructor, so an
NPC rider is never given a mounted brain. Not needed by this customer. The
Smash ride tests cannot run on this VM (no music registry: the opening
ceremony never releases the cast).

**Done 2026-11-21 (slice 3), a riderless shark in shipped content:** knock
a raider off in the cove's sky lookout (`pirate_sky_lookout`) and its shark
is the protagonist's to take. Two defects stood in the way:
1. A dead rider kept `Mounted` and the saddle, so its shark kept steering from
   a corpse's frame and refused every other rider. Now a rider's death ends the
   ride for the mount: the corpse loses `Mounted`, the ride's hold and its
   claim, and the saddle is freed. The corpse keeps `RidingOn` (GNU-ton's
   conductor finds the giant by it through his beaten finale), and a revival
   re-arms the ride and takes the saddle back unless somebody else is riding;
   a body a participant drives loses the record with its life. No shipped
   rider has a `MountedBrainCache`, so the revival no longer needs one
   (`a_dead_rider_frees_the_saddle_and_its_revival_takes_it_back`).
2. A riderless shark detonated on any tick with no time in it: the killing
   blow's impact hitstop froze the clock, the shark "did not move" while it
   wanted to charge, and the charge-crash test read a wall. The test now
   skips a zero-length tick.

Witness: `a_raider_s_shark_is_the_protagonist_s_once_the_raider_is_down`
(app_it).

**Next action:** Jon rides a raider's shark by hand in the sky lookout.

**Native follow-up, source `00bf92c22142`:**
[REL2/REL3](engine/relationship-endpoints-and-transitions.md#rel2--distinguish-an-active-ride-and-admit-its-transitions-once)
are selected under ARCHITECTURE-COMPOSITION. They preserve these customer
receipts while separating an active ride from reset intent, completing
scope admission, emitting one accepted end and preparing a coherent
destination for a mounted transfer. They do not require the remaining
`MountedBrainCache` customer or reopen possession claim ownership.

**Acceptance:** the customer mounts, the mount obeys the rider, and on
dismount the mount runs the brain it had before; the same after a rewind.

## Waiting — a person or a machine takes the next step

Each row names who or what it waits for in its **Next action**. An agent re-reads it only when that has happened.

### GNU-MOTION-PARTS — diagnose and fix non-smooth giant motion

**Measured and partly fixed 2026-11-10** ([owner](game/boss-cinematics-gnu2-and-motion.md#the-giants-motion-gnu-motion-parts)): the walk row's clock now follows distance walked, so the hooves stand still as the body speeds up and slows down, and rest starts at its first frame after a blow. Measured 2026-11-11 ([owner](game/boss-cinematics-gnu2-and-motion.md#the-giants-motion-gnu-motion-parts)): phases 2 and 3 walk at the same 135 per row second, the rider's rows start at 0 and step by one frame, and the stomp plays stretched like the windups. The two hitches read as portal transit's are settled (2026-11-12): the giant falling out of the well's exit collided with its own slope, now fixed (a body does not collide with the lines it carries), and the free fist's 280-unit jump was the probe swapping two fists' names, not a jump. Rendered part transforms are not measured yet.

**Failure:** GNU-ton's giant appears to move non-smoothly. It already has part-sheet assets, but the conductor pins mount animation rows separately from its `MountWalk` motion, rider and fist poses. Root/interpolation/part-pose timing has not been measured; missing runtime part use is a hypothesis, not a confirmed cause.

**Owner:** [motion diagnosis and fix](game/boss-cinematics-gnu2-and-motion.md), [semantic-part rendering](engine/semantic-part-rendering-and-ragdolls.md).

**Rendered parts measured and fixed in the source (2026-11-14, [owner](game/boss-cinematics-gnu2-and-motion.md#the-giants-motion-gnu-motion-parts)):** none of the giant's clips tweened, so its walk moved its legs 22-26 px at once every 0.08 s while its body slid smoothly; its publisher now tweens its four walking loops as every character's walk is tweened (renderer `81b31ac`).

**Republished 2026-11-17** (on this VM, which may render art since 2026-11-11): the published `giant_gnu_parts.ron` carries the four tweened walking clips (`tween: Linear`); the bundle before it had none.

**Measured at 60 and 144 Hz and fixed 2026-11-18 ([owner](game/boss-cinematics-gnu2-and-motion.md#the-giants-motion-gnu-motion-parts)):** sampled through the runtime's tween, the tweened walk still moved a hoof 26.2 sheet pixels at once every frame: each hoof is redrawn as another raster of itself, and a track tweened only to the same part. A track now also moves to the same picture redrawn (its size, angle and scale, within its own size); the worst move a sample is 5.5 px at 60 Hz and 5.1 at 144 Hz.

**Next action:** watch the walk in the fight on Jon's displays; turning, buck and rider remain as measured (smooth).

The separate [SUP1 source finding](engine/slopes-feet-ride-surfaces.md#sup1--keep-a-changing-supports-material-motion)
concerns physical support history and source phase. It does not establish
the cause of an observed visual hitch or invalidate the interpolation
receipts above. MOVING-SUPPORT owns the correction; this row retains the
visual product gate.

**Acceptance:** walking, turning, buck, rider movement and portal strikes are visually continuous and align with collision, footholds, hits and camera, across fixed ticks, relevant render rates, reset/replay and quality levels.

### LEADENSADDLE — the Frontier dimension and Marshal Gallus

**Request (Jon, 2026-11-20):** a robot chicken boss between a big chicken and
a person with a chicken's head (a swaggering rooster cyborg with a cowboy
hat and kit), a western biome for him with its own environment and western
music and a boss theme, an industrial spaceport town, reached by the gate
address `LEADENSADDLE`. Original work only: the named films and shows are
mood references.

**Design:** [`game/leadensaddle.md`](game/leadensaddle.md): the dimension
`frontier` (`frontier.ldtk`), the town of Dry Dock in the Leaden Saddle, the
boss Marshal Gallus in the Ore Corral, two music cues (the town's four
sections and the boss theme).

**State (2026-11-20):** slices 1 to 6 are built: the art, the world and its
gate, the fight (Rhai), the music (rendered here, not in this VM's
registry), the residents and their wing of the Hall of Characters.
`fight_discovery` plays the fight through its three phases; the aggressor
policy brought him to 7 of 60 and died, so his defeat and the opening of
the lookout's gate were not reached headless.

**Done 2026-11-20:** the defeat headless and a sealed corral. The way in
shuts behind the player while the Marshal fights (`corral_west_gate`, a lock
wall on the new condition `boss.at_rest`: open while a boss sleeps and once
it is beaten, shut from its intro to its fall). A walk-in from the refinery
lands past the gate (x 92 against the gate's 16-32) before it shuts. The
witness `marshal_gallus::the_marshal_falls_through_his_three_phases_and_opens_the_lookout`
strikes him with real blows between his moves: he passes Phase1, Phase2,
Enrage and Death, his fall is saved, the west gate stands on every frame he
fights, and both gates open after.

**Next action:** play the corral by hand (Jon) and judge the fight. **Done when:** a dial of `LEADENSADDLE`
arrives on Pad Three, the town and the refinery lead to the corral, the
Marshal's fight plays through its three phases headless to his defeat and
opens the lookout, the hall has his practice copy, and each room plays its
own rendered cue.

### SIDING — the Frontier's first slice: Halfway Siding, the Number 9, Clinker Flats

**Request (Jon, 2026-11-20):** a rough outline of the first slice of the
Frontier (`game/techno-western-frontier-first-slice.md`) with its assets:
backgrounds, sprites, music, and a basic assembly into rooms that can be
launched and walked; gameplay beats come later and by hand.

**State (2026-11-20):** built and walkable (see the page's "Built" section):
twelve rooms west of Dry Dock in `frontier.ldtk`, the auxiliary gate
`SIDING`, three new backdrops and skins, 36 scenery pieces, seven residents
and Alice and Bob at the inn, two cues (`halfway_siding`, `the_rustler`)
rendered here and not in this VM's registry.

**Next action:** walk the route by hand and judge the staging (the drop
through the trestle's planks, the cars' gaps, the jump off the Rustler);
decide the slice's gameplay beats with Jon. **Done when:** a dial of
`SIDING` arrives at the station, the inn, the trestle and Sprocket Row are
walked, a drop from the trestle lands on the freight, the freight's head
lets out into Clinker Flats, and both of Clinker's routes reach Pad Three.

### GATE-STARTUP-FLICKER — diagnose the wormhole at startup

**Owner:** [Gate network](game/gate-network.md#startup-rendering).
**State:** OPEN. Jon reports flicker on desktop and Steam Deck. Warmed High and
Ultra captures show an opening flare and continuous steady rendering. They do
not prove that cold startup is correct.
**Found and fixed 2026-11-11 ([startup rendering](game/gate-network.md#startup-rendering)):**
each tier landing at startup rebound the membrane and ring with a new
animator, unordered against the gate's own systems: one frame of the idle
cell, then the stable loop restarted at frame 0 (measured with a probe). The
rebind keeps the cursor and the gate's systems run after it.
**Next action:** a cold-start capture on desktop and the Deck to see whether
any flicker remains (the shader layer is unchecked).
**Acceptance:** the gate renders continuously after its opening on desktop and
Steam Deck. Keep intended opening animation separate from missing frames.

### MULTIPLAYER-SEAMS — make local multiplayer safe for netplay

**Owner:** [local-coop integration](engine/local-coop-integration.md) and
[peer-room-commit protocol](engine/peer-room-commit-protocol.md), across
input, dialogue, view/UI, combat, room lifecycle, and rollback owners.

**State (2026-11-04):** the local isolation slices are built: participant-owned
dialogue, viewport routing, blink clocks, fixed/adaptive split, friendly fire,
shot allegiance, damage-box scope, and per-view diagnostics. Ordinary online
peer crossings now commit on the shared input timeline **without a global hold
or rebase**. Delayed-peer, corrected-combat, concurrent-loading, arrival-reset,
rendered combat, and input-timing witnesses pass (latest `dc303e611`).

**Verified 2026-11-10:** every gate 1-5 guard passes on the current tree,
GPU captures included; the run found and fixed a panic when a camera retired
in the frame Bevy's render debug overlay first saw it
([verification](engine/peer-room-commit-protocol.md#verification-2026-11-10)).
On 2026-11-11 the feature run found that one peer's crossing closed the
other peer's conversation; fixed
([dialogue](engine/peer-room-commit-protocol.md#dialogue-during-another-peers-crossing-2026-11-11)).
On 2026-11-12 a participant's menu was witnessed open and answering its pad
through another participant's crossing, in both orders
([menu](engine/peer-room-commit-protocol.md#a-menu-during-another-participants-crossing-2026-11-12)).

**2026-11-14:** a crossing takes its members out of a live conversation by
the rule every leaving takes (`ActiveConversation::leave`), so a conversation
of three or more goes on when a joined member crosses; the guards above and
`walking_into_a_loading_zone` pass.

**Next action:** the maintainer's rendered two-controller playthrough (a
person at two pads; gate 5) and TEST-LANES' full feature run are what remain
of the **feature and tooling verification** for
input-driven peer room commits. Reproduce a normal crossing and a failed or
invalidated preparation with two peers; audit exact content/plan identity,
participant-specific interactions, replay, view/UI ownership, input ordering,
and cleanup. Keep full-party restores/rebuilds on their distinct global
barrier. Do not replace the native room constructor or reinstate a hold for
ordinary travel. The [protocol owner](engine/peer-room-commit-protocol.md)
contains the case-by-case gates and prior witness names.

**Acceptance:** both participants can continue independent movement, combat,
dialogue, menu and view presentation while one crosses/loads; two delayed
peers agree on application frames and outcomes across rollback, and refuse
mismatched or changed preparation without corrupting source rooms. Run the
focused harnesses and a rendered two-controller main-game playthrough.

**Blocked by:** none.

**Separate work:** menu leave is LOCAL-PLAY-MODES; participant recovery is
COOP-RECOVERY and PARTNER-RECOVERY. Do not duplicate their lifecycle rules here.

### ASSET-PREVIEW — local composition and release audit are separate operations

**Failure:** linked web preview composition hashes every selected input and
then audits the composed tree as if it were a release.

**Owner:** `scripts/package/package_asset_guard.py` and web preview setup.
Design: [asset library review, D](engine/asset-library-implementation-review.md#d-the-web-development-packager-does-release-level-work).

**Done 2026-11-10 (measured):** `compose --policy local` composes the
selected file set without hashing, checks the tree by membership and reports
missing inputs; `build_for_web.sh --served` uses it for a linked tree. An
indexed alias lookup (the old per-declaration scan was 60 of 105 profiled
seconds) speeds both policies: release 59 s to 38.6 s on this checkout, local
22 s ([review, D](engine/asset-library-implementation-review.md#d-the-web-development-packager-does-release-level-work)).

**Next action:** run the Android, Steam Deck and static web release
packagers' final asset contracts on a machine with every generated family;
this VM lacks four, so release composition refuses here before and after.

**Acceptance:** local preview uses the same selected file set without hashing
all media. Missing required files have a useful diagnostic. Android, Steam
Deck and static web release packages still pass their final asset contracts.

### ASSET-PRODUCT-LAYOUT — runtime roots by tier; editor products apart

**Owner:** `ambition_asset_manager` path builders, `ambition_sprite_sheet`,
the publish scripts. Plan:
[`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md#products-are-laid-out-by-what-they-are-q82-q83).

**Ruling:** Q82, Q83 (2026-10-04).

**Current state:** full quality is the bare `sprites/` root and the reduced
tiers are suffixed siblings. ✅ 2026-10-15, the ultrapack's runtime product
is cut by runtime unit: only the pack-plan groups a runtime reader loads ship
(`runtime_groups`), 1.8 MB, was 695 MB; the whole pack is a source in the
renderer's cache (owner page).

**Audit (2026-11-20):** the library's tier roots are named; the logical paths
and every reader still use suffixed siblings (`sprites_0_5x/`), and the packager
drops reduced tiers by a path pattern (`TIER_DIRECTORY`). The definition sites
are listed in the
[owner page](engine/asset-preparation-and-residency.md#products-are-laid-out-by-what-they-are-q82-q83).

**Next action (needs a machine that publishes core art):** move the renderer's
`TIERS`, the runtime readers, `catalog.json` and the accepted publication's
logical keys to `<root>/{full,half,quarter,potato}/` in one change. Then have
the packager select the `full` roots instead of matching `TIER_DIRECTORY`.

**Acceptance:** every tier is a named directory; the packager selects roots
and has no new exclusion; the packaged ultrapack holds only reachable pages;
the game draws the same at each tier.

## Receipts

Closed rows retained only where an open row, a script or an inbound link still needs their identity.

### BOSS-CHARACTER — one character path for bosses — ✅ DONE 2026-11-21

Done 2026-11-21: a boss is an instance of its prepared character; each profile names its `character`, the pack refuses a name no row answers, the Hall shows every boss character, and the inspector bridge is deleted. Owner: [boss character unification](engine/boss-character-unification.md).

### RADIANT-NPC-HTN — authored NPC lives and replaceable planning — ✅ DONE 2026-11-17

Done 2026-11-14 (slices 1–7; closed 2026-11-17): Nikobolt and Coulomba follow text-authored HTN schedules, share the lab affordance, replan, and persist through departure, re-entry and save/load. Owner: [radiant NPC planning](engine/radiant-npc-planning.md).

### BOSS-CINEMATIC-COVERAGE — give every boss a Wolfenstein-class name intro — ✅ DONE 2026-11-17

Done 2026-11-17: every boss shows its name card after it wakes (the encounter's `BossNameCardCue`, or its conductor's), in each local view of its room, and bespoke intros are not doubled. Owner: [boss cinematics](game/boss-cinematics-gnu2-and-motion.md).

### GNUTON-GNU2-FINALE — summon a second GNU through a real portal — ✅ DONE 2026-11-19

Done 2026-11-19: GNU2 is called through a real portal pair and fights beside GNU-ton; the space phase is retired. Since GNUTON-POLISH (2026-11-21) GNU2 escorts GNU-ton without a bar of its own. Owner: [boss cinematics](game/boss-cinematics-gnu2-and-motion.md).

### MOVING-SUPPORT — retain a support's shape motion

Done 2026-11-25 for the no-portal scope: a changing support publishes its start shape (`SurfaceChain::shape_before`), and every native reader (carry, prior support, ledges, point velocity) uses it. The portal cases are PORTAL-SIMILARITY's SUP1. Owner: [slopes, feet and ride surfaces](engine/slopes-feet-ride-surfaces.md).

### AUTHORED-PATHS — share geometry and retain movement authority

Done 2026-11-25 (PATH1): core's `PreparedPath` is the one path law (prepared geometry, a seeded phase, interval proposal and cut); hazards and platforms use it, and both old evaluators are deleted. Owner: [kinematic world objects](engine/kinematic-world-objects.md).

### ITEM-CONTENT — remove retired items without letting fixtures retain them — ✅ DONE 2026-11-17

Done 2026-11-17: the Javelin and the Sealed Note are gone from the item grid, `items.ron`, the intro dialogue and every test that granted them; their slots are reserved empty cells. Owner: [ability loadout](game/ability-loadout.md).

### CHARGEN-SPECIES — species parameters and asymmetric overlay views — ✅ DONE 2026-11-19

Done 2026-11-19: species heads expose their own parameters, and one-sided details keep their side in mirrored cells. Owner: [species knobs and overlays](game/chargen-species-knobs-and-2d-overlays.md).

### ASSET-SELECTION — one selected-file contract for runtime and tools — ✅ DONE 2026-11-17

Done 2026-11-10 (closed 2026-11-17): the Rust runtime reader and the Python packaging reader pass the same six selected-file cases, and the inspector finds music, the registry, the SFX bank and fonts through the selected set. Owner: [asset library review](engine/asset-library-implementation-review.md).

### ASSET-PUBLISH — selection does not audit old bundle payloads — ✅ DONE 2026-11-17

Done 2026-11-10 (closed 2026-11-17): accept is one copy-and-hash pass, publish checks only that a newly selected bundle has its members (1.1 s to 60 ms on 200 bundles), and deep payload inspection is the explicit `Store.audit()`. Owner: [asset library review](engine/asset-library-implementation-review.md).

### GEOMETRY-OBSERVABILITY — render the effective geometry behind gameplay — ✅ DONE 2026-11-17

Done 2026-11-17 (`ce8fbaf68`): F1 and headless JSON report the same effective collision, combat, interaction, portal and navigation geometry, with room, subject and tick context. Contract: [developer tools](../systems/developer-tools.md).

### ENEMY-NOTICE — complete first-sight beats on the existing brain paths — ✅ DONE 2026-11-18

Done 2026-11-18: skirmishers have the first-sight beat (`SkirmisherCfg::notice_s` from the profile's `tactics.notice_s`), the dilophosaurus and the pirate crew author it, and the crew's notice cues are new recipes. Owner: [enemy patterns](game/enemy-patterns.md).

### WORLD-LAYERS — fold the remaining long dimensions into connected regions — ✅ DONE 2026-11-18

Done 2026-11-18: the long Jungle and Magical worlds are folded into connected regions through the existing portal and gate machinery. Further arches wait for a traversal distance that warrants them. Owner: [world layers and travel](game/world-layers-and-travel.md).

### Music playback — owned handoffs and prepared entrances

Done 2026-11-23, with the continuation review's follow-through: music handoffs are owned and entrances prepared. Owner: [music programs and owned playback](engine/music-programs-and-playback.md); [diagnostics](../systems/music-playback-diagnostics.md).

### SCALE-FEEL — the same body behavior at a different scale

Done 2026-11-25 (SR1): the frame resolver converts each body's gravity response to its material scale, and an underivable response is a refused frame the kernel will not step. Owner: [scale-relative gameplay](engine/scale-relative-gameplay.md).

### DOOR-EDGES — outdoor joins that are still doors

Done 2026-11-16: every outdoor pair of the other worlds keeps its door, each with a stated reason ([verdicts](game/hobford-polish.md#door-census-of-the-other-worlds-row-5-2026-11-14)). Rule: [level design guide](../authoring/level-design-guide.md).

### SMASH-DEFECTS — select-screen, respawn and result lifecycle

Done 2026-11-10: SD-01 to SD-05 (off-page portraits drawn, default air jumps at respawn, a result clock that ran while paused, a stale mouse at route entry, invisible page controls), each with a headless test at its owner that a poison of its fix reddens. SD-06 (47 select-screen components marked changed on every idle frame, now 0) and SD-07 (one portrait resolution per occupied card per frame, now 0; a failed load restarted every frame, 8 ms a card on a VM missing two portraits) measured before and after; the windowed UI-layout saving of SD-06 is unmeasured and not claimed. Owner: [Smash defect review](demos/smash-defect-review-2026-10-31.md).

### NET-INPUT-WIDTH — remove idle room consensus from every frame

Done 2026-11-10: `PeerInput` is 80 bytes at wire identity 9 (was 168); the 96-byte verdict is an 8-byte token compared, never decoded, by each peer. All `two_peers` agreement cases pass. Measured link traffic fell only 0.5% in steady play and 4% across crossings, because GGRS already delta- and run-length-encodes inputs. Owner: [compact peer input](engine/island-rollback-and-compact-peer-input.md#result-2026-11-10).

### LOADOUT-AUTHORITY — equipment changes enter through simulation input

Done 2026-11-10: both menus turn an equip or a stow into a stamped host intent (`ItemEquipRequested`, and the new `ItemStowRequested`); the simulation applies an ability to the worn set (`apply_item_equips`) and a weapon or the portal gun to the hand (`ambition_held_items::apply_hand_requests`), on the body the requesting participant drives. The menu writes no body. Witnesses: `hand_request_tests` (own hand only, missing or ambiguous driver, portal pair kept), the menu parity tests through the real applier, and app_it `a_menu_equip_is_a_stamped_request` (equip and stow replay to one hand under the sync test). Schema 433. Owner: [ability loadout](game/ability-loadout.md#open-for-bombs).

### PARTY-TRAVEL-POLICIES — configurable group crossings and a leader view

Done 2026-11-04 (`4cb723b8c`, `5ed9bb73a`): independent/leader/any/all-member crossing policies, bidirectional gate idle timing, native body transfer and leader view are implemented. Default remains independent adaptive play; keep camera geometry out of travel authority. Owner: [game multiplayer](game/multiplayer.md).

### SHOT-END-REPLAY — a brain that saw its shot end replays differently — ✅ DONE 2026-10-11

Done 2026-10-11: the replay inconsistency was a stale body-motion read on rewind, not shot-end identity. See [integration audit](triage/2026-10-11-integration-source-audit.md).

### A4 — separate control authority from body execution on the real schedule — ✅ DONE 2026-10-09

Done 2026-10-09: accepted participant control is separated from body execution on the real simulation schedule. See [simulation authority](engine/simulation-authority-and-determinism.md).

### AUTHORITY-POLISH — one owner per mechanical fact, and no mirror in the rollback kernel — ✅ DONE 2026-10-09

Done 2026-10-09 (C11): no second mirror is retained as an authority for the surveyed mechanical facts. Reopen only for a concrete duplicate. See [consolidation plan](consolidation/consolidation-plan.md).

### SETTINGS-ROLLBACK — a settings change reaches simulation only at an admitted rebase — ✅ DONE 2026-10-08

Done 2026-10-08: settings reach simulation only through admitted rebase. Keep per-scope settings out of unvalidated mid-session writes.

### AUTHORED-INTERACTABLE-STATE — facing gates, per-chest and per-pickup persistence — ✅ DONE 2026-10-08

Done 2026-10-08: facing gates and chest/pickup state use explicit per-object persistence. See [authoring and tools](engine/authoring-and-tools.md).

### ID-PEER — remove host-local lineage from peer-stable mechanical identity — ✅ DONE 2026-10-03

Done 2026-10-03: peer-stable mechanical identity no longer inherits host-local lineage. Preserve the local/canonical identity boundary; consolidation ledger and checks depend on this closed row.

### A10 — candidate world / last-good-world publication — ✅ DONE, DEMOLITION CLOSED 2026-09-16

Done 2026-09-16: candidate-world and last-good-world publication are complete. A failed candidate cannot mutate the live published world. Keep this closed row for the consolidation-ledger guards.

### A12 — finish move-contact attribution and reflection identity — ✅ DONE 2026-09-19

Done 2026-09-19: move-contact and reflection attribution use occurrence identity; an outcome for another/no occurrence grants no move credit. This A12 is not the separate frontier A12 flow-validation task.

The `00bf92c22142` static review identifies a current body-strike
construction and early-overlap gap in that rule.
[CR1's construction follow-up](engine/combat-reception-and-injury.md#carry-attack-origin-through-strike-creation-and-contact)
owns the bounded correction. Preserve this historical receipt and reflection
work; the audit does not establish when the gap entered the source.
