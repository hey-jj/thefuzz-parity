//! Ranked extraction and duplicate removal.

use crate::fuzz;
use crate::utils::full_process;
use std::marker::PhantomData;

/// A scored item borrowed from the caller's choices.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Match<'a> {
    /// The unprocessed input item.
    pub item: &'a str,
    /// Integer similarity score.
    pub score: u8,
}

/// Selects a scoring function for extraction and duplicate removal.
#[derive(Clone, Copy)]
pub enum Scorer<'s> {
    /// Base indel similarity after default processing.
    Ratio,
    /// Partial indel similarity after default processing.
    PartialRatio,
    /// Token sort similarity.
    TokenSortRatio,
    /// Token set similarity.
    TokenSetRatio,
    /// Partial token sort similarity.
    PartialTokenSortRatio,
    /// Partial token set similarity.
    PartialTokenSetRatio,
    /// Weighted similarity.
    WRatio,
    /// Processed base similarity.
    QRatio,
    /// Weighted similarity without the U+0080 through U+00FF deletion step.
    UwRatio,
    /// Processed base similarity without the U+0080 through U+00FF deletion step.
    UqRatio,
    /// A caller-provided scorer. Inputs receive default processing first.
    Custom(&'s dyn Fn(&str, &str) -> u8),
}

impl std::fmt::Debug for Scorer<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Ratio => "Ratio",
            Self::PartialRatio => "PartialRatio",
            Self::TokenSortRatio => "TokenSortRatio",
            Self::TokenSetRatio => "TokenSetRatio",
            Self::PartialTokenSortRatio => "PartialTokenSortRatio",
            Self::PartialTokenSetRatio => "PartialTokenSetRatio",
            Self::WRatio => "WRatio",
            Self::QRatio => "QRatio",
            Self::UwRatio => "UwRatio",
            Self::UqRatio => "UqRatio",
            Self::Custom(_) => "Custom",
        };
        formatter.write_str(name)
    }
}

impl Scorer<'_> {
    fn force_ascii(self) -> bool {
        !matches!(self, Self::UwRatio | Self::UqRatio)
    }

    fn score_processed_raw(self, a: &str, b: &str) -> f64 {
        match self {
            Self::Ratio | Self::QRatio | Self::UqRatio => fuzz::ratio_raw(a, b) * 100.0,
            Self::PartialRatio => fuzz::partial_ratio_raw(a, b) * 100.0,
            Self::TokenSortRatio => fuzz::token_sort_raw(a, b, false, false, false) * 100.0,
            Self::TokenSetRatio => fuzz::token_set_raw(a, b, false, false, false) * 100.0,
            Self::PartialTokenSortRatio => fuzz::token_sort_raw(a, b, false, false, true) * 100.0,
            Self::PartialTokenSetRatio => fuzz::token_set_raw(a, b, false, false, true) * 100.0,
            Self::WRatio | Self::UwRatio => fuzz::wratio_raw(a, b, false, false) * 100.0,
            Self::Custom(score) => f64::from(score(a, b)),
        }
    }
}

/// Scores choices, filters by an inclusive cutoff, and returns a stable
/// descending ranking.
pub fn extract<'a, I>(
    query: &str,
    choices: I,
    scorer: Scorer<'_>,
    limit: Option<usize>,
    score_cutoff: u8,
) -> Vec<Match<'a>>
where
    I: IntoIterator<Item = &'a str>,
{
    let query = full_process(query, scorer.force_ascii());
    let mut matches = Vec::new();
    for (index, item) in choices.into_iter().enumerate() {
        let raw_score = score_choice_raw(&query, item, scorer);
        if raw_score >= f64::from(score_cutoff) {
            matches.push((
                index,
                raw_score,
                Match {
                    item,
                    score: fuzz::round_score(raw_score),
                },
            ));
        }
    }
    matches.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    matches.truncate(limit.unwrap_or(usize::MAX));
    matches.into_iter().map(|(_, _, item)| item).collect()
}

/// Returns the first highest-scoring choice at or above an inclusive cutoff.
pub fn extract_one<'a, I>(
    query: &str,
    choices: I,
    scorer: Scorer<'_>,
    score_cutoff: u8,
) -> Option<Match<'a>>
where
    I: IntoIterator<Item = &'a str>,
{
    let query = full_process(query, scorer.force_ascii());
    choices
        .into_iter()
        .filter_map(|item| {
            let raw_score = score_choice_raw(&query, item, scorer);
            (raw_score >= f64::from(score_cutoff)).then_some((raw_score, item))
        })
        .fold(None, |best, (raw_score, item)| match best {
            Some((best_score, _)) if best_score >= raw_score => best,
            _ => Some((raw_score, item)),
        })
        .map(|(raw_score, item)| Match {
            item,
            score: fuzz::round_score(raw_score),
        })
}

/// Scores choices lazily in input order and filters by an inclusive cutoff.
pub fn extract_without_order<'a, 's, I>(
    query: &str,
    choices: I,
    scorer: Scorer<'s>,
    score_cutoff: u8,
) -> ExtractWithoutOrder<'a, 's, I::IntoIter>
where
    I: IntoIterator<Item = &'a str>,
{
    ExtractWithoutOrder {
        choices: choices.into_iter(),
        query: full_process(query, scorer.force_ascii()),
        scorer,
        score_cutoff,
        item_lifetime: PhantomData,
    }
}

/// Lazy choice scores in their input order.
pub struct ExtractWithoutOrder<'a, 's, I> {
    choices: I,
    query: String,
    scorer: Scorer<'s>,
    score_cutoff: u8,
    item_lifetime: PhantomData<&'a str>,
}

impl<'a, I> Iterator for ExtractWithoutOrder<'a, '_, I>
where
    I: Iterator<Item = &'a str>,
{
    type Item = Match<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        for item in self.choices.by_ref() {
            let raw_score = score_choice_raw(&self.query, item, self.scorer);
            if raw_score >= f64::from(self.score_cutoff) {
                return Some(Match {
                    item,
                    score: fuzz::round_score(raw_score),
                });
            }
        }
        None
    }
}

/// Removes similar items and returns representatives in first-occurrence order.
#[must_use]
pub fn dedupe(items: &[&str], threshold: u8, scorer: Scorer<'_>) -> Vec<String> {
    let mut representatives = Vec::with_capacity(items.len());
    for &item in items {
        let representative = items
            .iter()
            .copied()
            .filter(|&candidate| score_pair(item, candidate, scorer) >= threshold)
            .max_by(|left, right| {
                left.chars()
                    .count()
                    .cmp(&right.chars().count())
                    .then_with(|| left.cmp(right))
            })
            .unwrap_or(item);
        if !representatives.contains(&representative) {
            representatives.push(representative);
        }
    }

    if representatives.len() == items.len() {
        items.iter().map(|item| (*item).to_owned()).collect()
    } else {
        representatives.into_iter().map(str::to_owned).collect()
    }
}

/// Extracts up to five matches with weighted similarity and cutoff zero.
pub fn extract_default<'a, I>(query: &str, choices: I) -> Vec<Match<'a>>
where
    I: IntoIterator<Item = &'a str>,
{
    extract(query, choices, Scorer::WRatio, Some(5), 0)
}

/// Extracts one match with weighted similarity and cutoff zero.
pub fn extract_one_default<'a, I>(query: &str, choices: I) -> Option<Match<'a>>
where
    I: IntoIterator<Item = &'a str>,
{
    extract_one(query, choices, Scorer::WRatio, 0)
}

/// Scores choices lazily with weighted similarity and cutoff zero.
pub fn extract_without_order_default<'a, I>(
    query: &str,
    choices: I,
) -> ExtractWithoutOrder<'a, 'static, I::IntoIter>
where
    I: IntoIterator<Item = &'a str>,
{
    extract_without_order(query, choices, Scorer::WRatio, 0)
}

/// Removes duplicates at threshold 70 with token set similarity.
#[must_use]
pub fn dedupe_default(items: &[&str]) -> Vec<String> {
    dedupe(items, 70, Scorer::TokenSetRatio)
}

fn score_pair(a: &str, b: &str, scorer: Scorer<'_>) -> u8 {
    let force_ascii = scorer.force_ascii();
    let left = full_process(a, force_ascii);
    let right = full_process(b, force_ascii);
    if left.is_empty() {
        0
    } else {
        fuzz::round_score(scorer.score_processed_raw(&left, &right))
    }
}

fn score_choice_raw(query: &str, item: &str, scorer: Scorer<'_>) -> f64 {
    if query.is_empty() {
        0.0
    } else {
        let processed = full_process(item, scorer.force_ascii());
        scorer.score_processed_raw(query, &processed)
    }
}
