# A resumed asset snapshot can discard an independent publish

**Tags:** `assets`, `staging`, `lost-update`, `publication`

A staged sprite root is a snapshot of all runtime sprite families. A direct
target publish can update the live roots while that snapshot remains after an
interrupted run. If a later focused publish resumes the snapshot and swaps all
roots, it discards the independent output for other targets.

The staging helper keeps the replaced roots in `publish_mirror`. Recovery can
copy those families over the live roots without deletion. This keeps outputs
that exist only in the current roots. Restore manifests, pages, part tables,
portraits, and quality tiers as one set.

A benchmark should interrupt one staged publish, complete a different direct
publish, and then resume the first. Both complete target outputs must remain.
A focused publish must not replace unrelated output with a stale snapshot.
