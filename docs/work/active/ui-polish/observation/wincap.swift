import AppKit
import CoreGraphics
import Foundation
import ImageIO
import ScreenCaptureKit
import UniformTypeIdentifiers

// Usage: wincap list | wincap shot PID OUT.png
// Captures only windows owned by the given process id (the synthetic BRN app).
let args = CommandLine.arguments
func fail(_ s: String) -> Never { FileHandle.standardError.write((s + "\n").data(using: .utf8)!); exit(1) }

_ = NSApplication.shared  // initialise the CoreGraphics session for capture
let sem = DispatchSemaphore(value: 0)
Task {
    do {
        let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: args.count >= 2 && args[1] != "listall")
        if args.count >= 2 && args[1] == "listall" {
            for d in content.displays { print("display", d.displayID, d.frame) }
            for w in content.windows where w.owningApplication?.applicationName == "brn-desktop" {
                print(w.windowID, w.owningApplication?.processID ?? -1, w.frame, w.isOnScreen, w.windowLayer, w.title ?? "")
            }
        }
        if args.count >= 2 && args[1] == "list" {
            for w in content.windows where (w.owningApplication?.applicationName ?? "").lowercased().contains("brn") {
                print(w.windowID, w.owningApplication?.processID ?? -1, w.owningApplication?.applicationName ?? "?", w.frame, w.title ?? "")
            }
        } else if args.count >= 4 && args[1] == "shot" {
            guard let pid = Int32(args[2]) else { fail("pid") }
            let candidates = content.windows.filter { ($0.owningApplication?.processID == pid || $0.windowID == CGWindowID(pid)) && $0.frame.width > 200 }
            guard let window = candidates.max(by: { $0.frame.width * $0.frame.height < $1.frame.width * $1.frame.height }) else { fail("no window for pid \(pid)") }
            let filter = SCContentFilter(desktopIndependentWindow: window)
            let config = SCStreamConfiguration()
            config.width = Int(window.frame.width * 2)
            config.height = Int(window.frame.height * 2)
            config.showsCursor = false
            let image = try await SCScreenshotManager.captureImage(contentFilter: filter, configuration: config)
            let url = URL(fileURLWithPath: args[3])
            guard let dest = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil) else { fail("dest") }
            CGImageDestinationAddImage(dest, image, nil)
            if !CGImageDestinationFinalize(dest) { fail("write") }
            print("saved", args[3], window.frame)
        } else {
            fail("usage")
        }
    } catch {
        fail("error: \(error)")
    }
    sem.signal()
}
sem.wait()
