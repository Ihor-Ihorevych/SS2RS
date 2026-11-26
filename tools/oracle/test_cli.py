import contextlib
import io
import json
import os
import struct
import tempfile
import unittest

from fakes import FakeProcess
from ss2_oracle import cli
from ss2_oracle.backend import AttachError

BASE = 0x400000


def fake_game(alive=True):
    p = FakeProcess(4)
    p.add_module("Sam2.exe", 0x00400000, 0x1000)
    p.add_module("Engine.dll", 0x10000000, 0x10000)
    p.poke(0x10000000 + 0x10, struct.pack("<I", 12345))
    p.alive = alive
    return p


CONFIG = {
    "format": 1,
    "fields": {
        "world.time": {"module": "Engine.dll", "offsets": [0x10], "type": "u32"},
        "player.health": {"module": "Engine.dll", "offsets": [0x40, 8], "type": "f32"},
    },
}


class CliTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.cfg_path = os.path.join(self.tmp.name, "targets.json")
        with open(self.cfg_path, "w") as f:
            json.dump(CONFIG, f)

    def run_cli(self, argv, process=None):
        out, err = io.StringIO(), io.StringIO()
        process = process or fake_game()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = cli.main(argv, attach=lambda args: process)
        return code, out.getvalue(), err.getvalue()

    def test_validate_config_needs_no_process(self):
        def boom(args):
            raise AssertionError("must not attach")

        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = cli.main(["--config", self.cfg_path, "--validate-config"], attach=boom)
        self.assertEqual(code, 0)
        self.assertIn("world.time", out.getvalue())

    def test_bad_config_exit_2_names_the_problem(self):
        with open(self.cfg_path, "w") as f:
            f.write('{"format": 1, "fields": {"x": {"module": "A.dll", "offsets": [-1], "type": "f32"}}}')
        code, out, err = self.run_cli(["--config", self.cfg_path, "--validate-config"])
        self.assertEqual(code, 2)
        self.assertIn("negative", err)

    def test_missing_config_exit_2(self):
        code, out, err = self.run_cli(["--config", os.path.join(self.tmp.name, "none.json"), "--dump-fields"])
        self.assertEqual(code, 2)
        self.assertIn("cannot read the config", err)

    def test_list_modules_needs_no_config(self):
        code, out, err = self.run_cli(["--config", os.path.join(self.tmp.name, "none.json"), "--list-modules"])
        self.assertEqual(code, 0)
        self.assertIn("Engine.dll", out)
        self.assertIn("0x10000000", out)

    def test_dump_fields_prints_values_and_marks_invalid(self):
        code, out, err = self.run_cli(["--config", self.cfg_path, "--dump-fields"])
        self.assertIn("world.time", out)
        self.assertIn("12345", out)
        self.assertIn("(invalid)", out)  # health chain starts at a null pointer
        self.assertEqual(code, 4)  # not every field valid

    def test_dump_fields_all_valid_exit_0(self):
        p = fake_game()
        p.add_region(0x900000, 0x100)
        p.poke(0x10000000 + 0x40, struct.pack("<I", 0x900000))
        p.poke(0x900008, struct.pack("<f", 100.0))
        code, out, err = self.run_cli(["--config", self.cfg_path, "--dump-fields"], p)
        self.assertEqual(code, 0)
        self.assertIn("100.0", out)

    def test_capture_writes_csv_and_sidecar(self):
        out_csv = os.path.join(self.tmp.name, "run.csv")
        code, out, err = self.run_cli(
            ["--config", self.cfg_path, "--out", out_csv, "--scenario", "unit", "--rate", "200", "--samples", "5"]
        )
        self.assertEqual(code, 0, err)
        with open(out_csv) as f:
            lines = f.read().splitlines()
        self.assertEqual(lines[0], "sample,t_wall_s,jitter_s,read_s,valid,n_invalid,world.time,player.health")
        self.assertEqual(len(lines), 6)
        self.assertTrue(lines[1].endswith(",0,1,12345,"))
        with open(out_csv + ".json") as f:
            side = json.load(f)
        self.assertEqual(side["scenario"], "unit")
        self.assertEqual(side["rate_hz"], 200.0)
        self.assertEqual(side["capture"]["samples"], 5)
        self.assertEqual(side["process"]["pointer_size"], 4)
        self.assertEqual(len(side["config_sha256"]), 64)
        self.assertNotIn("offsets", json.dumps(side))

    def test_capture_creates_the_output_folder(self):
        out_csv = os.path.join(self.tmp.name, "a", "b", "run.csv")
        code, _, err = self.run_cli(["--config", self.cfg_path, "--out", out_csv, "--scenario", "s", "--samples", "2"])
        self.assertEqual(code, 0, err)
        self.assertTrue(os.path.isfile(out_csv) and os.path.isfile(out_csv + ".json"))

    def test_capture_needs_scenario_and_out(self):
        code, _, err = self.run_cli(["--config", self.cfg_path, "--samples", "1"])
        self.assertEqual(code, 2)
        self.assertIn("--out", err)

    def test_capture_refuses_to_overwrite(self):
        out_csv = os.path.join(self.tmp.name, "run.csv")
        with open(out_csv, "w") as f:
            f.write("keep me")
        code, _, err = self.run_cli(["--config", self.cfg_path, "--out", out_csv, "--scenario", "s", "--samples", "1"])
        self.assertEqual(code, 2)
        with open(out_csv) as f:
            self.assertEqual(f.read(), "keep me")
        code, _, _ = self.run_cli(["--config", self.cfg_path, "--out", out_csv, "--scenario", "s", "--samples", "1", "--force"])
        self.assertEqual(code, 0)

    def test_bad_numbers_refused(self):
        out_csv = os.path.join(self.tmp.name, "x.csv")
        base = ["--config", self.cfg_path, "--out", out_csv, "--scenario", "s"]
        for extra in (["--rate", "0"], ["--rate", "5000"], ["--duration", "0"], ["--samples", "0"]):
            code, _, _ = self.run_cli(base + extra)
            self.assertEqual(code, 2, extra)

    def test_attach_failure_exit_3(self):
        def fail(args):
            raise AttachError("nope")

        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = cli.main(["--config", self.cfg_path, "--dump-fields"], attach=fail)
        self.assertEqual(code, 3)
        self.assertIn("nope", err.getvalue())

    def test_process_exits_during_capture(self):
        p = fake_game()
        n = {"i": 0}

        def die():
            n["i"] += 1
            if n["i"] > 6:
                p.alive = False

        p.on_read = die
        out_csv = os.path.join(self.tmp.name, "gone.csv")
        code, _, err = self.run_cli(
            ["--config", self.cfg_path, "--out", out_csv, "--scenario", "s", "--rate", "500", "--duration", "5"], p
        )
        self.assertEqual(code, 0)
        with open(out_csv + ".json") as f:
            self.assertEqual(json.load(f)["capture"]["ended"], "process exited")

    def test_pointer_size_mismatch_exit_2(self):
        doc = dict(CONFIG, pointer_size=8)
        with open(self.cfg_path, "w") as f:
            json.dump(doc, f)
        code, _, err = self.run_cli(["--config", self.cfg_path, "--dump-fields"])
        self.assertEqual(code, 2)
        self.assertIn("pointer_size", err)


if __name__ == "__main__":
    unittest.main()
