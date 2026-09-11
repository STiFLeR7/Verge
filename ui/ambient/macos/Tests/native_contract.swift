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
        XCTAssertTrue(panel.collectionBehavior.contains(.stationary))
        XCTAssertTrue(panel.collectionBehavior.contains(.fullScreenAuxiliary))
        XCTAssertFalse(panel.isOpaque)
        XCTAssertEqual(panel.backgroundColor, .clear)
        XCTAssertFalse(panel.hasShadow)
        XCTAssertFalse(panel.hidesOnDeactivate)
        XCTAssertFalse(panel.isMovable)
        XCTAssertFalse(panel.isMovableByWindowBackground)
        XCTAssertTrue(panel.becomesKeyOnlyIfNeeded)
        XCTAssertFalse(panel.isReleasedWhenClosed)
        XCTAssertFalse(panel.canBecomeKey)
        XCTAssertFalse(panel.canBecomeMain)
        XCTAssertEqual(panel.frame.maxX, screen.visibleFrame.maxX, accuracy: 0.5)
        XCTAssertEqual(panel.frame.midY, screen.frame.midY, accuracy: 0.5)

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
        let full = NSRect(x: -1920, y: 0, width: 1920, height: 1080)
        let visible = NSRect(x: -1920, y: 23, width: 1920, height: 1017)
        let frame = panelFrame(in: visible, fullFrame: full, height: 320)
        XCTAssertEqual(frame.maxX, visible.maxX)
        XCTAssertEqual(frame.midY, full.midY)
    }

    func testDockChangesDoNotShiftTheSidePanel() {
        let full = NSRect(x: 0, y: 0, width: 1512, height: 982)
        let hiddenDock = NSRect(x: 0, y: 24, width: 1512, height: 958)
        let shownDock = NSRect(x: 0, y: 88, width: 1512, height: 894)
        let hidden = panelFrame(in: hiddenDock, fullFrame: full, height: 320)
        let shown = panelFrame(in: shownDock, fullFrame: full, height: 320)
        XCTAssertEqual(hidden.midY, shown.midY)
        XCTAssertEqual(hidden.maxX, shown.maxX)
    }

    func testFractionalScreenFrameStillLandsFlushOnWholePoints() {
        let full = NSRect(x: 0.25, y: 0.25, width: 1511.5, height: 981.5)
        let visible = NSRect(x: 0.25, y: 23.75, width: 1511.5, height: 957.75)
        let frame = panelFrame(in: visible, fullFrame: full, height: 319.2)
        XCTAssertEqual(frame.maxX, visible.maxX.rounded(), accuracy: 0.001)
        for value in [frame.minX, frame.minY, frame.width, frame.height] {
            XCTAssertEqual(value, value.rounded())
        }
    }
}
