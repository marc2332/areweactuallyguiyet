import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from test_publish import source_run

CLI = Path(__file__).resolve().parents[1] / "bench.py"


class CliTests(unittest.TestCase):
    def cli(self, *arguments):
        return subprocess.run([sys.executable, str(CLI), *map(str, arguments)],
                              text=True, capture_output=True, check=False)

    def test_commands_are_discoverable(self):
        result = self.cli("list")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("freya", result.stdout)
        self.assertIn("whisker", result.stdout)
        for command in ("scroll", "dashboard", "parse", "sizes", "publish"):
            with self.subTest(command=command):
                self.assertEqual(self.cli(command, "--help").returncode, 0)

    def test_parse_only_saves_when_requested_and_rejects_incompatible_history(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            csv = root / "capture.csv"
            csv.write_text("fps,frametime,elapsed\n100,10,10000000\n1,1000,1010000000\n")
            history = root / "history.json"
            arguments = ("parse", "scroll", "freya", csv, "--duration", "1", "--history", history)
            result = self.cli(*arguments)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)["results"][0]["metrics"]["frames"], 1)
            self.assertFalse(history.exists())
            self.assertEqual(self.cli(*arguments, "--save").returncode, 0)
            self.assertEqual(len(json.loads(history.read_text())["runs"]), 1)
            incompatible = '{"schemaVersion": 1, "measurement": "old", "runs": []}'
            history.write_text(incompatible)
            self.assertNotEqual(self.cli(*arguments, "--save").returncode, 0)
            self.assertEqual(history.read_text(), incompatible)
            csv.write_text("not a frame capture\n")
            self.assertNotEqual(self.cli(*arguments, "--save").returncode, 0)
            self.assertEqual(history.read_text(), incompatible)

    def test_publish_defaults_to_a_dry_run(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "run.json"
            source.write_text(json.dumps(source_run()))
            destination = root / "absent/history.json"
            result = self.cli("publish", source, "--history", destination)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)["results"][0]["scenario"], "scroll-1000")
            self.assertFalse(destination.parent.exists())

    def test_invalid_options_fail_before_running_apps(self):
        for arguments in (
            ("scroll", "freya", "--rows", "0"),
            ("scroll", "freya", "--rows", "1000", "1000"),
            ("scroll", "freya", "--duration", "nan"),
            ("dashboard", "freya", "--rows", "1000"),
            ("dashboard", "freya", "--duration", "3"),
            ("parse", "scroll", "freya", "missing.csv"),
            ("parse", "dashboard", "freya", "missing.csv", "--duration", "1"),
            ("publish", "missing.json", "--latest"),
        ):
            with self.subTest(arguments=arguments):
                result = self.cli(*arguments)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertIn("error:", result.stderr)
