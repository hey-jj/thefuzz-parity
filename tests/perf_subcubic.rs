//! Measures the matcher against repeated, disjoint quadratic workloads.
#![warn(clippy::all, clippy::pedantic)]

use std::hint::black_box;
use std::time::{Duration, Instant};

use thefuzz_parity::{matcher::find_longest_match, ratio};

#[test]
fn perf_subcubic_smoke() {
    let (a, b) = pair(400);
    let start = Instant::now();
    let _ = find_longest_match(&a, &b);
    let _ = ratio(&a, &b);
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[test]
#[cfg_attr(debug_assertions, ignore = "release performance assertion")]
fn perf_subcubic() {
    let timings: Vec<_> = [1_000, 2_000, 4_000]
        .into_iter()
        .map(measure_best_of_three)
        .collect();

    assert!(timings[2] < Duration::from_millis(250), "{timings:?}");
    for pair in timings.windows(2) {
        let growth = pair[1].as_secs_f64() / pair[0].as_secs_f64().max(f64::EPSILON);
        assert!(growth < 8.0, "growth {growth:.2} from {timings:?}");
    }
}

fn measure_best_of_three(size: usize) -> Duration {
    let (a, b) = pair(size);
    (0..3)
        .map(|_| {
            let start = Instant::now();
            black_box(find_longest_match(&a, &b));
            black_box(ratio(&a, &b));
            start.elapsed()
        })
        .min()
        .expect("three timing samples must exist")
}

fn pair(size: usize) -> (String, String) {
    assert!(size > 1);
    ("a".repeat(size), "b".repeat(size))
}
