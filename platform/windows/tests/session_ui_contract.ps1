param([ValidateSet("Claude", "ChatGPT")][string]$Brand = "Claude")
# Run after: cargo build --release -p verge-platform-windows --example visual_states
# Exercises synthetic sessions in its own process; restores the pointer afterward.
$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class VergeSessionWindowCheck {
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr w,uint m,IntPtr a,IntPtr b);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags,uint x,uint y,uint data,UIntPtr extra);
    public delegate bool EnumProc(IntPtr hwnd, IntPtr param);
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left,Top,Right,Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X,Y; }
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr w);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr p);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr w,out uint p);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr w,out Rect r);
    [DllImport("user32.dll")] public static extern IntPtr GetWindowLongPtrW(IntPtr w,int index);
    [DllImport("user32.dll")] public static extern bool GetPhysicalCursorPos(out Point p);
    [DllImport("user32.dll")] public static extern bool SetPhysicalCursorPos(int x,int y);
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int n);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr w);
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] public static extern uint GetGuiResources(IntPtr process,uint flags);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr w,System.Text.StringBuilder name,int max);
    public static IntPtr Find(int pid, string className) {
        IntPtr found=IntPtr.Zero;
        EnumWindows((w,p)=> { uint id; GetWindowThreadProcessId(w,out id); var name=new System.Text.StringBuilder(256); GetClassNameW(w,name,256); if(id==pid && name.ToString()==className) found=w; return true; },IntPtr.Zero);
        return found;
    }
}
'@

[void][VergeSessionWindowCheck]::SetProcessDpiAwarenessContext([IntPtr](-4))
Add-Type -AssemblyName System.Drawing
$original=New-Object VergeSessionWindowCheck+Point
[void][VergeSessionWindowCheck]::GetPhysicalCursorPos([ref]$original)
[void][VergeSessionWindowCheck]::SetPhysicalCursorPos(100,100)
$repo=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
$out=Join-Path $repo ('docs/design/evidence/e2e-' + $Brand.ToLower())
[void](New-Item -ItemType Directory -Force -Path $out)
$app=Start-Process (Join-Path $repo 'target/release/examples/visual_states.exe') -ArgumentList 'sessions','--open',('--brand=' + $Brand) -WindowStyle Hidden -RedirectStandardError (Join-Path $env:TEMP 'verge-session-ui-error.log') -PassThru
function Capture($name) {
    # PowerShell can already have process DPI initialized; bind this thread to physical pixels.
    $previousDpi = [VergeSessionWindowCheck]::SetThreadDpiAwarenessContext([IntPtr](-4))
    $rect=New-Object VergeSessionWindowCheck+Rect
    $deadline=[DateTime]::UtcNow.AddSeconds(3)
    do {
        [void][VergeSessionWindowCheck]::GetWindowRect($hwnd,[ref]$rect)
        if (($rect.Right-$rect.Left) -ge [int][Math]::Round(300*$dpi)) { break }
        Start-Sleep -Milliseconds 50
    } while ([DateTime]::UtcNow -lt $deadline)
    if (($rect.Right-$rect.Left) -lt [int][Math]::Round(300*$dpi)) { throw 'Surface did not finish expanding' }
    Start-Sleep -Milliseconds 100
    $image=New-Object System.Drawing.Bitmap(($rect.Right-$rect.Left),($rect.Bottom-$rect.Top))
    $graphics=[System.Drawing.Graphics]::FromImage($image)
    try {
        [void][VergeSessionWindowCheck]::SetThreadDpiAwarenessContext([IntPtr](-4))
        $graphics.CopyFromScreen($rect.Left,$rect.Top,0,0,$image.Size)
        $image.Save((Join-Path $out $name),[System.Drawing.Imaging.ImageFormat]::Png)
    } finally { $graphics.Dispose(); $image.Dispose(); [void][VergeSessionWindowCheck]::SetThreadDpiAwarenessContext($previousDpi) }
}
function BodyHash($name) {
    $image=[System.Drawing.Bitmap]::FromFile((Join-Path $out $name))
    $crop=$image.Clone([System.Drawing.Rectangle]::new(
        [int][Math]::Round(16*$dpi),
        [int][Math]::Round(60*$dpi),
        [int][Math]::Round(220*$dpi),
        [int][Math]::Round(140*$dpi)
    ),$image.PixelFormat)
    $stream=[System.IO.MemoryStream]::new()
    try {
        $crop.Save($stream,[System.Drawing.Imaging.ImageFormat]::Png)
        return [Convert]::ToBase64String([System.Security.Cryptography.SHA256]::Create().ComputeHash($stream.ToArray()))
    } finally { $stream.Dispose(); $crop.Dispose(); $image.Dispose() }
}
function ClickFooter($fraction, $y) {
    $width = [int][Math]::Round(248 * $dpi)
    $left = [int][Math]::Round(16 * $dpi)
    $x = $left + [int][Math]::Round($width * $fraction)
    $point = ($y -shl 16) -bor $x
    [void][VergeSessionWindowCheck]::PostMessageW($hwnd,0x0201,[IntPtr]1,[IntPtr]$point)
    [void][VergeSessionWindowCheck]::PostMessageW($hwnd,0x0202,[IntPtr]0,[IntPtr]$point)
    Start-Sleep -Milliseconds 300
}
function Same($a,$b,$message) { if ((BodyHash $a) -ne (BodyHash $b)) { throw $message } }
function Different($a,$b,$message) { if ((BodyHash $a) -eq (BodyHash $b)) { throw $message } }
try {
    Start-Sleep -Seconds 2
    $hwnd=[VergeSessionWindowCheck]::Find($app.Id, 'VergeAmbientSurface')
    if ($hwnd -eq [IntPtr]::Zero) { throw 'Fixture did not open' }
    $dpi = [VergeSessionWindowCheck]::GetDpiForWindow($hwnd) / 96.0 * 0.9
    $footerDip = if ($Brand -eq "Claude") { 240 } else { 216 } # two usage rows versus one
    $footerY = [int][Math]::Round($footerDip * $dpi)
    Capture 'overview.png'
    [void][VergeSessionWindowCheck]::PostMessageW($hwnd,0x0100,[IntPtr]13,[IntPtr]0)
    Start-Sleep -Milliseconds 450
    Capture 'first.png'
    Different 'overview.png' 'first.png' 'Enter did not reveal session details'
    ClickFooter (11.0/12) $footerY
    Capture 'next.png'
    Different 'first.png' 'next.png' 'Right arrow did not switch session'
    ClickFooter (2.0/3) $footerY
    Capture 'counter.png'
    Same 'next.png' 'counter.png' 'Counter unexpectedly navigated'
    ClickFooter (5.0/12) $footerY
    Capture 'previous.png'
    Same 'first.png' 'previous.png' 'Left arrow did not restore first session'
    ClickFooter (5.0/12) $footerY
    Capture 'wrap.png'
    Same 'next.png' 'wrap.png' 'Previous did not wrap to last session'
    ClickFooter (11.0/12) $footerY
    Capture 'wrap-next.png'
    Same 'first.png' 'wrap-next.png' 'Next did not wrap to first session'
    ClickFooter 0.05 $footerY
    Capture 'back.png'
    Same 'overview.png' 'back.png' 'Back did not restore account view'
    [void][VergeSessionWindowCheck]::PostMessageW($hwnd,0x0100,[IntPtr]13,[IntPtr]0)
    Start-Sleep -Milliseconds 450
    [void][VergeSessionWindowCheck]::PostMessageW($hwnd,0x0100,[IntPtr]27,[IntPtr]0)
    Start-Sleep -Milliseconds 450
    Capture 'escape.png'
    Same 'overview.png' 'escape.png' 'Escape did not restore account view'
    Write-Output "PASS ${Brand}: Enter, mouse arrows, noninteractive counter, wrap, mouse Back, Escape; DPI=$dpi. Posted native input messages; synthetic sessions."

} finally {
    $app | Stop-Process -ErrorAction SilentlyContinue
    [void][VergeSessionWindowCheck]::SetPhysicalCursorPos($original.X,$original.Y)
}


