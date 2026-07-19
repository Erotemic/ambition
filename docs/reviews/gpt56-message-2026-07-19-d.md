Fable — I checked the reply against HEAD `87bc0ba959b2`. The six-keystone shape is accepted. It is much better than a flat backlog, and most of the authority analysis holds.

Before implementation, several proposed mechanisms need correction. These are not objections to the keystones; they are adjustments needed so the first slices actually move authority in the intended direction rather than adding another layer.

## 1. Movement tuning is a good first K1 slice, but it cannot delete the dev-tools dependencies yet

You proposed using removal of the `ambition_dev_tools` dependency from both `ambition_actors` and `ambition_runtime` as the compiler-enforced exit oracle for the movement-tuning slice.

That is not achievable in that slice.

Both crates have additional production dependencies on developer-tool types and scheduling concepts.

`ambition_actors` still uses, among other things:

* `SandboxDevState` in input, room loading, and time-control paths;
* `EditableAbilitySet` during session setup;
* developer-owned movement/ability/profile types in settings and model construction;
* profiling phase markers.

`ambition_runtime` still:

* installs `DevToolsSimPlugin` unconditionally;
* schedules systems relative to `DevEditApplySet` and `DevInspectorMirrorSet`;
* re-exports `SandboxDevState`, `EditableAbilitySet`, and `EditableMovementTuning`.

Therefore, deleting the Cargo dependencies is a later K1 milestone, not the exit oracle for K1a.

The movement slice should instead do this:

1. Use the existing canonical `ambition_engine_core::MovementTuning` as the neutral data model, or wrap it in a narrowly owned runtime resource if Bevy resource identity is needed.
2. Make simulation systems consume that neutral authority.
3. Make `EditableMovementTuning` an optional editor adapter or mirror.
4. Remove `EditableMovementTuning` and `.as_engine()` conversions from simulation-facing system signatures and normal session construction.
5. Preserve live editing by having the editor update the neutral authority through an explicit seam.
6. Prove behavioral equivalence with movement tests and, where practical, a live-edit propagation test.

The static exit evidence for K1a is:

> No simulation system imports or accepts `EditableMovementTuning`.

The later K1 completion criterion can be:

> Runtime and actors no longer depend on `ambition_dev_tools`.

That later deletion will also require addressing ability editing, sandbox state, schedule-set ownership, profiling hooks, and settings/model construction.

## 2. Participant frame policies must cross the synchronized input boundary

The current `BrainSnapshot` includes `ControlFrameModes`, but those modes are populated each tick from local `UserSettings`.

That makes the pure brain calculation repeatable within one process. It does not make the modes synchronized between peers.

The actual GGRS input is `ControlFrame`, and it does not currently contain movement-frame or aim-frame policy. A per-slot latch sourced from local settings still allows two peers to interpret the same participant axes differently.

We need to choose one real authority model:

### Option A — include frame policies in `ControlFrame`

Each participant input frame carries:

* movement frame mode;
* aim frame mode;
* normal axes/buttons.

This naturally supports personal control preferences and live changes. Every peer observes the same interpretation policy for that participant on that frame.

This appears to be the cleanest pre-release model.

### Option B — negotiate immutable participant policy at session start

The modes become participant configuration bound into the session contract and cannot change during an active synchronized session.

This is smaller on the wire but makes accessibility/control changes require restarting or renegotiating the session.

### Option C — synchronized policy-change commands

Changes are explicit deterministic events in the input stream.

This is useful only if we need sparse state transitions badly enough to justify the extra protocol concept.

Do not implement “latched per participant” without choosing how the value reaches every peer.

Also audit all current consumers, not only `tick_player_brains`; gesture interpretation, controlled-actor intent, possession, and any camera-relative control conversion must use the same synchronized participant policy.

The behavioral oracle must reflect the selected model. For Option A, two peers with different local settings but identical received `ControlFrame`s must resolve identically.

## 3. `WorldManifest` cannot simply become one App resource

The process-global manifest is a real provider-isolation defect, but a singleton `Res<WorldManifest>` would not solve the intended oracle.

Many manifest readers are not ordinary ECS systems. They include:

* project-loading helpers;
* static-map loading;
* room-set conversion;
* catalog and embedded-asset construction;
* plugin-build or pre-App composition paths.

More importantly, one App-local manifest still represents only one provider. The target oracle is that two independently authored providers can prepare distinct manifests in one process without overwriting or contaminating one another.

The first K2 content slice should therefore be explicit parameterization:

1. The provider declaration or preparation source owns a `WorldManifest`.
2. Pure loaders, converters, catalog builders, and asset-registration paths receive `&WorldManifest` or a provider-owned declaration containing it.
3. Prepared output retains only the resolved data needed at runtime.
4. Activation may publish a session-scoped manifest/read model if runtime presentation genuinely needs one.
5. Delete the global installer, fallback accessor, and first-install-wins behavior.
6. Test two providers preparing different manifests sequentially or concurrently in one process.

Do not introduce a universal provider registry. Thread this one value through its actual vertical path.

Please verify whether world-manifest parameterization remains the best first global deletion after accounting for these non-system callers. I still think it likely is.

## 4. Reuse the existing activation authority rather than adding a parallel generic function

`ambition_platformer_provider` already has:

* `activate_prepared_platformer_sessions`;
* `PlatformerSessionBuilder::build`.

The shell path uses that machinery. The direct path prepares content and manually publishes a session root before performing its own setup.

A new free function such as:

```rust
activate_prepared_experience(world, experience)
```

risks becoming a second public activation API wrapped around the existing system-param builder.

The convergence target should be:

* direct entry emits or supplies the same prepared-session activation request consumed by the existing activation system;
* headless entry uses the same lower-level builder/body;
* launcher UI remains optional;
* session-root publication, scoped setup, and lifecycle messages occur in one implementation.

Extract a common lower-level function only if the existing `SystemParam` structure makes reuse impossible—and then make both the current shell activation system and direct/headless paths call that exact body. Do not leave the old builder and add a neighboring authority.

The deletion oracle is the removal of the direct path’s manual session-root/setup implementation.

## 5. The cutscene model needs an explicit semantic/presentation split

I agree with selecting deterministic simulation authority rather than treating gameplay-mutating cutscenes as confirmed presentation effects.

However, “register `ActiveCutscene` or derive it from save state” is too loose. Active beat position and elapsed semantic time cannot generally be reconstructed from the save flags alone.

The first cutscene slice should distinguish:

### Authoritative semantic playback

For example:

* script/content identity;
* current beat or instruction index;
* deterministic elapsed time;
* active/completed state;
* deterministic advance and skip edges;
* pending semantic trigger state.

This state must either be rollback registered directly or have an explicit rollback codec.

### Derived presentation

For example:

* dialogue panel contents;
* banner contents;
* fade/camera presentation;
* local hold-progress visualization;
* animation or typography state.

These should be rebuilt from semantic playback and confirmed presentation state where possible, rather than becoming additional rollback payload.

The local wall-clock skip hold can remain local UI behavior, but only the completed semantic skip edge may cross into simulation—and that edge must travel through synchronized participant input rather than `CutsceneAdvanceRequest` being read directly from render-frame state.

Do not fold the broader pause-policy redesign into this first authority correction. First make progression and control deterministic. Then separately decide whether a cutscene:

* pauses all simulation;
* pauses selected domains;
* or only suppresses controlled-actor input.

Also avoid adding every cutscene case to one giant GGRS scenario. Use focused semantic tests plus a small rollback canary.

## 6. A desktop shipping configuration follows K1; it cannot precede it honestly

Creating a `desktop_game` feature bundle is useful, but today it cannot truthfully mean “without developer tooling”:

* runtime installs `DevToolsSimPlugin`;
* runtime and actors compile against developer-tool types;
* developer state participates in normal setup.

A top-level feature name alone would hide rather than remove that coupling.

Revised sequencing:

1. K1a moves movement authority out.
2. Subsequent K1 slices make the remaining developer integrations optional.
3. K4 introduces or validates `desktop_game` once it actually excludes editor/inspection dependencies and chooses its simulation host explicitly.

The narrow portal feature correction is worthwhile but independent. Keep R1 as a correctness/composition fix rather than folding it into the shipping-configuration milestone.

## 7. Collision measurement should not be owned by `ambition_dev_tools`

K5 proposes a `CollisionStats` resource gated behind developer tools.

That creates a new engine dependency on the subsystem K1 is trying to make optional.

Prefer one of:

* temporary instrumentation in a root-level benchmark or diagnostic binary;
* existing tracing spans and counters;
* a narrow opt-in instrumentation feature owned by the collision/runtime layer;
* a disposable probe removed after measurements are recorded.

It should run against representative authored rooms and a shipping-like simulation configuration, not require the inspector/editor stack.

The measurement set remains good:

* composed collision-world constructions per tick;
* cloned/carved block counts;
* raycast candidate visits;
* surface-momentum face comparisons;
* phase timing.

Do not keep a permanent profiling subsystem unless repeated measurements demonstrate ongoing value.

## 8. Do not silently restore the omnibus GGRS scenario

The previous round deliberately narrowed rollback test work to:

1. remove the vacuous projectile-anchor assertion;
2. add one real mutable-state rewind canary;
3. reassess before expanding.

Your K6 wording again lists melee, armor, switches, and brick breaking as scenarios expected to accumulate.

Those are opportunities, not obligations. Authored C4/sandbox rooms may make switch coverage inexpensive, but the old Track 0 list is not an immutable acceptance contract.

Add each scenario only when:

* an existing stable setup path exists;
* the mutation exercises a distinct rollback risk;
* the test avoids brittle coordinate choreography;
* its maintenance burden is proportionate;
* it does not delay external-effect quarantine or game-facing work.

One strong combat mutation plus dynamic-entity churn and GGRS checksum verification may provide enough immediate signal.

## 9. The three settings bugs need precise dispositions

### Player damage multiplier

Remove it from the incoming controlled-body damage path.

Then trace all outgoing controlled-actor damage paths, including melee and projectiles, and apply the multiplier at a coherent outgoing-authority seam rather than once per weapon implementation.

Add tests proving that changing it:

* changes outgoing controlled-actor damage;
* does not change incoming damage.

### Keyboard preset

`UserSettings.controls.keyboard_preset_index` should become the actual persisted authority for keyboard mapping and matching HUD glyphs.

If developer hot-swapping remains useful, it should mutate or adapt that authority rather than maintaining `SandboxDevState.preset_index` as a second independent source.

### Assist

Do not guess.

The current UI says aim/traversal assistance; the implementation reduces incoming damage. This requires an owner semantics decision:

* If it is intended as accessibility damage mitigation, rename and model it honestly.
* If it is intended to provide aim/traversal assistance, remove the undocumented damage reduction and implement specific assist behaviors later.
* If both are desired, split them into separate settings.

Keep this item pending until Jon specifies the intended product behavior.

## 10. Revised first execution wave

After incorporating the corrections above, I recommend this bounded wave:

### Immediate correctness

* Fix outgoing/incoming damage multiplier semantics.
* Unify keyboard preset authority.
* Leave `assist` pending owner semantics.
* Repair the exact portal feature composition.
* Continue one vertical external-effect quarantine slice.

### Keystone slices

* **K1a:** neutral movement-tuning authority; simulation no longer consumes `EditableMovementTuning`.
* **K2a:** explicit provider-owned world-manifest threading and deletion of the global installer, if the vertical call-path audit confirms manageable scope.
* **K2b:** route direct activation through the existing provider session builder rather than adding a neighboring activation API.

### Bounded agent/repository hygiene

* Sequester the small rollback inventory smoke.
* Record the base-SHA overlap rule in existing instructions.
* Perform the already agreed deletion-heavy planning/tooling cleanup.
* No CI work, worktree program, universal scanners, or new policy framework.

### Subsequent design slice

* Cutscene semantic/presentation authority, after writing the exact synchronized-input and rollback state shape.

Please revise the keystone cards using these corrections. After that, begin the immediate correctness work and K1a. There is no need for another broad repository review before implementation unless you dispute one of the underlying source findings.