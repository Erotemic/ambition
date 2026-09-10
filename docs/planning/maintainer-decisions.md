# Maintainer decisions

This is the compact durable record of decisions Jon made explicitly. It is not
an investigation log. Git history retains the rationale that was present when a
decision was recorded.

Confidence means:

- **High** — proceed on this basis; reopen only with new concrete evidence.
- **Medium** — current direction; implementation or play may refine it.
- **Low** — tentative preference or deliberately deferred choice.

Agents may add a short consequence when a new decision would otherwise be
ambiguous, but do not paste the investigation that led to it. Open questions
belong in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

## Decision ledger

| Date | Decision | Confidence |
|---|---|---:|
| 2026-07-16 | Perform the identified content evictions. | High |
| 2026-07-16 | Extract the reusable programmatic simulation surface as `ambition_sim_harness`. | High |
| 2026-07-16 | Extract the platformer-provider lifecycle from the `ambition_platformer2d` facade and consolidate the repeated provider protocol. | High |
| 2026-07-16 | Keep cutscenes and encounters as separate domain systems. | High |
| 2026-07-16 | Keep provider registration explicit in the host composition root. | High |
| 2026-07-16 | Defer any boss crate carve until boss behavior converges onto the canonical moveset/action path. | High |
| 2026-07-16 | Reject the proposed named-content scanner and stop adding poison-test ceremony by default. | High |
| 2026-07-16 | Keep the compiler term **lowering** for authored world IR becoming live ECS state. | High |
| 2026-07-16 | Repository-wide knowledge-base hygiene checks are CI/maintainer tools, not routine local validation. | High |
| 2026-07-16 | Preserve historical journals as historical records during documentation cleanup. | High |
| 2026-07-16 | A full rename of `ambition_platformer2d_actor_monolith/src/features/` may be worthwhile, but the name `sim` is not settled and the work is low priority. | Low |
| 2026-07-29 | **Label occlusion / transition nameplates are LOW PRIORITY** — not touched until combat is good. | Low |
| 2026-07-29 | **Do NOT gate or redesign `Interact` yet — it needs a design discussion.** | High |
| 2026-07-29 | **Build the portrait target → art resolver.** | High |
| 2026-07-29 | **Replace the invented lab dummy enemies with real ones that already exist.** | High |
| 2026-07-29 | **DI matters — Smash-style physics is wanted in Ambition itself, not only in versus.** | High |
| 2026-07-29 | **Generic versus ends on HEALTH. Smash Siblings is a separate, specified mode.** | High |
| 2026-07-29 | **Smash Siblings HUD: per-character portrait, stock icons, percentage. No score.** | High |
| 2026-07-31 | **Each robot version is a DIFFERENT CHARACTER. Intended.** | High |
| 2026-08-06 | **Input FILTERING is keyed to the PAD; BINDINGS stay machine-wide.** | High |
| 2026-08-06 | **Dialogue claims only the TALKER's input by default. It no longer stops the world.** | High |
| 2026-08-06 | **A conversation is SUSTAINED, not modal: it breaks on damage or separation, holds its participants if they are capable of it, and barks when it breaks.** | High |
| 2026-08-06 | **Any seat may pause, and the seat that paused drives the menu.** | High |
| 2026-07-30 | Defer the `bevy_ggrs` patch-table leak; revisit once upstream merges the `GgrsFrameTiming` accessor to crates.io. | High |
| 2026-08-08 | **YES — a game may compose this engine WITHOUT a given capability. Capabilities are OPTIONAL.** | High |
| 2026-08-08 | **AMENDMENT to the bbox route: the target figure height is an AUTHORED number with a per-character override — not a compiled constant.** | High |
| 2026-08-08 | **TAKE THE BBOX ROUTE — size and crop the character quad from `body_pixel_bbox`, not the padded frame.** | High |
| 2026-08-08 | **Run a MUCH smaller suite on a docs-only change. Bias toward running fewer tests.** | High |
| 2026-08-08 | **"Declared no abilities" does NOT mean "inherit the dev kit". Character capabilities are AUTHORED, EXPLICIT and COMPOSABLE.** | High |
| 2026-08-08 | **The Perfect Cellular Automaton CAN fly.** | High |
| 2026-08-08 | **Sanic does NOT have blink.** | High |
| 2026-08-08 | **Hitstun needs a REDESIGN in its own session, not a tweak.** | High |
| 2026-08-08 | **The three named robot heavies are DEPRIORITISED, not decided.** | Low |
| 2026-08-08 | **⭐⭐ THE ROLLBACK WIRE FORMAT IS UNSTABLE BY POLICY. The latest build is compatible with itself and nothing else. STOP ASKING.** | High |
| 2026-08-08 | **Portal orientation is AUTHORED per portal, in LDtk. No global setting. Rotation is the default; scale inversion is opt-in and may be unsupported per game.** | High |
| 2026-08-08 | **It is FINE that 1-1's first `?`-block drops its wand into a pit.** | Low |
| 2026-08-08 | **You SHOULD be able to hit GNU-ton during its special.** ⚠ and the boss's authoring is stale generally — not worth deep work unless it is a symptom of an architecture problem. | High |
| 2026-08-08 | **The world keeps living while you die — but it must KNOW you are dying and stop attacking you.** | High |
| 2026-08-08 | **YES — a crawler's collision volume orients with its attachment, and it is the SAME concept as a body under different gravity.** | High |
| 2026-08-08 | **Make most of the Hall cast dormant.** | Medium |
| 2026-08-08 | **Defer all GNU-ton work.** | Low |
| 2026-08-08 | **⭐ MARK staleness where it is found, with EVIDENCE. Sweep later, separately.** | High |
| 2026-08-08 | **(b) Fix the one site; leave the authored-id rule to prose. Revisit the refactor only if it becomes important.** | Medium |
| 2026-08-08 | **⚙ OPERATING NOTE: Jon does not have design intuitions about rollback, and has delegated that design to agents.** | High |
| 2026-08-10 | **⭐⭐ A CHARACTER IS A REUSABLE AUTHORED TEMPLATE, NOT A SINGLETON PERSON — and the enemy-archetype system is deleted.** (answers queue D48; opened D73, closed 2026-08-13) | High |
| 2026-08-13 | **⭐⭐ THERE IS NO SEPARATE "CAN FIGHT" CHARACTER PROPERTY. A character can fight exactly to the extent that its body has abilities/capabilities that can produce combat effects.** | High |
| 2026-08-13 | **Carl Stargan does NOT fly. He fights — because his BODY has abilities, not because of a fighter flag.** (closes D96 5) | High |
| 2026-08-13 | **Skitters ARE Puppy Slug.** `SmallSkitter` and `under_town_skitter` are authored as `npc_puppy_slug`. (closes 2 of D96 1/3/3b) | High |
| 2026-08-13 | **`large_brute` becomes a REAL authored reusable character — a Goblin Brute, with its own Python sprite generator.** (closes the goblin-lab heavy casting call) | High |
| 2026-08-13 | **The dive-drill and its anonymous `Target` may be DELETED. Deletion over architecture for disposable AI-authored content.** | High |
| 2026-08-13 | **Previously unauthored body health is TUNING, not a blocked product decision — pick reasonable numbers and AUTHOR them.** (closes D96 7 and D96 8) | High |
| 2026-08-13 | **Camera orientation is a per-view observer policy. Preserve the existing world-fixed/external-observer camera and add an optional controlled-body/view-subject-relative mode.** | High |
| 2026-08-17 | **⭐⭐ KEEP THE LANDED PER-BODY HITLAG FREEZE. The old *"do not reintroduce a per-body zero-dt"* prohibition is SUPERSEDED. D114 is CLOSED.** | High |
| 2026-08-17 | **⭐⭐ KEEP THE PER-TURN GATE SMALL — `cargo test --workspace --lib` stays OUT of `gate_suite.py` DELIBERATELY.** "Gate" continues to mean an EXECUTABLE gate; the pre-push checklist is a separate validation tier. **D160 CLOSED as an intentional policy choice, and `awaiting-maintainer-decision.md` §9 is ANSWERED.** | High |
| 2026-08-17 | **ACCEPT the CPU showcase's current pacing. Do NOT retune stock count, knockback or damage. D128's pacing/product-acceptance blocker is CLEARED** — D128 stays open for its engineering/presentation defects. | High |
| 2026-08-17 | **REPLACE the `central_hub_main` developer-note sign — do NOT delete it.** The hub wants an orientation sign; only the authoring-language content was wrong. | High |
| 2026-08-17 | **Inventory becomes Morrowind-style: the OCCURRENCE owns, and EVERY inventory entry carries a COUNT — usually 1.** | Medium |
| 2026-08-17 | **A line count is a PROXY. Decompose where it makes sense, and stop making the monolith worse.** | High |
| 2026-08-17 | **Split-screen layout is ADAPTIVE WITH HYSTERESIS.** | Medium |
| 2026-08-17 | **Sprite sizing: give every scale a SHARED UNIT first, then revisit the quad-from-bbox route.** | High |
| 2026-08-17 | **Capability progression SPLITS BY WHAT THE VERB IS: physical verbs are BODY-owned, knowledge is PARTICIPANT-owned.** | Medium |
| 2026-08-17 | **⭐⭐ A MODEL-BACKED CHARACTER IS A REMOTE PLAYER, NOT A BRAIN IN THE TICK — and the question is DEFERRED.** | Medium |
| 2026-08-17 | **The 23 already-clipped sprite sheets are fixed CASE BY CASE, driven by the draw-time warning.** | High |
| 2026-08-17 | **A dropped held weapon PERSISTS OR NOT PER ITEM — authored, not global.** | Medium |
| 2026-08-17 | **ONE WORLD UNIT IS ONE BASE-GRID PIXEL — 16 units to a tile.** | High |
| 2026-08-17 | **A declared character height is a CONTRACT: art scales to it, and a tight tolerance WARNS when the scale drifts.** | High |
| 2026-08-17 | **Landmarks are OPTIONAL SLOTS on a character package — authored when useful, never required.** | Medium |
| 2026-08-17 | **PROMOTE `engine/character-authoring-package.md` to a live ledger row, with canonical height as its FIRST SLICE.** | High |
| 2026-08-17 | **Giant and multi-part bodies declare canonical height by the SAME rule — one vocabulary, no exemption list.** | High |
| 2026-08-17 | **Camera shake stays CONSTANT IN THE WORLD; the field is renamed to say so.** | High |
| 2026-08-17 | **A projectile respects the AUTHORED HURT VOLUME — the same geometry melee uses.** | High |
| 2026-08-17 | **DEFER the per-creature ability absence list until the cast is bigger.** | Medium |
| 2026-08-18 | **MINIMIZE POISON TESTS — poison only below ~60% certainty that the guard bites.** | High |
| 2026-08-18 | **AUTHORING A CHARACTER'S SIZE MUST BE CONSISTENT AND TRIVIAL TO TUNE — one number, and the geometry follows.** | High |
| 2026-08-18 | **MARY-O IS ONE BRICK TALL SMALL (16) AND TWO GROWN (32), AND HER SMALL ART IS REWORKED TO HALF THE GROWN HEIGHT AT THE SAME WIDTH.** | High |
| 2026-08-18 | **THE TALL SPRITE MAY BE VISUALLY WIDER; THE COLLISION WIDTH STAYS IDENTICAL FOR BIG AND SMALL.** | High |
| 2026-08-19 | **D166's CPU-GRAB WORK IS THE POLICY HALF FIRST: the FIGHTER CAPABILITY owns what a HOLD is WORTH.** The mechanical fixes (a start-gate on the option list, tighter grab spacing) wait behind it. | High |
| 2026-08-19 | **`body.action_buffer` STAYS — a registered rollback row with no writer, DOCUMENTED as declared-but-unfed.** | Low |
| 2026-08-19 | **AI SLOP HONOURS ITS AUTHORED SIZE CONSTANT — every slop shrinks from 73.9 × 48 to 28 × 18.2.** | Medium |
| 2026-08-19 | **THE `.loop` CUE DERIVATION TRAP IS LEFT RECORDED — no exemption list, no fallback.** | Low |
| 2026-08-19 | **`test_oiler_svg_rig.py`'s EIGHT AUTHORING-STRUCTURE TESTS STAY. Oiler's SVG is settled.** | Low |
| 2026-08-19 | **D162's THREE SHEET-MANIFEST COLLISIONS GO TO JON PAIR BY PAIR — written up, not resolved by rule.** | Medium |
| 2026-08-20 | **AVOID PUSHOUT is about PORTALS, not bodies. Jostle is allowed — but it may NEVER be a mandatory part of the movement kernel.** | High |
| 2026-08-22 | **RENAME THE BLAST ZONE OUT OF EVERY WORLD — `World.edges: WorldEdgeMargins { fall, side, rise }`, Rust and LDtk keys in ONE change.** | High |
| 2026-08-22 | **THE BAKED SHEET REGISTRY KEYS BY FILE ROOT — a renderer target string may not be a durable engine identity.** | High |
| 2026-08-22 | **`SeatRawFrames` STAYS GENUINELY RAW — build the stage model, and world-dependent semantics land AFTER the boundary, not before it.** | High |
| 2026-08-22 | **SWEEP THE CRATES A CARVE TOUCHES — do not widen the gate to `--workspace --all-targets`.** | High |
| 2026-08-22 | **A LEVEL'S POSITION IS OWNED BY THE LAYOUT TOOL — and ownership FOLLOWS THE LAYOUT MODE.** | High |
| 2026-08-22 | **HEIGHT OWNS WORLD SIZE; SOURCE-ART DENSITY IS A SEPARATE CONTRACT — and the 1.0 warning is REMOVED, not replaced.** | High |
| 2026-08-22 | **A HIT'S ART FOLLOWS BOTH THE VICTIM'S MATERIAL AND THE BLOW'S STRENGTH.** | High |
| 2026-08-22 | **IMPACT HITSTOP IS A BOUNDED MATCH-LEVEL REQUEST FROM THE CONNECT — it is combat presentation, not a slot-0 affordance.** | High |
| 2026-08-22 | **A BOSS'S HOARD IS PER-BOSS EVENTUALLY; FOR THE DEMO IT IS CURRENCY.** | High |
| 2026-08-22 | **`DebugLabel` IS DEBUG, AND IT KEEPS SHIPPING — the whole world is scaffold.** | High |
| 2026-08-22 | **PROXIMITY-GATE EDGE-EXIT LABELS — and label visibility is a GAME-SELECTABLE POLICY, not a fixed rule.** | High |
| 2026-08-22 | **DISABLE rust-analyzer — no second target directory.** | High |
| 2026-08-22 | **CORRECT THE LEVEL-1 CPU FOR FEEL — the easiest rung is bad at FIGHTING, not self-destructive.** | Medium |
| 2026-08-22 | **MARY-O KEEPS THE 56 px SHARED COLLISION WIDTH, AND HER SHORT-FORM CROWN RISES ~6 px.** | High |
| 2026-08-22 | **FIX MARY-O'S WALK DIP PROPERLY — add a pose field that lowers the TORSO without moving `foot_y`.** | High |
| 2026-08-22 | **THE MARY-O RESTART REPORT IS CLOSED — it was Mary-O, and it is believed RESOLVED.** | Medium |
| 2026-08-22 | **ADVANCE THE `dev/ambition_dev_measurements` POINTER PERIODICALLY — the cadence does not matter.** | Low |
| 2026-09-10 | **A PUBLISHED COLLISION SURFACE PARTICIPATES IN PROJECTILE COLLISION, AND A DESTRUCTIBLE'S SURFACE PLUS ITS HURT VOLUME ARE ONE COMPOUND CONTACT** — damage once AND apply the surface response. (Q96) | High |


## Supplemental rulings that were previously stored as long-form sections

- **2026-08-15 — reset semantics:** the checkpoint is the reset baseline. A
  replay restores what the checkpoint promises; do not infer a second reset
  policy from entity lifetime.
- **2026-08-17 — item semantics:** physical occurrence, custody, entitlement and
  durability are separate facts. Dropped weapons persist or not **per item**, by
  authoring; ordinary drops may be room-scoped while story/unique items persist.
- **2026-09-02 — visual quality:** a lower quality setting may use fewer source
  pixels, but no mechanism may draw fewer pixels than the selected quality tier
  promises.
- **2026-09-05 — portal presentation:** portal presentation is a composition
  policy; Smash may disable the seamless presentation without weakening the
  reusable portal mechanism.
- **2026-09-05 — Limit:** the meter may fill from the obvious authored sources;
  generic meter validation must not encode one Smash balance doctrine.
- **2026-09-05 — demo items:** important demo items deserve real presentation;
  placeholder art is not a permanent design decision.
- **2026-09-05 — authorship:** what Jon explicitly authored is the demo's claim.
  Agents may polish execution, but should not replace that authored idea with a
  different move/content concept merely because it is easier to implement.
- **2026-09-10 — projectile contact with published surfaces (Q96):** a projectile
  must not know *"this is an ECS breakable"*; it must know *"the collision world
  published a surface with these collision semantics."* A
  `BreakableCollision::Solid` surface therefore participates in projectile
  collision. Where the same contributor supplies both a surface and a damageable
  volume at the same time of impact, they **coalesce into ONE compound contact**:
  damage the target once **and** apply the projectile's physical surface response.
  A bouncing shot damages a solid crate **and** bounces. *"Wall wins, therefore the
  crate is invulnerable"* is rejected. Where the surface lies before an INSET hurt
  volume, only the surface was reached — no damage yet.
- **2026-09-10 — projectile exemptions are POLICY, not FAMILY (Q96):** a ghost
  shot, phase shot, terrain-piercing round or one that ignores one-ways excludes
  appropriate **collision CLASSES**. Never a per-target carve-out.
- **2026-09-10 — contributor identity must be REAL identity (Q96):** not inferred
  from matching AABBs, and not from name strings such as `"ecs-breakable foo"`.
  This ratifies the projectile contact protocol's existing wording.

## Maintenance rule

When a decision is superseded, edit or replace the row. Do not append a second
page of commentary underneath it. Git history is the record of how the ruling
changed.
