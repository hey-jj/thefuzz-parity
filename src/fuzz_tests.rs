use super::{partial_ratio, partial_ratio_raw_chars};

/// The 0.1.0 scorer, copied unchanged from tag v0.1.0: scalar LCS rows and a
/// scan of every window.
mod v0_1_0 {
    pub(super) fn lcs_len(a: &[char], b: &[char]) -> usize {
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

    #[allow(clippy::cast_precision_loss)]
    fn ratio_raw_chars(a: &[char], b: &[char]) -> f64 {
        let total = a.len() + b.len();
        if total == 0 {
            1.0
        } else {
            let distance = total - 2 * lcs_len(a, b);
            1.0 - distance as f64 / total as f64
        }
    }

    pub(super) fn partial_ratio_raw_chars(a: &[char], b: &[char]) -> f64 {
        let (shorter, longer) = if a.len() <= b.len() { (a, b) } else { (b, a) };
        if shorter.is_empty() {
            return if longer.is_empty() { 1.0 } else { 0.0 };
        }

        let score = exhaustive_partial_ratio(shorter, longer);
        if a.len() == b.len() && score < 1.0 {
            let reverse = exhaustive_partial_ratio(longer, shorter);
            score.max(reverse)
        } else {
            score
        }
    }

    #[allow(clippy::float_cmp)]
    fn exhaustive_partial_ratio(shorter: &[char], longer: &[char]) -> f64 {
        let width = shorter.len();
        let mut best = 0.0_f64;

        for start in 0..=longer.len() - width {
            best = best.max(ratio_raw_chars(shorter, &longer[start..start + width]));
            if best == 1.0 {
                return best;
            }
        }

        for boundary_width in 1..width {
            best = best.max(ratio_raw_chars(shorter, &longer[..boundary_width]));
            best = best.max(ratio_raw_chars(
                shorter,
                &longer[longer.len() - boundary_width..],
            ));
        }

        best
    }
}

/// splitmix64, so every run checks the same pairs.
pub(crate) struct Rng(pub(crate) u64);

impl Rng {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub(crate) fn range(&mut self, lo: usize, hi: usize) -> usize {
        let span = u64::try_from(hi - lo + 1).expect("span fits in u64");
        lo + usize::try_from(self.next() % span).expect("offset fits in usize")
    }

    /// Random text over a small alphabet, so windows share many characters.
    pub(crate) fn text(&mut self, len: usize) -> Vec<char> {
        const POOL: [char; 8] = ['a', 'b', 'c', 'd', '\u{e9}', '\u{4e16}', '\u{1f600}', 'e'];
        let size = self.range(1, POOL.len());
        (0..len).map(|_| POOL[self.range(0, size - 1)]).collect()
    }
}

/// 10,000 seeded pairs with needles up to 150 characters, which crosses two
/// 64-bit word boundaries. Debug builds check the first 1,000 to keep the
/// scalar scan affordable, and release builds check all of them.
#[test]
fn partial_scores_equal_v0_1_0_scan() {
    let cases = if cfg!(debug_assertions) {
        1_000
    } else {
        10_000
    };
    let mut rng = Rng(0x0E7A_C710);
    for index in 0..cases {
        let (a_len, b_len) = match index % 4 {
            0 => (rng.range(0, 40), rng.range(0, 40)),
            1 => {
                let n = rng.range(50, 150);
                (n, rng.range(n, n + 40))
            }
            2 => {
                let n = rng.range(60, 150);
                (n, n)
            }
            _ => (rng.range(0, 20), rng.range(0, 200)),
        };
        let a = rng.text(a_len);
        let b = rng.text(b_len);
        let expected = v0_1_0::partial_ratio_raw_chars(&a, &b);
        let actual = partial_ratio_raw_chars(&a, &b);
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "case {index}: {actual} != {expected} for {a:?} / {b:?}"
        );
        let (a, b): (String, String) = (a.iter().collect(), b.iter().collect());
        assert_eq!(
            partial_ratio(&a, &b),
            super::round_score(expected * 100.0),
            "case {index}"
        );
    }
}
