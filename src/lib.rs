#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]
#![deny(clippy::disallowed_methods)]
//! Fuzzy string scores and ranked choice extraction.
//!
//! All matcher indices and lengths count Unicode scalar values. Scoring does not
//! normalize Unicode text.

pub mod fuzz;
pub mod matcher;
pub mod process;
pub mod utils;

mod unicode_tables;

pub use fuzz::{
    partial_ratio, partial_token_set_ratio, partial_token_sort_ratio, qratio, ratio,
    token_set_ratio, token_sort_ratio, uqratio, uwratio, wratio,
};
