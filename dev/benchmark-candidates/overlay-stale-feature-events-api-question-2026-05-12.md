# Benchmark candidate: overlay patches must preserve current typed event APIs

## Context

An overlay for Ambition added generated foreground parallax layers and touched
`crates/ambition_app/src/app/world_flow.rs` to play switch and generic SFX
from `FeatureEvents`.

The patch was prepared from an older source archive where `FeatureEvents` still
had public side-channel vectors:

```rust
pub switches_activated_pos: Vec<ae::Vec2>,
pub sfx_plays: Vec<(ambition_sfx::SfxId, ae::Vec2)>,
```

The current repo had already migrated cross-system effects into a typed
effects table with accessor methods:

```rust
pub effects: Vec<GameplayEffect>,

pub fn switch_activations(&self) -> impl Iterator<Item = (&str, ae::Vec2)> + '_
pub fn sfx_plays(&self) -> impl Iterator<Item = (ambition_sfx::SfxId, ae::Vec2)> + '_
```

After applying the full-file overlay, `cargo` failed with:

```text
error[E0609]: no field `switches_activated_pos` on type `&features::events::FeatureEvents`
error[E0615]: attempted to take value of method `sfx_plays` on type `&features::events::FeatureEvents`
```

## Benchmark question

You are preparing a full-file overlay for a Rust game repo. The overlay was
started from a source archive, but the user's checkout may have moved forward.
One edited file needs to consume switch activations and generic SFX emitted by
`FeatureEvents`.

Given this current API shape:

```rust
pub enum GameplayEffect {
    ActivateSwitch { payload: String, pos: ae::Vec2 },
    PlaySfx { id: ambition_sfx::SfxId, pos: ae::Vec2 },
    // ...
}

#[derive(Default, Clone, Debug)]
pub struct FeatureEvents {
    pub effects: Vec<GameplayEffect>,
    // presentation vectors omitted
}

impl FeatureEvents {
    pub fn switch_activations(&self) -> impl Iterator<Item = (&str, ae::Vec2)> + '_ { /* ... */ }
    pub fn sfx_plays(&self) -> impl Iterator<Item = (ambition_sfx::SfxId, ae::Vec2)> + '_ { /* ... */ }
}
```

What should the overlay code in `handle_feature_events` do, and what validation
step should be performed before handoff?

## Expected answer

Do not resurrect or directly read stale side-channel fields. Consume the current
accessor methods so the typed effects table remains the single source of truth:

```rust
for (_payload, pos) in events.switch_activations() {
    sfx.push(SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_SWITCH_TOGGLE,
        pos,
    });
}

for (id, pos) in events.sfx_plays() {
    sfx.push(SfxMessage::Play { id, pos });
}
```

Before handoff, re-check every full-file overlay against the current checkout or
current code search result, not just the uploaded archive. Then run at least:

```bash
cargo fmt --all
cargo check -p ambition_sandbox
```

If the environment cannot run Cargo, state that explicitly and still inspect the
current API for edited call sites.

## What this tests

- Whether an agent notices that an uploaded archive may be stale relative to the
  user's checkout.
- Whether it preserves the current typed event-table invariant instead of
  reintroducing old parallel vectors.
- Whether full-file overlays are treated as API-rebase tasks, not just text
  generation from old files.
