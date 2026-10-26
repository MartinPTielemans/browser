#!/bin/sh
# Screenshots one process's main window with its child windows (menus, the command bar) drawn
# over it, as on screen, but nothing from other apps. A screen capture would include whatever
# else is open on the owner's Mac. With "all", it takes every window of the process on that display,
# such as two browser windows, in one picture.
# Usage: tools/window-screenshot.sh <pid> <output.png> [all]
set -e
[ $# -eq 2 ] || [ $# -eq 3 ] && [ "${3:-all}" = all ] ||
  { echo "usage: $0 <pid> <output.png> [all]" >&2; exit 2; }
PID="$1" OUT="$2" ALL="${3:-}" swift - <<'SWIFT'
import Foundation
import ImageIO
import ScreenCaptureKit
import UniformTypeIdentifiers

let pid = Int32(ProcessInfo.processInfo.environment["PID"]!)!
let out = URL(fileURLWithPath: ProcessInfo.processInfo.environment["OUT"]!)
let all = ProcessInfo.processInfo.environment["ALL"] == "all"

func fail(_ message: String) -> Never {
  FileHandle.standardError.write(Data((message + "\n").utf8))
  exit(1)
}

let content = try await SCShareableContent.excludingDesktopWindows(true, onScreenWindowsOnly: true)
let windows = content.windows.filter { $0.owningApplication?.processID == pid }
// The largest normal window; a process can also own invisible helper windows.
guard let main = windows.filter({ $0.windowLayer == 0 })
  .max(by: { $0.frame.width * $0.frame.height < $1.frame.width * $1.frame.height })
else { fail("process \(pid) has no window on screen") }
guard let display = content.displays.first(where: { $0.frame.intersects(main.frame) })
else { fail("no display shows the window") }

let frame = all
  ? windows.filter { $0.windowLayer == 0 && $0.frame.intersects(display.frame) }
    .reduce(main.frame) { $0.union($1.frame) }.intersection(display.frame)
  : main.frame
let filter = SCContentFilter(display: display, including: windows)
let config = SCStreamConfiguration()
config.sourceRect = frame.offsetBy(dx: -display.frame.minX, dy: -display.frame.minY)
config.width = Int(frame.width * CGFloat(filter.pointPixelScale))
config.height = Int(frame.height * CGFloat(filter.pointPixelScale))
config.showsCursor = false
let image: CGImage = try await SCScreenshotManager.captureImage(contentFilter: filter, configuration: config)

guard let file = CGImageDestinationCreateWithURL(out as CFURL, UTType.png.identifier as CFString, 1, nil)
else { fail("can't write \(out.path)") }
CGImageDestinationAddImage(file, image, nil)
guard CGImageDestinationFinalize(file) else { fail("can't write \(out.path)") }
SWIFT
