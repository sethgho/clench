import SwiftUI

/// The fold of a two-screen phone, from SwiftUI's `onHingeChange` (iOS 27.1
/// and later). Devices without a hinge, and older systems, report
/// ``unknown``; layouts then follow the window's size alone.
enum HingePosture: Equatable {
    case unknown, closed, partiallyOpen, fullyOpen
}

extension View {
    /// Keeps `posture` in step with the device's hinge.
    func observeHinge(_ posture: Binding<HingePosture>) -> some View {
        modifier(HingeObserver(posture: posture))
    }
}

private struct HingeObserver: ViewModifier {
    @Binding var posture: HingePosture

    func body(content: Content) -> some View {
        #if os(iOS)
        if #available(iOS 27.1, *) {
            content.onHingeChange { _, context in
                posture = Self.posture(context.hinge)
            }
        } else {
            content
        }
        #else
        content
        #endif
    }

    #if os(iOS)
    @available(iOS 27.1, *)
    private static func posture(_ hinge: DeviceHinge?) -> HingePosture {
        guard let hinge else { return .unknown }
        switch hinge.status {
        case .closed: return .closed
        case .partiallyOpen: return .partiallyOpen
        case .fullyOpen: return .fullyOpen
        default: return .unknown
        }
    }
    #endif
}
