# Collision-invariant oracle — first sweep findings (2026-06-04)

Autonomous long-run deliverable for TODO §D **"Headless collision-invariant fuzz
oracle"** + groundwork for Jon's deferred non-autonomous **OOB-fixing** session
(Jon's List: *"significant numbers of out of bound errors… defer for a
non-autonomous run"*). The oracle is the detection legwork so that session starts
with a precise repro list instead of flying around by hand.

## What it is

`crates/ambition_sandbox/tests/collision_invariant_oracle.rs` — a fuzz-driven
per-tick invariant checker over the deterministic `SandboxSim`. Each tick it
reads the player's live AABB + the room's Solid collision world and flags:

- **EMBEDDED-IN-SOLID** — player center sits inside a Solid block (the
  "teleported into a wall" / clipped-through signature).
- **OOB-ABOVE-CEILING** — center went above `y=0` (the bug Jon hit flying up).
- **OOB-BELOW-FLOOR** / **OOB-SIDE** — center left the world the other ways
  (below is usually a legit gap fall; the catalog labels the side so a human can
  tell bug from gap).
- **TELEPORT** — a single-tick jump > 250px that is **not** a door load or a
  death→respawn (both filtered via `active_room` / `resets` deltas), so it only
  fires on a genuine same-room in-place warp. (Blink carries 150px, under the bar.)

It is a **diagnostic, not a CI gate** (deliberate): OOB-via-authored-gap is
expected in some rooms, and the embed/teleport classes are the *deferred* bugs —
a hard assert would false-positive on gap rooms or red-light CI on a known-
deferred bug. So the in-CI `collision_oracle_smoke` only proves the harness runs;
the catalog comes from the `#[ignore]`d full sweep:

```bash
cargo test -p ambition_sandbox --test collision_invariant_oracle \
    -- --ignored --nocapture
```

## First-sweep result — the serious OOB classes do NOT reproduce

**58 rooms × 3 seeds = 174 episodes, 52,200 steps, 1349 violations.**
Every single violation is **OOB-SIDE** or **OOB-BELOW-FLOOR**. There were:

- **0 EMBEDDED-IN-SOLID** — no player ever had its center deep inside a wall.
- **0 OOB-ABOVE-CEILING** — nobody popped out the top flying up.
- **0 TELEPORT** — no in-place warps.

This is the headline: the **"teleported into a wall"** and **"flew up and popped
OOB above the ceiling"** bugs Jon reported this session **did not reproduce in
52k fuzzed steps across every room** — consistent with the two OOB fixes shipped
earlier in this run (`resolve_x_penetration` far-edge/never-eject de-pen, and the
ceiling-graze swept-de-pen defer-regardless-of-immediate-contact;
`crates/ambition_sandbox/src/engine_core/movement/collision.rs`). The stuck-in-
wall class looks closed.

Caveat (be honest): the fuzz is biased toward vertical/fly/pogo/jump input to
stress ceilings, but it is *random*, not Jon's exact input sequence — "did not
reproduce" reduces, not eliminates, the possibility. A targeted replay of Jon's
trace is the complement. Also the embed check is center-based (catches deep
embedding, not a partial clip that resolves the same tick), and a same-tick
OOB→respawn is invisible to a post-tick observation.

## The catalog — minor edge-straddle OOB, by room

All remaining violations are the player **center crossing a world boundary by
~17–26px** — i.e. the AABB straddling the edge (body mostly in-bounds, center
just past). This is the "permitted OOB" the base fuzzer already tolerates, and it
clusters in rooms with **open / exit boundaries**:

| Room | Kind | Count | First repro (seed/tick/pos) |
|------|------|------:|------|
| under_town_pipes | OOB-SIDE | 415 | 1 / 25 / (-19,613) |
| intro_escape_shaft | OOB-SIDE | 280 | 1 / 28 / (-17,1124) |
| under_town_pipes | OOB-BELOW-FLOOR | 188 | 1 / 112 / (-136,787) |
| ninja_dojo | OOB-SIDE | 117 | 2026 / 143 / (1822,1519) |
| alice_relay | OOB-SIDE | 94 | 2026 / 153 / (1042,428) |
| bob_relay | OOB-SIDE | 89 | 2026 / 156 / (1048,392) |
| pirate_sky_lookout | OOB-SIDE | 78 | 42 / 79 / (-22,547) |
| intro_escape_shaft | OOB-BELOW-FLOOR | 48 | 1 / 112 / (-96,1299) |
| tiny_chamber | OOB-SIDE | 40 | 2026 / 189 / (926,156) |

Reproduce one: `cargo run -p ambition_sandbox --bin rl_random_walker -- <STEPS>
<SEED>` after launching that room as `--start-room`.

### Interpretation (for the interactive session)

The high-count rooms (`under_town_pipes`, `intro_escape_shaft`) have boundaries
the player walks/falls off easily — almost certainly authored **EdgeExit / gap**
boundaries (a shaft is open top-to-bottom by design), so the side/below OOB there
is expected. The `*_relay` and `tiny_chamber` / `ninja_dojo` side-OOB are small
(18–26px past a right/left wall) and worth a glance: either a legit edge-exit or a
"you can nudge your center a half-body past the boundary wall" looseness.

The triage that needs a human (or a follow-up the oracle can't do blind): **cross-
reference each OOB-SIDE room against its LDtk LoadingZone/EdgeExit geometry** — an
OOB at an authored exit is fine; an OOB through a *solid boundary wall* with no
exit is the real bug. That cross-check (read the room's loading zones from the
sim and suppress OOB within an exit's span) is the natural next enhancement to the
oracle, deferred because the exit geometry isn't in the observation surface yet.

## Follow-up enhancements (deferred)

1. Suppress OOB that lands within an authored EdgeExit/LoadingZone span (turns the
   side/below noise into only genuine through-wall OOB).
2. A scripted-replay mode that feeds Jon's captured OOB trace inputs (targeted
   repro alongside the random sweep).
3. Optionally promote EMBEDDED/ABOVE/TELEPORT to a CI gate once they're confirmed
   zero on a stable build (they are zero today) — a regression tripwire for the
   stuck-in-wall class that was just fixed.
