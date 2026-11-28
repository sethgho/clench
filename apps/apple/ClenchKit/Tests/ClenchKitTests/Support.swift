import Foundation

enum Repo {
    /// The repository root, found from this file's location.
    static let root: URL = URL(filePath: #filePath)
        .deletingLastPathComponent() // ClenchKitTests
        .deletingLastPathComponent() // Tests
        .deletingLastPathComponent() // ClenchKit
        .deletingLastPathComponent() // apple
        .deletingLastPathComponent() // apps
        .deletingLastPathComponent()

    static func fixture(_ name: String) throws -> (data: Data, url: URL?) {
        let file = root.appending(path: "fixtures/\(name)")
        let data = try Data(contentsOf: file)
        let urlFile = file.deletingPathExtension().appendingPathExtension("url")
        let url = (try? String(contentsOf: urlFile, encoding: .utf8))
            .flatMap { URL(string: $0.trimmingCharacters(in: .whitespacesAndNewlines)) }
        return (data, url)
    }

    static func temporaryDirectory() throws -> URL {
        let dir = FileManager.default.temporaryDirectory
            .appending(path: "clenchkit-tests-\(UUID().uuidString)", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir
    }
}
