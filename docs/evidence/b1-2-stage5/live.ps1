# Drives the interactive Explorer with posted Win32 mouse messages (left-drag
# orbit, wheel zoom) and captures the window with PrintWindow, without moving the
# real cursor. Usage (repo root, release build present):
#   powershell -File docs/evidence/b1-2-stage5/live.ps1 -Out <dir>
param([string]$Out, [string]$Vtk = "$env:USERPROFILE\Downloads\rodero2021\01-svf-order.vtk")
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force $Out | Out-Null
Add-Type -ReferencedAssemblies System.Drawing @'
using System; using System.Runtime.InteropServices; using System.Drawing; using System.Drawing.Imaging;
public static class W {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint f);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  public static void Shot(IntPtr h, string path) {
    RECT r; GetClientRect(h, out r);
    using (var bmp = new Bitmap(r.R - r.L, r.B - r.T)) {
      using (var g = Graphics.FromImage(bmp)) { var dc = g.GetHdc(); PrintWindow(h, dc, 3); g.ReleaseHdc(dc); }
      bmp.Save(path, ImageFormat.Png);
    }
  }
  public static IntPtr XY(int x, int y) { return (IntPtr)((y << 16) | (x & 0xFFFF)); }
}
'@
[W]::SetProcessDPIAware() | Out-Null
$exe = Join-Path $PSScriptRoot '..\..\..\target\release\svf-explorer.exe'
$p = Start-Process -PassThru -FilePath $exe -ArgumentList @(
  '--vtk', $Vtk, '--vtk-cell-axial', 'fibres', '--vtk-cell-tags', 'ID=1,2',
  '--vtk-scalar', 'RHO.dat', '--vtk-scalar-exclude', '-10', '--camera-yaw', '270', '--camera-zoom', '1.5')
$h = [IntPtr]::Zero
for ($i = 0; $i -lt 120 -and $h -eq [IntPtr]::Zero; $i++) { Start-Sleep -Milliseconds 500; $p.Refresh(); $h = $p.MainWindowHandle }
if ($h -eq [IntPtr]::Zero) { throw 'no window' }
Start-Sleep -Seconds 20   # let the mesh load and the first frames render
$n = 0
function Snap($tag) { $script:n++; [W]::Shot($h, ("{0}\{1:D2}-{2}.png" -f $Out, $script:n, $tag)); }
$WM_MOVE = 0x200; $WM_LDOWN = 0x201; $WM_LUP = 0x202; $WM_WHEEL = 0x20A; $MK_L = [IntPtr]1
$cx = 600; $cy = 450
function Drag([int]$dx, [int]$dy, [int]$steps, [string]$tag, [int]$every) {
  [W]::PostMessage($h, $WM_MOVE, [IntPtr]0, [W]::XY($cx, $cy)) | Out-Null; Start-Sleep -Milliseconds 100
  [W]::PostMessage($h, $WM_LDOWN, $MK_L, [W]::XY($cx, $cy)) | Out-Null; Start-Sleep -Milliseconds 100
  for ($s = 1; $s -le $steps; $s++) {
    [W]::PostMessage($h, $WM_MOVE, $MK_L, [W]::XY($cx + [int]($dx * $s / $steps), $cy + [int]($dy * $s / $steps))) | Out-Null
    Start-Sleep -Milliseconds 120
    if ($s % $every -eq 0) { Snap "$tag$s" }
  }
  [W]::PostMessage($h, $WM_LUP, [IntPtr]0, [W]::XY($cx + $dx, $cy + $dy)) | Out-Null; Start-Sleep -Milliseconds 200
}
function Wheel([int]$notches, [string]$tag) {
  $d = if ($notches -gt 0) { 120 } else { -120 }
  for ($k = 1; $k -le [math]::Abs($notches); $k++) {
    [W]::PostMessage($h, $WM_WHEEL, [IntPtr]($d -shl 16), [W]::XY(800, 500)) | Out-Null; Start-Sleep -Milliseconds 150
  }
  Start-Sleep -Milliseconds 300; Snap $tag
}
Snap 'start'
Drag 360 0 36 'orbitH' 6        # horizontal drag, snapshot every 6 steps
Drag 0 150 15 'orbitV' 5        # vertical drag
Drag 0 -150 15 'orbitVback' 15
1..6 | ForEach-Object { Wheel 1 "in$_" }
Drag 200 80 20 'zoomedOrbit' 10  # rotate while zoomed in
Wheel -6 'out6'; Wheel -12 'out18'
Wheel 12 'back'
$p.Refresh(); "alive=$(-not $p.HasExited) frames=$n"
Stop-Process -Id $p.Id -Force


