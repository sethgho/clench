import Foundation

/// The app's `clench://` URLs.
///
/// * `clench://read?id=<uuid>` opens a queued item in the reader.
/// * `clench://queue` shows the queue.
public enum DeepLink: Equatable, Sendable {
    case read(UUID)
    case queue

    public static let scheme = "clench"

    public init?(url: URL) {
        guard url.scheme?.lowercased() == Self.scheme else { return nil }
        switch url.host()?.lowercased() {
        case "read":
            let id = URLComponents(url: url, resolvingAgainstBaseURL: false)?
                .queryItems?.first { $0.name == "id" }?.value
            guard let id, let uuid = UUID(uuidString: id) else { return nil }
            self = .read(uuid)
        case "queue", nil:
            self = .queue
        default:
            return nil
        }
    }

    public var url: URL {
        var c = URLComponents()
        c.scheme = Self.scheme
        switch self {
        case let .read(id):
            c.host = "read"
            c.queryItems = [URLQueryItem(name: "id", value: id.uuidString)]
        case .queue:
            c.host = "queue"
        }
        return c.url!
    }
}
