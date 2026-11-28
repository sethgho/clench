import Foundation

/// What arrives from a share sheet or the paste box, before it is queued.
public enum SharedPayload: Sendable, Equatable {
    /// A link to fetch later.
    case url(URL, title: String?)
    /// A page as the browser rendered it (from Safari's preprocessing).
    case page(html: String, url: URL?, title: String?)
    /// Plain text, read as written.
    case text(String, title: String?)
    /// Pasted text whose format (HTML, Markdown or plain) the engine detects.
    case pasted(String)
    /// A file's bytes (a PDF, a text or Markdown file) with its type.
    case file(Data, contentType: String?, name: String)

    /// A queue item for this payload plus the bytes to store with it.
    public func queueEntry(added: Date = .now) -> (item: QueueItem, content: Data?, contentType: String?) {
        switch self {
        case let .url(url, title):
            let item = QueueItem(url: url, title: Self.title(title, fallback: url.host() ?? url.absoluteString), added: added)
            return (item, nil, nil)
        case let .page(html, url, title):
            let item = QueueItem(url: url, title: Self.title(title, fallback: url?.host() ?? "Web page"), added: added)
            return (item, Data(html.utf8), "text/html; charset=utf-8")
        case let .text(text, title):
            let item = QueueItem(url: nil, title: Self.title(title, fallback: Self.firstLine(of: text)), added: added)
            return (item, Data(text.utf8), "text/plain; charset=utf-8")
        case let .pasted(text):
            let item = QueueItem(url: nil, title: Self.firstLine(of: text), added: added)
            return (item, Data(text.utf8), nil)
        case let .file(data, contentType, name):
            let title = (name as NSString).deletingPathExtension
            let item = QueueItem(url: nil, title: Self.title(title, fallback: "Document"), added: added)
            return (item, data, contentType)
        }
    }

    private static func title(_ given: String?, fallback: String) -> String {
        let trimmed = given?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return trimmed.isEmpty ? fallback : trimmed
    }

    private static func firstLine(of text: String, limit: Int = 80) -> String {
        let line = text.split(whereSeparator: \.isNewline).first.map(String.init) ?? ""
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        if trimmed.isEmpty { return "Pasted text" }
        return trimmed.count > limit ? String(trimmed.prefix(limit)) + "…" : trimmed
    }
}
