# Runtime efficiency architecture — consolidated

**State:** current authority is
[`performance-and-iteration.md`](performance-and-iteration.md).

This file formerly synthesized a broad CPU/runtime optimization program. The
measurement campaign has now answered enough of those hypotheses that two active
ranking documents would create conflicting guidance.

Standing conclusions that should not be rediscovered:

- measured representative simulation CPU is currently healthy and broad rather
  than dominated by one hot gameplay system;
- system count is an ownership/composition signal, not a frame-time proxy;
- generic change-driven projection measured too little leverage to justify a
  campaign;
- parallel scheduling of the current tiny-system workload produced very high
  context-switch/parking overhead;
- capability removal did not materially improve frame time or measured plugin
  registration startup;
- run-condition work may improve semantic activation boundaries but has not
  earned a major CPU claim;
- current user-visible performance pressure is better funded in weak-GPU raster
  cost and asset/device materialization/residency.

Use the performance plan for current measurements and next work. The historical
ranking/evidence remains in git history.
