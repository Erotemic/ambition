# Shared spatial grid queries

The desktop autoplay capture `desktop-autoplay-pair-20261118T204859Z` reports
`Grid::near` at 2.13% of sampled cycles. This shared index serves movement and
navigation queries.

A shared grid is immutable after construction. Build the cell lists with a
tree, then store them in a sorted table for lookup. Keep a list of all indexed
entries in source order for queries that use the global fallback. Such a query
must not gather repeated entries from every cell and sort them again.

The result must retain visible list order. Check local and global regions,
point contacts, hidden base entries, added entries, and swept bounds against a
linear scan. Unbounded geometry must remain a candidate.

Measure through `Layered::near`, including exact bounds filtering. Separate
small and large worlds, local and global regions, and layers with hidden entries.
A faster isolated lookup does not establish a faster frame or navigation build.
