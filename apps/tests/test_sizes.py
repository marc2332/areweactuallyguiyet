import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import sizes


class BinarySizesTests(unittest.TestCase):
    def test_collect_measures_files_and_labels_variants(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            manifest = output / "Cargo.toml"
            manifest.write_text('[dependencies]\nfreya = "=0.5.1"\n')
            binary = output / "freya-scroll"
            binary.write_bytes(b"x" * 1234)
            apps = {
                identifier: {"id": identifier, "title": "Freya Scroll 2000", "manifest": manifest}
                for identifier in ("freya", "freya-optimized")
            }
            with patch("bench.build", return_value=binary) as build:
                snapshot = sizes.collect(apps, output, True)
            self.assertEqual(build.call_count, 2)
            self.assertTrue(all(call.args[2] for call in build.call_args_list))
            self.assertEqual(snapshot["results"][0]["bytes"], 1234)
            self.assertEqual(snapshot["results"][1]["name"], "Freya (Optimized)")
            self.assertEqual(snapshot["results"][0]["dependencies"], {"freya": "=0.5.1"})
            self.assertEqual(snapshot["profile"], "release")
            json.dumps(snapshot, allow_nan=False)


if __name__ == "__main__":
    unittest.main()
