import AppKit
import ApplicationServices
import CoreGraphics
import Foundation

// Synthetic-app input helper. Usage:
//   input trusted
//   input activate PID
//   input click X Y            (global screen points)
//   input dclick X Y
//   input rclick X Y
//   input key KEYCODE [cmd,shift,alt,ctrl]
//   input type TEXT
//   input scroll X Y DY
//   input resize PID W H       (via Accessibility)
let a = CommandLine.arguments
_ = NSApplication.shared
func post(_ e: CGEvent?) { e?.post(tap: .cghidEventTap); usleep(30_000) }
func flags(_ s: String) -> CGEventFlags {
    var f: CGEventFlags = []
    for p in s.split(separator: ",") {
        switch p {
        case "cmd": f.insert(.maskCommand)
        case "shift": f.insert(.maskShift)
        case "alt": f.insert(.maskAlternate)
        case "ctrl": f.insert(.maskControl)
        default: break
        }
    }
    return f
}
switch a[1] {
case "trusted":
    print("AX trusted:", AXIsProcessTrusted())
case "activate":
    let app = NSRunningApplication(processIdentifier: pid_t(a[2])!)
    print("activate:", app?.activate(options: [.activateAllWindows]) ?? false)
    usleep(300_000)
case "click", "dclick", "rclick":
    let p = CGPoint(x: Double(a[2])!, y: Double(a[3])!)
    let right = a[1] == "rclick"
    let down: CGEventType = right ? .rightMouseDown : .leftMouseDown
    let up: CGEventType = right ? .rightMouseUp : .leftMouseUp
    let button: CGMouseButton = right ? .right : .left
    post(CGEvent(mouseEventSource: nil, mouseType: .mouseMoved, mouseCursorPosition: p, mouseButton: .left))
    let clicks = a[1] == "dclick" ? 2 : 1
    for n in 1...clicks {
        let d = CGEvent(mouseEventSource: nil, mouseType: down, mouseCursorPosition: p, mouseButton: button)
        d?.setIntegerValueField(.mouseEventClickState, value: Int64(n))
        post(d)
        let u = CGEvent(mouseEventSource: nil, mouseType: up, mouseCursorPosition: p, mouseButton: button)
        u?.setIntegerValueField(.mouseEventClickState, value: Int64(n))
        post(u)
    }
case "move":
    post(CGEvent(mouseEventSource: nil, mouseType: .mouseMoved, mouseCursorPosition: CGPoint(x: Double(a[2])!, y: Double(a[3])!), mouseButton: .left))
case "key":
    let code = CGKeyCode(a[2])!
    let f = a.count > 3 ? flags(a[3]) : []
    let d = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: true); d?.flags = f; post(d)
    let u = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: false); u?.flags = f; post(u)
case "type":
    for ch in a[2].utf16 {
        var c = ch
        let d = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: true); d?.keyboardSetUnicodeString(stringLength: 1, unicodeString: &c); post(d)
        let u = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: false); u?.keyboardSetUnicodeString(stringLength: 1, unicodeString: &c); post(u)
    }
case "scroll":
    post(CGEvent(mouseEventSource: nil, mouseType: .mouseMoved, mouseCursorPosition: CGPoint(x: Double(a[2])!, y: Double(a[3])!), mouseButton: .left))
    post(CGEvent(scrollWheelEvent2Source: nil, units: .pixel, wheelCount: 1, wheel1: Int32(a[4])!, wheel2: 0, wheel3: 0))
case "resize":
    let app = AXUIElementCreateApplication(pid_t(a[2])!)
    var value: CFTypeRef?
    AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &value)
    guard let windows = value as? [AXUIElement], let w = windows.first else { print("no AX window"); exit(1) }
    var size = CGSize(width: Double(a[3])!, height: Double(a[4])!)
    var origin = CGPoint(x: 70, y: 70)
    let pv = AXValueCreate(.cgPoint, &origin)!
    let sv = AXValueCreate(.cgSize, &size)!
    print("pos:", AXUIElementSetAttributeValue(w, kAXPositionAttribute as CFString, pv).rawValue,
          "size:", AXUIElementSetAttributeValue(w, kAXSizeAttribute as CFString, sv).rawValue)
default:
    print("usage")
}
