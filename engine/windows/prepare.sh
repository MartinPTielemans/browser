#!/bin/bash
# Prepares ungoogled-chromium-windows at the tag pins.ini names, in the directory its CI builds in,
# to build our engine: its build.py with three hooks. After its patches and domain substitution, it
# runs series.sh (the core and our series). It builds what ILO_TARGETS names: chrome, or with
# ILO_CORE_ONLY the core's own objects, the compile check. And beside its package it leaves
# chrome.dll.lib, which the app links. Run by .github/workflows/engine-windows.yml, in Git Bash.
set -euo pipefail
[ $# -eq 1 ] || { echo "usage: prepare.sh <directory>" >&2 && exit 2; }
ucw=$1
engine=$(cd "$(dirname "$0")/.." && pwd)

ucw_tag=$(git config -f "$engine/pins.ini" ungoogled-chromium-windows.tag)
git clone -q -c advice.detachedHead=false -c core.autocrlf=false --depth 1 --branch "$ucw_tag" \
  --recurse-submodules --shallow-submodules \
  https://github.com/ungoogled-software/ungoogled-chromium-windows.git "$ucw"
. "$engine/tree.sh"
check_ungoogled "$ucw/ungoogled-chromium" "ungoogled-chromium-windows $ucw_tag"
# PlayReady, which Chromium turns on only for Chrome's own builds, plays protected video on
# Windows (decision 1). build.py writes this file's args after ungoogled-chromium's.
printf '\nenable_playready=true\n' >>"$ucw/flags.windows.gn"

# Each hook replaces a line build.py has at this tag, and fails if it's gone.
hook() {
  local anchor=$1 replacement=$2
  ANCHOR=$anchor REPLACEMENT=$replacement perl -0pi -e \
    '$n = s/\Q$ENV{ANCHOR}\E/$ENV{REPLACEMENT}/; die "build.py has no \"$ENV{ANCHOR}\"\n" unless $n' \
    "$ucw/build.py"
}
hook "    # Check if rust-toolchain folder has been populated" \
"    # ilo: the core and our series, once (engine/windows/prepare.sh).
    if not (source_tree / 'browser_core').exists():
        subprocess.run([os.environ['ILO_BASH'], os.environ['ILO_ENGINE'] + '/windows/series.sh',
                        str(source_tree)], check=True)

    # Check if rust-toolchain folder has been populated"
hook "    ninja_commandline.append('chrome')
    ninja_commandline.append('chromedriver')
    ninja_commandline.append('mini_installer')" \
"    if os.environ.get('ILO_CORE_ONLY'):
        # Only the core's own objects: a source file's name with ^ builds what it compiles to.
        ninja_commandline += ['../../' + p.relative_to(source_tree).as_posix() + '^'
                              for p in (source_tree / 'browser_core').rglob('*.cc')]
    else:
        ninja_commandline.append('chrome')"
hook "        subprocess.run([sys.executable, 'package.py', '--cpu-arch', '32bit' if args.x86 else 'arm' if args.arm else '64bit'])" \
"        if not os.environ.get('ILO_CORE_ONLY'):
            subprocess.run([sys.executable, 'package.py', '--cpu-arch', '64bit'], check=True)
            shutil.copy2('build/src/out/Default/chrome.dll.lib', 'build/ungoogled-chromium_chrome.dll.lib')"
