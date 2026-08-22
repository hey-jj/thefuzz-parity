use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};
use thefuzz_parity::{fuzz, matcher, process, utils};

fn main() {
    let text = proptest::collection::vec(unicode_edge_scalar(), 0..32)
        .prop_map(|chars| chars.into_iter().collect::<String>());
    let config = Config {
        cases: 5_000,
        failure_persistence: None,
        ..Config::default()
    };
    TestRunner::new(config)
        .run(&(text.clone(), text), |(left, right)| {
            exercise(&left, &right);
            Ok(())
        })
        .expect("all generated strings must complete");
}

fn unicode_edge_scalar() -> BoxedStrategy<char> {
    let astral = (0x1_0000_u32..=0x10_ffff)
        .prop_map(|code| char::from_u32(code).expect("astral range must contain scalar values"));
    let combining = prop_oneof![
        0x0300_u32..=0x036f,
        0x05b0_u32..=0x05bd,
        0x064b_u32..=0x0652,
        0x0900_u32..=0x0903,
        0x093a_u32..=0x094f,
    ]
    .prop_map(|code| char::from_u32(code).expect("mark ranges must contain scalar values"));
    prop_oneof![any::<char>(), astral, combining].boxed()
}

fn exercise(a: &str, b: &str) {
    let _ = fuzz::ratio(a, b);
    let _ = fuzz::partial_ratio(a, b);
    let _ = fuzz::token_sort_ratio(a, b, true, true);
    let _ = fuzz::token_set_ratio(a, b, true, true);
    let _ = fuzz::partial_token_sort_ratio(a, b, true, true);
    let _ = fuzz::partial_token_set_ratio(a, b, true, true);
    let _ = fuzz::wratio(a, b, true, true);
    let _ = fuzz::qratio(a, b, true, true);
    let _ = fuzz::uwratio(a, b);
    let _ = fuzz::uqratio(a, b);
    let _ = utils::full_process(a, true);
    let _ = utils::full_process(a, false);
    let _ = matcher::find_longest_match(a, b);
    let choices = [a, b];
    let _ = process::extract(a, choices, process::Scorer::WRatio, None, 0);
    let _ = process::extract_one(a, choices, process::Scorer::WRatio, 0);
    let _: Vec<_> =
        process::extract_without_order(a, choices, process::Scorer::WRatio, 0).collect();
    let _ = process::dedupe(&choices, 70, process::Scorer::TokenSetRatio);
}
