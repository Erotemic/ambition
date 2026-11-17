# Measure before promotion

Build the example WASM module as described in [README.md](README.md). Then run:

```bash
python3 examples/projects/pulse_forge/profile.py --samples 3 --cycles 10
python3 examples/projects/pulse_forge/measure_edits.py --samples 3
```

The first command builds one release test host with `desktop_player`, default
features disabled, and `CARGO_INCREMENTAL=0`. It then runs Rhai, WASM, and linked
Rust in separate processes. Backend order rotates between samples. Each backend
also runs without invocation counters. The Rhai patrol stays active in all runs.
No other build or benchmark should run during these samples.

Each cycle uses the parity fixture's 280 input frames at 60 Hz. A 70-frame hold
charges five pulses. The first release spends 60 of 72 mana; the second release
cannot pay. The fixture resets mana between cycles and checks the refusal's
final balance. A pulse batches four projectile requests in one invocation.
Charge is the small frequent callback. Pulse includes both waiting callbacks
and the four-request batches. The benchmark does not serialize the parity trace.
The separate parity fixture checks detailed behavior and populated rollback.

`ExtensionProfile` is an optional host diagnostic resource. Its counters cannot
affect gameplay and are not rollback state. It records calls, faults, entry
elapsed time, and backend time. Native backend time surrounds the entry function.
Loaded backend time surrounds the backend call, including ABI conversion and
fresh WASM instance setup. It is not pure guest algorithm time. Host time is
entry elapsed time minus backend time; it includes observation preparation,
host serialization, state validation, and commit. Domain lowering is outside
that entry interval and inside the complete simulation step.

The complete step measurement includes the fixed simulation tick and the
headless harness's App update. Every measured update must advance exactly one
tick. It excludes startup. Compare the runs without counters for whole-step
cost; the counters add clock reads and map updates. Process RSS and peak RSS
come from Linux `/proc/self/status`. They include content, the engine, VM code,
and allocator retention. They do not isolate guest allocation counts or prove
a leak. Other platforms need a memory provider for this measurement.

Raw samples, the executable hash, source and artifact hashes, toolchain, CPU,
Cargo command, and build log go to `target/pulse-forge-profile`. The edit probe
writes `target/pulse-forge-edits`. It changes only the example's speed constant
from 240 to 241 and restores it. Run it while those example files are idle.
It records no-op build controls, module build time, native release-host build
time, and a separate behavior-probe build. Edit-to-observation time ends when a
restarted simulation contains a projectile with the changed speed. Native total
includes that extra probe build. This is a conservative headless restart loop;
it does not measure live reload latency or visual readiness.

The invocation benchmark's first host build is setup cost, not a module-edit or
native-relink measurement. The edit probe records those build classes separately.
The WASM builder invokes only the selected portable crate and its public value
dependencies. The native command deliberately builds and links an executable.

## Executed results

Linux x86_64, 2026-11-17; Intel Core i9-11900K VM with 12 logical CPUs and
62.8 GiB RAM, Linux 6.8.0, Rust 1.95.0, clang/mold. Source is on virtiofs;
the assigned Cargo target is bound to ext4. These are three samples per backend
and counter mode, 2,800 ticks per process, 50,400 measured ticks in total.
They are a small game workload, not a production scaling claim.

| Backend | Complete step, median of sample means | Sample mean range | RSS after ticks | Process peak RSS |
| --- | ---: | ---: | ---: | ---: |
| Rhai | 648.9 us | 613.2–660.6 us | 680–684 MiB | 885–889 MiB |
| WASM | 740.8 us | 728.5–743.4 us | 686–689 MiB | 890–893 MiB |
| Linked Rust | 629.5 us | 604.1–688.5 us | 683–684 MiB | 893–896 MiB |

Those rows use counters disabled. Startup to tick 8 takes 3.09–3.15 s and is
outside the step interval. Peak memory includes startup. There is no measured
allocation count or isolated guest-memory saving.

The counted runs have zero faults and the same invocation counts in every
sample. The table gives median time per call. Host and backend columns have
separate medians, so rounding can prevent their sum from matching total time.

| Emitter entry | Calls per sample | Backend | Host time/call | Backend time/call | Total time/call |
| --- | ---: | --- | ---: | ---: | ---: |
| Charge | 1,420 | Rhai | 4.52 us | 25.59 us | 30.11 us |
| Charge | 1,420 | WASM | 5.36 us | 97.86 us | 103.24 us |
| Charge | 1,420 | Linked Rust | 2.51 us | 0.58 us | 3.07 us |
| Pulse, including waiting calls | 660 | Rhai | 4.40 us | 25.73 us | 30.13 us |
| Pulse, including waiting calls | 660 | WASM | 5.32 us | 102.44 us | 107.76 us |
| Pulse, including waiting calls | 660 | Linked Rust | 2.37 us | 0.84 us | 3.21 us |

The unchanged Rhai patrol runs 2,800 calls per sample. Its backend interval is
45–51 us/call across backend sample medians. Counted complete-step sample means
are 614–650 us for Rhai, 736–756 us for WASM, and 601–624 us for linked Rust.
Variation between processes is larger than the clock overhead this experiment
can isolate. Do not subtract these separate process samples to claim a
negative counter cost.

WASM is slower for this mechanic. Every waiting callback still pays ABI and
fresh-instance costs. Keep the four projectiles in one request batch. Do not
promote this small emitter to WASM as a performance optimization. Rhai is the
appropriate edit path here. Linked Rust sharply reduces its callback cost,
but the complete native and Rhai step ranges overlap. These samples do not
establish a whole-game speedup. Keep linked Rust as a release choice; measure
other workloads before making a broader performance claim.

The benchmark host SHA-256 is
`45322ed80e7f1491b5d879e35a7912e0ab98b753737f08321ad99ca2106cf8f6`.
The WASM SHA-256 is
`7cdfa44fedf283fbf8304c510d8baa8d8fc212ae8d845c3f8494cecf5e844856`.
The final raw report is `target/pulse-forge-profile-final/report.json`.
Its setup rebuild took 613 s. That rebuild added the diagnostic API through
the runtime graph; it is not a warm edit/relink measurement.

Three speed edits per backend all reached a projectile at exactly 241 world
units/s. Sources and the baseline WASM output were restored after each sample.

| Backend | Module or release-host build | Extra behavior-probe build | Edit to observed changed projectile |
| --- | ---: | ---: | ---: |
| Rhai | No Cargo invocation | None | 3.131–3.173 s |
| WASM | 0.421–0.470 s, module only | None | 3.602–3.676 s |
| Linked Rust | 3.734–3.818 s, release host | 3.056–3.127 s | 10.004–10.061 s |

Warm unchanged controls took 0.238 s for the module, 1.075 s for the release
host, and 0.901 s for the probe. Build intervals include Cargo's normal
startup and fingerprint work. Native edit-to-observation includes the extra
probe build; it must not be presented as the shipping host's relink time.
These restart results do not estimate the source hot-reload loop. The report
is `target/pulse-forge-edits/report.json`.

The [retained raw observations](../../../dev/journals/2026-11-17-procedural-backends.json)
contain every sample, hashes, commands, and machine metadata. The earlier
provisional run used an older example WASM output and is not this acceptance
result.
