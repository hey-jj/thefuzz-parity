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
    let (pattern, text) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    Pattern::new(pattern).lcs(text.iter().copied())
}

/// Bit vectors that mark where each character occurs in a fixed pattern.
///
/// Bit `i % 64` of word `i / 64` is set when the pattern holds the character
/// at index `i`. One pass over a text then yields the longest common
/// subsequence length of the pattern and every prefix of that text.
pub(crate) struct Pattern {
    len: usize,
    words: usize,
    ascii: Vec<u64>,
    ascii_present: [bool; 128],
    other: HashMap<char, Vec<u64>>,
}

impl Pattern {
    pub(crate) fn new(chars: &[char]) -> Self {
        let words = (chars.len() + 63) / 64;
        let mut pattern = Self {
            len: chars.len(),
            words,
            ascii: vec![0_u64; 128 * words],
            ascii_present: [false; 128],
            other: HashMap::new(),
        };
        for (index, &ch) in chars.iter().enumerate() {
            let bit = 1_u64 << (index % 64);
            let code = ch as usize;
            if code < 128 {
                pattern.ascii_present[code] = true;
                pattern.ascii[code * words + index / 64] |= bit;
            } else {
                pattern
                    .other
                    .entry(ch)
                    .or_insert_with(|| vec![0_u64; words])[index / 64] |= bit;
            }
        }
        pattern
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    fn matches(&self, ch: char) -> Option<&[u64]> {
        let code = ch as usize;
        if code < 128 {
            if self.ascii_present[code] {
                Some(&self.ascii[code * self.words..(code + 1) * self.words])
            } else {
                None
            }
        } else {
            self.other.get(&ch).map(Vec::as_slice)
        }
    }

    /// Advances the state by one text character. Set bits in the state count
    /// the common subsequence matched so far.
    fn step(&self, state: &mut [u64], ch: char) {
        let Some(matches) = self.matches(ch) else {
            return;
        };
        if let ([word], [matched]) = (&mut *state, matches) {
            *word = word.wrapping_sub(matched & !*word) & (*word | matched);
            return;
        }
        // Subtract across words, carrying each borrow into the next word.
        let mut borrow = 0_u64;
        for (word, &matched) in state.iter_mut().zip(matches) {
            let available = matched & !*word;
            let (first, borrow_first) = word.overflowing_sub(available);
            let (second, borrow_second) = first.overflowing_sub(borrow);
            *word = second & (*word | matched);
            borrow = u64::from(borrow_first | borrow_second);
        }
    }

    fn count(state: &[u64]) -> usize {
        state.iter().map(|word| word.count_ones() as usize).sum()
    }

    /// Returns the longest common subsequence length of the pattern and `text`.
    pub(crate) fn lcs(&self, text: impl IntoIterator<Item = char>) -> usize {
        if self.len == 0 {
            return 0;
        }
        let mut state = vec![0_u64; self.words];
        for ch in text {
            self.step(&mut state, ch);
        }
        Self::count(&state)
    }

    /// Calls `visit(i, lcs)` after the `i`th character of `text`, where `lcs`
    /// is the longest common subsequence length of the pattern and the first
    /// `i` characters.
    pub(crate) fn prefix_lcs(
        &self,
        text: impl IntoIterator<Item = char>,
        mut visit: impl FnMut(usize, usize),
    ) {
        let mut state = vec![0_u64; self.words];
        for (index, ch) in text.into_iter().enumerate() {
            self.step(&mut state, ch);
            visit(index + 1, Self::count(&state));
        }
    }
}

#[cfg(test)]
pub(crate) fn lcs_len_scalar(a: &[char], b: &[char]) -> usize {
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
