import math
import struct
import unittest

from fakes import FakeClock, FakeProcess
from ss2_oracle.backend import ProcessGone
from ss2_oracle.config import parse_config
from ss2_oracle.reader import FieldReader, format_number, missing_modules

import json


def cfg(fields, **extra):
    d = {"format": 1, "fields": fields}
    d.update(extra)
    return parse_config(json.dumps(d))


def field(offsets, ftype="f32", module="Fake.dll", **extra):
    d = {"module": module, "offsets": offsets, "type": ftype}
    d.update(extra)
    return d


BASE = 0x400000  # a fake base, not a real address


class FormatTests(unittest.TestCase):
    def test_f32_is_shortest_roundtrip(self):
        for value in (0.1, 100.0, 1.5, -2.25, 3.4028234663852886e38, 1e-45):
            as_f32 = struct.unpack("<f", struct.pack("<f", value))[0]
            text = format_number(as_f32, "f32")
            self.assertEqual(struct.pack("<f", float(text)), struct.pack("<f", as_f32), text)
        self.assertEqual(format_number(struct.unpack("<f", struct.pack("<f", 0.1))[0], "f32"), "0.1")
        self.assertEqual(format_number(100.0, "f32"), "100.0")

    def test_f32_needing_many_digits_is_exact_but_not_longer_than_needed(self):
        def f32(x):
            return struct.unpack("<f", struct.pack("<f", x))[0]

        self.assertEqual(format_number(f32(1.2345678), "f32"), "1.2345678")
        self.assertEqual(format_number(f32(3.4028235e38), "f32"), "3.4028235e+38")
        self.assertEqual(format_number(f32(0.3), "f32"), "0.3")

    def test_f64_uses_repr(self):
        self.assertEqual(format_number(0.1, "f64"), "0.1")
        self.assertEqual(format_number(1 / 3, "f64"), repr(1 / 3))

    def test_ints_exact(self):
        self.assertEqual(format_number(-5, "i32"), "-5")
        self.assertEqual(format_number(2**63 - 1, "i64"), str(2**63 - 1))
        self.assertEqual(format_number(2**64 - 1, "u64"), str(2**64 - 1))

    def test_non_finite(self):
        self.assertEqual(format_number(float("nan"), "f32"), "nan")
        self.assertEqual(format_number(float("inf"), "f32"), "inf")
        self.assertEqual(format_number(float("-inf"), "f64"), "-inf")

    def test_scale_gives_float(self):
        self.assertEqual(format_number(3, "i64", 0.5), "1.5")
        self.assertEqual(format_number(1500000, "i64", 1e-6), repr(1500000 * 1e-6))


class DecodeTests(unittest.TestCase):
    def setUp(self):
        self.p = FakeProcess(4)
        self.p.add_module("Fake.dll", BASE)

    def read_one(self, ftype, raw):
        self.p.poke(BASE + 0x10, raw)
        r = FieldReader(self.p, cfg({"v": field([0x10], ftype)}))
        return r.sample().cells[0]

    def test_all_types_little_endian(self):
        self.assertEqual(self.read_one("i8", b"\xff"), "-1")
        self.assertEqual(self.read_one("u8", b"\xff"), "255")
        self.assertEqual(self.read_one("i16", b"\xfe\xff"), "-2")
        self.assertEqual(self.read_one("u16", b"\xfe\xff"), "65534")
        self.assertEqual(self.read_one("i32", struct.pack("<i", -123456)), "-123456")
        self.assertEqual(self.read_one("u32", struct.pack("<I", 4000000000)), "4000000000")
        self.assertEqual(self.read_one("i64", struct.pack("<q", -(2**40))), str(-(2**40)))
        self.assertEqual(self.read_one("u64", struct.pack("<Q", 2**63)), str(2**63))
        self.assertEqual(self.read_one("f32", struct.pack("<f", 12.5)), "12.5")
        self.assertEqual(self.read_one("f64", struct.pack("<d", -0.125)), "-0.125")

    def test_nan_float_is_written_not_dropped(self):
        self.assertEqual(self.read_one("f32", struct.pack("<f", math.nan)), "nan")

    def test_read_at_end_of_region_boundary(self):
        # a value whose last byte is the last byte of the module: readable
        self.p.poke(BASE + 0x10000 - 4, struct.pack("<I", 7))
        r = FieldReader(self.p, cfg({"v": field([0x10000 - 4], "u32")}))
        self.assertEqual(r.sample().cells, ["7"])
        # one byte further: not readable, not a crash
        r = FieldReader(self.p, cfg({"v": field([0x10000 - 3], "u32")}))
        s = r.sample()
        self.assertEqual((s.cells, s.n_invalid, s.valid), ([""], 1, False))


class ChainTests(unittest.TestCase):
    def setUp(self):
        self.p = FakeProcess(4)
        self.p.add_module("Fake.dll", BASE)
        self.p.add_region(0x900000, 0x1000)
        self.p.add_region(0xA00000, 0x1000)

    def test_global_value_no_dereference(self):
        self.p.poke_f32(BASE + 0x20, 1.5)
        r = FieldReader(self.p, cfg({"x": field([0x20])}))
        self.assertEqual(r.sample().cells, ["1.5"])

    def test_one_and_two_dereferences(self):
        self.p.poke_ptr(BASE + 0x100, 0x900000)  # global holds a pointer
        self.p.poke_ptr(0x900000 + 0x8, 0xA00000)  # which points at a struct holding another
        self.p.poke_f32(0xA00000 + 0x30, -7.25)
        self.p.poke_f32(0x900000 + 0x40, 2.5)
        r = FieldReader(
            self.p,
            cfg({"deep": field([0x100, 0x8, 0x30]), "shallow": field([0x100, 0x40])}),
        )
        s = r.sample()
        self.assertEqual(s.cells, ["-7.25", "2.5"])
        self.assertTrue(s.valid)

    def test_null_pointer_gives_empty_cell_not_crash(self):
        self.p.poke_ptr(BASE + 0x100, 0)
        r = FieldReader(self.p, cfg({"a": field([0x100, 0x8]), "b": field([0x20], "u32")}))
        self.p.poke(BASE + 0x20, struct.pack("<I", 9))
        s = r.sample()
        self.assertEqual(s.cells, ["", "9"])
        self.assertEqual((s.n_invalid, s.valid), (1, False))

    def test_null_pointer_is_not_followed_even_if_low_memory_is_readable(self):
        self.p.add_region(0, 0x1000)  # something readable at address zero
        self.p.poke_f32(0x8, 55.0)
        self.p.poke_ptr(BASE + 0x100, 0)
        r = FieldReader(self.p, cfg({"a": field([0x100, 0x8])}))
        self.assertEqual(r.sample().cells, [""])

    def test_null_in_the_middle_of_a_chain(self):
        self.p.poke_ptr(BASE + 0x100, 0x900000)
        self.p.poke_ptr(0x900000 + 0x8, 0)
        r = FieldReader(self.p, cfg({"a": field([0x100, 0x8, 0x30])}))
        self.assertEqual(r.sample().cells, [""])

    def test_wild_pointer_is_invalid_not_error(self):
        self.p.poke_ptr(BASE + 0x100, 0xDEAD0000)  # not mapped in the fake
        r = FieldReader(self.p, cfg({"a": field([0x100, 0x8])}))
        self.assertEqual(r.sample().cells, [""])

    def test_pointer_plus_offset_overflowing_32_bits_is_invalid(self):
        self.p.poke_ptr(BASE + 0x100, 0xFFFFFFF0)
        r = FieldReader(self.p, cfg({"a": field([0x100, 0x100])}))
        self.assertEqual(r.sample().cells, [""])

    def test_value_recovers_when_pointer_becomes_valid(self):
        self.p.poke_ptr(BASE + 0x100, 0)
        r = FieldReader(self.p, cfg({"a": field([0x100, 0x8])}))
        self.assertEqual(r.sample().cells, [""])
        self.p.poke_ptr(BASE + 0x100, 0x900000)
        self.p.poke_f32(0x900008, 3.0)
        self.assertEqual(r.sample().cells, ["3.0"])

    def test_pointer_width_8(self):
        p = FakeProcess(8)
        p.add_module("Fake.dll", 0x7FF600000000)
        p.add_region(0x1_0000_0000, 0x100)
        p.poke_ptr(0x7FF600000000 + 0x10, 0x1_0000_0000)
        p.poke(0x1_0000_0000 + 4, struct.pack("<I", 77))
        r = FieldReader(p, cfg({"v": field([0x10, 4], "u32")}))
        self.assertEqual(r.sample().cells, ["77"])

    def test_config_pointer_size_mismatch_refused(self):
        with self.assertRaises(ValueError):
            FieldReader(self.p, cfg({"x": field([0])}, pointer_size=8))
        FieldReader(self.p, cfg({"x": field([0])}, pointer_size=4))

    def test_sample_reads_a_shared_pointer_once(self):
        self.p.poke_ptr(BASE + 0x100, 0x900000)
        r = FieldReader(self.p, cfg({"a": field([0x100, 0x0]), "b": field([0x100, 0x4]), "c": field([0x100, 0x8])}))
        r.sample()
        pointer_reads = [x for x in self.p.reads if x == (BASE + 0x100, 4)]
        self.assertEqual(len(pointer_reads), 1)

    def test_repeat_steps_an_offset(self):
        # an array of 3 entity pointers in a global, each pointing at a struct with a float at +0x10
        for i in range(3):
            self.p.poke_ptr(BASE + 0x200 + 4 * i, 0x900000 + 0x100 * i)
            self.p.poke_f32(0x900000 + 0x100 * i + 0x10, float(i) + 0.5)
        self.p.poke_ptr(BASE + 0x200 + 4 * 3, 0)  # a fourth, null
        r = FieldReader(self.p, cfg({"e": field([0x200, 0x10], repeat={"count": 4, "stride": 4, "at": 0})}))
        self.assertEqual(r.sample().cells, ["0.5", "1.5", "2.5", ""])
        self.assertEqual([c for c, _ in r.describe()], ["e[0]", "e[1]", "e[2]", "e[3]"])

    def test_repeat_on_the_last_offset(self):
        self.p.poke_ptr(BASE + 0x100, 0x900000)
        for i in range(2):
            self.p.poke_f32(0x900000 + 0x20 + 4 * i, 10.0 * (i + 1))
        r = FieldReader(self.p, cfg({"v": field([0x100, 0x20], repeat={"count": 2, "stride": 4, "at": 1})}))
        self.assertEqual(r.sample().cells, ["10.0", "20.0"])

    def test_scale_applies(self):
        self.p.poke(BASE + 0x20, struct.pack("<q", 2500000))
        r = FieldReader(self.p, cfg({"t": field([0x20], "i64", scale=1e-6)}))
        self.assertEqual(r.sample().cells, [repr(2500000 * 1e-6)])


class ModuleTests(unittest.TestCase):
    def test_module_base_is_found_at_run_time_not_assumed(self):
        a = FakeProcess(4)
        a.add_module("Fake.dll", 0x10000000)
        a.poke(0x10000000 + 0x40, struct.pack("<I", 1))
        b = FakeProcess(4)
        b.add_module("Fake.dll", 0x20000000)  # relocated
        b.poke(0x20000000 + 0x40, struct.pack("<I", 2))
        c = cfg({"v": field([0x40], "u32")})
        self.assertEqual(FieldReader(a, c).sample().cells, ["1"])
        self.assertEqual(FieldReader(b, c).sample().cells, ["2"])

    def test_module_name_case_insensitive(self):
        p = FakeProcess(4)
        p.add_module("ENGINE.DLL", BASE)
        p.poke(BASE, struct.pack("<I", 5))
        self.assertEqual(FieldReader(p, cfg({"v": field([0], "u32", module="engine.dll")})).sample().cells, ["5"])

    def test_missing_module_is_invalid_until_it_loads(self):
        clock = FakeClock()
        p = FakeProcess(4)
        p.add_module("Other.dll", 0x300000)
        r = FieldReader(p, cfg({"v": field([0], "u32")}), clock=clock, module_retry_s=0.5)
        self.assertEqual(r.sample().cells, [""])
        # the module appears, but the retry interval has not passed: still empty, and no new lookup
        p.add_module("Fake.dll", BASE)
        p.poke(BASE, struct.pack("<I", 3))
        calls = p.module_list_calls
        self.assertEqual(r.sample().cells, [""])
        self.assertEqual(p.module_list_calls, calls)
        clock.advance(0.6)
        self.assertEqual(r.sample().cells, ["3"])

    def test_missing_modules_helper(self):
        p = FakeProcess(4)
        p.add_module("A.dll", 0x1000)
        c = cfg({"x": field([0], module="A.dll"), "y": field([0], module="B.dll"), "z": field([4], module="B.dll")})
        self.assertEqual(missing_modules(c, p.list_modules()), ["B.dll"])


class ShortReadProcess(FakeProcess):
    """A backend that sometimes hands back fewer bytes than asked for."""

    def read(self, address, size):
        return super().read(address, size)[:-1]


class ReadErrorWhenDead(FakeProcess):
    """A backend that reports a dead process as a plain read error (the reader must still notice)."""

    def read(self, address, size):
        if not self.alive:
            from ss2_oracle.backend import ReadError

            raise ReadError("gone")
        return super().read(address, size)


class WidthTests(unittest.TestCase):
    def test_chain_address_beyond_the_pointer_width_is_invalid(self):
        p = FakeProcess(4)
        p.add_module("Fake.dll", 0xFFFF0000, 0x10)
        p.add_region(0x1_0000_0000, 0x100)  # readable, but not addressable by a 32-bit process
        # a pointer at module+8 holds 0xFFFFFFF0; adding 0x20 passes 2^32
        p.poke_ptr(0xFFFF0008, 0xFFFFFFF0)
        p.add_region(0xFFFFFFF0, 0x10)
        r = FieldReader(p, cfg({"v": field([0x8, 0x20], "u32")}))
        self.assertEqual(r.sample().cells, [""])

    def test_middle_step_beyond_the_pointer_width_is_not_dereferenced(self):
        p = FakeProcess(4)
        p.add_module("Fake.dll", 0xFFFF0000, 0x1000)
        p.add_region(0x1_0000_0000, 0x100)
        p.poke_ptr(0xFFFF0008, 0xFFFFFFF0)  # first pointer: module + 8
        p.poke_ptr(0x1_0000_0010, 0xFFFF0100)  # what a 64-bit-wrapping read would find
        p.poke(0xFFFF0100 + 4, struct.pack("<I", 99))
        r = FieldReader(p, cfg({"v": field([0x8, 0x20, 0x4], "u32")}))
        self.assertEqual(r.sample().cells, [""])

    def test_global_beyond_the_pointer_width_is_invalid(self):
        p = FakeProcess(4)
        p.add_module("Fake.dll", 0xFFFF0000, 0x10)
        p.add_region(0x1_0000_0000, 0x100)
        r = FieldReader(p, cfg({"v": field([0x10000], "u32")}))  # 0xFFFF0000 + 0x10000 = 2^32
        self.assertEqual(r.sample().cells, [""])

    def test_short_read_is_invalid_not_a_crash(self):
        p = ShortReadProcess(4)
        p.add_module("Fake.dll", BASE)
        r = FieldReader(p, cfg({"v": field([0x10], "u32")}))
        self.assertEqual(r.sample().cells, [""])


class ExitTests(unittest.TestCase):
    def test_exit_detected_when_the_backend_reports_it_as_a_read_error(self):
        p = ReadErrorWhenDead(4)
        p.add_module("Fake.dll", BASE)
        r = FieldReader(p, cfg({"v": field([0x10], "u32")}))
        self.assertEqual(r.sample().cells, ["0"])
        p.alive = False
        with self.assertRaises(ProcessGone):
            r.sample()

    def test_process_exit_raises_process_gone(self):
        p = FakeProcess(4)
        p.add_module("Fake.dll", BASE)
        r = FieldReader(p, cfg({"v": field([0], "u32")}))
        r.sample()
        p.alive = False
        with self.assertRaises(ProcessGone):
            r.sample()

    def test_failed_read_of_live_process_is_not_exit(self):
        p = FakeProcess(4)
        p.add_module("Fake.dll", BASE)
        r = FieldReader(p, cfg({"v": field([0x20000], "u32")}))  # outside the module
        self.assertEqual(r.sample().cells, [""])


if __name__ == "__main__":
    unittest.main()
