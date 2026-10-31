"""The app's top-level windows on Windows, for lib.sh's quit and what it says of an app that didn't.

`close PID` asks each browser window of process PID to close, as its close button does: the
top-level windows holding the UI's view (platform/windows/host.cc's BrowserUI). `list PID` prints
each top-level window of the process, with its class, title, state and whether it's a browser
window.
Usage: app-windows.py close|list PID"""

import ctypes
import ctypes.wintypes as w
import sys

# Git Bash's grep reads UTF-8 lines ending in \n alone.
sys.stdout.reconfigure(encoding="utf-8", newline="\n")
user32 = ctypes.windll.user32
WM_CLOSE = 0x0010
GW_OWNER = 4
EnumProc = ctypes.WINFUNCTYPE(w.BOOL, w.HWND, w.LPARAM)


def text(call, hwnd):
    buffer = ctypes.create_unicode_buffer(256)
    call(hwnd, buffer, len(buffer))
    return buffer.value


def holds_ui(hwnd):
    return bool(user32.FindWindowExW(hwnd, None, "BrowserUI", None))


def windows(pid):
    found = []

    def each(hwnd, _):
        owner = w.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
        if owner.value == pid:
            found.append(hwnd)
        return True

    # Front to back.
    user32.EnumWindows(EnumProc(each), 0)
    return found


def main():
    if len(sys.argv) != 3 or sys.argv[1] not in ("close", "list"):
        sys.exit(__doc__.splitlines()[-1])
    pid = int(sys.argv[2])
    for hwnd in windows(pid):
        browser = holds_ui(hwnd)
        if sys.argv[1] == "close":
            if browser:
                user32.PostMessageW(hwnd, WM_CLOSE, 0, 0)
            continue
        states = [
            "browser" if browser else "",
            "visible" if user32.IsWindowVisible(hwnd) else "hidden",
            "disabled" if not user32.IsWindowEnabled(hwnd) else "",
            "owned" if user32.GetWindow(hwnd, GW_OWNER) else "",
        ]
        print(
            '%#x %s "%s" %s'
            % (
                hwnd or 0,
                text(user32.GetClassNameW, hwnd),
                text(user32.GetWindowTextW, hwnd),
                " ".join(state for state in states if state),
            )
        )


main()
