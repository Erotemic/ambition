//! Render a room or route into an offscreen screenshot using the production app
//! composition and camera policy.
//!
//! Usage:
//! `cargo run -p ambition_app_tools --bin capture_scene -- <ROOM_ID> <X,Y|player> [OUT.png] [WIDTHxHEIGHT] [--warmup N] [--character ID] [--include-ui]`
//!
//! For Smash captures, `--press touch:XxY,...` can drive the select screen before
//! capture; keep coordinate recipes covered by host tests because the UI layout can
//! move.

use std::path::PathBuf;

use ambition_app::app::{PresentationSetupSet, StartRoomOverride};
use ambition_platformer2d::assets::materialization::CharacterLoadStates;
use ambition_platformer2d::characters::load_demand::CharacterLoadDemand;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::camera_layers::{FrontHudCamera, MainCamera};
use ambition_platformer2d::platformer::schedule::SimScheduleExt;
use ambition_platformer2d::render::rendering::{
    camera_follow, sync_parallax_layers, CameraViewState,
};
use ambition_platformer2d::sim_view::camera_snapshot::{
    resolve_follow_camera_snapshot, CameraFocus2d, CameraSnapshotResolveInput,
    CameraSnapshotResolveMode,
};
use bevy::app::AppExit;
use bevy::camera::{ImageRenderTarget, RenderTarget};
use bevy::ecs::system::SystemParam;
use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::picking::pointer::{
    Location, PointerAction, PointerButton, PointerId, PointerInput, PointerLocation,
};
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::{TextureFormat, TextureUsages};

#[derive(Resource, Clone, Debug)]
struct SceneCaptureConfig {
    room_id: String,
    focus: ae::Vec2,
    output: PathBuf,
    size: UVec2,
    warmup_frames: u32,
    /// How many shots. `1` (the default) is a single photograph. Higher values
    /// re-arm the capture after each readback and number the files
    /// `<stem>.0000.png`, so a move can be photographed while it plays.
    frames: usize,
    /// Sim frames to advance between shots of a sequence. `1` photographs every
    /// frame; higher samples a long move without hundreds of files.
    stride: u32,
    include_ui: bool,
    /// Frame the whole room instead of a point (`--fit-room`).
    ///
    /// The focus positional shows what is happening at a point; this shows
    /// what the room looks like, where scale problems are visible. It bypasses
    /// the camera snapshot, because that resolver clamps to the room and to
    /// camera zones.
    fit_room: bool,
    /// Optional `character_catalog.ron` id to spawn the player AS (its sprite +
    /// moveset). `None` = the default protagonist. Behind `--character <id>`.
    ///
    /// On `--route smash_gameplay` it seats a match instead (two of this
    /// character), so a smash move is photographed under smash rules.
    character: Option<String>,
    /// When the focus positional is the literal `player`, center the camera on
    /// the live player entity's position after warmup (no coordinate hunting).
    follow_player: bool,
    /// Stand the player beside the first body wearing this catalog character
    /// on a sim tick (`--player-beside ID[@TICK]`), so an interaction with a
    /// body that roams can be photographed without chasing it.
    player_beside: Option<(String, u64)>,
    /// Press Interact on the tick `--player-beside` arrives
    /// (`--interact-on-arrival`). A room capture's keys reach menus and
    /// lobbies, not gameplay, so this is how an interaction is filmed.
    interact_on_arrival: bool,
    /// Press Interact on this sim tick (`--interact-at TICK`): a door or a
    /// switch the player stands at (`--player-at`).
    interact_at: Option<u64>,
    /// World flags to record, each on its sim tick (`--flag NAME[@TICK]`), so
    /// a state the world gets from a flag (a room's look, a gate) can be
    /// photographed, and a change of it can be filmed.
    flags: Vec<(String, u64)>,
    /// Switches to turn on, each on its sim tick (`--switch NAME[@TICK]`), so
    /// what a switch commands (a gate's boot sequence) can be filmed without
    /// a press. A switch is saved state, not a world flag.
    switches: Vec<(String, u64)>,
    /// The hour of the day to set the clock to, on its sim tick
    /// (`--hour H[@TICK]`), to photograph a room at a time of day.
    hour: Option<(f32, u64)>,
    /// Looks to put on bodies, each on its sim tick
    /// (`--wear ID=LOOK[@TICK]`): the first body that wears catalog character
    /// ID wears LOOK from then on, as the conversation's `<<transform>>` asks.
    /// Keys do not reach a conversation's choices, so this is how a look of a
    /// character with a transform is photographed, and its change filmed.
    wears: Vec<(String, String, u64)>,
    /// Put the primary player at a world point on a sim tick
    /// (`--player-at X,Y[@TICK][:VX,VY]`), at rest or launched, one time for
    /// each use of the option: a body that stands in a portal's plane can be
    /// photographed there, a walk through it can be filmed step by step, and
    /// the seam lab launches the robot through a pair.
    player_at: Vec<(Vec2, u64, Vec2)>,
    /// A `--player-at` placement also puts the body in the climbing mode
    /// (`--player-climbs`). No capture key reaches the simulation, so a body
    /// on a ladder is photographed with no body that climbs onto it. The mode
    /// stays only while the body touches a climbable region.
    player_climbs: bool,
    /// One side of a portal window seam check (`--seam PORTAL:window|far`,
    /// read by `scripts/check_portal_window_seams.py`).
    seam: Option<SeamCheck>,
    /// Controls the primary player holds from a sim tick on
    /// (`--player-holds NAMES[@TICK]`). No capture key reaches the
    /// simulation, so this is how a pose that needs a held control (a look
    /// up, a crawl, a guard, a swing in a slow fall) is photographed.
    player_holds: Vec<(HeldControls, u64)>,
    /// Portal shots to fire, each on its sim tick
    /// (`--portal-shot TICK:X,Y:DX,DY[:b]`), so a portal that is placed
    /// and a portal that is replaced can be filmed.
    portal_shots: Vec<PortalShotAt>,
    /// Shots to fire, each on its sim tick (`--shot`), so the look of a
    /// projectile is photographed with no body that fires it.
    shots: Vec<ShotAt>,
    /// Items to throw, each on its sim tick (`--item`), so a prop crossing a
    /// portal is filmed at the speed and the place a scenario wants.
    items: Vec<ItemAt>,
    /// Draw portals with their body pieces only, no view windows
    /// (`--portal-effect off`), so a body crossing a portal is measured as it
    /// is drawn and not again in a window that sees it.
    portal_effect_off: bool,
    /// Draw these limbs of a part-drawn body (their `limb:<slot>` tracks)
    /// nowhere, in the body or in its limbs' twins (`--limbs-unseen`); empty
    /// for every limb. A film with the limbs and one without differ by the
    /// limbs' pixels alone, so a limb's picture can be measured through a
    /// portal pair (`scripts/check_limb_through_portal.py`).
    limbs_unseen: Option<Vec<String>>,
    /// The pairs carry what goes through them (`--carry on|off`): off draws
    /// each drawable whole on its side, to compare a film against
    /// (`scripts/check_portal_carry.py`).
    carry: bool,
    /// Worn gear the primary player puts on, each from a sim tick
    /// (`--gear ID[@TICK]`): a gear id of the `worn_gear` content.
    gear: Vec<(String, u64)>,
    /// Advance exactly one sim tick each frame (`--tick-per-frame`), so a
    /// film is the same each run and its shots are a known number of ticks
    /// apart, whatever the time a frame takes to write.
    tick_per_frame: bool,
    /// Bubble shields to draw, each at a place (`--guard`), so the look of a
    /// guard in a state is photographed with no body that holds one.
    guards: Vec<GuardAt>,
    /// Take every part-drawn body apart by a teleport warp, again and again
    /// (`--body-warp out|in[@SECONDS]`), so the warp can be photographed
    /// without a blink.
    body_warp: Option<ambition_platformer2d::render::rendering::actors::rigged::BodyWarpPreview>,
    /// Keep the developer overlays in the shot (`--dev-overlays`). By default a
    /// verification screenshot shows the product.
    dev_overlays: bool,
    /// Draw the navigation graph in the shot (`--nav-overlay`).
    nav_overlay: bool,
    /// Put the combat debug view in the shot (`--combat-overlay`).
    ///
    /// `--dev-overlays` only stops the tool from hiding what a build shows.
    /// The combat volumes are a settings-menu preset, off by default. This
    /// turns on the `DebugViewMode::Combat` preset, the same state the menu
    /// produces, so the result is a configuration a player can reach.
    combat_overlay: bool,
    /// Hold every boss's health at this fraction of its max (`--boss-hp 0.5`),
    /// so art a boss wears by its wounds — the Flying Spaghetti Monster's sauce
    /// — can be photographed without fighting it there.
    boss_hp: Option<f32>,
    /// Blows on each boss (`--boss-blow TICK[xHEARTS]`): on that sim tick the
    /// boss loses that much health, so what a boss does when a blow lands on
    /// it can be photographed without a player that swings.
    boss_blows: Vec<(u64, i32)>,
    /// The fog in front of the sky of the two-state room look
    /// (`--look-depth FOG,PATCHES`), to tune it with photographs: the numbers
    /// of `RoomLookDepth`, in its field order.
    look_depth: Option<[f32; 2]>,
    /// `--camera-zoom NAME`: how much of the world the camera shows
    /// (`CameraZoomPreset`). A wider view shows what a default capture can
    /// not, like the second image in a row of portal images.
    camera_zoom: Option<ambition_platformer2d::persistence::settings::video::CameraZoomPreset>,
    /// The look of the portal windows (`--portal-windows`): seamless unless
    /// asked, whatever the user's own settings say, so a seam check measures
    /// a seamless window.
    portal_windows: ambition_platformer2d::persistence::settings::video::PortalWindowLook,
    /// Screen post-process effects to force on (`--screen-effect crt,vignette`).
    ///
    /// Without this flag a capture cannot show post-process. The effects are
    /// `UserSettings.video.shaders` state, every strength defaults to zero, and
    /// a windowless host uses `PersistenceRoot::isolated()` (a fresh temp
    /// directory), so the player's `settings.ron` is not read. This writes the
    /// same fields the settings menu writes.
    screen_effects: Vec<ScreenEffect>,
    /// Input to deliver after reaching the route and before the shutter:
    /// key taps and glass taps (`--press touch:167x523,touch:167x523`).
    ///
    /// A generic input vocabulary, not a `--smash-cpu` flag, so any route with
    /// a lobby can use it.
    ///
    /// A key has no position: a bare `Enter` commits wherever the select
    /// cursor sits. `smash_in_the_host.rs` seats a fighter by moving the cursor
    /// to a rect centre and then pressing Enter. `touch:XxY` carries a
    /// position through two real `TouchInput` messages folded by Bevy's
    /// `touch_screen_input_system`, the seam `select_screen::touch_tests`
    /// covers. See [`PressStep::Touch`]. Arrow keys still suit lists (launcher
    /// rows, menus) and gameplay.
    ///
    /// A finger also drives a Bevy picking pointer on the capture image (see
    /// [`Glass`]), so a tap reaches the cube menu and is blocked by whatever
    /// node covers it, as on a phone. `touch:AxB>CxD` is a drag.
    press: Vec<PressStep>,
    /// Open the shutter N press-driving frames in, instead of after the
    /// sequence completes (`--press-during N`).
    ///
    /// The default shutter waits for the sequence to finish and for the state
    /// it asked for. That cannot capture a frame that exists only while input
    /// is delivered (a page turn's first frame, a held key's pose).
    ///
    /// `None` keeps the default behaviour. `Some(n)` shoots on the frame after
    /// `n` frames of press driving, so `--press Enter --press-during 1` shows
    /// the frame where Enter is still held (a tap is two frames).
    ///
    /// If the sequence is spent before frame `n`, that is a hard failure: the
    /// post-press image would be the wrong moment.
    press_during: Option<u32>,
    /// Photograph a shell route instead of a room (`--route <id>`). The
    /// launcher, startup cards, versus stage, and HUD are routes.
    route: Option<String>,
}

#[derive(Resource, Clone, Debug)]
struct SceneCaptureTarget {
    image: Handle<Image>,
}

#[derive(Resource, Debug, Default)]
struct SceneCaptureRuntime {
    frames: u32,
    wait_frames: u32,
    requested: bool,
    completed: bool,
    failed: bool,
    /// Route mode only: the shell has been told where to go. One request, not
    /// one per frame — a `GoTo` every update would restart the route forever.
    route_requested: bool,
    /// How far through `--press` the driver is, and whether the tapped key is
    /// still held. A press and its release are two frames because the driven
    /// surfaces read edges.
    press_cursor: usize,
    press_held: Option<KeyCode>,
    /// Which shot of a sequence this is.
    shot: usize,
    /// Frames still to wait before re-arming the next shot of a sequence.
    stride_left: u32,
    /// The finger a `touch:X,Y` step put down and has not lifted yet. Same
    /// two-frame shape as a key tap, but separate from `press_held` because
    /// the two use different seams.
    touch_held: Option<Vec2>,
    /// Where a `touch:A>B` drag still has to pass through, last point first.
    touch_path: Vec<Vec2>,
    /// The id the next finger goes down with. Fresh per tap: `Touches` keys
    /// everything by id, and reusing an id not yet retired is a state a phone
    /// never produces.
    next_touch_id: u64,
    /// Frames left on a `wait` step.
    press_wait: u32,
    /// Frames the press driver has run for: the clock `--press-during` counts
    /// in. Unlike `frames`, it excludes warmup and is not zeroed when a
    /// sequence completes.
    press_frames: u32,
    /// The sequence ran out before the `--press-during` frame. A flag, not an
    /// immediate exit, because `complete_press_sequence_if_spent` has no
    /// `Commands`.
    press_during_missed: bool,
    /// The frame the last key was released on. The sequence usually starts a
    /// route change, so the shutter waits for the requested state instead of
    /// the moment the presses were accepted.
    press_done_frame: Option<u32>,
    /// How many cameras the route capture has adopted, so the count is
    /// printed when it changes, not every frame.
    cameras_adopted: usize,
    /// Has the world this capture photographs finished being built?
    ///
    /// Warmup counts from here, not from boot. Otherwise the body lands on a
    /// different idle tick each run, and because nameplate opacity is ranked
    /// by distance from the focus, small drift reorders the labels.
    world_ready: bool,
    /// The world was ready one time or more. A press sequence sets
    /// `world_ready` back to wait for the state it asked for; this stays.
    ever_ready: bool,
    /// Frames a capture that follows the player has waited, after its press
    /// sequence, for a player that is not there.
    no_player_after_press: u32,
}

const USAGE: &str = "\
capture_scene — photograph a room or a shell route through the real render stack.

USAGE:
    capture_scene <ROOM_ID> <X,Y|player> [OUT.png] [WIDTHxHEIGHT] [OPTIONS]
    capture_scene --route <ROUTE_ID> [OUT.png] [WIDTHxHEIGHT] [OPTIONS]

OPTIONS:
    --warmup N          frames to settle before the first shot [default: 60]
    --frames N          take N shots, numbered <stem>.NNNN.png [default: 1]
    --stride K          sim frames between shots of a sequence [default: 1]
    --character ID      spawn the player AS this catalog character
                        (with --route smash_gameplay: seats a match of two)
    --route ID          photograph a shell route instead of a room
    --press SEQ         drive input first, e.g. `Down,Enter` or `touch:167x523`
                        (`hold:up` / `release:up` / `wait:30` also work).
                        `touch:AxB>CxD` drags a finger from A,B to C,D.
                        Keys reach menus and lobbies, not gameplay.
                        A sequence that ends the session (a quit to the
                        title) leaves no player: name the focus as X,Y.
    --player-beside ID[@TICK]
                        stand the player at the edge of the first body
                        wearing catalog character ID on sim tick TICK
                        [default: 60]
    --interact-on-arrival
                        press Interact on the tick --player-beside arrives
    --interact-at TICK  press Interact on sim tick TICK (a door or a switch
                        the player stands at, --player-at)
    --body-warp out|in[@SECONDS]
                        take every part-drawn body apart by the teleport warp
                        of a blink, one time each SECONDS [default: 1.0], on
                        the row it draws
    --wear ID=LOOK[@TICK]
                        the first body wearing catalog character ID wears
                        LOOK from sim tick TICK [default: 60], as a
                        conversation's `<<transform>>` asks; repeat for more.
                        A look with a `transform` row plays it.
    --flag NAME[@TICK]  record the world flag NAME on sim tick TICK
                        [default: 1]; repeat for more flags. A room that reads
                        its state from a flag is photographed in that state.
    --hour H[@TICK]     set the time of day to hour H (0 to 24) at sim tick
                        TICK [default: 2], under the game's day cycle: a
                        sunlit room's light and a room's music follow it.
    --switch NAME[@TICK]
                        turn the switch NAME on at sim tick TICK [default: 1],
                        as a press would; repeat for more. Films what a switch
                        commands, such as a gate portal booting.
    --boss-blow TICK[xHEARTS|xall]
                        take HEARTS [default: 1] of each boss's health on sim
                        tick TICK, and flash it, as a blow that lands does;
                        repeat for more blows. A blow leaves a boss 1 heart;
                        `xall` is the killing blow, which films its death. A boss that
                        refuses damage (asleep, in its intro) takes none. What a boss does when it is hurt is
                        photographed with no player that swings.
    --player-at X,Y[@TICK][:VX,VY]
                        put the player at world X,Y (its centre) on sim tick
                        TICK [default: 60], at rest or moving at VX,VY px/s;
                        repeat for more places
    --player-climbs     a --player-at placement also puts the body in the
                        climbing mode, to photograph it on a ladder; give a
                        point in a climbable region
    --player-holds NAMES[@TICK]
                        the player holds the controls NAMES from sim tick TICK
                        [default: 60]: up, down, left, right, walk, jump,
                        attack, special, shield, projectile (the fire
                        button, held to charge), joined with `+`, or `none`.
                        A button the hold before did not have down is pressed
                        on TICK and held after it. Repeat for a sequence; the
                        last hold that started applies
    --portal-shot TICK:X,Y:DX,DY[:b]
                        fire a portal shot on sim tick TICK from world X,Y
                        along DX,DY: end A of the capture's own pair, or end
                        B; repeat for more shots. A second shot of one end
                        replaces the first portal of that end. The pair is
                        not a gun's: a gun's portals go when no gun is in the
                        room.
    --shot TICK:X,Y:DX,DY:LOOK
                        fire a projectile on sim tick TICK from world X,Y
                        along DX,DY, to photograph its look; repeat for more.
                        LOOK is a projectile look id: fireball, hadouken and
                        hadouken_super are the named kinds (their own body
                        and flight); any other id flies straight on the body
                        of a plain ranged shot; `-` is a shot that names no
                        look. The player owns the shot.
    --tick-per-frame    advance exactly one sim tick each frame, and hold the
                        clock while a shot is read back: shots are --stride
                        ticks apart and a film is the same each run (each
                        shot logs its sim tick)
    --seam PORTAL:window|far
                        one side of a portal window seam check
                        (`scripts/check_portal_window_seams.py`). Both sides
                        hide the parallax layer in front of the play, which
                        is not seen through a portal. `window` writes
                        <OUT>.seam.json beside the shot: the window of PORTAL
                        in pixels, and the map from each pixel of it to the
                        pixel of the `far` shot that shows the same place.
                        `far` turns the windows off and puts the camera where
                        the view through PORTAL is. PORTAL is a channel (as
                        `c131`) or a world X,Y (the portal nearest to it).
    --portal-effect off|cones
                        draw portals with body pieces only (`off`), no view
                        windows, or with the windows (`cones`, the default)
    --carry on|off      the portal pairs carry what goes through them (on, the
                        default), or draw each drawable whole on its side (off,
                        to compare a film against)
    --gear ID[@TICK]    the player wears the worn gear ID (a `worn_gear` id, as
                        `space_helmet`) from sim tick TICK [default: 1]; repeat
                        for more
    --limbs-unseen all|SLOT[,SLOT]
                        draw no limb (or these limbs, as `hand_right`) of a
                        part-drawn body: its `limb:<slot>` tracks, in the body
                        and in the limb's twin. With --tick-per-frame, a film
                        with the limbs and one without differ by the limbs'
                        pixels alone
    --item TICK:X,Y:VX,VY:ID
                        throw held item ID into the world on sim tick TICK at
                        world X,Y with velocity VX,VY (px/s), as a hand
                        releases one; repeat for more. `seam_gauge` is the
                        plate the seam lab measures portals with.
    --guard X,Y:INTEGRITY[:parry|:hit|:blow@PHASE]
                        draw a bubble shield at world X,Y, INTEGRITY whole
                        (1 to 0); `parry` draws it in a parry, `hit` gives it
                        a blow two times each second, `blow@PHASE` holds one
                        moment of the beat of a blow (0 to 1); repeat for
                        more. No body holds it: it is to photograph the look.
    --press-during N    open the shutter N press-driving frames in, INSTEAD of
                        after the sequence finishes — the only way to photograph
                        a frame that exists only WHILE an input is being
                        delivered (a menu's opening frame, a key's held pose).
                        `--press Enter --press-during 1` shoots with Enter still
                        down; a tap's press and release are two frames. With
                        `--frames N` the stride is counted in press frames too.
                        ⛔ Fails (exit 2) rather than falling back to the
                        ordinary capture if the sequence runs out first.
    --include-ui        keep the game's UI in the shot
    --dev-overlays      stop silencing the developer chrome
    --nav-overlay       draw the surface graph a navigating body is advised from
                        (surfaces green, hops yellow, drops orange)
    --combat-overlay    force the COMBAT gizmos on (hitboxes, collision boxes)
    --look-depth FOG,PATCHES
                        the fog in front of the sky of the two-state room look
                        (`RoomLookDepth`): how much, from 0 to 1, and how much
                        it is in patches, from 0 to 1. The default is 0.22,0.5.
    --portal-windows seamless|tinted-blue|tinted-gold
                        the look of the portal windows (the player's
                        `Video > Portal Windows`); seamless by default,
                        whatever the user's settings say
    --camera-zoom NAME  how much of the world the camera shows: duel (the
                        default, 568 by 320 units), tight, combat, arena,
                        cinematic or debug (1600 by 900)
                        The blur of the sky is in its art (`room_look_sky.py`).
    --boss-hp F         hold every boss's health at fraction F of its max, to
                        photograph art it wears by its wounds
    --screen-effect E   force screen post-process effects on, comma separated:
                        crt, grain, vignette, robot, underwater, deep_dream.
                        The `low` tier draws them at half strength; name
                        `high` or `ultra` for the full effect. The tool says
                        so when a tier scales them to nothing.

QUALITY:
    A capture is taken at the `low` visual quality profile unless
    `AMBITION_QUALITY_PROFILE` names another (potato, low, medium, high,
    ultra). `low` has the backdrop (two layers), sprites at half resolution
    and screen shaders at half strength. To judge art, name `high` or `ultra`.
    Do not use `potato` to look at the game: it is the tier for a machine that
    can not run it, with no backdrop and no screen shader. Measured 2026-09-01
    in `water_world`: sprite coverage 631,267 at potato against 14,564,876 at
    ultra (world units).
    --fit-room          frame the whole room instead of a point
    -h, --help          print this and exit

NOTES:
    `--dev-overlays` does NOT turn the gizmo pass on — it only stops this tool
    silencing the HUD. For boxes in the shot you want `--combat-overlay`.

    --frames is what makes an animation: a single shot keeps the exact output
    path you named, a sequence numbers its files.
";

fn main() {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        print!("{USAGE}");
        return;
    }
    let config = match SceneCaptureConfig::from_args(std::env::args().skip(1).collect()) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}\n");
            eprint!("{USAGE}");
            std::process::exit(2);
        }
    };

    eprintln!(
        "capture_scene: visual quality `{}`",
        ambition_platformer2d::render::capture::name_capture_quality_profile()
    );
    match config.route.as_deref() {
        Some(route) => eprintln!(
            "capture_scene: route={route} size={}x{} out={}",
            config.size.x,
            config.size.y,
            config.output.display(),
        ),
        None => eprintln!(
            "capture_scene: room={} focus=({:.1},{:.1}) size={}x{} out={}",
            config.room_id,
            config.focus.x,
            config.focus.y,
            config.size.x,
            config.size.y,
            config.output.display(),
        ),
    }

    let mut app = build_capture_app(&config);

    match config.route.clone() {
        Some(route_id) => install_route_capture(&mut app, route_id),
        None => install_room_capture(&mut app),
    }
    // The app's exit is the tool's. Dropped, each failure below exited 0, and
    // a script that asked `$?` was told a capture with no image had passed.
    if let AppExit::Error(code) = app.run() {
        std::process::exit(i32::from(code.get()));
    }
}

/// The one app this tool builds.
///
/// `build_visible_app_with` gives a hook to add capture systems to the
/// production composition. A room and a route differ only by one boolean
/// (the initial route) and by which capture systems are installed.
fn build_capture_app(config: &SceneCaptureConfig) -> App {
    // Always the offscreen-GPU mode. `NoWindow` sets `backends: None` and has
    // no render app, so a readback could never complete.
    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::OffscreenGpu,
        // This boolean chooses the initial route; both arms are shell-hosted.
        // `true` boots the launcher, where a `--route` capture navigates from.
        // `false` boots straight to gameplay, which is a room capture (and
        // what `--direct` and every `--start-room` alias mean).
        config.route.is_some(),
        |app| {
            // The engine's log, which every windowless host disables.
            //
            // `build_visible_app` drops `LogPlugin` for `NoWindow` and
            // `OffscreenGpu` because tests build several Apps per process and
            // the tracing subscriber is process-global. A capture builds one
            // App and exits. Added in the hook, before the simulation plugins
            // build: Bevy makes a system's span when the system is added, and a
            // span made before the subscriber exists stays off, so Tracy and
            // `perf_probe` would not see that system.
            app.add_plugins(bevy::log::LogPlugin {
                #[cfg(feature = "perf_probe")]
                custom_layer: ambition_platformer2d::dev_tools::perf_probe::log_layer,
                ..Default::default()
            });
            if config.route.is_none() {
                app.insert_resource(StartRoomOverride(config.room_id.clone()));
                // Refuse an unknown room: otherwise the capture silently
                // photographs the hub and exits 0.
                app.insert_resource(ambition_app::app::StartRoomMustResolve);
            }
            // Optional "play as this character" override.
            if let Some(character_id) = config.character.clone() {
                eprintln!("capture_scene: player wears character '{character_id}'");
                app.insert_resource(ambition_app::app::StartingCharacterOverride(
                    ambition_platformer2d::actors::avatar::StartingCharacter::new(character_id),
                ));
            }
        },
    );
    // The surface this run draws to (queue Z′8). A capture that cannot show
    // a layout would show a different layout convincingly.
    app.insert_resource(
        ambition_platformer2d::host::gameplay_presentation::HeadlessDisplaySurface(
            ambition_platformer2d::engine_core::Vec2::new(
                config.size.x as f32,
                config.size.y as f32,
            ),
        ),
    );
    app.insert_resource(config.clone());
    if let Some(preview) = config.body_warp {
        app.insert_resource(preview);
    }
    app.insert_resource(SceneCaptureRuntime::default());
    app
}

/// `--boss-hp`: hold every boss at the configured fraction of its health, every
/// frame, so nothing the fight does moves it off the state being photographed.
/// A sim system: health is rollback state, and a write from outside the
/// rewinding schedule would survive a rewind.
/// `--look-depth`: write the numbers of the sky of the two-state look. The
/// developer inspector writes the same resource in a window.
fn force_look_depth(
    config: Res<SceneCaptureConfig>,
    depth: Option<ResMut<ambition_content::presentation::room_look::RoomLookDepth>>,
) {
    let (Some([fog, fog_patches]), Some(mut depth)) = (config.look_depth, depth) else {
        return;
    };
    let wanted = ambition_content::presentation::room_look::RoomLookDepth { fog, fog_patches };
    if *depth != wanted {
        *depth = wanted;
    }
}

/// `--portal-windows`: write the window look of the user settings, every
/// frame, as `force_camera_zoom` does.
fn force_portal_windows(
    config: Res<SceneCaptureConfig>,
    mut settings: ResMut<ambition_platformer2d::persistence::settings::UserSettings>,
) {
    if settings.video.portal_windows != config.portal_windows {
        settings.video.portal_windows = config.portal_windows;
    }
}

/// `--camera-zoom`: write the zoom of the user settings. Every frame, as
/// `force_screen_effects` does: the settings load writes this state too.
fn force_camera_zoom(
    config: Res<SceneCaptureConfig>,
    mut settings: ResMut<ambition_platformer2d::persistence::settings::UserSettings>,
) {
    if let Some(zoom) = config.camera_zoom {
        if settings.video.camera_zoom != zoom {
            settings.video.camera_zoom = zoom;
        }
    }
}

fn hold_boss_health(
    config: Res<SceneCaptureConfig>,
    mut bosses: Query<
        &mut ambition_platformer2d::characters::actor::BodyHealth,
        With<ambition_platformer2d::boss_encounter::BossConfig>,
    >,
) {
    let Some(fraction) = config.boss_hp else {
        return;
    };
    for mut health in &mut bosses {
        let want = ((health.max() as f32) * fraction).ceil().max(1.0) as i32;
        if health.current() != want {
            health.health.current = want;
        }
    }
}

/// `--boss-blow TICKxall`: the blow that kills.
const KILLING_BLOW: i32 = i32::MAX;

/// `--boss-blow`: take the configured health from each boss on its tick, and
/// flash it as a blow that lands does. A sim system that names its tick, as
/// [`place_player_at`] is.
fn blow_on_bosses(
    config: Res<SceneCaptureConfig>,
    tick: Res<ambition_platformer2d::time::SimTick>,
    mut bosses: Query<
        (
            &mut ambition_platformer2d::characters::actor::BodyHealth,
            &mut ambition_platformer2d::characters::actor::BodyCombat,
            &mut ambition_platformer2d::boss_encounter::BossEncounter,
        ),
        With<ambition_platformer2d::boss_encounter::BossConfig>,
    >,
) {
    // The flash of a blow a boss's limb passed on (`PASSED_BLOW_FLASH_S`).
    const BLOW_FLASH_S: f32 = 0.18;
    for (_, hearts) in config.boss_blows.iter().filter(|(on_tick, _)| *on_tick == tick.get()) {
        for (mut health, mut combat, mut status) in &mut bosses {
            // A blow lands only where one can: not on a boss whose encounter
            // refuses damage (asleep, in its intro: a successor waiting for
            // its predecessor to fall).
            if status.encounter.as_ref().is_none_or(|state| state.boss_invulnerable()) {
                continue;
            }
            let floor = if *hearts == KILLING_BLOW { 0 } else { 1 };
            health.health.current = (health.current() - hearts).max(floor);
            combat.hit_flash = BLOW_FLASH_S;
            // The killing blow is the death edge, as a landed hit's is
            // (`boss_hit`): the encounter goes to `Death` on this tick.
            if *hearts == KILLING_BLOW {
                if let Some(state) = status.encounter.as_mut() {
                    let _ = state.kill();
                }
            }
        }
        eprintln!("capture_scene: a blow of {hearts} on each boss on tick {}", tick.get());
    }
}

/// `--flag`: record each configured world flag on its tick, through the
/// world-fact domain's own request, as an authored `world.set_flag` does. A
/// sim system that names its tick: a replay of that tick asks again, and a
/// replay of any other does not.
fn record_flags(
    config: Res<SceneCaptureConfig>,
    tick: Res<ambition_platformer2d::time::SimTick>,
    mut requests: MessageWriter<ambition_platformer2d::combat::events::SetFlagRequested>,
) {
    for (id, on_tick) in &config.flags {
        if tick.get() == *on_tick {
            requests.write(ambition_platformer2d::combat::events::SetFlagRequested {
                id: id.clone(),
                on: true,
            });
            eprintln!("capture_scene: recorded the flag {id} on tick {on_tick}");
        }
    }
}

/// `--hour`: set the time of day on its tick, under the game's day cycle. A sim
/// system that names its tick, as [`record_flags`] is.
fn set_the_hour(
    config: Res<SceneCaptureConfig>,
    tick: Res<ambition_platformer2d::time::SimTick>,
    cycle: Option<Res<ambition_platformer2d::time::DayCycle>>,
    clock: Option<ResMut<ambition_platformer2d::time::DayClock>>,
) {
    let (Some((hour, on_tick)), Some(cycle), Some(mut clock)) = (config.hour, cycle, clock) else {
        return;
    };
    if tick.get() == on_tick {
        *clock = ambition_platformer2d::time::DayClock::at_hour(&cycle, hour);
        eprintln!("capture_scene: set the hour to {hour} at tick {on_tick}");
    }
}

/// `--switch`: turn each configured switch on at its tick, in the save, where a
/// press's toggle writes it. A sim system that names its tick, as
/// [`record_flags`] is.
fn turn_on_switches(
    config: Res<SceneCaptureConfig>,
    tick: Res<ambition_platformer2d::time::SimTick>,
    mut save: ResMut<ambition_platformer2d::persistence::save::AmbitionGameSave>,
) {
    for (id, on_tick) in &config.switches {
        if tick.get() == *on_tick {
            save.data_mut().set_switch(id, true);
            eprintln!("capture_scene: turned the switch {id} on at tick {on_tick}");
        }
    }
}

/// `--wear`: ask the first body that wears a character to wear a look, on its
/// tick, through the request a conversation's `<<transform>>` writes. A sim
/// system that names its tick, as [`record_flags`] is.
fn ask_bodies_to_wear(
    config: Res<SceneCaptureConfig>,
    tick: Res<ambition_platformer2d::time::SimTick>,
    ids: ambition_platformer2d::platformer::lifecycle::LiveBodies,
    bodies: Query<(Entity, &ambition_platformer2d::characters::actor::WornCharacter)>,
    mut requests: MessageWriter<ambition_platformer2d::actors::features::WearRequested>,
) {
    for (wearer, look, on_tick) in &config.wears {
        if tick.get() != *on_tick {
            continue;
        }
        let body = bodies
            .iter()
            .find(|(_, worn)| worn.id() == wearer.as_str())
            .and_then(|(entity, _)| ids.id_of(entity));
        let Some(body) = body else {
            eprintln!("capture_scene: --wear found no body wearing {wearer} on tick {on_tick}");
            continue;
        };
        requests.write(ambition_platformer2d::actors::features::WearRequested {
            body,
            character: ambition_platformer2d::characters::actor::WornCharacter::new(look.as_str())
                .character()
                .clone(),
        });
        eprintln!("capture_scene: asked the body wearing {wearer} to wear {look} on tick {on_tick}");
    }
}

/// `--gear`: the primary player puts on each gear on its tick, as a
/// conductor's `ambition.boss.wear` does: an equipment row in its exclusive
/// slot.
fn put_on_gear(
    mut commands: Commands,
    config: Res<SceneCaptureConfig>,
    tick: Res<ambition_platformer2d::time::SimTick>,
    mut player: Query<
        (Entity, Option<&mut ambition_platformer2d::characters::equipment::WornEquipment>),
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
) {
    use ambition_platformer2d::characters::equipment::{EquipmentRow, WornEquipment};
    for (id, on_tick) in &config.gear {
        if tick.get() != *on_tick {
            continue;
        }
        let Some((body, worn)) = player.iter_mut().next() else {
            eprintln!("capture_scene: --gear found no player on tick {on_tick}");
            continue;
        };
        let row = EquipmentRow { id: id.clone(), exclusive_slot: Some("head".into()), ..Default::default() };
        match worn {
            Some(mut worn) => worn.equip(row),
            None => {
                commands.entity(body).insert(WornEquipment::new(vec![row]));
            }
        }
        eprintln!("capture_scene: the player put on {id} on tick {on_tick}");
    }
}

/// One `--player-holds`: the controls of the primary player from a sim tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct HeldControls {
    axis: Vec2,
    jump: bool,
    attack: bool,
    special: bool,
    shield: bool,
    projectile: bool,
}

impl HeldControls {
    /// `NAMES`: names joined with `+`. `none` holds nothing.
    fn parse(names: &str) -> Result<Self, String> {
        let mut held = Self::default();
        let mut walk = false;
        for name in names.split('+').map(str::trim) {
            match name.to_ascii_lowercase().as_str() {
                "none" => {}
                "up" => held.axis.y = -1.0,
                "down" => held.axis.y = 1.0,
                "left" => held.axis.x = -1.0,
                "right" => held.axis.x = 1.0,
                "walk" => walk = true,
                "jump" => held.jump = true,
                "attack" => held.attack = true,
                "special" => held.special = true,
                "shield" => held.shield = true,
                "projectile" => held.projectile = true,
                other => {
                    return Err(format!(
                        "--player-holds does not know '{other}'. Known: up, down, left, right, walk, \
                         jump, attack, special, shield, projectile, none"
                    ))
                }
            }
        }
        if walk {
            // The walk key of the keyboard: the stick at half.
            held.axis *= 0.5;
        }
        Ok(held)
    }

    /// The frame of one tick. `before` is the hold this one took over from, on
    /// the tick it starts, and `None` on each tick after: a button that the
    /// hold before did not have down is PRESSED on that tick, and held after.
    fn frame(self, before: Option<Self>) -> ae::ControlFrame {
        let pressed = |now: bool, was: fn(Self) -> bool| now && before.is_some_and(|before| !was(before));
        ae::ControlFrame {
            axis_x: self.axis.x,
            axis_y: self.axis.y,
            jump_held: self.jump,
            jump_pressed: pressed(self.jump, |held| held.jump),
            attack_held: self.attack,
            attack_pressed: pressed(self.attack, |held| held.attack),
            special_held: self.special,
            special_pressed: pressed(self.special, |held| held.special),
            shield_held: self.shield,
            projectile_held: self.projectile,
            projectile_pressed: pressed(self.projectile, |held| held.projectile),
            // A charged shot fires on the release edge.
            projectile_released: !self.projectile && before.is_some_and(|before| before.projectile),
            ..Default::default()
        }
    }
}

/// `--player-holds`: give the primary seat the controls of the last hold that
/// has started. It writes the seat's row of `SlotControls`, after the tick
/// took the input of the devices and before a body reads it: a sample of a
/// device (which holds nothing here) cannot replace it.
fn hold_player_controls(
    config: Res<SceneCaptureConfig>,
    tick: Res<ambition_platformer2d::time::SimTick>,
    mut slots: ResMut<ambition_platformer2d::sim::SlotControls>,
) {
    let now = tick.get();
    let Some((held, from)) =
        config.player_holds.iter().copied().filter(|(_, from)| *from <= now).max_by_key(|(_, from)| *from)
    else {
        return;
    };
    // The hold that was in force on the tick before this one started.
    let before = (now == from).then(|| {
        config
            .player_holds
            .iter()
            .copied()
            .filter(|(_, earlier)| *earlier < from)
            .max_by_key(|(_, earlier)| *earlier)
            .map_or_else(HeldControls::default, |(held, _)| held)
    });
    slots.set(ambition_platformer2d::sim::PlayerSlot::PRIMARY, held.frame(before));
    if now == from {
        eprintln!("capture_scene: the player holds {held:?} from tick {from}");
    }
}

/// The ticks after a `--player-climbs` placement for which the body is put
/// back in its place, in the climbing mode.
const PLAYER_CLIMBS_HOLD_TICKS: u64 = 3;

/// `--player-at`: put the primary player at the configured point on its tick,
/// at rest or launched, through the motion authority (ADR 0024). A sim system that names
/// its tick, as [`place_player_beside`] is.
fn place_player_at(
    config: Res<SceneCaptureConfig>,
    tick: Res<ambition_platformer2d::time::SimTick>,
    mut player: Query<
        (ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel),
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
) {
    // A body that climbs is held in its place for some ticks more: the mode
    // stays only while the body's contact with the climbable region is known,
    // and that contact is one tick behind a placement.
    let hold = if config.player_climbs { PLAYER_CLIMBS_HOLD_TICKS } else { 0 };
    let now = tick.get();
    let Some((at, arrival, vel)) =
        config.player_at.iter().copied().find(|(_, arrival, _)| (*arrival..=*arrival + hold).contains(&now))
    else {
        return;
    };
    let Ok((mut clusters, mut model)) = player.single_mut() else {
        return;
    };
    let mut clusters = clusters.as_clusters_mut();
    let launch = if vel == Vec2::ZERO {
        ae::movement::TransitVelocity::Zero
    } else {
        ae::movement::TransitVelocity::Set(ae::Vec2::new(vel.x, vel.y))
    };
    ae::movement::transit_body(&mut model, &mut clusters, ae::Vec2::new(at.x, at.y), launch);
    if config.player_climbs {
        // The climbing shape is the standing shape, so the mode is set with no
        // change of the body's size.
        clusters.body_mode.body_mode = ae::BodyMode::Climbing;
    }
    if now == arrival {
        eprintln!("capture_scene: put the player at {at:?} on tick {arrival}");
    }
}

/// One `--portal-shot`: a shot of the capture's own pair, fired on a sim tick.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PortalShotAt {
    tick: u64,
    origin: Vec2,
    dir: Vec2,
    /// End B of the pair, and not end A.
    end_b: bool,
}

/// End A of the pair `--portal-shot` opens: the last generated pair. It is an
/// authored channel, which no gun owns, so its portals stay with no gun in
/// the room (`despawn_orphaned_portals`).
const CAPTURE_PORTAL_PAIR: u8 = 254;

impl PortalShotAt {
    /// `TICK:X,Y:DX,DY[:b]`.
    fn parse(value: &str) -> Result<Self, String> {
        let bad = || format!("--portal-shot wants TICK:X,Y:DX,DY[:b], got '{value}'");
        let pair = |text: &str| -> Option<Vec2> {
            let (x, y) = text.split_once(',')?;
            Some(Vec2::new(x.trim().parse().ok()?, y.trim().parse().ok()?))
        };
        let parts: Vec<&str> = value.split(':').collect();
        let (tick, origin, dir, end_b) = match parts.as_slice() {
            [tick, origin, dir] => (tick, origin, dir, false),
            [tick, origin, dir, "b"] => (tick, origin, dir, true),
            _ => return Err(bad()),
        };
        Ok(Self {
            tick: tick.parse().map_err(|_| bad())?,
            origin: pair(origin).ok_or_else(bad)?,
            dir: pair(dir).filter(|dir| *dir != Vec2::ZERO).ok_or_else(bad)?,
            end_b,
        })
    }
}

/// One `--guard`: a bubble shield to draw at a place, in a state.
#[derive(Clone, Copy, Debug, PartialEq)]
struct GuardAt {
    at: Vec2,
    /// How whole the guard is, 1 to 0.
    integrity: f32,
    parrying: bool,
    /// It takes a blow two times each second.
    hit: bool,
    /// It holds one moment of the beat of a blow, 0 to 1 (`blow@PHASE`): a
    /// capture does not step the simulation one tick for each frame, so a
    /// beat of 18 ticks is in few of its frames.
    blow_held_at: Option<f32>,
}

impl GuardAt {
    /// `X,Y:INTEGRITY[:parry|:hit|:blow@PHASE]`.
    fn parse(value: &str) -> Result<Self, String> {
        let bad = || format!("--guard wants X,Y:INTEGRITY[:parry|:hit|:blow@PHASE], got '{value}'");
        let parts: Vec<&str> = value.split(':').collect();
        let (at, integrity, mode) = match parts.as_slice() {
            [at, integrity] => (at, integrity, ""),
            [at, integrity, mode] if matches!(*mode, "parry" | "hit") || mode.starts_with("blow@") => {
                (at, integrity, *mode)
            }
            _ => return Err(bad()),
        };
        let blow_held_at = match mode.strip_prefix("blow@") {
            Some(phase) => Some(phase.parse::<f32>().ok().filter(|phase| (0.0..1.0).contains(phase)).ok_or_else(bad)?),
            None => None,
        };
        let (x, y) = at.split_once(',').ok_or_else(bad)?;
        Ok(Self {
            at: Vec2::new(x.trim().parse().map_err(|_| bad())?, y.trim().parse().map_err(|_| bad())?),
            integrity: integrity.parse().map_err(|_| bad())?,
            parrying: mode == "parry",
            hit: mode == "hit",
            blow_held_at,
        })
    }
}

/// `--guard`: add each configured guard to the shield read model, each
/// sim tick, after the simulation's own guards. Presentation only: no body
/// holds these, and no rule reads them. Each is the size of the robot.
fn draw_guards(
    config: Res<SceneCaptureConfig>,
    tick: Res<ambition_platformer2d::time::SimTick>,
    rooms: ambition_platformer2d::platformer::lifecycle::LiveRooms,
    mut view: ResMut<ambition_platformer2d::sim_view::ShieldRingsView>,
) {
    use ambition_platformer2d::engine_core::Vec2 as SimVec2;
    for guard in &config.guards {
        // A blow each 30 ticks, from the upper left: the shieldstun it leaves
        // runs down for 8, and its beat for 18.
        let since_blow = (tick.get() % 30) as f32 / 60.0;
        let blow_phase =
            guard.blow_held_at.or(Some(since_blow / 0.30).filter(|phase| guard.hit && *phase < 1.0));
        view.0.push(ambition_platformer2d::sim_view::ShieldRingFact {
            pos: SimVec2::new(guard.at.x, guard.at.y),
            size: SimVec2::new(28.0, 46.0),
            parrying: guard.parrying,
            integrity: guard.integrity,
            stun_secs: if guard.hit { (0.13 - since_blow).max(0.0) } else { 0.0 },
            blow_phase,
            blow_from: SimVec2::new(-30.0, -20.0),
            gravity_dir: SimVec2::new(0.0, 1.0),
            room: rooms.sole(),
        });
    }
}

/// One `--shot`: a projectile with a look, fired on a sim tick.
#[derive(Clone, Debug, PartialEq)]
struct ShotAt {
    tick: u64,
    origin: Vec2,
    dir: Vec2,
    /// A projectile look. `fireball`, `hadouken` and `hadouken_super` are the
    /// named kinds, with the body and the flight of the kind. Any other id is
    /// an open look on the body of a plain ranged shot, and `-` is the look
    /// of a shot that names none.
    look: String,
}

impl ShotAt {
    /// `TICK:X,Y:DX,DY:LOOK`.
    fn parse(value: &str) -> Result<Self, String> {
        let bad = || format!("--shot wants TICK:X,Y:DX,DY:LOOK, got '{value}'");
        let pair = |text: &str| -> Option<Vec2> {
            let (x, y) = text.split_once(',')?;
            Some(Vec2::new(x.trim().parse().ok()?, y.trim().parse().ok()?))
        };
        let parts: Vec<&str> = value.split(':').collect();
        let [tick, origin, dir, look] = parts.as_slice() else {
            return Err(bad());
        };
        Ok(Self {
            tick: tick.parse().map_err(|_| bad())?,
            origin: pair(origin).ok_or_else(bad)?,
            dir: pair(dir).filter(|dir| *dir != Vec2::ZERO).ok_or_else(bad)?,
            look: if *look == "-" { String::new() } else { look.to_string() },
        })
    }
}

/// One sim tick of wall time (`SIM_TICK_HZ`).
fn one_tick() -> std::time::Duration {
    std::time::Duration::from_secs_f64(1.0 / ambition_platformer2d::runtime::SIM_TICK_HZ)
}

/// `--tick-per-frame`: a frame is one sim tick, and the clock stands still
/// while a shot is read back (a readback takes frames), so the shots of a
/// film are `--stride` ticks apart, exactly.
fn pace_the_clock_by_the_shots(
    runtime: Res<SceneCaptureRuntime>,
    mut strategy: ResMut<bevy::time::TimeUpdateStrategy>,
) {
    let step = if runtime.requested && !runtime.completed { std::time::Duration::ZERO } else { one_tick() };
    *strategy = bevy::time::TimeUpdateStrategy::ManualDuration(step);
}

/// `--item TICK:X,Y:VX,VY:ID`: a held item, thrown into the world at a place
/// with a velocity on a sim tick.
#[derive(Clone, Debug)]
struct ItemAt {
    tick: u64,
    at: Vec2,
    vel: Vec2,
    id: String,
}

impl ItemAt {
    fn parse(value: &str) -> Result<Self, String> {
        let bad = || format!("--item wants TICK:X,Y:VX,VY:ID, got '{value}'");
        let pair = |text: &str| -> Option<Vec2> {
            let (x, y) = text.split_once(',')?;
            Some(Vec2::new(x.trim().parse().ok()?, y.trim().parse().ok()?))
        };
        let parts: Vec<&str> = value.split(':').collect();
        let [tick, at, vel, id] = parts.as_slice() else {
            return Err(bad());
        };
        Ok(Self {
            tick: tick.parse().map_err(|_| bad())?,
            at: pair(at).ok_or_else(bad)?,
            vel: pair(vel).ok_or_else(bad)?,
            id: id.to_string(),
        })
    }
}

/// `--item`: throw each configured item on its tick, released as a hand
/// releases one (a free body under its own physics).
fn throw_items(
    config: Res<SceneCaptureConfig>,
    tick: Res<ambition_platformer2d::time::SimTick>,
    mut commands: Commands,
) {
    use ambition_platformer2d::engine_core::Vec2 as V;
    use ambition_platformer2d::held_items::GroundItem;
    for item in config.items.iter().filter(|item| item.tick == tick.get()) {
        let Some(spec) = ambition_platformer2d::character::held_item_by_id(&item.id) else {
            eprintln!("capture_scene: --item names no held item '{}'", item.id);
            continue;
        };
        commands.spawn((
            GroundItem::released(spec, V::new(item.at.x, item.at.y), V::new(item.vel.x, item.vel.y), V::splat(20.0)),
            ambition_platformer2d::item::ItemCustody::InWorld,
            Name::new(format!("Capture item: {}", item.id)),
        ));
    }
}

/// `--shot`: fire each configured shot on its tick, through the projectile
/// domain's own request, as a body that fires does. The primary player owns
/// it (a shot does not touch its owner). A sim system that names its tick, as
/// [`fire_portal_shots`] is.
fn fire_shots(
    config: Res<SceneCaptureConfig>,
    tick: Res<ambition_platformer2d::time::SimTick>,
    players: Query<Entity, ambition_platformer2d::platformer::markers::PrimaryPlayerOnly>,
    mut requests: MessageWriter<ambition_platformer2d::projectiles::ProjectileSpawnRequest>,
) {
    use ambition_platformer2d::projectiles::{
        InFlightProjectile, ProjectileBody, ProjectileKind, ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart,
    };
    for shot in config.shots.iter().filter(|shot| shot.tick == tick.get()) {
        let Ok(owner) = players.single() else {
            eprintln!("capture_scene: --shot found no player to own the shot on tick {}", shot.tick);
            continue;
        };
        let (origin, dir) = (
            ambition_platformer2d::engine_core::Vec2::new(shot.origin.x, shot.origin.y),
            ambition_platformer2d::engine_core::Vec2::new(shot.dir.x, shot.dir.y),
        );
        let kind = [ProjectileKind::Fireball, ProjectileKind::Hadouken, ProjectileKind::HadoukenSuper]
            .into_iter()
            .find(|kind| kind.visual_id() == shot.look);
        requests.write(match kind {
            Some(kind) => ProjectileSpawnRequest::named(
                owner,
                InFlightProjectile { body: ProjectileBody::from_spec(kind.spec(origin, dir, 1.0)) },
                kind,
                ProjectileStart::StepNextTick,
            ),
            // The body and the speed of a plain ranged shot.
            None => ProjectileSpawnRequest::open(
                owner,
                ProjectileSpawn {
                    origin,
                    dir,
                    speed: 300.0,
                    damage: 1,
                    max_lifetime: 3.0,
                    half_extent: ambition_platformer2d::engine_core::Vec2::new(10.0, 8.0),
                    gravity: 0.0,
                    visual_id: shot.look.clone(),
                    bounces: 0,
                    bounce_on_world_contact: false,
                    splash_half_extent: 0.0,
                    boomerang_return_s: None,
                },
                ProjectileStart::StepNextTick,
            ),
        });
        eprintln!("capture_scene: fired a shot on tick {}: {shot:?}", shot.tick);
    }
}

/// `--portal-shot`: fire each configured shot on its tick, through the portal
/// domain's own intent, as a gun does. A sim system that names its tick: a
/// replay of that tick fires again, and a replay of any other does not. The
/// shot has no identity and no room: a capture has one live room.
fn fire_portal_shots(
    config: Res<SceneCaptureConfig>,
    tick: Res<ambition_platformer2d::time::SimTick>,
    mut intents: MessageWriter<ambition_platformer2d::portal::PortalFireIntent>,
) {
    use ambition_platformer2d::portal::{PortalChannel, PortalChannelColor};
    for shot in config.portal_shots.iter().filter(|shot| shot.tick == tick.get()) {
        let end = PortalChannelColor::Indexed(CAPTURE_PORTAL_PAIR + u8::from(shot.end_b));
        intents.write(ambition_platformer2d::portal::PortalFireIntent {
            origin: shot.origin,
            dir: shot.dir,
            channel: PortalChannel::Authored(end),
            id: None,
            room: None,
        });
        eprintln!("capture_scene: fired a portal shot on tick {}: {shot:?}", shot.tick);
    }
}

/// `--limbs-unseen`: each part-drawn body draws none of its limbs' parts,
/// and each limb twin draws nothing, whatever the portal adapter asked.
fn draw_no_limbs(
    config: Res<SceneCaptureConfig>,
    mut commands: Commands,
    roots: Query<(
        Entity,
        &ambition_platformer2d::render::rendering::actors::rigged::RiggedPresentedBy,
        Option<&ambition_platformer2d::render::rendering::actors::rigged::LimbTwin>,
        Option<&ambition_platformer2d::render::rendering::actors::rigged::RiggedTrackMask>,
    )>,
    presentations: Query<&ambition_platformer2d::render::rendering::actors::rigged::RiggedPresentation>,
) {
    use ambition_platformer2d::render::rendering::actors::rigged::RiggedTrackMask;
    let Some(named) = config.limbs_unseen.as_ref() else {
        return;
    };
    let unseen = |slot: &str| named.is_empty() || named.iter().any(|named| named == slot);
    for (root, presented, twin, mask) in &roots {
        let Ok(presentation) = presentations.get(presented.owner()) else {
            continue;
        };
        let mut slots: Vec<String> = presentation
            .pages
            .flipbook
            .tracks
            .iter()
            .filter_map(|name| name.strip_prefix("limb:")?.split(':').next().map(str::to_owned))
            .filter(|slot| unseen(slot))
            .collect();
        slots.sort();
        slots.dedup();
        let wanted = match twin {
            // The body draws its parts over the limbs itself (below), and a
            // twin of an unseen limb draws nothing; any other twin, its limb.
            Some(twin) if twin.is_over() || unseen(&twin.slot) => RiggedTrackMask::nothing(),
            Some(_) => continue,
            None if slots.is_empty() => continue,
            // The body draws every part but the limbs the portal adapter
            // sent as twins and the unseen limbs.
            None => {
                let mut limbs = mask.filter(|mask| !mask.only).map_or_else(Vec::new, |mask| mask.limbs.clone());
                limbs.extend(slots);
                limbs.sort();
                limbs.dedup();
                RiggedTrackMask::without(limbs)
            }
        };
        if mask != Some(&wanted) {
            commands.entity(root).insert(wanted);
        }
    }
}

/// The systems a room capture adds on top of [`build_capture_app`].
fn install_room_capture(app: &mut App) {
    if app.world().resource::<SceneCaptureConfig>().tick_per_frame {
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(one_tick()));
        app.add_systems(Last, pace_the_clock_by_the_shots);
    }
    if app.world().resource::<SceneCaptureConfig>().portal_effect_off {
        app.add_systems(Update, |mut selection: ResMut<ambition_platformer2d::portal_presentation::PortalEffectSelection>| {
            selection.active = ambition_platformer2d::portal_presentation::PortalVisualEffect::Off;
        });
    }
    if !app.world().resource::<SceneCaptureConfig>().carry {
        app.insert_resource(ambition_platformer2d::portal_presentation::PortalCarrySelection { enabled: false });
    }
    if app.world().resource::<SceneCaptureConfig>().limbs_unseen.is_some() {
        use ambition_platformer2d::render::rendering::actors::rigged::{drive_rigged_presentations, follow_limb_twins};
        app.add_systems(
            Update,
            draw_no_limbs
                .after(ambition_platformer2d::portal_presentation::PortalObservationSet)
                .after(follow_limb_twins)
                .before(drive_rigged_presentations),
        );
    }
    let sim = app.sim_schedule();
    // Sim systems: each writes rollback state (health, a body's position, a
    // seat's interact buffer), and a write from outside the rewinding
    // schedule would survive a rewind.
    app.add_systems(
        sim,
        (hold_boss_health, blow_on_bosses, place_player_beside, place_player_at, record_flags, turn_on_switches, set_the_hour, ask_bodies_to_wear, put_on_gear, fire_portal_shots, fire_shots, throw_items),
    );
    if !app.world().resource::<SceneCaptureConfig>().player_holds.is_empty() {
        app.add_systems(
            sim,
            hold_player_controls
                .in_set(ambition_platformer2d::sim::Platformer2dSimulationPhase::PlayerInput)
                .after(ambition_platformer2d::runtime::host_input::publish_latched_slot_controls)
                .after(ambition_platformer2d::runtime::host_input::publish_seat_controls_when_nobody_else_does)
                .before(ambition_platformer2d::runtime::host_input::mirror_primary_slot_to_control_frame),
        );
    }
    if app.world().resource::<SceneCaptureConfig>().nav_overlay {
        app.insert_resource(ambition_app::dev::navigation_overlay::NavigationOverlay { shown: true });
    }
    if !app.world().resource::<SceneCaptureConfig>().guards.is_empty() {
        // In the schedule that rebuilds the view, after the rebuild: an edge
        // from `Update` to a sim system orders nothing, so a frame with no sim
        // tick appended the guards again onto the last tick's list.
        app.add_systems(sim, draw_guards.after(ambition_platformer2d::sim_view::rebuild_shield_views));
    }
    if app.world().resource::<SceneCaptureConfig>().seam.is_some() {
        app.add_systems(
            PostUpdate,
            hide_the_front_parallax.before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
        );
    }
    app.add_systems(Startup, setup_capture_target.after(PresentationSetupSet));
    app.add_systems(
        Update,
        (
            silence_dev_overlays,
            force_combat_overlay,
            force_screen_effects,
            force_look_depth,
            force_camera_zoom,
            force_portal_windows,
            // Before every presentation system that reads the camera: with no
            // order, a serial executor ran the motes and the daylight veil
            // against the camera before this pin, and drew them off the frame.
            apply_capture_snapshot
                .after(camera_follow)
                .before(sync_parallax_layers)
                .before(ambition_platformer2d::render::rendering::ambient_motes::drift_room_motes)
                .before(ambition_platformer2d::render::rendering::daylight::light_the_world),
            request_capture.after(sync_parallax_layers),
            adopt_menu_camera,
            finish_after_capture,
            fail_after_timeout,
            fail_when_the_shell_refuses_the_route,
        ),
    );
}

/// One step of a `--press` sequence.
///
/// `Wait` exists because presses are two frames (press, release; the surfaces
/// read edges), and a route change takes much longer.
#[derive(Clone, Copy, Debug)]
enum PressStep {
    Tap(KeyCode),
    /// Run this many frames without touching anything.
    Wait(u32),
    /// Press and keep holding (`hold:up`) until a matching `release:`.
    ///
    /// A tap cannot express a directional attack: `up,x` releases Up before
    /// attack is pressed, so the swing resolves forward. Tilts and aerials
    /// need a direction held while attack is pressed.
    Hold(KeyCode),
    /// Let go of a key an earlier `hold:` is still holding (`release:up`).
    Release(KeyCode),
    /// Tap the glass at a point (`touch:167x523`), in logical window pixels
    /// with a top-left origin: the space of `HitRect` and `Node { left, top }`,
    /// and of the capture's pixels at scale 1.
    ///
    /// A key is an edge with no position; a finger carries both, so one step
    /// type reaches every pointer widget. It sends real `TouchInput` messages
    /// (the pair winit emits), folded by Bevy's `touch_screen_input_system`,
    /// so it drives the same phone path the product ships.
    Touch(Vec2),
    /// Put a finger down, slide it, and lift it (`touch:600x400>600x150`).
    ///
    /// The slide takes [`DRAG_FRAMES`] frames, because a drag is recognized by
    /// how far the finger has travelled since it went down.
    Drag(Vec2, Vec2),
}

/// Frames a `touch:A>B` drag spends moving between its two points.
const DRAG_FRAMES: u32 = 8;

/// Parse `--press-during N` into the press-driving frame the shutter opens on.
///
/// `0` is refused, not clamped. It would mean a capture with no press in it,
/// which is spelled by leaving `--press` off. The first input frame is `1`.
fn parse_press_during(text: &str) -> Result<u32, String> {
    let frames = text
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("--press-during wants a frame count, got '{text}'"))?;
    if frames == 0 {
        return Err(
            "--press-during 0 would photograph the frame BEFORE the first press step, which is \
             what leaving --press off already does. The first frame of the input is 1."
                .to_string(),
        );
    }
    Ok(frames)
}

/// Parse `--press Down,Enter,wait,Down,Enter` into taps and pauses.
///
/// Names are the ones a person says aloud (not `ArrowDown`), because this is
/// typed by hand.
fn parse_press_sequence(text: &str) -> Result<Vec<PressStep>, String> {
    text.split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            if lower == "wait" {
                return Ok(PressStep::Wait(DEFAULT_WAIT_FRAMES));
            }
            if let Some(rest) = lower.strip_prefix("hold:") {
                return parse_key(rest).map(PressStep::Hold);
            }
            if let Some(rest) = lower.strip_prefix("release:") {
                return parse_key(rest).map(PressStep::Release);
            }
            if let Some(rest) = lower.strip_prefix("touch:") {
                if let Some((from, to)) = rest.split_once('>') {
                    return Ok(PressStep::Drag(parse_point(from)?, parse_point(to)?));
                }
                return parse_point(rest).map(PressStep::Touch);
            }
            if let Some(rest) = lower.strip_prefix("tap:") {
                return parse_point(rest).map(PressStep::Touch);
            }
            if let Some(rest) = lower.strip_prefix("wait:") {
                return rest
                    .parse::<u32>()
                    .map(PressStep::Wait)
                    .map_err(|_| format!("--press wait:N needs a frame count, got '{rest}'"));
            }
            parse_key(&lower).map(PressStep::Tap)
        })
        .collect()
}

/// `167x523` — one point in the `--press` vocabulary.
///
/// `x`, not a comma, because the comma separates steps. The size argument
/// already uses `WIDTHxHEIGHT`.
fn parse_point(text: &str) -> Result<Vec2, String> {
    let (x, y) = text
        .split_once('x')
        .ok_or_else(|| format!("--press touch:X x Y needs two numbers, got '{text}'"))?;
    let x = x
        .trim()
        .parse::<f32>()
        .map_err(|_| format!("--press touch: '{x}' is not a number"))?;
    let y = y
        .trim()
        .parse::<f32>()
        .map_err(|_| format!("--press touch: '{y}' is not a number"))?;
    Ok(Vec2::new(x, y))
}

/// One key name from the `--press` vocabulary.
fn parse_key(name: &str) -> Result<KeyCode, String> {
    match name {
        "up" | "arrowup" => Ok(KeyCode::ArrowUp),
        "down" | "arrowdown" => Ok(KeyCode::ArrowDown),
        "left" | "arrowleft" => Ok(KeyCode::ArrowLeft),
        "right" | "arrowright" => Ok(KeyCode::ArrowRight),
        "enter" | "return" => Ok(KeyCode::Enter),
        "space" => Ok(KeyCode::Space),
        "escape" | "esc" => Ok(KeyCode::Escape),
        "z" => Ok(KeyCode::KeyZ),
        "x" => Ok(KeyCode::KeyX),
        "c" => Ok(KeyCode::KeyC),
        // The rest of the default preset's action row. `z`/`x`/`c` are
        // jump/attack/dash; the preset also binds secondary=A,
        // quick_action=E, special=G, and interact=F.
        "a" => Ok(KeyCode::KeyA),
        "e" => Ok(KeyCode::KeyE),
        "f" => Ok(KeyCode::KeyF),
        "g" => Ok(KeyCode::KeyG),
        // The menu keys: `i` opens the inventory cube, `tab` the map.
        "i" => Ok(KeyCode::KeyI),
        "tab" => Ok(KeyCode::Tab),
        other => Err(format!(
            "--press does not know the key '{other}'. Known: up, down, left, \
                 right, enter, space, escape, z, x, c, a, e, f, g, i, tab, wait, wait:N, \
                 hold:KEY, release:KEY, touch:XxY"
        )),
    }
}

/// Frames a bare `wait` runs for — comfortably more than a route change.
const DEFAULT_WAIT_FRAMES: u32 = 120;

impl SceneCaptureConfig {
    fn from_args(args: Vec<String>) -> Result<Self, String> {
        let mut positional = Vec::new();
        let mut warmup_frames = 12u32;
        let mut include_ui = false;
        let mut dev_overlays = false;
        let mut combat_overlay = false;
        let mut nav_overlay = false;
        let mut boss_hp: Option<f32> = None;
        let mut boss_blows: Vec<(u64, i32)> = Vec::new();
        let mut look_depth: Option<[f32; 2]> = None;
        let mut camera_zoom = None;
        let mut portal_windows = ambition_platformer2d::persistence::settings::video::PortalWindowLook::Seamless;
        let mut screen_effects: Vec<ScreenEffect> = Vec::new();
        // One shot every frame by default.
        let mut frames: usize = 1;
        let mut stride: u32 = 1;
        let mut fit_room = false;
        let mut character: Option<String> = None;
        let mut route: Option<String> = None;
        let mut press: Vec<PressStep> = Vec::new();
        let mut press_during: Option<u32> = None;
        let mut player_beside: Option<(String, u64)> = None;
        let mut interact_on_arrival = false;
        let mut interact_at = None;
        let mut flags: Vec<(String, u64)> = Vec::new();
        let mut switches: Vec<(String, u64)> = Vec::new();
        let mut hour: Option<(f32, u64)> = None;
        let mut wears: Vec<(String, String, u64)> = Vec::new();
        let mut portal_shots: Vec<PortalShotAt> = Vec::new();
        let mut shots: Vec<ShotAt> = Vec::new();
        let mut items: Vec<ItemAt> = Vec::new();
        let mut portal_effect_off = false;
        let mut limbs_unseen: Option<Vec<String>> = None;
        let mut carry = true;
        let mut gear: Vec<(String, u64)> = Vec::new();
        let mut tick_per_frame = false;
        let mut guards: Vec<GuardAt> = Vec::new();
        let mut player_at: Vec<(Vec2, u64, Vec2)> = Vec::new();
        let mut player_climbs = false;
        let mut seam: Option<SeamCheck> = None;
        let mut player_holds: Vec<(HeldControls, u64)> = Vec::new();
        let mut body_warp = None;
        let mut i = 0usize;
        while i < args.len() {
            // Each arm returns how many arguments it consumed, so an arm that
            // forgets is a compile error, not a hang.
            let consumed = match args[i].as_str() {
                // This does not imply `--dev-overlays`. Clearing developer
                // chrome and turning on combat gizmos are separate concerns:
                // the gizmos need `DeveloperRuntimeState.debug` and the gizmo
                // toggles, which the chrome settings do not touch.
                // `force_combat_overlay` runs after the silencer.
                "--nav-overlay" => {
                    nav_overlay = true;
                    1
                }
                "--combat-overlay" => {
                    combat_overlay = true;
                    1
                }
                "--boss-hp" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--boss-hp requires a fraction".to_string());
                    };
                    boss_hp = Some(
                        value
                            .parse::<f32>()
                            .ok()
                            .filter(|f| (0.0..=1.0).contains(f))
                            .ok_or_else(|| {
                                format!("--boss-hp wants a fraction in [0, 1], got '{value}'")
                            })?,
                    );
                    2
                }
                "--look-depth" => {
                    let numbers: Vec<f32> = args
                        .get(i + 1)
                        .map(|value| value.split(',').filter_map(|n| n.trim().parse().ok()).collect())
                        .unwrap_or_default();
                    look_depth = Some(<[f32; 2]>::try_from(numbers).map_err(|_| {
                        "--look-depth wants two numbers: FOG,PATCHES (e.g. 0.22,0.5)".to_string()
                    })?);
                    2
                }
                "--portal-windows" => {
                    use ambition_platformer2d::persistence::settings::video::PortalWindowLook as Look;
                    portal_windows = match args.get(i + 1).map(String::as_str) {
                        Some("seamless") => Look::Seamless,
                        Some("tinted-blue") => Look::TintedBlue,
                        Some("tinted-gold") => Look::TintedGold,
                        other => {
                            return Err(format!("--portal-windows wants seamless, tinted-blue or tinted-gold, not {other:?}"))
                        }
                    };
                    2
                }
                "--camera-zoom" => {
                    use ambition_platformer2d::persistence::settings::video::CameraZoomPreset as Zoom;
                    camera_zoom = Some(match args.get(i + 1).map(String::as_str) {
                        Some("duel") => Zoom::Duel,
                        Some("tight") => Zoom::Tight,
                        Some("combat") => Zoom::Combat,
                        Some("arena") => Zoom::Arena,
                        Some("cinematic") => Zoom::Cinematic,
                        Some("debug") => Zoom::Debug,
                        other => {
                            return Err(format!(
                                "--camera-zoom wants duel, tight, combat, arena, cinematic or debug, not {other:?}"
                            ))
                        }
                    });
                    2
                }
                "--dev-overlays" => {
                    dev_overlays = true;
                    1
                }
                "--screen-effect" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--screen-effect requires a value".to_string());
                    };
                    for name in value.split(',').filter(|name| !name.trim().is_empty()) {
                        let effect = ScreenEffect::parse(name)?;
                        if !screen_effects.contains(&effect) {
                            screen_effects.push(effect);
                        }
                    }
                    2
                }
                "--include-ui" => {
                    include_ui = true;
                    1
                }
                "--fit-room" => {
                    fit_room = true;
                    1
                }
                "--interact-on-arrival" => {
                    interact_on_arrival = true;
                    1
                }
                "--interact-at" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--interact-at requires a tick".to_string());
                    };
                    interact_at = Some(
                        value.parse::<u64>().map_err(|_| format!("--interact-at wants a tick, got '{value}'"))?,
                    );
                    2
                }
                "--body-warp" => {
                    use ambition_platformer2d::render::rendering::actors::rigged::BodyWarpPreview;
                    use ambition_platformer2d::sprite_sheet::character::rigged::BodyWarp;
                    let Some(value) = args.get(i + 1) else {
                        return Err("--body-warp requires `out` or `in`".to_string());
                    };
                    let (kind, period) = value.split_once('@').unwrap_or((value.as_str(), "1.0"));
                    let warp = match kind {
                        "out" => BodyWarp::TeleportOut,
                        "in" => BodyWarp::TeleportIn,
                        other => return Err(format!("--body-warp wants `out` or `in`, got '{other}'")),
                    };
                    let period_s = period
                        .parse::<f32>()
                        .map_err(|_| format!("--body-warp KIND@SECONDS wants seconds, got '{period}'"))?;
                    body_warp = Some(BodyWarpPreview { warp, period_s });
                    2
                }
                "--hour" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--hour requires an hour".to_string());
                    };
                    let (h, tick) = match value.split_once('@') {
                        Some((h, tick)) => (h, tick.parse::<u64>().map_err(|_| format!("--hour H@TICK wants a sim tick, got '{tick}'"))?),
                        None => (value.as_str(), 2),
                    };
                    let h = h.parse::<f32>().map_err(|_| format!("--hour wants an hour from 0 to 24, got '{h}'"))?;
                    hour = Some((h, tick));
                    2
                }
                "--switch" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--switch requires a switch id".to_string());
                    };
                    switches.push(match value.split_once('@') {
                        Some((id, tick)) => (
                            id.to_string(),
                            tick.parse::<u64>().map_err(|_| {
                                format!("--switch NAME@TICK wants a sim tick, got '{tick}'")
                            })?,
                        ),
                        None => (value.clone(), 1),
                    });
                    2
                }
                "--flag" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--flag requires a flag name".to_string());
                    };
                    flags.push(match value.split_once('@') {
                        Some((id, tick)) => (
                            id.to_string(),
                            tick.parse::<u64>().map_err(|_| {
                                format!("--flag NAME@TICK wants a sim tick, got '{tick}'")
                            })?,
                        ),
                        None => (value.clone(), 1),
                    });
                    2
                }
                "--wear" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--wear requires ID=LOOK".to_string());
                    };
                    let (pair, tick) = match value.split_once('@') {
                        Some((pair, tick)) => (
                            pair,
                            tick.parse::<u64>()
                                .map_err(|_| format!("--wear ID=LOOK@TICK wants a sim tick, got '{tick}'"))?,
                        ),
                        None => (value.as_str(), 60),
                    };
                    let Some((wearer, look)) = pair.split_once('=') else {
                        return Err(format!("--wear wants ID=LOOK, got '{pair}'"));
                    };
                    wears.push((wearer.to_string(), look.to_string(), tick));
                    2
                }
                "--boss-blow" => {
                    let bad = |value: &str| format!("--boss-blow wants TICK[xHEARTS], got '{value}'");
                    let Some(value) = args.get(i + 1) else {
                        return Err("--boss-blow requires TICK[xHEARTS]".to_string());
                    };
                    let (tick, hearts) = match value.split_once('x') {
                        Some((tick, "all")) => (tick, KILLING_BLOW),
                        Some((tick, hearts)) => (tick, hearts.parse::<i32>().map_err(|_| bad(value))?),
                        None => (value.as_str(), 1),
                    };
                    boss_blows.push((tick.parse::<u64>().map_err(|_| bad(value))?, hearts));
                    2
                }
                "--player-at" => {
                    let bad = |value: &str| format!("--player-at wants X,Y[@TICK][:VX,VY], got '{value}'");
                    let Some(value) = args.get(i + 1) else {
                        return Err("--player-at requires X,Y[@TICK][:VX,VY]".to_string());
                    };
                    let (place, vel) = match value.split_once(':') {
                        Some((place, vel)) => {
                            let (vx, vy) = vel.split_once(',').ok_or_else(|| bad(value))?;
                            (
                                place,
                                Vec2::new(
                                    vx.trim().parse().map_err(|_| bad(value))?,
                                    vy.trim().parse().map_err(|_| bad(value))?,
                                ),
                            )
                        }
                        None => (value.as_str(), Vec2::ZERO),
                    };
                    let (point, tick) = match place.split_once('@') {
                        Some((point, tick)) => (point, tick.parse::<u64>().map_err(|_| bad(value))?),
                        None => (place, DEFAULT_ARRIVAL_TICK),
                    };
                    let (x, y) = point.split_once(',').ok_or_else(|| bad(value))?;
                    let at = Vec2::new(
                        x.trim().parse().map_err(|_| bad(value))?,
                        y.trim().parse().map_err(|_| bad(value))?,
                    );
                    player_at.push((at, tick, vel));
                    2
                }
                "--player-climbs" => {
                    player_climbs = true;
                    1
                }
                "--player-holds" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--player-holds requires NAMES[@TICK]".to_string());
                    };
                    let (names, tick) = match value.split_once('@') {
                        Some((names, tick)) => (
                            names,
                            tick.parse::<u64>()
                                .map_err(|_| format!("--player-holds wants NAMES[@TICK], got '{value}'"))?,
                        ),
                        None => (value.as_str(), 60),
                    };
                    player_holds.push((HeldControls::parse(names)?, tick));
                    2
                }
                "--portal-shot" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--portal-shot requires TICK:X,Y:DX,DY[:b]".to_string());
                    };
                    portal_shots.push(PortalShotAt::parse(value)?);
                    2
                }
                "--guard" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--guard requires X,Y:INTEGRITY[:parry|:hit|:blow@PHASE]".to_string());
                    };
                    guards.push(GuardAt::parse(value)?);
                    2
                }
                "--shot" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--shot requires TICK:X,Y:DX,DY:LOOK".to_string());
                    };
                    shots.push(ShotAt::parse(value)?);
                    2
                }
                "--seam" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--seam requires PORTAL:window or PORTAL:far".to_string());
                    };
                    let parsed = SeamCheck::parse(value)?;
                    // The far side is the world itself, with no window.
                    if parsed.far {
                        portal_effect_off = true;
                    }
                    seam = Some(parsed);
                    2
                }
                "--tick-per-frame" => {
                    tick_per_frame = true;
                    1
                }
                "--carry" => {
                    carry = match args.get(i + 1).map(String::as_str) {
                        Some("on") => true,
                        Some("off") => false,
                        other => return Err(format!("--carry wants on or off, not {other:?}")),
                    };
                    2
                }
                "--gear" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--gear requires ID[@TICK]".to_string());
                    };
                    let (id, tick) = match value.split_once('@') {
                        Some((id, tick)) => (id, tick.parse::<u64>().map_err(|_| format!("--gear: bad tick in {value}"))?),
                        None => (value.as_str(), 1),
                    };
                    gear.push((id.to_owned(), tick));
                    2
                }
                "--limbs-unseen" => {
                    limbs_unseen = match args.get(i + 1).map(String::as_str) {
                        Some("all") => Some(Vec::new()),
                        Some(slots) if !slots.starts_with("--") => {
                            Some(slots.split(',').map(str::to_owned).collect())
                        }
                        _ => return Err("--limbs-unseen wants all or SLOT[,SLOT]".to_string()),
                    };
                    2
                }
                "--portal-effect" => {
                    match args.get(i + 1).map(String::as_str) {
                        Some("off") => portal_effect_off = true,
                        Some("cones") => portal_effect_off = false,
                        _ => return Err("--portal-effect wants off or cones".to_string()),
                    }
                    2
                }
                "--item" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--item requires TICK:X,Y:VX,VY:ID".to_string());
                    };
                    items.push(ItemAt::parse(value)?);
                    2
                }
                "--player-beside" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--player-beside requires a catalog id".to_string());
                    };
                    player_beside = Some(match value.split_once('@') {
                        Some((id, tick)) => (
                            id.to_string(),
                            tick.parse::<u64>().map_err(|_| {
                                format!("--player-beside ID@TICK wants a sim tick, got '{tick}'")
                            })?,
                        ),
                        None => (value.clone(), DEFAULT_ARRIVAL_TICK),
                    });
                    2
                }
                // One render mode only: a flag that shows an empty window is
                // not worth a second composition.
                "--character" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--character requires a catalog id".to_string());
                    };
                    character = Some(value.clone());
                    2
                }
                arg if arg.starts_with("--character=") => {
                    character = Some(arg.trim_start_matches("--character=").to_string());
                    1
                }
                "--press" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--press requires a comma-separated key list".to_string());
                    };
                    press = parse_press_sequence(value)?;
                    2
                }
                arg if arg.starts_with("--press=") => {
                    press = parse_press_sequence(arg.trim_start_matches("--press="))?;
                    1
                }
                "--press-during" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--press-during requires a frame count".to_string());
                    };
                    press_during = Some(parse_press_during(value)?);
                    2
                }
                arg if arg.starts_with("--press-during=") => {
                    press_during = Some(parse_press_during(
                        arg.trim_start_matches("--press-during="),
                    )?);
                    1
                }
                "--route" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--route requires a shell route id".to_string());
                    };
                    route = Some(value.clone());
                    2
                }
                arg if arg.starts_with("--route=") => {
                    route = Some(arg.trim_start_matches("--route=").to_string());
                    1
                }
                "--warmup" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--warmup requires a frame count".to_string());
                    };
                    warmup_frames = value
                        .parse::<u32>()
                        .map_err(|_| format!("--warmup must be an integer, got '{value}'"))?;
                    2
                }
                "--frames" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--frames requires a count".to_string());
                    };
                    frames = value
                        .parse::<usize>()
                        .map_err(|_| format!("--frames wants a count, got '{value}'"))?
                        .max(1);
                    2
                }
                arg if arg.starts_with("--frames=") => {
                    let value = arg.trim_start_matches("--frames=");
                    frames = value
                        .parse::<usize>()
                        .map_err(|_| format!("--frames wants a count, got '{value}'"))?
                        .max(1);
                    1
                }
                "--stride" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--stride requires a frame count".to_string());
                    };
                    stride = value
                        .parse::<u32>()
                        .map_err(|_| format!("--stride wants a count, got '{value}'"))?
                        .max(1);
                    2
                }
                arg if arg.starts_with("--stride=") => {
                    let value = arg.trim_start_matches("--stride=");
                    stride = value
                        .parse::<u32>()
                        .map_err(|_| format!("--stride wants a count, got '{value}'"))?
                        .max(1);
                    1
                }
                arg if arg.starts_with("--warmup=") => {
                    let value = arg.trim_start_matches("--warmup=");
                    warmup_frames = value
                        .parse::<u32>()
                        .map_err(|_| format!("--warmup must be an integer, got '{value}'"))?;
                    1
                }
                other if other.starts_with('-') => {
                    return Err(format!("unknown option '{other}'"));
                }
                other => {
                    positional.push(other.to_string());
                    1
                }
            };
            i += consumed;
        }

        // `--press-during` names a frame of `--press`. Without a sequence it
        // would do nothing silently, so refuse it here, before any image.
        if press_during.is_some() && press.is_empty() {
            return Err(
                "--press-during counts frames of --press, and no --press was given. There is no \
                 input for the shutter to open during."
                    .to_string(),
            );
        }

        // A route has no room id and no focus point: the shell composes its
        // own surface and cameras.
        if let Some(route) = route.clone() {
            // Classify positionals instead of counting them, so an unexpected
            // argument is named before any work happens.
            let mut output: Option<PathBuf> = None;
            let mut size: Option<UVec2> = None;
            for value in &positional {
                if let Some(parsed) = parse_image_size(value) {
                    size = Some(parsed);
                } else if looks_like_image_path(value) {
                    output = Some(PathBuf::from(value));
                } else {
                    return Err(format!(
                        "--route takes no ROOM_ID or focus point: the shell composes its own \
                         surface and its own cameras. `{value}` is neither an output path \
                         (*.png / *.jpg) nor a size (WIDTHxHEIGHT).\n  \
                         usage: capture_scene --route <ROUTE_ID> [OUT.png] [WIDTHxHEIGHT]"
                    ));
                }
            }
            let output = output.unwrap_or_else(|| PathBuf::from(format!("/tmp/route_{route}.png")));
            let size = size.unwrap_or(UVec2::new(1280, 720));
            return Ok(Self {
                room_id: String::new(),
                focus: ae::Vec2::ZERO,
                output,
                size,
                warmup_frames: warmup_frames.max(90),
                frames,
                stride,
                // A route is its UI. Without it the capture is an empty clear
                // colour.
                include_ui: true,
                fit_room,
                character,
                follow_player: false,
                player_beside: None,
                interact_on_arrival: false,
                interact_at: None,
                flags: Vec::new(),
                switches: Vec::new(),
                hour: None,
                wears: Vec::new(),
                player_at: Vec::new(),
                player_climbs: false,
                seam: None,
                player_holds: Vec::new(),
                portal_shots: Vec::new(),
                shots: Vec::new(),
                items: Vec::new(),
                portal_effect_off: false,
                limbs_unseen: None,
                carry: true,
                gear: Vec::new(),
                tick_per_frame: false,
                guards: Vec::new(),
                body_warp: None,
                dev_overlays,
                combat_overlay,
                nav_overlay,
                boss_hp,
                boss_blows: Vec::new(),
                look_depth,
                camera_zoom,
                portal_windows,
                screen_effects,
                press,
                press_during,
                route: Some(route),
            });
        }

        let Some(room_id) = positional.first().cloned() else {
            return Err("missing ROOM_ID".to_string());
        };
        let Some(focus_text) = positional.get(1) else {
            return Err("missing X,Y focus (or the literal `player`)".to_string());
        };
        // `player` centers the camera on the live player entity (handy for
        // starting-character shots — no per-room spawn coordinate to look up).
        let follow_player = focus_text.eq_ignore_ascii_case("player");
        let focus = if follow_player {
            ae::Vec2::ZERO
        } else {
            parse_vec2(focus_text).ok_or_else(|| {
                format!("focus must be X,Y world coordinates or `player`, got '{focus_text}'")
            })?
        };
        let output = positional
            .get(2)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(format!("/tmp/room_{room_id}_game.png")));
        let size = positional
            .get(3)
            .and_then(|text| parse_image_size(text))
            .unwrap_or(UVec2::new(1280, 720));
        Ok(Self {
            room_id,
            focus,
            output,
            frames,
            stride,
            size,
            warmup_frames,
            include_ui,
            fit_room,
            character,
            dev_overlays,
            combat_overlay,
            nav_overlay,
            boss_hp,
            boss_blows,
            look_depth,
            camera_zoom,
            portal_windows,
            screen_effects,
            follow_player,
            player_beside,
            interact_on_arrival,
            interact_at,
            flags,
            switches,
            hour,
            wears,
            player_at,
            player_climbs,
            seam,
            player_holds,
            portal_shots,
            shots,
            items,
            portal_effect_off,
            limbs_unseen,
            carry,
            gear,
            tick_per_frame,
            guards,
            body_warp,
            route: None,
            press,
            press_during,
        })
    }
}

/// The press-driving frame that shot `shot` of a `--press-during` capture
/// opens on.
///
/// A function of the shot index, not a countdown, because the ordinary stride
/// (`stride_left`) is counted outside the press block and would stall the
/// press being photographed. Here the stride is in press-driving frames.
///
/// `None` when `--press-during` was not asked for, so the default tool is
/// unchanged.
fn press_during_shutter_frame(press_during: Option<u32>, shot: usize, stride: u32) -> Option<u32> {
    let first = press_during?;
    let offset = u32::try_from(shot)
        .unwrap_or(u32::MAX)
        .saturating_mul(stride.max(1));
    Some(first.saturating_add(offset))
}

/// Has the shutter's press-driving frame arrived?
///
/// Strictly greater: `--press-during 1` means one press frame has happened,
/// so the shot is on the next frame (for a tap, the key is still held).
/// Equality would photograph the frame whose render the step has not reached.
fn press_during_shutter_is_open(
    config: &SceneCaptureConfig,
    runtime: &SceneCaptureRuntime,
) -> bool {
    press_during_shutter_frame(config.press_during, runtime.shot, config.stride)
        .is_some_and(|at| runtime.press_frames > at)
}

/// When the press sequence is spent, end this capture and begin the next.
///
/// A confirmation starts a route change with its own load, cameras, and
/// readiness. Zeroing the frame count and clearing readiness re-runs the
/// route-wait machinery, and the deadline is recomputed with it.
///
/// Spent means all four are exhausted: no steps left, no tap awaiting its
/// release, no finger on the glass, no wait counting down. Every step type
/// (`Hold`, `Release`, `Wait`, tap) must reach this one check.
fn complete_press_sequence_if_spent(
    config: &SceneCaptureConfig,
    runtime: &mut SceneCaptureRuntime,
) {
    if runtime.press_cursor < config.press.len()
        || runtime.press_held.is_some()
        || runtime.touch_held.is_some()
        || runtime.press_wait > 0
    {
        return;
    }
    // The sequence ran out before the named frame. Falling through would
    // write the ordinary post-press image under a `--press-during` path, so
    // fail instead, as a route that adopts no camera does.
    if config.press_during.is_some() && !runtime.requested {
        runtime.press_during_missed = true;
        return;
    }
    if runtime.press_done_frame.is_some() {
        return;
    }
    runtime.press_done_frame = Some(0);
    runtime.frames = 0;
    runtime.world_ready = false;
    runtime.cameras_adopted = 0;
    eprintln!("capture_scene: press sequence complete; waiting for the state it asked for");
}

/// Silence developer overlays unless requested.
///
/// Enforce this as a system because startup reloads settings after construction.
/// Use normal player settings rather than disabling plugins so captures exercise
/// the same presentation configuration as the game.
fn silence_dev_overlays(
    config: Res<SceneCaptureConfig>,
    mut settings: ResMut<ambition_platformer2d::persistence::settings::UserSettings>,
    mut developer: Option<ResMut<ambition_platformer2d::dev_tools::dev_tools::DeveloperTools>>,
) {
    if config.dev_overlays {
        return;
    }
    if settings.gameplay.debug_hud_visible {
        settings.gameplay.debug_hud_visible = false;
    }
    if settings.video.show_fps {
        settings.video.show_fps = false;
    }
    if let Some(developer) = developer.as_mut() {
        if developer.show_hud {
            developer.show_hud = false;
        }
    }
}

/// One screen post-process effect, as `--screen-effect` names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScreenEffect {
    Crt,
    Grain,
    Vignette,
    Robot,
    Underwater,
    DeepDream,
}

impl ScreenEffect {
    fn parse(name: &str) -> Result<Self, String> {
        match name.trim().to_ascii_lowercase().as_str() {
            "crt" => Ok(Self::Crt),
            "grain" | "film_grain" => Ok(Self::Grain),
            "vignette" => Ok(Self::Vignette),
            "robot" | "robot_death" => Ok(Self::Robot),
            "underwater" => Ok(Self::Underwater),
            "deep_dream" | "deepdream" => Ok(Self::DeepDream),
            other => Err(format!(
                "unknown screen effect '{other}'; expected one of \
                 crt, grain, vignette, robot, underwater, deep_dream"
            )),
        }
    }

    /// Write this effect at full strength into the settings the shader reads.
    fn force(
        self,
        shaders: &mut ambition_platformer2d::persistence::settings::ScreenShaderSettings,
    ) {
        match self {
            Self::Crt => shaders.crt_strength = 1.0,
            Self::Grain => shaders.film_grain_strength = 1.0,
            Self::Vignette => shaders.vignette_strength = 1.0,
            Self::Robot => shaders.robot_death_strength = 1.0,
            Self::Underwater => shaders.underwater_strength = 1.0,
            Self::DeepDream => shaders.deep_dream_strength = 1.0,
        }
    }
}

/// Force the asked-for screen effects on, every frame, and say SO when the
/// visual-quality budget is about to scale them back to nothing.
///
/// The warning matters. `sync_screen_effect_settings_from_video_settings`
/// clamps the global strength to
/// `quality.budget.shaders.screen_shader_scale`, which is 0.0 on the Potato
/// tier, and a software rasteriser (`llvmpipe`) is seeded to Potato on a first
/// run. The effect then renders as nothing while every setting says it is on.
///
/// Every frame, like `force_combat_overlay`, because the settings load and the
/// quality seed both write this state and a Startup-only write races them.
fn force_screen_effects(
    config: Res<SceneCaptureConfig>,
    quality: Option<Res<ambition_platformer2d::render::quality::ResolvedVisualQuality>>,
    mut settings: ResMut<ambition_platformer2d::persistence::settings::UserSettings>,
    mut warned: Local<bool>,
) {
    if config.screen_effects.is_empty() {
        return;
    }
    let mut wanted = settings.video.shaders.clone();
    wanted.strength = 1.0;
    for effect in &config.screen_effects {
        effect.force(&mut wanted);
    }
    if settings.video.shaders != wanted {
        settings.video.shaders = wanted;
    }
    if !*warned {
        if let Some(quality) = quality.as_ref() {
            let scale = quality.budget.shaders.screen_shader_scale;
            if scale <= 0.001 {
                *warned = true;
                eprintln!(
                    "capture_scene: ⛔ the {:?} visual-quality tier scales screen \
                     shaders to {scale:.2}, so --screen-effect will photograph \
                     NOTHING. Re-run with AMBITION_QUALITY_PROFILE=high.",
                    quality.profile
                );
            }
        }
    }
}

/// Force the combat debug preset on, every frame, when `--combat-overlay` asked.
///
/// Every frame, because the settings load and the developer-tools default
/// both write this state and a Startup-only write races them. Idempotent.
fn force_combat_overlay(
    config: Res<SceneCaptureConfig>,
    mut dev_state: Option<ResMut<ambition_platformer2d::dev_tools::DeveloperRuntimeState>>,
    mut developer: Option<ResMut<ambition_platformer2d::dev_tools::dev_tools::DeveloperTools>>,
) {
    if !config.combat_overlay {
        return;
    }
    // Set all three gates the gizmo pass reads, so a swing is not shown
    // without its volume.
    if let (Some(dev_state), Some(developer)) = (dev_state.as_mut(), developer.as_mut()) {
        ambition_platformer2d::dev_tools::force_combat_overlay(
            dev_state,
            developer,
            Default::default(),
        );
    }
}

/// The systems a route capture adds on top of [`build_capture_app`], plus the
/// two things only a route needs to be told.
fn install_route_capture(app: &mut App, route_id: String) {
    // Only when it is the requested route: `compose_..._sequence` also makes
    // startup the initial route, which would put a card in front of every
    // other capture.
    if route_id == ambition_app::app::shell_host::AMBITION_STARTUP_ROUTE {
        ambition_app::app::shell_host::compose_ambition_startup_sequence(app);
    }

    let known: Vec<String> = {
        let catalog = app
            .world()
            .get_resource::<ambition_platformer2d::game_shell::ShellRouteCatalog>();
        catalog
            .map(|catalog| catalog.ids().map(|id| id.to_string()).collect())
            .unwrap_or_default()
    };
    if !known.is_empty() && !known.iter().any(|id| id == &route_id) {
        eprintln!(
            "capture_scene: unknown route '{route_id}'. Known routes: {}",
            known.join(", ")
        );
        std::process::exit(2);
    }

    app.add_systems(Startup, setup_route_capture_target);
    app.add_systems(
        Update,
        (
            silence_dev_overlays,
            force_combat_overlay,
            force_screen_effects,
            go_to_route,
            adopt_route_cameras,
            request_capture,
            finish_after_capture,
            fail_after_timeout,
            fail_when_the_shell_refuses_the_route,
        )
            .chain(),
    );
}

/// Drive the shell to the requested route — unless it is already there.
///
/// Wait for the router to settle first: before initialization there is no
/// active route to compare, and navigating would race the shell's boot.
/// `fail_after_timeout` covers a shell that never activates anything.
fn go_to_route(
    config: Res<SceneCaptureConfig>,
    mut runtime: ResMut<SceneCaptureRuntime>,
    router: Res<ambition_platformer2d::game_shell::ShellRouter>,
    mut commands: Commands,
) {
    if runtime.route_requested {
        return;
    }
    let Some(route) = config.route.clone() else {
        runtime.route_requested = true;
        return;
    };
    let Some(active) = router.active.as_ref() else {
        // The shell has not settled yet; do not race its boot.
        return;
    };
    runtime.route_requested = true;
    if active.route_id.as_str() == route {
        eprintln!(
            "capture_scene: the shell is already on '{route}'; photographing it \
             as booted rather than re-activating it"
        );
        return;
    }
    // A smash move must be photographed under smash rules, so seat a cast.
    // `--route smash_gameplay` with no roster shows an empty stage. This uses
    // the same two steps as `moveset_takes::reseat` (a roster resource and
    // the route change) instead of select-screen tap coordinates, which drift
    // and pick by grid cell, not by name.
    if route == ambition_demo_smash::SMASH_GAMEPLAY_ROUTE {
        if let Some(character) = config.character.clone() {
            eprintln!(
                "capture_scene: seating '{character}' twice on the smash stage,                  so the route comes up with a match rather than an empty stage"
            );
            commands.insert_resource(ambition_demo_smash::smash_roster([
                character.as_str(),
                character.as_str(),
            ]));
        } else {
            eprintln!(
                "capture_scene: '{route}' with no --character seats no cast; the                  stage will come up empty"
            );
        }
    }
    commands.write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
        ambition_platformer2d::game_shell::ShellRouteId::new(route),
    ));
}

/// Create the image a route capture draws into, and nothing else.
///
/// [`adopt_route_cameras`] retargets cameras every frame (a shell route has
/// no cameras at `Startup`); this makes the target they point at.
fn setup_route_capture_target(
    mut commands: Commands,
    config: Res<SceneCaptureConfig>,
    mut images: ResMut<Assets<Image>>,
) {
    if let Some(parent) = config.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        if let Err(error) = std::fs::create_dir_all(parent) {
            eprintln!(
                "capture_scene: failed to create output directory '{}': {error}",
                parent.display()
            );
            commands.write_message(AppExit::from_code(2));
            return;
        }
    }
    let mut capture_image = Image::new_target_texture(
        config.size.x.max(1),
        config.size.y.max(1),
        TextureFormat::Rgba8UnormSrgb,
        None,
    );
    capture_image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let image = images.add(capture_image);
    commands.insert_resource(SceneCaptureTarget { image });
}

/// Adopt a route's cameras AS they appear, every frame.
///
/// A shell route has no cameras at `Startup` — it spawns them when the route composes, several
/// frames in. The count is printed for that reason.
///
/// Only the cameras that draw to a window are the route's view. A camera that
/// draws into its own image (a rigged body's impostor atlas, a portal's view
/// cone) is part of the picture's making and keeps its target: retargeted, its
/// atlas stops being drawn, and on a frame where Bevy has sized its view for
/// its own image but it draws into this one, the render pass fails validation
/// and the app quits.
fn adopt_route_cameras(
    mut commands: Commands,
    target: Option<Res<SceneCaptureTarget>>,
    mut runtime: ResMut<SceneCaptureRuntime>,
    mut cameras: Query<(Entity, &mut Camera, Option<&RenderTarget>)>,
) {
    let Some(target_res) = target else {
        return;
    };
    let target = RenderTarget::Image(ImageRenderTarget::from(target_res.image.clone()));
    // One target, several cameras, so only the first may clear it. A shell
    // route composes a stack (world, UI, overlays); ordering by
    // `Camera::order` and clearing only on the lowest composites the stack.
    let mut ordered: Vec<(Entity, isize)> = cameras
        .iter()
        .filter(|(_, _, drawn_to)| match drawn_to {
            Some(RenderTarget::Image(image)) => image.handle == target_res.image,
            _ => true,
        })
        .map(|(entity, camera, _)| (entity, camera.order))
        .collect();
    ordered.sort_by_key(|(entity, order)| (*order, entity.index()));
    for (rank, (entity, order)) in ordered.iter().enumerate() {
        if let Ok((_, mut camera, _)) = cameras.get_mut(*entity) {
            camera.is_active = true;
            camera.clear_color = if rank == 0 {
                ClearColorConfig::Default
            } else {
                ClearColorConfig::None
            };
        }
        commands.entity(*entity).insert((target.clone(), Msaa::Off));
        eprintln!(
            "capture_scene: camera {rank} (order {order}) -> capture image{}",
            if rank == 0 { ", clears" } else { ", overlays" }
        );
    }
    if ordered.len() != runtime.cameras_adopted {
        eprintln!(
            "capture_scene: retargeted {} camera(s) to the capture image",
            ordered.len()
        );
        runtime.cameras_adopted = ordered.len();
    }
}

fn setup_capture_target(
    mut commands: Commands,
    config: Res<SceneCaptureConfig>,
    mut images: ResMut<Assets<Image>>,
    mut main_cameras: Query<(Entity, &mut Camera), With<MainCamera>>,
    mut hud_cameras: Query<(Entity, &mut Camera), (With<FrontHudCamera>, Without<MainCamera>)>,
) {
    if let Some(parent) = config.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        if let Err(error) = std::fs::create_dir_all(parent) {
            eprintln!(
                "capture_scene: failed to create output directory '{}': {error}",
                parent.display()
            );
            commands.write_message(AppExit::from_code(2));
            return;
        }
    }

    let mut capture_image = Image::new_target_texture(
        config.size.x.max(1),
        config.size.y.max(1),
        TextureFormat::Rgba8UnormSrgb,
        None,
    );
    capture_image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let image = images.add(capture_image);
    let target = RenderTarget::Image(ImageRenderTarget::from(image.clone()));

    for (entity, mut camera) in &mut main_cameras {
        camera.is_active = true;
        commands.entity(entity).insert((target.clone(), Msaa::Off));
    }
    for (entity, mut camera) in &mut hud_cameras {
        camera.is_active = config.include_ui;
        if config.include_ui {
            commands.entity(entity).insert((target.clone(), Msaa::Off));
        }
    }
    commands.insert_resource(SceneCaptureTarget { image });
}

/// Point the cube menu's camera at the capture image under `--include-ui`.
///
/// The menu owns that camera's `is_active` (it is off until the menu opens), so
/// only the target is adopted here — every frame, because the menu may spawn it
/// after `Startup`. Its sample count must match the other cameras on the target
/// (`Msaa::Off`), or it draws its clear and drops every mesh.
fn adopt_menu_camera(
    mut commands: Commands,
    config: Res<SceneCaptureConfig>,
    target: Option<Res<SceneCaptureTarget>>,
    cameras: Query<
        Entity,
        (
            With<ambition_menu_kaleidoscope::KaleidoscopePauseCamera>,
            Without<AdoptedMenuCamera>,
        ),
    >,
) {
    let (true, Some(target)) = (config.include_ui, target) else {
        return;
    };
    let want = RenderTarget::Image(ImageRenderTarget::from(target.image.clone()));
    for entity in &cameras {
        commands
            .entity(entity)
            .insert((want.clone(), Msaa::Off, AdoptedMenuCamera));
    }
}

/// The sim tick `--player-beside` arrives on when it names none: a second
/// into the session at 60 Hz, after the room has settled.
const DEFAULT_ARRIVAL_TICK: u64 = 60;

/// Stand the primary player beside the body `--player-beside` names, on the
/// sim tick it names: at that body's right edge, feet on its floor. With
/// `--interact-on-arrival`, press Interact on the same tick, in the primary
/// seat's interact buffer: the state the device path fills and doors,
/// dialogue and petting read.
///
/// Both writes are rollback state, so this runs in the sim, and it decides
/// from rollback state alone — the tick and the immutable config — so a replay
/// of that tick arrives again and a replay of any other does not. It remembers
/// nothing and reads nothing the host raised.
fn place_player_beside(
    config: Res<SceneCaptureConfig>,
    tick: Res<ambition_platformer2d::time::SimTick>,
    mut player: Query<
        (ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel),
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
    mut seats: ResMut<ambition_platformer2d::characters::control::SlotInteractionState>,
    others: Query<
        (
            &ambition_platformer2d::characters::actor::WornCharacter,
            &ambition_platformer2d::actor::BodyKinematics,
        ),
        Without<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
    >,
) {
    if config.interact_at == Some(tick.get()) {
        seats.primary_mut().interact_buffer_timer = 0.15;
    }
    let Some((id, arrival)) = config.player_beside.as_ref() else {
        return;
    };
    if tick.get() != *arrival {
        return;
    }
    let Some((_, target)) = others.iter().find(|(worn, _)| worn.id() == id.as_str()) else {
        eprintln!("capture_scene: --player-beside found no body wearing {id} on tick {arrival}");
        return;
    };
    let Ok((mut clusters, mut model)) = player.single_mut() else {
        return;
    };
    let mut clusters = clusters.as_clusters_mut();
    // Overlapping its right edge, where a player walking up would be: an
    // interaction reaches a body whose box its own box overlaps. Positions are
    // centres and y grows down: line the feet up. A teleport, at rest, through
    // the motion authority (ADR 0024).
    let at = ae::Vec2::new(
        target.pos.x + target.size.x * 0.5,
        target.pos.y + (target.size.y - clusters.kinematics.size.y) * 0.5,
    );
    ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
    if config.interact_on_arrival {
        seats.primary_mut().interact_buffer_timer = 0.15;
    }
    eprintln!(
        "capture_scene: placed the player beside {id} at {:?} on tick {arrival}",
        clusters.kinematics.pos
    );
}

/// A cube camera [`adopt_menu_camera`] has already pointed at the capture.
#[derive(Component)]
struct AdoptedMenuCamera;

fn apply_capture_snapshot(
    config: Res<SceneCaptureConfig>,
    world: ambition_platformer2d::platformer::lifecycle::SoleLiveRoom<
        ambition_platformer2d::engine_core::RoomGeometry,
    >,
    room_set: ambition_platformer2d::world::rooms::SoleLiveRoomSpec,
    user_settings: Res<ambition_platformer2d::persistence::settings::UserSettings>,
    ease_tuning: Res<ambition_platformer2d::platformer::camera_ease::CameraEaseTuning>,
    // `CameraViewState` is a component on the local view. A capture app
    // stages one view, so write that one.
    mut view_states: Query<
        (Entity, &mut CameraViewState),
        With<ambition_platformer2d::sim_view::LocalView>,
    >,
    // The sim body, not the render visual. `PlayerVisual` is a render-side
    // marker on an entity with no kinematics, so querying it would never
    // match and `player` focus would fall back to the origin.
    // With its standing size, so a crouch is framed as the game frames it
    // (`CameraFocus2d::stable_center`), not as a step down.
    player_q: Query<
        (&ambition_platformer2d::actor::BodyKinematics, Option<&ae::BodyBaseSize>),
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
    mut cameras: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
    host_view: Option<ResMut<ambition_platformer2d::portal_presentation::PortalCameraContinuityHostView>>,
    mut observer_views: Option<ResMut<ambition_platformer2d::portal_presentation::PortalObserverViews>>,
    portals: Query<&ambition_platformer2d::portal::PlacedPortal>,
    tuning: Option<Res<ambition_platformer2d::portal::PortalTuning>>,
) {
    let active_spec = room_set.spec();
    let (base_view_w, base_view_h) = user_settings.video.camera_zoom.base_view();
    let base_view = ae::Vec2::new(base_view_w, base_view_h);
    // A fixed focus is a point; a followed player is its body.
    let followed = player_q.iter().next().filter(|_| config.follow_player);
    let (focus_center, focus_size, focus_base) = match followed {
        Some((kin, base)) => (kin.pos, kin.size, base.map_or(kin.size, |b| b.base_size)),
        None => (config.focus, ae::Vec2::new(30.0, 48.0), ae::Vec2::new(30.0, 48.0)),
    };
    // `--fit-room`: the whole room, centred, scaled to fit. The resolver below
    // clamps to the room and to camera zones, so it cannot give a wide shot.
    if config.fit_room {
        // Use the projection's scaling mode, not a multiplier on the base
        // view: `scale` multiplies an extent that depends on the mode and the
        // viewport aspect.
        for (mut transform, mut projection) in &mut cameras {
            if let Projection::Orthographic(orthographic) = &mut *projection {
                orthographic.scale = 1.0;
                orthographic.scaling_mode = bevy::camera::ScalingMode::AutoMin {
                    min_width: world.0.size.x,
                    min_height: world.0.size.y,
                };
            }
            transform.translation.x = 0.0;
            transform.translation.y = 0.0;
            transform.rotation = Quat::IDENTITY;
        }
        return;
    }

    let snapshot = resolve_follow_camera_snapshot(
        CameraSnapshotResolveInput {
            world: &world.0,
            camera_zones: &active_spec.camera_zones,
            focus: CameraFocus2d {
                center_world: focus_center,
                tick_center_world: focus_center,
                size: focus_size,
                base_size: focus_base,
                facing: 1.0,
                velocity_world: ae::Vec2::ZERO,
            },
            base_view,
            viewport_px: ae::Vec2::new(config.size.x as f32, config.size.y as f32),
            aspect_policy: user_settings.video.camera_aspect,
            framing: user_settings.video.camera_framing,
            overview_scale: 1.0,
            encounter_scale: 1.0,
            overview_camera: false,
            snap_camera: true,
            blink: None,
            dt: 0.0,
            mode: CameraSnapshotResolveMode::Instant,
            extra_clamp_center_world: None,
            chart_transit: None,
            // This path bypasses the live resolve. A captured match goes
            // through `resolve_camera_observation`, which frames the cast.
            must_frame_world: None,
            ease_tuning: *ease_tuning,
            screen_framing: None,
            reference_frame: Default::default(),
            subject_down: None,
        },
        None,
    );

    let x = snapshot.center_world.x - world.0.size.x * 0.5;
    let y = world.0.size.y * 0.5 - snapshot.center_world.y;
    // `camera_follow` sampled the camera before this pin, into the host view
    // and into each view's own sample: its centre and how much of the world
    // it shows. The portal windows read the view's own
    // sample (`PortalObserverViews`), so pin both. A window drawn for the
    // follow camera's centre shows its far side's backdrop for the wrong eye
    // (29 px off on the c146/c147 corridor, 2026-10-16).
    for (view_entity, mut view_state) in &mut view_states {
        *view_state = CameraViewState::from(&snapshot);
        if let Some(observer_views) = observer_views.as_deref_mut() {
            let view = observer_views.of_mut(view_entity);
            if view.initialized {
                view.current_center_world = snapshot.center_world;
                view.visible_view = snapshot.visible_view;
            }
        }
    }
    if let Some(mut host_view) = host_view {
        if host_view.initialized {
            host_view.current_center_world = snapshot.center_world;
            host_view.visible_view = snapshot.visible_view;
        }
    }

    // The far side of a seam check: the camera is where the view through the
    // portal is. Only the camera moves; the views the windows read keep the
    // eye's own camera.
    let (x, y, rotation) = match config.seam.as_ref().filter(|seam| seam.far) {
        None => (x, y, snapshot.rotation_radians),
        Some(seam) => {
            let portals: Vec<_> = portals.iter().cloned().collect();
            let convention = tuning.as_deref().map(|t| t.convention.map_convention()).unwrap_or_default();
            let Some(through) = seam.pick(&portals).ok().and_then(|portal| {
                camera_through(&portals, portal, convention, snapshot.center_world, snapshot.rotation_radians)
            }) else {
                return;
            };
            let render = world_to_render(world.0.size, through.center);
            (render.x, render.y, through.rotation)
        }
    };
    for (mut transform, mut projection) in &mut cameras {
        if let Projection::Orthographic(orthographic) = &mut *projection {
            orthographic.scale = snapshot.orthographic_scale;
        }
        transform.translation.x = x;
        transform.translation.y = y;
        transform.rotation = Quat::from_rotation_z(rotation);
    }
}

/// One side of a portal window seam check (`--seam PORTAL:window|far`).
#[derive(Clone, Debug)]
struct SeamCheck {
    /// The portal whose window is checked: a channel name (`c131`), or a
    /// world `X,Y` (the portal nearest to it).
    portal: String,
    /// The far side: the camera is where the view through the portal is, and
    /// no window is drawn.
    far: bool,
}

impl SeamCheck {
    fn parse(value: &str) -> Result<Self, String> {
        let (channel, side) = value
            .rsplit_once(':')
            .ok_or_else(|| format!("--seam wants PORTAL:window or PORTAL:far, got '{value}'"))?;
        let far = match side {
            "window" => false,
            "far" => true,
            _ => return Err(format!("--seam side must be window or far, got '{side}'")),
        };
        Ok(Self { portal: channel.to_string(), far })
    }

    /// The portal this check names, in `portals`.
    fn pick<'a>(
        &self,
        portals: &'a [ambition_platformer2d::portal::PlacedPortal],
    ) -> Result<&'a ambition_platformer2d::portal::PlacedPortal, String> {
        let found = match parse_vec2(&self.portal) {
            Some(near) => portals.iter().min_by(|a, b| a.pos.distance(near).total_cmp(&b.pos.distance(near))),
            None => portals.iter().find(|p| p.channel.name() == self.portal),
        };
        found.ok_or_else(|| {
            let names: Vec<_> = portals.iter().map(|p| p.channel.name()).collect();
            format!("no portal {} in the room (it has {})", self.portal, names.join(", "))
        })
    }
}

/// A camera's place in the world: its centre (world, y down) and its turn
/// (render frame, the camera transform's).
#[derive(Clone, Copy, Debug)]
struct CameraPlace {
    center: Vec2,
    rotation: f32,
}

/// The image through `portal` of a camera at `center` turned by
/// `rotation`: where a camera on the far side shows what the window shows.
/// The centre goes through the pair's map, and the camera's `+x` too. A map
/// that is a reflection has its `+y` the other way; the pixel map of the
/// check ([`seam_pixel_map`]) carries that, a camera cannot.
fn camera_through(
    portals: &[ambition_platformer2d::portal::PlacedPortal],
    portal: &ambition_platformer2d::portal::PlacedPortal,
    convention: ambition_platformer2d::portal::pieces::MapConvention,
    center: Vec2,
    rotation: f32,
) -> Option<CameraPlace> {
    use ambition_platformer2d::portal::pieces::{map_point, portal_map_vec};
    let partner = ambition_platformer2d::portal::find_portal(portals, portal.channel.partner())?;
    let (enter, exit) = (portal.aperture(), partner.aperture());
    // The camera's `+x` in the world (y down).
    let x_axis = Vec2::new(rotation.cos(), -rotation.sin());
    let axis = portal_map_vec(x_axis, enter.frame.normal, exit.frame.normal, convention).normalize_or(Vec2::X);
    Some(CameraPlace {
        center: map_point(center, &enter.frame, &exit.frame, convention),
        rotation: f32::atan2(-axis.y, axis.x),
    })
}

/// A world point (y down, the room's corner at the origin) in the render
/// frame (y up, the room's centre at the origin).
fn world_to_render(room: Vec2, p: Vec2) -> Vec2 {
    Vec2::new(p.x - room.x * 0.5, room.y * 0.5 - p.y)
}

fn render_to_world(room: Vec2, r: Vec2) -> Vec2 {
    Vec2::new(r.x + room.x * 0.5, room.y * 0.5 - r.y)
}

/// The pixel of a camera's image at render point `r`, and back.
struct ShotFrame {
    center: Vec2,
    rotation: f32,
    area: bevy::math::Rect,
    size: Vec2,
}

impl ShotFrame {
    fn pixel(&self, r: Vec2) -> Vec2 {
        let view = Vec2::from_angle(-self.rotation).rotate(r - self.center);
        Vec2::new(
            (view.x - self.area.min.x) / self.area.width() * self.size.x,
            (self.area.max.y - view.y) / self.area.height() * self.size.y,
        )
    }

    fn render(&self, pixel: Vec2) -> Vec2 {
        let view = Vec2::new(
            self.area.min.x + pixel.x / self.size.x * self.area.width(),
            self.area.max.y - pixel.y / self.size.y * self.area.height(),
        );
        self.center + Vec2::from_angle(self.rotation).rotate(view)
    }
}

/// What the window side of a seam check reads at its shot.
#[derive(SystemParam)]
struct SeamProbe<'w, 's> {
    cones: Query<
        'w,
        's,
        (&'static Name, &'static Mesh2d, &'static GlobalTransform, &'static InheritedVisibility),
        With<ambition_platformer2d::portal_presentation::PortalConeMesh>,
    >,
    meshes: Res<'w, Assets<Mesh>>,
    cameras: Query<'w, 's, (&'static GlobalTransform, &'static Projection), With<MainCamera>>,
    portals: Query<'w, 's, &'static ambition_platformer2d::portal::PlacedPortal>,
    tuning: Option<Res<'w, ambition_platformer2d::portal::PortalTuning>>,
    rooms: Query<
        'w,
        's,
        &'static ambition_platformer2d::engine_core::RoomGeometry,
        With<ambition_platformer2d::platformer::lifecycle::RoomInstanceRoot>,
    >,
}

/// Write `<OUT>.seam.json` for the window side of a seam check: the window
/// of the portal in pixels, its aperture, and the affine map from a pixel of
/// this shot to the pixel of the far shot that shows the same place.
fn write_seam_window(config: &SceneCaptureConfig, probe: &SeamProbe) -> Result<(), String> {
    let Some(seam) = config.seam.as_ref().filter(|seam| !seam.far) else {
        return Ok(());
    };
    let room = probe.rooms.iter().next().ok_or("no live room")?.0.size;
    let (camera, projection) = probe.cameras.iter().next().ok_or("no main camera")?;
    let Projection::Orthographic(ortho) = projection else {
        return Err("the main camera is not orthographic".to_string());
    };
    let (_, turn, at) = camera.to_scale_rotation_translation();
    let near = ShotFrame {
        center: at.truncate(),
        rotation: turn.to_euler(EulerRot::ZYX).0,
        area: ortho.area,
        size: config.size.as_vec2(),
    };
    let portals: Vec<_> = probe.portals.iter().cloned().collect();
    let portal = seam.pick(&portals)?;
    let window_name = format!("Portal view window ({})", portal.channel.name());
    let mut window: Vec<[f32; 2]> = Vec::new();
    for (name, mesh, transform, visible) in &probe.cones {
        if name.as_str() != window_name || !visible.get() {
            continue;
        }
        let mesh = probe.meshes.get(&mesh.0).ok_or("the window has no mesh")?;
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            return Err("the window mesh has no positions".to_string());
        };
        window = positions
            .iter()
            .map(|p| near.pixel(transform.transform_point(Vec3::from_array(*p)).truncate()).to_array())
            .collect();
    }
    let convention = probe.tuning.as_deref().map(|t| t.convention.map_convention()).unwrap_or_default();
    let partner = ambition_platformer2d::portal::find_portal(&portals, portal.channel.partner())
        .ok_or_else(|| format!("portal {} has no partner", portal.channel.name()))?;
    let (enter, exit) = (portal.aperture(), partner.aperture());
    let through = camera_through(
        &portals,
        portal,
        convention,
        render_to_world(room, near.center),
        near.rotation,
    )
    .ok_or("the portal has no view through it")?;
    let far = ShotFrame {
        center: world_to_render(room, through.center),
        rotation: through.rotation,
        area: ortho.area,
        size: near.size,
    };
    // The map is affine: three pixels fix it.
    let to_far = |pixel: Vec2| {
        let world = render_to_world(room, near.render(pixel));
        let image = ambition_platformer2d::portal::pieces::map_point(world, &enter.frame, &exit.frame, convention);
        far.pixel(world_to_render(room, image))
    };
    let (o, ex, ey) = (to_far(Vec2::ZERO), to_far(Vec2::X), to_far(Vec2::Y));
    let affine = [ex.x - o.x, ey.x - o.x, o.x, ex.y - o.y, ey.y - o.y, o.y];
    let tangent = enter.frame.tangent() * enter.half_length;
    let aperture: Vec<[f32; 2]> = [enter.frame.origin - tangent, enter.frame.origin + tangent]
        .iter()
        .map(|end| near.pixel(world_to_render(room, *end)).to_array())
        .collect();
    let report = serde_json::json!({
        "channel": portal.channel.name(),
        "image": [config.size.x, config.size.y],
        "window": window,
        "aperture": aperture,
        "to_far": affine,
    });
    let path = config.output.with_extension("seam.json");
    std::fs::write(&path, serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// A seam check hides the parallax layer in front of the play on both sides:
/// it is between the eye and the portal, and the window does not show it.
fn hide_the_front_parallax(
    mut layers: Query<(&ambition_platformer2d::render::rendering::ParallaxLayerVisual, &mut Visibility)>,
) {
    for (layer, mut visibility) in &mut layers {
        if layer.z >= ambition_platformer2d::render::rendering::FOREGROUND_PARALLAX_Z {
            visibility.set_if_neq(Visibility::Hidden);
        }
    }
}

/// Generous: a route that loads assets can take a while, and a false failure
/// is as bad as a false success.
const ROUTE_CAMERA_GRACE_FRAMES: u32 = 600;

/// Whether there is a constructed world to count warmup from.
///
/// Two conditions:
///
/// The body exists. A player-focused capture waits for the body it centres
/// on; a coordinate-focused one has nothing to wait for.
///
/// Its art has a terminal answer. A decoded sheet resizes its body
/// (`SpritePosedBody` derives the collision box from the art), so art that
/// lands on a different frame each run makes the body settle differently.
/// `character_reveal_ready` says every staged character finished loading, one
/// way or the other (§4.9).
fn world_is_ready(
    player_q: &Query<
        &ambition_platformer2d::actor::BodyKinematics,
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
    follow_player: bool,
    art: Option<(&CharacterLoadDemand, &CharacterLoadStates)>,
) -> bool {
    if follow_player && player_q.iter().next().is_none() {
        return false;
    }
    match art {
        Some((demand, states)) => {
            ambition_platformer2d::assets::materialization::character_reveal_ready(demand, states)
        }
        // A composition with no character-load seam has no art to wait for.
        None => true,
    }
}

/// The glass a `touch:` step presses on.
///
/// A finger reaches the app two ways, and a phone drives both:
///
/// - `TouchInput` messages, which Bevy folds into `Touches`. The touch overlay
///   and the select screen read that.
/// - A picking pointer. Bevy makes one from the window's touch events, but a
///   capture has no window, so this spawns the pointer on the capture image.
///   Without it a tap never reaches a widget that uses Bevy picking (the cube
///   menu, Lunex controls, picked `bevy_ui` nodes), and a tap that should be
///   blocked by a node on top is not blocked.
#[derive(SystemParam)]
struct Glass<'w, 's> {
    fingers: MessageWriter<'w, TouchInput>,
    pointers: MessageWriter<'w, PointerInput>,
    target: Option<Res<'w, SceneCaptureTarget>>,
    commands: Commands<'w, 's>,
}

impl Glass<'_, '_> {
    fn location(&self, at: Vec2) -> Option<Location> {
        let image = self.target.as_ref()?.image.clone();
        let target = RenderTarget::Image(ImageRenderTarget::from(image)).normalize(None)?;
        Some(Location {
            target,
            position: at,
        })
    }

    fn touch(&mut self, phase: TouchPhase, id: u64, at: Vec2) {
        self.fingers.write(TouchInput {
            phase,
            position: at,
            window: Entity::PLACEHOLDER,
            force: None,
            id,
        });
    }

    fn down(&mut self, id: u64, at: Vec2) {
        self.touch(TouchPhase::Started, id, at);
        if let Some(location) = self.location(at) {
            let pointer = PointerId::Touch(id);
            // Bevy retires the pointer itself once `Touches` reports the lift.
            self.commands
                .spawn((pointer, PointerLocation::new(location.clone())));
            self.pointers.write(PointerInput::new(
                pointer,
                location,
                PointerAction::Press(PointerButton::Primary),
            ));
        }
    }

    fn slide(&mut self, id: u64, from: Vec2, to: Vec2) {
        self.touch(TouchPhase::Moved, id, to);
        if let Some(location) = self.location(to) {
            self.pointers.write(PointerInput::new(
                PointerId::Touch(id),
                location,
                PointerAction::Move { delta: to - from },
            ));
        }
    }

    fn up(&mut self, id: u64, at: Vec2) {
        self.touch(TouchPhase::Ended, id, at);
        if let Some(location) = self.location(at) {
            self.pointers.write(PointerInput::new(
                PointerId::Touch(id),
                location,
                PointerAction::Release(PointerButton::Primary),
            ));
        }
    }
}

/// Deliver one frame of the `--press` sequence: a countdown tick, a pending
/// release, or the next step. Separate from `request_capture` because
/// `--press-during` can photograph a press frame instead of driving it.
fn drive_press_frame(
    config: &SceneCaptureConfig,
    runtime: &mut SceneCaptureRuntime,
    keys: &mut ButtonInput<KeyCode>,
    glass: &mut Glass,
) {
    if runtime.press_wait > 0 {
        runtime.press_wait -= 1;
        complete_press_sequence_if_spent(config, runtime);
        return;
    }
    if let Some(key) = runtime.press_held.take() {
        keys.release(key);
        complete_press_sequence_if_spent(config, runtime);
    } else if let Some(at) = runtime.touch_held.take() {
        let id = runtime.next_touch_id - 1;
        if let Some(next) = runtime.touch_path.pop() {
            glass.slide(id, at, next);
            runtime.touch_held = Some(next);
            return;
        }
        // Lift where the finger now is. For a tap that is where it went down:
        // a touch that ends elsewhere is a drag.
        glass.up(id, at);
        complete_press_sequence_if_spent(config, runtime);
    } else {
        match config.press[runtime.press_cursor] {
            PressStep::Tap(key) => {
                keys.press(key);
                runtime.press_held = Some(key);
                eprintln!(
                    "capture_scene: pressed {key:?} ({} of {})",
                    runtime.press_cursor + 1,
                    config.press.len()
                );
            }
            PressStep::Hold(key) => {
                // Not recorded in `press_held`: that field is the tap awaiting
                // release next frame, and a hold outlives its step.
                keys.press(key);
                eprintln!(
                    "capture_scene: holding {key:?} ({} of {})",
                    runtime.press_cursor + 1,
                    config.press.len()
                );
            }
            PressStep::Release(key) => {
                keys.release(key);
                eprintln!(
                    "capture_scene: released {key:?} ({} of {})",
                    runtime.press_cursor + 1,
                    config.press.len()
                );
            }
            PressStep::Drag(from, to) => {
                let id = runtime.next_touch_id;
                runtime.next_touch_id += 1;
                glass.down(id, from);
                runtime.touch_held = Some(from);
                runtime.touch_path = (1..=DRAG_FRAMES)
                    .rev()
                    .map(|step| from.lerp(to, step as f32 / DRAG_FRAMES as f32))
                    .collect();
                eprintln!(
                    "capture_scene: dragging ({:.0}, {:.0}) to ({:.0}, {:.0}) ({} of {})",
                    from.x,
                    from.y,
                    to.x,
                    to.y,
                    runtime.press_cursor + 1,
                    config.press.len()
                );
            }
            PressStep::Touch(at) => {
                let id = runtime.next_touch_id;
                runtime.next_touch_id += 1;
                glass.down(id, at);
                runtime.touch_held = Some(at);
                eprintln!(
                    "capture_scene: touched ({:.0}, {:.0}) ({} of {})",
                    at.x,
                    at.y,
                    runtime.press_cursor + 1,
                    config.press.len()
                );
            }
            PressStep::Wait(frames) => {
                runtime.press_wait = frames;
                eprintln!(
                    "capture_scene: waiting {frames} frames ({} of {})",
                    runtime.press_cursor + 1,
                    config.press.len()
                );
            }
        }
        runtime.press_cursor += 1;
        complete_press_sequence_if_spent(config, runtime);
    }
}

fn request_capture(
    mut commands: Commands,
    config: Res<SceneCaptureConfig>,
    target: Option<Res<SceneCaptureTarget>>,
    mut runtime: ResMut<SceneCaptureRuntime>,
    player_q: Query<
        &ambition_platformer2d::actor::BodyKinematics,
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
    seated_q: Query<(
        &ambition_platformer2d::actor::MatchSeat,
        &ambition_platformer2d::actor::BodyKinematics,
    )>,
    art_demand: Option<Res<CharacterLoadDemand>>,
    art_states: Option<Res<CharacterLoadStates>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut glass: Glass,
    sim_tick: Option<Res<ambition_platformer2d::time::SimTick>>,
    seam: SeamProbe,
) {
    if runtime.requested || runtime.completed {
        if runtime.requested {
            runtime.wait_frames = runtime.wait_frames.saturating_add(1);
        }
        return;
    }
    if runtime.press_during_missed {
        runtime.failed = true;
        runtime.requested = true;
        eprintln!(
            "capture_scene: --press-during {} never arrived. The press sequence was spent after \
             {} driving frame(s){}, so the frame you named does not exist. No image is written: a \
             post-press capture filed under --press-during would be a photograph of a different \
             moment, which is exactly what this flag exists to avoid. Add a trailing `wait:N` step \
             to lengthen the sequence, or name an earlier frame.",
            config.press_during.unwrap_or(0),
            runtime.press_frames,
            if config.frames > 1 {
                format!(
                    " and {} of {} shot(s) were taken",
                    runtime.shot, config.frames
                )
            } else {
                String::new()
            },
        );
        commands.write_message(AppExit::from_code(2));
        return;
    }
    // The gap between shots of a sequence. Counted here because this system
    // runs once per frame; the readback handler fires on a GPU event.
    if runtime.stride_left > 0 {
        runtime.stride_left -= 1;
        return;
    }
    // Warmup counts from a ready world, not from boot. Frames spent waiting
    // for the body are latency, not warmup.
    if !runtime.world_ready {
        let art = art_demand.as_deref().zip(art_states.as_deref());
        if !world_is_ready(&player_q, config.follow_player, art) {
            // A press sequence can end the session (a quit to the title).
            // Then no player comes, and this wait had no end (measured
            // 2026-10-09: two runs of 25 minutes with no image).
            if config.follow_player && runtime.press_done_frame.is_some() && player_q.iter().next().is_none() {
                runtime.no_player_after_press += 1;
                if runtime.no_player_after_press > ROUTE_CAMERA_GRACE_FRAMES {
                    eprintln!(
                        "capture_scene: the press sequence is complete and there is no player to follow \
                         after {ROUTE_CAMERA_GRACE_FRAMES} frames. If the sequence ends the session (a quit \
                         to the title), name the focus as X,Y and not `player`. No image is written."
                    );
                    commands.write_message(AppExit::from_code(2));
                }
            }
            return;
        }
        runtime.world_ready = true;
        runtime.ever_ready = true;
    }
    runtime.frames += 1;
    if runtime.frames < config.warmup_frames.max(1) {
        return;
    }
    // Drive the route's own lobby before the shutter. `press_wait` belongs
    // in this condition, or a trailing `wait:N` is never counted down.
    if runtime.press_cursor < config.press.len()
        || runtime.press_held.is_some()
        || runtime.touch_held.is_some()
        || runtime.press_wait > 0
    {
        runtime.press_frames = runtime.press_frames.saturating_add(1);
        // The shutter opens mid-sequence, and this frame drives nothing.
        // The photograph shows the world as the previous frame's input
        // left it; for a tap, the key is still down.
        if !press_during_shutter_is_open(&config, &runtime) {
            drive_press_frame(&config, &mut runtime, &mut keys, &mut glass);
            return;
        }
        // A shutter frame drives nothing, so it is not a press frame. Counting
        // it would let the clock run on while a readback is in flight, past
        // every later shot's frame, and the sequence would never drive again.
        runtime.press_frames -= 1;
    }
    if let Some(kin) = player_q.iter().next() {
        println!(
            "capture_scene: subject at ({:.4}, {:.4}) after {} warmup tick(s)",
            kin.pos.x, kin.pos.y, runtime.frames
        );
    } else {
        // Seat order, not query order: Bevy iterates by archetype, so
        // unsorted rows would differ between captures of the same match.
        let mut seated: Vec<_> = seated_q
            .iter()
            .map(|(seat, kin)| (seat.0, kin.pos))
            .collect();
        seated.sort_by_key(|(seat, _)| *seat);
        if seated.is_empty() {
            // Say so: "no pose line" and "no subject" must be distinguishable.
            println!(
                "capture_scene: NO SUBJECT — no primary player and no seated body \
                 after {} warmup tick(s); this image proves nothing about a pose",
                runtime.frames
            );
        } else {
            for (seat, pos) in seated {
                println!(
                    "capture_scene: seat {seat} at ({:.4}, {:.4}) after {} warmup tick(s)",
                    pos.x, pos.y, runtime.frames
                );
            }
        }
    }
    // A route capture waits for a camera, not for a clock. Warmup is a
    // duration; readiness is a fact. Otherwise a route that never builds a
    // camera gives a blank PNG and success. Rooms are exempt: this binary
    // sets up their camera itself; a route's comes from the shell later.
    if config.route.is_some() && runtime.cameras_adopted == 0 {
        if runtime.frames < config.warmup_frames.max(1) + ROUTE_CAMERA_GRACE_FRAMES {
            return;
        }
        runtime.failed = true;
        runtime.requested = true;
        eprintln!(
            "capture_scene: route '{}' adopted NO camera within {} frames. Nothing would \
             have been drawn, so no image is written — a blank PNG reported as success is \
             the failure this check exists for. Check that the route id is real and that \
             its presentation actually builds a camera.",
            config.route.as_deref().unwrap_or("<none>"),
            config.warmup_frames.max(1) + ROUTE_CAMERA_GRACE_FRAMES,
        );
        commands.write_message(AppExit::from_code(2));
        return;
    }
    let Some(target) = target else {
        return;
    };
    if let Err(why) = write_seam_window(&config, &seam) {
        eprintln!("capture_scene: --seam: {why}. No image is written.");
        runtime.failed = true;
        runtime.requested = true;
        commands.write_message(AppExit::from_code(2));
        return;
    }
    commands
        .spawn(Readback::texture(target.image.clone()))
        .observe(save_readback_to_disk);
    runtime.requested = true;
    // The sim tick of the shot, so a film lines up with a sim trace.
    eprintln!(
        "capture_scene: shot {} at sim tick {} -> {}",
        runtime.shot,
        sim_tick.map_or(0, |tick| tick.get()),
        config.output.display()
    );
}

fn finish_after_capture(
    mut commands: Commands,
    config: Res<SceneCaptureConfig>,
    runtime: Res<SceneCaptureRuntime>,
) {
    if !runtime.completed || runtime.failed {
        return;
    }
    if config.frames > 1 {
        println!(
            "capture_scene: wrote {} frame(s) as {}.NNNN.{} ({}x{} px, stride {})",
            config.frames,
            config.output.with_extension("").display(),
            config
                .output
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_else(|| "png".to_string()),
            config.size.x,
            config.size.y,
            config.stride,
        );
    } else {
        println!(
            "capture_scene: wrote {} ({}x{} px)",
            config.output.display(),
            config.size.x,
            config.size.y,
        );
    }
    commands.write_message(AppExit::Success);
}

fn save_readback_to_disk(
    event: On<ReadbackComplete>,
    mut commands: Commands,
    config: Res<SceneCaptureConfig>,
    mut runtime: ResMut<SceneCaptureRuntime>,
) {
    commands.entity(event.entity).despawn();
    let width = config.size.x.max(1);
    let height = config.size.y.max(1);
    let row_bytes = width as usize * 4;
    let padded_row_bytes = row_bytes.div_ceil(256) * 256;
    let expected = padded_row_bytes * height as usize;
    if event.data.len() < expected {
        eprintln!(
            "capture_scene: readback returned {} bytes, expected at least {expected}",
            event.data.len()
        );
        runtime.failed = true;
        runtime.completed = true;
        commands.write_message(AppExit::from_code(1));
        return;
    }

    let mut pixels = vec![0u8; row_bytes * height as usize];
    for y in 0..height as usize {
        let src = y * padded_row_bytes;
        let dst = y * row_bytes;
        pixels[dst..dst + row_bytes].copy_from_slice(&event.data[src..src + row_bytes]);
    }

    let Some(image) = image::RgbaImage::from_raw(width, height, pixels) else {
        eprintln!("capture_scene: failed to build PNG buffer");
        runtime.failed = true;
        runtime.completed = true;
        commands.write_message(AppExit::from_code(1));
        return;
    };
    // One shot keeps its exact name; a sequence numbers its files. Existing
    // recipes depend on the single-shot path.
    let path = if config.frames <= 1 {
        config.output.clone()
    } else {
        let stem = config
            .output
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "frame".to_string());
        let ext = config
            .output
            .extension()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "png".to_string());
        config
            .output
            .with_file_name(format!("{stem}.{:04}.{ext}", runtime.shot))
    };
    if let Err(error) = image.save(&path) {
        eprintln!(
            "capture_scene: failed to save '{}': {error}",
            path.display()
        );
        runtime.failed = true;
        runtime.completed = true;
        commands.write_message(AppExit::from_code(1));
        return;
    }
    runtime.shot += 1;
    if runtime.shot < config.frames {
        // Re-arm, do not complete. `request_capture` is guarded on
        // `requested || completed`, so clear both. The stride makes the scene
        // different by the next shot.
        runtime.requested = false;
        runtime.wait_frames = 0;
        // A `--press-during` sequence counts its stride in press frames
        // (`press_during_shutter_frame`), so it skips `stride_left`. That
        // countdown returns before the press driver and would freeze the
        // input mid-delivery.
        runtime.stride_left = if config.press_during.is_some() {
            0
        } else {
            config.stride
        };
        return;
    }
    runtime.completed = true;
}

/// The budget is the readiness policies' own allowance plus the readback's
/// slack. A flat frame limit would fire before the route-specific
/// diagnostic, and a `--warmup` above it could never complete.
fn fail_after_timeout(
    mut commands: Commands,
    runtime: Res<SceneCaptureRuntime>,
    config: Res<SceneCaptureConfig>,
) {
    if runtime.completed {
        return;
    }
    // Whatever the readiness policies may legitimately still be waiting for,
    // plus the same slack the readback itself gets.
    let budget = config.warmup_frames.max(1) + ROUTE_CAMERA_GRACE_FRAMES;
    if runtime.frames > budget || runtime.wait_frames > 600 {
        eprintln!(
            "capture_scene: timed out waiting for texture readback after {} frames \
             (warmup {} + grace {})",
            runtime.frames, config.warmup_frames, ROUTE_CAMERA_GRACE_FRAMES
        );
        commands.write_message(AppExit::from_code(1));
    }
}

/// A refused route never gives the capture its world, and the wait for the
/// world counts no frame toward [`fail_after_timeout`]. So the shell's own
/// reason ends the capture (measured 2026-10-09: `--character alice`, an id
/// the catalog does not have, waited out six timeouts of five minutes).
///
/// Only before the world is ready the first time: a press sequence can ask
/// for a route the shell refuses, and that is the subject of its photograph.
fn fail_when_the_shell_refuses_the_route(
    mut commands: Commands,
    runtime: Res<SceneCaptureRuntime>,
    failures: Option<Res<ambition_platformer2d::game_shell::ShellFailureLog>>,
) {
    if runtime.ever_ready || runtime.completed {
        return;
    }
    let Some(reason) = failures.as_ref().and_then(|failures| failures.latest()) else {
        return;
    };
    eprintln!(
        "capture_scene: the shell refused a route before the world was ready, so there is nothing to \
         photograph. No image is written. The shell's reason: {reason}"
    );
    commands.write_message(AppExit::from_code(2));
}

fn parse_vec2(text: &str) -> Option<ae::Vec2> {
    let (x, y) = text.split_once(',')?;
    Some(ae::Vec2::new(
        x.trim().parse().ok()?,
        y.trim().parse().ok()?,
    ))
}

/// Does this argument name an image file the encoder can actually write?
///
/// Checked at parse time.
fn looks_like_image_path(value: &str) -> bool {
    std::path::Path::new(value)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "bmp" | "tga" | "webp"
            )
        })
}

fn parse_image_size(text: &str) -> Option<UVec2> {
    let (w, h) = text.split_once('x').or_else(|| text.split_once('X'))?;
    Some(UVec2::new(w.trim().parse().ok()?, h.trim().parse().ok()?))
}

#[cfg(test)]
mod press_during_tests {
    use super::*;

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_string).collect()
    }

    fn room_args(extra: &str) -> Vec<String> {
        args(&format!("blink_run 0,0 /tmp/t.png {extra}"))
    }

    #[test]
    fn the_flag_names_a_frame_of_the_press_and_both_spellings_agree() {
        let spaced = SceneCaptureConfig::from_args(room_args("--press Enter --press-during 3"))
            .expect("spaced form parses");
        let joined = SceneCaptureConfig::from_args(room_args("--press=Enter --press-during=3"))
            .expect("joined form parses");
        assert_eq!(spaced.press_during, Some(3));
        assert_eq!(joined.press_during, Some(3));
    }

    #[test]
    fn a_capture_that_did_not_ask_for_the_flag_does_not_get_it() {
        // `press_during_shutter_frame` returning `None` keeps every existing
        // capture unchanged, so assert it directly.
        let config = SceneCaptureConfig::from_args(room_args("--press Down,Enter"))
            .expect("a plain press capture parses");
        assert_eq!(config.press_during, None);
        for shot in 0..4 {
            for stride in 1..4 {
                assert_eq!(
                    press_during_shutter_frame(config.press_during, shot, stride),
                    None,
                    "a capture without --press-during must never open the shutter mid-sequence"
                );
            }
        }
    }

    #[test]
    fn the_shutter_opens_the_frame_after_the_one_the_caller_named() {
        // `--press-during 1`: one press frame has happened, so the shot is on
        // the next one, where a tap's key is still held.
        let config = SceneCaptureConfig::from_args(room_args("--press Enter --press-during 1"))
            .expect("parses");
        let mut runtime = SceneCaptureRuntime::default();
        runtime.press_frames = 1;
        assert!(
            !press_during_shutter_is_open(&config, &runtime),
            "the frame the step was delivered on has not been rendered with it yet"
        );
        runtime.press_frames = 2;
        assert!(press_during_shutter_is_open(&config, &runtime));
    }

    #[test]
    fn a_sequence_spaces_its_shots_by_the_stride_in_press_frames() {
        // The ordinary `stride_left` countdown returns before the press
        // driver, so these are counted in press-driving frames.
        let config = SceneCaptureConfig::from_args(room_args(
            "--press hold:up --press-during 2 --frames 3 --stride 5",
        ))
        .expect("parses");
        let at = |shot| press_during_shutter_frame(config.press_during, shot, config.stride);
        assert_eq!(at(0), Some(2));
        assert_eq!(at(1), Some(7));
        assert_eq!(at(2), Some(12));
    }

    #[test]
    fn a_frame_count_the_press_cannot_reach_is_refused_not_rounded() {
        // A sequence too short to contain the named frame must not return the
        // ordinary post-press capture.
        let config = SceneCaptureConfig::from_args(room_args("--press Enter --press-during 9"))
            .expect("parses");
        let mut runtime = SceneCaptureRuntime::default();
        // Enter is two frames: press and release. "Spent" is the cursor, not
        // the frame count.
        runtime.press_frames = 2;
        runtime.press_cursor = config.press.len();
        assert!(!press_during_shutter_is_open(&config, &runtime));
        complete_press_sequence_if_spent(&config, &mut runtime);
        assert!(
            runtime.press_during_missed,
            "a spent sequence must report the frame it never reached"
        );
        assert!(
            runtime.press_done_frame.is_none(),
            "and must NOT fall through into the ordinary post-press shutter"
        );
    }

    #[test]
    fn a_press_that_reaches_the_frame_never_reports_a_miss() {
        // The other path: a sequence still running when the shutter opens
        // reports nothing, so the guard above is not trivially always firing.
        let config =
            SceneCaptureConfig::from_args(room_args("--press hold:up,wait:30 --press-during 2"))
                .expect("parses");
        let mut runtime = SceneCaptureRuntime::default();
        runtime.press_cursor = 1;
        runtime.press_wait = 20;
        complete_press_sequence_if_spent(&config, &mut runtime);
        assert!(!runtime.press_during_missed);
    }

    #[test]
    fn the_flag_is_refused_where_it_would_read_as_honoured_and_do_nothing() {
        let no_press = SceneCaptureConfig::from_args(room_args("--press-during 2"))
            .expect_err("--press-during without --press is refused");
        assert!(no_press.contains("no --press was given"), "{no_press}");

        let zero = SceneCaptureConfig::from_args(room_args("--press Enter --press-during 0"))
            .expect_err("frame 0 is refused");
        assert!(zero.contains("first frame of the input is 1"), "{zero}");

        let words = SceneCaptureConfig::from_args(room_args("--press Enter --press-during soon"))
            .expect_err("a non-integer is refused");
        assert!(words.contains("wants a frame count"), "{words}");
    }
}
