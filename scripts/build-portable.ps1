$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    cargo build --release -p verge-desktop --bin verge --bin verge-state --bin verge-claude-hook
    if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
    New-Item -ItemType Directory -Force dist/windows | Out-Null
    Copy-Item target/release/verge.exe,target/release/verge-state.exe,target/release/verge-claude-hook.exe,LICENSE dist/windows/
    Copy-Item THIRD_PARTY_NOTICES.md dist/windows/
    Copy-Item platform/windows/assets/fonts/OFL.txt dist/windows/
    Compress-Archive -Path dist/windows/* -DestinationPath dist/verge-windows-x64.zip -Force
} finally { Pop-Location }
