param([switch]$InactivityOnly)
# Run after: cargo build --release -p verge-platform-windows --example visual_states
# Exercises only the fixture process we create. Restores the pointer and closes that process.
$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class VergeWindowCheck {
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
    [DllImport("user32.dll")] public static extern uint GetGuiResources(IntPtr process,uint flags);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr w,System.Text.StringBuilder name,int max);
    public static IntPtr Find(int pid) {
        IntPtr found=IntPtr.Zero;
        EnumWindows((w,p)=> { uint id; GetWindowThreadProcessId(w,out id); var name=new System.Text.StringBuilder(256); GetClassNameW(w,name,256); if(id==pid && name.ToString()=="VergeAmbientSurface") found=w; return true; },IntPtr.Zero);
        return found;
    }
}
'@
function Assert($condition, $message) { if (!$condition) { throw $message } }
[void][VergeWindowCheck]::SetProcessDpiAwarenessContext([IntPtr](-4))
$original = New-Object VergeWindowCheck+Point
[void][VergeWindowCheck]::GetPhysicalCursorPos([ref]$original)
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
$exe = Join-Path $repo 'target/release/examples/visual_states.exe'
$previousDelay = $env:VERGE_PREVIEW_DELAY_MS
$env:VERGE_PREVIEW_DELAY_MS = '1800'
$fixture = Start-Process -FilePath $exe -ArgumentList 'working' -WindowStyle Hidden -PassThru
$env:VERGE_PREVIEW_DELAY_MS = $previousDelay
try {
    Start-Sleep -Milliseconds 1500
    $hwnd = [VergeWindowCheck]::Find($fixture.Id)
    Assert ($hwnd -ne [IntPtr]::Zero) 'Fixture window was not created'
    Assert ([VergeWindowCheck]::IsWindowVisible($hwnd)) 'The overlay is hidden by console startup flags'
    $screen = [VergeWindowCheck]::GetSystemMetrics(0)
    $rect = New-Object VergeWindowCheck+Rect
    [void][VergeWindowCheck]::SetPhysicalCursorPos(20,20)
    Start-Sleep -Milliseconds 550
    [void][VergeWindowCheck]::GetWindowRect($hwnd,[ref]$rect)
    $compactWidth = $rect.Right-$rect.Left
    $compactTop = $rect.Top
    $style = [VergeWindowCheck]::GetWindowLongPtrW($hwnd,-20).ToInt64()
    Assert (($style -band 0x08080088) -eq 0x08080088) 'Lost topmost/layered/toolwindow/noactivate flags'
    Assert (($style -band 0x20) -ne 0) 'Outside must be WS_EX_TRANSPARENT'
    Assert ($rect.Right -eq $screen) 'Right edge detached from display'
    $dpi = [VergeWindowCheck]::GetDpiForWindow($hwnd)
    Assert ($compactWidth -eq [Math]::Floor(84*0.9*$dpi/96+0.5)) 'Compact width did not honor DPI'
     $ringY = $compactTop + [int][Math]::Round((44+28+26)*0.9*$dpi/96)
    $gdiBefore = [VergeWindowCheck]::GetGuiResources($fixture.Handle,0)
    $hoverCycles = if ($InactivityOnly) { 0 } else { 6 }
    for ($i=0; $i -lt $hoverCycles; $i++) {
        [void][VergeWindowCheck]::SetPhysicalCursorPos($screen-[int]($compactWidth/2),$ringY)
        Start-Sleep -Milliseconds 430
        [void][VergeWindowCheck]::GetWindowRect($hwnd,[ref]$rect)
        $at = New-Object VergeWindowCheck+Point
        [void][VergeWindowCheck]::GetPhysicalCursorPos([ref]$at)
        Assert ($at.X -eq $screen-[int]($compactWidth/2) -and $at.Y -eq $ringY) "Pointer changed: expected $($screen-[int]($compactWidth/2)),$ringY; actual $($at.X),$($at.Y). Retry while the pointer is still"
        Assert (($rect.Right-$rect.Left) -gt $compactWidth) 'Hover did not expand'
        Assert ($rect.Right -eq $screen -and $rect.Top -eq $compactTop) 'Expansion moved its screen anchor'
        $style = [VergeWindowCheck]::GetWindowLongPtrW($hwnd,-20).ToInt64()
        Assert (($style -band 0x20) -eq 0) 'Visible material must be interactive'
        [void][VergeWindowCheck]::SetPhysicalCursorPos(20,20)
        Start-Sleep -Milliseconds 800
        [void][VergeWindowCheck]::GetWindowRect($hwnd,[ref]$rect)
        Assert (($rect.Right-$rect.Left) -eq $compactWidth) 'Hover exit did not settle compact'
    }
    $gdiAfter = [VergeWindowCheck]::GetGuiResources($fixture.Handle,0)
    Assert ($gdiAfter -le $gdiBefore+4) "GDI objects leaked: $gdiBefore to $gdiAfter"
    # Real 30-second timer while the source continues reporting Working.
    $idleWidth = [int][Math]::Floor(5*0.9*$dpi/96+0.5)
    $deadline = [DateTime]::UtcNow.AddSeconds(35)
    do {
        Start-Sleep -Milliseconds 250
        [void][VergeWindowCheck]::GetWindowRect($hwnd,[ref]$rect)
    } while (($rect.Right-$rect.Left) -ne $idleWidth -and [DateTime]::UtcNow -lt $deadline)
    Assert (($rect.Right-$rect.Left) -eq $idleWidth) 'Working surface did not collapse after inactivity'
    [void][VergeWindowCheck]::SetPhysicalCursorPos($rect.Right-2,[int](($rect.Top+$rect.Bottom)/2))
    Start-Sleep -Milliseconds 430
    [void][VergeWindowCheck]::GetWindowRect($hwnd,[ref]$rect)
    Assert (($rect.Right-$rect.Left) -gt $idleWidth) 'Hover did not wake the idle bar'
    [void][VergeWindowCheck]::SetPhysicalCursorPos(20,20)
    Write-Output "PASS: working-session inactivity collapses to $idleWidth px; physical hover wakes it."
    Stop-Process -Id $fixture.Id
    $foreground=[VergeWindowCheck]::GetForegroundWindow()
    $fixture=Start-Process -FilePath $exe -ArgumentList 'reminder' -WindowStyle Hidden -PassThru
    Start-Sleep -Milliseconds 1700
    $hwnd=[VergeWindowCheck]::Find($fixture.Id)
    [void][VergeWindowCheck]::GetWindowRect($hwnd,[ref]$rect)
    Assert (($rect.Right-$rect.Left) -gt $compactWidth) 'Usage reminder did not reveal itself'
    Assert ([VergeWindowCheck]::GetForegroundWindow() -eq $foreground) 'Reminder stole keyboard focus'
    Write-Output "PASS: DPI=$dpi, edge=$screen, compact=$compactWidth px; $hoverCycles expansion/collapse cycles during 1800 ms data delays; reminder opens without focus; dynamic click-through; GDI objects $gdiBefore -> $gdiAfter."
} finally {
    [void][VergeWindowCheck]::SetPhysicalCursorPos($original.X,$original.Y)
    if (!$fixture.HasExited) { Stop-Process -Id $fixture.Id }
}





