import AppKit
import CoreText

struct Snapshot: Decodable {
    let version: Int
    let tools: [Tool]
}
struct Tool: Decodable {
    let name: String
    let state: String
    let activity: String?
    let summary: String?
    let limits: [Limit]
    let sessions: [Session]
}
struct Limit: Decodable { let name: String; let fraction: Double?; let reset: String }
struct Session: Decodable { let id: String; let title: String; let lines: [String] }

final class Panel: NSPanel {
    override var canBecomeKey: Bool { false }
    override var canBecomeMain: Bool { false }
}
final class HoverView: NSView {
    var entered: (() -> Void)?
    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        trackingAreas.forEach(removeTrackingArea)
        addTrackingArea(NSTrackingArea(rect: bounds, options: [.mouseEnteredAndExited, .mouseMoved, .activeAlways, .inVisibleRect], owner: self, userInfo: nil))
    }
    override func mouseEntered(with event: NSEvent) { entered?() }
    override func mouseMoved(with event: NSEvent) { entered?() }
}
final class App: NSObject, NSApplicationDelegate {
    let panel = Panel(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
    let view = HoverView()
    let stack = NSStackView()
    var tools: [Tool] = []
    var selected = 0
    var session: String? = nil
    var lastInteraction = Date()
    var collapsed = false
    var busy = false
    var timer: Timer?
    var status: NSStatusItem?
    var message = "Reading local sessions..."

    func applicationDidFinishLaunching(_ notification: Notification) {
        for name in ["Inter-Regular", "Inter-SemiBold"] {
            if let url = Bundle.main.url(forResource: name, withExtension: "ttf") {
                CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil)
            }
        }
        panel.level = .statusBar
        panel.collectionBehavior = [.canJoinAllSpaces, .stationary, .fullScreenAuxiliary]
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hidesOnDeactivate = false
        panel.hasShadow = false
        view.wantsLayer = true
        view.layer?.backgroundColor = NSColor.black.cgColor
        view.layer?.cornerRadius = 16
        panel.contentView = view
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 8
        stack.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(stack)
        NSLayoutConstraint.activate([stack.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 18), stack.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -18), stack.topAnchor.constraint(equalTo: view.topAnchor, constant: 16)])
        view.entered = { [weak self] in
            guard let self = self else { return }
            let wasCollapsed = self.collapsed; self.touch()
            if wasCollapsed { self.render() }
        }
        status = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        status?.button?.title = "Verge"
        let menu = NSMenu()
        menu.addItem(withTitle: "Quit Verge", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        status?.menu = menu
        render()
        panel.orderFrontRegardless()
        timer = Timer.scheduledTimer(withTimeInterval: 3, repeats: true) { [weak self] _ in self?.refresh() }
        refresh()
    }
    func touch() { lastInteraction = Date(); collapsed = false }
    func text(_ value: String, size: CGFloat = 13) {
        let label = NSTextField(wrappingLabelWithString: value)
        label.font = NSFont(name: "Inter-Regular", size: size) ?? NSFont.systemFont(ofSize: size)
        label.textColor = .white
        stack.addArrangedSubview(label)
    }
    func button(_ title: String, action: Selector) -> NSButton {
        let b = NSButton(title: title, target: self, action: action)
        b.font = NSFont(name: "Inter-Regular", size: 12) ?? NSFont.systemFont(ofSize: 12)
        b.bezelStyle = .rounded
        return b
    }
    func render() {
        stack.arrangedSubviews.forEach { stack.removeArrangedSubview($0); $0.removeFromSuperview() }
        guard let screen = panel.screen ?? NSScreen.main else { return }
        let frame = screen.visibleFrame
        let height: CGFloat = collapsed ? 6 : 320
        panel.setFrame(NSRect(x: frame.maxX - 340, y: frame.midY - height / 2, width: 340, height: height), display: true)
        stack.isHidden = collapsed
        view.layer?.backgroundColor = (collapsed ? NSColor.labelColor : NSColor.black).cgColor
        guard !collapsed else { return }
        guard !tools.isEmpty else { text(message); return }
        selected = min(selected, tools.count - 1)
        let tool = tools[selected]
        let providers = NSStackView()
        for (i, item) in tools.enumerated() {
            let b = button(item.name, action: #selector(selectTool(_:))); b.tag = i; providers.addArrangedSubview(b)
        }
        stack.addArrangedSubview(providers)
        text(tool.name, size: 20)
        if let id = session, let index = tool.sessions.firstIndex(where: { $0.id == id }) {
            text(tool.sessions[index].title)
            tool.sessions[index].lines.prefix(5).forEach { text($0) }
            let counter = NSTextField(labelWithString: "\(index + 1) / \(tool.sessions.count)")
            counter.textColor = .white
            counter.font = NSFont(name: "Inter-Regular", size: 12) ?? NSFont.systemFont(ofSize: 12)
            let nav = NSStackView(views: [button("Back", action: #selector(back)), button("\u{2039}", action: #selector(previous)), counter, button("\u{203a}", action: #selector(next))])
            stack.addArrangedSubview(nav)
        } else {
            text(tool.activity ?? "Unknown")
            for limit in tool.limits.prefix(2) {
                text(limit.name)
                let usage = limit.fraction.map { "\(Int($0 * 100))% used" } ?? "Usage unavailable"
                text("\(usage)  \(limit.reset)")
            }
            if !tool.sessions.isEmpty { stack.addArrangedSubview(button(tool.summary ?? "Sessions", action: #selector(openSessions))) }
        }
    }
    @objc func selectTool(_ sender: NSButton) { selected = sender.tag; session = nil; touch(); render() }
    @objc func back() { session = nil; touch(); render() }
    @objc func openSessions() {
        guard selected < tools.count else { return }
        session = tools[selected].sessions.first?.id; touch(); render()
    }
    @objc func previous() { move(-1) }
    @objc func next() { move(1) }
    func move(_ delta: Int) {
        guard selected < tools.count, !tools[selected].sessions.isEmpty else { return }
        let count = tools[selected].sessions.count
        let index = tools[selected].sessions.firstIndex(where: { $0.id == session }) ?? 0
        session = tools[selected].sessions[(index + delta + count) % count].id
        touch(); render()
    }
    func refresh() {
        collapsed = !tools.contains(where: { $0.state == "Waiting" }) && Date().timeIntervalSince(lastInteraction) >= 30
        render()
        guard !busy else { return }; busy = true
        DispatchQueue.global(qos: .utility).async {
            let process = Process()
            process.executableURL = Bundle.main.executableURL?.deletingLastPathComponent().appendingPathComponent("verge-state")
            let pipe = Pipe(); process.standardOutput = pipe; process.standardError = FileHandle.nullDevice
            var snapshot: Snapshot?
            do {
                try process.run()
                DispatchQueue.global().asyncAfter(deadline: .now() + 5) { if process.isRunning { process.terminate() } }
                let data = pipe.fileHandleForReading.readDataToEndOfFile()
                process.waitUntilExit()
                if process.terminationStatus == 0 { snapshot = try JSONDecoder().decode(Snapshot.self, from: data) }
            } catch { }
            let result = snapshot
            DispatchQueue.main.async {
                self.busy = false
                if let result = result, result.version == 1 {
                    let name = self.selected < self.tools.count ? self.tools[self.selected].name : nil
                    self.tools = result.tools
                    self.selected = self.tools.firstIndex(where: { $0.name == name }) ?? 0
                    if self.selected >= self.tools.count || !self.tools[self.selected].sessions.contains(where: { $0.id == self.session }) { self.session = nil }
                    if self.tools.contains(where: { $0.state == "Waiting" }) { self.touch() }
                    self.message = "No local sessions detected"
                } else { self.tools = []; self.message = "Local session reader unavailable" }
                self.render()
            }
        }
    }
}
// Build-host contract: decode the actual Rust bridge without opening a window.
if CommandLine.arguments.contains("--check-snapshot") {
    do {
        let snapshot = try JSONDecoder().decode(Snapshot.self, from: FileHandle.standardInput.readDataToEndOfFile())
        guard snapshot.version == 1 else { exit(1) }
        print("Presentation bridge decoded")
        exit(0)
    } catch { fputs("Invalid presentation snapshot: \(error)\n", stderr); exit(1) }
}
let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let delegate = App()
app.delegate = delegate
app.run()
