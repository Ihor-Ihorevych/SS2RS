import csv
import io
import json
import struct
import unittest

from fakes import FakeClock, FakeProcess
from ss2_oracle.capture import FIXED_COLUMNS, build_sidecar, run_capture
from ss2_oracle.config import parse_config
from ss2_oracle.reader import FieldReader

BASE = 0x400000


def make(read_cost=0.0, fields=None):
    clock = FakeClock()
    p = FakeProcess(4)
    p.add_module("Fake.dll", BASE)
    p.poke(BASE + 0x10, struct.pack("<I", 1))
    p.on_read = (lambda: clock.advance(read_cost)) if read_cost else None
    fields = fields or {
        "t": {"module": "Fake.dll", "offsets": [0x10], "type": "u32"},
        "ptr": {"module": "Fake.dll", "offsets": [0x20, 4], "type": "f32"},
    }
    cfg = parse_config(json.dumps({"format": 1, "fields": fields}))
    return clock, p, FieldReader(p, cfg, clock=clock)


def rows(text):
    return list(csv.reader(io.StringIO(text)))


class CaptureTests(unittest.TestCase):
    def run_it(self, reader, clock, **kw):
        out = io.StringIO()
        stats = run_capture(reader, out, clock=clock, sleep=clock.sleep, **kw)
        return out.getvalue(), stats

    def test_header_and_row_shape(self):
        clock, p, reader = make()
        text, stats = self.run_it(reader, clock, rate_hz=30, max_samples=3)
        r = rows(text)
        self.assertEqual(r[0], FIXED_COLUMNS + ["t", "ptr"])
        self.assertEqual(len(r), 4)
        self.assertTrue(all(len(row) == len(r[0]) for row in r))
        self.assertTrue(text.endswith("\n") and "\r" not in text)
        self.assertEqual(stats.samples, 3)
        self.assertEqual(stats.ended, "sample count reached")

    def test_first_sample_is_at_zero_and_spacing_is_the_period(self):
        clock, p, reader = make()
        text, _ = self.run_it(reader, clock, rate_hz=30, max_samples=5)
        times = [float(row[1]) for row in rows(text)[1:]]
        self.assertEqual(times[0], 0.0)
        for a, b in zip(times, times[1:]):
            self.assertAlmostEqual(b - a, 1 / 30, places=5)

    def test_duration_stops_before_the_first_slot_past_it(self):
        clock, p, reader = make()
        text, stats = self.run_it(reader, clock, rate_hz=10, duration_s=1.0)
        self.assertEqual(stats.samples, 10)  # slots 0.0 .. 0.9
        self.assertEqual(stats.ended, "duration reached")

    def test_values_and_validity_columns(self):
        clock, p, reader = make()
        p.poke(BASE + 0x20, struct.pack("<I", 0))  # null pointer: the second column is empty
        text, stats = self.run_it(reader, clock, rate_hz=30, max_samples=2)
        row = rows(text)[1]
        self.assertEqual(row[4:], ["0", "1", "1", ""])  # valid=0, n_invalid=1, t=1, ptr empty
        self.assertEqual(stats.invalid_rows, 2)

    def test_a_row_recovers_when_the_pointer_appears_mid_capture(self):
        clock, p, reader = make()
        p.add_region(0x900000, 0x100)
        state = {"n": 0}

        def tick():
            pass

        text_parts = []
        out = io.StringIO()

        def stop():
            state["n"] += 1
            if state["n"] == 3:  # before the third sample the level "finishes loading"
                p.poke(BASE + 0x20, struct.pack("<I", 0x900000))
                p.poke(0x900004, struct.pack("<f", 2.5))
            return False

        run_capture(reader, out, 30, max_samples=5, clock=clock, sleep=clock.sleep, stop=stop)
        r = rows(out.getvalue())[1:]
        self.assertEqual([row[4] for row in r], ["0", "0", "1", "1", "1"])
        self.assertEqual([row[7] for row in r], ["", "", "2.5", "2.5", "2.5"])

    def test_jitter_zero_on_a_perfect_clock(self):
        clock, p, reader = make()
        text, stats = self.run_it(reader, clock, rate_hz=30, max_samples=10)
        for row in rows(text)[1:]:
            self.assertAlmostEqual(float(row[2]), 0.0, places=6)
        self.assertEqual(stats.missed_slots, 0)

    def test_late_sample_records_jitter_and_slow_reads_miss_slots(self):
        # reading costs 25 ms per read; two reads per sample (pointer + value read) -> slow
        clock, p, reader = make(read_cost=0.025)
        text, stats = self.run_it(reader, clock, rate_hz=30, max_samples=4)
        r = rows(text)[1:]
        self.assertGreater(stats.missed_slots, 0)
        self.assertEqual([int(x[0]) for x in r], [0, 1, 2, 3])
        # a sample is never started before its slot (jitter is never negative) and a slow
        # sample does not make the loop start the next one early to catch up: with an exact clock
        # every sample starts exactly on its own slot
        self.assertTrue(all(float(x[2]) >= -1e-9 for x in r))
        self.assertTrue(all(float(x[2]) < 1e-6 for x in r), [x[2] for x in r])
        # the real start times are in the file, so the true spacing is recoverable
        times = [float(x[1]) for x in r]
        self.assertGreater(times[1] - times[0], 1 / 30 + 0.01)
        self.assertGreater(float(r[0][3]), 0.04)  # read_s is the time the reads took

    def test_externally_delayed_start_shows_as_positive_jitter(self):
        clock, p, reader = make()
        out = io.StringIO()
        calls = {"n": 0}

        def slow_sleep(seconds):
            calls["n"] += 1
            clock.sleep(seconds + (0.007 if calls["n"] == 3 else 0.0))  # the OS oversleeps once

        run_capture(reader, out, 30, max_samples=5, clock=clock, sleep=slow_sleep)
        jitter = [float(x[2]) for x in rows(out.getvalue())[1:]]
        self.assertAlmostEqual(max(jitter), 0.007, places=5)
        self.assertEqual(sum(1 for j in jitter if j > 1e-6), 1)

    def test_process_exit_ends_cleanly_with_the_rows_so_far(self):
        clock, p, reader = make()
        count = {"n": 0}

        def die():
            count["n"] += 1
            if count["n"] >= 5:
                p.alive = False

        p.on_read = die
        text, stats = self.run_it(reader, clock, rate_hz=30)
        self.assertEqual(stats.ended, "process exited")
        self.assertGreaterEqual(stats.samples, 1)
        self.assertEqual(len(rows(text)) - 1, stats.samples)

    def test_stop_callback(self):
        clock, p, reader = make()
        n = {"i": 0}

        def stop():
            n["i"] += 1
            return n["i"] > 2

        text, stats = self.run_it(reader, clock, rate_hz=30, stop=stop)
        self.assertEqual((stats.samples, stats.ended), (2, "stopped"))

    def test_bad_rates_refused(self):
        clock, p, reader = make()
        for bad in (0, -1, float("inf"), float("nan")):
            with self.assertRaises(ValueError):
                run_capture(reader, io.StringIO(), bad, max_samples=1, clock=clock, sleep=clock.sleep)

    def test_summary_numbers(self):
        clock, p, reader = make()
        _, stats = self.run_it(reader, clock, rate_hz=20, max_samples=11)
        s = stats.summary()
        self.assertEqual(s["samples"], 11)
        self.assertAlmostEqual(s["interval_s"]["mean"], 0.05, places=6)
        self.assertAlmostEqual(s["interval_s"]["min"], 0.05, places=6)
        self.assertEqual(s["jitter_s"]["max"] < 1e-6, True)

    def test_jitter_statistics_match_the_rows(self):
        clock, p, reader = make()
        out = io.StringIO()
        calls = {"n": 0}

        def slow_sleep(seconds):
            calls["n"] += 1
            clock.sleep(seconds + (0.004 if calls["n"] % 2 == 0 else 0.0))

        stats = run_capture(reader, out, 30, max_samples=9, clock=clock, sleep=slow_sleep)
        jitter = [float(x[2]) for x in rows(out.getvalue())[1:]]
        s = stats.summary()["jitter_s"]
        self.assertAlmostEqual(s["max"], max(jitter), places=5)
        self.assertAlmostEqual(s["mean"], sum(jitter) / len(jitter), places=5)
        self.assertAlmostEqual(s["p95"], sorted(jitter)[-1], places=5)

    def test_empty_summary_is_json_safe(self):
        clock, p, reader = make()
        _, stats = self.run_it(reader, clock, rate_hz=20, max_samples=1)
        json.dumps(stats.summary())

    def test_sidecar_has_no_offsets_or_addresses(self):
        clock, p, reader = make()
        _, stats = self.run_it(reader, clock, rate_hz=20, max_samples=2)
        side = build_sidecar("scn", 20.0, "Sam2.exe", 4242, 4, reader.describe(), stats, {"id": "abc"}, "deadbeef", "2026-01-01T00:00:00Z")
        text = json.dumps(side)
        self.assertEqual(side["scenario"], "scn")
        self.assertEqual(side["columns"], [{"name": "t", "type": "u32"}, {"name": "ptr", "type": "f32"}])
        self.assertNotIn("offsets", text)
        self.assertNotIn(hex(BASE)[2:], text.lower())
        self.assertEqual(side["rate_hz"], 20.0)


if __name__ == "__main__":
    unittest.main()
