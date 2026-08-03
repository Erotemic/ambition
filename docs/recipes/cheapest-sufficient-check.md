# The cheapest command that settles a change

⭐ **Front 3b of the test-iteration campaign** (`docs/planning/test-iteration-cost-2026-08-02.md`).
Jon, 2026-08-02: *"run_tests looks so alluring to an agent, it prevents it from
running the focused test that actually matters, and instead it just runs all the
junk."* A faster front door does not fix that. Knowing which narrow command is
SUFFICIENT does, and that is what this page is.

Pick the row for what you changed, run it, read what the row says it does not
cover, and stop. There is no CI; the exhaustive sweep has an owner and a cadence
(Jon, periodically), and running it mid-edit duplicates that sweep rather than
adding safety.


  | what you changed | run this | what it does NOT cover |
  |---|---|---|
  | Rust inside ONE crate, no public API moved | `cargo test -p <crate>` | anything composing that crate; feature-gated tests |
  | anything a `#[cfg(feature = …)]` gates | `cargo test -p <crate> --features <f>` | that the app still builds; other combinations |
  | a crate SEAM: a trait, a re-export, a dependency edge, a registration | `cargo check -p ambition_app` then `cargo test -p ambition_app --test app_it -- <module>` | ⛔ a defect that only exists where two CONTENT crates meet — no per-crate job can see it (see the `Empowered` double-registration, 2026-08-03) |
  | a rollback registration, a schedule pin, a message channel | `cargo test -p ambition_app --test app_it -- rollback_` **and** `python3 -m pytest scripts/tests/ -q` | feature-gated channels: only the union job compiles those |
  | Bevy app WIRING (plugins, systems, ordering) | `cargo test -p ambition_app --test app_it -- <module>` | ⚠ pin `TimeUpdateStrategy` in any new test app or it measures the machine's load |
  | authored CONTENT (LDtk, catalogs, characters) | `cargo test -p ambition_app --test app_it -- declared_art_resolves registered_character_art` | that it plays; use `capture_scene` |
  | anything you can SEE | `capture_scene --route <id> out.png 1280x720 --warmup N` | correctness; it only proves what is drawn |
  | generated assets or a regen script | the regen script, then the guard it feeds | ⚠ another session may be regenerating the same tree |
  | Python tooling / a guard | `python3 -m pytest scripts/tests/ -q` | everything Rust |
  | a runner or workspace-wide change | `./run_tests.sh` | feature-gated tests, the consumer fixtures, the wasm check |

## Why each caveat is there

Every "does NOT cover" column above is a defect this repository actually had:

* **two content crates meeting** — `Empowered` was registered for rollback by
  Mary-O and by Sanic; each demo's own tests passed, `cargo check -p ambition_app`
  passed, and the app — which composes both — panicked on the first frame,
  killing 56 tests. No per-crate job can see that class.
* **feature-gated channels** — three `causal` message channels sat outside both
  rollback oracles because the default job never compiled them. Only the union
  graph did.
* **`TimeUpdateStrategy`** — under `Automatic`, `app.update()` advances the clock
  by REAL time, so a test that asserts a distance or a count measures how busy
  the machine is. Three separate tests have been fixed for this.
* **another session regenerating the tree** — a 63-minute suite run once outlived
  its own inputs, and two jobs failed on `include_str!` for files that existed
  before and after but not during.
