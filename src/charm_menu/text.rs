//! Fitting text into a width. The caller passes the measuring function (Macroquad's
//! `measure_text` in the game, a fixed width per character in tests), so this stays pure.

const ELLIPSIS: &str = "...";

/// `text` as it is when it fits in `max_width`; else its longest start that fits with "..."
/// added, or nothing at all when not even that fits.
pub fn fit_line(text: &str, max_width: f32, measure: &dyn Fn(&str) -> f32) -> String {
    if measure(text) <= max_width {
        return text.to_string();
    }
    let ends: Vec<usize> = text.char_indices().map(|(at, _)| at).collect();
    for &end in ends.iter().rev() {
        let candidate = format!("{}{ELLIPSIS}", text[..end].trim_end());
        if measure(&candidate) <= max_width {
            return candidate;
        }
    }
    String::new()
}

/// `text` broken at spaces into at most `max_lines` lines that each fit in `max_width`. Text
/// that needs more lines ends its last line with "...", and a single word wider than a line is
/// cut the same way.
pub fn wrap_lines(
    text: &str,
    max_width: f32,
    max_lines: usize,
    measure: &dyn Fn(&str) -> f32,
) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut lines = Vec::new();
    let mut next = 0;
    while next < words.len() && lines.len() < max_lines {
        let mut line = String::new();
        while next < words.len() {
            let candidate = if line.is_empty() {
                words[next].to_string()
            } else {
                format!("{line} {}", words[next])
            };
            if measure(&candidate) > max_width {
                break;
            }
            line = candidate;
            next += 1;
        }
        if line.is_empty() {
            // One word wider than the whole line.
            line = fit_line(words[next], max_width, measure);
            next += 1;
        }
        if lines.len() + 1 == max_lines && next < words.len() {
            let all = format!("{line} {}", words[next..].join(" "));
            line = fit_line(&all, max_width, measure);
            next = words.len();
        }
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests;
