# Cost of the bridge warden

Date: 2026-11-20. The script-first program's capstone (slice C7): a warden
that hears native defeats, counts them in the world, and lowers a bridge
through a confirmed world value. Method as in
[scripted-stage-entry-cost.md](scripted-stage-entry-cost.md).

Host: desktop Linux VM, x86_64, Intel Xeon Gold 5222 reported by the guest,
14 CPUs. Development/test profile (workspace at optimization level 0,
dependencies at 3). Other tests ran beside some of these on the same box, so
read the session wall times as loose. These are development-loop figures,
not release or device budgets.

```bash
cargo test -p ambition_app --test app_it -- scripted_bridge_warden --test-threads=1 --nocapture
```

Each arm prints `BRIDGE_COST` rows from `ExtensionProfile`: the whole entry
(state read, observations, invocation, validation, commit) and the backend
call (VM execution and conversion; for WASM, a fresh instance per call).
Native combat, the world domain and the gate are outside the entry totals.
Replays add calls again.

## The playable project

`examples/projects/bridge_warden`, played by the runner from the yard's
first tick until the bridge is lowered.

| Run | Ticks | Entry calls | Faults | Entry total | Backend total | Session wall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Sync-test host, 4-tick rewinds | 212 | 966 | 0 | 471.97 ms | 395.71 ms | 28.40 s |
| Game's own session, after the live edit | 140 | 133 | 0 | 59.39 ms | 49.51 ms | 1.84 s |

Almost all of it is the yard's `story_tick` entry: 924 of 966 calls and
462.5 ms on the sync-test host, 128 of 133 calls and 58.2 ms on the game's
session, about **0.45-0.50 ms per call**. It runs every tick the yard is live
and returns at once after its first tick, so the cost is the per-call floor
of a story entry (its trigger carries every body, flag and mark of the room),
not the mechanism. The warden's reactive methods ran 5 calls for 1.18 ms on
the game's session (one post, after the edit): about 0.24 ms per call, and
only when something happens.

## Rhai and the portable module, the same methods

The capstone arm in `blink_run` (80 ticks, sync-test host). The warden's
`down`, `counted` and `lowered` methods, in Rhai and as the WASM module
`fixture::bridge_warden`:

| Methods | Calls | Entry total | Backend total | Per call |
| --- | ---: | ---: | ---: | ---: |
| Rhai (`bridge::warden/*`) | 40 | 11.89 ms | 9.30 ms | 0.30 ms |
| WASM (`fixture::bridge_warden/*`) | 40 | 30.26 ms | 26.34 ms | 0.76 ms |

Both heard the same outcomes in the same order. At this size the portable
module costs more per call, mostly in instantiation; promote it for the
contract, not for speed, until a heavier body of work is measured. The
arm's own caller (`run`, a status tick that queries, creates and blasts) was
330 calls and 160 ms: a per-tick entry again dominates.

## Edit loop

The game's own session, the project's script edited while the yard runs
(`posts_to_lower` 2 to 1): published **155 ms** (11 frames) after the save;
the bridge lowered **2.0 s** after the save, including the walk and the
strikes. A sync-test host refuses the reload (the harness owns its timeline).

## What to do with it

The mechanism itself is cheap: event-driven methods at a few tenths of a
millisecond, only when a post falls or an outcome arrives. A per-tick entry
that does nothing after its first tick is the cost to avoid: a room that
needs a once-per-visit setup pays 0.45 ms a tick in a debug build for a
story tick that returns. A once-per-visit trigger for a room would remove
it; until one exists, keep per-tick room entries to rooms that need them.
