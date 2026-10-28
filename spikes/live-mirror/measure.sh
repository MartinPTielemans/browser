#!/bin/sh
# Spike (#359): runs app/check.sh's mirror run ROUNDS (3) times for each mode in turn, the picture
# alone first, then MIRROR=host and MIRROR=portal, so drift over the session hits each alike. Keeps
# every line in app/build/mirror-results.txt with the captures beside it, and prints each mode's
# means. Build the app first, with the core from this branch (README.md).
set -e
cd "$(dirname "$0")/../../app"
rounds="${ROUNDS:-3}"
results="$PWD/build/mirror-results.txt"
keep="$PWD/build/mirror-runs"
rm -rf "$results" "$keep"
mkdir -p "$keep"
for round in $(seq "$rounds"); do
  for mode in picture host portal; do
    if [ "$mode" = picture ]; then
      MIRROR= ./check.sh mirror
    else
      MIRROR=$mode ./check.sh mirror
    fi
    cat build/check/mirror.txt >> "$results"
    mkdir -p "$keep/$round-$mode"
    cp build/check/mirror* "$keep/$round-$mode/"
  done
done
# Each mode's and phase's means: CPU seconds, then idle wakeups a second.
awk '
  {
    key = $2 " " $3
    rounds[key]++
    sub(/^[^:]*: [^:]*: /, "")
    n = split($0, part, "; ")
    for (i = 1; i <= n; i++) {
      split(part[i], word, " ")
      if (word[2] == "cpu") {
        cpu[key, word[1]] += word[3]
        wakes[key, word[1]] += word[5]
        names[word[1]] = 1
      } else if (word[1] == "GPU") {
        gpu[key] += word[3]
      }
    }
  }
  END {
    for (key in rounds) {
      line = key ":"
      for (name in names) {
        line = line sprintf(" %s %.2f s %.1f/s,", name, cpu[key, name] / rounds[key],
          wakes[key, name] / rounds[key])
      }
      print line sprintf(" GPU active %.1f percent", gpu[key] / rounds[key])
    }
  }' "$results" | sort
echo "n/a counts as 0 in the means. Every run: $results"
