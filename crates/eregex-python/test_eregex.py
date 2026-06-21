"""Smoke tests for the eregex Python bindings (run with plain unittest)."""

import unittest

import eregex as P


class TestConstruction(unittest.TestCase):
    def test_basic(self):
        re = P.Regex(r"(\w+)\s+(\w+)")
        self.assertEqual(re.pattern, r"(\w+)\s+(\w+)")
        self.assertEqual(re.capture_count, 2)
        self.assertIn("Regex", repr(re))

    def test_flags_constant(self):
        re = P.Regex("hello", P.IGNORECASE)
        # `flags` returns resolved flags (UNICODE + VERSION1 are defaults),
        # so check membership via bitwise AND.
        self.assertTrue(re.flags & P.IGNORECASE)

    def test_parse_flags(self):
        self.assertEqual(P.parse_flags("im"), P.IGNORECASE | P.MULTILINE)
        re = P.Regex("hello", P.parse_flags("i"))
        self.assertTrue(re.flags & P.IGNORECASE)
        self.assertTrue(re.is_match("HELLO"))

    def test_bad_pattern_raises(self):
        with self.assertRaises(ValueError):
            P.Regex("(")

    def test_compile_helper(self):
        self.assertIsInstance(P.compile(r"\d+"), P.Regex)
        with self.assertRaises(ValueError):
            P.compile("(")


class TestFind(unittest.TestCase):
    def test_find_basic(self):
        re = P.Regex(r"(\w+)\s+(\w+)")
        m = re.find("hello world")
        self.assertIsNotNone(m)
        self.assertEqual(m.matched, "hello world")
        self.assertEqual(m.start, 0)
        self.assertEqual(m.end, 11)
        self.assertEqual(m.span, (0, 11))
        self.assertEqual(m.input, "hello world")
        self.assertEqual(m.group(1), "hello")
        self.assertEqual(m.group(2), "world")
        with self.assertRaises(IndexError):
            m.group(99)
        self.assertEqual(m.groups, ["hello world", "hello", "world"])

    def test_find_no_match(self):
        re = P.Regex(r"(\w+)\s+(\w+)")
        self.assertIsNone(re.find("oneword"))

    def test_indexing(self):
        re = P.Regex(r"(\w+)\s+(\w+)")
        m = re.find("hello world")
        self.assertEqual(m[0], "hello world")
        self.assertEqual(m[1], "hello")
        self.assertEqual(m[2], "world")
        self.assertEqual(len(m), 3)

    def test_named_groups(self):
        re = P.Regex(r"(?P<host>\w+)=(?P<port>\d+)")
        m = re.find("srv=8080")
        self.assertEqual(m.named_groups, {"host": "srv", "port": "8080"})
        self.assertEqual(m.group("host"), "srv")
        self.assertEqual(m["port"], "8080")

    def test_group_no_args(self):
        re = P.Regex(r"(\w+)\s+(\w+)")
        m = re.find("hello world")
        self.assertEqual(m.group(), ("hello world", "hello", "world"))

    def test_group_multiple_args(self):
        re = P.Regex(r"(\w+)\s+(\w+)")
        m = re.find("hello world")
        self.assertEqual(m.group(2, 1), ("world", "hello"))


class TestRepeatedCaptures(unittest.TestCase):
    def test_captures(self):
        m = P.Regex(r"(\w)+").find("abc")
        self.assertEqual(m.captures(1), ["a", "b", "c"])

    def test_captures_by_name(self):
        m = P.Regex(r"(?P<c>\w)+").find("xy")
        self.assertEqual(m.captures_by_name("c"), ["x", "y"])
        self.assertEqual(m.captures_dict, {"c": ["x", "y"]})

    def test_all_captures(self):
        m = P.Regex(r"(\w)+").find("abc")
        self.assertEqual(m.all_captures[1], ["a", "b", "c"])


class TestAnchors(unittest.TestCase):
    def test_match_at_start(self):
        digits = P.Regex(r"\d+")
        self.assertEqual(digits.match_at_start("123abc").matched, "123")
        self.assertIsNone(digits.match_at_start("abc123"))

    def test_fullmatch(self):
        digits = P.Regex(r"\d+")
        self.assertEqual(digits.fullmatch("123").matched, "123")
        self.assertEqual(digits.fullmatch("1234").matched, "1234")
        self.assertIsNone(P.Regex(r"\d{3}").fullmatch("1234"))


class TestFindAll(unittest.TestCase):
    def test_findall(self):
        re = P.Regex(r"\d+")
        all_matches = re.findall("a1 bb 22 c333")
        self.assertEqual([m.matched for m in all_matches], ["1", "22", "333"])


class TestSubstitution(unittest.TestCase):
    def test_replace(self):
        re = P.Regex(r"(?P<a>\d)(?P<b>\d)")
        self.assertEqual(re.replace("12 x", "${b}${a}"), "21 x")

    def test_replace_all(self):
        re = P.Regex(r"(?P<a>\d)(?P<b>\d)")
        self.assertEqual(re.replace_all("12 34", "${b}${a}"), "21 43")

    def test_split(self):
        self.assertEqual(P.Regex(r"\s+").split("a  b c"), ["a", "b", "c"])


class TestPartial(unittest.TestCase):
    def setUp(self):
        self.re = P.Regex(r"token=([a-z]+)([0-9]+)")

    def test_partial(self):
        p = self.re.find_partial("x token=abc")
        self.assertIsNotNone(p)
        self.assertTrue(p.is_partial)
        self.assertEqual(p.status, "partial")
        self.assertEqual(p.matched, "token=abc")
        self.assertEqual(p.group(1), "abc")
        self.assertEqual(p.group_state(1), "matched")
        self.assertEqual(p.group_state(2), "partial")

    def test_full(self):
        p = self.re.find_partial("token=abc123")
        self.assertTrue(p.is_full)
        self.assertEqual(p.status, "full")

    def test_hard_mismatch(self):
        self.assertIsNone(self.re.find_partial("x token=abc!"))


class TestHelpers(unittest.TestCase):
    def test_escape(self):
        self.assertEqual(P.escape("a.b*c"), r"a\.b\*c")

    def test_is_match_module(self):
        self.assertTrue(P.is_match(r"\d+", "abc 123"))
        self.assertFalse(P.is_match(r"\d+", "no digits"))

    def test_escape_special_only(self):
        self.assertEqual(P.escape_special_only("a.b!"), r"a\.b!")

    def test_introspection(self):
        re = P.Regex(r"(?P<year>\d{4})-(?P<month>\d{2})")
        self.assertEqual(sorted(re.group_names()), ["month", "year"])
        self.assertEqual(re.group_index("month"), 2)
        self.assertIsNone(re.group_index("nope"))


if __name__ == "__main__":
    unittest.main()
