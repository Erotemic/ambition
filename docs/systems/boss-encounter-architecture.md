# Boss encounter architecture direction

The current sandbox bosses share the same coarse encounter skeleton: intro,
phase thresholds, transition, stagger, enrage, death, music request, and reward
sync. That is useful scaffolding, but it should not become the place where every
boss-specific trick is hard-coded.

Future boss work should keep three layers distinct:

1. **Encounter progression**: phase timing, save-state transitions, music and
   cutscene requests, and victory events. This remains the generic state-machine
   layer.
2. **Boss behavior**: movement, attacks, arena interactions, tells, and special
   vulnerabilities. This should become per-boss data/code rather than more
   branches inside the generic encounter update loop.
3. **Rewards and aftermath**: defeat drops, quest advancement, arena cleanup,
   and reload synchronization. Mockingbird's pirate-hoard chest is the first
   example, but future bosses should use a reward table/profile instead of
   adding one-off `sync_<boss>_...` systems.

The sandbox-side `boss_encounter` module is split around these seams so richer
bosses can add behavior profiles without turning the facade into a long mixed
system. If new bosses start needing custom gravity, moving arena hazards,
scripted props, or multi-stage weak points, prefer introducing a per-boss runtime
profile or behavior plugin over extending the generic encounter loop with named
special cases.
