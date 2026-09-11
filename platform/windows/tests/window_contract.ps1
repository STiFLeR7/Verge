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
    [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Auto)] public struct MonitorInfo { public int Size; public Rect Monitor; public Rect Work; public uint Flags; }
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr w);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr p);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr w,out uint p);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr w,out Rect r);
    [DllImport("user32.dll")] public static extern IntPtr GetWindowLongPtrW(IntPtr w,int index);
    [DllImport("user32.dll")] public static extern IntPtr MonitorFromWindow(IntPtr w,uint flags);
    [DllImport("user32.dll", CharSet=CharSet.Auto)] public static extern bool GetMonitorInfo(IntPtr monitor,ref MonitorInfo info);
    [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr w,int x,int y,int width,int height,bool repaint);
    [DllImport("user32.dll", EntryPoint="SendMessageW")] public static extern IntPtr SendMessage(IntPtr w,uint message,UIntPtr wp,IntPtr lp);
    [DllImport("user32.dll", EntryPoint="SendMessageW")] public static extern IntPtr SendMessageRect(IntPtr w,uint message,UIntPtr wp,ref Rect lp);
    [DllImport("user32.dll")] public static extern bool GetPhysicalCursorPos(out Point p);
    [DllImport("user32.dll")] public static extern bool SetPhysicalCursorPos(int x,int y);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr w);
    [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr w,uint command);
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int n);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr w);
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] public static extern uint GetGuiResources(IntPtr process,uint flags);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr w,System.Text.StringBuilder name,int max);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr w,System.Text.StringBuilder name,int max);
    public static IntPtr Find(int pid) {
        IntPtr found=IntPtr.Zero;
        EnumWindows((w,p)=> { uint id; GetWindowThreadProcessId(w,out id); var name=new System.Text.StringBuilder(256); GetClassNameW(w,name,256); if(id==pid && name.ToString()=="VergeAmbientSurface") found=w; return true; },IntPtr.Zero);
        return found;
    }
    public static IntPtr FindTitle(int pid,string title) {
        IntPtr found=IntPtr.Zero;
        EnumWindows((w,p)=> { uint id; GetWindowThreadProcessId(w,out id); var name=new System.Text.StringBuilder(256); GetWindowTextW(w,name,256); if(id==pid && name.ToString()==title) found=w; return true; },IntPtr.Zero);
        return found;
    }
    public static bool IsAbove(IntPtr upper, IntPtr lower) {
        for (var current=GetWindow(lower,3); current!=IntPtr.Zero; current=GetWindow(current,3)) if(current==upper) return true;
        return false;
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
    $windowStyle = [VergeWindowCheck]::GetWindowLongPtrW($hwnd,-16).ToInt64()
    Assert (($windowStyle -band 0x80000000L) -ne 0) 'Window is not WS_POPUP'
    Assert (($windowStyle -band 0x00C40000L) -eq 0) 'Window regained caption or resize-frame styles'
    Assert (($style -band 0x08080088) -eq 0x08080088) 'Lost topmost/layered/toolwindow/noactivate flags'
    Assert (($style -band 0x20) -ne 0) 'Outside must be WS_EX_TRANSPARENT'
    Assert ($rect.Right -eq $screen) 'Right edge detached from display'
    $monitor = [VergeWindowCheck]::MonitorFromWindow($hwnd,2)
    $monitorInfo = New-Object VergeWindowCheck+MonitorInfo
    $monitorInfo.Size = [Runtime.InteropServices.Marshal]::SizeOf($monitorInfo)
    Assert ([VergeWindowCheck]::GetMonitorInfo($monitor,[ref]$monitorInfo)) 'Could not read monitor work area'
    Assert ($rect.Right -eq $monitorInfo.Work.Right) 'Right edge detached from monitor work area'
    Assert ($rect.Top -ge $monitorInfo.Work.Top -and $rect.Bottom -le $monitorInfo.Work.Bottom) 'Surface escaped monitor work area'
    $dpi = [VergeWindowCheck]::GetDpiForWindow($hwnd)
    Assert ($compactWidth -eq [Math]::Floor(84*0.9*$dpi/96+0.5)) 'Compact width did not honor DPI'
     $ringY = $compactTop + [int][Math]::Round((44+28+26)*0.9*$dpi/96)
    $gdiBefore = [VergeWindowCheck]::GetGuiResources($fixture.Handle,0)
    $hoverCycles = if ($InactivityOnly) { 0 } else { 6 }
    for ($i=0; $i -lt $hoverCycles; $i++) {
        [void][VergeWindowCheck]::SetPhysicalCursorPos($screen-[int]($compactWidth/2),$ringY)
        $placed = New-Object VergeWindowCheck+Point
        [void][VergeWindowCheck]::GetPhysicalCursorPos([ref]$placed)
        Start-Sleep -Milliseconds 430
        [void][VergeWindowCheck]::GetWindowRect($hwnd,[ref]$rect)
        $at = New-Object VergeWindowCheck+Point
        [void][VergeWindowCheck]::GetPhysicalCursorPos([ref]$at)
        Assert ($at.X -eq $placed.X -and $at.Y -eq $placed.Y) "Surface moved the pointer: before $($placed.X),$($placed.Y); after $($at.X),$($at.Y)"
        Assert (($rect.Right-$rect.Left) -gt $compactWidth) 'Hover did not expand'
        Assert ($rect.Right -eq $screen -and $rect.Top -eq $compactTop) 'Expansion moved its screen anchor'
        $style = [VergeWindowCheck]::GetWindowLongPtrW($hwnd,-20).ToInt64()
        Assert (($style -band 0x20) -eq 0) 'Visible material must be interactive'
        [void][VergeWindowCheck]::SetPhysicalCursorPos(20,20)
        Start-Sleep -Milliseconds 800
        [void][VergeWindowCheck]::GetWindowRect($hwnd,[ref]$rect)
        Assert (($rect.Right-$rect.Left) -eq $compactWidth) 'Hover exit did not settle compact'
    }
    $fullscreenSource = @'
using System;
using System.Runtime.InteropServices;
public static class FullscreenWindow {
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern IntPtr CreateWindowExW(uint ex,string cls,string title,uint style,int x,int y,int width,int height,IntPtr parent,IntPtr menu,IntPtr instance,IntPtr param);
    [DllImport("user32.dll")] static extern int GetSystemMetrics(int metric);
    public static IntPtr Open() { return CreateWindowExW(0,"STATIC","VergeFullscreenFixture",0x90000000,0,0,GetSystemMetrics(0),GetSystemMetrics(1),IntPtr.Zero,IntPtr.Zero,IntPtr.Zero,IntPtr.Zero); }
}
'@
    $fullscreenScript = "Add-Type @'`n$fullscreenSource`n'@; [void][FullscreenWindow]::Open(); Start-Sleep -Seconds 60"
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($fullscreenScript))
    $fullscreenProcess = Start-Process -FilePath pwsh -ArgumentList '-NoProfile','-EncodedCommand',$encoded -WindowStyle Hidden -PassThru
    try {
        $deadline = [DateTime]::UtcNow.AddSeconds(5)
        do {
            Start-Sleep -Milliseconds 100
            $fullscreen = [VergeWindowCheck]::FindTitle($fullscreenProcess.Id,'VergeFullscreenFixture')
        } while ($fullscreen -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
        Assert ($fullscreen -ne [IntPtr]::Zero) 'Fullscreen fixture did not open'
        [void][VergeWindowCheck]::SetForegroundWindow($fullscreen)
        Start-Sleep -Milliseconds 3300
        Assert ([VergeWindowCheck]::IsAbove($hwnd,$fullscreen)) 'Verge dropped below fullscreen content'
    } finally {
        if (!$fullscreenProcess.HasExited) { Stop-Process -Id $fullscreenProcess.Id }
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
    [void][VergeWindowCheck]::SetPhysicalCursorPos([int](($monitorInfo.Work.Left+$monitorInfo.Work.Right)/2),[int](($monitorInfo.Work.Top+$monitorInfo.Work.Bottom)/2))
    Start-Sleep -Milliseconds 900
    [void][VergeWindowCheck]::GetWindowRect($hwnd,[ref]$rect)
    $movedWidth = $rect.Right-$rect.Left
    $movedHeight = $rect.Bottom-$rect.Top
    Assert ([VergeWindowCheck]::MoveWindow($hwnd,$monitorInfo.Work.Left,$monitorInfo.Work.Top,$movedWidth,$movedHeight,$true)) 'Could not move fixture before display-change check'
    [void][VergeWindowCheck]::SendMessage($hwnd,0x007E,[UIntPtr]::Zero,[IntPtr]::Zero)
    Start-Sleep -Milliseconds 250
    [void][VergeWindowCheck]::GetWindowRect($hwnd,[ref]$rect)
    Assert ($rect.Right -eq $monitorInfo.Work.Right) 'WM_DISPLAYCHANGE did not restore the work-area anchor'
    Assert ([VergeWindowCheck]::MoveWindow($hwnd,$monitorInfo.Work.Left,$monitorInfo.Work.Top,$movedWidth,$movedHeight,$true)) 'Could not move fixture before DPI-change check'
    $suggested = New-Object VergeWindowCheck+Rect
    $suggested.Left = $monitorInfo.Work.Left
    $suggested.Top = $monitorInfo.Work.Top
    $suggested.Right = $suggested.Left + $movedWidth
    $suggested.Bottom = $suggested.Top + $movedHeight
    $dpiWord = [UIntPtr]::new([uint64]($dpi -bor ($dpi -shl 16)))
    [void][VergeWindowCheck]::SendMessageRect($hwnd,0x02E0,$dpiWord,[ref]$suggested)
    Start-Sleep -Milliseconds 250
    [void][VergeWindowCheck]::GetWindowRect($hwnd,[ref]$rect)
    Assert ($rect.Right -eq $monitorInfo.Work.Right) 'WM_DPICHANGED did not restore the work-area anchor'
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





