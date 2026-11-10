# Authoring guides

Use the guide for the content you are editing. Runtime behavior is owned by
game and engine code; the authored sources describe dialogue, characters,
assets, and world content. Generated outputs should be regenerated from
those sources instead of edited directly.

- [Narrative guide](narrative-guide.md) — dialogue, character voice,
  writing conventions, and narrative content.
- [Anti-LLMism style guide](anti-llmism-style-guide.md) — concrete rules
  for player-facing writing; its hard bans are scoped to dialogue.
- [Content pack validation](../recipes/validating-a-content-pack.md) —
  add or change authored RON and run the compiler without rebuilding Rust.
- [LDtk authoring](../recipes/ldtk-authoring.md) — edit and validate rooms,
  levels, geometry, and spatial fields.
- [Adding an asset](../recipes/adding-an-asset.md) — source assets,
  generated products, and publication.
- [Generated audio](../tools/generated-audio-tools.md) — music and SFX
  authoring tools.

[Content and provider boundaries](../concepts/content-and-provider-boundaries.md)
explains which layer owns each kind of data. [Agent-native
authoring](../concepts/agent-native-authoring.md) explains discovery,
mutation, validation, provenance, and review across authoring tools.
