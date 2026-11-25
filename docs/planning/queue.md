# The queue — live execution order

This file contains **executable, unresolved engineering work**, not a work log.
An item remains here only with a reproducible defect or concrete customer,
a next action, and a checkable acceptance condition. Re-measure the source
before starting a row; retire it upon completion. Focused owner documents
carry architecture, measurements and implementation history; Git history
carries closed campaigns. Unselected or customer-gated plans live in
[`tracks.md`](tracks.md), and product questions in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

## Current order and selection

00. **Part-first publication (Jon, 2026-11-18: the top task; done 2026-11-20).** A character
   the game draws from parts is still painted whole, frame by frame, at
   every publish: 472 sheets, 73,531 frames and 1.22 GB of baked pages the
   game never loads. Publish characters as parts only, paint full frames
   only on request (the reference sheet, not `regen/assets.sh`), and let the
   asset inspector play animations from parts:
   [PART-FIRST](#part-first--publish-characters-as-parts-not-frames---done-2026-11-20) and its
   [plan](engine/part-first-publication.md). Every row below waits on it.
0. **Getting the intro right (Jon, 2026-11-14: the top task).** World polish
   of level 1 (Hobford) comes before every row below: make the world rich and
   believable, natural vertical movement from the hills, doors only into
   interiors, scenes staged for the camera and the dialogue. Plan and state:
   [Hobford polish](game/hobford-polish.md). Return to the rows below only
   when there is nothing left to improve there. Its scripting needs (actors
   that walk, leave and pose; cutscenes that run commands and wait on
   conditions; lasting presence; the raid's survivors fleeing with the case)
   are [story sequences](engine/story-sequences.md): SS1 (actor commands),
   SS2 (cutscene `Command`/`WaitFor` beats) and a first SS4 (`starts_when`)
   landed 2026-11-14 with the raid's last raider running out through the
   lane with the case (since 2026-11-17 a brute, beaten, bombs the Creator
   and takes it); SS3's first part (a death in a scene stays: the
   Creator lies where he fell on a later visit) landed the same day, and
   its `present_when` landed 2026-11-17 (the game show's goat and car are
   actors in their closets); SS5 (cutscene camera pans that
   hold until released) landed the same day, as did `actor.face`/`actor.pose`; SS6 (retreat) is
   covered by SS1+SS4 (the raid's escape is one). These first slices are
   done; SS7/SS8 below complete playback identity, role binding and gameplay
   settlement. Dark
   rooms and the first tunnel (under Hobford, the mill wheel to the hollow
   oak) are [dark rooms and tunnels](engine/dark-rooms-and-tunnels.md): the
   player's glow by quality tier, DR1 (room darkness: the catacombs and
   the ossuary), TN2 (the salt run under Hobford) and DR2 (the light map
   at High: players, `LightSource` lamps, and whatever carries
   `CastsLight`, which actor glows and fires do) and DR3 (Ultra's shadows:
   light stops at the room's solid ground), TN3 (the Lantern, found in
   the salt run) and TN1 (the navigation graph plans over painted ground)
   are done (2026-11-16): the plan is complete.
0b. **Script-first Ambition gameplay.** Follow the
   [capability program](engine/script-first-ambition-gameplay.md) for new
   flagship gameplay. The first slice adds the Boss of the Game's scripted
   encore jets and the Lightning inventory skill (`31443e23b`). The
   [capability census](engine/script-first-capability-census.md) records the
   existing contracts and the next useful gaps. Keep working native game
   mechanics. Do not use migration counts or port counts as acceptance.
   The [authoring recipe](../recipes/scripted-stage-attacks.md) describes the
   built-host edit loop and domain authority.
   **Retained receipt:** [first-class procedural game authoring](engine/first-class-procedural-game-authoring.md)
   proves A1–A10 on desktop Linux (`9c1ccf62d`): authored mechanics,
   three-backend parity, saved-state refusal, release delivery, and measured
   promotion. Keep its fixtures. The older
   [runtime authoring audit](engine/runtime-authoring-audit-2026-11-14/README.md)
   records prior P1–P5 work; its proposed migrations need a separately
   selected product benefit before execution.

1. **Network architecture (maintainer priority, updated 2026-11-10):** [NET-AUTHORITY-EXPERIMENT](#net-authority-experiment--compare-hosted-world-replication-with-ggrs) compares player-hosted authoritative Ambition (with client prediction) against existing GGRS; Smash keeps bounded rollback. [NET-INPUT-WIDTH](#net-input-width--remove-idle-room-consensus-from-every-frame) compacted the GGRS input (done 2026-11-10); its measurement says message count, not input width, drives that backend's link traffic. **Direction set 2026-11-10:** the metroidvania moves to a hybrid model, with rollback kept inside a room boundary while it is easy to maintain and its room-level cost measured, and hosted authority for the world around the rooms: [SIMULATION-ISLANDS](#simulation-islands--room-level-rollback-inside-a-hybrid-world) is selected and runs beside the experiment.
2. **Multiplayer correctness:** [MULTIPLAYER-SEAMS](#multiplayer-seams--make-local-multiplayer-safe-for-netplay) feature/tooling validation;
   then LOCAL-PLAY-MODES presentation (LOADOUT-AUTHORITY and the menu leave
   done 2026-11-10). Honor the
   2026-10-19 ruling order for CHARACTER-FORM, COOP-RECOVERY,
   PARTNER-RECOVERY and DIALOGUE-CHECKPOINT; do not re-implement
   already completed morph-ball visibility, joining, party travel, baseline
   split-view or peer room-commit mechanics.
3. **Selected product/asset fixes (2026-11-01 triage):** ASSET-SELECTION,
   SMASH-DEFECTS (done 2026-11-10), ITEM-CONTENT, ASSET-PUBLISH and
   ASSET-PREVIEW in that order.
   A9's completed profile proof is not an automatic invitation to rewrite the
   capability architecture. Continue AP14 ownership work before lower-priority
   ENEMY-NOTICE (done 2026-11-18), WORLD-LAYERS (done 2026-11-18) and POSE-CONTINUITY; then ABILITY-VARIANTS (its laser sound waits for the SFX bank) and
   CHARGEN-SPECIES (done 2026-11-19).
4. **NPC life and boss product work:** [RADIANT-NPC-HTN](#radiant-npc-htn--authored-npc-lives-and-replaceable-planning---done-2026-11-17) tests text-authored HTN planning and persistent schedules. [BOSS-CINEMATIC-COVERAGE](#boss-cinematic-coverage--give-every-boss-a-wolfenstein-class-name-intro---done-2026-11-17), [GNUTON-GNU2-FINALE](#gnuton-gnu2-finale--summon-a-second-gnu-through-a-real-portal---done-2026-11-19), and [GNU-MOTION-PARTS](#gnu-motion-parts--diagnose-and-fix-non-smooth-giant-motion) are selected implementation customers; follow their owner plans and preserve existing shipped encounter phases until replacements pass.
5. **Developer iteration and inspection:** [GEOMETRY-OBSERVABILITY](#geometry-observability--render-the-effective-geometry-behind-gameplay---done-2026-11-17) is complete: F1 and headless reports expose effective collision, combat, interaction, portal and navigation geometry. Its contract is in the developer-tools system document. [TEST-LANES](#test-lanes--keep-required-test-lanes-executable) and CONTENT-AS-DATA
   remain selected at their stated boundaries (TOOL-DISCOVERY and EDITOR-SHELL
   are complete: [tooling layout](engine/tooling-layout-and-editor-consolidation.md),
   T0-T5; retired from this queue 2026-11-11). Portal, rendering,
   combat and map rows below are independent bounded work, not a second
   priority list; select them after correctness constraints are respected.
   The [A2 source-coverage correction](engine/authoring-and-tools.md#static-source-coverage)
   keeps navigation useful without a running engine: index present source
   roots within their own ignore rules and report unavailable/stale corpora
   instead of zero. It does not reopen TOOL-DISCOVERY.

The 2026-11-21 follow-up selects
[SCALE-FEEL](#scale-feel--the-same-body-behavior-at-a-different-scale) and
[PRESENTATION-COMPOSITION](#presentation-composition--order-appearances-and-add-a-25d-view)
as bounded movement/presentation work within this product order. Fix ordinary
scaled behavior and cart order through their current owners; the optional
3D view follows the shared sampling boundary and does not block those fixes.

The 2026-11-22 Smash review keeps D72 and Smash-only demo polish behind the
main Ambition work. Shared action/time corrections ACT1/ACT2 stay within
ARCHITECTURE-COMPOSITION; shared landing/recoil/clip and source-time binding stays within
POSE-CONTINUITY. Neither selection promotes the full Smash roadmap.
The [inventory order](demos/smash-parity-inventory.md#17-current-implementation-order)
and [feel acceptance](demos/smash-feel-audit-2026-10-29.md) state the later pass.

The 2026-11-22 static moving-world review adds
[MOVING-SUPPORT](#moving-support--retain-a-supports-shape-motion) and
[AUTHORED-PATHS](#authored-paths--share-geometry-and-retain-movement-authority)
as bounded common-engine work within this order. GNU supplies the changing
support customer; shipped hazards and actor routes supply path customers.
Cross-version saved-content compatibility is deferred to 2027 and is not a
near-term audit or implementation priority. Current-session rollback and
native reconstruction keep their existing correctness obligations.

Current source may be ahead of these dated ordering decisions. Do not treat
old measurements as an obligation to redo completed work. Detailed
architecture proposals in
[the reassessment](engine/architecture-reassessment-and-composable-multiplayer.md)
do not by themselves authorize networking/physics/rollback rewrites.

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
   [FORM-REBIND](#form-rebind--admit-and-apply-a-complete-form-on-the-existing-body)
   supplies L2's live form/generation customer and can be corrected before
   broader linking. The independent
   [authored-empty geometry slice](engine/combat-model.md#preserve-an-authored-empty-geometry-sample)
   carries Unauthored/Empty/Shape through the sprite/combat boundary.
   An empty selected strike sample contributes no strike or AI strike
   region; its window entry is completed work without repeated per-tick
   resolution. Keep capture and push coverage separate. Preserve CLIP3's
   time map and the current geometry sampling cut, and the existing pulse
   plan's memory ownership where an empty intermediate segment needs it.
4. [World composition W1/W2](engine/reusable-authored-world-composition.md#implementation-slices)
   uses existing local references, then a different mechanism. General format
   work follows only if the operation is shared. (W1 landed 2026-11-22: the Bridge Warden is made with its binding to the
   posts' local keys; no result indexing or follow-up attach. W2's native
   binding landed 2026-11-24: a `lift_called` lift waits at both stops and a
   `MoveLift` switch in its room sends it. `create_mechanism` landed the same
   day: one scripted request builds lifts and the switches that send them,
   bound by local key, with two copies in one room kept apart. Open: the
   door, which needs a per-instance owner; lift occurrences; the
   second-instance witness, which waits on two concurrent copies of one
   room. Rollback through one child's retirement is witnessed.) Producer-validity delegation
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
   body's automatic exit from a room another seat keeps. What remains is
   ISL-3a's: its transfer footprint consumes this cohort.)
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
   rate. Open: CPU-only and human-seat Smash acceptance.)

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
    road and omniscient target are unchanged. Open: a `TargetBelief`
    direct-evidence budget and retention over time.
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
association once. Preserve the native cursor, hurt-part sample, death
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

**State:** PLANNED (source review at `3ed50e318392`, 2026-11-21).
This is developer-iteration work within the product order above. No new
compile-time, runtime or binary-size result is claimed.

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
549 crates; no current consumer's graph changes). Next, make `game`
default-off in the launch descriptors. Try explicit optional feature families before adding packages.
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

### BOSS-CHARACTER — one character path for bosses — ✅ DONE 2026-11-21

**Done 2026-11-21.** A boss is an instance of its prepared character. Each
profile names its `character`, and the pack refuses a name no row answers.
The Hall shows every boss character, and the inspector bridge is deleted.
The common seed builds the boss body, sized once through the character at
the encounter's `scale` and `max_hp` (`before_sizing`, `size_from_character`,
`compose_boss_role`). The body carries `CharacterForm`. A boss speaks as its
character (`voice` is deleted), and a boss sheet draws its character's image.
The bill name stays the placement's, as an occurrence fact. Headless frame cost
equals the baseline within noise, and the Hall's ordinary characters cost what
they did. The [owner plan](engine/boss-character-unification.md) records
slices 1 to 4 and the decisions.

### SCRIPT-COMPLETION — author gameplay scenarios without an engine build — ✅ DONE 2026-11-20

**Request (Jon, 2026-11-19, above queue order):** finish the reactive scripting road: signals, further lifecycle facts, full duplicated-room gameplay, committed world-domain communication, and the authoring guide ([reactive mechanisms](../sdk/reactive-mechanisms.md)). The aim is gameplay scenarios authored in scripts, with no engine rebuild, at the level of a general-purpose engine.

**Owner:** [addressed behavior delivery](engine/addressed-behavior-inbox.md); [capability census](engine/script-first-capability-census.md).

**State:** done 2026-11-19: accepted defeat is a subscribable fact (`object_defeated`), and so are a spent breakable (`object_broken`, any road that breaks one) and a taken pickup (`object_collected`, naming who took it), and authored signals (`emit_signal` → `object_signal`) are admitted whole or refused, the sender named by the runtime, and the result lists who heard ([guide](../sdk/reactive-mechanisms.md#send-a-signal)). Done 2026-11-19: a room entered again is a new copy, and a watch subscribed in the old copy hears nothing from the new one while a fresh lookup's watch does (`a_room_entered_again_is_a_new_copy_and_only_its_own_watch_hears_its_barrier`). Done 2026-11-19: the committed world road: `post_world_signal` is held until the sending tick is confirmed, admitted on the next live tick to each subscriber wherever it is then, and queued again when the subscriber's room replays the admission tick (`a_world_post_crosses_rooms_once_and_only_when_confirmed`, [guide](../sdk/reactive-mechanisms.md#post-a-signal-to-another-room)). Measured 2026-11-19: no production road holds two live copies of one room. A crossing into a live room joins it (`ambition_platformer2d_publication/src/transaction.rs`: "A crossing that joins a live room mints nothing"), and a copy is minted only when a retired room is entered again, which the re-entry witness covers; equal placement keys across instances are covered by the shared locator check. Done 2026-11-20 (C6): a retirement by any road is `object_retired`, published at the tail of the first tick a subscribed source no longer resolves, once per subscriber, with what each heard kept as rollback state on it, so a creator's retirement (`retire_creation`), an owner's despawn and a room's teardown are one fact (`scripted_retirement`).

**Acceptance:** each item has a loaded Rhai witness through the real host, with rollback health, and its section in the authoring guide.

**Program completion (Jon, 2026-11-20: "It is the top priority to finish that completely"):** done 2026-11-20. Slices C1–C7 of the [completion plan](engine/script-first-ambition-gameplay.md#completion-plan-2026-11-20) are done, and the [handoff](engine/script-first-ambition-gameplay.md#completion-handoff-2026-11-20) gives each gate item its evidence. The capstone is the playable [bridge warden](../../examples/projects/bridge_warden/README.md) project.

### SCRIPT-ROOM-ONCE — a room's setup without a per-tick story entry — ✅ DONE 2026-11-20

**Found 2026-11-20 by the bridge warden's [cost](../../dev/benchmark-candidates/bridge-warden-cost.md):** a room that sets itself up once per visit can only do it from `story_tick`, which runs every tick the room is live and carries every body, flag and mark of the room. The yard's entry returns at once after its first tick and still costs about 0.45 ms a tick in a debug build, 98% of the project's script cost.

**Owner:** room story (`crates/ambition_platformer2d_runtime/src/story_script.rs`).

**Next step:** a trigger the room owner publishes once when a room instance goes live (and again on a fresh copy), rollback-safe: a rewind across the first tick must run it again, and a replay must not run it twice. Check first whether the room clock already holds a first-tick fact that can carry it with no new state.

**Acceptance:** the bridge warden's yard uses it; its cost row loses the per-tick entry; a sync-test witness shows one setup per visit.

**State:** done 2026-11-20. `story_began` (`ambition.story.began`) is the room story's once-per-instance trigger: the same facts and scope as `story_tick`, invoked the first running tick of each live instance. The room root carries `StoryBegun` (canonical rollback state, GGRS schema 521) beside the records the setup wrote, so a rewind across the first tick runs it again with the state it rewound. The bridge warden's yard uses it: the project's entry calls fell from 966 to 42 on the sync-test host (472 ms to 15 ms) and from 133 to 5 on the game's session ([cost](../../dev/benchmark-candidates/bridge-warden-cost.md)); `the_bridge_warden_project_is_played_through_by_its_runner` shows one warden and two posts through the rewinds and a fresh set on a second visit. Guide: [room story scripts](../recipes/iterating-without-a-rebuild.md#room-story-scripts).

### INUGAMI-CHASE — a spirit you run from — ✅ DONE 2026-11-19

**Request (Jon, 2026-11-19):** it got stuck and the rubber band did not feel like running from it. No collision in the chase, homing on you; it swells until you cannot get around it; her talismans shrink it to a size you fight.

**State:** done 2026-11-19. Warded, it is a spirit: `ambition.boss.intangible` lowers into the body fact `PassesSolids`, so its own movement meets none of the room's solids (the movement kernel stepped it into the arena's walls before, whatever the pose said). It homes in a straight line at your measured pace, corrected toward a gap from its face, so a runner at full tilt holds it just behind and one who stops is caught. Warded it swells to fill the passage; her talisman break shrinks it back. Its arrival into the final hall is a flight, not a snap.

**Reopened and fixed 2026-11-19 (Jon: "it still just gets stuck. It doesn't chase you"; "a development test ... not one that runs in ANY test suite").** `game/ambition_app/examples/inugami_e2e.rs` plays the whole fight through the `wolfenstein` autoplay (`autoplay.ron`: the fireball, flight, and an `assist` of endless health and a fifth of a boss's health more per landed blow). It prints each of 19 steps as it comes and fails on a late step or a stalled chase. It found two defects. First, the crash: between phases the boss pattern's move outranked it, so the head stalked instead of flying at the wall, and the 2.4 s beat could end first; the run began in the arena's middle. The crash now finishes on its own clock, and no move starts while it rears. Second, the run's lunge was held where it began: for 1.6 s the head sat still while the robot fled at 725 px/s. Its gather and skid now keep after you, and its dart starts where the gather ends. After the fix the head holds 220 px off the runner's back from the arena, down the drop and west into the hall; then Kagero, the finish, hellfire and the cage, all 19 steps in 82 s. Run `cargo run -p ambition_app --example inugami_e2e`; watch it with `AMBITION_AUTOPLAY=wolfenstein ./run_game.sh -- --start-room castle_godwin_throne`.

### PART-FIRST — publish characters as parts, not frames — ✅ DONE 2026-11-20

**Defect (measured 2026-11-18):** the publisher paints every frame of a part-drawn character whole, packs and tiers the sheets, and paints each frame again to check its draws, though the game never loads those pages: 472 sheets, 73,531 frames, 1.22 GB. `gnu_ton_boss` alone takes 93 minutes; a GNU2 frame costs 2.5 s, nearly all rotation onto a frame no player sees.

**Owner:** [part-first publication](engine/part-first-publication.md).

**Next action:** PF0 and PF1 are done, and PF2's module road (2026-11-18: 414 sheets publish page-less by `build_sheet(parts=True)`, all 434 audited against every painted frame, boss art excepted; [state](engine/part-first-publication.md#state-2026-11-18)). Done 2026-11-18: PF3 (`sprites.sh --reference`; the asset inspector plays page-less sheets from parts), the generator road (config sheets recorded once, page-less, tiers derived), and PF2b content (nine boss sheets drawn from their characters' parts; a faceless part-drawn boss flinches whole; a page-less boss sheet resolves by its declared path). Done 2026-11-18: Noether, player_robot_v3 and Mary-O v2 record once and publish page-less (the robot kept its page until the LDtk `PlayerStart` tileset was retargeted, done 2026-11-20). Done 2026-11-18: `sandbag` records once and publishes page-less; GNU-ton's giant mount records its parts before the layer pass and publishes page-less, drawn from parts on the boss road; GNU2, Jeff Hinter armored and Fukuzawa publish page-less too, their rigid pieces composed into one part each (a bone's plates, a segment's armour) instead of drawn one by one. Done 2026-11-19: PF6, the full sprite republish on the agent's VM (1,297 builds in 4,675 s wall at 6 jobs; the store's sprites from 2,587 MB to 1,275 MB, sheet pages from 1,920 MB to 600 MB: [result](engine/part-first-publication.md#state-2026-11-18)). Done 2026-11-20: the LDtk tileset retarget, [Q180](awaiting-maintainer-decision.md#q180--what-should-the-ldtk-worlds-playerstart-icon-point-at-so-the-player-robot-can-publish-without-a-sheet-page) option (a) taken as a reversible assumption: all 20 worlds draw `PlayerStart` from the robot's portrait page, and `player_robot_v3` left `baked_pages`; republished 2026-11-20 (176 s), the store holds its parts, portraits and record at every tier and no sheet page. PF4 is done per target (2026-11-19): an in-between oracle of each rig chose its tweened clips (906 more clips tween in 115 targets; 24 default tweens that misplace limbs step; a mirrored limb no longer tweens through zero width, and a clip played once holds its last frame), declared as `tween_rows` in `sprites.yaml`; the clips of rigs that cannot pose an in-between keep the default ([result](engine/part-first-publication.md#state-2026-11-18)). PF5 is closed by measurement (2026-11-19): the Art cast kit converted to rigid pieces replays every frame exactly but costs 1.1–2.3x the trimmed page at 13–55 draws a frame, so the kit keeps its pages ([result](engine/part-first-publication.md#state-2026-11-18)).

**Acceptance:** the plan's PF1–PF6: no baked pages for a part-drawn character, full reference sheets only on request, the asset inspector animating from parts, and a full regeneration's time and bytes measured before and after.

**State:** done 2026-11-20. Every character drawn from parts publishes page-less. What still keeps a page says why in `sprites.yaml` `baked_pages`: five boss sheets that no catalog character draws (the Clockwork Warden and the Hooded Sentinel have part flipbooks but no character row to demand them by; the Gram Reaper, GNU-ton's scholar and Marshal Gallus publish no part flipbook), and the cosmic Mary-O by design (a hybrid). The art kit and props keep pages by PF5's measurement.

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

### RADIANT-NPC-HTN — authored NPC lives and replaceable planning — ✅ DONE 2026-11-17

**State:** done 2026-11-14 (slices 1–7).

**Customer:** civilian NPCs need schedules, commitments, social interactions, smart-object affordances and meaningful offscreen life; the current `CharacterBrainTemplate` policy catalog is closed. Nikobolt and Coulomba share one laboratory and are the initial integrated customer.

**Owner:** [Radiant NPC planner](engine/radiant-npc-planning.md), [agentic character runtime](engine/agentic-character-runtime.md).

**Done 2026-11-10 (slice 1):** compared `bevy_htn` 0.0.1 (Bevy 0.15, text domains, reflected component state, random plan ids) and `bevy_bae` 0.1.0 (Bevy 0.18, Rust-bundle domains, `World` properties) with a standalone core, and chose the core: `crates/ambition_htn` (no Bevy; RON domain validated when read; facts in, an explained plan out; 4.9 us per plan on the scientists' benchmark day) ([result](engine/radiant-npc-planning.md#slice-1-result-2026-11-10-a-standalone-core)).

**Done 2026-11-10 (slice 2, first part):** a body can be sent to a place: `Visit` (rollback state), answered by the navigation advisor and settled `Arrived` / `NoRoute`; the basement dog witnesses both ([result](engine/radiant-npc-planning.md#slice-2-first-part-2026-11-10-a-body-sent-to-a-place)).

**Done 2026-11-10 (slice 2):** the executor (`body::life`): a game's `LifeBook` (Ambition's pack file `data/npc_lives.ron`, schema `npc_lives`, republished by a content reload) gives a character a `Life` (rollback state); it plans on each new hour from its own facts, each `go_to` is a `Visit` to a place the room names (signs `workbench` and `lunch` in the coil lab), other verbs hold for their authored hours, and a step done makes its effects facts. Nikobolt works at the bench and lunches at noon; Coulomba (no life) is the control; the day resimulates under the bench hand-over's sync test ([result](engine/radiant-npc-planning.md#slice-2-2026-11-10-nikobolt-lives-his-day)).

**Done 2026-11-10 (slice 3):** two scientists, one bench: a claim is simulation state (`Life::claim`; a lost claim replans at once and the life keeps why), observed facts (`Free`, `Present`) are read from the world when a body plans, and a spent plan replans when what it observes changes. Never two holders at once; both get a turn; the hand-over survives a sync test ([result](engine/radiant-npc-planning.md#slice-3-2026-11-10-two-scientists-one-bench)).

**Done 2026-11-10 (slice 4):** interrupt and replan: a conversation stops the talked-to body's plan (outside the gameplay gate, which dialogue stops) and it replans when the conversation ends; a refused route or a place gone replans at once and leaves `unreachable:<place>` / `absent:<place>` for the domain; each new plan says why ([result](engine/radiant-npc-planning.md#slice-4-2026-11-10-interrupt-and-replan)).

**Done 2026-11-10 (slice 6):** an edit to `data/npc_lives.ron`, published by a content reload, changes Nikobolt's next plan in the running shipped game with no Rust build; hours are `authored` facts read from the book at each plan ([result](engine/radiant-npc-planning.md#slice-6-2026-11-10-an-edited-life-in-the-running-game)).

**Done 2026-11-10 (slice 5, first part):** offscreen and re-entry: a `LifeLedger` keeps each character's facts and plan hour; a body built again catches up the hours it missed by schedule arithmetic and is put where the schedule sent it. Away from 9:30 to 19:30, Nikobolt is at the door gone home with 7 experiments and Coulomba at the bench with 8, both fed ([result](engine/radiant-npc-planning.md#slice-5-first-part-2026-11-10-offscreen-and-re-entry)).

**Done 2026-11-10 (slice 5, second part):** the lives are in the save (v6) and come back in a session started from it; a session without it is the control ([result](engine/radiant-npc-planning.md#slice-5-second-part-2026-11-10-the-lives-in-the-save)).

**Done 2026-11-10 (review findings 2, 3, 4, 6, 8):** `daily` facts start each day again (day-2 lunch witnessed); `say` speaks a line and a verb neither the executor nor `holds` knows is refused when the book is read; the save is written only when a life changed; the executor plans through the `ambition_life_planning::LifePlanner` contract (the HTN is one planner); offscreen hours book an exclusive thing by the hours its holders' steps take, in character-id order ([offscreen](engine/radiant-npc-planning.md#slice-5-first-part-2026-11-10-offscreen-and-re-entry)).

**Done (slice 7, recorded 2026-11-14):** hosted authority. The experiment chose the hybrid (NA-4), and its plan row (2026-11-13) is the slice: the host plans, and a sleeping guest holds each life as the host's row, not a planner (`a_hosted_guest_holds_the_hosts_lives`, [result](engine/hosted-world-networking-experiment.md#an-npc-lifes-plan-row-2026-11-13)). Every clause of the acceptance now has a witness.

**Next action:** none in this row. Done 2026-11-10: the day clock is in the save (v7), so a session from a save goes on at the hour its lives planned in. Done 2026-11-10: offscreen bookings outlive the tick in the life ledger, so bodies that come back on different ticks share a thing's hours. Done 2026-11-10: the bookings are in the save (v8). Done 2026-11-11: a live holder's hours are booked, for the time held, for a body that catches up.

**Acceptance:** both scientists follow different authored schedules, share an exclusive lab affordance, react/replan, persist on departure and reentry, explain their plans and remain correct under save/reload and hosted authority. Change their activity policies without an engine-host compile. Do not require LLM inference or all-peer rollback.

### BOSS-CINEMATIC-COVERAGE — give every boss a Wolfenstein-class name intro — ✅ DONE 2026-11-17

**State:** the roster-wide card landed 2026-11-10 (every boss shows its name after it wakes: the encounter's `BossNameCardCue` by default, its conductor's for Wolfenstein and the Inugami), and each local view shows the card of the boss in its own room (2026-11-10; see the [owner](game/boss-cinematics-gnu2-and-motion.md#name-cards-boss-cinematic-coverage)). Done: the rendered witnesses below are the acceptance's evidence (two non-Wolfenstein fights, and two views).

**Owner:** [boss cinematic and GNU2 plan](game/boss-cinematics-gnu2-and-motion.md), [boss content](game/bosses.md).

**Rendered 2026-11-10:** a GPU capture witness draws the card for two non-Wolfenstein bosses (the T-rex and the Clockwork Warden), counted in the name's own rectangle against a capture before the wake ([owner](game/boss-cinematics-gnu2-and-motion.md#name-cards-boss-cinematic-coverage)).

**Rendered two views 2026-11-10:** each of two split views in two boss rooms draws its own boss's card in its own half ([owner](game/boss-cinematics-gnu2-and-motion.md#name-cards-boss-cinematic-coverage)). The same captures showed the boss health bar, a title banner and a dialogue box spanning both views; fixed 2026-11-14 (a bar per view; the stale banner gone).

**Next action:** none on the card. No skip input exists; if one is added, it advances the phase state and the card follows.

**Acceptance:** each active boss gets a readable cinematic name/title reveal, at correct intro phase and view, with safe skip/replay/cleanup; bespoke intros still work without double presentation. Roster census and rendered witnesses cover multiple non-Wolfenstein fights.

### GNUTON-GNU2-FINALE — summon a second GNU through a real portal — ✅ DONE 2026-11-19

**Failure:** GNU-ton's phase-3 well uses a real floor/overhead portal pair but changes the existing hall's sky and gravity to represent space. The preferred new finale is a large live portal summoning physically present GNU2, followed by simultaneous GNU and GNU2 combat. GNU2 is published and catalogued (`npc_giant_gnu2`, 2026-11-18): the giant's rows, seams and surfaces in its harness.

**Owner:** [boss finale and acceptance](game/boss-cinematics-gnu2-and-motion.md) and [GNU-ton debts](game/gnu-ton-fight-debts.md).

**Steps 2-6 done 2026-11-19 ([owner](game/boss-cinematics-gnu2-and-motion.md#gnu2s-finale-gnuton-gnu2-finale)):** GNU2 sleeps in a sealed cell under each arena's floor, wakes when GNU-ton enrages (`successors_wake: Enraged`), opens its own held pair and walks through it into the hall. It has its own conductor (`gnu2`: walk, stomp shocks, buck), the boss bar stacks every fought boss, and each arena's way out waits on both (the handle has a grille for each). The scholar's enrage now calls GNU2 instead of launching to space.

**Step 7 done 2026-11-19:** space is retired. The well, the ascent, the tides, the helmets and the space sky (`presentation/gnu_ton_sky.rs`) are gone, and the conductor's record is v2 (its call clock replaces the launch's).

**Next action:** none; watch the two-giant fight on Jon's displays.

**Acceptance:** physically observed GNU2 portal emergence, simultaneous independently damageable GNU/GNU2 giants with readable counters, either defeat order supported, door gating on both plus rider yield, no duplicate spawn on retries, and both bosses safely cleaned on teardown. Keep intra-island portal semantics and existing phase 1/2 gameplay.

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

### MOVING-SUPPORT — retain a support's shape motion

**State:** PLANNED, static source review at `00bf92c22142`, 2026-11-22.

**Failure / customer:** GNU's drawn hump maps its current polyline by an
endpoint similarity, including length change. Its published motion keeps
only displacement and turn. Prior support, carry, point velocity and swept
bounds then describe a rigid history that cannot match the real samples.
The source phase is also predicted as `PinnedRow.elapsed + 1/60`, although
the walk uses a distance-based clock. These are source deductions; no new
runtime reproduction or speed claim is made.

**Owner:** [SUP1](engine/slopes-feet-ride-surfaces.md#sup1--keep-a-changing-supports-material-motion).
Native support geometry keeps the current endpoint-similarity law, source
material coordinates and the source owner's actual phase span. M1/M2 owns
accepted host motion and chart changes; P3/M3 owns contact response. The
full curved hump's axial art correspondence is a separate authoring choice.

**Next action:** extend the current length-changing mount fixture through
true previous/current geometry and material-point carry. Move the producer,
prior-support query, carry, ledges, material contact, velocity and bounds
to one bounded interval. Make the native source-phase/host publication cut
coherent; `+configured_dt` is not a fix for another owner's clock. Keep
static/translation/rigid fast paths and delete the lossy reduction for
changing shapes. The local slice can land before all persistent support
attachments or portal work; moving/portal completion needs their actual cut.

**Acceptance:** one native expanding/shrinking support and one GNU source
transition preserve the same material place, full swept extent, contact
time and velocity. A ledge or frozen rider consumes that same motion.
Row/facing/availability cuts never become fabricated velocity. Ordinary
rigid traffic gains no general deformation solve or per-rider shape copy.
The existing visual play gate remains separate.

### AUTHORED-PATHS — share geometry and retain movement authority

**State:** PLANNED, static source review at `00bf92c22142`, 2026-11-22.

**Failure / customer:** combat and world duplicate waypoint evaluation.
Both admit a coincident-point cycle that consumes no distance and never
ends. The admitted initial offset is unused. Two shipped actor routes
attach path state, but movement discards its advance and the patrol brain
does not use it. Shipped moving hazards provide the first shared-law customer.

**Owner:** [PATH1/PATH2](engine/kinematic-world-objects.md#path1--prepare-one-spatial-path-law),
with [LDtk authoring](engine/ldtk-authoring-and-world-tools.md).
Core owns prepared spatial geometry and traversal. Platforms, hazards and
brains keep admission, damage and steering policy respectively.

**Next action:** PATH1 prepares finite geometry, seeds prescribed-mover
pose/phase together and provides a pure interval with sampling and cutting
before native commit.
Reduce complete cycles for endpoint lookup without erasing actual travel.
Migrate hazards/platforms and delete both old evaluators. PATH2 separately
makes the two grounded routes produce ordinary intent and progress from
accepted arrival under the selected patrol/controller policy. Keep the
actor's admitted spawn place. Delete its discarded post-movement advance;
do not assign a path endpoint directly to an actor. Keep wrapped platform teleport out of
carry. No general data-structure library or motion interpreter is selected.

**Acceptance:** admitted geometry cannot cause zero-progress traversal;
nonzero initial phase has one native meaning; restored continuation gives
the same next result. One real LDtk-to-hazard publication proves preparation
and seeding. One route-driven actor reaches/reverses at its target and
retains route progress when blocked by a wall. The shared endpoint sampler
does not claim continuous hazard contact or complete coupled portal admission.

### ITEM-CONTENT — remove retired items without letting fixtures retain them — ✅ DONE 2026-11-17

**Done 2026-11-10 for the grid; the held javelin ruled 2026-11-14.** The Javelin and the Sealed Note are gone
from the item grid, `items.ron`, the intro dialogue and every test that
granted them. Their slots became "Empty" reserved cells (`Item::Empty3`,
`Item::Empty4`), as the Axe's and the Map Fragment's were, so no other item's
index or `dialog_id` moved; `Item::Empty4` is the Lantern since 2026-11-16
(TN3). Replacements:

- **The intro handover:** Alice sends Bob a Spare Battery for his survey lamp
  (`intro.yarn`). Bob takes one only once Alice's flag is set, so a battery
  bought at the port is not hers. The flag names (`alice_route_note_carried`,
  `bob_field_survey_received`) are the save's and stay. Witness:
  `alices_battery_reaches_bob` (renamed from `the_note_travels_from_alice_to_bob`),
  with a new control: a bought battery is not offered to Bob.
- **The bag-minted throwable** of the checkpoint and save fixtures: the
  Gun-Sword (`death_restores_the_checkpoint`) and the Grapple
  (`two_persistence_authorities_for_one_item`, whose authored reward is
  already the gun-sword), thrown by Shield+Attack, which throws anything a
  hand holds.

**Usage list, 2026-11-10** (method: whole-word match of each item's
`dialog_id` and `held_item_id` in content files under
`game/ambition_content/assets`, `tools/ambition_ldtk_tools/specs` and
`game/ambition_map_assets`, `items.ron` excluded; an ability id such as `dash`
or `fireball` also matches move and boss names, so its count is an upper
bound, not a list of grants): Fireball 8 files, Eye Laser 1, Lightning 4,
Dash 20, Grapple 20, Mark / Recall 4, Gun-Sword 24, Puppy-Slug Gun 3, Bomb 24,
Health Cell 3, Mana Cell 2, Spare Battery 3 (quests, the port's shop, the
intro), Field Survey 1; Sword Stance, Axe Stance, Whip Stance, Precision
Blink, Contextual Blink, Dash II and Portal Gun name no content file (the
portal gun is placed by its own spawn kind; the stances and blinks are worn
through `items.ron` rows).

**Ruled 2026-11-14 (portals owner):** the held object `javelin` stays. It
is a held-item spec with its own art row, not the removed grid item, and it
is the portal similarity lab's gauge because it is asymmetric
(`tools/ambition_ldtk_tools/specs/portal_similarity_slice.yaml`): a
reflection is read off which end leads, which a round coconut cannot show.
The held item and save unit tests that throw it test a held object, which it
still is. No placement, dialogue, recipe or save reader names the grid
Javelin.

**Owner:** item content, held specs, world placements, intro dialogue and their
behavioral fixtures. Design: [ability loadout](game/ability-loadout.md).

**Acceptance:** the equipment and quest pages reflect actual content. No
placement, dialogue, recipe or save reader names a removed item. The retained
custody and persistent-world witnesses still exercise real production paths.

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
Then Precision Blink's quick press and double press (open 2; what the
single press does is [Q179](awaiting-maintainer-decision.md#q179--what-does-a-quick-press-of-precision-blink-do-if-two-quick-presses-are-the-directional-blink)), and the
stance and Dash II placeholders through the same technique/equipment seam,
not named engine branches.

**Acceptance:** each real variant changes its declared slot on any supported
body. Replacing or removing equipment restores the prepared intrinsic kit.
A placeholder stays visibly unavailable until it has an admitted implementation.
Tiers, equipment and checkpoint/save state agree. Keep quality selection out
of gameplay authority.

### CHARGEN-SPECIES — species parameters and asymmetric overlay views — ✅ DONE 2026-11-19

**Failure:** the overlay mechanism is built, but species heads still expose
mainly muzzle and ear choices. Mirrored left cells move asymmetric details
to the wrong side.

**Owner:** renderer `chargen` specification, species geometry, overlay data and
studio applicability. Design: [species knobs and overlays](game/chargen-species-knobs-and-2d-overlays.md).

**Dog and pigeon done 2026-11-11 (renderer 93d8e69):** the dog has stop,
skull, flews, ear length, fold and set, and a mask, blaze or saddle marking;
the pigeon has beak curve, cere, neck sheen and eye ring
([knobs](game/chargen-species-knobs-and-2d-overlays.md)). Defaults draw the
old heads byte for byte and 3447 records keep every older field; no resident
is a dog and the four pigeon residents pin the defaults, so no published
product changed and nothing was republished.

**Cat done 2026-11-14:** ear size, cheek fluff and tabby stripes
([knobs](game/chargen-species-knobs-and-2d-overlays.md)); defaults draw the
old head byte for byte, older records keep every field, Mina pins the
defaults, so nothing was republished.

**Horse done 2026-11-14 (renderer 85f728e):** face length and a star,
strip, blaze or snip face mark; defaults draw the old head byte for byte,
older records keep every field, and no resident is a horse.

**Bird done 2026-11-14:** a tuft, swept, fan or no crest and its size;
defaults draw the old head byte for byte, and no resident is a bird.

**Sea lion and alligator done 2026-11-14:** whisker pads and whisker
length; snout width and two, four or no scute rows. Nagisa pins the
alligator defaults. A redone resident check (the old one loaded 239 of 243
residents as errors) finds all 243 drawings identical to the renderer
before the cat, and only Mina's and Nagisa's records gain their pins.

**Game scale checked 2026-11-18:** each species (dog twice, pigeon, cat,
horse twice, bird twice, sea lion, alligator, finch), generated with its
knobs at the ends of their ranges, rigged and drawn in eight clips (idle,
walk, run, jump, attack, hurt, talk, crouch) from the views the sheet
publishes. Every head stays on its neck in every frame, and the knobs read:
the dog's low folded ears, saddle and mask, the cat's big ears and stripes,
the horse's long and short faces, the bird's fan and swept crests, the
alligator's wide snout and four scute rows. At this scale the pigeon's cere
and eye ring and the sea lion's whisker pads are barely there; they are
head-crop details, not silhouette. ([knobs](game/chargen-species-knobs-and-2d-overlays.md#game-scale-check-2026-11-18))

**One-sided details done 2026-11-19:** a character with a detail on one
side of the body (`rig.one_sided`: a kimono's wrap, a nihongami's
ornaments, an overlay with `data-figure="mirrored"` cells) is also drawn as
its mirror image (`View.mirrored`), and its sheet gains `<row>~mirrored`
for every row: the mirror image's frame flipped, which the animator draws
for a body facing left. Ten residents are one-sided (seven in kimono, and
André Rebouças, Nikobolt and Coulomba, whose mirrored cells move the watch
chains and the cocked hat's plume, bow and roses); all ten republished, 47
mirrored rows in every quality tier and in the part flipbook. A true left
view (negative yaw) was rejected by measurement: the house shade is on the
screen's left and the hands are not mirror twins, so it differed from its
right twin on 20-30 % of a symmetric resident's pixels. Items held in a
hand stay in the far hand, as on every hand-made sheet
([design](game/chargen-species-knobs-and-2d-overlays.md#as-built-overlays-renderer-chargenoverlaypy)).

**Acceptance:** head crops and posed clips show distinct valid shapes in every
view; heads stay attached and asymmetric features keep their side. Existing
seeds retain their established fields. Publish each changed product and its
quality variants through the current pipeline. No runtime shader redraws the
character or changes gameplay geometry.

### ASSET-SELECTION — one selected-file contract for runtime and tools — ✅ DONE 2026-11-17

**Failure:** the inspector reads loose audio and font paths while the runtime
uses selected publications. Python packaging and Rust also differ on missing
publications and stale generation identities.

**Owner:** `ambition_asset_manager::content_library`,
`scripts/lib/content_libraries.py`, `tools/ambition_editor/asset_index`, and
provider library installation.
Design: [asset library review, A and E](engine/asset-library-implementation-review.md#a-selection-is-correct-in-the-runtime-but-not-uniform-in-the-tools).

**Done 2026-11-10:** the selected-file contract is six shared cases that the
Rust runtime reader and the Python packaging reader both pass (Python now
treats an empty store as an empty selection and a stale identity as an
advisory; the packager states its stricter "has no publication" refusal at
its own boundary). The inspector locates music, the registry, the SFX bank
and fonts through the selected set and lists shadowed loose copies as
unselected workspace products; on this checkout 19 of 19 published cues now
have their audio located, where none did
([review, A and E](engine/asset-library-implementation-review.md#a-selection-is-correct-in-the-runtime-but-not-uniform-in-the-tools)).

**Done 2026-11-14:** the old-publication clause at the host's launch road:
`a_publication_an_older_tool_recorded_starts_without_a_rebuild`
(`ambition_platformer2d::content::libraries`, feature `content_pack`)
installs a store whose recorded generation is an older tool's and whose
digest is wrong, and the product it names is the file served: no rebuild,
no audit.

**Next action:** none in this row.

**Host registration landed in `fb1621179`:** providers import identities and retain the
host's selected source. Explicit conflicting selections still refuse. Guard:
`a_provider_import_keeps_the_host_selection_but_an_explicit_conflict_refuses`.
The native app with served-provider support and standalone served Mary-O step
successfully. Keep provider imports separate from explicit source selection.

**Acceptance:** shared behavioral fixtures produce the same selected membership
and origin in both languages. Published-only assets resolve in the inspector;
loose unselected copies remain visible as workspace products. A usable old
publication can run without a rebuild or a deep audit. A native host with
served-content support enabled admits its shared library once; standalone
served games still run.

### ASSET-PUBLISH — selection does not audit old bundle payloads — ✅ DONE 2026-11-17

**Failure:** acceptance hashes the source and copied product; selection calls
`integrity()` for every selected bundle, including unchanged bundles.

**Owner:** renderer publication store and library producer scripts.
Design: [asset library review, B](engine/asset-library-implementation-review.md#b-publication-works-but-checks-the-past-on-every-change).

**Done 2026-11-10 (measured):** accept is one copy-and-hash pass (a 2.1 MB
unit: 6.3 MB read before, 2.1 MB after); publish selects from accepted
manifests with ownership checks and the atomic swap, and checks only that a
newly selected bundle has its members (onto 200 bundles: 1.1-1.2 s and 423
MB read before, 59-67 ms and 1.1 MB after); deep payload inspection is the
explicit `Store.audit()`
([review, B](engine/asset-library-implementation-review.md#b-publication-works-but-checks-the-past-on-every-change)).

**Next action:** none in this row. No release path calls `audit()` yet; the
release asset contract is ASSET-PREVIEW's boundary.

**Acceptance:** focused publication does not read or hash old payloads.
Concurrent independent publications survive the merge; a failed producer
leaves the previous selection usable. Stale parents remain advisory. Report
measured cost; do not add a new binary cache or store service.

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

### GEOMETRY-OBSERVABILITY — render the effective geometry behind gameplay — ✅ DONE 2026-11-17

**Done:** `ce8fbaf68` connects the real F1 host to shared geometry reports,
adds native parts and exact portal cuts, and records room/subject/tick context.
F1 supports view, room and body selection; headless JSON uses the same queries.
[Contract and commands](../systems/developer-tools.md).

**Guard:** `app_it::observed_geometry` checks the visible F1 schedule, equal
headless output, two live rooms, two subjects in one room, and a missed swing
followed by a real hit. Owner tests check exact portal caps and refusal states.
Keep debug collection off during normal play. Do not replace effective geometry
with an unlabelled authored or coarse outline.

### ENEMY-NOTICE — complete first-sight beats on the existing brain paths — ✅ DONE 2026-11-18

**Failure:** the dilophosaurus and other skirmishers have authored notice art
or sound but no notice interval in `SkirmisherCfg`. The enemy-pattern plan's
claim that all fighter-template enemies lack notice is stale: fighter
engagement already implements it.

**Owner:** `ambition_characters::brain::state_machine`, combat brain emission,
and authored character profiles. Design: [enemy patterns](game/enemy-patterns.md#not-yet).

**Done 2026-11-10:** skirmishers have the first-sight beat
(`SkirmisherCfg::notice_s`, read from the profile's `tactics.notice_s`, through
the shared `Brain::is_noticing` projection). The dilophosaurus is authored
(0.4 s and its notice cue).

**Audit 2026-11-10 (pirates and ninjas), corrected 2026-11-18:** the
renderer gave the crew `notice` and `windup` rows on 2026-10-30 (`bdfb278`)
and they are published: the raider, quartermaster, lookout, cutlass viper,
the three named heavies (Broadside Bess, Iron Mary, Salt Annet), the shadow
duelists (every look) and the oni leader. Without rows: the navigator (until
2026-11-18), the ninja heavies (until 2026-11-18), Kagero (an ally) and the admiral. Only
the oni leader had a `.notice` cue.

**Done 2026-11-18:** the crew's `Skirmisher`s notice. The raider (0.4 s),
Iron Mary (0.5 s) and the rangers' shared `pirate_ranger` profile (the
lookout and the navigator, 0.35 s) author `tactics.notice_s`; the crew's
hail `enemy.pirate.notice` and the heavy's bellow `enemy.pirate_heavy.notice`
are new recipes, levelled to -22 dBFS loudest 100 ms beside the oni's
(rendered here; the SFX bank is a build product this VM has never had); the
navigator took the lookout's telegraph rows (renderer `3441940`, published).
Witness `the_crew_and_the_duelists_are_seen_and_heard_noticing_before_they_wind_up`:
each is pinned in its `notice` row and plays its cue before it winds up.

**Done 2026-11-18 (melee):** `pirate_striker` (the quartermaster, the
cutlass viper; 0.35 s) and `pirate_brute` (Bess, Salt Annet; 0.5 s) are
`MeleeBrute` profiles with the beat and the crew's cues; the Shadow Duelist
and the Blue Scarf run the new `ninja_duelist` profile (0.3 s, the shadow
oni's alert). Their one-stroke kits gave `Smash` nothing to choose: from
200 px the cadence is unchanged (a windup every 44 ticks) and the first
windup comes sooner after the beat; sight is now the aggro radius. A
`MeleeBrute` no longer notices again after each of its own swings or stuns
(it read the AI's held intent as a lost foe).

**Done 2026-11-18 (the dojo's heavies):** the Ninja Heavy, the Bruiser and
the Wrestler got `notice` and `windup` rows (renderer `f70df11`, published),
a shut-mouthed grunt `enemy.ninja_heavy.notice` (SFX `cf1742d`, -22.1 dBFS)
and the `ninja_heavy` profile (`MeleeBrute`, 0.5 s) in place of the
`melee_brute_brute` preset. The audit's "axe heavy" is no character (the
axe is a held prop), and Kagero is an ally (`stand_still`), not a foe.

**Closed 2026-11-18 (the admiral):** this row's earlier next action was
wrong. The admiral has no `Skirmisher` profile (that was the raider's): it
ships the `melee_brute_striker` preset, and both rooms that place it (the
pirate cove's quest giver in `pvn.ldtk` and the hall exhibit) override its
brain with `stand_still`. Provoked, it fights as `pirate_boarder`
(`Smash`): a reaction to a blow, not a first sight. It never meets a foe
on first sight, so `notice` and `windup` rows on its fighter sheet would be
art no brain pins; none were drawn. The eye-band examiner keeps
`medium_striker` (`Smash`) for the rank exam, and the skirmisher profiles
without notice art or cue keep their declared policy (fight on first
sight), as the acceptance allows.

**Acceptance:** first sight produces one authored pose and cue before attack;
loss and reacquisition follow the declared policy. Hurt interruption and
ranged volley timing still work. A body without notice data keeps its declared
policy. Use the same beat projection for every controller.

### WORLD-LAYERS — fold the remaining long dimensions into connected regions — ✅ DONE 2026-11-18

**Failure:** the current Jungle world is 25792 by 1888 pixels; Magical is
17408 by 2688. Both still have long horizontal layouts. Travel portal kinds,
reach checks and same-dimension rules already landed in `c818c0801`.

**Owner:** `tools/ambition_ldtk_tools/specs` generators and
`game/ambition_map_assets`. Design: [world layers](game/world-layers-and-travel.md),
[region composition](game/world-composition.md),
[remaining room detail](game/world-detail-24h.md),
[Art dimension follow-up](game/art-dimension.md#ideas-for-later), and
[History dimension follow-up](game/history-dimension.md#the-road-ahead).

**Magical's sky done 2026-11-14 (MG-1):** the Starlight Heights over the
square and the road, joined by the beanstalk and by the star spring and its
cloud gap; walked by keys in `the_magical_dimension.rs`
([MG-1](game/world-layers-and-travel.md#mg-1--magical-sky-and-region-loop-done-2026-11-14)).

**Magical's arches done 2026-11-14 (MG-2):** STEADY (heights), CASTLE
(keep) and CANDLE (heart); the heart is two hops from the heights
([MG-2](game/world-layers-and-travel.md#mg-2--magical-arch-travel-done-2026-11-14)).

**Jungle folded 2026-11-14 (JL-1):** canopy over the ridge up a vine,
the Smoking Mountain east of the canopy, the tar pits under the ridge east
of the fossil beds with a seep back up: an underground loop with no portal
([JL-1](game/world-layers-and-travel.md#jl-1--jungle-geometry-and-walking-loops-done-2026-11-14)).

**Jungle's arches done 2026-11-18 (JL-2):** the `jungle_arch` prop, a
door-sized stone arch; GIANTS (trailhead), TALLEST (treetops), LIANAS
(canopy) and DIGSITE (tar pits), a chain at reach 6400
([JL-2](game/world-layers-and-travel.md#jl-2--jungle-arch-travel-done-2026-11-18)).

**Magical's arches drawn 2026-11-18:** the `starlit_arch` dressing of the
same assembly (pale stone, gold stars set in it, glints), standing where the
round rings stood, so every reach is as it was.

**Done.** Further arches in other planet-like worlds wait for a traversal
distance that warrants them (the plan's closing rule); topological
dimensions are not required to use the layer model.
Then recheck the old room-detail counts and Art's remaining publication. Reuse the existing portal and
gate machinery. Keep authored rooms editable in LDtk. Recheck old room-detail
counts before selecting missing props, secrets or region links. Then recheck
Art's remaining publication, cues and places against the live assets. Keep its
completed first region.

**Acceptance:** the flat map matches room placement; neighboring regions have
loops and distinct vertical connections. Real movement reaches their exits,
shortcuts and arches. Hidden rooms and permanent breaks survive save/load.
Review the semantic LDtk diff and capture the changed places. Do not add a
second travel implementation or turn the target aspect ratio into a test.

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

### LOCAL-PLAY-MODES — two local modes, explicit join and menu leave — ✅ DONE 2026-11-17

**Owner:** `ambition_input` (seating, `sources.rs`), Ambition's seating
declaration (`ambition_content::provider::declare_ambition_seating`), the join
road (`session/join.rs`) and the settings owner
([multiplayer](game/multiplayer.md), ruling Q153 2026-10-19).

**Built 2026-10-19 (slices 1 and 2):** Ambition's route decides a channel
plan, so devices never size the session (`ambition_route_seating`).
Single-player is `[Shared]`: the keyboard and every unclaimed pad drive player
one, the pad in use, also a pad connected late. The gameplay setting
`accept_local_joins` makes it `[Shared, Claimed]`: the second handle is
reserved from the start and idle until a pad that player one is not using
presses Start (`ambition_input::claim_local_seats`, `LocalSeatClaims`). The
claimed pad drives seat 1 only. The simulation joins a seat with no body on its
Start press (`session/join.rs`); Jump is not the gesture. A change of plan
rebuilds the local rollback session, with no restart. A standing claim keeps
its channel when the setting goes off. Witnesses: `local_play_modes.rs`,
`a_second_seat_joins_the_session.rs` (the rewound join; Jump builds nothing),
`a_joined_seat_has_a_view_and_a_hud.rs` (a real pad's Start, with two pads,
keyboard plus one pad, and a pad connected late; control: setting off),
`provider::tests::ambition_is_single_player_until_the_setting_accepts_a_join`.

**Built 2026-10-24 (first integration correction,
[local-coop integration](engine/local-coop-integration.md)):** the shipped
host builds a camera for each view the adaptive split opens
(`SplitViewRigsPlugin`, one capability for every host), and every player body's
picture follows its own pose and live room (`sync_visuals`). Witnesses:
`local_play_modes.rs` (left stick through the raw axis event; the split
camera's view, layers, screen effects and its own half of the display),
`room_placement_tests::each_player_visual_follows_its_own_body_in_its_live_room`.
The maintainer reports the co-op baseline now works in the main game
with two players (2026-10-24); see `MULTIPLAYER-SEAMS` for new playtest defects.

**Built 2026-10-28 (join placement, `1f0956abb`):** the common body recipe
now joins at a clear nearby position in the primary body's live room.
The search has a fixed limit and declines a blocked join. See the guards in
[local-coop integration](engine/local-coop-integration.md).

**Built 2026-11-10 (slice 3, Leave):** each seat's System menu offers
"Leave Game" to a joined local seat and "Remove Player N" to the primary seat
for each joined local seat (`SystemMenuModel::with_seat_departure`, attached
by `menu::departure::MenuDepartures`; only in a session that joins home
bodies). Choosing it writes `ParticipantLeaveRequested` through
`HostIntentWriter`, releases the seat's pad claim and closes the menu. The
simulation (`session::leave::retire_leaving_participants`, before the join on
the stamped tick) ends the membership, despawns the seat's player bodies, and
puts what each holds down where it stood (`ambition_held_items::put_down`; the
bag is session-wide). The primary seat cannot leave this way; Quit to Title is
its way out. Its menu, view and HUD row go with the membership and the body;
no room is reset; Start joins again. Witnesses: `a_seat_leaves_the_session.rs`
(the object lies once where the seat stood, also next door where its view
closes; the primary's leave is refused; a second join; the leave and rejoin
under a two-seat sync test),
`lunex_kaleidoscope_app_tests::remove_player_writes_a_stamped_leave_and_frees_the_pad`,
`system::tests::departure_rows_are_attached_per_seat_above_quit_to_title`.
The menu already answered to the seat that opened it (`SeatMenuFrames`, one
menu per member).

**Built 2026-11-13 (a remote guest's leave, hosted world):** a guest of the
hosted world that disconnects leaves: the host notices its seated client
entity is gone and writes the same stamped `ParticipantLeaveRequested`
(`ambition_hosted_world::retire_departed_guests`). Witness
`a_hosted_guest_that_disconnects_leaves_the_session`: seat 1 joined the
host's session, a guest seated as seat 1 disconnects, and seat 1's
membership and body end; a guest naming no seat (the control) leaves seat 1
in. A GGRS peer's leave is not built (Smash keeps GGRS, and
Ambition's remote play is the hosted world).

**Slice 4 (presentation) is met by existing witnesses (checked 2026-11-10):**
the join opens the two-player presentation (seat 1's own HUD on the shared
view, `a_joined_seat_has_its_own_hud_on_the_shared_view`, drawn in the
rendered host by `a_joined_seat_has_a_drawn_hud_in_the_rendered_host`); its own
view opens when the two are apart and the adaptive merge closes it when they
regroup (`a_joined_seat_gets_its_own_view_when_the_primary_leaves_its_room`,
`a_second_view_opens_when_the_players_drift_apart_in_one_room_and_closes_when_they_regroup`);
the leave brings the single-player presentation back (`a_seat_leaves_the_session`).

**Next action:** none required. Nice to have: a portal that Player 2 drops
out of, presentation only. A remote guest's leave is built for the hosted
world (2026-11-13).

**Acceptance:** the controller and multiplayer tests of the 2026-10-19
assignment (section 11), each a simulation-level witness with a negative
control.

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

### Music playback — owned handoffs and prepared entrances

**Owner:** [music programs and owned playback](engine/music-programs-and-playback.md),
with [MA1](engine/asset-authoring-and-prepared-media.md#ma1--export-playback-facts-link-game-music-policy)
for producer facts. [Diagnostics](../systems/music-playback-diagnostics.md)
remain an independent observer.

**State:** done, including the continuation review follow-through, 2026-11-23.

**Receipt:** fresh music could stop its own bank, destination
selection could resume an intermediate track, and fight media loaded at entry.
MUS1–MUS4 now use one bounded executor, one occurrence-owned goal, prepared
entrances, native section boundaries, strict recipes and explicit scene policy.
`047f1134a` checks Castle Godwin's entrance, door exits, reentry and death restore.
`828fb4155` releases endings completed during a host stall. `b60a3b918` checks
Hobford's shipped ending and room successor. `6715d352d` checks scene pause
and slow motion. `2aee2e597` prepares ending wait gains for the current section.
Music renderer `eefe5db` exports score inventory; the game registry consumes it
and removes its YAML parser. Guards: focused command, selection and preparation
tests, native software captures, pack inspection and source-only inventory tests.
Keep simple and adaptive playback on the same owner. Stronger timing, pickups,
independent tails and bridge overlays need an authored customer and supported
products. Physical-speaker and listening acceptance remain separate.
The older unexplained death/respawn case did not recur in the native route;
its historical cause remains unknown.

`2fb366d1e` closes the [same-product continuation finding](engine/music-programs-and-playback.md#a-continuation-use-is-distinct-from-its-scheduling-deadline).
A new use retains its predecessor independently of a start deadline. Same-section
continuations and self-repeat execute once; live mix changes do not restart.
Guard: 154 audio tests, the extended native short-completion capture, and
assembled-app compilation. Keep completion and retry state scoped to the use.

### GATE-REDIAL — a new address closes the old wormhole and opens a new one (done)

**Owner:** the gate network (`docs/planning/game/gate-network.md`, the gate
state machine and its membrane presentation); portals owner. Jon 2026-11-12:
changing a gate's address while it is open only recolours the open
wormhole. A redial is two events: the open wormhole closes (its closing
beat), then the gate dials and opens a wormhole to the new address (its
opening beat), with the connection to the new destination only once open.

**Acceptance:** with a gate open to A, dialling B plays the close then the
open, the old destination stops being reachable at the close, B is
reachable only after the open, and a body in the membrane during the change
is not carried to B.

**Built 2026-11-12:** a redial's link waits (`GateLink::opens_after_close`)
until both its rings are Off; dialling the place a ring already leads to
changes nothing. Witness `a_redial_closes_the_open_wormhole_and_opens_a_new_one`.

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

### CHARACTER-FORM — a change of form is the same body in the new form's geometry — ✅ DONE 2026-11-17

**Receipt:** Q159's geometry, skin and possession separation is built. A
home body once retained its old collision box while drawing another
character's art, and ordinary construction used a different sizing road.
The shared prepared-body grant now applies standing-height geometry at
construction and replacement (slice 1, `86ae35083`, 2026-10-19), and explicit
body geometry takes precedence on both roads (2026-11-12). The four forms
missing intrinsic size gained authored standing heights. Skin remains
presentation only; possession changes the driver of an existing body.
The old form's authored hurtbox is withdrawn when the new form supplies
none (2026-11-13).

**Guards:** `a_player_that_changes_form_takes_the_size_of_that_form`,
`an_explicit_body_is_built_at_its_box_not_its_sheets`,
`an_explicit_body_is_its_own_box_not_its_sheets`, and
`a_skin_change_moves_no_collider_hurtbox_or_movement_value`.

**Standing rules:** preserve the entity, physical identity, accumulated
damage and control/custody claims. An explicit body keeps its authored
box. A form without explicit geometry or intrinsic size uses the box
provided by its construction road, or the live body's box at replacement.
Do not turn a skin or possession into a form change. The boss-specific
sizing residual remains with GNUTON-GNU2-FINALE.

The completed ability refresh covered home bodies under `HomeBodyAbilities`.
The 2026-11-22 static review identifies ordinary intrinsic rebind and early
admission as a separate residual below; the geometry receipt is unchanged.

### FORM-REBIND — admit and apply a complete form on the existing body

**State:** DONE 2026-11-24. Admission: narrative wear resolves the look and
its beat sheet in `SessionCast` before any write; a look only N+1 holds is
refused whole. Rebind: construction and a change of form share
`IntrinsicForm` (verbs, flight mode, gravity response, aerial tuning); the
applied form is recorded on `PersonaBaseline`, role flight is declared
(`FlightStatedByRole`), and the Mary-O cosmic AirJump lend is deleted. See
the [receipt](engine/controlled-character-actor-kernel.md#rebind-intrinsic-configuration-through-the-native-state-owners).
Planned from the static review of `00bf92c22142`, 2026-11-22.
**Owner:** [native form contract](engine/controlled-character-actor-kernel.md#form-rebind--admit-and-apply-the-complete-intrinsic-form),
with character A3, linking L2 and the existing movement/ability owners.

**Failure / customer:** the Hall's sheep-to-cloud transformation does not
rebind the intrinsic abilities, flight and gravity configuration that a
fresh cloud receives. Separately, narrative wear checks the App catalog
and writes the form/beat before the session cast admits it. Earlier
physical projection can already withdraw the old grant. A later ID repair
does not prove refusal had no consequences.

**Next action:** admit the selected form and metadata before publication
or withdrawal. Reuse resolved intrinsic form decisions at construction
and replacement, composing explicit role, home and match terms. Settle
changed flight policy under current permissions at the native cut,
including when effective bits stay unchanged. Keep live movement state.
Remove the duplicate Mary-O cosmic AirJump lend only after the common
rebind covers that composition.
Apply the existing ACT1/IN5 continuation rules when this admitted change
also replaces an action source or input binding; preserve unaffected work.

**Acceptance:** a refused N/N+1 form request preserves native facts and
beats at the refusal cut. A fresh cloud and an ordinary sheep transformed
to cloud receive the same intrinsic configuration under the same terms;
returning to sheep retracts the cloud's contribution. Possession changes
no form. Same-form reapplication preserves pending launch or momentum,
and permission changes settle the selected flight policy without
manufacturing it from masked bits. No runtime result is claimed here.

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

### DOOR-EDGES — outdoor joins that are still doors

**Done 2026-11-16** ([verdicts](game/hobford-polish.md#door-census-of-the-other-worlds-row-5-2026-11-14)):
every pair keeps its door. Borobudur is across a strait; the colosseum,
the pvn dojo and the tyrant's hall are interiors; the pvn rooms are old
developer-hub rooms; GNU-ton's chambers are enclosed puzzles. The
colosseum is now visible from the shore: its facade
(`history_colosseum_facade`) stands round the door. From the 2026-11-14
census, [Hobford polish](game/hobford-polish.md#door-census-of-the-other-worlds-row-5-2026-11-14) row 5.
Jon's rule (2026-11-14, [level design guide](../authoring/level-design-guide.md)):
edges join the parts of one connected place, doors go into interiors.
Level 1 follows it. The census lists the other worlds' door joins that may
be outdoor joins: history (Borobudur and Srivijaya; the shore and the
colosseum), pvn (the cove's doors to the docks, the lookout, the dojo and
the arena), science (GNU-ton's truss chain) and dinosaur (the canopy and
the tyrant's hall). **Next action:** look at each pair's two rooms; where
both are outdoors, its owning generator writes an edge on the matching
sides (the arrival is 92 px in on flat ground) and the rooms sit side by
side on the world map; where one is a building, keep the door and make the
building visible from outside.

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
Encore"; the end is his retirement, not his death. Q175 (the giant's
identity) and Q176 (is a resolve bar a voluntary retirement) hold defaults.

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

### PLAYER-DEATH — what a death is in Ambition — ✅ DONE 2026-11-17

**Owner:** `session/death.rs` (the beat), the presentation owner, and the bag
(currency). Request (Jon, 2026-11-12): Ambition needs a presented player
death (Mary-O has one) and a rule for what a death costs: a Hollow Knight
style penalty undone by going back to where you fell, with a twist; or a
portal that yoinks you away before you bite it ("You're too valuable to be
destroyed"); or a lost currency you can win back, with a bigger penalty when
you cannot pay the safety fee. Q174 holds the rule; its default is the rescue
portal with a fee.

**Built 2026-11-12 (slice 1, the beat drawn):** Ambition's rooms hold a
death for 1 s (`replay_level_after_death_s`, was 0), so the robot's death row
plays, and declare a `death_look` (a new scoped rule, `DeathLook`): the
effect drawn where the body is taken when its window opens
(`digital_departure`, code strands dissolving) and where it comes back
(`digital_arrival`), by `runtime::death_look`. Measured with a temporary
probe in the hub: the departure on the death's tick at the body, the arrival
64 ticks later where it is back in play.

**Built 2026-11-13 (slice 2, the fee):** a scoped rule `DeathFee`
(`room_rules.ron`: `death_fee: Some((coins: 50))`), and
`runtime::death_fee`: a death owes the fee, and it is taken when the body is
back in play (a level that goes back puts the wallet back to its checkpoint,
which would undo a fee taken at the death). The coins taken lie where the
body was taken (its last safe ground) as a pickup drawn from the `DeathFees`
ledger (rollback state, schema 465): placed while its room is live, again
after the room goes back, off the ledger when fetched; one parcel a
participant, so a second death before the fetch loses the first. A wallet
that holds less gives all it has and goes into debt for the rest
(`BodyWallet::charge`: a negative balance, at most one fee; the HUD reads
`-$30`); the next coins earned pay the debt first. A fetched parcel is not a
reward the restore keeps (`ReturnedCoins`), or the next death would pay its
coins twice. Measured with a temporary probe in the hub: 120 → 70 at the
death, the parcel of 50 where the body was, 120 when fetched, 70 again at
the next death; with 20: −30 and a parcel of 20, −10 when fetched, −30 at
the next death.

**Built 2026-11-13 (slice 3, the rescue drawn):** the death look is the gate
network's own door (`rescue_portal_fx`, a sheet that draws its own start and
end): `rescue_portal_take` opens round the body, pulls it in and shuts, and
the body is not drawn from 0.42 s (`DeathLook::taken_after_s`, marked
`TakenByDeathLook` by the view, which `body_presentation_hidden` honours);
`rescue_portal_give` lets it out where it comes back. The fee's coins lie in
a sealed parcel (`rescue_parcel_prop`, `DeathFee::parcel_art`, a prop-sheet
row). Filmed in the hub (`capture_scene --player-dies 600 --purse 120`): the
door takes the robot and the spot is empty, the robot steps out of the
light at the respawn, the parcel lies where it fell.

**Built 2026-11-13 (slice 4, heard and said):** the door's sounds are the
wormhole's (`vfx.rescue_portal.rescue_portal_take`: the wormhole's swallow
and the door shutting behind the body; `_give`: its release and a two-note
chime), and the purse says what the death cost: a purse that drops shows
the drop beside it for 2.5 s (`$70  -50`), and one in debt reads owed
(`-$30 owed`). Not a line over the body: the body comes back at its room's
spawn, and in the hub the door plates and signs leave a speech line (the
last label family) no place, so the layout hides it; and the gameplay
banner is drawn only in the developer inspector. Filmed in the hub
(`capture_scene --player-dies 600 --purse 20 --include-ui --tick-per-frame`;
`--purse` now holds the wallet only until the death).

**Next action:** none. The acceptance's co-op half holds with the fee in
force: the 13 `a_second_seat_joins_the_session` witnesses (recovery, the
wipe, a departed seat, R5) pass in Ambition's rooms, which declare it.

**Acceptance:** a death in Ambition reads as one on screen; the fee is taken
and lies where the body fell until fetched; with too little to pay, the
bigger penalty applies; co-op recovery and the wipe keep their rules.

### HIT-FEEL — hits that answer — ✅ DONE 2026-11-17

**Owner:** combat feel (`ambition_combat::feel`, kill beat, hit-stop), the
sound and effect recipes, and each move's poise. Request (Jon, 2026-11-12):
hits must feel more responsive: better hit sounds, a little recoil on the
attacker, better hit effects, and enemies with more knockback or super armor
depending on their move.

**Measured 2026-11-12** (a temporary probe: the robot's first slash on a
passive goblin, goblin brute and exploding mite in the hub): both bodies
froze 0.036 s (the engine's hitlag law is a share of a platform fighter's
reference, 11 damage, and Ambition's hits deal 1 to 3, so every hit got its
floor); no camera shake (the shake is the freeze above the weakest); no
attacker recoil; knockback (47, -34) px/s, 7 px of travel, the same on the
brute as on the goblin (the slash authors a 58 px/s launch, and no catalog
row authors a weight); two sounds and a slash, a speech bubble and an
impact spark.

**Built 2026-11-12 (slice 1, the freeze and the recoil):** a `hit_feel` room
rule (`ambition_combat::hit_feel`, declared apart from `CombatRules` as the
landing-lag share is). Ambition's: hitlag `Affine(0.06 + 0.015 per damage,
max 0.15)`, so an ordinary hit holds both bodies 0.075 s and a heavy one
0.105 s (which shakes); and `attacker_recoil: 110`, a melee connect in the
air pushing the attacker back from what it struck (measured: 110 px/s held
after the freeze, 2 px a tick). Not on the ground: the run law brakes a
pushed speed to 0 on the next tick, and holding it would be the recoil
lock, which reads as hitstun.

**Built 2026-11-13 (slice 2a, armor):** an enemy's `Swipe` or `Lunge` melee
can say `armored: true`: super armor (`WindowTag::Armor`) from halfway
through the windup to the end of the strike, so a hit lands its damage and
not its launch; the first half of the windup and the recovery stay open. The
heavy bodies wear it: the goblin brute and bruiser (`brute_hammer`), the
heavy pirate, the triceratops, living statue, clay golem, aurochs, steam
automaton, hippo, mammoth, diprotodon and `brute_lunge`. Measured on a goblin
brute: a hit in the window took 1 HP at (0, 0) px/s and no hitstun, and the
swing went on; the next hit after it launched it at 108 px/s with 0.18 s of
hitstun. A `Slam` has no armor flag yet (four heavy sets use one).

**Built 2026-11-13 (slice 2b, knockback):** `hit_feel.knockback_scale`
(the `base_knockback_scale` of a game that declares no combat ruleset).
Ambition's is 2.0: eight ticks after the robot's slash a goblin moves at
(83, -19) px/s, against (36, 0), and travels 21 px, against 7. A goblin and
a brute are launched alike (no catalog row authors a weight); the brute's
difference is its armored swing.

**Built 2026-11-13 (slice 3, the contact and the armor seen):** an actor's
hit spark is drawn where the blow lands (`HitEvent::contact_on_box`): in the
overlap of the strike and the body, on the side the blow came from. It was
half way between the strike's centre and the body's, which a blade that
covers the body put in its middle. Measured on a hub goblin: the robot's
slash marks (1760.9, 949.0), 6.6 px inside its near face at the robot's
height; half way between the centres gave (1774.1, 943.1), its middle. An
open armor window is drawn: a steel sheen on the body's overlay
(`FeatureView::armored` from `MovePlayback::armor_now`, after the flashes and
before the i-frame blink), so a heavy move's armor is seen. Filmed on the
steam automaton in `history_industrial_age`: the body turns steel-blue
through its punch's windup and strike; at 0.30-0.55 it washed the art out,
so it shimmers 0.20-0.42. The sounds already answer the
weight: the robot's slash names the material selector, and a hit of 3 or more
(Ambition's heavy line) on flesh plays `flesh.deep` (430 ms, a thump falling
from 170 to 62 Hz) in place of `flesh.light`; on metal, `gong` in place of
`chink`.

**Built 2026-11-13 (slice 4, the slams armored):** a `Slam` melee can say
`armored: true` as a `Swipe` or `Lunge` can (`SlamSpec::armored`), and the
four heavy slams wear it: `war_drum_slam`, `terracotta_chop`,
`kraken_arm_slam`, `cave_bear_maul` (damage 3, windups 0.8-0.9 s). Probed
on the built move: an `Armor` window 0.4-1.0 s on a 0.8 s windup and a
0.2 s strike, none unarmored. The light slams (stagehand, scorpion man,
mummy, the club) stay open. Codec shape unchanged.

The heavy spark was already there, and the premise "no separate heavy art"
was stale: a connect on the room's heavy line draws `hit_hard` (since
6420ab194d, 2026-10-29) where a light one draws `hit_soft`, and larger with
strength. One rule (`StrikeWeightRules::is_heavy`, `combat::util`) picks
both it and the heavy sound.

**Next action:** none.

**Acceptance:** a landed hit has a distinct sound, a hit effect at the contact,
attacker recoil and hit-stop; each enemy move declares knockback or armor,
and a heavy move's armor is visible.

### APPLE-PORTALS — space apple trees and GNU-ton's catching portals

**Owner:** the GNU-ton encounter (`boss_encounters/`) and the portal owner.
Request (Jon, 2026-11-12, polish): space apple trees with apples on them;
for apple rain, GNU-ton summons portals on the ground that catch the falling
apples, which fall out of the portal's other end.

**Built.** In the apple rain he holds two catching pairs (`ch::catch`,
`hold_catching_pairs`): each is a mouth in the floor and one in the air,
one over where the player stood when the rain was called and one beside it,
clear of the giant. They open over the telegraph and close after the last
apples have fallen. A floor mouth is never under the giant, where the
scholar lands for eureka, or under an air mouth. Three space apple trees,
each on a rock moored to the ceiling, hang over the arena
(`shoulders_apple_tree_*`, `gnu_ton_shoulders` only; the Hall of Bosses'
copy `gnu_ton_arena` has no generator and is unchanged). Witness:
`gnu_ton_fight::an_apple_that_falls_into_a_floor_mouth_falls_out_over_the_player_and_hurts`.

**Acceptance (met):** an apple that falls into a ground portal comes out of
its partner with its momentum carried, and hurts as it did.

### MATH-POLISH — the hotel's spectral wing, the service lift, the reaper's wall and the mites

**Owner:** the math-polish subagent (Jon, 2026-11-12: "You can use up to 2
subagents"). Reports (Jon, 2026-11-12):

1. **Built.** In the hotel's spectral wing, background tiles of different
   widths do not line up, with seams; the torches' glowing dots do not
   repeat smoothly. Was: a prop is drawn at its art's shape, so a wall panel
   cut short drew a small panel in bare wall; the gap had panels at a third
   of the scale; the art did not tile; the motes drifted and jumped back
   each loop. Now 3:4 panels on whole quarters of tiling art, equal floor
   strips, looping motes (494e0f184f, with 2 and 3).
2. **Built.** The service lift has no visible barrier. Was: its lock wall
   drew the guest-room door exactly over the painted lift door. Now the
   lift's shut gate (`hylbert_lift_gate`).
3. **Built.** In the gram reaper fight a door sprite is used as a wall.
   Was: the treasury seal drew `hylbert_room_door`. Now the treasury's
   sealed grille (`hylbert_treasury_seal`).
4. **Built.** The dividing mite does not die when it divides. Was: the
   halves came at the kill, 30 px out, over a death row, a hold and a
   blink. Now its death row is the division and the halves stand where its
   lobes are when the row ends; the parent is gone (`Corpse::Gone`) and
   shows no death burst.
5. **Built.** The exploding mites go off only when they die. Was: in reach
   they planted a fuse and ended themselves, the blast came at the kill and
   the fuse-shaped death row played after it. Now the mite snaps, and a
   death's blast comes at the end of its death row (the fuse);
   `death_effects::run_death_row_finales`. Q177 holds the choice.

**Acceptance:** each report fixed and seen in a render or a probe; the mites'
behaviour has a witness.

### PORTAL-CONE-LAYERING — a window replaces everything under it (done)

Jon, 2026-11-13: the cone previews exactly the world the observer will see
after crossing, and no ordinary world drawable may appear over it; overlapping
cones keep their viewer-dependent order; partial crossings and nested views
stay consistent; one shared rule, no door-, actor- or scenery-specific
exception.

*Done 2026-11-13.* The window drew at 9.5, under scenery (10.55 to 10.95),
doors (10.96), bodies (15, 20), effects (30) and nameplates (38). So every
near-room drawable behind a window drew a second copy over it. A far-side
compositor redrew bodies behind the opening, but it tested the opening rect,
not the wedge, and saw only bodies. Now the window draws at
`PORTAL_WINDOW_Z = WORLD_Z_FX + 9`, over the play plane and under the
atmosphere in front of it (front motes, foreground parallax, daylight veil).
The pairwise and proximity order of overlapping windows is kept inside
`z .. z + 0.35`. The exit-slice band keeps its own top
(`PORTAL_EXIT_COPY_TOP = 9.5`), which the carry also uses to leave the ground
alone. The window mesh starts at the aperture face, so no near-side body is
under it. The far-side compositor, its candidate publisher, the visibility
resolver it fed and the folded-mesh pane cut are retired; the design page
(`engine/render-animation-and-vfx.md`, "A portal window is over the play
plane") says why.

Measured: `portal_chain_lab` from x=120 drew the sandbag, its nameplate and
the hint sign twice (near copy over the window); now once. Window seams
(`check_portal_window_seams.py`, A–D and gnu_floor, low and high): 10/10,
at most 0.03% differing. Limb through a pair (`check_limb_through_portal.py`,
windows on and off): 4/4. A walk into the `portal_lab` B wall pair, filmed
every 4 ticks: the near half in front, the rest in the window, no second
copy. `check_portal_seam_over_time.py` gnu_hammer was red at the old z
too: the fight's timing and channels had moved, and the film (`c129` from
tick 704) caught no blow. Re-aimed at `c128` from tick 302 (the fist goes in
at 313; from 326 its impact burst covers the band, in neither picture): low
and potato 23/23, with a carried part compared in 6 and 7 frames.

### SMASH-PORTAL-RECOVERY — GNU-ton's portal up-b, and a better recovery (done)

**Owner:** portals (mine) and the Smash movesets. Request (Jon, 2026-11-12):
GNU-ton gets the portal up-b in Smash; Alice may keep hers (two characters
may share a similar move); the portal up-b works better as a recovery.

**Built 2026-11-12:** measured with a probe on the live Smash stage (90 px
off the main platform's lip, facing it, falling, neutral stick): the pair
(exit 150 straight up, both mouths facing each other) caught the fighter in
a fall loop under the ledge from 0, 80, 160 and 240 px below the lip, and
she fell when it closed. A stick tilt turned both mouths and threw her back
out the way she came. Now the shaft leans toward her facing
(`lean_degrees`) and the mouths stay level, so one fall puts her up and
over: rise 190, lean 25 hangs her on the ledge from 0, 80 and 160 (240 is
too deep). Alice keeps the move; GNU-ton's up-b is the same pair
(`action_at_a_distance`, in place of `levity`). Witness:
`smash_it::the_portal_recovery_reaches_the_ledge`.

**Acceptance:** GNU-ton's up-b opens the portal pair; from below the stage
edge, a portal up-b returns the fighter to the ledge or the stage in the
cases measured, where it did not before.

### PLAYER-LIGHT — a wider, subtler player highlight (done)

**Owner:** the player light (render). Request (Jon, 2026-11-12): the player
lighting / highlight has a larger radius and is more subtle.

**Built 2026-11-12:** `ActorGlow::player_highlight` is radius 1.6 body
heights (was 0.85) at peak alpha 0.018 (was 0.042): the same light at half a
body height from the middle (0.013), now fading out over twice the distance
instead of ending near the outline.

### COOP-RECOVERY — independent respawn and a full-party wipe — ✅ DONE 2026-11-17

**Owner:** `session/death.rs`, `sandbox_reset.rs`, the checkpoint restore
(`session/checkpoint.rs`). Rulings: Q151 amended and Q153 (2026-10-19, the
later row: independent multiplayer).

**Current state (the Q153 default, 2026-10-05):** a death sends back only the
primary's room, and only when nobody is in play there; a fallen seat 1 comes
back beside the primary; a fallen primary waits while a seat plays in its room.
The restore's subject is the primary body (`PrimaryPlayerOnly`), so the
operations are told apart by seat, not by the active set, and there is no
full-party-wipe condition as such. Ambition's death beat is 0 s
(`room_rules.ron`), so an individual return is immediate and a wipe needs two
deaths on one tick.

**Built 2026-11-11 (the wipe's restore):** a participant out of play is not
a survivor, so when every participant is out of play the checkpoint restore
spares no room and no participant (`resume_at_checkpoint_on_reset`). Measured
with a temporary probe: Bob takes the gun-sword in the hub after Alice's
checkpoint and carries it to `duel_arena`; both die on one tick. Before, Bob
was spared and kept it (held 1, ledger `InCustody`); after, it is back on its
pedestal (lying 1, no row), one copy, and both are back in play. The wipe
also now pins the facts (DIALOGUE-CHECKPOINT). With Ambition's 0 s beat a
wipe still needs two deaths on one tick (the declared delay is open).

**Built 2026-11-12 (slice 1: return alone, and the wait):** a participant
whose room a living participant holds comes back alone at that room's
respawn point (the checkpoint when it is in that room, else the room's
spawn), health full, with no restore; the partner is not touched
(`return_a_fallen_participant`, which replaces the return beside the
primary; the primary no longer waits for its room to empty). A death while
another participant plays waits `recovery_wait_s` (new `room_rules` field;
Ambition declares 2 s, a tunable) beyond the beat, so a partner that falls
in that time makes the wipe. Witnesses in `a_second_seat_joins_the_session`
(the seat back at the spawn with the primary walked away; the primary back
alone after the wait, its room not sent back, then sent back when it falls
alone; a fall inside the wait making a committed wipe restore) and the
restated `a_death_with_bob_holding_an_object_in_the_room_keeps_one_copy`
(carried out then Alice falls alone; both fall on one tick).

**Built 2026-11-12 (slice 2: local recovery):** a second seat that falls
alone in its room while another plays asks `RecoverAtCheckpoint(slot)`; the
restore runs with it as subject (`RestoreTo::Recover`), sparing every other
room and win, covering only its view. Any close with nobody in play is the
wipe. Witness `a_second_seat_that_dies_alone_in_another_room_recovers_in_its_own_room`.

**Built 2026-11-12 (slice 3: a participant that left):** the roster is
the live player bodies and a leave despawns the seat's, so a departed seat
holds no room. Measured with a temporary probe in the hub (frames from the
primary's death to a committed restore with the primary in play): no join 4;
join then leave 4; seat 1 dead and leaving inside its own window 4; seat 1
staying never (control: the primary comes back alone). One arm was wrong: a
primary that fell beside seat 1 kept the partner wait after seat 1 left, 124
frames. With nobody in play a window owes only its beat now
(`close_death_interlude` caps it at `interlude` in rooms that declare a
`recovery_wait`; a match's stock windows are untouched); after: 25.

**Built 2026-11-13 (R5: a local recovery duplicates no object):** the
witness `a_seat_that_recovers_alone_gives_back_what_it_took_since_the_checkpoint`
passes. Its block was the join: in `blink_run` the corridor is exactly the
robot's height (48), a resting body overlaps floor and ceiling by float
error (1.7e-5), and the spawn clearance's strict overlap called every place
beside the primary inside solid. The clearance now ignores 0.01 of contact
(`rooms::spawn::RESTING_SLACK`). Past the join, the witness counted every
ground item in every live room, and `portal_bridge` lays out its own: it
now names the item, and finds one copy, lying in seat 1's room.

**Next action:** none; the rows below are the design, for reference:

- **Individual recovery (the default):** the participant respawns by the
  single-player respawn-location policy, for itself. With nobody else active
  in its room, that is a local room recovery: its room goes back to its
  checkpoint and the other participants' rooms and wins are spared (the Q151
  restore, with the dying participant as its subject, not the primary). With
  a living participant in its room, the room does not go back; only the
  participant returns.
- **Full-party wipe:** every active participant is out of play; the
  checkpoint restore spares nothing.
- A participant that left is not counted. The wipe reads death state now,
  not past deaths.
- **The wipe stays reachable:** the respawn waits a declared delay while
  another participant plays (a `room_rules` field); a partner that falls in
  that time makes the wipe.

**Acceptance:** same room, no room reset and the survivor untouched;
different rooms, room B and its wins unchanged; both dead, the checkpoint
restores; a removed participant not counted; a local recovery duplicates no
object.

### PARTNER-RECOVERY — an optional respawn beside a living partner — ✅ DONE 2026-11-17

**Owner:** as COOP-RECOVERY, plus the settings owner. Ruling: Q153
(2026-10-19, the later row).

**Built 2026-11-12 (the policy):** the Gameplay setting "Respawn Beside
Partner" (`coop_respawn_beside_partner`, off by default) is frozen into the
next Ambition session as the session rule `ParticipantRecovery` (prepared
content identity section `world.participant-recovery`, beside the
friendly-fire rule). With it on, while anyone plays, a fall sends no room
back (`close_death_interlude`) and the fallen participant comes back beside
a partner (`return_a_fallen_participant`): at the partner's place when they
share a room, else by a crossing into the partner's room (`NearBody`), then
back in play there. With nobody in play it is the wipe, as before.
Witnesses in `a_second_seat_joins_the_session`
(`with_the_setting_a_fallen_seat_comes_back_beside_its_partner`,
`with_the_setting_a_primary_alone_in_its_room_comes_back_beside_its_partner_next_door`);
their controls are the default's witnesses in the same file.

**Measured 2026-11-13 (the presentation):** the partner's room already shows
the fallen arriving. PLAYER-DEATH's arrival (`death_look::draw_death_arrivals`,
the gate's `rescue_portal_give` door and its sound) is drawn where and when
`OutOfPlay` comes off, in that body's room, and partner recovery takes it
off only after the crossing has put the body beside the partner. Probed in
both witnesses (temporary print, deleted): the seat's arrival in the shared
room, and the primary's at (1524, 1928) in seat 1's room, on the frame the
witness finds it back in play within 96 px of seat 1.

**Next action:** none. Teleporting while alive is a separate possible
feature, not in this row.

### DIALOGUE-CHECKPOINT — story facts and their objects share one horizon

**Owner:** the checkpoint (`OwnedItemsBaseline`, Q124), the world fact store
(`world.set_flag`, quest steps) and the dialogue effects. Ruling: Q164
(2026-10-19).

**Current failure (measured 2026-10-09, `a_playthrough_of_the_persistent_world`):**
the player gives Alice's note to Bob, gets the survey, and dies before any
checkpoint: the survey leaves the bag (objects go back), and
`bob_field_survey_received`, the quest step and the wall it opened stay (facts
have no checkpoint copy). The object is lost for good while the world says it
was received.

**Built 2026-11-11 (slice 1):** the save's flags and quest steps have a
checkpoint (`session::fact_horizon`): a baseline taken at each checkpoint
commit and when a session's save is applied (a New Game's wipe takes the fresh
save's), pinned into a restore that spares no participant, put back in
`CheckpointDomainApply` with the quest registry rehydrated by its boot rule
(`QuestRegistry::rehydrate`), mirrored into the save the rebuilt room is
prepared from, and verified. Measured with a temporary probe in
`a_death_sends_the_blink_back_to_where_it_lay`: before, a death before any
checkpoint left (survey 0, note flag on, survey flag on, quest step 2, lock
open); after, (survey 0, both flags off, step 0, lock standing). Dialogue
visits are not restored (Q134). A co-op death that spares a participant pins
no facts, as before.

**Built 2026-11-13 (slice 2):** `<<checkpoint>>` in Yarn banks what the
conversation did. The command writes `CheckpointEstablishRequested` (the
talking seat) through the narrative ledger. `PendingAuthoredCheckpoint` holds
it, ungated, since a conversation holds the game. On the next tick of play
`commit_authored_checkpoint` records the seat's body's place and raises
`CheckpointCommitted` before the capture, so the one baseline takes the
flags, the bag and the place the same lines moved. Schema 466. The Kernel
Guide's test menu has the authored customer (`hub_guide__test_checkpoint`).
Measured with a temporary probe in the shipped shell session: choose it,
then die. The banked flag stays set and the potion stays in the bag (4). A
flag written after the bank (the control) is taken back. Which story beats
should bank is content's choice; Bob's hand-over does not.

**Built 2026-11-13 (slice 3):** a co-op death already kept the facts
(slice 1). Their objects did not stay with them. The restore built the bag
from the checkpoint's plus the kept pickup, chest and defeat grants, so what
a conversation gave or took went back while its flag stayed. That was read
from the restore, not measured. `ConversationBagSinceCheckpoint` now records
each conversation's bag move (grant, purchase, sale and the payer's coins).
A restore that keeps the facts replays the moves onto the bag and the
subject's purse. One that pins the facts drops them. A checkpoint commit
forgets them, and a room rewind drops its own (the bag side of a rewind stays
`BagUsesInRooms`'s). Schema 467. Measured with temporary probes (grant 2
Health Cells, sell 1 for 5, set a flag, die). Two players, Bob spared: flag
on, cells 3→4, purse 0→5, all kept. Solo control: flag off, cells 3, purse
0.

**Built 2026-11-13 (slice 4):** dialogue history was already apart from the
facts: visits are not in the fact horizon, so a death keeps them (slice 1).
But it was one count for the whole session. A visit now names its seat
(`PersistedDialogVisit::seat`; a save written before reads as the primary
seat's). The conversation's input owner decides whose count rises and whose
history Yarn's `visit_count` reads (`ActiveConversation::history_seat`; a
conversation every seat drives is the primary's). Schema 468. A census of
the scripts: no branch on `visit_count` gives an item, a flag or a quest
step. Monty Carlo's show number only picks the car's curtain, and the choice
wins it.

**Done.** All four slices are in.

**Acceptance:** the dialogue arms of section 11: solo rollback consistent; an
authored checkpointing dialogue banks its consequences; a single co-op death
does not roll shared facts behind the survivor; a seen marker grants no reward
or quest step.

### GROUNDED-ATTACK-MOMENTUM — measure before changing the slide — ✅ DONE 2026-11-19 (Q181 is Jon's)

**Owner:** [unified movement kernel](engine/unified-movement-kernel.md) and
the moveset (`MoveFrameData::roots_steering`). Ruling: Q169 (filed as Q166,
2026-10-19).

**Current state (2026-10-13):** a rooted grounded move takes the stick; each
law then brakes at its hands-off rate. The axis laws stop in about 3 px from
270 px/s; Sanic's surface-momentum law coasts 351 px from 530 px/s.

**Measured 2026-11-10, the robot:** a grounded attack takes the stick (it
walks through the swing at run speed) and keeps no speed of its own (hands off,
it brakes as standing does, in under a pixel). That is the authored Hollow
Knight model; no change is recommended
([table](engine/unified-movement-kernel.md#grounded-attack-momentum-grounded-attack-momentum-q169)).

**Measured 2026-11-11, Smash** (`fighter_motion demo`, [table](engine/unified-movement-kernel.md#grounded-attack-momentum-grounded-attack-momentum-q169)):
a grounded move keeps the run's speed and brakes at traction (a jab out of a
let-go run slides the 42.5 px stop slide), it does not take the stick, and
George's dash attack carries the run 115.6 px on its authored impulse. No
change recommended.

**Measured 2026-11-19, Sanic** ([table](engine/unified-movement-kernel.md#grounded-attack-momentum-grounded-attack-momentum-q169)):
his only roll is the spin dash, and it is the run's law in a ball's box
(friction 400 px/s², the stick drives and brakes it, a jump out of it keeps
the surface speed and the ball). A roll that commits (Sonic's half friction,
input that only brakes) is a feel change filed as
[Q181](awaiting-maintainer-decision.md). The 2026-11-11 frozen ball does not
reproduce on main (14 spin dashes).

**Measured 2026-11-19, Smash** ([table](engine/unified-movement-kernel.md#grounded-attack-momentum-grounded-attack-momentum-q169)):
a dash jump keeps the dash's speed; a dash into shield slid on at 345 px/s
off the stage (the guard's brake was bounded by the run speed, and the dash
is faster), fixed: it now stops in 82 px. A grounded move does not stop at a
ledge (a jab let go 20 px before it slides off; the dash attack is carried
off from 60 px).

**Done 2026-11-19: the ledge stop.** A grounded move with `MoveGates::stops_at_ledge` holds its owner's middle on the floor's end ([table](engine/unified-movement-kernel.md#grounded-attack-momentum-grounded-attack-momentum-q169)): the Smash posture's grounded moves and every authored rooted grounded move carry it; the jab let go 20 px out now stops 0.5 px short of the lip. Left: Q181 (a committed roll for Sanic) waits for Jon. Originally: measure, for
representative laws and moves: whether the move
takes the stick; whether it keeps horizontal speed; which braking law applies
while it plays; whether authored move motion adds travel; whether it can carry
its user off a ledge. Compare with Smash Ultimate (traction, dash cancels) and
Hollow Knight (Cyclone Slash, Dash Slash). Recommend; implement only what the
evidence justifies, as per-move data if a law cannot express it. The AI's
move commitment and ledge risk are separate work.

**Acceptance:** a measurement table in the kernel page, one witness per
distinguished fact (coast, brake, authored displacement, ledge carry), and a
recommendation.

### SMALL-VIEWPORT-UI — screens fit a phone in landscape

**Owner:** the menu backend (`crates/ambition_menu/src/render/bevy_ui/`) and
each pixel-laid screen. Ruling: Q165 (2026-10-19), screen by screen.

**Current failure (measured 2026-10-09 at 640x360):** the Smash select screen
shows 3 of 6 roster columns and no START; the dialogue box covers half the
screen; the touch stick covers each menu's left side.

**Done 2026-11-17, the Smash select screen:** its layout was already a
function of the viewport (six columns paged with arrows, START and the four
player cards), but it read only a `Window`, so a windowless capture laid it
out at 1280x720 and cropped it: the 2026-10-09 measurement above was that
crop. It now falls back to the `HeadlessDisplaySurface` a capture declares,
as the host's own layout does; `capture_scene --route smash_select` at
640x360 shows the whole screen. What still covers it at 640x360 is the touch
overlay: the stick's fixed footprint (about 210 px of a 360 px height) hides
P1's card and the page arrow, and the action buttons sit over START.

**Done 2026-11-17, the dialogue box:** at 640x360 its body drew four lines
into a two-line box, under the choice window. A text node that is itself an
item of the panel's column was measured at one line, and the 48 px floor hid
that at 1280x720, where a greeting fits two lines. The body text now sits in a
row of its own, as the hint does. The phone-landscape profile also stopped
adding a 58 px inset to a reading rect that already leaves out the HUD band,
and it uses a smaller portrait and gap. Witness: `capture_scene
math_hotel_ballroom player OUT 640x360 --warmup 200 --player-beside
npc_math_monty --interact-at 90 --include-ui` shows the whole box and its
four body lines on screen; at 1280x720 the box is unchanged. (A capture on a
machine without the generated fonts falls back to Bevy's font, which has no
`·` or `–`.) What still covers the box is the touch action cluster: Q178.

**Blocked by:** Q178 for the touch overlay on small menus.

**Next action:** the touch overlay waits on
[Q178](awaiting-maintainer-decision.md#q178--on-a-small-screen-does-the-touch-overlay-give-way-to-a-menu);
the in-session pause cube is partly done (2026-11-17). Its page is laid in
face-relative units (`Rl`/`Rh`, `game/ambition_menu_kaleidoscope/src/page.rs`),
so the whole face shrinks with the screen: at 640x360 the six System rows
read at about 8 px. `InventoryLayout` now carries the System row count with
the item columns, from the menu region: under 480 px high the face shows 4
taller rows and a 1.35x description, and the wheel, drag and window helpers
take the count from the same region. Witness: `capture_scene --route
ambition_gameplay OUT 640x360 --press "wait:60,Escape,wait:30" --include-ui`
(1280x720 unchanged at six rows). The cube camera on a short screen also
narrows its field of view so the active face fills 96% of the height, not
80% (`SHORT_SCREEN_FACE_FILL`, `fit_cube_camera_regions`): labels read at
about 13 px. The inventory face is next. Maintainer 2026-11-17: mobile UX
waits behind higher-value queue items, so the rest of this row is deferred.

### INPUT-GLYPH-ART — draw prompts and a controller layout from the published glyphs — ✅ DONE 2026-11-19

**Art done 2026-11-18** (renderer target `input_glyphs`, published under `sprites/input_glyphs/`): gamepad glyphs for each `GamepadStyle` (xbox, playstation, switch, generic), keycaps named by `key_label`, mouse glyphs, and xbox/playstation/switch/generic pad, keyboard and mouse pictures, each with a `_layout.ron` that places every control (Bevy `GamepadButton`/`KeyCode`/`MouseButton` names) in the full-size picture. The naming rule (style, control) → file is in the target's docstring; ⚠ Bevy's `LeftTrigger` is the bumper and `LeftTrigger2` the trigger.

**Done 2026-11-19, the dialogue box:** its input hint draws the answering
seat's button pictures. `PhysicalControl::glyph_path` names a control's file
by the renderer's rule (pad family and Bevy's button word, or the key's
label); `DialogControls` carries the confirm and back controls' paths beside
their labels; the box splits its hint line at each control and draws the
picture when the asset catalog has the file (`try_image_path_for_load`), the
name when it has not (an `F13`, a headless app). A rebind moves the picture,
as it moves the label. Witness: `capture_scene math_hotel_ballroom player
OUT 1280x720 --warmup 200 --player-beside npc_math_monty --interact-at 90
--include-ui` shows the F and Backspace keycaps (a long key keeps its wide
cap); at 640x360 the hint wraps to two lines.

**Done 2026-11-19, the control prompt and the signs that name controls:**
`PromptEntry::glyph` is the picture of the control `binding` names, from
the same owner by the same choice (`SeatBindings::glyph_for_slot`), so a
rebind or another pad moves both. A sign written `{action:ID}`
(`ControlLegend`) draws that picture in place of the name when the catalog
has it, and the name when not: the sign becomes text spans, the control a
padded placeholder span, and a sprite sits on that span's laid-out run box
(`place_legend_glyphs`, which also gives the spans the sign's font and fade).
A view's mirrored copy keeps the names. Three sandbox signs now name their
controls (`basement_treasure`, `basement_npcs`, `gap_run`). Witnesses: the
prompt tests assert the Z keycap, the F13 rebind, Xbox A and PlayStation
Cross paths; `capture_scene basement_treasure player OUT 1280x720 --warmup
120 --player-at 1100,600 [--device xbox|playstation]` shows the F keycap, RB
and R1 (`--device` is new).

**Done 2026-11-19, the controller-layout page:** System → Controller Layout
(after Rebind Controls) draws the seat's device picture with each gameplay
action it binds on that device named in a column beside the picture, joined
to its control by a leader line. Controls on the picture's left half are
named in the left column and the others in the right; a column spreads its
labels so none overlaps. The app loads every device's `_layout.ron`
(`DeviceLayout`, `bindings.rs`). The page reads the live `SeatBindings`, so
a rebind moves the label, and the seat's device chooses the picture. Menu-only
actions (`is_menu_only`) are left out: they share the play buttons and would
crowd them. The entry is absent until the device's layout has loaded. The
menu model gained `MenuNode::Image`, drawn by the cube and by `bevy_ui`.
Witness: `capture_scene crawl_lab player OUT 1280x720 --include-ui --press
"i,wait:60,e,wait:90,down,down,down,down,down,enter,wait:60"` (keyboard), or
`--device xbox|playstation --press
"i,wait:60,touch:1023x360,wait:90,touch:640x455,wait:60"`.

⛔ Before this page, `capture_scene --include-ui` had not drawn the open cube
menu at all. Bevy measures a camera's target only when the camera is added
or its projection, viewport or window changes. The cube camera was measured
against the window, then retargeted to the capture image, so it had no target
size and the renderer skipped it. `adopt_menu_camera` now touches the
projection, so the cube films again.

**Acceptance:** a prompt shows the pressed button's glyph for keyboard, an Xbox-like and a PlayStation-like pad; a rebind moves the glyph; the layout page names every bound action on its control.

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

**Selected next action:** Step 5 of the action plan: a behavioural coverage
matrix, starting with the hosted-Smash tests and the larger reload and
two-room modules, before any active test is deleted. Steps 1 to 4 are done:
- step 1, the Smash setup is shared (2026-11-14);
- step 2, the Versus setup is consolidated, its waits named by the fact each
  observes (2026-11-21);
- step 3, the first diagnostic queue is classified (2026-11-21);
- step 4, the repeated inputs of six modules are shared, with no test merged
  (2026-11-21, [record](test-suite-consolidation-action-plan-2026-10-26.md#step-4-consolidate-repeated-inputs-not-the-behaviors)).

Step 5 has started: `two_players_two_live_rooms.rs` (71 tests) is read, with
0 removals proposed ([record](test-suite-consolidation-action-plan-2026-10-26.md#step-5-record-two_players_two_live_roomsrs-2026-11-21)).
`a_room_rewinds_alone.rs` (18 witnesses, 14 hand-run controls) and the three
reload modules (27 tests) are read too, also with 0 removals. What remains of
step 5 is the hosted-Smash half.
On this VM the hosted-Smash half of step 5 waits on the music environment:
a Smash match start waits for entry music. Use
current `AGENTS.md` verification policy: no test-count/source-count floors or
new meta-test machinery as a proxy for behavior.

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
migration. The SDK/demo path retains old generation readers, but still has
process-wide selection and callbacks that can refuse after selecting a pack.

**Contact customer receipt:** the named `body_contact` customers are done (2026-11-19).
The Sanic badnik's contact is a module, and its toucher roll and
contact-harm facts are asked of their owners through
`sanic.contact.touchers`. The Mary-O snake shell is a module that keeps its
phase in the snake's record and holds, withdraws, pins and runs down through
the new `body_contact` requests. The plant is a brain
and pose customer, not a contact rule. Recheck which customers are still native
before migrating them.

**Next action (source `f41218979625`, 2026-11-21):** complete SDK/demo P5
step 4 cut 2 at the owner above. A valid pack can fail App audio/cast
composition after its rules are already selected and still report published.
Prepare each owner product once from all of its changed input schemas;
admit all affected values before changing selection. Extract private cast
preparation, then publish products and App-local selection under the existing
boundary. Bind rules at operation boundaries, without World lookups in every
leaf helper. This selects the concrete SDK coherence defect, not all open
migrations in the older runtime-authoring audit.

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
Sanic rung-5 run still left the stage under grounded attack momentum; measure
and resolve that in GROUNDED-ATTACK-MOMENTUM (Q169), not by repeating a
fixed-range or stale-view brain workaround.

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

### D72 — continue Smash parity from the inventory

**Owner:** [Smash inventory](demos/smash-parity-inventory.md).
**State:** later product pass, behind the main Ambition improvements
(maintainer direction, 2026-11-22). The S0 gate is closed. Preserve its
receipts; do not select another parity feature merely to keep the gate busy.

**Next implementation when selected:** follow
[§17](demos/smash-parity-inventory.md#17-current-implementation-order).
Use shared ACT1/ACT2, the CR1 reaction refinement and CLIP1–CLIP3 from their
engine owners, then one timed
floor recovery and actual CPU return. Review a bounded short match before
more roster art, strategy tuning or feature breadth. The
[feel owner](demos/smash-feel-audit-2026-10-29.md) states the demo acceptance;
it is not another feature ledger.

**Acceptance:** the chosen short demo has coherent action/reaction timing,
readable landing/roll/launch transitions and a CPU recovery that reaches
support. Headless checks establish mechanics; timed captures and human play
establish the presentation and feel. Product decisions and intentionally thin
stand-ins retain the inventory's existing rulings.

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
release on room transfer. Its owner retains those receipts. P1 still needs
the published accepted/refused reply to the requesting module. Start
P2 with the existing max-hops-zero recovery fixture extended through generic
transit, then a moving exit and retained target trajectory. P4 can independently
prove/fix full-turn bounds and repeated-link candidate exclusion. P3 removes
static contact-velocity reconstruction and the AABB snag authority after the
accepted motion contract is in place. P5/P6 complete observer identity,
chart transport and demand/ready-image capture without replacing working pools.

The follow-up [SCALE-FEEL](#scale-feel--the-same-body-behavior-at-a-different-scale)
supplies the selected consumer-unit law; [PRESENTATION-COMPOSITION](#presentation-composition--order-appearances-and-add-a-25d-view)
supplies local appearance order and a later 3D rendering consumer. Keep P2/P3
time/contact and P5/P6 observer/capture authority here.

The `00bf92c22142` [query audit](engine/world-geometry-and-spatial-semantics.md)
adds two bounded continuations in this lane. T3 preserves foreign geometry
publication when one room replays; prove it before another full fixed step.
Current material overlap makes water membership and physical switch touch
read accepted parts, retaining their maps for response and preference.
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

### SCALE-FEEL — the same body behavior at a different scale

**State:** PLANNED, maintainer-selected 2026-11-21; source `f41218979625`.

**Failure / customer:** canonical controls scale but ordinary body gravity
does not, changing jump time and relative height. Fixed contact bands,
self-motion events, projectile/item gravity and relation anchors also have
incomplete unit ownership. The current half-scale portal customer must keep
normal action timing at its new size. This is source analysis, not a new
runtime reproduction.

**Owner:** [scale-relative gameplay](engine/scale-relative-gameplay.md).
SR1–SR3 use the existing frame, movement, combat, item and relation owners.
P2/P3 keep portal time/contact authority; E4 keeps the standing experiment.

**Next action:** SR1 converts canonical gravity response at the shared frame
resolver and brings runtime/probe inputs into agreement. The kernel consumes
the resolved world frame unchanged. SR2 classifies support/ledge/crawler
bands and destination-relative exit assistance. SR3 completes the listed
action, splash, free-mover and relation consumers. Keep authored mass and
damage policies; do not scale every world vector or add another solver.

**Acceptance:** equivalent scenes at `0.5`, `1` and `2` retain normalized
nonzero motion and action timing. Use the real frame writer, a near-boundary
contact, source/victim combat units and actual relation/crossing witnesses.
A fixed world field/platform keeps its own meaning when only the actor
scales. Preserve canonical tuning, numeric refusal and the identity fast
path. A gravity-only repair does not close the whole packet.

### PRESENTATION-COMPOSITION — order appearances and add a 2.5D view

**State:** PLANNED, maintainer-selected 2026-11-21; source `f41218979625`.

**Failure / customer:** Scout must appear behind the cart panel inside and
in front outside. Current seating writes root z 14.45 behind panel 14.5,
while a hit flash adds 1.5 and a wielded item uses player z. Equal-root rig
parts can interleave. Numeric z also controls portal carry eligibility.
The future 2.5D consumer must coexist with current 2D authoring/rendering.
These are source deductions; no new image or device measurement was made.

**Owner:** [presentation order and optional 2.5D rendering](engine/presentation-order-and-25d-rendering.md).
PVS-2 retains visual sampling; portal P5/P6 retains projection/capture.

**Next action:** PO1 gives the cart an ordered interior slot and places the
whole body appearance, held art and applicable effects through one exact
owner/room relation. Resolve placement before drawable writers. PO2 extends
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

### PORTAL-STRUCTURAL-FX — sprite and particle effects ignore the panes (done)

Jon, 2026-10-17: "it should not be structurally possible to add something
into the game that doesn't work with portals." Meshes built on the CPU now
have one road that folds them through pairs and out of far-side panes
(`ambition_render::folded_mesh`, used by every line look). The `fx.rs`
effects have none: particles, beams, impacts and slash sprites are drawn
whole at z 30 and up, over a pane on its far side, and are not cut at a
crossing. Measured the same day by reading `portal_compositing.rs` (they
carry no candidate marker). Next: give them that one road (a sprite effect
through the far-side and transit pieces by construction, or the folded
writer for quads), not a per-effect opt-in.


**Far side, done 2026-10-18:** a pane composites every top-level drawable of
its room drawn at or above the window (`publish_portal_compositing_candidates`),
not only the three marked kinds, so a particle, a beam or a slash behind a
pane is hidden and redrawn in its uncovered parts by construction. Two kinds
opt out: the compositor's own copies and the parallax layers (placed against
the view). Witness: `a_far_side_effect_is_composited_with_no_marker_of_its_own`
(red under the old marker filter, without the parallax opt-out, and without
the portal-room check). Cost (release, 2000 sprites, 1000 over the window,
one pane): 64.9 µs a frame, against 34.7 µs for the old rule, which took
only the 100 marked ones; with no pane 16.3 µs against 24.7 µs. A candidate
that stays one is written in place: an insert command for each was 179 µs.
*Retired 2026-11-13 (PORTAL-CONE-LAYERING): the window draws over the play
plane, so nothing behind it needs compositing.*

**Crossing, done 2026-10-25 (63797b0ac):** the portal carry
(`ambition_portal2d_presentation::carry`) cuts every top-level `Sprite`,
`Mesh2d` and `Text2d` that reaches past an entry plane inside the opening, and
draws its part past the plane out of the exit. It needs no marker, so every
`fx.rs` particle, beam, impact and text is carried. A child goes with its
parent. Witness: `a_free_drawable_keeps_the_room_policy_beside_a_body_with_its_own_policy`
(a plain unmarked sprite across an entry is cut into pieces and captured).
`sync_portal_body_pieces`, named by the earlier open note, no longer exists.
No effect in `fx.rs`, `fx/`, `lines/` or the slash visuals draws with
gizmos or another kind, so nothing is left open here.

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

### RIG-IMPOSTOR-CONTAINMENT — a part-drawn body is drawn whole or refused

**The invariant:** every pixel a composited body draws lies in its cell. One
camera draws a whole page with no per-cell scissor, so a part past its cell
is cut from its own body and drawn into its neighbour's.

**Owner:** `ambition_render::rendering::actors::rigged` (`impostor_margin`,
`posed_reach`), `ambition_sprite_sheet::character::rigged` (`art_overhang`)
and `scripts/measure/measure_rigged_parity.py`. Plan:
[`engine/mary-o-part-realization.md`](engine/mary-o-part-realization.md).

**Scope since 37497a444 (2026-10-05):** a part-drawn body draws its parts in
the world, and uses an impostor cell only while something composes it
(`ComposedBodyDemand`: hit flash, portal pieces, deep dream, quasar) or its
frame fades as one picture. So the invariant applies to that composed road
only.

**Solved, by kind of draw:**

- A published frame (2026-10-05). The cell is the frame plus
  `impostor_margin`, which covers the flipbook's stated `art_overhang`. The
  oni leader's banner is drawn whole (`a_cell_holds_every_draw_of_its_body`).
- A tween's in-between (2026-10-06, review P2). `art_overhang` is a
  conservative envelope of every draw `tween_into` can make, taken when the
  flipbook is read: each moving draw is sampled finely enough that no corner
  moves more than half a pixel between samples, and that slack is added
  (`tween_overhang`). A part that turns between two frames reaches where
  neither frame does (`the_overhang_of_a_tweened_clip_covers_its_in_betweens`;
  the published corpus: `no_in_between_of_a_published_flipbook_reaches_past_its_overhang`).
  Measured on the 123 published flipbooks with a tweened clip: the in-betweens
  move the stated overhang of 12 of them, by 0.21 px at most; no cell changed.
- Parts placed by a `PartPose` (2026-10-06, review P2). A pose is in no file,
  so its reach is measured each frame (`posed_reach`). While it is more than
  the margin, the body is drawn directly, as a body whose frame fits no cell
  is: whole, and its readers see no image, with one warning. The hold that
  smooths composition demand smooths this too
  (`a_part_pose_that_reaches_past_its_cell_is_drawn_directly`).

**Open:**

1. `scripts/measure/measure_rigged_parity.py` still clips its oracle to the cell, so
   a cut part cannot read as a parity failure there. The clip was added when
   the cell was the frame plus 16 px (the oni leader's banner read as 1153
   wrong pixels of a correct draw). With the margin from `art_overhang` the
   clip should remove nothing. Remove it and run the harness on a GPU
   (`rigged_sprite_parity`); acceptance: the oni leader passes unclipped, and
   with `impostor_margin` poisoned back to 16 px it fails.
   **2026-11-21:** the clip is removed, and both `ambition_render` examples
   compile again: they disabled a `WinitPlugin` that this crate's bevy, built
   without winit, does not have. The harness runs on this VM (llvmpipe, with
   `AMBITION_CONTENT_LIBRARY=game/ambition_content_library/library.json`), but
   it stops at the oni leader's baked sheet,
   `sprites/npc_ninja_shadow_oni_leader_spritesheet.png`, which it reads from
   the content library. PART-FIRST no longer publishes baked pages, and
   `sprites.sh --reference` paints into `.agent/sprite_reference/<target>/`.
   **Done 2026-11-21:** the acceptance's first half passes. With
   `--reference .agent/sprite_reference`, the example draws the baked side
   from a `sprites.sh --reference` paint. That paint is packed and trimmed by
   its own record, so its atlas comes from that record
   (`spec_from_manifest_text`). The scorer reads the paint's sheet and
   flipbook too. A target is a sheet id: `ninja_shadow_oni_leader`. The
   character's `npc_` id missed the flipbook, and the impostor margin then
   quietly fell back to 16 px.

   Unclipped, the oni leader's 864 gated frames, both facings, are inside the
   bounds; the worst part blob is 4 px. 184 frames are art the baked frame
   cuts off and the parts still draw.

   The 18 blink frames read about 70% wrong. They are the engine's teleport
   warp, drawn by design on a blink row that draws the body whole
   (`body_warp`). Neither the baked road nor the published draws carry it, so
   the scorer reports them and does not gate them.

   The other half (`impostor_margin` poisoned to 16 px fails) was not run: no
   poison runs without Jon asking. The scorer needs `scipy`, which the
   renderer's venv lacks. Run it with `uv pip install --target DIR scipy` and
   `PYTHONPATH=DIR`.
2. A posed body that reaches past its cell loses its composed readers (its
   hit flash overlay, its portal pieces) while it does. If a ragdoll must keep
   them, give the presentation a cell chosen from the pose's reach, or let a
   pose declare an envelope. No shipped body carries a `PartPose` today.
3. A two-body shared-page test that reads the pixels of both cells needs a
   GPU; the three proofs above are on the draw geometry.

### DENSE-MELEE-ROOM — author the dense-melee development room — ✅ DONE 2026-11-19

**Owner:** content; first measured customer
[`engine/bounded-perception-and-attention.md`](engine/bounded-perception-and-attention.md#acceptance-for-the-open-increment).

**Ruling:** Q93 (2026-10-04): keep or create a deliberate dense-melee
development/stress room. It is a real scenario, not polished content.

**Done 2026-11-19:** `dense_melee_pit` in the sandbox, written by
`game/ambition_map_assets/ambition_content/generators/dense_melee_pit/make_specs.py`:
a 960×448 pit with two one-way ledges and 200 `EnemySpawn`s (goblin, goblin
raider, goblin squat, lab raider; each names the `medium_striker` policy, which
builds the tactical view). The way in is a door at the east end of
`basement_enemies`. `measure_perception_density.sh --room dense_melee_pit
--brain "" --extents shipped --populations "4 8 12 25 50 100 200"` measures it
on its own cast. Shipped viewport, 600 ticks:

| population | offered | visible | kept |
|---|---|---|---|
| 4 / 8 / 12 | 5 / 9 / 13 | 4 / 8 / 12 | 4 / 8 / 12 |
| 25 / 50 / 100 / 200 | 26 / 51 / 101 / 201 | 25 / 44.7 / 68.2 / 141.7 | 16 (the budget) |

The population cap had never applied to an `EnemySpawn`: only `NpcSpawn`
counted, which is all the hall holds, so a capped run of this room built all
200 (`offered` 201 at every cap). Authored enemies now spend the same quota,
after the placements (`RoomFeatureConstructionPlan::prepare`). The census and
`ms_per_tick` come from the one `hall_bench` run; camera framing needs a
rendered run (the pit's camera zone is the whole room) and was not read here.

**Acceptance:** in the room `kept` tracks population until the budget caps it,
and the census, frame time and camera framing are readable from one run.

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
