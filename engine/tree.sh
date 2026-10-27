# What setup.sh and linux/prepare.sh both do to a prepared Chromium tree. Sourced; each function
# takes the tree's path.

engine=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

# Chromium's instructions and skills for its own contributors' agents. Ours are the repository's
# AGENTS.md. Some are symlinks to an AGENTS.md beside them. Chrome's own `skills` folders are a
# browser feature, not agent skills, so they stay.
remove_agent_instructions() {
  local src=$1
  find "$src" \( -name AGENTS.md -o -name CLAUDE.md -o -name GEMINI.md -o -name SKILL.md \
    -o -name .cursorrules -o -name .rules -o -name copilot-instructions.md \) \
    \( -type f -o -type l \) -delete
  find "$src" \( -name .claude -o -name .gemini -o -name .cursor -o -name .agents \) -type d -prune \
    -exec rm -rf {} +
  rm -rf "$src/agents"
}

# Checks that a platform repository's ungoogled-chromium submodule builds the pinned tag, or a
# later commit after it (ungoogled-chromium-macos 156.0.8078.47 builds one that fixes the tag's
# patches), for the pinned Chromium version. Takes the submodule's path and the platform tag.
check_ungoogled() {
  local uc=$1 platform_tag=$2 version uc_tag pinned
  version=$(git config -f "$engine/pins.ini" chromium.version)
  uc_tag=$(git config -f "$engine/pins.ini" ungoogled-chromium.tag)
  pinned=$(git ls-remote https://github.com/ungoogled-software/ungoogled-chromium.git \
    "refs/tags/$uc_tag^{}" | cut -f1)
  # The submodule is a shallow clone.
  git -C "$uc" fetch -q --deepen=20
  if ! git -C "$uc" merge-base --is-ancestor "$pinned" HEAD 2>/dev/null; then
    echo "$platform_tag doesn't build ungoogled-chromium $uc_tag" >&2
    exit 1
  fi
  if [ "$(cat "$uc/chromium_version.txt")" != "$version" ]; then
    echo "ungoogled-chromium $uc_tag is for Chromium $(cat "$uc/chromium_version.txt"), not $version" >&2
    exit 1
  fi
}

# Links the core into the tree as //browser_core, so the next build has every edit to it, and
# applies our series. A patch already in the tree is skipped, so this is safe to run again; one
# that neither applies cleanly nor is in the tree has drifted from it.
apply_series() {
  local src=$1 name patch
  ln -sfn "$(dirname "$engine")/core" "$src/browser_core"
  while read -r name; do
    patch=$engine/patches/$name
    if patch -d "$src" -p1 -F0 -N --dry-run -s <"$patch" >/dev/null 2>&1; then
      patch -d "$src" -p1 -F0 -N -s <"$patch"
      echo "applied $name" >&2
    elif ! patch -d "$src" -p1 -F0 -R --dry-run -s <"$patch" >/dev/null 2>&1; then
      echo "$name has drifted: it doesn't apply, and it isn't in the tree" >&2
      exit 1
    fi
  done < <(grep -v -e '^#' -e '^$' "$engine/series")
}
