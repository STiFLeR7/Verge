# Run after: cargo build --release -p verge-platform-windows --example visual_states
# Exercises only the fixture process we create. Restores the pointer and closes that process.
$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class VergePermissionWindowCheck {
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr w,uint m,IntPtr a,IntPtr b);
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
    public static IntPtr Find(int pid, string className) {
        IntPtr found=IntPtr.Zero;
        EnumWindows((w,p)=> { uint id; GetWindowThreadProcessId(w,out id); var name=new System.Text.StringBuilder(256); GetClassNameW(w,name,256); if(id==pid && name.ToString()==className) found=w; return true; },IntPtr.Zero);
        return found;
    }
}
'@
function Assert($condition, $message) { if (!$condition) { throw $message } }

$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
$exe = Join-Path $repo 'target/release/examples/visual_states.exe'
$dir = Join-Path $env:TEMP ('verge-permission-ui-' + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $dir | Out-Null
$previous = $env:VERGE_PREVIEW_DECISION_FILE
foreach ($mode in @('Deny','Dismiss','Approve','ReviewClose')) {
    $path = Join-Path $dir $mode
    $env:VERGE_PREVIEW_DECISION_FILE = $path
    $fixture = Start-Process -FilePath $exe -ArgumentList '--open' -WindowStyle Hidden -PassThru
    $env:VERGE_PREVIEW_DECISION_FILE = $previous
    try {
        Start-Sleep -Milliseconds 1800
        $hwnd = [VergePermissionWindowCheck]::Find($fixture.Id, 'VergeAmbientSurface')
        Assert ($hwnd -ne [IntPtr]::Zero) 'Fixture overlay missing'
        Assert (!(Test-Path -LiteralPath $path)) 'Opening alone made a decision'
        if ($mode -eq 'Dismiss') {
            [void][VergePermissionWindowCheck]::PostMessageW($hwnd,0x100,[IntPtr]27,[IntPtr]::Zero)
        } else {
            if ($mode -ne 'Deny') {
                [void][VergePermissionWindowCheck]::PostMessageW($hwnd,0x100,[IntPtr]9,[IntPtr]::Zero)
                [void][VergePermissionWindowCheck]::PostMessageW($hwnd,0x100,[IntPtr]9,[IntPtr]::Zero)
            }
            [void][VergePermissionWindowCheck]::PostMessageW($hwnd,0x100,[IntPtr]13,[IntPtr]::Zero)
            if ($mode -ne 'Deny') {
                Start-Sleep -Milliseconds 500
                $review = [VergePermissionWindowCheck]::Find($fixture.Id,'VergePermissionReview')
                Assert ($review -ne [IntPtr]::Zero) 'Summarized action bypassed full review'
                Assert (!(Test-Path -LiteralPath $path)) 'Review entry approved before confirmation'
                if ($mode -eq 'Approve') {
                    [void][VergePermissionWindowCheck]::PostMessageW($review,0x111,[IntPtr]1,[IntPtr]::Zero)
                } else {
                    [void][VergePermissionWindowCheck]::PostMessageW($review,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
                }
            }
        }
        Start-Sleep -Milliseconds 500
        Assert (Test-Path -LiteralPath $path) "No result for $mode"
        $expected = if ($mode -eq 'ReviewClose') { 'Deny' } else { $mode }
        Assert ((Get-Content -LiteralPath $path -Raw) -eq $expected) "Wrong decision for $mode"
        Write-Output "PASS native permission UI: $mode; fixture request/session only"
    } finally { if (!$fixture.HasExited) { Stop-Process -Id $fixture.Id } }
}
