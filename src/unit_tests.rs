use super::{find_longest_match, lcs_len, lcs_len_scalar, MatchBlock};

#[test]
fn lcs_matches_scalar_across_word_boundaries() {
    for length in [0, 1, 63, 64, 65, 127, 128, 129, 191, 192, 193, 200] {
        let repeated = vec!['a'; length];
        assert_eq!(lcs_len(&repeated, &repeated), length);
        assert_eq!(lcs_len(&repeated, &[]), 0);

        let a: Vec<_> = "abcde".chars().cycle().take(length).collect();
        let b: Vec<_> = "edcba".chars().cycle().take(200).collect();
        assert_eq!(lcs_len(&a, &b), lcs_len_scalar(&a, &b));
        assert_eq!(lcs_len(&b, &a), lcs_len_scalar(&b, &a));

        let mut distant = vec!['x'; length];
        distant.push('a');
        assert_eq!(lcs_len(&distant, &['a', 'x']), 1);
        assert_eq!(lcs_len(&distant, &['x', 'a']), usize::from(length > 0) + 1);
    }
}

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

/// 10,000 seeded pairs over small alphabets, up to 200 characters each, so the
/// bit-parallel rows see dense matches and multi-word borrows.
#[test]
fn lcs_matches_scalar_on_seeded_pairs() {
    let mut rng = crate::fuzz::tests::Rng(0x0B17_9A12);
    for _ in 0..10_000 {
        let a_len = rng.range(0, 200);
        let b_len = rng.range(0, 200);
        let a = rng.text(a_len);
        let b = rng.text(b_len);
        assert_eq!(lcs_len(&a, &b), lcs_len_scalar(&a, &b), "{a:?} / {b:?}");
    }
}
