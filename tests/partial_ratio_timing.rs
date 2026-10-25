//! Release-mode timing for `partial_ratio`.
//!
//! Each bound sits at least 20 times above the time measured on an Apple
//! silicon laptop, so it catches a return to the 0.1.0 cost, which took over
//! two seconds at 1,000 characters, without failing on a busy machine. Each
//! check prints the best of three runs and asserts the score as well.
#![warn(clippy::all, clippy::pedantic)]

use std::hint::black_box;
use std::time::{Duration, Instant};

use thefuzz_parity::partial_ratio;

/// Two different strings with no shared long runs.
fn pair(len: usize, haystack_len: usize) -> (String, String) {
    let a = (0..len)
        .map(|i| char::from(b'a' + u8::try_from((17 * i + i / 7) % 26).expect("below 26")))
        .collect();
    let b = (0..haystack_len)
        .map(|i| char::from(b'a' + u8::try_from((11 * i + i / 5 + 3) % 26).expect("below 26")))
        .collect();
    (a, b)
}

fn check((len, haystack_len): (usize, usize), expected: u8, limit_ms: u64) {
    let (a, b) = pair(len, haystack_len);
    assert_eq!(
        partial_ratio(&a, &b),
        expected,
        "{len} vs {haystack_len} chars"
    );
    let elapsed = (0..3)
        .map(|_| {
            let start = Instant::now();
            black_box(partial_ratio(black_box(&a), black_box(&b)));
            start.elapsed()
        })
        .min()
        .expect("three samples");
    println!("partial_ratio {len} vs {haystack_len} chars: {elapsed:.3?}");
    assert!(
        elapsed < Duration::from_millis(limit_ms),
        "partial_ratio {len} vs {haystack_len} chars took {elapsed:?}"
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "release timing test")]
fn partial_ratio_timing() {
    check((100, 100), 27, 5);
    check((1_000, 1_000), 27, 20);
    check((4_000, 4_000), 27, 100);
    check((1_000, 2_000), 26, 200);
}
