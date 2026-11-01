#!/bin/bash
# Prints the notes for VERSION from CHANGELOG.md: the lines under its "## VERSION" heading, up to the
# next "## " heading, without the heading. release.yml makes them the release's body, which the site's
# /releases page and the update prompts show. Fails when the section is missing or empty, and with
# --final when its heading still says "(draft)", so a release never goes out with unreviewed notes.
# Usage: tools/changelog.sh VERSION [--final] [CHANGELOG]
set -euo pipefail
usage="usage: tools/changelog.sh VERSION [--final] [CHANGELOG]"
version=${1:?$usage}
shift
final=
if [ "${1:-}" = --final ]; then final=1 && shift; fi
file=${1:-"$(cd "$(dirname "$0")/.." && pwd)/CHANGELOG.md"}
name=${file#"$PWD"/}

heading=$(grep -E -m1 "^## $(printf '%s' "$version" | sed 's/\./\\./g')( \(draft\))?[[:space:]]*$" "$file" || true)
if [ -z "$heading" ]; then
  echo "$name has no section \"## $version\". Write the release's notes there first." >&2
  exit 1
fi
if [ -n "$final" ] && [[ $heading == *"(draft)"* ]]; then
  echo "$name's notes for $version are still marked draft. Review them and take \"(draft)\" off the heading." >&2
  exit 1
fi
# The section's lines, without the blank ones before them; $(…) drops those after.
notes=$(awk -v h="$heading" '
  $0 == h { on = 1; next }
  on && /^## / { exit }
  on { print }
' "$file" | sed '/./,$!d')
if [ -z "$notes" ]; then
  echo "$name's section for $version is empty." >&2
  exit 1
fi
printf '%s\n' "$notes"
