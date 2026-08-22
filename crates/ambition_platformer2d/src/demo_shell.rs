//! The launcher conventions a demo's own standalone shell speaks.
//!
//! Every demo ships a binary that is headless by default and windowed under
//! `--features visible`, and `run_game.sh` drives all of them the same way:
//! `--window` to draw, `--ticks N` to bound a sim-only run. The conventions are
//! the LAUNCHER's, so they live once rather than in each shell.

/// How many ticks a sim-only shell should step, from `--ticks N`, falling
/// back to `default` when the caller did not say.
///
/// this was THREE byte-identical copies and a fourth shell that did not
/// have it at all. `sanic`, `mary-o` and `twintrack` each carried the same
/// six-line `parse_ticks`; the smash shell hardcoded 600 and ignored the flag,
/// so `./run_game.sh smash --headless -- --ticks 3` stepped six hundred and said
/// so. The help text documents `--ticks` as a demo-shell convention — which made
/// the odd one out a stale doc rather than a missing feature.
pub fn headless_ticks(default: u32) -> u32 {
    ticks_from(std::env::args().skip(1), default)
}

/// The parse itself, over any argument sequence, so it is testable without a
/// process.
fn ticks_from(args: impl IntoIterator<Item = String>, default: u32) -> u32 {
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--ticks" {
            return args.next().and_then(|n| n.parse().ok()).unwrap_or(default);
        }
        if let Some(n) = arg.strip_prefix("--ticks=") {
            return n.parse().unwrap_or(default);
        }
    }
    default
}

/// Whether this shell was asked to draw (`--window`).
///
/// The visible build still decides whether it CAN draw; this is only the ask.
pub fn wants_a_window() -> bool {
    std::env::args().any(|arg| arg == "--window")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(raw: &[&str]) -> Vec<String> {
        raw.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn a_shell_with_no_flag_steps_its_own_default() {
        assert_eq!(ticks_from(args(&[]), 600), 600);
        assert_eq!(ticks_from(args(&["--window"]), 42), 42);
    }

    #[test]
    fn both_spellings_of_the_flag_are_read() {
        assert_eq!(ticks_from(args(&["--ticks", "5"]), 600), 5);
        assert_eq!(ticks_from(args(&["--ticks=5"]), 600), 5);
    }

    /// a garbled count falls back rather than panicking. A launcher flag is
    /// a convenience; a demo that aborts on a typo is worse than one that runs
    /// its default and prints how many ticks it ran.
    #[test]
    fn a_count_that_is_not_a_number_falls_back() {
        assert_eq!(ticks_from(args(&["--ticks", "lots"]), 600), 600);
        assert_eq!(ticks_from(args(&["--ticks"]), 600), 600);
    }
}
