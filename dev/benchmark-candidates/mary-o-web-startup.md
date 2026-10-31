# Mary-O browser baseline

First measured on 2026-10-30. The completion check below is from 2026-10-31.
This is a browser prototype, with one provider and
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

## Initial results

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

A separate native phase probe reads the selected library through the public
sprite APIs. Its input has 1,868 sheet manifest files. The second run measured:

| Phase | Time |
| --- | ---: |
| Read library | 134 ms |
| Build snapshot | 281 ms |
| Read texts | 1,592 ms |
| Prepare metadata, including an input clone | 5,241 ms |
| Compute mechanics digest | 89 ms |

Preparing each metadata family alone took 3,240 ms for part flipbooks,
1,638 ms for sheet records, 61 ms for rigs, 61 ms for portraits and 12 ms for
pack catalogs. These are diagnostic subsets, not proposed runtime omissions.
The next preparation profile should inspect the part and sheet owners.
The source, compiled-library hash and outputs are under
`target/mary-o-metadata-phase-probe/`. The selected publication generation is
`190ae640577132f5f8da4475308bbad7966927f98f8c597b8e4f62e8ae3bff72`.
The probe ran beside Cargo. These are warm local measurements, not browser
results or a controlled optimization comparison.

## Part metadata encoding probe

The same selection has 924 part-table manifests: 231 targets at four quality
levels. All 924 native cache entries match their source path, size, time and
schema key. Compare their RON and bincode bytes in logical path order:

| Encoding | Raw bytes | Concatenated gzip bytes |
| --- | ---: | ---: |
| RON | 106,721,290 | 7,137,528 |
| Bincode | 35,647,335 | 6,076,612 |

Python gzip used level 9 and time 0. The binary form saves 71,073,955 raw bytes
and 1,060,916 gzip bytes in this subset. This is not a WASM build or a browser
load. Measure the full package, decode time and memory before selecting it.
The current web embed path includes RON metadata; the native cache does not
reduce those embedded bytes. The result, selection ID and schema hash are in
`target/mary-o-metadata-phase-probe/part-encoding-sizes.json`.

## Initial content limits

The first selected publication had no Mary-O music files or soft-glow image.
The compiler reported eight absent music tracks. Browser visits reported missing
`support_theme/full.ogg` and `sprites/entities/soft_glow.png`. SFX loaded from the
shared bank. The completion check uses eight accepted music cues and a
procedural glow mask.

The tool suite passed 296 tests. Its remaining Hall test reports two exhibits
without an order entry: `bad_apple` and `gravity_apple`. The focused authoring
checks passed 19 tests. No map content changed during the authoring move.

## Initial verification

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

Use 32 MiB as the working budget for the captured cold playable load, including
music. The completion check measured 30.12 MiB. This budget covers the captured
scene. Measure later room and form loads separately. A 20 MiB cold load remains
a later size target.

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

## Completion check (2026-10-31)

Source: `a01e4553b`. Build with
`./build_for_web.sh --game mary-o --copy-assets --brotli`. Brotli uses quality 9.
The browser, viewport and persistent-profile method are the same as the initial
measurement. The module uses the current library file index and eight accepted
Mary-O music cues. Each cue passed its renderer dependency check. A scoped run
keeps the full library registry unpublished; Mary-O supplies its own registry.

| Measure | Result |
| --- | ---: |
| Raw JS and WASM | 195.86 MiB |
| Gzip JS and WASM | 23.71 MiB |
| Brotli JS and WASM | 16.68 MiB |
| Cold smoke visit, with music | 30.12 MiB |
| Repeat smoke visit | 14.30 KiB |
| WASM linear memory at capture | 539.19 MiB |
| Cold canvas start | 17.79 s |
| Repeat canvas start | 16.89 s |
| Available asset files | 10622 |
| Available asset bytes | 1933.12 MiB |

The code transfer is about 69% below the first 54.54 MiB gzip prototype. This
comparison includes build features, asset transport and compression changes.
Brotli code is about 30% below gzip for this final package. The browser received
Brotli code and the SFX bank. Image and audio files load on demand.

The cold visit has two HTTP 404 responses; the repeat visit has one. These are
library ambiguity probes for short music paths. Each probe is followed by a
successful request under `__libraries/ambition_shared/`. No requested music
file remains unresolved. The browser reported no JavaScript errors. The glow
has no external image request.

Separate captures show a keyboard jump and a touch jump at 844 by 390. All eight
music files decode in Chromium. A gesture-restricted Web Audio probe measured
an output peak of 0.477 before keyboard input and 0.417 during the jump sample.
These are mixed output samples. They do not isolate the SFX channel or test a
physical audio device.

The web-release build and the 10,622-file package contract passed. The cue
freshness check reports eight accepted bundles and no missing or stale cue in
the requested set. The content preflight and 21 focused Python checks passed.
This lane covers cue publication, stale render refusal, package ownership,
registry generation, content status and HTTP compression. It omits the full
repository suite, other browser engines and physical devices. The full shared
music library still needs its other cues; the scoped result covers Mary-O.

Current local artifacts:

- `target/mary-o-web-completion/`: network, console, audio and input receipts.
- `target/mary-o-web-completion-keyboard/`: keyboard captures.
- `target/mary-o-web-completion-touch/`: touch captures.
- The completion directory's `tools/` has the separate browser probes.
- `game/ambition_demo_mary_o_app/web/`: portable package and byte contract.

Publication generation: `d6a3d911f99504617c170d0120fc2e39c1e449c477cfa715e7d3dbb31ca8a373`.

Selection SHA-256: `72dc6a5b7eb5909e277cb3afe261ae062c9528f7d47e7c5077f3430fc5fd20f3`.

WASM SHA-256: `af54f65b7b4e3281e91c48a60f70f2e2e17acadca22bfee0a17633f1ea4d97a1`.
