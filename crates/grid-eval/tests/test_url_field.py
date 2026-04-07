"""Tests for URLField – covering the whitespace-stripping bug and its fix."""

from __future__ import annotations

import unittest

from validators.url import ValidationError


# ---------------------------------------------------------------------------
# Helper – import both the buggy and the fixed field so tests can reference
# each directly and clearly demonstrate the before/after behaviour.
# ---------------------------------------------------------------------------
from fields.url import URLField as BuggyURLField           # original
from fields.url_fixed import URLField as FixedURLField     # patched


class TestBuggyURLField(unittest.TestCase):
    """Demonstrate the existing bug: whitespace around a valid URL causes
    an unexpected ValidationError instead of returning the stripped URL."""

    def setUp(self) -> None:
        self.field = BuggyURLField()

    # ------------------------------------------------------------------
    # Cases that work correctly even in the buggy version
    # ------------------------------------------------------------------

    def test_valid_url_no_whitespace(self) -> None:
        """A URL with no surrounding spaces validates correctly."""
        url = "https://example.com/path?q=1#frag"
        self.assertEqual(self.field.clean(url), url)

    def test_http_scheme_accepted(self) -> None:
        """http:// is in the default allowed schemes."""
        self.assertEqual(self.field.clean("http://example.com"), "http://example.com")

    def test_disallowed_scheme_raises(self) -> None:
        """ftp:// is not in the default scheme list."""
        with self.assertRaises(ValidationError):
            self.field.clean("ftp://example.com")

    def test_empty_string_required_raises(self) -> None:
        """Empty input raises ValidationError when field is required."""
        with self.assertRaises(ValidationError):
            self.field.clean("")

    def test_none_required_raises(self) -> None:
        """None input raises ValidationError when field is required."""
        with self.assertRaises(ValidationError):
            self.field.clean(None)

    # ------------------------------------------------------------------
    # Cases that expose the bug
    # ------------------------------------------------------------------

    def test_leading_whitespace_raises_unexpectedly(self) -> None:
        """BUG: leading space causes ValidationError on an otherwise valid URL."""
        with self.assertRaises(ValidationError):
            # Should succeed (strip then validate), but instead blows up.
            self.field.clean("  https://example.com")

    def test_trailing_whitespace_raises_unexpectedly(self) -> None:
        """BUG: trailing space causes ValidationError on an otherwise valid URL."""
        with self.assertRaises(ValidationError):
            self.field.clean("https://example.com  ")

    def test_both_sides_whitespace_raises_unexpectedly(self) -> None:
        """BUG: spaces on both sides cause ValidationError."""
        with self.assertRaises(ValidationError):
            self.field.clean("  https://example.com/path  ")

    def test_tab_whitespace_raises_unexpectedly(self) -> None:
        """BUG: tab characters around a URL cause ValidationError."""
        with self.assertRaises(ValidationError):
            self.field.clean("\thttps://example.com\t")

    def test_newline_whitespace_raises_unexpectedly(self) -> None:
        """BUG: newlines around a URL cause ValidationError."""
        with self.assertRaises(ValidationError):
            self.field.clean("\nhttps://example.com\n")

    def test_buggy_clean_returns_unstripped_value(self) -> None:
        """BUG: even when validation passes, the raw un-stripped string is
        returned to the caller instead of the cleaned, stripped value."""
        raw = "https://example.com"
        result = self.field.clean(raw)
        # Works here only because there's no whitespace to begin with.
        self.assertEqual(result, raw)


class TestFixedURLField(unittest.TestCase):
    """Verify the fixed URLField handles all whitespace edge-cases correctly."""

    def setUp(self) -> None:
        self.field = FixedURLField()

    # ------------------------------------------------------------------
    # Previously-buggy cases now pass
    # ------------------------------------------------------------------

    def test_leading_whitespace_is_stripped_and_valid(self) -> None:
        """Leading whitespace is silently stripped; URL is accepted."""
        self.assertEqual(self.field.clean("  https://example.com"), "https://example.com")

    def test_trailing_whitespace_is_stripped_and_valid(self) -> None:
        """Trailing whitespace is silently stripped; URL is accepted."""
        self.assertEqual(self.field.clean("https://example.com  "), "https://example.com")

    def test_both_sides_whitespace_stripped(self) -> None:
        """Whitespace on both sides is stripped; URL is accepted."""
        self.assertEqual(
            self.field.clean("  https://example.com/path  "),
            "https://example.com/path",
        )

    def test_tab_characters_stripped(self) -> None:
        """Tab characters are stripped; URL is accepted."""
        self.assertEqual(
            self.field.clean("\thttps://example.com\t"),
            "https://example.com",
        )

    def test_newline_characters_stripped(self) -> None:
        """Newline characters are stripped; URL is accepted."""
        self.assertEqual(
            self.field.clean("\nhttps://example.com\n"),
            "https://example.com",
        )

    def test_mixed_whitespace_stripped(self) -> None:
        """Mixed whitespace (spaces, tabs, newlines) is stripped."""
        self.assertEqual(
            self.field.clean(" \t\n https://example.com \n\t "),
            "https://example.com",
        )

    # ------------------------------------------------------------------
    # Existing correct behaviour is preserved
    # ------------------------------------------------------------------

    def test_valid_url_no_whitespace(self) -> None:
        """A URL without surrounding whitespace still validates correctly."""
        url = "https://example.com/path?q=1#frag"
        self.assertEqual(self.field.clean(url), url)

    def test_http_scheme_accepted(self) -> None:
        """http:// is in the default allowed schemes."""
        self.assertEqual(self.field.clean("http://example.com"), "http://example.com")

    def test_disallowed_scheme_raises(self) -> None:
        """ftp:// raises ValidationError regardless of whitespace."""
        with self.assertRaises(ValidationError):
            self.field.clean("ftp://example.com")

    def test_disallowed_scheme_with_whitespace_raises(self) -> None:
        """Stripping happens before scheme check, so bad schemes are still caught."""
        with self.assertRaises(ValidationError):
            self.field.clean("  ftp://example.com  ")

    def test_empty_string_required_raises(self) -> None:
        """Empty input raises ValidationError when field is required."""
        with self.assertRaises(ValidationError):
            self.field.clean("")

    def test_whitespace_only_required_raises(self) -> None:
        """Whitespace-only input is treated as empty and raises ValidationError."""
        with self.assertRaises(ValidationError):
            self.field.clean("   ")

    def test_none_required_raises(self) -> None:
        """None raises ValidationError when field is required."""
        with self.assertRaises(ValidationError):
            self.field.clean(None)

    def test_not_required_empty_string_returns_empty(self) -> None:
        """Optional field returns empty string for empty input."""
        field = FixedURLField(required=False)
        self.assertEqual(field.clean(""), "")

    def test_not_required_whitespace_only_returns_empty(self) -> None:
        """Optional field strips whitespace-only input to empty string."""
        field = FixedURLField(required=False)
        self.assertEqual(field.clean("   "), "")

    def test_custom_schemes_accepted(self) -> None:
        """A field with custom schemes accepts only those schemes."""
        field = FixedURLField(schemes=("ftp",))
        self.assertEqual(field.clean("ftp://files.example.com"), "ftp://files.example.com")

    def test_custom_schemes_with_whitespace(self) -> None:
        """Whitespace is stripped before the custom-scheme check."""
        field = FixedURLField(schemes=("ftp",))
        self.assertEqual(
            field.clean("  ftp://files.example.com  "),
            "ftp://files.example.com",
        )

    def test_url_with_port(self) -> None:
        """URLs with an explicit port number are accepted."""
        self.assertEqual(
            self.field.clean("  https://example.com:8080/path  "),
            "https://example.com:8080/path",
        )

    def test_url_with_query_and_fragment(self) -> None:
        """Full URLs including query-string and fragment are accepted."""
        self.assertEqual(
            self.field.clean("  https://example.com/p?k=v#anchor  "),
            "https://example.com/p?k=v#anchor",
        )

    def test_max_length_applied_after_strip(self) -> None:
        """max_length is measured on the stripped value, not the raw input."""
        field = FixedURLField(max_length=22)
        # "https://example.com" is exactly 19 chars – fits within 22.
        self.assertEqual(
            field.clean("  https://example.com  "),
            "https://example.com",
        )

    def test_max_length_exceeded_raises(self) -> None:
        """Stripped URL that still exceeds max_length raises ValidationError."""
        field = FixedURLField(max_length=10)
        with self.assertRaises(ValidationError):
            field.clean("https://example.com")

    def test_invalid_url_raises(self) -> None:
        """An invalid URL (not matching the URL pattern) still raises."""
        with self.assertRaises(ValidationError):
            self.field.clean("not-a-url")

    def test_invalid_url_with_whitespace_raises(self) -> None:
        """Stripping does not rescue a fundamentally invalid URL."""
        with self.assertRaises(ValidationError):
            self.field.clean("  not-a-url  ")


if __name__ == "__main__":
    unittest.main()
