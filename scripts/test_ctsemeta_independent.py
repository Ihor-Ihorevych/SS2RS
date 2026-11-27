"""Synthetic alignment checks for the independent T01-050d walker."""

from contextlib import redirect_stdout
import io
from pathlib import Path
import struct
import tempfile
import unittest
from zipfile import ZipFile

from tools.re.ctsemeta_independent_verify import WalkError, scan, walk


def words(*values):
    return struct.pack(f"<{len(values)}I", *values)


def string(value):
    return words(len(value)) + value


def typedef(type_id, name, definition):
    return b"DTTY" + words(type_id) + string(name) + definition


def instance(parent, *members):
    return words(2, 5, parent) + b"STMB" + words(len(members)) + b"".join(
        string(str(index).encode()) + words(type_id)
        for index, type_id in enumerate(members)
    )


def sample(value, *, record_type=3, list_element=0):
    table = b"".join([
        typedef(1, b"INDEX", words(1, 13, 0)),
        typedef(2, b"ExampleBase", instance(0xFFFFFFFF, 1)),
        typedef(3, b"Example", instance(2, 4, 5)),
        typedef(4, b"UWORD[2]", words(0, 4, 0)),
        b"ADIM" + words(1, 2),
        typedef(5, b"CStaticArray<T>", words(0, 6, list_element)),
    ])
    return (
        b"CTSEMETA" + words(0x1234ABCD, 8) + string(b"synthetic")
        + b"INFO" + words(0, 0, 0, 7, 1)
        + b"RFIL" + words(0) + b"IDNT" + words(0)
        + b"EXTY" + words(2, 0) + string(b"UWORD") + words(6) + string(b"ExternalClass")
        + b"INTY" + words(5) + table
        + b"EXOB" + words(0) + b"OBTY" + words(1, 40, 3) + b"EDTY" + words(0)
        + b"OBJS" + words(1, 40, record_type) + value
        + b"EDOB" + words(0) + b"METAEND "
    )


class IndependentWalkTests(unittest.TestCase):
    def setUp(self):
        self.value = struct.pack("<HHH", 1, 2, 3) + b"STAR" + words(2)
        self.value += struct.pack("<HH", 4, 5)

    def test_base_first_alias_fixed_array_and_inline_elements(self):
        data = sample(self.value)
        self.assertEqual(walk(data), (1, len(data) - 16))

    def test_bad_marker_is_attributed_to_the_object_type(self):
        with self.assertRaisesRegex(WalkError, r"Example.*STAR.*DCON"):
            walk(sample(self.value.replace(b"STAR", b"DCON")))

    def test_empty_array_does_not_require_an_inline_element_definition(self):
        data = sample(self.value[:10] + words(0), list_element=6)
        self.assertEqual(walk(data), (1, len(data) - 16))

    def test_record_header_must_match_obty(self):
        with self.assertRaisesRegex(WalkError, r"Example.*OBTY"):
            walk(sample(self.value, record_type=2))

    def test_edob_must_follow_the_final_value_without_padding(self):
        with self.assertRaisesRegex(WalkError, r"Example.*EDOB"):
            walk(sample(self.value + b"\x00"))

    def test_partial_install_does_not_pass_the_retail_proof(self):
        with tempfile.TemporaryDirectory() as directory:
            install = Path(directory)
            with ZipFile(install / "sample.gro", "w") as archive:
                archive.writestr("sample.bin", sample(self.value))
            with redirect_stdout(io.StringIO()) as report:
                self.assertEqual(scan(install), 1)
            self.assertIn("retail coverage incomplete", report.getvalue())


if __name__ == "__main__":
    unittest.main()
