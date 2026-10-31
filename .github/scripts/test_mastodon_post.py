"""Tests of mastodon_post.py. Run from the repo root:

    python3 -m unittest discover -s .github/scripts -p 'test_*.py'
"""
import pathlib
import subprocess
import unittest

import mastodon_post as post

ROOT = pathlib.Path(__file__).resolve().parents[2]
REPO = "melastmohican/epdpi"


def notes(epdpi, esp):
    """The file release.yml writes, from two lists of changelog lines."""
    return "## epdpi\n" + "\n".join(epdpi) + "\n\n## epdpi-esp\n" + "\n".join(esp) + "\n\n"


def build(epdpi, esp, version="0.1.0", **kwargs):
    return post.build(version, REPO, notes(epdpi, esp), **kwargs)


SHORT_EPDPI = ["### Added", "- A driver for e-paper panels. It has more.", "- A second thing."]
SHORT_ESP = ["### Added", "- A bus for the ESP32-S3. It is fast."]


class CountedLength(unittest.TestCase):
    def test_a_url_counts_as_23_whatever_its_length(self):
        short = "https://a.io/x"
        long = "https://example.com/" + "a" * 200
        self.assertEqual(post.counted_length(short), 23)
        self.assertEqual(post.counted_length(long), 23)
        self.assertEqual(post.counted_length(f"see {long} now"), 4 + 23 + 4)

    def test_other_text_counts_by_character(self):
        self.assertEqual(post.counted_length("released \U0001f389"), 10)


class ParseNotes(unittest.TestCase):
    def test_first_sentence_of_each_bullet_without_backticks(self):
        parsed = post.parse_notes(
            notes(["### Added", "- A `no_std` driver for panels. It uses `embedded-hal` 1.0.", "- Seven modes."], [])
        )
        self.assertEqual(parsed["epdpi"], ["A no_std driver for panels.", "Seven modes."])

    def test_a_version_number_does_not_end_a_sentence(self):
        parsed = post.parse_notes(notes(["### Added", "- Works with esp-hal 1.1.x only. More text."], []))
        self.assertEqual(parsed["epdpi"], ["Works with esp-hal 1.1.x only."])

    def test_wrapped_bullets_are_joined(self):
        parsed = post.parse_notes(notes(["### Added", "- A driver for the panel", "  that wraps over lines. Then more."], []))
        self.assertEqual(parsed["epdpi"], ["A driver for the panel that wraps over lines."])

    def test_notes_bullets_are_left_out(self):
        parsed = post.parse_notes(
            notes(["### Added", "- In the post.", "", "### Notes", "- Not in the post.", "- Neither."], ["### Notes", "- No."])
        )
        self.assertEqual(parsed["epdpi"], ["In the post."])
        self.assertEqual(parsed["epdpi-esp"], [])


class Build(unittest.TestCase):
    def test_a_short_changelog_is_all_there(self):
        text = build(SHORT_EPDPI, SHORT_ESP)
        self.assertLessEqual(post.counted_length(text), post.LIMIT)
        self.assertIn("epdpi and epdpi-esp v0.1.0 released", text)
        self.assertIn("epdpi\n- A driver for e-paper panels.\n- A second thing.", text)
        self.assertIn("epdpi-esp\n- A bus for the ESP32-S3.", text)
        self.assertTrue(text.endswith("https://crates.io/crates/epdpi-esp"))
        for link in [
            "https://github.com/melastmohican/epdpi/releases/tag/v0.1.0",
            "https://crates.io/crates/epdpi\n",
        ]:
            self.assertIn(link, text)

    def test_a_long_changelog_is_cut_under_the_limit_by_whole_bullets(self):
        many = ["### Added"] + [f"- Bullet number {i} says something of a fair length here." for i in range(20)]
        text = build(many, many)
        # A fixed number, not the constant: the post keeps 10 characters below the limit.
        self.assertLessEqual(post.counted_length(text), 490)
        # Both crates are there, and what is kept is the first bullets, whole.
        self.assertIn("epdpi\n- Bullet number 0 says something of a fair length here.", text)
        self.assertIn("epdpi-esp\n- Bullet number 0 says something of a fair length here.", text)
        self.assertNotIn("Bullet number 19", text)
        self.assertNotIn(post.ELLIPSIS, text)

    def test_the_crates_share_the_room_half_each(self):
        many = ["### Added"] + [f"- Bullet number {i} says something of a fair length here." for i in range(20)]
        text = build(many, many)
        epdpi_part = text.split("\n\nepdpi-esp\n")[0].split("\n\n")[1]
        esp_part = text.split("\n\nepdpi-esp\n")[1].split("\n\n")[0]
        self.assertLessEqual(abs(epdpi_part.count("\n") - esp_part.count("\n")), 1)

    def test_a_short_crate_leaves_its_room_to_the_other(self):
        many = ["### Added"] + [f"- Bullet number {i} says something of a fair length here." for i in range(20)]
        text = build(many, ["### Added", "- Tiny."])
        self.assertIn("epdpi-esp\n- Tiny.", text)
        # More than half the room goes to epdpi.
        half_bullets = text.count("Bullet number")
        both_long = build(many, many).count("Bullet number") // 2
        self.assertGreater(half_bullets, both_long)
        self.assertLessEqual(post.counted_length(text), post.LIMIT)

    def test_a_short_epdpi_leaves_its_room_to_epdpi_esp(self):
        many = ["### Added"] + [f"- Bullet number {i} says something of a fair length here." for i in range(20)]
        text = build(["### Added", "- Tiny."], many)
        self.assertIn("epdpi\n- Tiny.", text)
        esp_bullets = text.count("Bullet number")
        both_long = build(many, many).count("Bullet number") // 2
        self.assertGreater(esp_bullets, both_long)
        self.assertLessEqual(post.counted_length(text), 490)

    def test_one_very_long_bullet_is_cut_at_a_word_with_an_ellipsis(self):
        # Words that cannot be cut and still look whole: "w012x" cut anywhere does not end in x.
        long_bullet = "- " + " ".join(f"w{i:03d}x" for i in range(400)) + "."
        # Several limits, so the cut lands in different places of a word.
        for limit in range(300, 501, 7):
            text = build(["### Added", long_bullet], ["### Added", "- A bus. It is fast."], limit=limit)
            counted = post.counted_length(text)
            self.assertLessEqual(counted, limit - 10, f"limit {limit}")
            self.assertIn(post.ELLIPSIS, text, f"limit {limit}")
            last_word = text.split(post.ELLIPSIS)[0].rsplit(" ", 1)[1]
            self.assertRegex(last_word, r"^w\d{3}x$", f"cut inside a word at limit {limit}")
            self.assertIn("epdpi-esp\n- A bus.", text)
        # The cut fills the room, and keeps the margin below the limit.
        text = build(["### Added", long_bullet], ["### Added", "- A bus. It is fast."])
        self.assertLessEqual(post.counted_length(text), 490)

    def test_a_changelog_with_no_bullets_gives_the_header_and_the_links(self):
        text = build(["### Added", "", "Nothing yet."], ["### Notes", "- Only notes."])
        self.assertEqual(
            text,
            "epdpi and epdpi-esp v0.1.0 released \U0001f389\n\n"
            "https://github.com/melastmohican/epdpi/releases/tag/v0.1.0\n"
            "https://crates.io/crates/epdpi\n"
            "https://crates.io/crates/epdpi-esp",
        )

    def test_only_one_crate_with_bullets_gets_all_the_room(self):
        many = ["### Added"] + [f"- Bullet number {i} says something of a fair length here." for i in range(20)]
        text = build(many, ["### Added"])
        self.assertNotIn("epdpi-esp\n", text.replace("https://crates.io/crates/epdpi-esp", ""))
        self.assertLessEqual(post.counted_length(text), post.LIMIT)
        # The room is used: what is left over is less than one more bullet.
        self.assertLess(post.LIMIT - post.MARGIN - post.counted_length(text), 60)

    def test_a_tiny_limit_still_gives_a_post_inside_it_or_no_sections(self):
        text = build(SHORT_EPDPI, SHORT_ESP, limit=170)
        self.assertLessEqual(post.counted_length(text), 170)
        self.assertIn("epdpi and epdpi-esp v0.1.0", text)

    def test_the_real_changelogs_of_this_repo_fit(self):
        out = []
        for crate in post.CRATES:
            section = subprocess.run(
                [str(ROOT / ".github/scripts/changelog-section.sh"), str(ROOT / crate / "CHANGELOG.md"), "0.1.0"],
                check=True,
                capture_output=True,
                text=True,
            ).stdout
            out.append(f"## {crate}\n{section}\n\n")
        text = post.build("0.1.0", REPO, "".join(out))
        print("\n--- the post for 0.1.0, as the changelogs read now ---\n" + text + "\n---")
        self.assertLessEqual(post.counted_length(text), post.LIMIT)
        self.assertIn("epdpi\n- A no_std driver", text)
        self.assertIn("epdpi-esp\n- A fast LCD_CAM", text)
        # More than the first bullet of each crate, so the post says something.
        self.assertGreaterEqual(text.count("\n- "), 5)
        self.assertNotIn("Notes", text)
        self.assertNotIn("non_exhaustive", text)


if __name__ == "__main__":
    unittest.main()
