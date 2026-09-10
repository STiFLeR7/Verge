param([string]$BinaryDirectory)
# Preserve the existing status line and hooks. Back up originals before writing.
$ErrorActionPreference='Stop'
$repo=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (!$BinaryDirectory) {$BinaryDirectory=Join-Path $repo 'target/debug'}
$BinaryDirectory=[IO.Path]::GetFullPath($BinaryDirectory)
foreach ($binary in @('verge.exe','verge-claude-hook.exe')) {
    if (!(Test-Path -LiteralPath (Join-Path $BinaryDirectory $binary) -PathType Leaf)) {
        throw "Missing $binary in $BinaryDirectory. Build both binaries before installing; no settings changed."
    }
}
$gate='"'+(Join-Path $BinaryDirectory 'verge-claude-hook.exe').Replace('\','/')+'"'
Write-Warning 'Direct approval gate: start Verge from this binary directory before using Claude. If Verge is stopped, unreachable, or a request times out, permission-gated calls are denied. Restart Claude after installation.'
$settingsPath=Join-Path $env:USERPROFILE '.claude/settings.json'
$statusPath=Join-Path $env:USERPROFILE '.claude/statusline-command.js'
$bridge=(Join-Path $repo 'scripts/claude-signal.cjs').Replace('\','/')
$settings=Get-Content -Raw -LiteralPath $settingsPath | ConvertFrom-Json
$gates=@($settings.hooks.PermissionRequest | ForEach-Object { $_.hooks } | Where-Object { $_.command -like '*verge-claude-hook.exe*' })
if ($gates | Where-Object { $_.command -ne $gate }) {throw 'A different Verge approval helper is already registered. Remove that registration before switching binary directories; no settings changed.'}
$expected='node "'+$statusPath.Replace('\','/')+'"'
if ($settings.statusLine.command -ne $expected) {throw 'Status-line command changed; inspect it before installing.'}
$source=[IO.File]::ReadAllText($statusPath)
$marker='// Verge metadata observer'
if (!$source.Contains($marker)) {
    $anchor='  const c = {'
    if (!$source.Contains($anchor)) {throw 'Status-line source changed; no edits made.'}
    Copy-Item -LiteralPath $statusPath -Destination ($statusPath+'.verge-backup') -ErrorAction Stop
    $source=$source.Replace($anchor,('  '+$marker+"`n"+'  try { require("'+$bridge+'").usage(data); } catch {}'+"`n"+$anchor))
    [IO.File]::WriteAllText($statusPath,$source,[Text.UTF8Encoding]::new($false))
}
if (!$settings.hooks) {$settings | Add-Member -NotePropertyName hooks -NotePropertyValue ([pscustomobject]@{}) -Force}
if (!$settings.statusLine.refreshInterval) {$settings.statusLine | Add-Member -NotePropertyName refreshInterval -NotePropertyValue 15 -Force}
$command='node "'+$bridge+'"'
foreach ($event in @('UserPromptSubmit','PreToolUse','PostToolUse','PostToolUseFailure','StopFailure','PermissionRequest','Stop','SessionEnd','Notification')) {
    $existing=@()
    if ($settings.hooks.PSObject.Properties.Name -contains $event) {$existing=@($settings.hooks.$event)}
    if ($existing | Where-Object { $_.hooks | Where-Object command -eq $command }) {continue}
    $entry=@{hooks=@(@{type='command';command=$command;timeout=2})}
    if ($event -eq 'Notification') {$entry.matcher='permission_prompt'}
    $settings.hooks | Add-Member -NotePropertyName $event -NotePropertyValue @($existing+$entry) -Force
}
# Keep metadata observation separate from the synchronous decision hook.
$existing=@($settings.hooks.PermissionRequest)
if (!($existing | Where-Object { $_.hooks | Where-Object command -eq $gate })) {
    $entry=@{hooks=@(@{type='command';command=$gate;timeout=130})}
    $settings.hooks | Add-Member -NotePropertyName PermissionRequest -NotePropertyValue @($existing+$entry) -Force
}
if (!(Test-Path -LiteralPath ($settingsPath+'.verge-backup'))) {Copy-Item -LiteralPath $settingsPath -Destination ($settingsPath+'.verge-backup')}
$tempPath=$settingsPath+'.verge-tmp'
[IO.File]::WriteAllText($tempPath,($settings|ConvertTo-Json -Depth 100),[Text.UTF8Encoding]::new($false))
Move-Item -LiteralPath $tempPath -Destination $settingsPath -Force
Write-Output 'Installed metadata hooks and the synchronous Verge approval gate; preserved the existing status line and other hooks. Original files have .verge-backup copies.'
