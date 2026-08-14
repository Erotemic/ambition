# Web platform parity — the browser runs a different application

> **Opened 2026-08-14** from Jon's report: the wasm builds and links, the page and
> canvas load, the overlay says *"click the canvas to capture keyboard"* and never
> clears, and **no shell or game presentation ever appears**. Assets return `200`;
> only `.meta` sidecar probes 404.

## ⭐ THE CAUSE IS MEASURED, AND THE SOURCE ALREADY NAMED IT

`run_web()` (`game/ambition_app/src/app/cli.rs`) composes:

```text
DefaultPlugins (canvas)  ·  init_engine_states  ·  serialize_frame_schedules
GameAssetConfig          ·  insert_starting_character_override
SimulationHost::Ggrs
AmbitionGameSimulationPlugin + AmbitionGameLdtkRuntimePlugin + AmbitionGamePresentationPlugin
AmbitionAssetSourcePlugin
app.run()
```

`build_visible_app_with` composes all of that **and three more things**:

```text
app.insert_resource(AmbitionShellHosted)          ⟵ absent on web
compose_ambition_shell_host[_booting_to](app)     ⟵ absent on web
install_ambition_shell_visuals(app)               ⟵ absent on web
```

⛔⛔ **and the builder's own doc comment describes this exact failure, from the
last time somebody hand-spelled the composition:**

> *`capture_scene` therefore spelled the whole composition out by hand — and the
> hand-spelled copy silently lost the `--route` positional, the headless display
> surface, `--dev-overlays`, `--combat-overlay`, and (2026-08-06 → 08-08, for two
> days) **the entire room**, because `install_ambition_shell_visuals` was never
> added to it.*

No shell host ⇒ no route. No route ⇒ no title screen and no gameplay route to
boot into. No `install_ambition_shell_visuals` ⇒ no room. That is the blank
canvas, and it is the same defect that file already paid for once.

⚠ **`build_visible_app_with` is `#[cfg(not(target_arch = "wasm32"))]`**, so the
web path cannot call it. That gate is why the third copy exists — and the comment
above says why the second one was a mistake.

## The shape of the repair

⛔ **do not paste the three missing calls into `run_web`.** That leaves two
builders and the next composition input added to one will be missing from the
other, which is the whole history above repeating.

```text
platform host foundation        ← differs: native window vs browser canvas,
                                   arg parsing, simulation-host choice
            ↓
ONE visible Ambition composition ← identical: states, sim, ldtk, presentation,
                                   shell host, initial route, shell visuals,
                                   asset source
            ↓
platform run                    ← differs: App::run on both, but the native
                                   builder RETURNS the app
```

The seam in `build_visible_app_with` is already clean: everything from
`insert_starting_character_override` to the `AmbitionAssetSourcePlugin` is a pure
function of `(render mode, shell_hosted, asset_config, compose_inputs)`. Extract
that, compile it on every target, and let both entry points call it.

## Open

- ▢ **Factor the one visible composition** and make `run_web` a platform
  foundation plus a call to it. Deletion payoff: the third hand-spelled
  composition stops existing.
- ▢ **A web-boot acceptance test.** Link ≠ boots: the release-link gate passes on
  an app that shows a blank canvas. The narrowest contract is that the web
  persona installs the same shell/route/visuals composition the native one does —
  behaviourally, on a small `App`, not by grepping source.
- ▢ **`web/index.html`'s status is not truthful.** It announces "click the canvas
  to capture keyboard" after wasm init and never clears on click. It must not
  imply readiness it does not know about — DOM status is browser-shell
  presentation, never authority over game readiness.
- ▢ **Web packaging symlinks an implementation crate.** `build_for_web.sh
  --served` publishes `web/assets -> crates/ambition_platformer2d_actor_monolith/
  assets`. ⛔ the fix is NOT a newer hard-coded crate path: the repo already has
  `AssetId` / `AssetManifest` / `AssetProfile` / `AssetLocation` and a layered
  source, and web-served should consume that canonical publication tree. The
  acceptance condition Jon named: *the served-web path keeps working if the
  actor-monolith crate is renamed or deleted.*
- ▢ **Audit that web and native resolve the same logical `AssetId`s** — a
  representative set (defaults RON, a controlled-body sprite, a boss sprite, a
  font, the SFX bank, music, one game-owned and one engine-owned asset). Only the
  physical delivery should differ.

## ⚠ NOT the bug

`.meta` 404s beside `200`s for the real asset are Bevy probing optional metadata
sidecars. The favicon 404 is not game-blocking. Neither explains a blank canvas,
and chasing a clean HTTP log is not this row.
