//! Character-indexed matching primitives.

use std::collections::HashMap;

/// A longest contiguous match within two inputs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MatchBlock {
    /// Start index in the first input, counted in Unicode scalar values.
    pub a: usize,
    /// Start index in the second input, counted in Unicode scalar values.
    pub b: usize,
    /// Match length, counted in Unicode scalar values.
    pub size: usize,
}

/// Finds the longest contiguous common substring.
///
/// Equal-length matches prefer the lowest index in `a`, then the lowest index
/// in `b`. The search does not discard frequent characters.
#[must_use]
pub fn find_longest_match(a: &str, b: &str) -> MatchBlock {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    find_longest_match_chars(&a_chars, &b_chars)
}

pub(crate) fn find_longest_match_chars(a: &[char], b: &[char]) -> MatchBlock {
    let mut positions: HashMap<char, Vec<usize>> = HashMap::new();
    for (index, ch) in b.iter().copied().enumerate() {
        positions.entry(ch).or_default().push(index);
    }

    let mut previous = vec![0_usize; b.len() + 1];
    let mut current = vec![0_usize; b.len() + 1];
    let mut best = MatchBlock::default();

    for (a_index, ch) in a.iter().copied().enumerate() {
        current.fill(0);
        if let Some(indices) = positions.get(&ch) {
            for &b_index in indices {
                let size = previous[b_index] + 1;
                current[b_index + 1] = size;
                let candidate = MatchBlock {
                    a: a_index + 1 - size,
                    b: b_index + 1 - size,
                    size,
                };
                if candidate.size > best.size
                    || (candidate.size == best.size
                        && (candidate.a, candidate.b) < (best.a, best.b))
                {
                    best = candidate;
                }
            }
        }
        std::mem::swap(&mut previous, &mut current);
    }

    best
}

pub(crate) fn lcs_len(a: &[char], b: &[char]) -> usize {
    let (outer, inner) = if a.len() < b.len() { (b, a) } else { (a, b) };
    let mut row = vec![0_usize; inner.len() + 1];

    for outer_ch in outer {
        let mut diagonal = 0_usize;
        for (index, inner_ch) in inner.iter().enumerate() {
            let above = row[index + 1];
            row[index + 1] = if outer_ch == inner_ch {
                diagonal + 1
            } else {
                row[index].max(above)
            };
            diagonal = above;
        }
    }

    row[inner.len()]
}

#[cfg(test)]
#[path = "unit_tests.rs"]
mod unit_tests;
