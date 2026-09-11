param(
    [Parameter(Mandatory)][string]$Executable,
    [string[]]$ArgumentList=@(),
    [double]$DurationMinutes=480,
    [int]$SampleSeconds=60,
    [string]$OutputPath=(Join-Path $PWD 'verge-soak.csv')
)
$ErrorActionPreference='Stop'
if($DurationMinutes -le 0 -or $SampleSeconds -le 0){throw 'DurationMinutes and SampleSeconds must be positive'}
if(!(Test-Path -LiteralPath $Executable -PathType Leaf)){throw "Executable not found: $Executable"}
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class VergeSoakNative {
  [DllImport("user32.dll")] public static extern uint GetGuiResources(IntPtr process,uint flags);
}
'@
$process=Start-Process -FilePath $Executable -ArgumentList $ArgumentList -WindowStyle Hidden -PassThru
$samples=@()
$started=[DateTime]::UtcNow
$deadline=$started.AddMinutes($DurationMinutes)
try {
    while([DateTime]::UtcNow -lt $deadline) {
        Start-Sleep -Seconds $SampleSeconds
        $process.Refresh()
        if($process.HasExited){throw "Process exited during soak with code $($process.ExitCode)"}
        $samples += [pscustomobject]@{
            timestamp=[DateTime]::UtcNow.ToString('o')
            rss_bytes=$process.WorkingSet64
            handles=$process.HandleCount
            gdi=[VergeSoakNative]::GetGuiResources($process.Handle,0)
            user=[VergeSoakNative]::GetGuiResources($process.Handle,1)
        }
        $samples | Export-Csv -NoTypeInformation -LiteralPath $OutputPath
    }
} finally {
    if(!$process.HasExited){$process | Stop-Process -Force}
}
if($samples.Count -lt 1){throw 'No soak samples were collected'}
$baselineIndex=[Math]::Min($samples.Count-1,[Math]::Floor(1800/$SampleSeconds))
$baseline=$samples[$baselineIndex]
$last=$samples[-1]
if($last.gdi-$baseline.gdi -gt 8 -or $last.user-$baseline.user -gt 8){throw 'GDI/USER handles exceeded the +8 release threshold'}
$tail=@($samples | Select-Object -Last 5)
$monotonic=$tail.Count -eq 5
for($i=1;$i -lt $tail.Count;$i++){$monotonic = $monotonic -and $tail[$i].rss_bytes -ge $tail[$i-1].rss_bytes}
if($monotonic -and $tail[-1].rss_bytes-$tail[0].rss_bytes -gt 25MB){throw 'Resident memory grew monotonically by more than 25 MiB'}
Write-Output "PASS: $($samples.Count) soak samples written to $OutputPath"
