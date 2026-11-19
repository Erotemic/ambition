# Profiler memory phase ownership

A collector limit does not bound an exporter. Tracy's CSV exporter loads the
trace into a new worker. The profile runs a totals export and then an instance
export. Stream instance rows and bound each exporter process separately.
Retain the raw trace if export fails. Compiler concurrency and exporter RAM
are separate choices.

The live game and collector need separate scopes too. Calculate their budget
after the warm build. If the game reaches its limit, the collector must stay
alive to save the trace. Lower the collector's recording threshold to leave
space for saving inside its own hard limit. Record each scope's result so a
timed shutdown and a scope OOM can be distinguished.

A scope limit does not reserve RAM against growth in other jobs. Large exports
and full integration-test runs can still exhaust a small VM when they overlap.
Measure those jobs separately.

A whole-command RAM limit also covers compiler processes. It can turn an
ordinary parallel build into compiler OOM failures before capture starts.
Keep that limit explicit. Do not add a default compiler job limit to address
an unproved capture or export failure.

Check memory snapshots before assigning a crash to a phase. If available host
RAM rises by approximately the VM's allocation before game launch, export is
not established as the point where that VM disappeared. A guest log cannot
prove a host OOM kill. Use the host kernel or OOM-manager record for that claim.
