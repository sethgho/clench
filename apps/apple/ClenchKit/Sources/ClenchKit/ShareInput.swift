import Foundation
import UniformTypeIdentifiers

/// Reads what a share sheet hands an extension into a ``SharedPayload``.
///
/// Preference order: Safari's preprocessing result (selected text, else the
/// rendered page), then a web URL, then plain text.
public enum ShareInput {
    /// The dictionary key under which Safari delivers the result of the
    /// extension's `Preprocess.js`.
    static let preprocessingResultsKey = "NSExtensionJavaScriptPreprocessingResultsKey"

    @MainActor
    public static func payload(from items: [NSExtensionItem]) async -> SharedPayload? {
        let providers = items.flatMap { $0.attachments ?? [] }
        let title = items.lazy.compactMap { $0.attributedContentText?.string }.first

        for provider in providers where provider.hasItemConformingToTypeIdentifier(UTType.propertyList.identifier) {
            if let dict = try? await provider.loadItem(forTypeIdentifier: UTType.propertyList.identifier) as? NSDictionary,
               let results = dict[preprocessingResultsKey] as? [String: Any],
               let payload = payload(fromPreprocessing: results) {
                return payload
            }
        }
        for provider in providers where provider.hasItemConformingToTypeIdentifier(UTType.url.identifier) {
            if let url = try? await provider.loadItem(forTypeIdentifier: UTType.url.identifier) as? URL,
               url.scheme == "http" || url.scheme == "https" {
                return .url(url, title: title)
            }
        }
        for provider in providers where provider.hasItemConformingToTypeIdentifier(UTType.plainText.identifier) {
            if let text = try? await provider.loadItem(forTypeIdentifier: UTType.plainText.identifier) as? String {
                return payload(fromText: text, title: nil)
            }
        }
        return nil
    }

    /// The payload for Safari's preprocessing result: selected text when
    /// there is a selection, otherwise the rendered page.
    public static func payload(fromPreprocessing results: [String: Any]) -> SharedPayload? {
        let url = (results["url"] as? String).flatMap(URL.init(string:))
        let title = results["title"] as? String
        if let selection = results["selection"] as? String,
           !selection.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return .text(selection, title: title)
        }
        if let html = results["html"] as? String, !html.isEmpty {
            return .page(html: html, url: url, title: title)
        }
        return url.map { .url($0, title: title) }
    }

    /// Text that is a single web address becomes a link; anything else is
    /// read as plain text.
    public static func payload(fromText text: String, title: String?) -> SharedPayload? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }
        if let url = webURL(trimmed) { return .url(url, title: title) }
        return .text(text, title: title)
    }

    /// Like ``payload(fromText:title:)``, but text that is not a link is
    /// left for the engine to recognise as HTML, Markdown or plain text.
    public static func payload(fromPaste text: String) -> SharedPayload? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }
        if let url = webURL(trimmed) { return .url(url, title: nil) }
        return .pasted(text)
    }

    private static func webURL(_ trimmed: String) -> URL? {
        guard !trimmed.contains(where: \.isWhitespace),
              let url = URL(string: trimmed),
              url.scheme == "http" || url.scheme == "https",
              url.host() != nil
        else { return nil }
        return url
    }
}
