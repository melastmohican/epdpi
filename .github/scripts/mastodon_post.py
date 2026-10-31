#!/usr/bin/env python3
"""Builds the text of the Mastodon post for a release. No network, only text.

    mastodon_post.py --version 0.1.0 --repo melastmohican/epdpi --notes release-notes.md

`--notes` is the file `release.yml` writes: for each crate a line `## <crate>` followed by that crate's
section of its CHANGELOG.md. The post is a header, for each crate the first sentence of each bullet
(bullets under `### Notes` are for readers of the changelog and are left out), and three links.

Mastodon allows 500 characters and counts every URL as 23, whatever its length. The text is kept
under the limit less a margin: the two crates share what is left (half each, and what one does not
use goes to the other), a crate that is too long loses whole trailing bullets first, and only a
single bullet that is still too long is cut, at a word boundary, with an ellipsis.

The post goes to stdout, the length that Mastodon will count to stderr.
"""
import argparse
import re
import sys

CRATES = ["epdpi", "epdpi-esp"]
LIMIT = 500
MARGIN = 10
URL_LENGTH = 23
URL = re.compile(r"https?://\S+")
ELLIPSIS = "…"


def counted_length(text):
    """Characters Mastodon counts: every URL is 23, the rest by code point."""
    return len(URL.sub("x" * URL_LENGTH, text))


def first_sentence(text):
    """The text up to and including the first full stop that ends a sentence, without backticks."""
    text = " ".join(text.replace("`", "").split())
    end = re.search(r"\.(\s|$)", text)
    return text[: end.start() + 1] if end else text


def parse_notes(text):
    """crate -> first sentences of its bullets, in order, without the `### Notes` bullets."""
    crates = {}
    crate = None
    skip = False
    bullet = None

    def finish():
        nonlocal bullet
        if bullet is not None and crate is not None:
            sentence = first_sentence(bullet)
            if sentence:
                crates[crate].append(sentence)
        bullet = None

    for line in text.splitlines():
        if line.startswith("## "):
            finish()
            crate = line[3:].strip()
            crates.setdefault(crate, [])
            skip = False
        elif line.startswith("### "):
            finish()
            skip = line[4:].strip().lower() == "notes"
        elif line.startswith("- ") and not skip:
            finish()
            bullet = line[2:]
        elif line.startswith("  ") and bullet is not None:
            bullet += " " + line.strip()
        elif not line.strip():
            finish()
    finish()
    return crates


def section(label, bullets):
    return "\n".join([label] + ["- " + b for b in bullets])


def fit(label, bullets, budget):
    """The section for one crate in at most `budget` counted characters, or "" when nothing fits."""
    if not bullets:
        return ""
    kept = list(bullets)
    while len(kept) > 1 and counted_length(section(label, kept)) > budget:
        kept.pop()
    text = section(label, kept)
    if counted_length(text) <= budget:
        return text
    # One bullet is still too long: cut it at a word boundary.
    room = budget - counted_length(label + "\n- ") - len(ELLIPSIS)
    if room < 10:
        return ""
    cut = kept[0][:room]
    if len(kept[0]) > room and " " in cut:
        cut = cut[: cut.rindex(" ")]
    return section(label, [cut.rstrip(" ,;:") + ELLIPSIS])


def build(version, repo, notes, limit=LIMIT):
    crates = parse_notes(notes)
    header = f"epdpi and epdpi-esp v{version} released \U0001f389"
    links = "\n".join(
        [f"https://github.com/{repo}/releases/tag/v{version}"]
        + [f"https://crates.io/crates/{name}" for name in CRATES]
    )
    sections = [(name, crates.get(name, [])) for name in CRATES]
    sections = [(name, bullets) for name, bullets in sections if bullets]

    # Room for the sections: what is left after the header, the links and the blank lines between blocks.
    blocks = 2 + len(sections)
    avail = limit - MARGIN - counted_length(header) - counted_length(links) - 2 * (blocks - 1)

    texts = [""] * len(sections)
    if len(sections) == 2:
        space = 2  # the blank line between the two sections
        half = (avail - space) // 2
        texts = [fit(name, bullets, half) for name, bullets in sections]
        # What one crate does not use goes to the other: epdpi first, then epdpi-esp gets its share back.
        used = [counted_length(text) for text in texts]
        texts[0] = fit(sections[0][0], sections[0][1], avail - space - used[1])
        used[0] = counted_length(texts[0])
        texts[1] = fit(sections[1][0], sections[1][1], avail - space - used[0])
    elif sections:
        texts = [fit(sections[0][0], sections[0][1], avail)]

    parts = [header] + [text for text in texts if text] + [links]
    return "\n\n".join(parts)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--version", required=True)
    parser.add_argument("--repo", required=True, help="owner/name on GitHub")
    parser.add_argument("--notes", required=True, help="file with a `## <crate>` section per crate")
    parser.add_argument("--limit", type=int, default=LIMIT)
    args = parser.parse_args()

    with open(args.notes, encoding="utf-8") as handle:
        notes = handle.read()
    text = build(args.version, args.repo, notes, args.limit)
    print(text)
    length = counted_length(text)
    print(f"mastodon_post: {length} of {args.limit} characters counted", file=sys.stderr)
    if length > args.limit:
        print("mastodon_post: the post is over the limit", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
