//! Static arity lint for Yarn dialogue commands (moved to the content
//! crate with the yarn payload — R3.2; the lint guards authored CONTENT).
//!
//! Yarn only compiles + runs under the `ui` feature, so a `<<command>>` call
//! with the wrong argument count crashes the *running game*, not any test —
//! exactly the `<<give_item "sealednote">>` panic ("Passed too few arguments to
//! YarnFn") that shipped and crashed on taking Alice's note (fixed `9c52e787`).

/// MUST match the `In<...>` tuple arities of the generic commands in `ambition_dialog` and the
/// game commands in `yarn_vocabulary.rs` (both are `ui`-gated, so this table is duplicated
/// here to remain runtime-independent): no `In`  0, `In<T>`  1, `In<(A, B)>`  2.
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

/// Yarn built-ins (`if`/`set`/`jump`/…) and inline functions (`can_afford(…)`, called inside
/// `<<if …>>`) are naturally skipped — they aren't in the table.
fn extract_command_calls(file: &str, text: &str) -> Vec<CommandCall> {
    // ⭐ ONE DEFINITION OF AN EXECUTABLE REGION. This function used to carry its
    // own `<<`/`>>` walk — the only one in the repo that was RIGHT, while three
    // other instruments scanned whole files and grew private prose heuristics.
    // It now calls the library so there is nothing left to drift from.
    ambition_content::dialogue::yarn::executable_regions(text)
        .into_iter()
        .filter_map(|(line, inner)| {
            let mut parts = inner.splitn(2, char::is_whitespace);
            let name = parts.next().unwrap_or("").trim();
            let args = parts.next().unwrap_or("");
            expected_arity(name).map(|_| CommandCall {
                file: file.to_string(),
                line,
                name: name.to_string(),
                arg_count: count_args(args),
            })
        })
        .collect()
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

    /// The three numbers one shop line writes, and where each was found.
    struct ShopLine {
        file: String,
        line: usize,
        label: u32,
        guard: u32,
        item: String,
        charged: u32,
    }

    /// Pull `-> Buy Axe — 25g <<if can_afford(25)>>` and the `<<buy_item "axe" 25>>`
    /// that follows it.
    ///
    /// ⚠ The `buy_item` is looked for on the NEXT FEW lines rather than the same
    /// one, because Yarn puts an option's body under it, indented.
    fn shop_lines(file: &str, text: &str) -> Vec<ShopLine> {
        let lines: Vec<&str> = text.split('\n').collect();
        let mut out = Vec::new();
        for (index, raw) in lines.iter().enumerate() {
            let Some(after) = raw.split_once("can_afford(") else {
                continue;
            };
            let Some(guard) = after.1.split(')').next().and_then(|n| n.trim().parse().ok()) else {
                continue;
            };
            // The price the PLAYER reads, written as `25g` in the option text.
            let Some(label) = after
                .0
                .split_whitespace()
                .filter_map(|word| word.trim_end_matches('g').parse::<u32>().ok())
                .next_back()
            else {
                continue;
            };
            for follow in lines.iter().take(index + 4).skip(index + 1) {
                let Some(rest) = follow.split_once("<<buy_item ") else {
                    continue;
                };
                let mut args = rest.1.trim_end_matches(">>").trim().splitn(2, '"').nth(1);
                let Some(tail) = args.take() else { continue };
                let Some((item, price)) = tail.split_once('"') else {
                    continue;
                };
                let Some(charged) = price.trim().trim_end_matches(">>").trim().parse().ok() else {
                    continue;
                };
                out.push(ShopLine {
                    file: file.to_string(),
                    line: index + 1,
                    label,
                    guard,
                    item: item.to_string(),
                    charged,
                });
                break;
            }
        }
        out
    }

    /// ⛔⛔ A SHOP LINE STATES ITS PRICE THREE TIMES AND NOTHING CHECKED THAT THEY
    /// AGREE.
    ///
    /// `-> Buy Axe — 25g <<if can_afford(25)>>` / `<<buy_item "axe" 25>>` writes
    /// one fact in three places: the number the PLAYER READS, the number the
    /// menu GREYS OUT ON, and the number the wallet is CHARGED. An author
    /// changing a price edits one line and the other two go quietly wrong, each
    /// in a different way:
    ///
    /// - label ≠ charged — the player is told one price and billed another;
    /// - guard > charged — an affordable item looks unaffordable and cannot be
    ///   bought at all;
    /// - guard < charged — the option is offered, the player picks it, and
    ///   `shop::buy` refuses for lack of funds. **The menu entry does nothing
    ///   and says nothing**, which is the worst of the three because it reads
    ///   as a broken game rather than a wrong number.
    ///
    /// ⭐ THIS IS THE CODE-SIDE DEFECT ONE LAYER OUT. `wallet.can_afford` and
    /// `cmd_buy_item` read the authored price two different ways until
    /// 2026-09-04, when `ambition_items::shop::authored_price` became the single
    /// reading. That fixed the two CONSUMERS; this checks the three
    /// STATEMENTS. One fact, three writers, is the same shape wherever it sits.
    ///
    /// ⚠ NOT a style rule. It asserts nothing about how a shop line is phrased —
    /// only that the numbers a line already wrote say the same thing.
    #[test]
    fn a_shop_lines_three_prices_agree() {
        let mut files = Vec::new();
        yarn_files(&dialogue_root(), &mut files);
        let mut found = Vec::new();
        for path in &files {
            let text = std::fs::read_to_string(path).expect("authored dialogue is readable");
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("<unnamed>");
            found.extend(shop_lines(name, &text));
        }

        // ⛔ A FLOOR, because this walks a directory and parses by shape: if the
        // menu is rewritten or the option grammar changes, an empty walk would
        // pass every assertion below it.
        assert!(
            found.len() >= 10,
            "only {} shop line(s) parsed across {} authored .yarn file(s) — the \
             option grammar this reads has changed, and an empty walk cannot fail",
            found.len(),
            files.len()
        );

        let disagreeing: Vec<String> = found
            .iter()
            .filter(|line| !(line.label == line.guard && line.guard == line.charged))
            .map(|line| {
                format!(
                    "{}:{} `{}` — the player reads {}g, the menu gates on {}, the wallet is charged {}",
                    line.file, line.line, line.item, line.label, line.guard, line.charged
                )
            })
            .collect();
        assert!(
            disagreeing.is_empty(),
            "a shop line's three statements of one price disagree:\n  {}",
            disagreeing.join("\n  ")
        );
    }

    /// ⛔⛔ A SELL LINE'S GUARD AND ITS SALE MUST NAME THE SAME ITEM.
    ///
    /// `-> Sell Axe — 12g <<if condition("inventory.holds", "axe")>>` /
    /// `<<sell_item "axe" 12>>` states the ITEM twice and the PRICE twice. The
    /// item pair is the sharper of the two: if the guard and the sale disagree,
    /// the option appears when the player holds one thing and sells another —
    /// or appears and does nothing, because `shop::sell` refuses an item that is
    /// not owned. Neither failure says anything to the player.
    ///
    /// ⚠ THE PRICE CHECK IS CONDITIONAL AND THE ITEM CHECK IS NOT, which is a
    /// fact about the content rather than caution. `intro.yarn`'s
    /// `<<sell_item "sealednote" 0>>` is a hand-over beat, not a sale: it has no
    /// price in its label because there is no price. ⇒ Requiring a label price
    /// everywhere would fail a line that is correct, so the price pair is
    /// checked only where the author wrote one.
    ///
    /// ⭐ Measured before the rule was written: four guarded sell lines, all
    /// four agreeing on the item, and three of them stating a price that
    /// matches. It lands green as a ratchet rather than a repair.
    #[test]
    fn a_sell_lines_guard_and_its_sale_name_the_same_item() {
        let mut files = Vec::new();
        yarn_files(&dialogue_root(), &mut files);

        let mut checked = 0usize;
        let mut wrong: Vec<String> = Vec::new();
        for path in &files {
            let text = std::fs::read_to_string(path).expect("authored dialogue is readable");
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("<unnamed>")
                .to_string();
            let lines: Vec<&str> = text.split('\n').collect();
            for (index, raw) in lines.iter().enumerate() {
                let Some(after) = raw.split_once("inventory.holds\"") else {
                    continue;
                };
                let Some(guarded) = after.1.split('"').nth(1) else {
                    continue;
                };
                let label = after
                    .0
                    .split_whitespace()
                    .filter_map(|word| word.trim_end_matches('g').parse::<u32>().ok())
                    .next_back();
                for follow in lines.iter().take(index + 4).skip(index + 1) {
                    let Some(rest) = follow.split_once("<<sell_item ") else {
                        continue;
                    };
                    let Some(tail) = rest.1.splitn(2, '"').nth(1) else {
                        continue;
                    };
                    let Some((sold, price)) = tail.split_once('"') else {
                        continue;
                    };
                    checked += 1;
                    if sold != guarded {
                        wrong.push(format!(
                            "{name}:{}: the option appears when the player holds `{guarded}` \
                             and then sells `{sold}`",
                            index + 1
                        ));
                    }
                    if let (Some(shown), Ok(paid)) = (
                        label,
                        price.trim().trim_end_matches(">>").trim().parse::<u32>(),
                    ) {
                        if shown != paid {
                            wrong.push(format!(
                                "{name}:{}: the option offers {shown}g for `{sold}` and pays {paid}",
                                index + 1
                            ));
                        }
                    }
                    break;
                }
            }
        }

        // ⛔ A FLOOR: this parses by shape, so a rewritten option grammar would
        // leave it walking nothing and passing.
        assert!(
            checked >= 3,
            "only {checked} guarded sell line(s) parsed across {} authored .yarn \
             file(s) — the option grammar this reads has changed",
            files.len()
        );
        assert!(
            wrong.is_empty(),
            "a sell line's guard and its sale disagree:\n  {}",
            wrong.join("\n  ")
        );
    }

    /// ⛔⛔ EVERY AUTHORED ITEM ID MUST RESOLVE, and the resolver is asked
    /// rather than re-implemented.
    ///
    /// `<<give_item "sealednote" 1>>`, `<<buy_item "axe" 25>>` and
    /// `condition("inventory.holds", "gunsword")` all pass an author-typed
    /// string. `Item::from_dialog_id` is the single owner of loose spelling —
    /// it accepts `HealthPotion`, `health_potion` and `healthcell` alike — so a
    /// misspelling is not a compile error and not a Yarn error. The command
    /// `warn!`s and returns; the condition answers `Unanswerable`, which
    /// collapses to false. ⇒ Either way the line silently does nothing.
    ///
    /// ⭐ THIS CALLS `Item::from_dialog_id` INSTEAD OF LISTING THE ITEMS. A
    /// hand-kept table of valid spellings would be a second authority on
    /// normalisation and would drift the first time an alias is added — the
    /// exact defect `normalize_item_id` was deleted for, which was a second copy
    /// of that logic that agreed until it did not.
    #[test]
    fn every_authored_item_id_resolves_to_a_real_item() {
        use ambition_items::Item;

        let mut files = Vec::new();
        yarn_files(&dialogue_root(), &mut files);
        let mut asked: Vec<(String, usize, String)> = Vec::new();
        for path in &files {
            let text = std::fs::read_to_string(path).expect("authored dialogue is readable");
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("<unnamed>")
                .to_string();
            for (index, line) in text.split('\n').enumerate() {
                for verb in ["give_item ", "buy_item ", "sell_item "] {
                    if let Some(rest) = line.split_once(verb) {
                        if let Some(id) = rest.1.split('"').nth(1) {
                            asked.push((name.clone(), index + 1, id.to_string()));
                        }
                    }
                }
                if let Some(rest) = line.split_once("inventory.holds\"") {
                    if let Some(id) = rest.1.split('"').nth(1) {
                        asked.push((name.clone(), index + 1, id.to_string()));
                    }
                }
            }
        }

        // ⛔ A FLOOR ABOVE THE LARGEST SINGLE FILE: `kernel.yarn` alone supplies
        // most of these, so a floor of 1 would survive losing every other file.
        assert!(
            asked.len() >= 15,
            "only {} authored item id(s) parsed across {} .yarn file(s) — the \
             command grammar this reads has changed, and an empty walk cannot fail",
            asked.len(),
            files.len()
        );

        let unresolved: Vec<String> = asked
            .iter()
            .filter(|(_, _, id)| Item::from_dialog_id(id).is_none())
            .map(|(file, line, id)| format!("{file}:{line}: `{id}`"))
            .collect();
        assert!(
            unresolved.is_empty(),
            "authored dialogue names items no catalog spelling resolves, so those \
             lines silently do nothing — a command `warn!`s and returns, a \
             condition answers unanswerable and the branch stays shut:\n  {}",
            unresolved.join("\n  ")
        );
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
