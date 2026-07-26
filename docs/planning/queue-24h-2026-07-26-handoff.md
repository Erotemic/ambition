# Handoff — the 24h planning sweep, 2026-07-26

**Read this first, then [queue-24h-2026-07-26.md](queue-24h-2026-07-26.md).** That
file is the live ledger the goal guard reads; this file is what a fresh agent needs
that the ledger does not say.

---

## 1. What the run is

Jon's instruction, verbatim: *"address everything in docs/planning and every follow
up that is found while implementing docs planning. The hook should basically say:
there's no way you finished everything in docs/planning, get back to it."*

Enforced by `scripts/goal_guard.py`, armed from `.goal/planning-sweep.json`,
deadline **2026-07-27T15:00Z**. Check status any time:

```bash
python3 scripts/goal_guard.py --status
```

**The ledger check is open BY DESIGN.** An empty queue means the next item has not
been written down, not that the work is done. Refill it from `docs/planning`.

⚠ `.goal/` is gitignored, so the armed goal is NOT in git. A committed copy lives
at [queue-24h-2026-07-26-goal.json](queue-24h-2026-07-26-goal.json). If a fresh
session finds the guard disarmed:

```bash
cp docs/planning/queue-24h-2026-07-26-goal.json .goal/planning-sweep.json
python3 scripts/goal_guard.py --arm .goal/planning-sweep.json
```

## 2. How to work here (learned the hard way this run)

- **`./run_tests.sh --fast` is job 1 only.** The full `./run_tests.sh` (18 jobs) is
  the only thing that compiles the demo apps, and `cargo check --all-targets` does
  NOT. Two broken startup blocks hid there earlier in this campaign.
- **`./run_tests.sh -p <crate>` can run jobs the 18-job plan does not.** A red
  default-features Mary-O job exists at HEAD (A21). Don't mistake it for your own
  regression — verify by stashing.
- **NEVER write a goal check that greps for the ABSENCE of an identifier.** It went
  red on prose three times: documenting a removal breaks the guard that verified
  it. Name a test instead.
- **A green test proves nothing until you have seen it red.** Two checks passed for
  the wrong reason this run, and one fixture made its own point vacuous (an
  authored torso the same size as the body box). Break the fix, watch the test
  fail, restore.
- **Read the output, not the exit code.** A background job reported success while
  the suite was 17/18.
- Cargo lives at `$HOME/.cargo/bin/cargo`; the hook PATH has no cargo.

## 3. The instrument built this run — use it

`crates/ambition_runtime/src/rollback/probes.rs` — per-component and per-resource
localization across the save/load boundary. A GGRS sync test reports ONE aggregate
checksum; this names the type.

```bash
./run_tests.sh --heavy -k which_component     # localizer
./run_tests.sh --heavy -k which_population    # older entity-class bisection
./run_tests.sh --heavy -k list_what_every_waiver   # what each waiver covers
```

It turned a day of bisection into a 3-second answer, and it is now coupled to the
registration seam: a type cannot be rollback-registered, or declared derived,
without also getting a probe. **If any rollback divergence appears, run the
localizer FIRST.**

⚠ That coupling sentence was written before it was true (F3). `record_probe` was
called from five of the ten state-bearing registration arms, so 145 types —
including `ProjectileOwner`, the state A6 turned on — were invisible to the
localizer while it reported success. All five arms probe now, and
`every_state_bearing_rollback_registration_owns_a_localization_probe` (not
`#[ignore]`d) is what makes the sentence checkable rather than aspirational. If
you add a registration arm, it needs a probe or that test fails.

Two design points that matter if you extend it: per-entity checksums combine with
a wrapping SUM (XOR annihilates equal pairs — a component held identically by two
entities censused as `0x0`), and derived probes are compared at the RESIMULATION
boundary only, because derived state is legitimately absent right after a restore.

## 4. The recurring defect class — this is the real content of the run

Six of the eleven things fixed were the same shape: **state that looks accounted
for and is not.**

- `ProjectileOwner` was `declare_rollback_derived` naming a system whose query
  could not see enemy projectiles. It was registered as a lie, so every guard
  passed. That was the entire equipment-oracle divergence (A6).
- `BossAnimFrame` — a sim-owned cursor boss hurtbox geometry derives from — was
  swallowed by a CRATE-PREFIX waiver reading "sprite metadata / asset binding". The
  type's own first doc line says *Sim-owned* (A9/A18).
- `PogoTarget`, `PogoTargetContributor`, `ChestFeature` were simply never in the
  swept population, because no swept room had a boss, a chest, or a portal (A19).
- Authored hurtboxes were published into a component **no damage path read** (A7).
- `write_from` tagged cues with a source **nothing authorized**, so they were
  silently denied (A4).

The pattern: a mechanism exists, looks wired, and has no consumer or no checker.
When you add a seam, ask *who reads this* and *what would fail if it lied*.
`declare_rollback_derived_component`/`_resource` and the probe coupling exist to
make that question mechanical rather than remembered.

## 5. State of play

**Closed and committed:** A1–A9, A11, A12, A13, A17, A18, A19, plus A5's
provenance half, plus **F1–F5** (a second GPT-5.6 review of the same run —
section F of the ledger). The rollback oracle is green and un-quarantined.

**The goal is ARMED**, 19 checks: the original 13 plus one per closed F row. Six
of the F checks were green at arming rather than red, which is the correct state
for a regression guard on already-closed work — the RED-at-arming rule in §1
applies to a check for work not yet done.

**What F1–F5 changed, if you are picking up cold:**

* a body's cue attribution is read from `BodyPresentationSource`, published on the
  SIM schedule before the move clock; `advance_move_playback` re-derives nothing;
* `CharacterLoadStates` is history (`staged_tokens()`), `StagedCast` is the roster
  (`cast()`), and it belongs to one `SessionScopeId`;
* the rollback localizer probes all 250 state-bearing registrations, not 105, and
  a non-ignored test forces that coupling;
* the coverage sweep's population comes from the rollback VOCABULARY, so it
  reaches transients — which turned up `PortalGunPickup` and `PortalHostScanned`
  as unregistered sim state.

**Open, in the order I would take them:**

1. **A16** — the oracle's route has NEVER broken the brick or flipped the switch.
   Currently guarded by an INVERTED assertion that goes red the moment the route
   reaches them; restore the real assertions then.
2. **A10** — audit for other vacuous A-not-B fixtures (see §2).
3. **A20/A21** — mounts, falling sand and the shop have no LDtk room, so sweeping
   them needs a Rust fixture; and the default-features demo jobs are outside the
   plan.
4. **A14** — `MessageReader` cursors are `Local` state GGRS never rewinds. Measured
   NOT to be A6's cause (hoisting both changed the checksums not at all, so it was
   reverted rather than landed as churn) but a real latent hazard the roadmap's
   Task 1 already calls out.
5. **C3 first, then C1/C2/C4** — `character-definition-design.md` §0 follow-ups.
   C3 was promoted by F6: production fighter construction still reads
   `CharacterCatalog` for its action set, moveset, and movement tuning, so
   registering a character does NOT yet reach a production-spawned body. The
   §7.10 fight test projects the prepared definition by hand, and the slice table
   now says so. Doing C3 is what makes "read models derive from the prepared
   authority" true. Then: the effort migration off absolute
   `patrol_speed`/`chase_speed`; **no character authors a `HurtboxDoc` yet** (now
   worth doing, since as of A7 the seam genuinely reaches damage); no versus mode.
6. **D1–D4** — `competitive-2d-platformer-engine-roadmap.md`: eleven of twelve tasks
   still carry no status marker, which is its own defect.

## 6. Jon's answered question, for continuity

He asked whether we can start **Super Smash Siblings**. Answer given: the character
layer is ready — two providers' characters trade damage through the production
chain, and percent-scaling knockback already exists (`scaled_knockback`).

⚠ Sharpen that first clause with what F6 established: the DAMAGE path is production,
and the body CONSTRUCTION in that test is not. A Smash match will need C3 (project
the prepared definition onto a spawned fighter) before it can seat a registered
character without hand-inserting its components. What does
not exist is a MATCH layer: `ControllerBinding` has no consumer outside its own
module and the fight test, and there are zero hits repo-wide for stocks, blast
zones, or KO. Recommended first slice: a `MatchSession` consuming
`MatchParticipantRoster` that spawns N fighters on a stage, routes each
`Human { device_slot }` to a device, and adds blast zones + stocks + respawn.

⚠ His standing note defers Matchbox netplay *until* Smash. A6 is fixed, so that
gate is now open — but re-read the triage doc before building on it.
