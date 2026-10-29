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
        # Only the core's own objects, those its BUILD.gn has on Windows, as ninja knows no other.
        graph = subprocess.run(ninja_commandline + ['-t', 'targets', 'all'],
                               capture_output=True, text=True, check=True).stdout
        targets = [line.split(': ')[0] for line in graph.splitlines()]
        core = [t for t in targets if t.startswith('obj/browser_core/') and t.endswith('.obj')]
        # With no target, ninja would build its default, the whole browser.
        if not core:
            raise RuntimeError('ninja has no objects of the core')
        ninja_commandline += core
    else:
        ninja_commandline.append('chrome')"
hook "        subprocess.run([sys.executable, 'package.py', '--cpu-arch', '32bit' if args.x86 else 'arm' if args.arm else '64bit'])" \
"        if not os.environ.get('ILO_CORE_ONLY'):
            subprocess.run([sys.executable, 'package.py', '--cpu-arch', '64bit'], check=True)
            shutil.copy2('build/src/out/Default/chrome.dll.lib', 'build/ungoogled-chromium_chrome.dll.lib')"
