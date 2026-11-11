import Foundation
import CoreGraphics
import ApplicationServices
import Carbon

// Restore the flags observed before our shortcut. Clearing every flag would
// release modifiers the user is still holding; retaining the injected flags
// on key-up can instead leave Option/Control/Shift active in the session.
func shortcutEvents(key: CGKeyCode, source: CGEventSource, baseline: CGEventFlags) -> (CGEvent, CGEvent)? {
    guard let down = CGEvent(keyboardEventSource: source, virtualKey: key, keyDown: true),
          let up = CGEvent(keyboardEventSource: source, virtualKey: key, keyDown: false) else { return nil }
    // The internal binding must match exactly even if another modifier was held.
    down.flags = [.maskControl, .maskAlternate, .maskShift]
    up.flags = baseline
    return (down, up)
}

func modifierSnapshot() -> [String: Any] {
    let keys: [(String, CGKeyCode, CGKeyCode)] = [
        ("control", 59, 62), ("option", 58, 61),
        ("shift", 56, 60), ("command", 55, 54)
    ]
    func state(_ source: CGEventSourceStateID) -> [String: Any] {
        var result: [String: Any] = [:]
        for (name, left, right) in keys {
            result[name] = [
                "left": CGEventSource.keyState(source, key: left),
                "right": CGEventSource.keyState(source, key: right)
            ]
        }
        // Per-key and aggregate flags may disagree when injected input is stale.
        let flags = CGEventSource.flagsState(source)
        result["flags"] = [
            "control": flags.contains(.maskControl), "option": flags.contains(.maskAlternate),
            "shift": flags.contains(.maskShift), "command": flags.contains(.maskCommand)
        ]
        return result
    }
    return ["hid": state(.hidSystemState), "session": state(.combinedSessionState)]
}

let action = CommandLine.arguments.dropFirst().first ?? "check"
if action == "self-test" {
    // Inspect real CGEvents without posting any input or asking for permissions.
    guard let source = CGEventSource(stateID: .privateState) else { exit(4) }
    for key: CGKeyCode in [79, 80] {
        for baseline: CGEventFlags in [[], [.maskCommand], [.maskAlternate, .maskShift], [.maskAlphaShift, .maskControl]] {
            guard let (down, up) = shortcutEvents(key: key, source: source, baseline: baseline) else { exit(4) }
            precondition(down.type == .keyDown && up.type == .keyUp)
            precondition(down.getIntegerValueField(.keyboardEventKeycode) == Int64(key))
            precondition(up.getIntegerValueField(.keyboardEventKeycode) == Int64(key))
            precondition(down.flags == [.maskControl, .maskAlternate, .maskShift])
            precondition(up.flags == baseline, "Shortcut must restore pre-existing modifier flags")
        }
    }
    print("mac shortcut modifier tests passed")
    exit(0)
}
if action == "watch-modifiers" {
    guard CommandLine.arguments.count == 3,
          let seconds = Int(CommandLine.arguments[2]), (1...60).contains(seconds) else { exit(2) }
    // Bounded, read-only polling. Never inspect ordinary keys or typed text.
    let deadline = ProcessInfo.processInfo.systemUptime + Double(seconds)
    var previous: Data?
    repeat {
        let modifiers = modifierSnapshot()
        let snapshot = try JSONSerialization.data(withJSONObject: modifiers, options: [.sortedKeys])
        if snapshot != previous {
            let output: [String: Any] = ["time": Date().timeIntervalSince1970, "modifiers": modifiers]
            let data = try JSONSerialization.data(withJSONObject: output, options: [.sortedKeys])
            print(String(data: data, encoding: .utf8)!)
            fflush(stdout)
            previous = snapshot
        }
        usleep(20000)
    } while ProcessInfo.processInfo.systemUptime < deadline
    exit(0)
}
if action == "screens" {
    var count: UInt32 = 0
    CGGetActiveDisplayList(0, nil, &count)
    var ids = [CGDirectDisplayID](repeating: 0, count: Int(count))
    CGGetActiveDisplayList(count, &ids, &count)
    let panels: [[String: Any]] = ids.prefix(Int(count)).map { id in
        let b = CGDisplayBounds(id)
        return ["id": String(id), "name": CGDisplayIsBuiltin(id) != 0 ? "内置屏幕" : "屏幕 \(id)", "internal": CGDisplayIsBuiltin(id) != 0,
                "bounds": ["x": b.origin.x, "y": b.origin.y, "width": b.width, "height": b.height]]
    }
    let data = try JSONSerialization.data(withJSONObject: panels)
    print(String(data: data, encoding: .utf8)!)
    exit(0)
}
if action == "check" {
    print(AXIsProcessTrusted() ? "true" : "false")
    exit(0)
}
if action == "diagnose" {
    // Read-only checks: never request permissions or post input events here.
    let status: [String: Any] = [
        "accessibility": AXIsProcessTrusted(),
        "postEventAccess": CGPreflightPostEventAccess(),
        "secureEventInput": IsSecureEventInputEnabled(),
        "modifiers": modifierSnapshot()
    ]
    let data = try JSONSerialization.data(withJSONObject: status)
    print(String(data: data, encoding: .utf8)!)
    exit(0)
}
if action == "pointer" {
    guard CommandLine.arguments.count == 4,
          let x = Double(CommandLine.arguments[2]), let y = Double(CommandLine.arguments[3]),
          x.isFinite, y.isFinite else { exit(2) }
    guard AXIsProcessTrusted() else { fputs("需要辅助功能权限才能定位鼠标\n", stderr); exit(3) }
    let target = CGPoint(x: x, y: y)
    guard CGWarpMouseCursorPosition(target) == .success else { fputs("无法定位鼠标\n", stderr); exit(6) }
    CGEvent(mouseEventSource: nil, mouseType: .mouseMoved, mouseCursorPosition: target, mouseButton: .left)?.post(tap: .cghidEventTap)
    print("positioned")
    exit(0)
}
guard action == "host" || action == "peer" else { fputs("Invalid shortcut target\n", stderr); exit(2) }
guard AXIsProcessTrusted() else { fputs("需要辅助功能权限才能发送切换快捷键\n", stderr); exit(3) }
let key: CGKeyCode = action == "host" ? 79 : 80 // F18 and F19
let baseline = CGEventSource.flagsState(.combinedSessionState)
guard let source = CGEventSource(stateID: .hidSystemState),
      let (down, up) = shortcutEvents(key: key, source: source, baseline: baseline) else { exit(4) }
down.post(tap: .cghidEventTap)
usleep(35000)
up.post(tap: .cghidEventTap)
print("sent")
