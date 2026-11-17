# Cost of a scripted stage attack

Date: 2026-11-17. Source: `31443e23b` and `ec483e1ac`.

## Scenario

The real Ambition Hall of Bosses practice stage runs the Boss of the Game
encounter. The fixture advances its health to the encore, then runs the
normal phase transitions, agent entrances, warning, jet strike, collision,
and damage. It places the recipient in the east outer jet during the warning
and rebases the rollback history. The final witness compares the canonical
cadence and health on the first run and through four-tick rewinds.

Host: desktop Linux VM, x86_64, Intel i9-11900K reported by the guest. Cargo
uses the development/test profile. Workspace code has optimization level 0;
external dependencies have level 3. Tests run one at a time. These figures
are development-loop measurements, not release or device frame budgets.

Command:

```bash
cargo test -p ambition_app --test app_it -- the_boss_stage_has_scripted_pyro --test-threads=1 --nocapture
```

`ExtensionProfile` measures the real entry, including state reads,
conversion, VM execution, validation, and commit. Backend time includes the
loaded call and conversion. Counters are diagnostic state; replay adds calls
again. Native projectile and combat work is outside the entry total.

| Run | Ticks | Entry calls | Faults | Entry total | Backend total | Session step wall time |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Sync-test host, no rewind | 680 | 155 | 0 | 34.668 ms | 29.129 ms | 8.081 s |
| Same host, four-tick rewinds | 680 | 761 | 0 | 191.011 ms | 157.559 ms | 59.946 s |

The first run averages 0.224 ms per active entry call, or 0.051 ms per
simulated tick across this window. Idle beats reset the record without a VM
call. Each call has at most eight requests; each strike has at most ten
two-jet pulses. This is a bounded, infrequent sequence on one boss. Keep it
in Rhai at this workload. A dense population or a device release needs its
own measurement before that choice can be extended.

The secondary Lightning skill also ran in a real sandbox session. In its
180-tick held-fire window, 180 calls cost 39.175 ms total and 31.869 ms in
the backend. Four-tick rewinds made 900 calls, at 206.779 ms total and
166.441 ms in the backend. Unlike the boss beat, its equipped cadence entry
also runs on idle input so a release cannot reset the firing delay. This
cost applies per equipped body; do not extend it to a dense population
without a new measurement.

## Edit loop

The same assembled-session executable edits a watched Rhai file while the
game runs. Invalid source keeps the last generation. A repaired source
changes `JET_SPEED` from 720 to 500; the next live projectiles carry 500.
Measured source-edit to live jets: **4.646 seconds**. This includes watcher
polling and the remaining phase/warning playback. It is not a compile-only
latency. No Cargo process runs inside the witness. The
[recipe](../../docs/recipes/scripted-stage-attacks.md) gives both the normal
desktop command and direct use of the built witness.

Lightning's source-edit to live-bolt result was 0.549 seconds in the same
development profile.

## Measurement lesson

Compare canonical state across rewinds. A message buffer can contain
requests from the replay window, so its raw count is not the count of shots
on the final simulated tick. Use the contact path to establish damage, and
the body record and health state to establish replay equality. Use the same
sync-test host with check distance zero for the first-run control.
