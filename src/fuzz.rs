//! Fuzzy string scoring functions.

use std::collections::BTreeSet;

use crate::matcher::{lcs_len, Pattern};
use crate::utils::full_process;

/// Computes the indel similarity of two strings.
#[must_use]
pub fn ratio(a: &str, b: &str) -> u8 {
    round_score(
        ratio_raw_chars(
            &a.chars().collect::<Vec<_>>(),
            &b.chars().collect::<Vec<_>>(),
        ) * 100.0,
    )
}

/// Finds the best indel similarity between the shorter input and any aligned
/// window of the longer input, skipping windows that cannot beat the best score
/// found so far.
#[must_use]
pub fn partial_ratio(a: &str, b: &str) -> u8 {
    round_score(
        partial_ratio_raw_chars(
            &a.chars().collect::<Vec<_>>(),
            &b.chars().collect::<Vec<_>>(),
        ) * 100.0,
    )
}

/// Sorts tokens before scoring them.
#[must_use]
pub fn token_sort_ratio(a: &str, b: &str, force_ascii: bool, process: bool) -> u8 {
    round_score(token_sort_raw(a, b, force_ascii, process, false) * 100.0)
}

/// Compares token intersections and differences.
#[must_use]
pub fn token_set_ratio(a: &str, b: &str, force_ascii: bool, process: bool) -> u8 {
    round_score(token_set_raw(a, b, force_ascii, process, false) * 100.0)
}

/// Sorts tokens before partial scoring.
#[must_use]
pub fn partial_token_sort_ratio(a: &str, b: &str, force_ascii: bool, process: bool) -> u8 {
    round_score(token_sort_raw(a, b, force_ascii, process, true) * 100.0)
}

/// Compares token intersections and differences with partial scoring.
#[must_use]
pub fn partial_token_set_ratio(a: &str, b: &str, force_ascii: bool, process: bool) -> u8 {
    round_score(token_set_raw(a, b, force_ascii, process, true) * 100.0)
}

/// Combines base, token, and partial similarities.
#[must_use]
pub fn wratio(a: &str, b: &str, force_ascii: bool, process: bool) -> u8 {
    round_score(wratio_raw(a, b, force_ascii, process) * 100.0)
}

/// Processes both inputs and computes their indel similarity.
#[must_use]
pub fn qratio(a: &str, b: &str, force_ascii: bool, process: bool) -> u8 {
    let (left, right) = prepare_pair(a, b, force_ascii, process);
    if left.is_empty() || right.is_empty() {
        return 0;
    }
    round_score(ratio_raw(&left, &right) * 100.0)
}

/// Runs [`wratio`] without deleting code points from U+0080 through U+00FF.
/// Letter and number classification still replaces other characters with spaces.
#[must_use]
pub fn uwratio(a: &str, b: &str) -> u8 {
    wratio(a, b, false, true)
}

/// Runs [`qratio`] without deleting code points from U+0080 through U+00FF.
/// Letter and number classification still replaces other characters with spaces.
#[must_use]
pub fn uqratio(a: &str, b: &str) -> u8 {
    qratio(a, b, false, true)
}

pub(crate) fn ratio_raw(a: &str, b: &str) -> f64 {
    ratio_raw_chars(
        &a.chars().collect::<Vec<_>>(),
        &b.chars().collect::<Vec<_>>(),
    )
}

fn ratio_raw_chars(a: &[char], b: &[char]) -> f64 {
    similarity(a.len(), b.len(), lcs_len(a, b))
}

/// Indel similarity of two sequences from their lengths and the length of
/// their longest common subsequence.
#[allow(clippy::cast_precision_loss)]
fn similarity(a_len: usize, b_len: usize, lcs: usize) -> f64 {
    let total = a_len + b_len;
    if total == 0 {
        1.0
    } else {
        let distance = total - 2 * lcs;
        1.0 - distance as f64 / total as f64
    }
}

pub(crate) fn partial_ratio_raw(a: &str, b: &str) -> f64 {
    partial_ratio_raw_chars(
        &a.chars().collect::<Vec<_>>(),
        &b.chars().collect::<Vec<_>>(),
    )
}

fn partial_ratio_raw_chars(a: &[char], b: &[char]) -> f64 {
    let (shorter, longer) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    if shorter.is_empty() {
        return if longer.is_empty() { 1.0 } else { 0.0 };
    }

    let score = best_window(shorter, longer);
    if a.len() == b.len() && score < 1.0 {
        score.max(best_window(longer, shorter))
    } else {
        score
    }
}

/// Best similarity of `needle` against the windows of `haystack`: every
/// full-width window, every prefix shorter than the needle, and every suffix
/// up to the needle's length. Requires `0 < needle.len() <= haystack.len()`.
#[allow(clippy::float_cmp)]
fn best_window(needle: &[char], haystack: &[char]) -> f64 {
    let width = needle.len();
    let pattern = Pattern::new(needle);
    let mut best = pruned_full_windows(&pattern, haystack);
    if best == 1.0 {
        return best;
    }

    // One pass over the haystack scores every prefix window.
    pattern.prefix_lcs(haystack[..width - 1].iter().copied(), |len, lcs| {
        best = best.max(similarity(width, len, lcs));
    });
    // Reversing both sides turns each suffix window into a prefix window.
    let reversed: Vec<char> = needle.iter().rev().copied().collect();
    Pattern::new(&reversed).prefix_lcs(haystack.iter().rev().take(width).copied(), |len, lcs| {
        best = best.max(similarity(width, len, lcs));
    });
    best
}

/// Best similarity over the full-width windows that start before the last one.
///
/// Neighboring windows differ in one character at each end, so their indel
/// distances differ by at most 2, and every distance between equal-length
/// sequences is even. Those two facts bound the smallest distance inside a
/// span from its endpoint distances. Spans whose bound cannot beat the best
/// distance found so far are skipped, and the rest are split in half. The
/// final window is scored with the suffix windows.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]
fn pruned_full_windows(pattern: &Pattern, haystack: &[char]) -> f64 {
    let width = pattern.len();
    let last_start = haystack.len() - width;
    if last_start == 0 {
        return 0.0;
    }

    let maximum = 2 * width;
    let distance =
        |start: usize| maximum - 2 * pattern.lcs(haystack[start..start + width].iter().copied());
    let mut cutoff = maximum;
    let mut best_distance = None;
    let mut scores = vec![usize::MAX; last_start];
    let mut spans = vec![(0, last_start - 1)];
    let mut next_spans = Vec::new();

    while !spans.is_empty() {
        for &(first, second) in &spans {
            for start in [first, second] {
                if scores[start] == usize::MAX {
                    let found = distance(start);
                    scores[start] = found;
                    if found < cutoff {
                        cutoff = found;
                        best_distance = Some(found);
                        if found == 0 {
                            return 1.0;
                        }
                    }
                }
            }

            let cells = second - first;
            if cells == 1 {
                continue;
            }
            let known_edits = scores[first].abs_diff(scores[second]);
            let max_improvement = (cells - known_edits / 2) / 2 * 2;
            let lowest = scores[first].min(scores[second]) as isize - max_improvement as isize;
            if lowest < cutoff as isize {
                let center = cells / 2;
                next_spans.push((first, first + center));
                next_spans.push((first + center, second));
            }
        }
        std::mem::swap(&mut spans, &mut next_spans);
        next_spans.clear();
    }

    best_distance.map_or(0.0, |found| 1.0 - found as f64 / maximum as f64)
}

fn prepare_pair(a: &str, b: &str, force_ascii: bool, process: bool) -> (String, String) {
    if process {
        (full_process(a, force_ascii), full_process(b, force_ascii))
    } else {
        (a.to_owned(), b.to_owned())
    }
}

fn sorted_tokens(value: &str) -> String {
    let mut tokens: Vec<&str> = value.split_whitespace().collect();
    tokens.sort_unstable();
    tokens.join(" ")
}

pub(crate) fn token_sort_raw(
    a: &str,
    b: &str,
    force_ascii: bool,
    process: bool,
    partial: bool,
) -> f64 {
    let (left, right) = prepare_pair(a, b, force_ascii, process);
    let left = sorted_tokens(&left);
    let right = sorted_tokens(&right);
    if partial {
        partial_ratio_raw(&left, &right)
    } else {
        ratio_raw(&left, &right)
    }
}

pub(crate) fn token_set_raw(
    a: &str,
    b: &str,
    force_ascii: bool,
    process: bool,
    partial: bool,
) -> f64 {
    let (left, right) = prepare_pair(a, b, force_ascii, process);
    let left: BTreeSet<&str> = left.split_whitespace().collect();
    let right: BTreeSet<&str> = right.split_whitespace().collect();
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let intersection: Vec<&str> = left.intersection(&right).copied().collect();
    let left_only: Vec<&str> = left.difference(&right).copied().collect();
    let right_only: Vec<&str> = right.difference(&left).copied().collect();
    let common = intersection.join(" ");
    let left_joined = join_parts(&common, &left_only.join(" "));
    let right_joined = join_parts(&common, &right_only.join(" "));
    let score = if partial {
        partial_ratio_raw
    } else {
        ratio_raw
    };

    score(&common, &left_joined)
        .max(score(&common, &right_joined))
        .max(score(&left_joined, &right_joined))
}

fn join_parts(common: &str, remainder: &str) -> String {
    match (common.is_empty(), remainder.is_empty()) {
        (true, _) => remainder.to_owned(),
        (_, true) => common.to_owned(),
        (false, false) => format!("{common} {remainder}"),
    }
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn wratio_raw(a: &str, b: &str, force_ascii: bool, process: bool) -> f64 {
    let (left, right) = prepare_pair(a, b, force_ascii, process);
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let base = normalized_scorer_value(ratio_raw(&left, &right));
    let left_len = left.chars().count();
    let right_len = right.chars().count();
    let length_ratio = left_len.max(right_len) as f64 / left_len.min(right_len) as f64;

    if length_ratio < 1.5 {
        base.max(0.95 * normalized_scorer_value(token_sort_raw(&left, &right, false, false, false)))
            .max(0.95 * normalized_scorer_value(token_set_raw(&left, &right, false, false, false)))
    } else {
        let partial_scale = if length_ratio <= 8.0 { 0.9 } else { 0.6 };
        base.max(partial_scale * normalized_scorer_value(partial_ratio_raw(&left, &right)))
            .max(
                0.95 * partial_scale
                    * normalized_scorer_value(token_sort_raw(&left, &right, false, false, true)),
            )
            .max(
                0.95 * partial_scale
                    * normalized_scorer_value(token_set_raw(&left, &right, false, false, true)),
            )
    }
}

/// Preserves the floating-point value exposed by a component scorer while
/// expressing that value on the normalized scale used by the composite.
fn normalized_scorer_value(value: f64) -> f64 {
    (value * 100.0) / 100.0
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::float_cmp
)]
pub(crate) fn round_score(value: f64) -> u8 {
    let floor = value.floor();
    let fraction = value - floor;
    let rounded = if fraction < 0.5 {
        floor
    } else if fraction > 0.5 || floor % 2.0 == 1.0 {
        floor + 1.0
    } else {
        floor
    };
    rounded.clamp(0.0, 100.0) as u8
}

#[cfg(test)]
#[path = "fuzz_tests.rs"]
pub(crate) mod tests;
