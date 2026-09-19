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

**MEASURED 2026-09-19, after the maintainer ruled twenty of this page's
questions in one pass.** ⚠ This list is the BLOCKING set, not the important
set: a question can matter and block nothing.

⭐⭐ **THE BLOCKING SET IS NOW TWO.** The 2026-09-19 rulings closed the
composition cluster (`Q146`, `Q144`, `Q108`, `Q106`, `Q100`, `Q97`), the
contact-provenance question (`Q101`), the difficulty cluster (`Q127`), throws
and rage (`Q133`), gravity switches (`Q137`), item-menu latency (`Q140`),
cutscene fade (`Q143`), and explicitly declared **not blocked** the set that had
been read as pending rulings: `Q132`, `Q136`, `Q138`, `Q139`, `Q122`, `Q104`,
`Q110`, `Q145`, `Q141`. Every one of those is recorded in
[`maintainer-decisions.md`](maintainer-decisions.md) and deleted here.

⛔⛤ **SO THE WORK IS NOW ENGINEERING, AND SAYING OTHERWISE IS THE FAILURE MODE
THIS SECTION EXISTS TO PREVENT.** The maintainer's instruction with the rulings
was explicit: *"Do not invent replacement maintainer blockers where these
decisions already determine the direction."* A row whose ruling has landed and
whose implementation is unfinished is a QUEUE row, not a question. ⇒ Before
filing a new `Q`, check that the ruling above does not already decide it.

| question | what it blocks | and if it stays open |
|---|---|---|
| [`Q147`](#q147--must-a-candidate-session-be-built-from-the-generation-its-own-activation-commits-or-from-the-one-current-before-it) | **P1** `CANDIDATE-GENERATION-ORDER`, entirely — the row's only remaining work is the answer | the two ordering edges keep pinning opposite ends of one set, a candidate keeps building from the generation before its own activation commits, and the next reader rediscovers it from scratch. ✔ Today's shape is pinned by a guard meanwhile, so the answer cannot be made moot by a silent move |
| [`Q69`](#q69--at-potato-should-character-sprites-fall-back-to-the-0_25x-tier) | **P1** `D-POTATO-ASPECT`, entirely — the row's only `Blocked by:` | a content/quality call; nothing else in the row is startable without it |

⭐ **AND TWO THINGS THAT LOOK LIKE BLOCKERS AND ARE NOT, WHICH IS THE USEFUL
HALF OF MEASURING THIS.**

- [`Q128`](#q128--should-the-simulation-tick-be-rebased-when-peers-agree-to-start-or-stay-an-absolute-per-app-count)
  is a **coordination re-arm condition** on `C03` and `C05`, not a gate: both
  rows say *"if that road is ruled and started while this migration is in
  flight, coordinate rather than assume."* Its other half — the timeline
  comparison — is blocked by `N2`'s absent P2P session, which is engineering
  rather than a ruling, **and which the 2026-09-19 priority adjustment
  deprioritises outright: netplay is not this year's goal.**
- `Q142` is named in `ID-PEER` only as history (*"after `Q142` added three"*),
  and it is three-quarters resolved. Matching on a question number near the
  word "blocked" finds mentions, not gates.

⚠ **THE DERIVATION METHOD STAYS, EVEN THOUGH THE SET SHRANK**, because it is
the part that was wrong twice. `queue.md` states gates in a canonical
`**Blocked by:**` field; deriving them from prose instead missed two the first
time and eight the second, and both misses read as *"this row is pickable"*.
`scripts/check_blocking_set_names_every_gate.py` fails if this section omits a
gate the field names, and it prints the live triple rather than letting a
sentence here go stale. ⇒ When a corpus has a structured field for the fact you
are deriving, deriving it from prose is not a conservative choice, it is a
different and worse question.

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

⭐⭐ **THE TRADE IS MEASURABLE AND IT IS LARGER THAN "a bit blurrier",
2026-09-19.** Reading the four shipped catalogs
(`crates/ambition_platformer2d_actor_monolith/assets/sprite_packs/<tier>/ultrapack.json`):

| tier | scale | page size | pages | on disk | targets |
|---|--:|--:|--:|--:|--:|
| `full` | 1.0 | 2048 | 122 | 262M | 179 |
| `half` | 0.5 | 1024 | 133 | 121M | 179 |
| `quarter` (`0_25x`) | 0.25 | 512 | 154 | **48M** | 179 |
| `potato` | **0.0625** | 256 | 42 | **6.9M** | 179 |

⇒ **Falling back to `0_25x` at `potato` is a 7× increase in sprite-pack bytes
on the weakest hardware the game targets** — 48M against 6.9M. That is the
cost of the proposal, and it is the number this question was missing.

⭐ **AND ALL FOUR TIERS COVER THE SAME 179 TARGETS**, so this is not a coverage
question and the fallback would not be filling a hole. ⚠ The page COUNT rising
122 → 133 → 154 and then collapsing to 42 looks like missing content and is
not: page size halves every tier, so the middle tiers pack the same art into
more, smaller pages.

⛔⛤ **THE LADDER IS NOT UNIFORM, AND THAT MAY BE THE REAL SUBJECT.** The scales
are 1.0, 0.5, 0.25 — and then **0.0625**. Every step halves except the last,
which quarters. `potato` is not one step below `quarter`, it is two, and the
name vocabulary (`full`/`half`/`quarter`/`potato`) hides that: the fourth name
is the only one that is not a fraction. ⇒ A reader choosing a tier from the
names would predict 0.125.

**The decision:**

* **(a) Fall back to `0_25x` for character sprites at `potato`.** Characters
  stay legible on the lowest budget. ⚠ Costs up to 7× the pack bytes there,
  and it makes `potato` mean "quarter for characters, potato for everything
  else" — a per-domain tier policy rather than one budget.
* **(b) Keep `potato` at 0.0625 for characters.** Today's behaviour. ⚠ The
  interim quality is whatever 0.0625× produces, which is what prompted the
  question.
* **(c) Change the LADDER instead** — make `potato` 0.125 so the steps are
  uniform, and regenerate. ⚠ Not an interim policy, and it needs the
  generator/trim repair the row says is measured separately; it is listed
  because the non-uniform step is a plausible cause of the thing being
  complained about, and (a) would paper over it.

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

## Q103 — what should an unprepared character id inherit at wear time?

Prepared characters fold catalog movement tuning and motion model at admission, but the wear road still has a fallback for ids outside the prepared registry. Choose one contract: inherit the catalog's authored tuning at wear time, inherit engine defaults, or refuse an unprepared wear. The shipped compositions currently have no orphan prepared ids, so this is a boundary-policy decision rather than a live content defect.

## Q109 — should a simulated identity be able to name its room instance?

Current deterministic ids identify authored/simulated objects but do not encode a room-instance dimension, so a second instance of the same authored room can collide with an already-live identity. Decide whether room-instance identity belongs in canonical `SimId` semantics or should be represented by a separate deterministic scope. This gates A8's two-instance proof.

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
(`crates/ambition_platformer2d_shared_tangle/src/lifecycle/session.rs:437`),
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
`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`. ⛤ **ITS
`#[ignore]` REASON NO LONGER READS "demonstrates an unfixed defect" AND THIS
SENTENCE SAID IT DID UNTIL 2026-09-19** — the probe is `#[ignore = "PROBE,
print-only: ..."]` like its nine siblings, and the defect it found is held by a
running `#[test]`, `a_mid_session_load_does_not_reach_back_across_the_rewind`.
⇒ An ignore REASON is a claim about the tree, and it drifts like any other. ⛔ Note its fixture shape:
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

⚠ **EVERY COUNT IN THIS SECTION IS DATED, AND THE POPULATION KEEPS GROWING
BECAUSE THE TREE DOES.** Re-measured 2026-09-19: **121 presence-filtered
components across 21 registering crates, 92 registered, 27 waived, 2 owed.**
The 95/73/18/4 above and the 104/83/20/1 below are what the same instrument
said on 2026-09-17 and 2026-09-18; they are kept as written, because a count
re-stated without its reference point is the defect this page keeps finding.
⇒ Today's two owed are the two the closing paragraph names, so the heading and
the decision agree — it was the middle reading that had drifted, and the
heading a reader would have distrusted first was the one that was right.

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
it anyway would be the sweep this row warned against: the guard's waivers each
state a measurement — 18 of them when this paragraph was written, 27 as of
2026-09-19 — and a fourth row added because its three neighbours moved would be
a waiver with the opposite sign and no measurement behind it.

## Q147 — must a candidate session be built from the generation its own activation commits, or from the one current before it?

**FILED 2026-09-19 BECAUSE THE ROW ALREADY SAID "MAINTAINER CALL" AND NAMED NO
`Q`.** `queue.md`'s `CANDIDATE-GENERATION-ORDER` states *"it is a maintainer
call about what a candidate is supposed to see"* and carried no `Blocked by:`
field, which is the same gap `Q146` was filed for four days after `C07`'s gate
named a ruling nobody had asked for.

**The measurement, and both halves of it are correct.** `commit_content_generation`
must run `.after(AmbitionGameShellSet::Pending)` because that is where
`advance_pending_route` produces `RouteActivated`; reading it earlier was a
frame-late bug fixed 2026-09-12. `prepare_candidate_platformer_session` must run
`.before` that same set, because a candidate that cannot be built must never
retire the session that is playing — A10.5's last-good-world guarantee. ⇒ **No
ordering edge can put the commit before construction**: they pin opposite ends
of one set. A candidate is therefore prepared from the content generation
current BEFORE its own activation committed.

⚠ **AND IT IS NOT `Q118`'S REMAINING INTERVAL, THOUGH THEY SIT ON THE SAME
FRAME.** `Q118`'s open half is an AUTHORIZATION going stale inside one
transaction — a boundary change ordered after the publication breaker and
before the commit acts on the activation. This question is about which content
generation a candidate WORLD is constructed from. One is about a permission
that stops being true; the other is about an input that was read too early.

⚠ **NOT A SCHEDULE PROBLEM, WHICH IS WHY IT IS HERE.** Every edge involved is
individually load-bearing and separately witnessed. What is undecided is what a
candidate is SUPPOSED to see.

✔ **AND TODAY'S SHAPE IS PINNED WHILE THIS IS OPEN**, so the answer cannot be
made moot by a silent move:
`the_candidate_is_built_before_the_router_advances_and_providers_only_adopts`
(`crates/ambition_platformer2d_provider/src/lifecycle.rs`) asserts both edges
and that `GameplaySessionSet::Providers` holds adoption and NOT construction.
Poison-verified on all four arms.

**The decision:**

* **(a) Correct by design — a candidate sees the generation that was live when
  it was built.** The activation's own content commit is a separate transaction
  that lands after it, and a candidate is a world for the ROUTE, not for the
  content edit riding along with it. ⚠ Cost: nothing changes, the row closes,
  and the asymmetry stays a documented property rather than a defect. ⛔ The
  thing to check before choosing it: whether any road can activate a route
  *because of* a content change that the candidate it builds cannot see.
* **(b) A candidate must see its own committed generation.** ⚠ Cost: an edge
  cannot deliver it. Either the candidate RE-FINGERPRINTS at adoption — cheap,
  but it means the built world and its identity claim come from different
  moments — or preparation moves after the commit, which puts construction
  after the router has already evaluated the gate and **re-opens exactly the
  hole A10.5 closed**: a session retired for a candidate that then fails to
  build. That second road owes a replacement for the last-good-world guarantee
  before it can be taken.
