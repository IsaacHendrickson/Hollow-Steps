//! Size guard for the two docs every agent reads on every task.
//!
//! Reads only `AGENTS.md` and `FEATURES.md` in the repository root. A `FEATURES.md` line
//! or an `AGENTS.md` module line over 200 characters fails with the file and line number,
//! and so does an `AGENTS.md` over 24 KB, counted as if every line ended with CRLF so the
//! limit holds on every checkout. Longer text belongs in `docs/`.

use std::fs;
use std::path::Path;

const MAX_LINE_CHARS: usize = 200;
const MAX_AGENTS_BYTES: usize = 24 * 1024;
const MODULE_LINE_PREFIX: &str = "- `src/";

fn read_root_file(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("cannot read {name}: {error}"))
}

/// Line number (from 1) and length in characters of every line over `limit` that starts
/// with `prefix` (every line when `prefix` is empty).
fn long_lines(text: &str, prefix: &str, limit: usize) -> Vec<(usize, usize)> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| line.starts_with(prefix))
        .map(|(index, line)| (index + 1, line.chars().count()))
        .filter(|(_, length)| *length > limit)
        .collect()
}

/// Size of `text` in bytes as if every line ended with CRLF: each line's own bytes plus 2.
/// An LF checkout and a CRLF (Windows) checkout of the same file count the same, so the
/// size limit holds on both. For an LF file this is its bytes plus the number of lines.
fn size_with_crlf(text: &str) -> usize {
    text.lines().map(|line| line.len() + 2).sum()
}

fn describe(file: &str, offenders: &[(usize, usize)]) -> String {
    offenders
        .iter()
        .map(|(line, length)| {
            format!("{file} line {line} is {length} characters (limit {MAX_LINE_CHARS})")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn features_lines_stay_short() {
    let offenders = long_lines(&read_root_file("FEATURES.md"), "", MAX_LINE_CHARS);
    assert!(
        offenders.is_empty(),
        "move the detail to docs/features-detail.md:\n{}",
        describe("FEATURES.md", &offenders)
    );
}

#[test]
fn agents_module_lines_stay_short() {
    let offenders = long_lines(
        &read_root_file("AGENTS.md"),
        MODULE_LINE_PREFIX,
        MAX_LINE_CHARS,
    );
    assert!(
        offenders.is_empty(),
        "move the detail to docs/modules.md:\n{}",
        describe("AGENTS.md", &offenders)
    );
}

#[test]
fn agents_md_stays_small() {
    let size = size_with_crlf(&read_root_file("AGENTS.md"));
    assert!(
        size <= MAX_AGENTS_BYTES,
        "AGENTS.md is {size} bytes counted with CRLF line endings (limit {MAX_AGENTS_BYTES}); \
         move detail to docs/"
    );
}

#[test]
fn the_size_counts_every_line_as_ending_with_crlf() {
    // Bytes plus one per line for an LF file: "ab\n" is 3 bytes, 1 line.
    assert_eq!(size_with_crlf("ab\ncd\n"), "ab\ncd\n".len() + 2);
    // The same text with CRLF endings already has those bytes, and counts the same.
    assert_eq!(size_with_crlf("ab\r\ncd\r\n"), "ab\r\ncd\r\n".len());
    assert_eq!(size_with_crlf("ab\ncd\n"), size_with_crlf("ab\r\ncd\r\n"));
}

#[test]
fn the_size_counts_blank_lines_a_missing_final_break_and_bytes() {
    assert_eq!(size_with_crlf(""), 0);
    assert_eq!(size_with_crlf("\n\n"), 4);
    // A last line with no line break still counts the CRLF it would have.
    assert_eq!(size_with_crlf("ab\ncd"), size_with_crlf("ab\ncd\n"));
    // Bytes, not characters: "é" is two bytes.
    assert_eq!(size_with_crlf("é\n"), 4);
}

#[test]
fn the_size_limit_is_exact_on_either_line_ending() {
    let exact = "x".repeat(MAX_AGENTS_BYTES - 2);
    for ending in ["\n", "\r\n"] {
        assert_eq!(
            size_with_crlf(&format!("{exact}{ending}")),
            MAX_AGENTS_BYTES
        );
        assert_eq!(
            size_with_crlf(&format!("{exact}x{ending}")),
            MAX_AGENTS_BYTES + 1
        );
    }
}

#[test]
fn the_checker_names_the_line_and_looks_only_at_the_chosen_lines() {
    let long = "x".repeat(MAX_LINE_CHARS + 1);
    let exact = "x".repeat(MAX_LINE_CHARS);
    let text = format!("- `src/a.rs`: fine\n{exact}\n- `src/b.rs`: {long}\nplain {long}\n");

    let module_lines = long_lines(&text, MODULE_LINE_PREFIX, MAX_LINE_CHARS);
    assert_eq!(
        module_lines,
        vec![(3, "- `src/b.rs`: ".len() + MAX_LINE_CHARS + 1)]
    );

    let every_line: Vec<usize> = long_lines(&text, "", MAX_LINE_CHARS)
        .iter()
        .map(|(line, _)| *line)
        .collect();
    assert_eq!(every_line, vec![3, 4]);
}

#[test]
fn a_line_of_exactly_the_limit_passes_and_one_more_character_fails() {
    let exact = "y".repeat(MAX_LINE_CHARS);
    assert!(long_lines(&exact, "", MAX_LINE_CHARS).is_empty());
    assert_eq!(
        long_lines(&format!("{exact}y"), "", MAX_LINE_CHARS),
        vec![(1, MAX_LINE_CHARS + 1)]
    );
}

#[test]
fn length_counts_characters_not_bytes() {
    let wide = "é".repeat(MAX_LINE_CHARS);
    assert!(wide.len() > MAX_LINE_CHARS);
    assert!(long_lines(&wide, "", MAX_LINE_CHARS).is_empty());
}

#[test]
fn windows_line_endings_do_not_count_toward_the_length() {
    let exact = "z".repeat(MAX_LINE_CHARS);
    assert!(long_lines(&format!("{exact}\r\nnext\r\n"), "", MAX_LINE_CHARS).is_empty());
}

#[test]
fn the_error_message_names_file_line_and_length() {
    let message = describe("FEATURES.md", &[(12, 213)]);
    assert!(message.contains("FEATURES.md line 12 is 213 characters"));
}
