# LV2 diagnostics are not audio inputs

On 2026-10-29, plugin repair moved foreign processor binaries outside the active
plugin index. Music publication then rejected completed renders. Eighteen cached
cues had LV2 metadata in their dependency records.

For the first rejected cue, the dependency comparison found only three changed
`plugin_metadata_fingerprint` values. The selected plugin files, host executable,
score and render code were unchanged. The selected effects used Guitarix URIs.

`_lv2_dependency_identity` in the music renderer hashes both standard output and
standard error from `lv2info`. That command can report faults in unrelated plugin
bundles. Repair of an unrelated bundle can thus change the selected effect's
render identity and prevent publication.

Keep diagnostics separate from audio dependency identity. Base that identity on
the selected plugin, its metadata and its files. When this boundary changes,
check that repair of an unrelated plugin does not invalidate an unchanged effect.
Do not treat a completed cache import as proof of current source freshness.
