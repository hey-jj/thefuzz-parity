use super::{find_longest_match, lcs_len, MatchBlock};

#[test]
fn longest_match_uses_leftmost_tie_breaks() {
    assert_eq!(
        find_longest_match("abXab", "abYab"),
        MatchBlock {
            a: 0,
            b: 0,
            size: 2,
        }
    );
}

#[test]
fn lcs_handles_empty_and_disjoint_inputs() {
    assert_eq!(lcs_len(&[], &['a']), 0);
    assert_eq!(lcs_len(&['a', 'b'], &['x', 'y']), 0);
}

#[test]
fn no_autojunk_for_frequent_characters() {
    let repeated = "x".repeat(250);
    assert_eq!(
        find_longest_match(&repeated, &repeated),
        MatchBlock {
            a: 0,
            b: 0,
            size: 250,
        }
    );
}
