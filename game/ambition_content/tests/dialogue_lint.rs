//! Static arity lint for Yarn dialogue commands (moved to the content
//! crate with the yarn payload — R3.2; the lint guards authored CONTENT).
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

/// Fixed-arity Yarn commands and their expected argument counts. MUST match the
/// `In<...>` tuple arities of the generic commands in `ambition_dialog` and
/// the game commands in `dialog/yarn_bindings.rs` (both are `ui`-gated, so
/// this table is duplicated here to remain runtime-independent): no `In` ⇒ 0, `In<T>` ⇒ 1, `In<(A, B)>` ⇒ 2.
const FIXED_ARITY_COMMANDS: &[(&str, usize)] = &[
    ("present_speaker", 1),
    ("portrait_clip", 1),
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
    ("challenge", 0),
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

    /// One scanned `[...]` span and whether it is a well-formed Yarn markup tag.
    struct MarkupSpan {
        text: String,
        well_formed: bool,
    }

    /// Scan a line for `[...]` markup spans and classify each. Mirrors the
    /// open/self-close grammar in `yarnspinner_runtime::markup::line_parser`:
    /// inside `[name ...]`, after the tag name every whitespace-separated token
    /// must be a `key=value` property (or the span ends in `]` / `/]`). A bare
    /// word — the `[MULTIPLE VOICES]` stage-direction mistake — makes the
    /// runtime parser panic with "Expected a = inside markup" the moment that
    /// line is *delivered* (which the compile guard cannot see, since markup is
    /// parsed lazily, not at compile time).
    fn scan_markup_spans(line: &str) -> Vec<MarkupSpan> {
        let bytes = line.as_bytes();
        let mut spans = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'[' && (i == 0 || bytes[i - 1] != b'\\') {
                // Find the closing `]` (markup tags do not nest a literal `]`).
                if let Some(rel) = line[i + 1..].find(']') {
                    let inner = &line[i + 1..i + 1 + rel];
                    spans.push(MarkupSpan {
                        text: line[i..i + 1 + rel + 1].to_string(),
                        well_formed: markup_inner_well_formed(inner),
                    });
                    i += 1 + rel + 1;
                    continue;
                }
            }
            // Advance by one full char to stay UTF-8 safe.
            i += line[i..].chars().next().map_or(1, char::len_utf8);
        }
        spans
    }

    /// True if the content between `[` and `]` is a well-formed marker.
    fn markup_inner_well_formed(inner: &str) -> bool {
        // `[/]` close-all, or `[/name]` close tag (no properties allowed).
        if inner == "/" {
            return true;
        }
        if let Some(name) = inner.strip_prefix('/') {
            return !name.is_empty() && !name.contains(char::is_whitespace);
        }
        // Open / self-closing tag: strip a trailing self-close slash.
        let body = inner.strip_suffix('/').unwrap_or(inner).trim_end();
        let mut tokens = body.split_whitespace();
        // First token is the tag name (optionally `name=value`); subsequent
        // tokens must each be a `key=value` property.
        if tokens.next().is_none() {
            return false; // `[]` is not a valid marker
        }
        tokens.all(|t| t.contains('='))
    }

    #[test]
    fn markup_well_formed_classifier_matches_yarn_grammar() {
        // Real markup the codebase uses — must pass.
        for ok in [
            "shout",
            "/shout",
            "b",
            "/b",
            "/",
            "wave speed=10",
            "select 1=a 2=b",
            "x/",
        ] {
            assert!(
                markup_inner_well_formed(ok),
                "`[{ok}]` should be well-formed"
            );
        }
        // The reported crash + relatives — bare words without `=`.
        for bad in ["MULTIPLE VOICES", "STAGE DIRECTION", "a b c", ""] {
            assert!(
                !markup_inner_well_formed(bad),
                "`[{bad}]` should be flagged (would panic at line delivery)"
            );
        }
        // End-to-end: the scanner pulls the bad span out of a speaker line.
        let spans = scan_markup_spans("Agent Swarm: [MULTIPLE VOICES] hello [shout]hi[/shout]");
        assert_eq!(spans.len(), 3);
        assert!(!spans[0].well_formed, "[MULTIPLE VOICES] is malformed");
        assert!(
            spans[1].well_formed && spans[2].well_formed,
            "[shout]/[/shout] ok"
        );
        // Escaped brackets are literal text, not markup.
        assert!(scan_markup_spans(r"a \[literal] b").is_empty());
    }

    #[test]
    fn no_malformed_yarn_markup_tags() {
        let mut files = Vec::new();
        yarn_files(&dialogue_root(), &mut files);
        assert!(!files.is_empty(), "found no .yarn files");

        let mut violations = Vec::new();
        let mut well_formed_seen = 0usize;
        for file in &files {
            let text = std::fs::read_to_string(file).expect("read yarn file");
            let label = file
                .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                .unwrap_or(file)
                .to_string_lossy()
                .into_owned();
            for (n, line) in text.lines().enumerate() {
                // Skip structural lines (no displayed markup): headers, the
                // node delimiters, and `//` comments.
                let trimmed = line.trim_start();
                if trimmed.starts_with("title:")
                    || trimmed == "---"
                    || trimmed == "==="
                    || trimmed.starts_with("//")
                {
                    continue;
                }
                for span in scan_markup_spans(line) {
                    if span.well_formed {
                        well_formed_seen += 1;
                    } else {
                        violations.push(format!(
                            "{label}:{}: malformed Yarn markup tag `{}` — a bracketed token \
                             without `=` makes the runtime panic (\"Expected a = inside markup\") \
                             when this line is shown. Use `(parens)` for stage directions, escape \
                             as `\\[...\\]`, or write a real `[tag]...[/tag]`.",
                            n + 1,
                            span.text,
                        ));
                    }
                }
            }
        }

        assert!(
            violations.is_empty(),
            "Yarn markup violations (each crashes the running game at line delivery):\n{}",
            violations.join("\n")
        );
        // Guard the lint itself: we author real `[shout]`/`[whisper]`/`[b]`
        // markup, so the scanner must keep finding well-formed spans.
        assert!(
            well_formed_seen >= 2,
            "only {well_formed_seen} well-formed markup spans found — the scanner may have \
             stopped matching `[...]`; verify scan_markup_spans"
        );
    }
}
