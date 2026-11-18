# Script contact consumption cost

The loaded `scripted_contact_consumption` fixture uses one captured Rhai
library for competing support claims and an autonomous head-contact purchase.
It consumes native authored breakables through their owner. The same operation
commits the shared inventory bag and caller wallet, then publishes a correlated
result. Native movement, collision publication, persistent claims, save/load,
and GGRS replay run in the fixture.

## Local observation

On 2026-11-18, the cached development test build reported:

| Entry | Calls | Backend microseconds per call | Host microseconds per call |
| --- | ---: | ---: | ---: |
| Claim contact | 200 | 202.0 | 238.3 |
| Claim result | 33 | 125.3 | 154.7 |
| Purchase contact | 100 | 216.2 | 265.3 |
| Purchase result | 10 | 199.9 | 250.5 |

These are profiler call counts, including GGRS replay. They are not object or
reward counts. The fixtures admit one claim and one purchase. The test process
also ran other fixtures, so these values are observations under concurrent
load, not an isolated backend benchmark.

Backend time covers Rhai execution. Host time covers invocation work. Neither
column measures native movement, contact collection, domain lowering,
inventory history, collision rebuild, rendering, asset startup, or save/load.
The native consume operation checks the selected contact and scans the live
breakable population for its exact owner. Item application uses the existing
bounded catalog and exact transaction core. Measure that complete workload
before using contact rules on a large body population.

## Use

Use this family for occasional authored object rules. Keep algorithmic state
in host records. Make result handlers idempotent: an exact retry can deliver
the stored terminal result again. Reuse the native object and inventory owners;
do not create script-owned collision or reward ledgers. The same portable
contract permits a costly rule to move to Rust without changing its authority.
