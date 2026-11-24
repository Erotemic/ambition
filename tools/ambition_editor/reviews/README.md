# Asset reviews

Review files for characters, images, sound effects, fonts and worlds:
`<kind>/<asset>/<version>[--<part>].review.yaml`, schema `ambition.asset_review.v1`.
They have the same keys as the music tool's reviews (`ambition.music_review.v2`).
Music reviews stay in the music tool, under `reviews/music/`.

Save reviews from the page with `./scripts/asset_inspector.py`, or with
`python3 scripts/asset_inspector.py reviews add ID --score N`. Read the work
queue with `python3 scripts/asset_inspector.py reviews backlog --json`. These
files are authored data: commit them.

See the [tool guide](../README.md#asset-reviews) for the format and the workflow.
