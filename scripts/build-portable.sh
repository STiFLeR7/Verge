#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
targetdir="${CARGO_TARGET_DIR:-target}"
case "$(uname -s)" in
  Linux)
    cargo build --release -p verge-desktop --bin verge --bin verge-state
    mkdir -p dist/linux
    cp "$targetdir/release/verge" "$targetdir/release/verge-state" LICENSE THIRD_PARTY_NOTICES.md dist/linux/
    cp platform/windows/assets/fonts/Inter-*.ttf platform/windows/assets/fonts/OFL.txt dist/linux/
    tar -czf dist/verge-linux-"$(uname -m)".tar.gz -C dist/linux .
    ;;
  Darwin)
    cargo build --release -p verge-desktop --bin verge-state
    swift build -c release --package-path ui/ambient/macos
    app="dist/Verge.app/Contents"
    mkdir -p "$app/MacOS" "$app/Resources"
    swiftbin="$(swift build -c release --package-path ui/ambient/macos --show-bin-path)"
    "$targetdir/release/verge-state" | "$swiftbin/verge-macos" --check-snapshot
    cp "$swiftbin/verge-macos" "$targetdir/release/verge-state" "$app/MacOS/"
    cp platform/windows/assets/fonts/Inter-*.ttf platform/windows/assets/fonts/OFL.txt THIRD_PARTY_NOTICES.md LICENSE "$app/Resources/"
    cat > "$app/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>verge-macos</string>
<key>CFBundleIdentifier</key><string>app.verge.desktop</string>
<key>CFBundleName</key><string>Verge</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>1</string>
<key>LSMinimumSystemVersion</key><string>13.0</string>
<key>LSUIElement</key><true/>
</dict></plist>
PLIST
    codesign --force --deep --sign - dist/Verge.app
    ditto -c -k --keepParent dist/Verge.app dist/verge-macos-"$(uname -m)".zip
    ;;
  *) echo "Use build-portable.ps1 on Windows" >&2; exit 1 ;;
esac
