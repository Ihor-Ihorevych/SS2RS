import json
import struct
import unittest

from fakes import FakeProcess
from ss2_oracle.config import ConfigError, parse_config
from ss2_oracle.reader import FieldReader

TABLE = {"module": "Fake.dll", "table": "0x100", "count": "0x104", "record_size": 16, "generation_at": 0, "object_at": 8}


def cfg(fields, table=TABLE, **extra):
    doc = {"format": 1, "fields": fields}
    if table is not None:
        doc["handle_table"] = table
    doc.update(extra)
    return parse_config(json.dumps(doc))


def handle_field(chain, ftype="u32"):
    return {"module": "Fake.dll", "offsets": chain, "type": ftype}


def h(offset):
    return {"offset": offset, "handle": True}


BASE = 0x400000
RECORDS = 0xB00000
OBJ_A = 0xC00000  # an object whose +0x20 holds a handle to object B
OBJ_B = 0xC10000  # holds the value


def world(count=8):
    p = FakeProcess(4)
    p.add_module("Fake.dll", BASE)
    p.add_region(RECORDS, 16 * 8)  # the memory is there whatever the count variable says
    p.add_region(OBJ_A, 0x100)
    p.add_region(OBJ_B, 0x100)
    p.poke_ptr(BASE + 0x100, RECORDS)
    p.poke(BASE + 0x104, struct.pack("<I", count))
    # handle (gen 5, index 3) -> object A; handle (gen 7, index 4) -> object B
    p.poke(RECORDS + 3 * 16 + 0, struct.pack("<I", 5))
    p.poke(RECORDS + 3 * 16 + 8, struct.pack("<I", OBJ_A))
    p.poke(RECORDS + 4 * 16 + 0, struct.pack("<I", 7))
    p.poke(RECORDS + 4 * 16 + 8, struct.pack("<I", OBJ_B))
    # a global holds the handle of A; A+0x20 holds the handle of B; B+0x30 holds a float
    p.poke(BASE + 0x200, struct.pack("<I", (5 << 24) | 3))
    p.poke(OBJ_A + 0x20, struct.pack("<I", (7 << 24) | 4))
    p.poke_f32(OBJ_B + 0x30, 100.0)
    return p


class HandleConfigTests(unittest.TestCase):
    def test_parse(self):
        c = cfg({"x": handle_field([0x200, h(0x20), h(0x30)], "f32")})
        self.assertEqual(c.fields[0].handles, (False, True, True))
        self.assertEqual(c.fields[0].offsets, (0x200, 0x20, 0x30))
        self.assertEqual(c.handle_table.index_bits, 24)
        self.assertEqual((c.handle_table.table, c.handle_table.count), (0x100, 0x104))

    def test_handle_step_needs_a_table(self):
        with self.assertRaises(ConfigError):
            cfg({"x": handle_field([0x200, h(0x20)])}, table=None)

    def test_first_step_cannot_be_a_handle(self):
        with self.assertRaises(ConfigError):
            cfg({"x": handle_field([h(0x200), 4])})

    def test_bad_step_objects(self):
        for bad in ({"offset": 4}, {"offset": 4, "handle": False}, {"offset": 4, "handle": True, "x": 1}, {"offset": -4, "handle": True}, {"handle": True}):
            with self.assertRaises(ConfigError, msg=repr(bad)):
                cfg({"x": handle_field([0x200, bad])})

    def test_generation_and_object_must_fit_in_the_record(self):
        for change in ({"generation_at": 12}, {"object_at": 12}):  # 12 + 4 = 16: fits
            cfg({"x": handle_field([0x200, h(0x20)])}, table=dict(TABLE, **change))
        for change in ({"generation_at": 13}, {"object_at": 13}):  # 13 + 4 = 17: does not
            with self.assertRaises(ConfigError):
                cfg({"x": handle_field([0x200, h(0x20)])}, table=dict(TABLE, **change))

    def test_bad_tables(self):
        for change in (
            {"module": "a/b.dll"},
            {"record_size": 2},
            {"record_size": 100000},
            {"object_at": 14},
            {"generation_at": 13},
            {"index_bits": 0},
            {"index_bits": 32},
            {"index_bits": True},
            {"extra": 1},
            {"table": -1},
        ):
            table = dict(TABLE, **change)
            with self.assertRaises(ConfigError, msg=repr(change)):
                cfg({"x": handle_field([0x200, h(0x20)])}, table=table)
        for key in ("module", "table", "count", "record_size", "generation_at", "object_at"):
            table = {k: v for k, v in TABLE.items() if k != key}
            with self.assertRaises(ConfigError, msg=key):
                cfg({"x": handle_field([0x200, h(0x20)])}, table=table)
        with self.assertRaises(ConfigError):
            cfg({"x": handle_field([0x200, h(0x20)])}, table=[1])


class HandleReadTests(unittest.TestCase):
    def read(self, p, chain=None, ftype="f32"):
        chain = chain or [0x200, h(0x20), h(0x30)]
        return FieldReader(p, cfg({"x": handle_field(chain, ftype)})).sample().cells

    def test_two_handle_hops(self):
        self.assertEqual(self.read(world()), ["100.0"])

    def test_handle_then_pointer(self):
        p = world()
        p.poke_ptr(OBJ_B + 0x40, 0xC20000)
        p.add_region(0xC20000, 0x20)
        p.poke(0xC20008, struct.pack("<i", -3))
        self.assertEqual(self.read(p, [0x200, h(0x20), h(0x40), 8], "i32"), ["-3"])

    def test_null_handle_is_invalid(self):
        p = world()
        p.poke(BASE + 0x200, struct.pack("<I", 0))
        self.assertEqual(self.read(p), [""])

    def test_stale_generation_is_invalid(self):
        p = world()
        p.poke(BASE + 0x200, struct.pack("<I", (6 << 24) | 3))  # the record says generation 5
        self.assertEqual(self.read(p), [""])

    def test_generation_is_compared_exactly(self):
        p = world()
        p.poke(BASE + 0x200, struct.pack("<I", (4 << 24) | 3))
        self.assertEqual(self.read(p), [""])

    def test_index_out_of_range_is_invalid(self):
        # the chain's second hop is index 4: a table of 4 records has no such record, 5 has
        self.assertEqual(self.read(world(count=4)), [""])
        self.assertEqual(self.read(world(count=5)), ["100.0"])
        p = world(count=5)
        p.poke(BASE + 0x200, struct.pack("<I", (5 << 24) | 5))  # the first hop: index 5 of 5
        self.assertEqual(self.read(p), [""])

    def test_handle_zero_is_never_resolved_even_if_record_zero_is_valid(self):
        p = world()
        p.poke(RECORDS + 0, struct.pack("<I", 0))  # generation 0
        p.poke(RECORDS + 8, struct.pack("<I", OBJ_A))  # a live object in record 0
        p.poke(BASE + 0x200, struct.pack("<I", 0))
        self.assertEqual(self.read(p), [""])

    def test_index_uses_all_24_bits(self):
        p = FakeProcess(4)
        p.add_module("Fake.dll", BASE)
        big = 0x10003  # above 16 bits
        p.add_region(RECORDS, 16 * (big + 2))
        p.add_region(0xD00000, 0x100)
        p.poke_ptr(BASE + 0x100, RECORDS)
        p.poke(BASE + 0x104, struct.pack("<I", big + 2))
        p.poke(RECORDS + big * 16 + 0, struct.pack("<I", 5))
        p.poke(RECORDS + big * 16 + 8, struct.pack("<I", 0xD00000))
        p.poke(BASE + 0x200, struct.pack("<I", (5 << 24) | big))
        p.poke_f32(0xD00000 + 0x30, 4.5)
        c = cfg({"x": handle_field([0x200, h(0x30)], "f32")})
        self.assertEqual(FieldReader(p, c).sample().cells, ["4.5"])

    def test_null_object_in_record_is_invalid(self):
        p = world()
        p.poke(RECORDS + 3 * 16 + 8, struct.pack("<I", 0))
        self.assertEqual(self.read(p), [""])

    def test_no_table_yet_is_invalid(self):
        p = world()
        p.poke_ptr(BASE + 0x100, 0)
        p.add_region(0, 0x1000)  # readable memory at zero must not be mistaken for a table
        p.poke(3 * 16 + 0, struct.pack("<I", 5))
        p.poke(3 * 16 + 8, struct.pack("<I", OBJ_A))
        p.poke(4 * 16 + 0, struct.pack("<I", 7))
        p.poke(4 * 16 + 8, struct.pack("<I", OBJ_B))
        self.assertEqual(self.read(p), [""])

    def test_other_record_layout(self):
        # 24-byte records, generation at +4, object at +12
        p = FakeProcess(4)
        p.add_module("Fake.dll", BASE)
        p.add_region(RECORDS, 24 * 8)
        p.add_region(OBJ_A, 0x100)
        p.poke_ptr(BASE + 0x100, RECORDS)
        p.poke(BASE + 0x104, struct.pack("<I", 8))
        p.poke(RECORDS + 3 * 24 + 4, struct.pack("<I", 5))
        p.poke(RECORDS + 3 * 24 + 12, struct.pack("<I", OBJ_A))
        p.poke(BASE + 0x200, struct.pack("<I", (5 << 24) | 3))
        p.poke_f32(OBJ_A + 0x30, 7.5)
        table = dict(TABLE, record_size=24, generation_at=4, object_at=12)
        c = cfg({"x": handle_field([0x200, h(0x30)], "f32")}, table=table)
        self.assertEqual(FieldReader(p, c).sample().cells, ["7.5"])

    def test_table_module_missing_is_invalid(self):
        p = FakeProcess(4)
        p.add_module("Other.dll", 0x500000)
        cfg_ = cfg({"x": {"module": "Other.dll", "offsets": [0x10, h(0x20)], "type": "f32"}})
        p.poke(0x500010, struct.pack("<I", 0x1_0003))
        self.assertEqual(FieldReader(p, cfg_).sample().cells, [""])

    def test_index_bits_is_configurable(self):
        p = world()
        table = dict(TABLE, index_bits=16)
        p.poke(BASE + 0x200, struct.pack("<I", (5 << 16) | 3))
        p.poke(OBJ_A + 0x20, struct.pack("<I", (7 << 16) | 4))
        c = cfg({"x": handle_field([0x200, h(0x20), h(0x30)], "f32")}, table=table)
        self.assertEqual(FieldReader(p, c).sample().cells, ["100.0"])

    def test_repeat_over_handle_slots(self):
        # three slots at A+0x20, +0x24, +0x28; slots 0 and 2 hold handles, slot 1 is empty
        p = world()
        p.poke(OBJ_A + 0x24, struct.pack("<I", 0))
        p.poke(OBJ_A + 0x28, struct.pack("<I", (7 << 24) | 4))
        field = handle_field([0x200, h(0x20), h(0x30)], "f32")
        field["repeat"] = {"count": 3, "stride": 4, "at": 1}
        self.assertEqual(FieldReader(p, cfg({"x": field})).sample().cells, ["100.0", "", "100.0"])

    def test_handle_reads_are_cached_within_a_sample(self):
        p = world()
        fields = {"a": handle_field([0x200, h(0x20), h(0x30)], "f32"), "b": handle_field([0x200, h(0x20), h(0x34)], "f32")}
        FieldReader(p, cfg(fields)).sample()
        table_reads = [r for r in p.reads if r == (BASE + 0x100, 4)]
        self.assertEqual(len(table_reads), 1)  # the table address is read once per sample, not per hop or field


if __name__ == "__main__":
    unittest.main()
