//! Text processing used by composite scorers.

use crate::unicode_tables::ALNUM_RANGES;

/// Replaces characters outside Unicode letter and number categories with
/// spaces, lowercases the result, and trims the ends.
///
/// With `force_ascii`, code points from U+0080 through U+00FF are deleted
/// before classification.
#[must_use]
pub fn full_process(value: &str, force_ascii: bool) -> String {
    let mut processed = String::new();
    for ch in value.chars() {
        let code = u32::from(ch);
        if force_ascii && (0x80..=0xff).contains(&code) {
            continue;
        }
        if is_letter_or_number(code) {
            if ch == '\u{130}' {
                processed.push('i');
            } else {
                processed.extend(ch.to_lowercase());
            }
        } else {
            processed.push(' ');
        }
    }
    processed.trim().to_owned()
}

fn is_letter_or_number(code: u32) -> bool {
    ALNUM_RANGES
        .binary_search_by(|&(start, end)| {
            if end < code {
                std::cmp::Ordering::Less
            } else if start > code {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}
