# Contract tension choices

## U+0130 lowercase behavior

The public score contract requires exact results. `full_process` maps U+0130 to
`i`, which makes `uqratio("İ", "i")` return 100. Other characters continue to use
Rust's full lowercase iterator. This resolves the reviewed case without adding a
second casing table to the Unicode range generator.

## Conformance annotations

The shipped conformance fixture is a byte-for-byte copy of the frozen behavioral
data. Its annotation fields contain none of the package scrub literals, so no
field was changed. The fixture keeps all 490 scores, 27 processing cases, and 8
selector cases with their audit annotations.
