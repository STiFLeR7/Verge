import AppKit
import XCTest
@testable import verge_macos

@MainActor
final class NativePanelContract: XCTestCase {
    func testPanelUsesTheV1WindowContract() throws {
        let delegate = App()
        delegate.applicationDidFinishLaunching(
            Notification(name: NSApplication.didFinishLaunchingNotification)
        )
        defer {
            delegate.timer?.invalidate()
            delegate.panel.orderOut(nil)
        }

        let panel = delegate.panel
        let screen = try XCTUnwrap(panel.screen ?? NSScreen.main)
        XCTAssertTrue(panel.styleMask.contains(.borderless))
        XCTAssertTrue(panel.styleMask.contains(.nonactivatingPanel))
        XCTAssertEqual(panel.level, .statusBar)
        XCTAssertTrue(panel.collectionBehavior.contains(.canJoinAllSpaces))
        XCTAssertTrue(panel.collectionBehavior.contains(.fullScreenAuxiliary))
        XCTAssertFalse(panel.isOpaque)
        XCTAssertEqual(panel.backgroundColor, .clear)
        XCTAssertFalse(panel.hasShadow)
        XCTAssertFalse(panel.canBecomeKey)
        XCTAssertFalse(panel.canBecomeMain)
        XCTAssertEqual(panel.frame.maxX, screen.visibleFrame.maxX, accuracy: 0.5)
        XCTAssertEqual(panel.frame.midY, screen.visibleFrame.midY, accuracy: 0.5)
    }
}
