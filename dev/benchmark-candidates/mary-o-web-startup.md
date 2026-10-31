# Mary-O browser baseline

Measured on 2026-10-30. This is a browser prototype, with one provider and
simulation for the native and browser hosts.

## Method

Build with `./build_for_web.sh --game mary-o --copy-assets`. Serve with
`scripts/web_bundle.py serve`. Run `scripts/measure_web.cjs` against that URL.
The [web recipe](../../docs/recipes/web-build.md#standalone-mary-o) has the commands.

Tools: Rust 1.98.1, wasm-bindgen 0.2.127, Chromium 151.0.0.0, WebGL2 through
SwiftShader, and a 1024 by 576 viewport. The build uses `web-release`, thin LTO,
and three Cargo jobs. It does not run wasm-opt. Code and the SFX bank use gzip.

Both visits use one fresh temporary persistent browser profile. An incognito
profile did not retain the large responses and gave a false repeat-visit
baseline. Transfer counts are CDP `encodedDataLength` response counts over
local HTTP. They are not a wide-area network or TLS measurement.

## Results

| Measure | Result |
| --- | ---: |
| Raw JS and WASM | 195.82 MiB |
| Gzip JS and WASM | 23.70 MiB |
| Cold smoke visit | 36.66 MiB |
| Repeat smoke visit | 14.71 KiB |
| WASM linear memory at capture | 489.5 MiB |
| Cold canvas start | 18.23 s |
| Repeat canvas start | 16.72 s |
| Available asset files | 10608 |
| Available asset bytes | 1919.44 MiB |

Available assets are the full selected library plus the game pack. They are
not an initial download. The browser fetches image and audio bytes on demand.
The code still embeds library metadata and registers 1,896 sprite sheets.

An earlier local code prototype was 54.54 MiB with gzip. The current code is
about 57% smaller. Several inputs changed, including asset transport and build
features. This is not a measurement of LTO alone.

The final warm edit build took 7 min 34 s. The first isolated build took
26 min 53 s with a different cache state. A warm native package-input query
took 0.06 s and used 45,600 KiB peak RSS. The query is not the build bottleneck.

A Rust 1.98.1 probe embedded 4 MiB behind a public static and then behind a
function returning a private static. Both emitted about 4.2 MB of Rust metadata.
A getter alone did not reduce the metadata size. The local probe is under
`target/mary-o-metadata-probe/`.

Canvas start is a host milestone. It does not prove that a scene is ready.
The captures show the game, a keyboard jump, and a touch jump at 844 by 390.
The browser reported no JavaScript errors. A Web Audio probe with gesture
restrictions measured a nonzero SFX output peak of 0.501. It did not test a
person's audio device. Hardware frame rate and physical devices were not tested.

## Follow-up probes (2026-10-31)

The static build lists 4,284 metadata files, with 132,257,496 source bytes.
Python zlib level 9 compresses each file to 11,357,456 bytes in total. Every
file decompresses to its exact source bytes. Gzip level 9 over the source
texts together is 10,427,844 bytes; over the compressed files together it is
11,255,154 bytes. Per-file compression saves raw storage but adds 827,310 bytes
to this transfer proxy. This is not a new WASM build or browser measurement.
Do not claim a bandwidth gain from this change. The probe result is at
`target/mary-o-metadata-compression-probe/results.json`.

A native metadata-only test took 7.24 s wall time, 4.81 s user CPU time, and
241,052 KiB peak RSS under `/usr/bin/time -v`. It ran beside a Cargo build.
The test was
`enemy_quad_matches_its_box::the_ai_slops_box_has_the_shape_its_sheet_publishes`
in the existing `mary_o_it` binary. It reads the shared sprite fixture and
checks geometry; it creates no game app. This is one sample, not a controlled
before-and-after comparison. Kernel sampling was unavailable because the host
sets `perf_event_paranoid` to 4.

## Content limits

The selected publication has no Mary-O music files or soft-glow image. The
compiler reports eight absent music tracks. Browser visits report missing
`support_theme/full.ogg` and `sprites/entities/soft_glow.png`. SFX load from the
shared bank. Keep these missing media separate from transport failures.

The tool suite passed 296 tests. Its remaining Hall test reports two exhibits
without an order entry: `bad_apple` and `gravity_apple`. The focused authoring
checks passed 19 tests. No map content changed during the authoring move.

## Verification

The assembled native app check passed. The content compiler, sprite metadata,
and facade library tests passed 208 tests, with three ignored. Mary-O's content
library passed 204 tests, with one ignored.

The full host run passed 124 tests, failed two obsolete checks, and ignored
three tests. One check compared the logical texture canvas with the body and
ignored frame trim. The other required a retired loose-file path. The remaining
body checks and the terrain-skin behavior passed in a targeted three-test run
after those checks were removed.

The workspace policy suite passed 34 tests. Its engine scope reports eight
violations in unchanged files: optional character catalogs, a module-size
limit, and pose/velocity writes in camera scenarios. Repository documentation
checks also report existing broken links and planning references. These broad
checks are not green.

## Next size pass

Use 20 MiB for the initial playable load as a proposed working target,
including music. It is not an accepted limit or a measured result.

First measure the game's full metadata dependency closure through the schema
owners. Keep late-bound actors, forms, poses, and attachments in that closure.
Then measure a bank containing the game's declared cues. These changes can
reduce transfer and memory before further texture or font work.

## Inputs and local artifacts

The build started from superproject `c8f39ddfc` with this web patch. The map
submodule's README update is `f83a7d8`; Mary-O's map bytes did not change.

- Map SHA-256: `274eedd1a5054363a03c9b9eca2ed361f92ec31d15e31387f176b6243ea46e30`.
- Publication selection SHA-256: `6ec97fc9d600f06c7f233ef4cd93313f1ac12bd315c4b7ca97f12682e1f6b21b`.
- WASM SHA-256: `a03c8e86437f309e7ccb67df8dc34620872e7fdf973148ddff60ee671c2ea4e8`.
- Package contract and hashes: `game/ambition_demo_mary_o_app/web/.assets_contract.json`
  and `.assets_sha256`.
- Bundle inventory: `game/ambition_demo_mary_o_app/web/bundle-report.json`.
- Browser traffic, console, and images: `target/mary-o-web-final/`.
- Touch images: `target/mary-o-web-final-touch/`.

The package and captures are generated files. The source and this measurement
record are tracked.
