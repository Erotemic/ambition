# Sprite bundle publication

**State:** complete, 2026-10-28. Parent: [authoring and tools](authoring-and-tools.md).
Implementation: `58ba17f4a`. Integration: `77c367d74`.

Mutable installation and reused staging directories could leave mixed files,
old companions, and false freshness evidence. Registered products now build from
source snapshots into immutable bundles. One atomic selection supplies runtime
loading, embedding, packaging, and inspection. The
[sprite publication guide](../../tools/sprite-publication.md) defines the workflow.

Verification built all 466 runtime targets and four shared atlases from source.
All 470 bundles passed deep freshness checks. The copied package contained all
9,083 selected runtime files with matching hashes. Quality verification checked
1,064 products with no defects or missing evidence. Native owner, inspector,
reload, director-card, and ten assembled-app art checks passed. The app compiled
with static assets enabled. The broader tooling and documentation checks still
have unrelated findings; this receipt does not claim those checks pass.

The renderer's publication-store, artifact, and source-snapshot tests guard
failed requests, changed bases, required pages, member hashes, source changes,
and parent dependencies. Do not restore custom runtime installers, reused
render staging, or raw sprite directory scans as publication evidence.
