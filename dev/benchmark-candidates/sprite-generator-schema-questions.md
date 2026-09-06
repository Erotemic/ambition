# Sprite generator schema overlay questions

## Q: How do you add new fields to a dataclass-backed YAML config without clobbering fields from earlier overlays?

### Context

A procedural sprite generator uses `RenderConfig(**render_data)` and
`CharacterJob(...)` to load YAML files. Earlier work added `crop`,
`crop_padding`, `variant`, `faction`, `role`, `music_cue`, and `tags`.
A later overlay added `name`, `output_name`, and `spec` overrides for a new
character target, but replaced `config.py` with an older schema.

### Failure

Existing YAML files with `render.crop` failed at load time:

```text
TypeError: RenderConfig.__init__() got an unexpected keyword argument 'crop'
```

Faction code that constructed `CharacterJob(variant=..., faction=..., ...)`
would also fail if only the new schema survived.

### Expected answer

When evolving a dataclass-backed YAML schema, merge fields from all active
schema versions rather than replacing the dataclass with the version needed by
the new feature. Preserve previously accepted YAML keys and constructor
parameters, then add new fields and defaults. If a new patch introduces
`output_name` and `spec` overrides, `RenderConfig` must still include
`crop` / `crop_padding`, and `CharacterJob` must still accept faction metadata
and variant fields.

A good fix is:

- keep `RenderConfig.crop` and `RenderConfig.crop_padding`;
- keep `CharacterJob.variant`, `faction`, `role`, `music_cue`, and `tags`;
- add `CharacterJob.output_name` and `spec_overrides`;
- parse `spec` and `spec_overrides` as aliases;
- update adapters to apply overrides without removing existing adapter targets.
