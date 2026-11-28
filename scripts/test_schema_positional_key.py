"""Unit test for the by-position re-keying fix (docs/re/ctsemeta-types.md invariant 4, T01-426).

A class's own `STMB` member list is not guaranteed to have distinct names (`CBloomProperties`
declares a member literally named `"21"` twice, and five more classes do the same for one other
name each). `tools/re/wld_entity_schema.py` and `tools/re/ep_schema.py` both used to key a class's
per-member observed-value statistics by the plain string `f"{cls}.{m}"`, so both occurrences of a
repeated name were pooled into one accumulator even though the row-emission loop already (correctly)
walked the member list by file position -- the generated doc printed two rows with identical
"Observed" text instead of each occurrence's own. `positional_key()` is the shared fix both scripts
now call, both to aggregate and to label rows (run: python -m unittest discover -s scripts).
"""

import collections
import unittest

from tools.re.ep_schema import positional_key
from tools.re.wld_entity_schema import positional_key as wld_positional_key


class PositionalKeyTests(unittest.TestCase):
    def test_first_occurrence_is_unsuffixed(self):
        """Every member that is not duplicated keeps exactly its old key, so regenerating the docs
        only changes rows for the six classes with a repeated member name."""
        seen = collections.Counter()
        self.assertEqual(positional_key(seen, "CBloomProperties.21"), "CBloomProperties.21")

    def test_second_occurrence_gets_a_distinguishing_suffix(self):
        seen = collections.Counter()
        first = positional_key(seen, "CBloomProperties.21")
        second = positional_key(seen, "CBloomProperties.21")
        self.assertEqual(first, "CBloomProperties.21")
        self.assertEqual(second, "CBloomProperties.21#2")
        self.assertNotEqual(first, second)

    def test_third_occurrence_is_distinguished_too(self):
        seen = collections.Counter()
        keys = [positional_key(seen, "X.9") for _ in range(3)]
        self.assertEqual(keys, ["X.9", "X.9#2", "X.9#3"])

    def test_unrelated_keys_do_not_affect_each_others_count(self):
        seen = collections.Counter()
        self.assertEqual(positional_key(seen, "A.1"), "A.1")
        self.assertEqual(positional_key(seen, "B.1"), "B.1")
        self.assertEqual(positional_key(seen, "A.1"), "A.1#2")

    def test_wld_entity_schema_uses_the_same_rule_as_ep_schema(self):
        for key in ("CAllMovingAnimation.7", "CDirectionalLightProperties.12", "SCameraEntity.24"):
            seen_a, seen_b = collections.Counter(), collections.Counter()
            self.assertEqual(
                [positional_key(seen_a, key) for _ in range(2)],
                [wld_positional_key(seen_b, key) for _ in range(2)],
            )


if __name__ == "__main__":
    unittest.main()
