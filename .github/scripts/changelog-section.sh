#!/usr/bin/env bash
# Prints the body of the `## [<version>]` section of a Keep a Changelog file, without the
# heading, up to the next `## [` heading or the link list at the bottom.
#
#   changelog-section.sh <CHANGELOG.md> <version>      (version can also be "Unreleased")
set -euo pipefail

file="${1:?usage: changelog-section.sh <file> <version>}"
version="${2:?usage: changelog-section.sh <file> <version>}"

awk -v ver="$version" '
  $0 ~ "^## \\[" ver "\\]"  { capture = 1; next }
  capture && /^## \[/        { exit }
  capture && /^\[[^]]+\]: /  { exit }
  capture                    { print }
' "$file"
