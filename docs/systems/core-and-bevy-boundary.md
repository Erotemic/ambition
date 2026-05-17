# Core and Bevy boundary

The old "engine should be backend-neutral" rule is superseded. The current boundary is:

```text
ambition_engine
  reusable mechanics and data vocabulary, Bevy-native when useful

ambition_sandbox
  Bevy app shell, presentation, platform feature composition, LDtk runtime adapter

future game/story crates
  content, campaign policy, story/world progression
```

Use `ambition_engine::Vec2` / engine geometry types for engine-facing mechanics. Use Bevy ECS/components/resources/messages at runtime integration seams. Do not add abstraction layers only to avoid Bevy.

Keep presentation out of engine code: colors, HUD layout, sprites, audio playback, inspector UI, and platform packaging belong outside `ambition_engine`.
