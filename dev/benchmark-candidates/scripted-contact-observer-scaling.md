# Script contact observer scaling

Measured on 2026-11-18 with the loaded
[`crowded_contact_observer_cost`](../../game/ambition_app/tests/scripted_body_contacts.rs)
probe. The baseline source was `9b7b1a928`, with the same probe added. Both runs
used the managed mold 2.42.1 linker and Rust 1.95.0 on an Intel i9-11900K host.
Workspace code used the default unoptimized test profile; dependencies used the
repository's optimized development profile. This is a local debug sample.

Run the current probe with:

```bash
cargo test -p ambition_app --test app_it crowded_contact_observer_cost -- --ignored --nocapture
```

Each live room has 256 stationary physical bodies in a 16 by 16 grid. Adjacent
columns touch. Rooms reuse authored body IDs and coordinates. Four or all 256
bodies per room bind a small loaded Rhai observer. The observer rejects incomplete
geometry and updates its own record. The probe checks script faults and record
creation. It uses the actual headless host, with rollback disabled for timing.
The separate loaded contact fixtures check GGRS replay.

There are four warm steps and 32 measured steps per case. Timers bracket the
native sampler in its real collect stage, with small scheduling and timer costs
included. Complete step time includes the rest of the headless host and Rhai
execution. It excludes device rendering and active AI for the probe bodies.

## Median cost in milliseconds

| Rooms | Bodies | Observers | Collection before | Collection after | Step before | Step after |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 256 | 4 | 0.755 | 0.903 | 9.430 | 10.040 |
| 1 | 256 | 256 | 10.773 | 3.335 | 50.467 | 44.904 |
| 2 | 512 | 8 | 1.411 | 1.587 | 11.092 | 12.225 |
| 2 | 512 | 512 | 22.115 | 7.000 | 94.782 | 84.594 |

## P95 cost in milliseconds

| Rooms | Bodies | Observers | Collection before | Collection after | Step before | Step after |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 256 | 4 | 0.838 | 1.013 | 11.916 | 13.549 |
| 1 | 256 | 256 | 11.436 | 4.418 | 52.666 | 57.134 |
| 2 | 512 | 8 | 1.715 | 1.722 | 18.524 | 13.920 |
| 2 | 512 | 512 | 24.799 | 8.801 | 103.884 | 100.831 |

## Interpretation and remaining limits

The prior sampler compared each observer with all other room bodies. The native
collector now prepares each material piece once and uses the engine's layered
geometry grid for candidates. Session and room partitions remain separate.
Exact native surface checks retain portal pieces and closed resting faces.
Candidates return to identity order. Refused geometry remains an incomplete
read even without a usable envelope.

Collection with many observers is about three times faster in this sample.
Building the index adds less than 0.2 ms to the median collection cost with few
observers. Full-step improvement is smaller, and P95 remains noisy. Per-tick
Rhai observation for every body is still too costly for a 60 Hz step here.
This change does not establish a release or mobile frame budget. Use compiled
modules or fewer per-tick observers when that workload requires them. Event
subscriptions that report state changes remain separate capability work.

Geometry preparation and index construction still visit the room bodies.
Dense overlaps and very large pieces can retain many candidates. Unknown
geometry can require a refusal for every observer. Output limits bound facts;
they do not guarantee a fixed collection budget. The room-story poller is not
covered by this sample and still needs a separate measurement if it is used
with many observers.
