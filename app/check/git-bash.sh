# What the runs use from the Mac that Git Bash lacks or has only for its own processes. check.sh
# sources it on Windows.

# The ids of the Windows processes whose command line, its backslashes as slashes, matches $1, as
# pgrep -f finds them. Git Bash's ps shows a native process's executable, never its arguments.
windows_procs() {
  PATTERN="$1" powershell.exe -NoProfile -NonInteractive -Command '
    Get-CimInstance Win32_Process | Where-Object {
      $_.CommandLine -and ($_.CommandLine -replace "\\", "/") -match $env:PATTERN
    } | ForEach-Object { $_.ProcessId }' | tr -d '\r'
}
# pgrep -f and pkill [-9 | -KILL] -f on those, ending them by id, never by image name, which would
# reach any other Chrome too.
pgrep() {
  [ "$1" = -f ] || { echo "pgrep: only -f" >&2; return 2; }
  ids=$(windows_procs "$2")
  [ -n "$ids" ] && echo "$ids"
}
pkill() {
  force=
  case "$1" in -9 | -KILL) force=//F; shift ;; esac
  [ "$1" = -f ] || { echo "pkill: only -f" >&2; return 2; }
  ids=$(windows_procs "$2")
  [ -n "$ids" ] || return 1
  for id in $ids; do
    taskkill $force //PID "$id" > /dev/null 2>&1 || true
  done
}
# Closes the windows the process $1 shows, as their close buttons would. taskkill without /F closes
# every top-level window, the hidden Chrome window of each profile too, and when one of those goes
# first it closes its tabs, which the browser then archives, and the app doesn't quit.
close_windows() {
  WINPID="$1" powershell.exe -NoProfile -NonInteractive -Command '
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class Shown {
  delegate bool EnumProc(IntPtr hwnd, IntPtr data);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc proc, IntPtr data);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
  [DllImport("user32.dll")] static extern IntPtr GetWindow(IntPtr hwnd, uint cmd);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
  [DllImport("user32.dll")] static extern bool PostMessageW(IntPtr hwnd, uint msg, IntPtr w, IntPtr l);
  public static int Close(uint process) {
    int closed = 0;
    EnumWindows((hwnd, data) => {
      uint owner;
      GetWindowThreadProcessId(hwnd, out owner);
      // Unowned: an overlay is owned by its browser window and goes with it.
      if (owner == process && IsWindowVisible(hwnd) && GetWindow(hwnd, 4) == IntPtr.Zero) {
        PostMessageW(hwnd, 0x0010, IntPtr.Zero, IntPtr.Zero);
        closed++;
      }
      return true;
    }, IntPtr.Zero);
    return closed;
  }
}
"@
    if ([Shown]::Close([uint32]$env:WINPID) -eq 0) { exit 1 }' > /dev/null
}
# Runs the app. A native program can't read Git Bash's fifo, so cat passes the driver's commands on
# through a pipe, and exec keeps $! the app's, whose Windows id is in /proc/$!/winpid. chrome_elf
# starts Crashpad before the app adds its profile, in the folder chrome.exe's own --user-data-dir
# names, or else the prod build's, so the profile the app will use, a run's own too, goes there
# as well: the crash run's reports are then in its Crashpad folder, where the core works with them.
windows_browser() {
  exec "$app/chrome.exe" "--user-data-dir=$(cygpath -w "${BROWSER_PROFILE_DIR:-$profile_dir}")" \
    "$@" < <(cat)
}
# Writes the image $1 as the BMP $2, as sips does on the Mac.
to_bmp() {
  IN="$(cygpath -w "$1")" OUT="$(cygpath -w "$2")" powershell.exe -NoProfile -NonInteractive \
    -Command 'Add-Type -AssemblyName System.Drawing
      $image = [System.Drawing.Image]::FromFile($env:IN)
      $image.Save($env:OUT, [System.Drawing.Imaging.ImageFormat]::Bmp)
      $image.Dispose()'
}
