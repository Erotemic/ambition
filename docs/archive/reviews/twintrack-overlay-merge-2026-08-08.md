# The TwinTrack Relativity Festival overlay — merge verdict

`untracked/ambition-twintrack-relativity-festival-overlay-2026-08-06-1538.zip`,
30 files, 770 KB, stamped 2026-08-06 15:38. Jon, 2026-08-08: *"integrating the
new SR code in [the zip]"*.

⛔ **It is a MERGE, not an unpack, and the file-by-file verdict came first.**

---

## Is it new work, or an old snapshot?

Jon asked directly, because an accidentally-made old zip would look similar from
a distance. The answer is **new work on a stale base**, and the evidence is:

- **All 30 paths already exist on `main` and all 30 differ.** An old snapshot
  would have matched some past state; this matches none.
- **24 of the 30 were last touched on `main` BEFORE the zip was cut**, so for
  those the zip is `main`'s content plus edits — a strict successor.
- **The two contested engine files 3-way merged with ZERO conflicts** against
  their real bases (`d50257858`, `c399ce4ca`). A wrong base, or a snapshot of an
  older `main`, would have fought or been a no-op; instead every hunk is an
  addition.
- **It contains design `main` has never had**: `AbilitySet::fly_toggle`
  (separating *can fly* from *may toggle flight*, so a spacecraft authors
  permanent free flight instead of carrying a button it must not press),
  `MovementTuning::flight_invariant_speed`, and a relativistic proper-velocity
  arm **inside the existing shared flight limb** rather than beside it.

⚠ **The anomaly, and it is the reason this took a merge.** The demo files are
written against the relativity API as it was *before* `a301a79a0` (*"A
worldline's identity stops being its caption"*), which landed 09:39 on 08-06 —
about six hours BEFORE the zip's 15:38 stamp. So that branch was cut earlier
that day and never rebased. Ordinary parallel work; it just means a wholesale
copy silently reverts `a301a79a0`.

---

## The file-by-file verdict

### Taken wholesale — 24 files

`main` had not touched them since before the zip, so the overlay's version is
`main`'s plus the SR author's edits.

Engine: `ambition_characters/src/action_scheme.rs`,
`ambition_dev_tools/src/dev_tools/editable.rs`,
`ambition_platformer2d_core/src/{body_clusters,motion_codec,snapshot_impls}.rs`,
`ambition_platformer2d_core/src/movement/{abilities,authority,integration,tuning}.rs`
and `movement/tests/glide_and_air.rs`, `ambition_relativity2d/src/signals.rs`.
Docs: ADR 0011, `demos/README.md`, `demos/twintrack.md`, `engine/relativity.md`,
`engine/slower-light.md`. Demo: the two `Cargo.toml`s, `MODULES.md`,
`chase_beacon.rs`, `twintrack_app/src/lib.rs`.

### Merged, clean — 3 files

| file | base | outcome |
|---|---|---|
| `ambition_platformer2d_core/src/abilities.rs` | `d50257858` | keeps `can_hold_station` (08-06's dialogue-continuity work) **and** gains `fly_toggle` |
| `ambition_characters/src/actor/character_catalog/entry.rs` | `c399ce4ca` | clean |
| `docs/planning/tracks.md` | pre-`f346a938e` | clean; gains the SR-4/SR-5 rows |

### Merged, resolved by hand — 4 files

- **`rollback/registry.rs`** — both sides bumped past v13 independently. `main`
  reached v17 (narrative ledger, combat participation); the overlay carried its
  own v14 and v15. Resolved as **v18**, one entry covering both overlay encoding
  changes, because they land in one commit rather than the two the overlay
  carried. `main`'s v15–v17 history is preserved verbatim.
- **`twintrack/src/lib.rs`, `observatory.rs`, `twintrack_app/tests/twintrack_it.rs`**
  — base `a301a79a0~1`. The conflicts were the overlay REWRITING sections that
  `a301a79a0` had merely renamed a constructor inside. Resolved to the overlay's
  **content** with `main`'s **API** as the target: the overlay owns what the demo
  does, `main` owns the types it calls.

### Regenerated, not merged — 2 files

The overlay's copies are v15-era and would have reverted today's work.

- `game/ambition_app/tests/rollback_schema_baseline.txt` — regenerated from the
  live registry. `twintrack.chase_beacon` LEAVES the wire format (the single
  chase beacon became the character roster); `twintrack.character` joins it.
- `docs/planning/engine/slice-evidence/rollback-schema-baseline.json` —
  **unchanged**, and that is the correct answer rather than an omission. ⛔ this
  ratchet reads the RUNTIME's registrations only; every `twintrack.*` name is
  registered by the demo crate, which its scanner never reads. Adding one here
  reports STALE immediately. (Found by doing it wrong first.)

---

## Bugs in the overlay itself, fixed on the way in

Neither is merge damage — both would have failed in the overlay's own tree.

1. **`observatory.rs` never imported `DJ_POS`** while using it eight times. Its
   own `use crate::{..}` block omits it.
2. **`update_doppler_music_visuals` panicked Bevy's parameter validation.**
   Three queries take `&mut Visibility`, and a `With<..>` marker does not prove
   exclusivity — nothing stops an entity carrying two markers. Fixed with
   `Without<..>` filters stating that the bars, the labels and the beat rings are
   three different bodies. ⚠ this is the param-panic class: it compiles and
   fails at first run.

## API drift fixed, all in the demo layer

`a301a79a0` and the session-scope change, mechanically:

- `WorldlineTracked2d(String)` → `WorldlineTracked2d::new(..)` (identity and
  caption are two values now)
- `history.tracks` is keyed by `WorldlineTrackId`, not `&str`
- `SessionRoot.0` is a `SessionScopeId`, not an `Entity` — both spawn helpers
  retyped
- `RelativityClockObservation2d::proper_time` → `proper_time_seconds`

## Gates

`cargo check -p ambition_app` green; `check_absence_contracts.py --check` 25/25;
schema baseline and exit oracle green at v18.
