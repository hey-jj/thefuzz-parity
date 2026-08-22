#!/usr/bin/env python3
"""Write letter and number ranges from Unicode 13.0.0 data.

Usage: python3 tools/gen_unicode_tables.py /path/to/UnicodeData.txt > src/unicode_tables.rs

Text processing keeps a character when its general category is L* or N*. The
script writes sorted, non-overlapping ranges for binary search.

UnicodeData.txt 13.0.0 sha256:
bdbffbbfc8ad4d3a6d01b5891510458f3d36f7170422af4ea2bed3211a73e8bb
"""
import sys
from pathlib import Path


def main(path):
    rows = [line.split(";") for line in Path(path).read_text(encoding="utf-8").splitlines()]
    keep = set()
    k = 0
    while k < len(rows):
        f = rows[k]
        cp = int(f[0], 16)
        cat = f[2]
        if f[1].endswith(", First>"):
            end = int(rows[k + 1][0], 16)
            if cat[0] in "LN":
                keep.update(range(cp, end + 1))
            k += 2
            continue
        if cat[0] in "LN":
            keep.add(cp)
        k += 1

    ranges = []
    for cp in sorted(keep):
        if ranges and ranges[-1][1] == cp - 1:
            ranges[-1][1] = cp
        else:
            ranges.append([cp, cp])

    out = sys.stdout
    out.write("// Unicode 13.0.0 letter and number category ranges.\n\n")
    out.write("/// Inclusive code point ranges whose general category is L* or N*.\n")
    out.write("pub(crate) static ALNUM_RANGES: &[(u32, u32)] = &[\n")
    for a, b in ranges:
        out.write(f"    (0x{a:04X}, 0x{b:04X}),\n")
    out.write("];\n")


if __name__ == "__main__":
    main(sys.argv[1])
