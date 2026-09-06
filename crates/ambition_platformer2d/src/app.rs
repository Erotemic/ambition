//! High-level platformer application composition.
//!
//! Consumers state policy; this module owns engine installation order. In
//! particular, asset sources precede `AssetPlugin`, engine state precedes engine
//! plugins, engine plugins precede host/shell plugins, and platformer assets are
//! prepared after content registration but before presentation.
//!
//! [`PlatformerApp::try_build`] refuses missing or unserved initial routes. Manual
//! stepping pins frame dt to the configured simulation tick. This module sequences
//! composition only; gameplay behavior remains in the domain crates.
//!
//! ```ignore
//! use crate::app::prelude::*;
//!
//! PlatformerApp::windowed("My Game")
//!     .mount(MyModule::default())
//!     .run();
//! ```

use bevy::app::Plugins;
use bevy::prelude::*;

use crate::world::rooms::RoomMetadata;

/// Curated imports for a game's `main`.
///
/// This domain prelude avoids exposing the broader implementation topology of
/// `crate::prelude`.
pub mod prelude {
    pub use super::{
        host_status, AssetSource, CompositionError, Display, GameModule, HostStatus, ModuleDraft,
        ModuleManifest, PlatformerApp, SessionMode, StartAt, EMPTY_CHARACTER_ROSTER_RON,
        MINIMAL_CHARACTER_ROSTER_RON,
    };
    pub use bevy::prelude::App;

    /// The room types this module's own signatures demand.
    ///
    /// A prelude that omits the types its own signatures require is a prelude
    /// that sends its reader into `crates/`.
    pub use crate::world::rooms::{RoomMetadata, RoomSpec};
}

/// Did my game actually start?
///
/// The one question a consumer could not ask. Every consumer would have invented that same smoke
/// test, badly.
///
/// [`HostStatus::Running`] deliberately carries `prepared`, because "a route is
/// active" and "a session was prepared for it" are different facts and the gap
/// between them is exactly the empty host: an earlier draft of the fixture's
/// headless binary "ran" 120 ticks of a host that had activated nothing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HostStatus {
    /// No shell router exists. The engine was never installed into this `App`.
    NotComposed,
    /// Composed, but the router has not initialized yet — usually means no
    /// `update()` has run.
    Initializing,
    /// A route is being prepared. Normal for a few frames after boot; a host
    /// stuck here is a preparation that never completed.
    Activating { route: String },
    /// Why routing refused this host.
    ///
    /// Stored explicitly so headless and no-log consumers can diagnose failed activation.
    Refused { reasons: Vec<String> },
    /// A route is live.
    ///
    ///  `prepared == false` is the quiet failure: the router is pointing at a
    /// route and no prepared session sits behind it, so the world is empty and
    /// nothing says why.
    Running {
        route: String,
        experience: String,
        prepared: bool,
    },
}

impl HostStatus {
    /// Live AND backed by a prepared session — the state a consumer means when
    /// it asks "did it start?".
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running { prepared: true, .. })
    }

    /// Routing refused this host. A poll loop should STOP rather than spin.
    ///
    /// The distinction `Activating` could not make. Spinning 600 ticks on a
    /// decision the engine reached on tick 3 is not patience, it is a consumer
    /// with no way to know.
    pub fn is_refused(&self) -> bool {
        matches!(self, Self::Refused { .. })
    }

    /// Why routing refused, if it did.
    pub fn refusal(&self) -> &[String] {
        match self {
            Self::Refused { reasons } => reasons,
            _ => &[],
        }
    }

    /// The active route, if any.
    pub fn route(&self) -> Option<&str> {
        match self {
            Self::Activating { route } | Self::Running { route, .. } => Some(route),
            _ => None,
        }
    }
}

/// Read [`HostStatus`] off a composed `App`.
///
/// A read-model over what the shell already holds — it computes nothing and
/// stores nothing, so it cannot disagree with the router about what is running.
pub fn host_status(app: &App) -> HostStatus {
    let Some(router) = app.world().get_resource::<crate::game_shell::ShellRouter>() else {
        return HostStatus::NotComposed;
    };
    if let Some(active) = router.active.as_ref() {
        return HostStatus::Running {
            route: active.route_id.as_str().to_string(),
            experience: active.experience_id.as_str().to_string(),
            prepared: active.prepared_session.is_some(),
        };
    }
    if let Some(failures) = app
        .world()
        .get_resource::<crate::game_shell::ShellFailureLog>()
    {
        if !failures.is_empty() {
            return HostStatus::Refused {
                reasons: failures.reasons().to_vec(),
            };
        }
    }
    if let Some(pending) = router.pending.as_ref() {
        return HostStatus::Activating {
            route: pending.route_id.as_str().to_string(),
        };
    }
    HostStatus::Initializing
}

/// A named asset tree the game owns, layered over the engine's own.
///
/// Carried as data rather than applied as a call, because the moment it
/// must be applied is a moment only the engine knows (rule 1) — and because a
/// declaration can be CHECKED in the one composition shape where the engine
/// cannot apply it. See [`PlatformerApp::install_into`].
#[derive(Clone, Debug)]
pub struct AssetSource {
    name: String,
    //  read by the native installer, which builds a layered filesystem reader
    // from it. A browser build declares the same source and reads it over HTTP
    // from the page origin, so it never consults the root — but the DECLARATION
    // must still carry it, which is the whole point of checking declarations in
    // a composition shape the engine cannot apply.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
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
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SessionMode {
    #[default]
    FixedStep,
}

/// What a module needs the engine to know before the Bevy foundation
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
/// `FnOnce`, drained through a `Mutex` by [`DeclaredCapabilities`]. A bound that excludes the API's
/// own types is the API being wrong, not the caller.
type CapabilityInstaller = Box<dyn FnOnce(&mut App) + Send + 'static>;

/// A character roster with nothing in it.
///
/// A value obtainable only by brute-forcing diagnostics is a value the engine knows and would not
/// say.
pub const EMPTY_CHARACTER_ROSTER_RON: &str =
    "(brain_presets: {}, action_set_presets: {}, characters: {})";

/// A playable experience, as a value.
///
/// A game could COMPOSE through the SDK and still could not DECLARE what it was.
///
/// Outlander names all four for other reasons too, so with one consumer this
/// hole was invisible. That is the consumer matrix earning its place.
struct ExperienceDefinition {
    label: String,
    description: String,
    starting_character: String,
    rooms: Vec<crate::world::rooms::RoomSpec>,
    starting_room: String,
}

/// A roster with ONE character — the case a game actually starts from.
///
///  Published because [`EMPTY_CHARACTER_ROSTER_RON`] solved the case nobody needs. It gave up
/// after four cycles and opened a fixture — which is the SDK's acceptance test failing by the
/// SDK's own remedy.
///
/// The enum-valued fields, since those are the ones no error message will tell
/// you:
///
/// * `tier` — `MainHall` for an ordinary character.
/// * `body_kind` — `Standard`.
/// * `composition` — `None` unless the body is assembled from parts.
/// * `move_style` — `Walk`.
/// * a brain preset value — `StandStill` for a character that does not act.
pub const MINIMAL_CHARACTER_ROSTER_RON: &str = r#"(
    brain_presets: { "still": StandStill },
    action_set_presets: {
        "walk_only": (
            move_style: Walk,
            melee: None,
            ranged: None,
            special: None,
        ),
    },
    characters: {
        "my_hero": (
            display_name: "My Hero",
            spritesheet: "my_hero.png",
            manifest: "my_hero_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "still",
            default_action_set: "walk_only",
            tags: ["player"],
        ),
    },
)"#;

/// One experience declaration owned by one module. A composition may contain
/// multiple independent drafts before they are validated and published.
struct ExperienceDraft {
    /// Which module declared it, so a collision can name both sides.
    owner: String,
    id: String,
    launcher_route: Option<String>,
    gameplay_route: Option<String>,
    room: Option<RoomMetadata>,
    characters: Option<CharacterContent>,
    definition: Option<ExperienceDefinition>,
    declared_silence: bool,
}

/// What a module says about its cast.
///
/// An `Option<CharacterContent>` rather than a bare `Option<&str>` so that
/// "authored a roster", "authored none ON PURPOSE" and "said nothing" are three
/// distinguishable states. The third is the one that must fail.
enum CharacterContent {
    Ron(&'static str),
    DeclaredEmpty,
}
/// One module-provided rollback registration, applied at install time.
///
/// Boxed rather than typed because the registrations are heterogeneous — each
/// names a different component — and a `ModuleDraft` must hold them together.
type RollbackContribution = Box<dyn FnOnce(&mut App) + Send + Sync>;

/// The inert accumulation a module writes into.
///
/// Nothing here is live when `define` returns. ADR 0032 decision 1: the
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
    /// Every declared experience, in declaration order. The FIRST is the host's
    /// initial route — a host starts somewhere, and "the first one mounted"
    /// is a rule a consumer can predict without a second knob to set.
    experiences: Vec<ExperienceDraft>,
    /// Which experience subsequent calls apply to.
    current: Option<usize>,
    capabilities: Vec<CapabilityInstaller>,
    /// What the mounted capabilities need REWOUND, declared by the module that
    /// mounts them. See [`ModuleDraft::requires_rollback`].
    required_rollback: Vec<&'static [ambition_platformer2d_core::snapshot::RequiredRollbackState]>,
    /// The rollback registrations the module CONTRIBUTES, to satisfy the
    /// requirements above. See [`ModuleDraft::provides_rollback`].
    provided_rollback: Vec<RollbackContribution>,
    /// The SEMANTIC ACTIONS the mounted capabilities contribute. See
    /// [`ModuleDraft::actions`].
    actions: Vec<&'static [ambition_input::SemanticActionDef]>,
    conflicts: Vec<String>,
}

impl ModuleDraft {
    /// Begin declaring an experience. Subsequent calls apply to it.
    ///
    /// A composition may declare several distinct experience ids; redeclaring an
    /// id is a conflict. Secondary experiences currently inherit primary asset
    /// policy for music catalogs, SFX bank publication, and startup room theme.
    /// TODO(experience-assets): virtualize those asset policies per experience.
    pub fn experience(&mut self, id: impl Into<String>) -> &mut Self {
        let id = id.into();
        let owner = self.defining.clone();
        if let Some(existing) = self.experiences.iter().find(|e| e.id == id) {
            self.conflicts.push(format!(
                "two modules declare the experience `{id}`: `{}` and `{owner}`. \
                 Experiences are keyed by id, so two modules may coexist only \
                 with distinct ids.",
                existing.owner
            ));
            return self;
        }
        self.experiences.push(ExperienceDraft {
            owner,
            id,
            launcher_route: None,
            gameplay_route: None,
            room: None,
            characters: None,
            definition: None,
            declared_silence: false,
        });
        self.current = Some(self.experiences.len() - 1);
        self
    }

    /// Where the host starts, and what `QuitToHome` resolves to.
    pub fn launcher_route(&mut self, route: impl Into<String>) -> &mut Self {
        let route = route.into();
        self.on_current("launcher route", move |e| {
            e.launcher_route = Some(route);
        })
    }

    /// The route this experience registers for its session. Required — rule 7.
    pub fn gameplay_route(&mut self, route: impl Into<String>) -> &mut Self {
        let route = route.into();
        self.on_current("gameplay route", move |e| {
            e.gameplay_route = Some(route);
        })
    }

    /// Declare this experience's character roster, as catalog RON.
    pub fn characters(&mut self, catalog_ron: &'static str) -> &mut Self {
        self.on_current("characters", move |e| {
            e.characters = Some(CharacterContent::Ron(catalog_ron));
        })
    }

    /// Declare, explicitly, that this experience authors no characters.
    ///
    ///  Instead of a silent default. `PlatformerAssetsPlugin` refuses to
    /// substitute an empty catalog — *"silently substituting an empty catalog
    /// is how a game ships with its bosses drawn as the fallback body and
    /// nobody notices"* — and that judgement is right. What this changes is WHO
    /// says the catalog is empty. See [`EMPTY_CHARACTER_ROSTER_RON`].
    pub fn no_characters(&mut self) -> &mut Self {
        self.on_current("characters", |e| {
            e.characters = Some(CharacterContent::DeclaredEmpty);
        })
    }

    /// The room whose metadata picks block and biome art at `Startup`.
    pub fn room(&mut self, room: RoomMetadata) -> &mut Self {
        self.on_current("room", move |e| e.room = Some(room))
    }

    /// Declare, explicitly, that this experience authors no sound.
    ///
    /// Preparation REFUSES an experience whose provider registered no audio
    /// fragment, so silence has always been mandatory paperwork — there was
    /// simply no word for it on the public surface.
    pub fn no_audio(&mut self) -> &mut Self {
        self.on_current("audio", |e| e.declared_silence = true)
    }

    /// Declare the playable content of this experience.
    ///
    /// The engine assembles the prepared source and installs the authoring
    /// through the same `PlatformerExperienceAuthoring` seam a provider plugin
    /// would use — the draft removes the boilerplate, not the authority.
    pub fn playable(
        &mut self,
        label: impl Into<String>,
        description: impl Into<String>,
        starting_character: impl Into<String>,
        starting_room: impl Into<String>,
        rooms: Vec<crate::world::rooms::RoomSpec>,
    ) -> &mut Self {
        let definition = ExperienceDefinition {
            label: label.into(),
            description: description.into(),
            starting_character: starting_character.into(),
            rooms,
            starting_room: starting_room.into(),
        };
        self.on_current("playable content", move |e| {
            e.definition = Some(definition);
        })
    }

    /// Declare a capability. Installed by the engine, in its own order.
    ///
    /// Capabilities belong to the COMPOSITION rather than to one experience: a
    /// plugin installs systems into an `App`, and an App has one schedule.
    ///
    ///  PROVISIONAL, and deliberately under-promised. A capability is an
    /// opaque closure: the engine can run it, and cannot ask it what it
    /// provides, needs, or conflicts with. So there is no dependency
    /// resolution, no conflict detection, and no claim that two modules
    /// declaring overlapping capabilities compose in either order — today they
    /// install in declaration order and whichever wrote last wins, which is
    /// Bevy's own behaviour and not a contract this API is defending.
    pub fn capability<M>(&mut self, plugin: impl Plugins<M> + Send + 'static) -> &mut Self {
        self.capabilities.push(Box::new(move |app: &mut App| {
            app.add_plugins(plugin);
        }));
        self
    }

    /// Declare rollback state required by capabilities mounted in this module.
    ///
    /// Requirements are checked only when a rollback registry exists. Missing
    /// registrations make rollback composition fail rather than desynchronize.
    pub fn requires_rollback(
        &mut self,
        required: &'static [ambition_platformer2d_core::snapshot::RequiredRollbackState],
    ) -> &mut Self {
        self.required_rollback.push(required);
        self
    }

    /// Provide registration satisfying a declared rollback requirement.
    ///
    /// `owner` and `name` must exactly match the requirement. The module hosts
    /// the registration so the capability itself remains backend-independent.
    #[cfg(feature = "rollback")]
    pub fn provides_rollback<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        projection: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: bevy::prelude::Component<Mutability = bevy::ecs::component::Mutable> + Clone,
    {
        self.provided_rollback.push(Box::new(move |app: &mut App| {
            use crate::rollback::AmbitionRollbackApp;
            app.rollback_component_clone_probed::<T>(owner, name, projection);
        }));
        self
    }

    /// Declare the semantic actions a mounted capability contributes.
    ///
    /// `Platformer2dInputActionMonolith` is a closed leafwing enum a capability cannot extend, so
    /// the OPEN half is `crate::input::SemanticActionId` and the registry
    /// that holds it. This is where a capability's actions reach a composition:
    ///
    /// ```ignore
    /// module.capability(capability_demo::PulsePlugin::default())
    ///       .actions(&[capability_demo::PULSE_ACTION]);
    /// ```
    ///
    /// The engine's own vocabulary is installed automatically, so a prompt, a
    /// help screen or a rebind UI can ask one `ActionRegistry` what may be
    /// pressed in a context and get the game's actions beside the engine's.
    ///
    ///  two owners for one action id is a composition REFUSAL, for the same
    /// reason an ambiguous content schema is: letting it through means the
    /// winner is decided by iteration order.
    pub fn actions(&mut self, actions: &'static [ambition_input::SemanticActionDef]) -> &mut Self {
        self.actions.push(actions);
        self
    }

    /// Apply an edit to the experience currently being declared.
    ///
    /// The explicit association prevents routes from attaching to the wrong experience or none.
    fn on_current(&mut self, what: &str, edit: impl FnOnce(&mut ExperienceDraft)) -> &mut Self {
        let owner = self.defining.clone();
        match self.current.and_then(|i| self.experiences.get_mut(i)) {
            Some(experience) => edit(experience),
            None => self.conflicts.push(format!(
                "`{owner}` declared a {what} before naming an experience; call \
                 `experience(id)` first so the {what} has an owner"
            )),
        }
        self
    }
}

/// A game module: what it needs before the foundation, and what it declares.
///
/// Both methods take `&self` so modules may carry configuration:
///
/// ```ignore
/// PlatformerApp::windowed("Sanic").mount(SanicModule { difficulty: Hard })
/// ```
pub trait GameModule {
    fn manifest(&self) -> ModuleManifest;

    /// Accumulate into the draft. Never touches `App`.
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
    /// Which pass produced these, and therefore what has NOT been checked yet.
    pub stage: CompositionStage,
}

/// Which pass of composition refused.
///
/// ADR 0032's promise — *"a draft yields one build error listing every conflict
/// in the experience"* — is true WITHIN a pass and cannot be true across them:
/// the second pass's checks need the capabilities BUILT (a module may
/// legitimately register its roster through one), so a draft that does not
/// assemble cannot be asked whether its roster exists.
///
/// Fix it, rebuild for ten minutes, discover the next one.
///
/// Naming the stage does not merge the passes — that is not possible — but it
/// turns a funnel into a STATED funnel, which is the difference between "this is
/// everything" and "this is everything I could see from here".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionStage {
    /// The DRAFT was refused: nothing was assembled, so no capability-dependent
    /// check has run.
    Declaration,
    /// The draft was accepted and the capabilities built; this is a fact about
    /// the assembled app, and everything before it passed.
    Assembly,
}

impl CompositionError {
    /// A refusal from the declaration pass. Later checks did NOT run.
    pub fn declaration(problems: Vec<String>) -> Self {
        Self {
            problems,
            stage: CompositionStage::Declaration,
        }
    }

    /// A refusal from the assembly pass. Everything before it passed.
    pub fn assembly(problems: Vec<String>) -> Self {
        Self {
            problems,
            stage: CompositionStage::Assembly,
        }
    }
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
        if self.stage == CompositionStage::Declaration {
            writeln!(
                formatter,
                "  (these are the DECLARATION's problems. The capability-dependent \
                 checks — routes, roster — have not run yet, so fixing these may \
                 reveal more.)"
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for CompositionError {}

/// Where the host lands when it starts.
///
///  Two policies, because there are two real hosts and the builder offered
/// only one. against `game/ambition_app`: it boots into a
/// LAUNCHER listing every registered experience, and had to configure that by
/// hand — registering a shell experience as its home route and writing
/// `ShellHostConfiguration.spec` itself — because `ShellComposition` boots into
/// the primary's gameplay route and nothing else.
///
/// That was the last piece of host composition a real consumer still assembled
/// for itself, and it is the third time the same shape has appeared: the SDK
/// expressed one option (one face, one experience, one start policy) while the
/// shipped host needed another.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StartAt {
    /// Straight into the first mounted experience. Right for a single game.
    #[default]
    PrimaryGameplay,
    /// Into a launcher listing every mounted experience. Right for a host that
    /// ships more than one.
    Launcher,
}

/// How a windowed face meets the GPU and the desktop.
///
/// Three, because "windowed" was two questions wearing one boolean: whether
/// there is a real wgpu backend, and whether there is a window on it. The
/// combination that had no name is the one a capture tool needs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Display {
    /// A real backend presenting to a real window. The shipped game.
    #[default]
    Window,
    /// A real backend with NO window: frames are rendered and can be read back,
    /// nothing is presented.
    ///
    /// This is the face an offscreen capture needs, and it is an ENGINE fact
    /// rather than a tool's, for the same reason [`Self::NoGpu`] is: disabling
    /// `winit` also removes the app RUNNER, so an offscreen app must be stepped
    /// by its caller. A consumer rediscovering that is the leak, not the
    /// convenience.
    Offscreen,
    /// No backend at all — the render graph is built against `backends: None`.
    /// For GPU-less CI that still has to assert something was DRAWN. There is
    /// no `RenderApp`, so nothing can be read back.
    NoGpu,
}

impl Display {
    /// Is there a real wgpu backend behind this face?
    fn has_backend(self) -> bool {
        !matches!(self, Self::NoGpu)
    }

    /// Is anything presented to a desktop window?
    fn has_window(self) -> bool {
        matches!(self, Self::Window)
    }
}

/// How the game meets a display.
#[derive(Clone, Debug)]
enum Face {
    Headless,
    Windowed { title: String, display: Display },
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

/// The composition. State policy; the engine states the order.
pub struct PlatformerApp {
    face: Face,
    session: SessionMode,
    /// Internal rollback composition without making rollback a public [`SessionMode`].
    ///
    /// This keeps one composition authority while avoiding a public mode commitment.
    rollback_participants: Option<usize>,
    game_assets: bool,
    start_at: StartAt,
    manifests: Vec<ModuleManifest>,
    draft: ModuleDraft,
}

impl PlatformerApp {
    /// A game that opens a window.
    pub fn windowed(title: impl Into<String>) -> Self {
        Self::with_face(Face::Windowed {
            title: title.into(),
            display: Display::Window,
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
    pub fn without_gpu(self) -> Self {
        self.set_display(Display::NoGpu, "without_gpu")
    }

    /// Render for real, present to nothing.
    ///
    /// The face an offscreen capture runs on: a full render graph on a real
    /// backend, with no window and therefore no `winit` — which also means no
    /// app runner, so the caller steps the app itself. That is what makes a
    /// burst of frames exactly as long as the caller says.
    pub fn offscreen(self) -> Self {
        self.set_display(Display::Offscreen, "offscreen")
    }

    fn set_display(mut self, display: Display, verb: &str) -> Self {
        match &mut self.face {
            Face::Windowed { display: slot, .. } => *slot = display,
            Face::Headless => self.draft.conflicts.push(format!(
                "`{verb}` needs a windowed face; headless has no render graph"
            )),
        }
        self
    }

    /// Mount a module: fold in its manifest, and let it define itself.
    ///
    /// `define` runs now, into an inert draft. Nothing is live.
    ///
    /// A module may only modify an experience it declared during its own `define`; anything
    /// cross-module has to name its target, and nothing does yet.
    pub fn mount(mut self, module: impl GameModule) -> Self {
        let manifest = module.manifest();
        self.draft.defining = manifest.id().to_string();
        self.draft.current = None;
        module.define(&mut self.draft);
        self.draft.defining.clear();
        self.draft.current = None;
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
    ///  A failed installation is FATAL to that `App`, and retrying is not a
    /// supported operation. Everything decidable from the declaration is
    /// decided before the first mutation — a `CompositionStage::Declaration`
    /// refusal has touched nothing, and there is a test that asks the `App`
    /// rather than trusting the stage. An `Assembly` refusal is the other case
    /// by definition: those checks need the built App, so by the time one fails,
    /// asset sources, the Bevy foundation and some capabilities are installed.
    /// Drop it and compose again.
    ///
    /// This is a stated limit rather than a plan.
    ///
    ///  This form cannot honor rule 1, and says so instead of pretending.
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
            rollback_participants,
            game_assets,
            start_at,
            manifests,
            draft,
        } = self;

        let sources: Vec<AssetSource> = manifests
            .into_iter()
            .flat_map(|manifest| manifest.asset_sources)
            .collect();

        let mut problems = draft.conflicts.clone();
        if draft.experiences.is_empty() {
            problems.push("no module declared an experience id".into());
        }
        for experience in &draft.experiences {
            if experience.gameplay_route.is_none() {
                // Rule 7, as a type rather than as a comment.
                problems.push(format!(
                    "experience `{}` declared no gameplay route; a host that names \
                     none prepares and activates nothing, and does it silently",
                    experience.id
                ));
            }
        }
        // Only the PRIMARY needs a launcher: it is the host's home, and a host
        // has one. Requiring one per experience would be inventing a rule the
        // shell does not have.
        if let Some(primary) = draft.experiences.first() {
            if primary.launcher_route.is_none() {
                problems.push(format!(
                    "experience `{}` is first and therefore the host's home, but \
                     declared no launcher route; `QuitToHome` would have nowhere \
                     to land",
                    primary.id
                ));
            }
        }
        let prepares_art = matches!(face, Face::Windowed { .. }) || game_assets;
        debug_assert!(
            !matches!(face, Face::Windowed { display, .. } if display.has_backend() && !prepares_art),
            "a face with a real backend always prepares the art it would draw"
        );
        // ── Slice H ── a facade built without the `ambition_render` capability
        // has no presentation to install, and a composition that prepares art
        // must be REFUSED here rather than silently drawing nothing. This
        // refusal was probed red: `minimal_game` with the capability removed
        // fails its windowed tests on exactly this message.
        #[cfg(not(feature = "ambition_render"))]
        if prepares_art {
            problems.push(
                "this composition prepares art (a windowed face, or `with_game_assets`), \
                 but `ambition_platformer2d` was built without the `ambition_render` capability. \
                 Enable the `ambition_render` feature (on by default via \
                 `all_capabilities`), or compose headless without `with_game_assets`."
                    .to_string(),
            );
        }
        for experience in &draft.experiences {
            if let Some(definition) = experience.definition.as_ref() {
                if definition.rooms.is_empty() {
                    problems.push(format!(
                        "experience `{}` is playable but declares no rooms",
                        experience.id
                    ));
                }
            }
        }
        if !sources.is_empty() && app.is_plugin_added::<bevy::asset::AssetPlugin>() {
            for source in &sources {
                problems.push(format!(
                    "asset source `{}://` was declared, but `AssetPlugin` has already \
                     built in this App and Bevy seals its sources there. Let \
                     `PlatformerApp::build` own the whole stack, or register the \
                     source before adding `DefaultPlugins`.",
                    source.name
                ));
            }
        }
        // ── PREPARE the declared content, before anything is installed ──
        //
        // This is the practical half: everything that can be decided from the declaration is
        // decided here, and what remains in assembly genuinely needs the built App to answer —
        // a fragment conflict is a fact about what is already registered.
        let mut prepared_casts: Vec<(String, PreparedCast)> = Vec::new();
        for experience in &draft.experiences {
            match prepare_declared_cast(experience) {
                Ok(Some(prepared)) => prepared_casts.push((experience.id.clone(), prepared)),
                Ok(None) => {}
                Err(reasons) => problems.extend(reasons),
            }
        }

        if !problems.is_empty() {
            return Err(CompositionError {
                stage: CompositionStage::Declaration,
                problems,
            });
        }

        // ── Rule 1 ── before any AssetPlugin, in every face.
        //
        //  desktop only. `consumer_source` LAYERS a game's asset tree over the
        // engine's by reading both from disk, and `#[cfg(not(wasm32))]` says so
        // at its definition — on the web there is no engine root to layer under,
        // because the served bundle already IS the merged tree. The refusal
        // below is not a smaller version of the desktop behaviour; it is the
        // statement that a declaration nobody can honour must not pass silently.
        #[cfg(not(target_arch = "wasm32"))]
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
        #[cfg(target_arch = "wasm32")]
        if let Some(source) = sources.first() {
            return Err(CompositionError {
                stage: CompositionStage::Declaration,
                problems: vec![format!(
                    "asset source `{}://` layers a game tree over the engine's from                      DISK, which the web build has no way to do — the served bundle                      is already one merged tree. Ship the game's assets inside it                      rather than declaring a source.",
                    source.name
                )],
            });
        }

        // ── Rules 2, 3, 4 ── the Bevy foundation.
        match &face {
            Face::Headless => crate::engine::add_headless_foundation(app),
            Face::Windowed { title, display } => install_windowed_foundation(app, title, *display),
        }

        // ── The cast and the silence, per experience, through the seams a
        // provider plugin would have used.
        //
        // Both fragments were BUILT in the declaration pass; what is left here is
        // registration, and only the roster's can still fail — on a conflict with
        // a fragment some capability registered, which is a fact about the built
        // App and cannot be known earlier.
        for (id, prepared) in prepared_casts {
            let PreparedCast { characters, audio } = prepared;
            if let Some(fragment) = characters {
                use crate::characters::actor::character_catalog::registry::CharacterCatalogAppExt as _;
                app.try_register_character_catalog_fragment(fragment)
                    .map_err(|error| CompositionError {
                        stage: CompositionStage::Assembly,
                        problems: vec![format!(
                            "the character roster declared by `{id}` conflicts with one \
                             already registered: {error}"
                        )],
                    })?;
            }
            if let Some(fragment) = audio {
                use crate::audio::catalog::AudioCatalogAppExt as _;
                app.register_audio_catalog_fragment(fragment);
            }
        }

        // A starting character nobody authored: refuse at build, do not hang.
        for experience in &draft.experiences {
            let Some(definition) = experience.definition.as_ref() else {
                continue;
            };
            let roster = app
                .world()
                .get_resource::<crate::characters::actor::character_catalog::CharacterCatalog>();
            if let Some(roster) = roster {
                if roster.get(definition.starting_character.as_str()).is_none() {
                    let known: Vec<&str> = roster.iter().map(|(id, _)| id.as_str()).collect();
                    let known = if known.is_empty() {
                        "none — the declared roster is empty".to_string()
                    } else {
                        known.join(", ")
                    };
                    return Err(CompositionError {
                        stage: CompositionStage::Assembly,
                        problems: vec![format!(
                            "`{}` starts as character `{}`, which no declared roster \
                             contains, so the host would prepare NOTHING and wait \
                             forever. Characters available: {known}",
                            experience.id, definition.starting_character
                        )],
                    });
                }
            }
        }

        // ── Rule 5 ── engine, then host, then shell.
        #[cfg(feature = "rollback")]
        if let Some(participants) = rollback_participants {
            app.add_plugins(crate::rollback::RollbackEnginePlugin);
            // The declaration travels with the composition, so a restart reads
            // the count the game stated rather than re-sampling live devices.
            app.insert_resource(crate::rollback::DeclaredParticipants(participants));
            // The public builder owns session start after construction, so keep
            // the backend's local maintainer from racing it.
            let mut policy = app
                .world()
                .get_resource::<crate::rollback::local_session::LocalSessionPolicy>()
                .copied()
                .unwrap_or_default();
            policy.autostart = false;
            app.insert_resource(policy);
        } else {
            app.add_plugins(crate::engine::PlatformerEnginePlugins::fixed_tick());
        }
        #[cfg(not(feature = "rollback"))]
        app.add_plugins(crate::engine::PlatformerEnginePlugins::fixed_tick());
        app.add_plugins(crate::windowed_host::PlatformerHostPlugins);

        // Every experience's authoring, lowered into one capability bundle so
        // installation order is the engine's rather than the mount order's.
        let mut capabilities = draft.capabilities;
        for experience in &draft.experiences {
            if let Some(installer) = experience_installer(experience) {
                capabilities.push(installer);
            }
        }

        let primary = draft
            .experiences
            .first()
            .expect("checked above: at least one experience");
        let primary_id = primary.id.clone();
        let primary_launcher = primary
            .launcher_route
            .clone()
            .expect("checked above: the primary declares a launcher");
        let primary_gameplay = primary
            .gameplay_route
            .clone()
            .expect("checked above: every experience declares a gameplay route");
        let declared_routes: Vec<(String, String)> = draft
            .experiences
            .iter()
            .map(|e| {
                (
                    e.id.clone(),
                    e.gameplay_route.clone().expect("checked above"),
                )
            })
            .collect();

        crate::provider::ShellComposition::new(
            primary_id,
            primary_launcher.clone(),
            primary_gameplay,
        )
        .install(
            app,
            DeclaredCapabilities(std::sync::Mutex::new(capabilities)),
        );

        // ── The start policy ──
        //
        // Applied AFTER `ShellComposition` rather than instead of it: the
        // composition also installs the frontend audio context, the route
        // table and the experience plugins, and forking it for one field would
        // be two composition paths for one difference.
        if matches!(start_at, StartAt::Launcher) {
            use crate::game_shell::{
                ShellHostConfiguration, ShellHostSpec, ShellLaunchCatalog, ShellRouteCatalog,
                ShellRouteSpec,
            };
            app.world_mut()
                .resource_mut::<ShellRouteCatalog>()
                .register(ShellRouteSpec::new(
                    primary_launcher.clone(),
                    ShellLaunchCatalog::basic_experience_id(),
                ));
            app.world_mut()
                .resource_mut::<ShellHostConfiguration>()
                .spec = Some(ShellHostSpec::new(
                primary_launcher.clone(),
                primary_launcher.clone(),
            ));
        }

        // ── The semantic action vocabulary this composition understands ──
        //
        // Engine actions plus whatever the modules declared, as ONE registry a
        // prompt / help screen / rebind UI can ask. Built here rather than by a
        // capability because a registry belongs to the composition — the same
        // rule the content compiler's `SchemaRegistry` follows.
        {
            let mut registry = ambition_input::ActionRegistry::with_engine_actions();
            let mut conflicts = Vec::new();
            for declared in &draft.actions {
                for action in declared.iter() {
                    if let Err(conflict) = registry.register(action.clone()) {
                        conflicts.push(conflict.to_string());
                    }
                }
            }
            if !conflicts.is_empty() {
                return Err(CompositionError {
                    stage: CompositionStage::Assembly,
                    problems: conflicts,
                });
            }
            app.insert_resource(ambition_input::InstalledActions(registry));
        }

        // ── What the mounted capabilities need REWOUND ──
        //
        //  Checked AFTER the capabilities built, for the same reason the cast
        // check below is: a capability registers its rollback state while it
        // installs, so asking the draft would always report everything missing.
        //
        //  FIRST among the post-capability checks, and deliberately: it is the
        // only one here whose failure is SILENT at runtime. An unregistered
        // route refuses to activate immediately and loudly; missing rollback
        // state produces a desync much later and far from its cause, so a
        // composition carrying both should hear about this one.
        //
        //  Only when this composition ASKED FOR a rollback session.
        //
        // What distinguishes them is the DECLARATION — `rollback(n)` — not the presence of a
        // registry the engine installs either way. APPLY WHAT THE MODULE PROVIDED, then check
        // what it required.
        if rollback_participants.is_some() {
            for contribute in draft.provided_rollback {
                contribute(app);
            }
        }
        let rollback_registry = rollback_participants.is_some().then(|| {
            app.world()
                .get_resource::<crate::runtime::rollback::RollbackRegistry>()
        });
        if let Some(Some(registry)) = rollback_registry {
            let missing: Vec<String> = draft
                .required_rollback
                .iter()
                .flat_map(|required| registry.missing_required_state(required))
                .map(|req| {
                    format!(
                        "capability `{}` requires rollback state `{}`, which nothing \
                         registered — {}",
                        req.owner, req.name, req.why
                    )
                })
                .collect();
            if !missing.is_empty() {
                return Err(CompositionError {
                    stage: CompositionStage::Assembly,
                    problems: missing,
                });
            }
        }

        // ── Rule 7, ACTUALLY enforced, for EVERY declared experience ──
        //
        // The capability plugins have built, so the routes they register exist.
        // A declared route that no capability registers means the host would
        // prepare and activate NOTHING while appearing to run.
        {
            let catalog = app
                .world()
                .get_resource::<crate::game_shell::ShellRouteCatalog>()
                .map(|catalog| {
                    let ids: Vec<String> = catalog.ids().map(str::to_string).collect();
                    ids
                });
            if let Some(available) = catalog {
                let missing: Vec<String> = declared_routes
                    .iter()
                    .filter(|(_, route)| !available.iter().any(|known| known == route))
                    .map(|(experience, route)| {
                        format!(
                            "experience `{experience}` declared gameplay route \
                             `{route}`, which no mounted capability registers — call \
                             `playable(..)`, or install a capability that registers it"
                        )
                    })
                    .chain(
                        (!available.iter().any(|known| known == &primary_launcher)).then(|| {
                            format!(
                                "the launcher route `{primary_launcher}` is not \
                                     registered by any mounted capability"
                            )
                        }),
                    )
                    .collect();
                if !missing.is_empty() {
                    let known = if available.is_empty() {
                        "none — no capability registered any route".to_string()
                    } else {
                        available.join(", ")
                    };
                    return Err(CompositionError {
                        stage: CompositionStage::Assembly,
                        problems: missing
                            .into_iter()
                            .map(|problem| format!("{problem}. Registered routes: {known}"))
                            .collect(),
                    });
                }
            }
        }

        // ── A cast must EXIST by now, and "nobody said" is the failure ──
        //
        //  Checked AFTER the capabilities built, not against the draft. A
        // module may legitimately register its roster through a capability —
        // Outlander does, and an earlier version of this rewrite moved the
        // check to draft-declaration and broke it. What must fail is a
        // composition that prepares art with NO roster from any source.
        if prepares_art
            && app
                .world()
                .get_resource::<crate::characters::actor::character_catalog::CharacterCatalog>()
                .is_none()
        {
            return Err(CompositionError {
                stage: CompositionStage::Assembly,
                problems: vec![
                    "this composition prepares art and no character roster exists: no \
                     experience declared one and no mounted capability registered one. \
                     Either declare a roster with `characters(..)`, or say \
                     `no_characters()` if this game genuinely has no cast — an empty \
                     roster is valid, but the engine will not GUESS that you meant \
                     one, because a cast that silently vanished looks exactly like a \
                     cast that never existed."
                        .to_string(),
                ],
            });
        }

        // ── Rule 6 ── assets after the content that fills their catalogs,
        // before the presentation that draws them. Rides the `ambition_render`
        // capability; a composition that needs it without the feature was
        // already refused above, so this cfg never silently skips work.
        #[cfg(feature = "ambition_render")]
        {
            let windowed = matches!(face, Face::Windowed { .. });
            if windowed || game_assets {
                if !windowed {
                    app.init_asset::<Image>();
                    app.init_asset::<TextureAtlasLayout>();
                }
                //  The PRIMARY's policy, for every mounted experience, and
                // that is a stated limit rather than an oversight — see
                // `ModuleDraft::experience`. The plugin resolves three things
                // per experience id (the music fold, the SFX bank attribution,
                // the startup room theme) and installs once, so a secondary
                // experience gets the primary's three. The cast is unaffected:
                // catalog fragments merge, which is what makes the second
                // experience's characters draw correctly anyway.
                let room = draft
                    .experiences
                    .first()
                    .and_then(|e| e.room.clone())
                    .unwrap_or_default();
                app.add_plugins(
                    crate::game_assets::PlatformerAssetsPlugin::for_experience(
                        draft.experiences[0].id.clone(),
                    )
                    .with_room(room),
                );
            }
            if windowed {
                app.add_plugins(crate::presentation::PlatformerPresentationPlugin);
            }
        }

        // ── Rule 8 ── one update is one tick.
        if matches!(face, Face::Headless) {
            //  The two hosts need DIFFERENT dt values, and the difference is
            // one nanosecond. `Time::<Fixed>::from_hz(60.0)` rounds to
            // 16_666_667ns; GGRS wants the truncated 16_666_666. Feeding a GGRS
            // host the rounded value cost the fixture's parity walk 192
            // `update()` calls to reach a state the fixed-tick host reached in
            // 180. LEAK, found by migrating the fixture onto this
            // builder; the rule existed only in a comment on the code being
            // deleted.
            // ⭐ RULE 8 ASKS THE SAME QUESTION EVERYBODY ELSE DOES. It briefly
            // passed `rollback_participants.is_some()` under a comment claiming
            // `SimulationHost` might not exist yet — MEASURED FALSE: Rule 5
            // installs either `RollbackEnginePlugin` (which inserts
            // `SimulationHost::Rollback`) or the fixed-tick foundation (which
            // calls `set_simulation_host`), and Rule 5 is ~280 lines above this.
            // So the last host interpretation is gone from the clock policy and
            // there is genuinely one answer.
            enable_manual_stepping(app);
        }

        Ok(())
    }

    /// Boot into a launcher listing every mounted experience, instead of
    /// straight into the first one.
    ///
    /// What a host that ships several games needs, and what `game/ambition_app`
    /// wrote by hand before this existed.
    pub fn start_at_launcher(mut self) -> Self {
        self.start_at = StartAt::Launcher;
        self
    }

    /// Decode and publish this game's art even with no display.
    ///
    ///  It is a POLICY knob rather than a face, and the difference is load-bearing. That was a
    /// misreading — the criterion is about content identity in slice B, not about which plugins
    /// a display-less host installs — and the fixture's rollback parity test caught it: under
    /// GGRS the sim advances only through session requests, so extra asset frames are frames
    /// the sim does not move, and the two hosts reached the same world state twelve `update()`
    /// calls apart.
    pub fn with_game_assets(mut self) -> Self {
        self.game_assets = true;
        self
    }

    /// Compose this host for rollback, seating `participants` local
    /// players.
    ///
    /// ADR 0031 deferred rollback-as-a-public-knob deliberately: it is "a far
    /// larger promise than a clock — frozen schema, complete authoritative
    /// baseline, stable participants, deterministic activation, lifecycle
    /// rebasing, confirmation boundaries. Its own slice, its own acceptance
    /// tests." Slice F is that slice; see [`crate::rollback`] for how each of
    /// the six is kept.
    ///
    /// This is the COMPOSITION half only. It selects the GGRS host and freezes
    /// the participant count; it does not start a session, because a session
    /// rebases frame zero onto a world that has to be CONSTRUCTED first. Call
    /// [`crate::rollback::start`] on the built app.
    ///
    ///  `participants` is asked for rather than defaulted. Every path that
    /// guessed it guessed ONE, and the engine ran a rollback oracle over a
    /// single input stream for the week its couch versus mode seated four.
    ///
    /// A public API that reports a topology the running session does not have is worse than one
    /// that refuses, so this refuses.
    #[cfg(feature = "rollback")]
    pub fn rollback(mut self, participants: usize) -> Self {
        let seats = crate::characters::control::SlotControls::MAX_SLOTS;
        if participants == 0 || participants > seats {
            self.draft.conflicts.push(format!(
                "`rollback({participants})` cannot be seated: a session carries \
                 between 1 and {seats} participants. Zero participants is a \
                 session with no input streams to compare, and more than {seats} \
                 is more than the control slots the engine holds."
            ));
            return self;
        }
        self.rollback_participants = Some(participants);
        self
    }

    fn with_face(face: Face) -> Self {
        Self {
            face,
            session: SessionMode::FixedStep,
            rollback_participants: None,
            game_assets: false,
            start_at: StartAt::PrimaryGameplay,
            manifests: Vec::new(),
            draft: ModuleDraft::default(),
        }
    }
}

/// One experience's declared content, built and ready to register.
///
/// The output of the declaration pass's prepare step.
struct PreparedCast {
    characters:
        Option<crate::characters::actor::character_catalog::registry::CharacterCatalogFragment>,
    audio: Option<crate::audio::catalog::AudioCatalogFragment>,
}

/// Build one experience's cast and silence fragments from its declaration.
///
/// PURE — it reads the draft and touches no `App`. That is what lets its
/// failures be declaration problems rather than half-installed apps, and it is
/// the same prepare/commit split the rollback rebase uses: do the only fallible
/// work first, then commit infallibly.
///
/// Not `insert_resource(CharacterCatalog)`: that would be a second authority on
/// what the cast is, and fragments MERGE, which is what lets several
/// experiences coexist in one composition at all.
fn prepare_declared_cast(
    experience: &ExperienceDraft,
) -> Result<Option<PreparedCast>, Vec<String>> {
    use crate::characters::actor::character_catalog::registry::CharacterCatalogFragment;

    let mut problems = Vec::new();
    let characters = match experience.characters {
        Some(CharacterContent::Ron(ron)) => Some(ron),
        Some(CharacterContent::DeclaredEmpty) => Some(EMPTY_CHARACTER_ROSTER_RON),
        None => None,
    }
    .and_then(|ron| {
        match CharacterCatalogFragment::from_ron(experience.id.clone(), None::<String>, ron) {
            Ok(fragment) => Some(fragment),
            Err(error) => {
                problems.push(format!(
                    "the character roster declared by `{}` did not parse: {error}",
                    experience.id
                ));
                None
            }
        }
    });

    let audio = experience.declared_silence.then(|| {
        crate::audio::catalog::AudioCatalogFragment::new(experience.id.clone(), None, None)
    });
    let audio = match audio {
        Some(Ok(fragment)) => Some(fragment),
        Some(Err(error)) => {
            problems.push(format!(
                "`{}` declared silence and the empty audio fragment was rejected: {error}",
                experience.id
            ));
            None
        }
        None => None,
    };

    if !problems.is_empty() {
        return Err(problems);
    }
    if characters.is_none() && audio.is_none() {
        return Ok(None);
    }
    Ok(Some(PreparedCast { characters, audio }))
}

/// Lower one experience's playable declaration into a deferred install.
///
/// Through the SAME `PlatformerExperienceAuthoring` seam a provider plugin
/// would have used — the draft removes the boilerplate, not the authority.
fn experience_installer(experience: &ExperienceDraft) -> Option<CapabilityInstaller> {
    let definition = experience.definition.as_ref()?;
    let id = experience.id.clone();
    let route = experience.gameplay_route.clone()?;
    let label = definition.label.clone();
    let description = definition.description.clone();
    let starting_character = definition.starting_character.clone();
    let starting_room = definition.starting_room.clone();
    let rooms = definition.rooms.clone();

    Some(Box::new(move |app: &mut App| {
        use crate::runtime::demo_fixture::{ActiveRoomMetadata, RoomSet, StartingCharacter};
        let Some(first) = rooms.first().cloned() else {
            return;
        };
        let starting = rooms
            .iter()
            .find(|room| room.id == starting_room)
            .cloned()
            .unwrap_or(first);
        let geometry = crate::engine_core::RoomGeometry(starting.world.clone());
        let metadata = ActiveRoomMetadata(starting.metadata.clone());
        let prepared = crate::runtime::PreparedPlatformerSource::new(
            id.clone(),
            RoomSet::from_parts(starting_room.clone(), rooms.clone(), Vec::new()),
            geometry,
            metadata,
            StartingCharacter::new(starting_character.clone()),
        );
        crate::provider::PlatformerExperienceAuthoring::new(
            id.clone(),
            route.clone(),
            label.clone(),
            description.clone(),
            format!("Prepare {id}"),
            crate::provider::AuthoredCatalogFragments::new(starting_character.clone(), id.clone()),
        )
        .with_defense_presentation(
            ambition_platformer2d_shared_tangle::gameplay_presentation::DefensePresentationPolicy::shared_iframe_blink(),
        )
        .install(app, move || prepared.clone());
    }))
}

/// `DefaultPlugins`, configured. Rules 2, 3 and 4.
///
/// Public because a standalone demo shell needs exactly this and nothing else:
/// three demos hand-roll their own `DefaultPlugins` today and each re-derives
/// the disables that [`Display`] documents. A fourth copy would be the leak.
/// The manual-step period for the simulation host this app is running — the ONE
/// answer to *"how much time is one externally-driven `App::update()` worth"*.
///
/// ⛔⛤ THE TWO HOSTS DIFFER BY ONE NANOSECOND AND IT IS NOT COSMETIC.
/// `Time::<Fixed>::from_hz(60.0)` rounds to `16_666_667ns`; GGRS truncates to
/// `16_666_666`. Feeding a GGRS host the rounded value cost a parity fixture 192
/// `update()` calls to reach a state the fixed-tick host reached in 180 — the
/// accumulator gains a tick every few thousand frames.
///
/// ⛔⛔ AND THE CALLER DOES NOT GET TO ANSWER WHICH HOST IT HAS. This briefly took
/// a `rollback: bool`, which moved the arithmetic into one place and left the
/// DECISION scattered — `moveset_takes` passed a literal `true` under a comment
/// admitting the app should know. `SimulationHost` is already a resource and is
/// already the canonical answer; asking it is the difference between one
/// authority and one formula with many opinions about its input.
///
/// `RenderFrame` REFUSES rather than defaulting: that host has no fixed tick at
/// all — it advances with the render frame — so "one update is one tick" is not
/// a promise it can keep, and quietly handing back a 60Hz period would fabricate
/// a guarantee.
pub fn manual_step_period(app: &App) -> Option<std::time::Duration> {
    use crate::runtime::SimulationHost;
    match app
        .world()
        .get_resource::<SimulationHost>()
        .copied()
        .unwrap_or_default()
    {
        SimulationHost::Rollback => Some(host_step_period(app, true)),
        SimulationHost::Fixed60Hz => Some(host_step_period(app, false)),
        SimulationHost::RenderFrame => None,
    }
}

/// THE arithmetic, in one place, for the two hosts that have a fixed tick.
///
/// ⛔ PRIVATE, and reached only through [`manual_step_period`]. Nothing outside
/// this file gets to state which host it has; the resource is the answer.
fn host_step_period(app: &App, rollback: bool) -> std::time::Duration {
    if rollback {
        std::time::Duration::from_nanos(1_000_000_000u64 / crate::runtime::SIM_TICK_HZ as u64)
    } else {
        app.world().resource::<Time<Fixed>>().timestep()
    }
}

/// Put an already-built app under manual stepping, so one `update()` is one
/// simulation tick. Returns the period installed.
///
/// ⭐ FOR ANY APP A DRIVER STEPS, headless or not. Rule 8 calls this for the
/// headless face; an offscreen or windowed app that a driver steps itself wants
/// the same contract and reaches it the same way. Presence of presentation and
/// who owns the clock are INDEPENDENT questions — a GPU capture of a
/// deterministic simulation needs both, which is precisely the case this exists
/// for and the reason it is not gated on a face.
///
/// ⛔ INSTALL BEFORE THE SESSION RUNS. Switching a live rollback host from wall
/// time to manual time leaves whatever the accumulator had already banked, so
/// the first few steps are not one-for-one.
///
/// # Panics
///
/// Under `SimulationHost::RenderFrame`, which cannot honour the contract. A
/// silent no-op would leave a driver stepping by the wall clock while believing
/// otherwise, which is the failure this whole seam exists to end.
pub fn enable_manual_stepping(app: &mut App) -> std::time::Duration {
    let period = manual_step_period(app).expect(
        "manual stepping needs a fixed-tick simulation host; `SimulationHost::RenderFrame` \
         advances with the render frame and cannot promise one tick per update",
    );
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(period));
    period
}

pub fn install_windowed_foundation(app: &mut App, title: &str, display: Display) {
    use bevy::window::{ExitCondition, Window, WindowPlugin};

    let window = display.has_window();
    let plugins = DefaultPlugins
        .set(bevy::asset::AssetPlugin {
            // Rule 2: the engine knows where its own content is.
            file_path: crate::asset_manager::actors_desktop_asset_root(),
            ..Default::default()
        })
        .set(WindowPlugin {
            primary_window: window.then(|| Window {
                title: title.to_string(),
                ..Default::default()
            }),
            // Without a window there is nothing whose closing could mean "quit",
            // so the app exits when its caller says so and not before.
            exit_condition: if window {
                ExitCondition::OnAllClosed
            } else {
                ExitCondition::DontExit
            },
            close_when_requested: window,
            ..Default::default()
        });

    match display {
        Display::Window => app.add_plugins(plugins),
        // Rule 3, offscreen half: a REAL backend and a real render graph, with
        // no window in front of it.
        //
        // Disabling `winit` is what removes the window, and it takes the app
        // RUNNER with it — so an offscreen app is stepped by its caller rather
        // than by `run()`. That is the property a capture wants (a burst is
        // exactly as many frames as it asks for) and the trap a consumer falls
        // into (`run()` returns immediately and nothing is ever drawn), which is
        // why this lives here and is written down.
        //
        // The core pipeline and gizmo passes STAY, unlike the no-GPU face
        // below: without them the graph produces no picture, and a capture that
        // reads back an empty texture reports success on a transparent PNG.
        Display::Offscreen => {
            let plugins = plugins.disable::<bevy::winit::WinitPlugin>();
            #[cfg(not(target_arch = "wasm32"))]
            let plugins = plugins.disable::<bevy::app::TerminalCtrlCHandlerPlugin>();
            app.add_plugins(plugins)
        }
        // Rule 3. A `backends: None` renderer has no RenderApp, and
        // process-global logging / Ctrl+C handlers belong to an executable
        // rather than to a manually stepped fixture.
        Display::NoGpu => {
            use bevy::render::settings::{RenderCreation, WgpuSettings};
            use bevy::render::RenderPlugin;
            let plugins = plugins
                .disable::<bevy::log::LogPlugin>()
                .disable::<bevy::core_pipeline::CorePipelinePlugin>()
                .disable::<bevy::gizmos_render::GizmoRenderPlugin>()
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                        backends: None,
                        ..Default::default()
                    })),
                    ..Default::default()
                })
                .disable::<bevy::winit::WinitPlugin>();
            //  desktop only: there is no terminal, and no such plugin, on the web.
            // `disable` on a plugin that does not exist for the target is a COMPILE
            // error, not a no-op, so the cfg has to move the call and not the type.
            #[cfg(not(target_arch = "wasm32"))]
            let plugins = plugins.disable::<bevy::app::TerminalCtrlCHandlerPlugin>();
            app.add_plugins(plugins)
        }
    };

    // Rule 4: after Bevy's StatesPlugin exists, before the sim plugins whose
    // run conditions read the state.
    crate::engine::init_engine_states(app);
}

/// Declaration-time refusals that no consumer crate can reach.
///
/// These live here rather than in `fixtures/external_consumer` for the reason
/// that fixture states about itself: a consumer test proves what a third party
/// CAN do, and both cases below are things a third party must NOT be able to
/// do. They are also pure draft arithmetic — `try_build` refuses at the
/// declaration pass, before an `App` is touched — so they cost a `App::new()`.
#[cfg(test)]
mod tests {
    use super::*;

    /// Well-formed: declares its own experience and every required route.
    struct FirstGame;
    impl GameModule for FirstGame {
        fn manifest(&self) -> ModuleManifest {
            ModuleManifest::new("first")
        }
        fn define(&self, module: &mut ModuleDraft) {
            module
                .experience("first")
                .launcher_route("home")
                .gameplay_route("first/play");
        }
    }

    /// The hazard, as a module: it routes without ever naming an experience.
    struct SecondGameThatForgot;
    impl GameModule for SecondGameThatForgot {
        fn manifest(&self) -> ModuleManifest {
            ModuleManifest::new("second")
        }
        fn define(&self, module: &mut ModuleDraft) {
            module.gameplay_route("second/play");
        }
    }

    /// A module cannot modify the experience the PREVIOUS module declared.
    ///
    /// Deterministic, and with no diagnostic anywhere.
    #[test]
    fn a_module_that_routes_before_declaring_an_experience_is_refused() {
        let refused = PlatformerApp::headless()
            .mount(FirstGame)
            .mount(SecondGameThatForgot)
            .try_build()
            .expect_err(
                "a module that never named an experience must not silently \
                 edit the previous module's",
            );

        assert_eq!(refused.stage, CompositionStage::Declaration);
        let message = refused.to_string();
        assert!(
            message.contains("`second` declared a gameplay route before naming an experience"),
            "the refusal must name the module that forgot: {message}"
        );
        //  Non-vacuity: the FIRST module's identical call is fine, so the
        // refusal is about the boundary rather than about the method.
        assert!(
            !message.contains("`first` declared"),
            "the well-formed module was blamed too: {message}"
        );
    }

    /// A roster that does not parse is refused BEFORE the `App` is touched.
    ///
    /// The prepare step moved to the declaration pass.
    ///
    /// The `install_into` form is what makes this observable: the consumer owns
    /// the `App`, so "was anything installed" is a question it can ask.
    #[test]
    fn a_roster_that_does_not_parse_is_refused_before_anything_is_installed() {
        struct MalformedRoster;
        impl GameModule for MalformedRoster {
            fn manifest(&self) -> ModuleManifest {
                ModuleManifest::new("malformed")
            }
            fn define(&self, module: &mut ModuleDraft) {
                module
                    .experience("malformed")
                    .launcher_route("home")
                    .gameplay_route("malformed/play")
                    .characters("this is not a catalog");
            }
        }

        let mut app = App::new();
        let refused = PlatformerApp::headless()
            .mount(MalformedRoster)
            .install_into(&mut app)
            .expect_err("a roster that does not parse must refuse");

        assert_eq!(
            refused.stage,
            CompositionStage::Declaration,
            "the parse is pure, so its failure belongs to the pass that runs \
             before any mutation: {refused}"
        );
        assert!(
            refused.to_string().contains("did not parse"),
            "the refusal must say what was wrong with the roster: {refused}"
        );
        //  the half that makes the stage mean anything. A `Declaration`
        // refusal claims nothing was installed; this asks the `App`.
        assert!(
            !app.is_plugin_added::<bevy::asset::AssetPlugin>(),
            "the composition refused at the declaration stage and had still \
             installed the Bevy foundation into the consumer's App"
        );

        // NON-VACUITY, and the control that makes the assertion above mean
        // something: a refusal from the ASSEMBLY pass has installed the
        // foundation, because assembly is defined as the pass that needs the
        // built App. `FirstGame` declares a route no capability registers, which
        // is exactly that kind of check.
        let mut assembled = App::new();
        let assembly_refusal = PlatformerApp::headless()
            .mount(FirstGame)
            .install_into(&mut assembled)
            .expect_err("a route nothing registers must refuse");
        assert_eq!(assembly_refusal.stage, CompositionStage::Assembly);
        assert!(
            assembled.is_plugin_added::<bevy::asset::AssetPlugin>(),
            "an assembly-stage refusal is supposed to have built the App it \
             refused on — if it has not, the check above proves nothing"
        );
    }

    /// A capability's semantic action reaches the composition's registry,
    /// beside the engine's own vocabulary and without editing a closed enum.
    ///
    /// `Platformer2dInputActionMonolith` is leafwing's concrete `Actionlike` and cannot grow a
    /// variant from outside. This is the open half arriving where a prompt, a
    /// help screen or a rebind UI can ask ONE question.
    #[test]
    fn a_capabilitys_action_lands_in_the_compositions_registry() {
        use ambition_input::{
            ActionControlKind, InstalledActions, SemanticActionDef, SemanticActionId,
            GAMEPLAY_CONTEXT,
        };

        const GRAPPLE: &[SemanticActionDef] = &[SemanticActionDef {
            id: SemanticActionId("grapple"),
            capability: "traversal",
            kind: ActionControlKind::Button,
            contexts: &[GAMEPLAY_CONTEXT],
            doc: "Fire the grapple",
        }];

        struct TraversalModule;
        impl GameModule for TraversalModule {
            fn manifest(&self) -> ModuleManifest {
                ModuleManifest::new("traversal")
            }
            fn define(&self, module: &mut ModuleDraft) {
                module
                    .experience("traversal")
                    .launcher_route("home")
                    .gameplay_route("traversal/play");
                module.actions(GRAPPLE);
            }
        }

        let mut app = App::new();
        // The composition refuses later (no capability registers the route),
        // which is fine: the registry is built BEFORE that, and what this test
        // is about is whether the action arrived.
        let _ = PlatformerApp::headless()
            .mount(TraversalModule)
            .install_into(&mut app);

        let installed = app
            .world()
            .get_resource::<InstalledActions>()
            .expect("the composition builds an action registry");
        assert_eq!(
            installed
                .get(SemanticActionId("grapple"))
                .map(|d| d.capability),
            Some("traversal"),
            "a capability's action is in the composition's vocabulary"
        );
        assert!(
            installed.get(SemanticActionId("jump")).is_some(),
            "and the engine's own vocabulary is there too, unasked"
        );
        assert!(
            installed
                .for_context(GAMEPLAY_CONTEXT)
                .any(|d| d.id.0 == "grapple"),
            "so ONE question answers what may be pressed here, for both"
        );
    }

    ///  two owners for one action id is a composition refusal, for the same
    /// reason an ambiguous content schema is: letting it through means the
    /// winner is decided by iteration order.
    #[test]
    fn two_capabilities_claiming_one_action_refuse_the_composition() {
        use ambition_input::{
            ActionControlKind, SemanticActionDef, SemanticActionId, GAMEPLAY_CONTEXT,
        };

        const STOLEN: &[SemanticActionDef] = &[SemanticActionDef {
            id: SemanticActionId("jump"),
            capability: "traversal",
            kind: ActionControlKind::Button,
            contexts: &[GAMEPLAY_CONTEXT],
            doc: "a second jump",
        }];

        struct Thief;
        impl GameModule for Thief {
            fn manifest(&self) -> ModuleManifest {
                ModuleManifest::new("thief")
            }
            fn define(&self, module: &mut ModuleDraft) {
                module
                    .experience("thief")
                    .launcher_route("home")
                    .gameplay_route("thief/play");
                module.actions(STOLEN);
            }
        }

        let mut app = App::new();
        let refused = PlatformerApp::headless()
            .mount(Thief)
            .install_into(&mut app)
            .expect_err("`jump` is the engine's");
        let message = refused.to_string();
        assert!(
            message.contains("jump") && message.contains("traversal") && message.contains("engine"),
            "the refusal names the action and BOTH claimants: {message}"
        );
    }

    /// A capability whose required rollback state nobody installed is REFUSED
    /// at assembly, not left to desync at the first rewind.
    ///
    /// A capability offers its rollback state and the composition installs it —
    /// which keeps the capability's dependency closure to foundations, and left
    /// a hole: nothing made the composition accept the offer. This is the
    /// refusal that closes it, and it is the same shape as the content
    /// compiler's "a `Runtime` schema must lower an artifact".
    #[test]
    fn a_capability_whose_rollback_state_is_missing_is_refused_with_the_reason() {
        use ambition_platformer2d_core::snapshot::RequiredRollbackState;

        const NEEDED: &[RequiredRollbackState] = &[RequiredRollbackState {
            owner: "test_capability",
            name: "test.cooldown",
            why: "a cooldown that is not rewound fires twice from one charge",
        }];

        struct ForgetfulModule;
        impl GameModule for ForgetfulModule {
            fn manifest(&self) -> ModuleManifest {
                ModuleManifest::new("forgetful")
            }
            fn define(&self, module: &mut ModuleDraft) {
                module
                    .experience("forgetful")
                    .launcher_route("home")
                    .gameplay_route("forgetful/play");
                // Declares the need and mounts nothing that registers it —
                // exactly the mistake the check exists for.
                module.requires_rollback(NEEDED);
            }
        }

        let mut app = App::new();
        app.init_resource::<crate::runtime::rollback::RollbackRegistry>();
        let refused = PlatformerApp::headless()
            //  the composition must ASK for rollback. A game that never
            // rewinds cannot desync, so the check is gated on the declaration
            // rather than on the registry — which the headless foundation
            // installs either way.
            .rollback(2)
            .mount(ForgetfulModule)
            .install_into(&mut app)
            .expect_err("an unregistered requirement must refuse");
        assert_eq!(refused.stage, CompositionStage::Assembly);
        let message = refused.to_string();
        assert!(
            message.contains("test.cooldown") && message.contains("test_capability"),
            "the refusal names the state and its owner: {message}"
        );
        assert!(
            message.contains("fires twice from one charge"),
            "and carries the capability's own WHY, so a host knows this is a desync rather \
             than an optional extra: {message}"
        );
    }

    ///  and it does NOT refuse a composition with no rollback host.
    ///
    /// A headless game with no session cannot desync, and a check that refused
    /// it would be the thing that breaks compositions rather than the thing
    /// that protects them.
    #[test]
    fn a_composition_with_no_rollback_host_is_not_asked_for_registrations() {
        use ambition_platformer2d_core::snapshot::RequiredRollbackState;

        const NEEDED: &[RequiredRollbackState] = &[RequiredRollbackState {
            owner: "test_capability",
            name: "test.cooldown",
            why: "would desync under a rewind, if there were rewinds",
        }];

        struct NeedyModule;
        impl GameModule for NeedyModule {
            fn manifest(&self) -> ModuleManifest {
                ModuleManifest::new("needy")
            }
            fn define(&self, module: &mut ModuleDraft) {
                module
                    .experience("needy")
                    .launcher_route("home")
                    .gameplay_route("needy/play");
                module.requires_rollback(NEEDED);
            }
        }

        // No `rollback(..)` declared: nothing rewinds here.  the registry
        // still EXISTS — the headless foundation installs one unconditionally,
        // which is exactly why gating on its presence was wrong.
        let mut app = App::new();
        let outcome = PlatformerApp::headless()
            .mount(NeedyModule)
            .install_into(&mut app);
        assert!(
            outcome.is_ok() || !format!("{outcome:?}").contains("test.cooldown"),
            "a composition that cannot rewind must not be refused for a rewind hazard: \
             {outcome:?}"
        );
    }

    /// A participant count no session can seat is refused at composition.
    #[test]
    fn a_participant_count_the_session_cannot_seat_is_refused() {
        let seats = crate::characters::control::SlotControls::MAX_SLOTS;
        for count in [0, seats + 1] {
            let refused = PlatformerApp::headless()
                .rollback(count)
                .mount(FirstGame)
                .try_build()
                .expect_err("an unseatable participant count must refuse");
            let message = refused.to_string();
            assert!(
                message.contains(&format!("`rollback({count})` cannot be seated")),
                "the refusal must name the count it rejected: {message}"
            );
        }
        // Non-vacuity: a count in range is accepted and travels with the
        // composition. Read off the draft rather than by building, because
        // building a rollback host is a whole engine and this is arithmetic.
        let composed = PlatformerApp::headless().rollback(seats).mount(FirstGame);
        assert!(
            composed.draft.conflicts.is_empty(),
            "a legal participant count was refused: {:?}",
            composed.draft.conflicts
        );
        assert_eq!(composed.rollback_participants, Some(seats));
    }
}
