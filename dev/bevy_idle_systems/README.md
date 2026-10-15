# The cost of an idle system

`cargo run --release` here. It prints the time per frame of a schedule of 100
or 1000 systems that find no entity to work on, with 2000 archetypes that
their queries do not match, on each executor:

- `idle-writer`: each system writes the same component, so no two can run at
  the same time;
- `gated`: the same systems behind a run condition that is false;
- `idle-reader`: each system only reads, so all can run at the same time.

Multiply by the systems that run each frame (`[census] systems_ran`, see
`docs/recipes/profiling.md`) to get the floor that a game pays to have them.

Result on the development VM (12 cores), 2026-10-14:

| executor | kind | 100 systems | 1000 systems |
| --- | --- | --- | --- |
| single-threaded | any | 0.014 us each | 0.016 us each |
| multi-threaded | idle-writer | 1.7 us each | 12 us each (12 ms a frame) |
| multi-threaded | gated | 0.023 us each | 0.038 us each |
| multi-threaded | idle-reader | 0.48 us each | 0.56 us each |

On the multi-threaded executor, an idle system that conflicts with its
neighbours costs more for each system added to the schedule.
