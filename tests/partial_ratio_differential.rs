//! Replays 2,000 seeded `partial_ratio` cases against recorded expected scores.
//!
//! The inputs are rebuilt here from a splitmix64 seed, so the fixture holds only
//! the scores. A quarter of the cases have needles of 200 to 1,000 characters,
//! and others cover equal lengths, edited copies, non-ASCII text, and lengths
//! at 64-bit word boundaries.
#![warn(clippy::all, clippy::pedantic)]

use serde::Deserialize;
use thefuzz_parity::partial_ratio;

#[derive(Deserialize)]
struct Fixture {
    seed: u64,
    case_count: usize,
    input_digest: String,
    scores: Vec<u8>,
}

const ASCII: &str = "abcdefghijklmnopqrstuvwxyz";
const WIDE: &str = "a\u{e9}\u{df}\u{436}\u{4e16}\u{754c}\u{1f600}\u{f1}b";

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn index(&mut self, len: usize) -> usize {
        usize::try_from(self.next() % len as u64).expect("index fits in usize")
    }

    fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.index(hi - lo + 1)
    }

    fn text(&mut self, alphabet: &[char], len: usize) -> Vec<char> {
        (0..len)
            .map(|_| alphabet[self.index(alphabet.len())])
            .collect()
    }
}

fn case(rng: &mut Rng, index: usize) -> (String, String) {
    let ascii: Vec<char> = ASCII.chars().collect();
    let wide: Vec<char> = WIDE.chars().collect();
    let (mut a, mut b);
    match index % 8 {
        0 => {
            let alpha = &ascii[..rng.range(2, 26)];
            let len = rng.range(1, 64);
            a = rng.text(alpha, len);
            let len = rng.range(1, 64);
            b = rng.text(alpha, len);
        }
        1 => {
            let alpha = &ascii[..rng.range(2, 6)];
            let n = rng.range(65, 200);
            a = rng.text(alpha, n);
            let len = rng.range(n, n + 300);
            b = rng.text(alpha, len);
        }
        2 => {
            let alpha = &ascii[..rng.range(2, 8)];
            let n = rng.range(65, 1000);
            a = rng.text(alpha, n);
            b = rng.text(alpha, n);
        }
        3 => {
            let alpha = &ascii[..rng.range(3, 26)];
            let n = rng.range(200, 1000);
            a = rng.text(alpha, n);
            let len = rng.range(n, n + 1000);
            b = rng.text(alpha, len);
        }
        4 => {
            let alpha = &ascii[..rng.range(2, 26)];
            let len = rng.range(10, 100);
            a = rng.text(alpha, len);
            let len = rng.range(100, 1000);
            b = rng.text(alpha, len);
        }
        5 => {
            let alpha = &wide[..rng.range(2, wide.len())];
            let len = rng.range(1, 300);
            a = rng.text(alpha, len);
            let len = rng.range(1, 300);
            b = rng.text(alpha, len);
        }
        6 => {
            let alpha = &ascii[..rng.range(4, 26)];
            let len = rng.range(130, 1200);
            b = rng.text(alpha, len);
            let n = rng.range(65, b.len() / 2);
            let start = rng.range(0, b.len() - n);
            a = b[start..start + n].to_vec();
            for _ in 0..rng.range(0, n / 4) {
                let at = rng.index(a.len());
                a[at] = alpha[rng.index(alpha.len())];
            }
        }
        _ => {
            let alpha = &ascii[..rng.range(2, 4)];
            let n = [63, 64, 65, 127, 128, 129][rng.index(6)];
            a = rng.text(alpha, n);
            let len = rng.range(n, n + 70);
            b = rng.text(alpha, len);
        }
    }
    if rng.next() % 2 == 1 {
        std::mem::swap(&mut a, &mut b);
    }
    (a.into_iter().collect(), b.into_iter().collect())
}

/// FNV-1a over the code points of `text`, then a 0xFFFF separator.
fn fnv(mut digest: u64, text: &str) -> u64 {
    for code in text.chars().map(u64::from).chain([0xFFFF]) {
        digest = (digest ^ code).wrapping_mul(0x0100_0000_01B3);
    }
    digest
}

#[test]
fn partial_ratio_matches_recorded_scores() {
    let fixture: Fixture =
        serde_json::from_str(include_str!("data/partial-ratio-differential.json"))
            .expect("the differential fixture must be valid JSON");
    assert_eq!(fixture.case_count, 2_000);
    assert_eq!(fixture.scores.len(), fixture.case_count);

    let mut rng = Rng(fixture.seed);
    let mut digest = 0xCBF2_9CE4_8422_2325_u64;
    let mut mismatches = Vec::new();
    for (index, &expected) in fixture.scores.iter().enumerate() {
        let (a, b) = case(&mut rng, index);
        digest = fnv(fnv(digest, &a), &b);
        let actual = partial_ratio(&a, &b);
        if actual != expected {
            mismatches.push((
                index,
                a.chars().count(),
                b.chars().count(),
                actual,
                expected,
            ));
        }
    }
    assert_eq!(
        format!("{digest:016x}"),
        fixture.input_digest,
        "the rebuilt inputs differ from the inputs the scores were recorded for"
    );
    assert!(
        mismatches.is_empty(),
        "{} of {} cases differ (index, len a, len b, actual, expected): {:?}",
        mismatches.len(),
        fixture.case_count,
        &mismatches[..mismatches.len().min(20)]
    );
}
