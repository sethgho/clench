import ClenchKit
import ClenchUI
import SwiftUI

#if os(iOS)
import UIKit

/// The share extension's entry point on iOS.
final class ShareViewController: UIViewController {
    override func viewDidLoad() {
        super.viewDidLoad()
        ClenchFonts.register()
        view.backgroundColor = .clear
        let model = ShareModel(context: extensionContext) { [weak self] url in
            await self?.open(url) ?? false
        }
        let host = UIHostingController(rootView: ShareView(model: model))
        host.view.backgroundColor = .clear
        addChild(host)
        host.view.frame = view.bounds
        host.view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        view.addSubview(host.view)
        host.didMove(toParent: self)
    }

    /// iOS lets only some extension points open URLs, and the share
    /// extension point is not documented as one of them, so this may report
    /// failure; the item is queued either way.
    private func open(_ url: URL) async -> Bool {
        guard let context = extensionContext else { return false }
        return await context.open(url)
    }
}

#elseif os(macOS)
import AppKit

/// The share extension's entry point on macOS.
final class ShareViewController: NSViewController {
    override func loadView() {
        ClenchFonts.register()
        let model = ShareModel(context: extensionContext) { url in
            NSWorkspace.shared.open(url)
        }
        let host = NSHostingView(rootView: ShareView(model: model))
        host.frame = NSRect(x: 0, y: 0, width: 540, height: 420)
        view = host
        preferredContentSize = host.fittingSize
    }
}
#endif
