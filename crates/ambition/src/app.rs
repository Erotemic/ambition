//! **Standing up a game.** The engine owns composition ordering; the consumer
//! states policy.
//!
//! Slice A3 of [the API 1.0 campaign](../../../docs/planning/engine/api-1.0-campaign.md),
//! implementing the call sites in `docs/sdk/api-prototype.md`. ADR 0031
//! decision 4: *a consumer states policy — windowed or headless, which
//! experience, where it starts. It does not sequence asset sources, engine
//! plugin groups, host groups, shell composition, asset preparation and
//! presentation. Every ordering constraint the engine knows is a rule the
//! engine states once.*
//!
//! ```ignore
//! use ambition::app::prelude::*;
//!
//! fn main() {
//!     PlatformerApp::windowed("My Game")
//!         .mount(MyModule::default())
//!         .run();
//! }
//! ```
//!
//! # The eight rules this module owns
//!
//! Every one was a line a third party had to write in the right place, and four
//! of them failed SILENTLY — which [the growth
//! method](../../../docs/planning/engine/api-growth-method.md) §3a prices at
//! triple, because a leak that panics teaches and a leak that falls back
//! quietly does not.
//!
//! 1. declared asset sources register **before** any `AssetPlugin` builds —
//!    Bevy seals its sources there. *(silent: assets resolve against the engine
//!    tree)*
//! 2. `AssetPlugin.file_path` is the engine's own asset root. *(silent: engine
//!    content does not load)*
//! 3. a GPU-less window needs five disables plus `RenderPlugin { backends:
//!    None }`.
//! 4. `init_engine_states` before the engine plugin groups. *(panic)*
//! 5. engine plugins, then host plugins, then the shell. *(panic)*
//! 6. `PlatformerAssetsPlugin` **after** the content that registers the
//!    catalogs it reads and **before** the presentation that draws what it
//!    installs. *(silent: unskinned bodies)*
//! 7. a host that names no initial route prepares and activates nothing.
//!    *(silent: an earlier draft of the fixture's headless binary "ran" 120
//!    ticks of an empty host)*
//! 8. manual stepping pins the frame dt to the tick dt, read back out of the
//!    world after the plugins built it. *(silent: frame dt drifts from tick
//!    dt)*
//!
//! Rule 7 is not enforced by ordering but by REFUSAL, in two steps:
//! [`PlatformerApp::try_build`] rejects a module that declares no gameplay
//! route, **and** rejects one whose declared route no mounted capability
//! registers — naming the routes that do exist.
//!
//! ⚠ Only the first half existed until 2026-07-30, while this paragraph claimed
//! "the empty host is unreachable rather than merely documented". It was
//! reachable: what was enforced is that a STRING had been supplied. The blind
//! agent run declared a route nothing served and got a host that built clean,
//! ran 60 ticks and spawned zero entities. An overclaimed guarantee is worse
//! than an absent one — it tells a consumer to stop looking — and the agent
//! found it only because it independently counted entities.
//!
//! # What this is not
//!
//! **It owns no behavior.** It re-exports contracts and sequences installs.
//! ADR 0031: *"if the facade ever grows a leaf system, it has become the next
//! monolith and this ADR has failed."* Assembly is not a leaf system, and the
//! umbrella is already where `game_assets` lives for the same reason — it is
//! the one surface allowed to see layers that may not see each other.
//!
//! **It is not a runtime.** ADR 0031 decision 5: a studio with an existing
//! Bevy `App` adds this without surrendering the `App`
//! ([`PlatformerApp::install_into`]). The engine owns ordering *within its own
//! installation*, not the consumer's process.

use bevy::app::Plugins;
use bevy::prelude::*;

use crate::world::rooms::RoomMetadata;

/// Curated imports for a game's `main`.
///
/// **A domain prelude, not the root one.** `ambition::prelude` re-exports
/// twenty-five crate mirrors; an agent told to import all of them has been told
/// nothing about which four matter. Campaign §A2: one enormous root prelude is
/// a discovery problem, not a convenience.
pub mod prelude {
    pub use super::{
        AssetSource, CompositionError, GameModule, ModuleDraft, ModuleManifest, PlatformerApp,
        SessionMode,
    };
    pub use bevy::prelude::App;
}

/// A named asset tree the game owns, layered over the engine's own.
///
/// Carried as **data** rather than applied as a call, because the moment it
/// must be applied is a moment only the engine knows (rule 1) — and because a
/// declaration can be CHECKED in the one composition shape where the engine
/// cannot apply it. See [`PlatformerApp::install_into`].
#[derive(Clone, Debug)]
pub struct AssetSource {
    name: String,
    root: String,
}

impl AssetSource {
    /// `name` is the URL scheme (`game://…`); `root` is the game's asset
    /// directory.
    ///
    /// Anything the game did not author falls through to the engine's tree, so
    /// a consumer overlays rather than replaces.
    pub fn at(name: impl Into<String>, root: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            root: root.into(),
        }
    }
}

/// How the simulation is clocked.
///
/// **Fixed-step only, deliberately.** Rollback is not a public knob in slice A:
/// it is a far larger promise than a clock — frozen schema, complete
/// authoritative baseline, stable participants, deterministic activation,
/// lifecycle rebasing, confirmation boundaries — and it gets its own slice with
/// its own acceptance tests. See the campaign's Deferred section.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SessionMode {
    #[default]
    FixedStep,
}

/// What a module needs the engine to know **before** the Bevy foundation
/// builds.
///
/// Separate from [`ModuleDraft`] because it is needed EARLIER, not because two
/// types are tidier. Asset sources must be registered before `AssetPlugin`;
/// routes and capabilities must not. Folding them into one method would put the
/// asset source behind the same barrier as the content and reintroduce rule 1
/// from the inside.
#[derive(Clone, Debug)]
pub struct ModuleManifest {
    id: String,
    asset_sources: Vec<AssetSource>,
}

impl ModuleManifest {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            asset_sources: Vec::new(),
        }
    }

    pub fn asset_source(mut self, source: AssetSource) -> Self {
        self.asset_sources.push(source);
        self
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

/// One declared capability, as the closure that installs it.
///
/// ADR 0032 decision 2: *capability is code, and code is declared, then lowered
/// by the engine.* You cannot serialise a Bevy system into a document, so the
/// declaration is a deferred install and the engine chooses when to run it.
///
/// A closure rather than `Box<dyn Plugin>` because `Plugin::build` takes
/// `&self` and cannot move a boxed plugin out of a collection it borrows.
///
/// `FnOnce`, drained through a `Mutex` by [`DeclaredCapabilities`]. The first
/// version was `Fn` + a `Clone` bound on the plugin, and the 2026-07-30 blind
/// run found what that costs: **the engine's own `CharacterCatalogPlugin` is
/// not `Clone`, so an engine plugin could not go through the engine's own
/// capability slot.** Every consumer would have written the same wrapper. A
/// bound that excludes the API's own types is the API being wrong, not the
/// caller.
type CapabilityInstaller = Box<dyn FnOnce(&mut App) + Send + 'static>;

/// The inert accumulation a module writes into.
///
/// **Nothing here is live when `define` returns.** ADR 0032 decision 1: the
/// engine — which calls `define` — seals the draft, validates it, and only then
/// installs anything. That is what makes "is the declaration complete?" an
/// answerable question, and it is why this is not `&mut App`.
///
/// Slice A holds only what host assembly consumes. `ContentPackDraft` and
/// everything under it is slice B; a content method here would be a method
/// whose input nothing can yet validate.
#[derive(Default)]
pub struct ModuleDraft {
    /// Which module is currently being defined, so a conflict can name both
    /// sides rather than just reporting that one exists.
    defining: String,
    experience: Option<(String, String)>,
    launcher_route: Option<(String, String)>,
    gameplay_route: Option<(String, String)>,
    room: Option<RoomMetadata>,
    capabilities: Vec<CapabilityInstaller>,
    conflicts: Vec<String>,
}

impl ModuleDraft {
    /// The experience id this game registers content under.
    pub fn experience(&mut self, id: impl Into<String>) -> &mut Self {
        self.claim("experience", id.into(), |draft| &mut draft.experience);
        self
    }

    /// Where the host starts, and what `QuitToHome` resolves to.
    pub fn launcher_route(&mut self, route: impl Into<String>) -> &mut Self {
        self.claim("launcher route", route.into(), |draft| {
            &mut draft.launcher_route
        });
        self
    }

    /// The route the experience registered for its session.
    ///
    /// Required — rule 7. A host that never names one prepares and activates
    /// nothing, and does it silently.
    pub fn gameplay_route(&mut self, route: impl Into<String>) -> &mut Self {
        self.claim("gameplay route", route.into(), |draft| {
            &mut draft.gameplay_route
        });
        self
    }

    /// The room whose metadata picks block and biome art at `Startup`.
    pub fn room(&mut self, room: RoomMetadata) -> &mut Self {
        self.room = Some(room);
        self
    }

    /// Declare a capability. Installed by the engine, in its own order.
    ///
    /// No `Clone` bound: a capability is installed exactly once, so requiring
    /// it to be duplicable was an artefact of the first implementation rather
    /// than a property of capabilities. See [`CapabilityInstaller`].
    pub fn capability<M>(&mut self, plugin: impl Plugins<M> + Send + 'static) -> &mut Self {
        self.capabilities.push(Box::new(move |app: &mut App| {
            app.add_plugins(plugin);
        }));
        self
    }

    /// Record a single-owner claim, or a conflict naming both claimants.
    ///
    /// ADR 0032: *"module inclusion is a merge, not an ordering"* — over `&mut
    /// App` the question is "did Sanic's plugin run before Mary-O's", and over
    /// a value it is a conflict with two names in it.
    fn claim(
        &mut self,
        what: &str,
        value: String,
        slot: impl Fn(&mut Self) -> &mut Option<(String, String)>,
    ) {
        let owner = self.defining.clone();
        if let Some((held_by, held)) = slot(self).clone() {
            self.conflicts.push(format!(
                "two modules declare the {what}: `{held_by}` says `{held}`, \
                 `{owner}` says `{value}`"
            ));
            return;
        }
        *slot(self) = Some((owner, value));
    }
}

/// A game module: what it needs before the foundation, and what it declares.
///
/// Both methods take `&self` — campaign §A2. Not because `Box<dyn GameModule>`
/// demands it, but because a receiver-less `define` or an associated `const ID`
/// forecloses parameterised modules for nothing:
///
/// ```ignore
/// PlatformerApp::windowed("Sanic").mount(SanicModule { difficulty: Hard })
/// ```
pub trait GameModule {
    fn manifest(&self) -> ModuleManifest;

    /// Accumulate into the draft. **Never touches `App`.**
    fn define(&self, module: &mut ModuleDraft);
}

/// Everything wrong with a declaration, at once.
///
/// ADR 0032: *"a draft yields one build error listing every conflict in the
/// experience. `&mut App` yields a resource-missing panic three plugins later
/// — the failure `ShellComposition` was created to end."*
#[derive(Debug, Clone)]
pub struct CompositionError {
    pub problems: Vec<String>,
}

impl std::fmt::Display for CompositionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            formatter,
            "this game's composition cannot be built ({} problem(s)):",
            self.problems.len()
        )?;
        for problem in &self.problems {
            writeln!(formatter, "  - {problem}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CompositionError {}

/// How the game meets a display.
#[derive(Clone, Debug)]
enum Face {
    Headless,
    Windowed { title: String, gpu: bool },
}

/// The lowering of every declared capability, in declaration order.
///
/// The `Mutex` is what lets `Plugin::build(&self)` consume `FnOnce` installers:
/// it drains rather than borrowing. Uncontended — `build` runs once, on one
/// thread — so it costs nothing and buys the removal of the `Clone` bound that
/// the blind run showed excluded the engine's own plugins.
struct DeclaredCapabilities(std::sync::Mutex<Vec<CapabilityInstaller>>);

impl Plugin for DeclaredCapabilities {
    fn build(&self, app: &mut App) {
        let drained: Vec<CapabilityInstaller> = self
            .0
            .lock()
            .expect("capability installers are drained once, on one thread")
            .drain(..)
            .collect();
        for install in drained {
            install(app);
        }
    }
}

/// **The composition.** State policy; the engine states the order.
pub struct PlatformerApp {
    face: Face,
    session: SessionMode,
    /// Not a [`SessionMode`] arm, on purpose.
    ///
    /// Rollback is not a public knob in slice A, so putting a `Rollback` arm on
    /// the public enum would promise exactly what the campaign defers. But the
    /// external fixture has a rollback host TODAY, and leaving it hand-composed
    /// while the other two faces went through the builder would end the slice
    /// with two composition paths — rule 4's violation, and calling it
    /// "deferred" would not make it one.
    ///
    /// So: one composition authority, one publicly supported mode, and an
    /// escape hatch that is impossible to reach by accident.
    rollback_unstable: bool,
    game_assets: bool,
    manifests: Vec<ModuleManifest>,
    draft: ModuleDraft,
}

impl PlatformerApp {
    /// A game that opens a window.
    pub fn windowed(title: impl Into<String>) -> Self {
        Self::with_face(Face::Windowed {
            title: title.into(),
            gpu: true,
        })
    }

    /// A game with no display: tests, tools, RL, trace replay.
    ///
    /// Frame dt is pinned to the tick dt, so one `App::update` is exactly one
    /// sim tick (rule 8).
    pub fn headless() -> Self {
        Self::with_face(Face::Headless)
    }

    /// Build the full render graph against no wgpu backend.
    ///
    /// For GPU-less CI that still has to assert something was DRAWN. This is an
    /// engine concern — the five disables it implies are rule 3, and a consumer
    /// re-deriving them is the leak, not the convenience.
    pub fn without_gpu(mut self) -> Self {
        if let Face::Windowed { gpu, .. } = &mut self.face {
            *gpu = false;
        } else {
            self.draft
                .conflicts
                .push("`without_gpu` needs a windowed face; headless has no render graph".into());
        }
        self
    }

    /// Mount a module: fold in its manifest, and let it define itself.
    ///
    /// `define` runs **now**, into an inert draft. Nothing is live.
    pub fn mount(mut self, module: impl GameModule) -> Self {
        let manifest = module.manifest();
        self.draft.defining = manifest.id().to_string();
        module.define(&mut self.draft);
        self.draft.defining.clear();
        self.manifests.push(manifest);
        self
    }

    /// The composition, or every reason it cannot be built.
    pub fn try_build(self) -> Result<App, CompositionError> {
        let mut app = App::new();
        self.install_into(&mut app)?;
        Ok(app)
    }

    /// The composition. Panics with every problem listed, for a `main` that
    /// would only `unwrap` anyway.
    pub fn build(self) -> App {
        match self.try_build() {
            Ok(app) => app,
            Err(error) => panic!("{error}"),
        }
    }

    /// Build and run.
    pub fn run(self) {
        self.build().run();
    }

    /// Install into an `App` the consumer already owns.
    ///
    /// ADR 0031 decision 5 — a studio must be able to add this without
    /// surrendering its `App`.
    ///
    /// ⚠ **This form cannot honor rule 1, and says so instead of pretending.**
    /// Asset sources must be registered before `AssetPlugin` builds; an `App`
    /// that already has `DefaultPlugins` is past that point, and no ordering
    /// inside this call fixes it. What the engine can still do is *notice*: a
    /// declared source with a sealed `AssetPlugin` is a
    /// [`CompositionError`] naming the source, not assets that quietly resolve
    /// against the wrong tree. A rule the engine cannot own must at least be a
    /// rule the engine enforces.
    pub fn install_into(self, app: &mut App) -> Result<(), CompositionError> {
        let Self {
            face,
            session: SessionMode::FixedStep,
            rollback_unstable,
            game_assets,
            manifests,
            draft,
        } = self;

        let sources: Vec<AssetSource> = manifests
            .into_iter()
            .flat_map(|manifest| manifest.asset_sources)
            .collect();

        let mut problems = draft.conflicts.clone();
        if draft.experience.is_none() {
            problems.push("no module declared an experience id".into());
        }
        if draft.gameplay_route.is_none() {
            // Rule 7, as a type rather than as a comment.
            problems.push(
                "no module declared a gameplay route; a host that names none prepares and \
                 activates nothing, and does it silently"
                    .into(),
            );
        }
        if draft.launcher_route.is_none() {
            problems.push(
                "no module declared a launcher route; `QuitToHome` would have nowhere to land"
                    .into(),
            );
        }
        if !sources.is_empty() && app.is_plugin_added::<bevy::asset::AssetPlugin>() {
            for source in &sources {
                problems.push(format!(
                    "asset source `{}://` was declared, but `AssetPlugin` has already built in \
                     this App and Bevy seals its sources there. Let `PlatformerApp::build` own \
                     the whole stack, or register the source before adding `DefaultPlugins`.",
                    source.name
                ));
            }
        }
        if !problems.is_empty() {
            return Err(CompositionError { problems });
        }

        let experience = draft.experience.expect("checked above").1;
        let launcher_route = draft.launcher_route.expect("checked above").1;
        let gameplay_route = draft.gameplay_route.expect("checked above").1;

        // ── Rule 1 ── before any AssetPlugin, in every face.
        //
        // The fixture only did this on its windowed path. Headless got away
        // with it because it draws nothing — which is not the same fact as it
        // being correct, and is exactly the kind of "works today" a consumer
        // cannot distinguish from a rule.
        for source in sources {
            use bevy::asset::AssetApp as _;
            app.register_asset_source(
                source.name,
                crate::asset_manager::consumer_source::layered_asset_source(
                    source.root,
                    crate::asset_manager::actors_desktop_asset_root(),
                ),
            );
        }

        // ── Rules 2, 3, 4 ── the Bevy foundation.
        match &face {
            Face::Headless => crate::engine::add_headless_foundation(app),
            Face::Windowed { title, gpu } => install_windowed_foundation(app, title, *gpu),
        }

        // ── Rule 5 ── engine, then host, then shell.
        if rollback_unstable {
            app.add_plugins(crate::engine::PlatformerEnginePlugins::rollback());
        } else {
            app.add_plugins(crate::engine::PlatformerEnginePlugins::fixed_tick());
        }
        app.add_plugins(crate::windowed_host::PlatformerHostPlugins);
        crate::provider::ShellComposition::new(
            experience.clone(),
            launcher_route.clone(),
            gameplay_route.clone(),
        )
        .install(app, DeclaredCapabilities(std::sync::Mutex::new(draft.capabilities)));

        // ── Rule 7, ACTUALLY enforced ──
        //
        // The capability plugins have built by now, so the routes they register
        // exist and a declared route can be checked against them.
        //
        // ⚠ This module used to CLAIM rule 7 was enforced "by TYPE, so the empty
        // host is unreachable rather than merely documented". It was not. What
        // was enforced is that a STRING was supplied. The 2026-07-30 blind run
        // declared `gameplay_route("blind_run/gameplay")` naming a route no
        // experience registered, and got a host that built clean, ran 60 ticks
        // and spawned ZERO entities — precisely the failure rule 7 names. The
        // agent found it only because it independently counted entities.
        //
        // An overclaimed guarantee is worse than an absent one: it tells a
        // consumer to stop looking. `ShellRouteCatalog::ids` exists so "a
        // refusal can NAME what was available", which is exactly this refusal.
        let unknown: Vec<(&str, &String)> = {
            let catalog = app
                .world()
                .get_resource::<crate::game_shell::ShellRouteCatalog>();
            match catalog {
                Some(catalog) => [("gameplay", &gameplay_route), ("launcher", &launcher_route)]
                    .into_iter()
                    .filter(|(_, route)| {
                        !catalog.contains(&crate::game_shell::ShellRouteId::from(route.as_str()))
                    })
                    .collect(),
                // No catalog at all means the shell did not install, which rule
                // 5 already covers; do not invent a second diagnosis for it.
                None => Vec::new(),
            }
        };
        if !unknown.is_empty() {
            let available: Vec<String> = app
                .world()
                .get_resource::<crate::game_shell::ShellRouteCatalog>()
                .map(|catalog| catalog.ids().map(str::to_string).collect())
                .unwrap_or_default();
            let available = if available.is_empty() {
                "none — no capability registered any route".to_string()
            } else {
                available.join(", ")
            };
            return Err(CompositionError {
                problems: unknown
                    .into_iter()
                    .map(|(kind, route)| {
                        format!(
                            "the declared {kind} route `{route}` is not registered by any                              mounted capability, so the host would prepare and activate                              NOTHING while appearing to run. Registered routes: {available}"
                        )
                    })
                    .collect(),
            });
        }

        // ── Rule 6 ── assets after the content that fills their catalogs,
        // before the presentation that draws them.
        //
        // In BOTH faces, deliberately. The fixture used to install these only
        // on its windowed path, which made a headless host and a visible host
        // consume different content — and ADR 0032's deletion criteria require
        // the opposite ("headless and visible hosts consume the same
        // prepared-content fingerprint"). A face decides what is DRAWN, not
        // what exists.
        let windowed = matches!(face, Face::Windowed { .. });
        if windowed || game_assets {
            if !windowed {
                // `PlatformerAssetsPlugin` builds sheet handles, so the asset
                // types it addresses have to exist. `DefaultPlugins` brings
                // these; the headless foundation deliberately does not
                // (`MinimalPlugins` plus asset/image/transform/state), so the
                // one face that needs them stated states them here.
                //
                // Found by MIGRATION, not by review: the fixture's asset test
                // hand-rolled a composition with `init_asset::<TextureAtlasLayout>`
                // in it, and why that line was there was recorded nowhere. A
                // rule surviving only inside one test's setup is a rule the
                // next consumer re-derives from a panic.
                app.init_asset::<Image>();
                app.init_asset::<TextureAtlasLayout>();
            }
            app.add_plugins(
                crate::game_assets::PlatformerAssetsPlugin::for_experience(experience)
                    .with_room(draft.room.unwrap_or_default()),
            );
        }
        if windowed {
            app.add_plugins(crate::presentation::PlatformerPresentationPlugin);
        }

        // ── Rule 8 ── one update is one tick.
        if matches!(face, Face::Headless) {
            // ⚠ The two hosts need DIFFERENT dt values, and the difference is
            // one nanosecond.
            //
            // `Time::<Fixed>::from_hz(60.0)` rounds to 16_666_667ns. GGRS wants
            // the truncated 16_666_666ns — `1e9 / SIM_TICK_HZ` in integer
            // arithmetic. Feeding a GGRS host the rounded value costs it real
            // frames: the fixture's own parity walk took 192 `update()` calls
            // to reach a world state the fixed-tick host reached in 180.
            //
            // LEAK, found 2026-07-30 by migrating the fixture onto this
            // builder. The rule existed — in a comment on the fixture's
            // hand-composed rollback app, reading "the frame dt must be the
            // tick dt exactly (integer nanos, no drift)" — and it existed
            // NOWHERE ELSE. A consumer who wrote the obvious thing got a host
            // that runs, simulates correctly, agrees on every checksum, and
            // quietly needs 7% more frames. That is the silent class, and it
            // survived only because one fixture had already been bitten.
            //
            // They differ because they are different clocks, not because one is
            // wrong: the fixed host steps Bevy's `Time<Fixed>` and must match
            // what that resource actually holds, while GGRS derives its own
            // rate from the frame dt.
            let frame_dt = if rollback_unstable {
                std::time::Duration::from_nanos(
                    1_000_000_000u64 / crate::runtime::SIM_TICK_HZ as u64,
                )
            } else {
                app.world().resource::<Time<Fixed>>().timestep()
            };
            app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(frame_dt));
        }

        Ok(())
    }

    /// Decode and publish this game's art even with no display.
    ///
    /// A window implies it — something has to be drawn. Headless does not, and
    /// the default is OFF because preparing art is not free: boot decode was
    /// measured at 627 megapixels / 2.5 GB, which a headless test paying for
    /// nothing it observes should not spend.
    ///
    /// ⚠ It is a POLICY knob rather than a face, and the difference is
    /// load-bearing. The first draft of this builder tied asset preparation to
    /// the windowed face, then moved it to BOTH faces citing ADR 0032's
    /// "headless and visible hosts consume the same prepared-content
    /// fingerprint". That was a misreading — the criterion is about content
    /// identity in slice B, not about which plugins a display-less host
    /// installs — and the fixture's rollback parity test caught it: under GGRS
    /// the sim advances only through session requests, so extra asset frames
    /// are frames the sim does not move, and the two hosts reached the same
    /// world state twelve `update()` calls apart.
    pub fn with_game_assets(mut self) -> Self {
        self.game_assets = true;
        self
    }

    /// Compose the same game under a GGRS rollback host instead of the fixed
    /// tick.
    ///
    /// ⚠ **Not public API.** [`SessionMode`] exposes fixed-step only and that is
    /// the promise; this exists so the engine's own rollback fixtures compose
    /// through the SAME builder as everything else rather than keeping a second
    /// hand-ordered path alive beside it.
    ///
    /// Making it a supported knob is its own slice with its own acceptance
    /// tests, because it is a far larger promise than a clock: frozen schema,
    /// complete authoritative baseline, stable participants, deterministic
    /// activation, lifecycle rebasing, confirmation boundaries. See the
    /// campaign's Deferred section.
    #[doc(hidden)]
    pub fn unstable_rollback_session(mut self) -> Self {
        self.rollback_unstable = true;
        self
    }

    fn with_face(face: Face) -> Self {
        Self {
            face,
            session: SessionMode::FixedStep,
            rollback_unstable: false,
            game_assets: false,
            manifests: Vec::new(),
            draft: ModuleDraft::default(),
        }
    }
}

/// `DefaultPlugins`, configured. Rules 2, 3 and 4.
fn install_windowed_foundation(app: &mut App, title: &str, gpu: bool) {
    use bevy::window::{ExitCondition, Window, WindowPlugin};

    let plugins = DefaultPlugins
        .set(bevy::asset::AssetPlugin {
            // Rule 2: the engine knows where its own content is.
            file_path: crate::asset_manager::actors_desktop_asset_root(),
            ..Default::default()
        })
        .set(WindowPlugin {
            primary_window: gpu.then(|| Window {
                title: title.to_string(),
                ..Default::default()
            }),
            exit_condition: if gpu {
                ExitCondition::OnAllClosed
            } else {
                ExitCondition::DontExit
            },
            close_when_requested: gpu,
            ..Default::default()
        });

    if gpu {
        app.add_plugins(plugins);
    } else {
        // Rule 3. A `backends: None` renderer has no RenderApp, and
        // process-global logging / Ctrl+C handlers belong to an executable
        // rather than to a manually stepped fixture.
        use bevy::render::settings::{RenderCreation, WgpuSettings};
        use bevy::render::RenderPlugin;
        app.add_plugins(
            plugins
                .disable::<bevy::log::LogPlugin>()
                .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
                .disable::<bevy::core_pipeline::CorePipelinePlugin>()
                .disable::<bevy::gizmos_render::GizmoRenderPlugin>()
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        backends: None,
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .disable::<bevy::winit::WinitPlugin>(),
        );
    }

    // Rule 4: after Bevy's StatesPlugin exists, before the sim plugins whose
    // run conditions read the state.
    crate::engine::init_engine_states(app);
}
