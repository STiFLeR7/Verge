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
            delegate.stop()
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

        panel.setFrameOrigin(.zero)
        NotificationCenter.default.post(name: NSApplication.didChangeScreenParametersNotification, object: nil)
        XCTAssertEqual(panel.frame.maxX, (panel.screen ?? screen).visibleFrame.maxX, accuracy: 0.5)
    }

    func testSessionNavigationAndInactivity() {
        let delegate = App()
        delegate.applicationDidFinishLaunching(
            Notification(name: NSApplication.didFinishLaunchingNotification)
        )
        defer { delegate.stop() }
        delegate.tools = [Tool(
            name: "ChatGPT", state: "Working", activity: "Working", summary: "2 sessions",
            limits: [], sessions: [
                Session(id: "first", title: "First", lines: ["Model · fixture"]),
                Session(id: "second", title: "Second", lines: ["Model · fixture"]),
            ]
        )]

        delegate.openSessions()
        XCTAssertEqual(delegate.session, "first")
        delegate.previous()
        XCTAssertEqual(delegate.session, "second")
        delegate.next()
        XCTAssertEqual(delegate.session, "first")
        delegate.back()
        XCTAssertNil(delegate.session)

        delegate.lastInteraction = Date(timeIntervalSinceNow: -31)
        delegate.updateInactivity()
        XCTAssertTrue(delegate.collapsed)
        delegate.view.entered?()
        XCTAssertFalse(delegate.collapsed)
        delegate.tools[0] = Tool(name: "ChatGPT", state: "Waiting", activity: "Waiting", summary: nil, limits: [], sessions: [])
        delegate.lastInteraction = Date(timeIntervalSinceNow: -31)
        delegate.updateInactivity()
        XCTAssertFalse(delegate.collapsed)
        XCTAssertFalse(delegate.panel.isKeyWindow)
        XCTAssertFalse(delegate.panel.isMainWindow)
    }

    func testPanelFrameSupportsNegativeOriginDisplays() {
        let visible = NSRect(x: -1920, y: 23, width: 1920, height: 1057)
        let frame = panelFrame(in: visible, height: 320)
        XCTAssertEqual(frame.maxX, visible.maxX)
        XCTAssertEqual(frame.midY, visible.midY)
    }
}
