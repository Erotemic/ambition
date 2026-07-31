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
        host_status, AssetSource, CompositionError, GameModule, HostStatus, ModuleDraft,
        ModuleManifest, PlatformerApp, SessionMode, StartAt, EMPTY_CHARACTER_ROSTER_RON,
        MINIMAL_CHARACTER_ROSTER_RON,
    };
    pub use bevy::prelude::App;

    /// The room types this module's own signatures demand.
    ///
    /// ⚠ Re-exported here because blind run 2 (2026-07-30) had to open
    /// `crates/ambition_world/src/lib.rs` to find them — the ONE engine source
    /// file it opened, and therefore the field §2c says names the next leak.
    /// `ModuleDraft::playable` takes `Vec<RoomSpec>` and `ModuleDraft::room`
    /// takes `RoomMetadata`; neither was reachable from the prelude that
    /// declares them, rustdoc rendered both as unlinked text, and rustc offered
    /// no import suggestion for `RoomSpec` at all.
    ///
    /// A prelude that omits the types its own signatures require is a prelude
    /// that sends its reader into `crates/`.
    pub use crate::world::rooms::{RoomMetadata, RoomSpec};
}

/// **Did my game actually start?**
///
/// The one question a consumer could not ask. Four of the eight ordering rules
/// [`PlatformerApp`] owns fail SILENTLY, and before this there was no supported
/// way to check — the 2026-07-30 blind agent fell back to
/// `app.world().entities().len()`, which is raw Bevy and says nothing about
/// routes. Every consumer would have invented that same smoke test, badly.
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
    /// Routing REFUSED this host, and this is why.
    ///
    /// ⚠ Before slice C this state was indistinguishable from
    /// [`HostStatus::Activating`]: the reason existed, reached `error!`, and
    /// never reached the consumer. A headless test with no log subscriber saw a
    /// host that simply never started, and the campaign burned a whole slice
    /// discovering that on its own new consumer.
    Refused { reasons: Vec<String> },
    /// A route is live.
    ///
    /// ⚠ `prepared == false` is the quiet failure: the router is pointing at a
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
    ///
    /// Both halves on purpose. `Running { prepared: false }` answering `true`
    /// here would make this read-model agree with the bug it exists to expose.
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
    // A recorded refusal outranks "still pending": the router can hold a
    // pending route whose load already failed, which is exactly the state that
    // used to read as `Activating` forever.
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

/// A character roster with nothing in it.
///
/// Published because the 2026-07-30 blind agent had to RECOVER this value: it
/// needed a `CharacterCatalog` to reach the windowed face, found no `Default`
/// and no documented schema, fed the parser `"()"`, and scripted a loop over
/// the resulting *"Unexpected missing field named X"* errors until the struct
/// closed. A value obtainable only by brute-forcing diagnostics is a value the
/// engine knows and would not say.
pub const EMPTY_CHARACTER_ROSTER_RON: &str =
    "(brain_presets: {}, action_set_presets: {}, characters: {})";

/// A playable experience, as a value.
///
/// ⚠ **This exists because the SECOND consumer measured what the first could
/// not.** The movement-only minimal game named five `ambition::` modules, and
/// four of them were here: it had to build a `PreparedPlatformerSource` by hand
/// (`ambition::runtime`), wrap it in `PlatformerExperienceAuthoring`
/// (`ambition::provider`), and construct the room and geometry to put in it
/// (`ambition::world`, `ambition::engine_core`). A game could COMPOSE through
/// the SDK and still could not DECLARE what it was.
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

/// **A roster with ONE character** — the case a game actually starts from.
///
/// ⚠ Published because [`EMPTY_CHARACTER_ROSTER_RON`] solved the case nobody
/// needs. Blind run 3 tried to derive this by starting from the empty roster
/// and letting the parser name what was missing, and reported two things that
/// make that route a dead end: the parser names exactly ONE missing field per
/// build-and-run cycle, and it stops dead at the first ENUM-typed field
/// (`tier: ""` → `Expected identifier`) because variant names cannot be
/// guessed. It gave up after four cycles and opened a fixture — which is the
/// SDK's acceptance test failing by the SDK's own remedy.
///
/// The enum-valued fields, since those are the ones no error message will tell
/// you:
///
/// * `tier` — `MainHall` for an ordinary character.
/// * `body_kind` — `Standard`.
/// * `composition` — `None` unless the body is assembled from parts.
/// * `playable_kit` — **`Authored`** if this character's own action set is the
///   authority, `HostCode` if it should wear the host protagonist's kit
///   instead. These mean opposite things and the wrong one silently overrides
///   everything you declared below it.
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
            playable_kit: Authored,
            tags: ["player"],
        ),
    },
)"#;

/// One experience, as declared by one module.
///
/// ⚠ **A composition holds MANY of these, and it did not until slice D.** The
/// draft carried a single global `experience`, so mounting two games side by
/// side was impossible — the second module's `experience()` collided with the
/// first instead of sitting beside it. That blocked two consumer-matrix rows at
/// once: `module-standalone-and-embedded` (embedded MEANS coexisting) and
/// `ambition-itself` (the shipped host registers four).
///
/// The shell was never the limitation. `ShellRouteCatalog` and
/// `ShellExperienceRegistry` already hold many, and `game/ambition_app`
/// composes four by hand today. The limitation was this struct being a set of
/// loose fields on the draft, written in slice A when one experience was the
/// only case that existed.
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
    /// Every declared experience, in declaration order. The FIRST is the host's
    /// initial route — a host starts somewhere, and "the first one mounted"
    /// is a rule a consumer can predict without a second knob to set.
    experiences: Vec<ExperienceDraft>,
    /// Which experience subsequent calls apply to.
    current: Option<usize>,
    capabilities: Vec<CapabilityInstaller>,
    conflicts: Vec<String>,
}

impl ModuleDraft {
    /// Begin declaring an experience. Subsequent calls apply to it.
    ///
    /// A composition may hold several. Declaring the SAME id twice is a
    /// conflict naming both modules; declaring a different one starts a new
    /// experience beside the first.
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

    /// Declare, explicitly, that this experience authors **no** characters.
    ///
    /// ⚠ Instead of a silent default. `PlatformerAssetsPlugin` refuses to
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

    /// Declare, explicitly, that this experience authors **no sound**.
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
    pub fn capability<M>(&mut self, plugin: impl Plugins<M> + Send + 'static) -> &mut Self {
        self.capabilities.push(Box::new(move |app: &mut App| {
            app.add_plugins(plugin);
        }));
        self
    }

    /// Apply an edit to the experience currently being declared.
    ///
    /// ⚠ Declaring anything before `experience()` is a CONFLICT, not a silent
    /// no-op. With one global experience the ordering did not matter; with
    /// several, "which one did that route attach to" has a wrong answer, and a
    /// route silently attached to nothing is precisely the empty host this
    /// campaign spent slice C making impossible.
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
    /// Which pass produced these, and therefore what has NOT been checked yet.
    pub stage: CompositionStage,
}

/// **Which pass of composition refused.**
///
/// ADR 0032's promise — *"a draft yields one build error listing every conflict
/// in the experience"* — is true WITHIN a pass and cannot be true across them:
/// the second pass's checks need the capabilities BUILT (a module may
/// legitimately register its roster through one), so a draft that does not
/// assemble cannot be asked whether its roster exists.
///
/// That is a funnel, and it was a SILENT one until 2026-07-31, when the
/// slice-H red probe walked into it: a fixture built without the render
/// capability and declaring no cast was told only about the capability, and its
/// error said `1 problem(s)` as if that were the whole list. Fix it, rebuild for
/// ten minutes, discover the next one.
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
/// ⚠ **Two policies, because there are two real hosts and the builder offered
/// only one.** Measured 2026-07-30 against `game/ambition_app`: it boots into a
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
    ///
    /// ⚠ **The draft's experience cursor is MODULE-LOCAL, and it was not.**
    /// `current` survived this boundary, so a module that called an
    /// experience-scoped method before declaring its own experience silently
    /// edited the PREVIOUS module's — module B's `gameplay_route` landing on
    /// experience A, deterministically, with no diagnostic. A module may only
    /// modify an experience it declared during its own `define`; anything
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
        // ── Slice H ── a facade built without the `ambition_render` capability
        // has no presentation to install, and a composition that prepares art
        // must be REFUSED here rather than silently drawing nothing. This
        // refusal was probed red: `minimal_game` with the capability removed
        // fails its windowed tests on exactly this message.
        #[cfg(not(feature = "ambition_render"))]
        if prepares_art {
            problems.push(
                "this composition prepares art (a windowed face, or `with_game_assets`), \
                 but `ambition` was built without the `ambition_render` capability. \
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
        if !problems.is_empty() {
            return Err(CompositionError {
                stage: CompositionStage::Declaration,
                problems,
            });
        }

        // ── Rule 1 ── before any AssetPlugin, in every face.
        //
        // ⚠ desktop only. `consumer_source` LAYERS a game's asset tree over the
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
            Face::Windowed { title, gpu } => install_windowed_foundation(app, title, *gpu),
        }

        // ── The cast and the silence, per experience, through the seams a
        // provider plugin would have used.
        for experience in &draft.experiences {
            register_declared_cast(app, experience)?;
            if experience.declared_silence {
                use crate::audio::catalog::{AudioCatalogAppExt, AudioCatalogFragment};
                let fragment = AudioCatalogFragment::new(experience.id.clone(), None, None)
                    .map_err(|error| CompositionError {
                        stage: CompositionStage::Assembly,
                        problems: vec![format!(
                            "`{}` declared silence and the empty audio fragment was \
                             rejected: {error}",
                            experience.id
                        )],
                    })?;
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
        if let Some(participants) = rollback_participants {
            app.add_plugins(crate::engine::PlatformerEnginePlugins::rollback());
            // The declaration travels with the composition, so a restart reads
            // the count the game stated rather than re-sampling live devices.
            app.insert_resource(crate::rollback::DeclaredParticipants(participants));
        } else {
            app.add_plugins(crate::engine::PlatformerEnginePlugins::fixed_tick());
        }
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
        // ⚠ Checked AFTER the capabilities built, not against the draft. A
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
            // ⚠ The two hosts need DIFFERENT dt values, and the difference is
            // one nanosecond. `Time::<Fixed>::from_hz(60.0)` rounds to
            // 16_666_667ns; GGRS wants the truncated 16_666_666. Feeding a GGRS
            // host the rounded value cost the fixture's parity walk 192
            // `update()` calls to reach a state the fixed-tick host reached in
            // 180. LEAK, found 2026-07-30 by migrating the fixture onto this
            // builder; the rule existed only in a comment on the code being
            // deleted.
            let frame_dt = if rollback_participants.is_some() {
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

    /// **Compose this host for rollback**, seating `participants` local
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
    /// ⚠ `participants` is asked for rather than defaulted. Every path that
    /// guessed it guessed ONE, and the engine ran a rollback oracle over a
    /// single input stream for the week its couch versus mode seated four.
    ///
    /// ⚠ **A count the session cannot seat is REFUSED here, not clamped
    /// downstream.** `SyncTestSettings::player_count` clamps into `1..=MAX_SLOTS`
    /// because it is settings data arriving from a dev tool, and that clamp used
    /// to run *after* this value had already been reported back to the
    /// consumer — `rollback(0)` returned a `RollbackSession` claiming zero
    /// participants over a session GGRS built with one. A public API that
    /// reports a topology the running session does not have is worse than one
    /// that refuses, so this refuses.
    pub fn rollback(mut self, participants: usize) -> Self {
        let seats = crate::characters::brain::SlotControls::MAX_SLOTS;
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

/// Register one experience's declared cast, through the fragment seam every
/// in-repo provider uses.
///
/// Not `insert_resource(CharacterCatalog)`: that would be a second authority on
/// what the cast is, and fragments MERGE, which is what lets several
/// experiences coexist in one composition at all.
fn register_declared_cast(
    app: &mut App,
    experience: &ExperienceDraft,
) -> Result<(), CompositionError> {
    use crate::characters::actor::character_catalog::registry::{
        CharacterCatalogAppExt as _, CharacterCatalogFragment,
    };
    let ron = match experience.characters {
        Some(CharacterContent::Ron(ron)) => ron,
        Some(CharacterContent::DeclaredEmpty) => EMPTY_CHARACTER_ROSTER_RON,
        None => return Ok(()),
    };
    let fragment = CharacterCatalogFragment::from_ron(experience.id.clone(), None::<String>, ron)
        .map_err(|error| CompositionError {
        stage: CompositionStage::Assembly,
        problems: vec![format!(
            "the character roster declared by `{}` did not parse: {error}",
            experience.id
        )],
    })?;
    app.try_register_character_catalog_fragment(fragment)
        .map_err(|error| CompositionError {
            stage: CompositionStage::Assembly,
            problems: vec![format!(
                "the character roster declared by `{}` conflicts with one already \
                 registered: {error}",
                experience.id
            )],
        })?;
    Ok(())
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
        use crate::runtime::demo_fixture::{
            ActiveRoomMetadata, LdtkRuntimeIndex, RoomSet, StartingCharacter,
        };
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
            LdtkRuntimeIndex::default(),
        );
        crate::provider::PlatformerExperienceAuthoring::new(
            id.clone(),
            route.clone(),
            label.clone(),
            description.clone(),
            format!("Prepare {id}"),
            crate::provider::AuthoredCatalogFragments::new(starting_character.clone(), id.clone()),
        )
        .install(app, move || prepared.clone());
    }))
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
        let plugins = plugins
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::core_pipeline::CorePipelinePlugin>()
            .disable::<bevy::gizmos_render::GizmoRenderPlugin>()
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    backends: None,
                    ..Default::default()
                }),
                ..Default::default()
            })
            .disable::<bevy::winit::WinitPlugin>();
        // ⚠ desktop only: there is no terminal, and no such plugin, on the web.
        // `disable` on a plugin that does not exist for the target is a COMPILE
        // error, not a no-op, so the cfg has to move the call and not the type.
        #[cfg(not(target_arch = "wasm32"))]
        let plugins = plugins.disable::<bevy::app::TerminalCtrlCHandlerPlugin>();
        app.add_plugins(plugins);
    }

    // Rule 4: after Bevy's StatesPlugin exists, before the sim plugins whose
    // run conditions read the state.
    crate::engine::init_engine_states(app);
}

/// **Declaration-time refusals that no consumer crate can reach.**
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

    /// **A module cannot modify the experience the PREVIOUS module declared.**
    ///
    /// The draft's cursor used to survive the `mount` boundary, so this exact
    /// sequence — declare, mount, route — silently overwrote experience
    /// `first`'s gameplay route with `second/play` and built a host where the
    /// second game was unreachable and the first one led somewhere it never
    /// asked for. Deterministic, and with no diagnostic anywhere.
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
        // ⚠ Non-vacuity: the FIRST module's identical call is fine, so the
        // refusal is about the boundary rather than about the method.
        assert!(
            !message.contains("`first` declared"),
            "the well-formed module was blamed too: {message}"
        );
    }

    /// **A participant count no session can seat is refused at composition.**
    ///
    /// Not clamped downstream and reported back as if it had been honoured:
    /// `rollback(0)` used to return a `RollbackSession` claiming zero
    /// participants over a GGRS session built with one.
    #[test]
    fn a_participant_count_the_session_cannot_seat_is_refused() {
        let seats = crate::characters::brain::SlotControls::MAX_SLOTS;
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
