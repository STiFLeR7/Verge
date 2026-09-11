param(
    [Parameter(Mandatory)][string]$ArtifactDirectory,
    [Parameter(Mandatory)][string]$ExpectedVersion,
    [switch]$RequireWindowsSignature
)
$ErrorActionPreference='Stop'
$ArtifactDirectory=[IO.Path]::GetFullPath($ArtifactDirectory)
$archives=@{
    Windows=Join-Path $ArtifactDirectory 'verge-windows-x64.zip'
    Linux=Join-Path $ArtifactDirectory 'verge-linux-x86_64.tar.gz'
    macOS=Join-Path $ArtifactDirectory 'verge-macos-arm64.zip'
}
function Get-ArchiveListing([string]$Archive) {
    $listing=@(& tar -tf $Archive)
    if($LASTEXITCODE -ne 0){throw "Cannot inspect $Archive"}
    return @($listing | ForEach-Object { ($_ -replace '^[.][/\\]','') -replace '\\','/' })
}
foreach($entry in $archives.GetEnumerator()) {
    if(!(Test-Path -LiteralPath $entry.Value -PathType Leaf)){throw "Missing $($entry.Key) archive: $($entry.Value)"}
    $listing=Get-ArchiveListing $entry.Value
    if($listing -match '(?i)(\.credentials|\.claude|sessions[/\\]|e2e-|private|fixture)'){throw "Private/test content found in $($entry.Key) archive"}
}
function Read-ArchiveEntry([string]$Archive,[string[]]$Candidates) {
    foreach($candidate in $Candidates) {
        $value=& tar -xOf $Archive $candidate 2>$null
        if($LASTEXITCODE -eq 0){return ($value -join "`n").Trim()}
    }
    throw "Missing required entry in ${Archive}: $($Candidates -join ', ')"
}
$windows=$archives.Windows
$linux=$archives.Linux
$mac=$archives.macOS
foreach($item in @(
    @($windows,@('VERSION')),
    @($linux,@('./VERSION','VERSION')),
    @($mac,@('Verge.app/Contents/Resources/VERSION'))
)) {
    if((Read-ArchiveEntry $item[0] $item[1]) -ne $ExpectedVersion){throw "Wrong packaged version in $($item[0])"}
}
foreach($item in @(
    @($windows,@('LICENSE')),
    @($linux,@('./LICENSE','LICENSE')),
    @($mac,@('Verge.app/Contents/Resources/LICENSE'))
)) {
    if((Read-ArchiveEntry $item[0] $item[1]) -match 'No license is granted'){throw 'Release distribution terms have not been selected'}
}
$required=@{
    Windows=@('verge.exe','verge-state.exe','verge-claude-hook.exe','THIRD_PARTY_NOTICES.md','OFL.txt','scripts/install-claude-signals.ps1','scripts/claude-signal.cjs')
    Linux=@('verge','verge-state','THIRD_PARTY_NOTICES.md','OFL.txt')
    macOS=@('Verge.app/Contents/MacOS/verge-macos','Verge.app/Contents/MacOS/verge-state','Verge.app/Contents/Resources/THIRD_PARTY_NOTICES.md','Verge.app/Contents/Resources/OFL.txt')
}
foreach($entry in $archives.GetEnumerator()) {
    $listing=Get-ArchiveListing $entry.Value
    foreach($path in $required[$entry.Key]){if($listing -notcontains $path){throw "Missing $path in $($entry.Key) archive"}}
}
if($RequireWindowsSignature) {
    $temp=Join-Path ([IO.Path]::GetTempPath()) ('verge-release-'+[Guid]::NewGuid())
    try {
        Expand-Archive -LiteralPath $windows -DestinationPath $temp
        foreach($name in @('verge.exe','verge-state.exe','verge-claude-hook.exe')) {
            if((Get-AuthenticodeSignature (Join-Path $temp $name)).Status -ne 'Valid'){throw "$name does not have a valid Windows signature"}
        }
    } finally { if(Test-Path -LiteralPath $temp){Remove-Item -LiteralPath $temp -Recurse -Force} }
}
Write-Output "PASS: release archives contain version $ExpectedVersion, distribution terms, notices, and required binaries."
