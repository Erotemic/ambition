# Current risks

This file is the extracted risk register from `docs/CURRENT_STATE.md` plus the current review protocol for high-risk Ambition systems.

Related split:

- [`state.md`](state.md)
- [`risks.md`](risks.md)
- [`next.md`](next.md)

## Known high-risk areas

Spatial reasoning and geometry code need extra review. In particular:

- LDtk chunk-to-active-area composition,
- LDtk validator checks for `EdgeExit`/solid overlap and transition arrivals that would start outside the target active area or inside authored solids,
- room transition arrival repair,
- loading-zone placement and labels,
- camera/world coordinate conversion,
- collision edge-touch semantics,
- blink destination search,
- moving hazards/platforms,
- non-Euclidean seams or chart transforms.

When touching these systems, add an `AMBITION_REVIEW:` comment if the logic is easy to get subtly wrong, and add tests or debug visualization when practical. See `docs/AGENT_HANDOFF.md`.

## Spatial review rule

Spatial reasoning and geometry code need extra review. When a branch of logic is easy to get subtly wrong, add an `AMBITION_REVIEW:` comment and back it with tests, trace evidence, or debug visualization when practical.

## Dev-memory searches

```bash
rg -n "movement|collision|sweep|teleport|wall|ledge|blink" dev/journals dev/benchmark-candidates
rg -n "LDtk|LoadingZone|activeArea|transition|arrival|solid" dev/journals dev/benchmark-candidates
rg -n "Android|asset|android_main|overlay|entrypoint" dev/journals dev/benchmark-candidates
```

See also:

- [`../concepts/movement-collision.md`](../concepts/movement-collision.md)
- [`../concepts/ldtk-world-composition.md`](../concepts/ldtk-world-composition.md)
- [`../concepts/patch-overlays-and-repo-state.md`](../concepts/patch-overlays-and-repo-state.md)
