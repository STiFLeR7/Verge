// swift-tools-version: 5.9
import PackageDescription
let package = Package(name: "VergeMacOS", platforms: [.macOS(.v13)], targets: [
    .executableTarget(name: "verge-macos", path: "Sources")
])
