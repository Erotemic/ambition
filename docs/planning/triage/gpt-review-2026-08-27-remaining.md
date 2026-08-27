# GPT 5.6 review — the work it left open (relayed by Jon, 2026-08-27)

Reviewed HEAD `a945c1de5`. This is the SECOND pass: the reviewer re-read main
after the first sixteen items landed, and these are what survived that re-read
plus what the fixes themselves opened. ⛔ Do not re-work the first pass; that
one is closed and its rows are in `queue.md`.

⚠ Some rows below say a proof is FALSELY CLOSED. That is the most valuable kind
of finding in this file and the least visible one — a green test whose subject
is not what its name says. Prefer those over the new features.

Status marks: ▢ open · ▣ done (with sha) · ⊘ retired with a reason.

## Priority 1 — recovery and the shark

- ▣ **R1 — CLOSED `533bd9a05`. The shark recovery episode's cost is refunded
  before it is paid.** Measured frame by frame: spent on the press frame, full
  again on the next one, because the landing-class refresh asks `on_ground`
  every TICK and a body that never left the floor answers yes. Not a vehicle
  rule — every fighter's grounded up-B was refunded the same way.
  `MotionStepContext::recovery_commitment_outstanding` is derived each tick from
  the `MovePlayback` (no second copy to desync) and `RecoveryRefresh::Withheld`
  holds the recovery back while still refreshing the aerial resources. Poisoned:
  forcing `Answered` fails the flinch refund and the lease-release arm.
  `call_the_shark` is `RecoveryUse::SpendWithoutFreefall`; `start_move` spends
  the charge; but the move begins GROUNDED, and ordinary grounded movement
  refreshes `recovery_charges` before the shark takes movement authority. So the
  Pirate boards with a full charge and "flinch while mounted refreshes recovery"
  cannot be tested at all. ⛔ The fix is NOT removing grounded refresh — a
  grounded Up-B that stays on the floor needs ordinary landing-class semantics.
  Represent the vehicle recovery EPISODE so its committed cost survives the
  handoff to sustained movement authority. Belongs beside R13's route
  semantics, not as a Pirate-id exception.
  Acceptance: grounded `call_the_shark` commits the cost · after boarding and
  before any hit the Pirate has a SPENT recovery · a real weak flinch lands,
  leaves `RidingOn` intact, restores it · leaving the ride WITHOUT that flinch
  manufactures nothing · ordinary grounded recovery unchanged. Reuse the weak
  hit in `smash_ride.rs::a_flinch_leaves_the_admiral_aboard_and_a_launch_takes_him_off`,
  which lands correctly as of `a945c1d`.

- ▣ **R2 — CLOSED `65b89da85`. The assembled mounted-launch test is green with
  the fix reverted.** Replaced with EQUIVALENCE: the same strike on the same
  admiral, airborne under his own jump and airborne on the shark, must leave him
  with the same velocity. Both roads produce `(-1884.0382, -1884.0382)` to the
  last decimal. Poisoned: reverting the deferral releases the mounted arm at
  exactly `(0, 0)`. Sampled on the RELEASE TICK — four frames of gravity is what
  buried the difference before.
  The kernel regression covers the deferred-launch behaviour honestly. The
  ASSEMBLED shark test does not: its lateral-velocity assertion survives
  reverting the fix because something else produces lateral motion. Build the
  two-arm poison — same Pirate, same strong hit, comparable start; arm A
  unmounted, arm B mounted — and compare the ACTUAL launch velocity after
  release. ⛔ No gravity-only or unrelated-velocity checks.

## Priority 2 — two inspector proofs that are falsely closed

- ▣ **R3 — CLOSED `630411c89`. The shark-health census scans a hand-kept table,
  not the roster.** `SmashRoster::assemble` against a live registry: 21 fighters,
  21 movesets. Ranged resolved the way `trigger_moveset_moves` resolves it, at a
  full charge. Worst melee 36 (George Booul's forward smash), worst ranged 14
  (Projectile Polygon's charge shot). The shark's 40 holds — the census was the
  defect. The ranged maximum is asserted non-zero as a premise.
  `authored_movesets.rs` no longer claims to be the cast.
  `a_recovery_mount_cannot_be_deleted_by_one_hit` claims the whole selectable
  cast and actually builds `ambition_content::authored_movesets::tables()` plus
  George Booul by hand. That helper is INCOMPLETE — Pointed / Projectile /
  Pugnacious Polygon, Author, Actor, Officer, Medic, Mary-O and Sanic are
  selectable and absent. The population road already exists:
  `SmashRoster::assemble(registry)` and each `prepared.kit.projectable_moveset()`.
  Also: a one-hit maximum that inspects only `MoveWindow.volumes` cannot see
  RANGED offense. Resolve a ranged event the way runtime does — the move's
  equipped weapon when `spec.equips` supplies one, else the prepared
  character's `ActionSet.ranged` — and include the full-charge ceiling for
  chargeable ones. Keep `SUMMON_SHARK_HEALTH > max single connection`; 40 HP may
  well still hold. ⛔ Delete every claim that `authored_movesets::tables()` is
  the selectable cast.

## Priority 3 — concrete gameplay defects

- ▣ **R5 — CLOSED `69a0918f5`. A held bomb explodes where it was picked up.**
  `ItemWorldPos` answers "where is this item" generically: `InWorld` is the
  world's copy, `Held` is the holder's HAND (the same `rider_hand_world_pos` the
  wielded-item presentation draws with). ⛔ Not by keeping `GroundItem::pos`
  updated for held items — that gives a held thing two authorities. `bomb.rs` blasts at
  `GroundItem.pos`; the fuse keeps burning under `ItemCustody::Held`; held-item
  physics deliberately stops updating that position. Give item custody a
  generic world-position semantic (`InWorld` → `GroundItem.pos`, `Held{holder}`
  → the holder's hand) and have the bomb CONSUME it rather than Smash code
  writing `GroundItem.pos`. Regression: pick up at X, carry to Y, expire, blast
  at Y.

- ▣ **R6 — CLOSED `216316bef`. Gun-sword discharge is keyed to the string
  `"gun_sword"`.** `Discharge` (muzzle, fire cue, recoil) is authored on
  `RangedActionSpec` beside the visual that was already there; both gun-swords
  share `gun_sword_discharge()` and keep their own damage, speed and assist. The
  fire site knows no weapon's name and its held-item query is gone. Regression
  through the REAL side-B in the shipped composition (lasersword, 8 damage, hand
  muzzle, -390 kick) plus a headless cue arm. Poisoned three ways.
  `brain_effects.rs` gates the lasersword projectile visual, the hand muzzle,
  `weapon.lasersword.fire` and the stronger recoil on an id compare. The
  Admiral's side-B equips `admiral_gun_sword` and gets none of it, though the
  move plainly intends all of it. ⛔ Do not add a second string compare.
  Author a DISCHARGE PROFILE on the weapon / ranged-action data (projectile
  visual, muzzle policy, fire SFX, recoil profile) that both ids share while
  keeping their own damage, speed and aim assist. Regression through the real
  Admiral side-B, including its 8 damage and its assist.

- ▣ **R7 — CLOSED (with R6's push). Aim assist bends toward `OutOfPlay`
  bodies.** The scan asked `health.alive()`, and a spent stock calls
  `health.reset()`, so a fighter in its death beat read FULL HEALTH.
  `body_is_untouchable` is the gate. The regression's dead candidate is the
  BETTER target, and both halves are asserted so the arm turns on eligibility
  rather than geometry. The candidate scan in
  `brain_effects.rs` filters on health and relation and never rejects
  `OutOfPlay`, so a respawning fighter steals an assisted shot. Distinct from
  the CPU target-selection fix already landed. Use the shared liveness
  semantic, not another local health approximation. Regression: two hostile
  candidates, the nearer one `OutOfPlay`, shot bends to the live one.

- ▢ **R8 — no deliberate stow for a stored charge.** Stored charge survives
  interruption and clears on death, but there is no player action equivalent to
  putting a Samus/Mewtwo charge away. Define the interaction and route it
  through GENERIC charge semantics — ⛔ not a Projectile Polygon id check.
  Flow: partial charge → deliberate stow → `StoredMoveCharge` exists → other
  actions resume → later neutral-B resumes or releases the bank → firing
  consumes it. Death clearing stays.

- ▢ **R9 — the ponytail dies on its first body hit.** `projectile/systems.rs`
  despawns on contact, so the landed return-flight work is unobservable in
  combat. Deleting the despawn machine-guns overlapping victims instead.
  Rule: each victim hit at most once per LEG; the return leg re-arms every
  victim; overlapping for several ticks does not repeat; hitting A does not
  protect B. The leg may be derived from motion, but the per-victim history is
  AUTHORITATIVE ROLLBACK STATE — so this owes the component, the snapshot
  codec, the schema baseline, the presence probes and the recreation tests.

- ▢ **R10 — item interaction is keyed to the `ControlledSubject` singleton.**
  Pickup and throw serve exactly one locally possessed body; Smash has several.
  Move press-gated item interaction onto the participant/body control
  population so each local seat picks up, drops, throws and uses ITS OWN item.
  Keep the semantic-control-edge consumption rule (one press must not both
  throw and jab). Regression: two local players, two items, neither stealing
  the other's control path.

- ▢ **R11 — "hard impact" means "touched static world geometry".** The
  impact-speed sampling bug is fixed, but the bomb still learns about hard
  impacts only through the world-geometry `SettledItem` path. Decide the other
  hard-collision categories for a thrown bomb and implement them from the
  COLLISION AUTHORITY's facts, not distance checks in `bomb.rs`. At minimum
  make the policy explicit and tested.

## Priority 4 — architecture still open

- ▢ **R12 — finish the `PoseOwnedExternally` contract.** It currently means
  four things (zero voluntary axes, verbs cleared, tumbling launch travel
  deferred, kernel otherwise runs) and says nothing about what an externally
  posed body still ADVANCES. Decide ownership explicitly for gravity,
  collision/contact transitions, grounded state, maneuver state,
  jumps/dodges/resources, external launches, velocity/displacement and
  surface-momentum bookkeeping. ⛔ Not by teaching the mount layer Smash tumble
  policy. End state: ONE displacement authority, with the rider's combat and
  action controls still working. Contract tests for whatever is deliberately
  allowed to advance.

- ▢ **R13 — D250: recovery AI needs sustained-authority routes.** CPU recovery
  understands only a `RecoveryLift`, a one-shot displacement. `call_the_shark`
  is deliberately not one: it summons a steerable flying body and BUYS SECONDS
  OF MOVEMENT AUTHORITY. ⛔ Do not fake a lift and do not special-case the id.
  Extend route semantics to burst displacement / sustained authority /
  teleport, with the shark as the first sustained customer. Coordinate with R1
  so the player's recovery budget and the AI's description of the same episode
  do not become two models.

## Priority 5 — proof and bookkeeping

- ▣ **R14 — CLOSED. Pin the production boomerang constructor.** The dynamics
  test lives in `ambition_projectiles`, which does not depend on
  `ambition_characters`, so it restates the values it drives. The new arm
  asserts the RULE (`max_lifetime == 2 · out_s`) across a range — one sample
  cannot tell `2·out_s` from `out_s + 0.34` — and each test says which half the
  other owns. Poisoned: `+ 0.15` fails it.
  `a_boomerang_turns_around_and_returns_to_where_it_was_thrown` hand-writes
  `boomerang_return_s = OUT_S` and `max_lifetime = OUT_S * 2`, so restoring the
  old `+0.15` in `ProjectileFlight::boomerang` leaves it green. Read the real
  constructor's values in a test; reintroducing the old lifetime must go red.
  Keep the dynamics assertions.

- ▢ **R15 — the `moveset_takes` ownership poison was never built.** Ownership
  recording is implemented; the adversarial proof is not. Record a take for a
  known HITLESS subject move while the live CPU opponent actually attacks and
  fires. Require: the opponent's output IS present in contextual frame data,
  and the subject-owned hitbox/projectile maxima stay ZERO.

- ▣ **R16 — CLOSED. Direct Robot special regressions at the seam.** Five arms
  from one table: neutral shields, side/up/down/airborne-down do not. Plus the
  compatibility body, which has no repertoire and must still shield on any press
  — that is what the layer was written for. ⚠ The air-dodge half IS the airborne
  row: `shield_held` is what arms and spends one, so the cause is asserted rather
  than the symptom. Poisoned. ⛔ There is ONE resolver: the fix calls
  `attack_dir_from_axis` + `move_for_directional_verb`, the same two the combat
  road uses. Behaviour is fixed
  (`22361bab3`) but only `moveset_takes` proves it. Add poisons at the seam:
  neutral Special raises the bubble shield · side Special synthesizes no
  `shield_held` · down Special likewise · an airborne directional Special does
  not arm or spend the air dodge merely because the Robot owns a bubble shield.
  ⛔ If this is refactored, do not end with two independent directional-Special
  resolvers — extract one pure semantic.

- ▣ **R17 — CLOSED. One blink per Author teleport.** Counted through the real
  move in the shipped composition, for that body only, with a premise that he
  crossed 250px. Poisoned: restoring the authored cue reads 2. The duplicate `player.blink`
  request is gone from the code; add the regression through the real teleport
  that counts EXACTLY one cue.

- ◐ **R18 — repair stale review and planning status.** `queue.md` (D252, D253,
  D254) and `JONS_OBSERVATIONS_BUGS_AND_ISSUES.md` are current as of this push.
  What remains is whatever the OPEN rows below close. D253 and D252 are closed
  as of `4a551ccc5`. Still to correct: the Author double-blink row (fixed at
  `947b97b`), `tick_departures` docs (already corrected), the `shark_ride_probe`
  fixed-frame-240 wait (already condition-based). REOPEN: shark health (R3),
  ranged inspector, the flinch test's recovery-refund half (R1), and the shark
  launch's assembled equivalence proof (R2) — the kernel half is done, the
  assembled half is not. ⛔ Do not preserve any claim that a hand-maintained
  authoring table is the selectable roster.

- ▢ **R19 — trim incident history out of runtime source.** Recent fixes left
  forensic narrative in production files. KEEP the invariant, the reason a
  non-obvious implementation exists, and which authority owns the decision.
  REMOVE dates, model attribution, dead hypotheses, debugging chronology and
  historical values that no longer explain current behaviour. Planning docs own
  incident history. ⚠ Do this LAST and conservatively, so architectural
  explanation survives the trim.
