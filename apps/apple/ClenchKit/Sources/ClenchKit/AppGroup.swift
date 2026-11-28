import Foundation

/// Where the app and its share extension keep shared state.
///
/// The App Group identifier comes from the `ClenchAppGroup` key of the
/// running bundle's Info.plist, which the Xcode project fills from the
/// `CLENCH_APP_GROUP` build setting. When the group container is not
/// available (an unsigned build, a missing entitlement), a private
/// directory in Application Support is used instead, so the app still
/// works on its own but the share extension cannot reach its queue.
public enum AppGroup {
    public static let infoPlistKey = "ClenchAppGroup"

    public static var identifier: String? {
        guard let value = Bundle.main.object(forInfoDictionaryKey: infoPlistKey) as? String,
              !value.isEmpty, !value.contains("$(")
        else { return nil }
        return value
    }

    /// The shared container, or nil when this build has no App Group.
    public static var sharedContainer: URL? {
        guard let identifier else { return nil }
        return FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: identifier)
    }

    /// The directory holding the queue: the shared container when there is
    /// one, otherwise a private fallback.
    public static func storageDirectory() throws -> URL {
        // On macOS the container URL comes back even for a group the
        // process is not entitled to (an ad-hoc build), and the sandbox
        // then refuses to write there, so the shared directory is used only
        // once it could be created.
        if let shared = sharedContainer {
            let dir = shared.appending(path: "Library/Application Support/clench", directoryHint: .isDirectory)
            if (try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)) != nil,
               FileManager.default.isWritableFile(atPath: dir.path(percentEncoded: false)) {
                return dir
            }
        }
        let base = try FileManager.default.url(
            for: .applicationSupportDirectory, in: .userDomainMask,
            appropriateFor: nil, create: true)
        let dir = base.appending(path: "clench", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir
    }
}
