# Scripted room prop factories

Native runtime props use the same placement constructor as authored
breakables. Keep collision, health, stand collapse, respawn, and visual state
in their native owners. A factory library supplies data, not a second solver.

## Witness

```bash
cargo test -p ambition_app --test app_it -- scripted_breakable_creation --nocapture
```

The fixture creates room and body props, observes actual contacts, and applies
native rewards. It checks both controllers, competing action identities,
refusal without partial creation, authored limits, room departure, and GGRS.
A source-library edit changes future widths without changing existing props.

The 2026-11-18 local development run reported:

| Entry | Calls | Backend average, microseconds | Host average, microseconds |
| --- | ---: | ---: | ---: |
| Room result | 12 | 111 | 138 |
| Room factory tick | 144 | 268 | 311 |
| Body result | 16 | 80 | 99 |
| Body contact behavior | 328 | 174 | 208 |

`ExtensionProfile` measures the backend and complete host invocation. Native
reducers, physics, and rendering run outside these intervals. The fixture runs
in parallel with other tests and includes GGRS resimulation. These values are
local evidence, not a release budget.

## Lifetime distinctions

A room replacement can retain its root entity and change `LiveRoomInstance`.
Check the room identity and the retired objects, not root entity disappearance.
A prepared construction context must reject a source whose room changed.

Timed runtime props have room lifetime. Do not add their timers to the saved
world-return schedule without a native reconstruction recipe. Authored
placements have that recipe and keep their cross-room return records.

A live art request must reach the sheet loader. A registered kind can also be
scenery in a distant room. The scenery reach filter must not suppress its
current runtime use. The live feature feed supplies this demand; do not mark
the kind global or pre-place hidden scenery to force it to load.
