# Changelog

## 0.1.1 - 2026-10-25

- `partial_ratio` now uses bit-parallel rows and the same pruned window search
  as the Python scorer it mirrors. Two 1,000-character strings score in about 60
  microseconds in release mode, against 2.3 seconds in 0.1.0 and 21.7
  milliseconds in the pinned Python release.
- No score changes, in `partial_ratio` or in the token, weighted, and `process`
  scorers built on it.

## 0.1.0 - 2026-08-22

### Added

- Added Unicode scalar-based ratio, partial, token, and weighted scorers.
- Added ranked extraction, single-match selection, lazy filtering, and duplicate removal.
- Added exact conformance coverage for scoring, processing, Unicode input, and selector behavior.
- Added normalized weighted scoring with one final scale operation.
- Added exact long-input window scoring and simple lowercase handling for U+0130.
- Added unique representative output for duplicate input strings.
