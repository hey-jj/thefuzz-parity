import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from tools import gen_unicode_tables


class UnicodeTableGeneratorTests(unittest.TestCase):
    def test_generator_emits_only_membership_table(self):
        rows = "\n".join(
            [
                "0030;DIGIT ZERO;Nd",
                "0041;LATIN CAPITAL LETTER A;Lu",
                "0300;COMBINING GRAVE ACCENT;Mn",
            ]
        )
        with tempfile.TemporaryDirectory() as directory:
            data_path = Path(directory) / "UnicodeData.txt"
            data_path.write_text(rows, encoding="utf-8")
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                gen_unicode_tables.main(data_path)

        text = output.getvalue()
        self.assertEqual(text.count("static "), 1)
        self.assertIn("static ALNUM_RANGES", text)
        self.assertNotIn("LOWER_RUNS", text)


if __name__ == "__main__":
    unittest.main()
