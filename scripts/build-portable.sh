#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
targetdir="${CARGO_TARGET_DIR:-target}"
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)"
case "$(uname -s)" in
  Linux)
    cargo build --release -p verge-desktop --bin verge --bin verge-state
    rm -rf dist/linux
    mkdir -p dist/linux
    cp "$targetdir/release/verge" "$targetdir/release/verge-state" LICENSE THIRD_PARTY_NOTICES.md dist/linux/
    cp platform/windows/assets/fonts/Inter-*.ttf platform/windows/assets/fonts/OFL.txt dist/linux/
    printf '%s' "$version" > dist/linux/VERSION
    tar -czf dist/verge-linux-"$(uname -m)".tar.gz -C dist/linux .
    ;;
  Darwin)
    cargo build --release -p verge-desktop --bin verge-state
    swift build -c release --package-path ui/ambient/macos
    rm -rf dist/Verge.app
    app="dist/Verge.app/Contents"
    mkdir -p "$app/MacOS" "$app/Resources"
    swiftbin="$(swift build -c release --package-path ui/ambient/macos --show-bin-path)"
    "$targetdir/release/verge-state" | "$swiftbin/verge-macos" --check-snapshot
    "$swiftbin/verge-macos" --check-ui-contract
    cp "$swiftbin/verge-macos" "$targetdir/release/verge-state" "$app/MacOS/"
    cp platform/windows/assets/fonts/Inter-*.ttf platform/windows/assets/fonts/OFL.txt THIRD_PARTY_NOTICES.md LICENSE "$app/Resources/"
    printf '%s' "$version" > "$app/Resources/VERSION"
    cat > "$app/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>verge-macos</string>
<key>CFBundleIdentifier</key><string>app.verge.desktop</string>
<key>CFBundleName</key><string>Verge</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>1</string>
<key>CFBundleShortVersionString</key><string>VERSION_PLACEHOLDER</string>
<key>LSMinimumSystemVersion</key><string>13.0</string>
<key>LSUIElement</key><true/>
</dict></plist>
PLIST
    sed -i '' "s/VERSION_PLACEHOLDER/$version/" "$app/Info.plist"
    identity="${MACOS_SIGNING_IDENTITY:--}"
    if [[ "$identity" == "-" ]]; then
      codesign --force --deep --sign - dist/Verge.app
    else
      codesign --force --deep --options runtime --timestamp --sign "$identity" dist/Verge.app
    fi
    ditto -c -k --keepParent dist/Verge.app dist/verge-macos-"$(uname -m)".zip
    ;;
  *) echo "Use build-portable.ps1 on Windows" >&2; exit 1 ;;
esac
