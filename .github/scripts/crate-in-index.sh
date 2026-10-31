#!/usr/bin/env bash
# Is <version> of <crate> listed in the crates.io sparse index?
#
#   crate-in-index.sh <crate> <version>
#
# Exit 0: listed (a yanked version counts, it can never be published again).
# Exit 10: not listed, because the crate or the version is missing.
# Exit 1: anything else (no network, an unexpected HTTP status).
#
# The index is queried directly. `cargo info` run inside the workspace resolves the local package
# and would always say "published".
set -euo pipefail

name="${1:?usage: crate-in-index.sh <crate> <version>}"
version="${2:?usage: crate-in-index.sh <crate> <version>}"

# Sparse index layout: 1 character -> 1/<name>, 2 -> 2/<name>, 3 -> 3/<first>/<name>,
# 4 or more -> <first two>/<next two>/<name>. Names are lower case in the index.
lower="$(printf '%s' "$name" | tr '[:upper:]' '[:lower:]')"
case ${#lower} in
  1) path="1/$lower" ;;
  2) path="2/$lower" ;;
  3) path="3/${lower:0:1}/$lower" ;;
  *) path="${lower:0:2}/${lower:2:2}/$lower" ;;
esac

body="$(mktemp)"
trap 'rm -f "$body"' EXIT
code="$(curl -sS -o "$body" -w '%{http_code}' "https://index.crates.io/$path")"
case "$code" in
  404)
    exit 10
    ;;
  200)
    # One JSON document per line, one line per version.
    if jq -e --arg v "$version" 'select(.vers == $v)' "$body" > /dev/null; then
      exit 0
    fi
    exit 10
    ;;
  *)
    echo "unexpected HTTP $code from the crates.io index for $name" >&2
    exit 1
    ;;
esac
