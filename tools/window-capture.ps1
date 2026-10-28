# Windows' counterpart of tools/headless.sh, for a CI runner, whose desktop is its own: runs a
# program, and once it has logged a line matching <ready> and the picture has stopped changing,
# captures the screen where its main window is, overlays over it included. Its log (stderr) goes
# beside the capture, and the program goes when it ends. -Moving lets that many pixels keep
# changing, such as a blinking caret. On a desktop someone uses, the capture holds whatever covers
# the window, so it's for CI only.
# Usage: window-capture.ps1 <output.png> <ready> <program> [-Moving <n>]
param(
  [Parameter(Mandatory)] [string] $Out,
  [Parameter(Mandatory)] [string] $Ready,
  [Parameter(Mandatory)] [string] $Program,
  [int] $Moving = 0
)
$ErrorActionPreference = 'Stop'
Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @'
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;

public static class Capture {
  [DllImport("user32.dll")] static extern bool SetProcessDPIAware();
  [DllImport("dwmapi.dll")] static extern int DwmGetWindowAttribute(IntPtr window, int attribute, out Rect rect, int size);
  [StructLayout(LayoutKind.Sequential)] struct Rect { public int Left, Top, Right, Bottom; }

  // Screen pixels, not the scaled points a DPI-unaware process gets.
  public static void Aware() { SetProcessDPIAware(); }

  // The window's frame as drawn, without the invisible borders it resizes by.
  public static Rectangle Bounds(IntPtr window) {
    Rect rect;
    Marshal.ThrowExceptionForHR(DwmGetWindowAttribute(window, 9, out rect, Marshal.SizeOf(typeof(Rect))));
    return Rectangle.FromLTRB(rect.Left, rect.Top, rect.Right, rect.Bottom);
  }

  public static Bitmap Screen(Rectangle area) {
    var picture = new Bitmap(area.Width, area.Height, PixelFormat.Format32bppArgb);
    using (var graphics = Graphics.FromImage(picture)) graphics.CopyFromScreen(area.Location, Point.Empty, area.Size);
    return picture;
  }

  static int[] Pixels(Bitmap picture) {
    var data = picture.LockBits(new Rectangle(Point.Empty, picture.Size), ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
    var pixels = new int[picture.Width * picture.Height];
    Marshal.Copy(data.Scan0, pixels, 0, pixels.Length);
    picture.UnlockBits(data);
    return pixels;
  }

  // How many pixels differ between two pictures of one size.
  public static int Differ(Bitmap a, Bitmap b) {
    int[] x = Pixels(a), y = Pixels(b);
    int count = 0;
    for (int i = 0; i < x.Length; i++) if (x[i] != y[i]) count++;
    return count;
  }

  // How many pixels differ from the first, which a window that hasn't drawn yet has everywhere.
  public static int Varied(Bitmap picture) {
    int[] pixels = Pixels(picture);
    int count = 0;
    foreach (int pixel in pixels) if (pixel != pixels[0]) count++;
    return count;
  }
}
'@
[Capture]::Aware()

$Out = [IO.Path]::GetFullPath($Out)
$log = [IO.Path]::ChangeExtension($Out, 'log')
New-Item -ItemType Directory -Force (Split-Path $Out) | Out-Null
$program = Start-Process -FilePath $Program -RedirectStandardError $log -PassThru -WindowStyle Normal
try {
  $deadline = (Get-Date).AddSeconds(30)
  while (-not (Select-String -Quiet -Path $log -Pattern $Ready -ErrorAction SilentlyContinue)) {
    if ($program.HasExited -or (Get-Date) -gt $deadline) {
      Get-Content $log -Tail 30 -ErrorAction SilentlyContinue | Write-Error -ErrorAction Continue
      throw "No `"$Ready`" in the program's log."
    }
    Start-Sleep -Milliseconds 100
  }
  $last = $null
  $taken = $false
  for ($try = 0; $try -lt 100 -and -not $taken; $try++) {
    $program.Refresh()
    if ($program.MainWindowHandle -ne 0) {
      $now = [Capture]::Screen([Capture]::Bounds($program.MainWindowHandle))
      if ($last -and $last.Size -eq $now.Size -and [Capture]::Differ($last, $now) -le $Moving -and
          [Capture]::Varied($now) -gt $Moving) {
        $now.Save($Out, [Drawing.Imaging.ImageFormat]::Png)
        $taken = $true
      }
      if ($last) { $last.Dispose() }
      $last = $now
    }
    Start-Sleep -Milliseconds 200
  }
  if (-not $taken) { throw 'The picture never stood still.' }
  $Out
} finally {
  if (-not $program.HasExited) { Stop-Process -Id $program.Id -Force -ErrorAction SilentlyContinue }
}
