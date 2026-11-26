import json
import os
import tempfile
import unittest

from ss2_oracle.config import ConfigError, default_config_path, load_config, parse_config


def doc(fields, **extra):
    d = {"format": 1, "fields": fields}
    d.update(extra)
    return json.dumps(d)


GOOD = {"module": "Fake.dll", "offsets": [0x1000, 8], "type": "f32"}


class ConfigTests(unittest.TestCase):
    def test_minimal_good_config(self):
        cfg = parse_config(doc({"a.b": GOOD}))
        (f,) = cfg.fields
        self.assertEqual((f.name, f.module, f.offsets, f.type), ("a.b", "Fake.dll", (0x1000, 8), "f32"))
        self.assertEqual(cfg.column_names(), ["a.b"])
        self.assertIsNone(cfg.pointer_size)

    def test_hex_strings_and_order_preserved(self):
        cfg = parse_config(
            doc(
                {
                    "z": {"module": "M.dll", "offsets": ["0x10", "4"], "type": "i32"},
                    "a": {"module": "M.dll", "offsets": [1], "type": "u32"},
                }
            )
        )
        self.assertEqual([f.name for f in cfg.fields], ["z", "a"])
        self.assertEqual(cfg.fields[0].offsets, (16, 4))

    def test_every_type_is_accepted(self):
        for t in ("f32", "f64", "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64"):
            parse_config(doc({"x": dict(GOOD, type=t)}))

    def test_bad_type_rejected(self):
        for bad in ("float", "F32", "", None, 5, "i128"):
            with self.assertRaises(ConfigError, msg=repr(bad)):
                parse_config(doc({"x": dict(GOOD, type=bad)}))

    def test_negative_offset_rejected(self):
        for bad in ([-1], [0x10, -4], ["-0x10"], ["-5"]):
            with self.assertRaises(ConfigError, msg=repr(bad)):
                parse_config(doc({"x": dict(GOOD, offsets=bad)}))

    def test_other_bad_offsets_rejected(self):
        for bad in ([], "0x10", [1.5], [True], [None], ["zz"], [2**32], list(range(17))):
            with self.assertRaises(ConfigError, msg=repr(bad)):
                parse_config(doc({"x": dict(GOOD, offsets=bad)}))

    def test_largest_offset_and_longest_chain_accepted(self):
        parse_config(doc({"x": dict(GOOD, offsets=[2**32 - 1])}))
        parse_config(doc({"x": dict(GOOD, offsets=[0] * 16)}))

    def test_missing_or_bad_module_rejected(self):
        for bad in (None, "", 5, "C:\\x\\Fake.dll", "dir/Fake.dll", "a b.dll"):
            spec = dict(GOOD)
            if bad is None:
                del spec["module"]
            else:
                spec["module"] = bad
            with self.assertRaises(ConfigError, msg=repr(bad)):
                parse_config(doc({"x": spec}))

    def test_bad_field_names_rejected(self):
        for bad in ("", "1a", "a b", "a[0]", "a-b", "valid", "t_wall_s", "sample", "jitter_s", "read_s", "n_invalid"):
            with self.assertRaises(ConfigError, msg=repr(bad)):
                parse_config(doc({bad: GOOD}))

    def test_unknown_keys_rejected(self):
        with self.assertRaises(ConfigError):
            parse_config(doc({"x": dict(GOOD, extra=1)}))
        with self.assertRaises(ConfigError):
            parse_config(doc({"x": GOOD}, surprise=1))

    def test_format_and_structure(self):
        for text in ("{", "[]", "{}", json.dumps({"fields": {"x": GOOD}}), json.dumps({"format": 2, "fields": {"x": GOOD}}), doc({})):
            with self.assertRaises(ConfigError, msg=text):
                parse_config(text)

    def test_duplicate_keys_rejected(self):
        text = '{"format": 1, "fields": {"x": {"module": "A.dll", "offsets": [1], "type": "i32"}, "x": {"module": "A.dll", "offsets": [2], "type": "i32"}}}'
        with self.assertRaises(ConfigError):
            parse_config(text)

    def test_pointer_size(self):
        self.assertEqual(parse_config(doc({"x": GOOD}, pointer_size=4)).pointer_size, 4)
        with self.assertRaises(ConfigError):
            parse_config(doc({"x": GOOD}, pointer_size=2))

    def test_scale(self):
        cfg = parse_config(doc({"x": dict(GOOD, scale=0.5)}))
        self.assertEqual(cfg.fields[0].scale, 0.5)
        for bad in ("1", True, float("inf")):
            text = doc({"x": dict(GOOD, scale=bad)}) if bad != float("inf") else '{"format":1,"fields":{"x":{"module":"A.dll","offsets":[1],"type":"f32","scale":Infinity}}}'
            with self.assertRaises(ConfigError, msg=repr(bad)):
                parse_config(text)

    def test_repeat(self):
        cfg = parse_config(doc({"p": dict(GOOD, repeat={"count": 3, "stride": 4, "at": 1})}))
        self.assertEqual(cfg.column_names(), ["p[0]", "p[1]", "p[2]"])
        bads = [
            {"count": 0, "stride": 4, "at": 1},
            {"count": 3, "stride": -4, "at": 1},
            {"count": 3, "stride": 4, "at": 2},
            {"count": 3, "stride": 4},
            {"count": 3, "stride": 4, "at": 1, "x": 1},
            {"count": True, "stride": 4, "at": 1},
            {"count": 99999, "stride": 4, "at": 1},
            "no",
        ]
        for bad in bads:
            with self.assertRaises(ConfigError, msg=repr(bad)):
                parse_config(doc({"p": dict(GOOD, repeat=bad)}))

    def test_repeat_columns_are_distinct_from_plain_names(self):
        cfg = parse_config(doc({"p": dict(GOOD, repeat={"count": 2, "stride": 4, "at": 0}), "q": GOOD}))
        self.assertEqual(cfg.column_names(), ["p[0]", "p[1]", "q"])

    def test_load_config_errors_name_the_file(self):
        with tempfile.TemporaryDirectory() as d:
            missing = os.path.join(d, "nope.json")
            with self.assertRaises(ConfigError) as ctx:
                load_config(missing)
            self.assertIn("nope.json", str(ctx.exception))
            bad = os.path.join(d, "bad.json")
            with open(bad, "w") as f:
                f.write("{")
            with self.assertRaises(ConfigError) as ctx:
                load_config(bad)
            self.assertIn("bad.json", str(ctx.exception))
            good = os.path.join(d, "good.json")
            with open(good, "w") as f:
                f.write(doc({"x": GOOD}))
            self.assertEqual(load_config(good).column_names(), ["x"])

    def test_default_path_uses_localappdata(self):
        old = os.environ.get("LOCALAPPDATA")
        os.environ["LOCALAPPDATA"] = os.path.join("X", "Local")
        try:
            self.assertEqual(default_config_path(), os.path.join("X", "Local", "ss2rs-oracle", "targets.json"))
        finally:
            if old is None:
                del os.environ["LOCALAPPDATA"]
            else:
                os.environ["LOCALAPPDATA"] = old


if __name__ == "__main__":
    unittest.main()
