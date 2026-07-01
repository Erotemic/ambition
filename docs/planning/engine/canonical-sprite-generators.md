# Canonical sprite generators (dissolving the adapter layer)

Status: DONE (2026-07-01). Landed in renderer submodule commit `d49b152`.
The adapter layer is dissolved; all nine generators are canonical
`CharacterGenerator`s. Renderer suite 148 pass / 30 skip, unchanged from
baseline. A follow-up remains: unify the `TackonTarget` / `AdapterTarget` split
in `registry/discovery.py` (both are one concept now that generators are the
canonical shape).

Goal (from Jon's steer): make the sprite-generation code logically organized and
canonical so it *reads beautifully* — **not** by adding adapters that reshape
per-target output into a canonical form, but by making the generators *be*
canonical. Scope: the adapter seam **and** the target generator files. The
canonical shape is owned by the Python renderer; Rust keeps consuming the
emitted `*_spritesheet.ron`.

## The problem

`authoring/adapters.py` (804 lines) is a layer of nine near-identical wrapper
classes — `GoblinAdapter`, `RobotAdapter`, `BossAdapter`, … — each of which:

- holds a per-target generator (`SideGoblinGenerator`, …),
- re-exposes `generator.ANIMATIONS` as `animations()`,
- forwards `sample_spec(job)` to `generator.sample_spec(seed, archetype, …)`,
- forwards `render_frame(...)` to `generator.render_animation_frame(...)` with a
  target-specific `parse_background`.

The generators are *not* canonical; the adapter reshapes each one into the
canonical interface the sheet pipeline consumes. Two further symptoms:

- `registry/discovery.py` carries two parallel "target" kinds — `TackonTarget`
  (Python-module targets) and `AdapterTarget` (YAML-config targets that wrap an
  adapter) — for what is one concept.
- The authored gameplay geometry (`RobotAdapter.attack_hitboxes` / `body_inset`,
  `BossAdapter.hurtbox_parts`) lives in `adapters.py`, away from the sprite it
  describes.

## The canonical shape

One base, `CharacterGenerator` (in `authoring/generator.py`). A target's
generator **is** a `CharacterGenerator`; the pipeline renders any
`CharacterGenerator`. There is no wrapper object.

The base owns the shared machinery:

- `animations()` → `dict(self.ANIMATIONS)`
- `sample_spec(job)` → `_apply_overrides(self.build_spec(job), job)`
- `render_single` / `render_canonical`
- `spec_dict`, default `canonical_pose`, and default (empty) `attack_hitboxes` /
  `hurtbox_parts` / `body_inset` hooks.

Each generator implements only what is genuinely its own:

- `ANIMATIONS` (class attribute)
- `build_spec(self, job)` — sample the spec from the job (the old
  `sample_spec(seed, archetype, …)` body, reading the seed/archetype/held_item
  and name off the job)
- `render_frame(self, spec, animation, frame_index, size, job)` — render one
  frame, calling its own low-level renderer with its own background parser
- the authored gameplay geometry that belongs to *that* character
  (`attack_hitboxes` / `hurtbox_parts` / `body_inset`), co-located with it.

`TARGETS` maps each target id straight to a generator instance; `get_generator`
replaces `get_adapter`. `adapters.py` is deleted.

## Execution order (checkpointed on main)

1. `CharacterGenerator` base.
2. Convert `goblin` (pattern proof) and flip its registry entry.
3. Convert the rest (`boss`, `robot`, `ninja`, `toon`, `trent_elder`,
   `bob_engineer`, `alice_cryptographer`, `sandbag`), moving authored geometry
   onto its generator.
4. Delete the adapter wrappers, rename `get_adapter` → `get_generator`, update
   call sites (`sheet.py`, `canonical.py`, `cli/commands.py`,
   `registry/discovery.py`, tests).

Safety net: `tools/ambition_sprite2d_renderer/tests` (148 green at baseline;
`test_gen2d.py` exercises every generator through the interface). Run after each
step. The one pre-existing failure — `test_ldtk_manifest` pinning a drifted
`DEFAULT_ENTITY_SPRITE_MAP` — is unrelated.
