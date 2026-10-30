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
  [DllImport("user32.dll")] static extern IntPtr SendMessageTimeoutW(IntPtr hwnd, uint msg, IntPtr w, IntPtr l, uint flags, uint timeout, IntPtr result);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassNameW(IntPtr hwnd, System.Text.StringBuilder name, int size);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowTextW(IntPtr hwnd, System.Text.StringBuilder name, int size);
  [StructLayout(LayoutKind.Sequential)] struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
  [DllImport("user32.dll")] static extern IntPtr GetWindowLongPtrW(IntPtr hwnd, int index);
  public static int Close(uint process) {
    int closed = 0;
    EnumWindows((hwnd, data) => {
      uint owner;
      GetWindowThreadProcessId(hwnd, out owner);
      if (owner == process) {
        var cls = new System.Text.StringBuilder(256); GetClassNameW(hwnd, cls, 256);
        var text = new System.Text.StringBuilder(256); GetWindowTextW(hwnd, text, 256);
        RECT r; GetWindowRect(hwnd, out r);
        Console.Error.WriteLine("diag window " + hwnd + " class " + cls + " title [" + text + "] visible " + IsWindowVisible(hwnd) + " owner " + GetWindow(hwnd, 4) + " rect " + r.Left + "," + r.Top + "," + r.Right + "," + r.Bottom + " style " + GetWindowLongPtrW(hwnd, -16).ToInt64().ToString("x") + " ex " + GetWindowLongPtrW(hwnd, -20).ToInt64().ToString("x"));
      }
      // Unowned: an overlay is owned by its browser window and goes with it.
      if (owner == process && IsWindowVisible(hwnd) && GetWindow(hwnd, 4) == IntPtr.Zero) {
        // WM_QUERYENDSESSION, then WM_ENDSESSION, as Windows ends a session.
        SendMessageTimeoutW(hwnd, 0x0011, IntPtr.Zero, IntPtr.Zero, 0, 5000, IntPtr.Zero);
        PostMessageW(hwnd, 0x0016, new IntPtr(1), IntPtr.Zero);
        closed++;
      }
      return true;
    }, IntPtr.Zero);
    return closed;
  }
}
"@
    if ([Shown]::Close([uint32]$env:WINPID) -eq 0) { exit 1 }' 2>&1 | tr -d '\r' >&2
}
# Runs the app. A native program can't read Git Bash's fifo, so cat passes the driver's commands on
# through a pipe, and exec keeps $! the app's, whose Windows id is in /proc/$!/winpid.
windows_browser() {
  exec "$app/chrome.exe" "$@" < <(cat)
}
# Writes the image $1 as the BMP $2, as sips does on the Mac.
to_bmp() {
  IN="$(cygpath -w "$1")" OUT="$(cygpath -w "$2")" powershell.exe -NoProfile -NonInteractive \
    -Command 'Add-Type -AssemblyName System.Drawing
      $image = [System.Drawing.Image]::FromFile($env:IN)
      $image.Save($env:OUT, [System.Drawing.Imaging.ImageFormat]::Bmp)
      $image.Dispose()'
}
