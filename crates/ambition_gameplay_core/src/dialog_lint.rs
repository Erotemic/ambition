//! Static arity lint for Yarn dialogue commands (Refactor 9).
//!
//! Yarn only compiles + runs under the `ui` feature, so a `<<command>>` call
//! with the wrong argument count crashes the *running game*, not any test —
//! exactly the `<<give_item "sealednote">>` panic ("Passed too few arguments to
//! YarnFn") that shipped and crashed on taking Alice's note (fixed `9c52e787`).
//!
//! This is a pure-text check (no Yarn runtime — `#[cfg(test)]` only, so it runs
//! in every test configuration including lean/headless ones): every fixed-arity
//! command call in `assets/dialogue/**/*.yarn` must pass the right number of
//! arguments, so the whole class of crash is caught at `cargo test` time.

#![cfg(test)]

/// Fixed-arity Yarn commands and their expected argument counts. MUST match the
/// `In<...>` tuple arities of the `cmd_*` fns in `dialog/yarn_bindings.rs`
/// (which is `ui`-gated, hence the table is duplicated here to stay
/// runtime-independent): no `In` ⇒ 0, `In<T>` ⇒ 1, `In<(A, B)>` ⇒ 2.
const FIXED_ARITY_COMMANDS: &[(&str, usize)] = &[
    ("give_item", 2),
    ("buy_item", 2),
    ("sell_item", 2),
    ("set_flag", 1),
    ("clear_flag", 1),
    ("spawn_chest", 1),
    ("play_sfx", 1),
    ("camera_zoom", 1),
    ("spawn_fireworks", 0),
    ("watch_cut_rope_video", 0),
    ("reset_cut_rope_room", 0),
];

fn expected_arity(name: &str) -> Option<usize> {
    FIXED_ARITY_COMMANDS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, a)| *a)
}

/// Count the arguments in a Yarn command body (everything after the command
/// name), treating a double-quoted span as a single argument so
/// `give_item "a b" 1` counts as 2.
fn count_args(args: &str) -> usize {
    let mut count = 0;
    let mut chars = args.chars().peekable();
    loop {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        match chars.peek() {
            None => break,
            Some('"') => {
                chars.next(); // opening quote
                while let Some(c) = chars.next() {
                    if c == '"' {
                        break;
                    }
                }
                count += 1;
            }
            Some(_) => {
                while chars.peek().is_some_and(|c| !c.is_whitespace()) {
                    chars.next();
                }
                count += 1;
            }
        }
    }
    count
}

/// A single `<<...>>` command call found in a dialogue file.
struct CommandCall {
    file: String,
    line: usize,
    name: String,
    arg_count: usize,
}

/// Extract every `<<command ...>>` call whose first token is a known fixed-arity
/// command. Yarn built-ins (`if`/`set`/`jump`/…) and inline functions
/// (`can_afford(…)`, called inside `<<if …>>`) are naturally skipped — they
/// aren't in the table.
fn extract_command_calls(file: &str, text: &str) -> Vec<CommandCall> {
    let mut calls = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let mut rest = line;
        while let Some(open) = rest.find("<<") {
            let after = &rest[open + 2..];
            let Some(close) = after.find(">>") else {
                break;
            };
            let inner = after[..close].trim();
            let mut parts = inner.splitn(2, char::is_whitespace);
            let name = parts.next().unwrap_or("").trim();
            let args = parts.next().unwrap_or("");
            if expected_arity(name).is_some() {
                calls.push(CommandCall {
                    file: file.to_string(),
                    line: i + 1,
                    name: name.to_string(),
                    arg_count: count_args(args),
                });
            }
            rest = &after[close + 2..];
        }
    }
    calls
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn dialogue_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/dialogue")
    }

    fn yarn_files(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                yarn_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "yarn") {
                out.push(path);
            }
        }
    }

    #[test]
    fn count_args_is_quote_aware() {
        assert_eq!(count_args(""), 0);
        assert_eq!(count_args("\"sealednote\" 1"), 2);
        assert_eq!(count_args("\"sealednote\""), 1);
        assert_eq!(
            count_args("\"a b c\" 1"),
            2,
            "a quoted span with spaces is one arg"
        );
        assert_eq!(count_args("HealthPotion 3"), 2);
        assert_eq!(count_args("   42   "), 1);
    }

    #[test]
    fn every_fixed_arity_command_call_has_the_right_arg_count() {
        let mut files = Vec::new();
        yarn_files(&dialogue_root(), &mut files);
        assert!(
            !files.is_empty(),
            "found no .yarn files under {} — did the dialogue assets move?",
            dialogue_root().display()
        );

        let mut violations = Vec::new();
        let mut checked = 0usize;
        for file in &files {
            let text = std::fs::read_to_string(file).expect("read yarn file");
            let label = file
                .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                .unwrap_or(file)
                .to_string_lossy()
                .into_owned();
            for call in extract_command_calls(&label, &text) {
                checked += 1;
                let expected = expected_arity(&call.name).unwrap();
                if call.arg_count != expected {
                    violations.push(format!(
                        "{}:{}: <<{}>> takes {} arg(s) but was called with {} — this would \
                         panic the running game ('Passed too {} arguments to YarnFn')",
                        call.file,
                        call.line,
                        call.name,
                        expected,
                        call.arg_count,
                        if call.arg_count < expected {
                            "few"
                        } else {
                            "many"
                        },
                    ));
                }
            }
        }

        assert!(
            violations.is_empty(),
            "Yarn command arity violations (each crashes at runtime):\n{}",
            violations.join("\n")
        );
        // Guard the lint itself: if this drops to ~0 the parser silently stopped
        // finding commands (e.g. a `<<>>` syntax change), defeating the check.
        assert!(
            checked >= 5,
            "only {checked} fixed-arity command calls found across {} files — the lint may have \
             stopped matching; verify the <<...>> scanner",
            files.len()
        );
    }
}
