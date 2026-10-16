//! ⭐ A RUNNING DEMO PLAYS AN EDIT TO ITS OWN PACK, with no Rust build and no
//! restart.
//!
//! A game whose [`EmbeddedPack`] is read off disk (a build without the crate's
//! `static_content` feature) adds [`EmbeddedPackReloadPlugin`]. The plugin
//! looks at `pack.ron` and at each source it declares. When a file changes, it
//! compiles the pack again from disk and stages the result. Between two
//! sessions it publishes the staged generation, and every later
//! [`EmbeddedPack::prepared`] reads it.
//!
//! The rules of the road:
//!
//! - A pack that does not compile is refused, with the file and the field
//!   named, and the running pack stays.
//! - A save that changes no content publishes nothing.
//! - The game names the schemas it reads where it uses them (`live`), and the
//!   schemas it read once at start but can state again (`restated`: each with
//!   the function that states it again from the new generation). A change to
//!   another schema is refused: the game read that schema once when it
//!   started (its cast, its move tables, its audio), so a publication would
//!   give a mix of two generations. Restart to play that edit.
//! - The staged pack is published only when no session is live and no route
//!   is preparing one. A session reads one generation from its preparation to
//!   its end, so a rollback cannot read two.
//!
//! Ambition's own pack has a fuller road (`ambition_content::reload`): it
//! publishes into a live session at a rollback boundary. This road is for the
//! demo packs, whose rules are read through a `static`.

use bevy::prelude::*;

use super::{changed_domains, EmbeddedPack, PreparedContentPack};

/// Frames between two looks at the files. A stat per file is cheap, and a
/// third of a second is less than the time to change from an editor to the
/// game.
const POLL_FRAMES: u32 = 20;

/// Watch `pack`'s files and publish each admitted edit between sessions.
///
/// `live` are the schemas the game reads where it uses them, each through
/// `pack.prepared()` at that time. `restated` are the schemas it read once at
/// start, each with the function that states it again. With an embedded pack
/// it adds nothing.
///
/// Add it where the game reads the pack: a second add of one pack adds its
/// schemas to the same watch, so the code that reads a schema at start can
/// give its restater beside that read. A host of several demos watches each
/// pack.
#[derive(Clone)]
pub struct EmbeddedPackReloadPlugin {
    pub pack: &'static EmbeddedPack,
    pub live: &'static [&'static str],
    pub restated: Vec<Restater>,
}

/// A schema the game read once at start, and how to state it again.
#[derive(Clone)]
pub struct Restater {
    pub schema: &'static str,
    /// States the schema again from the published generation. It runs
    /// between two sessions, after the publication, when the schema changed.
    pub restate: std::sync::Arc<dyn Fn(&mut World, &PreparedContentPack) + Send + Sync>,
}

impl EmbeddedPackReloadPlugin {
    /// The pack's `room_rules`, declared in `scope` once at start
    /// (`declare_room_rules`), stated again at each publication that changes
    /// them.
    pub fn room_rules(pack: &'static EmbeddedPack, scope: crate::combat::scoped_rules::RulesScope) -> Self {
        use crate::combat::room_rules::{lowered_room_rules, restate_room_rules, ROOM_RULES_SCHEMA};
        Self {
            pack,
            live: &[],
            restated: vec![Restater::new(ROOM_RULES_SCHEMA, move |world, pack| {
                restate_room_rules(world, scope, lowered_room_rules(pack));
            })],
        }
    }
}

impl EmbeddedPackReloadPlugin {
    /// The pack's `prop_sheets`, registered once at start
    /// (`register_prop_sheet_rows`), restated at each publication that
    /// changes them; their sheets are loaded again.
    pub fn prop_sheets(pack: &'static EmbeddedPack) -> Self {
        use crate::sprite_sheet::prop_sheets::{lowered_prop_sheets, PROP_SHEETS_SCHEMA};
        let rows = |pack: &PreparedContentPack| lowered_prop_sheets(pack).cloned().unwrap_or_default();
        Self::owning_rows(pack, PROP_SHEETS_SCHEMA, rows(pack.prepared()).into_keys().collect(), move |world, owned, pack| {
            crate::actors::assets::game_assets::restate_prop_sheet_rows(world, owned, &rows(pack))
        })
    }

    /// The pack's `platform_looks`, registered once at start
    /// (`register_platform_look_rows`), restated at each publication that
    /// changes them.
    #[cfg(feature = "ambition_render")]
    pub fn platform_looks(pack: &'static EmbeddedPack) -> Self {
        use crate::render::rendering::moving_platforms::PlatformLooks;
        use crate::sprite_sheet::prop_sheets::{lowered_platform_looks, PLATFORM_LOOKS_SCHEMA};
        let rows = |pack: &PreparedContentPack| lowered_platform_looks(pack).cloned().unwrap_or_default();
        Self::owning_rows(pack, PLATFORM_LOOKS_SCHEMA, rows(pack.prepared()).into_keys().collect(), move |world, owned, pack| {
            world
                .get_resource_or_insert_with(PlatformLooks::default)
                .restate(owned.iter().map(String::as_str), &rows(pack))
        })
    }

    /// The pack's `projectile_visuals`, registered once at start
    /// (`register_projectile_visual`), restated at each publication that
    /// changes them.
    pub fn projectile_looks(pack: &'static EmbeddedPack) -> Self {
        use crate::projectiles::content_schema::{lowered_projectile_looks, PROJECTILE_VISUALS_SCHEMA};
        use crate::projectiles::ProjectileVisualCatalog;
        let rows = |pack: &PreparedContentPack| lowered_projectile_looks(pack).cloned().unwrap_or_default();
        Self::owning_rows(pack, PROJECTILE_VISUALS_SCHEMA, rows(pack.prepared()).into_keys().collect(), move |world, owned, pack| {
            world
                .get_resource_or_insert_with(ProjectileVisualCatalog::default)
                .restate(owned.iter().map(String::as_str), &rows(pack))
        })
    }

    /// The pack's `music_registry` and `sfx_registry`, registered once at
    /// start as `provider`'s audio fragment, published again at each
    /// publication that changes them (`publish_provider_audio`).
    pub fn audio(pack: &'static EmbeddedPack, provider: &'static str) -> Self {
        use crate::audio::content_schema::{
            lowered_music_registry, lowered_sfx_registry, MUSIC_REGISTRY_SCHEMA, SFX_REGISTRY_SCHEMA,
        };
        let publish = move |world: &mut World, pack: &PreparedContentPack| {
            let music = lowered_music_registry(pack).cloned();
            let sfx = lowered_sfx_registry(pack).cloned();
            if let Err(problem) = crate::audio::catalog::publish_provider_audio(world, provider, music, sfx) {
                error!("the reload of `{}` keeps the audio of `{provider}`: {problem}", pack.id.0);
            }
        };
        // Each restater publishes both registries, so an edit to both runs
        // the same publication twice, with the same result. A registry the
        // pack does not state has no restater.
        let stated = &pack.prepared().schemas;
        let restated = [MUSIC_REGISTRY_SCHEMA, SFX_REGISTRY_SCHEMA]
            .into_iter()
            .filter(|schema| stated.keys().any(|stated| stated.0 == *schema))
            .map(|schema| Restater::new(schema, publish))
            .collect();
        Self { pack, live: &[], restated }
    }

    /// A restater for rows this pack shares a catalog with other providers
    /// in: it remembers the ids the pack owns (`owned`, from the start), so a
    /// restatement replaces only those. On a refusal (an id of another
    /// provider) the catalog keeps the pack's rows, as Ambition's reload does.
    fn owning_rows(
        pack: &'static EmbeddedPack,
        schema: &'static str,
        owned: Vec<String>,
        restate: impl Fn(&mut World, &[String], &PreparedContentPack) -> Result<Vec<String>, String> + Send + Sync + 'static,
    ) -> Self {
        let owned = std::sync::Mutex::new(owned);
        Self {
            pack,
            live: &[],
            restated: vec![Restater::new(schema, move |world, pack| {
                let mut owned = owned.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                match restate(world, &owned, pack) {
                    Ok(now) => *owned = now,
                    Err(problem) => error!("the reload of `{}` keeps its `{schema}` rows: {problem}", pack.id.0),
                }
            })],
        }
    }
}

impl Restater {
    pub fn new(
        schema: &'static str,
        restate: impl Fn(&mut World, &PreparedContentPack) + Send + Sync + 'static,
    ) -> Self {
        Self { schema, restate: std::sync::Arc::new(restate) }
    }
}

impl Plugin for EmbeddedPackReloadPlugin {
    fn build(&self, app: &mut App) {
        let Some(root) = self.pack.root_on_disk() else {
            return;
        };
        // A name the pack does not state is a typo, and it would refuse each
        // edit to the schema it meant.
        let pack = self.pack.prepared();
        for schema in self.live.iter().chain(self.restated.iter().map(|restater| &restater.schema)) {
            assert!(
                pack.schemas.keys().any(|stated| stated.0 == *schema),
                "the live schema `{schema}` is not a schema of the pack `{}`; it states {:?}",
                pack.id.0,
                pack.schemas.keys().map(|stated| stated.0.as_str()).collect::<Vec<_>>(),
            );
        }
        if !app.world().contains_resource::<EmbeddedPackWatches>() {
            app.init_resource::<EmbeddedPackWatches>().add_systems(
                Update,
                (look_at_the_pack_sources, publish_the_staged_pack_between_sessions).chain(),
            );
        }
        let mut watches = app.world_mut().resource_mut::<EmbeddedPackWatches>();
        match watches.0.iter_mut().find(|watch| std::ptr::eq(watch.pack, self.pack)) {
            Some(watch) => {
                for schema in self.live {
                    if !watch.live.contains(schema) {
                        watch.live.push(schema);
                    }
                }
                watch.restated.extend(self.restated.iter().cloned());
            }
            None => watches.0.push(EmbeddedPackWatch::new(self.clone(), root.into())),
        }
    }

    fn is_unique(&self) -> bool {
        false
    }
}

/// One watch for each pack read off disk.
#[derive(Resource, Default)]
pub struct EmbeddedPackWatches(pub Vec<EmbeddedPackWatch>);

impl EmbeddedPackWatches {
    /// The watch of `pack`, when the pack is read off disk.
    pub fn of(&self, pack: &'static EmbeddedPack) -> Option<&EmbeddedPackWatch> {
        self.0.iter().find(|watch| std::ptr::eq(watch.pack, pack))
    }
}

/// What the watch of one pack saw and did. Public for tests and the inspector.
pub struct EmbeddedPackWatch {
    pack: &'static EmbeddedPack,
    live: Vec<&'static str>,
    restated: Vec<Restater>,
    root: std::path::PathBuf,
    files: Vec<(String, Option<std::time::SystemTime>)>,
    frames_until_poll: u32,
    staged: Option<PreparedContentPack>,
    /// Generations this watch published.
    pub published: u32,
    /// Why the last change was refused, while no later change was admitted.
    pub refused: Option<String>,
}

impl EmbeddedPackWatch {
    fn new(plugin: EmbeddedPackReloadPlugin, root: std::path::PathBuf) -> Self {
        let files = stamped(&root, watched_files(&root));
        let EmbeddedPackReloadPlugin { pack, live, restated } = plugin;
        Self { pack, live: live.to_vec(), restated, root, files, frames_until_poll: POLL_FRAMES, staged: None, published: 0, refused: None }
    }

    /// Look at the files at the next update.
    pub fn poll_now(&mut self) {
        self.frames_until_poll = 0;
    }

    fn takes_part(&self, schema: &str) -> bool {
        self.live.contains(&schema) || self.restated.iter().any(|restater| restater.schema == schema)
    }

    /// The schemas this watch publishes: the live ones and the restated ones.
    pub fn schemas(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.live.iter().copied().chain(self.restated.iter().map(|restater| restater.schema))
    }

    /// An admitted change waits for the end of the session.
    pub fn is_staged(&self) -> bool {
        self.staged.is_some()
    }
}

/// The manifest and each source it declares.
fn watched_files(root: &std::path::Path) -> Vec<String> {
    let declared = std::fs::read_to_string(root.join("pack.ron"))
        .map(|manifest| ambition_content_pack::embed::declared_paths(&manifest))
        .unwrap_or_default();
    std::iter::once("pack.ron".to_string()).chain(declared).collect()
}

fn stamped(root: &std::path::Path, files: Vec<String>) -> Vec<(String, Option<std::time::SystemTime>)> {
    files
        .into_iter()
        .map(|path| {
            let seen = modified(&root.join(&path));
            (path, seen)
        })
        .collect()
}

fn modified(path: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn look_at_the_pack_sources(mut watches: ResMut<EmbeddedPackWatches>) {
    for watch in &mut watches.0 {
        look_at(watch);
    }
}

/// On a changed file, compile the pack from disk and stage it or refuse it.
fn look_at(watch: &mut EmbeddedPackWatch) {
    if watch.frames_until_poll > 0 {
        watch.frames_until_poll -= 1;
        return;
    }
    watch.frames_until_poll = POLL_FRAMES;
    let mut changed = false;
    for (path, seen) in &mut watch.files {
        let now = modified(&watch.root.join(path.as_str()));
        if now != *seen {
            *seen = now;
            changed = true;
        }
    }
    if !changed {
        return;
    }
    // The manifest can name other files now. Each is looked at from here on.
    watch.files = stamped(&watch.root, watched_files(&watch.root));
    let candidate = match watch.pack.compile() {
        Ok(candidate) => candidate,
        Err(failure) => {
            let why = format!("the pack `{}` does not compile; the running pack stays:\n{failure}", watch.root.display());
            error!("{why}");
            watch.staged = None;
            watch.refused = Some(why);
            return;
        }
    };
    let running = watch.pack.prepared();
    if candidate.fingerprint == running.fingerprint {
        // A save with no change, or an edit put back before it was published.
        watch.staged = None;
        watch.refused = None;
        return;
    }
    let read_at_start: Vec<String> = changed_domains(running, &candidate)
        .into_iter()
        .map(|schema| schema.0)
        .filter(|schema| !watch.takes_part(schema))
        .collect();
    if !read_at_start.is_empty() {
        let why = format!(
            "the edit to the pack `{}` changes {read_at_start:?}, which the game reads only when it \
             starts; restart the game to play it. The running pack stays.",
            running.id.0
        );
        warn!("{why}");
        watch.staged = None;
        watch.refused = Some(why);
        return;
    }
    info!("the edit to the pack `{}` is admitted; it is played from the next session", running.id.0);
    watch.staged = Some(candidate);
    watch.refused = None;
}

/// Publish the staged generation when no session is live and none is being
/// prepared.
fn publish_the_staged_pack_between_sessions(world: &mut World) {
    let in_a_session = world
        .get_resource::<crate::actor::ActiveSessionScope>()
        .is_some_and(|scope| scope.current().is_some());
    let preparing = world
        .get_resource::<crate::game_shell::ShellRouter>()
        .is_some_and(|router| router.pending.is_some());
    if in_a_session || preparing {
        return;
    }
    let mut to_restate = Vec::new();
    for watch in &mut world.resource_mut::<EmbeddedPackWatches>().0 {
        let Some(generation) = watch.staged.take() else {
            continue;
        };
        let changed = changed_domains(watch.pack.prepared(), &generation);
        info!("the pack `{}` is published again from its files", generation.id.0);
        watch.pack.publish(generation);
        watch.published += 1;
        to_restate.extend(
            watch
                .restated
                .iter()
                .filter(|restater| changed.iter().any(|schema| schema.0 == restater.schema))
                .map(|restater| (watch.pack, restater.restate.clone())),
        );
    }
    for (pack, restate) in to_restate {
        restate(world, pack.prepared());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::room_rules::lowered_room_rules;

    /// A pack in a new directory with one `room_rules` source. Leaked, as a
    /// game's `static` is.
    fn pack_with_rules(name: &str, rules: &str) -> (&'static EmbeddedPack, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("live_reload_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::write(
            root.join("pack.ron"),
            r#"(id: "live_reload_probe", version: "0.1.0", namespace: "probe", requires: [],
                sources: [(path: "data/room_rules.ron", schema: "room_rules", version: 1)])"#,
        )
        .unwrap();
        std::fs::write(root.join("data/room_rules.ron"), rules).unwrap();
        let leaked_root: &'static str = Box::leak(root.to_string_lossy().into_owned().into_boxed_str());
        (Box::leak(Box::new(EmbeddedPack::rooted(leaked_root, None))), root)
    }

    fn app_watching(pack: &'static EmbeddedPack, live: &'static [&'static str]) -> App {
        let mut app = App::new();
        app.init_resource::<crate::actor::ActiveSessionScope>();
        app.add_plugins(EmbeddedPackReloadPlugin { pack, live, restated: Vec::new() });
        app
    }

    fn watch_of<'a>(app: &'a App, pack: &'static EmbeddedPack) -> &'a EmbeddedPackWatch {
        app.world().resource::<EmbeddedPackWatches>().of(pack).expect("the probe pack is read off disk")
    }

    /// Save `text` and let the watch look at it. The file's time is moved on
    /// by one second, so a file system with coarse times sees the change.
    fn save_and_look(app: &mut App, file: &std::path::Path, text: &str) {
        std::fs::write(file, text).unwrap();
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(1);
        std::fs::File::options().write(true).open(file).unwrap().set_modified(later).unwrap();
        for watch in &mut app.world_mut().resource_mut::<EmbeddedPackWatches>().0 {
            watch.poll_now();
        }
        app.update();
    }

    fn heavy(pack: &PreparedContentPack) -> crate::combat::room_rules::RoomRules {
        *lowered_room_rules(pack).expect("the probe pack states room rules")
    }

    /// ⭐ An edit to a live schema is staged while a session is live, and is
    /// what the pack reads from the end of that session. The value read is
    /// the value a new compile of the saved files gives.
    #[test]
    fn an_edit_to_a_live_schema_is_played_from_the_next_session() {
        let (pack, root) = pack_with_rules("next_session", "(heavy_hit_damage: Some(12))");
        let mut app = app_watching(pack, &["room_rules"]);
        let before = heavy(pack.prepared());
        app.world_mut().resource_mut::<crate::actor::ActiveSessionScope>().begin();

        save_and_look(&mut app, &root.join("data/room_rules.ron"), "(heavy_hit_damage: Some(30))");
        let watch = watch_of(&app, pack);
        assert_eq!(watch.published, 0, "nothing is published while a session is live");
        assert_eq!(heavy(pack.prepared()), before, "the live session reads the generation it began with");
        assert!(watch.is_staged(), "the edit is admitted: {:?}", watch.refused);

        app.world_mut().resource_mut::<crate::actor::ActiveSessionScope>().clear();
        app.update();
        let authority = EmbeddedPack::rooted(pack.root_on_disk().unwrap(), None).compile().unwrap();
        assert_eq!(watch_of(&app, pack).published, 1);
        assert_eq!(heavy(pack.prepared()), heavy(&authority), "the pack reads the saved file");
        assert_ne!(heavy(pack.prepared()), before);
        let _ = std::fs::remove_dir_all(root);
    }

    /// A route that is preparing its session holds the publication: the
    /// session it prepares reads one generation from its preparation on.
    #[test]
    fn a_route_that_is_preparing_holds_the_publication() {
        use crate::game_shell::{PendingShellRoute, ShellActivationId, ShellRouteId, ShellRouter};

        let (pack, root) = pack_with_rules("preparing", "(heavy_hit_damage: Some(12))");
        let mut app = app_watching(pack, &["room_rules"]);
        let before = heavy(pack.prepared());
        let mut router = ShellRouter::default();
        router.pending = Some(PendingShellRoute {
            reserved_activation: ShellActivationId(1),
            route_id: ShellRouteId::new("match"),
            push_history: false,
            barrier: crate::load::LoadBarrierRef::new("load", "barrier"),
            requires_prepared_session: true,
            terminal_reported: false,
            request: None,
        });
        app.insert_resource(router);

        save_and_look(&mut app, &root.join("data/room_rules.ron"), "(heavy_hit_damage: Some(30))");
        assert_eq!(watch_of(&app, pack).published, 0, "nothing is published while a route prepares");
        assert_eq!(heavy(pack.prepared()), before);

        app.world_mut().resource_mut::<ShellRouter>().pending = None;
        app.update();
        assert_eq!(watch_of(&app, pack).published, 1);
        assert_ne!(heavy(pack.prepared()), before);
        let _ = std::fs::remove_dir_all(root);
    }

    /// What the probe's restater stated last.
    #[derive(Resource, Default)]
    struct Restated(Vec<crate::combat::room_rules::RoomRules>);

    fn restate_probe(world: &mut World, pack: &PreparedContentPack) {
        world.get_resource_or_insert_with(Restated::default).0.push(heavy(pack));
    }

    /// ⭐ A schema the game read once at start is stated again from the
    /// published generation, between sessions and not before.
    #[test]
    fn a_restated_schema_is_stated_again_from_the_published_generation() {
        let (pack, root) = pack_with_rules("restated", "(heavy_hit_damage: Some(12))");
        let mut app = App::new();
        app.init_resource::<crate::actor::ActiveSessionScope>();
        app.add_plugins(EmbeddedPackReloadPlugin {
            pack,
            live: &[],
            restated: vec![Restater::new("room_rules", restate_probe)],
        });
        app.init_resource::<Restated>();
        app.world_mut().resource_mut::<crate::actor::ActiveSessionScope>().begin();

        save_and_look(&mut app, &root.join("data/room_rules.ron"), "(heavy_hit_damage: Some(30))");
        assert!(app.world().resource::<Restated>().0.is_empty(), "nothing is stated again while a session is live");
        assert!(watch_of(&app, pack).is_staged(), "the edit is admitted: {:?}", watch_of(&app, pack).refused);

        app.world_mut().resource_mut::<crate::actor::ActiveSessionScope>().clear();
        app.update();
        let authority = EmbeddedPack::rooted(pack.root_on_disk().unwrap(), None).compile().unwrap();
        assert_eq!(app.world().resource::<Restated>().0, vec![heavy(&authority)], "stated once, from the saved file");
        let _ = std::fs::remove_dir_all(root);
    }

    /// The room-rules restater a demo gives states the scope's rules again:
    /// the next room of that mode is governed by the saved file.
    #[test]
    fn the_room_rules_restater_states_the_scopes_rules_again() {
        use crate::combat::scoped_rules::{ActiveRoom, DeclaredRules, RulesScope};
        use crate::combat::strike_weight::StrikeWeightRules;

        let (pack, root) = pack_with_rules("room_rules_restater", "(heavy_hit_damage: Some(12))");
        let scope = RulesScope::Mode("probe_mode");
        let mut app = App::new();
        app.add_plugins(EmbeddedPackReloadPlugin::room_rules(pack, scope));
        let mut declared = DeclaredRules::<StrikeWeightRules>::default();
        declared.declare(scope, heavy(pack.prepared()).strike_weight.expect("the probe states a heavy line"));
        app.insert_resource(declared);
        let heavy_line = |app: &App| {
            app.world()
                .resource::<DeclaredRules<StrikeWeightRules>>()
                .governing(ActiveRoom::live(Some("probe_mode")))
                .map(|rules| rules.heavy_at_damage)
        };
        assert_eq!(heavy_line(&app), Some(12));

        save_and_look(&mut app, &root.join("data/room_rules.ron"), "(heavy_hit_damage: Some(30))");
        assert_eq!(heavy_line(&app), Some(30));
        let _ = std::fs::remove_dir_all(root);
    }

    /// A pack in a new directory with one source of `schema` at `path`.
    fn pack_with(name: &str, path: &str, schema: &str, text: &str) -> (&'static EmbeddedPack, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("live_reload_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::write(
            root.join("pack.ron"),
            format!(
                r#"(id: "live_reload_probe", version: "0.1.0", namespace: "probe", requires: [],
                    sources: [(path: "{path}", schema: "{schema}", version: 1)])"#
            ),
        )
        .unwrap();
        std::fs::write(root.join(path), text).unwrap();
        let leaked_root: &'static str = Box::leak(root.to_string_lossy().into_owned().into_boxed_str());
        (Box::leak(Box::new(EmbeddedPack::rooted(leaked_root, None))), root)
    }

    /// ⭐ Rows a pack shares a catalog with: a publication replaces the
    /// pack's own rows (changed, added, dropped) and keeps another
    /// provider's; an edit that takes another provider's id keeps the pack's
    /// rows.
    #[test]
    fn a_pack_restates_only_its_own_rows_of_a_shared_catalog() {
        use crate::actors::assets::game_assets::{PropSheets, PropSheetsAppExt};
        use crate::sprite_sheet::prop_sheets::lowered_prop_sheets;

        let rows_of = |pack: &PreparedContentPack| lowered_prop_sheets(pack).cloned().unwrap_or_default();
        let (pack, root) = pack_with(
            "owned_rows",
            "data/prop_sheets.ron",
            "prop_sheets",
            r#"{ "probe_prop": (frame_sample_inset: 2), "probe_other": () }"#,
        );
        let mut app = App::new();
        app.register_prop_sheet_rows(&rows_of(pack.prepared()));
        let (foreign, foreign_root) = pack_with("owned_rows_foreign", "data/prop_sheets.ron", "prop_sheets", r#"{ "foreign_prop": () }"#);
        app.register_prop_sheet_rows(&rows_of(foreign.prepared()));
        let _ = std::fs::remove_dir_all(foreign_root);
        app.add_plugins(EmbeddedPackReloadPlugin::prop_sheets(pack));
        let kinds = |app: &App| {
            let mut kinds: Vec<String> =
                app.world().resource::<PropSheets>().iter().map(|(kind, _)| kind.to_string()).collect();
            kinds.sort();
            kinds
        };
        let source_of = |app: &App, kind: &str| {
            app.world().resource::<PropSheets>().iter().find(|(known, _)| *known == kind).map(|(_, source)| format!("{source:?}"))
        };
        let file = root.join("data/prop_sheets.ron");

        save_and_look(&mut app, &file, r#"{ "probe_prop": (frame_sample_inset: 5), "probe_new": () }"#);
        assert_eq!(kinds(&app), ["foreign_prop", "probe_new", "probe_prop"]);
        let mut authority = App::new();
        authority.register_prop_sheet_rows(&rows_of(&EmbeddedPack::rooted(pack.root_on_disk().unwrap(), None).compile().unwrap()));
        assert_eq!(source_of(&app, "probe_prop"), source_of(&authority, "probe_prop"), "the row is the saved file's");
        assert_ne!(source_of(&app, "probe_prop"), source_of(&{
            let mut before = App::new();
            before.register_prop_sheet_rows(&rows_of(&{
                let (old, old_root) = pack_with("owned_rows_before", "data/prop_sheets.ron", "prop_sheets", r#"{ "probe_prop": (frame_sample_inset: 2) }"#);
                let compiled = old.compile().unwrap();
                let _ = std::fs::remove_dir_all(old_root);
                compiled
            }));
            before
        }, "probe_prop"), "the edit changed the row");

        save_and_look(&mut app, &file, r#"{ "probe_prop": (), "foreign_prop": () }"#);
        assert_eq!(kinds(&app), ["foreign_prop", "probe_new", "probe_prop"], "another provider's id is not taken");
        assert_eq!(source_of(&app, "probe_prop"), source_of(&authority, "probe_prop"), "the pack's rows are kept");

        // `probe_new` is the pack's since the first publication.
        save_and_look(&mut app, &file, r#"{ "probe_prop": () }"#);
        assert_eq!(kinds(&app), ["foreign_prop", "probe_prop"], "a row the pack added and then dropped is gone");
        let _ = std::fs::remove_dir_all(root);
    }

    /// ⭐ An edit to the pack's music is the provider's music in the audio
    /// catalog from the next session, and another provider's music stays.
    #[test]
    fn an_edit_to_the_music_is_the_providers_music_from_the_next_session() {
        use crate::audio::catalog::{AudioCatalogAppExt, AudioCatalogRegistry};
        use crate::audio::content_schema::lowered_music_registry;

        let (pack, root) = pack_with(
            "audio",
            "data/music_registry.ron",
            "music_registry",
            r#"(default_track: "probe_theme", tracks: [(id: "probe_theme", display_name: "Probe Theme")])"#,
        );
        let (host, host_root) = pack_with(
            "audio_host",
            "data/music_registry.ron",
            "music_registry",
            r#"(default_track: "host_theme", tracks: [(id: "host_theme", display_name: "Host Theme")])"#,
        );
        let mut app = App::new();
        app.init_resource::<crate::actor::ActiveSessionScope>();
        app.register_audio_catalog_fragment(pack.audio("probe"));
        app.register_audio_catalog_fragment(host.audio("host"));
        app.add_plugins(EmbeddedPackReloadPlugin::audio(pack, "probe"));
        let host_music = app.world().resource::<AudioCatalogRegistry>().music_for("host").cloned();
        app.world_mut().resource_mut::<crate::actor::ActiveSessionScope>().begin();

        save_and_look(
            &mut app,
            &root.join("data/music_registry.ron"),
            r#"(default_track: "probe_theme", tracks: [(id: "probe_theme", display_name: "Probe Theme"), (id: "probe_sting", display_name: "Probe Sting", one_shot: true)])"#,
        );
        let tracks = |app: &App| app.world().resource::<AudioCatalogRegistry>().music_for("probe").map(|music| music.tracks.len());
        assert_eq!(tracks(&app), Some(1), "the live session keeps its music");

        app.world_mut().resource_mut::<crate::actor::ActiveSessionScope>().clear();
        app.update();
        let authority = EmbeddedPack::rooted(pack.root_on_disk().unwrap(), None).compile().unwrap();
        let catalog = app.world().resource::<AudioCatalogRegistry>();
        assert_eq!(catalog.music_for("probe"), lowered_music_registry(&authority), "the provider's music is the saved file's");
        assert_eq!(catalog.music_for("host").cloned(), host_music, "another provider's music stays");
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(host_root);
    }

    /// A host of two demos watches both packs, and a second add of one pack
    /// does not watch it twice.
    #[test]
    fn each_pack_is_watched_once() {
        let (first, first_root) = pack_with_rules("first", "(heavy_hit_damage: Some(12))");
        let (second, second_root) = pack_with_rules("second", "(heavy_hit_damage: Some(12))");
        let mut app = app_watching(first, &["room_rules"]);
        app.add_plugins(EmbeddedPackReloadPlugin {
            pack: first,
            live: &["room_rules"],
            restated: vec![Restater::new("room_rules", restate_probe)],
        });
        app.add_plugins(EmbeddedPackReloadPlugin { pack: second, live: &["room_rules"], restated: Vec::new() });
        let watches = app.world().resource::<EmbeddedPackWatches>();
        assert_eq!(watches.0.len(), 2);
        let schemas: Vec<_> = watches.of(first).expect("the first pack is watched").schemas().collect();
        assert_eq!(schemas, ["room_rules", "room_rules"], "the second add gives its restater to the same watch");
        assert!(watches.of(second).is_some());
        let _ = std::fs::remove_dir_all(first_root);
        let _ = std::fs::remove_dir_all(second_root);
    }

    /// A live schema the pack does not state is refused when the plugin is
    /// added.
    #[test]
    #[should_panic(expected = "the live schema `room_rulez` is not a schema of the pack")]
    fn a_live_schema_the_pack_does_not_state_is_refused() {
        let (pack, _root) = pack_with_rules("typo", "(heavy_hit_damage: Some(12))");
        let _ = app_watching(pack, &["room_rulez"]);
    }

    /// The control: the same edit to a schema the game reads only when it
    /// starts is refused, and nothing is published.
    #[test]
    fn an_edit_to_a_schema_read_at_start_is_refused() {
        let (pack, root) = pack_with_rules("read_at_start", "(heavy_hit_damage: Some(12))");
        let mut app = app_watching(pack, &[]);
        let before = heavy(pack.prepared());

        save_and_look(&mut app, &root.join("data/room_rules.ron"), "(heavy_hit_damage: Some(30))");
        app.update();
        let watch = watch_of(&app, pack);
        let why = watch.refused.as_deref().expect("the edit is refused");
        assert!(why.contains("room_rules") && why.contains("restart"), "{why}");
        assert_eq!(watch.published, 0);
        assert_eq!(heavy(pack.prepared()), before);
        let _ = std::fs::remove_dir_all(root);
    }

    /// A value the schema refuses is refused with its field named, and a save
    /// that puts the text back publishes nothing.
    #[test]
    fn a_refused_value_names_its_field_and_an_unchanged_save_publishes_nothing() {
        let (pack, root) = pack_with_rules("refused", "(heavy_hit_damage: Some(12))");
        let mut app = app_watching(pack, &["room_rules"]);
        let file = root.join("data/room_rules.ron");
        let before = heavy(pack.prepared());

        save_and_look(&mut app, &file, "(heavy_hit_damage: Some(0))");
        let why = watch_of(&app, pack).refused.clone().expect("a damage of 0 is refused");
        assert!(why.contains("heavy_hit_damage") && why.contains("room_rules.ron"), "{why}");

        save_and_look(&mut app, &file, "(heavy_hit_damage: Some(12))");
        app.update();
        let watch = watch_of(&app, pack);
        assert!(!watch.is_staged() && watch.refused.is_none() && watch.published == 0);
        assert_eq!(heavy(pack.prepared()), before);
        let _ = std::fs::remove_dir_all(root);
    }
}
