//! Replays the shipped score, processing, and selector cases.
#![warn(clippy::all, clippy::pedantic)]

use std::collections::BTreeSet;

use serde::Deserialize;
use serde_json::Value;
use thefuzz_parity::{fuzz, process, utils};

#[derive(Deserialize)]
struct Fixture {
    vector_count: usize,
    vectors: Vec<ScoreVector>,
    full_process_cases: Vec<ProcessVector>,
    process_cases: Value,
}

#[derive(Deserialize)]
struct ScoreVector {
    input_a: String,
    input_b: String,
    function: String,
    expected_score: u8,
}

#[derive(Deserialize)]
struct ProcessVector {
    input: String,
    force_ascii: bool,
    output: String,
}

fn fixture() -> Fixture {
    serde_json::from_str(include_str!("data/conformance-vectors.json"))
        .expect("the conformance fixture must be valid JSON")
}

fn score(vector: &ScoreVector) -> u8 {
    match vector.function.as_str() {
        "ratio" => fuzz::ratio(&vector.input_a, &vector.input_b),
        "partial_ratio" => fuzz::partial_ratio(&vector.input_a, &vector.input_b),
        "token_sort_ratio" => fuzz::token_sort_ratio(&vector.input_a, &vector.input_b, true, true),
        "token_set_ratio" => fuzz::token_set_ratio(&vector.input_a, &vector.input_b, true, true),
        "partial_token_sort_ratio" => {
            fuzz::partial_token_sort_ratio(&vector.input_a, &vector.input_b, true, true)
        }
        "partial_token_set_ratio" => {
            fuzz::partial_token_set_ratio(&vector.input_a, &vector.input_b, true, true)
        }
        "token_sort_ratio(force_ascii=False)" => {
            fuzz::token_sort_ratio(&vector.input_a, &vector.input_b, false, true)
        }
        "token_set_ratio(force_ascii=False)" => {
            fuzz::token_set_ratio(&vector.input_a, &vector.input_b, false, true)
        }
        "WRatio" => fuzz::wratio(&vector.input_a, &vector.input_b, true, true),
        "QRatio" => fuzz::qratio(&vector.input_a, &vector.input_b, true, true),
        "UWRatio" => fuzz::uwratio(&vector.input_a, &vector.input_b),
        "UQRatio" => fuzz::uqratio(&vector.input_a, &vector.input_b),
        function => panic!("unrecognized scorer {function}"),
    }
}

#[test]
fn all_score_vectors_match() {
    let fixture = fixture();
    assert_eq!(fixture.vector_count, 490);
    assert_eq!(fixture.vectors.len(), fixture.vector_count);
    for vector in &fixture.vectors {
        assert_eq!(
            score(vector),
            vector.expected_score,
            "{} failed for {:?} and {:?}",
            vector.function,
            vector.input_a,
            vector.input_b
        );
    }
}

#[test]
fn all_full_process_vectors_match() {
    let fixture = fixture();
    assert_eq!(fixture.full_process_cases.len(), 27);
    for case in fixture.full_process_cases {
        assert_eq!(
            utils::full_process(&case.input, case.force_ascii),
            case.output,
            "processing failed for {:?}",
            case.input
        );
    }
}

#[test]
fn all_selector_vectors_match() {
    let fixture = fixture();
    let cases = fixture
        .process_cases
        .as_object()
        .expect("selector cases must form an object");
    assert_eq!(cases.len(), 8);

    let extract_default = &cases["extract_default"];
    let choices = strings(extract_default, "choices");
    let actual = process::extract(
        text(extract_default, "query"),
        choices.iter().copied(),
        process::Scorer::WRatio,
        Some(number(extract_default, "limit")),
        0,
    );
    assert_matches(&actual, &extract_default["result"]);

    let extract_one = &cases["extractOne_default"];
    let choices = strings(extract_one, "choices");
    assert_match(
        process::extract_one(
            text(extract_one, "query"),
            choices.iter().copied(),
            process::Scorer::WRatio,
            0,
        ),
        &extract_one["result"],
    );

    let cutoff_none = &cases["extractOne_cutoff_none"];
    let choices = strings(cutoff_none, "choices");
    assert_eq!(
        process::extract_one(
            text(cutoff_none, "query"),
            choices.iter().copied(),
            process::Scorer::WRatio,
            u8::try_from(number(cutoff_none, "score_cutoff")).expect("cutoff must fit in a score"),
        ),
        None
    );

    let unicode = &cases["extractOne_unicode"];
    let choices = strings(unicode, "choices");
    assert_match(
        process::extract_one(
            text(unicode, "query"),
            choices.iter().copied(),
            process::Scorer::WRatio,
            0,
        ),
        &unicode["result"],
    );

    let ratio = &cases["extract_scorer_ratio"];
    let choices = strings(ratio, "choices");
    let actual = process::extract(
        text(ratio, "query"),
        choices.iter().copied(),
        process::Scorer::Ratio,
        Some(number(ratio, "limit")),
        0,
    );
    assert_matches(&actual, &ratio["result"]);

    let unordered = &cases["extractWithoutOrder_cutoff50"];
    let choices = strings(unordered, "choices");
    let actual: Vec<_> = process::extract_without_order(
        text(unordered, "query"),
        choices.iter().copied(),
        process::Scorer::WRatio,
        u8::try_from(number(unordered, "score_cutoff")).expect("cutoff must fit in a score"),
    )
    .collect();
    assert_matches(&actual, &unordered["result"]);

    for name in ["dedupe_default", "dedupe_no_dupes_returns_original"] {
        let case = &cases[name];
        let input = strings(case, "input");
        let actual: BTreeSet<_> = process::dedupe(
            &input,
            u8::try_from(number(case, "threshold")).expect("threshold must fit in a score"),
            process::Scorer::TokenSetRatio,
        )
        .into_iter()
        .collect();
        let expected: BTreeSet<_> = strings(case, "result")
            .into_iter()
            .map(str::to_owned)
            .collect();
        assert_eq!(actual, expected, "{name} differed");
    }
}

#[test]
fn conformance_fixture_retains_audit_annotations() {
    let fixture: Value = serde_json::from_str(include_str!("data/conformance-vectors.json"))
        .expect("the conformance fixture must be valid JSON");
    assert!(fixture["provenance"]["amendment_1"].is_object());
    assert_eq!(count_key(&fixture, "amendment"), 108);
    assert_eq!(count_key(&fixture, "provenance"), 109);
}

fn count_key(value: &Value, key: &str) -> usize {
    match value {
        Value::Array(values) => values.iter().map(|value| count_key(value, key)).sum(),
        Value::Object(values) => {
            usize::from(values.contains_key(key))
                + values
                    .values()
                    .map(|value| count_key(value, key))
                    .sum::<usize>()
        }
        _ => 0,
    }
}

#[test]
fn fp_combining_mark() {
    assert_eq!(utils::full_process("שָׁלוֹם", true), "ש  לו ם");
    assert_eq!(fuzz::token_sort_ratio("שָׁלוֹם", "שלום", true, true), 60);
    assert_eq!(fuzz::token_set_ratio("हिन्दी", "हिंदी", true, true), 100);
}

#[test]
fn wratio_normalized() {
    assert_eq!(
        fuzz::wratio("xxyyxxyyy", "  xxyxy x  yy y yxxyx    yx  ", true, true),
        67
    );
    assert_eq!(fuzz::wratio("caa cabbbc", " bcc0 0 000", true, true), 29);
    assert_eq!(
        fuzz::wratio(
            "bbeaff add ccebbddbc edaba deefbedebfcdb",
            "bdcffaaef ebaefbdbceff a cebdaddfecf",
            true,
            true,
        ),
        52
    );
}

#[test]
fn partial_ratio_long_exact_windows() {
    let boundary_left = format!("{}{}", "a".repeat(10), "b".repeat(247));
    let boundary_right = format!("{}{}", "a".repeat(10), "c".repeat(247));
    assert_eq!(fuzz::partial_ratio(&boundary_left, &boundary_right), 7);

    let shorter = "cabbcddcaccaaadcadddddbcbcadabdadacccabcacdcaccadabddacaadddcadbdadacabdddcbdbcddccabccadabadccbaaacbdccbdbcdbcbbcbdadcdabadddbbadbcdcaccabacdaaccaaaddbdccdacddbbcddadabababacbabddadbaddddddcdccaabaadbdbadddcbaacccbcadacdccaacadddcbbacdbbdcbaabbdbcabbdbcbdb";
    let longer = "bdadcdabaadbdbcabcacdcbaaadddcbaaccbdbcbdbabdadaccbadddbbccadabadcabdddcbdbbadbcdcaccaaaddbdccdaadcadddddacbbacbabcabbcddcaccaadabababbacdbbdcbbcbcadcddccacaccadadccaacadddcaabbdbcabcbcbddacaadadacbbcddbcdbcbbcccaabacdaaccaaddddddcddccbdddcadbdadacacddddadbbdbbbadbabadaabaabbccca";
    assert_eq!(fuzz::partial_ratio(shorter, longer), 68);
}

#[test]
#[allow(clippy::bytes_count_to_len)]
fn char_length_semantics() {
    assert_eq!("日本語".chars().count(), 3);
    assert_eq!("日本語".bytes().count(), 9);
    assert_eq!("abcdefg".chars().count(), 7);
    assert_eq!("abcdefg".bytes().count(), 7);
    assert_eq!(fuzz::partial_ratio("日本語", "abcdefg"), 0);
}

#[test]
fn process_cutoff_ties_and_custom_scorer() {
    let choices = ["first", "second", "third"];
    let fixed = |_a: &str, _b: &str| 70;
    let matches = process::extract("query", choices, process::Scorer::Custom(&fixed), None, 70);
    assert_eq!(
        matches.iter().map(|item| item.item).collect::<Vec<_>>(),
        choices
    );
    assert_eq!(
        process::extract_one("query", choices, process::Scorer::Custom(&fixed), 70,)
            .map(|item| item.item),
        Some("first")
    );
}

#[test]
fn ratio_rounds_half_to_even() {
    let even_left = "a".repeat(133);
    let even_right = format!("{}{}", "a".repeat(133), "b".repeat(134));
    assert_eq!(fuzz::ratio(&even_left, &even_right), 66);

    let odd_left = "a".repeat(27);
    let odd_right = format!("{}{}", "a".repeat(27), "b".repeat(26));
    assert_eq!(fuzz::ratio(&odd_left, &odd_right), 68);
}

#[test]
fn simple_lowercase_mapping_matches_public_scores() {
    assert_eq!(utils::full_process("İ", false), "i");
    assert_eq!(fuzz::uqratio("İ", "i"), 100);
}

#[test]
fn extract_ranks_unrounded_scores() {
    let choices = ["chicago cubs vs chicago white sox", "braves vs mets"];
    let matches = process::extract("cubs", choices, process::Scorer::Ratio, None, 0);
    assert_eq!(matches[0].item, "braves vs mets");
    assert_eq!(matches[0].score, 22);
    assert_eq!(matches[1].score, 22);
}

#[test]
fn process_standard_scorer_processes_once() {
    let matched = process::extract_one("İ", ["i"], process::Scorer::QRatio, 0)
        .expect("the choice must meet cutoff zero");
    assert_eq!(matched.score, 100);
}

#[test]
fn scorer_unicode_variant_names_are_idiomatic() {
    assert_eq!(format!("{:?}", process::Scorer::UwRatio), "UwRatio");
    assert_eq!(format!("{:?}", process::Scorer::UqRatio), "UqRatio");
}

#[test]
fn dedupe_uses_first_occurrence_order() {
    let items = [
        "Frodo Baggin",
        "Frodo Baggins",
        "F. Baggins",
        "Samwise G.",
        "Gandalf",
        "Bilbo Baggins",
    ];
    assert_eq!(
        process::dedupe(&items, 70, process::Scorer::TokenSetRatio),
        ["Frodo Baggins", "Samwise G.", "Gandalf"]
    );
}

#[test]
fn dedupe_collapses_exact_duplicates() {
    assert_eq!(
        process::dedupe(&["same", "same"], 70, process::Scorer::TokenSetRatio),
        ["same"]
    );
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().expect("fixture field must be text")
}

fn number(value: &Value, key: &str) -> usize {
    usize::try_from(
        value[key]
            .as_u64()
            .expect("fixture field must be an unsigned integer"),
    )
    .expect("fixture integer must fit usize")
}

fn strings<'a>(value: &'a Value, key: &str) -> Vec<&'a str> {
    value[key]
        .as_array()
        .expect("fixture field must be an array")
        .iter()
        .map(|item| item.as_str().expect("fixture array item must be text"))
        .collect()
}

fn assert_matches(actual: &[process::Match<'_>], expected: &Value) {
    let expected = expected.as_array().expect("result must be an array");
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(
            actual.item,
            expected[0].as_str().expect("item must be text"),
            "actual matches: {actual:?}; expected row: {expected:?}"
        );
        assert_eq!(
            u64::from(actual.score),
            expected[1].as_u64().expect("score must be an integer")
        );
    }
}

fn assert_match(actual: Option<process::Match<'_>>, expected: &Value) {
    let expected = expected.as_array().expect("result must be an array");
    let actual = actual.expect("a result was expected");
    assert_eq!(
        actual.item,
        expected[0].as_str().expect("item must be text")
    );
    assert_eq!(
        u64::from(actual.score),
        expected[1].as_u64().expect("score must be an integer")
    );
}
