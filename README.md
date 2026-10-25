# thefuzz-parity

`thefuzz-parity` scores string similarity and ranks choices. It counts Unicode
scalar values for matcher indices, lengths, and slices. The crate has no runtime
dependencies.

The scoring module provides base, partial, token, and weighted scorers. Text
processing replaces characters outside Unicode letter and number categories with
spaces, lowercases the result, and trims its ends. Interior space runs remain.

```rust
use thefuzz_parity::{partial_ratio, token_set_ratio};

assert_eq!(partial_ratio("こんにちは世界", "こんにちは"), 100);
assert_eq!(token_set_ratio("new york", "new york mets", true, true), 100);
```

The `process` module ranks borrowed choices and returns each input string unchanged.
Cutoffs are inclusive. Equal unrounded scores retain input order.

```rust
use thefuzz_parity::process::{extract, Scorer};

let choices = ["new york mets", "chicago cubs", "new york jets"];
let matches = extract("new york", choices, Scorer::WRatio, Some(2), 0);

assert_eq!(matches.len(), 2);
assert_eq!(matches[0].item, "new york mets");
```

Duplicate removal chooses the longest member of each similarity group. A length
tie selects the lexicographically greatest string. The output contains each
representative once, ordered by the first input that selects it.

## Compatibility

The minimum supported Rust version is 1.70.0. Version 0.1 uses the standard
library and forbids unsafe code.

`partial_ratio` returns the best score over every alignment of the shorter input
in the longer one, skipping alignments that cannot beat the best score found so
far. Two 1,000-character strings take about 60 microseconds in release mode.

## Relation to thefuzz

Not affiliated with SeatGeek or the thefuzz maintainers. Score parity is a tested
claim against thefuzz 0.22.1 with rapidfuzz 3.14.5, covering 490 fixed vectors and
2,000 seeded `partial_ratio` cases with inputs up to 2,000 characters.
`partial_ratio` uses the same pruned window search as rapidfuzz, which thefuzz
calls for this scorer.

## License

MIT
