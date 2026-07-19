# Shell vanity sequence — animated startup card, and the frontend polish it unlocks

Status: **planned, not started** (2026-07-19). Author: Opus 4.8.
Owner surface: `ambition_game_shell` (presentation), `ambition_menu` (pointer
input), `tools/vanity_card_prep` (asset export), `scripts/` (IPFS hydration).

Motivation: Jon has an authored vanity-card animation (robot hands the game to a
human; "I MADE THIS") sitting in `assets/vanity_card/`, currently reachable only
via a pygame preview and a GIF. It should play as the startup card, in engine.

## What already exists (do not rebuild)

The "animated vanity card" segment kind is **already a shell concept**:

- `ShellSegmentPresentation::ImageSequence { frames, frames_per_second, alt_text }`
  — `crates/ambition_game_shell/src/sequence.rs:29`, builder at `sequence.rs:98`.
- It is genuinely rendered — `crates/ambition_game_shell/src/basic_presentation.rs:450`.
- `ShellSegmentRole::Vanity` — `sequence.rs:12`.
- `fade_basic_sequence_card` (`basic_presentation.rs:382`) already eases content
  alpha in/hold/out against an opaque black backdrop, so a card fades up from and
  back down to black for free. Curve unit-tested (`basic_presentation.rs:476`).
- The startup card is host-composed in `compose_ambition_startup_sequence`
  (`game/ambition_app/src/app/shell_host.rs:158`) — today a single `TextCard`,
  "Powered by Ambition", 3.6 s.

**Nothing anywhere constructs an `ImageSequence`.** This is a fill-in-the-variant
job, not a new subsystem. Do not add a second presentation variant for animation.

## Decisions

**D1. Generalize the one variant to per-frame holds; do not add a second.**
`frames_per_second` is a lossy encoding of what the card actually is. The
authored card is beat-based — holds of 0.7 / 0.5 / 1.2 s interleaved with 6 fps
play runs (`tools/vanity_card_prep/config.yaml`, `panel_animation.beats`), and
the exported GIF confirms it: durations `[80, 660, 160×5, 240, 490, 240, 410,
1160]` ms. Uniform fps forces faking holds by duplicating frames.

Per-frame holds are also the *efficiency* answer: every hold becomes one image
with a long duration instead of N identical frames, collapsing the 12 GIF frames
to ~6–8 unique images. Ship only unique frames.

**D2. Play once, hold last.** The current lookup is
`((elapsed * fps) as usize) % frames.len()` — it loops forever. A vanity card
plays through and holds the punchline so the existing fade-out lands on it.

**D3. Derive segment duration from the frame holds.** `auto_advance_after` drives
both the advance *and* the fade, independently of frame timing. Hand-setting it
lets the animation drift against the card's own lifetime. Sum the holds.

**D4. Stable root, swap the image in place.** `shell_frame_key`
(`basic_presentation.rs:404`) folds `frame_index` into the key, and
`render_basic_shell` despawns + respawns the entire UI tree on key change
(`basic_presentation.rs:155-165`) — ~50 full teardowns over a 4 s card. Key on
segment identity; mutate `ImageNode.image` per frame in the system that already
runs over that query.

**D5. Frames are plain local files. IPFS is hydration, not runtime resolution.**
Load via `asset_server.load()` like every other image. Explicitly do NOT route
through `AssetLocation::IpfsGateway`: `bevy_asset_path()` returns `None` for that
variant (`crates/ambition_asset_manager/src/location.rs:109`), so it is
unloadable in practice, and `AssetProfile::IpfsGatewayPlaceholder` is never
selected by any build or CLI flag. Routing the card through it would guarantee
the card never renders.

**D6. Commit the manifest, gitignore the pixels.** The manifest is text and tiny;
it is the contract. The pixels are payload (`assets/.gitignore:6`). A committed
manifest is what lets a bare clone render "missing frame 3 of 8" with *correct
timing* rather than showing nothing.

**D7. Per-frame degradation, not whole-card fallback.** Jon's call: each frame
independently resolves to either its image or a "missing frame N" placeholder.
Sequence length is unaffected, and which frames are absent is visible. Detection
via `LoadState::Failed(_)` — the established pattern
(`game/ambition_app/src/app/startup_loading.rs:435`,
`game/ambition_app/src/app/world_flow/room_transition_assets.rs:328`).

This is the **primary** runtime path, not a defensive corner: see "IPFS reality".

## IPFS reality (why D5/D6/D7 matter)

`assets/vanity_card.ipfs` is tracked — a whole-directory sidecar, CID
`bafybei…x4vsi`, 12.67 MiB / 51 items, pinned as
`pkg:github/Erotemic/ambition#assets/vanity_card`.

But **the sidecars and the Rust `IpfsGateway` machinery are disconnected
systems.** Nothing in the Rust code reads a `.ipfs` file. There is no fetch tool
in `scripts/` or `tools/`; re-hydration is a manual `ipfs get <cid>` into the
sidecar's `rel_path`. The sidecar records one *directory* CID with no path→CID
table, while the Rust manifest wants a CID per entry — the two formats do not
line up, and bridging them is unimplemented.

Consequences taken as given by this plan:

- Frames are absent on a fresh clone by default. Placeholders are the norm.
- That CID snapshots the **working** directory, including `parts/` (5.4 MB of kit
  sheets) and `poses/`. Fetching 13 MB to show a 4 s card is the wrong shape —
  hence a second, lean sidecar covering only runtime frames (VC3).
- The missing hydration command affects six sidecars, not just this one
  (`assets/{backgrounds,icons,concept_art,vanity_card}.ipfs`,
  `crates/ambition_actors/assets/fonts/bundled.ipfs`,
  `tools/LDtk-1.5.3-installer.AppImage.ipfs`). One script closes all of them
  (VC5) — high leverage, and it serves the regen-on-fresh-clone invariant.

## Task cards

### VC1 — per-frame holds in the shell data model
File: `crates/ambition_game_shell/src/sequence.rs`

- Add `pub struct ShellSequenceFrame { pub asset_path: String, pub hold: Duration }`.
- `ImageSequence { frames: Vec<ShellSequenceFrame>, alt_text: String }` — drop
  `frames_per_second` from the variant.
- Keep `ShellSegmentSpec::image_sequence(id, paths, fps, alt)` as the uniform
  constructor (fills equal holds from fps) so the existing call shape and its
  positive-fps assert at `sequence.rs:108` survive. Add
  `image_sequence_timed(id, frames, alt)` for the authored case.
- `total_duration()` = sum of holds; both constructors set
  `policy.auto_advance_after = Some(total_duration())` (D3).
- `frame_at(elapsed) -> usize`: cumulative scan, clamped to last (D2).

`frame_at` and `total_duration` are pure — unit-test them here.

### VC2 — stable root, in-place swap, missing-frame placeholder
File: `crates/ambition_game_shell/src/basic_presentation.rs`

- `shell_frame_key` (`:404`): for `ImageSequence`, key on segment id only, so the
  root spawns once per segment (D4).
- Root keeps **both** an image child and a text child alive for the whole
  segment; per-frame work toggles visibility and content rather than respawning.
- Per-frame system (extend `fade_basic_sequence_card`, `:382` — it already
  queries the right entities): set `ImageNode.image` to the current frame's
  handle each frame.
- On `LoadState::Failed(_)` for that frame's handle: hide the image child, show
  the text child with `missing frame {i+1} of {n}` (D7). Timing untouched.

Note `Handle::default()` is already used as a placeholder handle
(`crates/ambition_asset_manager/src/bevy_integration.rs:101`) if a neutral
texture is wanted rather than bare text.

### VC3 — export unique frames + manifest from the prep tool
Files: `tools/vanity_card_prep/` (new export alongside `export_gif`,
`frame_demo.py:256`), `tools/vanity_card_prep/run.sh`

`export_gif` already performs the beat→frame expansion with bubbles composited
via `render_frame_pil` (`frame_demo.py:210`). Add an export that:

- walks the same beat expansion, **dedups consecutive identical frames** into
  `(image, hold)` pairs (this is the D1 collapse, computed automatically),
- writes numbered PNGs into the gitignored payload dir,
- writes a **committed** RON manifest — `[(path, hold_ms), …]` — outside the
  ignored tree, e.g. `assets/vanity_card.sequence.ron`, sitting next to
  `assets/vanity_card.ipfs` for symmetry (D6). Confirm the path against the app
  asset root (`AssetPlugin { file_path: asset_root }`,
  `game/ambition_app/src/app/cli.rs:680`).
- Add a lean second sidecar for just the exported frames.

Per the RON-parser-drift rule, if a Python writer and a Rust reader both touch
this format, add a Rust parse test over the committed manifest.

### VC4 — compose the real card
File: `game/ambition_app/src/app/shell_host.rs:158`

Host reads the committed manifest → builds the `ImageSequence` segment, replacing
the `TextCard`. Named content stays out of core: the shell crate knows about
sequences, the host knows about *this* card. `--direct` continues to skip the
whole startup route (`shell_host.rs:146-153`).

### VC5 — IPFS hydration script (unblocks everything; do first)
File: `scripts/fetch_ipfs_assets.py` (new)

Read any `.ipfs` sidecar, `ipfs get <cid>` into its `rel_path`, skip if already
populated. Note `icons.ipfs` uses an older `schema_version: 1` key layout while
the others omit it — the format is not consistently versioned, so parse
defensively. Document in `AGENTS.md` / `docs/`. Fixes the reproducibility hole
for all six sidecars.

### VC6 — title menu fade-in (opportunistic; Jon selected)
Logged open at `dev/journals/code_smells.md` §3 as needing an alpha ramp on the
rebuild-on-change `ambition_menu` launcher tree. **VC2 builds exactly that
machinery** (stable root + per-frame alpha), so generalize it: a
`ShellFadeIn { started, duration }` component on a root plus one system walking
descendants ramping `TextColor` / `BackgroundColor` / `ImageNode` / `BorderColor`
alpha — serving both the vanity card and the launcher on one path.

Care: do not ramp the launcher's own opaque backdrop along with its content.

### VC7 — pointer / touch selection (independent; Jon flagged)
File: `crates/ambition_menu/src/render/bevy_ui/mod.rs`

Menu rows already spawn with `Button` (`spawn.rs:156`) and carry
`AmbitionMenuControl { kind, action, focus }` (`spawn.rs:160`), so they are
pickable and self-identifying — but **nothing reads `Interaction`**. The only
pointer observers in the stack are the three scrollbar ones (`mod.rs:537-539`).
So scrollbar drag works with mouse/touch; choosing a row does not. The shell
launcher's own input is keyboard/gamepad-only (`basic_presentation.rs:517`).

Fix: one `Pointer<Click>` observer reading `.focus` to move the cursor and
`.action` to activate, emitting the **same** neutral commands keyboard nav emits.
Explicitly not a parallel mouse-driven selection path.

Leverage: the launcher, the shell pause menu, and Ambition's kaleidoscope menu
all render through `spawn_bevy_ui_menu_with_assets` — one observer fixes pointer
and touch selection for all three.

Priority note: Jon rated this low, and it is low for desktop. But AGENTS.md
commits to preserving the Android/mobile/touch path, and the title screen is the
first thing a touch user hits — keyboard-only launcher means the game cannot be
started at all on mobile. Low urgency, load-bearing for a supported platform.

## Verification

Headless, via the existing `MinimalShellPlugins` harness
(`crates/ambition_game_shell/src/tests.rs:221`, startup-sequence integration at
`:385`):

- `frame_at` / `total_duration` / hold-last are pure — direct unit tests (VC1).
- Missing-frame degradation is testable headlessly: register an `ImageSequence`
  over paths that do not exist, step the app, assert the sequence still advances
  on schedule and completes. **This is the invariant that matters** — absent
  assets must not change timing or block handoff to the launcher.
- Segment-duration derivation: assert `auto_advance_after == sum(holds)` so
  animation and card lifetime cannot drift (D3).

Only the actual pixels and the fade tuning ship blind.

## Deferred / not in scope

- Bridging the directory-CID sidecar format to the per-entry Rust
  `AssetManifest` (`crates/ambition_asset_manager/src/manifest.rs:38`). Real gap,
  larger than this work.
- Menu soundtrack starting on the vanity card rather than the title screen
  (`code_smells.md` §3b) — cross-crate audio plumbing; Jon deselected.
- Sprite-atlas packing of the frames. `ImageNode` supports `texture_atlas` and
  the tool already emits `preview/strip.png`, so it is available. At ~8 frames
  loaded once for a 4 s card it buys nothing on memory, and the one risk it
  mitigates — staggered async loads popping in — is better handled by the 0.55 s
  fade-in covering the load. Revisit only if pop-in is actually observed.
