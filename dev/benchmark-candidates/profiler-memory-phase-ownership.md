# Profiler memory phase ownership

A collector limit does not bound an exporter. Tracy's CSV exporter loads the
trace into a new worker. The profile runs a totals export and then an instance
export. Stream instance rows and bound each exporter process separately.
Retain the raw trace if export fails. Compiler concurrency and exporter RAM
are separate choices.

A whole-command RAM limit also covers compiler processes. It can turn an
ordinary parallel build into compiler OOM failures before capture starts.
Keep that limit explicit. Do not add a default compiler job limit to address
an unproved capture or export failure.

Check memory snapshots before assigning a crash to a phase. If available host
RAM rises by approximately the VM's allocation before game launch, export is
not established as the point where that VM disappeared. A guest log cannot
prove a host OOM kill. Use the host kernel or OOM-manager record for that claim.
