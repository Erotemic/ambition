# D120 — a platform capability is enabled beside the dependency that needs it

⚠ **EVIDENCE, NOT AUTHORITY.** Closed case file. The RULE it produced is live and
restated in the ledger; this is the investigation that produced it.

- ▢ **D120 — A platform capability is enabled beside the dependency that needs
  it, not at the app.**

⭐ **found by the web link failure (2026-08-14), and the evidence is in the
repo.** `ggrs` publishes ONE browser feature —
`wasm-bindgen = ["instant/wasm-bindgen", "getrandom/js"]` — and this tree was
supplying HALF of it from `game/ambition_app/Cargo.toml`, as three direct
`getrandom` dependencies whose own comment said they existed *"to make Cargo
feature-unify"*. The RNG half was covered and the CLOCK half was not, so a
browser build compiled and then died at `rust-lld: undefined symbol: now`. Fixed
by giving `ambition_platformer2d_runtime` — which owns `bevy_ggrs` — a
`web_platform` feature and forwarding the facade's existing one into it.

⚠ **the surviving siblings are `getrandom_03` / `getrandom_04`**, still declared
at the app for the same reason. They are NOT the same defect: their owners
publish no forwarding feature, so the app IS their nearest owner. They read like
the deleted line, which is exactly why the difference is worth writing down.

⛔ **do not turn this into a Cargo-feature redesign.** The rule to apply when a
NEW target-specific need appears: enable it in the crate that declares the
dependency, and let the app forward a semantic capability. A future consumer of
an Ambition runtime crate should be able to ask for browser support without
knowing what that crate depends on.

