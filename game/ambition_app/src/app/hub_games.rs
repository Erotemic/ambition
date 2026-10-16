//! Which games this hub installs.
//!
//! The hub is the shell, the launcher and the games it is given. Each game
//! installs through its provider, and the launcher lists the games that
//! registered, so a hub of one game is the same program with a shorter list.
//! `AMBITION_HUB_GAMES` names them (see [`HUB_GAMES_ENV`]); unset means all.

/// The env var that names the games this hub installs, comma-separated:
/// `ambition`, `sanic`, `mary_o`, `smash`, `versus`. Unset means all of them.
///
/// An APK has no environment, so the value at build time is the fallback:
/// `AMBITION_HUB_GAMES=mary_o ./build_for_android.sh ...`.
pub const HUB_GAMES_ENV: &str = "AMBITION_HUB_GAMES";

/// Whether this hub installs `game`.
pub fn installs(game: &str) -> bool {
    selection().is_none_or(|games| games.iter().any(|g| g == game))
}

/// The named games, or `None` when the hub installs all of them.
pub fn selection() -> Option<&'static [String]> {
    static SELECTION: std::sync::OnceLock<Option<Vec<String>>> = std::sync::OnceLock::new();
    SELECTION
        .get_or_init(|| {
            let list = std::env::var(HUB_GAMES_ENV)
                .ok()
                .or_else(|| option_env!("AMBITION_HUB_GAMES").map(str::to_owned))?;
            Some(
                list.split(',')
                    .map(|game| game.trim().to_owned())
                    .filter(|game| !game.is_empty())
                    .collect(),
            )
        })
        .as_deref()
}

/// The env var that names the route the hub boots into in place of the
/// launcher, for example `mary_o_gameplay`. Unset means the launcher.
///
/// It is a measurement and test input: a profile of the hub with one game
/// live starts in that game, so the recording window cannot miss it. The
/// hub still installs every game it is given. As for [`HUB_GAMES_ENV`], an
/// APK reads the value at build time.
pub const BOOT_ROUTE_ENV: &str = "AMBITION_BOOT_ROUTE";

/// The route [`BOOT_ROUTE_ENV`] names, if any.
pub fn boot_route() -> Option<String> {
    std::env::var(BOOT_ROUTE_ENV)
        .ok()
        .or_else(|| option_env!("AMBITION_BOOT_ROUTE").map(str::to_owned))
        .map(|route| route.trim().to_owned())
        .filter(|route| !route.is_empty())
}
